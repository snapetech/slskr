#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="${TMPDIR:-/tmp}"
tmp_dir="$(mktemp -d "$tmp_root/slskr-live-interop-account-validation.XXXXXX")"
chmod 700 "$tmp_dir"
trap 'rm -rf -- "$tmp_dir"' EXIT

mkdir -p "$tmp_dir/bin"
cat >"$tmp_dir/bin/getent" <<'GETENT'
#!/usr/bin/env bash
: >"$SLSKR_TEST_GETENT_MARKER"
exit 0
GETENT
chmod 700 "$tmp_dir/bin/getent"

write_accounts() {
  local destination="$1"
  local account_count="$2"
  local count_setting="$3"
  {
    if [[ "$count_setting" != "default" ]]; then
      printf 'SLSKR_TEST_ACCOUNT_COUNT=%s\n' "$count_setting"
    fi
    for index in $(seq 1 "$account_count"); do
      printf 'SLSKR_TEST_%s_USERNAME=test-user-%s\n' "$index" "$index"
      printf 'SLSKR_TEST_%s_PASSWORD=test-password-%s\n' "$index" "$index"
    done
  } >"$destination"
  chmod 600 "$destination"
}

run_missing_account_case() {
  local name="$1"
  local account_count="$2"
  local count_setting="$3"
  local vpn_enabled="$4"
  local expected_error="$5"
  local env_file="$tmp_dir/$name.env"
  local stderr_file="$tmp_dir/$name.stderr"
  local stdout_file="$tmp_dir/$name.stdout"
  local marker="$tmp_dir/$name.getent-called"

  write_accounts "$env_file" "$account_count" "$count_setting"
  if env \
    PATH="$tmp_dir/bin:$PATH" \
    SLSKR_LIVE_ENV_FILE="$env_file" \
    SLSKR_LIVE_EXTRA_ENV_FILE="$tmp_dir/no-extra-env" \
    SLSKR_PROTON_CREDENTIAL_POOL_FILE="$tmp_dir/no-pool-env" \
    SLSKR_INTEROP_OUTPUT_DIR="$tmp_dir/$name-output" \
    SLSKR_TEST_GETENT_MARKER="$marker" \
    SLSKR_LIVE_VPN_ENABLED="$vpn_enabled" \
    bash "$repo_root/scripts/run-live-interop-matrix.sh" \
    >"$stdout_file" 2>"$stderr_file"; then
    echo "$name unexpectedly accepted incomplete account credentials" >&2
    exit 1
  fi

  if ! grep -Fq "$expected_error" "$stderr_file"; then
    echo "$name returned an unexpected validation error" >&2
    cat "$stderr_file" >&2
    exit 1
  fi
  if [[ -e "$marker" ]]; then
    echo "$name resolved the public Soulseek host before validating credentials" >&2
    exit 1
  fi
}

run_missing_account_case default-matrix 4 4 0 \
  'missing required env var: SLSKR_TEST_5_USERNAME'
run_missing_account_case default-count 5 default 0 \
  'missing required env var: SLSKR_TEST_6_USERNAME'
run_missing_account_case vpn-matrix 6 6 1 \
  'missing required env var: SLSKR_TEST_7_USERNAME'

echo "live interop account validation tests passed"
