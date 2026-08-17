#!/usr/bin/env bash

set -euo pipefail

changelog="${CHANGELOG_PATH:-CHANGELOG.md}"
version="${1:-}"

fail() {
  echo "[validate-changelog] ERROR: $*" >&2
  exit 1
}

[[ -f "$changelog" ]] || fail "changelog not found: $changelog"

unreleased_count="$(rg -c --no-filename '^## \[Unreleased\]$' "$changelog" || true)"
(( unreleased_count == 1 )) || fail "$changelog must contain exactly one ## [Unreleased] section"

if grep -Eqi 'TODO|TBD|Add release notes here|placeholder' "$changelog"; then
  fail "$changelog contains placeholder wording"
fi

unreleased_section="$(awk '
  /^## \[Unreleased\]$/ { in_section = 1; next }
  in_section && /^## \[/ { exit }
  in_section { print }
' "$changelog")"

has_bullet() {
  grep -Eq '^[[:space:]]*-[[:space:]]+\S' <<<"$1"
}

latest_version_section() {
  awk '
    /^## \[[^]]+\] — [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ {
      if (found) exit
      found = 1
      next
    }
    found { print }
  ' "$changelog"
}

if [[ -z "$version" ]]; then
  if ! has_bullet "$unreleased_section" && ! has_bullet "$(latest_version_section)"; then
    fail "$changelog needs a meaningful bullet under ## [Unreleased] or its latest dated release section"
  fi
  echo "Changelog structure and release detail validated."
  exit 0
fi

section="$(awk -v version="$version" '
  index($0, "## [" version "] — ") == 1 && substr($0, length("## [" version "] — ") + 1) ~ /^[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ {
    in_section = 1
    next
  }
  in_section && /^## \[/ { exit }
  in_section { print }
' "$changelog")"

if ! has_bullet "$section"; then
  section="$unreleased_section"
fi
if ! has_bullet "$section"; then
  section="$(latest_version_section)"
fi
has_bullet "$section" || fail "$changelog needs a meaningful section for $version or a fallback release section"

printf '%s\n' "$section" | grep -Eq '^[[:space:]]*-[[:space:]]+\S' \
  || fail "the $version changelog section must contain a meaningful bullet"
printf '%s\n' "$section" | grep -Eqi 'TODO|TBD|placeholder|Add release notes here' \
  && fail "the $version changelog section contains placeholder wording"

echo "Changelog validated for $version."
