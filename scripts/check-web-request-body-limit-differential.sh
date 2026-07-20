#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
dll="$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
work_dir="${SLSKR_BODY_LIMIT_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-body-limit-differential.XXXXXX")}"
keep_artifacts="${SLSKR_BODY_LIMIT_DIFFERENTIAL_KEEP:-0}"
api_key="differential-controller-token-32"
daemon_pid=""
request_auth_header="X-API-Key: $api_key"

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
  local limit="${2:-}"
  mkdir -p "$state"
  {
    printf '%s\n' \
      'flags:' \
      '  no_connect: true' \
      'dht:' \
      '  enabled: false' \
      'web:' \
      '  authentication:' \
      '    api_keys:' \
      '      differential:' \
      "        key: $api_key" \
      '        role: administrator' \
      '        cidr: 127.0.0.0/8' \
      '  rate_limiting:' \
      '    enabled: false'
    if [[ -n "$limit" ]]; then
      printf '  max_request_body_size: %s\n' "$limit"
    fi
  } >"$state/slskd.yml.tmp"
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
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=false SLSKR_API_TOKEN="$api_key"
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn
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
    if curl --silent --max-time 1 -H "$request_auth_header" "$base_url/api/v0/options" >/dev/null; then
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

authenticate_admin() {
  local base_url="$1"
  local password="slskd"
  local response
  response="$(curl --silent --show-error --max-time 10 \
    -H 'Content-Type: application/json' \
    --data-binary "{\"username\":\"slskd\",\"password\":\"$password\"}" \
    "$base_url/api/v0/session")"
  local token
  token="$(python3 - "$response" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
token = value.get("token")
if not isinstance(token, str) or not token:
    raise SystemExit(f"admin login did not return a token: {value!r}")
print(token)
PY
)"
  request_auth_header="Authorization: Bearer $token"
}

make_json_yaml_payload() {
  local size="$1"
  local destination="$2"
  python3 - "$size" "$destination" <<'PY'
import json, pathlib, sys
size = int(sys.argv[1])
# JSON quotes plus the YAML comment marker and newline account for five bytes.
payload = json.dumps("#" + ("x" * (size - 5)) + "\n", separators=(",", ":"))
assert len(payload.encode()) == size
pathlib.Path(sys.argv[2]).write_text(payload, encoding="utf-8")
PY
}

probe_body() {
  local base_url="$1"
  local size="$2"
  local label="$3"
  local payload="$work_dir/payload-$size.json"
  local headers="$work_dir/$label.headers"
  local body="$work_dir/$label.body"
  make_json_yaml_payload "$size" "$payload"
  local status
  status="$(curl --silent --show-error --max-time 30 \
    -H "$request_auth_header" -H 'Content-Type: application/json' \
    --data-binary "@$payload" -D "$headers" -o "$body" \
    -w '%{http_code}' "$base_url/api/v0/options/yaml/validate")"
  python3 - "$status" "$headers" "$body" <<'PY'
import json, pathlib, sys
headers = pathlib.Path(sys.argv[2]).read_text(encoding="iso-8859-1").lower()
body = pathlib.Path(sys.argv[3]).read_bytes()
content_type = ""
for line in headers.splitlines():
    if line.startswith("content-type:"):
        content_type = line.split(":", 1)[1].strip()
print(json.dumps({
    "status": int(sys.argv[1]),
    "contentType": content_type,
    "bodyBytes": len(body),
    "problem": json.loads(body) if body and "json" in content_type else None,
}, sort_keys=True))
PY
}

assert_frozen_oversize() {
  local result="$1"
  python3 - "$result" <<'PY'
import json, sys
row = json.loads(sys.argv[1])
problem = row["problem"] or {}
if row["status"] != 500 or row["contentType"] != "application/problem+json":
    raise SystemExit(f"frozen oversized-body response changed: {row!r}")
expected = {
    "title": "Internal Server Error",
    "status": 500,
    "detail": "An unexpected error occurred.",
}
if any(problem.get(key) != value for key, value in expected.items()) or not problem.get("traceId"):
    raise SystemExit(f"frozen oversized-body Problem Details changed: {row!r}")
PY
}

options_snapshot() {
  local base_url="$1"
  curl --silent --show-error --max-time 10 -H "$request_auth_header" \
    "$base_url/api/v0/options"
}

application_snapshot() {
  local base_url="$1"
  curl --silent --show-error --max-time 10 -H "$request_auth_header" \
    "$base_url/api/v0/application"
}

capture_validation_suite() {
  local base_url="$1"
  local implementation="$2"
  python3 - "$base_url" "$request_auth_header" >"$work_dir/$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
auth_name, auth_value = sys.argv[2].split(": ", 1)
cases = {
    "null": "web:\n  max_request_body_size: null\n",
    "minimum": "web:\n  max_request_body_size: 1\n",
    "maximum": "web:\n  max_request_body_size: 2147483647\n",
    "zero": "web:\n  max_request_body_size: 0\n",
    "negative": "web:\n  max_request_body_size: -1\n",
    "overflow": "web:\n  max_request_body_size: 2147483648\n",
    "array": "web:\n  max_request_body_size: []\n",
    "numeric-string": "web:\n  max_request_body_size: '2048'\n",
    "invalid-string": "web:\n  max_request_body_size: nope\n",
    "web-array": "web: []\n",
}
for label, yaml in cases.items():
    connection = http.client.HTTPConnection(url.hostname, url.port, timeout=10)
    connection.request(
        "POST",
        "/api/v0/options/yaml/validate",
        body=json.dumps(yaml, separators=(",", ":")),
        headers={"Content-Type": "application/json", auth_name: auth_value},
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

run_configured_suite() {
  local implementation="$1"
  if [[ "$implementation" == upstream ]]; then
    request_auth_header="X-API-Key: $api_key"
  else
    request_auth_header="Authorization: Bearer $api_key"
  fi
  local state="$work_dir/$implementation-configured"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local log="$work_dir/$implementation-configured.log"
  local base_url="http://127.0.0.1:$port"
  write_config "$state" 1024
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  authenticate_admin "$base_url" "$implementation"
  capture_validation_suite "$base_url" "$implementation"

  local at_limit over_limit
  at_limit="$(probe_body "$base_url" 1024 "$implementation-at-limit")"
  over_limit="$(probe_body "$base_url" 1025 "$implementation-over-limit")"
  python3 - "$at_limit" <<'PY'
import json, sys
row = json.loads(sys.argv[1])
if row["status"] == 500:
    raise SystemExit(f"body at configured limit was rejected: {row!r}")
PY
  assert_frozen_oversize "$over_limit"

  write_config "$state" 2048
  local current=""
  local application=""
  for _ in $(seq 1 100); do
    current="$(options_snapshot "$base_url")"
    application="$(application_snapshot "$base_url")"
    if python3 - "$current" "$application" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
application = json.loads(sys.argv[2])
raise SystemExit(0 if value.get("web", {}).get("maxRequestBodySize") == 2048 and application.get("pendingRestart") is True else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$current" "$application" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
application = json.loads(sys.argv[2])
if value.get("web", {}).get("maxRequestBodySize") != 2048 or application.get("pendingRestart") is not True:
    raise SystemExit(f"watched body limit was not projected as restart-pending: options={value!r} application={application!r}")
PY
  assert_frozen_oversize "$(probe_body "$base_url" 1025 "$implementation-before-restart")"

  stop_daemon
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  authenticate_admin "$base_url" "$implementation"
  local after_restart
  after_restart="$(probe_body "$base_url" 1025 "$implementation-after-restart")"
  python3 - "$after_restart" <<'PY'
import json, sys
row = json.loads(sys.argv[1])
if row["status"] == 500:
    raise SystemExit(f"new body limit was not active after restart: {row!r}")
PY
  stop_daemon
}

[[ -f "$dll" ]] || { printf 'missing frozen slskdN binary: %s\n' "$dll" >&2; exit 1; }
cargo build -p slskr --manifest-path "$repo_root/Cargo.toml" >/dev/null
run_configured_suite upstream
run_configured_suite slskr
diff -u "$work_dir/upstream-validation.jsonl" "$work_dir/slskr-validation.jsonl"
printf 'web request-body limit differential passed against frozen slskdN; frozen 500 defect is tracked by draft PR #276\n'
