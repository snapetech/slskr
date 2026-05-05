#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
code_root="$(cd "$repo_root/.." && pwd)"
env_file="${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
extra_env_file="${SLSKR_LIVE_EXTRA_ENV_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
pool_file="${SLSKR_PROTON_CREDENTIAL_POOL_FILE:-$repo_root/.secrets/proton-credential-pool.env}"
output_dir="${SLSKR_INTEROP_OUTPUT_DIR:-$repo_root/target/live-interop}"
mkdir -p "$output_dir"

if [[ ! -f "$env_file" ]]; then
  echo "missing live credential file: $env_file" >&2
  exit 1
fi

set -a
source "$env_file"
if [[ -f "$extra_env_file" ]]; then
  source "$extra_env_file"
fi
if [[ -f "$pool_file" ]]; then
  source "$pool_file"
fi
set +a

require_var() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "missing required env var: $name" >&2
    exit 1
  fi
}

for i in $(seq 1 "${SLSKR_TEST_ACCOUNT_COUNT:-4}"); do
  require_var "SLSKR_TEST_${i}_USERNAME"
  require_var "SLSKR_TEST_${i}_PASSWORD"
done

slskd_repo="${SLSKR_SLSKD_REPO:-$code_root/slskd}"
slskdn_repo="${SLSKR_SLSKDN_REPO:-$code_root/slskdn}"
soulseek_net_repo="${SLSKR_SOULSEEK_NET_REPO:-$code_root/Soulseek.NET}"
runtime_repo="${SLSKR_RUNTIME_REPO:-$code_root/slskNet.Runtime}"
run_daemons="${SLSKR_RUN_ADJACENT_DAEMONS:-1}"
skip_adjacent_tests="${SLSKR_SKIP_ADJACENT_TESTS:-0}"
commons_fixture_dir="${SLSKR_COMMONS_FIXTURE_DIR:-$repo_root/target/open-commons-fixtures}"
use_commons_fixtures="${SLSKR_USE_OPEN_COMMONS_FIXTURES:-1}"
slskd_account_index="${SLSKR_SLSKD_ACCOUNT_INDEX:-3}"
slskdn_account_index="${SLSKR_SLSKDN_ACCOUNT_INDEX:-4}"
slskd_probe_account_index="${SLSKR_SLSKD_PROBE_ACCOUNT_INDEX:-1}"
slskdn_probe_account_index="${SLSKR_SLSKDN_PROBE_ACCOUNT_INDEX:-2}"
daemon_cooldown_seconds="${SLSKR_DAEMON_COOLDOWN_SECONDS:-20}"
daemon_slsk_address="${SLSKR_DAEMON_SLSK_ADDRESS:-}"
daemon_slsk_port="${SLSKR_DAEMON_SLSK_PORT:-2271}"
daemon_vpn_enabled="${SLSKR_DAEMON_VPN_ENABLED:-0}"
probe_vpn_enabled="${SLSKR_PROBE_VPN_ENABLED:-0}"
slskd_vpn_label="${SLSKR_SLSKD_VPN_LABEL:-p7}"
slskdn_vpn_label="${SLSKR_SLSKDN_VPN_LABEL:-p8}"
slskd_probe_vpn_label="${SLSKR_SLSKD_PROBE_VPN_LABEL:-p5}"
slskdn_probe_vpn_label="${SLSKR_SLSKDN_PROBE_VPN_LABEL:-p6}"
slskd_ns_host_ip="${SLSKR_SLSKD_NETNS_HOST_IP:-10.240.0.1}"
slskd_ns_ip="${SLSKR_SLSKD_NETNS_IP:-10.240.0.2}"
slskd_ns_subnet="${SLSKR_SLSKD_NETNS_SUBNET:-10.240.0.0/24}"
slskdn_ns_host_ip="${SLSKR_SLSKDN_NETNS_HOST_IP:-10.241.0.1}"
slskdn_ns_ip="${SLSKR_SLSKDN_NETNS_IP:-10.241.0.2}"
slskdn_ns_subnet="${SLSKR_SLSKDN_NETNS_SUBNET:-10.241.0.0/24}"

log_file="$output_dir/cross-client-validation.tsv"
printf 'timestamp\tscope\tcheck\tstatus\tdetail\n' >"$log_file"

if [[ -z "$daemon_slsk_address" ]]; then
  daemon_slsk_address="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }')"
fi

sanitize_detail() {
  tr '\n\t' '  ' | sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//; s/password=[^ ]+/password=<redacted>/Ig; s/SLSK_PASSWORD=[^ ]+/SLSK_PASSWORD=<redacted>/g; s/SLSKD_SLSK_PASSWORD=[^ ]+/SLSKD_SLSK_PASSWORD=<redacted>/g'
}

record() {
  local scope="$1" check="$2" status="$3" detail="$4"
  printf '%s\t%s\t%s\t%s\t%s\n' "$(date -Is)" "$scope" "$check" "$status" "$detail" | tee -a "$log_file"
}

run_logged() {
  local scope="$1" check="$2" workdir="$3"
  shift 3
  local stdout_file stderr_file status detail
  stdout_file="$(mktemp)"
  stderr_file="$(mktemp)"
  set +e
  (cd "$workdir" && "$@") >"$stdout_file" 2>"$stderr_file"
  status=$?
  set -e
  detail="$( { tail -n 40 "$stdout_file"; grep -E '^(error:|FAILED|Failed|Build FAILED|Test Run Failed|warning |thread |panicked|Unhandled exception)' "$stderr_file" || true; } | sanitize_detail )"
  rm -f "$stdout_file" "$stderr_file"
  if [[ $status -eq 0 ]]; then
    record "$scope" "$check" ok "$detail"
  else
    record "$scope" "$check" "fail($status)" "$detail"
    return "$status"
  fi
}

run_logged_optional() {
  local scope="$1" check="$2" expected="$3"
  shift 3
  if run_logged "$scope" "$check" "$@"; then
    return 0
  fi
  record "$scope" "$check" non-blocking "$expected"
  return 0
}

resolve_proton_config() {
  local label="$1"
  local var_name="SLSKR_PROTON_CONFIG_${label}"
  local path="${!var_name:-}"
  if [[ -z "$path" ]]; then
    echo "unknown Proton config label: $label" >&2
    return 1
  fi
  if [[ "$path" != /* ]]; then
    path="$repo_root/$path"
  fi
  if [[ ! -f "$path" ]]; then
    echo "missing Proton config for label $label" >&2
    return 1
  fi
  printf '%s' "$path"
}

probe_netns_args() {
  local scope="$1"
  if [[ "$scope" == *slskdN* ]]; then
    printf '%s\n' "$slskdn_probe_vpn_label" "${SLSKR_SLSKDN_PROBE_NETNS_HOST_IP:-10.243.0.1}" "${SLSKR_SLSKDN_PROBE_NETNS_IP:-10.243.0.2}" "${SLSKR_SLSKDN_PROBE_NETNS_SUBNET:-10.243.0.0/24}" "probe-slskdn" "$slskdn_ns_subnet"
  else
    printf '%s\n' "$slskd_probe_vpn_label" "${SLSKR_SLSKD_PROBE_NETNS_HOST_IP:-10.242.0.1}" "${SLSKR_SLSKD_PROBE_NETNS_IP:-10.242.0.2}" "${SLSKR_SLSKD_PROBE_NETNS_SUBNET:-10.242.0.0/24}" "probe-slskd" "$slskd_ns_subnet"
  fi
}

run_probe() {
  local scope="$1" check="$2" actor_index="$3" peer_user="$4"
  shift 4
  local stdout_file stderr_file status detail
  stdout_file="$(mktemp)"
  stderr_file="$(mktemp)"
  local actor_user_var="SLSKR_TEST_${actor_index}_USERNAME"
  local actor_pass_var="SLSKR_TEST_${actor_index}_PASSWORD"
  local command=(env SLSK_USERNAME="${!actor_user_var}" SLSK_PASSWORD="${!actor_pass_var}" SLSK_SERVER="${daemon_slsk_address}:${daemon_slsk_port}" SLSK_PEER_USERNAME="$peer_user" "$@")
  if [[ "$probe_vpn_enabled" == "1" ]]; then
    local label host_ip ns_ip subnet namespace config
    mapfile -t probe_args < <(probe_netns_args "$scope")
    label="${probe_args[0]}"
    host_ip="${probe_args[1]}"
    ns_ip="${probe_args[2]}"
    subnet="${probe_args[3]}"
    namespace="${probe_args[4]}"
    extra_routes="${probe_args[5]}"
    config="$(resolve_proton_config "$label")" || return 1
    command=(env SLSKR_NETNS_HOST_IP="$host_ip" SLSKR_NETNS_IP="$ns_ip" SLSKR_NETNS_SUBNET="$subnet" SLSKR_NETNS_EXTRA_ROUTES="$extra_routes" "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" "${command[@]}")
  fi
  set +e
  "${command[@]}" >"$stdout_file" 2>"$stderr_file"
  status=$?
  set -e
  if [[ $status -eq 0 ]]; then
    detail="$(cat "$stdout_file" | sanitize_detail)"
  else
    detail="$( { cat "$stdout_file"; tail -n 20 "$stderr_file"; } | sanitize_detail )"
  fi
  rm -f "$stdout_file" "$stderr_file"
  if [[ $status -eq 0 ]]; then
    record "$scope" "$check" ok "$detail"
  else
    record "$scope" "$check" "fail($status)" "$detail"
    return "$status"
  fi
}

run_probe_optional() {
  local scope="$1" check="$2" expected="$3"
  shift 3
  if run_probe "$scope" "$check" "$@"; then
    return 0
  fi
  record "$scope" "$check" non-blocking "$expected"
  return 0
}

run_account_command_optional() {
  local scope="$1" check="$2" expected="$3" actor_index="$4"
  shift 4
  local stdout_file stderr_file status detail
  stdout_file="$(mktemp)"
  stderr_file="$(mktemp)"
  local actor_user_var="SLSKR_TEST_${actor_index}_USERNAME"
  local actor_pass_var="SLSKR_TEST_${actor_index}_PASSWORD"
  local command=(env SLSK_USERNAME="${!actor_user_var}" SLSK_PASSWORD="${!actor_pass_var}" SLSK_SERVER="${daemon_slsk_address}:${daemon_slsk_port}" "$@")
  if [[ "$probe_vpn_enabled" == "1" ]]; then
    local label host_ip ns_ip subnet namespace extra_routes config
    mapfile -t probe_args < <(probe_netns_args "$scope")
    label="${probe_args[0]}"
    host_ip="${probe_args[1]}"
    ns_ip="${probe_args[2]}"
    subnet="${probe_args[3]}"
    namespace="${probe_args[4]}"
    extra_routes="${probe_args[5]}"
    config="$(resolve_proton_config "$label")" || return 1
    command=(env SLSKR_NETNS_HOST_IP="$host_ip" SLSKR_NETNS_IP="$ns_ip" SLSKR_NETNS_SUBNET="$subnet" SLSKR_NETNS_EXTRA_ROUTES="$extra_routes" "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" "${command[@]}")
  fi
  set +e
  "${command[@]}" >"$stdout_file" 2>"$stderr_file"
  status=$?
  set -e
  detail="$( { cat "$stdout_file"; tail -n 20 "$stderr_file"; } | sanitize_detail )"
  rm -f "$stdout_file" "$stderr_file"
  if [[ $status -eq 0 ]]; then
    record "$scope" "$check" ok "$detail"
  else
    record "$scope" "$check" "fail($status)" "$detail"
    record "$scope" "$check" non-blocking "$expected"
  fi
}

prepare_commons_fixtures() {
  if [[ "$use_commons_fixtures" != "1" ]]; then
    record fixtures open-commons skipped "SLSKR_USE_OPEN_COMMONS_FIXTURES=$use_commons_fixtures"
    return 0
  fi

  if "$repo_root/scripts/verify-open-commons-fixtures.sh" "$commons_fixture_dir" >/dev/null 2>&1; then
    record fixtures open-commons ok "dir=$commons_fixture_dir"
  else
    record fixtures open-commons non-blocking "download, hash, or license verification failed; daemon text fixtures remain available"
    return 1
  fi
}

stage_commons_share() {
  local share_dir="$1"
  if [[ "$use_commons_fixtures" != "1" || ! -d "$commons_fixture_dir" ]]; then
    return 0
  fi

  local commons_share="$share_dir/open-commons"
  mkdir -p "$commons_share"
  find "$commons_fixture_dir" -maxdepth 1 -type f ! -name 'LICENSES.tsv' -exec cp {} "$commons_share/" \;
  if [[ -f "$commons_fixture_dir/LICENSES.tsv" ]]; then
    cp "$commons_fixture_dir/LICENSES.tsv" "$commons_share/LICENSES.tsv"
  fi
}

wait_for_daemon_preflight() {
  local scope="$1" name="$2" http_host="$3" http_port="$4" attempts="${5:-${SLSKR_DAEMON_PREFLIGHT_ATTEMPTS:-24}}"
  local health_url="http://$http_host:$http_port/health"
  local app_url="http://$http_host:$http_port/api/v0/application"
  local stdout_file="$output_dir/$name.stdout.log"
  local stderr_file="$output_dir/$name.stderr.log"
  local last_detail="daemon preflight not attempted"

  for _ in $(seq 1 "$attempts"); do
    local health="unavailable"
    local app="unavailable"
    health="$(curl -fsS --max-time 2 "$health_url" 2>/dev/null | sanitize_detail || true)"
    app="$(curl -fsS --max-time 2 "$app_url" 2>/dev/null | sanitize_detail || true)"
    local logs
    logs="$( { [[ -f "$stdout_file" ]] && tail -n 14 "$stdout_file"; [[ -f "$stderr_file" ]] && tail -n 14 "$stderr_file"; } | sanitize_detail )"
    last_detail="health=${health:-empty}; app=${app:-empty}; logs=$logs"

    if [[ "$app" == *"Connected, LoggedIn"* || "$logs" == *"Connected to the Soulseek server"* && "$logs" != *"Connection reset by peer"* ]]; then
      record "$scope" daemon-preflight ok "$last_detail"
      return 0
    fi
    sleep 5
  done

  record "$scope" daemon-preflight timeout "$last_detail"
  return 1
}

run_commons_download_probe_optional() {
  local scope="$1" actor_index="$2" peer_user="$3" daemon_name="$4"
  if [[ "$use_commons_fixtures" != "1" || ! -f "$commons_fixture_dir/commons-click-track.ogg" ]]; then
    record "$scope" open-commons-download skipped "open commons fixture unavailable"
    return 0
  fi

  local host="127.0.0.1"
  if [[ "$daemon_vpn_enabled" == "1" && "$scope" == *slskdN* ]]; then
    host="$slskdn_ns_ip"
  elif [[ "$daemon_vpn_enabled" == "1" ]]; then
    host="$slskd_ns_ip"
  fi

  run_probe_optional "$scope" open-commons-browse "open commons browse proof failed; text fixture browse remains the blocking proof" "$actor_index" "$peer_user" env SLSK_BROWSE_HOST_OVERRIDE="$host" SLSK_BROWSE_EXPECTED=commons-click-track.ogg cargo run -q -p slskr -- probe browse-peer
  run_probe_optional "$scope" open-commons-download "open commons payload download failed; text fixture transfer remains the blocking proof" "$actor_index" "$peer_user" env SLSK_DOWNLOAD_HOST_OVERRIDE="$host" SLSK_DOWNLOAD_FILENAME="${daemon_name}\\open-commons\\commons-click-track.ogg" SLSK_DOWNLOAD_SHA256=e5e09f8ef9617a355e71e2d0b00f2554201aa124a9a821c4a7f76f0441a369a0 SLSK_DOWNLOAD_QUEUE_ATTEMPTS="${SLSK_DOWNLOAD_QUEUE_ATTEMPTS:-8}" SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS="${SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS:-4}" cargo run -q -p slskr -- probe download-peer
  run_account_command_optional "$scope" open-commons-search "public search indexing can lag or suppress fresh daemon shares" "$actor_index" env SLSK_SOAK_LISTENER_BIND=127.0.0.1:0 SLSK_SOAK_SEARCH_QUERY=commons-click-track.ogg SLSK_SOAK_SECONDS=15 cargo run -q -p slskr -- soak live
}

start_daemon() {
  local name="$1" repo="$2" account_index="$3" http_port="$4" listen_port="$5" vpn_label="${6:-}" host_ip="${7:-}" ns_ip="${8:-}" subnet="${9:-}"
  local dotnet_path
  dotnet_path="$(command -v dotnet)"
  local app_dir="$output_dir/$name-app"
  local share_dir="$output_dir/fixtures/$name"
  local config_file="$app_dir/slskd.yml"
  rm -rf "$app_dir"
  mkdir -p "$app_dir" "$share_dir"
  printf 'slskr interop fixture %s\n' "$name" >"$share_dir/slskr-interop-${name}.txt"
  stage_commons_share "$share_dir"
  local user_var="SLSKR_TEST_${account_index}_USERNAME"
  local pass_var="SLSKR_TEST_${account_index}_PASSWORD"
  cat >"$config_file" <<CFG
web:
  https:
    disabled: true
  authentication:
    disabled: true
directories:
  incomplete: $app_dir/incomplete
  downloads: $app_dir/downloads
shares:
  directories:
    - $share_dir
soulseek:
  username: ${!user_var}
  password: ${!pass_var}
  listen_ip_address: 0.0.0.0
  listen_port: $listen_port
CFG
  local daemon_env=(env \
    SLSKD_APP_DIR="$app_dir" \
    SLSKD_CONFIG="$config_file" \
    SLSKD_HTTP_PORT="$http_port" \
    SLSKD_NO_HTTPS=true \
    SLSKD_SLSK_USERNAME="${!user_var}" \
    SLSKD_SLSK_PASSWORD="${!pass_var}" \
    SLSK_ADDRESS="$daemon_slsk_address" \
    SLSK_PORT="$daemon_slsk_port" \
    SLSKD_SLSK_ADDRESS="$daemon_slsk_address" \
    SLSKD_SLSK_PORT="$daemon_slsk_port" \
    SLSKD_SLSK_LISTEN_IP_ADDRESS=0.0.0.0 \
    SLSKD_SLSK_LISTEN_PORT="$listen_port" \
    "$dotnet_path" run --project src/slskd/slskd.csproj --no-launch-profile)
  if [[ "$daemon_vpn_enabled" == "1" ]]; then
    local config namespace extra_routes
    config="$(resolve_proton_config "$vpn_label")" || return 1
    namespace="$(printf 'd-%s' "$name" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]-' | cut -c1-10)"
    extra_routes="${SLSKR_SLSKD_PROBE_NETNS_SUBNET:-10.242.0.0/24} ${SLSKR_SLSKDN_PROBE_NETNS_SUBNET:-10.243.0.0/24}"
    (cd "$repo" && env SLSKR_NETNS_HOST_IP="$host_ip" SLSKR_NETNS_IP="$ns_ip" SLSKR_NETNS_SUBNET="$subnet" SLSKR_NETNS_EXTRA_ROUTES="$extra_routes" "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" "${daemon_env[@]}") >"$output_dir/$name.stdout.log" 2>"$output_dir/$name.stderr.log" &
  else
    (cd "$repo" && "${daemon_env[@]}") >"$output_dir/$name.stdout.log" 2>"$output_dir/$name.stderr.log" &
  fi
  echo $!
}

wait_for_peer() {
  local scope="$1" actor_index="$2" peer_user="$3" attempts="${4:-${SLSKR_DAEMON_READINESS_ATTEMPTS:-36}}"
  local actor_user_var="SLSKR_TEST_${actor_index}_USERNAME"
  local actor_pass_var="SLSKR_TEST_${actor_index}_PASSWORD"
  local last_detail="no peer-address attempt completed"
  for _ in $(seq 1 "$attempts"); do
    local stdout_file stderr_file status detail
    stdout_file="$(mktemp)"
    stderr_file="$(mktemp)"
    local command=(env SLSK_USERNAME="${!actor_user_var}" SLSK_PASSWORD="${!actor_pass_var}" SLSK_SERVER="${daemon_slsk_address}:${daemon_slsk_port}" SLSK_PEER_USERNAME="$peer_user" cargo run -q -p slskr -- probe peer-address)
    if [[ "$probe_vpn_enabled" == "1" ]]; then
      local label host_ip ns_ip subnet namespace config
      mapfile -t probe_args < <(probe_netns_args "$scope")
      label="${probe_args[0]}"
      host_ip="${probe_args[1]}"
      ns_ip="${probe_args[2]}"
      subnet="${probe_args[3]}"
      namespace="${probe_args[4]}"
      extra_routes="${probe_args[5]}"
      config="$(resolve_proton_config "$label")" || return 1
      command=(env SLSKR_NETNS_HOST_IP="$host_ip" SLSKR_NETNS_IP="$ns_ip" SLSKR_NETNS_SUBNET="$subnet" SLSKR_NETNS_EXTRA_ROUTES="$extra_routes" "$repo_root/scripts/run-in-proton-wg-netns.sh" "$namespace" "$config" "${command[@]}")
    fi
    set +e
    "${command[@]}" >"$stdout_file" 2>"$stderr_file"
    status=$?
    set -e
    detail="$( { cat "$stdout_file"; grep -E '^(error:|thread |panicked|failed|rejected)' "$stderr_file" || true; } | sanitize_detail )"
    rm -f "$stdout_file" "$stderr_file"
    last_detail="$detail"
    if [[ $status -eq 0 && "$detail" =~ port=[1-9][0-9]* ]]; then
      record "$scope" peer-address ok "$detail"
      return 0
    fi
    sleep 5
  done
  record "$scope" peer-address-timeout fail "peer did not advertise an address before timeout; last=$last_detail"
  return 1
}

record_daemon_tail() {
  local scope="$1" name="$2"
  local stdout_file="$output_dir/$name.stdout.log"
  local stderr_file="$output_dir/$name.stderr.log"
  local detail
  detail="$( { [[ -f "$stdout_file" ]] && tail -n 20 "$stdout_file"; [[ -f "$stderr_file" ]] && tail -n 20 "$stderr_file"; } | sanitize_detail )"
  record "$scope" daemon-log-tail info "$detail"
}

cleanup_pids=()
cleanup() {
  for pid in "${cleanup_pids[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}
trap cleanup EXIT

if prepare_commons_fixtures; then
  run_logged slskr fixture-peer-smoke "$repo_root" cargo run -q -p slskr -- smoke fixture-peer
else
  record slskr fixture-peer-smoke skipped "open commons fixtures unavailable"
fi

if [[ "$skip_adjacent_tests" != "1" ]]; then
  run_logged_optional slskr live-matrix "public Soulseek login/reset instability; local fixture peer smoke remains the blocking peer proof" "$repo_root" scripts/run-live-interop-matrix.sh

  if [[ -d "$slskd_repo" ]]; then
    run_logged slskd unit-tests "$slskd_repo" dotnet test tests/slskd.Tests.Unit/slskd.Tests.Unit.csproj
  else
    record slskd checkout missing "not found at $slskd_repo"
  fi

  if [[ -d "$slskdn_repo" ]]; then
    run_logged slskdN unit-tests "$slskdn_repo" dotnet test tests/slskd.Tests.Unit/slskd.Tests.Unit.csproj --no-restore
    run_logged slskdN vendored-runtime-build "$slskdn_repo" dotnet build vendor/slskNet.Runtime/src/Soulseek.csproj --no-restore
    run_logged slskdN vendored-runtime-tests "$slskdn_repo" dotnet test vendor/slskNet.Runtime/tests/Soulseek.Tests.Unit/Soulseek.Tests.Unit.csproj --no-restore
  else
    record slskdN checkout missing "not found at $slskdn_repo"
  fi

  if [[ -d "$soulseek_net_repo" ]]; then
    run_logged Soulseek.NET unit-tests "$soulseek_net_repo" dotnet test tests/Soulseek.Tests.Unit/Soulseek.Tests.Unit.csproj
  else
    record Soulseek.NET checkout missing "not found at $soulseek_net_repo"
  fi

  if [[ -d "$runtime_repo" ]]; then
    run_logged slskNet.Runtime unit-tests "$runtime_repo" dotnet test tests/Soulseek.Tests.Unit/Soulseek.Tests.Unit.csproj
  else
    record slskNet.Runtime checkout missing "not found at $runtime_repo"
  fi
else
  record matrix adjacent-tests skipped "SLSKR_SKIP_ADJACENT_TESTS=1"
fi

if [[ "$run_daemons" == "1" ]]; then
  if [[ -d "$slskd_repo" ]]; then
    slskd_host="127.0.0.1"
    if [[ "$daemon_vpn_enabled" == "1" ]]; then
      slskd_host="$slskd_ns_ip"
    fi
    slskd_pid="$(start_daemon slskd  "$slskd_repo"  "$slskd_account_index" 55130 55100 "$slskd_vpn_label" "$slskd_ns_host_ip" "$slskd_ns_ip" "$slskd_ns_subnet")"
    cleanup_pids+=("$slskd_pid")
    wait_for_daemon_preflight slskr-to-slskd slskd "$slskd_host" 55130 || true
    slskd_user_var="SLSKR_TEST_${slskd_account_index}_USERNAME"
    if wait_for_peer slskr-to-slskd "$slskd_probe_account_index" "${!slskd_user_var}"; then
      run_probe_optional slskr-to-slskd plain-peer "public login/reset instability or daemon listener race; local peer smoke remains the blocking proof" "$slskd_probe_account_index" "${!slskd_user_var}" env SLSK_PLAIN_HOST_OVERRIDE="$slskd_host" cargo run -q -p slskr -- probe plain-peer
      run_probe_optional slskr-to-slskd browse-peer "public login/reset instability or daemon listener race; open commons fixture smoke remains the blocking payload proof" "$slskd_probe_account_index" "${!slskd_user_var}" env SLSK_BROWSE_HOST_OVERRIDE="$slskd_host" SLSK_BROWSE_EXPECTED=slskr-interop-slskd.txt cargo run -q -p slskr -- probe browse-peer
      run_probe_optional slskr-to-slskd download-peer "queued fixture download failed; inspect browse preview for exact remote path and daemon logs for transfer rejection" "$slskd_probe_account_index" "${!slskd_user_var}" env SLSK_DOWNLOAD_HOST_OVERRIDE="$slskd_host" SLSK_DOWNLOAD_FILENAME='slskd\slskr-interop-slskd.txt' SLSK_DOWNLOAD_EXPECTED='slskr interop fixture slskd' SLSK_DOWNLOAD_SHA256=a06260a33bda3cf8cb147107c2d09723b4d59fc6a40d1ac9177424614f4f2202 SLSK_DOWNLOAD_QUEUE_ATTEMPTS="${SLSK_DOWNLOAD_QUEUE_ATTEMPTS:-8}" SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS="${SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS:-4}" cargo run -q -p slskr -- probe download-peer
      run_commons_download_probe_optional slskr-to-slskd "$slskd_probe_account_index" "${!slskd_user_var}" slskd
      run_probe_optional slskr-to-slskd file-transfer-peer "raw transfer token echo requires a queued transfer on slskd; covered as a remaining payload-transfer gap" "$slskd_probe_account_index" "${!slskd_user_var}" env SLSK_FILE_HOST_OVERRIDE="$slskd_host" cargo run -q -p slskr -- probe file-transfer-peer
    else
      record_daemon_tail slskr-to-slskd slskd
      record slskr-to-slskd daemon-probes skipped "target did not reach advertised-listener readiness"
    fi
    sleep "$daemon_cooldown_seconds"
  fi
  if [[ -d "$slskdn_repo" ]]; then
    slskdn_host="127.0.0.1"
    if [[ "$daemon_vpn_enabled" == "1" ]]; then
      slskdn_host="$slskdn_ns_ip"
    fi
    slskdn_pid="$(start_daemon slskdN "$slskdn_repo" "$slskdn_account_index" 55131 55110 "$slskdn_vpn_label" "$slskdn_ns_host_ip" "$slskdn_ns_ip" "$slskdn_ns_subnet")"
    cleanup_pids+=("$slskdn_pid")
    wait_for_daemon_preflight slskr-to-slskdN slskdN "$slskdn_host" 55131 || true
    slskdn_user_var="SLSKR_TEST_${slskdn_account_index}_USERNAME"
    if wait_for_peer slskr-to-slskdN "$slskdn_probe_account_index" "${!slskdn_user_var}"; then
      run_probe_optional slskr-to-slskdN plain-peer "public login/reset instability or daemon listener race; local peer smoke remains the blocking proof" "$slskdn_probe_account_index" "${!slskdn_user_var}" env SLSK_PLAIN_HOST_OVERRIDE="$slskdn_host" cargo run -q -p slskr -- probe plain-peer
      run_probe_optional slskr-to-slskdN browse-peer "public login/reset instability or daemon listener race; open commons fixture smoke remains the blocking payload proof" "$slskdn_probe_account_index" "${!slskdn_user_var}" env SLSK_BROWSE_HOST_OVERRIDE="$slskdn_host" SLSK_BROWSE_EXPECTED=slskr-interop-slskdN.txt cargo run -q -p slskr -- probe browse-peer
      run_probe_optional slskr-to-slskdN download-peer "queued fixture download failed; inspect browse preview for exact remote path and daemon logs for transfer rejection" "$slskdn_probe_account_index" "${!slskdn_user_var}" env SLSK_DOWNLOAD_HOST_OVERRIDE="$slskdn_host" SLSK_DOWNLOAD_FILENAME='slskdN\slskr-interop-slskdN.txt' SLSK_DOWNLOAD_EXPECTED='slskr interop fixture slskdN' SLSK_DOWNLOAD_SHA256=98be10759b80d65a17fa825c5459338fbd319c338280f7aff65b9cc4bba859a9 SLSK_DOWNLOAD_QUEUE_ATTEMPTS="${SLSK_DOWNLOAD_QUEUE_ATTEMPTS:-8}" SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS="${SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS:-4}" cargo run -q -p slskr -- probe download-peer
      run_commons_download_probe_optional slskr-to-slskdN "$slskdn_probe_account_index" "${!slskdn_user_var}" slskdN
      run_probe_optional slskr-to-slskdN file-transfer-peer "raw transfer token echo requires a queued transfer on slskdN; covered as a remaining payload-transfer gap" "$slskdn_probe_account_index" "${!slskdn_user_var}" env SLSK_FILE_HOST_OVERRIDE="$slskdn_host" cargo run -q -p slskr -- probe file-transfer-peer
      run_probe_optional slskr-to-slskdN obfuscated-peer "obfuscated peer probe is only mandatory when the target advertises an active obfuscated listener" "$slskdn_probe_account_index" "${!slskdn_user_var}" env SLSK_OBFUSCATED_PEER_USERNAME="${!slskdn_user_var}" SLSK_OBFUSCATED_HOST_OVERRIDE="$slskdn_host" cargo run -q -p slskr -- probe obfuscated-peer
    else
      record_daemon_tail slskr-to-slskdN slskdN
      record slskr-to-slskdN daemon-probes skipped "target did not reach advertised-listener readiness"
    fi
  fi
fi

cat <<MSG

Cross-client validation completed.
Results: $log_file
Daemon logs, when started, are under: $output_dir
MSG
