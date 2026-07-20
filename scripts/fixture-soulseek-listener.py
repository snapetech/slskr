#!/usr/bin/env python3
"""Small TCP fixture that records accepted and currently active connections."""

from __future__ import annotations

import hashlib
import json
import os
import signal
import socket
import struct
import sys
import tempfile
import threading
import time
import zlib
from pathlib import Path


if len(sys.argv) not in {4, 5, 6, 9}:
    raise SystemExit(
        "usage: fixture-soulseek-listener.py HOST PORT STATUS_FILE [login-success] "
        "[login-success-private INJECTION_FILE] "
        "[PEER_HOST REGULAR_PORT OBFUSCATED_PORT obfuscated-only|regular-only|both]"
    )

host = sys.argv[1]
port = int(sys.argv[2])
status_path = Path(sys.argv[3])
login_success = len(sys.argv) == 5 and sys.argv[4] == "login-success"
private_message_fixture = len(sys.argv) == 6
if private_message_fixture and sys.argv[4] != "login-success-private":
    raise SystemExit("private-message fixture requires login-success-private mode")
private_message_injection_path = Path(sys.argv[5]) if private_message_fixture else None
peer_fixture = len(sys.argv) == 9
if peer_fixture and sys.argv[4] != "login-success":
    raise SystemExit("peer endpoint fixture requires login-success mode")
login_success = login_success or private_message_fixture or peer_fixture
peer_host = sys.argv[5] if peer_fixture else ""
peer_regular_port = int(sys.argv[6]) if peer_fixture else 0
peer_obfuscated_port = int(sys.argv[7]) if peer_fixture else 0
peer_listener_mode = sys.argv[8] if peer_fixture else ""
if peer_fixture and peer_listener_mode not in {"obfuscated-only", "regular-only", "both"}:
    raise SystemExit(f"invalid peer listener mode: {peer_listener_mode}")
lock = threading.Lock()
accepted = 0
active = 0
set_wait_ports: list[int] = []
set_wait_port_messages: list[dict[str, int | None]] = []
login_usernames: list[str] = []
login_password_sha256: list[str] = []
peer_address_requests: list[str] = []
peer_accept_order: list[str] = []
regular_peer_accepts = 0
obfuscated_peer_accepts = 0
peer_message_codes: list[int] = []
peer_search_response_tokens: list[int] = []
private_message_acks: list[int] = []
private_message_responses: list[dict[str, str]] = []
injected_private_message_ids: list[int] = []
stopping = threading.Event()
listeners: list[socket.socket] = []


def write_status() -> None:
    with lock:
        value = {
            "accepted": accepted,
            "active": active,
            "set_wait_ports": list(set_wait_ports),
            "set_wait_port_messages": list(set_wait_port_messages),
            "login_usernames": list(login_usernames),
            "login_password_sha256": list(login_password_sha256),
            "peer_address_requests": list(peer_address_requests),
            "peer_accept_order": list(peer_accept_order),
            "regular_peer_accepts": regular_peer_accepts,
            "obfuscated_peer_accepts": obfuscated_peer_accepts,
            "peer_message_codes": list(peer_message_codes),
            "peer_search_response_tokens": list(peer_search_response_tokens),
            "private_message_acks": list(private_message_acks),
            "private_message_responses": list(private_message_responses),
            "injected_private_message_ids": list(injected_private_message_ids),
        }
    status_path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{status_path.name}.", suffix=".tmp", dir=status_path.parent
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, sort_keys=True, separators=(",", ":"))
        os.replace(temporary, status_path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def handle_connection(connection: socket.socket) -> None:
    global active
    buffered = bytearray()
    login_answered = False
    try:
        connection.settimeout(0.25)
        while not stopping.is_set():
            try:
                if (
                    private_message_fixture
                    and login_answered
                    and private_message_injection_path is not None
                    and private_message_injection_path.exists()
                ):
                    try:
                        injections = json.loads(
                            private_message_injection_path.read_text(encoding="utf-8")
                        )
                    except (OSError, json.JSONDecodeError):
                        injections = []
                    for injection in injections if isinstance(injections, list) else []:
                        if not isinstance(injection, dict):
                            continue
                        message_id = injection.get("id")
                        username = injection.get("username")
                        message = injection.get("message")
                        is_new = injection.get("is_new", True)
                        if (
                            not isinstance(message_id, int)
                            or not isinstance(username, str)
                            or not isinstance(message, str)
                            or not isinstance(is_new, bool)
                        ):
                            continue
                        with lock:
                            already_injected = message_id in injected_private_message_ids
                        if already_injected:
                            continue
                        username_bytes = username.encode("utf-8")
                        message_bytes = message.encode("utf-8")
                        response_payload = (
                            struct.pack("<II", message_id, int(time.time()))
                            + struct.pack("<I", len(username_bytes))
                            + username_bytes
                            + struct.pack("<I", len(message_bytes))
                            + message_bytes
                            + bytes((1 if is_new else 0,))
                        )
                        connection.sendall(
                            struct.pack("<II", len(response_payload) + 4, 22)
                            + response_payload
                        )
                        with lock:
                            injected_private_message_ids.append(message_id)
                        write_status()
                data = connection.recv(65_536)
                if not data:
                    break
                buffered.extend(data)
                while len(buffered) >= 4:
                    length = struct.unpack_from("<I", buffered)[0]
                    if length < 4 or len(buffered) < length + 4:
                        break
                    code = struct.unpack_from("<I", buffered, 4)[0]
                    payload = bytes(buffered[8 : length + 4])
                    del buffered[: length + 4]
                    if code == 2 and len(payload) >= 4:
                        advertised_port = struct.unpack_from("<I", payload)[0]
                        obfuscation_type = (
                            struct.unpack_from("<I", payload, 4)[0]
                            if len(payload) >= 8
                            else None
                        )
                        obfuscated_port = (
                            struct.unpack_from("<I", payload, 8)[0]
                            if len(payload) >= 12
                            else None
                        )
                        with lock:
                            set_wait_ports.append(advertised_port)
                            set_wait_port_messages.append(
                                {
                                    "port": advertised_port,
                                    "obfuscation_type": obfuscation_type,
                                    "obfuscated_port": obfuscated_port,
                                    "payload_length": len(payload),
                                }
                            )
                        write_status()
                    if private_message_fixture and code == 23 and len(payload) >= 4:
                        with lock:
                            private_message_acks.append(struct.unpack_from("<I", payload)[0])
                        write_status()
                    if private_message_fixture and code == 22 and len(payload) >= 8:
                        username_size = struct.unpack_from("<I", payload)[0]
                        message_size_offset = 4 + username_size
                        if len(payload) >= message_size_offset + 4:
                            message_size = struct.unpack_from(
                                "<I", payload, message_size_offset
                            )[0]
                            message_offset = message_size_offset + 4
                            if len(payload) >= message_offset + message_size:
                                try:
                                    response_username = payload[
                                        4:message_size_offset
                                    ].decode("utf-8")
                                    response_message = payload[
                                        message_offset : message_offset + message_size
                                    ].decode("utf-8")
                                except UnicodeDecodeError:
                                    pass
                                else:
                                    with lock:
                                        private_message_responses.append(
                                            {
                                                "username": response_username,
                                                "message": response_message,
                                            }
                                        )
                                    write_status()
                    if login_success and code == 36 and len(payload) >= 4:
                        username_size = struct.unpack_from("<I", payload)[0]
                        if len(payload) < 4 + username_size:
                            break
                        username_bytes = payload[4 : 4 + username_size]
                        response_payload = (
                            struct.pack("<I", len(username_bytes))
                            + username_bytes
                            + struct.pack("<IIIII", 1024, 0, 0, 1, 1)
                        )
                        connection.sendall(
                            struct.pack("<II", len(response_payload) + 4, 36)
                            + response_payload
                        )
                    if login_success and code == 92:
                        response_payload = struct.pack("<I", 0)
                        connection.sendall(
                            struct.pack("<II", len(response_payload) + 4, 92)
                            + response_payload
                        )
                    if peer_fixture and code == 3 and len(payload) >= 4:
                        username_size = struct.unpack_from("<I", payload)[0]
                        if len(payload) < 4 + username_size:
                            break
                        username_bytes = payload[4 : 4 + username_size]
                        try:
                            requested_username = username_bytes.decode("utf-8")
                        except UnicodeDecodeError:
                            break
                        with lock:
                            peer_address_requests.append(requested_username)
                        write_status()
                        response_payload = (
                            struct.pack("<I", len(username_bytes))
                            + username_bytes
                            + bytes((1, 0, 0, 127))
                            + struct.pack("<I", peer_regular_port)
                            + struct.pack("<I", 1)
                            + struct.pack("<H", peer_obfuscated_port)
                        )
                        connection.sendall(
                            struct.pack("<II", len(response_payload) + 4, 3)
                            + response_payload
                        )
                    if login_success and code == 1 and not login_answered:
                        cursor = 0

                        def read_login_string() -> bytes:
                            nonlocal cursor
                            if len(payload) < cursor + 4:
                                raise ValueError("truncated login string length")
                            size = struct.unpack_from("<I", payload, cursor)[0]
                            cursor += 4
                            if len(payload) < cursor + size:
                                raise ValueError("truncated login string")
                            value = payload[cursor : cursor + size]
                            cursor += size
                            return value

                        try:
                            username = read_login_string().decode("utf-8")
                            password = read_login_string()
                        except (UnicodeDecodeError, ValueError):
                            break
                        with lock:
                            login_usernames.append(username)
                            login_password_sha256.append(
                                hashlib.sha256(password).hexdigest()
                            )
                        write_status()
                        greeting = b"fixture login accepted"
                        password_hash = b"fixture-hash"
                        payload = (
                            b"\x01"
                            + struct.pack("<I", len(greeting))
                            + greeting
                            + bytes((1, 0, 0, 127))
                            + struct.pack("<I", len(password_hash))
                            + password_hash
                            + b"\x00"
                        )
                        connection.sendall(
                            struct.pack("<II", len(payload) + 4, 1) + payload
                        )
                        login_answered = True
            except TimeoutError:
                continue
            except OSError:
                break
    finally:
        try:
            connection.close()
        except OSError:
            pass
        with lock:
            active -= 1
        write_status()


def handle_peer_connection(connection: socket.socket, transport: str) -> None:
    global regular_peer_accepts, obfuscated_peer_accepts
    with lock:
        peer_accept_order.append(transport)
        if transport == "regular":
            regular_peer_accepts += 1
        else:
            obfuscated_peer_accepts += 1
    write_status()
    buffered = bytearray()
    init_received = False
    try:
        connection.settimeout(0.25)
        deadline = time.monotonic() + 2.0
        while not stopping.is_set() and time.monotonic() < deadline:
            try:
                chunk = connection.recv(65_536)
                if not chunk:
                    break
                buffered.extend(chunk)
                if not init_received and len(buffered) >= 4:
                    init_length = struct.unpack_from("<I", buffered)[0]
                    if len(buffered) >= 4 + init_length:
                        del buffered[: 4 + init_length]
                        init_received = True
                while init_received and len(buffered) >= 4:
                    message_length = struct.unpack_from("<I", buffered)[0]
                    if message_length < 4 or len(buffered) < 4 + message_length:
                        break
                    message = bytes(buffered[4 : 4 + message_length])
                    del buffered[: 4 + message_length]
                    code = struct.unpack_from("<I", message)[0]
                    token = None
                    if code == 9:
                        try:
                            payload = zlib.decompress(message[4:])
                            username_length = struct.unpack_from("<I", payload)[0]
                            token = struct.unpack_from("<I", payload, 4 + username_length)[0]
                        except (OSError, struct.error, zlib.error):
                            token = None
                    with lock:
                        peer_message_codes.append(code)
                        if token is not None:
                            peer_search_response_tokens.append(token)
                    write_status()
            except TimeoutError:
                continue
            except OSError:
                break
    finally:
        try:
            connection.close()
        except OSError:
            pass


def run_peer_listener(
    bind_port: int, transport: str, ready: threading.Event
) -> None:
    peer_listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    peer_listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    peer_listener.bind((peer_host, bind_port))
    peer_listener.listen(16)
    peer_listener.settimeout(0.25)
    listeners.append(peer_listener)
    ready.set()
    while not stopping.is_set():
        try:
            connection, _address = peer_listener.accept()
        except TimeoutError:
            continue
        except OSError:
            break
        threading.Thread(
            target=handle_peer_connection,
            args=(connection, transport),
            daemon=True,
        ).start()


listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind((host, port))
listener.listen(16)
listener.settimeout(0.25)
listeners.append(listener)

peer_ready_events: list[threading.Event] = []
if peer_fixture and peer_listener_mode in {"regular-only", "both"}:
    ready = threading.Event()
    peer_ready_events.append(ready)
    threading.Thread(
        target=run_peer_listener,
        args=(peer_regular_port, "regular", ready),
        daemon=True,
    ).start()
if peer_fixture and peer_listener_mode in {"obfuscated-only", "both"}:
    ready = threading.Event()
    peer_ready_events.append(ready)
    threading.Thread(
        target=run_peer_listener,
        args=(peer_obfuscated_port, "obfuscated", ready),
        daemon=True,
    ).start()
for ready in peer_ready_events:
    if not ready.wait(timeout=2.0):
        raise SystemExit("peer listener did not become ready")


def stop(_signum: int, _frame: object) -> None:
    stopping.set()
    try:
        for active_listener in listeners:
            active_listener.close()
    except OSError:
        pass


signal.signal(signal.SIGTERM, stop)
signal.signal(signal.SIGINT, stop)
write_status()

while not stopping.is_set():
    try:
        connection, _address = listener.accept()
    except TimeoutError:
        continue
    except OSError:
        break
    with lock:
        accepted += 1
        active += 1
    write_status()
    threading.Thread(target=handle_connection, args=(connection,), daemon=True).start()

stopping.set()
