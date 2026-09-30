#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_bin="${PYTHON_BIN:-python3}"
package_venv=""

package_dir="$(mktemp -d)"
install_dir="$(mktemp -d)"
package_source_dir="$(mktemp -d)"
cp -a client-python/. "$package_source_dir/"

cleanup_package_check() {
  rm -rf "$package_dir" "$install_dir" "$package_source_dir"
  if [[ -n "$package_venv" ]]; then
    rm -rf "$package_venv"
  fi
}
trap cleanup_package_check EXIT

if ! "$python_bin" -m build --version >/dev/null 2>&1; then
  package_venv="$(mktemp -d "${TMPDIR:-/tmp}/slskr-python-package.XXXXXX")"
  "$python_bin" -m venv "$package_venv"
  PIP_NO_CACHE_DIR=1 "$package_venv/bin/python" -m pip install --upgrade pip >/dev/null
  PIP_NO_CACHE_DIR=1 "$package_venv/bin/python" -m pip install -r client-python/constraints.txt >/dev/null
  python_bin="$package_venv/bin/python"
fi

"$python_bin" -m build --sdist --wheel --outdir "$package_dir" "$package_source_dir"

mapfile -t artifacts < <(find "$package_dir" -maxdepth 1 -type f \( -name '*.whl' -o -name '*.tar.gz' \) -printf '%f\n' | sort)
if [[ "${#artifacts[@]}" -ne 2 ]]; then
  printf 'python package check failed: expected one wheel and one sdist, found %s\n' "${#artifacts[@]}" >&2
  exit 1
fi

wheel_path="$(find "$package_dir" -maxdepth 1 -type f -name '*.whl' -print -quit)"
"$python_bin" -m pip install --no-deps --target "$install_dir" "$wheel_path" >/dev/null
PYTHONPATH="$install_dir" "$python_bin" - <<'PY'
import slskr

assert slskr.__version__ == "1.0.0"
assert slskr.SlskrClient is not None
assert slskr.WebSocketClient is not None
PY

printf 'Python source/wheel package check passed: %s\n' "${artifacts[*]}"
