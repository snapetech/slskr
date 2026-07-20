#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_bin="${PYTHON:-python3}"
upstream_repo="${SLSKR_UPSTREAM_GIT_REPO:-$repo_root/../slskdn}"
slskd_ref="${SLSKR_SLSKD_REF:-16e5d86ec9a91120f3ef40b85cb22036566b788a}"
slskdn_ref="${SLSKR_SLSKDN_REF:-65a14a8b821de4df4ab7ef3ab3b156d7206837a3}"
work_dir="${SLSKR_OPTIONS_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-options-differential.XXXXXX")}"
keep_artifacts="${SLSKR_OPTIONS_DIFFERENTIAL_KEEP:-0}"
scenarios="${SLSKR_OPTIONS_DIFFERENTIAL_SCENARIOS:-all}"
slskd_root="${SLSKR_SLSKD_ROOT:-$work_dir/slskd}"
slskdn_root="${SLSKR_SLSKDN_ROOT:-$work_dir/slskdn}"
created_slskd_worktree=0
created_slskdn_worktree=0
daemon_pid=""
soulseek_fixture_pid=""
listener_blocker_pid=""
lidarr_fixture_pid=""

pick_free_port() {
  "$python_bin" - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("0.0.0.0", 0))
    print(sock.getsockname()[1])
PY
}

pick_free_udp_port() {
  "$python_bin" - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
    sock.bind(("0.0.0.0", 0))
    print(sock.getsockname()[1])
PY
}

pick_free_port_with_free_successor() {
  "$python_bin" - <<'PY'
import socket
for _ in range(256):
    first = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    first.bind(("0.0.0.0", 0))
    port = first.getsockname()[1]
    if port >= 65535:
        first.close()
        continue
    second = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        second.bind(("0.0.0.0", port + 1))
    except OSError:
        first.close()
        second.close()
        continue
    print(port)
    first.close()
    second.close()
    break
else:
    raise SystemExit("unable to allocate adjacent free TCP ports")
PY
}

stop_daemon() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
    # Frozen Soulseek listeners can remain unavailable briefly after host
    # shutdown even though the process has exited.
    sleep 0.5
  fi
  daemon_pid=""
}

stop_soulseek_fixture() {
  if [[ -n "$soulseek_fixture_pid" ]] && kill -0 "$soulseek_fixture_pid" 2>/dev/null; then
    kill "$soulseek_fixture_pid" 2>/dev/null || true
    wait "$soulseek_fixture_pid" 2>/dev/null || true
  fi
  soulseek_fixture_pid=""
}

stop_listener_blocker() {
  if [[ -n "$listener_blocker_pid" ]] && kill -0 "$listener_blocker_pid" 2>/dev/null; then
    kill "$listener_blocker_pid" 2>/dev/null || true
    wait "$listener_blocker_pid" 2>/dev/null || true
  fi
  listener_blocker_pid=""
}

stop_lidarr_fixture() {
  if [[ -n "$lidarr_fixture_pid" ]] && kill -0 "$lidarr_fixture_pid" 2>/dev/null; then
    kill "$lidarr_fixture_pid" 2>/dev/null || true
    wait "$lidarr_fixture_pid" 2>/dev/null || true
  fi
  lidarr_fixture_pid=""
}

cleanup() {
  stop_daemon
  stop_soulseek_fixture
  stop_listener_blocker
  stop_lidarr_fixture
  if [[ "$created_slskd_worktree" == "1" ]]; then
    git -C "$upstream_repo" worktree remove --force "$slskd_root" >/dev/null 2>&1 || true
  fi
  if [[ "$created_slskdn_worktree" == "1" ]]; then
    git -C "$upstream_repo" worktree remove --force "$slskdn_root" >/dev/null 2>&1 || true
  fi
  if [[ "$keep_artifacts" != "1" ]]; then
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

materialize_target() {
  local root="$1"
  local ref="$2"
  local created_variable="$3"
  if [[ -d "$root/.git" || -f "$root/.git" ]]; then
    local actual
    actual="$(git -C "$root" rev-parse HEAD)"
    if [[ "$actual" != "$ref" ]]; then
      printf 'options differential failed: %s is at %s, expected %s\n' "$root" "$actual" "$ref" >&2
      exit 1
    fi
    return
  fi
  mkdir -p "$(dirname "$root")"
  git -C "$upstream_repo" worktree add --detach "$root" "$ref" >/dev/null
  printf -v "$created_variable" '%s' 1
}

scenario_enabled() {
  [[ "$scenarios" == all || ",$scenarios," == *",$1,"* ]]
}

wait_for_options() {
  local base_url="$1"
  local output="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" >"$output"; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'options differential failed: daemon exited before %s became ready\n' "$base_url" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'options differential failed: timed out waiting for %s\n' "$base_url" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_share_files() {
  local base_url="$1"
  local alias="$2"
  local expected_files="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/shares" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); alias=sys.argv[1]; expected=int(sys.argv[2]); raise SystemExit(0 if any(share.get("alias") == alias and share.get("files") == expected for shares in value.values() for share in shares) else 1)' "$alias" "$expected_files" 2>/dev/null; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'options differential failed: daemon exited while waiting for share %s\n' "$alias" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'options differential failed: timed out waiting for share %s\n' "$alias" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_request() {
  local suite_dir="$1"
  local label="$2"
  local method="$3"
  local url="$4"
  local payload="$5"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --request "$method" \
    --header 'Content-Type: application/json' \
    --data-binary "$payload" \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

capture_mutation_suite() {
  local base_url="$1"
  local suite_dir="$2"
  capture_request "$suite_dir" patch-null PATCH "$base_url/api/v0/options" 'null'
  capture_request "$suite_dir" patch-array PATCH "$base_url/api/v0/options" '[]'
  capture_request "$suite_dir" patch-valid PATCH "$base_url/api/v0/options" \
    '{"soulseek":{"listenPort":50300}}'
  capture_request "$suite_dir" yaml-valid POST "$base_url/api/v0/options/yaml/validate" \
    '"debug: false\n"'
  capture_request "$suite_dir" yaml-invalid POST "$base_url/api/v0/options/yaml/validate" \
    '"web: [unterminated"'
}

capture_get() {
  local suite_dir="$1"
  local label="$2"
  local url="$3"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

capture_delete() {
  local suite_dir="$1"
  local label="$2"
  local url="$3"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --request DELETE \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

capture_put() {
  local suite_dir="$1"
  local label="$2"
  local url="$3"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --request PUT \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

compare_mutation_suites() {
  local target="$1"
  local expected="$2"
  local actual="$3"
  if ! diff -ru "$expected" "$actual"; then
    printf 'options mutation differential failed for %s\n' "$target" >&2
    exit 1
  fi
}

normalize_directory_suite() {
  local source="$1"
  local destination="$2"
  "$python_bin" - "$source" "$destination" <<'PY'
import json
import pathlib
import re
import shutil
import sys

source = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
destination.mkdir(parents=True, exist_ok=True)
timestamp = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,7})?Z$")

def normalize(value):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"createdAt", "modifiedAt"}:
                if not isinstance(child, str) or not timestamp.fullmatch(child):
                    raise SystemExit(f"invalid filesystem timestamp {child!r} in {source}")
                value[key] = "<TIMESTAMP>"
            else:
                normalize(child)
    elif isinstance(value, list):
        for child in value:
            normalize(child)

for path in source.iterdir():
    output = destination / path.name
    if path.suffix == ".meta":
        shutil.copyfile(path, output)
        continue
    body = path.read_text(encoding="utf-8")
    if not body:
        output.write_text("", encoding="utf-8")
        continue
    try:
        value = json.loads(body)
    except json.JSONDecodeError:
        output.write_text(body, encoding="utf-8")
        continue
    if path.name.startswith("restart-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("management-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("management-options-"):
        value = {"remoteFileManagement": value["remoteFileManagement"]}
    elif path.name.startswith("configuration-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("configuration-options-"):
        value = {
            "remoteConfiguration": value["remoteConfiguration"],
            "debug": value["debug"],
        }
    elif path.name.startswith("configuration-location-"):
        if not isinstance(value, str) or not value.endswith("/slskd.yml"):
            raise SystemExit(f"invalid configuration location {value!r} in {source}")
        value = "<CONFIG_PATH>/slskd.yml"
    elif path.name.startswith("configuration-debug-"):
        if not isinstance(value, str) or not value:
            raise SystemExit(f"invalid configuration debug view in {source}")
        value = re.sub(
            r"/tmp/slskr-options-differential\.[^/]+/state-(?:slskd|slskdn)-configuration-(?:upstream|slskr)",
            "<STATE_DIR>",
            value,
        )
        value = re.sub(
            r"(?m)^(\s*key=)[A-Za-z0-9_+/-]{32,64}( \(DefaultValueConfigurationProvider\))$",
            r"\1<JWT_KEY>\2",
            value,
        )
    elif path.name.startswith("debug-options-") or path.name.startswith("debug-startup-"):
        value = {"debug": value["debug"]}
    elif path.name.startswith("debug-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("debug-view-") and isinstance(value, str):
        value = re.sub(
            r"/tmp/slskr-options-differential\.[^/]+/state-(?:slskd|slskdn)-debug-(?:upstream|slskr)",
            "<STATE_DIR>",
            value,
        )
        value = re.sub(
            r"(?m)^(\s*key=)[A-Za-z0-9_+/-]{32,64}( \(DefaultValueConfigurationProvider\))$",
            r"\1<JWT_KEY>\2",
            value,
        )
    elif path.name.startswith("blacklist-options-") or path.name.startswith("blacklist-startup-"):
        blacklist = value["blacklist"]
        configured_file = blacklist.get("file", "")
        value = {
            "enabled": blacklist["enabled"],
            "file": pathlib.Path(configured_file).name if configured_file else "",
        }
    elif path.name.startswith("blacklist-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("blacklist-validation-") and isinstance(value, str):
        value = re.sub(
            r"/tmp/slskr-options-differential\.[^/]+/state-(?:slskd|slskdn)-blacklist-(?:upstream|slskr)",
            "<STATE_DIR>",
            value,
        )
    elif path.name.startswith("groups-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("groups-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("template-options-") or path.name.startswith("template-startup-"):
        value = {
            "completedPathTemplate": value["global"]["download"]["completedPathTemplate"],
        }
    elif path.name.startswith("template-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("template-debug-") and isinstance(value, str):
        lines = [
            line.strip()
            for line in value.splitlines()
            if line.strip().startswith("completedpathtemplate=")
        ]
        if len(lines) != 1:
            raise SystemExit(f"invalid completed-template debug view in {source}: {lines!r}")
        value = lines[0]
    elif path.name.startswith("completed-template-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("dht-options-") or path.name.startswith("dht-startup-"):
        value = {
            "enabled": value["dhtRendezvous"]["enabled"],
            "dhtPort": value["dhtRendezvous"]["dhtPort"],
        }
    elif path.name.startswith("dht-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("dht-status-"):
        for key in ("isEnabled", "isDhtRunning"):
            if not isinstance(value.get(key), bool):
                raise SystemExit(f"invalid DHT status {key} in {source}: {value!r}")
        # Public-DHT bootstrap readiness races independently across the two
        # processes. Controlled local testnet coverage proves that semantic;
        # this differential compares the deterministic configured lifecycle.
        value = {"isEnabled": value["isEnabled"]}
    elif path.name.startswith("dht-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("auto-options-") or path.name.startswith("auto-startup-"):
        value = value["soulseek"]["privateMessageAutoResponse"]
    elif path.name.startswith("auto-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("auto-fixture-"):
        value = {
            "private_message_acks": value["private_message_acks"],
            "private_message_responses": value["private_message_responses"],
            "injected_private_message_ids": value["injected_private_message_ids"],
        }
    elif path.name.startswith("auto-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("endpoint-options-") or path.name.startswith("endpoint-startup-"):
        value = {
            "address": value["soulseek"]["address"],
            "port": value["soulseek"]["port"],
        }
    elif path.name.startswith("endpoint-application-"):
        server = {
            "isConnected": value["server"]["isConnected"],
        }
        for key in ("address", "ipEndPoint"):
            if key in value["server"]:
                server[key] = value["server"][key]
        value = {
            "pendingReconnect": value["pendingReconnect"],
            "server": server,
        }
    elif path.name.startswith("endpoint-server-"):
        selected = {
            "isConnected": value["isConnected"],
        }
        for key in ("address", "ipEndPoint"):
            if key in value:
                selected[key] = value[key]
        value = selected
    elif path.name.startswith("endpoint-network-"):
        value = {
            "accepted": value["accepted"],
            "active": value["active"],
        }
    elif path.name.startswith("endpoint-debug-") and isinstance(value, str):
        soulseek_lines = []
        in_soulseek = False
        for line in value.splitlines():
            if line == "  soulseek:":
                in_soulseek = True
                continue
            if in_soulseek and line.startswith("  ") and not line.startswith("    "):
                break
            if in_soulseek and (line.startswith("    address=") or line.startswith("    port=")):
                soulseek_lines.append(line.strip())
        if len(soulseek_lines) != 2:
            raise SystemExit(f"invalid Soulseek endpoint debug view in {source}: {soulseek_lines!r}")
        value = soulseek_lines
    elif path.name.startswith("endpoint-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
        value = re.sub(
            r"(?m)^(\s*key=)[A-Za-z0-9_+/-]{32,64}( \(DefaultValueConfigurationProvider\))$",
            r"\1<JWT_KEY>\2",
            value,
        )
    elif path.name.startswith("credential-options-") or path.name.startswith("credential-startup-"):
        soulseek = value["soulseek"]
        value = {
            key: soulseek[key]
            for key in ("username", "password")
            if key in soulseek
        }
    elif path.name.startswith("credential-application-"):
        value = {"pendingReconnect": value["pendingReconnect"]}
    elif path.name.startswith("credential-network-"):
        value = {
            "accepted": value["accepted"],
            "active": value["active"],
            "loginUsernames": value.get("login_usernames", []),
            "loginPasswordSha256": value.get("login_password_sha256", []),
        }
    elif path.name.startswith("credential-debug-") and isinstance(value, str):
        soulseek_lines = []
        in_soulseek = False
        for line in value.splitlines():
            if line == "  soulseek:":
                in_soulseek = True
                continue
            if in_soulseek and line.startswith("  ") and not line.startswith("    "):
                break
            if in_soulseek and (line.startswith("    username=") or line.startswith("    password=")):
                soulseek_lines.append(line.strip())
        if len(soulseek_lines) not in (0, 2):
            raise SystemExit(f"missing credential debug provider lines in {source}")
        value = "\n".join(soulseek_lines)
    elif path.name.startswith("obfuscation-options-") or path.name.startswith("obfuscation-startup-"):
        value = value["soulseek"]["obfuscation"]
    elif path.name.startswith("obfuscation-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("obfuscation-application-"):
        value = {"pendingReconnect": value["pendingReconnect"]}
    elif path.name.startswith("obfuscation-network-"):
        messages = value.get("set_wait_port_messages", [])
        value = {
            "advertisementCount": len(messages),
            "lastAdvertisement": messages[-1] if messages else None,
        }
    elif path.name.startswith("configuration-security-"):
        if isinstance(value, dict) and "traceId" in value:
            trace_id = value["traceId"]
            if not isinstance(trace_id, str) or not re.fullmatch(r"0H[A-Z0-9]{11}:[0-9]{8}", trace_id):
                raise SystemExit(f"invalid ASP.NET traceId {trace_id!r} in {source}")
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("no-connect-options-"):
        value = {"noConnect": value["flags"]["noConnect"]}
    elif path.name.startswith("no-connect-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("no-connect-network-"):
        value = {
            "connectionObserved": value.get("accepted", 0) > 0,
            "connectionActive": value.get("active", 0) > 0,
        }
    elif path.name == "no-connect-invalid-watch.body":
        if isinstance(value, dict) and "traceId" in value:
            trace_id = value["traceId"]
            if not isinstance(trace_id, str) or not re.fullmatch(r"0H[A-Z0-9]{11}:[0-9]{8}", trace_id):
                raise SystemExit(f"invalid ASP.NET traceId {trace_id!r} in {source}")
            value["traceId"] = "<TRACE_ID>"
        else:
            value = {"noConnect": value["flags"]["noConnect"]}
    elif path.name == "listener-invalid-watch.body":
        if isinstance(value, dict) and "traceId" in value:
            trace_id = value["traceId"]
            if not isinstance(trace_id, str) or not re.fullmatch(r"0H[A-Z0-9]{11}:[0-9]{8}", trace_id):
                raise SystemExit(f"invalid ASP.NET traceId {trace_id!r} in {source}")
            value["traceId"] = "<TRACE_ID>"
        elif value in (
            "A validation error has occurred.",
            "Collection was modified; enumeration operation may not execute.",
        ):
            value = "<FROZEN_LISTENER_VALIDATION_FAILURE>"
    elif path.name.startswith("config-watch-options-"):
        value = {"noConfigWatch": value["flags"]["noConfigWatch"]}
    elif path.name.startswith("config-watch-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("description-options-"):
        value = {"description": value["soulseek"]["description"]}
    elif path.name.startswith("description-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("application-"):
        value = value["shares"]
    elif path.name.startswith("storage-options-"):
        value = value["directories"]
    elif path.name.startswith("options-"):
        value = value["shares"]["directories"]
    normalize(value)
    output.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")), encoding="utf-8")
PY
}

normalize_options() {
  local source="$1"
  local destination="$2"
  local normalize_directories="${3:-yes}"
  "$python_bin" - "$source" "$destination" "$normalize_directories" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    value = json.load(handle)
if sys.argv[3] == "yes":
    value["directories"]["downloads"] = "<APP_DIR>/downloads"
    value["directories"]["incomplete"] = "<APP_DIR>/incomplete"
value["web"]["port"] = "<HTTP_PORT>"
if isinstance(value["web"].get("https"), dict):
    value["web"]["https"]["port"] = "<HTTPS_PORT>"
with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(value, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

run_directory_scenario() {
  local target="$1"
  local root="$2"
  local port
  local https_port
  local listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local downloads="$work_dir/$target-custom-downloads"
  local incomplete="$work_dir/$target-custom-incomplete"
  local shared="$work_dir/$target-custom-shared"
  local upstream_state="$work_dir/state-$target-directories-upstream"
  local slskr_state="$work_dir/state-$target-directories-slskr"
  local upstream_json="$work_dir/$target-directories-upstream.json"
  local slskr_json="$work_dir/$target-directories-slskr.json"
  local upstream_restart_json="$work_dir/$target-directories-upstream-restart.json"
  local slskr_restart_json="$work_dir/$target-directories-slskr-restart.json"
  local upstream_normalized="$work_dir/$target-directories-upstream.normalized.json"
  local slskr_normalized="$work_dir/$target-directories-slskr.normalized.json"
  local upstream_suite="$work_dir/$target-directories-upstream-files"
  local slskr_suite="$work_dir/$target-directories-slskr-files"
  local upstream_delete_suite="$work_dir/$target-directories-upstream-deletes"
  local slskr_delete_suite="$work_dir/$target-directories-slskr-deletes"
  local upstream_normalized_suite="$work_dir/$target-directories-upstream-files.normalized"
  local slskr_normalized_suite="$work_dir/$target-directories-slskr-files.normalized"
  local upstream_log="$work_dir/$target-directories-upstream.log"
  local slskr_log="$work_dir/$target-directories-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$downloads" "$incomplete" "$shared" "$upstream_state" "$slskr_state"
  mkdir -p "$downloads/nested/deeper" "$incomplete/nested" "$shared/included" "$shared/excluded"
  printf 'completed fixture\n' >"$downloads/song.flac"
  printf 'nested fixture\n' >"$downloads/nested/nested.flac"
  printf 'deep fixture\n' >"$downloads/nested/deeper/deep.flac"
  printf 'incomplete fixture\n' >"$incomplete/incomplete.part"
  printf 'nested incomplete fixture\n' >"$incomplete/nested/nested.part"
  printf 'shared fixture\n' >"$shared/shared.flac"
  printf 'included fixture\n' >"$shared/included/public.flac"
  printf 'excluded fixture\n' >"$shared/excluded/secret.flac"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_IP_ADDRESS=127.0.0.1
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_REMOTE_FILE_MANAGEMENT=true
    export SLSKD_INSTANCE_NAME="$target-directory-proof"
    export SLSKD_DOWNLOADS_DIR="$downloads"
    export SLSKD_INCOMPLETE_DIR="$incomplete"
    export SLSKD_SHARED_DIR="[Library]$shared;!$shared/excluded"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$upstream_json" "$upstream_log"
  wait_for_share_files "$base_url" Library 2 "$upstream_log"
  capture_get "$upstream_suite" downloads "$base_url/api/v0/files/downloads/directories?limit=100&offset=0"
  capture_get "$upstream_suite" downloads-recursive "$base_url/api/v0/files/downloads/directories?recursive=true"
  capture_get "$upstream_suite" downloads-subdirectory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  capture_get "$upstream_suite" incomplete "$base_url/api/v0/files/incomplete/directories?limit=100&offset=0"
  capture_get "$upstream_suite" incomplete-recursive "$base_url/api/v0/files/incomplete/directories?recursive=true"
  capture_get "$upstream_suite" incomplete-subdirectory "$base_url/api/v0/files/incomplete/directories/bmVzdGVk"
  capture_get "$upstream_suite" shares "$base_url/api/v0/shares"
  capture_get "$upstream_suite" shares-contents "$base_url/api/v0/shares/contents"
  capture_get "$upstream_suite" share-library "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$upstream_suite" share-library-contents "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$upstream_suite" share-excluded "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  capture_get "$upstream_suite" share-excluded-contents "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B/contents"
  capture_get "$upstream_suite" share-alias-is-not-id "$base_url/api/v0/shares/Library"
  capture_get "$upstream_suite" share-lowercase-id "$base_url/api/v0/shares/b8100f5ba8bd048a7cf11d116fbbd73130c3c6f5"
  capture_delete "$upstream_delete_suite" existing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$upstream_delete_suite" missing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$upstream_delete_suite" existing-directory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  stop_daemon

  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_REMOTE_FILE_MANAGEMENT=true
    export SLSKD_INSTANCE_NAME="$target-directory-proof"
    export SLSKD_DOWNLOADS_DIR="$downloads"
    export SLSKD_INCOMPLETE_DIR="$incomplete"
    export SLSKD_SHARED_DIR="[Library]$shared;!$shared/excluded"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$upstream_restart_json" "$upstream_log"
  wait_for_share_files "$base_url" Library 2 "$upstream_log"
  capture_get "$upstream_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$upstream_suite" share-library-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$upstream_suite" share-library-contents-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$upstream_suite" share-excluded-restarted "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  stop_daemon

  mkdir -p "$downloads/nested/deeper"
  printf 'completed fixture\n' >"$downloads/song.flac"
  printf 'nested fixture\n' >"$downloads/nested/nested.flac"
  printf 'deep fixture\n' >"$downloads/nested/deeper/deep.flac"

  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-ip-address 127.0.0.1 \
      --slsk-listen-port "$listen_port" \
      --instance-name "$target-directory-proof" \
      --downloads "$downloads" \
      --incomplete "$incomplete" \
      --shared "[Library]$shared;!$shared/excluded" \
      --no-connect \
      --remote-file-management \
      --remote-configuration
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$slskr_json" "$slskr_log"
  wait_for_share_files "$base_url" Library 2 "$slskr_log"
  capture_get "$slskr_suite" downloads "$base_url/api/v0/files/downloads/directories?limit=100&offset=0"
  capture_get "$slskr_suite" downloads-recursive "$base_url/api/v0/files/downloads/directories?recursive=true"
  capture_get "$slskr_suite" downloads-subdirectory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  capture_get "$slskr_suite" incomplete "$base_url/api/v0/files/incomplete/directories?limit=100&offset=0"
  capture_get "$slskr_suite" incomplete-recursive "$base_url/api/v0/files/incomplete/directories?recursive=true"
  capture_get "$slskr_suite" incomplete-subdirectory "$base_url/api/v0/files/incomplete/directories/bmVzdGVk"
  capture_get "$slskr_suite" shares "$base_url/api/v0/shares"
  capture_get "$slskr_suite" shares-contents "$base_url/api/v0/shares/contents"
  capture_get "$slskr_suite" share-library "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$slskr_suite" share-library-contents "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$slskr_suite" share-excluded "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  capture_get "$slskr_suite" share-excluded-contents "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B/contents"
  capture_get "$slskr_suite" share-alias-is-not-id "$base_url/api/v0/shares/Library"
  capture_get "$slskr_suite" share-lowercase-id "$base_url/api/v0/shares/b8100f5ba8bd048a7cf11d116fbbd73130c3c6f5"
  capture_delete "$slskr_delete_suite" existing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$slskr_delete_suite" missing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$slskr_delete_suite" existing-directory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  stop_daemon

  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --instance-name "$target-directory-proof" \
      --downloads "$downloads" \
      --incomplete "$incomplete" \
      --shared "[Library]$shared;!$shared/excluded" \
      --no-connect \
      --remote-file-management \
      --remote-configuration
  ) >>"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$slskr_restart_json" "$slskr_log"
  wait_for_share_files "$base_url" Library 2 "$slskr_log"
  capture_get "$slskr_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$slskr_suite" share-library-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$slskr_suite" share-library-contents-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$slskr_suite" share-excluded-restarted "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  stop_daemon

  normalize_options "$upstream_json" "$upstream_normalized" no
  normalize_options "$slskr_json" "$slskr_normalized" no
  if ! cmp --silent "$upstream_normalized" "$slskr_normalized"; then
    printf 'custom directory options differential failed for %s\n' "$target" >&2
    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
    exit 1
  fi
  normalize_directory_suite "$upstream_suite" "$upstream_normalized_suite"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized_suite"
  if ! diff -ru "$upstream_normalized_suite" "$slskr_normalized_suite"; then
    printf 'custom directory file-management differential failed for %s\n' "$target" >&2
    exit 1
  fi
  if ! diff -ru "$upstream_delete_suite" "$slskr_delete_suite"; then
    printf 'custom directory delete differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s custom directory differential passed\n' "$target"
}

write_share_watch_yaml() {
  local path="$1"
  local alias="$2"
  local directory="$3"
  local temporary="$path.tmp"
  printf 'shares:\n  directories:\n    - "[%s]%s"\n' "$alias" "$directory" >"$temporary"
  mv "$temporary" "$path"
}

write_share_watch_yaml_in_place() {
  local path="$1"
  local alias="$2"
  local directory="$3"
  printf 'shares:\n  directories:\n    - "[%s]%s"\n' "$alias" "$directory" >"$path"
}

wait_for_share_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; values=json.load(sys.stdin)["shares"]["directories"]; print(values[0] if values else "")' 2>/dev/null || true)"
    if [[ "$current" == "$expected" ]]; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'share watch differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'share watch differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

run_share_watch_scenario() {
  local target="$1"
  local root="$2"
  local port
  local https_port
  local listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local old_share="$work_dir/$target-watch-old"
  local new_share="$work_dir/$target-watch-new"
  local upstream_state="$work_dir/state-$target-watch-upstream"
  local slskr_state="$work_dir/state-$target-watch-slskr"
  local upstream_suite="$work_dir/$target-watch-upstream"
  local slskr_suite="$work_dir/$target-watch-slskr"
  local upstream_normalized="$work_dir/$target-watch-upstream.normalized"
  local slskr_normalized="$work_dir/$target-watch-slskr.normalized"
  local upstream_log="$work_dir/$target-watch-upstream.log"
  local slskr_log="$work_dir/$target-watch-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
  local old_raw="[Old]$old_share"
  local new_raw="[New]$new_share"

  mkdir -p "$old_share" "$new_share" "$upstream_state" "$slskr_state"
  printf 'old watched fixture\n' >"$old_share/old.flac"
  printf 'new watched fixture\n' >"$new_share/new.flac"
  write_share_watch_yaml "$upstream_state/slskd.yml" Old "$old_share"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-watch-upstream-options.json" "$upstream_log"
  wait_for_share_files "$base_url" Old 1 "$upstream_log"
  capture_get "$upstream_suite" options-before "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-before "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-before "$base_url/api/v0/application"
  printf 'shares: [unterminated' >"$upstream_state/slskd.yml.tmp"
  mv "$upstream_state/slskd.yml.tmp" "$upstream_state/slskd.yml"
  wait_for_share_option "$base_url" "" "$upstream_log"
  capture_get "$upstream_suite" options-invalid "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-invalid "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-invalid "$base_url/api/v0/application"
  write_share_watch_yaml "$upstream_state/slskd.yml" New "$new_share"
  wait_for_share_option "$base_url" "$new_raw" "$upstream_log"
  capture_get "$upstream_suite" options-watched "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-watched "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-watched "$base_url/api/v0/application"
  capture_put "$upstream_suite" shares-rescan-response "$base_url/api/v0/shares"
  wait_for_share_files "$base_url" New 1 "$upstream_log"
  capture_get "$upstream_suite" shares-rescanned "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-rescanned "$base_url/api/v0/application"
  stop_daemon

  write_share_watch_yaml "$slskr_state/slskd.yml" Old "$old_share"
  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-watch-slskr-options.json" "$slskr_log"
  wait_for_share_files "$base_url" Old 1 "$slskr_log"
  capture_get "$slskr_suite" options-before "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-before "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-before "$base_url/api/v0/application"
  printf 'shares: [unterminated' >"$slskr_state/slskd.yml.tmp"
  mv "$slskr_state/slskd.yml.tmp" "$slskr_state/slskd.yml"
  wait_for_share_option "$base_url" "" "$slskr_log"
  capture_get "$slskr_suite" options-invalid "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-invalid "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-invalid "$base_url/api/v0/application"
  write_share_watch_yaml "$slskr_state/slskd.yml" New "$new_share"
  wait_for_share_option "$base_url" "$new_raw" "$slskr_log"
  capture_get "$slskr_suite" options-watched "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-watched "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-watched "$base_url/api/v0/application"
  capture_put "$slskr_suite" shares-rescan-response "$base_url/api/v0/shares"
  wait_for_share_files "$base_url" New 1 "$slskr_log"
  capture_get "$slskr_suite" shares-rescanned "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-rescanned "$base_url/api/v0/application"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'share watch differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s share watch differential passed\n' "$target"
}

run_no_watch_upload_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local old_share="$work_dir/$target-no-watch-old"
  local new_share="$work_dir/$target-no-watch-new"
  local upstream_state="$work_dir/state-$target-no-watch-upstream"
  local slskr_state="$work_dir/state-$target-no-watch-slskr"
  local upstream_suite="$work_dir/$target-no-watch-upstream"
  local slskr_suite="$work_dir/$target-no-watch-slskr"
  local upstream_normalized="$work_dir/$target-no-watch-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-watch-slskr.normalized"
  local upstream_log="$work_dir/$target-no-watch-upstream.log"
  local slskr_log="$work_dir/$target-no-watch-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
  local new_raw="[New]$new_share"
  local payload

  mkdir -p "$old_share" "$new_share" "$upstream_state" "$slskr_state"
  printf 'old no-watch fixture\n' >"$old_share/old.flac"
  printf 'new no-watch fixture\n' >"$new_share/new.flac"
  payload="$($python_bin -c 'import json,sys; print(json.dumps("shares:\n  directories:\n    - \"[New]" + sys.argv[1] + "\"\n"))' "$new_share")"

  write_share_watch_yaml "$upstream_state/slskd.yml" Old "$old_share"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONFIG_WATCH=true
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-upstream-options.json" "$upstream_log"
  wait_for_share_files "$base_url" Old 1 "$upstream_log"
  capture_request "$upstream_suite" yaml-put PUT "$base_url/api/v0/options/yaml" "$payload"
  wait_for_share_option "$base_url" "$new_raw" "$upstream_log"
  capture_get "$upstream_suite" options-uploaded "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-uploaded "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-uploaded "$base_url/api/v0/application"
  capture_get "$upstream_suite" yaml-uploaded "$base_url/api/v0/options/yaml"
  write_share_watch_yaml_in_place "$upstream_state/slskd.yml" Direct "$new_share"
  wait_for_share_option "$base_url" "[Direct]$new_share" "$upstream_log"
  capture_get "$upstream_suite" options-direct-write "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-direct-write "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-direct-write "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONFIG_WATCH=true
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-upstream-restart-options.json" "$upstream_log"
  wait_for_share_files "$base_url" Direct 1 "$upstream_log"
  capture_get "$upstream_suite" options-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-restarted "$base_url/api/v0/application"
  stop_daemon

  write_share_watch_yaml "$slskr_state/slskd.yml" Old "$old_share"
  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect \
      --no-config-watch \
      --remote-configuration
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-slskr-options.json" "$slskr_log"
  wait_for_share_files "$base_url" Old 1 "$slskr_log"
  capture_request "$slskr_suite" yaml-put PUT "$base_url/api/v0/options/yaml" "$payload"
  wait_for_share_option "$base_url" "$new_raw" "$slskr_log"
  capture_get "$slskr_suite" options-uploaded "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-uploaded "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-uploaded "$base_url/api/v0/application"
  capture_get "$slskr_suite" yaml-uploaded "$base_url/api/v0/options/yaml"
  write_share_watch_yaml_in_place "$slskr_state/slskd.yml" Direct "$new_share"
  wait_for_share_option "$base_url" "[Direct]$new_share" "$slskr_log"
  capture_get "$slskr_suite" options-direct-write "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-direct-write "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-direct-write "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect \
      --no-config-watch \
      --remote-configuration
  ) >>"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-slskr-restart-options.json" "$slskr_log"
  wait_for_share_files "$base_url" Direct 1 "$slskr_log"
  capture_get "$slskr_suite" options-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-restarted "$base_url/api/v0/application"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-watch YAML upload differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-watch YAML upload differential passed\n' "$target"
}

write_storage_watch_yaml() {
  local path="$1"
  local downloads="$2"
  local incomplete="$3"
  local temporary="$path.tmp"
  printf 'directories:\n  downloads: "%s"\n  incomplete: "%s"\n' "$downloads" "$incomplete" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_download_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["directories"]["downloads"])' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'storage watch differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'storage watch differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

run_storage_restart_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local old_downloads="$work_dir/$target-restart-old-downloads"
  local old_incomplete="$work_dir/$target-restart-old-incomplete"
  local new_downloads="$work_dir/$target-restart-new-downloads"
  local new_incomplete="$work_dir/$target-restart-new-incomplete"
  local upstream_state="$work_dir/state-$target-restart-upstream"
  local slskr_state="$work_dir/state-$target-restart-slskr"
  local upstream_suite="$work_dir/$target-restart-upstream"
  local slskr_suite="$work_dir/$target-restart-slskr"
  local upstream_normalized="$work_dir/$target-restart-upstream.normalized"
  local slskr_normalized="$work_dir/$target-restart-slskr.normalized"
  local upstream_log="$work_dir/$target-restart-upstream.log"
  local slskr_log="$work_dir/$target-restart-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$old_downloads" "$old_incomplete" "$new_downloads" "$new_incomplete" "$upstream_state" "$slskr_state"
  printf 'old download\n' >"$old_downloads/old.flac"
  printf 'old incomplete\n' >"$old_incomplete/old.part"
  printf 'new download\n' >"$new_downloads/new.flac"
  printf 'new incomplete\n' >"$new_incomplete/new.part"

  write_storage_watch_yaml "$upstream_state/slskd.yml" "$old_downloads" "$old_incomplete"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-upstream-options.json" "$upstream_log"
  capture_get "$upstream_suite" storage-options-before "$base_url/api/v0/options"
  capture_get "$upstream_suite" downloads-before "$base_url/api/v0/files/downloads/directories"
  capture_get "$upstream_suite" incomplete-before "$base_url/api/v0/files/incomplete/directories"
  capture_get "$upstream_suite" restart-application-before "$base_url/api/v0/application"
  write_storage_watch_yaml "$upstream_state/slskd.yml" "$new_downloads" "$new_incomplete"
  wait_for_download_option "$base_url" "$new_downloads" "$upstream_log"
  capture_get "$upstream_suite" storage-options-watched "$base_url/api/v0/options"
  capture_get "$upstream_suite" downloads-watched "$base_url/api/v0/files/downloads/directories"
  capture_get "$upstream_suite" incomplete-watched "$base_url/api/v0/files/incomplete/directories"
  capture_get "$upstream_suite" restart-application-watched "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-upstream-reloaded-options.json" "$upstream_log"
  capture_get "$upstream_suite" storage-options-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" downloads-restarted "$base_url/api/v0/files/downloads/directories"
  capture_get "$upstream_suite" incomplete-restarted "$base_url/api/v0/files/incomplete/directories"
  capture_get "$upstream_suite" restart-application-restarted "$base_url/api/v0/application"
  stop_daemon

  write_storage_watch_yaml "$slskr_state/slskd.yml" "$old_downloads" "$old_incomplete"
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-slskr-options.json" "$slskr_log"
  capture_get "$slskr_suite" storage-options-before "$base_url/api/v0/options"
  capture_get "$slskr_suite" downloads-before "$base_url/api/v0/files/downloads/directories"
  capture_get "$slskr_suite" incomplete-before "$base_url/api/v0/files/incomplete/directories"
  capture_get "$slskr_suite" restart-application-before "$base_url/api/v0/application"
  write_storage_watch_yaml "$slskr_state/slskd.yml" "$new_downloads" "$new_incomplete"
  wait_for_download_option "$base_url" "$new_downloads" "$slskr_log"
  capture_get "$slskr_suite" storage-options-watched "$base_url/api/v0/options"
  capture_get "$slskr_suite" downloads-watched "$base_url/api/v0/files/downloads/directories"
  capture_get "$slskr_suite" incomplete-watched "$base_url/api/v0/files/incomplete/directories"
  capture_get "$slskr_suite" restart-application-watched "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >>"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-slskr-reloaded-options.json" "$slskr_log"
  capture_get "$slskr_suite" storage-options-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" downloads-restarted "$base_url/api/v0/files/downloads/directories"
  capture_get "$slskr_suite" incomplete-restarted "$base_url/api/v0/files/incomplete/directories"
  capture_get "$slskr_suite" restart-application-restarted "$base_url/api/v0/application"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'storage restart differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s storage restart differential passed\n' "$target"
}

write_management_watch_yaml() {
  local path="$1"
  local enabled="$2"
  local downloads="$3"
  local incomplete="$4"
  local temporary="$path.tmp"
  printf 'remote_file_management: %s\ndirectories:\n  downloads: "%s"\n  incomplete: "%s"\n' \
    "$enabled" "$downloads" "$incomplete" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_management_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteFileManagement"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'remote file-management differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'remote file-management differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

run_remote_file_management_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local downloads="$work_dir/$target-management-downloads"
  local incomplete="$work_dir/$target-management-incomplete"
  local upstream_state="$work_dir/state-$target-management-upstream"
  local slskr_state="$work_dir/state-$target-management-slskr"
  local upstream_suite="$work_dir/$target-management-upstream"
  local slskr_suite="$work_dir/$target-management-slskr"
  local upstream_normalized="$work_dir/$target-management-upstream.normalized"
  local slskr_normalized="$work_dir/$target-management-slskr.normalized"
  local upstream_log="$work_dir/$target-management-upstream.log"
  local slskr_log="$work_dir/$target-management-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$downloads" "$incomplete" "$upstream_state" "$slskr_state"
  printf 'denied before\n' >"$downloads/denied-before.flac"
  printf 'allowed watched\n' >"$downloads/allowed-watched.flac"
  printf 'allowed restarted\n' >"$downloads/allowed-restarted.flac"
  printf 'denied watched\n' >"$downloads/denied-watched.flac"
  printf 'denied restarted\n' >"$downloads/denied-restarted.flac"

  write_management_watch_yaml "$upstream_state/slskd.yml" false "$downloads" "$incomplete"
  (
    export SLSKD_APP_DIR="$upstream_state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-upstream-options.json" "$upstream_log"
  capture_get "$upstream_suite" management-options-before "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-before "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-denied-before "$base_url/api/v0/files/downloads/files/ZGVuaWVkLWJlZm9yZS5mbGFj"
  write_management_watch_yaml "$upstream_state/slskd.yml" true "$downloads" "$incomplete"
  wait_for_management_option "$base_url" true "$upstream_log"
  capture_get "$upstream_suite" management-options-enabled "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-enabled "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-allowed-watched "$base_url/api/v0/files/downloads/files/YWxsb3dlZC13YXRjaGVkLmZsYWM="
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-upstream-restart-options.json" "$upstream_log"
  capture_get "$upstream_suite" management-options-enabled-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-enabled-restarted "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-allowed-restarted "$base_url/api/v0/files/downloads/files/YWxsb3dlZC1yZXN0YXJ0ZWQuZmxhYw=="
  write_management_watch_yaml "$upstream_state/slskd.yml" false "$downloads" "$incomplete"
  wait_for_management_option "$base_url" false "$upstream_log"
  capture_get "$upstream_suite" management-options-disabled "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-disabled "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-denied-watched "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXdhdGNoZWQuZmxhYw=="
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-upstream-disabled-restart-options.json" "$upstream_log"
  capture_get "$upstream_suite" management-options-disabled-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-disabled-restarted "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-denied-restarted "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXJlc3RhcnRlZC5mbGFj"
  stop_daemon

  printf 'denied before\n' >"$downloads/denied-before.flac"
  printf 'allowed watched\n' >"$downloads/allowed-watched.flac"
  printf 'allowed restarted\n' >"$downloads/allowed-restarted.flac"
  printf 'denied watched\n' >"$downloads/denied-watched.flac"
  printf 'denied restarted\n' >"$downloads/denied-restarted.flac"
  write_management_watch_yaml "$slskr_state/slskd.yml" false "$downloads" "$incomplete"
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-slskr-options.json" "$slskr_log"
  capture_get "$slskr_suite" management-options-before "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-before "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-denied-before "$base_url/api/v0/files/downloads/files/ZGVuaWVkLWJlZm9yZS5mbGFj"
  write_management_watch_yaml "$slskr_state/slskd.yml" true "$downloads" "$incomplete"
  wait_for_management_option "$base_url" true "$slskr_log"
  capture_get "$slskr_suite" management-options-enabled "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-enabled "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-allowed-watched "$base_url/api/v0/files/downloads/files/YWxsb3dlZC13YXRjaGVkLmZsYWM="
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >>"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-slskr-restart-options.json" "$slskr_log"
  capture_get "$slskr_suite" management-options-enabled-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-enabled-restarted "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-allowed-restarted "$base_url/api/v0/files/downloads/files/YWxsb3dlZC1yZXN0YXJ0ZWQuZmxhYw=="
  write_management_watch_yaml "$slskr_state/slskd.yml" false "$downloads" "$incomplete"
  wait_for_management_option "$base_url" false "$slskr_log"
  capture_get "$slskr_suite" management-options-disabled "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-disabled "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-denied-watched "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXdhdGNoZWQuZmxhYw=="
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >>"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-slskr-disabled-restart-options.json" "$slskr_log"
  capture_get "$slskr_suite" management-options-disabled-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-disabled-restarted "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-denied-restarted "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXJlc3RhcnRlZC5mbGFj"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'remote file-management differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s remote file-management differential passed\n' "$target"
}

write_remote_configuration_yaml() {
  local path="$1"
  local enabled="$2"
  local temporary="$path.tmp"
  printf 'debug: true\nremote_configuration: %s\n' "$enabled" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_configuration_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteConfiguration"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'remote configuration differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'remote configuration differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_remote_configuration_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  capture_get "$suite" "configuration-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "configuration-options-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "configuration-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "configuration-yaml-$stage" "$base_url/api/v0/options/yaml"
  capture_get "$suite" "configuration-location-$stage" "$base_url/api/v0/options/yaml/location"
  capture_get "$suite" "configuration-debug-$stage" "$base_url/api/v0/options/debug"
  capture_request "$suite" "configuration-validate-$stage" POST \
    "$base_url/api/v0/options/yaml/validate" '"debug: true\n"'
  capture_request "$suite" "configuration-patch-$stage" PATCH \
    "$base_url/api/v0/options" '{}'
  if [[ "$target" == "slskdn" && ("$stage" == disabled || "$stage" == enabled) ]]; then
    capture_get "$suite" "configuration-security-current-before-$stage" \
      "$base_url/api/v0/security/adversarial"
    capture_request "$suite" "configuration-security-$stage" PUT \
      "$base_url/api/v0/security/adversarial" '{}'
    if [[ "$stage" == enabled ]]; then
      capture_get "$suite" "configuration-security-current-default-$stage" \
        "$base_url/api/v0/security/adversarial"
      capture_get "$suite" "configuration-security-yaml-$stage" \
        "$base_url/api/v0/options/yaml"
      capture_request "$suite" "configuration-security-custom-$stage" PUT \
        "$base_url/api/v0/security/adversarial" \
        '{"enabled":true,"profile":"Custom","privacy":{"enabled":true,"padding":{"enabled":true,"bucketSizes":[256,512]}}}'
      capture_get "$suite" "configuration-security-current-custom-$stage" \
        "$base_url/api/v0/security/adversarial"
      capture_get "$suite" "configuration-security-yaml-custom-$stage" \
        "$base_url/api/v0/options/yaml"
      capture_request "$suite" "configuration-security-invalid-$stage" PUT \
        "$base_url/api/v0/security/adversarial" \
        '{"privacy":{"padding":{"bucketSizes":[256,0]}}}'
      capture_get "$suite" "configuration-security-current-after-invalid-$stage" \
        "$base_url/api/v0/security/adversarial"
      capture_get "$suite" "configuration-security-yaml-after-invalid-$stage" \
        "$base_url/api/v0/options/yaml"
    fi
  fi
}

start_remote_configuration_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local port="$6"
  local https_port="$7"
  local listen_port="$8"
  local append="${9:-false}"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$append" == true ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$append" == true ]]; then
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec "$repo_root/target/debug/slskr" serve
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec "$repo_root/target/debug/slskr" serve
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

run_remote_configuration_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local false_payload
  false_payload="$($python_bin -c 'import json; print(json.dumps("debug: true\nremote_configuration: false\n"))')"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-configuration-$implementation"
    local suite="$work_dir/$target-configuration-$implementation"
    local log="$work_dir/$target-configuration-$implementation.log"
    mkdir -p "$state"
    write_remote_configuration_yaml "$state/slskd.yml" false
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-configuration-$implementation-options.json" "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" disabled

    write_remote_configuration_yaml "$state/slskd.yml" true
    wait_for_configuration_option "$base_url" true "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" enabled
    capture_request "$suite" configuration-yaml-self-disable PUT \
      "$base_url/api/v0/options/yaml" "$false_payload"
    wait_for_configuration_option "$base_url" false "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" self-disabled
    # Allow the polling watcher to observe the uploaded false value before the
    # next direct write; the upload path itself applies synchronously.
    sleep 0.3

    write_remote_configuration_yaml "$state/slskd.yml" true
    wait_for_configuration_option "$base_url" true "$log"
    stop_daemon
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-configuration-$implementation-restart-options.json" "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" enabled-restarted
    stop_daemon

    write_remote_configuration_yaml "$state/slskd.yml" false
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-configuration-$implementation-disabled-restart-options.json" "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" disabled-restarted
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-configuration-upstream.normalized"
  local slskr_normalized="$work_dir/$target-configuration-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-configuration-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-configuration-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'remote configuration differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s remote configuration differential passed\n' "$target"
}

write_debug_yaml() {
  local path="$1"
  local enabled="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: %s\n' "$enabled" >"$temporary"
  mv "$temporary" "$path"
}

write_debug_omitted_yaml() {
  local path="$1"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\n' >"$temporary"
  mv "$temporary" "$path"
}

wait_for_debug_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["debug"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'debug differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'debug differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_debug_log_state() {
  local suite="$1"
  local stage="$2"
  local log="$3"
  local observed=false
  if rg -q '\bDBG\]|\[Debug\]' "$log"; then
    observed=true
  fi
  printf '{"debugObserved":%s}' "$observed" >"$suite/debug-log-$stage.body"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/debug-log-$stage.meta"
}

capture_debug_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local log="$4"
  mkdir -p "$suite"
  capture_get "$suite" "debug-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "debug-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "debug-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "debug-view-$stage" "$base_url/api/v0/options/debug"
  capture_debug_log_state "$suite" "$stage" "$log"
}

start_debug_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local port="$6"
  local https_port="$7"
  local listen_port="$8"
  local debug_override="${9:-false}"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      if [[ "$debug_override" == true ]]; then
        export SLSKD_DEBUG=true
      fi
      exec dotnet "$dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      if [[ "$debug_override" == true ]]; then
        export SLSKD_DEBUG=true
      fi
      exec "$repo_root/target/debug/slskr" serve
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

run_debug_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local invalid_payload
  invalid_payload="$($python_bin -c 'import json; print(json.dumps("remote_configuration: true\ndebug: invalid\n"))')"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-debug-$implementation"
    local suite="$work_dir/$target-debug-$implementation"
    local log="$work_dir/$target-debug-$implementation.log"
    mkdir -p "$state" "$suite"
    write_debug_yaml "$state/slskd.yml" false
    start_debug_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-debug-$implementation-false.json" "$log"
    capture_debug_stage "$base_url" "$suite" false-startup "$log"

    write_debug_yaml "$state/slskd.yml" true
    wait_for_debug_option "$base_url" true "$log"
    capture_debug_stage "$base_url" "$suite" true-watched "$log"
    capture_request "$suite" debug-validate-invalid POST \
      "$base_url/api/v0/options/yaml/validate" "$invalid_payload"
    stop_daemon

    start_debug_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-debug-$implementation-true.json" "$log"
    capture_debug_stage "$base_url" "$suite" true-restarted "$log"

    write_debug_yaml "$state/slskd.yml" false
    wait_for_debug_option "$base_url" false "$log"
    capture_debug_stage "$base_url" "$suite" false-watched "$log"
    stop_daemon

    start_debug_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-debug-$implementation-false-restarted.json" "$log"
    capture_debug_stage "$base_url" "$suite" false-restarted "$log"
    stop_daemon

    write_debug_yaml "$state/slskd.yml" false
    start_debug_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-debug-$implementation-override.json" "$log"
    capture_debug_stage "$base_url" "$suite" yaml-precedence "$log"
    stop_daemon

    write_debug_omitted_yaml "$state/slskd.yml"
    start_debug_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-debug-$implementation-environment-cli.json" "$log"
    capture_debug_stage "$base_url" "$suite" environment-cli-enabled "$log"
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-debug-upstream.normalized"
  local slskr_normalized="$work_dir/$target-debug-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-debug-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-debug-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'debug differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s debug differential passed\n' "$target"
}

write_server_endpoint_yaml() {
  local path="$1"
  local address="$2"
  local port="$3"
  local listen_port="$4"
  local no_connect="${5:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nsoulseek:\n  address: "%s"\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$no_connect" "$address" "$port" "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

write_server_endpoint_omitted_yaml() {
  local path="$1"
  local listen_port="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\ndht:\n  enabled: false\nflags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_server_endpoint_option() {
  local base_url="$1"
  local expected_address="$2"
  local expected_port="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["soulseek"]; raise SystemExit(0 if value["address"] == sys.argv[1] and value["port"] == int(sys.argv[2]) else 1)' \
        "$expected_address" "$expected_port" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'server endpoint differential failed: daemon exited while waiting for %s:%s\n' \
        "$expected_address" "$expected_port" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'server endpoint differential failed: timed out waiting for %s:%s\n' \
    "$expected_address" "$expected_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_pending_reconnect() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/application" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["pendingReconnect"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'server endpoint differential failed: daemon exited while waiting for pendingReconnect=%s\n' \
        "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'server endpoint differential failed: timed out waiting for pendingReconnect=%s\n' \
    "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_server_endpoint_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local fixture_status="$4"
  capture_get "$suite" "endpoint-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "endpoint-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "endpoint-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "endpoint-server-$stage" "$base_url/api/v0/server"
  capture_fixture_status "$suite" "endpoint-network-$stage" "$fixture_status"
}

capture_server_endpoint_precedence() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "endpoint-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "endpoint-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "endpoint-debug-$stage" "$base_url/api/v0/options/debug"
}

start_server_endpoint_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local environment_address="${8:-}"
  local environment_port="${9:-}"
  local cli_address="${10:-}"
  local cli_port="${11:-}"
  local append="${12:-false}"
  local cli_args=()
  [[ -n "$cli_address" ]] && cli_args+=(--slsk-address "$cli_address")
  [[ -n "$cli_port" ]] && cli_args+=(--slsk-port "$cli_port")
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$append" == true ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_address" ]] && export SLSKD_SLSK_ADDRESS="$environment_address"
        [[ -n "$environment_port" ]] && export SLSKD_SLSK_PORT="$environment_port"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_address" ]] && export SLSKD_SLSK_ADDRESS="$environment_address"
        [[ -n "$environment_port" ]] && export SLSKD_SLSK_PORT="$environment_port"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$append" == true ]]; then
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_address" ]] && export SLSKD_SLSK_ADDRESS="$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_SLSK_PORT="$environment_port"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_address" ]] && export SLSKD_SLSK_ADDRESS="$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_SLSK_PORT="$environment_port"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" "${cli_args[@]}"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

disconnect_server_endpoint() {
  local base_url="$1"
  curl --fail --silent --show-error --max-time 5 \
    --request DELETE --header 'Content-Type: application/json' \
    --data-binary '"endpoint switch"' "$base_url/api/v0/server" >/dev/null
}

connect_server_endpoint() {
  local base_url="$1"
  curl --fail --silent --show-error --max-time 5 \
    --request PUT "$base_url/api/v0/server" >/dev/null
}

run_server_endpoint_scenario() {
  local target="$1"
  local root="$2"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local server_port_a="$(pick_free_port)"
  local server_port_b="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-endpoint-$implementation"
    local suite="$work_dir/$target-endpoint-$implementation"
    local log="$work_dir/$target-endpoint-$implementation.log"
    local fixture_status_a="$work_dir/$target-endpoint-$implementation-a.json"
    local fixture_status_b="$work_dir/$target-endpoint-$implementation-b.json"
    local fixture_log_a="$work_dir/$target-endpoint-$implementation-a.log"
    local fixture_log_b="$work_dir/$target-endpoint-$implementation-b.log"
    mkdir -p "$state" "$suite"

    start_soulseek_fixture "$server_port_a" "$fixture_status_a" "$fixture_log_a" login-success 0.0.0.0
    write_server_endpoint_yaml "$state/slskd.yml" 127.0.0.1 "$server_port_a" "$listen_port"
    start_server_endpoint_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-endpoint-$implementation-options.json" "$log"
    wait_for_fixture_active "$fixture_status_a" 1 "$log"
    capture_server_endpoint_stage "$base_url" "$suite" initial "$fixture_status_a"

    local low_port_payload high_port_payload invalid_port_payload
    low_port_payload="$($python_bin -c 'import json; print(json.dumps("flags:\n  no_connect: true\nsoulseek:\n  port: 1023\n"))')"
    high_port_payload="$($python_bin -c 'import json; print(json.dumps("flags:\n  no_connect: true\nsoulseek:\n  port: 65536\n"))')"
    invalid_port_payload="$($python_bin -c 'import json; print(json.dumps("flags:\n  no_connect: true\nsoulseek:\n  port: invalid\n"))')"
    capture_request "$suite" endpoint-validation-low-port POST \
      "$base_url/api/v0/options/yaml/validate" "$low_port_payload"
    capture_request "$suite" endpoint-validation-high-port POST \
      "$base_url/api/v0/options/yaml/validate" "$high_port_payload"
    capture_request "$suite" endpoint-validation-invalid-port POST \
      "$base_url/api/v0/options/yaml/validate" "$invalid_port_payload"

    write_server_endpoint_yaml "$state/slskd.yml" 127.0.0.2 "$server_port_b" "$listen_port"
    wait_for_server_endpoint_option "$base_url" 127.0.0.2 "$server_port_b" "$log"
    wait_for_pending_reconnect "$base_url" true "$log"
    wait_for_fixture_active "$fixture_status_a" 1 "$log"
    capture_server_endpoint_stage "$base_url" "$suite" watched "$fixture_status_a"

    disconnect_server_endpoint "$base_url"
    wait_for_fixture_active "$fixture_status_a" 0 "$log"
    wait_for_pending_reconnect "$base_url" false "$log"
    capture_server_endpoint_stage "$base_url" "$suite" disconnected "$fixture_status_a"
    stop_soulseek_fixture

    start_soulseek_fixture "$server_port_b" "$fixture_status_b" "$fixture_log_b" login-success 0.0.0.0
    connect_server_endpoint "$base_url"
    wait_for_fixture_active "$fixture_status_b" 1 "$log"
    capture_server_endpoint_stage "$base_url" "$suite" reconnected "$fixture_status_b"
    stop_daemon
    wait_for_fixture_active "$fixture_status_b" 0 "$log"

    start_server_endpoint_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" "" "" true
    wait_for_options "$base_url" "$work_dir/$target-endpoint-$implementation-restarted.json" "$log"
    wait_for_fixture_active "$fixture_status_b" 1 "$log"
    capture_server_endpoint_stage "$base_url" "$suite" restarted "$fixture_status_b"
    stop_daemon
    stop_soulseek_fixture

    local precedence_state="$work_dir/state-$target-endpoint-precedence-$implementation"
    local precedence_log="$work_dir/$target-endpoint-precedence-$implementation.log"
    mkdir -p "$precedence_state"
    write_server_endpoint_yaml "$precedence_state/slskd.yml" yaml-address.example 30001 "$listen_port" true
    start_server_endpoint_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" env-address.example 30002
    wait_for_options "$base_url" "$work_dir/$target-endpoint-$implementation-yaml-precedence.json" "$precedence_log"
    capture_server_endpoint_precedence "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_server_endpoint_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" env-address.example 30002 \
      cli-address.example 30003 true
    wait_for_options "$base_url" "$work_dir/$target-endpoint-$implementation-cli-precedence.json" "$precedence_log"
    capture_server_endpoint_precedence "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    write_server_endpoint_omitted_yaml "$precedence_state/slskd.yml" "$listen_port"
    start_server_endpoint_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" env-address.example 30002 "" "" true
    wait_for_options "$base_url" "$work_dir/$target-endpoint-$implementation-environment.json" "$precedence_log"
    capture_server_endpoint_precedence "$base_url" "$suite" environment-with-yaml-omitted
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-endpoint-upstream.normalized"
  local slskr_normalized="$work_dir/$target-endpoint-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-endpoint-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-endpoint-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'server endpoint differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s server endpoint differential passed\n' "$target"
}

write_credentials_yaml() {
  local path="$1"
  local username="$2"
  local password="$3"
  local server_port="$4"
  local listen_port="$5"
  local no_connect="${6:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: "%s"\n  password: "%s"\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$no_connect" "$server_port" "$username" "$password" "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

write_credentials_omitted_yaml() {
  local path="$1"
  local server_port="$2"
  local listen_port="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: true\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$server_port" "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

password_sha256() {
  "$python_bin" -c 'import hashlib,sys; print(hashlib.sha256(sys.argv[1].encode()).hexdigest())' "$1"
}

wait_for_credential_option() {
  local base_url="$1"
  local expected_username="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["soulseek"]; raise SystemExit(0 if value.get("username") == sys.argv[1] else 1)' \
        "$expected_username" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'credential differential failed: daemon exited while waiting for username %s\n' \
        "$expected_username" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'credential differential failed: timed out waiting for username %s\n' \
    "$expected_username" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_fixture_login() {
  local status_file="$1"
  local expected_count="$2"
  local expected_username="$3"
  local expected_password_hash="$4"
  local log="$5"
  for _ in $(seq 1 600); do
    if "$python_bin" - "$status_file" "$expected_count" "$expected_username" "$expected_password_hash" <<'PY' 2>/dev/null
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
count=int(sys.argv[2])
users=value.get("login_usernames", [])
hashes=value.get("login_password_sha256", [])
raise SystemExit(0 if len(users) == count and len(hashes) == count and users[-1] == sys.argv[3] and hashes[-1] == sys.argv[4] else 1)
PY
    then
      return
    fi
    if [[ -n "$daemon_pid" ]] && ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'credential differential failed: daemon exited while waiting for fixture login %s\n' \
        "$expected_username" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'credential differential failed: timed out waiting for fixture login %s\n' \
    "$expected_username" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_credential_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local fixture_status="$4"
  capture_get "$suite" "credential-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "credential-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "credential-application-$stage" "$base_url/api/v0/application"
  capture_fixture_status "$suite" "credential-network-$stage" "$fixture_status"
}

capture_credential_precedence() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "credential-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "credential-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "credential-debug-$stage" "$base_url/api/v0/options/debug"
}

start_credential_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local environment_username="${8:-}"
  local environment_password="${9:-}"
  local cli_username="${10:-}"
  local cli_password="${11:-}"
  local append="${12:-false}"
  local cli_args=()
  [[ -n "$cli_username" ]] && cli_args+=(--slsk-username "$cli_username")
  [[ -n "$cli_password" ]] && cli_args+=(--slsk-password "$cli_password")
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$append" == true ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_username" ]] && export SLSKD_SLSK_USERNAME="$environment_username"
        [[ -n "$environment_password" ]] && export SLSKD_SLSK_PASSWORD="$environment_password"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_username" ]] && export SLSKD_SLSK_USERNAME="$environment_username"
        [[ -n "$environment_password" ]] && export SLSKD_SLSK_PASSWORD="$environment_password"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$append" == true ]]; then
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_username" ]] && export SLSKD_SLSK_USERNAME="$environment_username"
      [[ -n "$environment_password" ]] && export SLSKD_SLSK_PASSWORD="$environment_password"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_username" ]] && export SLSKD_SLSK_USERNAME="$environment_username"
      [[ -n "$environment_password" ]] && export SLSKD_SLSK_PASSWORD="$environment_password"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

run_credential_scenario() {
  local target="$1"
  local root="$2"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local server_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  local hash_a="$(password_sha256 credential-password-a)"
  local hash_b="$(password_sha256 credential-password-b)"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-credential-$implementation"
    local suite="$work_dir/$target-credential-$implementation"
    local log="$work_dir/$target-credential-$implementation.log"
    local fixture_status="$work_dir/$target-credential-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-credential-$implementation-fixture.log"
    mkdir -p "$state" "$suite"

    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success 0.0.0.0
    write_credentials_yaml "$state/slskd.yml" credential-user-a credential-password-a \
      "$server_port" "$listen_port"
    start_credential_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-credential-$implementation-options.json" "$log"
    wait_for_fixture_login "$fixture_status" 1 credential-user-a "$hash_a" "$log"
    capture_credential_stage "$base_url" "$suite" initial "$fixture_status"

    write_credentials_yaml "$state/slskd.yml" credential-user-b credential-password-b \
      "$server_port" "$listen_port"
    wait_for_credential_option "$base_url" credential-user-b "$log"
    wait_for_pending_reconnect "$base_url" true "$log"
    wait_for_fixture_login "$fixture_status" 1 credential-user-a "$hash_a" "$log"
    capture_credential_stage "$base_url" "$suite" watched "$fixture_status"

    disconnect_server_endpoint "$base_url"
    wait_for_fixture_active "$fixture_status" 0 "$log"
    wait_for_pending_reconnect "$base_url" false "$log"
    capture_credential_stage "$base_url" "$suite" disconnected "$fixture_status"
    connect_server_endpoint "$base_url"
    wait_for_fixture_login "$fixture_status" 2 credential-user-b "$hash_b" "$log"
    capture_credential_stage "$base_url" "$suite" reconnected "$fixture_status"
    stop_daemon
    wait_for_fixture_active "$fixture_status" 0 "$log"

    start_credential_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" "" "" true
    wait_for_options "$base_url" "$work_dir/$target-credential-$implementation-restarted.json" "$log"
    wait_for_fixture_login "$fixture_status" 3 credential-user-b "$hash_b" "$log"
    capture_credential_stage "$base_url" "$suite" restarted "$fixture_status"
    stop_daemon
    stop_soulseek_fixture

    local precedence_state="$work_dir/state-$target-credential-precedence-$implementation"
    local precedence_log="$work_dir/$target-credential-precedence-$implementation.log"
    mkdir -p "$precedence_state"
    write_credentials_yaml "$precedence_state/slskd.yml" credential-yaml credential-yaml-password \
      "$server_port" "$listen_port" true
    start_credential_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" credential-env credential-env-password
    wait_for_options "$base_url" "$work_dir/$target-credential-$implementation-yaml-precedence.json" "$precedence_log"
    capture_credential_precedence "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_credential_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" credential-env credential-env-password \
      credential-cli credential-cli-password true
    wait_for_options "$base_url" "$work_dir/$target-credential-$implementation-cli-precedence.json" "$precedence_log"
    capture_credential_precedence "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    write_credentials_omitted_yaml "$precedence_state/slskd.yml" "$server_port" "$listen_port"
    start_credential_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" credential-env credential-env-password "" "" true
    wait_for_options "$base_url" "$work_dir/$target-credential-$implementation-environment.json" "$precedence_log"
    capture_credential_precedence "$base_url" "$suite" environment-with-yaml-omitted
    stop_daemon

    start_credential_daemon "$target" "$root" "$implementation" "$precedence_state" \
      "$precedence_log" "$http_port" "$https_port" "" "" "" "" true
    wait_for_options "$base_url" "$work_dir/$target-credential-$implementation-default.json" "$precedence_log"
    capture_credential_precedence "$base_url" "$suite" defaults
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-credential-upstream.normalized"
  local slskr_normalized="$work_dir/$target-credential-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-credential-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-credential-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'credential differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s credential differential passed\n' "$target"
}

write_config_watch_yaml() {
  local path="$1"
  local no_config_watch="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_config_watch: %s\n' \
    "$no_config_watch" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_config_watch_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["flags"]["noConfigWatch"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'config-watch differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'config-watch differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_config_watch_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "config-watch-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "config-watch-application-$stage" "$base_url/api/v0/application"
}

run_config_watch_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local true_payload
  local false_payload
  true_payload="$($python_bin -c 'import json; print(json.dumps("remote_configuration: true\nflags:\n  no_config_watch: true\n"))')"
  false_payload="$($python_bin -c 'import json; print(json.dumps("remote_configuration: true\nflags:\n  no_config_watch: false\n"))')"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-config-watch-$implementation"
    local suite="$work_dir/$target-config-watch-$implementation"
    local log="$work_dir/$target-config-watch-$implementation.log"
    mkdir -p "$state"
    write_config_watch_yaml "$state/slskd.yml" false
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-config-watch-$implementation-options.json" "$log"
    capture_config_watch_stage "$base_url" "$suite" disabled

    write_config_watch_yaml "$state/slskd.yml" true
    wait_for_config_watch_option "$base_url" true "$log"
    capture_config_watch_stage "$base_url" "$suite" enabled-watched
    capture_request "$suite" config-watch-yaml-put-enabled PUT \
      "$base_url/api/v0/options/yaml" "$true_payload"
    capture_config_watch_stage "$base_url" "$suite" enabled-uploaded
    stop_daemon

    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-config-watch-$implementation-enabled-restart-options.json" "$log"
    capture_config_watch_stage "$base_url" "$suite" enabled-restarted
    write_config_watch_yaml "$state/slskd.yml" false
    wait_for_config_watch_option "$base_url" false "$log"
    capture_config_watch_stage "$base_url" "$suite" disabled-watched
    capture_request "$suite" config-watch-yaml-put-disabled PUT \
      "$base_url/api/v0/options/yaml" "$false_payload"
    capture_config_watch_stage "$base_url" "$suite" disabled-uploaded
    stop_daemon

    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-config-watch-$implementation-disabled-restart-options.json" "$log"
    capture_config_watch_stage "$base_url" "$suite" disabled-restarted
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-config-watch-upstream.normalized"
  local slskr_normalized="$work_dir/$target-config-watch-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-config-watch-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-config-watch-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'config-watch differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s config-watch differential passed\n' "$target"
}

write_description_yaml() {
  local path="$1"
  local description="$2"
  local server_port="$3"
  local listen_port="$4"
  local picture="$5"
  local temporary="$path.tmp"
  printf 'flags:\n  no_connect: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  description: "%s"\n  picture: "%s"\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$server_port" "$description" "$picture" "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_description_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["soulseek"]["description"])' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'description differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'description differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_direct_user_info() {
  local suite="$1"
  local label="$2"
  local port="$3"
  local daemon_log="$4"
  mkdir -p "$suite"
  for _ in $(seq 1 100); do
    if SLSK_DIRECT_PEER_HOST=127.0.0.1 \
      SLSK_DIRECT_PEER_PORT="$port" \
      SLSK_DIRECT_PEER_TIMEOUT_SECONDS=2 \
      SLSK_DIRECT_USER_INFO_INCLUDE_PICTURE=true \
      "$repo_root/target/debug/slskr" direct-user-info-probe \
      >"$suite/$label.body.tmp" 2>"$suite/$label.error"; then
      mv "$suite/$label.body.tmp" "$suite/$label.body"
      rm -f "$suite/$label.error"
      printf 'status=200\ncontent-type=application/json\n' >"$suite/$label.meta"
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'description differential failed: daemon exited before peer listener probe\n' >&2
      tail -120 "$daemon_log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'description differential failed: direct user-info probe did not succeed\n' >&2
  cat "$suite/$label.error" >&2 || true
  tail -120 "$daemon_log" >&2 || true
  exit 1
}

capture_description_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "description-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "description-application-$stage" "$base_url/api/v0/application"
}

run_description_scenario() {
  local target="$1"
  local root="$2"

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-description-$implementation"
    local suite="$work_dir/$target-description-$implementation"
    local log="$work_dir/$target-description-$implementation.log"
    local fixture_status="$work_dir/$target-description-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-description-$implementation-fixture.log"
    local picture_before="$state/picture-before.bin"
    local picture_watched="$state/picture-watched.bin"
    mkdir -p "$state"
    printf '\x00\x01\x02\xff' >"$picture_before"
    printf '\x09\x08\x07' >"$picture_watched"
    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success
    write_description_yaml "$state/slskd.yml" "old description" "$server_port" "$listen_port" "$picture_before"
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-description-$implementation-options.json" "$log"
    capture_description_stage "$base_url" "$suite" before
    capture_direct_user_info "$suite" description-peer-before "$listen_port" "$log"

    write_description_yaml "$state/slskd.yml" "new description Ω" "$server_port" "$listen_port" "$picture_watched"
    wait_for_description_option "$base_url" "new description Ω" "$log"
    capture_description_stage "$base_url" "$suite" watched
    capture_direct_user_info "$suite" description-peer-watched "$listen_port" "$log"
    if [[ "$target" == slskdn ]]; then
      capture_request "$suite" description-nowplaying-put PUT \
        "$base_url/api/v0/nowplaying" '{"artist":"Fixture Artist","title":"Fixture Track"}'
      capture_direct_user_info "$suite" description-peer-nowplaying "$listen_port" "$log"
      capture_delete "$suite" description-nowplaying-delete "$base_url/api/v0/nowplaying"
      capture_direct_user_info "$suite" description-peer-cleared "$listen_port" "$log"
    fi
    stop_daemon

    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/$target-description-$implementation-restart-options.json" "$log"
    capture_description_stage "$base_url" "$suite" restarted
    capture_direct_user_info "$suite" description-peer-restarted "$listen_port" "$log"
    stop_daemon
    stop_soulseek_fixture
  done

  local upstream_normalized="$work_dir/$target-description-upstream.normalized"
  local slskr_normalized="$work_dir/$target-description-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-description-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-description-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'description differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s description differential passed\n' "$target"
}

write_no_connect_yaml() {
  local path="$1"
  local no_connect="$2"
  local server_port="$3"
  local listen_port="$4"
  local listen_ip="${5:-0.0.0.0}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: %s\n  listen_port: %s\n' \
    "$no_connect" "$server_port" "$listen_ip" "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_no_connect_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["flags"]["noConnect"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'no-connect differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'no-connect differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

start_soulseek_fixture() {
  local port="$1"
  local status="$2"
  local log="$3"
  local mode="${4:-}"
  local host="${5:-127.0.0.1}"
  "$python_bin" "$repo_root/scripts/fixture-soulseek-listener.py" \
    "$host" "$port" "$status" ${mode:+"$mode"} >"$log" 2>&1 &
  soulseek_fixture_pid="$!"
  for _ in $(seq 1 100); do
    [[ -s "$status" ]] && return
    if ! kill -0 "$soulseek_fixture_pid" 2>/dev/null; then
      printf 'no-connect differential failed: fixture listener exited\n' >&2
      cat "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'no-connect differential failed: fixture listener did not become ready\n' >&2
  exit 1
}

wait_for_fixture_active() {
  local status="$1"
  local expected="$2"
  local daemon_log="$3"
  for _ in $(seq 1 300); do
    if "$python_bin" - "$status" "$expected" <<'PY'
import json,sys
try:
    value=json.load(open(sys.argv[1], encoding="utf-8"))
except (FileNotFoundError, json.JSONDecodeError):
    raise SystemExit(1)
expected=int(sys.argv[2])
active=int(value.get("active", 0))
accepted=int(value.get("accepted", 0))
raise SystemExit(0 if active == expected and (expected == 0 or accepted > 0) else 1)
PY
    then
      return
    fi
    sleep 0.05
  done
  printf 'no-connect differential failed: fixture active state did not reach %s\n' "$expected" >&2
  cat "$status" >&2 || true
  tail -120 "$daemon_log" >&2 || true
  exit 1
}

assert_fixture_never_connected() {
  local status="$1"
  local daemon_log="$2"
  sleep 0.5
  if ! "$python_bin" - "$status" <<'PY'
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
raise SystemExit(0 if value.get("accepted") == 0 and value.get("active") == 0 else 1)
PY
  then
    printf 'no-connect differential failed: connection occurred while startup flag was set\n' >&2
    cat "$status" >&2 || true
    tail -120 "$daemon_log" >&2 || true
    exit 1
  fi
}

capture_fixture_status() {
  local suite="$1"
  local label="$2"
  local status="$3"
  mkdir -p "$suite"
  cp "$status" "$suite/$label.body"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/$label.meta"
}

start_no_connect_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local append="${8:-false}"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$append" == true ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        exec dotnet "$dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$append" == true ]]; then
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

capture_no_connect_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "no-connect-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "no-connect-application-$stage" "$base_url/api/v0/application"
}

run_no_connect_invalid_watch_case() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local suite="$4"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local server_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  local state="$work_dir/state-$target-no-connect-invalid-$implementation"
  local log="$work_dir/$target-no-connect-invalid-$implementation.log"
  mkdir -p "$state" "$suite"
  write_no_connect_yaml "$state/slskd.yml" true "$server_port" "$listen_port" 127.0.0.1
  start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
    "$http_port" "$https_port"
  wait_for_options "$base_url" "$work_dir/$target-no-connect-invalid-$implementation-options.json" "$log"
  write_no_connect_yaml "$state/slskd.yml" false "$server_port" "$listen_port" 127.0.0.1

  if [[ "$target" == slskdn ]]; then
    local observed=false
    for _ in $(seq 1 200); do
      local status
      status="$(curl --silent --show-error --max-time 1 \
        --output "$suite/no-connect-invalid-watch.body" \
        --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
        "$base_url/api/v0/options" \
        >"$suite/no-connect-invalid-watch.meta" 2>/dev/null \
        && sed -n 's/^status=//p' "$suite/no-connect-invalid-watch.meta" || true)"
      if [[ "$status" == 500 ]]; then
        observed=true
        break
      fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then
        printf 'no-connect differential failed: slskdN exited before exposing invalid options state\n' >&2
        tail -120 "$log" >&2 || true
        exit 1
      fi
      sleep 0.05
    done
    if [[ "$observed" != true ]]; then
      printf 'no-connect differential failed: slskdN did not expose its watched validation failure\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    write_no_connect_yaml "$state/slskd.yml" true "$server_port" "$listen_port" 127.0.0.1
    wait_for_no_connect_option "$base_url" true "$log"
    stop_daemon
  else
    wait_for_no_connect_option "$base_url" false "$log"
    capture_get "$suite" no-connect-invalid-watch "$base_url/api/v0/options"
    stop_daemon
  fi
}

run_no_connect_scenario() {
  local target="$1"
  local root="$2"

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-connect-$implementation"
    local suite="$work_dir/$target-no-connect-$implementation"
    local log="$work_dir/$target-no-connect-$implementation.log"
    local fixture_status="$work_dir/$target-no-connect-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-no-connect-$implementation-fixture.log"
    mkdir -p "$state"
    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log"
    write_no_connect_yaml "$state/slskd.yml" true "$server_port" "$listen_port"
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-no-connect-$implementation-options.json" "$log"
    assert_fixture_never_connected "$fixture_status" "$log"
    capture_no_connect_stage "$base_url" "$suite" startup-disabled
    capture_fixture_status "$suite" no-connect-network-startup-disabled "$fixture_status"
    local loopback_validation_payload
    loopback_validation_payload="$($python_bin -c 'import json,sys; print(json.dumps("remote_configuration: true\nflags:\n  no_connect: false\nsoulseek:\n  address: 127.0.0.1\n  port: " + sys.argv[1] + "\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 127.0.0.1\n  listen_port: " + sys.argv[2] + "\n"))' "$server_port" "$listen_port")"
    capture_request "$suite" no-connect-loopback-validation POST \
      "$base_url/api/v0/options/yaml/validate" "$loopback_validation_payload"

    write_no_connect_yaml "$state/slskd.yml" false "$server_port" "$listen_port"
    wait_for_no_connect_option "$base_url" false "$log"
    assert_fixture_never_connected "$fixture_status" "$log"
    capture_no_connect_stage "$base_url" "$suite" watched-enabled
    capture_fixture_status "$suite" no-connect-network-watched-enabled "$fixture_status"
    stop_daemon

    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/$target-no-connect-$implementation-restart-options.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    capture_no_connect_stage "$base_url" "$suite" restarted-enabled
    capture_fixture_status "$suite" no-connect-network-restarted-enabled "$fixture_status"

    write_no_connect_yaml "$state/slskd.yml" true "$server_port" "$listen_port"
    wait_for_no_connect_option "$base_url" true "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    capture_no_connect_stage "$base_url" "$suite" watched-disabled
    capture_fixture_status "$suite" no-connect-network-watched-disabled "$fixture_status"
    stop_daemon
    wait_for_fixture_active "$fixture_status" 0 "$log"

    local accepted_before
    accepted_before="$($python_bin -c 'import json,sys; print(json.load(open(sys.argv[1]))["accepted"])' "$fixture_status")"
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/$target-no-connect-$implementation-disabled-restart-options.json" "$log"
    sleep 0.5
    if ! "$python_bin" - "$fixture_status" "$accepted_before" <<'PY'
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
raise SystemExit(0 if value.get("accepted") == int(sys.argv[2]) and value.get("active") == 0 else 1)
PY
    then
      printf 'no-connect differential failed: restart with flag set opened a connection\n' >&2
      cat "$fixture_status" >&2 || true
      exit 1
    fi
    capture_no_connect_stage "$base_url" "$suite" restarted-disabled
    capture_fixture_status "$suite" no-connect-network-restarted-disabled "$fixture_status"
    stop_daemon
    stop_soulseek_fixture
    run_no_connect_invalid_watch_case "$target" "$root" "$implementation" "$suite"
  done

  local upstream_normalized="$work_dir/$target-no-connect-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-connect-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-connect-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-connect-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-connect differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-connect differential passed\n' "$target"
}

host_ipv4_address() {
  "$python_bin" - <<'PY'
import socket
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
try:
    sock.connect(("192.0.2.1", 9))
    address = sock.getsockname()[0]
finally:
    sock.close()
if address.startswith("127."):
    raise SystemExit("listener differential requires a non-loopback IPv4 address")
print(address)
PY
}

write_swagger_yaml() {
  local path="$1"
  local swagger="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ndht:\n  enabled: false\n' >"$temporary"
  if [[ "$swagger" != __unset__ ]]; then
    printf 'feature:\n  swagger: %s\n' "$swagger" >>"$temporary"
  fi
  mv "$temporary" "$path"
}

start_swagger_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_swagger="$9"
  local command_line_swagger="${10}"
  local append="${11:-false}"
  (
    unset SLSKD_SWAGGER
    if [[ "$environment_swagger" != __unset__ ]]; then
      export SLSKD_SWAGGER="$environment_swagger"
    fi
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      local args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
      [[ "$command_line_swagger" == true ]] && args+=(--swagger)
      if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      local args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
      [[ "$command_line_swagger" == true ]] && args+=(--swagger)
      if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
    fi
  ) &
  daemon_pid="$!"
}

wait_for_swagger_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 400); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value["feature"]["swagger"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'swagger differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'swagger differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_swagger_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/swagger-$stage.body" <<'PY'
import http.client,json,os,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=3)
    connection.request("GET",path)
    response=connection.getresponse()
    body=response.read()
    result={"status":response.status,"type":response.getheader("Content-Type","").split(";",1)[0].lower(),"location":response.getheader("Location",""),"body":body}
    connection.close()
    return result
options=json.loads(get("/api/v0/options")["body"])
startup=json.loads(get("/api/v0/options/startup")["body"])
application=json.loads(get("/api/v0/application")["body"])
routes={path:get(path) for path in ("/swagger","/swagger/index.html","/swagger/v0/swagger.json","/swagger/index.js","/swagger/swagger-ui.css","/swagger/index.css","/swagger/swagger-ui-bundle.js","/swagger/swagger-ui-standalone-preset.js")}
spec={}
if routes["/swagger/v0/swagger.json"]["status"] == 200:
    spec=json.loads(routes["/swagger/v0/swagger.json"]["body"])
index=routes["/swagger/index.html"]["body"].decode("utf-8",errors="replace")
index_js=routes["/swagger/index.js"]["body"].decode("utf-8",errors="replace")
print(json.dumps({
    "current":options["feature"]["swagger"],"startup":startup["feature"]["swagger"],"pendingRestart":application["pendingRestart"],
    "routes":{path:{"status":value["status"],"type":value["type"],"location":value["location"]} for path,value in routes.items()},
    "indexShell":routes["/swagger/index.html"]["status"] != 200 or ("Swagger UI" in index and "swagger-ui-bundle.js" in index and "index.js" in index),
    "indexTargetsV0":routes["/swagger/index.js"]["status"] != 200 or "/swagger/v0/swagger.json" in index_js,
    "specOpenApi":spec.get("openapi"),"specTitle":spec.get("info",{}).get("title"),"specHasPaths":not spec or bool(spec.get("paths")),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/swagger-$stage.meta"
}

run_swagger_scenario() {
  local target="$1"
  local root="$2"
  local default=false
  [[ "$target" == slskdn ]] && default=true
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-swagger-$implementation"
    local suite="$work_dir/$target-swagger-$implementation"
    local log="$work_dir/$target-swagger-$implementation.log"
    mkdir -p "$state" "$suite"

    write_swagger_yaml "$state/slskd.yml" __unset__
    start_swagger_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_swagger_option "$base_url" "$default" "$log"
    capture_swagger_stage "$base_url" "$suite" default
    stop_daemon

    write_swagger_yaml "$state/slskd.yml" false
    start_swagger_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false true
    wait_for_swagger_option "$base_url" false "$log"
    capture_swagger_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_swagger_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true true
    wait_for_swagger_option "$base_url" true "$log"
    capture_swagger_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon

    write_swagger_yaml "$state/slskd.yml" false
    start_swagger_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    wait_for_swagger_option "$base_url" false "$log"
    capture_swagger_stage "$base_url" "$suite" lifecycle-startup
    write_swagger_yaml "$state/slskd.yml" true
    wait_for_swagger_option "$base_url" true "$log"
    capture_swagger_stage "$base_url" "$suite" lifecycle-watched
    stop_daemon

    start_swagger_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    wait_for_swagger_option "$base_url" true "$log"
    capture_swagger_stage "$base_url" "$suite" lifecycle-restarted
    capture_request "$suite" swagger-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'feature:\n  swagger: null\n')"
    capture_request "$suite" swagger-validation-parent-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'feature: null\n')"
    capture_request "$suite" swagger-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'feature:\n  swagger: nope\n')"
    capture_request "$suite" swagger-validation-parent-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'feature: [true]\n')"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-swagger-upstream.normalized"
  local slskr_normalized="$work_dir/$target-swagger-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-swagger-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-swagger-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'swagger differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s swagger differential passed\n' "$target"
}

write_metrics_yaml() {
  local path="$1"
  local enabled="${2:-__unset__}"
  local url="${3:-__unset__}"
  local auth_disabled="${4:-__unset__}"
  local username="${5:-__unset__}"
  local password="${6:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ndht:\n  enabled: false\n' >"$temporary"
  if [[ "$enabled" != __unset__ ]]; then
    printf 'metrics:\n  enabled: %s\n  url: %s\n  authentication:\n    disabled: %s\n    username: %s\n    password: %s\n' \
      "$enabled" "$url" "$auth_disabled" "$username" "$password" >>"$temporary"
  fi
  mv "$temporary" "$path"
}

start_metrics_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_mode="$9"
  local command_line_mode="${10}"
  local append="${11:-false}"
  (
    unset SLSKD_METRICS SLSKD_METRICS_URL SLSKD_METRICS_NO_AUTH
    unset SLSKD_METRICS_USERNAME SLSKD_METRICS_PASSWORD
    if [[ "$environment_mode" == lower ]]; then
      export SLSKD_METRICS=false SLSKD_METRICS_URL=environment-metrics
      export SLSKD_METRICS_NO_AUTH=false SLSKD_METRICS_USERNAME=environment-user
      export SLSKD_METRICS_PASSWORD=environment-pass
    fi
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
    fi
    if [[ "$command_line_mode" == override ]]; then
      args+=(--metrics --metrics-url cli-metrics --metrics-no-auth --metrics-username cli-user --metrics-password cli-pass)
    fi
    if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
  ) &
  daemon_pid="$!"
}

wait_for_metrics_option() {
  local base_url="$1"
  local enabled="$2"
  local url="$3"
  local auth_disabled="$4"
  local username="$5"
  local log="$6"
  for _ in $(seq 1 500); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; m=json.load(sys.stdin)["metrics"]; raise SystemExit(0 if m["enabled"] == (sys.argv[1] == "true") and m["url"] == sys.argv[2] and m["authentication"]["disabled"] == (sys.argv[3] == "true") and m["authentication"]["username"] == sys.argv[4] and m["authentication"]["password"] == "*****" else 1)' \
        "$enabled" "$url" "$auth_disabled" "$username" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'metrics differential failed: daemon exited while waiting for %s\n' "$url" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'metrics differential failed: timed out waiting for %s\n' "$url" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_metrics_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local first_path="$4"
  local second_path="${5:-$first_path}"
  local username="${6:-metrics-user}"
  local password="${7:-metrics-pass}"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" "$first_path" "$second_path" "$username" "$password" >"$suite/metrics-$stage.body" <<'PY'
import base64,http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
paths=list(dict.fromkeys(sys.argv[2:4]))
valid=base64.b64encode(f"{sys.argv[4]}:{sys.argv[5]}".encode()).decode()
headers={"none":{},"malformed":{"Authorization":"Basic !!!"},"wrong":{"Authorization":"Basic d3Jvbmc6d3Jvbmc="},"correct":{"Authorization":f"Basic {valid}"}}
def get(path, headers=None):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path,headers=headers or {})
    response=connection.getresponse()
    body=response.read()
    result={"status":response.status,"type":response.getheader("Content-Type","").split(";",1)[0].lower(),"authenticate":response.getheader("WWW-Authenticate","").lower(),"body":body}
    connection.close()
    return result
options=json.loads(get("/api/v0/options")["body"])
startup=json.loads(get("/api/v0/options/startup")["body"])
application=json.loads(get("/api/v0/application")["body"])
routes={}
for path in paths:
    routes[path]={}
    for name,request_headers in headers.items():
        response=get(path,request_headers)
        text=response["body"].decode("utf-8",errors="replace")
        routes[path][name]={
            "status":response["status"],"type":response["type"],"authenticate":response["authenticate"],
            "prometheus":response["status"] != 200 or response["type"] != "text/plain" or ("# HELP" in text and "# TYPE" in text and "\nsl" in text),
        }
print(json.dumps({"current":options["metrics"],"startup":startup["metrics"],"pendingRestart":application["pendingRestart"],"routes":routes},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/metrics-$stage.meta"
}

run_metrics_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-metrics-$implementation"
    local suite="$work_dir/$target-metrics-$implementation"
    local log="$work_dir/$target-metrics-$implementation.log"
    mkdir -p "$state" "$suite"

    write_metrics_yaml "$state/slskd.yml"
    start_metrics_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none
    wait_for_metrics_option "$base_url" false /metrics false slskd "$log"
    capture_metrics_stage "$base_url" "$suite" default /metrics
    stop_daemon

    write_metrics_yaml "$state/slskd.yml" true yaml-metrics true yaml-user yaml-pass
    start_metrics_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" lower none true
    wait_for_metrics_option "$base_url" true yaml-metrics true yaml-user "$log"
    capture_metrics_stage "$base_url" "$suite" yaml-over-environment /yaml-metrics /yaml-metrics yaml-user yaml-pass
    stop_daemon

    write_metrics_yaml "$state/slskd.yml" false yaml-disabled false yaml-user yaml-pass
    start_metrics_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none override true
    wait_for_metrics_option "$base_url" true cli-metrics true cli-user "$log"
    capture_metrics_stage "$base_url" "$suite" cli-over-yaml /cli-metrics /cli-metrics cli-user cli-pass
    stop_daemon

    write_metrics_yaml "$state/slskd.yml" true old-metrics true old-user old-pass
    start_metrics_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_metrics_option "$base_url" true old-metrics true old-user "$log"
    capture_metrics_stage "$base_url" "$suite" lifecycle-startup /old-metrics /old-metrics old-user old-pass
    write_metrics_yaml "$state/slskd.yml" true new-metrics false metrics-user metrics-pass
    wait_for_metrics_option "$base_url" true new-metrics false metrics-user "$log"
    capture_metrics_stage "$base_url" "$suite" lifecycle-watched /old-metrics /old-metrics old-user old-pass
    stop_daemon

    start_metrics_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_metrics_option "$base_url" true new-metrics false metrics-user "$log"
    capture_metrics_stage "$base_url" "$suite" lifecycle-restarted /new-metrics /new-metrics metrics-user metrics-pass
    capture_request "$suite" metrics-validation-parent-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics: null\n')"
    capture_request "$suite" metrics-validation-parent-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics: []\n')"
    capture_request "$suite" metrics-validation-enabled-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics:\n  enabled: nope\n')"
    capture_request "$suite" metrics-validation-url-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics:\n  url: [bad]\n')"
    capture_request "$suite" metrics-validation-auth-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics:\n  authentication: []\n')"
    capture_request "$suite" metrics-validation-disabled-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics:\n  authentication:\n    disabled: nope\n')"
    capture_request "$suite" metrics-validation-empty-credentials-disabled POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics:\n  enabled: true\n  authentication:\n    disabled: true\n    username: ""\n    password: ""\n')"
    capture_request "$suite" metrics-validation-empty-password-enabled POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'metrics:\n  enabled: true\n  authentication:\n    username: valid\n    password: ""\n')"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-metrics-upstream.normalized"
  local slskr_normalized="$work_dir/$target-metrics-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-metrics-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-metrics-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'metrics differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s metrics differential passed\n' "$target"
}

write_headless_yaml() {
  local path="$1"
  local headless="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ndht:\n  enabled: false\n' >"$temporary"
  if [[ "$headless" != __unset__ ]]; then
    printf 'headless: %s\n' "$headless" >>"$temporary"
  fi
  mv "$temporary" "$path"
}

start_headless_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_headless="$9"
  local command_line_headless="${10}"
  local append="${11:-false}"
  (
    unset SLSKD_HEADLESS
    [[ "$environment_headless" != __unset__ ]] && export SLSKD_HEADLESS="$environment_headless"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
    fi
    [[ "$command_line_headless" == true ]] && args+=(--headless)
    if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
  ) &
  daemon_pid="$!"
}

wait_for_headless_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 500); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["headless"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'headless differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'headless differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_headless_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local capture_runtime="$4"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" "$capture_runtime" >"$suite/headless-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def request(method,path,body=None):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    headers={"Content-Type":"application/json"} if body is not None else {}
    connection.request(method,path,body=body,headers=headers)
    response=connection.getresponse()
    payload=response.read()
    result={"status":response.status,"type":response.getheader("Content-Type","").split(";",1)[0].lower(),"empty":len(payload) == 0}
    connection.close()
    return result,payload
options=json.loads(request("GET","/api/v0/options")[1])
startup=json.loads(request("GET","/api/v0/options/startup")[1])
application=json.loads(request("GET","/api/v0/application")[1])
routes={}
if sys.argv[2] == "true":
    for path in ("/","/missing-client-route"):
        routes[path]=request("GET",path)[0]
    routes["/api/v0/application"]=request("GET","/api/v0/application")[0]
    routes["login"]=request("POST","/api/v0/session",'{"username":"slskd","password":"slskd"}')[0]
print(json.dumps({"current":options["headless"],"startup":startup["headless"],"pendingRestart":application["pendingRestart"],"routes":routes},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/headless-$stage.meta"
}

run_headless_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-headless-$implementation"
    local suite="$work_dir/$target-headless-$implementation"
    local log="$work_dir/$target-headless-$implementation.log"
    mkdir -p "$state" "$suite"

    write_headless_yaml "$state/slskd.yml"
    start_headless_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_headless_option "$base_url" false "$log"
    capture_headless_stage "$base_url" "$suite" default false
    stop_daemon

    write_headless_yaml "$state/slskd.yml" true
    start_headless_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false true
    wait_for_headless_option "$base_url" true "$log"
    capture_headless_stage "$base_url" "$suite" yaml-over-environment true
    stop_daemon

    write_headless_yaml "$state/slskd.yml" false
    start_headless_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true true
    wait_for_headless_option "$base_url" true "$log"
    capture_headless_stage "$base_url" "$suite" cli-over-yaml true
    stop_daemon

    write_headless_yaml "$state/slskd.yml" false
    start_headless_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    wait_for_headless_option "$base_url" false "$log"
    capture_headless_stage "$base_url" "$suite" lifecycle-startup false
    write_headless_yaml "$state/slskd.yml" true
    wait_for_headless_option "$base_url" true "$log"
    capture_headless_stage "$base_url" "$suite" lifecycle-watched false
    stop_daemon

    start_headless_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    wait_for_headless_option "$base_url" true "$log"
    capture_headless_stage "$base_url" "$suite" lifecycle-restarted true
    capture_request "$suite" headless-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'headless: null\n')"
    capture_request "$suite" headless-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'headless: nope\n')"
    capture_request "$suite" headless-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'headless: [true]\n')"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-headless-upstream.normalized"
  local slskr_normalized="$work_dir/$target-headless-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-headless-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-headless-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'headless differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s headless differential passed\n' "$target"
}

write_no_start_yaml() {
  local path="$1"
  local no_start="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n' >"$temporary"
  if [[ "$no_start" != __unset__ ]]; then
    printf '  no_start: %s\n' "$no_start" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_no_start_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_no_start="$9"
  local command_line_no_start="${10}"
  local append="${11:-false}"
  (
    unset SLSKD_NO_START
    [[ "$environment_no_start" != __unset__ ]] && export SLSKD_NO_START="$environment_no_start"
    export SLSKD_NO_VERSION_CHECK=true
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKR_PERSISTENCE_ENABLED=true
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
    fi
    [[ "$command_line_no_start" == true ]] && args+=(--no-start)
    if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
  ) &
  daemon_pid="$!"
}

wait_for_no_start_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["noStart"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'no-start differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'no-start differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_no_start_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/no-start-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse()
    body=response.read()
    connection.close()
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
application=get("/api/v0/application")
print(json.dumps({"current":options["flags"]["noStart"],"startup":startup["flags"]["noStart"],"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-start-$stage.meta"
}

capture_no_start_exit() {
  local suite="$1"
  local stage="$2"
  local log="$3"
  local base_url="$4"
  for _ in $(seq 1 600); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      break
    fi
    sleep 0.05
  done
  if kill -0 "$daemon_pid" 2>/dev/null; then
    printf 'no-start differential failed: process did not exit\n' >&2
    tail -120 "$log" >&2 || true
    exit 1
  fi
  local status=0
  wait "$daemon_pid" || status="$?"
  daemon_pid=""
  local listener=false
  if curl --silent --fail --max-time 1 "$base_url/api/v0/options" >/dev/null 2>&1; then
    listener=true
  fi
  local message=false
  if grep -Fq "Quitting because 'no-start' option is enabled" "$log"; then
    message=true
  fi
  "$python_bin" - "$status" "$listener" "$message" >"$suite/no-start-$stage.body" <<'PY'
import json,sys
print(json.dumps({"exit":int(sys.argv[1]),"listener":sys.argv[2] == "true","message":sys.argv[3] == "true"},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-start-$stage.meta"
}

run_no_start_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-start-$implementation"
    local suite="$work_dir/$target-no-start-$implementation"
    local log="$work_dir/$target-no-start-$implementation.log"
    mkdir -p "$state" "$suite"

    write_no_start_yaml "$state/slskd.yml"
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_start_option "$base_url" false "$log"
    capture_no_start_stage "$base_url" "$suite" default
    stop_daemon

    write_no_start_yaml "$state/slskd.yml" false
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false true
    wait_for_no_start_option "$base_url" false "$log"
    capture_no_start_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_no_start_yaml "$state/slskd.yml"
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false true
    capture_no_start_exit "$suite" environment-exit "$log" "$base_url"

    write_no_start_yaml "$state/slskd.yml" false
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true true
    capture_no_start_exit "$suite" cli-over-yaml-exit "$log" "$base_url"

    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    wait_for_no_start_option "$base_url" false "$log"
    capture_no_start_stage "$base_url" "$suite" lifecycle-startup
    write_no_start_yaml "$state/slskd.yml" true
    wait_for_no_start_option "$base_url" true "$log"
    capture_no_start_stage "$base_url" "$suite" lifecycle-watched
    capture_request "$suite" no-start-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_start: null\n')"
    capture_request "$suite" no-start-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_start: nope\n')"
    capture_request "$suite" no-start-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_start: [true]\n')"
    stop_daemon

    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    capture_no_start_exit "$suite" lifecycle-restarted "$log" "$base_url"
  done
  local upstream_normalized="$work_dir/$target-no-start-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-start-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-start-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-start-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-start differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-start differential passed\n' "$target"
}

write_no_logo_yaml() {
  local path="$1"
  local no_logo="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n' >"$temporary"
  if [[ "$no_logo" != __unset__ ]]; then
    printf '  no_logo: %s\n' "$no_logo" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_no_logo_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_no_logo="$9"
  local command_line_no_logo="${10}"
  (
    unset SLSKD_NO_LOGO
    [[ "$environment_no_logo" != __unset__ ]] && export SLSKD_NO_LOGO="$environment_no_logo"
    export SLSKD_NO_VERSION_CHECK=true
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
    fi
    [[ "$command_line_no_logo" == true ]] && args+=(--no-logo)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_no_logo_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["noLogo"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'no-logo differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'no-logo differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_no_logo_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local target="$4"
  local log="$5"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" "$target" "$log" >"$suite/no-logo-$stage.body" <<'PY'
import http.client,json,pathlib,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse()
    body=response.read()
    connection.close()
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
application=get("/api/v0/application")
log=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
target=sys.argv[2]
slskd_marker="This program is free software: you can redistribute it and/or modify"
slskdn_marker="GNU AFFERO GENERAL PUBLIC LICENSE"
version_marker="│ 0.0.0 (0.0.0)" if target == "slskd" else "│                     0.0.0 (0.0.0)"
print(json.dumps({
    "current":options["flags"]["noLogo"],
    "startup":startup["flags"]["noLogo"],
    "pendingRestart":application["pendingRestart"],
    "banner":slskd_marker in log if target == "slskd" else slskdn_marker in log,
    "otherBanner":slskdn_marker in log if target == "slskd" else slskd_marker in log,
    "version":version_marker in log,
    "development":"DEVELOPMENT" in log,
    "website":"https://slskd.org" in log,
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-logo-$stage.meta"
}

run_no_logo_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-logo-$implementation"
    local suite="$work_dir/$target-no-logo-$implementation"
    local log
    mkdir -p "$state" "$suite"

    write_no_logo_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-logo-$implementation-default.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_logo_option "$base_url" false "$log"
    capture_no_logo_stage "$base_url" "$suite" default "$target" "$log"
    stop_daemon

    write_no_logo_yaml "$state/slskd.yml" true
    log="$work_dir/$target-no-logo-$implementation-yaml.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" yaml-over-environment "$target" "$log"
    stop_daemon

    write_no_logo_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-logo-$implementation-environment.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" environment "$target" "$log"
    stop_daemon

    write_no_logo_yaml "$state/slskd.yml" false
    log="$work_dir/$target-no-logo-$implementation-cli.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" cli-over-yaml "$target" "$log"
    stop_daemon

    log="$work_dir/$target-no-logo-$implementation-lifecycle.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_logo_option "$base_url" false "$log"
    capture_no_logo_stage "$base_url" "$suite" lifecycle-startup "$target" "$log"
    write_no_logo_yaml "$state/slskd.yml" true
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" lifecycle-watched "$target" "$log"
    capture_request "$suite" no-logo-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_logo: null\n')"
    capture_request "$suite" no-logo-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_logo: nope\n')"
    capture_request "$suite" no-logo-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_logo: [true]\n')"
    stop_daemon

    log="$work_dir/$target-no-logo-$implementation-restarted.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" lifecycle-restarted "$target" "$log"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-no-logo-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-logo-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-logo-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-logo-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-logo differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-logo differential passed\n' "$target"
}

write_no_version_check_yaml() {
  local path="$1"
  local disabled="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n' >"$temporary"
  if [[ "$disabled" != __unset__ ]]; then
    printf '  no_version_check: %s\n' "$disabled" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_no_version_check_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8" environment_disabled="$9"
  local command_line_disabled="${10}"
  (
    unset SLSKD_NO_VERSION_CHECK
    [[ "$environment_disabled" != __unset__ ]] && export SLSKD_NO_VERSION_CHECK="$environment_disabled"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      mkdir -p "$root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
      export SLSKD_CONTENT_PATH=wwwroot
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo)
    fi
    [[ "$command_line_disabled" == true ]] && args+=(--no-version-check)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_no_version_check_option() {
  local base_url="$1" expected="$2" log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["noVersionCheck"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null; then return; fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
    sleep 0.05
  done
  printf 'no-version-check differential timed out waiting for %s\n' "$expected" >&2
  exit 1
}

capture_no_version_check_stage() {
  local base_url="$1" suite="$2" stage="$3" target="$4" log="$5" expect_initial="$6"
  "$python_bin" - "$base_url" "$target" "$log" "$expect_initial" >"$suite/no-version-check-$stage.body" <<'PY'
import http.client,json,pathlib,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    return json.loads(body)
options=get("/api/v0/options"); startup=get("/api/v0/options/startup")
application=get("/api/v0/application"); version=get("/api/v0/application/version/latest")
log=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
check_log="skip" if "Skipping version check for Development build" in log else "check" if "Checking GitHub Releases for latest version" in log else "none"
result={"current":options["flags"]["noVersionCheck"],"startup":startup["flags"]["noVersionCheck"],"pendingRestart":application["pendingRestart"],"checkLog":check_log,"full":version["full"],"currentVersion":version["current"],"isCanary":version["isCanary"],"isDevelopment":version["isDevelopment"]}
if sys.argv[4] == "true":
    result["versionKeys"]=sorted(version)
    result["latest"]=version.get("latest")
    result["latestTag"]=version.get("latestTag")
    result["latestUrl"]=version.get("latestUrl")
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-version-check-$stage.meta"
}

run_no_version_check_scenario() {
  local target="$1" root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-version-check-$implementation"
    local suite="$work_dir/$target-no-version-check-$implementation"
    local log
    mkdir -p "$state/wwwroot" "$suite"

    write_no_version_check_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-version-check-$implementation-default.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_version_check_option "$base_url" false "$log"
    capture_no_version_check_stage "$base_url" "$suite" default "$target" "$log" false
    stop_daemon

    write_no_version_check_yaml "$state/slskd.yml" true
    log="$work_dir/$target-no-version-check-$implementation-yaml.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" yaml-over-environment "$target" "$log" true
    stop_daemon

    write_no_version_check_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-version-check-$implementation-environment.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" environment "$target" "$log" true
    stop_daemon

    write_no_version_check_yaml "$state/slskd.yml" false
    log="$work_dir/$target-no-version-check-$implementation-cli.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" cli-over-yaml "$target" "$log" true
    stop_daemon

    log="$work_dir/$target-no-version-check-$implementation-lifecycle.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_version_check_option "$base_url" false "$log"
    capture_no_version_check_stage "$base_url" "$suite" lifecycle-startup "$target" "$log" false
    write_no_version_check_yaml "$state/slskd.yml" true
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" lifecycle-watched "$target" "$log" false
    capture_request "$suite" no-version-check-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_version_check: null\n')"
    capture_request "$suite" no-version-check-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_version_check: nope\n')"
    capture_request "$suite" no-version-check-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_version_check: [true]\n')"
    stop_daemon

    log="$work_dir/$target-no-version-check-$implementation-restarted.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" lifecycle-restarted "$target" "$log" true
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-no-version-check-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-version-check-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-version-check-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-version-check-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'no-version-check differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s no-version-check differential passed\n' "$target"
}

write_experimental_yaml() {
  local path="$1"
  local enabled="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n  no_version_check: true\n' >"$temporary"
  if [[ "$enabled" != __unset__ ]]; then
    printf '  experimental: %s\n' "$enabled" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_experimental_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8" environment_enabled="$9"
  local command_line_enabled="${10}"
  (
    unset SLSKD_EXPERIMENTAL
    [[ "$environment_enabled" != __unset__ ]] && export SLSKD_EXPERIMENTAL="$environment_enabled"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo)
    fi
    [[ "$command_line_enabled" == true ]] && args+=(--experimental)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_experimental_option() {
  local base_url="$1" expected="$2" log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["experimental"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null; then return; fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
    sleep 0.05
  done
  printf 'experimental differential timed out waiting for %s\n' "$expected" >&2
  exit 1
}

capture_experimental_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/experimental-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    return response.status,json.loads(body)
_,options=get("/api/v0/options")
_,startup=get("/api/v0/options/startup")
_,application=get("/api/v0/application")
server_status,server=get("/api/v0/server")
result={
    "current":options["flags"]["experimental"],
    "startup":startup["flags"]["experimental"],
    "pendingRestart":application["pendingRestart"],
    "serverStatus":server_status,
    "isConnected":server["isConnected"],
}
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/experimental-$stage.meta"
}

run_experimental_scenario() {
  local target="$1" root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-experimental-$implementation"
    local suite="$work_dir/$target-experimental-$implementation"
    local log
    mkdir -p "$state" "$suite"

    write_experimental_yaml "$state/slskd.yml"
    log="$work_dir/$target-experimental-$implementation-default.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_experimental_option "$base_url" false "$log"
    capture_experimental_stage "$base_url" "$suite" default
    stop_daemon

    write_experimental_yaml "$state/slskd.yml" true
    log="$work_dir/$target-experimental-$implementation-yaml.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_experimental_yaml "$state/slskd.yml"
    log="$work_dir/$target-experimental-$implementation-environment.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" environment
    stop_daemon

    write_experimental_yaml "$state/slskd.yml" false
    log="$work_dir/$target-experimental-$implementation-cli.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon

    log="$work_dir/$target-experimental-$implementation-lifecycle.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_experimental_option "$base_url" false "$log"
    capture_experimental_stage "$base_url" "$suite" lifecycle-startup
    write_experimental_yaml "$state/slskd.yml" true
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" lifecycle-watched
    capture_request "$suite" experimental-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  experimental: null\n')"
    capture_request "$suite" experimental-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  experimental: nope\n')"
    capture_request "$suite" experimental-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  experimental: [true]\n')"
    stop_daemon

    log="$work_dir/$target-experimental-$implementation-restarted.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" lifecycle-restarted
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-experimental-upstream.normalized"
  local slskr_normalized="$work_dir/$target-experimental-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-experimental-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-experimental-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'experimental differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s experimental differential passed\n' "$target"
}

write_case_sensitive_regex_yaml() {
  local path="$1"
  local enabled="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n  no_version_check: true\n' >"$temporary"
  if [[ "$enabled" != __unset__ ]]; then
    printf '  case_sensitive_reg_ex: %s\n' "$enabled" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_case_sensitive_regex_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8" environment_enabled="$9"
  local command_line_enabled="${10}"
  (
    unset SLSKD_CASE_SENSITIVE_REGEX
    [[ "$environment_enabled" != __unset__ ]] && export SLSKD_CASE_SENSITIVE_REGEX="$environment_enabled"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      mkdir -p "$root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
      export SLSKD_CONTENT_PATH=wwwroot
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo)
    fi
    [[ "$command_line_enabled" == true ]] && args+=(--case-sensitive-regex)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_case_sensitive_regex_option() {
  local base_url="$1" expected="$2" log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["caseSensitiveRegEx"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null; then return; fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
    sleep 0.05
  done
  printf 'case-sensitive-regex differential timed out waiting for %s\n' "$expected" >&2
  exit 1
}

capture_case_sensitive_regex_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/case-sensitive-regex-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
application=get("/api/v0/application")
result={
    "current":options["flags"]["caseSensitiveRegEx"],
    "startup":startup["flags"]["caseSensitiveRegEx"],
    "pendingRestart":application["pendingRestart"],
}
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/case-sensitive-regex-$stage.meta"
}

run_case_sensitive_regex_scenario() {
  local target="$1" root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-case-sensitive-regex-$implementation"
    local suite="$work_dir/$target-case-sensitive-regex-$implementation"
    local log
    mkdir -p "$state/wwwroot" "$suite"

    write_case_sensitive_regex_yaml "$state/slskd.yml"
    log="$work_dir/$target-case-sensitive-regex-$implementation-default.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_case_sensitive_regex_option "$base_url" false "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" default
    stop_daemon

    write_case_sensitive_regex_yaml "$state/slskd.yml" true
    log="$work_dir/$target-case-sensitive-regex-$implementation-yaml.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_case_sensitive_regex_yaml "$state/slskd.yml"
    log="$work_dir/$target-case-sensitive-regex-$implementation-environment.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" environment
    stop_daemon

    write_case_sensitive_regex_yaml "$state/slskd.yml" false
    log="$work_dir/$target-case-sensitive-regex-$implementation-cli.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon

    log="$work_dir/$target-case-sensitive-regex-$implementation-lifecycle.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_case_sensitive_regex_option "$base_url" false "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" lifecycle-startup
    write_case_sensitive_regex_yaml "$state/slskd.yml" true
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" lifecycle-watched
    capture_request "$suite" case-sensitive-regex-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  case_sensitive_reg_ex: null\n')"
    capture_request "$suite" case-sensitive-regex-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  case_sensitive_reg_ex: nope\n')"
    capture_request "$suite" case-sensitive-regex-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  case_sensitive_reg_ex: [true]\n')"
    capture_request "$suite" regex-validation-lookaround POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'shares:\n  filters:\n    - \'(?<=/)secret(?=\\.flac$)\'\n')"
    capture_request "$suite" regex-validation-numbered-backreference POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(secret)\\1$\'\n')"
    local named_backreference_yaml
    if [[ "$target" == slskd ]]; then
      named_backreference_yaml=$'transfers:\n  groups:\n    blacklisted:\n      patterns:\n        - \'^(?<stem>case)\\k<stem>peer$\'\n'
    else
      named_backreference_yaml=$'groups:\n  blacklisted:\n    patterns:\n      - \'^(?<stem>case)\\k<stem>peer$\'\n'
    fi
    capture_request "$suite" regex-validation-named-backreference POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$named_backreference_yaml")"
    capture_request "$suite" regex-validation-atomic-group POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(?>a|ab)c$\'\n')"
    capture_request "$suite" regex-validation-conditional POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(a)?b(?(1)c|d)$\'\n')"
    capture_request "$suite" regex-validation-assertion-conditional POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(?(?=secret)secret|public)$\'\n')"
    capture_request "$suite" regex-validation-explicit-capture POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(?n)^(secret)(?<stem>secret)\\k<stem>$\'\n')"
    capture_request "$suite" regex-validation-control-escape POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^\\cA$\'\n')"
    capture_request "$suite" regex-validation-z-anchor POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'secret\\Z\'\n')"
    capture_request "$suite" regex-validation-unicode-category POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^\\p{L}+$\'\n')"
    capture_request "$suite" regex-validation-basic-latin-block POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^\\p{IsBasicLatin}+$\'\n')"
    capture_request "$suite" regex-validation-invalid-braced-hex POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'\\x{41}\'\n')"
    capture_request "$suite" regex-validation-invalid-possessive POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'secret++\'\n')"
    capture_request "$suite" regex-validation-invalid-keep-out POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'secret\\Kvalue\'\n')"
    capture_request "$suite" regex-validation-invalid-recursion POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(?R)\'\n')"
    capture_request "$suite" regex-validation-invalid-backtracking-verb POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(*FAIL)\'\n')"
    capture_request "$suite" regex-validation-invalid-absent-expression POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(?~secret)\'\n')"
    capture_request "$suite" regex-validation-invalid-property-alias POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'\\p{Letter}\'\n')"
    capture_request "$suite" regex-validation-invalid-horizontal-space POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'\\h\'\n')"
    stop_daemon

    log="$work_dir/$target-case-sensitive-regex-$implementation-restarted.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" lifecycle-restarted
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-case-sensitive-regex-upstream.normalized"
  local slskr_normalized="$work_dir/$target-case-sensitive-regex-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-case-sensitive-regex-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-case-sensitive-regex-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'case-sensitive-regex differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s case-sensitive-regex differential passed\n' "$target"
}

write_regex_runtime_yaml() {
  local path="$1" share="$2" case_sensitive="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_version_check: true\n  force_share_scan: true\n  case_sensitive_reg_ex: %s\nshares:\n  directories:\n    - "[Probe]%s"\n  filters:\n    - '\''(?<=/)secret(?=\\.flac$)'\''\ndht:\n  enabled: false\n' \
    "$case_sensitive" "$share" >"$temporary"
  mv "$temporary" "$path"
}

start_regex_runtime_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8"
  (
    if [[ "$implementation" == upstream ]]; then
      mkdir -p "$root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      export SLSKD_CONTENT_PATH=wwwroot
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" \
        --slsk-listen-port "$listen_port" --no-logo
    fi
  ) >"$log" 2>&1 &
  daemon_pid="$!"
}

capture_regex_runtime_stage() {
  local target="$1" base_url="$2" suite="$3" stage="$4"
  "$python_bin" - "$target" "$base_url" >"$suite/regex-runtime-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
target=sys.argv[1]; url=urllib.parse.urlsplit(sys.argv[2])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=10)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200:
        raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
shares=get("/api/v0/shares")
probe=next(share for group in shares.values() for share in group if share.get("alias") == "Probe")
result={
    "current":options["flags"]["caseSensitiveRegEx"],
    "startup":startup["flags"]["caseSensitiveRegEx"],
    "filters":options["shares"]["filters"],
    "files":probe["files"],
}
if target == "slskdn":
    result["libraryMatches"]=len(get("/api/v0/library/items?query=SECRET")["items"])
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/regex-runtime-$stage.meta"
}

run_regex_runtime_scenario() {
  local target="$1" root="$2"
  local share="$work_dir/$target-regex-runtime-share"
  mkdir -p "$share"
  printf 'secret\n' >"$share/SECRET.flac"
  printf 'public\n' >"$share/PUBLIC.flac"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-regex-runtime-$implementation"
    local suite="$work_dir/$target-regex-runtime-$implementation"
    local log="$work_dir/$target-regex-runtime-$implementation.log"
    mkdir -p "$state/wwwroot" "$suite"

    write_regex_runtime_yaml "$state/slskd.yml" "$share" false
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-runtime-insensitive-options.json" "$log"
    wait_for_share_files "$base_url" Probe 1 "$log"
    capture_regex_runtime_stage "$target" "$base_url" "$suite" insensitive
    stop_daemon

    write_regex_runtime_yaml "$state/slskd.yml" "$share" true
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-runtime-sensitive-options.json" "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_regex_runtime_stage "$target" "$base_url" "$suite" sensitive
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-regex-runtime-upstream.normalized"
  local slskr_normalized="$work_dir/$target-regex-runtime-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-regex-runtime-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-regex-runtime-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'regex runtime differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s regex runtime differential passed\n' "$target"
}

write_regex_protocol_yaml() {
  local path="$1" target="$2" share="$3" case_sensitive="$4" server_port="$5"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: false\n  no_version_check: true\n  force_share_scan: true\n  case_sensitive_reg_ex: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: regex-probe\n  password: regex-probe\nshares:\n  directories:\n    - "[Probe]%s"\nfilters:\n  search:\n    request:\n      - '\''(?n)^(?(?=secret)(secret)(?<stem>secret)\\k<stem>|blocked\\Z)$'\''\n      - '\''^(?<named>a)(b)\\1\\2$'\''\n' \
    "$case_sensitive" "$server_port" "$share" >"$temporary"
  printf 'transfers:\n  groups:\n    blacklisted:\n      patterns:\n        - '\''^(?<stem>case)\\k<stem>peer$'\''\n' >>"$temporary"
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

write_regex_invariant_protocol_yaml() {
  local path="$1" share="$2" server_port="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: false\n  no_version_check: true\n  force_share_scan: true\n  case_sensitive_reg_ex: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: regex-probe\n  password: regex-probe\nshares:\n  directories:\n    - "[Probe]%s"\nfilters:\n  search:\n    request:\n      - '\''^s$'\''\n      - '\''^σ$'\''\n      - '\''^k$'\''\n      - '\''^ß$'\''\ndht:\n  enabled: false\n' \
    "$server_port" "$share" >"$temporary"
  mv "$temporary" "$path"
}

capture_regex_protocol_stage() {
  local target="$1" port="$2" suite="$3" stage="$4" fixture_status="$5"
  "$python_bin" - "$target" "$port" "$fixture_status" >"$suite/regex-protocol-$stage.body" <<'PY'
import json,pathlib,socket,struct,sys,time
target=sys.argv[1]
port=int(sys.argv[2])
status_path=pathlib.Path(sys.argv[3])

def string(value):
    data=value.encode("utf-8")
    return struct.pack("<I",len(data))+data

def frame_init(username):
    payload=string(username)+string("P")+struct.pack("<I",0)
    return struct.pack("<I",len(payload)+1)+b"\x01"+payload

def frame_search(query, token):
    payload=struct.pack("<I",token)+string(query)
    return struct.pack("<II",len(payload)+4,8)+payload

def recv_exact(sock, length):
    value=b""
    while len(value)<length:
        chunk=sock.recv(length-len(value))
        if not chunk:
            return None
        value+=chunk
    return value

def probe(username, query, token):
    deadline=time.monotonic()+10
    while True:
        try:
            sock=socket.create_connection(("127.0.0.1",port),timeout=0.5)
            break
        except OSError:
            if time.monotonic()>=deadline:
                return "connect-error"
            time.sleep(0.05)
    try:
        sock.settimeout(1.5)
        sock.sendall(frame_init(username)+frame_search(query,token))
        header=recv_exact(sock,4)
        if header is None:
            return False
        length=struct.unpack("<I",header)[0]
        body=recv_exact(sock,length)
        return body is not None and length>=4 and struct.unpack("<I",body[:4])[0]==9
    except (ConnectionError,TimeoutError,socket.timeout,OSError):
        return False
    finally:
        sock.close()

same_connection={
    "allowed":probe("AllowedPeer","SECRETSECRETSECRET",101),
    "filtered":probe("AllowedPeer","secretsecretsecret",102),
    "caseVariantBlacklist":probe("CaseCasePeer","SECRETSECRETSECRET",103),
    "exactBlacklist":probe("casecasepeer","SECRETSECRETSECRET",104),
    "dollarFinalNewlineFiltered":probe("AllowedPeer","secretsecretsecret\n",105),
    "zSingleNewlineFiltered":probe("AllowedPeer","blocked\n",106),
    "zMultipleNewlinesAllowed":probe("AllowedPeer","blocked\n\n",107),
    "mixedNumberingFiltered":probe("AllowedPeer","abba",108),
    "encounterOrderAllowed":probe("AllowedPeer","abab",109),
}

expected_requests=["AllowedPeer","CaseCasePeer","casecasepeer"] if target=="slskd" else ["AllowedPeer"]
deadline=time.monotonic()+20
status={}
while time.monotonic()<deadline:
    try:
        status=json.loads(status_path.read_text(encoding="utf-8"))
    except (OSError,json.JSONDecodeError):
        status={}
    requests=sorted(set(status.get("peer_address_requests",[])))
    if requests==expected_requests:
        break
    time.sleep(0.05)
value={
    "sameConnectionResponses":same_connection,
    "peerAddressRequests":sorted(set(status.get("peer_address_requests",[]))),
    "outboundSearchResponseTokens":sorted(set(status.get("peer_search_response_tokens",[]))),
    "target":target,
}
print(json.dumps(value,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/regex-protocol-$stage.meta"
}

capture_regex_invariant_protocol_stage() {
  local port="$1" suite="$2"
  "$python_bin" - "$port" >"$suite/regex-protocol-invariant.body" <<'PY'
import json,socket,struct,sys,time
port=int(sys.argv[1])
def string(value):
    data=value.encode("utf-8")
    return struct.pack("<I",len(data))+data
def frame_init(username):
    payload=string(username)+string("P")+struct.pack("<I",0)
    return struct.pack("<I",len(payload)+1)+b"\x01"+payload
def frame_search(query,token):
    payload=struct.pack("<I",token)+string(query)
    return struct.pack("<II",len(payload)+4,8)+payload
def recv_exact(sock,length):
    value=b""
    while len(value)<length:
        chunk=sock.recv(length-len(value))
        if not chunk: return None
        value+=chunk
    return value
def probe(query,token):
    deadline=time.monotonic()+10
    while True:
        try:
            sock=socket.create_connection(("127.0.0.1",port),timeout=0.5)
            break
        except OSError:
            if time.monotonic()>=deadline: return "connect-error"
            time.sleep(0.05)
    try:
        sock.settimeout(1.5)
        sock.sendall(frame_init("AllowedPeer")+frame_search(query,token))
        header=recv_exact(sock,4)
        if header is None: return False
        length=struct.unpack("<I",header)[0]
        body=recv_exact(sock,length)
        return body is not None and length>=4 and struct.unpack("<I",body[:4])[0]==9
    except (ConnectionError,TimeoutError,socket.timeout,OSError):
        return False
    finally:
        sock.close()
print(json.dumps({
    "longSAllowed":probe("ſ",201),
    "finalSigmaAllowed":probe("ς",202),
    "kelvinFiltered":probe("K",203),
    "capitalSharpSFiltered":probe("ẞ",204),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/regex-protocol-invariant.meta"
}

run_regex_protocol_scenario() {
  local target="$1" root="$2"
  local share="$work_dir/$target-regex-protocol-share"
  mkdir -p "$share"
  printf 'secret\n' >"$share/SECRET.flac"
  printf 'blocked\n' >"$share/BLOCKED.flac"
  printf 'abba\n' >"$share/ABBA.flac"
  printf 'abab\n' >"$share/ABAB.flac"
  printf 'long-s\n' >"$share/ſ.flac"
  printf 'final-sigma\n' >"$share/ς.flac"
  printf 'kelvin\n' >"$share/K.flac"
  printf 'sharp-s\n' >"$share/ẞ.flac"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)" server_port="$(pick_free_port)"
    local peer_regular_port="$(pick_free_port)" peer_obfuscated_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-regex-protocol-$implementation"
    local suite="$work_dir/$target-regex-protocol-$implementation"
    local log="$work_dir/$target-regex-protocol-$implementation.log"
    local fixture_status="$work_dir/$target-regex-protocol-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-regex-protocol-$implementation-fixture.log"
    mkdir -p "$state/wwwroot" "$suite"

    start_soulseek_peer_fixture "$server_port" "$fixture_status" "$fixture_log" \
      "$peer_regular_port" "$peer_obfuscated_port" regular-only
    write_regex_protocol_yaml "$state/slskd.yml" "$target" "$share" true "$server_port"
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-protocol-options.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_share_files "$base_url" Probe 8 "$log"
    capture_regex_protocol_stage "$target" "$listen_port" "$suite" sensitive "$fixture_status"
    stop_daemon

    write_regex_invariant_protocol_yaml "$state/slskd.yml" "$share" "$server_port"
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-protocol-invariant-options.json" "$log"
    wait_for_share_files "$base_url" Probe 8 "$log"
    capture_regex_invariant_protocol_stage "$listen_port" "$suite"
    stop_daemon
    stop_soulseek_fixture
  done
  local upstream_normalized="$work_dir/$target-regex-protocol-upstream.normalized"
  local slskr_normalized="$work_dir/$target-regex-protocol-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-regex-protocol-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-regex-protocol-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'regex protocol differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s regex protocol differential passed\n' "$target"
}

write_share_scan_flags_yaml() {
  local path="$1"
  local share_path="$2"
  local no_share_scan="${3:-false}"
  local force_share_scan="${4:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: %s\n  force_share_scan: %s\nshares:\n  directories:\n    - "[Probe]%s"\ndht:\n  enabled: false\n' \
    "$no_share_scan" "$force_share_scan" "$share_path" >"$temporary"
  mv "$temporary" "$path"
}

start_share_scan_flags_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_mode="$9"
  local command_line_mode="${10}"
  local append="${11:-false}"
  (
    unset SLSKD_NO_SHARE_SCAN SLSKD_FORCE_SHARE_SCAN
    if [[ "$environment_mode" == false ]]; then
      export SLSKD_NO_SHARE_SCAN=false SLSKD_FORCE_SHARE_SCAN=false
    fi
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKR_PERSISTENCE_ENABLED=true
      args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
    fi
    [[ "$command_line_mode" == force ]] && args+=(--force-share-scan)
    if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
  ) &
  daemon_pid="$!"
}

wait_for_share_scan_flags() {
  local base_url="$1"
  local expected_no="$2"
  local expected_force="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; f=json.load(sys.stdin)["flags"]; raise SystemExit(0 if f["noShareScan"] == (sys.argv[1] == "true") and f["forceShareScan"] == (sys.argv[2] == "true") else 1)' \
        "$expected_no" "$expected_force" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'share scan flags differential failed: daemon exited while waiting for %s/%s\n' "$expected_no" "$expected_force" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'share scan flags differential failed: timed out waiting for %s/%s\n' "$expected_no" "$expected_force" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_share_scan_flags_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/share-scan-flags-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse()
    body=response.read()
    connection.close()
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
application=get("/api/v0/application")
shares=get("/api/v0/shares")
probe=next(share for host in shares.values() for share in host if share.get("alias") == "Probe")
selected=lambda flags:{"noShareScan":flags["noShareScan"],"forceShareScan":flags["forceShareScan"]}
share_state=application["shares"]
print(json.dumps({
    "current":selected(options["flags"]),"startup":selected(startup["flags"]),
    "pendingRestart":application["pendingRestart"],
    "share":{"hasFiles":"files" in probe,"files":probe.get("files"),"hasDirectories":"directories" in probe,"directories":probe.get("directories")},
    "state":dict(
        {key:share_state[key] for key in ("scanPending","scanning","ready","faulted","cancelled","scanProgress","directories","files")},
        hostsPresent="hosts" in share_state,
        hostCount=len(share_state.get("hosts") or []),
    ),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/share-scan-flags-$stage.meta"
}

run_share_scan_flags_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-share-scan-flags-$implementation"
    local share_path="$state/share"
    local suite="$work_dir/$target-share-scan-flags-$implementation"
    local log="$work_dir/$target-share-scan-flags-$implementation.log"
    mkdir -p "$state" "$share_path" "$suite"
    printf 'first\n' >"$share_path/first.txt"

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" false false
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none
    wait_for_share_scan_flags "$base_url" false false "$log"
    wait_for_share_files "$base_url" Probe 1 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" initial-scan
    stop_daemon

    printf 'second\n' >"$share_path/second.txt"
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" false false "$log"
    wait_for_share_files "$base_url" Probe 1 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" cached-startup
    stop_daemon

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" false true
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false none true
    wait_for_share_scan_flags "$base_url" false true "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" yaml-force-over-environment
    stop_daemon

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" true true
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" true true "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" no-scan-wins-conflict
    stop_daemon

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" false false
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none force true
    wait_for_share_scan_flags "$base_url" false true "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" cli-force-over-yaml
    stop_daemon

    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" false false "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" lifecycle-startup
    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" true true
    wait_for_share_scan_flags "$base_url" true true "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" lifecycle-watched
    stop_daemon

    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" true true "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" lifecycle-restarted
    capture_request "$suite" share-scan-flags-validation-parent-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags: null\n')"
    capture_request "$suite" share-scan-flags-validation-parent-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags: []\n')"
    capture_request "$suite" share-scan-flags-validation-no-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_share_scan: null\n')"
    capture_request "$suite" share-scan-flags-validation-no-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_share_scan: nope\n')"
    capture_request "$suite" share-scan-flags-validation-no-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_share_scan: [true]\n')"
    capture_request "$suite" share-scan-flags-validation-force-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  force_share_scan: null\n')"
    capture_request "$suite" share-scan-flags-validation-force-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  force_share_scan: nope\n')"
    capture_request "$suite" share-scan-flags-validation-force-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  force_share_scan: [true]\n')"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-share-scan-flags-upstream.normalized"
  local slskr_normalized="$work_dir/$target-share-scan-flags-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-share-scan-flags-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-share-scan-flags-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'share scan flags differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s share scan flags differential passed\n' "$target"
}

write_instance_yaml() {
  local path="$1"
  local instance_yaml="$2"
  local temporary="$path.tmp"
  printf 'instance_name: %s\nremote_configuration: true\nflags:\n  no_connect: true\ndht:\n  enabled: false\n' \
    "$instance_yaml" >"$temporary"
  mv "$temporary" "$path"
}

start_instance_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local environment_name="$8"
  local command_line_name="$9"
  local append="${10:-false}"
  local redirection='>'
  [[ "$append" == true ]] && redirection='>>'

  (
    unset SLSKD_INSTANCE_NAME SLSKR_INSTANCE_NAME
    if [[ "$environment_name" != __unset__ ]]; then
      export SLSKD_INSTANCE_NAME="$environment_name"
    fi
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      local args=(dotnet "$dll")
      if [[ "$command_line_name" != __unset__ ]]; then
        args+=(-i "$command_line_name")
      fi
      if [[ "$redirection" == '>>' ]]; then
        exec "${args[@]}" >>"$log" 2>&1
      else
        exec "${args[@]}" >"$log" 2>&1
      fi
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      local args=("$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port")
      if [[ "$command_line_name" != __unset__ ]]; then
        args+=(-i "$command_line_name")
      fi
      if [[ "$redirection" == '>>' ]]; then
        exec "${args[@]}" >>"$log" 2>&1
      else
        exec "${args[@]}" >"$log" 2>&1
      fi
    fi
  ) &
  daemon_pid="$!"
}

wait_for_instance_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 400); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value.get("instanceName") == sys.argv[1] else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'instance-name differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'instance-name differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_instance_stage() {
  local implementation="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local log="$5"
  local expected_diagnostic="$6"
  local forbidden_diagnostic="${7:-__none__}"
  local options_file="$suite/instance-$stage-options.raw"
  local startup_file="$suite/instance-$stage-startup.raw"
  local application_file="$suite/instance-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$options_file"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup_file"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application_file"
  "$python_bin" - "$options_file" "$startup_file" "$application_file" "$log" \
    "$implementation" "$expected_diagnostic" "$forbidden_diagnostic" \
    >"$suite/instance-$stage.body" <<'PY'
import json,sys
options=json.load(open(sys.argv[1], encoding="utf-8"))
startup=json.load(open(sys.argv[2], encoding="utf-8"))
application=json.load(open(sys.argv[3], encoding="utf-8"))
log=open(sys.argv[4], encoding="utf-8", errors="replace").read().splitlines()
implementation=sys.argv[5]
expected=sys.argv[6]
forbidden=sys.argv[7]
def diagnostic(name):
    if implementation == "upstream":
        return any(f"Instance Name: {name}" in line for line in log)
    return any(f" instance {name} listening " in line for line in log)
print(json.dumps({
    "current": options.get("instanceName"),
    "startup": startup.get("instanceName"),
    "pendingRestart": application.get("pendingRestart"),
    "startupDiagnosticMatches": True if expected == "__skip__" else diagnostic(expected),
    "forbiddenStartupDiagnosticAbsent": True if forbidden == "__none__" else not diagnostic(forbidden),
}, sort_keys=True, separators=(",", ":")))
PY
  rm -f "$options_file" "$startup_file" "$application_file"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/instance-$stage.meta"
}

run_instance_name_scenario() {
  local target="$1"
  local root="$2"
  local long_name
  long_name="$($python_bin -c 'print("a" * 300)')"
  local long_control_name=$'line one\n'"$long_name"
  local long_control_yaml
  long_control_yaml="$($python_bin -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$long_control_name")"

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-instance-$implementation"
    local suite="$work_dir/$target-instance-$implementation"
    local log="$work_dir/$target-instance-$implementation.log"
    mkdir -p "$state" "$suite"

    write_instance_yaml "$state/slskd.yml" '"yaml-wins-environment"'
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" environment-loses __unset__
    wait_for_options "$base_url" "$work_dir/$target-instance-$implementation-yaml-precedence.json" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" yaml-precedence "$log" yaml-wins-environment
    stop_daemon

    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" environment-loses cli-wins true
    wait_for_options "$base_url" "$work_dir/$target-instance-$implementation-cli-precedence.json" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" cli-precedence "$log" cli-wins
    stop_daemon

    write_instance_yaml "$state/slskd.yml" '"watched-old"'
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" __unset__ __unset__ true
    wait_for_options "$base_url" "$work_dir/$target-instance-$implementation-lifecycle.json" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" lifecycle-startup "$log" watched-old

    write_instance_yaml "$state/slskd.yml" 123
    wait_for_instance_option "$base_url" 123 "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" lifecycle-watched-numeric "$log" watched-old 123

    write_instance_yaml "$state/slskd.yml" '""'
    wait_for_instance_option "$base_url" default "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" lifecycle-watched-empty "$log" watched-old default
    stop_daemon

    write_instance_yaml "$state/slskd.yml" "$long_control_yaml"
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" __unset__ __unset__ true
    wait_for_instance_option "$base_url" "$long_control_name" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" long-control-restarted "$log" __skip__
    stop_daemon

    write_instance_yaml "$state/slskd.yml" '""'
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" __unset__ __unset__ true
    wait_for_instance_option "$base_url" default "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" empty-restarted "$log" default

    capture_request "$suite" instance-validation-null POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: null\n')"
    capture_request "$suite" instance-validation-empty POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: ""\n')"
    capture_request "$suite" instance-validation-number POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: 123\n')"
    capture_request "$suite" instance-validation-bool POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: true\n')"
    capture_request "$suite" instance-validation-array POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: [one, two]\n')"
    capture_request "$suite" instance-validation-object POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name:\n  nested: value\n')"
    capture_request "$suite" instance-validation-long-control POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload "instance_name: $long_control_yaml"$'\n')"
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-instance-upstream.normalized"
  local slskr_normalized="$work_dir/$target-instance-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-instance-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-instance-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'instance-name differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s instance-name differential passed\n' "$target"
}

write_completed_template_yaml() {
  local path="$1"
  local template="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: true\ntransfers:\n  download:\n    completed_path_template: "%s"\n' \
    "$template" >"$temporary"
  mv "$temporary" "$path"
}

start_completed_template_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  local environment_template="${7:-}"
  local cli_template="${8:-}"
  local cli_args=()
  [[ -n "$cli_template" ]] && cli_args+=(--download-completed-path-template "$cli_template")
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_template" ]] && export SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE="$environment_template"
      exec dotnet "$dll" "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_template" ]] && export SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE="$environment_template"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_for_completed_template() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value["global"]["download"]["completedPathTemplate"] == sys.argv[1] else 1)' \
        "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'completed-template differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'completed-template differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_completed_template_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "template-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "template-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "template-debug-$stage" "$base_url/api/v0/options/debug"
  capture_get "$suite" "template-application-$stage" "$base_url/api/v0/application"
}

run_completed_template_scenario() {
  local root="$1"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-template-$implementation"
    local suite="$work_dir/slskdn-template-$implementation"
    local log="$work_dir/slskdn-template-$implementation.log"
    mkdir -p "$state" "$suite"

    write_completed_template_yaml "$state/slskd.yml" 'yaml/{uploader}'
    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" 'environment/{uploader}'
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-yaml.json" "$log"
    wait_for_completed_template "$base_url" 'yaml/{uploader}' "$log"
    capture_completed_template_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" 'environment/{uploader}' 'cli/{remote_folder}' true
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-cli.json" "$log"
    wait_for_completed_template "$base_url" 'cli/{remote_folder}' "$log"
    capture_completed_template_stage "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    write_completed_template_yaml "$state/slskd.yml" 'startup/{uploader}'
    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" true
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-startup.json" "$log"
    wait_for_completed_template "$base_url" 'startup/{uploader}' "$log"
    capture_completed_template_stage "$base_url" "$suite" lifecycle-startup

    write_completed_template_yaml "$state/slskd.yml" 'watched/{remote_folder}'
    wait_for_completed_template "$base_url" 'watched/{remote_folder}' "$log"
    capture_completed_template_stage "$base_url" "$suite" lifecycle-watched

    for validation in \
      'completed-template-validation-string|transfers:\n  download:\n    completed_path_template: "valid/{uploader}"\n' \
      'completed-template-validation-empty|transfers:\n  download:\n    completed_path_template: ""\n' \
      'completed-template-validation-null|transfers:\n  download:\n    completed_path_template: null\n' \
      'completed-template-validation-number|transfers:\n  download:\n    completed_path_template: 123\n' \
      'completed-template-validation-array|transfers:\n  download:\n    completed_path_template: [one, two]\n' \
      'completed-template-validation-object|transfers:\n  download:\n    completed_path_template:\n      nested: value\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon

    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" true
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-restarted.json" "$log"
    wait_for_completed_template "$base_url" 'watched/{remote_folder}' "$log"
    capture_completed_template_stage "$base_url" "$suite" lifecycle-restarted
    stop_daemon
  done

  local upstream_normalized="$work_dir/slskdn-template-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-template-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-template-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-template-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'completed-template differential failed for slskdN\n' >&2
    exit 1
  fi
  printf 'slskdn completed-template differential passed\n'
}

write_private_message_auto_response_yaml() {
  local path="$1"
  local no_connect="$2"
  local server_port="$3"
  local listen_port="$4"
  local enabled="$5"
  local message="$6"
  local cooldown="$7"
  local temporary="$path.tmp"
  local message_value="\"$message\""
  [[ "$message" == __NULL__ ]] && message_value=null
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n  private_message_auto_response:\n    enabled: %s\n    message: %s\n    cooldown_minutes: %s\n' \
    "$no_connect" "$server_port" "$listen_port" "$enabled" "$message_value" "$cooldown" \
    >"$temporary"
  mv "$temporary" "$path"
}

start_private_message_auto_response_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  local environment_enabled="${7:-}"
  local environment_message="${8:-}"
  local environment_cooldown="${9:-}"
  local cli_enabled="${10:-false}"
  local cli_message="${11:-}"
  local cli_cooldown="${12:-}"
  local cli_args=()
  [[ "$cli_enabled" == true ]] && cli_args+=(--slsk-private-message-auto-response)
  [[ -n "$cli_message" ]] && cli_args+=(--slsk-private-message-auto-response-message "$cli_message")
  [[ -n "$cli_cooldown" ]] && cli_args+=(--slsk-private-message-auto-response-cooldown-minutes "$cli_cooldown")
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE="$environment_enabled"
      [[ -n "$environment_message" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE="$environment_message"
      [[ -n "$environment_cooldown" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES="$environment_cooldown"
      exec dotnet "$dll" "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE="$environment_enabled"
      [[ -n "$environment_message" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE="$environment_message"
      [[ -n "$environment_cooldown" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES="$environment_cooldown"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

start_private_message_fixture() {
  local port="$1"
  local status="$2"
  local log="$3"
  local injection="$4"
  "$python_bin" "$repo_root/scripts/fixture-soulseek-listener.py" \
    127.0.0.1 "$port" "$status" login-success-private "$injection" >"$log" 2>&1 &
  soulseek_fixture_pid="$!"
  for _ in $(seq 1 100); do
    [[ -s "$status" ]] && return
    if ! kill -0 "$soulseek_fixture_pid" 2>/dev/null; then
      printf 'private-message auto-response fixture exited\n' >&2
      cat "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'private-message auto-response fixture did not become ready\n' >&2
  exit 1
}

wait_for_private_message_auto_response_options() {
  local base_url="$1"
  local enabled="$2"
  local message="$3"
  local cooldown="$4"
  local log="$5"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["soulseek"]["privateMessageAutoResponse"]; message=None if sys.argv[2] == "__NULL__" else sys.argv[2]; raise SystemExit(0 if value == {"enabled": sys.argv[1] == "true", "message": message, "cooldownMinutes": int(sys.argv[3])} else 1)' \
        "$enabled" "$message" "$cooldown" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'private-message auto-response daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'private-message auto-response timed out waiting for options\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_private_message_fixture() {
  local status="$1"
  local expected_injected="$2"
  local expected_responses="$3"
  local expected_message="${4:-}"
  local daemon_log="$5"
  for _ in $(seq 1 600); do
    if "$python_bin" - "$status" "$expected_injected" "$expected_responses" "$expected_message" <<'PY'
import json,sys
try:
    value=json.load(open(sys.argv[1], encoding="utf-8"))
except (FileNotFoundError, json.JSONDecodeError):
    raise SystemExit(1)
injected=value.get("injected_private_message_ids", [])
responses=value.get("private_message_responses", [])
expected_injected=int(sys.argv[2])
expected_responses=int(sys.argv[3])
expected_message=sys.argv[4]
valid=expected_injected in injected and len(responses) == expected_responses
if valid and expected_message:
    valid=responses[-1].get("message") == expected_message
raise SystemExit(0 if valid else 1)
PY
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'private-message auto-response daemon exited while waiting for fixture\n' >&2
      tail -120 "$daemon_log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'private-message auto-response timed out waiting for fixture\n' >&2
  tail -120 "$daemon_log" >&2 || true
  exit 1
}

capture_private_message_auto_response_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local fixture_status="${4:-}"
  capture_get "$suite" "auto-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "auto-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "auto-application-$stage" "$base_url/api/v0/application"
  if [[ -n "$fixture_status" ]]; then
    cp "$fixture_status" "$suite/auto-fixture-$stage.body"
  fi
}

run_private_message_auto_response_scenario() {
  local root="$1"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-auto-response-$implementation"
    local suite="$work_dir/slskdn-auto-response-$implementation"
    local log="$work_dir/slskdn-auto-response-$implementation.log"
    local fixture_status="$work_dir/slskdn-auto-response-$implementation-fixture.json"
    local fixture_log="$work_dir/slskdn-auto-response-$implementation-fixture.log"
    local injection="$work_dir/slskdn-auto-response-$implementation-injection.json"
    mkdir -p "$state" "$suite"

    write_private_message_auto_response_yaml "$state/slskd.yml" true "$server_port" "$listen_port" true 'yaml response' 15
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false 'environment response' 30
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-yaml.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" true 'yaml response' 15 "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_private_message_auto_response_yaml "$state/slskd.yml" true "$server_port" "$listen_port" false 'yaml response' 15
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false 'environment response' 30 true 'cli response' 45
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-cli.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" true 'cli response' 45 "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    printf '[]\n' >"$injection"
    start_private_message_fixture "$server_port" "$fixture_status" "$fixture_log" "$injection"
    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true 'startup response' 15
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-startup.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" true 'startup response' 15 "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 1 1 'startup response' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-startup "$fixture_status"

    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 2 1 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 2 1 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-cooldown "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true 'watched response' 20
    wait_for_private_message_auto_response_options "$base_url" true 'watched response' 20 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 3 2 'watched response' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-watched "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true '   ' 20
    wait_for_private_message_auto_response_options "$base_url" true '   ' 20 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"},{"id":4,"username":"FixtureThree","message":"Are you human?"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 4 2 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 4 2 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-blank "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true __NULL__ 20
    wait_for_private_message_auto_response_options "$base_url" true "Hi, I'm human and testing a slskdN client. Shares may be temporarily unavailable while I validate the client." 20 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"},{"id":4,"username":"FixtureThree","message":"Are you human?"},{"id":5,"username":"FixtureFour","message":"Please prove you are not a bot"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 5 3 "Hi, I'm human and testing a slskdN client. Shares may be temporarily unavailable while I validate the client." "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-null "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" false 'disabled response' 25
    wait_for_private_message_auto_response_options "$base_url" false 'disabled response' 25 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"},{"id":4,"username":"FixtureThree","message":"Are you human?"},{"id":5,"username":"FixtureFour","message":"Please prove you are not a bot"},{"id":6,"username":"FixtureFive","message":"Human verification challenge"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 6 3 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 6 3 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-disabled "$fixture_status"

    for validation in \
      'auto-validation-valid|soulseek:\n  private_message_auto_response:\n    enabled: true\n    message: response\n    cooldown_minutes: 1\n' \
      'auto-validation-disabled|soulseek:\n  private_message_auto_response:\n    enabled: false\n    message: response\n    cooldown_minutes: 1440\n' \
      'auto-validation-enabled-string|soulseek:\n  private_message_auto_response:\n    enabled: nope\n' \
      'auto-validation-message-empty|soulseek:\n  private_message_auto_response:\n    message: ""\n' \
      'auto-validation-message-null|soulseek:\n  private_message_auto_response:\n    message: null\n' \
      'auto-validation-message-number|soulseek:\n  private_message_auto_response:\n    message: 123\n' \
      'auto-validation-message-array|soulseek:\n  private_message_auto_response:\n    message: [response]\n' \
      'auto-validation-message-object|soulseek:\n  private_message_auto_response:\n    message:\n      nested: response\n' \
      'auto-validation-cooldown-zero|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: 0\n' \
      'auto-validation-cooldown-high|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: 1441\n' \
      'auto-validation-cooldown-numeric-string|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: "15"\n' \
      'auto-validation-cooldown-string|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon
    stop_soulseek_fixture

    rm -f "$fixture_status"
    printf '[]\n' >"$injection"
    start_private_message_fixture "$server_port" "$fixture_status" "$fixture_log" "$injection"
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-restarted.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" false 'disabled response' 25 "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    printf '[{"id":7,"username":"FixtureSix","message":"Are you human?"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 7 0 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 7 0 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-restarted "$fixture_status"
    stop_daemon
    stop_soulseek_fixture
  done

  local upstream_normalized="$work_dir/slskdn-auto-response-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-auto-response-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-response-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-response-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'private-message auto-response differential failed for slskdN\n' >&2
    exit 1
  fi
  printf 'slskdn private-message auto-response differential passed\n'
}

write_download_auto_retry_yaml() {
  local path="$1"
  local no_connect="$2"
  local server_port="$3"
  local listen_port="$4"
  local enabled="$5"
  local retry_delay="$6"
  local check_interval="$7"
  local max_attempts="$8"
  local max_files="$9"
  local max_files_per_peer="${10}"
  local peer_cooldown="${11}"
  local alternate_sources="${12}"
  local max_searches="${13}"
  local tolerance="${14}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\ntransfers:\n  download:\n    auto_retry:\n      enabled: %s\n      retry_delay_seconds: %s\n      check_interval_seconds: %s\n      max_attempts: %s\n      max_files_per_cycle: %s\n      max_files_per_peer_per_cycle: %s\n      peer_cooldown_seconds: %s\n      alternate_sources_enabled: %s\n      max_alternate_source_searches_per_cycle: %s\n      alternate_source_size_tolerance_percent: %s\n' \
    "$no_connect" "$server_port" "$listen_port" "$enabled" "$retry_delay" \
    "$check_interval" "$max_attempts" "$max_files" "$max_files_per_peer" \
    "$peer_cooldown" "$alternate_sources" "$max_searches" "$tolerance" \
    >"$temporary"
  mv "$temporary" "$path"
}

seed_download_auto_retry_failures() {
  local implementation="$1"
  local state="$2"
  if [[ "$implementation" == upstream ]]; then
    sqlite3 "$state/data/transfers.db" <<'SQL'
INSERT INTO Transfers (Id, RequestId, Username, Direction, Filename, Size, StartOffset, BatchId, DestinationDirectory, LocalFilename, Attempts, NextAttemptAt, State, StateDescription, RequestedAt, UpdatedAt, EnqueuedAt, StartedAt, EndedAt, BytesTransferred, AverageSpeed, PlaceInQueue, Exception, BitRate, SampleRate, BitDepth, Length, Artist, Album, Title, TrackNumber, Year, Removed)
VALUES
('00000000-0000-0000-0000-000000000001', NULL, 'fixture-peer-a', 'Download', 'Remote/A-First.flac', 1000, 0, NULL, NULL, NULL, 1, NULL, 144, 'Completed, TimedOut', datetime('now','-40 seconds'), datetime('now','-40 seconds'), NULL, NULL, datetime('now','-40 seconds'), 0, 0, NULL, 'timed out', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0),
('00000000-0000-0000-0000-000000000002', NULL, 'fixture-peer-a', 'Download', 'Remote/A-Second.flac', 1000, 0, NULL, NULL, NULL, 1, NULL, 144, 'Completed, TimedOut', datetime('now','-39 seconds'), datetime('now','-39 seconds'), NULL, NULL, datetime('now','-39 seconds'), 0, 0, NULL, 'timed out', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0),
('00000000-0000-0000-0000-000000000003', NULL, 'fixture-peer-b', 'Download', 'Remote/B-First.flac', 1000, 0, NULL, NULL, NULL, 1, NULL, 144, 'Completed, TimedOut', datetime('now','-38 seconds'), datetime('now','-38 seconds'), NULL, NULL, datetime('now','-38 seconds'), 0, 0, NULL, 'timed out', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0);
SQL
  else
    "$python_bin" - "$state/transfer-state.json" <<'PY'
import json,sys,time
now=int(time.time())
entries=[]
for ident,peer,name,age in [
    (1,"fixture-peer-a","Remote/A-First.flac",40),
    (2,"fixture-peer-a","Remote/A-Second.flac",39),
    (3,"fixture-peer-b","Remote/B-First.flac",38),
]:
    entries.append({
        "id":ident,"direction":0,"token":ident,"peer_username":peer,
        "filename":name,"local_path":None,"batch_id":None,"request_id":None,
        "request_name":None,"destination_directory":None,"bit_rate":None,
        "sample_rate":None,"bit_depth":None,"length_seconds":None,"artist":None,
        "album":None,"title":None,"track_number":None,"year":None,"size":1000,
        "bytes_transferred":0,"status":"failed","reason":"timed out",
        "requested_at":now-age,"started_at":None,"start_offset":0,
        "updated_at":now-age,"updated_at_ms":0,
    })
with open(sys.argv[1],"w",encoding="utf-8") as handle:
    json.dump({"version":1,"entries":entries},handle,separators=(",",":"))
PY
  fi
}

wait_for_download_auto_retry_options() {
  local base_url="$1"
  local enabled="$2"
  local tolerance="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["global"]["download"]["autoRetry"]; expected={"enabled":sys.argv[1]=="true","retryDelaySeconds":10,"checkIntervalSeconds":10,"maxAttempts":1,"maxFilesPerCycle":2,"maxFilesPerPeerPerCycle":1,"peerCooldownSeconds":60,"alternateSourcesEnabled":False,"maxAlternateSourceSearchesPerCycle":0,"alternateSourceSizeTolerancePercent":float(sys.argv[2])}; raise SystemExit(0 if value==expected else 1)' \
        "$enabled" "$tolerance" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for watched options\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_download_auto_retry_defaults() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["global"]["download"]["autoRetry"]; expected={"enabled":True,"retryDelaySeconds":1800,"checkIntervalSeconds":300,"maxAttempts":5,"maxFilesPerCycle":10,"maxFilesPerPeerPerCycle":1,"peerCooldownSeconds":900,"alternateSourcesEnabled":True,"maxAlternateSourceSearchesPerCycle":1,"alternateSourceSizeTolerancePercent":5}; raise SystemExit(0 if value==expected else 1)' 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for null/default binding\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for null/default binding\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_download_auto_retry_count() {
  local implementation="$1"
  local state="$2"
  local expected="$3"
  local log="$4"
  for _ in $(seq 1 300); do
    local count=''
    if [[ "$implementation" == upstream ]]; then
      count="$(sqlite3 "$state/data/transfers.db" 'SELECT COUNT(*) FROM Transfers;' 2>/dev/null || true)"
    else
      count="$($python_bin - "$state/transfer-state.json" 2>/dev/null <<'PY' || true
import json,sys
print(len(json.load(open(sys.argv[1],encoding="utf-8"))["entries"]))
PY
)"
    fi
    [[ "$count" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for %s transfers\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for %s transfers\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_download_auto_retry_requests() {
  local fixture_status="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 300); do
    if "$python_bin" - "$fixture_status" "$expected" 2>/dev/null <<'PY'
import json,sys
fixture=json.load(open(sys.argv[1],encoding="utf-8"))
requests=fixture.get("peer_address_requests",[])
expected=int(sys.argv[2])
expected_peers=["fixture-peer-a","fixture-peer-b"]
raise SystemExit(0 if len(requests)==expected and sorted(requests)==expected_peers else 1)
PY
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for %s peer requests\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for %s peer requests\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_download_auto_retry_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local current="$suite/auto-retry-$stage-current.raw"
  local startup="$suite/auto-retry-$stage-startup.raw"
  local application="$suite/auto-retry-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application"
  "$python_bin" - "$current" "$startup" "$application" >"$suite/auto-retry-$stage.body" <<'PY'
import json,sys
current=json.load(open(sys.argv[1],encoding="utf-8"))["global"]["download"]["autoRetry"]
startup=json.load(open(sys.argv[2],encoding="utf-8"))["global"]["download"]["autoRetry"]
application=json.load(open(sys.argv[3],encoding="utf-8"))
print(json.dumps({"current":current,"startup":startup,"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/auto-retry-$stage.meta"
  rm -f "$current" "$startup" "$application"
}

capture_download_auto_retry_behavior() {
  local fixture_status="$1"
  local suite="$2"
  "$python_bin" - "$fixture_status" >"$suite/auto-retry-behavior.body" <<'PY'
import json,sys
fixture=json.load(open(sys.argv[1],encoding="utf-8"))
all_requests=fixture.get("peer_address_requests",[])
requests=sorted(set(all_requests))
print(json.dumps({
    "initialFailures":3,
    "retryCount":len(all_requests),
    "boundedToTwo":len(all_requests)==2,
    "contactedPeers":requests,
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/auto-retry-behavior.meta"
}

run_download_auto_retry_scenario() {
  local root="$1"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local peer_regular_port="$(pick_free_port)"
    local peer_obfuscated_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-auto-retry-$implementation"
    local suite="$work_dir/slskdn-auto-retry-$implementation"
    local log="$work_dir/slskdn-auto-retry-$implementation.log"
    local fixture_status="$work_dir/slskdn-auto-retry-$implementation-fixture.json"
    local fixture_log="$work_dir/slskdn-auto-retry-$implementation-fixture.log"
    mkdir -p "$state" "$suite"

    write_download_auto_retry_yaml "$state/slskd.yml" true "$server_port" "$listen_port" \
      false 10 10 1 2 1 60 false 0 5.5
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-auto-retry-$implementation-initialize.json" "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" startup-disabled
    stop_daemon
    seed_download_auto_retry_failures "$implementation" "$state"

    start_soulseek_peer_fixture "$server_port" "$fixture_status" "$fixture_log" \
      "$peer_regular_port" "$peer_obfuscated_port" regular-only
    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      false 10 10 1 2 1 60 false 0 5.5
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/slskdn-auto-retry-$implementation-lifecycle.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    sleep 1
    wait_for_download_auto_retry_count "$implementation" "$state" 3 "$log"

    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      true 10 10 1 2 1 60 false 0 5.5
    wait_for_download_auto_retry_options "$base_url" true 5.5 "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" watched-enabled
    wait_for_download_auto_retry_requests "$fixture_status" 2 "$log"
    sleep 1
    wait_for_download_auto_retry_requests "$fixture_status" 2 "$log"
    sleep 11
    wait_for_download_auto_retry_requests "$fixture_status" 2 "$log"
    capture_download_auto_retry_behavior "$fixture_status" "$suite"

    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      null null null null null null null null null null
    wait_for_download_auto_retry_defaults "$base_url" "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" watched-null-defaults

    for validation in \
      'auto-retry-validation-valid|transfers:\n  download:\n    auto_retry:\n      enabled: true\n      retry_delay_seconds: 10\n      check_interval_seconds: 10\n      max_attempts: 0\n      max_files_per_cycle: 1\n      max_files_per_peer_per_cycle: 1\n      peer_cooldown_seconds: 60\n      alternate_sources_enabled: false\n      max_alternate_source_searches_per_cycle: 0\n      alternate_source_size_tolerance_percent: 5.5\n' \
      'auto-retry-validation-strings|transfers:\n  download:\n    auto_retry:\n      enabled: "true"\n      retry_delay_seconds: "10"\n      alternate_source_size_tolerance_percent: "5.5"\n' \
      'auto-retry-validation-null-defaults|transfers:\n  download:\n    auto_retry:\n      enabled: null\n      max_attempts: null\n      max_alternate_source_searches_per_cycle: null\n      alternate_source_size_tolerance_percent: null\n' \
      'auto-retry-validation-null-invalid|transfers:\n  download:\n    auto_retry:\n      retry_delay_seconds: null\n' \
      'auto-retry-validation-object-null|transfers:\n  download:\n    auto_retry: null\n' \
      'auto-retry-validation-object-scalar|transfers:\n  download:\n    auto_retry: true\n' \
      'auto-retry-validation-delay-low|transfers:\n  download:\n    auto_retry:\n      retry_delay_seconds: 9\n' \
      'auto-retry-validation-interval-high|transfers:\n  download:\n    auto_retry:\n      check_interval_seconds: 3601\n' \
      'auto-retry-validation-attempts-high|transfers:\n  download:\n    auto_retry:\n      max_attempts: 101\n' \
      'auto-retry-validation-files-zero|transfers:\n  download:\n    auto_retry:\n      max_files_per_cycle: 0\n' \
      'auto-retry-validation-peer-files-high|transfers:\n  download:\n    auto_retry:\n      max_files_per_peer_per_cycle: 21\n' \
      'auto-retry-validation-cooldown-low|transfers:\n  download:\n    auto_retry:\n      peer_cooldown_seconds: 59\n' \
      'auto-retry-validation-searches-high|transfers:\n  download:\n    auto_retry:\n      max_alternate_source_searches_per_cycle: 11\n' \
      'auto-retry-validation-enabled-invalid|transfers:\n  download:\n    auto_retry:\n      enabled: nope\n' \
      'auto-retry-validation-tolerance-frozen-low|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: -0.5\n' \
      'auto-retry-validation-tolerance-frozen-high|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: 100.5\n' \
      'auto-retry-validation-tolerance-low|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: -0.5001\n' \
      'auto-retry-validation-tolerance-high|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: 100.5001\n' \
      'auto-retry-validation-tolerance-text|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done

    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      false 10 10 1 2 1 60 false 0 5.5
    wait_for_download_auto_retry_options "$base_url" false 5.5 "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" watched-disabled
    stop_daemon
    stop_soulseek_fixture

    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/slskdn-auto-retry-$implementation-restarted.json" "$log"
    wait_for_download_auto_retry_options "$base_url" false 5.5 "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" restarted-disabled
    stop_daemon
  done

  local upstream_normalized="$work_dir/slskdn-auto-retry-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-auto-retry-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-retry-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-retry-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'slskdn download auto-retry differential failed\n' >&2
    exit 1
  fi
  printf 'slskdn download auto-retry differential passed\n'
}

write_blacklist_yaml() {
  local path="$1"
  local server_port="$2"
  local listen_port="$3"
  local share_dir="$4"
  local enabled="$5"
  local blacklist_file="$6"
  local no_connect="${7:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nblacklist:\n  enabled: %s\n  file: "%s"\nshares:\n  directories:\n    - "%s"\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$no_connect" "$enabled" "$blacklist_file" "$share_dir" "$server_port" "$listen_port" \
    >"$temporary"
  mv "$temporary" "$path"
}

start_blacklist_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local environment_enabled="${8:-}"
  local environment_file="${9:-}"
  local cli_enabled="${10:-false}"
  local cli_file="${11:-}"
  local append="${12:-false}"
  local cli_args=()
  [[ "$cli_enabled" == true ]] && cli_args+=(--enable-blacklist)
  [[ -n "$cli_file" ]] && cli_args+=(--blacklist-file "$cli_file")
  local redirect='>'
  [[ "$append" == true ]] && redirect='>>'
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$redirect" == '>>' ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
        [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
        [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$redirect" == '>>' ]]; then
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
      [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
      [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_for_blacklist_option() {
  local base_url="$1"
  local enabled="$2"
  local file="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["blacklist"]; raise SystemExit(0 if value.get("enabled") == (sys.argv[1] == "true") and value.get("file", "") == sys.argv[2] else 1)' "$enabled" "$file" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'blacklist differential failed: daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'blacklist differential failed: timed out waiting for enabled=%s file=%s\n' "$enabled" "$file" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_blacklist_listener() {
  local listen_port="$1"
  local log="$2"
  local consecutive=0
  for _ in $(seq 1 600); do
    if ss -H -ltn "sport = :$listen_port" | rg -q '^LISTEN'; then
      consecutive=$((consecutive + 1))
      [[ "$consecutive" -ge 10 ]] && return
    else
      consecutive=0
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'blacklist differential failed: daemon exited while waiting for peer listener\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'blacklist differential failed: peer listener did not stabilize on 127.0.0.1:%s\n' "$listen_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_blacklist_peer_suite() {
  local suite="$1"
  local label="$2"
  local listen_port="$3"
  local expected_blacklisted="$4"
  mkdir -p "$suite"
  local captured=false
  for _ in $(seq 1 300); do
    if "$python_bin" - "$listen_port" "$expected_blacklisted" \
      >"$suite/$label.body.tmp" 2>"$suite/$label.error" <<'PY'
import json
import socket
import struct
import sys
import time
import zlib

port = int(sys.argv[1])
expected = sys.argv[2] == "true"
probe_username = f"blacklist-probe-{time.time_ns()}"

def string(value):
    encoded = value.encode("utf-8")
    return struct.pack("<I", len(encoded)) + encoded

def receive_exact(connection, length):
    value = b""
    while len(value) < length:
        chunk = connection.recv(length - len(value))
        if not chunk:
            raise EOFError
        value += chunk
    return value

def request(code, payload=b"", timeout=1.5):
    connection = socket.create_connection(("127.0.0.1", port), timeout=2)
    connection.settimeout(timeout)
    initialization = string(probe_username) + string("P") + struct.pack("<I", 0)
    connection.sendall(struct.pack("<I", len(initialization) + 1) + b"\x01" + initialization)
    connection.sendall(struct.pack("<II", len(payload) + 4, code) + payload)
    try:
        length = struct.unpack("<I", receive_exact(connection, 4))[0]
        frame = receive_exact(connection, length)
        return struct.unpack("<I", frame[:4])[0], frame[4:]
    except (socket.timeout, EOFError):
        return None
    finally:
        connection.close()

def read_string(payload, offset):
    length = struct.unpack_from("<I", payload, offset)[0]
    offset += 4
    value = payload[offset:offset + length].decode("utf-8")
    return value, offset + length

user_info = request(15)
restricted_user_info = False
if user_info and user_info[0] == 16:
    payload = user_info[1]
    _, offset = read_string(payload, 0)
    has_picture = payload[offset] != 0
    offset += 1
    if has_picture:
        picture_length = struct.unpack_from("<I", payload, offset)[0]
        offset += 4 + picture_length
    _, queue_size = struct.unpack_from("<II", payload, offset)
    slots_free = payload[offset + 8] != 0
    restricted_user_info = queue_size == 2_147_483_647 and not slots_free

browse = request(4)
browse_empty = bool(
    browse and browse[0] == 5 and zlib.decompress(browse[1]) == bytes(12)
)

search = request(8, struct.pack("<I", 123) + string("fixture")) if expected else None
search_suppressed = expected and search is None

folder = request(36, struct.pack("<I", 124) + string("share"))
folder_empty = False
if folder and folder[0] == 37:
    try:
        payload = zlib.decompress(folder[1])
        token = struct.unpack_from("<I", payload, 0)[0]
        outer, offset = read_string(payload, 4)
        directory_count = struct.unpack_from("<I", payload, offset)[0]
        offset += 4
        inner, offset = read_string(payload, offset)
        file_count = struct.unpack_from("<I", payload, offset)[0]
        folder_empty = token == 124 and outer == "share" and directory_count == 1 and inner == "share" and file_count == 0
    except (IndexError, UnicodeDecodeError, struct.error, zlib.error):
        folder_empty = False

transfer = request(40, struct.pack("<II", 0, 125) + string("share\\fixture.txt"))
file_not_shared = False
if transfer and transfer[0] == 41:
    payload = transfer[1]
    allowed = payload[4] != 0
    if not allowed:
        reason, _ = read_string(payload, 5)
        file_not_shared = reason == "File not shared."

result = {
    "browseEmpty": browse_empty,
    "fileNotShared": file_not_shared,
    "folderEmpty": folder_empty,
    "restrictedUserInfo": restricted_user_info,
    "searchSuppressed": search_suppressed,
}
peer_checks = {key: value for key, value in result.items() if key != "searchSuppressed"}
if any(value != expected for value in peer_checks.values()) or (expected and not search_suppressed):
    raise SystemExit(f"blacklist peer behavior mismatch: expected={expected} actual={result}")
print(json.dumps(result, sort_keys=True, separators=(",", ":")))
PY
    then
      mv "$suite/$label.body.tmp" "$suite/$label.body"
      captured=true
      break
    fi
    sleep 0.1
  done
  if [[ "$captured" != true ]]; then
    printf '%s: ' "$label" >&2
    cat "$suite/$label.error" >&2
    rm -f "$suite/$label.body.tmp" "$suite/$label.error"
    return 1
  fi
  rm -f "$suite/$label.error"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/$label.meta"
}

capture_blacklist_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "blacklist-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "blacklist-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "blacklist-application-$stage" "$base_url/api/v0/application"
}

run_blacklist_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-blacklist-$implementation"
    local suite="$work_dir/$target-blacklist-$implementation"
    local log="$work_dir/$target-blacklist-$implementation.log"
    local fixture_status="$work_dir/$target-blacklist-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-blacklist-$implementation-fixture.log"
    local share_dir="$state/share"
    local cidr_file="$state/blacklist-cidr.txt"
    local p2p_file="$state/blacklist-p2p.txt"
    local dat_file="$state/blacklist-dat.txt"
    mkdir -p "$state" "$suite" "$share_dir"
    printf 'fixture-data' >"$share_dir/fixture.txt"
    printf '127.0.0.1/32\n' >"$cidr_file"
    printf 'loopback:127.0.0.1-127.0.0.1\n' >"$p2p_file"
    printf '127.000.000.001 - 127.000.000.001 , 000 , loopback\n' >"$dat_file"

    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success
    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" false "$cidr_file"
    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-startup.json" "$log"
    wait_for_blacklist_option "$base_url" false "$cidr_file" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    wait_for_share_files "$base_url" share 1 "$log"
    capture_blacklist_stage "$base_url" "$suite" startup-disabled
    capture_blacklist_peer_suite "$suite" peer-startup-disabled "$listen_port" false

    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" true "$cidr_file"
    wait_for_blacklist_option "$base_url" true "$cidr_file" "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_stage "$base_url" "$suite" watched-cidr
    capture_blacklist_peer_suite "$suite" peer-watched-cidr "$listen_port" true

    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" false "$p2p_file"
    wait_for_blacklist_option "$base_url" false "$p2p_file" "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_peer_suite "$suite" peer-watched-disabled "$listen_port" false
    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" true "$p2p_file"
    wait_for_blacklist_option "$base_url" true "$p2p_file" "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_stage "$base_url" "$suite" watched-p2p
    capture_blacklist_peer_suite "$suite" peer-watched-p2p "$listen_port" true

    for validation in \
      "blacklist-validation-valid-cidr|blacklist:\n  enabled: true\n  file: '$cidr_file'\n" \
      "blacklist-validation-valid-p2p|blacklist:\n  enabled: true\n  file: '$p2p_file'\n" \
      "blacklist-validation-valid-dat|blacklist:\n  enabled: true\n  file: '$dat_file'\n" \
      "blacklist-validation-disabled|blacklist:\n  enabled: false\n" \
      "blacklist-validation-enabled-null|blacklist:\n  enabled: null\n" \
      "blacklist-validation-file-null|blacklist:\n  enabled: true\n  file: null\n" \
      "blacklist-validation-enabled-text|blacklist:\n  enabled: nope\n  file: '$cidr_file'\n" \
      "blacklist-validation-parent-null|blacklist: null\n" \
      "blacklist-validation-parent-array|blacklist: []\n" \
      "blacklist-validation-missing|blacklist:\n  enabled: true\n  file: '$state/missing.txt'\n"
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon
    stop_soulseek_fixture

    rm -f "$fixture_status"
    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success
    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" false "" true
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-restart.json" "$log"
    wait_for_blacklist_option "$base_url" true "$p2p_file" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_stage "$base_url" "$suite" restarted
    capture_blacklist_peer_suite "$suite" peer-restarted "$listen_port" true
    stop_daemon
    stop_soulseek_fixture

    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" false "$cidr_file" true
    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true "$p2p_file"
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-precedence-yaml.json" "$log"
    wait_for_blacklist_option "$base_url" false "$cidr_file" "$log"
    capture_blacklist_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false "$cidr_file" true "$p2p_file" true
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-precedence-cli.json" "$log"
    wait_for_blacklist_option "$base_url" true "$p2p_file" "$log"
    capture_blacklist_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-blacklist-upstream.normalized"
  local slskr_normalized="$work_dir/$target-blacklist-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-blacklist-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-blacklist-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'blacklist differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s blacklist differential passed\n' "$target"
}

write_transfer_groups_yaml() {
  local path="$1"
  local upload_slots="$2"
  local default_priority="$3"
  local friend_priority="$4"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ntransfers:\n  upload:\n    slots: %s\n    speed_limit: 1200\n    limits:\n      queued:\n        files: 50\n        megabytes: 500\n      daily: ~\n      weekly:\n        failures: 8\n  groups:\n    default:\n      upload:\n        priority: %s\n        strategy: firstinfirstout\n        slots: 9\n        limits:\n          queued:\n            files: 25\n    leechers:\n      thresholds:\n        files: 4\n        directories: 2\n      upload:\n        priority: 90\n        strategy: roundrobin\n        slots: 1\n        speed_limit: 100\n    blacklisted:\n      members: [blocked-user]\n      patterns: ["^evil-"]\n      cidrs: [192.0.2.0/24]\n    user_defined:\n      friends:\n        upload:\n          priority: %s\n          strategy: firstinfirstout\n          slots: 7\n          limits:\n            queued:\n              files: 100\n        members: [friend, ally]\n' \
    "$upload_slots" "$default_priority" "$friend_priority" >"$temporary"
  mv "$temporary" "$path"
}

capture_transfer_groups_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local current="$work_dir/$target-transfer-groups-$stage-current-$$.json"
  local startup="$work_dir/$target-transfer-groups-$stage-startup-$$.json"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
  "$python_bin" - "$target" "$current" "$startup" >"$suite/groups-$stage.body" <<'PY'
import json,sys
target,current_path,startup_path=sys.argv[1:]
current=json.load(open(current_path,encoding="utf-8"))
startup=json.load(open(startup_path,encoding="utf-8"))
def project(value):
    if target == "slskdn":
        return {"upload":value["global"]["upload"],"limits":value["global"]["limits"],"groups":value["groups"]}
    return {"upload":value["transfers"]["upload"],"groups":value["transfers"]["groups"]}
print(json.dumps({"current":project(current),"startup":project(startup)},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/groups-$stage.meta"
  rm -f "$current" "$startup"
  capture_get "$suite" "group-member-$stage" "$base_url/api/v0/users/friend/group"
  capture_get "$suite" "group-blacklisted-$stage" "$base_url/api/v0/users/blocked-user/group"
  capture_get "$suite" "groups-application-$stage" "$base_url/api/v0/application"
}

run_transfer_groups_scenario() {
  local target="$1"
  local root="$2"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-transfer-groups-$implementation"
    local suite="$work_dir/$target-transfer-groups-$implementation"
    local log="$work_dir/$target-transfer-groups-$implementation.log"
    mkdir -p "$state" "$suite"
    write_transfer_groups_yaml "$state/slskd.yml" 20 20 5
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target" SLSKD_NO_AUTH=true
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port"
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-transfer-groups-$implementation-options.json" "$log"
    capture_transfer_groups_stage "$target" "$base_url" "$suite" startup

    for validation in \
      $'valid|transfers:\n  upload:\n    slots: 10\n    speed_limit: 100\n  groups:\n    default:\n      upload:\n        priority: 1\n        strategy: roundrobin\n        slots: 2\n' \
      $'daily-null|transfers:\n  upload:\n    limits:\n      daily: null\n' \
      $'invalid-upload-slots|transfers:\n  upload:\n    slots: 0\n' \
      $'invalid-priority|transfers:\n  groups:\n    default:\n      upload:\n        priority: 0\n' \
      $'invalid-strategy|transfers:\n  groups:\n    default:\n      upload:\n        strategy: invalid\n' \
      $'invalid-limit|transfers:\n  groups:\n    default:\n      upload:\n        limits:\n          queued:\n            files: 0\n' \
      $'duplicate-membership|transfers:\n  groups:\n    user_defined:\n      first:\n        members: [same-user]\n      second:\n        members: [same-user]\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "groups-validation-$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$("$python_bin" -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml")"
    done

    write_transfer_groups_yaml "$state/slskd.yml" 21 30 4
    local watched=false
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
        | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); target=sys.argv[1]; slots=value["global"]["upload"]["slots"] if target == "slskdn" else value["transfers"]["upload"]["slots"]; raise SystemExit(0 if slots == 21 else 1)' "$target" 2>/dev/null
      then
        watched=true
        break
      fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then
        printf 'transfer groups differential failed: daemon exited during watched reload\n' >&2
        tail -120 "$log" >&2 || true
        exit 1
      fi
      sleep 0.1
    done
    if [[ "$watched" != true ]]; then
      printf 'transfer groups differential failed: watched options did not update\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    capture_transfer_groups_stage "$target" "$base_url" "$suite" watched
    stop_daemon

    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target" SLSKD_NO_AUTH=true
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port"
      ) >>"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-transfer-groups-$implementation-restarted-options.json" "$log"
    capture_transfer_groups_stage "$target" "$base_url" "$suite" restarted
    stop_daemon
  done

  normalize_directory_suite "$work_dir/$target-transfer-groups-upstream" "$work_dir/$target-transfer-groups-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-transfer-groups-slskr" "$work_dir/$target-transfer-groups-slskr.normalized"
  if ! diff -ru "$work_dir/$target-transfer-groups-upstream.normalized" "$work_dir/$target-transfer-groups-slskr.normalized"; then
    printf 'transfer groups differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s transfer groups differential passed\n' "$target"
}

write_transfer_download_yaml() {
  local path="$1" target="$2" slots="$3" speed="$4" variant="$5"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ntransfers:\n  download:\n    slots: %s\n    speed_limit: %s\n' "$slots" "$speed" >"$temporary"
  if [[ "$target" == slskd ]]; then
    printf '    retry:\n      partial: %s\n      attempts: %s\n      delay: %s\n      max_delay: %s\n    destination:\n      subdirectory: "Music/${SOURCE_USERNAME}"\n      exists: overwrite\n      permissions:\n        mode: "0750"\n' \
      "$( [[ "$variant" == startup ]] && printf overwrite || printf resume )" \
      "$( [[ "$variant" == startup ]] && printf 4 || printf 5 )" \
      "$( [[ "$variant" == startup ]] && printf 1200 || printf 1300 )" \
      "$( [[ "$variant" == startup ]] && printf 31000 || printf 32000 )" >>"$temporary"
  else
    printf '    retry:\n      incomplete: %s\n      attempts: %s\n      delay: %s\n      max_delay: %s\n    completed_layout: %s\n    auto_replace_stuck: %s\n    auto_replace_threshold: %s\n    auto_replace_interval: %s\n' \
      "$( [[ "$variant" == startup ]] && printf overwrite || printf resume )" \
      "$( [[ "$variant" == startup ]] && printf 4 || printf 5 )" \
      "$( [[ "$variant" == startup ]] && printf 1200 || printf 1300 )" \
      "$( [[ "$variant" == startup ]] && printf 31000 || printf 32000 )" \
      "$( [[ "$variant" == startup ]] && printf uploader_folder || printf batch_id )" \
      "$( [[ "$variant" == startup ]] && printf true || printf false )" \
      "$( [[ "$variant" == startup ]] && printf 7.5 || printf 8.5 )" \
      "$( [[ "$variant" == startup ]] && printf 90 || printf 100 )" >>"$temporary"
  fi
  mv "$temporary" "$path"
}

start_transfer_download_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8"
  if [[ "$implementation" == upstream ]]; then
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target" SLSKD_NO_AUTH=true
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

capture_transfer_download_stage() {
  local target="$1" base_url="$2" suite="$3" stage="$4"
  "$python_bin" - "$target" "$base_url" >"$suite/download-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
target=sys.argv[1]; url=urllib.parse.urlsplit(sys.argv[2])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200: raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
current=get("/api/v0/options"); startup=get("/api/v0/options/startup"); application=get("/api/v0/application")
def project(value):
    return value["global"]["download"] if target == "slskdn" else value["transfers"]["download"]
print(json.dumps({"current":project(current),"startup":project(startup),"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/download-$stage.meta"
}

run_transfer_download_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-transfer-download-$implementation"
    local suite="$work_dir/$target-transfer-download-$implementation"
    local log="$work_dir/$target-transfer-download-$implementation.log"
    mkdir -p "$state" "$suite"
    write_transfer_download_yaml "$state/slskd.yml" "$target" 4 777 startup
    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-transfer-download-$implementation-options.json" "$log"
    capture_transfer_download_stage "$target" "$base_url" "$suite" startup

    local common_valid=$'transfers:\n  download:\n    slots: 2\n    speed_limit: 3\n    retry:\n      attempts: 2\n      delay: 5000\n      max_delay: 60000\n'
    capture_request "$suite" download-validation-valid POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$common_valid")"
    capture_request "$suite" download-validation-slots-zero POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    slots: 0\n')"
    capture_request "$suite" download-validation-speed-zero POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    speed_limit: 0\n')"
    capture_request "$suite" download-validation-attempts-zero POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    retry:\n      attempts: 0\n')"
    if [[ "$target" == slskd ]]; then
      capture_request "$suite" download-validation-strategy POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    retry:\n      partial: invalid\n')"
      capture_request "$suite" download-validation-mode POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    destination:\n      permissions:\n        mode: "999"\n')"
      capture_request "$suite" download-validation-traversal POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    destination:\n      subdirectory: ../escape\n')"
    else
      capture_request "$suite" download-validation-strategy POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    retry:\n      incomplete: invalid\n')"
      capture_request "$suite" download-validation-threshold POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    auto_replace_threshold: 0\n')"
      capture_request "$suite" download-validation-interval POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    auto_replace_interval: 9\n')"
    fi

    write_transfer_download_yaml "$state/slskd.yml" "$target" 5 778 watched
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); target=sys.argv[1]; value=value["global"]["download"] if target == "slskdn" else value["transfers"]["download"]; raise SystemExit(0 if value["speedLimit"] == 778 else 1)' "$target" 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/application" | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["pendingRestart"] else 1)' 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    capture_transfer_download_stage "$target" "$base_url" "$suite" watched
    stop_daemon

    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-transfer-download-$implementation-restarted-options.json" "$log"
    capture_transfer_download_stage "$target" "$base_url" "$suite" restarted
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-transfer-download-upstream" "$work_dir/$target-transfer-download-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-transfer-download-slskr" "$work_dir/$target-transfer-download-slskr.normalized"
  if ! diff -ru "$work_dir/$target-transfer-download-upstream.normalized" "$work_dir/$target-transfer-download-slskr.normalized"; then
    printf 'transfer download differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s transfer download differential passed\n' "$target"
}

write_soulseek_connection_yaml() {
  local path="$1" variant="$2"
  local read write transfer queue connect inactivity transfer_timeout proxy_port username password
  if [[ "$variant" == startup ]]; then
    read=2048; write=3072; transfer=81920; queue=5
    connect=1000; inactivity=2000; transfer_timeout=30000
    proxy_port=1080; username=proxy-user; password=proxy-secret
  else
    read=4096; write=5120; transfer=98304; queue=6
    connect=1100; inactivity=2100; transfer_timeout=31000
    proxy_port=1081; username=watched-user; password=watched-secret
  fi
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\nsoulseek:\n  connection:\n    buffer:\n      read: %s\n      write: %s\n      transfer: %s\n      write_queue: %s\n    timeout:\n      connect: %s\n      inactivity: %s\n      transfer: %s\n    proxy:\n      enabled: true\n      address: 127.0.0.1\n      port: %s\n      username: %s\n      password: %s\n' \
    "$read" "$write" "$transfer" "$queue" "$connect" "$inactivity" "$transfer_timeout" \
    "$proxy_port" "$username" "$password" >"$temporary"
  mv "$temporary" "$path"
}

capture_soulseek_connection_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/connection-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200: raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
current=get("/api/v0/options"); startup=get("/api/v0/options/startup"); application=get("/api/v0/application")
print(json.dumps({"current":current["soulseek"]["connection"],"startup":startup["soulseek"]["connection"],"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/connection-$stage.meta"
}

run_soulseek_connection_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-soulseek-connection-$implementation"
    local suite="$work_dir/$target-soulseek-connection-$implementation"
    local log="$work_dir/$target-soulseek-connection-$implementation.log"
    mkdir -p "$state" "$suite"
    write_soulseek_connection_yaml "$state/slskd.yml" startup
    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-connection-$implementation-options.json" "$log"
    capture_soulseek_connection_stage "$base_url" "$suite" startup

    for validation in \
      $'valid|soulseek:\n  connection:\n    buffer:\n      read: 1024\n      write: 1024\n      transfer: 81920\n      write_queue: 5\n    timeout:\n      connect: 1000\n      inactivity: 1000\n      transfer: 30000\n' \
      $'read-low|soulseek:\n  connection:\n    buffer:\n      read: 1023\n' \
      $'write-low|soulseek:\n  connection:\n    buffer:\n      write: 1023\n' \
      $'transfer-buffer-low|soulseek:\n  connection:\n    buffer:\n      transfer: 81919\n' \
      $'write-queue-low|soulseek:\n  connection:\n    buffer:\n      write_queue: 4\n' \
      $'connect-timeout-low|soulseek:\n  connection:\n    timeout:\n      connect: 999\n' \
      $'inactivity-timeout-low|soulseek:\n  connection:\n    timeout:\n      inactivity: 999\n' \
      $'transfer-timeout-low|soulseek:\n  connection:\n    timeout:\n      transfer: 29999\n' \
      $'proxy-missing-endpoint|soulseek:\n  connection:\n    proxy:\n      enabled: true\n' \
      $'proxy-valid|soulseek:\n  connection:\n    proxy:\n      enabled: true\n      address: 127.0.0.1\n      port: 1080\n      username: user\n      password: secret\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "connection-validation-$label" POST \
        "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$yaml")"
    done

    write_soulseek_connection_yaml "$state/slskd.yml" watched
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
        | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["soulseek"]["connection"]["buffer"]["read"] == 4096 else 1)' 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    capture_soulseek_connection_stage "$base_url" "$suite" watched
    stop_daemon

    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-connection-$implementation-restarted-options.json" "$log"
    capture_soulseek_connection_stage "$base_url" "$suite" restarted
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-soulseek-connection-upstream" "$work_dir/$target-soulseek-connection-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-soulseek-connection-slskr" "$work_dir/$target-soulseek-connection-slskr.normalized"
  if ! diff -ru "$work_dir/$target-soulseek-connection-upstream.normalized" "$work_dir/$target-soulseek-connection-slskr.normalized"; then
    printf 'Soulseek connection differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s Soulseek connection differential passed\n' "$target"
}

write_soulseek_profile_distributed_yaml() {
  local path="$1" picture="$2" variant="$3"
  local diagnostic disabled disable_children child_limit logging
  if [[ "$variant" == startup ]]; then
    diagnostic=trace; disabled=false; disable_children=false; child_limit=3; logging=true
  else
    diagnostic=debug; disabled=false; disable_children=true; child_limit=2; logging=false
  fi
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\nsoulseek:\n  picture: "%s"\n  diagnostic_level: %s\n  distributed_network:\n    disabled: %s\n    disable_children: %s\n    child_limit: %s\n    logging: %s\n' \
    "$picture" "$diagnostic" "$disabled" "$disable_children" "$child_limit" "$logging" >"$temporary"
  mv "$temporary" "$path"
}

capture_soulseek_profile_distributed_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/profile-distributed-$stage.body" <<'PY'
import http.client,json,os,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200: raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
current=get("/api/v0/options"); startup=get("/api/v0/options/startup"); application=get("/api/v0/application")
def project(value):
    soulseek=value["soulseek"]
    picture=soulseek.get("picture")
    return {"picture":os.path.basename(picture) if picture else picture,"diagnosticLevel":soulseek["diagnosticLevel"],"distributedNetwork":soulseek["distributedNetwork"]}
print(json.dumps({"current":project(current),"startup":project(startup),"application":application["distributedNetwork"],"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/profile-distributed-$stage.meta"
}

run_soulseek_profile_distributed_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-soulseek-profile-distributed-$implementation"
    local suite="$work_dir/$target-soulseek-profile-distributed-$implementation"
    local log="$work_dir/$target-soulseek-profile-distributed-$implementation.log"
    local picture_startup="$state/picture-startup.bin"
    local picture_watched="$state/picture-watched.bin"
    mkdir -p "$state" "$suite"
    printf '\x00\x01\x02\xff' >"$picture_startup"
    printf '\x09\x08\x07' >"$picture_watched"
    write_soulseek_profile_distributed_yaml "$state/slskd.yml" "$picture_startup" startup
    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-profile-distributed-$implementation-options.json" "$log"
    capture_soulseek_profile_distributed_stage "$base_url" "$suite" startup

    for validation in \
      $'diagnostic-trace|soulseek:\n  diagnostic_level: trace\n' \
      $'diagnostic-invalid|soulseek:\n  diagnostic_level: verbose\n' \
      $'picture-missing|soulseek:\n  picture: /tmp/slskr-picture-that-does-not-exist\n' \
      $'distributed-valid|soulseek:\n  distributed_network:\n    disabled: false\n    disable_children: false\n    child_limit: 1\n    logging: true\n' \
      $'distributed-zero|soulseek:\n  distributed_network:\n    child_limit: 0\n' \
      $'distributed-overflow|soulseek:\n  distributed_network:\n    child_limit: 2147483648\n' \
      $'distributed-bool|soulseek:\n  distributed_network:\n    disabled: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "profile-distributed-validation-$label" POST \
        "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$yaml")"
    done

    write_soulseek_profile_distributed_yaml "$state/slskd.yml" "$picture_watched" watched
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
        | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["soulseek"]; raise SystemExit(0 if value["diagnosticLevel"] == "debug" and value["distributedNetwork"]["childLimit"] == 2 else 1)' 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    capture_soulseek_profile_distributed_stage "$base_url" "$suite" watched
    stop_daemon

    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-profile-distributed-$implementation-restarted-options.json" "$log"
    capture_soulseek_profile_distributed_stage "$base_url" "$suite" restarted
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-soulseek-profile-distributed-upstream" "$work_dir/$target-soulseek-profile-distributed-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-soulseek-profile-distributed-slskr" "$work_dir/$target-soulseek-profile-distributed-slskr.normalized"
  if ! diff -ru "$work_dir/$target-soulseek-profile-distributed-upstream.normalized" "$work_dir/$target-soulseek-profile-distributed-slskr.normalized"; then
    printf 'Soulseek profile/distributed differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s Soulseek profile/distributed differential passed\n' "$target"
}

write_dht_yaml() {
  local path="$1"
  local enabled="$2"
  local dht_port="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\nweb:\n  rate_limiting:\n    enabled: false\ndht:\n  enabled: %s\n  dht_port: %s\n' \
    "$enabled" "$dht_port" >"$temporary"
  mv "$temporary" "$path"
}

start_dht_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      exec dotnet "$dll"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      exec "$repo_root/target/debug/slskr" serve
    ) >>"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_for_dht_options() {
  local base_url="$1"
  local expected_enabled="$2"
  local expected_port="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["dhtRendezvous"]; raise SystemExit(0 if value["enabled"] == (sys.argv[1] == "true") and value["dhtPort"] == int(sys.argv[2]) else 1)' \
        "$expected_enabled" "$expected_port" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'DHT differential failed: daemon exited while waiting for enabled=%s port=%s\n' \
        "$expected_enabled" "$expected_port" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'DHT differential failed: timed out waiting for enabled=%s port=%s\n' \
    "$expected_enabled" "$expected_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_udp_state() {
  local port="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local state
    state="$($python_bin - "$port" <<'PY'
import socket,sys
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
try:
    sock.bind(("0.0.0.0", int(sys.argv[1])))
except OSError:
    print("bound")
else:
    print("free")
finally:
    sock.close()
PY
)"
    if [[ "$state" == "$expected" ]]; then
      return
    fi
    if [[ -n "$daemon_pid" ]] && ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'DHT differential failed: daemon exited while waiting for UDP %s to be %s\n' \
        "$port" "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'DHT differential failed: timed out waiting for UDP %s to be %s\n' \
    "$port" "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_dht_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "dht-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "dht-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "dht-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "dht-status-$stage" "$base_url/api/v0/dht/status"
}

run_dht_scenario() {
  local root="$1"
  local first_port="$(pick_free_udp_port)"
  local second_port="$(pick_free_udp_port)"
  while [[ "$second_port" == "$first_port" ]]; do
    second_port="$(pick_free_udp_port)"
  done

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-dht-$implementation"
    local suite="$work_dir/slskdn-dht-$implementation"
    local log="$work_dir/slskdn-dht-$implementation.log"
    mkdir -p "$state" "$suite"

    write_dht_yaml "$state/slskd.yml" true "$first_port"
    start_dht_daemon "$root" "$implementation" "$state" "$log" "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-dht-$implementation-enabled.json" "$log"
    wait_for_dht_options "$base_url" true "$first_port" "$log"
    wait_for_udp_state "$first_port" bound "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" enabled-startup

    write_dht_yaml "$state/slskd.yml" false "$second_port"
    wait_for_dht_options "$base_url" false "$second_port" "$log"
    wait_for_udp_state "$first_port" bound "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" disabled-watched

    for validation in \
      'dht-validation-enabled-zero|dht:\n  enabled: true\n  dht_port: 0\n' \
      'dht-validation-enabled-high|dht:\n  enabled: true\n  dht_port: 65536\n' \
      'dht-validation-disabled-zero|dht:\n  enabled: false\n  dht_port: 0\n' \
      'dht-validation-invalid-enabled|dht:\n  enabled: nope\n  dht_port: 50305\n' \
      'dht-validation-invalid-port|dht:\n  enabled: true\n  dht_port: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon
    wait_for_udp_state "$first_port" free "$log"

    start_dht_daemon "$root" "$implementation" "$state" "$log" "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-dht-$implementation-disabled.json" "$log"
    wait_for_dht_options "$base_url" false "$second_port" "$log"
    wait_for_udp_state "$first_port" free "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" disabled-restarted

    write_dht_yaml "$state/slskd.yml" true "$second_port"
    wait_for_dht_options "$base_url" true "$second_port" "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" enabled-watched
    stop_daemon

    start_dht_daemon "$root" "$implementation" "$state" "$log" "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-dht-$implementation-reenabled.json" "$log"
    wait_for_dht_options "$base_url" true "$second_port" "$log"
    wait_for_udp_state "$second_port" bound "$log"
    capture_dht_stage "$base_url" "$suite" enabled-restarted
    stop_daemon
    wait_for_udp_state "$second_port" free "$log"
  done

  local upstream_normalized="$work_dir/slskdn-dht-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-dht-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-dht-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-dht-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'DHT differential failed for slskdN\n' >&2
    exit 1
  fi
  printf 'slskdn DHT differential passed\n'
}

write_listener_yaml() {
  local path="$1"
  local server_port="$2"
  local listen_ip="$3"
  local listen_port="$4"
  local obfuscation_enabled="${5:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  description: listener differential\n  listen_ip_address: %s\n  listen_port: %s\n  obfuscation:\n    enabled: %s\n' \
    "$server_port" "$listen_ip" "$listen_port" "$obfuscation_enabled" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_listener_option() {
  local base_url="$1"
  local expected_ip="$2"
  local expected_port="$3"
  local log="$4"
  for _ in $(seq 1 400); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value["soulseek"]["listenIpAddress"] == sys.argv[1] and value["soulseek"]["listenPort"] == int(sys.argv[2]) else 1)' "$expected_ip" "$expected_port" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'listener differential failed: daemon exited while waiting for %s:%s\n' "$expected_ip" "$expected_port" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'listener differential failed: timed out waiting for %s:%s\n' "$expected_ip" "$expected_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

direct_user_info_is_open() {
  local host="$1"
  local port="$2"
  "$python_bin" - "$host" "$port" <<'PY'
import socket,sys
try:
    connection=socket.create_connection((sys.argv[1], int(sys.argv[2])), timeout=0.5)
except OSError:
    raise SystemExit(1)
connection.close()
PY
}

wait_for_direct_user_info_state() {
  local host="$1"
  local port="$2"
  local expected="$3"
  local log="$4"
  for _ in $(seq 1 120); do
    local actual=closed
    if direct_user_info_is_open "$host" "$port"; then
      actual=open
    fi
    [[ "$actual" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'listener differential failed: daemon exited while waiting for %s:%s to be %s\n' "$host" "$port" "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'listener differential failed: %s:%s did not become %s\n' "$host" "$port" "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_advertised_port() {
  local status="$1"
  local expected_port="$2"
  local minimum_count="$3"
  local log="$4"
  for _ in $(seq 1 300); do
    if "$python_bin" - "$status" "$expected_port" "$minimum_count" <<'PY'
import json,sys
try:
    value=json.load(open(sys.argv[1], encoding="utf-8"))
except (FileNotFoundError, json.JSONDecodeError):
    raise SystemExit(1)
ports=value.get("set_wait_ports", [])
raise SystemExit(0 if len(ports) >= int(sys.argv[3]) and ports[-1] == int(sys.argv[2]) else 1)
PY
    then
      return
    fi
    sleep 0.05
  done
  printf 'listener differential failed: server advertisement did not reach port %s at count %s\n' "$expected_port" "$minimum_count" >&2
  cat "$status" >&2 || true
  tail -120 "$log" >&2 || true
  exit 1
}

advertisement_count() {
  "$python_bin" -c 'import json,sys; print(len(json.load(open(sys.argv[1], encoding="utf-8")).get("set_wait_ports", [])))' "$1"
}

wait_for_advertisement_count() {
  local status="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 300); do
    local actual
    actual="$(advertisement_count "$status" 2>/dev/null || true)"
    if [[ "$actual" == "$expected" ]]; then
      sleep 0.2
      actual="$(advertisement_count "$status" 2>/dev/null || true)"
      [[ "$actual" == "$expected" ]] && return
    fi
    if [[ "$actual" =~ ^[0-9]+$ && "$actual" -gt "$expected" ]]; then
      break
    fi
    sleep 0.05
  done
  printf 'listener differential failed: expected %s SetListenPort messages\n' "$expected" >&2
  cat "$status" >&2 || true
  tail -120 "$log" >&2 || true
  exit 1
}

capture_listener_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local expected_ip="$5"
  local expected_port="$6"
  local old_port="$7"
  local new_port="$8"
  local host_ip="$9"
  local fixture_status="${10}"
  local options_file="$suite/listener-$stage-options.raw"
  local application_file="$suite/listener-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$options_file"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application_file"
  local local_old=false local_new=false host_new=false
  direct_user_info_is_open 127.0.0.1 "$old_port" && local_old=true
  direct_user_info_is_open 127.0.0.1 "$new_port" && local_new=true
  direct_user_info_is_open "$host_ip" "$new_port" && host_new=true
  local local_obfuscated=false host_obfuscated=false
  if [[ "$expected_port" -lt 65535 ]]; then
    direct_user_info_is_open 127.0.0.1 "$((expected_port + 1))" && local_obfuscated=true
    direct_user_info_is_open "$host_ip" "$((expected_port + 1))" && host_obfuscated=true
  fi
  "$python_bin" - "$options_file" "$application_file" "$fixture_status" \
    "$target" "$expected_ip" "$expected_port" "$local_old" "$local_new" "$host_new" \
    "$local_obfuscated" "$host_obfuscated" "$stage" \
    >"$suite/listener-$stage.body" <<'PY'
import json,sys
options=json.load(open(sys.argv[1], encoding="utf-8"))
application=json.load(open(sys.argv[2], encoding="utf-8"))
fixture=json.load(open(sys.argv[3], encoding="utf-8"))
target=sys.argv[4]
expected_ip=sys.argv[5]
expected_port=int(sys.argv[6])
ports=fixture.get("set_wait_ports", [])
messages=fixture.get("set_wait_port_messages", [])
message=messages[-1] if messages else {}
expects_obfuscation=target == "slskdn" and expected_port < 65535
expected_message_count={
    "startup": 2,
    "port-watched": 3,
    "port-restarted": 5,
    "ip-watched": 5,
    "ip-restarted": 7,
}[sys.argv[12]]
print(json.dumps({
    "optionsIpMatches": options["soulseek"]["listenIpAddress"] == expected_ip,
    "optionsPortMatches": options["soulseek"]["listenPort"] == expected_port,
    "pendingReconnect": application["pendingReconnect"],
    "pendingRestart": application["pendingRestart"],
    "serverState": application["server"]["state"],
    "localOldOpen": sys.argv[7] == "true",
    "localNewOpen": sys.argv[8] == "true",
    "hostNewOpen": sys.argv[9] == "true",
    "localObfuscatedOpen": sys.argv[10] == "true",
    "hostObfuscatedOpen": sys.argv[11] == "true",
    "advertisedPortMatches": bool(ports) and ports[-1] == expected_port,
    "advertisementCountMatches": len(messages) == expected_message_count,
    "advertisementMetadataMatches": bool(message) and (
        message.get("payload_length") == (12 if expects_obfuscation else 4)
        and message.get("obfuscation_type") == (1 if expects_obfuscation else None)
        and message.get("obfuscated_port") == (expected_port + 1 if expects_obfuscation else None)
    ),
}, sort_keys=True, separators=(",", ":")))
PY
  rm -f "$options_file" "$application_file"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/listener-$stage.meta"
}

listener_validation_payload() {
  local yaml="$1"
  "$python_bin" -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml"
}

write_obfuscation_yaml() {
  local path="$1"
  local listen_port="$2"
  local enabled="$3"
  local mode="$4"
  local obfuscated_port="$5"
  local advertise_regular_port="$6"
  local prefer_outbound="$7"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n  obfuscation:\n    enabled: %s\n    mode: %s\n    listen_port: %s\n    advertise_regular_port: %s\n    prefer_outbound: %s\n' \
    "$listen_port" "$enabled" "$mode" "$obfuscated_port" \
    "$advertise_regular_port" "$prefer_outbound" >"$temporary"
  mv "$temporary" "$path"
}

write_obfuscation_omitted_yaml() {
  local path="$1"
  local listen_port="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

start_obfuscation_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  local environment_enabled="${7:-}"
  local environment_mode="${8:-}"
  local environment_listen_port="${9:-}"
  local environment_advertise_regular="${10:-}"
  local environment_prefer_outbound="${11:-}"
  local cli_enabled="${12:-false}"
  local cli_mode="${13:-}"
  local cli_listen_port="${14:-}"
  local cli_advertise_regular="${15:-false}"
  local cli_prefer_outbound="${16:-false}"
  local append="${17:-false}"
  local cli_args=()
  [[ "$cli_enabled" == true ]] && cli_args+=(--slsk-obfuscation)
  [[ -n "$cli_mode" ]] && cli_args+=(--slsk-obfuscation-mode "$cli_mode")
  [[ -n "$cli_listen_port" ]] && cli_args+=(--slsk-obfuscation-listen-port "$cli_listen_port")
  [[ "$cli_advertise_regular" == true ]] && cli_args+=(--slsk-obfuscation-advertise-regular-port)
  [[ "$cli_prefer_outbound" == true ]] && cli_args+=(--slsk-obfuscation-prefer-outbound)
  run_obfuscation_process() {
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_OBFUSCATION="$environment_enabled"
      [[ -n "$environment_mode" ]] && export SLSKD_SLSK_OBFUSCATION_MODE="$environment_mode"
      [[ -n "$environment_listen_port" ]] && export SLSKD_SLSK_OBFUSCATION_LISTEN_PORT="$environment_listen_port"
      [[ -n "$environment_advertise_regular" ]] && export SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT="$environment_advertise_regular"
      [[ -n "$environment_prefer_outbound" ]] && export SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND="$environment_prefer_outbound"
      exec dotnet "$dll" "${cli_args[@]}"
    else
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_OBFUSCATION="$environment_enabled"
      [[ -n "$environment_mode" ]] && export SLSKD_SLSK_OBFUSCATION_MODE="$environment_mode"
      [[ -n "$environment_listen_port" ]] && export SLSKD_SLSK_OBFUSCATION_LISTEN_PORT="$environment_listen_port"
      [[ -n "$environment_advertise_regular" ]] && export SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT="$environment_advertise_regular"
      [[ -n "$environment_prefer_outbound" ]] && export SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND="$environment_prefer_outbound"
      exec "$repo_root/target/debug/slskr" serve "${cli_args[@]}"
    fi
  }
  if [[ "$append" == true ]]; then
    run_obfuscation_process >>"$log" 2>&1 &
  else
    run_obfuscation_process >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

capture_obfuscation_options() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "obfuscation-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "obfuscation-startup-$stage" "$base_url/api/v0/options/startup"
}

capture_obfuscation_invalid_startup() {
  local root="$1" implementation="$2" state="$3" suite="$4" log="$5"
  local http_port="$6" https_port="$7" regular_port="$8" obfuscated_port="$9"
  write_obfuscation_yaml "$state/slskd.yml" "$regular_port" true compatibility \
    "$obfuscated_port" false true
  start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
    "$http_port" "$https_port"
  for _ in $(seq 1 300); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then break; fi
    sleep 0.05
  done
  if kill -0 "$daemon_pid" 2>/dev/null; then
    printf 'obfuscation invalid-startup differential: daemon did not exit\n' >&2
    stop_daemon
    exit 1
  fi
  local exit_code=0
  wait "$daemon_pid" || exit_code="$?"
  daemon_pid=""
  local matched=false
  if grep -q 'regular peer port must be advertised' "$log"; then matched=true; fi
  "$python_bin" - "$exit_code" "$matched" >"$suite/obfuscation-invalid-startup.body" <<'PY'
import json,sys
print(json.dumps({
    "exitCode":int(sys.argv[1]),
    "regularAdvertisementFailure":sys.argv[2] == "true",
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/obfuscation-invalid-startup.meta"
}

run_obfuscation_options_scenario() {
  local root="$1"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local regular_port="$(pick_free_port)"
  local yaml_obfuscated_port="$(pick_free_port)"
  local environment_obfuscated_port="$(pick_free_port)"
  local cli_obfuscated_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"

  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-obfuscation-$implementation"
    local suite="$work_dir/slskdn-obfuscation-$implementation"
    local log="$work_dir/slskdn-obfuscation-$implementation.log"
    mkdir -p "$state" "$suite"

    write_obfuscation_yaml "$state/slskd.yml" "$regular_port" false prefer \
      "$yaml_obfuscated_port" false false
    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true compatibility "$environment_obfuscated_port" true true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-yaml.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" yaml-over-environment

    for validation in \
      $'mode-invalid|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    mode: invalid\n' \
      $'mode-only|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    enabled: true\n    mode: only\n' \
      $'listen-low|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    listen_port: 1023\n' \
      $'listen-high|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    listen_port: 65536\n' \
      $'listen-same|flags:\n  no_connect: true\nsoulseek:\n  listen_port: 50300\n  obfuscation:\n    enabled: true\n    listen_port: 50300\n' \
      $'listen-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    listen_port: nope\n' \
      $'enabled-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    enabled: nope\n' \
      $'advertise-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    advertise_regular_port: nope\n' \
      $'prefer-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    prefer_outbound: nope\n' \
      $'advertise-disabled|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    enabled: true\n    advertise_regular_port: false\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "obfuscation-validation-$label" POST \
        "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$yaml")"
    done
    stop_daemon

    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true compatibility "$environment_obfuscated_port" true true \
      true compatibility "$cli_obfuscated_port" true true true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-cli.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    write_obfuscation_omitted_yaml "$state/slskd.yml" "$regular_port"
    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false prefer "$environment_obfuscated_port" false false \
      false "" "" false false true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-environment.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" environment-with-yaml-omitted
    stop_daemon

    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" "" "" "" false "" "" false false true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-defaults.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" defaults
    stop_daemon

    capture_obfuscation_invalid_startup "$root" "$implementation" "$state" "$suite" "$log" \
      "$http_port" "$https_port" "$regular_port" "$yaml_obfuscated_port"
  done

  local upstream_normalized="$work_dir/slskdn-obfuscation-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-obfuscation-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'obfuscation options differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn obfuscation options differential passed\n'
}

write_obfuscation_runtime_yaml() {
  local path="$1"
  local server_port="$2"
  local regular_port="$3"
  local enabled="$4"
  local mode="$5"
  local obfuscated_port="$6"
  local advertise_regular_port="$7"
  local prefer_outbound="$8"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: false\ndht:\n  enabled: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n  obfuscation:\n    enabled: %s\n    mode: %s\n    listen_port: %s\n    advertise_regular_port: %s\n    prefer_outbound: %s\n' \
    "$server_port" "$regular_port" "$enabled" "$mode" "$obfuscated_port" \
    "$advertise_regular_port" "$prefer_outbound" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_obfuscation_option() {
  local base_url="$1"
  local enabled="$2"
  local mode="$3"
  local listen_port="$4"
  local advertise_regular="$5"
  local prefer_outbound="$6"
  local log="$7"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; v=json.load(sys.stdin)["soulseek"]["obfuscation"]; expected={"enabled":sys.argv[1]=="true","mode":sys.argv[2],"listenPort":int(sys.argv[3]),"advertiseRegularPort":sys.argv[4]=="true","preferOutbound":sys.argv[5]=="true"}; raise SystemExit(0 if all(v[k] == x for k,x in expected.items()) else 1)' \
        "$enabled" "$mode" "$listen_port" "$advertise_regular" "$prefer_outbound" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'obfuscation runtime differential: daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'obfuscation runtime differential: timed out waiting for options\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_obfuscation_runtime_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local fixture_status="$4"
  local regular_port="$5"
  local obfuscated_port="$6"
  local expected_obfuscated_open=false
  local regular_open=false
  direct_user_info_is_open 127.0.0.1 "$regular_port" && regular_open=true
  if [[ "$obfuscated_port" != 0 ]]; then
    direct_user_info_is_open 127.0.0.1 "$obfuscated_port" && expected_obfuscated_open=true
  fi
  capture_obfuscation_options "$base_url" "$suite" "$stage"
  capture_get "$suite" "obfuscation-application-$stage" "$base_url/api/v0/application"
  capture_fixture_status "$suite" "obfuscation-network-$stage" "$fixture_status"
  "$python_bin" - "$regular_open" "$expected_obfuscated_open" >"$suite/obfuscation-sockets-$stage.body" <<'PY'
import json,sys
print(json.dumps({"regularOpen":sys.argv[1] == "true","obfuscatedOpen":sys.argv[2] == "true"},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/obfuscation-sockets-$stage.meta"
}

run_obfuscation_runtime_scenario() {
  local root="$1"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local server_port="$(pick_free_port)"
  local regular_port="$(pick_free_port)"
  local obfuscated_port_a="$(pick_free_port)"
  local obfuscated_port_b="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-obfuscation-runtime-$implementation"
    local suite="$work_dir/slskdn-obfuscation-runtime-$implementation"
    local log="$work_dir/slskdn-obfuscation-runtime-$implementation.log"
    local fixture_status="$work_dir/slskdn-obfuscation-runtime-$implementation-fixture.json"
    local fixture_log="$work_dir/slskdn-obfuscation-runtime-$implementation-fixture.log"
    mkdir -p "$state" "$suite"
    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success 0.0.0.0

    write_obfuscation_runtime_yaml "$state/slskd.yml" "$server_port" "$regular_port" \
      true compatibility "$obfuscated_port_a" true true
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-runtime-$implementation-start.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_advertisement_count "$fixture_status" 2 "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$obfuscated_port_a" open "$log"
    capture_obfuscation_runtime_stage "$base_url" "$suite" enabled-explicit "$fixture_status" \
      "$regular_port" "$obfuscated_port_a"

    write_obfuscation_runtime_yaml "$state/slskd.yml" "$server_port" "$regular_port" \
      false prefer "$obfuscated_port_a" false false
    wait_for_obfuscation_option "$base_url" false prefer "$obfuscated_port_a" false false "$log"
    wait_for_pending_reconnect "$base_url" true "$log"
    wait_for_advertisement_count "$fixture_status" 3 "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$obfuscated_port_a" open "$log"
    capture_obfuscation_runtime_stage "$base_url" "$suite" disabled-watched "$fixture_status" \
      "$regular_port" "$obfuscated_port_a"

    stop_daemon
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-runtime-$implementation-disabled.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_advertisement_count "$fixture_status" 5 "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$obfuscated_port_a" closed "$log"
    capture_obfuscation_runtime_stage "$base_url" "$suite" disabled-restarted "$fixture_status" \
      "$regular_port" "$obfuscated_port_a"

    write_obfuscation_runtime_yaml "$state/slskd.yml" "$server_port" "$regular_port" \
      true prefer "$obfuscated_port_b" true true
    wait_for_obfuscation_option "$base_url" true prefer "$obfuscated_port_b" true true "$log"
    wait_for_pending_reconnect "$base_url" true "$log"
    wait_for_advertisement_count "$fixture_status" 6 "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$obfuscated_port_b" closed "$log"
    capture_obfuscation_runtime_stage "$base_url" "$suite" reenabled-watched "$fixture_status" \
      "$regular_port" "$obfuscated_port_b"

    stop_daemon
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-runtime-$implementation-enabled.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_advertisement_count "$fixture_status" 8 "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$obfuscated_port_b" open "$log"
    capture_obfuscation_runtime_stage "$base_url" "$suite" reenabled-restarted "$fixture_status" \
      "$regular_port" "$obfuscated_port_b"
    stop_daemon
    stop_soulseek_fixture
  done

  local upstream_normalized="$work_dir/slskdn-obfuscation-runtime-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-obfuscation-runtime-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-runtime-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-runtime-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'obfuscation runtime differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn obfuscation runtime differential passed\n'
}

start_soulseek_peer_fixture() {
  local server_port="$1"
  local status="$2"
  local log="$3"
  local regular_port="$4"
  local obfuscated_port="$5"
  local listener_mode="$6"
  "$python_bin" "$repo_root/scripts/fixture-soulseek-listener.py" \
    0.0.0.0 "$server_port" "$status" login-success 0.0.0.0 \
    "$regular_port" "$obfuscated_port" "$listener_mode" >"$log" 2>&1 &
  soulseek_fixture_pid="$!"
  for _ in $(seq 1 100); do
    [[ -s "$status" ]] && return
    if ! kill -0 "$soulseek_fixture_pid" 2>/dev/null; then
      printf 'obfuscation outbound differential: fixture exited\n' >&2
      cat "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'obfuscation outbound differential: fixture did not become ready\n' >&2
  exit 1
}

capture_obfuscation_outbound() {
  local suite="$1"
  local stage="$2"
  local status="$3"
  "$python_bin" - "$status" >"$suite/obfuscation-outbound-$stage.body" <<'PY'
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
print(json.dumps({
    "addressRequested": bool(value.get("peer_address_requests")),
    "regularAccepted": value.get("regular_peer_accepts", 0) > 0,
    "obfuscatedAccepted": value.get("obfuscated_peer_accepts", 0) > 0,
}, sort_keys=True, separators=(",", ":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/obfuscation-outbound-$stage.meta"
}

trigger_obfuscation_outbound() {
  local implementation="$1"
  local base_url="$2"
  if [[ "$implementation" == upstream ]]; then
    curl --silent --max-time 3 "$base_url/api/v0/users/fixture-peer/browse" >/dev/null || true
  else
    curl --fail --silent --max-time 3 --request POST \
      "$base_url/api/v0/users/fixture-peer/browse/request" >/dev/null
    sleep 3
  fi
}

run_obfuscation_outbound_scenario() {
  local root="$1"
  local cases=(
    'compatibility-enabled|true|compatibility|true|obfuscated-only'
    'prefer-flag-disabled|true|prefer|false|obfuscated-only'
    'prefer-enabled|true|prefer|true|obfuscated-only'
    'obfuscation-disabled|false|prefer|true|obfuscated-only'
    'prefer-regular-fallback|true|prefer|true|regular-only'
  )
  for implementation in upstream slskr; do
    local suite="$work_dir/slskdn-obfuscation-outbound-$implementation"
    mkdir -p "$suite"
    for item in "${cases[@]}"; do
      IFS='|' read -r stage enabled mode prefer listener_mode <<<"$item"
      local http_port="$(pick_free_port)"
      local https_port="$(pick_free_port)"
      local server_port="$(pick_free_port)"
      local daemon_regular_port="$(pick_free_port)"
      local daemon_obfuscated_port="$(pick_free_port)"
      local peer_regular_port="$(pick_free_port)"
      local peer_obfuscated_port="$(pick_free_port)"
      local base_url="http://127.0.0.1:$http_port"
      local state="$work_dir/state-slskdn-obfuscation-outbound-$implementation-$stage"
      local log="$work_dir/slskdn-obfuscation-outbound-$implementation-$stage.log"
      local fixture_status="$work_dir/slskdn-obfuscation-outbound-$implementation-$stage-fixture.json"
      local fixture_log="$work_dir/slskdn-obfuscation-outbound-$implementation-$stage-fixture.log"
      mkdir -p "$state"
      start_soulseek_peer_fixture "$server_port" "$fixture_status" "$fixture_log" \
        "$peer_regular_port" "$peer_obfuscated_port" "$listener_mode"
      write_obfuscation_runtime_yaml "$state/slskd.yml" "$server_port" \
        "$daemon_regular_port" "$enabled" "$mode" "$daemon_obfuscated_port" true "$prefer"
      start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
        "$http_port" "$https_port"
      wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-outbound-$implementation-$stage.json" "$log"
      wait_for_fixture_active "$fixture_status" 1 "$log"
      trigger_obfuscation_outbound "$implementation" "$base_url"
      capture_obfuscation_outbound "$suite" "$stage" "$fixture_status"
      stop_daemon
      stop_soulseek_fixture
    done
  done

  local upstream_normalized="$work_dir/slskdn-obfuscation-outbound-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-obfuscation-outbound-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-outbound-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-outbound-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'obfuscation outbound differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn obfuscation outbound differential passed\n'
}

start_listener_blocker() {
  local host="$1"
  local port="$2"
  local status="$3"
  "$python_bin" - "$host" "$port" "$status" <<'PY' &
import pathlib,socket,sys,time
listener=socket.socket(socket.AF_INET, socket.SOCK_STREAM)
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind((sys.argv[1], int(sys.argv[2])))
listener.listen(4)
pathlib.Path(sys.argv[3]).write_text("ready", encoding="utf-8")
while True:
    time.sleep(1)
PY
  listener_blocker_pid="$!"
  for _ in $(seq 1 100); do
    [[ -s "$status" ]] && return
    kill -0 "$listener_blocker_pid" 2>/dev/null || {
      printf 'listener differential failed: conflict fixture exited\n' >&2
      exit 1
    }
    sleep 0.05
  done
  printf 'listener differential failed: conflict fixture did not become ready\n' >&2
  exit 1
}

run_listener_scenario() {
  local target="$1"
  local root="$2"
  local host_ip
  host_ip="$(host_ipv4_address)"
  local obfuscation_enabled=false
  [[ "$target" == slskdn ]] && obfuscation_enabled=true

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local old_port="$(pick_free_port_with_free_successor)"
    local new_port="$(pick_free_port_with_free_successor)"
    local busy_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-listener-$implementation"
    local suite="$work_dir/$target-listener-$implementation"
    local log="$work_dir/$target-listener-$implementation.log"
    local fixture_status="$work_dir/$target-listener-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-listener-$implementation-fixture.log"
    mkdir -p "$state" "$suite"
    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success
    write_listener_yaml "$state/slskd.yml" "$server_port" 0.0.0.0 "$old_port" "$obfuscation_enabled"
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-listener-$implementation-options.json" "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$old_port" open "$log"
    wait_for_advertised_port "$fixture_status" "$old_port" 2 "$log"
    wait_for_advertisement_count "$fixture_status" 2 "$log"
    capture_listener_stage "$target" "$base_url" "$suite" startup 0.0.0.0 "$old_port" \
      "$old_port" "$new_port" "$host_ip" "$fixture_status"

    write_listener_yaml "$state/slskd.yml" "$server_port" 0.0.0.0 "$new_port" "$obfuscation_enabled"
    wait_for_listener_option "$base_url" 0.0.0.0 "$new_port" "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$old_port" closed "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$new_port" open "$log"
    wait_for_advertised_port "$fixture_status" "$new_port" 3 "$log"
    wait_for_advertisement_count "$fixture_status" 3 "$log"
    capture_listener_stage "$target" "$base_url" "$suite" port-watched 0.0.0.0 "$new_port" \
      "$old_port" "$new_port" "$host_ip" "$fixture_status"

    stop_daemon
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/$target-listener-$implementation-port-restart.json" "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$new_port" open "$log"
    wait_for_advertised_port "$fixture_status" "$new_port" 5 "$log"
    wait_for_advertisement_count "$fixture_status" 5 "$log"
    capture_listener_stage "$target" "$base_url" "$suite" port-restarted 0.0.0.0 "$new_port" \
      "$old_port" "$new_port" "$host_ip" "$fixture_status"

    local advertisement_before_ip
    advertisement_before_ip="$(advertisement_count "$fixture_status")"
    write_listener_yaml "$state/slskd.yml" "$server_port" "$host_ip" "$new_port" "$obfuscation_enabled"
    wait_for_listener_option "$base_url" "$host_ip" "$new_port" "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$new_port" open "$log"
    wait_for_direct_user_info_state "$host_ip" "$new_port" open "$log"
    if [[ "$(advertisement_count "$fixture_status")" != "$advertisement_before_ip" ]]; then
      printf 'listener differential failed: failed IP-only update changed server advertisement\n' >&2
      exit 1
    fi
    wait_for_advertisement_count "$fixture_status" 5 "$log"
    capture_listener_stage "$target" "$base_url" "$suite" ip-watched "$host_ip" "$new_port" \
      "$old_port" "$new_port" "$host_ip" "$fixture_status"

    stop_daemon
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/$target-listener-$implementation-ip-restart.json" "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$new_port" closed "$log"
    wait_for_direct_user_info_state "$host_ip" "$new_port" open "$log"
    wait_for_advertisement_count "$fixture_status" 7 "$log"
    capture_listener_stage "$target" "$base_url" "$suite" ip-restarted "$host_ip" "$new_port" \
      "$old_port" "$new_port" "$host_ip" "$fixture_status"

    capture_request "$suite" listener-validation-bad-ip POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: not-an-ip\n  listen_port: 50300\n')"
    capture_request "$suite" listener-validation-low-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: 1023\n')"
    capture_request "$suite" listener-validation-high-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: 65536\n')"
    capture_request "$suite" listener-validation-text-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: nope\n')"
    capture_request "$suite" listener-validation-null-ip POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: null\n  listen_port: 50300\n')"
    capture_request "$suite" listener-validation-numeric-ip POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 123\n  listen_port: 50300\n')"
    capture_request "$suite" listener-validation-bool-ip POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: true\n  listen_port: 50300\n')"
    capture_request "$suite" listener-validation-null-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: null\n')"
    capture_request "$suite" listener-validation-bool-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: true\n')"
    capture_request "$suite" listener-validation-float-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: 50300.5\n')"
    capture_request "$suite" listener-validation-negative-port POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: -1\n')"
    capture_request "$suite" listener-validation-lower-bound POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: 1024\n')"
    capture_request "$suite" listener-validation-upper-bound POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'flags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: 65535\n')"

    local advertisement_before_conflict
    advertisement_before_conflict="$(advertisement_count "$fixture_status")"
    start_listener_blocker "$host_ip" "$busy_port" "$work_dir/$target-listener-$implementation-blocker.status"
    write_listener_yaml "$state/slskd.yml" "$server_port" "$host_ip" "$busy_port" "$obfuscation_enabled"
    wait_for_listener_option "$base_url" "$host_ip" "$busy_port" "$log"
    sleep 0.5
    wait_for_direct_user_info_state "$host_ip" "$new_port" open "$log"
    "$python_bin" - "$fixture_status" "$advertisement_before_conflict" >"$suite/listener-bind-conflict.body" <<'PY'
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
print(json.dumps({
    "advertisementUnchanged": len(value.get("set_wait_ports", [])) == int(sys.argv[2]),
    "previousListenerRetained": True,
}, sort_keys=True, separators=(",", ":")))
PY
    printf 'status=200\ncontent-type=application/json\n' >"$suite/listener-bind-conflict.meta"
    write_listener_yaml "$state/slskd.yml" "$server_port" "$host_ip" "$new_port" "$obfuscation_enabled"
    wait_for_listener_option "$base_url" "$host_ip" "$new_port" "$log"
    stop_listener_blocker

    # Run this case last: both frozen daemons can publish an invalid watched
    # options object to a background service after returning the target-shaped
    # HTTP 500, which nondeterministically stops the host. Capture the stable
    # response without claiming reliable in-process recovery from that bug.
    write_listener_yaml "$state/slskd.yml" "$server_port" "$host_ip" 1023 "$obfuscation_enabled"
    local invalid_observed=false
    for _ in $(seq 1 200); do
      local status
      status="$(curl --silent --show-error --max-time 1 \
        --output "$suite/listener-invalid-watch.body" \
        --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
        "$base_url/api/v0/options" >"$suite/listener-invalid-watch.meta" 2>/dev/null \
        && sed -n 's/^status=//p' "$suite/listener-invalid-watch.meta" || true)"
      if [[ "$status" == 500 ]]; then
        invalid_observed=true
        break
      fi
      sleep 0.05
    done
    [[ "$invalid_observed" == true ]] || {
      printf 'listener differential failed: invalid watched port did not produce HTTP 500 for %s/%s\n' "$target" "$implementation" >&2
      tail -120 "$log" >&2 || true
      exit 1
    }
    stop_daemon
    stop_soulseek_fixture
  done

  local upstream_normalized="$work_dir/$target-listener-upstream.normalized"
  local slskr_normalized="$work_dir/$target-listener-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-listener-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-listener-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'listener differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s listener differential passed\n' "$target"
}

write_web_listener_yaml() {
  local target="$1"
  local path="$2"
  local address="$3"
  local port="$4"
  local temporary="${path}.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ndht:\n  enabled: false\n' >"$temporary"
  if [[ "$address" != __omit__ ]]; then
    if [[ "$target" == slskd ]]; then
      printf 'web:\n  ip_address: "%s"\n  port: %s\n' "$address" "$port" >>"$temporary"
    else
      printf 'web:\n  address: "%s"\n  port: %s\n' "$address" "$port" >>"$temporary"
    fi
  fi
  mv "$temporary" "$path"
}

start_web_listener_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local https_port="$6"
  local environment_address="${7:-}"
  local environment_port="${8:-}"
  local command_address="${9:-}"
  local command_port="${10:-}"
  local append="${11:-false}"
  local soulseek_listen_port
  soulseek_listen_port="$(pick_free_port)"
  local address_environment=SLSKD_HTTP_IP_ADDRESS
  local address_flag=--http-ip-address
  if [[ "$target" == slskdn ]]; then
    address_environment=SLSKD_HTTP_ADDRESS
    address_flag=--http-address
  fi
  local -a command_line=()
  [[ -n "$command_address" ]] && command_line+=("$address_flag" "$command_address")
  [[ -n "$command_port" ]] && command_line+=(--http-port "$command_port")
  local redirect='>'
  [[ "$append" == true ]] && redirect='>>'
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$redirect" == '>>' ]]; then
      (
        unset SLSKD_HTTP_IP_ADDRESS SLSKD_HTTP_ADDRESS SLSKD_HTTP_PORT
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_REMOTE_CONFIGURATION=true SLSKD_DHT_ENABLED=false SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$soulseek_listen_port"
        [[ -n "$environment_address" ]] && export "$address_environment=$environment_address"
        [[ -n "$environment_port" ]] && export SLSKD_HTTP_PORT="$environment_port"
        exec dotnet "$dll" "${command_line[@]}"
      ) >>"$log" 2>&1 &
    else
      (
        unset SLSKD_HTTP_IP_ADDRESS SLSKD_HTTP_ADDRESS SLSKD_HTTP_PORT
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_REMOTE_CONFIGURATION=true SLSKD_DHT_ENABLED=false SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$soulseek_listen_port"
        [[ -n "$environment_address" ]] && export "$address_environment=$environment_address"
        [[ -n "$environment_port" ]] && export SLSKD_HTTP_PORT="$environment_port"
        exec dotnet "$dll" "${command_line[@]}"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$redirect" == '>>' ]]; then
    (
      unset SLSKD_HTTP_IP_ADDRESS SLSKD_HTTP_ADDRESS SLSKD_HTTP_PORT
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_SLSK_LISTEN_PORT="$soulseek_listen_port"
      [[ -n "$environment_address" ]] && export "$address_environment=$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_HTTP_PORT="$environment_port"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" "${command_line[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      unset SLSKD_HTTP_IP_ADDRESS SLSKD_HTTP_ADDRESS SLSKD_HTTP_PORT
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      export SLSKD_SLSK_LISTEN_PORT="$soulseek_listen_port"
      [[ -n "$environment_address" ]] && export "$address_environment=$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_HTTP_PORT="$environment_port"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" "${command_line[@]}"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

tcp_endpoint_is_open() {
  "$python_bin" - "$1" "$2" <<'PY'
import socket,sys
try:
    connection=socket.create_connection((sys.argv[1], int(sys.argv[2])), timeout=0.2)
    connection.close()
except OSError:
    raise SystemExit(1)
PY
}

wait_for_tcp_endpoint_state() {
  local host="$1"
  local port="$2"
  local expected="$3"
  local log="$4"
  for _ in $(seq 1 200); do
    local state=closed
    tcp_endpoint_is_open "$host" "$port" && state=open
    [[ "$state" == "$expected" ]] && return
    if [[ "$expected" == open ]] && ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'web listener differential failed: daemon exited waiting for %s:%s\n' "$host" "$port" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'web listener differential failed: %s:%s did not become %s\n' "$host" "$port" "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_web_listener_option() {
  local target="$1"
  local base_url="$2"
  local expected_address="$3"
  local expected_port="$4"
  local log="$5"
  for _ in $(seq 1 200); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" | "$python_bin" -c '
import json,sys
target,address,port=sys.argv[1:]
value=json.load(sys.stdin)
key="ipAddress" if target=="slskd" else "address"
raise SystemExit(0 if value["web"].get(key)==address and value["web"].get("port")==int(port) else 1)
' "$target" "$expected_address" "$expected_port"; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'web listener differential failed: daemon exited waiting for watched options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'web listener differential failed: watched options did not update\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_web_listener_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local expected_current_address="$5"
  local expected_current_port="$6"
  local expected_startup_address="$7"
  local expected_startup_port="$8"
  local expected_pending="$9"
  local old_port="${10}"
  local new_port="${11}"
  local expected_old_open="${12}"
  local expected_new_open="${13}"
  local expected_old_ipv6_open="${14}"
  local current="$suite/web-$stage-current.raw"
  local startup="$suite/web-$stage-startup.raw"
  local application="$suite/web-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application"
  local old_open=false new_open=false old_ipv6_open=false
  tcp_endpoint_is_open 127.0.0.1 "$old_port" && old_open=true
  tcp_endpoint_is_open 127.0.0.1 "$new_port" && new_open=true
  tcp_endpoint_is_open ::1 "$old_port" && old_ipv6_open=true
  "$python_bin" - "$current" "$startup" "$application" "$target" \
    "$expected_current_address" "$expected_current_port" \
    "$expected_startup_address" "$expected_startup_port" "$expected_pending" \
    "$old_open" "$new_open" "$old_ipv6_open" "$expected_old_open" \
    "$expected_new_open" "$expected_old_ipv6_open" >"$suite/web-$stage.body" <<'PY'
import json,sys
current=json.load(open(sys.argv[1], encoding="utf-8"))
startup=json.load(open(sys.argv[2], encoding="utf-8"))
application=json.load(open(sys.argv[3], encoding="utf-8"))
target=sys.argv[4]
key="ipAddress" if target=="slskd" else "address"
expected_current=None if sys.argv[5]=="__null__" else sys.argv[5]
expected_startup=None if sys.argv[7]=="__null__" else sys.argv[7]
result={
    "currentAddressMatches": current["web"].get(key)==expected_current,
    "currentPortMatches": current["web"].get("port")==int(sys.argv[6]),
    "startupAddressMatches": startup["web"].get(key)==expected_startup,
    "startupPortMatches": startup["web"].get("port")==int(sys.argv[8]),
    "pendingRestartMatches": application["pendingRestart"]==(sys.argv[9]=="true"),
    "oldEndpointMatches": (sys.argv[10]=="true")== (sys.argv[13]=="true"),
    "newEndpointMatches": (sys.argv[11]=="true")== (sys.argv[14]=="true"),
    "oldIpv6EndpointMatches": (sys.argv[12]=="true")== (sys.argv[15]=="true"),
}
print(json.dumps(result, sort_keys=True, separators=(",",":")))
if not all(result.values()):
    raise SystemExit(f"web listener stage mismatch: {result}")
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/web-$stage.meta"
  rm -f "$current" "$startup" "$application"
}

run_web_listener_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local suite="$work_dir/$target-web-listener-$implementation"
    mkdir -p "$suite"
    for precedence in default environment yaml command-line; do
      local state="$work_dir/state-$target-web-$implementation-$precedence"
      local log="$work_dir/$target-web-$implementation-$precedence.log"
      local https_port="$(pick_free_port)"
      local yaml_port="$(pick_free_port)"
      local environment_port="$(pick_free_port)"
      local command_candidate_port="$(pick_free_port)"
      local command_port=''
      local inactive_port="$(pick_free_port)"
      local expected_port="$yaml_port"
      local yaml_address=127.0.0.1
      local environment_address=127.0.0.1
      local command_address=''
      local expected_address=127.0.0.1
      mkdir -p "$state"
      case "$precedence" in
        default)
          write_web_listener_yaml "$target" "$state/slskd.yml" __omit__ "$yaml_port"
          environment_address=''
          environment_port=''
          command_port="$command_candidate_port"
          expected_port="$command_candidate_port"
          [[ "$target" == slskd ]] && expected_address=__null__
          ;;
        environment)
          write_web_listener_yaml "$target" "$state/slskd.yml" __omit__ "$yaml_port"
          expected_port="$environment_port"
          ;;
        yaml)
          write_web_listener_yaml "$target" "$state/slskd.yml" "$yaml_address" "$yaml_port"
          ;;
        command-line)
          write_web_listener_yaml "$target" "$state/slskd.yml" "$yaml_address" "$yaml_port"
          command_address=127.0.0.1
          command_port="$command_candidate_port"
          expected_port="$command_candidate_port"
          ;;
      esac
      start_web_listener_daemon "$target" "$root" "$implementation" "$state" "$log" \
        "$https_port" "$environment_address" "$environment_port" "$command_address" "$command_port"
      local base_url="http://127.0.0.1:$expected_port"
      wait_for_options "$base_url" "$work_dir/$target-web-$implementation-$precedence-options.json" "$log"
      wait_for_tcp_endpoint_state 127.0.0.1 "$expected_port" open "$log"
      capture_web_listener_stage "$target" "$base_url" "$suite" "precedence-$precedence" \
        "$expected_address" "$expected_port" "$expected_address" "$expected_port" false \
        "$expected_port" "$inactive_port" true false \
        "$([[ "$target" == slskd && "$precedence" == default ]] && printf true || printf false)"
      stop_daemon
    done

    local state="$work_dir/state-$target-web-$implementation-lifecycle"
    local log="$work_dir/$target-web-$implementation-lifecycle.log"
    local https_port="$(pick_free_port)"
    local old_port="$(pick_free_port)"
    local new_port="$(pick_free_port)"
    local old_address=127.0.0.1
    [[ "$target" == slskd ]] && old_address='127.0.0.1,::1'
    local new_address=0.0.0.0
    [[ "$target" == slskdn ]] && new_address='*'
    mkdir -p "$state"
    write_web_listener_yaml "$target" "$state/slskd.yml" "$old_address" "$old_port"
    start_web_listener_daemon "$target" "$root" "$implementation" "$state" "$log" "$https_port"
    local old_base="http://127.0.0.1:$old_port"
    wait_for_options "$old_base" "$work_dir/$target-web-$implementation-lifecycle-options.json" "$log"
    wait_for_tcp_endpoint_state 127.0.0.1 "$old_port" open "$log"
    [[ "$target" == slskd ]] && wait_for_tcp_endpoint_state ::1 "$old_port" open "$log"
    capture_web_listener_stage "$target" "$old_base" "$suite" lifecycle-startup \
      "$old_address" "$old_port" "$old_address" "$old_port" false "$old_port" "$new_port" \
      true false "$([[ "$target" == slskd ]] && printf true || printf false)"

    write_web_listener_yaml "$target" "$state/slskd.yml" "$new_address" "$new_port"
    wait_for_web_listener_option "$target" "$old_base" "$new_address" "$new_port" "$log"
    wait_for_tcp_endpoint_state 127.0.0.1 "$old_port" open "$log"
    wait_for_tcp_endpoint_state 127.0.0.1 "$new_port" closed "$log"
    capture_web_listener_stage "$target" "$old_base" "$suite" lifecycle-watched \
      "$new_address" "$new_port" "$old_address" "$old_port" true "$old_port" "$new_port" \
      true false "$([[ "$target" == slskd ]] && printf true || printf false)"

    stop_daemon
    start_web_listener_daemon "$target" "$root" "$implementation" "$state" "$log" "$https_port" '' '' '' '' true
    local new_base="http://127.0.0.1:$new_port"
    wait_for_options "$new_base" "$work_dir/$target-web-$implementation-lifecycle-restart-options.json" "$log"
    wait_for_tcp_endpoint_state 127.0.0.1 "$old_port" closed "$log"
    wait_for_tcp_endpoint_state 127.0.0.1 "$new_port" open "$log"
    capture_web_listener_stage "$target" "$new_base" "$suite" lifecycle-restarted \
      "$new_address" "$new_port" "$new_address" "$new_port" false "$old_port" "$new_port" \
      false true false

    local validation_cases=(
      $'port-zero|web:\n  port: 0\n'
      $'port-one|web:\n  port: 1\n'
      $'port-high|web:\n  port: 65536\n'
      $'port-text|web:\n  port: nope\n'
      $'port-bool|web:\n  port: true\n'
      $'port-null|web:\n  port: null\n'
    )
    if [[ "$target" == slskd ]]; then
      validation_cases+=(
        $'ip-invalid|web:\n  ip_address: nope\n'
        $'ip-comma|web:\n  ip_address: 127.0.0.1,::1\n'
        $'ip-empty|web:\n  ip_address: ""\n'
        $'ip-null|web:\n  ip_address: null\n'
        $'ip-number|web:\n  ip_address: 123\n'
        $'ip-bool|web:\n  ip_address: true\n'
      )
    fi
    for item in "${validation_cases[@]}"; do
      local label="${item%%|*}"
      local yaml="${item#*|}"
      local payload="$($python_bin -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml")"
      capture_request "$suite" "web-validation-$label" POST \
        "$new_base/api/v0/options/yaml/validate" "$payload"
    done
    stop_daemon

    local conflict_state="$work_dir/state-$target-web-$implementation-conflict"
    local conflict_log="$work_dir/$target-web-$implementation-conflict.log"
    local conflict_port="$(pick_free_port)"
    local conflict_status="$work_dir/$target-web-$implementation-conflict.status"
    mkdir -p "$conflict_state"
    write_web_listener_yaml "$target" "$conflict_state/slskd.yml" 127.0.0.1 "$conflict_port"
    start_listener_blocker 127.0.0.1 "$conflict_port" "$conflict_status"
    start_web_listener_daemon "$target" "$root" "$implementation" "$conflict_state" \
      "$conflict_log" "$(pick_free_port)"
    local exited=false
    for _ in $(seq 1 200); do
      if ! kill -0 "$daemon_pid" 2>/dev/null; then
        exited=true
        break
      fi
      sleep 0.05
    done
    if [[ "$exited" != true ]]; then
      printf 'web listener differential failed: occupied-port startup did not exit for %s/%s\n' \
        "$target" "$implementation" >&2
      tail -120 "$conflict_log" >&2 || true
      exit 1
    fi
    local exit_code=0
    wait "$daemon_pid" || exit_code="$?"
    daemon_pid=''
    local http_unavailable=true
    if curl --silent --max-time 0.2 "http://127.0.0.1:$conflict_port/api/health" >/dev/null 2>&1; then
      http_unavailable=false
    fi
    "$python_bin" - "$exit_code" "$http_unavailable" >"$suite/web-bind-conflict.body" <<'PY'
import json,sys
print(json.dumps({
    "exitCode": int(sys.argv[1]),
    "httpUnavailable": sys.argv[2] == "true",
}, sort_keys=True, separators=(",",":")))
PY
    printf 'status=200\ncontent-type=application/json\n' >"$suite/web-bind-conflict.meta"
    stop_listener_blocker
  done

  local upstream_normalized="$work_dir/$target-web-listener-upstream.normalized"
  local slskr_normalized="$work_dir/$target-web-listener-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-web-listener-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-web-listener-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'web listener differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s web listener differential passed\n' "$target"
}

write_integration_yaml() {
  local path="$1"
  local spotify_client_id="$2"
  local spotify_max_items="$3"
  local spotify_market="$4"
  local lidarr_max_items="$5"
  local temporary="$path.tmp"
  printf 'flags:\n  no_connect: true\nintegrations:\n  spotify:\n    enabled: true\n    client_id: "%s"\n    client_secret: "fixture-secret"\n    redirect_uri: "http://127.0.0.1/spotify-callback"\n    timeout_seconds: 21\n    max_items_per_import: %s\n    market: "%s"\n  youtube:\n    enabled: true\n    api_key: "fixture-youtube-key"\n  lastfm:\n    enabled: true\n    api_key: "fixture-lastfm-key"\n  ntfy:\n    enabled: false\n    url: "https://ntfy.sh/fixture"\n    access_token: "fixture-ntfy-token"\n    notification_prefix: "Fixture Ntfy"\n    notify_on_private_message: false\n    notify_on_room_mention: false\n  pushover:\n    enabled: false\n    user_key: "fixture-user-key"\n    token: "fixture-pushover-token"\n    notification_prefix: "Fixture Pushover"\n    notify_on_private_message: false\n    notify_on_room_mention: false\n  pushbullet:\n    enabled: false\n    access_token: "fixture-pushbullet-token"\n    notification_prefix: "Fixture Pushbullet"\n    notify_on_private_message: false\n    notify_on_room_mention: false\n    retry_attempts: 5\n    cooldown_time: 1234\n  ftp:\n    enabled: true\n    address: "ftp.example"\n    port: 2121\n    encryption_mode: "explicit"\n    ignore_certificate_errors: true\n    username: "fixture-ftp-user"\n    password: "fixture-ftp-password"\n    remote_path: "/incoming"\n    overwrite_existing: false\n    connection_timeout: 4321\n    retry_attempts: 5\n  webhooks:\n    my_webhook:\n      on: [Any, PrivateMessageReceived]\n      call:\n        url: "https://example.com/hook"\n        headers:\n          - name: Authorization\n            value: "fixture-header-secret"\n        ignore_certificate_errors: false\n      timeout: 1234\n      retry:\n        attempts: 2\n  lidarr:\n    enabled: false\n    url: "http://127.0.0.1:65534"\n    api_key: "fixture-key"\n    timeout_seconds: 22\n    sync_wanted_to_wishlist: true\n    sync_interval_seconds: 600\n    max_items_per_sync: %s\n    auto_download: true\n    wishlist_filter: "lossless"\n    wishlist_max_results: 44\n    auto_import_completed: true\n    import_path_from: "/downloads"\n    import_path_to: "/lidarr"\n    import_mode: "copy"\n    import_replace_existing_files: true\n' \
    "$spotify_client_id" "$spotify_max_items" "$spotify_market" "$lidarr_max_items" >"$temporary"
  "$python_bin" - "$temporary" <<'PY'
from pathlib import Path
import sys
path = Path(sys.argv[1])
text = path.read_text(encoding="utf-8")
text = text.replace(
    "  webhooks:\n",
    "  scripts:\n"
    "    fixture_event:\n"
    "      on: [Noop]\n"
    "      run:\n"
    "        executable: /bin/sh\n"
    "        arglist:\n"
    "          - -c\n"
    "          - 'printf %s \"$SLSKD_SCRIPT_DATA\" > script-event.json'\n"
    "  vpn:\n"
    "    enabled: false\n"
    "    port_forwarding: true\n"
    "    polling_interval: 3456\n"
    "    gluetun:\n"
    "      url: http://127.0.0.1:8000\n"
    "      timeout: 2345\n"
    "      auth: ignored-documentation-leaf\n"
    "      username: fixture-vpn-user\n"
    "      password: fixture-vpn-password\n"
    "      api_key: fixture-vpn-api-key\n"
    "  webhooks:\n",
    1,
)
path.write_text(text, encoding="utf-8")
PY
  mv "$temporary" "$path"
}

wait_for_integration_option() {
  local base_url="$1"
  local expected_market="$2"
  local expected_max_items="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["integration"]["spotify"]; raise SystemExit(0 if value["market"] == sys.argv[1] and value["maxItemsPerImport"] == int(sys.argv[2]) else 1)' \
        "$expected_market" "$expected_max_items" 2>/dev/null; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'integration differential failed: daemon exited while waiting for watched options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'integration differential failed: timed out waiting for %s/%s\n' \
    "$expected_market" "$expected_max_items" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_integration_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local current="$work_dir/integration-current-$$.json"
  local startup="$work_dir/integration-startup-$$.json"
  local application="$work_dir/integration-application-$$.json"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application"
  "$python_bin" - "$current" "$startup" "$application" >"$suite/stage-$stage.body" <<'PY'
import json,sys
current=json.load(open(sys.argv[1],encoding="utf-8"))
startup=json.load(open(sys.argv[2],encoding="utf-8"))
application=json.load(open(sys.argv[3],encoding="utf-8"))
print(json.dumps({
    "current": current["integration"],
    "startup": startup["integration"],
    "pendingRestart": application["pendingRestart"],
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/stage-$stage.meta"
  rm -f "$current" "$startup" "$application"
}

normalize_integration_suite() {
  local source="$1"
  local destination="$2"
  "$python_bin" - "$source" "$destination" <<'PY'
import json,pathlib,shutil,sys,urllib.parse
source=pathlib.Path(sys.argv[1]); destination=pathlib.Path(sys.argv[2])
destination.mkdir(parents=True,exist_ok=True)
def normalize(value):
    if isinstance(value,list):
        return [normalize(item) for item in value]
    if not isinstance(value,dict):
        return value
    result={key:normalize(item) for key,item in value.items()}
    if "importId" in result:
        result["importId"]="<import-id>"
    if "importedAt" in result:
        result["importedAt"]="<timestamp>"
    for key in ("lastSyncAt","nextSyncAt"):
        if result.get(key) is not None:
            result[key]="<timestamp>"
    for key in ("authorizationUrl","authorization_url"):
        url=result.get(key)
        if not isinstance(url,str):
            continue
        parsed=urllib.parse.urlsplit(url)
        query=urllib.parse.parse_qsl(parsed.query,keep_blank_values=True)
        query=[(name,"<state>" if name=="state" else "<challenge>" if name=="code_challenge" else item) for name,item in query]
        result[key]=urllib.parse.urlunsplit((parsed.scheme,parsed.netloc,parsed.path,urllib.parse.urlencode(query),parsed.fragment))
    if "state" in result:
        result["state"]="<state>"
    return result
for path in source.iterdir():
    target=destination/path.name
    if path.suffix != ".body":
        shutil.copy2(path,target); continue
    text=path.read_text(encoding="utf-8")
    try: value=json.loads(text)
    except json.JSONDecodeError:
        target.write_text(text,encoding="utf-8"); continue
    target.write_text(json.dumps(normalize(value),sort_keys=True,separators=(",",":")),encoding="utf-8")
PY
}

capture_integration_script_event() {
  local base_url="$1"
  local state="$2"
  local suite="$3"
  local output="$state/scripts/script-event.json"
  rm -f "$output"
  curl --fail --silent --max-time 2 -H 'Content-Type: application/json' \
    -X POST --data '"fixture-"' "$base_url/api/v0/events/Noop" >/dev/null
  for _ in $(seq 1 200); do
    [[ -f "$output" ]] && break
    sleep 0.01
  done
  if [[ ! -f "$output" ]]; then
    printf 'integration script differential failed: event output was not created\n' >&2
    return 1
  fi
  "$python_bin" - "$output" >"$suite/script-event.body" <<'PY'
import json,sys
value=json.load(open(sys.argv[1],encoding="utf-8"))
print(json.dumps({
    "type": value.get("type"),
    "version": value.get("version"),
    "hasId": isinstance(value.get("id"),str) and bool(value["id"]),
    "hasTimestamp": isinstance(value.get("timestamp"),str) and bool(value["timestamp"]),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/script-event.meta"
}

run_script_scenario() {
  local target="$1"
  local root="$2"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  local integration_key=integrations
  [[ "$target" == slskdn ]] && integration_key=integration

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-scripts-$implementation"
    local suite="$work_dir/$target-scripts-$implementation"
    local log="$work_dir/$target-scripts-$implementation.log"
    mkdir -p "$state" "$suite"
    printf 'flags:\n  no_connect: true\nintegrations:\n  scripts:\n    fixture_event:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        arglist:\n          - -c\n          - '\''printf %%s "$SLSKD_SCRIPT_DATA" > script-event.json'\''\n' >"$state/slskd.yml"
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target" SLSKD_NO_AUTH=true
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port"
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-scripts-$implementation-options.json" "$log"
    local current="$work_dir/$target-scripts-$implementation-current.json"
    local startup="$work_dir/$target-scripts-$implementation-startup.json"
    curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
    curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
    "$python_bin" - "$current" "$startup" "$integration_key" >"$suite/options.body" <<'PY'
import json,sys
current=json.load(open(sys.argv[1],encoding="utf-8"))
startup=json.load(open(sys.argv[2],encoding="utf-8"))
key=sys.argv[3]
print(json.dumps({"current":current[key]["scripts"],"startup":startup[key]["scripts"]},sort_keys=True,separators=(",",":")))
PY
    printf 'status=200\ncontent-type=application/json\n' >"$suite/options.meta"
    capture_integration_script_event "$base_url" "$state" "$suite"
    for validation in \
      $'valid-command|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        command: echo fixture\n' \
      $'valid-args|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        args: -c true\n' \
      $'invalid-event|integrations:\n  scripts:\n    fixture:\n      on: [NotAnEvent]\n      run:\n        command: echo fixture\n' \
      $'missing-mode|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run: {}\n' \
      $'both-modes|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        command: echo fixture\n        executable: /bin/sh\n' \
      $'args-conflict|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        args: -c true\n        arglist: [-c, true]\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "validation-$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$("$python_bin" -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml")"
    done
    stop_daemon
  done

  normalize_integration_suite "$work_dir/$target-scripts-upstream" "$work_dir/$target-scripts-upstream.normalized"
  normalize_integration_suite "$work_dir/$target-scripts-slskr" "$work_dir/$target-scripts-slskr.normalized"
  if ! diff -ru "$work_dir/$target-scripts-upstream.normalized" "$work_dir/$target-scripts-slskr.normalized"; then
    printf 'script integration differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s script integration differential passed\n' "$target"
}

run_integration_scenario() {
  local root="$1"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"

  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-integrations-$implementation"
    local suite="$work_dir/slskdn-integrations-$implementation"
    local log="$work_dir/slskdn-integrations-$implementation.log"
    mkdir -p "$state" "$suite"
    write_integration_yaml "$state/slskd.yml" yaml-client 2 CA 7
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKD_NO_AUTH=true
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" \
          --slsk-listen-port "$listen_port"
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-integrations-$implementation-options.json" "$log"
    capture_integration_stage "$base_url" "$suite" startup
    capture_get "$suite" spotify-status-startup "$base_url/api/v0/integrations/spotify/status"
    capture_request "$suite" spotify-authorize POST "$base_url/api/v0/integrations/spotify/authorize" '{}'
    capture_request "$suite" preview-startup POST "$base_url/api/v0/source-feed-imports/preview" \
      '{"sourceText":"Artist A - Track A\nArtist B - Track B\nArtist C - Track C","sourceKind":"auto","fetchProviderUrls":false,"limit":10}'
    capture_get "$suite" history-startup "$base_url/api/v0/source-feed-imports/history?limit=1"
    local import_id
    import_id="$(curl --fail --silent --max-time 2 "$base_url/api/v0/source-feed-imports/history?limit=1" | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)[0]["importId"])')"
    capture_get "$suite" history-detail "$base_url/api/v0/source-feed-imports/history/$import_id"
    capture_integration_script_event "$base_url" "$state" "$suite"

    for validation in \
      $'valid|integrations:\n  spotify:\n    enabled: true\n    client_id: test\n    timeout_seconds: 1\n    max_items_per_import: 5000\n    market: CA\n  lidarr:\n    enabled: true\n    url: http://127.0.0.1:8686\n    api_key: key\n    timeout_seconds: 120\n    sync_interval_seconds: 300\n    max_items_per_sync: 1000\n    wishlist_max_results: 1000\n    import_mode: Copy\n' \
      $'spotify-missing-client|integrations:\n  spotify:\n    enabled: true\n    market: CA\n' \
      $'spotify-market|integrations:\n  spotify:\n    market: CAN\n' \
      $'spotify-timeout-low|integrations:\n  spotify:\n    timeout_seconds: 0\n' \
      $'spotify-timeout-high|integrations:\n  spotify:\n    timeout_seconds: 121\n' \
      $'spotify-max-low|integrations:\n  spotify:\n    max_items_per_import: 0\n' \
      $'spotify-max-high|integrations:\n  spotify:\n    max_items_per_import: 5001\n' \
      $'youtube-missing-key|integrations:\n  youtube:\n    enabled: true\n' \
      $'lastfm-missing-key|integrations:\n  lastfm:\n    enabled: true\n' \
      $'ntfy-missing-url|integrations:\n  ntfy:\n    enabled: true\n' \
      $'pushover-missing-keys|integrations:\n  pushover:\n    enabled: true\n' \
      $'pushover-missing-token|integrations:\n  pushover:\n    enabled: true\n    user_key: key\n' \
      $'pushbullet-missing-token|integrations:\n  pushbullet:\n    enabled: true\n' \
      $'pushbullet-retry-low|integrations:\n  pushbullet:\n    retry_attempts: -1\n' \
      $'pushbullet-retry-high|integrations:\n  pushbullet:\n    retry_attempts: 6\n' \
      $'ftp-missing-address|integrations:\n  ftp:\n    enabled: true\n' \
      $'ftp-port-low|integrations:\n  ftp:\n    port: 0\n' \
      $'ftp-encryption-mode|integrations:\n  ftp:\n    encryption_mode: invalid\n' \
      $'ftp-timeout-low|integrations:\n  ftp:\n    connection_timeout: -1\n' \
      $'ftp-retry-high|integrations:\n  ftp:\n    retry_attempts: 6\n' \
      $'vpn-valid|integrations:\n  vpn:\n    enabled: true\n    port_forwarding: true\n    polling_interval: 500\n    gluetun:\n      url: http://127.0.0.1:8000\n      timeout: 10000\n' \
      $'vpn-missing-client|integrations:\n  vpn:\n    enabled: true\n' \
      $'vpn-relative-url|integrations:\n  vpn:\n    enabled: true\n    gluetun:\n      url: relative\n' \
      $'vpn-poll-low|integrations:\n  vpn:\n    polling_interval: 499\n' \
      $'vpn-timeout-low|integrations:\n  vpn:\n    gluetun:\n      timeout: 499\n' \
      $'vpn-timeout-high|integrations:\n  vpn:\n    gluetun:\n      timeout: 10001\n' \
      $'script-valid-command|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        command: echo fixture\n' \
      $'script-valid-args|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        args: -c true\n' \
      $'script-valid-arglist|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        arglist: [-c, true]\n' \
      $'script-invalid-event|integrations:\n  scripts:\n    fixture:\n      on: [NotAnEvent]\n      run:\n        command: echo fixture\n' \
      $'script-missing-mode|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run: {}\n' \
      $'script-both-modes|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        command: echo fixture\n        executable: /bin/sh\n' \
      $'script-args-conflict|integrations:\n  scripts:\n    fixture:\n      on: [Noop]\n      run:\n        executable: /bin/sh\n        args: -c true\n        arglist: [-c, true]\n' \
      $'webhook-url|integrations:\n  webhooks:\n    fixture:\n      on: [Any]\n      call:\n        url: relative\n' \
      $'webhook-timeout|integrations:\n  webhooks:\n    fixture:\n      on: [Any]\n      call:\n        url: https://example.com/hook\n      timeout: 499\n' \
      $'webhook-attempts|integrations:\n  webhooks:\n    fixture:\n      on: [Any]\n      call:\n        url: https://example.com/hook\n      retry:\n        attempts: 0\n' \
      $'lidarr-missing-url-key|integrations:\n  lidarr:\n    enabled: true\n' \
      $'lidarr-timeout-low|integrations:\n  lidarr:\n    timeout_seconds: 0\n' \
      $'lidarr-interval-low|integrations:\n  lidarr:\n    sync_interval_seconds: 299\n' \
      $'lidarr-max-high|integrations:\n  lidarr:\n    max_items_per_sync: 1001\n' \
      $'lidarr-results-low|integrations:\n  lidarr:\n    wishlist_max_results: 9\n' \
      $'lidarr-path-pair|integrations:\n  lidarr:\n    enabled: true\n    url: http://127.0.0.1:8686\n    api_key: key\n    auto_import_completed: true\n    import_path_from: /downloads\n' \
      $'lidarr-mode|integrations:\n  lidarr:\n    enabled: true\n    url: http://127.0.0.1:8686\n    api_key: key\n    import_mode: link\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "validation-$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml")"
    done

    write_integration_yaml "$state/slskd.yml" watched-client 3 GB 8
    wait_for_integration_option "$base_url" GB 3 "$log"
    capture_integration_stage "$base_url" "$suite" watched
    capture_get "$suite" spotify-status-watched "$base_url/api/v0/integrations/spotify/status"
    capture_request "$suite" preview-watched POST "$base_url/api/v0/source-feed-imports/preview" \
      '{"sourceText":"One\nTwo\nThree\nFour","sourceKind":"text","fetchProviderUrls":false,"limit":10}'
    stop_daemon

    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKD_NO_AUTH=true
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" \
          --slsk-listen-port "$listen_port"
      ) >>"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-integrations-$implementation-restart.json" "$log"
    capture_integration_stage "$base_url" "$suite" restarted
    capture_get "$suite" history-restarted "$base_url/api/v0/source-feed-imports/history?limit=2"
    stop_daemon

    printf 'flags:\n  no_connect: true\n' >"$state/slskd.yml"
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        export SLSKD_SPOTIFY=true SLSKD_SPOTIFY_CLIENT_ID=environment-client
        export SLSKD_SPOTIFY_CLIENT_SECRET=environment-secret
        export SLSKD_SPOTIFY_REDIRECT_URI=http://127.0.0.1/environment-callback
        export SLSKD_SPOTIFY_TIMEOUT=23 SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT=4 SLSKD_SPOTIFY_MARKET=DE
        export SLSKD_LIDARR=false SLSKD_LIDARR_URL=http://127.0.0.1:65533 SLSKD_LIDARR_API_KEY=environment-key
        export SLSKD_LIDARR_TIMEOUT=24 SLSKD_LIDARR_SYNC_WANTED=true SLSKD_LIDARR_SYNC_INTERVAL=650
        export SLSKD_LIDARR_SYNC_MAX_ITEMS=9 SLSKD_LIDARR_AUTO_DOWNLOAD=true
        export SLSKD_LIDARR_WISHLIST_FILTER=environment SLSKD_LIDARR_WISHLIST_MAX_RESULTS=45
        export SLSKD_LIDARR_AUTO_IMPORT_COMPLETED=true SLSKD_LIDARR_IMPORT_PATH_FROM=/environment
        export SLSKD_LIDARR_IMPORT_PATH_TO=/lidarr-environment SLSKD_LIDARR_IMPORT_MODE=copy
        export SLSKD_LIDARR_IMPORT_REPLACE_EXISTING=true
        export SLSKD_VPN=false SLSKD_VPN_PORT_FORWARDING=true SLSKD_VPN_POLLING_INTERVAL=4567
        export SLSKD_VPN_GLUETUN_URL=http://127.0.0.1:8100 SLSKD_VPN_GLUETUN_TIMEOUT=3456
        export SLSKD_VPN_GLUETUN_USERNAME=environment-vpn-user SLSKD_VPN_GLUETUN_PASSWORD=environment-vpn-password
        export SLSKD_VPN_GLUETUN_API_KEY=environment-vpn-api-key
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKD_NO_AUTH=true
        export SLSKD_SPOTIFY=true SLSKD_SPOTIFY_CLIENT_ID=environment-client
        export SLSKD_SPOTIFY_CLIENT_SECRET=environment-secret
        export SLSKD_SPOTIFY_REDIRECT_URI=http://127.0.0.1/environment-callback
        export SLSKD_SPOTIFY_TIMEOUT=23 SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT=4 SLSKD_SPOTIFY_MARKET=DE
        export SLSKD_LIDARR=false SLSKD_LIDARR_URL=http://127.0.0.1:65533 SLSKD_LIDARR_API_KEY=environment-key
        export SLSKD_LIDARR_TIMEOUT=24 SLSKD_LIDARR_SYNC_WANTED=true SLSKD_LIDARR_SYNC_INTERVAL=650
        export SLSKD_LIDARR_SYNC_MAX_ITEMS=9 SLSKD_LIDARR_AUTO_DOWNLOAD=true
        export SLSKD_LIDARR_WISHLIST_FILTER=environment SLSKD_LIDARR_WISHLIST_MAX_RESULTS=45
        export SLSKD_LIDARR_AUTO_IMPORT_COMPLETED=true SLSKD_LIDARR_IMPORT_PATH_FROM=/environment
        export SLSKD_LIDARR_IMPORT_PATH_TO=/lidarr-environment SLSKD_LIDARR_IMPORT_MODE=copy
        export SLSKD_LIDARR_IMPORT_REPLACE_EXISTING=true
        export SLSKD_VPN=false SLSKD_VPN_PORT_FORWARDING=true SLSKD_VPN_POLLING_INTERVAL=4567
        export SLSKD_VPN_GLUETUN_URL=http://127.0.0.1:8100 SLSKD_VPN_GLUETUN_TIMEOUT=3456
        export SLSKD_VPN_GLUETUN_USERNAME=environment-vpn-user SLSKD_VPN_GLUETUN_PASSWORD=environment-vpn-password
        export SLSKD_VPN_GLUETUN_API_KEY=environment-vpn-api-key
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" \
          --slsk-listen-port "$listen_port"
      ) >>"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-integrations-$implementation-environment.json" "$log"
    capture_integration_stage "$base_url" "$suite" environment
    capture_request "$suite" preview-environment POST "$base_url/api/v0/source-feed-imports/preview" \
      '{"sourceText":"One\nTwo\nThree\nFour\nFive","sourceKind":"text","fetchProviderUrls":false,"limit":10}'
    stop_daemon

    write_integration_yaml "$state/slskd.yml" yaml-client 2 CA 7
    local cli_args=(
      --spotify --spotify-client-id cli-client --spotify-client-secret cli-secret
      --spotify-redirect-uri http://127.0.0.1/cli-callback --spotify-timeout 25
      --spotify-max-items-per-import 5 --spotify-market FR
      --lidarr --lidarr-url http://127.0.0.1:65532 --lidarr-api-key cli-key --lidarr-timeout 26
      --lidarr-sync-wanted --lidarr-sync-interval 700 --lidarr-sync-max-items 10
      --lidarr-auto-download --lidarr-wishlist-filter cli --lidarr-wishlist-max-results 46
      --lidarr-auto-import-completed --lidarr-import-path-from /cli --lidarr-import-path-to /lidarr-cli
      --lidarr-import-mode move --lidarr-import-replace-existing
      --vpn-port-forwarding --vpn-polling-interval 5678 --vpn-gluetun-url http://127.0.0.1:8200
      --vpn-gluetun-timeout 4567 --vpn-gluetun-username cli-vpn-user
      --vpn-gluetun-password cli-vpn-password --vpn-gluetun-api-key cli-vpn-api-key
    )
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port" SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT=4
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll" "${cli_args[@]}"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKD_NO_AUTH=true
        export SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT=4
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" \
          --slsk-listen-port "$listen_port" "${cli_args[@]}"
      ) >>"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-integrations-$implementation-cli.json" "$log"
    capture_integration_stage "$base_url" "$suite" command-line
    capture_request "$suite" preview-command-line POST "$base_url/api/v0/source-feed-imports/preview" \
      '{"sourceText":"One\nTwo\nThree\nFour\nFive\nSix","sourceKind":"text","fetchProviderUrls":false,"limit":10}'
    stop_daemon
  done

  local upstream_normalized="$work_dir/slskdn-integrations-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-integrations-slskr.normalized"
  normalize_integration_suite "$work_dir/slskdn-integrations-upstream" "$upstream_normalized"
  normalize_integration_suite "$work_dir/slskdn-integrations-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'integration differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn integration differential passed\n'
}

write_lidarr_runtime_yaml() {
  local path="$1"
  local lidarr_url="$2"
  local temporary="$path.tmp"
  printf 'flags:\n  no_connect: true\ndht:\n  enabled: false\nintegrations:\n  lidarr:\n    enabled: true\n    url: "%s"\n    api_key: "fixture-key"\n    timeout_seconds: 5\n    sync_wanted_to_wishlist: false\n    sync_interval_seconds: 600\n    max_items_per_sync: 2\n    auto_download: true\n    wishlist_filter: "lossless"\n    wishlist_max_results: 44\n    auto_import_completed: true\n    import_path_from: "/downloads"\n    import_path_to: "/lidarr"\n    import_mode: "copy"\n    import_replace_existing_files: true\n' "$lidarr_url" >"$temporary"
  mv "$temporary" "$path"
}

capture_lidarr_wishlist_policy() {
  local base_url="$1"
  local suite="$2"
  local raw="$work_dir/lidarr-wishlist-$$.json"
  curl --fail --silent --max-time 5 "$base_url/api/v0/wishlist" >"$raw"
  "$python_bin" - "$raw" >"$suite/wishlist-policy.body" <<'PY'
import json,sys
rows=json.load(open(sys.argv[1],encoding="utf-8"))
projected=[{
    "searchText":row["searchText"],
    "filter":row["filter"],
    "enabled":row["enabled"],
    "autoDownload":row["autoDownload"],
    "maxResults":row["maxResults"],
} for row in rows]
print(json.dumps(sorted(projected,key=lambda row:row["searchText"]),sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/wishlist-policy.meta"
  rm -f "$raw"
}

run_lidarr_runtime_scenario() {
  local root="$1"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local fixture_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  local fixture_url="http://127.0.0.1:$fixture_port"
  local fixture_log="$work_dir/lidarr-fixture.log"

  "$python_bin" "$repo_root/scripts/fixture-lidarr.py" --port "$fixture_port" >"$fixture_log" 2>&1 &
  lidarr_fixture_pid="$!"
  for _ in $(seq 1 100); do
    curl --fail --silent --max-time 1 "$fixture_url/__status" >/dev/null 2>&1 && break
    if ! kill -0 "$lidarr_fixture_pid" 2>/dev/null; then
      printf 'Lidarr fixture exited before becoming ready\n' >&2
      cat "$fixture_log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  curl --fail --silent --max-time 1 "$fixture_url/__status" >/dev/null

  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-lidarr-runtime-$implementation"
    local suite="$work_dir/slskdn-lidarr-runtime-$implementation"
    local log="$work_dir/slskdn-lidarr-runtime-$implementation.log"
    mkdir -p "$state" "$suite"
    curl --fail --silent --max-time 2 --request POST "$fixture_url/__reset" >/dev/null
    write_lidarr_runtime_yaml "$state/slskd.yml" "$fixture_url"
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKD_NO_AUTH=true
        exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" \
          --slsk-listen-port "$listen_port"
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-lidarr-runtime-$implementation-options.json" "$log"
    capture_get "$suite" status "$base_url/api/v0/integrations/lidarr/status"
    capture_get "$suite" sync-status-before "$base_url/api/v0/integrations/lidarr/sync/status"
    capture_get "$suite" wanted "$base_url/api/v0/integrations/lidarr/wanted/missing?page=2&pageSize=2"
    capture_request "$suite" sync POST "$base_url/api/v0/integrations/lidarr/wanted/sync" '{}'
    capture_lidarr_wishlist_policy "$base_url" "$suite"
    capture_request "$suite" manual-import POST "$base_url/api/v0/integrations/lidarr/manualimport" \
      '{"directory":"/downloads/Fixture Album"}'
    capture_get "$suite" sync-status-after "$base_url/api/v0/integrations/lidarr/sync/status"
    capture_get "$suite" fixture-requests "$fixture_url/__status"
    stop_daemon
  done
  stop_lidarr_fixture

  local upstream_normalized="$work_dir/slskdn-lidarr-runtime-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-lidarr-runtime-slskr.normalized"
  normalize_integration_suite "$work_dir/slskdn-lidarr-runtime-upstream" "$upstream_normalized"
  normalize_integration_suite "$work_dir/slskdn-lidarr-runtime-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'Lidarr runtime differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn Lidarr runtime differential passed\n'
}

write_daemon_foundation_yaml() {
  local target="$1" path="$2" socket="$3" content="$4" pfx="$5"
  local target_only=""
  if [[ "$target" == slskdn ]]; then
    target_only=$'permissions:\n  file:\n    mode: "0640"\ntelemetry:\n  tracing:\n    enabled: false\n    exporter: console\n    jaeger_endpoint: collector.example\n    jaeger_port: 4318\n    otlp_endpoint: https://otlp.example\nfilters:\n  search_retention:\n    max_age_days: 4\n    max_count: 77\n    cleanup_interval_seconds: 3600\n'
  fi
  local failed=""
  if [[ "$target" == slskd ]]; then failed=', failed: 8'; fi
  printf '%s' "flags:
  no_connect: true
  force_migrations: true
  legacy_windows_tcp_keepalive: true
  log_sql: true
  log_unobserved_exceptions: true
  optimistic_relay_file_info: true
  volatile: true
logger:
  disk: true
  no_color: true
retention:
  search: 10
  logs: 9
  files:
    complete: 30
    incomplete: 31
  transfers:
    upload: {succeeded: 5, errored: 6, cancelled: 7$failed}
    download: {succeeded: 9, errored: 10, cancelled: 11$failed}
${target_only}web:
  socket: '$socket'
  url_base: /slsk
  content_path: '$content'
  logging: true
  https:
    disabled: false
    force: false
    certificate:
      pfx: '$pfx'
      password: foundation-password
  authentication:
    disabled: true
    api_keys:
      operator:
        key: 0123456789abcdef
        role: readwrite
        cidr: 127.0.0.1/32
" >"$path"
}

capture_daemon_foundation() {
  local target="$1" base_url="$2" https_url="$3" socket="$4" suite="$5"
  "$python_bin" - "$base_url" "$target" >"$suite/options.body" <<'PY'
import json,os,sys,urllib.request
with urllib.request.urlopen(sys.argv[1] + "/api/v0/options", timeout=5) as response:
    value=json.load(response)
target=sys.argv[2]
result={
 "flags":{key:value["flags"][key] for key in ["forceMigrations","legacyWindowsTcpKeepalive","logSQL","logUnobservedExceptions","optimisticRelayFileInfo","volatile"]},
 "logger":value["logger"], "permissions":value.get("permissions"), "retention":value["retention"],
 "web":{key:(os.path.basename(value["web"][key]) if key == "socket" else value["web"][key]) for key in ["socket","urlBase","contentPath","logging","https"]},
 "apiKeys":value["web"]["authentication"]["apiKeys"],
}
certificate=result["web"]["https"].get("certificate",{})
if certificate.get("pfx"): certificate["pfx"]=os.path.basename(certificate["pfx"])
if target == "slskdn": result["telemetry"]=value["telemetry"]
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/options.meta"
  curl --fail --silent --show-error --max-time 5 --insecure "$https_url/api/v0/options" \
    | "$python_bin" -c 'import json,sys; json.load(sys.stdin); print("{\"reachable\":true}")' \
    >"$suite/https-health.body"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/https-health.meta"
  local unix_status
  unix_status="$(curl --silent --show-error --max-time 5 --output /dev/null \
    --write-out '%{http_code}' --unix-socket "$socket" \
    http://localhost/slsk/api/v0/options)"
  printf '{"status":%s}\n' "$unix_status" >"$suite/unix-health.body"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/unix-health.meta"
}

run_daemon_foundation_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port/slsk"
  local https_url="https://127.0.0.1:$https_port/slsk"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-daemon-foundation-$implementation"
    local suite="$work_dir/$target-daemon-foundation-$implementation"
    local log="$work_dir/$target-daemon-foundation-$implementation.log"
    local socket="$state/slskd.sock" content="wwwroot"
    local pfx="$state/foundation.pfx"
    mkdir -p "$state" "$suite"
    openssl req -x509 -newkey rsa:2048 -nodes -subj '/CN=localhost' \
      -keyout "$state/foundation.key" -out "$state/foundation.crt" -days 1 >/dev/null 2>&1
    openssl pkcs12 -export -out "$pfx" -inkey "$state/foundation.key" \
      -in "$state/foundation.crt" -passout pass:foundation-password >/dev/null 2>&1
    write_daemon_foundation_yaml "$target" "$state/slskd.yml" "$socket" "$content" "$pfx"
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      mkdir -p "$(dirname "$dll")/wwwroot"
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      mkdir -p "$repo_root/target/debug/wwwroot"
      printf '<html>foundation</html>' >"$repo_root/target/debug/wwwroot/index.html"
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        exec "$repo_root/target/debug/slskr" serve
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-daemon-foundation-$implementation-options.json" "$log"
    capture_daemon_foundation "$target" "$base_url" "$https_url" "$socket" "$suite"
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-daemon-foundation-upstream" "$work_dir/$target-daemon-foundation-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-daemon-foundation-slskr" "$work_dir/$target-daemon-foundation-slskr.normalized"
  if ! diff -ru "$work_dir/$target-daemon-foundation-upstream.normalized" "$work_dir/$target-daemon-foundation-slskr.normalized"; then
    printf 'daemon foundation differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s daemon foundation differential passed\n' "$target"
}

write_core_workflow_yaml() {
  local target="$1" path="$2" destination="$3"
  local target_only=""
  local probe_line=""
  if [[ "$target" == slskdn ]]; then
    probe_line="  probe_media_attributes: false
"
    target_only="soulseek:
  liked_interests: [Ambient, Jazz]
  hated_interests: [spam]
destinations:
  folders:
    - name: Music
      path: '$destination'
      default: true
wishlist:
  enabled: true
  interval_seconds: 600
  auto_download: true
  max_results: 250
"
  fi
  printf '%s' "flags:
  no_connect: true
rooms: [Ambient, Jazz]
shares:
  cache:
    storage_mode: disk
    workers: 2
    retention: 120
${probe_line}throttling:
  search:
    incoming:
      concurrency: 4
      circuit_breaker: 600
      response_file_limit: 700
${target_only}web:
  authentication:
    disabled: true
" >"$path"
}

capture_core_workflow() {
  local target="$1" base_url="$2" suite="$3"
  mkdir -p "$suite"
  "$python_bin" - "$target" "$base_url" >"$suite/core.body" <<'PY'
import json,os,sys,urllib.request
target,base=sys.argv[1:]
with urllib.request.urlopen(base + "/api/v0/options", timeout=5) as response:
    value=json.load(response)
result={
    "rooms":value["rooms"],
    "shares":{"cache":value["shares"]["cache"]},
    "throttling":value["throttling"]["search"]["incoming"],
}
if target == "slskdn":
    result["shares"]["probeMediaAttributes"]=value["shares"]["probeMediaAttributes"]
    result["soulseek"]={
        "likedInterests":value["soulseek"]["likedInterests"],
        "hatedInterests":value["soulseek"]["hatedInterests"],
    }
    result["wishlist"]=value["wishlist"]
    result["destinations"]={"folders":[dict(item,path=os.path.basename(item["path"])) for item in value["destinations"]["folders"]]}
    with urllib.request.urlopen(base + "/api/v0/destinations", timeout=5) as response:
        destinations=json.load(response)
    result["destinationApi"]=[{
        "name":item["name"], "path":os.path.basename(item["path"]),
        "isDefault":item["isDefault"], "exists":item["exists"],
    } for item in destinations]
    request=urllib.request.Request(
        base + "/api/v0/wishlist",
        data=b'{"searchText":"fixture"}',
        headers={"Content-Type":"application/json"}, method="POST")
    with urllib.request.urlopen(request, timeout=5) as response:
        item=json.load(response)
        result["wishlistCreateStatus"]=response.status
    result["wishlistCreated"]={
        "searchText":item["searchText"], "enabled":item["enabled"],
        "autoDownload":item["autoDownload"], "maxResults":item["maxResults"],
    }
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/core.meta"
}

run_core_workflow_scenario() {
  local target="$1" root="$2"
  local port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-core-workflow-$implementation"
    local suite="$work_dir/$target-core-workflow-$implementation"
    local log="$work_dir/$target-core-workflow-$implementation.log"
    local destination="$state/music"
    mkdir -p "$state" "$suite" "$destination"
    write_core_workflow_yaml "$target" "$state/slskd.yml" "$destination"
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec "$repo_root/target/debug/slskr" serve
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-core-workflow-$implementation-options.json" "$log"
    capture_core_workflow "$target" "$base_url" "$suite"
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-core-workflow-upstream" "$work_dir/$target-core-workflow-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-core-workflow-slskr" "$work_dir/$target-core-workflow-slskr.normalized"
  if ! diff -ru "$work_dir/$target-core-workflow-upstream.normalized" "$work_dir/$target-core-workflow-slskr.normalized"; then
    printf 'core workflow differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s core workflow differential passed\n' "$target"
}

write_advanced_networking_security_yaml() {
  local path="$1" dht_port="$2" mesh_udp_port="$3" mesh_quic_port="$4"
  printf '%s' "flags:
  no_connect: true
dht:
  enabled: false
  dht_port: $dht_port
  overlay_port: 51012
  advertised_overlay_port: 51013
  vpn_port_sync: target_port
  bootstrap_routers: [router.example:6881]
  announce_interval_seconds: 120
  discovery_interval_seconds: 90
  min_neighbors: 7
  bootstrap_timeout_seconds: 20
  cold_bootstrap_timeout_seconds: 30
  lan_only_bootstrap_timeout_seconds: 10
  lan_only: true
  enable_upnp: true
  enable_stun: false
mesh:
  enabled: true
  enable_soulseek_capability_handshake: false
  enable_soulseek_rendezvous: false
  probe_soulseek_rendezvous_capabilities: false
  dht: {bootstrap_nodes: 17}
  overlay: {udp_port: $mesh_udp_port, quic_port: $mesh_quic_port}
  security: {enforceRemotePayloadLimits: true, maxRemotePayloadSize: 262144}
  sync_security:
    max_invalid_entries_per_window: 8
    max_invalid_messages_per_window: 4
    rate_limit_window_minutes: 2
    quarantine_violation_threshold: 2
    quarantine_duration_minutes: 11
    proof_of_possession_enabled: true
    consensus_min_peers: 4
    consensus_min_agreements: 2
    alert_threshold_signature_failures: 9
    alert_threshold_rate_limit_violations: 8
    alert_threshold_quarantine_events: 7
PodCore:
  Join: {SignatureMode: warn}
  Security: {SignatureMode: enforce}
overlay:
  enable: false
  listen_port: 51016
  enable_quic: true
  quic_listen_port: 51017
  share_quic_with_dht_port: false
  quic_backend_listen_port: 51018
  trusted_certificate_pins: {'127.0.0.1:51017': [pin-value]}
overlay_data:
  enable: false
  listen_port: 51019
  relay_authentication_token: overlay-token
  allowed_relay_destinations: ['8.8.8.8:443']
  max_concurrent_relays: 3
  max_relay_bytes_per_direction: 123456
  max_relay_duration_seconds: 45
  trusted_certificate_pins: {'127.0.0.1:51019': [data-pin]}
relay:
  enabled: false
  mode: controller
  controller:
    address: https://controller.example
    ignore_certificate_errors: true
    api_key: 1234567890abcdef
    secret: abcdef1234567890
    downloads: true
  agents:
    edge: {instance_name: edge-one, secret: 0123456789abcdef, cidr: 127.0.0.1/32}
security:
  enabled: true
  profile: Custom
  network_guard: {enabled: true, max_connections_per_ip: 12, max_global_connections: 345, max_messages_per_minute: 67, max_message_size: 8192}
  path_guard: {enabled: true, max_path_length: 333, max_path_depth: 13}
  content_safety: {enabled: true, verify_magic_bytes: false, quarantine_suspicious: false, quarantine_directory: /tmp/quarantine, block_executables: false}
  peer_reputation: {enabled: true, trusted_threshold: 80, untrusted_threshold: 10}
  violation_tracker: {enabled: true, violations_before_auto_ban: 3, base_ban_duration_minutes: 15}
  adversarial:
    privacy: {padding: {max_unpadded_bytes: 1024, max_padded_bytes: 2048}}
    anonymity:
      relay_only: {relay_peer_data_endpoints: ['8.8.4.4:443'], relay_authentication_token: anonymity-token}
web:
  authentication:
    disabled: true
" >"$path"
}

capture_advanced_networking_security() {
  local base_url="$1" suite="$2"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/advanced.body" <<'PY'
import json,sys,urllib.request
base=sys.argv[1]
with urllib.request.urlopen(base + "/api/v0/options", timeout=5) as response:
    value=json.load(response)
dht=value["dhtRendezvous"]
relay=value["relay"]
security=value["security"]
result={
 "dht":{key:dht[key] for key in ["advertisedOverlayPort","announceIntervalSeconds","bootstrapRouters","bootstrapTimeoutSeconds","coldBootstrapTimeoutSeconds","dhtPort","discoveryIntervalSeconds","enableStun","enableUpnp","enabled","lanOnly","lanOnlyBootstrapTimeoutSeconds","minNeighbors","overlayPort","vpnPortSync"]},
 "relay":{
   "enabled":relay["enabled"], "mode":relay["mode"],
   "controller":{key:relay["controller"][key] for key in ["address","apiKey","downloads","ignoreCertificateErrors","secret"]},
 },
 "security":{
   "enabled":security["enabled"], "profile":security["profile"],
   "networkGuard":{key:security["networkGuard"][key] for key in ["enabled","maxConnectionsPerIp","maxGlobalConnections","maxMessageSize","maxMessagesPerMinute"]},
   "pathGuard":{key:security["pathGuard"][key] for key in ["enabled","maxPathDepth","maxPathLength"]},
   "contentSafety":{key:security["contentSafety"][key] for key in ["blockExecutables","enabled","quarantineDirectory","quarantineSuspicious","verifyMagicBytes"]},
   "peerReputation":{key:security["peerReputation"][key] for key in ["enabled","trustedThreshold","untrustedThreshold"]},
   "violationTracker":{key:security["violationTracker"][key] for key in ["baseBanDurationMinutes","enabled","violationsBeforeAutoBan"]},
 },
}
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/advanced.meta"
  "$python_bin" - "$base_url" >"$suite/mesh.body" <<'PY'
import json,sys,urllib.request
with urllib.request.urlopen(sys.argv[1] + "/api/v0/mesh/stats", timeout=5) as response:
    value=json.load(response)
print(json.dumps(value,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/mesh.meta"
}

run_advanced_networking_security_scenario() {
  local root="$1"
  local port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local dht_port="$(pick_free_udp_port)" mesh_udp_port="$(pick_free_udp_port)" mesh_quic_port="$(pick_free_udp_port)"
  local base_url="http://127.0.0.1:$port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-advanced-networking-security-$implementation"
    local suite="$work_dir/slskdn-advanced-networking-security-$implementation"
    local log="$work_dir/slskdn-advanced-networking-security-$implementation.log"
    mkdir -p "$state" "$suite"
    write_advanced_networking_security_yaml "$state/slskd.yml" "$dht_port" "$mesh_udp_port" "$mesh_quic_port"
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec "$repo_root/target/debug/slskr" serve
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-advanced-networking-security-$implementation-options.json" "$log"
    capture_advanced_networking_security "$base_url" "$suite"
    stop_daemon
  done
  normalize_directory_suite "$work_dir/slskdn-advanced-networking-security-upstream" "$work_dir/slskdn-advanced-networking-security-upstream.normalized"
  normalize_directory_suite "$work_dir/slskdn-advanced-networking-security-slskr" "$work_dir/slskdn-advanced-networking-security-slskr.normalized"
  if ! diff -ru "$work_dir/slskdn-advanced-networking-security-upstream.normalized" "$work_dir/slskdn-advanced-networking-security-slskr.normalized"; then
    printf 'advanced networking/security differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn advanced networking/security differential passed\n'
}

write_media_advanced_service_yaml() {
  local path="$1"
  printf '%s' "flags:
  no_connect: true
feature:
  collectionsSharing: false
  streaming: false
  streamingRelayFallback: false
  meshParallelSearch: false
  meshPublishAvailability: false
  identityFriends: false
  solid: true
  scenePodBridge: true
  scenePodBridgeOptions: {proxyTransfers: true, exportPodAvailability: true}
  songId: true
  mesh: false
  dht: false
  pods: false
  socialFederation: false
  virtualSoulfind: true
  multiSourceDownloads: false
player:
  external_visualizer:
    enabled: true
    path: /bin/echo
    arguments: [visualizer, --fixture]
    working_directory: /tmp
    name: Fixture Visualizer
solid:
  allowInsecureHttp: true
  maxFetchBytes: 7654321
  timeoutSeconds: 23
  allowedHosts: [pod.example, identity.example]
  redirectPath: /fixture/callback
song_id:
  max_concurrent_runs: 7
virtualSoulfind:
  bridge:
    enabled: false
    port: 4322
    bindAddress: 127.0.0.2
    maxClients: 17
    requireAuth: true
    password: fixture-secret
    maxRequestsPerMinute: 71
    maxTransfersPerSession: 19
  disasterMode:
    auto: true
    force: true
    unavailableThresholdMinutes: 13
    enableGracefulDegradation: false
    recoveryCheckIntervalMinutes: 11
    recoveryHealthyChecksRequired: 5
web:
  authentication:
    disabled: true
" >"$path"
}

capture_media_advanced_service() {
  local base_url="$1" suite="$2"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/media.body" <<'PY'
import json,os,sys,urllib.request
base=sys.argv[1]
def get(path):
    with urllib.request.urlopen(base + path, timeout=5) as response:
        return json.load(response)
value=get("/api/v0/options")
feature=value["feature"]
visualizer=get("/api/v0/player/external-visualizer")
result={
 "feature":{key:feature[key] for key in [
   "collectionsSharing","streaming","streamingRelayFallback","meshParallelSearch",
   "meshPublishAvailability","identityFriends","solid","scenePodBridge","songId",
   "mesh","dht","pods","socialFederation","virtualSoulfind","multiSourceDownloads"]},
 "scenePodBridgeOptions":feature["scenePodBridgeOptions"],
 "player":value["player"]["externalVisualizer"],
 "solid":value["solid"],
 "songId":value["songId"],
 "bridge":value["virtualSoulfind"]["bridge"],
 "disasterMode":value["virtualSoulfind"]["disasterMode"],
 "visualizerStatus":visualizer,
 "solidStatus":get("/api/v0/solid/status"),
 "bridgeAdmin":get("/api/v0/bridge/admin/config"),
 "disasterStatus":get("/api/v0/virtualsoulfind/disaster-mode/status"),
 "songQueue":get("/api/v0/songid/runs/queue"),
}
for key in ("path","resolvedPath"):
    if result["visualizerStatus"].get(key):
        result["visualizerStatus"][key]=os.path.basename(result["visualizerStatus"][key])
for key in ("path","workingDirectory"):
    if result["player"].get(key):
        result["player"][key]=os.path.basename(result["player"][key])
if result["visualizerStatus"].get("workingDirectory"):
    result["visualizerStatus"]["workingDirectory"]=os.path.basename(result["visualizerStatus"]["workingDirectory"])
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/media.meta"
}

run_media_advanced_service_scenario() {
  local root="$1"
  local port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-media-advanced-service-$implementation"
    local suite="$work_dir/slskdn-media-advanced-service-$implementation"
    local log="$work_dir/slskdn-media-advanced-service-$implementation.log"
    mkdir -p "$state" "$suite"
    write_media_advanced_service_yaml "$state/slskd.yml"
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec "$repo_root/target/debug/slskr" serve
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-media-advanced-service-$implementation-options.json" "$log"
    capture_media_advanced_service "$base_url" "$suite"
    stop_daemon
  done
  normalize_directory_suite "$work_dir/slskdn-media-advanced-service-upstream" "$work_dir/slskdn-media-advanced-service-upstream.normalized"
  normalize_directory_suite "$work_dir/slskdn-media-advanced-service-slskr" "$work_dir/slskdn-media-advanced-service-slskr.normalized"
  if ! diff -ru "$work_dir/slskdn-media-advanced-service-upstream.normalized" "$work_dir/slskdn-media-advanced-service-slskr.normalized"; then
    printf 'media/advanced-service differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn media/advanced-service differential passed\n'
}

run_target() {
  local target="$1"
  local root="$2"
  local port
  local https_port
  local listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local upstream_state="$work_dir/state-$target-upstream"
  local slskr_state="$work_dir/state-$target-slskr"
  local upstream_json="$work_dir/$target-upstream.json"
  local slskr_json="$work_dir/$target-slskr.json"
  local upstream_normalized="$work_dir/$target-upstream.normalized.json"
  local slskr_normalized="$work_dir/$target-slskr.normalized.json"
  local upstream_mutations="$work_dir/$target-upstream-mutations"
  local slskr_mutations="$work_dir/$target-slskr-mutations"
  local upstream_log="$work_dir/$target-upstream.log"
  local slskr_log="$work_dir/$target-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$upstream_state" "$slskr_state"
  if [[ "$target" == "slskdn" ]]; then
    mkdir -p "$(dirname "$dll")/wwwroot"
  fi
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    export SLSKD_REMOTE_CONFIGURATION=true
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$upstream_json" "$upstream_log"
  capture_mutation_suite "$base_url" "$upstream_mutations"
  stop_daemon

  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    exec "$repo_root/target/debug/slskr" serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect \
      --remote-configuration
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$slskr_json" "$slskr_log"
  capture_mutation_suite "$base_url" "$slskr_mutations"
  stop_daemon

  normalize_options "$upstream_json" "$upstream_normalized"
  normalize_options "$slskr_json" "$slskr_normalized"
  if ! cmp --silent "$upstream_normalized" "$slskr_normalized"; then
    printf 'options differential failed for %s\n' "$target" >&2
    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
    exit 1
  fi
  compare_mutation_suites "$target" "$upstream_mutations" "$slskr_mutations"
  printf '%s options differential passed\n' "$target"
}

mkdir -p "$work_dir"
materialize_target "$slskd_root" "$slskd_ref" created_slskd_worktree
materialize_target "$slskdn_root" "$slskdn_ref" created_slskdn_worktree

export DOTNET_CLI_TELEMETRY_OPTOUT=1
dotnet build "$slskd_root/src/slskd/slskd.csproj" -c Release -r linux-x64 --self-contained false -v:q
dotnet build "$slskdn_root/src/slskd/slskd.csproj" -c Release -r linux-x64 --self-contained false -v:q
# Frozen slskdN resolves its default content path relative to the application
# base directory and refuses to start if it is absent. The audited API
# scenarios do not need UI assets, but they do need the validated directory.
mkdir -p "$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
cargo build -q -p slskr

if scenario_enabled options; then
  run_target slskd "$slskd_root"
  run_target slskdn "$slskdn_root"
fi
if scenario_enabled directories; then
  run_directory_scenario slskd "$slskd_root"
  run_directory_scenario slskdn "$slskdn_root"
fi
if scenario_enabled shares; then
  run_share_watch_scenario slskd "$slskd_root"
  run_share_watch_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-watch; then
  run_no_watch_upload_scenario slskd "$slskd_root"
  run_no_watch_upload_scenario slskdn "$slskdn_root"
fi
if scenario_enabled storage; then
  run_storage_restart_scenario slskd "$slskd_root"
  run_storage_restart_scenario slskdn "$slskdn_root"
fi
if scenario_enabled file-management; then
  run_remote_file_management_scenario slskd "$slskd_root"
  run_remote_file_management_scenario slskdn "$slskdn_root"
fi
if scenario_enabled remote-configuration; then
  run_remote_configuration_scenario slskd "$slskd_root"
  run_remote_configuration_scenario slskdn "$slskdn_root"
fi
if scenario_enabled debug; then
  run_debug_scenario slskd "$slskd_root"
  run_debug_scenario slskdn "$slskdn_root"
fi
if scenario_enabled swagger; then
  run_swagger_scenario slskd "$slskd_root"
  run_swagger_scenario slskdn "$slskdn_root"
fi
if scenario_enabled metrics; then
  run_metrics_scenario slskd "$slskd_root"
  run_metrics_scenario slskdn "$slskdn_root"
fi
if scenario_enabled headless; then
  run_headless_scenario slskd "$slskd_root"
  run_headless_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-start; then
  run_no_start_scenario slskd "$slskd_root"
  run_no_start_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-logo; then
  run_no_logo_scenario slskd "$slskd_root"
  run_no_logo_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-version-check; then
  run_no_version_check_scenario slskd "$slskd_root"
  run_no_version_check_scenario slskdn "$slskdn_root"
fi
if scenario_enabled experimental; then
  run_experimental_scenario slskd "$slskd_root"
  run_experimental_scenario slskdn "$slskdn_root"
fi
if scenario_enabled case-sensitive-regex; then
  run_case_sensitive_regex_scenario slskd "$slskd_root"
  run_case_sensitive_regex_scenario slskdn "$slskdn_root"
fi
if scenario_enabled regex-runtime; then
  run_regex_runtime_scenario slskd "$slskd_root"
  run_regex_runtime_scenario slskdn "$slskdn_root"
fi
if scenario_enabled regex-protocol; then
  run_regex_protocol_scenario slskd "$slskd_root"
  run_regex_protocol_scenario slskdn "$slskdn_root"
fi
if scenario_enabled share-scan-flags; then
  run_share_scan_flags_scenario slskd "$slskd_root"
  run_share_scan_flags_scenario slskdn "$slskdn_root"
fi
if scenario_enabled instance-name; then
  run_instance_name_scenario slskd "$slskd_root"
  run_instance_name_scenario slskdn "$slskdn_root"
fi
if scenario_enabled completed-template; then
  run_completed_template_scenario "$slskdn_root"
fi
if scenario_enabled private-message-auto-response; then
  run_private_message_auto_response_scenario "$slskdn_root"
fi
if scenario_enabled download-auto-retry; then
  run_download_auto_retry_scenario "$slskdn_root"
fi
if scenario_enabled blacklist; then
  run_blacklist_scenario slskd "$slskd_root"
  run_blacklist_scenario slskdn "$slskdn_root"
fi
if scenario_enabled transfer-groups; then
  run_transfer_groups_scenario slskd "$slskd_root"
  run_transfer_groups_scenario slskdn "$slskdn_root"
fi
if scenario_enabled transfer-download; then
  run_transfer_download_scenario slskd "$slskd_root"
  run_transfer_download_scenario slskdn "$slskdn_root"
fi
if scenario_enabled soulseek-connection; then
  run_soulseek_connection_scenario slskd "$slskd_root"
  run_soulseek_connection_scenario slskdn "$slskdn_root"
fi
if scenario_enabled soulseek-profile-distributed; then
  run_soulseek_profile_distributed_scenario slskd "$slskd_root"
  run_soulseek_profile_distributed_scenario slskdn "$slskdn_root"
fi
if scenario_enabled daemon-foundation; then
  run_daemon_foundation_scenario slskd "$slskd_root"
  run_daemon_foundation_scenario slskdn "$slskdn_root"
fi
if scenario_enabled core-workflow; then
  run_core_workflow_scenario slskd "$slskd_root"
  run_core_workflow_scenario slskdn "$slskdn_root"
fi
if scenario_enabled advanced-networking-security; then
  run_advanced_networking_security_scenario "$slskdn_root"
fi
if scenario_enabled media-advanced-service; then
  run_media_advanced_service_scenario "$slskdn_root"
fi
if scenario_enabled dht; then
  run_dht_scenario "$slskdn_root"
fi
if scenario_enabled server-endpoint; then
  run_server_endpoint_scenario slskd "$slskd_root"
  run_server_endpoint_scenario slskdn "$slskdn_root"
fi
if scenario_enabled credentials; then
  run_credential_scenario slskd "$slskd_root"
  run_credential_scenario slskdn "$slskdn_root"
fi
if scenario_enabled obfuscation; then
  run_obfuscation_options_scenario "$slskdn_root"
  run_obfuscation_runtime_scenario "$slskdn_root"
  run_obfuscation_outbound_scenario "$slskdn_root"
fi
if scenario_enabled no-connect; then
  run_no_connect_scenario slskd "$slskd_root"
  run_no_connect_scenario slskdn "$slskdn_root"
fi
if scenario_enabled config-watch; then
  run_config_watch_scenario slskd "$slskd_root"
  run_config_watch_scenario slskdn "$slskdn_root"
fi
if scenario_enabled description; then
  run_description_scenario slskd "$slskd_root"
  run_description_scenario slskdn "$slskdn_root"
fi
if scenario_enabled web-listener; then
  run_web_listener_scenario slskd "$slskd_root"
  run_web_listener_scenario slskdn "$slskdn_root"
fi
if scenario_enabled listener; then
  run_listener_scenario slskd "$slskd_root"
  run_listener_scenario slskdn "$slskdn_root"
fi
if scenario_enabled integrations; then
  run_script_scenario slskd "$slskd_root"
  run_script_scenario slskdn "$slskdn_root"
  run_integration_scenario "$slskdn_root"
fi
if scenario_enabled lidarr-runtime; then
  run_lidarr_runtime_scenario "$slskdn_root"
fi

printf 'controller options differential passed for frozen slskd + slskdN\n'
if [[ "$keep_artifacts" == "1" ]]; then
  printf 'options differential artifacts retained at %s\n' "$work_dir"
fi
