#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

mkdir -p "$temp_dir/bin"
cat >"$temp_dir/bin/rg" <<'EOF'
#!/usr/bin/env bash
exit "$TEST_RG_EXIT_CODE"
EOF
chmod +x "$temp_dir/bin/rg"

if PATH="$temp_dir/bin:$PATH" TEST_RG_EXIT_CODE=2 \
  "$repo_root/scripts/run-council-scan.sh" \
  >"$temp_dir/error-output" 2>&1; then
  printf 'council scan regression failed: matcher errors were accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 2 ]]; then
    printf 'council scan regression failed: matcher error returned %s, expected 2\n' "$status" >&2
    exit 1
  fi
fi

PATH="$temp_dir/bin:$PATH" TEST_RG_EXIT_CODE=1 \
  "$repo_root/scripts/run-council-scan.sh" \
  >"$temp_dir/empty-output"

zero_count="$(grep -cE '^\| .+ \| 0 \|$' "$temp_dir/empty-output")"
if [[ "$zero_count" -ne 6 ]]; then
  printf 'council scan regression failed: zero-match output has %s zero-count rows, expected 6\n' "$zero_count" >&2
  exit 1
fi

if PATH="$temp_dir/bin:$PATH" TEST_RG_EXIT_CODE=2 \
  "$repo_root/scripts/scan-bug-council-candidates.sh" \
  >"$temp_dir/legacy-error-output" 2>&1; then
  printf 'legacy council scan regression failed: matcher errors were accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 2 ]]; then
    printf 'legacy council scan regression failed: matcher error returned %s, expected 2\n' "$status" >&2
    exit 1
  fi
fi

PATH="$temp_dir/bin:$PATH" TEST_RG_EXIT_CODE=1 \
  "$repo_root/scripts/scan-bug-council-candidates.sh" \
  >"$temp_dir/legacy-empty-output"
legacy_section_count="$(grep -c '^## ' "$temp_dir/legacy-empty-output")"
if [[ "$legacy_section_count" -ne 9 ]]; then
  printf 'legacy council scan regression failed: zero-match output has %s sections, expected 9\n' "$legacy_section_count" >&2
  exit 1
fi

for checker in check-csp-policy check-public-posture; do
  if PATH="$temp_dir/bin:$PATH" \
    "$repo_root/scripts/${checker}.sh" \
    >"$temp_dir/${checker}-error-output" 2>&1; then
    printf '%s regression failed: matcher errors were accepted\n' "$checker" >&2
    exit 1
  else
    status=$?
    if [[ "$status" -ne 2 ]]; then
      printf '%s regression failed: matcher error returned %s, expected 2\n' "$checker" "$status" >&2
      exit 1
    fi
  fi
done

if PATH="$temp_dir/bin:$PATH" LOCAL_IDENTITY_DENYLIST=fixture-token \
  "$repo_root/scripts/check-local-identity-leaks.sh" \
  >"$temp_dir/identity-error-output" 2>&1; then
  printf 'local identity scan regression failed: matcher errors were accepted\n' >&2
  exit 1
fi
if ! grep -q 'identity scan failed' "$temp_dir/identity-error-output"; then
  printf 'local identity scan regression failed: matcher error was not reported\n' >&2
  exit 1
fi

printf 'council and policy scan matcher error/zero-match regressions passed\n'
