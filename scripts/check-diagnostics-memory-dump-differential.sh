#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
dll="$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
work_dir="${SLSKR_DUMP_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-dump-differential.XXXXXX")}"
keep_artifacts="${SLSKR_DUMP_DIFFERENTIAL_KEEP:-0}"
daemon_pid=""

host_ip="$(ip -j route get 1.1.1.1 | python3 -c 'import json,sys
routes = json.load(sys.stdin)
if not routes or not routes[0].get("prefsrc"):
    raise SystemExit("no non-loopback IPv4 source address available")
print(routes[0]["prefsrc"])')"

pick_free_port() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

stop_daemon() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  daemon_pid=""
}

cleanup() {
  stop_daemon
  if [[ "$keep_artifacts" != 1 ]]; then
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

write_config() {
  local state="$1"
  local allow_memory="$2"
  local allow_remote="$3"
  local enforce="${4:-false}"
  mkdir -p "$state"
  printf '%s\n' \
    'flags:' \
    '  no_connect: true' \
    'remote_configuration: true' \
    'dht:' \
    '  enabled: false' \
    'diagnostics:' \
    "  allow_memory_dump: $allow_memory" \
    "  allow_remote_dump: $allow_remote" \
    'web:' \
    "  enforce_security: $enforce" \
    '  allow_remote_no_auth: true' \
    '  authentication:' \
    '    disabled: true' \
    '    passthrough:' \
    "      allowed_cidrs: '$host_ip/32'" \
    '  rate_limiting:' \
    '    enabled: false' >"$state/slskd.yml.tmp"
  mv "$state/slskd.yml.tmp" "$state/slskd.yml"
}

start_daemon() {
  local implementation="$1"
  local state="$2"
  local port="$3"
  local https_port="$4"
  local listen_port="$5"
  local log="$6"
  if [[ "$implementation" == upstream ]]; then
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_REMOTE_CONFIGURATION=true
      export SLSKD_HTTP_ADDRESS=0.0.0.0 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKR_REMOTE_CONFIGURATION=true
      export SLSKR_CONTROLLER_AUDIT_MODE=1
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-address 0.0.0.0 --http-port "$port" \
        --slsk-listen-port "$listen_port" --no-connect
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_ready() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 600); do
    if curl --noproxy '*' --silent --max-time 1 "$base_url/api/v0/application" >/dev/null; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      tail -120 "$log" >&2 || true
      return 1
    fi
    sleep 0.1
  done
  tail -120 "$log" >&2 || true
  return 1
}

options_snapshot() {
  curl --noproxy '*' --silent --show-error --max-time 10 "$1/api/v0/options"
}

application_snapshot() {
  curl --noproxy '*' --silent --show-error --max-time 10 "$1/api/v0/application"
}

capture_dump() {
  local base_url="$1"
  local destination="$2"
  local headers="$destination.headers"
  local body="$destination.body"
  local status
  status="$(curl --noproxy '*' --silent --show-error --max-time 180 \
    -X POST -D "$headers" -o "$body" -w '%{http_code}' \
    "$base_url/api/v0/application/dump")"
  python3 - "$status" "$headers" "$body" >"$destination" <<'PY'
import json, pathlib, sys
status = int(sys.argv[1])
headers = pathlib.Path(sys.argv[2]).read_text(errors="replace").splitlines()
body = pathlib.Path(sys.argv[3]).read_bytes()
selected = {}
for line in headers:
    if ":" not in line:
        continue
    key, value = line.split(":", 1)
    if key.lower() in {"content-type", "content-disposition"}:
        selected[key.lower()] = value.strip().lower()
print(json.dumps({
    "status": status,
    "contentType": selected.get("content-type", "").split(";", 1)[0],
    "contentDisposition": selected.get("content-disposition", ""),
    "bodyNonempty": bool(body),
}, sort_keys=True))
PY
  rm -f "$headers" "$body"
}

capture_validation_suite() {
  local base_url="$1"
  local implementation="$2"
  python3 - "$base_url" >"$work_dir/$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
cases = {
    "diagnostics-null": "diagnostics: null\n",
    "diagnostics-array": "diagnostics: []\n",
    "memory-null": "diagnostics:\n  allow_memory_dump: null\n",
    "memory-string": "diagnostics:\n  allow_memory_dump: 'true'\n",
    "memory-invalid": "diagnostics:\n  allow_memory_dump: 1\n",
    "remote-null": "diagnostics:\n  allow_remote_dump: null\n",
    "remote-string": "diagnostics:\n  allow_remote_dump: 'false'\n",
    "remote-invalid": "diagnostics:\n  allow_remote_dump: []\n",
}
for label, yaml in cases.items():
    connection = http.client.HTTPConnection(url.hostname, url.port, timeout=10)
    connection.request(
        "POST", "/api/v0/options/yaml/validate",
        body=json.dumps(yaml, separators=(",", ":")),
        headers={"Content-Type": "application/json"},
    )
    response = connection.getresponse()
    body = response.read()
    content_type = response.getheader("Content-Type", "").split(";", 1)[0].lower()
    try:
        value = json.loads(body) if body else None
    except json.JSONDecodeError:
        value = body.decode("utf-8", "replace")
    print(json.dumps({"label": label, "status": response.status, "type": content_type, "body": value}, sort_keys=True))
    connection.close()
PY
}

run_static_modes() {
  local implementation="$1"
  local state="$work_dir/$implementation-static-state"
  local log="$work_dir/$implementation-static.log"
  local port https_port listen_port base_loopback base_remote options
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_loopback="http://127.0.0.1:$port"
  base_remote="http://$host_ip:$port"

  write_config "$state" false false
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_loopback" "$log"
  capture_dump "$base_remote" "$work_dir/$implementation-disabled.json"
  options="$(options_snapshot "$base_loopback")"
  python3 - "$options" <<'PY'
import json, sys
diagnostics = json.loads(sys.argv[1]).get("diagnostics", {})
if diagnostics.get("allowMemoryDump") is not False or diagnostics.get("allowRemoteDump") is not False:
    raise SystemExit(f"diagnostics defaults mismatch: {diagnostics!r}")
PY
  capture_validation_suite "$base_loopback" "$implementation"
  stop_daemon

  write_config "$state" true false
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_loopback" "$log"
  capture_dump "$base_remote" "$work_dir/$implementation-local-only.json"
  stop_daemon
}

run_watch_restart() {
  local implementation="$1"
  local kind="$2"
  local state="$work_dir/$implementation-watch-$kind-state"
  local log="$work_dir/$implementation-watch-$kind.log"
  local port https_port listen_port base_loopback base_remote options application
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_loopback="http://127.0.0.1:$port"
  base_remote="http://$host_ip:$port"
  if [[ "$kind" == memory ]]; then
    write_config "$state" false false
  else
    write_config "$state" true false
  fi
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_loopback" "$log"
  if [[ "$kind" == memory ]]; then
    capture_dump "$base_remote" "$work_dir/$implementation-watch-$kind-before.json"
    write_config "$state" true false
  else
    capture_dump "$base_remote" "$work_dir/$implementation-watch-$kind-before.json"
    write_config "$state" true true
  fi

  for _ in $(seq 1 100); do
    options="$(options_snapshot "$base_loopback")"
    application="$(application_snapshot "$base_loopback")"
    if python3 - "$options" "$application" "$kind" <<'PY'
import json, sys
options, application, kind = json.loads(sys.argv[1]), json.loads(sys.argv[2]), sys.argv[3]
diagnostics = options.get("diagnostics", {})
expected = diagnostics.get("allowMemoryDump") is True and (kind == "memory" or diagnostics.get("allowRemoteDump") is True)
raise SystemExit(0 if expected and application.get("pendingRestart") is True else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$options" "$application" "$kind" <<'PY'
import json, sys
options, application, kind = json.loads(sys.argv[1]), json.loads(sys.argv[2]), sys.argv[3]
diagnostics = options.get("diagnostics", {})
expected = diagnostics.get("allowMemoryDump") is True and (kind == "memory" or diagnostics.get("allowRemoteDump") is True)
if not expected or application.get("pendingRestart") is not True:
    raise SystemExit(f"watched diagnostics projection mismatch: diagnostics={diagnostics!r} application={application!r}")
PY
  capture_dump "$base_remote" "$work_dir/$implementation-watch-$kind-prerestart.json"
  stop_daemon

  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_loopback" "$log"
  capture_dump "$base_remote" "$work_dir/$implementation-watch-$kind-postrestart.json"
  stop_daemon
}

run_hardening_rejection() {
  local implementation="$1"
  local state="$work_dir/$implementation-hardening-state"
  local log="$work_dir/$implementation-hardening.log"
  local port https_port listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  write_config "$state" true false true
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  for _ in $(seq 1 200); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      wait "$daemon_pid" 2>/dev/null || true
      daemon_pid=""
      rg -q 'MemoryDumpWithAuthDisabled' "$log" || { tail -120 "$log" >&2; return 1; }
      return
    fi
    sleep 0.05
  done
  tail -120 "$log" >&2 || true
  printf '%s did not reject MemoryDumpWithAuthDisabled\n' "$implementation" >&2
  return 1
}

[[ -f "$dll" ]] || { printf 'missing frozen slskdN binary: %s\n' "$dll" >&2; exit 1; }
cargo build -p slskr --manifest-path "$repo_root/Cargo.toml" >/dev/null
for implementation in upstream slskr; do
  run_static_modes "$implementation"
  run_watch_restart "$implementation" memory
  run_watch_restart "$implementation" remote
  run_hardening_rejection "$implementation"
done
for case in disabled local-only watch-memory-before watch-memory-prerestart watch-memory-postrestart watch-remote-before watch-remote-prerestart watch-remote-postrestart; do
  diff -u "$work_dir/upstream-$case.json" "$work_dir/slskr-$case.json"
done
diff -u "$work_dir/upstream-validation.jsonl" "$work_dir/slskr-validation.jsonl"
printf 'diagnostics memory-dump differential passed against frozen slskdN\n'
