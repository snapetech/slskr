#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/cross-client-validation-result.sh"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

cat >"$temp_dir/optional.tsv" <<'EOF'
timestamp	scope	check	status	detail
2026-10-02T00:00:00Z	web	optional-probe	non-blocking	optional remote probe; connection refused
2026-10-02T00:00:02Z	web	daemon-preflight	timeout	peer listener check is authoritative
EOF
if ! "$helper" "$temp_dir/optional.tsv"; then
  printf 'cross-client result regression failed: optional findings blocked success\n' >&2
  exit 1
fi

cat >"$temp_dir/required.tsv" <<'EOF'
timestamp	scope	check	status	detail
2026-10-02T00:00:00Z	web	peer-address-timeout	fail	peer did not advertise an address
EOF
if "$helper" "$temp_dir/required.tsv" >/dev/null 2>&1; then
  printf 'cross-client result regression failed: required failure was accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 1 ]]; then
    printf 'cross-client result regression failed: required failure returned %s, expected 1\n' "$status" >&2
    exit 1
  fi
fi

cat >"$temp_dir/failed-command.tsv" <<'EOF'
timestamp	scope	check	status	detail
2026-10-02T00:00:00Z	web	mandatory-command	fail(7)	command failed
EOF
if "$helper" "$temp_dir/failed-command.tsv" >/dev/null 2>&1; then
  printf 'cross-client result regression failed: nonzero required command was accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 1 ]]; then
    printf 'cross-client result regression failed: nonzero required command returned %s, expected 1\n' "$status" >&2
    exit 1
  fi
fi

: >"$temp_dir/empty.tsv"
if "$helper" "$temp_dir/empty.tsv" >/dev/null 2>&1; then
  printf 'cross-client result regression failed: empty result log was accepted\n' >&2
  exit 1
else
  status=$?
  if [[ "$status" -ne 2 ]]; then
    printf 'cross-client result regression failed: empty result log returned %s, expected 2\n' "$status" >&2
    exit 1
  fi
fi

printf 'cross-client result aggregation regression passed\n'
