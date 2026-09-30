#!/usr/bin/env python3
"""Deterministic local WebSocket fixture for the RF-042 SDK contract.

This intentionally implements only the RFC 6455 pieces needed by the three
SDK tests. It is not a daemon replacement: it validates subscription frames,
filters a fixed event sequence, and closes the first connection to exercise
client reconnect/resubscription behavior.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
from pathlib import Path
import socket
import socketserver
import struct
import sys
import threading
from typing import Any


MAX_FRAME_BYTES = 64 * 1024
CONTRACT_PATH = Path(__file__).with_name("live-subscription-contract.json")


class FixtureError(Exception):
    """Raised when an SDK violates the fixture contract."""


def load_contract() -> dict[str, Any]:
    with CONTRACT_PATH.open(encoding="utf-8") as contract_file:
        contract = json.load(contract_file)
    if contract.get("protocol") != "rf-042-live-subscription-v1":
        raise FixtureError("unexpected RF-042 contract version")
    return contract


def read_exact(connection: socket.socket, size: int) -> bytes:
    chunks = bytearray()
    while len(chunks) < size:
        chunk = connection.recv(size - len(chunks))
        if not chunk:
            raise EOFError("WebSocket peer closed the connection")
        chunks.extend(chunk)
    return bytes(chunks)


def read_http_request(connection: socket.socket) -> tuple[str, dict[str, str]]:
    request = bytearray()
    while b"\r\n\r\n" not in request:
        request.extend(connection.recv(4096))
        if len(request) > 16 * 1024:
            raise FixtureError("WebSocket handshake headers exceeded the fixture limit")
    header_block = bytes(request).split(b"\r\n\r\n", 1)[0]
    lines = header_block.decode("latin-1").split("\r\n")
    method, target, _version = lines[0].split(" ", 2)
    if method != "GET":
        raise FixtureError("WebSocket fixture requires a GET handshake")
    headers: dict[str, str] = {}
    for line in lines[1:]:
        if ":" in line:
            name, value = line.split(":", 1)
            headers[name.strip().lower()] = value.strip()
    return target, headers


def accept_handshake(connection: socket.socket, contract: dict[str, Any]) -> None:
    target, headers = read_http_request(connection)
    if target.split("?", 1)[0] != contract["path"]:
        raise FixtureError(f"unexpected WebSocket path: {target}")
    key = headers.get("sec-websocket-key")
    if not key:
        raise FixtureError("WebSocket handshake omitted Sec-WebSocket-Key")
    accept = base64.b64encode(
        # RFC 6455 mandates SHA-1 for Sec-WebSocket-Accept; this is a
        # handshake digest, not a security or content-integrity hash.
        hashlib.sha1(  # nosemgrep: python.lang.security.insecure-hash-algorithms.insecure-hash-algorithm-sha1
            (key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")
        ).digest()
    ).decode("ascii")
    protocol = headers.get("sec-websocket-protocol")
    selected_protocol = ""
    if protocol:
        selected_protocol = protocol.split(",", 1)[0].strip()
    response = (
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        f"Sec-WebSocket-Accept: {accept}\r\n"
        + (f"Sec-WebSocket-Protocol: {selected_protocol}\r\n" if selected_protocol else "")
        + "\r\n"
    )
    connection.sendall(response.encode("ascii"))


def read_frame(connection: socket.socket) -> tuple[int, bytes]:
    first, second = read_exact(connection, 2)
    opcode = first & 0x0F
    masked = bool(second & 0x80)
    length = second & 0x7F
    if length == 126:
        length = struct.unpack("!H", read_exact(connection, 2))[0]
    elif length == 127:
        length = struct.unpack("!Q", read_exact(connection, 8))[0]
    if length > MAX_FRAME_BYTES:
        raise FixtureError("WebSocket frame exceeded the fixture limit")
    if not masked:
        raise FixtureError("client WebSocket frames must be masked")
    mask = read_exact(connection, 4)
    payload = bytearray(read_exact(connection, length))
    for index in range(length):
        payload[index] ^= mask[index % 4]
    return opcode, bytes(payload)


def send_frame(connection: socket.socket, opcode: int, payload: bytes = b"") -> None:
    if len(payload) > MAX_FRAME_BYTES:
        raise FixtureError("fixture response exceeded the frame limit")
    if len(payload) < 126:
        header = bytes((0x80 | opcode, len(payload)))
    elif len(payload) <= 0xFFFF:
        header = bytes((0x80 | opcode, 126)) + struct.pack("!H", len(payload))
    else:
        header = bytes((0x80 | opcode, 127)) + struct.pack("!Q", len(payload))
    connection.sendall(header + payload)


def send_json(connection: socket.socket, value: dict[str, Any]) -> None:
    send_frame(connection, 0x1, json.dumps(value, separators=(",", ":")).encode("utf-8"))


class FixtureState:
    def __init__(self, contract: dict[str, Any]) -> None:
        self.contract = contract
        self.lock = threading.Lock()
        self.connections = 0
        self.failure: str | None = None

    def connection_number(self) -> int:
        with self.lock:
            self.connections += 1
            return self.connections

    def fail(self, message: str) -> None:
        with self.lock:
            if self.failure is None:
                self.failure = message
                print(f"ERROR {message}", file=sys.stderr, flush=True)


def read_json(connection: socket.socket) -> dict[str, Any]:
    while True:
        opcode, payload = read_frame(connection)
        if opcode == 0x8:
            raise EOFError("WebSocket peer sent close")
        if opcode == 0x9:
            send_frame(connection, 0xA, payload)
            continue
        if opcode != 0x1:
            raise FixtureError(f"unexpected client WebSocket opcode: {opcode}")
        try:
            message = json.loads(payload.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise FixtureError(f"invalid JSON subscription frame: {error}") from error
        if not isinstance(message, dict):
            raise FixtureError("subscription frame must be a JSON object")
        return message


def frame_topics(message: dict[str, Any], operation: str) -> set[str]:
    if message.get("type") != operation:
        raise FixtureError(f"expected {operation} frame, got {message.get('type')!r}")
    data = message.get("data")
    if not isinstance(data, dict) or not isinstance(data.get("topics"), list):
        raise FixtureError("subscription frame must use data.topics")
    topics = data["topics"]
    if not all(isinstance(topic, str) and topic for topic in topics):
        raise FixtureError("subscription topics must be non-empty strings")
    return set(topics)


def send_selected(
    connection: socket.socket,
    event: dict[str, Any],
    selected_topics: set[str],
) -> bool:
    if event["type"] not in selected_topics and event["topic"] not in selected_topics:
        return False
    send_json(connection, event)
    return True


class FixtureHandler(socketserver.BaseRequestHandler):
    def handle(self) -> None:
        state: FixtureState = self.server.fixture_state  # type: ignore[attr-defined]
        connection_number = state.connection_number()
        connection = self.request
        connection.settimeout(10)
        try:
            accept_handshake(connection, state.contract)
            if connection_number == 1:
                self.handle_first_connection(connection, state)
            elif connection_number == 2:
                self.handle_reconnect(connection, state)
            else:
                raise FixtureError("fixture accepts exactly one reconnect")
        except (EOFError, OSError) as error:
            if connection_number == 2 and not state.failure:
                return
            state.fail(f"connection {connection_number}: {error}")
        except FixtureError as error:
            state.fail(f"connection {connection_number}: {error}")
        finally:
            try:
                connection.close()
            except OSError:
                pass

    def handle_first_connection(
        self,
        connection: socket.socket,
        state: FixtureState,
    ) -> None:
        contract = state.contract
        selected_topics = frame_topics(
            read_json(connection), "subscribe"
        )
        expected_topics = set(contract["frames"]["subscribe"]["data"]["topics"])
        if selected_topics != expected_topics:
            raise FixtureError(
                f"initial topics differ: expected {sorted(expected_topics)}, "
                f"got {sorted(selected_topics)}"
            )
        send_selected(connection, contract["events"]["initial"], selected_topics)

        removed_topics = frame_topics(read_json(connection), "unsubscribe")
        expected_removed = set(contract["frames"]["unsubscribe"]["data"]["topics"])
        if removed_topics != expected_removed:
            raise FixtureError(
                f"unsubscribe topics differ: expected {sorted(expected_removed)}, "
                f"got {sorted(removed_topics)}"
            )
        selected_topics -= removed_topics
        # The filtered event is deliberately considered but not sent. This is
        # the server-side delivery proof rather than client-side dispatch only.
        send_selected(connection, contract["events"]["filtered"], selected_topics)
        send_selected(connection, contract["events"]["afterUnsubscribe"], selected_topics)
        send_frame(connection, 0x8, struct.pack("!H", 1000))

    def handle_reconnect(
        self,
        connection: socket.socket,
        state: FixtureState,
    ) -> None:
        selected_topics = frame_topics(read_json(connection), "subscribe")
        expected_topics = set(state.contract["frames"]["reconnectSubscribeTopics"])
        if selected_topics != expected_topics:
            raise FixtureError(
                f"reconnect topics differ: expected {sorted(expected_topics)}, "
                f"got {sorted(selected_topics)}"
            )
        send_selected(connection, state.contract["events"]["reconnect"], selected_topics)
        # Keep the connection alive until the SDK intentionally disconnects.
        while True:
            opcode, payload = read_frame(connection)
            if opcode == 0x8:
                return
            if opcode == 0x9:
                send_frame(connection, 0xA, payload)


class FixtureServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=0)
    args = parser.parse_args()
    contract = load_contract()
    state = FixtureState(contract)
    with FixtureServer(("127.0.0.1", args.port), FixtureHandler) as server:
        server.fixture_state = state  # type: ignore[attr-defined]
        print(f"PORT={server.server_address[1]}", flush=True)
        try:
            server.serve_forever(poll_interval=0.05)
        except KeyboardInterrupt:
            pass
        if state.failure is not None:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
