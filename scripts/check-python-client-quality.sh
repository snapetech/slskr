#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_bin="${PYTHON_BIN:-python3}"

if ! "$python_bin" -c 'import black, flake8, mypy' >/dev/null 2>&1; then
  quality_venv="$(mktemp -d "${TMPDIR:-/tmp}/slskr-python-quality.XXXXXX")"
  cleanup_quality_venv() {
    rm -rf "$quality_venv"
  }
  trap cleanup_quality_venv EXIT
  "$python_bin" -m venv "$quality_venv"
  PIP_NO_CACHE_DIR=1 "$quality_venv/bin/python" -m pip install --upgrade pip >/dev/null
  PIP_NO_CACHE_DIR=1 "$quality_venv/bin/python" -m pip install -r client-python/constraints.txt >/dev/null
  python_bin="$quality_venv/bin/python"
fi

ledger="docs/dev/bug-burndown-ledger.md"
status=0

if ! rg -n '^\| BUG-019 .* \| Verified \|$' "$ledger" >/dev/null; then
  printf 'python client quality check failed: BUG-019 must stay verified in council ledger\n' >&2
  status=1
fi

"$python_bin" - <<'PY'
import ast
import pathlib
import sys

root = pathlib.Path("client-python")
errors = []

for path in sorted(root.rglob("*.py")):
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    for node in ast.walk(tree):
        if isinstance(node, ast.ExceptHandler) and node.type is None:
            errors.append(f"{path}:{node.lineno}: bare except is not allowed")
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Name) and node.func.id == "print":
            path_parts = set(path.parts)
            if path.name != "websocket.py" and "examples" not in path_parts and "tests" not in path_parts:
                errors.append(f"{path}:{node.lineno}: library code should not print directly")

setup_text = (root / "setup.py").read_text(encoding="utf-8")
init_text = (root / "slskr" / "__init__.py").read_text(encoding="utf-8")
if 'version="1.0.0"' not in setup_text or '__version__ = "1.0.0"' not in init_text:
    errors.append("client-python package and module versions must stay aligned at 1.0.0")
for expected in ["pytest", "mypy", "flake8"]:
    if expected not in setup_text:
        errors.append(f"client-python setup.py must keep dev dependency marker for {expected}")

if errors:
    print("python client quality check failed:", file=sys.stderr)
    for error in errors:
        print(f"  {error}", file=sys.stderr)
    raise SystemExit(1)
PY

if ! "$python_bin" -m black --check client-python; then
  printf 'python client quality check failed: Black formatting is not clean\n' >&2
  status=1
fi

if ! "$python_bin" -m flake8 --config=client-python/.flake8 client-python; then
  printf 'python client quality check failed: Flake8 reported violations\n' >&2
  status=1
fi

if ! "$python_bin" -m mypy --config-file=client-python/pyproject.toml client-python/slskr; then
  printf 'python client quality check failed: mypy reported violations\n' >&2
  status=1
fi

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'python client quality check passed\n'
