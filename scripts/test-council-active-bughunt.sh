#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

mkdir -p "$temp_dir/bin"
cat >"$temp_dir/bin/rg" <<'EOF'
#!/usr/bin/env bash
exit "${TEST_RG_EXIT_CODE:-2}"
EOF
chmod +x "$temp_dir/bin/rg"

if PATH="$temp_dir/bin:$PATH" TEST_RG_EXIT_CODE=2 \
  COUNCIL_OUT_DIR="$temp_dir/error-output" \
  "$repo_root/scripts/run-council-active-bughunt.sh" \
  >/dev/null 2>&1; then
  printf 'active bughunt regression failed: matcher errors were accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 2 ]]; then
    printf 'active bughunt regression failed: matcher error returned %s, expected 2\n' "$status" >&2
    exit 1
  fi
fi

PATH="$temp_dir/bin:$PATH" TEST_RG_EXIT_CODE=1 \
  COUNCIL_OUT_DIR="$temp_dir/empty-output" \
  "$repo_root/scripts/run-council-active-bughunt.sh" \
  >/dev/null

section_count="$(grep -c '^## ' "$temp_dir/empty-output/active-bughunt.md")"
if [[ "$section_count" -ne 8 ]]; then
  printf 'active bughunt regression failed: zero-match report has %s sections, expected 8\n' "$section_count" >&2
  exit 1
fi

printf 'active bughunt matcher error/zero-match regression passed\n'
