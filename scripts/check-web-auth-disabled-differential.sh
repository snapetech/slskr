#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskd_root="${SLSKR_SLSKD_ROOT:-/tmp/slskr-parity-slskd}"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
work_dir="${SLSKR_AUTH_DISABLED_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-auth-disabled.XXXXXX")}"
keep_artifacts="${SLSKR_AUTH_DISABLED_DIFFERENTIAL_KEEP:-0}"
daemon_pid=""

cleanup() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  if [[ "$keep_artifacts" != 1 ]]; then
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

pick_free_port() {
  python3 - <<'PY'
import socket
with socket.socket() as sock:
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

write_auth_yaml() {
  local state="$1"
  local disabled="$2"
  mkdir -p "$state"
  {
    printf '%s\n' \
      'flags:' \
      '  no_connect: true' \
      'remote_configuration: true' \
      'dht:' \
      '  enabled: false' \
      'web:' \
      '  rate_limiting:' \
      '    enabled: false' \
      '  authentication:' \
      "    disabled: $disabled"
  } >"$state/slskd.yml.tmp"
  mv "$state/slskd.yml.tmp" "$state/slskd.yml"
}

start_daemon() {
  local target="$1"
  local implementation="$2"
  local state="$3"
  local port="$4"
  local https_port="$5"
  local listen_port="$6"
  local log="$7"
  shift 7
  if [[ "$implementation" == upstream ]]; then
    local root="$slskd_root"
    [[ "$target" == slskdn ]] && root="$slskdn_root"
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_REMOTE_CONFIGURATION=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$dll" "$@"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$port" \
        --slsk-listen-port "$listen_port" --no-connect --remote-configuration "$@"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_ready() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 400); do
    if curl --silent --fail --max-time 1 "$base_url/api/v0/session/enabled" >/dev/null; then
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

status() {
  local url="$1"
  local authorization="${2:-}"
  local args=(--silent --output /dev/null --write-out '%{http_code}' --max-time 10)
  [[ -z "$authorization" ]] || args+=(--header "Authorization: $authorization")
  curl "${args[@]}" "$url"
}

login() {
  local base_url="$1"
  local username="$2"
  local password="$3"
  local output="$4"
  curl --silent --max-time 10 --output "$output" --write-out '%{http_code}' \
    --header 'Content-Type: application/json' \
    --data "{\"username\":\"$username\",\"password\":\"$password\"}" \
    "$base_url/api/v0/session"
}

capture_default() {
  local target="$1"
  local implementation="$2"
  local state="$work_dir/$target-$implementation-default-state"
  local log="$work_dir/$target-$implementation-default.log"
  local port https_port listen_port base_url login_body login_status token
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  mkdir -p "$state"
  start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  login_body="$work_dir/$target-$implementation-login.json"
  login_status="$(login "$base_url" slskd slskd "$login_body")"
  token="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("token", ""))' "$login_body")"
  python3 - "$base_url" "$login_body" "$login_status" "$token" >"$work_dir/$target-$implementation-default.json" <<'PY'
import json, subprocess, sys
base, login_path, login_status, token = sys.argv[1:]
login = json.load(open(login_path, encoding="utf-8"))
def curl(path, authorization=None):
    command = ["curl", "--silent", "--output", "/dev/null", "--write-out", "%{http_code}", "--max-time", "10"]
    if authorization:
        command += ["--header", f"Authorization: {authorization}"]
    command.append(base + path)
    return int(subprocess.check_output(command, text=True))
enabled = subprocess.check_output(["curl", "--silent", "--max-time", "10", base + "/api/v0/session/enabled"], text=True)
bad = subprocess.check_output([
    "curl", "--silent", "--output", "/dev/null", "--write-out", "%{http_code}", "--max-time", "10",
    "--header", "Content-Type: application/json", "--data", '{"username":"slskd","password":"wrong"}',
    base + "/api/v0/session",
], text=True)
print(json.dumps({
    "enabled": enabled.strip(),
    "anonymousCheck": curl("/api/v0/session"),
    "anonymousProtected": curl("/api/v0/application"),
    "login": int(login_status),
    "badLogin": int(bad),
    "name": login.get("name"),
    "tokenType": login.get("tokenType"),
    "ttl": login.get("expires", 0) - login.get("issued", 0),
    "jwtCheck": curl("/api/v0/session", "Bearer " + token),
    "jwtProtected": curl("/api/v0/application", "Bearer " + token),
}, sort_keys=True))
PY
  stop_daemon
}

capture_layering() {
  local target="$1"
  local implementation="$2"
  local mode="$3"
  local state="$work_dir/$target-$implementation-$mode-state"
  local log="$work_dir/$target-$implementation-$mode.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  case "$mode" in
    yaml-over-env)
      write_auth_yaml "$state" false
      export SLSKD_NO_AUTH=true
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
      unset SLSKD_NO_AUTH
      ;;
    cli-over-yaml)
      write_auth_yaml "$state" false
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log" --no-auth
      ;;
  esac
  wait_ready "$base_url" "$log"
  printf '%s|%s\n' \
    "$(curl --silent --max-time 10 "$base_url/api/v0/session/enabled")" \
    "$(status "$base_url/api/v0/application")" \
    >"$work_dir/$target-$implementation-$mode.txt"
  stop_daemon
}

capture_watch() {
  local target="$1"
  local implementation="$2"
  local state="$work_dir/$target-$implementation-watch-state"
  local log="$work_dir/$target-$implementation-watch.log"
  local port https_port listen_port base_url options application
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_auth_yaml "$state" true
  start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  write_auth_yaml "$state" false
  for _ in $(seq 1 120); do
    options="$(curl --silent --max-time 10 "$base_url/api/v0/options")"
    application="$(curl --silent --max-time 10 "$base_url/api/v0/application")"
    if python3 - "$options" "$application" <<'PY'
import json, sys
options, application = map(json.loads, sys.argv[1:])
raise SystemExit(0 if options["web"]["authentication"]["disabled"] is False and application["pendingRestart"] is True else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$options" "$application" \
    "$(curl --silent --max-time 10 "$base_url/api/v0/session/enabled")" \
    "$(status "$base_url/api/v0/application")" \
    >"$work_dir/$target-$implementation-watch.json" <<'PY'
import json, sys
options, application = map(json.loads, sys.argv[1:3])
print(json.dumps({
    "projectedDisabled": options["web"]["authentication"]["disabled"],
    "pendingRestart": application["pendingRestart"],
    "startupEnabled": sys.argv[3],
    "startupProtected": int(sys.argv[4]),
}, sort_keys=True))
PY
  stop_daemon

  start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  printf '%s|%s\n' \
    "$(curl --silent --max-time 10 "$base_url/api/v0/session/enabled")" \
    "$(status "$base_url/api/v0/application")" \
    >"$work_dir/$target-$implementation-restart.txt"
  stop_daemon
}

cd "$repo_root"
cargo build -q -p slskr

for target in slskd slskdn; do
  for implementation in upstream slskr; do
    capture_default "$target" "$implementation"
    capture_layering "$target" "$implementation" yaml-over-env
    capture_layering "$target" "$implementation" cli-over-yaml
    capture_watch "$target" "$implementation"
  done
  for suffix in default.json yaml-over-env.txt cli-over-yaml.txt watch.json restart.txt; do
    if ! cmp --silent "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix"; then
      printf 'web authentication disabled differential failed: %s %s\n' "$target" "$suffix" >&2
      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
      exit 1
    fi
  done
done

printf 'web authentication disabled differential passed for frozen slskd and slskdN\n'
