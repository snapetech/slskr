#!/usr/bin/env python3
"""Run one real, bounded lifecycle comparison for the universal gate.

The matrix runner deliberately delegates product-specific work to this file
instead of manufacturing green observations.  Each invocation starts the
selected frozen daemon and a fresh slskR process, performs one lifecycle
operation against both HTTP surfaces, and writes the raw observations below
the supplied case directory.

This is a local no-connect lifecycle probe.  Network interoperability is
covered by the separate live transport runners; keeping this probe offline
makes failure/restart cases deterministic and keeps the memory footprint low.
"""

from __future__ import annotations

import hashlib
import http.client
import json
import os
import shutil
import signal
import socket
import stat
import subprocess
import sys
import time
import urllib.parse
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any


HTTP_TIMEOUT = float(os.environ.get("SLSKR_LIFECYCLE_HTTP_TIMEOUT_SECONDS", "3"))
START_TIMEOUT = float(os.environ.get("SLSKR_LIFECYCLE_START_TIMEOUT_SECONDS", "12"))


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def json_text(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


def tail(path: Path, limit: int = 100) -> str:
    if not path.is_file():
        return ""
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError as error:
        return f"<log read failed: {error}>"
    return "\n".join(lines[-limit:])[-12000:]


def status_class(status: int | None) -> str:
    if status is None:
        return "transport-error"
    return f"{status // 100}xx"


def body_json(body: str) -> Any:
    try:
        return json.loads(body)
    except (TypeError, json.JSONDecodeError):
        return None


def body_count(value: Any) -> int | None:
    if isinstance(value, list):
        return len(value)
    if isinstance(value, dict):
        for key in ("entries", "items", "results", "local", "transfers", "searches"):
            nested = value.get(key)
            if isinstance(nested, list):
                return len(nested)
    return None


class Daemon:
    def __init__(
        self,
        *,
        name: str,
        profile: str,
        binary: Path,
        case_directory: Path,
        replacement: bool,
    ) -> None:
        self.name = name
        self.profile = profile
        self.binary = binary
        self.case_directory = case_directory
        self.replacement = replacement
        self.root = case_directory / name
        self.root.mkdir(parents=True, exist_ok=True)
        self.app_directory = self.root / "app"
        self.state_directory = self.root / "state"
        self.share_directory = self.root / "share"
        self.download_directory = self.root / "downloads"
        self.incomplete_directory = self.root / "incomplete"
        for directory in (
            self.app_directory,
            self.state_directory,
            self.share_directory,
            self.download_directory,
            self.incomplete_directory,
        ):
            directory.mkdir(parents=True, exist_ok=True)
        self.fixture = self.share_directory / "lifecycle-fixture.bin"
        self.fixture.write_bytes(b"slskr universal lifecycle fixture\n")
        self.http_port = free_port()
        self.listen_port = free_port()
        self.dht_port = free_port()
        self.process: subprocess.Popen[bytes] | None = None
        self.log_path = self.root / "daemon.log"
        self.config_path = self.root / "config.yml"
        self.bad_configuration = False
        self.upgrade_marker = False
        self._write_configuration()

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self.http_port}"

    @property
    def health_path(self) -> str:
        return "/api/v0/session" if self.replacement else "/api/v0/application"

    def _write_configuration(self) -> None:
        if self.replacement:
            if self.bad_configuration:
                self.config_path.write_text("[invalid\n", encoding="utf-8")
            else:
                self.config_path.write_text(
                    "[flags]\n"
                    "no_logo = true\n"
                    "no_version_check = true\n"
                    "no_share_scan = true\n"
                    "[app]\n"
                    f'http_bind = "127.0.0.1:{self.http_port}"\n'
                    f'state_dir = "{self.state_directory}"\n'
                    "auto_connect = false\n"
                    "[network]\n"
                    "server_address = \"127.0.0.1:2242\"\n"
                    f"listen_port = {self.listen_port}\n"
                    "[listeners]\n"
                    f'regular_bind = "127.0.0.1:{self.listen_port}"\n'
                    "[web]\n"
                    'content_path = "../../web/build"\n'
                    "[shares]\n"
                    f'dirs = ["{self.share_directory}"]\n',
                    encoding="utf-8",
                )
            return

        if self.profile == "slskdn":
            if self.bad_configuration:
                self.config_path.write_text("web: [invalid\n", encoding="utf-8")
            else:
                self.config_path.write_text(
                    "debug: " + ("true\n" if self.upgrade_marker else "false\n")
                    + "web:\n"
                    + f"  port: {self.http_port}\n"
                    + "  address: 127.0.0.1\n"
                    + "  content_path: .\n"
                    + "  https:\n"
                    + "    disabled: true\n"
                    + "  authentication:\n"
                    + "    disabled: true\n"
                    + "directories:\n"
                    + f"  downloads: {self.download_directory}\n"
                    + f"  incomplete: {self.incomplete_directory}\n"
                    + "shares:\n"
                    + "  directories:\n"
                    + f"    - {self.share_directory}\n"
                    + "  cache:\n"
                    + "    storage_mode: disk\n"
                    + "soulseek:\n"
                    + "  address: 127.0.0.1\n"
                    + "  port: 2242\n"
                    + "  username: \"\"\n"
                    + "  password: \"\"\n"
                    + f"  listen_port: {self.listen_port}\n"
                    + "flags:\n"
                    + "  no_connect: true\n",
                    encoding="utf-8",
                )

        if self.bad_configuration:
            self.config_path.write_text("web: [invalid\n", encoding="utf-8")

    def _command_environment(self) -> tuple[list[str], dict[str, str]]:
        environment = os.environ.copy()
        environment.update(
            {
                "NODE_OPTIONS": "--max-old-space-size=512",
                "DOTNET_GCHeapHardLimit": "536870912",
                "COMPlus_GCHeapHardLimit": "536870912",
            }
        )
        if self.replacement:
            environment.update(
                {
                    "SLSKR_HTTP_BIND": f"127.0.0.1:{self.http_port}",
                    "SLSKR_STATE_DIR": str(self.state_directory),
                    "SLSKR_CONFIG": str(self.config_path),
                    "SLSKR_CONTROLLER_COMPATIBILITY_TARGET": self.profile,
                    "SLSKR_AUTO_CONNECT": "false",
                    "SLSKR_AUTH_DISABLED": "true",
                    "SLSKD_NO_HTTPS": "true",
                    "SLSKR_PERSISTENCE_ENABLED": "true",
                    "SLSKR_SHARE_DIRS": str(self.share_directory),
                    "SLSKR_DOWNLOADS_DIR": str(self.download_directory),
                    "SLSKR_LISTENER_BIND": f"127.0.0.1:{self.listen_port}",
                    "SLSK_SERVER": "127.0.0.1:2242",
                }
            )
            if self.profile == "slskdn":
                environment["SLSKR_DHT_PORT"] = str(self.dht_port)
                environment["SLSKR_OVERLAY_BIND"] = f"127.0.0.1:{self.dht_port}"
            return [str(self.binary), "serve"], environment

        environment.update(
            {
                "SLSKD_APP_DIR": str(self.app_directory),
                "SLSKD_HTTP_PORT": str(self.http_port),
                "SLSKD_HTTP_IP_ADDRESS": "127.0.0.1",
                "SLSKD_NO_HTTPS": "true",
                "SLSKD_NO_AUTH": "true",
                "SLSKD_NO_LOGO": "true",
                "SLSKD_NO_VERSION_CHECK": "true",
                "SLSKD_NO_CONNECT": "true",
                "SLSKD_NO_SHARE_SCAN": "true",
                "SLSKD_INCOMPLETE_DIR": str(self.incomplete_directory),
                "SLSKD_DOWNLOADS_DIR": str(self.download_directory),
                "SLSKD_SHARED_DIR": str(self.share_directory),
                "SLSKD_SLSK_ADDRESS": "127.0.0.1",
                    "SLSKD_SLSK_PORT": "2242",
                "SLSKD_SLSK_LISTEN_IP_ADDRESS": "127.0.0.1",
                "SLSKD_SLSK_LISTEN_PORT": str(self.listen_port),
            }
        )
        if self.profile == "slskdn":
            return [
                str(self.binary),
                "--config",
                str(self.config_path),
                "--app-dir",
                str(self.app_directory),
                "--no-connect",
                "--no-share-scan",
                "--no-version-check",
                "--no-logo",
            ], environment
        command = [
            str(self.binary),
            "--no-connect",
            "--no-share-scan",
            "--no-version-check",
            "--no-logo",
        ]
        if self.bad_configuration:
            command.extend(["--config", str(self.config_path)])
        return command, environment

    def start(self) -> dict[str, Any]:
        if self.process is not None and self.process.poll() is None:
            raise RuntimeError(f"{self.name} is already running")
        self._write_configuration()
        command, environment = self._command_environment()
        log = self.log_path.open("ab")
        try:
            self.process = subprocess.Popen(
                command,
                cwd=self.root,
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
        finally:
            log.close()
        deadline = time.monotonic() + START_TIMEOUT
        last: dict[str, Any] = {"status": None, "error": "not probed"}
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError(
                    f"{self.name} exited with {self.process.returncode}; log tail:\n{tail(self.log_path)}"
                )
            last = self.request("GET", self.health_path)
            if last.get("status") == 200:
                return {"ready": True, "health": last}
            time.sleep(0.2)
        raise RuntimeError(f"{self.name} did not become ready: {last}; log tail:\n{tail(self.log_path)}")

    def stop(self) -> dict[str, Any]:
        process = self.process
        if process is None:
            return {"stopped": True, "returnCode": None}
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
        try:
            return_code = process.wait(timeout=8)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            return_code = process.wait(timeout=3)
        self.process = None
        return {"stopped": True, "returnCode": return_code}

    def request(
        self,
        method: str,
        path: str,
        payload: Any | None = None,
        timeout: float = HTTP_TIMEOUT,
    ) -> dict[str, Any]:
        body = None
        headers = {"Accept": "application/json"}
        if payload is not None:
            body = json_text(payload).encode("utf-8")
            headers["Content-Type"] = "application/json"
        parsed_base_url = urllib.parse.urlsplit(self.base_url)
        if parsed_base_url.scheme not in {"http", "https"} or parsed_base_url.hostname is None:
            raise ValueError(f"unsupported lifecycle endpoint: {self.base_url}")
        connection_type = (
            http.client.HTTPSConnection
            if parsed_base_url.scheme == "https"
            else http.client.HTTPConnection
        )
        connection = connection_type(
            parsed_base_url.hostname,
            parsed_base_url.port,
            timeout=timeout,
        )
        request_path = path if path.startswith("/") else f"/{path}"
        started = time.monotonic()
        try:
            connection.request(method, request_path, body=body, headers=headers)
            response = connection.getresponse()
            raw = response.read(256 * 1024)
            return {
                "status": response.status,
                "statusClass": status_class(response.status),
                "body": raw.decode("utf-8", errors="replace"),
                "durationMs": round((time.monotonic() - started) * 1000, 2),
            }
        except (http.client.HTTPException, TimeoutError, OSError) as error:
            return {
                "status": None,
                "statusClass": "transport-error",
                "error": str(error),
                "durationMs": round((time.monotonic() - started) * 1000, 2),
            }
        finally:
            connection.close()

    def snapshot(self) -> dict[str, Any]:
        paths = {
            "health": self.health_path,
            "application": "/api/v0/application",
            "options": "/api/v0/options",
            "searches": "/api/v0/searches",
            "shares": "/api/v0/shares",
            "logs": "/api/v0/logs",
        }
        responses = {name: self.request("GET", path) for name, path in paths.items()}
        parsed = {name: body_json(response.get("body", "")) for name, response in responses.items()}
        application = parsed.get("application") if isinstance(parsed.get("application"), dict) else {}
        session = parsed.get("health") if isinstance(parsed.get("health"), dict) else {}
        server = application.get("server") if isinstance(application.get("server"), dict) else {}
        shares = parsed.get("shares")
        share_items = body_count(shares)
        if isinstance(shares, dict) and isinstance(shares.get("local"), list):
            share_items = len(shares["local"])
        connected = session.get("state") == "connected" or server.get("isConnected") is True
        return {
            "responses": {
                name: {
                    "status": value.get("status"),
                    "statusClass": value.get("statusClass"),
                    "bodySha256": hashlib.sha256(value.get("body", "").encode()).hexdigest(),
                }
                for name, value in responses.items()
            },
            "connected": connected,
            "searchCount": body_count(parsed.get("searches")),
            "shareCount": share_items,
            "shareReady": shares.get("ready") if isinstance(shares, dict) else None,
            "applicationState": server.get("state") or session.get("state"),
            "optionsPresent": responses["options"].get("status") == 200,
        }

    def files(self) -> list[dict[str, Any]]:
        result = []
        for path in sorted(self.root.rglob("*")):
            if not path.is_file() or path == self.log_path:
                continue
            try:
                result.append(
                    {
                        "path": str(path.relative_to(self.root)),
                        "size": path.stat().st_size,
                        "sha256": sha256(path),
                    }
                )
            except OSError:
                continue
        return result

    def state_candidates(self) -> list[Path]:
        if not self.replacement:
            primary_search_db = self.app_directory / "data" / "search.db"
            if primary_search_db.is_file():
                return [primary_search_db]
        candidates = []
        for base in (self.app_directory, self.state_directory):
            if base.is_dir():
                candidates.extend(
                    path
                    for path in sorted(base.rglob("*"))
                    if path.is_file() and path.suffix.lower() in {".db", ".sqlite", ".sqlite3", ".json"}
                )
        return candidates


def endpoint_for(daemon: Daemon, name: str) -> str:
    if name == "rescan":
        return "/api/v0/shares"
    if name == "searches":
        return "/api/v0/searches"
    if name == "transfer-retry":
        return "/api/v0/transfers/not-a-real-lifecycle-id/retry"
    return "/api/v0/users/__slskr_lifecycle_missing__/browse"


def start_pair(target: Daemon, replacement: Daemon) -> dict[str, Any]:
    observations = {}
    try:
        observations["targetStart"] = target.start()
        observations["replacementStart"] = replacement.start()
        observations["targetSnapshot"] = target.snapshot()
        observations["replacementSnapshot"] = replacement.snapshot()
        return observations
    except Exception:
        replacement.stop()
        target.stop()
        raise


def stop_pair(target: Daemon, replacement: Daemon) -> dict[str, Any]:
    return {"targetStop": target.stop(), "replacementStop": replacement.stop()}


def compare_signatures(target: Any, replacement: Any) -> dict[str, Any]:
    def lifecycle_state(value: Any) -> Any:
        if not isinstance(value, str):
            return value
        lowered = value.lower()
        if "connect" in lowered and "disconnect" not in lowered:
            return "connected"
        if lowered in {"none", "disconnected", "offline", "stopped"} or "disconnect" in lowered:
            return "disconnected"
        return lowered

    def signature(value: Any) -> Any:
        if isinstance(value, dict):
            result = {
                key: signature(value[key])
                for key in (
                    "ready",
                    "connected",
                    "searchCount",
                    "shareCount",
                    "shareReady",
                    "optionsPresent",
                    "statusClass",
                )
                if key in value
            }
            if "applicationState" in value:
                result["applicationState"] = lifecycle_state(value["applicationState"])
            return result
        return value

    left = signature(target)
    right = signature(replacement)
    return {"equal": left == right, "target": left, "replacement": right}


def start_failure_observation(daemon: Daemon) -> dict[str, Any]:
    try:
        result = daemon.start()
        daemon.stop()
        return {"started": True, "result": result}
    except Exception as error:
        daemon.stop()
        return {"started": False, "error": str(error)[-4000:]}


def run_search_operation(daemon: Daemon, suffix: str) -> dict[str, Any]:
    response = daemon.request(
        "POST",
        "/api/v0/searches",
        {"searchText": f"slskr-lifecycle-{suffix}", "searchTimeout": 5, "responseLimit": 10},
    )
    parsed = body_json(response.get("body", ""))
    identifier = None
    if isinstance(parsed, dict):
        identifier = parsed.get("id") or parsed.get("searchId")
    return {"create": response, "id": identifier}


def run_scenario(profile: str, scenario: str, case_directory: Path, target_binary: Path, replacement_binary: Path) -> dict[str, Any]:
    target = Daemon(
        name="target",
        profile=profile,
        binary=target_binary,
        case_directory=case_directory,
        replacement=False,
    )
    replacement = Daemon(
        name="replacement",
        profile=profile,
        binary=replacement_binary,
        case_directory=case_directory,
        replacement=True,
    )
    observations: dict[str, Any] = {"profile": profile, "scenario": scenario}
    parity = False
    try:
        if scenario == "restart":
            start_pair(target, replacement)
            before = {"target": target.snapshot(), "replacement": replacement.snapshot()}
            operations = {"target": run_search_operation(target, "restart"), "replacement": run_search_operation(replacement, "restart")}
            stop_pair(target, replacement)
            start_pair(target, replacement)
            after = {"target": target.snapshot(), "replacement": replacement.snapshot()}
            observations.update({"before": before, "operations": operations, "after": after})
            parity = (
                compare_signatures(after["target"], after["replacement"])["equal"]
                and operations["target"]["create"].get("statusClass")
                == operations["replacement"]["create"].get("statusClass")
                and operations["target"]["create"].get("body")
                == operations["replacement"]["create"].get("body")
            )
        elif scenario == "corrupt-state":
            start_pair(target, replacement)
            stop_pair(target, replacement)
            corrupted = {}
            for daemon in (target, replacement):
                candidates = daemon.state_candidates()
                candidate = candidates[0] if candidates else daemon.state_directory / "lifecycle-state.db"
                candidate.parent.mkdir(parents=True, exist_ok=True)
                candidate.write_bytes(b"not-a-valid-sqlite-or-json-state\x00\x01\n")
                corrupted[daemon.name] = str(candidate.relative_to(daemon.root))
            target_result = start_failure_observation(target)
            replacement_result = start_failure_observation(replacement)
            observations.update({"corrupted": corrupted, "target": target_result, "replacement": replacement_result})
            parity = target_result.get("started") == replacement_result.get("started")
        elif scenario == "cancel":
            start_pair(target, replacement)
            target_operation = run_search_operation(target, "cancel")
            replacement_operation = run_search_operation(replacement, "cancel")
            cancelled = {}
            for daemon, operation in ((target, target_operation), (replacement, replacement_operation)):
                identifier = operation.get("id")
                cancelled[daemon.name] = (
                    daemon.request("DELETE", f"/api/v0/searches/{identifier}") if identifier else {"status": None, "statusClass": "no-id"}
                )
            observations.update({"created": {"target": target_operation, "replacement": replacement_operation}, "cancelled": cancelled})
            parity = status_class(cancelled["target"].get("status")) == status_class(cancelled["replacement"].get("status"))
        elif scenario == "timeout":
            start_pair(target, replacement)
            target_result = target.request("GET", endpoint_for(target, "missing"), timeout=0.25)
            replacement_result = replacement.request("GET", endpoint_for(replacement, "missing"), timeout=0.25)
            observations.update({"target": target_result, "replacement": replacement_result})
            parity = target_result.get("statusClass") == replacement_result.get("statusClass")
        elif scenario == "retry":
            start_pair(target, replacement)
            target_result = target.request("POST", endpoint_for(target, "transfer-retry"))
            replacement_result = replacement.request("POST", endpoint_for(replacement, "transfer-retry"))
            observations.update({"target": target_result, "replacement": replacement_result})
            parity = target_result.get("statusClass") == replacement_result.get("statusClass")
        elif scenario == "resume":
            target_partial = target.download_directory / "resume" / "partial.bin"
            replacement_partial = replacement.download_directory / "resume" / "partial.bin"
            target_partial.parent.mkdir(parents=True, exist_ok=True)
            replacement_partial.parent.mkdir(parents=True, exist_ok=True)
            target_partial.write_bytes(b"partial-bytes")
            replacement_partial.write_bytes(b"partial-bytes")
            start_pair(target, replacement)
            before = {"target": sha256(target_partial), "replacement": sha256(replacement_partial)}
            target_result = target.request("POST", endpoint_for(target, "transfer-retry"))
            replacement_result = replacement.request("POST", endpoint_for(replacement, "transfer-retry"))
            after = {"target": sha256(target_partial), "replacement": sha256(replacement_partial)}
            observations.update({"before": before, "target": target_result, "replacement": replacement_result, "after": after})
            parity = before == after and target_result.get("statusClass") == replacement_result.get("statusClass")
        elif scenario == "concurrent-mutation":
            start_pair(target, replacement)

            def fire(daemon: Daemon) -> dict[str, Any]:
                return daemon.request("PUT", endpoint_for(daemon, "rescan"))

            with ThreadPoolExecutor(max_workers=4) as executor:
                target_results = list(executor.map(lambda _: fire(target), range(4)))
                replacement_results = list(executor.map(lambda _: fire(replacement), range(4)))
            target_classes = sorted(result.get("statusClass") for result in target_results)
            replacement_classes = sorted(result.get("statusClass") for result in replacement_results)
            observations.update({"target": target_results, "replacement": replacement_results})
            parity = target_classes == replacement_classes
        elif scenario == "upgrade":
            start_pair(target, replacement)
            baseline = {"target": target.snapshot(), "replacement": replacement.snapshot()}
            stop_pair(target, replacement)
            target.upgrade_marker = True
            replacement.upgrade_marker = True
            start_pair(target, replacement)
            upgraded = {"target": target.snapshot(), "replacement": replacement.snapshot()}
            observations.update({"baseline": baseline, "upgraded": upgraded})
            parity = compare_signatures(upgraded["target"], upgraded["replacement"])["equal"]
        elif scenario == "rollback":
            target.bad_configuration = True
            replacement.bad_configuration = True
            failed = {"target": start_failure_observation(target), "replacement": start_failure_observation(replacement)}
            target.bad_configuration = False
            replacement.bad_configuration = False
            recovered = {"target": start_failure_observation(target), "replacement": start_failure_observation(replacement)}
            observations.update({"failedConfiguration": failed, "recovered": recovered})
            parity = failed["target"].get("started") == failed["replacement"].get("started") and recovered["target"].get("started") == recovered["replacement"].get("started")
        elif scenario == "permissions":
            start_pair(target, replacement)
            stop_pair(target, replacement)
            permission_results = {}
            for daemon in (target, replacement):
                os.chmod(daemon.root, 0o500)
                permission_results[daemon.name] = start_failure_observation(daemon)
                os.chmod(
                    daemon.root,
                    stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR,
                )
            observations["permissions"] = permission_results
            parity = permission_results["target"].get("started") == permission_results["replacement"].get("started")
        elif scenario == "uninstall":
            start_pair(target, replacement)
            baseline = {"target": target.snapshot(), "replacement": replacement.snapshot()}
            stop_pair(target, replacement)
            for daemon in (target, replacement):
                shutil.rmtree(daemon.app_directory, ignore_errors=True)
                shutil.rmtree(daemon.state_directory, ignore_errors=True)
                daemon.app_directory.mkdir(parents=True, exist_ok=True)
                daemon.state_directory.mkdir(parents=True, exist_ok=True)
            start_pair(target, replacement)
            fresh = {"target": target.snapshot(), "replacement": replacement.snapshot()}
            observations.update({"baseline": baseline, "freshInstall": fresh})
            parity = compare_signatures(fresh["target"], fresh["replacement"])["equal"]
        else:
            raise ValueError(f"unsupported lifecycle scenario: {scenario}")
    finally:
        stop_pair(target, replacement)

    observations["parity"] = compare_signatures(
        observations.get("target", observations.get("after", {}).get("target")),
        observations.get("replacement", observations.get("after", {}).get("replacement")),
    ) if "parityComparison" not in observations else observations["parityComparison"]
    observations["status"] = "pass" if parity else "fail"
    observations["memoryGuard"] = {
        "residentLimit": "4GiB cgroup",
        "swap": "disabled",
        "processes": "serial case; two bounded daemons only",
    }
    return observations


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: run-universal-lifecycle-case.py <target> <scenario> <case-directory>", file=sys.stderr)
        return 2
    target, scenario, case_directory_text = sys.argv[1:]
    if target not in {"slskd", "slskdn"}:
        print(f"unsupported target: {target}", file=sys.stderr)
        return 2
    case_directory = Path(case_directory_text).resolve()
    case_directory.mkdir(parents=True, exist_ok=True)
    replacement_binary_text = os.environ.get("SLSKR_REPLACEMENT_BINARY")
    frozen_binary_text = os.environ.get(
        "SLSKR_FROZEN_SLSKD_BINARY" if target == "slskd" else "SLSKR_FROZEN_SLSKDN_BINARY"
    )
    if not replacement_binary_text or not frozen_binary_text:
        print("replacement and selected frozen binaries are required", file=sys.stderr)
        return 2
    try:
        evidence = run_scenario(
            target,
            scenario,
            case_directory,
            Path(frozen_binary_text),
            Path(replacement_binary_text),
        )
    except Exception as error:
        evidence = {
            "profile": target,
            "scenario": scenario,
            "status": "fail",
            "parity": False,
            "error": str(error)[-12000:],
            "memoryGuard": {"residentLimit": "4GiB cgroup", "swap": "disabled"},
        }
    output = case_directory / "lifecycle-observation.json"
    output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(f"{target}/{scenario}: {evidence['status']} evidence={output}")
    return 0 if evidence["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
