#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
env_file="${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
extra_env_file="${SLSKR_LIVE_EXTRA_ENV_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
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

run_sanitized() {
  local stdout_file="$1"
  local stderr_file="$2"
  shift 2
  "$@" >"$stdout_file" 2>"$stderr_file"
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
  run_sanitized "$stdout_file" "$stderr_file" env SLSK_USERNAME="$username" SLSK_PASSWORD="$password" cargo run -q -p slskr -- login smoke
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

for i in 1 2 3 4; do
  run_account_login "$i"
done

pair_log="$output_dir/slskr-local-peer-smoke.tsv"
printf 'timestamp\tcheck\tstatus\tdetail\n' >"$pair_log"
pair_stdout="$(mktemp)"
pair_stderr="$(mktemp)"
set +e
run_sanitized "$pair_stdout" "$pair_stderr" env SLSKR_INDIRECT_HOST_OVERRIDE="${SLSKR_INDIRECT_HOST_OVERRIDE:-127.0.0.1}" cargo run -q -p slskr -- smoke local-peer
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
social_stdout="$(mktemp)"
social_stderr="$(mktemp)"
set +e
run_sanitized "$social_stdout" "$social_stderr" env \
  SLSK_USERNAME="$SLSKR_TEST_1_USERNAME" \
  SLSK_PASSWORD="$SLSKR_TEST_1_PASSWORD" \
  SLSK_MESSAGE_USERNAME="$SLSKR_TEST_2_USERNAME" \
  SLSK_MESSAGE_PASSWORD="$SLSKR_TEST_2_PASSWORD" \
  cargo run -q -p slskr -- probe private-message
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
set +e
run_sanitized "$room_stdout" "$room_stderr" env \
  SLSK_USERNAME="$SLSKR_TEST_1_USERNAME" \
  SLSK_PASSWORD="$SLSKR_TEST_1_PASSWORD" \
  cargo run -q -p slskr -- probe room-message
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
