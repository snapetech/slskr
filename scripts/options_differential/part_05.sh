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

wait_for_obfuscation_server_connected() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 300); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/server" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin).get("isConnected") is True else 1)' \
        2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'obfuscation outbound differential: daemon exited before login completed\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'obfuscation outbound differential: timed out waiting for server login\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_obfuscation_address_request() {
  local status="$1"
  local log="$2"
  for _ in $(seq 1 300); do
    if "$python_bin" - "$status" <<'PY' 2>/dev/null
import json,sys
value=json.load(open(sys.argv[1], encoding="utf-8"))
raise SystemExit(0 if "fixture-peer" in value.get("peer_address_requests", []) else 1)
PY
    then
      # Allow the direct connection or regular fallback to settle after the
      # fixture has answered the address request.
      sleep 1
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'obfuscation outbound differential: daemon exited before requesting the peer address\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'obfuscation outbound differential: timed out waiting for peer address request\n' >&2
  cat "$status" >&2 || true
  tail -120 "$log" >&2 || true
  exit 1
}

trigger_obfuscation_outbound() {
  local implementation="$1"
  local base_url="$2"
  if [[ "$implementation" == upstream ]]; then
    curl --silent --max-time 30 "$base_url/api/v0/users/fixture-peer/browse" >/dev/null || true
  else
    curl --fail --silent --max-time 30 --request POST \
      "$base_url/api/v0/users/fixture-peer/browse/request" >/dev/null
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
      wait_for_obfuscation_server_connected "$base_url" "$log"
      trigger_obfuscation_outbound "$implementation" "$base_url"
      wait_for_obfuscation_address_request "$fixture_status" "$log"
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
      __released_port_not_sampled__ "$new_port" "$host_ip" "$fixture_status"

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
    # The original port has been released across later reconfigurations; a
    # different process can legitimately reuse it, so do not attribute a raw
    # TCP response there to this daemon.
    capture_listener_stage "$target" "$base_url" "$suite" ip-watched "$host_ip" "$new_port" \
      __released_port_not_sampled__ "$new_port" "$host_ip" "$fixture_status"

    stop_daemon
    start_no_connect_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/$target-listener-$implementation-ip-restart.json" "$log"
    wait_for_direct_user_info_state 127.0.0.1 "$new_port" closed "$log"
    wait_for_direct_user_info_state "$host_ip" "$new_port" open "$log"
    wait_for_advertisement_count "$fixture_status" 7 "$log"
    capture_listener_stage "$target" "$base_url" "$suite" ip-restarted "$host_ip" "$new_port" \
      __released_port_not_sampled__ "$new_port" "$host_ip" "$fixture_status"

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
      if curl --silent --show-error --max-time 1 \
        --output "$suite/listener-invalid-watch.body" \
        --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
        "$base_url/api/v0/options" >"$suite/listener-invalid-watch.meta" 2>/dev/null; then
        status="$(sed -n 's/^status=//p' "$suite/listener-invalid-watch.meta")"
      else
        status=""
      fi
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
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_SLSK_LISTEN_PORT="$soulseek_listen_port"
      [[ -n "$environment_address" ]] && export "$address_environment=$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_HTTP_PORT="$environment_port"
      slskr_exec serve --app-dir "$state" "${command_line[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      unset SLSKD_HTTP_IP_ADDRESS SLSKD_HTTP_ADDRESS SLSKD_HTTP_PORT
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_SLSK_LISTEN_PORT="$soulseek_listen_port"
      [[ -n "$environment_address" ]] && export "$address_environment=$environment_address"
      [[ -n "$environment_port" ]] && export SLSKD_HTTP_PORT="$environment_port"
      slskr_exec serve --app-dir "$state" "${command_line[@]}"
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
default_identity_values = {
    "slskd/0.0.0 (https://github.com/slskd/slskd)",
    "slskd/0.0.0 (https://github.com/snapetech/slskdn)",
    "slskR/0.0.0 (https://github.com/snapetech/slskr)",
    "slskd",
    "slskdN",
    "slskR",
    "slskr",
    "From slskd:",
    "From slskdN:",
    "From slskR:",
}
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
    for key in ("userAgent", "notificationPrefix"):
        if result.get(key) in default_identity_values:
            result[key]="<DEFAULT_PRODUCT_IDENTITY>"
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
        export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")" SLSKD_NO_AUTH=true
        slskr_exec serve --app-dir "$state" \
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
        export SLSKR_CONTROLLER_PROFILE=native SLSKD_NO_AUTH=true
        slskr_exec serve --app-dir "$state" \
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
        export SLSKR_CONTROLLER_PROFILE=native SLSKD_NO_AUTH=true
        slskr_exec serve --app-dir "$state" \
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
        export SLSKR_CONTROLLER_PROFILE=native SLSKD_NO_AUTH=true
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
        slskr_exec serve --app-dir "$state" \
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
        export SLSKR_CONTROLLER_PROFILE=native SLSKD_NO_AUTH=true
        export SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT=4
        slskr_exec serve --app-dir "$state" \
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
        export SLSKR_CONTROLLER_PROFILE=native SLSKD_NO_AUTH=true
        slskr_exec serve --app-dir "$state" \
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
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
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
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      mkdir -p "$repo_root/target/debug/wwwroot"
      printf '<html>foundation</html>' >"$repo_root/target/debug/wwwroot/index.html"
      (
        export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        slskr_exec serve
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
        export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        slskr_exec serve
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
  max_concurrent_streams: 7
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
