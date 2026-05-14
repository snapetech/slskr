#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
runner="$repo_root/scripts/run-bug-council-all-phases.sh"
failed=0

require_literal() {
  local literal="$1"
  local file="$2"

  if ! rg -q --fixed-strings "$literal" "$file"; then
    printf '%s is missing required council all-phases marker: %s\n' "${file#"$repo_root"/}" "$literal" >&2
    failed=1
  fi
}

assert_phase_done() {
  local phase_name="$1"

  if ! awk -F'|' -v phase="$phase_name" '
    $2 ~ /^[[:space:]]*[0-9]+[[:space:]]*$/ && $3 ~ phase {
      status = $4
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", status)
      found = 1
      if (status != "Done") {
        exit 2
      }
    }
    END {
      if (!found) {
        exit 3
      }
    }
  ' "$repo_root/docs/dev/bug-council-phases.md"; then
    printf 'Council phase is not Done or is missing: %s\n' "$phase_name" >&2
    failed=1
  fi
}

if [ ! -x "$runner" ]; then
  printf 'Council all-phases runner is missing or not executable: %s\n' "${runner#"$repo_root"/}" >&2
  exit 1
fi

require_literal "run-council-scan.sh" "$runner"
require_literal "check-council-active-backlog.sh" "$runner"
require_literal "check-council-inventory-closure.sh" "$runner"
require_literal "check-council-loop.sh" "$runner"
require_literal "check-council-negative-space.sh" "$runner"
require_literal "check-rust-protocol-taint-lens.sh" "$runner"
require_literal "check-rust-protocol-adversarial-corpus.sh" "$runner"
require_literal "Pending" "$runner"

require_literal 'scripts/check-bug-council-all-phases.sh' "$repo_root/scripts/check-remediation-baseline.sh"
require_literal "bug-council-active-backlog.md" "$repo_root/scripts/check-remediation-baseline.sh"
require_literal "check-council-inventory-closure.sh" "$repo_root/scripts/check-remediation-baseline.sh"

assert_phase_done "Mirror council process docs"
assert_phase_done "Severity/confidence retrofit"
assert_phase_done "Rust semantic-lens"
assert_phase_done "Rust loop-bound"
assert_phase_done "Multi-seed adversarial"
assert_phase_done "Council bughunt entrypoint"
assert_phase_done "All-phases council runner"
assert_phase_done "Active backlog"
assert_phase_done "Inventory closure"

if [ "$failed" -ne 0 ]; then
  exit 1
fi

printf 'Bug council all-phases runner is registered.\n'
