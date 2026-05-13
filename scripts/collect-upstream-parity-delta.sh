#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
since="${1:-2026-04-30}"
slskdn="${SLSKR_SLSKDN_REPO:-$repo_root/../slskdn}"
runtime="${SLSKR_SLSKNET_RUNTIME_REPO:-$repo_root/../slskNet.Runtime}"

print_repo() {
  local label="$1"
  local path="$2"

  if [[ ! -d "$path/.git" ]]; then
    printf '## %s\n\nMissing repository: %s\n\n' "$label" "$path"
    return
  fi

  printf '## %s\n\n' "$label"
  printf 'Repository: `%s`\n\n' "$path"
  printf '| Bucket | Commit | Subject |\n'
  printf '| --- | --- | --- |\n'

  git -C "$path" log --since="$since" --date=short --format='%H%x09%s' |
    while IFS=$'\t' read -r commit subject; do
      [[ -n "$commit" ]] || continue
      bucket="docs"
      case "$subject" in
        *runtime*|*Runtime*|*protocol*|*Protocol*|*obfuscat*|*capabilit*|*wishlist*)
          bucket="runtime/protocol"
          ;;
        *security*|*Harden*|*harden*|*bind*|*auth*|*CSRF*|*token*|*validation*)
          bucket="security"
          ;;
        *transfer*|*download*|*upload*|*room*|*message*|*search*|*browse*)
          bucket="app-behavior"
          ;;
        *web*|*UI*|*render*|*poll*|*page*)
          bucket="ui"
          ;;
        *release*|*packag*|*Docker*|*snap*|*PPA*|*dependency*|*deps*|*ci*|*workflow*)
          bucket="release/ci"
          ;;
      esac
      printf '| %s | `%s` | %s |\n' "$bucket" "${commit:0:10}" "$subject"
    done
  printf '\n'
}

cat <<HEADER
# Upstream Parity Delta

Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Since: $since

This report groups upstream commits for parity triage. Classify each row in
docs/parity/slskdn-slsknet-runtime-parity.md before closing the related work.

HEADER

print_repo "slskdN" "$slskdn"
print_repo "slskNet.Runtime" "$runtime"
