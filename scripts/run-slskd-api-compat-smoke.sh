#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_bin="${PYTHON:-python3}"
api_version="${SLSKD_API_VERSION:-0.2.4}"
api_token="${SLSKR_SLSKD_API_SMOKE_TOKEN:-}"
if [[ -z "$api_token" ]]; then
  echo "missing SLSKR_SLSKD_API_SMOKE_TOKEN" >&2
  exit 2
fi
work_dir="${SLSKR_SLSKD_API_SMOKE_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-slskd-api-smoke.XXXXXX")}"
state_dir="$work_dir/state"
log_file="$work_dir/slskr.log"
mkdir -p "$state_dir/downloads/foo" "$state_dir/incomplete/foo"
printf 'download fixture\n' >"$state_dir/downloads/foo.mp3"
printf 'incomplete fixture\n' >"$state_dir/incomplete/foo.mp3"

pick_free_port() {
  "$python_bin" - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

http_port="${SLSKR_SLSKD_API_SMOKE_PORT:-$(pick_free_port)}"
base_url="http://127.0.0.1:$http_port"
daemon_pid=""

cleanup() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [[ -n "${SLSKD_API_PYTHONPATH:-}" ]]; then
  api_pythonpath="$SLSKD_API_PYTHONPATH"
else
  api_pythonpath="$work_dir/python"
  "$python_bin" -m pip install --quiet --target "$api_pythonpath" "slskd-api==$api_version"
fi

cargo build -q -p slskr

(
  export SLSKR_HTTP_BIND="127.0.0.1:$http_port"
  export SLSKR_STATE_DIR="$state_dir"
  export SLSKR_API_TOKEN="$api_token"
  export SLSKR_AUTH_DISABLED=false
  export SLSKR_AUTO_CONNECT=false
  export SLSKR_RECONNECT=false
  export SLSKR_SHARE_FIXTURE="Virtual/Test.flac=42;Virtual/Album/Track.ogg=64"
  exec target/debug/slskr serve
) >"$log_file" 2>&1 &
daemon_pid="$!"

"$python_bin" - "$base_url" "$api_token" "$api_pythonpath" <<'PY'
import inspect
import sys
import time

base_url, api_token, api_pythonpath = sys.argv[1:4]
sys.path.insert(0, api_pythonpath)

import requests
from slskd_api import SlskdClient

deadline = time.time() + 10
while time.time() < deadline:
    try:
        if requests.get(f"{base_url}/api/health", timeout=1).ok:
            break
    except requests.RequestException:
        pass
    time.sleep(0.1)
else:
    raise SystemExit(f"slskr did not become healthy at {base_url}")

client = SlskdClient(host=base_url, api_key=api_token)
checks = []


def record(name, func, predicate=None):
    value = func()
    if predicate is not None and not predicate(value):
        raise AssertionError(f"{name} returned unexpected value: {value!r}")
    checks.append(name)
    return value


def has_keys(value, *keys):
    return isinstance(value, dict) and all(key in value for key in keys)


def is_search_state(value):
    return (
        has_keys(
            value,
            "id",
            "token",
            "searchText",
            "state",
            "isComplete",
            "fileCount",
            "lockedFileCount",
            "responseCount",
            "responses",
            "startedAt",
        )
        and isinstance(value["responses"], list)
    )


def is_search_file(value):
    return has_keys(
        value,
        "filename",
        "size",
        "code",
        "isLocked",
        "extension",
    )


def is_search_response(value):
    return (
        has_keys(
            value,
            "username",
            "token",
            "hasFreeUploadSlot",
            "queueLength",
            "uploadSpeed",
            "fileCount",
            "files",
            "lockedFileCount",
            "lockedFiles",
        )
        and isinstance(value["files"], list)
        and value["fileCount"] == len(value["files"])
        and all(is_search_file(item) for item in value["files"])
    )


def is_server_state(value):
    return (
        has_keys(
            value,
            "address",
            "ipEndPoint",
            "state",
            "isConnected",
            "isConnecting",
            "isLoggedIn",
            "isLoggingIn",
            "isTransitioning",
        )
        and value["state"] in {"Connected, LoggedIn", "Disconnected", "Connecting", "Disconnecting"}
    )


def is_event(value):
    return has_keys(value, "id", "timestamp", "type", "data")


def is_transfer_group(value, username=None):
    if not has_keys(value, "username", "directories") or not isinstance(value["directories"], list):
        return False
    if username is not None and value["username"] != username:
        return False
    return all(is_transfer_directory(item) for item in value["directories"])


def is_transfer_directory(value):
    return (
        has_keys(value, "directory", "fileCount", "files")
        and isinstance(value["files"], list)
        and value["fileCount"] == len(value["files"])
        and all(is_transfer_file(item) for item in value["files"])
    )


def is_transfer_file(value):
    return has_keys(
        value,
        "id",
        "username",
        "direction",
        "filename",
        "size",
        "startOffset",
        "state",
        "requestedAt",
        "bytesTransferred",
        "averageSpeed",
        "bytesRemaining",
        "percentComplete",
    ) and value["direction"] in {"Download", "Upload"}


def is_room(value):
    return has_keys(value, "name", "isPrivate", "users", "messages") and isinstance(value["users"], list) and isinstance(value["messages"], list)


def is_room_info(value):
    return has_keys(value, "name", "userCount", "isPrivate", "isOwned", "isModerated")


def is_room_message(value):
    return has_keys(value, "timestamp", "username", "message", "roomName")


def is_conversation(value, include_messages=None):
    if not has_keys(value, "username", "isActive", "unAcknowledgedMessageCount", "hasUnAcknowledgedMessages"):
        return False
    if include_messages is True and "messages" not in value:
        return False
    return "messages" not in value or isinstance(value["messages"], list)


def is_message(value):
    return has_keys(value, "timestamp", "id", "username", "direction", "message", "isAcknowledged", "wasReplayed")


def is_user_directory(value):
    return (
        has_keys(value, "name", "fileCount", "files")
        and isinstance(value["files"], list)
        and value["fileCount"] == len(value["files"])
        and all(is_user_file(item) for item in value["files"])
    )


def is_user_file(value):
    return has_keys(value, "filename", "size", "code", "extension", "attributeCount", "attributes")


def is_user_root(value):
    return has_keys(value, "directories", "directoryCount", "lockedDirectories", "lockedDirectoryCount")


def is_user_address(value):
    return has_keys(value, "addressFamily", "address", "port")


def is_user_info(value):
    return has_keys(value, "description", "hasFreeUploadSlot", "hasPicture", "picture", "queueLength", "uploadSlots")


def is_user_status(value):
    return has_keys(value, "presence", "isPrivileged") and value["presence"] in {"Offline", "Away", "Online"}


def is_browsing_status(value):
    return has_keys(value, "bytesTransferred", "bytesRemaining", "percentComplete", "size", "username")


def is_share_info(value):
    return has_keys(value, "id", "alias", "isExcluded", "localPath", "raw", "remotePath", "directories", "files")


def is_shares(value):
    return has_keys(value, "local") and isinstance(value["local"], list) and all(is_share_info(item) for item in value["local"])


def is_directory(value):
    return has_keys(value, "name", "fullName", "attributes", "createdAt", "modifiedAt")


def is_log_entry(value):
    return has_keys(value, "timestamp", "context", "level", "message")


def is_session_status(value):
    return (
        has_keys(value, "expires", "issued", "name", "notBefore", "token", "tokenType")
        and isinstance(value["expires"], int)
        and isinstance(value["issued"], int)
        and isinstance(value["notBefore"], int)
        and isinstance(value["token"], str)
        and value["tokenType"] == "ApiKey"
    )


def is_app_version(value):
    return (
        has_keys(
            value,
            "full",
            "current",
            "latest",
            "isUpdateAvailable",
            "isCanary",
            "isDevelopment",
        )
        and isinstance(value["full"], str)
        and isinstance(value["current"], str)
        and isinstance(value["latest"], str)
        and isinstance(value["isUpdateAvailable"], bool)
        and isinstance(value["isCanary"], bool)
        and isinstance(value["isDevelopment"], bool)
    )


def is_app_state(value):
    return (
        has_keys(
            value,
            "version",
            "pendingReconnect",
            "pendingRestart",
            "server",
            "connectionWatchdog",
            "relay",
            "user",
            "distributedNetwork",
            "shares",
            "rooms",
            "users",
        )
        and is_app_version(value["version"])
        and isinstance(value["pendingReconnect"], bool)
        and isinstance(value["pendingRestart"], bool)
        and is_server_state(value["server"])
        and isinstance(value["connectionWatchdog"], dict)
        and isinstance(value["relay"], dict)
        and isinstance(value["user"], dict)
        and isinstance(value["distributedNetwork"], dict)
        and is_shares(value["shares"])
        and isinstance(value["rooms"], list)
        and isinstance(value["users"], list)
    )


def is_transfer_summary(value):
    return (
        has_keys(
            value,
            "count",
            "downloads",
            "uploads",
            "totalBytes",
            "averageSpeed",
            "byDirection",
            "byState",
        )
        and isinstance(value["byDirection"], dict)
        and isinstance(value["byState"], dict)
    )


def is_transfer_histogram(value):
    return has_keys(value, "interval", "buckets") and isinstance(value["buckets"], list)


def is_transfer_leaderboard_entry(value):
    return has_keys(value, "username", "count", "totalBytes", "averageSpeed")


def is_user_transfer_report(value):
    return (
        has_keys(value, "username", "count", "transfers")
        and isinstance(value["transfers"], list)
        and all(is_transfer_file(item) for item in value["transfers"])
    )


def is_transfer_exception(value):
    return has_keys(value, "username", "direction", "filename", "state", "exception")


def is_transfer_exception_pareto(value):
    return has_keys(value, "exception", "count", "distinctUsers")


def is_directory_report(value):
    return has_keys(value, "path", "directory", "count", "totalBytes", "distinctUsers")


record("application.state", client.application.state, is_app_state)
record("application.version", client.application.version, lambda v: isinstance(v, str) and v)
record("application.check_updates", lambda: client.application.check_updates(forceCheck=True), is_app_version)
record("application.gc", client.application.gc, lambda v: v is True)
record("session.auth_valid", client.session.auth_valid, lambda v: v is True)
record("session.security_enabled", client.session.security_enabled, lambda v: isinstance(v, bool))
record("session.login", lambda: client.session.login("user", "pass"), is_session_status)
record("server.state", client.server.state, is_server_state)
record("server.connect", client.server.connect, lambda v: v is True)
record("server.disconnect", client.server.disconnect, lambda v: v is True)

created = record("searches.search_text", lambda: client.searches.search_text("slskd api smoke"), is_search_state)
identifier = created.get("id") or created.get("token")
record("searches.get_all", client.searches.get_all, lambda v: isinstance(v, list) and len(v) >= 1 and is_search_state(v[0]))
record("searches.state", lambda: client.searches.state(identifier), lambda v: is_search_state(v) and v.get("searchText") == "slskd api smoke")
record("searches.search_responses", lambda: client.searches.search_responses(identifier), lambda v: isinstance(v, list) and all(is_search_response(item) for item in v))
record("searches.stop", lambda: client.searches.stop(identifier), lambda v: v is True)
record("searches.delete", lambda: client.searches.delete(identifier), lambda v: v is True)

record("transfers.enqueue", lambda: client.transfers.enqueue("peer 1", [{"filename": "Remote/Song.mp3", "size": 99}]), lambda v: v is True)
record("transfers.get_all_downloads", client.transfers.get_all_downloads, lambda v: isinstance(v, list) and all(is_transfer_group(item) for item in v))
record("transfers.get_all_uploads", client.transfers.get_all_uploads, lambda v: isinstance(v, list) and all(is_transfer_group(item) for item in v))
record("transfers.get_downloads", lambda: client.transfers.get_downloads("peer 1"), lambda v: isinstance(v, list) and all(is_transfer_group(item, "peer 1") for item in v))
record("transfers.get_uploads", lambda: client.transfers.get_uploads("peer 1"), lambda v: isinstance(v, list) and all(is_transfer_group(item, "peer 1") for item in v))
record("transfers.get_queue_position", lambda: client.transfers.get_queue_position("peer 1", "1"), lambda v: isinstance(v, (int, str)))
record("transfers.get_download", lambda: client.transfers.get_download("peer 1", "1"), is_transfer_file)
record("transfers.cancel_download", lambda: client.transfers.cancel_download("peer 1", "1", remove=True), lambda v: v is True)
record("transfers.remove_completed_downloads", client.transfers.remove_completed_downloads, lambda v: v is True)
record("transfers.get_upload", lambda: client.transfers.get_upload("peer 1", "1"), is_transfer_file)
record("transfers.cancel_upload", lambda: client.transfers.cancel_upload("peer 1", "1", remove=True), lambda v: v is True)
record("transfers.remove_completed_uploads", client.transfers.remove_completed_uploads, lambda v: v is True)

record("rooms.get_all_joined", client.rooms.get_all_joined, lambda v: isinstance(v, list) and all(isinstance(item, str) for item in v))
record("rooms.get_all", client.rooms.get_all, lambda v: isinstance(v, list) and all(is_room_info(item) for item in v))
record("rooms.join", lambda: client.rooms.join("room space"), is_room)
record("rooms.get_joined", lambda: client.rooms.get_joined("room space"), is_room)
record("rooms.send", lambda: client.rooms.send("room space", "hello"), lambda v: v is True)
record("rooms.get_messages", lambda: client.rooms.get_messages("room space"), lambda v: isinstance(v, list) and all(is_room_message(item) for item in v))
record("rooms.set_ticker", lambda: client.rooms.set_ticker("room space", "ticker"), lambda v: v is True)
record("rooms.add_member", lambda: client.rooms.add_member("room space", "peer 1"), lambda v: v is True)
record("rooms.get_users", lambda: client.rooms.get_users("room space"), lambda v: isinstance(v, list))
record("rooms.leave", lambda: client.rooms.leave("room space"), lambda v: v is True)

record("conversations.get_all", client.conversations.get_all, lambda v: isinstance(v, list) and all(is_conversation(item, include_messages=True) for item in v))
record("conversations.send", lambda: client.conversations.send("peer 1", "hello"), lambda v: v is True)
record("conversations.get", lambda: client.conversations.get("peer 1", includeMessages=True), lambda v: is_conversation(v, include_messages=True))
record("conversations.get_messages", lambda: client.conversations.get_messages("peer 1"), lambda v: isinstance(v, list) and all(is_message(item) for item in v))
record("conversations.acknowledge", lambda: client.conversations.acknowledge("peer 1", 1), lambda v: v is True)
record("conversations.acknowledge_all", lambda: client.conversations.acknowledge_all("peer 1"), lambda v: v is True)
record("conversations.delete", lambda: client.conversations.delete("peer 1"), lambda v: v is True)

record("users.address", lambda: client.users.address("peer 1"), is_user_address)
record("users.browse", lambda: client.users.browse("peer 1"), is_user_root)
record("users.browsing_status", lambda: client.users.browsing_status("peer 1"), is_browsing_status)
record("users.directory", lambda: client.users.directory("peer 1", "Virtual"), lambda v: isinstance(v, list) and all(is_user_directory(item) for item in v))
record("users.info", lambda: client.users.info("peer 1"), is_user_info)
record("users.status", lambda: client.users.status("peer 1"), lambda v: is_user_status(v) and v.get("username") == "peer 1")

record("shares.get_all", client.shares.get_all, is_shares)
record("shares.start_scan", client.shares.start_scan, lambda v: v is True)
record("shares.cancel_scan", client.shares.cancel_scan, lambda v: v is True)
record("shares.get", lambda: client.shares.get("Virtual"), is_share_info)
record("shares.all_contents", client.shares.all_contents, lambda v: isinstance(v, list) and all(is_user_directory(item) for item in v))
record("shares.contents", lambda: client.shares.contents("Virtual"), lambda v: isinstance(v, list) and all(is_user_directory(item) for item in v))

record("files.get_downloads_dir", lambda: client.files.get_downloads_dir(recursive=True), is_directory)
record("files.get_downloaded_directory", lambda: client.files.get_downloaded_directory("foo", recursive=True), is_directory)
record("files.delete_downloaded_directory", lambda: client.files.delete_downloaded_directory("foo"), lambda v: v is True)
record("files.delete_downloaded_file", lambda: client.files.delete_downloaded_file("foo.mp3"), lambda v: v is True)
record("files.get_incomplete_dir", lambda: client.files.get_incomplete_dir(recursive=True), is_directory)
record("files.get_incomplete_directory", lambda: client.files.get_incomplete_directory("foo", recursive=True), is_directory)
record("files.delete_incomplete_directory", lambda: client.files.delete_incomplete_directory("foo"), lambda v: v is True)
record("files.delete_incomplete_file", lambda: client.files.delete_incomplete_file("foo.mp3"), lambda v: v is True)

record("relay.connect", client.relay.connect, lambda v: v is True)
record("relay.disconnect", client.relay.disconnect, lambda v: v is True)
record("relay.download_file", lambda: client.relay.download_file("token"), lambda v: v is True)
record("relay.upload_file", lambda: client.relay.upload_file("token"), lambda v: v is True)
record("relay.upload_share_info", lambda: client.relay.upload_share_info("token"), lambda v: v is True)

record("options.get", client.options.get, lambda v: isinstance(v, dict))
record("options.get_startup", client.options.get_startup, lambda v: isinstance(v, dict))
record("options.debug", client.options.debug, lambda v: isinstance(v, dict))
record("options.yaml_location", client.options.yaml_location, lambda v: isinstance(v, str) and v)
record("options.download_yaml", client.options.download_yaml, lambda v: isinstance(v, str))
record("options.upload_yaml", lambda: client.options.upload_yaml("app: {}"), lambda v: v is True)
record("options.validate_yaml", lambda: client.options.validate_yaml("app: {}"), lambda v: v == "")

record("events.get", client.events.get, lambda v: isinstance(v, list) and all(is_event(item) for item in v))
record("events.create", lambda: client.events.create("Smoke", {"ok": True}), lambda v: v is True)
record("logs.get", client.logs.get, lambda v: isinstance(v, list) and all(is_log_entry(item) for item in v))

if client.transfers.enqueue("telemetry peer", [{"filename": "Telemetry/Album/Track.flac", "size": 321}]) is not True:
    raise AssertionError("transfers.enqueue for telemetry fixture failed")

record("telemetry.get_metrics", client.telemetry.get_metrics, lambda v: isinstance(v, str) and "slskr_telemetry_transfers" in v)
record("telemetry.get_kpis", client.telemetry.get_kpis, lambda v: isinstance(v, str) and "kpis" in v)
record("telemetry.get_transfer_summary", client.telemetry.get_transfer_summary, is_transfer_summary)
record("telemetry.get_transfer_histogram", client.telemetry.get_transfer_histogram, is_transfer_histogram)
record("telemetry.get_transfer_leaderboard", lambda: client.telemetry.get_transfer_leaderboard("Download"), lambda v: isinstance(v, list) and len(v) >= 1 and all(is_transfer_leaderboard_entry(item) for item in v))
record("telemetry.get_user_transfers", lambda: client.telemetry.get_user_transfers("telemetry peer"), lambda v: is_user_transfer_report(v) and v["count"] >= 1)
record("telemetry.get_transfer_exceptions", lambda: client.telemetry.get_transfer_exceptions("Download"), lambda v: isinstance(v, list) and all(is_transfer_exception(item) for item in v))
record("telemetry.get_transfer_exceptions_pareto", lambda: client.telemetry.get_transfer_exceptions_pareto("Download"), lambda v: isinstance(v, list) and all(is_transfer_exception_pareto(item) for item in v))
record("telemetry.get_most_dl_directories", client.telemetry.get_most_dl_directories, lambda v: isinstance(v, list) and len(v) >= 1 and all(is_directory_report(item) for item in v))
record("application.restart", client.application.restart, lambda v: v is True)
record("application.stop", client.application.stop, lambda v: v is True)

covered = set(checks)
public_methods = set()
for api_name, api_value in vars(client).items():
    if api_name.startswith("_"):
        continue
    for method_name, method in inspect.getmembers(api_value, predicate=callable):
        if not method_name.startswith("_"):
            public_methods.add(f"{api_name}.{method_name}")

missing = sorted(public_methods - covered)
extra = sorted(covered - public_methods)
if missing or extra:
    raise AssertionError(
        f"slskd_api smoke coverage mismatch; missing={missing!r}, extra={extra!r}"
    )

print(f"slskd_api compatibility smoke passed: {len(checks)} calls")
for check in checks:
    print(check)
PY

echo "slskd_api smoke log: $log_file"
