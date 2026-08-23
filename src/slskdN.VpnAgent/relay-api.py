#!/usr/bin/env python3
"""Authenticated, read-only status API for the slskR self-hosted relay."""

from __future__ import annotations

import glob
import hmac
import json
import os
import pathlib
import re
import subprocess
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


STATE_DIR = pathlib.Path(sys.argv[1])
HOST = sys.argv[2]
PORT = int(sys.argv[3])
KEY_FILE = pathlib.Path(os.environ["SLSKR_RELAY_API_KEY_FILE"])


def read_env(path: pathlib.Path) -> dict[str, str]:
    values: dict[str, str] = {}
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            if "=" in line and not line.lstrip().startswith("#"):
                key, value = line.split("=", 1)
                values[key] = value
    except OSError:
        pass
    return values


def run(*args: str, timeout: int = 4) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            args,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return subprocess.CompletedProcess(args, 1, "", "")


def api_keys() -> list[str]:
    try:
        return [
            line.strip()
            for line in KEY_FILE.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
    except OSError:
        return []


def forwards() -> list[dict[str, object]]:
    result: list[dict[str, object]] = []
    for path in sorted(glob.glob(str(STATE_DIR / "pf*.env"))):
        values = read_env(pathlib.Path(path))
        try:
            slot = int(values.get("slot", pathlib.Path(path).stem[2:]))
            local_port = int(values["local_port"])
            target_port = int(values["target_port"])
            public_port = int(values["public_port"])
        except (KeyError, ValueError):
            continue
        result.append(
            {
                "slot": slot,
                "localPort": local_port,
                "targetPort": target_port,
                "proto": values.get("proto", "tcp"),
                "publicPort": public_port,
                "publicIPAddress": values.get("public_ip", "") or None,
                "namespace": values.get("namespace", ""),
            }
        )
    return result


def interface_counter(name: str) -> int:
    path = pathlib.Path("/sys/class/net") / os.environ["SLSKR_RELAY_IFACE"] / "statistics" / name
    try:
        return int(path.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        return 0


def latency(output: str) -> float | None:
    match = re.search(r"(?:time[=<]|\bin\s+)([0-9.]+)\s*(ms|s)\b", output, re.I)
    if not match:
        return None
    value = float(match.group(1))
    return value * 1000 if match.group(2).lower() == "s" else value


def transport_status() -> dict[str, object]:
    tunnel = os.environ["SLSKR_RELAY_TUNNEL_TYPE"].lower()
    home = os.environ["SLSKR_RELAY_HOME_IP"]
    rx = tx = latest = 0
    path = ""
    if tunnel == "tailscale":
        status = run("tailscale", "status", "--json")
        backend = peer_online = False
        if status.returncode == 0:
            try:
                root = json.loads(status.stdout)
                backend = root.get("BackendState") == "Running"
                for peer in (root.get("Peer") or {}).values():
                    if home in peer.get("TailscaleIPs", []):
                        peer_online = bool(peer.get("Online"))
                        rx = int(peer.get("RxBytes", 0) or 0)
                        tx = int(peer.get("TxBytes", 0) or 0)
                        break
            except (TypeError, ValueError):
                pass
        ping = run("tailscale", "ping", "--c", "1", "--timeout", "2s", home)
        path = next((line.strip() for line in ping.stdout.splitlines() if line.strip()), "")
        connected = backend and peer_online and ping.returncode == 0
    else:
        result = run("wg", "show", os.environ["SLSKR_RELAY_IFACE"], "latest-handshakes")
        if result.returncode == 0:
            for line in result.stdout.splitlines():
                fields = line.split()
                for value in fields[1:]:
                    if value.isdigit():
                        latest = max(latest, int(value))
        ping = run("ping", "-n", "-c", "1", "-W", "2", home)
        path = "wireguard" if ping.returncode == 0 else ""
        connected = (
            latest > 0
            and int(time.time()) - latest <= int(os.environ["SLSKR_RELAY_HANDSHAKE_MAX_AGE"])
            and ping.returncode == 0
        )
        rx = interface_counter("rx_bytes")
        tx = interface_counter("tx_bytes")
    active = run(
        "conntrack",
        "-L",
        "-p",
        "tcp",
        "--dport",
        os.environ["SLSKR_RELAY_PUBLIC_PORT"],
    )
    active_connections = (
        len([line for line in active.stdout.splitlines() if line.strip()])
        if active.returncode == 0
        else 0
    )
    latest_activity = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(latest)) if latest else None
    return {
        "connected": connected,
        "latencyMs": latency(ping.stdout),
        "rxBytes": rx,
        "txBytes": tx,
        "latestHandshakeAt": latest_activity,
        "path": path,
        "activeConnections": active_connections,
    }


def relay_status() -> dict[str, object]:
    state = read_env(STATE_DIR / "relay.env")
    transport = transport_status()
    return {
        "mode": "self-hosted-relay",
        "transport": os.environ["SLSKR_RELAY_TUNNEL_TYPE"],
        "connected": transport["connected"],
        "publicIp": state.get("public_ip", ""),
        "publicPort": int(state.get("public_port", "0") or 0),
        "targetPort": int(state.get("target_port", os.environ["SLSKR_RELAY_TARGET_PORT"]) or 0),
        "latencyMs": transport["latencyMs"],
        "rxBytes": transport["rxBytes"],
        "txBytes": transport["txBytes"],
        "activeConnections": transport["activeConnections"],
        "connectionLimit": int(os.environ["SLSKR_RELAY_CONNECTION_LIMIT"]),
        "bandwidthLimitMbit": int(os.environ["SLSKR_RELAY_BANDWIDTH_MBIT"]),
        "latestHandshakeAt": transport["latestHandshakeAt"],
        "path": transport["path"],
    }


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args: object) -> None:
        return

    def send_json(self, status: int, payload: object) -> None:
        body = json.dumps(payload, separators=(",", ":")).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def authorized(self) -> bool:
        supplied = self.headers.get("X-API-Key", "")
        authorization = self.headers.get("Authorization", "")
        if not supplied and authorization.lower().startswith("bearer "):
            supplied = authorization[7:].strip()
        return any(hmac.compare_digest(supplied, key) for key in api_keys())

    def do_GET(self) -> None:  # noqa: N802
        if not self.authorized():
            self.send_json(401, {"error": "unauthorized"})
            return
        path = self.path.split("?", 1)[0]
        state = read_env(STATE_DIR / "relay.env")
        public_ip = state.get("public_ip", "")
        public_port = int(state.get("public_port", "0") or 0)
        transport = transport_status()
        connected = bool(transport["connected"])
        if path == "/v1/publicip/ip":
            self.send_json(
                200,
                {
                    "public_ip": public_ip if connected else "",
                    "city": "",
                    "country": "",
                    "region": "",
                    "location": "",
                    "organization": "self-hosted relay",
                    "postal_code": "",
                    "timezone": "",
                },
            )
        elif path in ("/v1/portforward", "/v1/openvpn/portforwarded"):
            self.send_json(200, {"port": public_port})
        elif path in ("/v1/slskr/portforwards", "/v1/slskdN/portforwards"):
            target_port = int(state.get("target_port", "0") or 0)
            self.send_json(
                200,
                {
                    "mode": "self-hosted-relay",
                    "claimed": 1 if public_port > 0 else 0,
                    "forwards": [
                        {
                            "slot": 0,
                            "localPort": target_port,
                            "targetPort": target_port,
                            "proto": "tcp",
                            "publicPort": public_port,
                            "publicIPAddress": public_ip,
                            "namespace": os.environ["SLSKR_RELAY_IFACE"],
                        }
                    ],
                },
            )
        elif path in ("/v1/slskr/relay", "/v1/slskdN/relay"):
            self.send_json(200, relay_status())
        elif path in ("/v1/openvpn/status", "/v1/wireguard/status", "/v1/vpn/status"):
            self.send_json(200, {"status": "running" if connected else "stopped"})
        else:
            self.send_json(404, {"error": "not found"})


ThreadingHTTPServer((HOST, PORT), Handler).serve_forever()
