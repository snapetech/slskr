#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskd_root="${SLSKR_SLSKD_ROOT:-/tmp/slskr-parity-slskd}"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
work_dir="${SLSKR_AUTH_CREDENTIALS_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-auth-credentials.XXXXXX")}"
keep_artifacts="${SLSKR_AUTH_CREDENTIALS_DIFFERENTIAL_KEEP:-0}"
daemon_pid=""

cleanup() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  [[ "$keep_artifacts" == 1 ]] || rm -rf "$work_dir"
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

write_yaml() {
  local state="$1" disabled="$2" username="$3" password="$4" key="$5" ttl="$6"
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
    if [[ "$username" == __null__ ]]; then
      printf '%s\n' '    username: null'
    elif [[ "$username" != __omit__ ]]; then
      printf "    username: '%s'\n" "$username"
    fi
    if [[ "$password" == __null__ ]]; then
      printf '%s\n' '    password: null'
    elif [[ "$password" != __omit__ ]]; then
      printf "    password: '%s'\n" "$password"
    fi
    if [[ "$key" != __omit__ || "$ttl" != __omit__ ]]; then
      printf '%s\n' '    jwt:'
      if [[ "$key" == __null__ ]]; then
        printf '%s\n' '      key: null'
      elif [[ "$key" != __omit__ ]]; then
        printf "      key: '%s'\n" "$key"
      fi
      [[ "$ttl" == __omit__ ]] || printf "      ttl: %s\n" "$ttl"
    fi
  } >"$state/slskd.yml.tmp"
  mv "$state/slskd.yml.tmp" "$state/slskd.yml"
}

start_daemon() {
  local target="$1" implementation="$2" state="$3" port="$4" https_port="$5" listen_port="$6" log="$7"
  shift 7
  if [[ "$implementation" == upstream ]]; then
    local root="$slskd_root"
    [[ "$target" == slskdn ]] && root="$slskdn_root"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_REMOTE_CONFIGURATION=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll" "$@"
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
  local base_url="$1" log="$2"
  for _ in $(seq 1 500); do
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

login() {
  local base_url="$1" username="$2" password="$3" output="$4"
  curl --silent --max-time 10 --output "$output" --write-out '%{http_code}' \
    --header 'Content-Type: application/json' \
    --data "{\"username\":\"$username\",\"password\":\"$password\"}" \
    "$base_url/api/v0/session"
}

token_from() {
  python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("token", ""))' "$1"
}

status() {
  local url="$1" token="${2:-}"
  local args=(--silent --output /dev/null --write-out '%{http_code}' --max-time 10)
  [[ -z "$token" ]] || args+=(--header "Authorization: Bearer $token")
  curl "${args[@]}" "$url"
}

capture_effective() {
  local target="$1" implementation="$2" mode="$3"
  local state="$work_dir/$target-$implementation-$mode-state"
  local log="$work_dir/$target-$implementation-$mode.log"
  local port https_port listen_port base_url username password bad_username bad_password expected_ttl
  port="$(pick_free_port)"; https_port="$(pick_free_port)"; listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  username=slskd; password=slskd; bad_username=wrong; bad_password=wrong
  expected_ttl=604800000
  [[ "$target" == slskdn ]] && expected_ttl=3600000
  mkdir -p "$state"
  unset SLSKD_USERNAME SLSKD_PASSWORD SLSKD_JWT_KEY SLSKD_JWT_TTL
  export SLSKD_DEBUG=true
  case "$mode" in
    default)
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
      ;;
    environment)
      username=environment-user; password=environment-pass; expected_ttl=7200000
      export SLSKD_USERNAME="$username" SLSKD_PASSWORD="$password"
      export SLSKD_JWT_KEY=eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee SLSKD_JWT_TTL="$expected_ttl"
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
      unset SLSKD_USERNAME SLSKD_PASSWORD SLSKD_JWT_KEY SLSKD_JWT_TTL
      ;;
    yaml-over-env)
      username=yaml-user; password=yaml-pass; expected_ttl=8400000
      write_yaml "$state" false "$username" "$password" yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy "$expected_ttl"
      export SLSKD_USERNAME=environment-user SLSKD_PASSWORD=environment-pass
      export SLSKD_JWT_KEY=eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee SLSKD_JWT_TTL=7200000
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
      unset SLSKD_USERNAME SLSKD_PASSWORD SLSKD_JWT_KEY SLSKD_JWT_TTL
      ;;
    cli-over-yaml)
      username=cli-user; password=cli-pass; expected_ttl=9600000
      write_yaml "$state" false yaml-user yaml-pass yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy 8400000
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log" \
        --username "$username" --password "$password" \
        --jwt-key cccccccccccccccccccccccccccccccc --jwt-ttl "$expected_ttl"
      ;;
    cli-short-over-yaml)
      username=short-user; password=short-pass; expected_ttl=9600000
      write_yaml "$state" false yaml-user yaml-pass yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy 8400000
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log" \
        -u "$username" -p "$password" \
        --jwt-key cccccccccccccccccccccccccccccccc --jwt-ttl "$expected_ttl"
      ;;
    null-credentials)
      write_yaml "$state" false __null__ __null__ __omit__ __omit__
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
      ;;
    null-jwt-leaves)
      write_yaml "$state" false __omit__ __omit__ __null__ null
      start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
      ;;
  esac
  unset SLSKD_DEBUG
  wait_ready "$base_url" "$log"
  local good_body="$work_dir/$target-$implementation-$mode-good.json"
  local bad_body="$work_dir/$target-$implementation-$mode-bad.json"
  local good_status bad_status token options debug_view
  good_status="$(login "$base_url" "$username" "$password" "$good_body")"
  bad_status="$(login "$base_url" "$bad_username" "$bad_password" "$bad_body")"
  token="$(token_from "$good_body")"
  options="$(curl --silent --max-time 10 --header "Authorization: Bearer $token" "$base_url/api/v0/options")"
  debug_view="$(curl --silent --max-time 10 --header "Authorization: Bearer $token" "$base_url/api/v0/options/debug")"
  python3 - "$good_body" "$good_status" "$bad_status" "$options" "$expected_ttl" \
    "$username" "$password" "$debug_view" "$target" >"$work_dir/$target-$implementation-$mode.json" <<'PY'
import json, pathlib, sys
login = json.load(open(sys.argv[1], encoding="utf-8"))
options = json.loads(sys.argv[4])
auth = options["web"]["authentication"]
raw = sys.argv[4]
debug = json.loads(sys.argv[8])
target = sys.argv[9]
lines = {}
in_web = in_auth = in_jwt = False
for line in debug.splitlines():
    if line == "  web:":
        in_web = True
        continue
    if in_web and line.startswith("  ") and not line.startswith("    "):
        break
    if in_web and line == "    authentication:":
        in_auth = True
        continue
    if in_auth and line.startswith("    ") and not line.startswith("      "):
        in_auth = False
    if in_auth and line == "      jwt:":
        in_jwt = True
        continue
    if in_jwt and line.startswith("      ") and not line.startswith("        "):
        in_jwt = False
    if in_auth and line.startswith("      username="):
        lines["username"] = line.strip().split("=", 1)[1].split(" (", 1)[0]
    elif in_auth and line.startswith("      password="):
        lines["password"] = line.strip().split("=", 1)[1].split(" (", 1)[0]
    elif in_jwt and line.startswith("        key="):
        lines["key"] = line.strip().split("=", 1)[1].split(" (", 1)[0]
    elif in_jwt and line.startswith("        ttl="):
        lines["ttl"] = line.strip().split("=", 1)[1].split(" (", 1)[0]
assert int(sys.argv[2]) == 200
assert int(sys.argv[3]) == 401
assert login.get("name") == sys.argv[6]
assert login.get("tokenType") == "Bearer"
assert login.get("expires", 0) - login.get("issued", 0) == int(sys.argv[5]) // 1000
assert auth.get("username") == sys.argv[6]
assert auth.get("password") == "*****"
assert auth.get("jwt", {}).get("key") == "*****"
assert auth.get("jwt", {}).get("ttl") == int(sys.argv[5])
assert lines["username"] == sys.argv[6]
assert lines["ttl"] == sys.argv[5]
assert lines["password"] == (sys.argv[7] if target == "slskd" else "*****")
assert (lines["key"] == "*****") is (target == "slskdn")
print(json.dumps({
    "goodLogin": int(sys.argv[2]),
    "badLogin": int(sys.argv[3]),
    "name": login.get("name"),
    "tokenType": login.get("tokenType"),
    "issuedTtl": login.get("expires", 0) - login.get("issued", 0),
    "expectedTtl": int(sys.argv[5]) // 1000,
    "projectedUsername": auth.get("username"),
    "projectedPassword": auth.get("password"),
    "projectedJwtKey": auth.get("jwt", {}).get("key"),
    "projectedJwtTtl": auth.get("jwt", {}).get("ttl"),
    "passwordLeaked": sys.argv[7] in raw,
    "debugUsername": lines["username"],
    "debugPassword": lines["password"],
    "debugJwtKeyProfile": "masked" if lines["key"] == "*****" else "clear",
    "debugJwtTtl": int(lines["ttl"]),
}, sort_keys=True))
PY
  stop_daemon
}

capture_validation() {
  local target="$1" implementation="$2"
  local state="$work_dir/$target-$implementation-validation-state"
  local log="$work_dir/$target-$implementation-validation.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"; https_port="$(pick_free_port)"; listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_yaml "$state" true __omit__ __omit__ __omit__ __omit__
  start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  python3 - "$base_url" >"$work_dir/$target-$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
long = "x" * 256
cases = {
    "username-null": "web:\n  authentication:\n    username: null\n",
    "username-empty": "web:\n  authentication:\n    username: ''\n",
    "username-space": "web:\n  authentication:\n    username: ' '\n",
    "username-bool": "web:\n  authentication:\n    username: true\n",
    "username-long": f"web:\n  authentication:\n    username: '{long}'\n",
    "username-array": "web:\n  authentication:\n    username: []\n",
    "password-empty": "web:\n  authentication:\n    password: ''\n",
    "password-space": "web:\n  authentication:\n    password: ' '\n",
    "password-bool": "web:\n  authentication:\n    password: true\n",
    "jwt-null": "web:\n  authentication:\n    jwt: null\n",
    "jwt-array": "web:\n  authentication:\n    jwt: []\n",
    "key-null": "web:\n  authentication:\n    jwt:\n      key: null\n",
    "key-short": "web:\n  authentication:\n    jwt:\n      key: '" + "k" * 31 + "'\n",
    "key-min": "web:\n  authentication:\n    jwt:\n      key: '" + "k" * 32 + "'\n",
    "key-long": "web:\n  authentication:\n    jwt:\n      key: '" + "k" * 256 + "'\n",
    "ttl-null": "web:\n  authentication:\n    jwt:\n      ttl: null\n",
    "ttl-low": "web:\n  authentication:\n    jwt:\n      ttl: 3599\n",
    "ttl-min": "web:\n  authentication:\n    jwt:\n      ttl: 3600\n",
    "ttl-max": "web:\n  authentication:\n    jwt:\n      ttl: 2147483647\n",
    "ttl-over": "web:\n  authentication:\n    jwt:\n      ttl: 2147483648\n",
    "ttl-string": "web:\n  authentication:\n    jwt:\n      ttl: '3600'\n",
}
for label, yaml in cases.items():
    connection = http.client.HTTPConnection(url.hostname, url.port, timeout=10)
    connection.request("POST", "/api/v0/options/yaml/validate", json.dumps(yaml), {"Content-Type": "application/json"})
    response = connection.getresponse()
    body = response.read()
    try:
        value = json.loads(body) if body else None
    except json.JSONDecodeError:
        value = body.decode("utf-8", "replace")
    print(json.dumps({"label": label, "status": response.status, "body": value}, sort_keys=True))
    connection.close()
PY
  stop_daemon
}

capture_watch() {
  local target="$1" implementation="$2"
  local state="$work_dir/$target-$implementation-watch-state"
  local log="$work_dir/$target-$implementation-watch.log"
  local port https_port listen_port base_url initial_ttl
  port="$(pick_free_port)"; https_port="$(pick_free_port)"; listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  initial_ttl=7200000
  write_yaml "$state" false before-user before-pass aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa "$initial_ttl"
  export SLSKD_DEBUG=true
  start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  unset SLSKD_DEBUG
  wait_ready "$base_url" "$log"
  local initial_body="$work_dir/$target-$implementation-watch-initial.json"
  [[ "$(login "$base_url" before-user before-pass "$initial_body")" == 200 ]]
  local initial_token
  initial_token="$(token_from "$initial_body")"

  write_yaml "$state" false after-user after-pass aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa "$initial_ttl"
  local options application
  for _ in $(seq 1 120); do
    options="$(curl --silent --max-time 10 --header "Authorization: Bearer $initial_token" "$base_url/api/v0/options")"
    application="$(curl --silent --max-time 10 --header "Authorization: Bearer $initial_token" "$base_url/api/v0/application")"
    if python3 - "$options" "$application" <<'PY'
import json,sys
o,a=map(json.loads,sys.argv[1:])
raise SystemExit(0 if o["web"]["authentication"]["username"] == "after-user" and a["pendingRestart"] is False else 1)
PY
    then break; fi
    sleep 0.1
  done
  python3 - "$options" "$application" <<'PY'
import json,sys
options,application=map(json.loads,sys.argv[1:])
auth=options["web"]["authentication"]
assert auth["username"] == "after-user"
assert auth["password"] == "*****"
assert auth["jwt"]["ttl"] == 7200000
assert application["pendingRestart"] is False
PY
  local old_body="$work_dir/$target-$implementation-watch-old.json"
  local live_body="$work_dir/$target-$implementation-watch-live.json"
  local old_status live_status live_token
  old_status="$(login "$base_url" before-user before-pass "$old_body")"
  live_status="$(login "$base_url" after-user after-pass "$live_body")"
  live_token="$(token_from "$live_body")"

  write_yaml "$state" false after-user after-pass bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb 8400000
  for _ in $(seq 1 120); do
    options="$(curl --silent --max-time 10 --header "Authorization: Bearer $initial_token" "$base_url/api/v0/options")"
    application="$(curl --silent --max-time 10 --header "Authorization: Bearer $initial_token" "$base_url/api/v0/application")"
    if python3 - "$options" "$application" <<'PY'
import json,sys
o,a=map(json.loads,sys.argv[1:])
auth=o["web"]["authentication"]
raise SystemExit(0 if auth["jwt"]["ttl"] == 8400000 and a["pendingRestart"] is True else 1)
PY
    then break; fi
    sleep 0.1
  done
  python3 - "$options" "$application" <<'PY'
import json,sys
options,application=map(json.loads,sys.argv[1:])
auth=options["web"]["authentication"]
assert auth["username"] == "after-user"
assert auth["password"] == "*****"
assert auth["jwt"]["key"] == "*****"
assert auth["jwt"]["ttl"] == 8400000
assert application["pendingRestart"] is True
PY
  local pre_body="$work_dir/$target-$implementation-watch-pre.json"
  [[ "$(login "$base_url" after-user after-pass "$pre_body")" == 200 ]]
  local debug_view
  debug_view="$(curl --silent --max-time 10 --header "Authorization: Bearer $initial_token" "$base_url/api/v0/options/debug")"
  python3 - "$initial_body" "$live_body" "$pre_body" "$old_status" "$live_status" \
    "$(status "$base_url/api/v0/session" "$initial_token")" \
    "$(status "$base_url/api/v0/session" "$live_token")" "$options" "$application" \
    "$debug_view" "$target" \
    >"$work_dir/$target-$implementation-watch.json" <<'PY'
import json,sys
initial,live,pre=[json.load(open(path,encoding="utf-8")) for path in sys.argv[1:4]]
options,application=map(json.loads,sys.argv[8:10])
debug=json.loads(sys.argv[10]); target=sys.argv[11]
auth=options["web"]["authentication"]
assert initial["expires"]-initial["issued"] == 7200
assert int(sys.argv[4]) == 401
assert int(sys.argv[5]) == 200
assert int(sys.argv[6]) == 200
assert int(sys.argv[7]) == 200
assert live["expires"]-live["issued"] == 7200
assert pre["expires"]-pre["issued"] == 7200
if target == "slskd":
    assert "password=after-pass" in debug
    assert "key=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" in debug
else:
    assert "password=after-pass" not in debug
    assert "key=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" not in debug
print(json.dumps({
    "initialTtl": initial["expires"]-initial["issued"],
    "oldCredentialsAfterLiveChange": int(sys.argv[4]),
    "newCredentialsAfterLiveChange": int(sys.argv[5]),
    "initialTokenBeforeRestart": int(sys.argv[6]),
    "liveTokenBeforeRestart": int(sys.argv[7]),
    "liveTtlBeforeRestart": live["expires"]-live["issued"],
    "newTokenAfterJwtWatchBeforeRestartTtl": pre["expires"]-pre["issued"],
    "projectedUsername": auth["username"],
    "projectedPassword": auth["password"],
    "projectedKey": auth["jwt"]["key"],
    "projectedTtl": auth["jwt"]["ttl"],
    "pendingRestart": application["pendingRestart"],
    "debugPasswordProfile": "clear" if "password=after-pass" in debug else "masked",
    "debugKeyProfile": "clear" if "key=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" in debug else "masked",
},sort_keys=True))
PY
  stop_daemon

  export SLSKD_DEBUG=true
  start_daemon "$target" "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  unset SLSKD_DEBUG
  wait_ready "$base_url" "$log"
  local restart_body="$work_dir/$target-$implementation-watch-restart.json"
  local restart_status restart_token
  restart_status="$(login "$base_url" after-user after-pass "$restart_body")"
  restart_token="$(token_from "$restart_body")"
  python3 - "$restart_body" "$restart_status" \
    "$(status "$base_url/api/v0/session" "$initial_token")" \
    "$(status "$base_url/api/v0/session" "$restart_token")" \
    >"$work_dir/$target-$implementation-restart.json" <<'PY'
import json,sys
login=json.load(open(sys.argv[1],encoding="utf-8"))
assert int(sys.argv[2]) == 200
assert login["expires"]-login["issued"] == 8400
assert int(sys.argv[3]) == 401
assert int(sys.argv[4]) == 200
print(json.dumps({
    "login": int(sys.argv[2]),
    "ttl": login["expires"]-login["issued"],
    "oldKeyToken": int(sys.argv[3]),
    "newKeyToken": int(sys.argv[4]),
},sort_keys=True))
PY
  stop_daemon
}

cd "$repo_root"
cargo build -q -p slskr

for target in slskd slskdn; do
  for implementation in upstream slskr; do
    for mode in default environment yaml-over-env cli-over-yaml cli-short-over-yaml null-credentials null-jwt-leaves; do
      capture_effective "$target" "$implementation" "$mode"
    done
    capture_validation "$target" "$implementation"
    capture_watch "$target" "$implementation"
  done
  for suffix in default.json environment.json yaml-over-env.json cli-over-yaml.json cli-short-over-yaml.json null-credentials.json null-jwt-leaves.json validation.jsonl watch.json restart.json; do
    if ! cmp --silent "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix"; then
      printf 'web authentication credentials differential failed: %s %s\n' "$target" "$suffix" >&2
      diff -u "$work_dir/$target-upstream-$suffix" "$work_dir/$target-slskr-$suffix" >&2 || true
      exit 1
    fi
  done
done

printf 'web authentication credential and JWT differential passed for frozen slskd and slskdN\n'
