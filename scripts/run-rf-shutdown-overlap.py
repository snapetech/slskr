#!/usr/bin/env python3
"""Retain bounded live proof of shutdown during a real filesystem share scan."""

from __future__ import annotations

import argparse
import concurrent.futures
import datetime
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import sqlite3
import struct
import subprocess
import tempfile
import time
import urllib.error
import urllib.request


def file_digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main() -> None:
    repo = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=repo / "target/debug/slskr")
    parser.add_argument("--output", type=Path, default=repo / "target/rf-live-shutdown-overlap.json")
    parser.add_argument("--fixture-files", type=int, default=20_000)
    args = parser.parse_args()
    if os.name != "posix":
        parser.error("this proof requires Unix SIGTERM semantics")
    if not 1_000 <= args.fixture_files <= 200_000:
        parser.error("--fixture-files must be between 1000 and 200000")
    binary = args.binary.resolve(strict=True)
    record = {
        "schemaVersion": 1,
        "sourceSha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
        "dirtyWorktree": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=repo, text=True)),
        "startedUtc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "binarySha256": file_digest(binary),
        "harnessSha256": file_digest(Path(__file__)),
        "rustVersion": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "case": "live-share-scan-distributed-SIGTERM-and-crash-restart",
        "controllerProfile": "native",
        "fixtureFiles": args.fixture_files,
        "disabledServices": ["dht", "mesh", "overlay", "https"],
    }
    with tempfile.TemporaryDirectory(prefix="slskr-rf-shutdown-") as work:
        root = Path(work)
        shares = root / "shares"
        shares.mkdir()
        for index in range(args.fixture_files):
            directory = shares / f"d{index // 200:04d}"
            if index % 200 == 0:
                directory.mkdir()
            (directory / f"f{index % 200:03d}.bin").touch()
        config = root / "config.toml"
        config.write_text(
            "[dht]\nenabled = false\n[mesh]\nenabled = false\n"
            "enable_overlay = false\nenable_dht = false\nenable_stun = false\n"
            "[overlay]\nenable = false\n[overlay_data]\nenable = false\n"
            "[mesh_gateway]\nenabled = false\n"
        )
        env = {key: value for key, value in os.environ.items() if not key.startswith(("SLSKR_", "SLSKD_"))}
        with socket.socket() as http_reservation, socket.socket() as peer_reservation:
            http_reservation.bind(("127.0.0.1", 0))
            peer_reservation.bind(("127.0.0.1", 0))
            http_port = http_reservation.getsockname()[1]
            peer_port = peer_reservation.getsockname()[1]
        env.update(
            SLSKR_CONFIG=str(config), SLSKR_STATE_DIR=str(root / "state"),
            SLSKR_HTTP_BIND=f"127.0.0.1:{http_port}", SLSKR_AUTH_DISABLED="true",
            SLSKD_NO_HTTPS="true", SLSKR_LISTENER_BIND=f"127.0.0.1:{peer_port}",
            SLSKR_SHARE_DIRS=str(shares), SLSKR_SHARES_PROBE_MEDIA_ATTRIBUTES="false",
            SLSKR_CONTROLLER_PROFILE="native", SLSKR_PERSISTENCE_ENABLED="true",
            SLSKD_SLSK_USERNAME="rf-fixture",
        )
        command = [str(binary), "serve", "--no-connect", "--no-share-scan", "--no-logo", "--no-version-check"]
        base = f"http://127.0.0.1:{http_port}"

        def request(path: str, method: str = "GET") -> bytes:
            req = urllib.request.Request(base + path, data=b"" if method == "PUT" else None, method=method)
            with urllib.request.urlopen(req, timeout=5) as response:
                return response.read()

        def ready(proc: subprocess.Popen) -> dict:
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                if proc.poll() is not None:
                    raise RuntimeError("daemon exited before readiness")
                try:
                    return json.loads(request("/api/v0/application"))
                except urllib.error.URLError:
                    time.sleep(.02)
            raise RuntimeError("daemon readiness timeout")

        def stop(proc: subprocess.Popen) -> tuple[int, float]:
            started = time.monotonic()
            proc.send_signal(signal.SIGTERM)
            return proc.wait(timeout=10), round(time.monotonic() - started, 6)

        def cleanup(proc: subprocess.Popen) -> None:
            if proc.poll() is None:
                proc.kill()
                proc.wait(timeout=5)

        def distributed_snapshot() -> dict:
            with sqlite3.connect(root / "state/slskr.db") as db:
                db.execute("BEGIN")
                tree = db.execute("select branch_level, branch_root, parent_username from distributed_tree_state where id = 1").fetchone()
                children = db.execute("select username, depth from distributed_children order by username").fetchall()
                return {"tree": list(tree) if tree else None, "children": [list(row) for row in children]}

        def recv_exact(peer: socket.socket, length: int) -> bytes:
            result = bytearray()
            while len(result) < length:
                chunk = peer.recv(length - len(result))
                if not chunk:
                    raise RuntimeError("distributed fixture socket closed during initialization")
                result.extend(chunk)
            return bytes(result)

        def distributed_child(depth: int) -> tuple[socket.socket, dict]:
            deadline = time.monotonic() + 5
            while True:
                try:
                    peer = socket.create_connection(("127.0.0.1", peer_port), timeout=5)
                    break
                except ConnectionRefusedError:
                    if time.monotonic() >= deadline:
                        raise
                    time.sleep(.02)
            try:
                def string(value: bytes) -> bytes:
                    return struct.pack("<I", len(value)) + value

                init = b"\x01" + string(b"child") + string(b"D") + struct.pack("<I", 0)
                peer.sendall(struct.pack("<I", len(init)) + init)
                for code in (4, 5):
                    length = struct.unpack("<I", recv_exact(peer, 4))[0]
                    if not 1 <= length <= 8192:
                        raise RuntimeError("invalid distributed fixture frame length")
                    frame = recv_exact(peer, length)
                    if frame[0] != code:
                        raise RuntimeError("unexpected distributed branch initialization")
                payload = b"\x07" + struct.pack("<I", depth)
                peer.sendall(struct.pack("<I", len(payload)) + payload)
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    snapshot = distributed_snapshot()
                    if snapshot["children"] == [["child", depth]]:
                        return peer, snapshot
                    time.sleep(.002)
                raise RuntimeError("distributed child depth did not persist")
            except BaseException:
                peer.close()
                raise

        peer = None
        with (root / "daemon.log").open("w") as log:
            proc = subprocess.Popen(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                record["initialShares"] = ready(proc)["shares"]
                peer, record["distributedBeforeShutdown"] = distributed_child(3)
                with concurrent.futures.ThreadPoolExecutor(max_workers=1) as executor:
                    pending = executor.submit(request, "/api/shares", "PUT")
                    deadline = time.monotonic() + 5
                    while time.monotonic() < deadline:
                        observed = json.loads(request("/api/v0/application"))["shares"]
                        if observed.get("scanning"):
                            record["observedScanning"] = observed
                            break
                        if pending.done():
                            pending.result()
                            raise RuntimeError("scan finished before shutdown overlap was observed")
                        time.sleep(.001)
                    else:
                        raise RuntimeError("active scan was not observed")
                    record["exitCode"], record["shutdownSeconds"] = stop(proc)
                    record["distributedSocketClosedAtShutdown"] = peer.recv(1) == b""
                    try:
                        pending.result(timeout=6)
                        record["scanRequestOutcome"] = "returned"
                    except urllib.error.HTTPError as error:
                        record["scanRequestOutcome"] = "http-error"
                        record["scanRequestHttpStatus"] = error.code
                    except (urllib.error.URLError, ConnectionError) as error:
                        record["scanRequestOutcome"] = type(error).__name__
                if record["exitCode"] != 0:
                    raise RuntimeError("daemon did not exit cleanly during the scan")
                if "shutdown signal received" not in (root / "daemon.log").read_text():
                    raise RuntimeError("graceful shutdown signal was not recorded")
                with sqlite3.connect(root / "state/slskr.db") as db:
                    record["durableShareRowsAfterShutdown"] = db.execute("select count(*) from share_files").fetchone()[0]
                    record["sqliteIntegrity"] = db.execute("pragma integrity_check").fetchone()[0]
                if record["durableShareRowsAfterShutdown"] != 0 or record["sqliteIntegrity"] != "ok":
                    raise RuntimeError("shutdown published a partial or invalid durable share index")
                record["distributedAfterShutdown"] = distributed_snapshot()
                expected_shutdown = {"tree": [0, "rf-fixture", None], "children": []}
                if record["distributedAfterShutdown"] != expected_shutdown:
                    raise RuntimeError(f"shutdown did not persist the final disconnected snapshot: {record['distributedAfterShutdown']!r}")
                if not record["distributedSocketClosedAtShutdown"]:
                    raise RuntimeError("shutdown left the distributed socket open")
            finally:
                cleanup(proc)
                if peer is not None:
                    peer.close()
                    peer = None
        with (root / "restart.log").open("w") as log:
            proc = subprocess.Popen(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                record["restartedShares"] = ready(proc)["shares"]
                if record["restartedShares"]["files"] != 0:
                    raise RuntimeError("restart restored a partial share index")
                record["distributedAfterRestart"] = distributed_snapshot()
                if record["distributedAfterRestart"] != record["distributedAfterShutdown"]:
                    raise RuntimeError("restart changed the retained distributed snapshot")
                peer, record["distributedBeforeCrash"] = distributed_child(7)
                proc.kill()
                record["crashExitCode"] = proc.wait(timeout=5)
                if record["crashExitCode"] != -signal.SIGKILL:
                    raise RuntimeError("forced crash did not use SIGKILL")
            finally:
                cleanup(proc)
                if peer is not None:
                    peer.close()
                    peer = None
        with (root / "crash-restart.log").open("w") as log:
            proc = subprocess.Popen(command, cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                record["crashRestartShares"] = ready(proc)["shares"]
                record["distributedAfterCrashRestart"] = distributed_snapshot()
                if record["distributedAfterCrashRestart"] != record["distributedBeforeCrash"]:
                    raise RuntimeError("crash/restart lost the committed distributed snapshot")
                with sqlite3.connect(root / "state/slskr.db") as db:
                    record["crashRestartSqliteIntegrity"] = db.execute("pragma integrity_check").fetchone()[0]
                if record["crashRestartSqliteIntegrity"] != "ok" or record["crashRestartShares"]["files"] != 0:
                    raise RuntimeError("crash/restart restored corrupt or partial state")
                record["restartExitCode"], record["restartShutdownSeconds"] = stop(proc)
                if record["restartExitCode"] != 0:
                    raise RuntimeError("restarted daemon did not exit cleanly")
            finally:
                cleanup(proc)
    record["passed"] = True
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    main()
