#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
env_file="${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
secret_path="${SLSKR_BAO_SECRET_PATH:-kv/slskr/live-interop/test-accounts}"

if [[ ! -f "$env_file" ]]; then
  echo "missing live credential file: $env_file" >&2
  exit 1
fi

if ! command -v bao >/dev/null 2>&1; then
  echo "bao CLI is not installed" >&2
  exit 1
fi

set -a
source "$env_file"
set +a

required=(
  SLSKR_TEST_ACCOUNT_COUNT
  SLSKR_TEST_1_USERNAME SLSKR_TEST_1_PASSWORD
  SLSKR_TEST_2_USERNAME SLSKR_TEST_2_PASSWORD
  SLSKR_TEST_3_USERNAME SLSKR_TEST_3_PASSWORD
  SLSKR_TEST_4_USERNAME SLSKR_TEST_4_PASSWORD
  SLSKR_TEST_ROOM SLSKR_TEST_SEARCH_QUERY
)

for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "missing required env var: $name" >&2
    exit 1
  fi
done

bao kv put "$secret_path" \
  account_count="$SLSKR_TEST_ACCOUNT_COUNT" \
  test_1_username="$SLSKR_TEST_1_USERNAME" \
  test_1_password="$SLSKR_TEST_1_PASSWORD" \
  test_2_username="$SLSKR_TEST_2_USERNAME" \
  test_2_password="$SLSKR_TEST_2_PASSWORD" \
  test_3_username="$SLSKR_TEST_3_USERNAME" \
  test_3_password="$SLSKR_TEST_3_PASSWORD" \
  test_4_username="$SLSKR_TEST_4_USERNAME" \
  test_4_password="$SLSKR_TEST_4_PASSWORD" \
  test_room="$SLSKR_TEST_ROOM" \
  test_search_query="$SLSKR_TEST_SEARCH_QUERY"

echo "stored live interop credentials at $secret_path"
