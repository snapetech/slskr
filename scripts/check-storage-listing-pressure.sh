#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

if ! rg -n 'SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES|SLSKD_STORAGE_RECURSIVE_LIST_MAX_ENTRIES' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: recursive storage listing budget constants are missing\n' >&2
  status=1
fi

if ! rg -n 'SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES|reserve_storage_scan_entry' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: per-directory scan budget is missing\n' >&2
  status=1
fi

if ! rg -n 'SLSKD_STORAGE_MAX_RECURSION_DEPTH|slskd_recursive_storage_listing_bounds_directory_depth' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: recursive depth budget is missing\n' >&2
  status=1
fi

if ! rg -n 'StorageDirectoryListOptions|limit.*offset|from_query' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: storage listing pagination options are missing\n' >&2
  status=1
fi

if ! rg -n 'truncated|entryCount' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: storage listing response metadata is missing\n' >&2
  status=1
fi

if ! rg -n 'slskd_recursive_storage_listing_has_lower_budget|slskd_storage_directory_routes_support_pagination' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: recursive budget and pagination regression tests are missing\n' >&2
  status=1
fi

if ! rg -n 'check_rate_limit\(rate_limit_remote_addr, username\)' crates/slskr/src/main.rs >/dev/null; then
  printf 'storage listing pressure check failed: HTTP route rate-limit coverage is missing\n' >&2
  status=1
fi

if ! rg -n 'BUG-009 .*Verified' docs/dev/bug-burndown-ledger.md >/dev/null; then
  printf 'storage listing pressure check failed: BUG-009 must be marked verified in the council ledger\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'storage listing pressure check passed\n'
