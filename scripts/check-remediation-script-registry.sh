#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

baseline="scripts/check-remediation-baseline.sh"
status=0

is_standalone_check() {
  case "$1" in
    scripts/check-proton-wg-labels.sh | scripts/check-public-posture.sh | scripts/check-release-package.sh)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

while IFS= read -r -d '' gate; do
  gate="${gate#./}"
  if [[ ! -x "$gate" ]]; then
    printf 'remediation registry failed: %s is not executable\n' "$gate" >&2
    status=1
  fi

  if [[ "$gate" != "$baseline" ]] && ! is_standalone_check "$gate" && ! grep -Fq "  $gate" "$baseline"; then
    printf 'remediation registry failed: %s is not registered in %s\n' "$gate" "$baseline" >&2
    status=1
  fi
done < <(find scripts -maxdepth 1 -type f -name 'check-*.sh' -print0 | sort -z)

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'remediation script registry passed\n'
