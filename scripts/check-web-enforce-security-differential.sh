#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
slskdn_root="${SLSKR_SLSKDN_ROOT:-/tmp/slskr-parity-slskdn-frozen}"
dll="$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
work_dir="${SLSKR_ENFORCE_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-enforce-differential.XXXXXX")}"
keep_artifacts="${SLSKR_ENFORCE_DIFFERENTIAL_KEEP:-0}"
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
  local enforce="$2"
  local hash_from_audio="$3"
  local allow_memory="${4:-false}"
  local metrics_password="${5:-strong-password}"
  mkdir -p "$state"
  printf '%s\n' \
    'flags:' \
    '  no_connect: true' \
    "  hash_from_audio_file_enabled: $hash_from_audio" \
    'remote_configuration: true' \
    'dht:' \
    '  enabled: false' \
    'diagnostics:' \
    "  allow_memory_dump: $allow_memory" \
    'metrics:' \
    '  enabled: true' \
    '  authentication:' \
    '    disabled: false' \
    '    username: slskd' \
    "    password: '$metrics_password'" \
    'web:' \
    "  enforce_security: $enforce" \
    '  allow_remote_no_auth: true' \
    '  https:' \
    '    disabled: true' \
    '  authentication:' \
    '    disabled: true' \
    '    passthrough:' \
    "      allowed_cidrs: '127.0.0.0/8'" \
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
  local environment_enforce="${7:-__unset__}"
  local cli_enforce="${8:-false}"
  if [[ "$implementation" == upstream ]]; then
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_REMOTE_CONFIGURATION=true
      export SLSKD_HTTP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
      export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      if [[ "$environment_enforce" != __unset__ ]]; then
        export SLSKD_ENFORCE_SECURITY="$environment_enforce"
      else
        unset SLSKD_ENFORCE_SECURITY || true
      fi
      args=()
      [[ "$cli_enforce" == true ]] && args+=(--enforce-security)
      exec dotnet "$dll" "${args[@]}"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_COMPATIBILITY_TARGET=slskdn SLSKR_REMOTE_CONFIGURATION=true
      if [[ "$environment_enforce" != __unset__ ]]; then
        export SLSKD_ENFORCE_SECURITY="$environment_enforce"
      else
        unset SLSKD_ENFORCE_SECURITY || true
      fi
      args=()
      [[ "$cli_enforce" == true ]] && args+=(--enforce-security)
      exec "$repo_root/target/debug/slskr" serve --app-dir "$state" \
        --http-address 127.0.0.1 --http-port "$port" \
        --slsk-listen-port "$listen_port" --no-connect "${args[@]}"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_ready() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 600); do
    if curl --noproxy '*' --silent --max-time 1 "$base_url/api/v0/options" >/dev/null; then
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

wait_rejected() {
  local log="$1"
  local rule="$2"
  for _ in $(seq 1 300); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      wait "$daemon_pid" 2>/dev/null || true
      daemon_pid=""
      rg -qi "$rule" "$log" || { tail -120 "$log" >&2; return 1; }
      return
    fi
    sleep 0.05
  done
  tail -120 "$log" >&2 || true
  printf 'daemon did not reject %s\n' "$rule" >&2
  return 1
}

options_snapshot() {
  curl --noproxy '*' --silent --show-error --max-time 10 "$1/api/v0/options"
}

application_snapshot() {
  curl --noproxy '*' --silent --show-error --max-time 10 "$1/api/v0/application"
}

capture_options() {
  local base_url="$1"
  local destination="$2"
  options_snapshot "$base_url" | python3 -c 'import json,sys
value=json.load(sys.stdin)
print(json.dumps({
  "enforceSecurity": value.get("web", {}).get("enforceSecurity"),
  "hashFromAudioFileEnabled": value.get("flags", {}).get("hashFromAudioFileEnabled"),
}, sort_keys=True))' >"$destination"
}

capture_validation_suite() {
  local base_url="$1"
  local implementation="$2"
  python3 - "$base_url" >"$work_dir/$implementation-validation.jsonl" <<'PY'
import http.client, json, sys, urllib.parse
url = urllib.parse.urlsplit(sys.argv[1])
cases = {
    "enforce-null": "web:\n  enforce_security: null\n",
    "enforce-string": "web:\n  enforce_security: 'true'\n",
    "enforce-invalid": "web:\n  enforce_security: 1\n",
    "hash-null": "flags:\n  hash_from_audio_file_enabled: null\n",
    "hash-string": "flags:\n  hash_from_audio_file_enabled: 'true'\n",
    "hash-invalid": "flags:\n  hash_from_audio_file_enabled: 1\n",
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

run_precedence() {
  local implementation="$1"
  local mode="$2"
  local state="$work_dir/$implementation-precedence-$mode-state"
  local log="$work_dir/$implementation-precedence-$mode.log"
  local port https_port listen_port base_url env_enforce=__unset__ cli_enforce=false yaml_enforce=false
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  case "$mode" in
    default) ;;
    environment) yaml_enforce=false; env_enforce=true ;;
    yaml-over-environment) yaml_enforce=false; env_enforce=true ;;
    cli-over-yaml) yaml_enforce=false; cli_enforce=true ;;
  esac
  if [[ "$mode" == environment ]]; then
    mkdir -p "$state"
    printf '%s\n' \
      'flags:' '  no_connect: true' \
      'remote_configuration: true' \
      'dht:' '  enabled: false' \
      'metrics:' '  enabled: false' \
      'web:' '  allow_remote_no_auth: true' '  https:' '    disabled: true' \
      '  authentication:' '    disabled: true' '    passthrough:' "      allowed_cidrs: '127.0.0.0/8'" \
      '  rate_limiting:' '    enabled: false' >"$state/slskd.yml"
  else
    write_config "$state" "$yaml_enforce" false false strong-password
  fi
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log" "$env_enforce" "$cli_enforce"
  wait_ready "$base_url" "$log"
  capture_options "$base_url" "$work_dir/$implementation-precedence-$mode.json"
  if [[ "$mode" == default ]]; then
    capture_validation_suite "$base_url" "$implementation"
  fi
  stop_daemon
}

run_rule() {
  local implementation="$1"
  local rule="$2"
  local enforce="$3"
  local state="$work_dir/$implementation-rule-$rule-$enforce-state"
  local log="$work_dir/$implementation-rule-$rule-$enforce.log"
  local port https_port listen_port base_url
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  if [[ "$rule" == WeakMetricsPassword ]]; then
    write_config "$state" "$enforce" false false ' '
  else
    write_config "$state" "$enforce" true false strong-password
  fi
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  if [[ "$rule" == WeakMetricsPassword ]]; then
    wait_rejected "$log" 'Metrics authentication password'
  else
    wait_rejected "$log" "$rule"
  fi
}

run_watch_enforce() {
  local implementation="$1"
  local state="$work_dir/$implementation-watch-enforce-state"
  local log="$work_dir/$implementation-watch-enforce.log"
  local port https_port listen_port base_url options application
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" false false true strong-password
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  write_config "$state" true false true strong-password
  for _ in $(seq 1 100); do
    options="$(options_snapshot "$base_url")"
    application="$(application_snapshot "$base_url")"
    if python3 - "$options" "$application" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
raise SystemExit(0 if options.get("web", {}).get("enforceSecurity") is True and application.get("pendingRestart") is True else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$options" "$application" >"$work_dir/$implementation-watch-enforce.json" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
print(json.dumps({
  "enforceSecurity": options.get("web", {}).get("enforceSecurity"),
  "allowMemoryDump": options.get("diagnostics", {}).get("allowMemoryDump"),
  "pendingRestart": application.get("pendingRestart"),
}, sort_keys=True))
PY
  stop_daemon
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_rejected "$log" MemoryDumpWithAuthDisabled
}

run_watch_hash() {
  local implementation="$1"
  local state="$work_dir/$implementation-watch-hash-state"
  local log="$work_dir/$implementation-watch-hash.log"
  local port https_port listen_port base_url options application
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  base_url="http://127.0.0.1:$port"
  write_config "$state" false false false strong-password
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_ready "$base_url" "$log"
  write_config "$state" false true false strong-password
  for _ in $(seq 1 100); do
    options="$(options_snapshot "$base_url")"
    application="$(application_snapshot "$base_url")"
    if python3 - "$options" "$application" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
raise SystemExit(0 if options.get("flags", {}).get("hashFromAudioFileEnabled") is True and application.get("pendingRestart") is True else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  python3 - "$options" "$application" >"$work_dir/$implementation-watch-hash.json" <<'PY'
import json, sys
options, application = json.loads(sys.argv[1]), json.loads(sys.argv[2])
print(json.dumps({
  "hashFromAudioFileEnabled": options.get("flags", {}).get("hashFromAudioFileEnabled"),
  "pendingRestart": application.get("pendingRestart"),
}, sort_keys=True))
PY
  stop_daemon
  start_daemon "$implementation" "$state" "$port" "$https_port" "$listen_port" "$log"
  wait_rejected "$log" HashFromAudioFileEnabled
}

[[ -f "$dll" ]] || { printf 'missing frozen slskdN binary: %s\n' "$dll" >&2; exit 1; }
cargo build -p slskr --manifest-path "$repo_root/Cargo.toml" >/dev/null
for implementation in upstream slskr; do
  for mode in default environment yaml-over-environment cli-over-yaml; do
    run_precedence "$implementation" "$mode"
  done
  run_rule "$implementation" WeakMetricsPassword false
  run_rule "$implementation" WeakMetricsPassword true
  run_rule "$implementation" HashFromAudioFileEnabled false
  run_watch_enforce "$implementation"
  run_watch_hash "$implementation"
done
for mode in default environment yaml-over-environment cli-over-yaml; do
  diff -u "$work_dir/upstream-precedence-$mode.json" "$work_dir/slskr-precedence-$mode.json"
done
diff -u "$work_dir/upstream-validation.jsonl" "$work_dir/slskr-validation.jsonl"
diff -u "$work_dir/upstream-watch-enforce.json" "$work_dir/slskr-watch-enforce.json"
diff -u "$work_dir/upstream-watch-hash.json" "$work_dir/slskr-watch-hash.json"
printf 'web enforce-security differential passed against frozen slskdN\n'
