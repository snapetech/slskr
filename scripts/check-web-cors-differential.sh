#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
dll="$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
work_dir="${SLSKR_CORS_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-cors-differential.XXXXXX")}"
keep_artifacts="${SLSKR_CORS_DIFFERENTIAL_KEEP:-0}"
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
  local enabled="$2"
  local credentials="$3"
  local origins="$4"
  local headers="$5"
  local methods="$6"
  local enforce_security="${7:-false}"
  mkdir -p "$state"
  {
    printf '%s\n' \
      'flags:' \
      '  no_connect: true' \
      'remote_configuration: true' \
      'dht:' \
      '  enabled: false' \
      'web:' \
      "  enforce_security: $enforce_security" \
      '  authentication:' \
      '    api_keys:' \
      '      differential:' \
      "        key: $api_key" \
      '        role: administrator' \
      '        cidr: 127.0.0.0/8' \
      '  rate_limiting:' \
      '    enabled: false' \
      '  cors:' \
      "    enabled: $enabled" \
      "    allow_credentials: $credentials" \
      "    allowed_origins: $origins" \
      "    allowed_headers: $headers" \
      "    allowed_methods: $methods"
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
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_REMOTE_CONFIGURATION=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=false SLSKR_API_TOKEN="$api_key"
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

probe() {
  local base_url="$1"
  local label="$2"
  local method="$3"
  local origin="$4"
  local requested_method="$5"
  local requested_headers="$6"
  local destination="$7"
  local headers_file="$work_dir/$label.headers"
  local body_file="$work_dir/$label.body"
  local args=(-X "$method")
  [[ "$origin" == none ]] || args+=(-H "Origin: $origin")
  [[ "$requested_method" == none ]] || args+=(-H "Access-Control-Request-Method: $requested_method")
  [[ "$requested_headers" == none ]] || args+=(-H "Access-Control-Request-Headers: $requested_headers")
  local status
  status="$(curl --silent --show-error --max-time 10 "${args[@]}" \
    -D "$headers_file" -o "$body_file" -w '%{http_code}' \
    "$base_url/api/v0/session")"
  python3 - "$label" "$status" "$headers_file" >>"$destination" <<'PY'
import json, pathlib, sys
headers = {}
for line in pathlib.Path(sys.argv[3]).read_text(encoding="iso-8859-1").splitlines():
    if ":" not in line:
        continue
    name, value = line.split(":", 1)
    name = name.strip().lower()
    if name.startswith("access-control-") or name == "vary":
        headers[name] = value.strip()
row = {"label": sys.argv[1], "headers": headers}
# Route authorization/method status is tracked by the endpoint denominator.
# The CORS denominator owns the middleware's genuine-preflight short circuit.
if "-preflight-" in sys.argv[1] and not sys.argv[1].startswith("disabled-"):
    row["status"] = int(sys.argv[2])
print(json.dumps(row, sort_keys=True))
PY
}

capture_matrix() {
  local base_url="$1"
  local implementation="$2"
  local mode="$3"
  local destination="$work_dir/$implementation-$mode.jsonl"
  : >"$destination"
  probe "$base_url" "$mode-normal-allowed" GET https://allowed.example none none "$destination"
  probe "$base_url" "$mode-normal-evil" GET https://evil.example none none "$destination"
  probe "$base_url" "$mode-preflight-allowed" OPTIONS https://allowed.example GET X-Custom "$destination"
  probe "$base_url" "$mode-preflight-unlisted-request" OPTIONS https://allowed.example DELETE X-Bad "$destination"
  probe "$base_url" "$mode-preflight-evil" OPTIONS https://evil.example GET X-Custom "$destination"
  probe "$base_url" "$mode-ordinary-options" OPTIONS https://allowed.example none none "$destination"
  probe "$base_url" "$mode-no-origin" GET none none none "$destination"
}

capture_validation_suite() {
  local base_url="$1"
  local implementation="$2"
  python3 - "$base_url" "$request_auth_header" >"$work_dir/$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
auth_name, auth_value = sys.argv[2].split(": ", 1)
cases = {
    "enforce-null": "web:\n  enforce_security: null\n",
    "enforce-string": "web:\n  enforce_security: 'true'\n",
    "enforce-invalid": "web:\n  enforce_security: nope\n",
    "enforce-wildcard-startup-only": "web:\n  enforce_security: true\n  cors:\n    enabled: true\n    allow_credentials: true\n    allowed_origins: ['*']\n",
    "parent-null": "web:\n  cors: null\n",
    "parent-array": "web:\n  cors: []\n",
    "enabled-null": "web:\n  cors:\n    enabled: null\n",
    "enabled-string": "web:\n  cors:\n    enabled: 'true'\n",
    "enabled-invalid": "web:\n  cors:\n    enabled: nope\n",
    "credentials-null": "web:\n  cors:\n    allow_credentials: null\n",
    "credentials-invalid": "web:\n  cors:\n    allow_credentials: 1\n",
    "arrays-null": "web:\n  cors:\n    allowed_origins: null\n    allowed_headers: null\n    allowed_methods: null\n",
    "arrays-empty": "web:\n  cors:\n    allowed_origins: []\n    allowed_headers: []\n    allowed_methods: []\n",
    "origin-scalar": "web:\n  cors:\n    allowed_origins: '*'\n",
    "origin-number": "web:\n  cors:\n    allowed_origins: [1]\n",
    "header-mixed": "web:\n  cors:\n    allowed_headers: [X-Good, 1]\n",
    "header-bool": "web:\n  cors:\n    allowed_headers: [true]\n",
    "method-null-element": "web:\n  cors:\n    allowed_methods: [null]\n",
    "origin-object-element": "web:\n  cors:\n    allowed_origins: [{}]\n",
    "method-array-element": "web:\n  cors:\n    allowed_methods: [[]]\n",
    "method-object": "web:\n  cors:\n    allowed_methods: {}\n",
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

run_mode() {
  local implementation="$1"
  local mode="$2"
  local enabled=false credentials=false enforce_security=false origins='[]' headers='[]' methods='[]'
  case "$mode" in
    explicit)
      enabled=true
      origins='[https://allowed.example, https://other.example]'
      headers='[X-Custom, Content-Type]'
      methods='[GET, POST]'
      ;;
    wildcard)
      enabled=true
      origins='["*"]'
      ;;
    credentials)
      enabled=true
      credentials=true
      origins='[https://allowed.example]'
      ;;
    enforced-credentials)
      enabled=true
      credentials=true
      enforce_security=true
      origins='[https://allowed.example]'
      ;;
    disabled) ;;
  esac
  local state="$work_dir/$implementation-$mode-state"
  local log="$work_dir/$implementation-$mode.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" "$enabled" "$credentials" "$origins" "$headers" "$methods" "$enforce_security"
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  capture_matrix "$base_url" "$implementation" "$mode"
  if [[ "$mode" == explicit ]]; then
    authenticate_admin "$base_url" "$implementation"
    capture_validation_suite "$base_url" "$implementation"
  fi
  stop_daemon
}

run_enforced_wildcard_rejection() {
  local implementation="$1"
  local state="$work_dir/$implementation-enforced-wildcard-state"
  local log="$work_dir/$implementation-enforced-wildcard.log"
  local port https_port listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  write_config "$state" true true '["*"]' '[]' '[]' true
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  local exited=false
  for _ in $(seq 1 200); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      exited=true
      break
    fi
    sleep 0.05
  done
  if [[ "$exited" != true ]]; then
    tail -120 "$log" >&2 || true
    printf '%s accepted credentialed wildcard CORS with enforce_security=true\n' "$implementation" >&2
    return 1
  fi
  wait "$daemon_pid" 2>/dev/null || true
  daemon_pid=""
  rg -q 'CorsCredentialsWithWildcard' "$log" || {
    tail -120 "$log" >&2 || true
    printf '%s startup rejection omitted the frozen rule name\n' "$implementation" >&2
    return 1
  }
}

run_watch_restart() {
  local implementation="$1"
  local state="$work_dir/$implementation-watch-state"
  local log="$work_dir/$implementation-watch.log"
  local port https_port listen_port base_url current application
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" true false '[https://allowed.example]' '[]' '[]'
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  authenticate_admin "$base_url" "$implementation"

  write_config "$state" true false '[https://replacement.example]' '[X-Replacement]' '[PATCH]'
  current=""
  application=""
  for _ in $(seq 1 100); do
    current="$(options_snapshot "$base_url")"
    application="$(application_snapshot "$base_url")"
    if python3 - "$current" "$application" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
cors = options.get("web", {}).get("cors", {})
raise SystemExit(0 if cors.get("allowedOrigins") == ["https://replacement.example"] and application.get("pendingRestart") is False else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$current" "$application" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
cors = options.get("web", {}).get("cors", {})
expected = {
    "enabled": True,
    "allowCredentials": False,
    "allowedOrigins": ["https://replacement.example"],
    "allowedHeaders": ["X-Replacement"],
    "allowedMethods": ["PATCH"],
}
if cors != expected or application.get("pendingRestart") is not False:
    raise SystemExit(f"watched CORS projection or frozen restart cue changed: cors={cors!r} application={application!r}")
PY
  local before="$work_dir/$implementation-watch-before.jsonl"
  : >"$before"
  probe "$base_url" watch-old-before GET https://allowed.example none none "$before"
  probe "$base_url" watch-new-before GET https://replacement.example none none "$before"
  stop_daemon

  request_auth_header="X-API-Key: $api_key"
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  local after="$work_dir/$implementation-watch-after.jsonl"
  : >"$after"
  probe "$base_url" watch-old-after GET https://allowed.example none none "$after"
  probe "$base_url" watch-new-after GET https://replacement.example none none "$after"
  stop_daemon
}

[[ -f "$dll" ]] || { printf 'missing frozen slskdN binary: %s\n' "$dll" >&2; exit 1; }
cargo build -p slskr --manifest-path "$repo_root/Cargo.toml" >/dev/null
for mode in disabled explicit wildcard credentials enforced-credentials; do
  run_mode upstream "$mode"
  request_auth_header="X-API-Key: $api_key"
  run_mode slskr "$mode"
  diff -u "$work_dir/upstream-$mode.jsonl" "$work_dir/slskr-$mode.jsonl"
done
run_enforced_wildcard_rejection upstream
run_enforced_wildcard_rejection slskr
diff -u "$work_dir/upstream-validation.jsonl" "$work_dir/slskr-validation.jsonl"
request_auth_header="X-API-Key: $api_key"
run_watch_restart upstream
request_auth_header="X-API-Key: $api_key"
run_watch_restart slskr
diff -u "$work_dir/upstream-watch-before.jsonl" "$work_dir/slskr-watch-before.jsonl"
diff -u "$work_dir/upstream-watch-after.jsonl" "$work_dir/slskr-watch-after.jsonl"
printf 'web CORS differential passed against frozen slskdN\n'
