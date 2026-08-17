#!/usr/bin/env bash

set -euo pipefail

version="${1:-}"
notes_path="${2:-}"

fail() {
  echo "[validate-release-notes] ERROR: $*" >&2
  exit 1
}

[[ -n "$version" && -n "$notes_path" ]] || fail "usage: $0 <version> <notes-path>"
[[ -s "$notes_path" ]] || fail "release notes are missing or empty: $notes_path"

grep -Fqx "# slskr $version" "$notes_path" || fail "notes title does not match $version"
grep -Fqx '## Highlights' "$notes_path" || fail "notes do not contain a Highlights section"
grep -Eq '^[[:space:]]*-[[:space:]]+\S' "$notes_path" || fail "release notes contain no highlight bullets"

if grep -Eqi 'TODO|TBD|placeholder|Add release notes here|No recorded changes' "$notes_path"; then
  fail "notes contain placeholder or empty-release wording"
fi

meaningful_chars="$(tr -d '[:space:]' <"$notes_path" | wc -c | tr -d ' ')"
(( meaningful_chars >= 80 )) || fail "release notes are too short to be informative (${meaningful_chars} non-space characters)"

echo "Release notes validated for $version."
