#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

ledger="docs/dev/bug-burndown-ledger.md"
status=0

for id in BUG-010 BUG-011 BUG-017 BUG-030; do
  if ! rg -n "^\| ${id} \|" "$ledger" >/dev/null; then
    printf 'openapi/docs drift check failed: %s is missing from council ledger\n' "$id" >&2
    status=1
  fi
done

python3 - <<'PY'
import json
from pathlib import Path

with Path("docs/openapi.json").open(encoding="utf-8") as handle:
    spec = json.load(handle)

if spec.get("openapi") != "3.0.0":
    raise SystemExit("docs/openapi.json must remain OpenAPI 3.0.0")
if not isinstance(spec.get("paths"), dict) or not spec["paths"]:
    raise SystemExit("docs/openapi.json must contain a non-empty paths object")
PY

for expected in 'generate_openapi_json' 'CHECKED_IN_OPENAPI_JSON' 'include_str!("../../../docs/openapi.json")' 'test_runtime_openapi_matches_checked_in_spec' 'slskd-compatible array'; do
  if ! rg -n -F "$expected" crates/slskr/src/openapi.rs >/dev/null; then
    printf 'openapi/docs drift check failed: expected OpenAPI regression token missing: %s\n' "$expected" >&2
    status=1
  fi
done

for expected in 'non-persisted' 'read-only' 'compatibility'; do
  if ! rg -n -F "$expected" docs/security-bug-burndown.md docs/dev/bug-burndown-ledger.md crates/slskr/src/openapi.rs crates/slskr/src/main.rs >/dev/null; then
    printf 'openapi/docs drift check failed: expected compatibility docs token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'openapi/docs drift check passed\n'
