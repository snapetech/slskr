#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
dll="$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
work_dir="${SLSKR_NO_AUTH_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-no-auth-differential.XXXXXX")}"
keep_artifacts="${SLSKR_NO_AUTH_DIFFERENTIAL_KEEP:-0}"
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
  local allow_remote="$2"
  local allowed_cidrs="$3"
  local enforce="${4:-false}"
  mkdir -p "$state"
  {
    printf '%s\n' \
      'flags:' \
      '  no_connect: true' \
      'remote_configuration: true' \
      'dht:' \
      '  enabled: false' \
      'web:' \
      "  enforce_security: $enforce" \
      "  allow_remote_no_auth: $allow_remote" \
      '  authentication:' \
      '    disabled: true' \
      '    passthrough:'
    if [[ "$allowed_cidrs" == __omit__ ]]; then
      printf '%s\n' '      {}'
    else
      printf "      allowed_cidrs: '%s'\n" "$allowed_cidrs"
    fi
    printf '%s\n' \
      '  rate_limiting:' \
      '    enabled: false'
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
      export SLSKD_HTTP_ADDRESS=0.0.0.0 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKR_REMOTE_CONFIGURATION=true
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

probe_status() {
  local url="$1"
  curl --noproxy '*' --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' "$url"
}

capture_mode() {
  local base_loopback="$1"
  local base_remote="$2"
  local implementation="$3"
  local mode="$4"
  local destination="$work_dir/$implementation-$mode.json"
  python3 - \
    "$(probe_status "$base_loopback/api/v0/application")" \
    "$(probe_status "$base_remote/api/v0/application")" \
    "$(probe_status "$base_remote/api/v0/application/build")" \
    "$mode" >"$destination" <<'PY'
import json, sys
print(json.dumps({
    "mode": sys.argv[4],
    "loopbackProtected": int(sys.argv[1]),
    "remoteProtected": int(sys.argv[2]),
    "remoteAnonymous": int(sys.argv[3]),
}, sort_keys=True))
PY
}

capture_validation_suite() {
  local base_url="$1"
  local implementation="$2"
  python3 - "$base_url" >"$work_dir/$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
cases = {
    "allow-null": "web:\n  allow_remote_no_auth: null\n",
    "allow-string": "web:\n  allow_remote_no_auth: 'true'\n",
    "allow-invalid": "web:\n  allow_remote_no_auth: 1\n",
    "auth-null": "web:\n  authentication: null\n",
    "auth-array": "web:\n  authentication: []\n",
    "disabled-null": "web:\n  authentication:\n    disabled: null\n",
    "disabled-string": "web:\n  authentication:\n    disabled: 'true'\n",
    "disabled-invalid": "web:\n  authentication:\n    disabled: nope\n",
    "passthrough-null": "web:\n  authentication:\n    passthrough: null\n",
    "passthrough-array": "web:\n  authentication:\n    passthrough: []\n",
    "cidrs-null": "web:\n  authentication:\n    passthrough:\n      allowed_cidrs: null\n",
    "cidrs-string": "web:\n  authentication:\n    passthrough:\n      allowed_cidrs: 192.0.2.0/24\n",
    "cidrs-number": "web:\n  authentication:\n    passthrough:\n      allowed_cidrs: 123\n",
    "cidrs-invalid-shape": "web:\n  authentication:\n    passthrough:\n      allowed_cidrs: []\n",
}
for label, yaml in cases.items():
    connection = http.client.HTTPConnection(url.hostname, url.port, timeout=10)
    connection.request(
        "POST",
        "/api/v0/options/yaml/validate",
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

run_mode() {
  local implementation="$1"
  local mode="$2"
  local allow=false cidrs='__omit__'
  case "$mode" in
    disabled-with-matching-cidr) cidrs="$host_ip/32" ;;
    enabled-mismatch) allow=true; cidrs='192.0.2.0/24' ;;
    enabled-match) allow=true; cidrs="$host_ip/32" ;;
    enabled-invalid) allow=true; cidrs='not-a-cidr' ;;
  esac
  local state="$work_dir/$implementation-$mode-state"
  local log="$work_dir/$implementation-$mode.log"
  local port https_port listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  write_config "$state" "$allow" "$cidrs"
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "http://127.0.0.1:$port" "$log"
  capture_mode "http://127.0.0.1:$port" "http://$host_ip:$port" "$implementation" "$mode"
  if [[ "$mode" == enabled-match ]]; then
    capture_validation_suite "http://127.0.0.1:$port" "$implementation"
  fi
  stop_daemon
}

options_snapshot() {
  local base_url="$1"
  curl --noproxy '*' --silent --show-error --max-time 10 "$base_url/api/v0/options"
}

application_snapshot() {
  local base_url="$1"
  curl --noproxy '*' --silent --show-error --max-time 10 "$base_url/api/v0/application"
}

run_watch_restart() {
  local implementation="$1"
  local kind="$2"
  local state="$work_dir/$implementation-watch-$kind-state"
  local log="$work_dir/$implementation-watch-$kind.log"
  local port https_port listen_port base_loopback base_remote expected_pending
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_loopback="http://127.0.0.1:$port"
  base_remote="http://$host_ip:$port"
  if [[ "$kind" == allow ]]; then
    write_config "$state" false "$host_ip/32"
    expected_pending=true
  else
    write_config "$state" true '192.0.2.0/24'
    expected_pending=false
  fi
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_loopback" "$log"
  [[ "$(probe_status "$base_remote/api/v0/application")" == 401 ]]

  write_config "$state" true "$host_ip/32"
  local options="" application=""
  for _ in $(seq 1 100); do
    options="$(options_snapshot "$base_loopback")"
    application="$(application_snapshot "$base_loopback")"
    if python3 - "$options" "$application" "$host_ip/32" "$expected_pending" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
web = options.get("web", {})
cidrs = web.get("authentication", {}).get("passthrough", {}).get("allowedCidrs")
expected_pending = sys.argv[4] == "true"
raise SystemExit(0 if web.get("allowRemoteNoAuth") is True and cidrs == sys.argv[3] and application.get("pendingRestart") is expected_pending else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$options" "$application" "$host_ip/32" "$expected_pending" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
web = options.get("web", {})
cidrs = web.get("authentication", {}).get("passthrough", {}).get("allowedCidrs")
expected_pending = sys.argv[4] == "true"
if web.get("allowRemoteNoAuth") is not True or cidrs != sys.argv[3] or application.get("pendingRestart") is not expected_pending:
    raise SystemExit(f"watched passthrough projection mismatch: web={web!r} application={application!r}")
PY
  [[ "$(probe_status "$base_remote/api/v0/application")" == 401 ]]
  stop_daemon

  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_loopback" "$log"
  [[ "$(probe_status "$base_remote/api/v0/application")" == 200 ]]
  stop_daemon
}

run_hardening_rejection() {
  local implementation="$1"
  local rule="$2"
  local allow=false cidrs='__omit__'
  if [[ "$rule" == RemoteNoAuthWithoutCidrs ]]; then
    allow=true
  fi
  local state="$work_dir/$implementation-hardening-$rule-state"
  local log="$work_dir/$implementation-hardening-$rule.log"
  local port https_port listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  write_config "$state" "$allow" "$cidrs" true
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  for _ in $(seq 1 200); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      wait "$daemon_pid" 2>/dev/null || true
      daemon_pid=""
      rg -q "$rule" "$log" || { tail -120 "$log" >&2; return 1; }
      return
    fi
    sleep 0.05
  done
  tail -120 "$log" >&2 || true
  printf '%s did not reject hardening rule %s\n' "$implementation" "$rule" >&2
  return 1
}

[[ -f "$dll" ]] || { printf 'missing frozen slskdN binary: %s\n' "$dll" >&2; exit 1; }
cargo build -p slskr --manifest-path "$repo_root/Cargo.toml" >/dev/null
for mode in disabled-with-matching-cidr enabled-mismatch enabled-match enabled-invalid; do
  run_mode upstream "$mode"
  run_mode slskr "$mode"
  diff -u "$work_dir/upstream-$mode.json" "$work_dir/slskr-$mode.json"
done
diff -u "$work_dir/upstream-validation.jsonl" "$work_dir/slskr-validation.jsonl"
for kind in allow cidrs; do
  run_watch_restart upstream "$kind"
  run_watch_restart slskr "$kind"
done
for rule in AuthDisabledNonLoopback RemoteNoAuthWithoutCidrs; do
  run_hardening_rejection upstream "$rule"
  run_hardening_rejection slskr "$rule"
done
printf 'web no-auth passthrough differential passed against frozen slskdN\n'
