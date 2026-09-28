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

# Contract suites must retain real module ownership after RF-024.
if ! python3 - <<'PY_CHECK'
from pathlib import Path
import re

root = Path("crates/slskr/src")
violations = []
for suite, budget in [("controller_tests", 2500), ("config_tests", 1200)]:
    registry = root / f"{suite}.rs"
    if re.search(r"\binclude!\s*\(", registry.read_text()):
        violations.append(f"{registry}: flat test includes are forbidden")
    for file in (root / suite).glob("*.rs"):
        lines = len(file.read_bytes().splitlines())
        if lines > budget:
            violations.append(f"{file}: {lines} lines exceed the {budget}-line owner budget")
for directory in ["config_parts", "config_file_parts"]:
    for file in (root / directory).glob("*.rs"):
        lines = len(file.read_bytes().splitlines())
        if lines > 1200:
            violations.append(f"{file}: {lines} lines exceed the 1200-line configuration owner budget")
for name, budget in [("config.rs", 400), ("config_file.rs", 300)]:
    registry = root / name
    if len(registry.read_bytes().splitlines()) > budget:
        violations.append(f"{name}: aggregate and registry exceed the {budget}-line budget")
    if re.search(r"\binclude!\s*\(", registry.read_text()):
        violations.append(f"{name}: flat configuration includes are forbidden")
web_root = Path("crates/slskr-web/src")
web_registry = web_root / "web_tests.rs"
if len(web_registry.read_bytes().splitlines()) > 100:
    violations.append("web_tests.rs: registry exceeds the 100-line budget")
if re.search(r"\binclude!\s*\(", web_registry.read_text()):
    violations.append("web_tests.rs: flat test includes are forbidden")
if 'include!("web_tests.rs")' in (web_root / "lib.rs").read_text():
    violations.append("slskr-web/lib.rs: tests must use a real module")
for file in (web_root / "web_tests").glob("*.rs"):
    lines = len(file.read_bytes().splitlines())
    if lines > 1000:
        violations.append(f"{file}: {lines} lines exceed the 1000-line Web test owner budget")
if violations:
    raise SystemExit("\n".join(violations))
print("controller/configuration ownership check passed")
PY_CHECK
then
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'rust module hygiene check passed\n'
