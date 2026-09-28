#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if rg -n '^#!\[allow\([^]]*dead_code' crates --glob '!target/**'; then
  printf 'rust module hygiene failed: crate/module-level dead_code allowances are not allowed\n' >&2
  status=1
fi

for file in crates/slskr/src/lib.rs crates/slskr/src/webhooks.rs crates/slskr/src/routing.rs; do
  if rg -n '^#!\[allow\([^]]*dead_code' "$file" >/dev/null; then
    printf 'rust module hygiene failed: broad dead_code allow remains in %s\n' "$file" >&2
    status=1
  fi
done

if ! rg -n '^\| BUG-025 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'rust module hygiene failed: BUG-025 must stay verified in council ledger\n' >&2
  status=1
fi

# Controller contract suites must retain real module ownership after RF-024.
if ! python3 - <<'PY_CHECK'
from pathlib import Path
import re

root = Path("crates/slskr/src")
registry = root / "controller_tests.rs"
violations = []
if re.search(r"\binclude!\s*\(", registry.read_text()):
    violations.append(f"{registry}: flat test includes are forbidden")
for file in (root / "controller_tests").glob("*.rs"):
    lines = len(file.read_bytes().splitlines())
    if lines > 2500:
        violations.append(f"{file}: {lines} lines exceed the 2500-line owner budget")
if violations:
    raise SystemExit("\n".join(violations))
print("controller test ownership check passed")
PY_CHECK
then
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'rust module hygiene check passed\n'
