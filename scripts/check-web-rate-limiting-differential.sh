#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
dll="$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
work_dir="${SLSKR_RATE_LIMIT_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-rate-limit-differential.XXXXXX")}"
keep_artifacts="${SLSKR_RATE_LIMIT_DIFFERENTIAL_KEEP:-0}"
daemon_pid=""

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
  local enabled="$2"
  local permit="${3:-2}"
  local window="${4:-0}"
  mkdir -p "$state"
  local temporary="$state/slskd.yml.tmp"
  cat >"$temporary" <<EOF
flags:
  no_connect: true
remote_configuration: true
dht:
  enabled: false
mesh_gateway:
  enabled: true
  csrf_token: differential-csrf-token
  allowed_services: [mesh-introspect]
web:
  authentication:
    api_keys:
      differential:
        key: differential-controller-token-32
        role: administrator
        cidr: 127.0.0.0/8
  rate_limiting:
    enabled: $enabled
    api_permit_limit: $permit
    api_window_seconds: $window
    federation_permit_limit: $permit
    federation_window_seconds: $window
    mesh_gateway_permit_limit: $permit
    mesh_gateway_window_seconds: $window
EOF
  mv "$temporary" "$state/slskd.yml"
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
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=false
      export SLSKD_REMOTE_CONFIGURATION=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=false SLSKR_API_TOKEN=differential-controller-token-32
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
      export SLSKR_REMOTE_CONFIGURATION=true
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$port" \
        --slsk-listen-port "$listen_port" --no-connect
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_ready() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 600); do
    if curl --silent --max-time 1 "$base_url/" >/dev/null; then
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

probe_policy() {
  local base_url="$1"
  local label="$2"
  local method="$3"
  local path="$4"
  local output="$5"
  local request_count="${6:-3}"
  local source_ip="${7:-127.0.0.2}"
  : >"$output"
  for request in $(seq 1 "$request_count"); do
    local headers="$work_dir/$label-$request.headers"
    local body="$work_dir/$label-$request.body"
    local status
    status="$(curl --silent --show-error --max-time 5 --interface "$source_ip" -X "$method" \
      -H 'Content-Type: application/json' -H 'X-Slskdn-Csrf: differential-csrf-token' \
      -d '{}' -D "$headers" -o "$body" \
      -w '%{http_code}' "$base_url$path")"
    python3 - "$status" "$headers" "$body" >>"$output" <<'PY'
import json, pathlib, sys
status = int(sys.argv[1])
headers = pathlib.Path(sys.argv[2]).read_text(encoding="iso-8859-1").lower()
body = pathlib.Path(sys.argv[3]).read_bytes()
content_type = ""
for line in headers.splitlines():
    if line.startswith("content-type:"):
        content_type = line.split(":", 1)[1].strip()
row = {
    "status": status,
    "bodyBytes": len(body),
    "contentType": content_type,
    "hasQuotaHeader": "ratelimit-" in headers or "x-ratelimit-" in headers,
}
if status == 500 and "json" in content_type:
    problem = json.loads(body)
    row.update({
        "problemTitle": problem.get("title"),
        "problemStatus": problem.get("status"),
        "problemDetail": problem.get("detail"),
        "hasTraceId": bool(problem.get("traceId")),
    })
print(json.dumps(row, sort_keys=True))
PY
  done
}

assert_enabled_probe() {
  local path="$1"
  local permit_limit="${2:-2}"
  python3 - "$path" "$permit_limit" <<'PY'
import json, pathlib, sys
rows = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines()]
permit_limit = int(sys.argv[2])
if len(rows) != permit_limit + 1 or any(row["status"] == 429 for row in rows[:permit_limit]):
    raise SystemExit(f"rate limit fired before the configured permit count: {rows!r}")
if rows[permit_limit] != {"status": 429, "bodyBytes": 0, "contentType": "", "hasQuotaHeader": False}:
    raise SystemExit(f"frozen 429 contract mismatch: {rows[permit_limit]!r}")
PY
}

assert_disabled_probe() {
  local path="$1"
  python3 - "$path" <<'PY'
import json, pathlib, sys
rows = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines()]
if any(row["status"] == 429 for row in rows):
    raise SystemExit(f"disabled rate limiter returned 429: {rows!r}")
PY
}

assert_invalid_permit_probe() {
  local path="$1"
  python3 - "$path" <<'PY'
import json, pathlib, sys
rows = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines()]
expected = {
    "status": 500,
    "problemTitle": "Internal Server Error",
    "problemStatus": 500,
    "problemDetail": "An unexpected error occurred.",
    "hasTraceId": True,
}
if len(rows) != 1 or any(rows[0].get(key) != value for key, value in expected.items()):
    raise SystemExit(f"non-positive permit failure contract mismatch: {rows!r}")
if rows[0].get("contentType") != "application/problem+json" or rows[0].get("hasQuotaHeader"):
    raise SystemExit(f"non-positive permit headers mismatch: {rows!r}")
PY
}

assert_authenticated_bypass() {
  local base_url="$1"
  local implementation="$2"
  local enabled="$3"
  local statuses=()
  for _ in 1 2 3 4; do
    local status
    status="$(curl --silent --show-error --max-time 5 --interface 127.0.0.3 \
      -H 'X-API-Key: differential-controller-token-32' -o /dev/null \
      -w '%{http_code}' "$base_url/api/v0/session")"
    statuses+=("$status")
  done
  if [[ "$enabled" == false || "$implementation" == slskr || "${SLSKR_SLSKDN_EXPECT_API_KEY_BYPASS:-0}" == 1 ]]; then
    [[ "${statuses[*]}" == "200 200 200 200" ]] || {
      printf 'authenticated API requests did not bypass the general limiter: %s\n' "${statuses[*]}" >&2
      return 1
    }
  else
    [[ "${statuses[*]}" == "200 200 429 429" ]] || {
      printf 'frozen slskdN API-key rate-limit defect changed unexpectedly: %s\n' "${statuses[*]}" >&2
      return 1
    }
    printf 'confirmed frozen slskdN API-key rate-limit defect; fix is draft PR #275\n'
  fi
}

admin_bearer_token() {
  local base_url="$1"
  local password="slskd"
  local response
  response="$(curl --silent --show-error --max-time 10 --interface 127.0.0.7 \
    -H 'Content-Type: application/json' \
    --data-binary "{\"username\":\"slskd\",\"password\":\"$password\"}" \
    "$base_url/api/v0/session")"
  python3 - "$response" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
token = value.get("token")
if not isinstance(token, str) or not token:
    raise SystemExit(f"admin login did not return a token: {value!r}")
print(token)
PY
}

capture_validation_suite() {
  local base_url="$1"
  local implementation="$2"
  local token
  token="$(admin_bearer_token "$base_url" "$implementation")"
  python3 - "$base_url" "$token" >"$work_dir/$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
token = sys.argv[2]
cases = {
    "parent-null": "web:\n  rate_limiting: null\n",
    "parent-array": "web:\n  rate_limiting: []\n",
    "enabled-null": "web:\n  rate_limiting:\n    enabled: null\n",
    "enabled-string": "web:\n  rate_limiting:\n    enabled: 'false'\n",
    "enabled-invalid": "web:\n  rate_limiting:\n    enabled: nope\n",
    "permit-zero": "web:\n  rate_limiting:\n    api_permit_limit: 0\n",
    "permit-negative": "web:\n  rate_limiting:\n    federation_permit_limit: -1\n",
    "window-negative": "web:\n  rate_limiting:\n    mesh_gateway_window_seconds: -1\n",
    "integer-null": "web:\n  rate_limiting:\n    api_window_seconds: null\n",
    "integer-string": "web:\n  rate_limiting:\n    api_window_seconds: '60'\n",
    "integer-float": "web:\n  rate_limiting:\n    api_window_seconds: 1.5\n",
    "integer-array": "web:\n  rate_limiting:\n    federation_window_seconds: []\n",
    "integer-overflow": "web:\n  rate_limiting:\n    mesh_gateway_permit_limit: 2147483648\n",
    "integer-underflow": "web:\n  rate_limiting:\n    mesh_gateway_permit_limit: -2147483649\n",
}
for label, yaml in cases.items():
    connection = http.client.HTTPConnection(url.hostname, url.port, timeout=10, source_address=("127.0.0.7", 0))
    connection.request(
        "POST",
        "/api/v0/options/yaml/validate",
        body=json.dumps(yaml, separators=(",", ":")),
        headers={"Content-Type": "application/json", "Authorization": "Bearer " + token},
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

run_implementation() {
  local implementation="$1"
  local enabled="$2"
  local permit="${3:-2}"
  local state="$work_dir/$implementation-$enabled-state"
  local log="$work_dir/$implementation-$enabled.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" "$enabled" "$permit"
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  assert_authenticated_bypass "$base_url" "$implementation" "$enabled"
  if [[ "$enabled" == true && "$permit" -gt 0 ]]; then
    capture_validation_suite "$base_url" "$implementation"
  fi

  local api="$work_dir/$implementation-$enabled-api.jsonl"
  probe_policy "$base_url" "$implementation-$enabled-api" GET /api/v0/options "$api"
  if [[ "$enabled" == true ]]; then
    assert_enabled_probe "$api"
    for spec in \
      'fed POST /actors/alice/inbox' \
      'event POST /api/v0/events/inject' \
      'warm POST /api/v0/slskdn/warm-cache/hints' \
      'mesh GET /mesh/status'; do
      read -r label method path <<<"$spec"
      local output="$work_dir/$implementation-$enabled-$label.jsonl"
      probe_policy "$base_url" "$implementation-$enabled-$label" "$method" "$path" "$output"
      assert_enabled_probe "$output"
    done
  else
    assert_disabled_probe "$api"
  fi
  stop_daemon
}

run_watch_restart() {
  local implementation="$1"
  local state="$work_dir/$implementation-watch-state"
  local log="$work_dir/$implementation-watch.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" true 2 0
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"

  write_config "$state" true 4 0
  sleep 1
  local before_restart="$work_dir/$implementation-watch-before-restart.jsonl"
  probe_policy "$base_url" "$implementation-watch-before" GET /api/v0/options \
    "$before_restart" 3 127.0.0.4
  assert_enabled_probe "$before_restart" 2
  stop_daemon

  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  local after_restart="$work_dir/$implementation-watch-after-restart.jsonl"
  probe_policy "$base_url" "$implementation-watch-after" GET /api/v0/options \
    "$after_restart" 5 127.0.0.5
  assert_enabled_probe "$after_restart" 4
  stop_daemon
}

run_invalid_permits() {
  local implementation="$1"
  local permit="$2"
  local state="$work_dir/$implementation-invalid-$permit-state"
  local log="$work_dir/$implementation-invalid-$permit.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" true "$permit" 0
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  for spec in \
    'api GET /api/v0/options' \
    'fed POST /actors/alice/inbox' \
    'event POST /api/v0/events/inject' \
    'warm POST /api/v0/slskdn/warm-cache/hints' \
    'mesh GET /mesh/status'; do
    read -r label method path <<<"$spec"
    local output="$work_dir/$implementation-invalid-$permit-$label.jsonl"
    probe_policy "$base_url" "$implementation-invalid-$permit-$label" "$method" "$path" \
      "$output" 1 127.0.0.6
    assert_invalid_permit_probe "$output"
  done
  stop_daemon
}

[[ -f "$dll" ]] || { printf 'missing frozen slskdN binary: %s\n' "$dll" >&2; exit 1; }
[[ -x "$repo_root/target/debug/slskr" ]] || { printf 'missing slskR debug binary\n' >&2; exit 1; }

run_implementation upstream true
run_implementation slskr true
run_implementation upstream false -1
run_implementation slskr false -1
run_watch_restart upstream
run_watch_restart slskr
run_invalid_permits upstream 0
run_invalid_permits slskr 0
run_invalid_permits upstream -1
run_invalid_permits slskr -1
diff -u "$work_dir/upstream-validation.jsonl" "$work_dir/slskr-validation.jsonl"

printf 'web rate-limiting differential passed for frozen slskdN and slskR\n'
if [[ "$keep_artifacts" == 1 ]]; then
  printf 'rate-limiting differential artifacts retained at %s\n' "$work_dir"
fi
