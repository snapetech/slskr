#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

mkdir -p "$temp_dir/bin" "$temp_dir/state"
cat >"$temp_dir/bin/grep" <<'EOF'
#!/usr/bin/env bash
exit "${TEST_GREP_EXIT_CODE:-2}"
EOF
chmod +x "$temp_dir/bin/grep"

pool_file="$temp_dir/pool.env"
output_file="$temp_dir/state/accounts.env"
original_file="$temp_dir/state/accounts.original"
: >"$pool_file"
cat >"$output_file" <<'EOF'
SAFE_SETTING=preserve-me
SLSKR_TEST_ACCOUNT_COUNT=7
SLSKR_TEST_7_USERNAME=old-user
SLSKR_TEST_7_PASSWORD=old-password
EOF
chmod 600 "$output_file"
cp "$output_file" "$original_file"

if PATH="$temp_dir/bin:$PATH" TEST_GREP_EXIT_CODE=2 \
  SLSKR_PROTON_CREDENTIAL_POOL_FILE="$pool_file" \
  SLSKR_GENERATED_ACCOUNTS_FILE="$output_file" \
  SLSKR_ACCOUNT_GENERATOR_LABELS=' ' \
  SLSK_SERVER=127.0.0.1:2242 \
  "$repo_root/scripts/generate-vpn-soulseek-accounts.sh" \
  >"$temp_dir/read-error-output" 2>&1; then
  printf 'generated account safety regression failed: unreadable source was accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 2 ]]; then
    printf 'generated account safety regression failed: read error returned %s, expected 2\n' "$status" >&2
    exit 1
  fi
fi
if ! cmp -s "$original_file" "$output_file"; then
  printf 'generated account safety regression failed: read error changed the existing file\n' >&2
  exit 1
fi

PATH="$PATH" \
  SLSKR_PROTON_CREDENTIAL_POOL_FILE="$pool_file" \
  SLSKR_GENERATED_ACCOUNTS_FILE="$output_file" \
  SLSKR_ACCOUNT_GENERATOR_LABELS=' ' \
  SLSK_SERVER=127.0.0.1:2242 \
  "$repo_root/scripts/generate-vpn-soulseek-accounts.sh" \
  >"$temp_dir/success-output"
if [[ "$(stat -c '%a' "$output_file" 2>/dev/null || stat -f '%Lp' "$output_file")" != "600" ]]; then
  printf 'generated account safety regression failed: output permissions are not 600\n' >&2
  exit 1
fi
if ! grep -Fxq 'SLSKR_TEST_ACCOUNT_COUNT=4' "$output_file" \
  || ! grep -Fxq 'SAFE_SETTING=preserve-me' "$output_file"; then
  printf 'generated account safety regression failed: successful regeneration lost retained data\n' >&2
  exit 1
fi

target_file="$temp_dir/state/symlink-target.env"
linked_output="$temp_dir/state/linked-accounts.env"
printf 'DO_NOT_REPLACE=yes\n' >"$target_file"
ln -s "$target_file" "$linked_output"
if SLSKR_PROTON_CREDENTIAL_POOL_FILE="$pool_file" \
  SLSKR_GENERATED_ACCOUNTS_FILE="$linked_output" \
  SLSKR_ACCOUNT_GENERATOR_LABELS=' ' \
  SLSK_SERVER=127.0.0.1:2242 \
  "$repo_root/scripts/generate-vpn-soulseek-accounts.sh" \
  >"$temp_dir/symlink-output" 2>&1; then
  printf 'generated account safety regression failed: symlink output was accepted\n' >&2
  exit 1
fi
if [[ "$(cat "$target_file")" != "DO_NOT_REPLACE=yes" ]]; then
  printf 'generated account safety regression failed: symlink target changed\n' >&2
  exit 1
fi

printf 'generated account read and symlink protections passed\n'
