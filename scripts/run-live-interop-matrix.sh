#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
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

live_vpn_enabled="${SLSKR_LIVE_VPN_ENABLED:-0}"
live_slsk_address="${SLSKR_LIVE_SLSK_ADDRESS:-}"
live_slsk_port="${SLSKR_LIVE_SLSK_PORT:-2271}"
private_message_sender_index="${SLSKR_PRIVATE_MESSAGE_SENDER_INDEX:-5}"
private_message_receiver_index="${SLSKR_PRIVATE_MESSAGE_RECEIVER_INDEX:-6}"
room_message_account_index="${SLSKR_ROOM_MESSAGE_ACCOUNT_INDEX:-6}"
local_peer_a_index="${SLSKR_LOCAL_PEER_A_INDEX:-5}"
local_peer_b_index="${SLSKR_LOCAL_PEER_B_INDEX:-6}"
if [[ -n "${SLSKR_LIVE_LOGIN_INDEXES:-}" ]]; then
  live_login_indexes="$SLSKR_LIVE_LOGIN_INDEXES"
elif [[ "$live_vpn_enabled" == "1" ]]; then
  live_login_indexes="5 6 7 8"
else
  live_login_indexes="1 2 3 4"
fi
if [[ -z "$live_slsk_address" ]]; then
  live_slsk_address="$(getent ahostsv4 vps.slsknet.org | awk 'NR == 1 { print $1 }' || true)"
fi

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

run_sanitized() {
  local stdout_file="$1"
  local stderr_file="$2"
  shift 2
  "$@" >"$stdout_file" 2>"$stderr_file"
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

run_live_command() {
  local index="$1"
  shift
  local command=(env CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}")
  if [[ -n "$live_slsk_address" ]]; then
    command+=(SLSK_SERVER="$live_slsk_address:$live_slsk_port")
  fi
  command+=("$@")

  if [[ "$live_vpn_enabled" == "1" ]]; then
    local label_var="SLSKR_LIVE_VPN_LABEL_${index}"
    local label="${!label_var:-p${index}}"
    local config
    config="$(resolve_proton_config "$label")" || return 1
    command=(
      env
      SLSKR_NETNS_HOST_IP="${SLSKR_LIVE_NETNS_HOST_PREFIX:-10.246}.${index}.1"
      SLSKR_NETNS_IP="${SLSKR_LIVE_NETNS_HOST_PREFIX:-10.246}.${index}.2"
      SLSKR_NETNS_SUBNET="${SLSKR_LIVE_NETNS_HOST_PREFIX:-10.246}.${index}.0/24"
      "$repo_root/scripts/run-in-proton-wg-netns.sh"
      "lv${index}"
      "$config"
      "${command[@]}"
    )
  fi

  "${command[@]}"
}

summarize_output() {
  local stdout_file="$1"
  local stderr_file="$2"
  {
    cat "$stdout_file"
    tail -n 20 "$stderr_file" || true
  } | tr '\n\t' '  ' | sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//'
}

log_file="$output_dir/slskr-login-smoke.tsv"
printf 'timestamp\taccount\tcheck\tstatus\tdetail\n' >"$log_file"

run_account_login() {
  local index="$1"
  local user_var="SLSKR_TEST_${index}_USERNAME"
  local pass_var="SLSKR_TEST_${index}_PASSWORD"
  local username="${!user_var}"
  local password="${!pass_var}"
  local stdout_file stderr_file detail status
  stdout_file="$(mktemp)"
  stderr_file="$(mktemp)"

  set +e
  SLSK_USERNAME="$username" \
  SLSK_PASSWORD="$password" \
    run_sanitized "$stdout_file" "$stderr_file" run_live_command "$index" cargo run -q -p slskr -- login smoke
  status=$?
  set -e

  detail="$(summarize_output "$stdout_file" "$stderr_file")"
  rm -f "$stdout_file" "$stderr_file"

  if [[ $status -eq 0 ]]; then
    printf '%s\t%s\tlogin-smoke\tok\t%s\n' "$(date -Is)" "$username" "$detail" | tee -a "$log_file"
  else
    printf '%s\t%s\tlogin-smoke\tfail(%s)\t%s\n' "$(date -Is)" "$username" "$status" "$detail" | tee -a "$log_file"
    return "$status"
  fi
}

for i in $live_login_indexes; do
  run_account_login "$i"
done

pair_log="$output_dir/slskr-local-peer-smoke.tsv"
printf 'timestamp\tcheck\tstatus\tdetail\n' >"$pair_log"
pair_stdout="$(mktemp)"
pair_stderr="$(mktemp)"
local_peer_a_user_var="SLSKR_TEST_${local_peer_a_index}_USERNAME"
local_peer_a_pass_var="SLSKR_TEST_${local_peer_a_index}_PASSWORD"
local_peer_b_user_var="SLSKR_TEST_${local_peer_b_index}_USERNAME"
local_peer_b_pass_var="SLSKR_TEST_${local_peer_b_index}_PASSWORD"
set +e
SLSKR_A_USERNAME="${!local_peer_a_user_var}" \
SLSKR_A_PASSWORD="${!local_peer_a_pass_var}" \
SLSKR_B_USERNAME="${!local_peer_b_user_var}" \
SLSKR_B_PASSWORD="${!local_peer_b_pass_var}" \
SLSKR_INDIRECT_HOST_OVERRIDE="${SLSKR_INDIRECT_HOST_OVERRIDE:-127.0.0.1}" \
  run_sanitized "$pair_stdout" "$pair_stderr" run_live_command "$local_peer_a_index" cargo run -q -p slskr -- smoke local-peer
pair_status=$?
set -e
pair_detail="$(summarize_output "$pair_stdout" "$pair_stderr")"
rm -f "$pair_stdout" "$pair_stderr"
if [[ $pair_status -eq 0 ]]; then
  printf '%s\tlocal-peer-smoke\tok\t%s\n' "$(date -Is)" "$pair_detail" | tee -a "$pair_log"
else
  printf '%s\tlocal-peer-smoke\tfail(%s)\t%s\n' "$(date -Is)" "$pair_status" "$pair_detail" | tee -a "$pair_log"
  exit "$pair_status"
fi

social_log="$output_dir/slskr-social-smoke.tsv"
printf 'timestamp\tcheck\tstatus\tdetail\n' >"$social_log"
private_sender_user_var="SLSKR_TEST_${private_message_sender_index}_USERNAME"
private_sender_pass_var="SLSKR_TEST_${private_message_sender_index}_PASSWORD"
private_receiver_user_var="SLSKR_TEST_${private_message_receiver_index}_USERNAME"
private_receiver_pass_var="SLSKR_TEST_${private_message_receiver_index}_PASSWORD"
social_stdout="$(mktemp)"
social_stderr="$(mktemp)"
set +e
SLSK_USERNAME="${!private_sender_user_var}" \
SLSK_PASSWORD="${!private_sender_pass_var}" \
SLSK_MESSAGE_USERNAME="${!private_receiver_user_var}" \
SLSK_MESSAGE_PASSWORD="${!private_receiver_pass_var}" \
  run_sanitized "$social_stdout" "$social_stderr" run_live_command "$private_message_sender_index" cargo run -q -p slskr -- probe private-message
social_status=$?
set -e
social_detail="$(summarize_output "$social_stdout" "$social_stderr")"
rm -f "$social_stdout" "$social_stderr"
if [[ $social_status -eq 0 ]]; then
  printf '%s\tprivate-message\tok\t%s\n' "$(date -Is)" "$social_detail" | tee -a "$social_log"
else
  printf '%s\tprivate-message\tfail(%s)\t%s\n' "$(date -Is)" "$social_status" "$social_detail" | tee -a "$social_log"
  exit "$social_status"
fi

room_stdout="$(mktemp)"
room_stderr="$(mktemp)"
room_user_var="SLSKR_TEST_${room_message_account_index}_USERNAME"
room_pass_var="SLSKR_TEST_${room_message_account_index}_PASSWORD"
set +e
SLSK_USERNAME="${!room_user_var}" \
SLSK_PASSWORD="${!room_pass_var}" \
  run_sanitized "$room_stdout" "$room_stderr" run_live_command "$room_message_account_index" cargo run -q -p slskr -- probe room-message
room_status=$?
set -e
room_detail="$(summarize_output "$room_stdout" "$room_stderr")"
rm -f "$room_stdout" "$room_stderr"
if [[ $room_status -eq 0 ]]; then
  printf '%s\troom-message\tok\t%s\n' "$(date -Is)" "$room_detail" | tee -a "$social_log"
else
  printf '%s\troom-message\tfail(%s)\t%s\n' "$(date -Is)" "$room_status" "$room_detail" | tee -a "$social_log"
  exit "$room_status"
fi

cat <<MSG

Initial slskr live interop checks completed.
Results:
- $log_file
- $pair_log
- $social_log

Cross-client adapters are tracked in docs/live-interop-test-matrix.md.
MSG
