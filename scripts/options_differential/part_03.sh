capture_no_start_exit() {
  local suite="$1"
  local stage="$2"
  local log="$3"
  local base_url="$4"
  for _ in $(seq 1 600); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      break
    fi
    sleep 0.05
  done
  if kill -0 "$daemon_pid" 2>/dev/null; then
    printf 'no-start differential failed: process did not exit\n' >&2
    tail -120 "$log" >&2 || true
    exit 1
  fi
  local status=0
  wait "$daemon_pid" || status="$?"
  daemon_pid=""
  local listener=false
  if curl --silent --fail --max-time 1 "$base_url/api/v0/options" >/dev/null 2>&1; then
    listener=true
  fi
  local message=false
  if grep -Fq "Quitting because 'no-start' option is enabled" "$log"; then
    message=true
  fi
  "$python_bin" - "$status" "$listener" "$message" >"$suite/no-start-$stage.body" <<'PY'
import json,sys
print(json.dumps({"exit":int(sys.argv[1]),"listener":sys.argv[2] == "true","message":sys.argv[3] == "true"},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-start-$stage.meta"
}

run_no_start_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-start-$implementation"
    local suite="$work_dir/$target-no-start-$implementation"
    local log="$work_dir/$target-no-start-$implementation.log"
    mkdir -p "$state" "$suite"

    write_no_start_yaml "$state/slskd.yml"
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_start_option "$base_url" false "$log"
    capture_no_start_stage "$base_url" "$suite" default
    stop_daemon

    write_no_start_yaml "$state/slskd.yml" false
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false true
    wait_for_no_start_option "$base_url" false "$log"
    capture_no_start_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_no_start_yaml "$state/slskd.yml"
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false true
    capture_no_start_exit "$suite" environment-exit "$log" "$base_url"

    write_no_start_yaml "$state/slskd.yml" false
    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true true
    capture_no_start_exit "$suite" cli-over-yaml-exit "$log" "$base_url"

    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    wait_for_no_start_option "$base_url" false "$log"
    capture_no_start_stage "$base_url" "$suite" lifecycle-startup
    write_no_start_yaml "$state/slskd.yml" true
    wait_for_no_start_option "$base_url" true "$log"
    capture_no_start_stage "$base_url" "$suite" lifecycle-watched
    capture_request "$suite" no-start-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_start: null\n')"
    capture_request "$suite" no-start-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_start: nope\n')"
    capture_request "$suite" no-start-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_start: [true]\n')"
    stop_daemon

    start_no_start_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false true
    capture_no_start_exit "$suite" lifecycle-restarted "$log" "$base_url"
  done
  local upstream_normalized="$work_dir/$target-no-start-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-start-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-start-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-start-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-start differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-start differential passed\n' "$target"
}

write_no_logo_yaml() {
  local path="$1"
  local no_logo="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n' >"$temporary"
  if [[ "$no_logo" != __unset__ ]]; then
    printf '  no_logo: %s\n' "$no_logo" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_no_logo_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_no_logo="$9"
  local command_line_no_logo="${10}"
  (
    unset SLSKD_NO_LOGO
    [[ "$environment_no_logo" != __unset__ ]] && export SLSKD_NO_LOGO="$environment_no_logo"
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
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
    fi
    [[ "$command_line_no_logo" == true ]] && args+=(--no-logo)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_no_logo_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["noLogo"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'no-logo differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'no-logo differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_no_logo_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local target="$4"
  local log="$5"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" "$target" "$log" >"$suite/no-logo-$stage.body" <<'PY'
import http.client,json,pathlib,sys,urllib.parse
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
log=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
target=sys.argv[2]
slskd_marker="This program is free software: you can redistribute it and/or modify"
slskdn_marker="GNU AFFERO GENERAL PUBLIC LICENSE"
native_marker="native Soulseek client, daemon, and Web UI"
has_native_logo=native_marker in log
has_slskd_logo=slskd_marker in log
has_slskdn_logo=slskdn_marker in log and not has_native_logo
print(json.dumps({
    "current":options["flags"]["noLogo"],
    "startup":startup["flags"]["noLogo"],
    "pendingRestart":application["pendingRestart"],
    # The frozen controllers and native slskR intentionally use different
    # product identities. Compare startup-logo semantics, not branding text.
    "banner":has_slskd_logo or has_slskdn_logo or has_native_logo,
    "otherBanner":has_slskdn_logo if target == "slskd" else has_slskd_logo,
    "version":"0.0.0 (0.0.0)" in log or "slskr 0.0.0" in log,
    "development":"DEVELOPMENT" in log,
    "website":any(marker in log for marker in ("https://slskd.org", "https://github.com/snapetech/slskr")),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-logo-$stage.meta"
}

run_no_logo_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-logo-$implementation"
    local suite="$work_dir/$target-no-logo-$implementation"
    local log
    mkdir -p "$state" "$suite"

    write_no_logo_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-logo-$implementation-default.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_logo_option "$base_url" false "$log"
    capture_no_logo_stage "$base_url" "$suite" default "$target" "$log"
    stop_daemon

    write_no_logo_yaml "$state/slskd.yml" true
    log="$work_dir/$target-no-logo-$implementation-yaml.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" yaml-over-environment "$target" "$log"
    stop_daemon

    write_no_logo_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-logo-$implementation-environment.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" environment "$target" "$log"
    stop_daemon

    write_no_logo_yaml "$state/slskd.yml" false
    log="$work_dir/$target-no-logo-$implementation-cli.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" cli-over-yaml "$target" "$log"
    stop_daemon

    log="$work_dir/$target-no-logo-$implementation-lifecycle.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_logo_option "$base_url" false "$log"
    capture_no_logo_stage "$base_url" "$suite" lifecycle-startup "$target" "$log"
    write_no_logo_yaml "$state/slskd.yml" true
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" lifecycle-watched "$target" "$log"
    capture_request "$suite" no-logo-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_logo: null\n')"
    capture_request "$suite" no-logo-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_logo: nope\n')"
    capture_request "$suite" no-logo-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_logo: [true]\n')"
    stop_daemon

    log="$work_dir/$target-no-logo-$implementation-restarted.log"
    start_no_logo_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_logo_option "$base_url" true "$log"
    capture_no_logo_stage "$base_url" "$suite" lifecycle-restarted "$target" "$log"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-no-logo-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-logo-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-logo-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-logo-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-logo differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-logo differential passed\n' "$target"
}

write_no_version_check_yaml() {
  local path="$1"
  local disabled="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n' >"$temporary"
  if [[ "$disabled" != __unset__ ]]; then
    printf '  no_version_check: %s\n' "$disabled" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_no_version_check_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8" environment_disabled="$9"
  local command_line_disabled="${10}"
  (
    unset SLSKD_NO_VERSION_CHECK
    [[ "$environment_disabled" != __unset__ ]] && export SLSKD_NO_VERSION_CHECK="$environment_disabled"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      mkdir -p "$root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
      export SLSKD_CONTENT_PATH=wwwroot
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo)
    fi
    [[ "$command_line_disabled" == true ]] && args+=(--no-version-check)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_no_version_check_option() {
  local base_url="$1" expected="$2" log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["noVersionCheck"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null; then return; fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
    sleep 0.05
  done
  printf 'no-version-check differential timed out waiting for %s\n' "$expected" >&2
  exit 1
}

capture_no_version_check_stage() {
  local base_url="$1" suite="$2" stage="$3" target="$4" log="$5" expect_initial="$6"
  "$python_bin" - "$base_url" "$target" "$log" "$expect_initial" >"$suite/no-version-check-$stage.body" <<'PY'
import http.client,json,pathlib,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    return json.loads(body)
options=get("/api/v0/options"); startup=get("/api/v0/options/startup")
application=get("/api/v0/application"); version=get("/api/v0/application/version/latest")
log=pathlib.Path(sys.argv[3]).read_text(encoding="utf-8")
check_log="skip" if "Skipping version check for Development build" in log else "check" if "Checking GitHub Releases for latest version" in log else "none"
result={"current":options["flags"]["noVersionCheck"],"startup":startup["flags"]["noVersionCheck"],"pendingRestart":application["pendingRestart"],"checkLog":check_log,"full":version["full"],"currentVersion":version["current"],"isCanary":version["isCanary"],"isDevelopment":version["isDevelopment"]}
if sys.argv[4] == "true":
    result["versionKeys"]=sorted(version)
    result["latest"]=version.get("latest")
    result["latestTag"]=version.get("latestTag")
    result["latestUrl"]=version.get("latestUrl")
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/no-version-check-$stage.meta"
}

run_no_version_check_scenario() {
  local target="$1" root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-no-version-check-$implementation"
    local suite="$work_dir/$target-no-version-check-$implementation"
    local log
    mkdir -p "$state/wwwroot" "$suite"

    write_no_version_check_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-version-check-$implementation-default.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_version_check_option "$base_url" false "$log"
    capture_no_version_check_stage "$base_url" "$suite" default "$target" "$log" false
    stop_daemon

    write_no_version_check_yaml "$state/slskd.yml" true
    log="$work_dir/$target-no-version-check-$implementation-yaml.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" yaml-over-environment "$target" "$log" true
    stop_daemon

    write_no_version_check_yaml "$state/slskd.yml"
    log="$work_dir/$target-no-version-check-$implementation-environment.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" environment "$target" "$log" true
    stop_daemon

    write_no_version_check_yaml "$state/slskd.yml" false
    log="$work_dir/$target-no-version-check-$implementation-cli.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" cli-over-yaml "$target" "$log" true
    stop_daemon

    log="$work_dir/$target-no-version-check-$implementation-lifecycle.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_version_check_option "$base_url" false "$log"
    capture_no_version_check_stage "$base_url" "$suite" lifecycle-startup "$target" "$log" false
    write_no_version_check_yaml "$state/slskd.yml" true
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" lifecycle-watched "$target" "$log" false
    capture_request "$suite" no-version-check-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_version_check: null\n')"
    capture_request "$suite" no-version-check-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_version_check: nope\n')"
    capture_request "$suite" no-version-check-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_version_check: [true]\n')"
    stop_daemon

    log="$work_dir/$target-no-version-check-$implementation-restarted.log"
    start_no_version_check_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_no_version_check_option "$base_url" true "$log"
    capture_no_version_check_stage "$base_url" "$suite" lifecycle-restarted "$target" "$log" true
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-no-version-check-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-version-check-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-no-version-check-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-no-version-check-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'no-version-check differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s no-version-check differential passed\n' "$target"
}

write_experimental_yaml() {
  local path="$1"
  local enabled="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n  no_version_check: true\n' >"$temporary"
  if [[ "$enabled" != __unset__ ]]; then
    printf '  experimental: %s\n' "$enabled" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_experimental_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8" environment_enabled="$9"
  local command_line_enabled="${10}"
  (
    unset SLSKD_EXPERIMENTAL
    [[ "$environment_enabled" != __unset__ ]] && export SLSKD_EXPERIMENTAL="$environment_enabled"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo)
    fi
    [[ "$command_line_enabled" == true ]] && args+=(--experimental)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_experimental_option() {
  local base_url="$1" expected="$2" log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["experimental"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null; then return; fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
    sleep 0.05
  done
  printf 'experimental differential timed out waiting for %s\n' "$expected" >&2
  exit 1
}

capture_experimental_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/experimental-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    return response.status,json.loads(body)
_,options=get("/api/v0/options")
_,startup=get("/api/v0/options/startup")
_,application=get("/api/v0/application")
server_status,server=get("/api/v0/server")
result={
    "current":options["flags"]["experimental"],
    "startup":startup["flags"]["experimental"],
    "pendingRestart":application["pendingRestart"],
    "serverStatus":server_status,
    "isConnected":server["isConnected"],
}
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/experimental-$stage.meta"
}

run_experimental_scenario() {
  local target="$1" root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-experimental-$implementation"
    local suite="$work_dir/$target-experimental-$implementation"
    local log
    mkdir -p "$state" "$suite"

    write_experimental_yaml "$state/slskd.yml"
    log="$work_dir/$target-experimental-$implementation-default.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_experimental_option "$base_url" false "$log"
    capture_experimental_stage "$base_url" "$suite" default
    stop_daemon

    write_experimental_yaml "$state/slskd.yml" true
    log="$work_dir/$target-experimental-$implementation-yaml.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_experimental_yaml "$state/slskd.yml"
    log="$work_dir/$target-experimental-$implementation-environment.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" environment
    stop_daemon

    write_experimental_yaml "$state/slskd.yml" false
    log="$work_dir/$target-experimental-$implementation-cli.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon

    log="$work_dir/$target-experimental-$implementation-lifecycle.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_experimental_option "$base_url" false "$log"
    capture_experimental_stage "$base_url" "$suite" lifecycle-startup
    write_experimental_yaml "$state/slskd.yml" true
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" lifecycle-watched
    capture_request "$suite" experimental-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  experimental: null\n')"
    capture_request "$suite" experimental-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  experimental: nope\n')"
    capture_request "$suite" experimental-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  experimental: [true]\n')"
    stop_daemon

    log="$work_dir/$target-experimental-$implementation-restarted.log"
    start_experimental_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_experimental_option "$base_url" true "$log"
    capture_experimental_stage "$base_url" "$suite" lifecycle-restarted
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-experimental-upstream.normalized"
  local slskr_normalized="$work_dir/$target-experimental-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-experimental-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-experimental-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'experimental differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s experimental differential passed\n' "$target"
}

write_case_sensitive_regex_yaml() {
  local path="$1"
  local enabled="${2:-__unset__}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: true\n  no_version_check: true\n' >"$temporary"
  if [[ "$enabled" != __unset__ ]]; then
    printf '  case_sensitive_reg_ex: %s\n' "$enabled" >>"$temporary"
  fi
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

start_case_sensitive_regex_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8" environment_enabled="$9"
  local command_line_enabled="${10}"
  (
    unset SLSKD_CASE_SENSITIVE_REGEX
    [[ "$environment_enabled" != __unset__ ]] && export SLSKD_CASE_SENSITIVE_REGEX="$environment_enabled"
    local args=()
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      mkdir -p "$root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
      export SLSKD_CONTENT_PATH=wwwroot
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      args=(dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll")
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo)
    fi
    [[ "$command_line_enabled" == true ]] && args+=(--case-sensitive-regex)
    exec "${args[@]}" >"$log" 2>&1
  ) &
  daemon_pid="$!"
}

wait_for_case_sensitive_regex_option() {
  local base_url="$1" expected="$2" log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["flags"]["caseSensitiveRegEx"] == (sys.argv[1] == "true") else 1)' "$expected" 2>/dev/null; then return; fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
    sleep 0.05
  done
  printf 'case-sensitive-regex differential timed out waiting for %s\n' "$expected" >&2
  exit 1
}

capture_case_sensitive_regex_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/case-sensitive-regex-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
application=get("/api/v0/application")
result={
    "current":options["flags"]["caseSensitiveRegEx"],
    "startup":startup["flags"]["caseSensitiveRegEx"],
    "pendingRestart":application["pendingRestart"],
}
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/case-sensitive-regex-$stage.meta"
}

run_case_sensitive_regex_scenario() {
  local target="$1" root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-case-sensitive-regex-$implementation"
    local suite="$work_dir/$target-case-sensitive-regex-$implementation"
    local log
    mkdir -p "$state/wwwroot" "$suite"

    write_case_sensitive_regex_yaml "$state/slskd.yml"
    log="$work_dir/$target-case-sensitive-regex-$implementation-default.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_case_sensitive_regex_option "$base_url" false "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" default
    stop_daemon

    write_case_sensitive_regex_yaml "$state/slskd.yml" true
    log="$work_dir/$target-case-sensitive-regex-$implementation-yaml.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false false
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_case_sensitive_regex_yaml "$state/slskd.yml"
    log="$work_dir/$target-case-sensitive-regex-$implementation-environment.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" true false
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" environment
    stop_daemon

    write_case_sensitive_regex_yaml "$state/slskd.yml" false
    log="$work_dir/$target-case-sensitive-regex-$implementation-cli.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false true
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon

    log="$work_dir/$target-case-sensitive-regex-$implementation-lifecycle.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_case_sensitive_regex_option "$base_url" false "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" lifecycle-startup
    write_case_sensitive_regex_yaml "$state/slskd.yml" true
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" lifecycle-watched
    capture_request "$suite" case-sensitive-regex-validation-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  case_sensitive_reg_ex: null\n')"
    capture_request "$suite" case-sensitive-regex-validation-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  case_sensitive_reg_ex: nope\n')"
    capture_request "$suite" case-sensitive-regex-validation-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  case_sensitive_reg_ex: [true]\n')"
    capture_request "$suite" regex-validation-lookaround POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'shares:\n  filters:\n    - \'(?<=/)secret(?=\\.flac$)\'\n')"
    capture_request "$suite" regex-validation-numbered-backreference POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(secret)\\1$\'\n')"
    local named_backreference_yaml
    if [[ "$target" == slskd ]]; then
      named_backreference_yaml=$'transfers:\n  groups:\n    blacklisted:\n      patterns:\n        - \'^(?<stem>case)\\k<stem>peer$\'\n'
    else
      named_backreference_yaml=$'groups:\n  blacklisted:\n    patterns:\n      - \'^(?<stem>case)\\k<stem>peer$\'\n'
    fi
    capture_request "$suite" regex-validation-named-backreference POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$named_backreference_yaml")"
    capture_request "$suite" regex-validation-atomic-group POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(?>a|ab)c$\'\n')"
    capture_request "$suite" regex-validation-conditional POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(a)?b(?(1)c|d)$\'\n')"
    capture_request "$suite" regex-validation-assertion-conditional POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^(?(?=secret)secret|public)$\'\n')"
    capture_request "$suite" regex-validation-explicit-capture POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(?n)^(secret)(?<stem>secret)\\k<stem>$\'\n')"
    capture_request "$suite" regex-validation-control-escape POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^\\cA$\'\n')"
    capture_request "$suite" regex-validation-z-anchor POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'secret\\Z\'\n')"
    capture_request "$suite" regex-validation-unicode-category POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^\\p{L}+$\'\n')"
    capture_request "$suite" regex-validation-basic-latin-block POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'^\\p{IsBasicLatin}+$\'\n')"
    capture_request "$suite" regex-validation-invalid-braced-hex POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'\\x{41}\'\n')"
    capture_request "$suite" regex-validation-invalid-possessive POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'secret++\'\n')"
    capture_request "$suite" regex-validation-invalid-keep-out POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'secret\\Kvalue\'\n')"
    capture_request "$suite" regex-validation-invalid-recursion POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(?R)\'\n')"
    capture_request "$suite" regex-validation-invalid-backtracking-verb POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(*FAIL)\'\n')"
    capture_request "$suite" regex-validation-invalid-absent-expression POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'(?~secret)\'\n')"
    capture_request "$suite" regex-validation-invalid-property-alias POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'\\p{Letter}\'\n')"
    capture_request "$suite" regex-validation-invalid-horizontal-space POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'filters:\n  search:\n    request:\n      - \'\\h\'\n')"
    stop_daemon

    log="$work_dir/$target-case-sensitive-regex-$implementation-restarted.log"
    start_case_sensitive_regex_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" __unset__ false
    wait_for_case_sensitive_regex_option "$base_url" true "$log"
    capture_case_sensitive_regex_stage "$base_url" "$suite" lifecycle-restarted
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-case-sensitive-regex-upstream.normalized"
  local slskr_normalized="$work_dir/$target-case-sensitive-regex-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-case-sensitive-regex-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-case-sensitive-regex-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'case-sensitive-regex differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s case-sensitive-regex differential passed\n' "$target"
}

write_regex_runtime_yaml() {
  local path="$1" share="$2" case_sensitive="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_version_check: true\n  force_share_scan: true\n  case_sensitive_reg_ex: %s\nshares:\n  directories:\n    - "[Probe]%s"\n  filters:\n    - '\''(?<=/)secret(?=\\.flac$)'\''\ndht:\n  enabled: false\n' \
    "$case_sensitive" "$share" >"$temporary"
  mv "$temporary" "$path"
}

start_regex_runtime_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8"
  local overlay_port
  overlay_port="$(pick_free_port)"
  (
    if [[ "$implementation" == upstream ]]; then
      mkdir -p "$root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      export SLSKD_CONTENT_PATH=wwwroot
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_HTTPS_PORT="$https_port"
      export SLSKR_DHT_PORT="$(pick_free_udp_port)"
      export SLSKR_OVERLAY_BIND="127.0.0.1:$overlay_port"
      slskr_exec serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" \
        --slsk-listen-port "$listen_port" --no-logo
    fi
  ) >"$log" 2>&1 &
  daemon_pid="$!"
}

capture_regex_runtime_stage() {
  local target="$1" base_url="$2" suite="$3" stage="$4"
  "$python_bin" - "$target" "$base_url" >"$suite/regex-runtime-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
target=sys.argv[1]; url=urllib.parse.urlsplit(sys.argv[2])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=10)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200:
        raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
options=get("/api/v0/options")
startup=get("/api/v0/options/startup")
shares=get("/api/v0/shares")
probe=next(share for group in shares.values() for share in group if share.get("alias") == "Probe")
result={
    "current":options["flags"]["caseSensitiveRegEx"],
    "startup":startup["flags"]["caseSensitiveRegEx"],
    "filters":options["shares"]["filters"],
    "files":probe["files"],
}
if target == "slskdn":
    result["libraryMatches"]=len(get("/api/v0/library/items?query=SECRET")["items"])
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/regex-runtime-$stage.meta"
}

run_regex_runtime_scenario() {
  local target="$1" root="$2"
  local share="$work_dir/$target-regex-runtime-share"
  mkdir -p "$share"
  printf 'secret\n' >"$share/SECRET.flac"
  printf 'public\n' >"$share/PUBLIC.flac"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-regex-runtime-$implementation"
    local suite="$work_dir/$target-regex-runtime-$implementation"
    local log="$work_dir/$target-regex-runtime-$implementation.log"
    mkdir -p "$state/wwwroot" "$suite"

    write_regex_runtime_yaml "$state/slskd.yml" "$share" false
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-runtime-insensitive-options.json" "$log"
    wait_for_share_files "$base_url" Probe 1 "$log"
    capture_regex_runtime_stage "$target" "$base_url" "$suite" insensitive
    stop_daemon

    write_regex_runtime_yaml "$state/slskd.yml" "$share" true
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-runtime-sensitive-options.json" "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_regex_runtime_stage "$target" "$base_url" "$suite" sensitive
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-regex-runtime-upstream.normalized"
  local slskr_normalized="$work_dir/$target-regex-runtime-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-regex-runtime-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-regex-runtime-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'regex runtime differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s regex runtime differential passed\n' "$target"
}

write_regex_protocol_yaml() {
  local path="$1" target="$2" share="$3" case_sensitive="$4" server_port="$5"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: false\n  no_version_check: true\n  force_share_scan: true\n  case_sensitive_reg_ex: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: regex-probe\n  password: regex-probe\nshares:\n  directories:\n    - "[Probe]%s"\nfilters:\n  search:\n    request:\n      - '\''(?n)^(?(?=secret)(secret)(?<stem>secret)\\k<stem>|blocked\\Z)$'\''\n      - '\''^(?<named>a)(b)\\1\\2$'\''\n' \
    "$case_sensitive" "$server_port" "$share" >"$temporary"
  printf 'transfers:\n  groups:\n    blacklisted:\n      patterns:\n        - '\''^(?<stem>case)\\k<stem>peer$'\''\n' >>"$temporary"
  printf 'dht:\n  enabled: false\n' >>"$temporary"
  mv "$temporary" "$path"
}

write_regex_invariant_protocol_yaml() {
  local path="$1" share="$2" server_port="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: false\n  no_version_check: true\n  force_share_scan: true\n  case_sensitive_reg_ex: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: regex-probe\n  password: regex-probe\nshares:\n  directories:\n    - "[Probe]%s"\nfilters:\n  search:\n    request:\n      - '\''^s$'\''\n      - '\''^σ$'\''\n      - '\''^k$'\''\n      - '\''^ß$'\''\ndht:\n  enabled: false\n' \
    "$server_port" "$share" >"$temporary"
  mv "$temporary" "$path"
}

capture_regex_protocol_stage() {
  local target="$1" port="$2" suite="$3" stage="$4" fixture_status="$5"
  "$python_bin" - "$target" "$port" "$fixture_status" >"$suite/regex-protocol-$stage.body" <<'PY'
import json,pathlib,socket,struct,sys,time
target=sys.argv[1]
port=int(sys.argv[2])
status_path=pathlib.Path(sys.argv[3])

def string(value):
    data=value.encode("utf-8")
    return struct.pack("<I",len(data))+data

def frame_init(username):
    payload=string(username)+string("P")+struct.pack("<I",0)
    return struct.pack("<I",len(payload)+1)+b"\x01"+payload

def frame_search(query, token):
    payload=struct.pack("<I",token)+string(query)
    return struct.pack("<II",len(payload)+4,8)+payload

def recv_exact(sock, length):
    value=b""
    while len(value)<length:
        chunk=sock.recv(length-len(value))
        if not chunk:
            return None
        value+=chunk
    return value

def probe(username, query, token):
    deadline=time.monotonic()+10
    while True:
        try:
            sock=socket.create_connection(("127.0.0.1",port),timeout=0.5)
            break
        except OSError:
            if time.monotonic()>=deadline:
                return "connect-error"
            time.sleep(0.05)
    try:
        sock.settimeout(1.5)
        sock.sendall(frame_init(username)+frame_search(query,token))
        header=recv_exact(sock,4)
        if header is None:
            return False
        length=struct.unpack("<I",header)[0]
        body=recv_exact(sock,length)
        return body is not None and length>=4 and struct.unpack("<I",body[:4])[0]==9
    except (ConnectionError,TimeoutError,socket.timeout,OSError):
        return False
    finally:
        sock.close()

same_connection={
    "allowed":probe("AllowedPeer","SECRETSECRETSECRET",101),
    "filtered":probe("AllowedPeer","secretsecretsecret",102),
    "caseVariantBlacklist":probe("CaseCasePeer","SECRETSECRETSECRET",103),
    "exactBlacklist":probe("casecasepeer","SECRETSECRETSECRET",104),
    "dollarFinalNewlineFiltered":probe("AllowedPeer","secretsecretsecret\n",105),
    "zSingleNewlineFiltered":probe("AllowedPeer","blocked\n",106),
    "zMultipleNewlinesAllowed":probe("AllowedPeer","blocked\n\n",107),
    "mixedNumberingFiltered":probe("AllowedPeer","abba",108),
    "encounterOrderAllowed":probe("AllowedPeer","abab",109),
}

expected_requests=["AllowedPeer","CaseCasePeer","casecasepeer"] if target=="slskd" else ["AllowedPeer"]
deadline=time.monotonic()+20
status={}
while time.monotonic()<deadline:
    try:
        status=json.loads(status_path.read_text(encoding="utf-8"))
    except (OSError,json.JSONDecodeError):
        status={}
    requests=sorted(set(status.get("peer_address_requests",[])))
    if requests==expected_requests:
        break
    time.sleep(0.05)
value={
    "sameConnectionResponses":same_connection,
    "peerAddressRequests":sorted(set(status.get("peer_address_requests",[]))),
    "outboundSearchResponseTokens":sorted(set(status.get("peer_search_response_tokens",[]))),
    "target":target,
}
print(json.dumps(value,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/regex-protocol-$stage.meta"
}

capture_regex_invariant_protocol_stage() {
  local port="$1" suite="$2"
  "$python_bin" - "$port" >"$suite/regex-protocol-invariant.body" <<'PY'
import json,socket,struct,sys,time
port=int(sys.argv[1])
def string(value):
    data=value.encode("utf-8")
    return struct.pack("<I",len(data))+data
def frame_init(username):
    payload=string(username)+string("P")+struct.pack("<I",0)
    return struct.pack("<I",len(payload)+1)+b"\x01"+payload
def frame_search(query,token):
    payload=struct.pack("<I",token)+string(query)
    return struct.pack("<II",len(payload)+4,8)+payload
def recv_exact(sock,length):
    value=b""
    while len(value)<length:
        chunk=sock.recv(length-len(value))
        if not chunk: return None
        value+=chunk
    return value
def probe(query,token):
    deadline=time.monotonic()+10
    while True:
        try:
            sock=socket.create_connection(("127.0.0.1",port),timeout=0.5)
            break
        except OSError:
            if time.monotonic()>=deadline: return "connect-error"
            time.sleep(0.05)
    try:
        sock.settimeout(1.5)
        sock.sendall(frame_init("AllowedPeer")+frame_search(query,token))
        header=recv_exact(sock,4)
        if header is None: return False
        length=struct.unpack("<I",header)[0]
        body=recv_exact(sock,length)
        return body is not None and length>=4 and struct.unpack("<I",body[:4])[0]==9
    except (ConnectionError,TimeoutError,socket.timeout,OSError):
        return False
    finally:
        sock.close()
print(json.dumps({
    "longSAllowed":probe("ſ",201),
    "finalSigmaAllowed":probe("ς",202),
    "kelvinFiltered":probe("K",203),
    "capitalSharpSFiltered":probe("ẞ",204),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/regex-protocol-invariant.meta"
}

run_regex_protocol_scenario() {
  local target="$1" root="$2"
  local share="$work_dir/$target-regex-protocol-share"
  mkdir -p "$share"
  printf 'secret\n' >"$share/SECRET.flac"
  printf 'blocked\n' >"$share/BLOCKED.flac"
  printf 'abba\n' >"$share/ABBA.flac"
  printf 'abab\n' >"$share/ABAB.flac"
  printf 'long-s\n' >"$share/ſ.flac"
  printf 'final-sigma\n' >"$share/ς.flac"
  printf 'kelvin\n' >"$share/K.flac"
  printf 'sharp-s\n' >"$share/ẞ.flac"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)" server_port="$(pick_free_port)"
    local peer_regular_port="$(pick_free_port)" peer_obfuscated_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-regex-protocol-$implementation"
    local suite="$work_dir/$target-regex-protocol-$implementation"
    local log="$work_dir/$target-regex-protocol-$implementation.log"
    local fixture_status="$work_dir/$target-regex-protocol-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-regex-protocol-$implementation-fixture.log"
    mkdir -p "$state/wwwroot" "$suite"

    start_soulseek_peer_fixture "$server_port" "$fixture_status" "$fixture_log" \
      "$peer_regular_port" "$peer_obfuscated_port" regular-only
    write_regex_protocol_yaml "$state/slskd.yml" "$target" "$share" true "$server_port"
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-protocol-options.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_share_files "$base_url" Probe 8 "$log"
    capture_regex_protocol_stage "$target" "$listen_port" "$suite" sensitive "$fixture_status"
    stop_daemon

    write_regex_invariant_protocol_yaml "$state/slskd.yml" "$share" "$server_port"
    start_regex_runtime_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$state/regex-protocol-invariant-options.json" "$log"
    wait_for_share_files "$base_url" Probe 8 "$log"
    capture_regex_invariant_protocol_stage "$listen_port" "$suite"
    stop_daemon
    stop_soulseek_fixture
  done
  local upstream_normalized="$work_dir/$target-regex-protocol-upstream.normalized"
  local slskr_normalized="$work_dir/$target-regex-protocol-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-regex-protocol-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-regex-protocol-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then printf 'regex protocol differential failed for %s\n' "$target" >&2; exit 1; fi
  printf '%s regex protocol differential passed\n' "$target"
}

write_share_scan_flags_yaml() {
  local path="$1"
  local share_path="$2"
  local no_share_scan="${3:-false}"
  local force_share_scan="${4:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\n  no_share_scan: %s\n  force_share_scan: %s\nshares:\n  directories:\n    - "[Probe]%s"\ndht:\n  enabled: false\n' \
    "$no_share_scan" "$force_share_scan" "$share_path" >"$temporary"
  mv "$temporary" "$path"
}

start_share_scan_flags_daemon() {
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
    unset SLSKD_NO_SHARE_SCAN SLSKD_FORCE_SHARE_SCAN
    if [[ "$environment_mode" == false ]]; then
      export SLSKD_NO_SHARE_SCAN=false SLSKD_FORCE_SHARE_SCAN=false
    fi
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
    [[ "$command_line_mode" == force ]] && args+=(--force-share-scan)
    if [[ "$append" == true ]]; then exec "${args[@]}" >>"$log" 2>&1; else exec "${args[@]}" >"$log" 2>&1; fi
  ) &
  daemon_pid="$!"
}

wait_for_share_scan_flags() {
  local base_url="$1"
  local expected_no="$2"
  local expected_force="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; f=json.load(sys.stdin)["flags"]; raise SystemExit(0 if f["noShareScan"] == (sys.argv[1] == "true") and f["forceShareScan"] == (sys.argv[2] == "true") else 1)' \
        "$expected_no" "$expected_force" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'share scan flags differential failed: daemon exited while waiting for %s/%s\n' "$expected_no" "$expected_force" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'share scan flags differential failed: timed out waiting for %s/%s\n' "$expected_no" "$expected_force" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_share_scan_flags_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/share-scan-flags-$stage.body" <<'PY'
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
shares=get("/api/v0/shares")
probe=next(share for host in shares.values() for share in host if share.get("alias") == "Probe")
selected=lambda flags:{"noShareScan":flags["noShareScan"],"forceShareScan":flags["forceShareScan"]}
share_state=application["shares"]
print(json.dumps({
    "current":selected(options["flags"]),"startup":selected(startup["flags"]),
    "pendingRestart":application["pendingRestart"],
    "share":{"hasFiles":"files" in probe,"files":probe.get("files"),"hasDirectories":"directories" in probe,"directories":probe.get("directories")},
    "state":dict(
        {key:share_state[key] for key in ("scanPending","scanning","ready","faulted","cancelled","scanProgress","directories","files")},
        hostsPresent="hosts" in share_state,
        hostCount=len(share_state.get("hosts") or []),
    ),
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/share-scan-flags-$stage.meta"
}

run_share_scan_flags_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-share-scan-flags-$implementation"
    local share_path="$state/share"
    local suite="$work_dir/$target-share-scan-flags-$implementation"
    local log="$work_dir/$target-share-scan-flags-$implementation.log"
    mkdir -p "$state" "$share_path" "$suite"
    printf 'first\n' >"$share_path/first.txt"

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" false false
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none
    wait_for_share_scan_flags "$base_url" false false "$log"
    wait_for_share_files "$base_url" Probe 1 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" initial-scan
    stop_daemon

    printf 'second\n' >"$share_path/second.txt"
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" false false "$log"
    wait_for_share_files "$base_url" Probe 1 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" cached-startup
    stop_daemon

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" false true
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" false none true
    wait_for_share_scan_flags "$base_url" false true "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" yaml-force-over-environment
    stop_daemon

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" true true
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" true true "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" no-scan-wins-conflict
    stop_daemon

    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" false false
    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none force true
    wait_for_share_scan_flags "$base_url" false true "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" cli-force-over-yaml
    stop_daemon

    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" false false "$log"
    wait_for_share_files "$base_url" Probe 2 "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" lifecycle-startup
    write_share_scan_flags_yaml "$state/slskd.yml" "$share_path" true true
    wait_for_share_scan_flags "$base_url" true true "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" lifecycle-watched
    stop_daemon

    start_share_scan_flags_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port" none none true
    wait_for_share_scan_flags "$base_url" true true "$log"
    capture_share_scan_flags_stage "$base_url" "$suite" lifecycle-restarted
    capture_request "$suite" share-scan-flags-validation-parent-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags: null\n')"
    capture_request "$suite" share-scan-flags-validation-parent-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags: []\n')"
    capture_request "$suite" share-scan-flags-validation-no-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_share_scan: null\n')"
    capture_request "$suite" share-scan-flags-validation-no-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_share_scan: nope\n')"
    capture_request "$suite" share-scan-flags-validation-no-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  no_share_scan: [true]\n')"
    capture_request "$suite" share-scan-flags-validation-force-null POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  force_share_scan: null\n')"
    capture_request "$suite" share-scan-flags-validation-force-text POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  force_share_scan: nope\n')"
    capture_request "$suite" share-scan-flags-validation-force-array POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'flags:\n  force_share_scan: [true]\n')"
    stop_daemon
  done
  local upstream_normalized="$work_dir/$target-share-scan-flags-upstream.normalized"
  local slskr_normalized="$work_dir/$target-share-scan-flags-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-share-scan-flags-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-share-scan-flags-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'share scan flags differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s share scan flags differential passed\n' "$target"
}

write_instance_yaml() {
  local path="$1"
  local instance_yaml="$2"
  local temporary="$path.tmp"
  printf 'instance_name: %s\nremote_configuration: true\nflags:\n  no_connect: true\ndht:\n  enabled: false\n' \
    "$instance_yaml" >"$temporary"
  mv "$temporary" "$path"
}

start_instance_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local listen_port="$8"
  local environment_name="$9"
  local command_line_name="${10}"
  local append="${11:-false}"
  local redirection='>'
  [[ "$append" == true ]] && redirection='>>'

  (
    unset SLSKD_INSTANCE_NAME SLSKR_INSTANCE_NAME
    if [[ "$environment_name" != __unset__ ]]; then
      export SLSKD_INSTANCE_NAME="$environment_name"
    fi
    if [[ "$implementation" == upstream ]]; then
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      local args=(dotnet "$dll")
      if [[ "$command_line_name" != __unset__ ]]; then
        args+=(-i "$command_line_name")
      fi
      if [[ "$redirection" == '>>' ]]; then
        exec "${args[@]}" >>"$log" 2>&1
      else
        exec "${args[@]}" >"$log" 2>&1
      fi
    else
      export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      local args=(env "SLSKR_OVERLAY_BIND=127.0.0.1:${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT}" "$repo_root/target/debug/slskr" serve --app-dir "$state" --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port")
      if [[ "$command_line_name" != __unset__ ]]; then
        args+=(-i "$command_line_name")
      fi
      if [[ "$redirection" == '>>' ]]; then
        exec "${args[@]}" >>"$log" 2>&1
      else
        exec "${args[@]}" >"$log" 2>&1
      fi
    fi
  ) &
  daemon_pid="$!"
}

wait_for_instance_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 400); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value.get("instanceName") == sys.argv[1] else 1)' "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'instance-name differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'instance-name differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_instance_stage() {
  local implementation="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local log="$5"
  local expected_diagnostic="$6"
  local forbidden_diagnostic="${7:-__none__}"
  local options_file="$suite/instance-$stage-options.raw"
  local startup_file="$suite/instance-$stage-startup.raw"
  local application_file="$suite/instance-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$options_file"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup_file"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application_file"
  "$python_bin" - "$options_file" "$startup_file" "$application_file" "$log" \
    "$implementation" "$expected_diagnostic" "$forbidden_diagnostic" \
    >"$suite/instance-$stage.body" <<'PY'
import json,sys
options=json.load(open(sys.argv[1], encoding="utf-8"))
startup=json.load(open(sys.argv[2], encoding="utf-8"))
application=json.load(open(sys.argv[3], encoding="utf-8"))
log=open(sys.argv[4], encoding="utf-8", errors="replace").read().splitlines()
implementation=sys.argv[5]
expected=sys.argv[6]
forbidden=sys.argv[7]
def diagnostic(name):
    if implementation == "upstream":
        return any(f"Instance Name: {name}" in line for line in log)
    return any(f" instance {name} listening " in line for line in log)
print(json.dumps({
    "current": options.get("instanceName"),
    "startup": startup.get("instanceName"),
    "pendingRestart": application.get("pendingRestart"),
    "startupDiagnosticMatches": True if expected == "__skip__" else diagnostic(expected),
    "forbiddenStartupDiagnosticAbsent": True if forbidden == "__none__" else not diagnostic(forbidden),
}, sort_keys=True, separators=(",", ":")))
PY
  rm -f "$options_file" "$startup_file" "$application_file"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/instance-$stage.meta"
}

run_instance_name_scenario() {
  local target="$1"
  local root="$2"
  local long_name
  long_name="$($python_bin -c 'print("a" * 300)')"
  local long_control_name=$'line one\n'"$long_name"
  local long_control_yaml
  long_control_yaml="$($python_bin -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$long_control_name")"

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-instance-$implementation"
    local suite="$work_dir/$target-instance-$implementation"
    local log="$work_dir/$target-instance-$implementation.log"
    mkdir -p "$state" "$suite"

    write_instance_yaml "$state/slskd.yml" '"yaml-wins-environment"'
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port" environment-loses __unset__
    wait_for_options "$base_url" "$work_dir/$target-instance-$implementation-yaml-precedence.json" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" yaml-precedence "$log" yaml-wins-environment
    stop_daemon

    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port" environment-loses cli-wins true
    wait_for_options "$base_url" "$work_dir/$target-instance-$implementation-cli-precedence.json" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" cli-precedence "$log" cli-wins
    stop_daemon

    write_instance_yaml "$state/slskd.yml" '"watched-old"'
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port" __unset__ __unset__ true
    wait_for_options "$base_url" "$work_dir/$target-instance-$implementation-lifecycle.json" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" lifecycle-startup "$log" watched-old

    write_instance_yaml "$state/slskd.yml" 123
    wait_for_instance_option "$base_url" 123 "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" lifecycle-watched-numeric "$log" watched-old 123

    write_instance_yaml "$state/slskd.yml" '""'
    wait_for_instance_option "$base_url" default "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" lifecycle-watched-empty "$log" watched-old default
    stop_daemon

    write_instance_yaml "$state/slskd.yml" "$long_control_yaml"
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port" __unset__ __unset__ true
    wait_for_instance_option "$base_url" "$long_control_name" "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" long-control-restarted "$log" __skip__
    stop_daemon

    write_instance_yaml "$state/slskd.yml" '""'
    start_instance_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port" __unset__ __unset__ true
    wait_for_instance_option "$base_url" default "$log"
    capture_instance_stage "$implementation" "$base_url" "$suite" empty-restarted "$log" default

    capture_request "$suite" instance-validation-null POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: null\n')"
    capture_request "$suite" instance-validation-empty POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: ""\n')"
    capture_request "$suite" instance-validation-number POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: 123\n')"
    capture_request "$suite" instance-validation-bool POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: true\n')"
    capture_request "$suite" instance-validation-array POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name: [one, two]\n')"
    capture_request "$suite" instance-validation-object POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload $'instance_name:\n  nested: value\n')"
    capture_request "$suite" instance-validation-long-control POST \
      "$base_url/api/v0/options/yaml/validate" \
      "$(listener_validation_payload "instance_name: $long_control_yaml"$'\n')"
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-instance-upstream.normalized"
  local slskr_normalized="$work_dir/$target-instance-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-instance-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-instance-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'instance-name differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s instance-name differential passed\n' "$target"
}

write_completed_template_yaml() {
  local path="$1"
  local template="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: true\ntransfers:\n  download:\n    completed_path_template: "%s"\n' \
    "$template" >"$temporary"
  mv "$temporary" "$path"
}

start_completed_template_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  local environment_template="${7:-}"
  local cli_template="${8:-}"
  local cli_args=()
  [[ -n "$cli_template" ]] && cli_args+=(--download-completed-path-template "$cli_template")
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_template" ]] && export SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE="$environment_template"
      exec dotnet "$dll" "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE=native
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_template" ]] && export SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE="$environment_template"
      slskr_exec serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_for_completed_template() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value["global"]["download"]["completedPathTemplate"] == sys.argv[1] else 1)' \
        "$expected" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'completed-template differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'completed-template differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_completed_template_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "template-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "template-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "template-debug-$stage" "$base_url/api/v0/options/debug"
  capture_get "$suite" "template-application-$stage" "$base_url/api/v0/application"
}

run_completed_template_scenario() {
  local root="$1"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-template-$implementation"
    local suite="$work_dir/slskdn-template-$implementation"
    local log="$work_dir/slskdn-template-$implementation.log"
    mkdir -p "$state" "$suite"

    write_completed_template_yaml "$state/slskd.yml" 'yaml/{uploader}'
    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" 'environment/{uploader}'
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-yaml.json" "$log"
    wait_for_completed_template "$base_url" 'yaml/{uploader}' "$log"
    capture_completed_template_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" 'environment/{uploader}' 'cli/{remote_folder}' true
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-cli.json" "$log"
    wait_for_completed_template "$base_url" 'cli/{remote_folder}' "$log"
    capture_completed_template_stage "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    write_completed_template_yaml "$state/slskd.yml" 'startup/{uploader}'
    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" true
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-startup.json" "$log"
    wait_for_completed_template "$base_url" 'startup/{uploader}' "$log"
    capture_completed_template_stage "$base_url" "$suite" lifecycle-startup

    write_completed_template_yaml "$state/slskd.yml" 'watched/{remote_folder}'
    wait_for_completed_template "$base_url" 'watched/{remote_folder}' "$log"
    capture_completed_template_stage "$base_url" "$suite" lifecycle-watched

    for validation in \
      'completed-template-validation-string|transfers:\n  download:\n    completed_path_template: "valid/{uploader}"\n' \
      'completed-template-validation-empty|transfers:\n  download:\n    completed_path_template: ""\n' \
      'completed-template-validation-null|transfers:\n  download:\n    completed_path_template: null\n' \
      'completed-template-validation-number|transfers:\n  download:\n    completed_path_template: 123\n' \
      'completed-template-validation-array|transfers:\n  download:\n    completed_path_template: [one, two]\n' \
      'completed-template-validation-object|transfers:\n  download:\n    completed_path_template:\n      nested: value\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon

    start_completed_template_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" true
    wait_for_options "$base_url" "$work_dir/slskdn-template-$implementation-restarted.json" "$log"
    wait_for_completed_template "$base_url" 'watched/{remote_folder}' "$log"
    capture_completed_template_stage "$base_url" "$suite" lifecycle-restarted
    stop_daemon
  done

  local upstream_normalized="$work_dir/slskdn-template-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-template-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-template-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-template-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'completed-template differential failed for slskdN\n' >&2
    exit 1
  fi
  printf 'slskdn completed-template differential passed\n'
}

write_private_message_auto_response_yaml() {
  local path="$1"
  local no_connect="$2"
  local server_port="$3"
  local listen_port="$4"
  local enabled="$5"
  local message="$6"
  local cooldown="$7"
  local temporary="$path.tmp"
  local message_value="\"$message\""
  [[ "$message" == __NULL__ ]] && message_value=null
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n  private_message_auto_response:\n    enabled: %s\n    message: %s\n    cooldown_minutes: %s\n' \
    "$no_connect" "$server_port" "$listen_port" "$enabled" "$message_value" "$cooldown" \
    >"$temporary"
  mv "$temporary" "$path"
}

start_private_message_auto_response_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  local environment_enabled="${7:-}"
  local environment_message="${8:-}"
  local environment_cooldown="${9:-}"
  local cli_enabled="${10:-false}"
  local cli_message="${11:-}"
  local cli_cooldown="${12:-}"
  local cli_args=()
  [[ "$cli_enabled" == true ]] && cli_args+=(--slsk-private-message-auto-response)
  [[ -n "$cli_message" ]] && cli_args+=(--slsk-private-message-auto-response-message "$cli_message")
  [[ -n "$cli_cooldown" ]] && cli_args+=(--slsk-private-message-auto-response-cooldown-minutes "$cli_cooldown")
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE="$environment_enabled"
      [[ -n "$environment_message" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE="$environment_message"
      [[ -n "$environment_cooldown" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES="$environment_cooldown"
      exec dotnet "$dll" "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE=native
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE="$environment_enabled"
      [[ -n "$environment_message" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE="$environment_message"
      [[ -n "$environment_cooldown" ]] && export SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES="$environment_cooldown"
      slskr_exec serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

start_private_message_fixture() {
  local port="$1"
  local status="$2"
  local log="$3"
  local injection="$4"
  "$python_bin" "$repo_root/scripts/fixture-soulseek-listener.py" \
    127.0.0.1 "$port" "$status" login-success-private "$injection" >"$log" 2>&1 &
  soulseek_fixture_pid="$!"
  for _ in $(seq 1 100); do
    [[ -s "$status" ]] && return
    if ! kill -0 "$soulseek_fixture_pid" 2>/dev/null; then
      printf 'private-message auto-response fixture exited\n' >&2
      cat "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'private-message auto-response fixture did not become ready\n' >&2
  exit 1
}

wait_for_private_message_auto_response_options() {
  local base_url="$1"
  local enabled="$2"
  local message="$3"
  local cooldown="$4"
  local log="$5"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["soulseek"]["privateMessageAutoResponse"]; message=None if sys.argv[2] == "__NULL__" else sys.argv[2]; raise SystemExit(0 if value == {"enabled": sys.argv[1] == "true", "message": message, "cooldownMinutes": int(sys.argv[3])} else 1)' \
        "$enabled" "$message" "$cooldown" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'private-message auto-response daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'private-message auto-response timed out waiting for options\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_private_message_fixture() {
  local status="$1"
  local expected_injected="$2"
  local expected_responses="$3"
  local expected_message="${4:-}"
  local daemon_log="$5"
  for _ in $(seq 1 600); do
    if "$python_bin" - "$status" "$expected_injected" "$expected_responses" "$expected_message" <<'PY'
import json,sys
try:
    value=json.load(open(sys.argv[1], encoding="utf-8"))
except (FileNotFoundError, json.JSONDecodeError):
    raise SystemExit(1)
injected=value.get("injected_private_message_ids", [])
responses=value.get("private_message_responses", [])
expected_injected=int(sys.argv[2])
expected_responses=int(sys.argv[3])
expected_message=sys.argv[4]
valid=expected_injected in injected and len(responses) == expected_responses
if valid and expected_message:
    valid=responses[-1].get("message") == expected_message
raise SystemExit(0 if valid else 1)
PY
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'private-message auto-response daemon exited while waiting for fixture\n' >&2
      tail -120 "$daemon_log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'private-message auto-response timed out waiting for fixture\n' >&2
  tail -120 "$daemon_log" >&2 || true
  exit 1
}

capture_private_message_auto_response_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local fixture_status="${4:-}"
  capture_get "$suite" "auto-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "auto-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "auto-application-$stage" "$base_url/api/v0/application"
  if [[ -n "$fixture_status" ]]; then
    cp "$fixture_status" "$suite/auto-fixture-$stage.body"
  fi
}

run_private_message_auto_response_scenario() {
  local root="$1"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-auto-response-$implementation"
    local suite="$work_dir/slskdn-auto-response-$implementation"
    local log="$work_dir/slskdn-auto-response-$implementation.log"
    local fixture_status="$work_dir/slskdn-auto-response-$implementation-fixture.json"
    local fixture_log="$work_dir/slskdn-auto-response-$implementation-fixture.log"
    local injection="$work_dir/slskdn-auto-response-$implementation-injection.json"
    local runtime_default_message="Hi, I'm human and testing a slskdN client. Shares may be temporarily unavailable while I validate the client."
    local options_default_message="$runtime_default_message"
    if [[ "$implementation" == slskr ]]; then
      runtime_default_message="Hi, I'm human and testing an slskR client. Shares may be temporarily unavailable while I validate the client."
      options_default_message="Hi, I'm human and testing a slskr client. Shares may be temporarily unavailable while I validate the client."
    fi
    mkdir -p "$state" "$suite"

    write_private_message_auto_response_yaml "$state/slskd.yml" true "$server_port" "$listen_port" true 'yaml response' 15
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false 'environment response' 30
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-yaml.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" true 'yaml response' 15 "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    write_private_message_auto_response_yaml "$state/slskd.yml" true "$server_port" "$listen_port" false 'yaml response' 15
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false 'environment response' 30 true 'cli response' 45
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-cli.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" true 'cli response' 45 "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    printf '[]\n' >"$injection"
    start_private_message_fixture "$server_port" "$fixture_status" "$fixture_log" "$injection"
    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true 'startup response' 15
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-startup.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" true 'startup response' 15 "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 1 1 'startup response' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-startup "$fixture_status"

    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 2 1 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 2 1 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-cooldown "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true 'watched response' 20
    wait_for_private_message_auto_response_options "$base_url" true 'watched response' 20 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 3 2 'watched response' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-watched "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true '   ' 20
    wait_for_private_message_auto_response_options "$base_url" true '   ' 20 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"},{"id":4,"username":"FixtureThree","message":"Are you human?"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 4 2 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 4 2 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-blank "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" true __NULL__ 20
    # Each implementation exposes its own branded default in the options
    # sends the same value to the peer. The normalized suite compares these
    # known default identities without hiding configured text.
    wait_for_private_message_auto_response_options "$base_url" true "$options_default_message" 20 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"},{"id":4,"username":"FixtureThree","message":"Are you human?"},{"id":5,"username":"FixtureFour","message":"Please prove you are not a bot"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 5 3 "$runtime_default_message" "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-null "$fixture_status"

    write_private_message_auto_response_yaml "$state/slskd.yml" false "$server_port" "$listen_port" false 'disabled response' 25
    wait_for_private_message_auto_response_options "$base_url" false 'disabled response' 25 "$log"
    printf '[{"id":1,"username":"FixtureOne","message":"Please prove you are human"},{"id":2,"username":"FixtureOne","message":"Human verification challenge"},{"id":3,"username":"FixtureTwo","message":"Human verification challenge"},{"id":4,"username":"FixtureThree","message":"Are you human?"},{"id":5,"username":"FixtureFour","message":"Please prove you are not a bot"},{"id":6,"username":"FixtureFive","message":"Human verification challenge"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 6 3 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 6 3 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-disabled "$fixture_status"

    for validation in \
      'auto-validation-valid|soulseek:\n  private_message_auto_response:\n    enabled: true\n    message: response\n    cooldown_minutes: 1\n' \
      'auto-validation-disabled|soulseek:\n  private_message_auto_response:\n    enabled: false\n    message: response\n    cooldown_minutes: 1440\n' \
      'auto-validation-enabled-string|soulseek:\n  private_message_auto_response:\n    enabled: nope\n' \
      'auto-validation-message-empty|soulseek:\n  private_message_auto_response:\n    message: ""\n' \
      'auto-validation-message-null|soulseek:\n  private_message_auto_response:\n    message: null\n' \
      'auto-validation-message-number|soulseek:\n  private_message_auto_response:\n    message: 123\n' \
      'auto-validation-message-array|soulseek:\n  private_message_auto_response:\n    message: [response]\n' \
      'auto-validation-message-object|soulseek:\n  private_message_auto_response:\n    message:\n      nested: response\n' \
      'auto-validation-cooldown-zero|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: 0\n' \
      'auto-validation-cooldown-high|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: 1441\n' \
      'auto-validation-cooldown-numeric-string|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: "15"\n' \
      'auto-validation-cooldown-string|soulseek:\n  private_message_auto_response:\n    cooldown_minutes: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon
    stop_soulseek_fixture

    rm -f "$fixture_status"
    printf '[]\n' >"$injection"
    start_private_message_fixture "$server_port" "$fixture_status" "$fixture_log" "$injection"
    start_private_message_auto_response_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-auto-response-$implementation-restarted.json" "$log"
    wait_for_private_message_auto_response_options "$base_url" false 'disabled response' 25 "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    printf '[{"id":7,"username":"FixtureSix","message":"Are you human?"}]\n' >"$injection"
    wait_for_private_message_fixture "$fixture_status" 7 0 '' "$log"
    sleep 1
    wait_for_private_message_fixture "$fixture_status" 7 0 '' "$log"
    capture_private_message_auto_response_stage "$base_url" "$suite" lifecycle-restarted "$fixture_status"
    stop_daemon
    stop_soulseek_fixture
  done

  local upstream_normalized="$work_dir/slskdn-auto-response-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-auto-response-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-response-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-response-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'private-message auto-response differential failed for slskdN\n' >&2
    exit 1
  fi
  printf 'slskdn private-message auto-response differential passed\n'
}

