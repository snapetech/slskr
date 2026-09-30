#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

runtime_profile_for_reference() {
  case "$1" in
    slskd) printf '%s\n' legacy ;;
    slskdn) printf '%s\n' native ;;
    *) return 1 ;;
  esac
}

slskr_exec() {
  # The long-lived test service may own the default HTTPS port. Differential
  # cases must use the scenario's isolated HTTPS port while preserving HTTPS
  # behavior for the options comparison.
  local resolved_https_port="${https_port:-}"
  if [[ -z "$resolved_https_port" ]]; then
    resolved_https_port="$(pick_free_port)"
  fi
  export SLSKD_HTTPS_PORT="$resolved_https_port"
  local config_file="${SLSKR_CONFIG:-}"
  if [[ -z "$config_file" && -n "${SLSKD_APP_DIR:-}" ]]; then
    config_file="$SLSKD_APP_DIR/slskd.yml"
  fi
  local config_has_dht=false
  local config_has_overlay=false
  if [[ -f "$config_file" ]]; then
    # Match only top-level compatibility settings. Feature toggles such as
    # `feature.dht: false` must not suppress the isolated runtime port that
    # keeps an otherwise enabled service from colliding with another daemon.
    rg -q '^dht:' "$config_file" && config_has_dht=true
    rg -q '^(overlay|overlay_bind|listeners):' "$config_file" && config_has_overlay=true
  fi
  if [[ "$config_has_dht" != true && -z "${SLSKR_DHT_PORT:-}" ]]; then
    export SLSKR_DHT_PORT="$(pick_free_udp_port)"
  fi
  if [[ "$config_has_overlay" != true && -z "${SLSKR_OVERLAY_BIND:-}" ]]; then
    local configured_dht_port="${SLSKR_DHT_PORT:-}"
    if [[ -z "$configured_dht_port" && "$config_has_dht" == true ]]; then
        configured_dht_port="$(sed -nE 's/^dht_port:[[:space:]]*([0-9]+).*$/\1/p' "$config_file" | head -n 1)"
    fi
    local resolved_overlay_port="${overlay_port:-}"
    if [[ -z "$resolved_overlay_port" ]]; then
      resolved_overlay_port="$(pick_free_port)"
    fi
    while [[ -n "$configured_dht_port" && "$resolved_overlay_port" == "$configured_dht_port" ]]; do
      resolved_overlay_port="$(pick_free_port)"
    done
    export SLSKR_OVERLAY_BIND="127.0.0.1:$resolved_overlay_port"
  fi
  exec "$repo_root/target/debug/slskr" "$@"
}

python_bin="${PYTHON:-python3}"
upstream_repo="${SLSKR_UPSTREAM_GIT_REPO:-$repo_root/../slskdn}"
slskd_ref="${SLSKR_SLSKD_REF:-16e5d86ec9a91120f3ef40b85cb22036566b788a}"
slskdn_ref="${SLSKR_SLSKDN_REF:-65a14a8b821de4df4ab7ef3ab3b156d7206837a3}"
work_dir="${SLSKR_OPTIONS_DIFFERENTIAL_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-options-differential.XXXXXX")}"
keep_artifacts="${SLSKR_OPTIONS_DIFFERENTIAL_KEEP:-0}"
scenarios="${SLSKR_OPTIONS_DIFFERENTIAL_SCENARIOS:-all}"
slskd_root="${SLSKR_SLSKD_ROOT:-$work_dir/slskd}"
slskdn_root="${SLSKR_SLSKDN_ROOT:-$work_dir/slskdn}"
created_slskd_worktree=0
created_slskdn_worktree=0
daemon_pid=""
soulseek_fixture_pid=""
listener_blocker_pid=""
lidarr_fixture_pid=""
tcp_port_registry="$work_dir/.allocated-tcp-ports"
mkdir -p "$tcp_port_registry"

pick_free_port() {
  "$python_bin" - "$tcp_port_registry" <<'PY'
import os
import socket
import sys

registry = sys.argv[1]
for _ in range(256):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("0.0.0.0", 0))
        port = sock.getsockname()[1]
        try:
            os.mkdir(os.path.join(registry, str(port)))
        except FileExistsError:
            continue
        print(port)
        break
else:
    raise SystemExit("unable to allocate a unique free TCP port")
PY
}

pick_free_udp_port() {
  "$python_bin" - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
    sock.bind(("0.0.0.0", 0))
    print(sock.getsockname()[1])
PY
}

pick_free_port_with_free_successor() {
  "$python_bin" - "$tcp_port_registry" <<'PY'
import os
import socket
import sys

registry = sys.argv[1]
for _ in range(256):
    first = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    first.bind(("0.0.0.0", 0))
    port = first.getsockname()[1]
    if port >= 65535:
        first.close()
        continue
    second = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        second.bind(("0.0.0.0", port + 1))
    except OSError:
        first.close()
        second.close()
        continue
    first_claim = os.path.join(registry, str(port))
    second_claim = os.path.join(registry, str(port + 1))
    try:
        os.mkdir(first_claim)
    except FileExistsError:
        first.close()
        second.close()
        continue
    try:
        os.mkdir(second_claim)
    except FileExistsError:
        os.rmdir(first_claim)
        first.close()
        second.close()
        continue
    print(port)
    first.close()
    second.close()
    break
else:
    raise SystemExit("unable to allocate adjacent free TCP ports")
PY
}

stop_daemon() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
    # Frozen Soulseek listeners can remain unavailable briefly after host
    # shutdown even though the process has exited.
    sleep 0.5
  fi
  daemon_pid=""
}

stop_soulseek_fixture() {
  if [[ -n "$soulseek_fixture_pid" ]] && kill -0 "$soulseek_fixture_pid" 2>/dev/null; then
    kill "$soulseek_fixture_pid" 2>/dev/null || true
    wait "$soulseek_fixture_pid" 2>/dev/null || true
  fi
  soulseek_fixture_pid=""
}

stop_listener_blocker() {
  if [[ -n "$listener_blocker_pid" ]] && kill -0 "$listener_blocker_pid" 2>/dev/null; then
    kill "$listener_blocker_pid" 2>/dev/null || true
    wait "$listener_blocker_pid" 2>/dev/null || true
  fi
  listener_blocker_pid=""
}

stop_lidarr_fixture() {
  if [[ -n "$lidarr_fixture_pid" ]] && kill -0 "$lidarr_fixture_pid" 2>/dev/null; then
    kill "$lidarr_fixture_pid" 2>/dev/null || true
    wait "$lidarr_fixture_pid" 2>/dev/null || true
  fi
  lidarr_fixture_pid=""
}

cleanup() {
  stop_daemon
  stop_soulseek_fixture
  stop_listener_blocker
  stop_lidarr_fixture
  if [[ "$created_slskd_worktree" == "1" ]]; then
    git -C "$upstream_repo" worktree remove --force "$slskd_root" >/dev/null 2>&1 || true
  fi
  if [[ "$created_slskdn_worktree" == "1" ]]; then
    git -C "$upstream_repo" worktree remove --force "$slskdn_root" >/dev/null 2>&1 || true
  fi
  if [[ "$keep_artifacts" != "1" ]]; then
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

materialize_target() {
  local root="$1"
  local ref="$2"
  local created_variable="$3"
  if [[ -d "$root/.git" || -f "$root/.git" ]]; then
    local actual
    actual="$(git -C "$root" rev-parse HEAD)"
    if [[ "$actual" != "$ref" ]]; then
      printf 'options differential failed: %s is at %s, expected %s\n' "$root" "$actual" "$ref" >&2
      exit 1
    fi
    return
  fi
  mkdir -p "$(dirname "$root")"
  git -C "$upstream_repo" worktree add --detach "$root" "$ref" >/dev/null
  printf -v "$created_variable" '%s' 1
}

scenario_enabled() {
  [[ "$scenarios" == all || ",$scenarios," == *",$1,"* ]]
}

wait_for_options() {
  local base_url="$1"
  local output="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" >"$output"; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'options differential failed: daemon exited before %s became ready\n' "$base_url" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'options differential failed: timed out waiting for %s\n' "$base_url" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_share_files() {
  local base_url="$1"
  local alias="$2"
  local expected_files="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/shares" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); alias=sys.argv[1]; expected=int(sys.argv[2]); raise SystemExit(0 if any(share.get("alias") == alias and share.get("files") == expected for shares in value.values() for share in shares) else 1)' "$alias" "$expected_files" 2>/dev/null; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'options differential failed: daemon exited while waiting for share %s\n' "$alias" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'options differential failed: timed out waiting for share %s\n' "$alias" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_request() {
  local suite_dir="$1"
  local label="$2"
  local method="$3"
  local url="$4"
  local payload="$5"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --request "$method" \
    --header 'Content-Type: application/json' \
    --data-binary "$payload" \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

capture_mutation_suite() {
  local base_url="$1"
  local suite_dir="$2"
  capture_request "$suite_dir" patch-null PATCH "$base_url/api/v0/options" 'null'
  capture_request "$suite_dir" patch-array PATCH "$base_url/api/v0/options" '[]'
  capture_request "$suite_dir" patch-valid PATCH "$base_url/api/v0/options" \
    '{"soulseek":{"listenPort":50300}}'
  capture_request "$suite_dir" yaml-valid POST "$base_url/api/v0/options/yaml/validate" \
    '"debug: false\n"'
  capture_request "$suite_dir" yaml-invalid POST "$base_url/api/v0/options/yaml/validate" \
    '"web: [unterminated"'
}

capture_get() {
  local suite_dir="$1"
  local label="$2"
  local url="$3"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

capture_delete() {
  local suite_dir="$1"
  local label="$2"
  local url="$3"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --request DELETE \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

capture_put() {
  local suite_dir="$1"
  local label="$2"
  local url="$3"
  mkdir -p "$suite_dir"
  curl --silent --show-error --max-time 10 \
    --request PUT \
    --output "$suite_dir/$label.body" \
    --write-out $'status=%{http_code}\ncontent-type=%{content_type}\n' \
    "$url" >"$suite_dir/$label.meta"
}

compare_mutation_suites() {
  local target="$1"
  local expected="$2"
  local actual="$3"
  if ! diff -ru "$expected" "$actual"; then
    printf 'options mutation differential failed for %s\n' "$target" >&2
    exit 1
  fi
}

source scripts/options_differential/part_01.sh
source scripts/options_differential/part_02.sh
source scripts/options_differential/part_03.sh
source scripts/options_differential/part_04.sh
source scripts/options_differential/part_05.sh
source scripts/options_differential/part_06.sh
# A few legacy scenario launchers assemble an argument array instead of using
# slskr_exec. Give those cases a safe HTTPS fallback; scenarios that allocate
# an explicit port still override it in their child environment.
if [[ -z "${SLSKD_HTTPS_PORT:-}" ]]; then
  export SLSKD_HTTPS_PORT="$(pick_free_port)"
fi
if [[ -z "${SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT:-}" ]]; then
  export SLSKR_OPTIONS_DIFFERENTIAL_OVERLAY_PORT="$(pick_free_port)"
fi
materialize_target "$slskd_root" "$slskd_ref" created_slskd_worktree
materialize_target "$slskdn_root" "$slskdn_ref" created_slskdn_worktree

export DOTNET_CLI_TELEMETRY_OPTOUT=1
dotnet build "$slskd_root/src/slskd/slskd.csproj" -c Release -r linux-x64 --self-contained false -v:q
dotnet build "$slskdn_root/src/slskd/slskd.csproj" -c Release -r linux-x64 --self-contained false -v:q
# Frozen slskdN resolves its default content path relative to the application
# base directory and refuses to start if it is absent. The audited API
# scenarios do not need UI assets, but they do need the validated directory.
mkdir -p "$slskdn_root/src/slskd/bin/Release/net10.0/linux-x64/wwwroot"
if scenario_enabled options; then
  run_target slskd "$slskd_root"
  run_target slskdn "$slskdn_root"
fi
if scenario_enabled directories; then
  run_directory_scenario slskd "$slskd_root"
  run_directory_scenario slskdn "$slskdn_root"
fi
if scenario_enabled shares; then
  run_share_watch_scenario slskd "$slskd_root"
  run_share_watch_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-watch; then
  run_no_watch_upload_scenario slskd "$slskd_root"
  run_no_watch_upload_scenario slskdn "$slskdn_root"
fi
if scenario_enabled storage; then
  run_storage_restart_scenario slskd "$slskd_root"
  run_storage_restart_scenario slskdn "$slskdn_root"
fi
if scenario_enabled file-management; then
  run_remote_file_management_scenario slskd "$slskd_root"
  run_remote_file_management_scenario slskdn "$slskdn_root"
fi
if scenario_enabled remote-configuration; then
  run_remote_configuration_scenario slskd "$slskd_root"
  run_remote_configuration_scenario slskdn "$slskdn_root"
fi
if scenario_enabled debug; then
  run_debug_scenario slskd "$slskd_root"
  run_debug_scenario slskdn "$slskdn_root"
fi
if scenario_enabled swagger; then
  run_swagger_scenario slskd "$slskd_root"
  run_swagger_scenario slskdn "$slskdn_root"
fi
if scenario_enabled metrics; then
  run_metrics_scenario slskd "$slskd_root"
  run_metrics_scenario slskdn "$slskdn_root"
fi
if scenario_enabled headless; then
  run_headless_scenario slskd "$slskd_root"
  run_headless_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-start; then
  run_no_start_scenario slskd "$slskd_root"
  run_no_start_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-logo; then
  run_no_logo_scenario slskd "$slskd_root"
  run_no_logo_scenario slskdn "$slskdn_root"
fi
if scenario_enabled no-version-check; then
  run_no_version_check_scenario slskd "$slskd_root"
  run_no_version_check_scenario slskdn "$slskdn_root"
fi
if scenario_enabled experimental; then
  run_experimental_scenario slskd "$slskd_root"
  run_experimental_scenario slskdn "$slskdn_root"
fi
if scenario_enabled case-sensitive-regex; then
  run_case_sensitive_regex_scenario slskd "$slskd_root"
  run_case_sensitive_regex_scenario slskdn "$slskdn_root"
fi
if scenario_enabled regex-runtime; then
  run_regex_runtime_scenario slskd "$slskd_root"
  run_regex_runtime_scenario slskdn "$slskdn_root"
fi
if scenario_enabled regex-protocol; then
  run_regex_protocol_scenario slskd "$slskd_root"
  run_regex_protocol_scenario slskdn "$slskdn_root"
fi
if scenario_enabled share-scan-flags; then
  run_share_scan_flags_scenario slskd "$slskd_root"
  run_share_scan_flags_scenario slskdn "$slskdn_root"
fi
if scenario_enabled instance-name; then
  run_instance_name_scenario slskd "$slskd_root"
  run_instance_name_scenario slskdn "$slskdn_root"
fi
if scenario_enabled completed-template; then
  run_completed_template_scenario "$slskdn_root"
fi
if scenario_enabled private-message-auto-response; then
  run_private_message_auto_response_scenario "$slskdn_root"
fi
if scenario_enabled download-auto-retry; then
  run_download_auto_retry_scenario "$slskdn_root"
fi
if scenario_enabled blacklist; then
  run_blacklist_scenario slskd "$slskd_root"
  run_blacklist_scenario slskdn "$slskdn_root"
fi
if scenario_enabled transfer-groups; then
  run_transfer_groups_scenario slskd "$slskd_root"
  run_transfer_groups_scenario slskdn "$slskdn_root"
fi
if scenario_enabled transfer-download; then
  run_transfer_download_scenario slskd "$slskd_root"
  run_transfer_download_scenario slskdn "$slskdn_root"
fi
if scenario_enabled soulseek-connection; then
  run_soulseek_connection_scenario slskd "$slskd_root"
  run_soulseek_connection_scenario slskdn "$slskdn_root"
fi
if scenario_enabled soulseek-profile-distributed; then
  run_soulseek_profile_distributed_scenario slskd "$slskd_root"
  run_soulseek_profile_distributed_scenario slskdn "$slskdn_root"
fi
if scenario_enabled daemon-foundation; then
  run_daemon_foundation_scenario slskd "$slskd_root"
  run_daemon_foundation_scenario slskdn "$slskdn_root"
fi
if scenario_enabled core-workflow; then
  run_core_workflow_scenario slskd "$slskd_root"
  run_core_workflow_scenario slskdn "$slskdn_root"
fi
if scenario_enabled advanced-networking-security; then
  run_advanced_networking_security_scenario "$slskdn_root"
fi
if scenario_enabled media-advanced-service; then
  run_media_advanced_service_scenario "$slskdn_root"
fi
if scenario_enabled dht; then
  run_dht_scenario "$slskdn_root"
fi
if scenario_enabled server-endpoint; then
  run_server_endpoint_scenario slskd "$slskd_root"
  run_server_endpoint_scenario slskdn "$slskdn_root"
fi
if scenario_enabled credentials; then
  run_credential_scenario slskd "$slskd_root"
  run_credential_scenario slskdn "$slskdn_root"
fi
if scenario_enabled obfuscation; then
  run_obfuscation_options_scenario "$slskdn_root"
  run_obfuscation_runtime_scenario "$slskdn_root"
  run_obfuscation_outbound_scenario "$slskdn_root"
elif scenario_enabled obfuscation-outbound; then
  run_obfuscation_outbound_scenario "$slskdn_root"
fi
if scenario_enabled no-connect; then
  run_no_connect_scenario slskd "$slskd_root"
  run_no_connect_scenario slskdn "$slskdn_root"
fi
if scenario_enabled config-watch; then
  run_config_watch_scenario slskd "$slskd_root"
  run_config_watch_scenario slskdn "$slskdn_root"
fi
if scenario_enabled description; then
  run_description_scenario slskd "$slskd_root"
  run_description_scenario slskdn "$slskdn_root"
fi
if scenario_enabled web-listener; then
  run_web_listener_scenario slskd "$slskd_root"
  run_web_listener_scenario slskdn "$slskdn_root"
fi
if scenario_enabled listener; then
  run_listener_scenario slskd "$slskd_root"
  run_listener_scenario slskdn "$slskdn_root"
fi
if scenario_enabled integrations; then
  run_script_scenario slskd "$slskd_root"
  run_script_scenario slskdn "$slskdn_root"
  run_integration_scenario "$slskdn_root"
fi
if scenario_enabled lidarr-runtime; then
  run_lidarr_runtime_scenario "$slskdn_root"
fi

printf 'controller options differential passed for frozen slskd + slskdN\n'
if [[ "$keep_artifacts" == "1" ]]; then
  printf 'options differential artifacts retained at %s\n' "$work_dir"
fi
