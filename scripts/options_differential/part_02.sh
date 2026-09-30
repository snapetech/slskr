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
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      if [[ "$debug_override" == true ]]; then
        export SLSKD_DEBUG=true
      fi
      slskr_exec serve
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
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_address" ]] && export SLSKD_SLSK_ADDRESS="$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_SLSK_PORT="$environment_port"
      slskr_exec serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_address" ]] && export SLSKD_SLSK_ADDRESS="$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_SLSK_PORT="$environment_port"
      slskr_exec serve --app-dir "$state" \
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

password_digest() {
  "$python_bin" -c 'import hashlib,sys; print(hashlib.pbkdf2_hmac("sha256", sys.argv[1].encode(), b"slskr-fixture-listener-digest-salt-v1", 100_000).hex())' "$1"
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
  local expected_password_digest="$4"
  local log="$5"
  for _ in $(seq 1 600); do
    if "$python_bin" - "$status_file" "$expected_count" "$expected_username" "$expected_password_digest" <<'PY' 2>/dev/null
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
count=int(sys.argv[2])
users=value.get("login_usernames", [])
hashes=value.get("login_password_digest", [])
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
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_username" ]] && export SLSKD_SLSK_USERNAME="$environment_username"
      [[ -n "$environment_password" ]] && export SLSKD_SLSK_PASSWORD="$environment_password"
      slskr_exec serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_username" ]] && export SLSKD_SLSK_USERNAME="$environment_username"
      [[ -n "$environment_password" ]] && export SLSKD_SLSK_PASSWORD="$environment_password"
      slskr_exec serve "${cli_args[@]}"
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
  local hash_a="$(password_digest credential-password-a)"
  local hash_b="$(password_digest credential-password-b)"

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
  printf 'flags:\n  no_connect: false\ndht:\n  enabled: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  description: "%s"\n  picture: "%s"\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
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
  local listen_port="${9:-}"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$append" == true ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$listen_port" ]] && export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$listen_port" ]] && export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$append" == true ]]; then
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_HTTPS_PORT="$https_port"
      export SLSKR_DHT_PORT="$(pick_free_udp_port)"
      export SLSKR_OVERLAY_BIND="127.0.0.1:$(pick_free_port)"
      local -a args=(serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port")
      if [[ -n "$listen_port" ]]; then
        args+=(--slsk-listen-port "$listen_port")
      fi
      slskr_exec "${args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_HTTPS_PORT="$https_port"
      export SLSKR_DHT_PORT="$(pick_free_udp_port)"
      export SLSKR_OVERLAY_BIND="127.0.0.1:$(pick_free_port)"
      local -a args=(serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port")
      if [[ -n "$listen_port" ]]; then
        args+=(--slsk-listen-port "$listen_port")
      fi
      slskr_exec "${args[@]}"
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
    "$http_port" "$https_port" false "$listen_port"
  wait_for_options "$base_url" "$work_dir/$target-no-connect-invalid-$implementation-options.json" "$log"
  write_no_connect_yaml "$state/slskd.yml" false "$server_port" "$listen_port" 127.0.0.1

  if [[ "$target" == slskdn ]]; then
    local observed=false
    for _ in $(seq 1 200); do
      local status
      if curl --silent --show-error --max-time 1 \
        --output "$suite/no-connect-invalid-watch.body" \
        --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
        "$base_url/api/v0/options" \
        >"$suite/no-connect-invalid-watch.meta" 2>/dev/null; then
        status="$(sed -n 's/^status=//p' "$suite/no-connect-invalid-watch.meta")"
      else
        status=""
      fi
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
      "$http_port" "$https_port" false "$listen_port"
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
      "$http_port" "$https_port" true "$listen_port"
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
      "$http_port" "$https_port" true "$listen_port"
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
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      local args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
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
      # The scenario polls the options endpoint while exercising metrics
      # reloads. Keep that test-only polling allowance bounded and isolated
      # from the production default web rate limit.
      export SLSKD_WEB_API_PERMIT_LIMIT=1000
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
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
    local default_metrics_username=slskd
    # Frozen upstream profiles retain the slskd default. The replacement uses
    # the selected reference profile, so native slskR keeps its branded value.
    if [[ "$implementation" == slskr ]] && [[ "$(runtime_profile_for_reference "$target")" == native ]]; then
      default_metrics_username=slskr
    fi
    mkdir -p "$state" "$suite"

    write_metrics_yaml "$state/slskd.yml"
    start_metrics_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none
    wait_for_metrics_option "$base_url" false /metrics false "$default_metrics_username" "$log"
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
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
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
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKR_PERSISTENCE_ENABLED=true
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
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

