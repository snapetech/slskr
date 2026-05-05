#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

section() {
  printf '\n==> %s\n' "$1"
}

run_go_tests() {
  section "Go client tests"
  if command -v go >/dev/null 2>&1; then
    (cd client-go && go test ./...)
    return
  fi

  if command -v docker >/dev/null 2>&1; then
    docker run --rm \
      --user "$(id -u):$(id -g)" \
      -e GOCACHE=/tmp/go-build \
      -e GOMODCACHE=/tmp/go-mod \
      -v "$repo_root/client-go:/src" \
      -w /src \
      golang:1.23 \
      go test ./...
    return
  fi

  printf 'go is not installed and docker fallback is unavailable; cannot run Go client tests.\n' >&2
  exit 1
}

run_python_checks() {
  section "Python client quality/test check"
  if ! command -v python3 >/dev/null 2>&1; then
    printf 'python3 is not installed; cannot check Python client.\n' >&2
    exit 1
  fi

  if git ls-files 'client-python/__pycache__/*' 'client-python/**/__pycache__/*' 'client-python/**/*.egg-info/*' 'client-python/*.egg-info/*' | grep -q .; then
    printf 'tracked Python build artifacts found under client-python; remove __pycache__ and egg-info outputs from git.\n' >&2
    exit 1
  fi

  local venv_dir
  venv_dir="$(mktemp -d)"
  cleanup_python_artifacts() {
    rm -rf "$venv_dir"
    find client-python -type d \( -name __pycache__ -o -name '*.egg-info' \) -prune -exec rm -rf {} +
    find client-python -type f -name '*.pyc' -delete
  }
  trap cleanup_python_artifacts RETURN

  scripts/check-python-client-quality.sh

  PYTHONDONTWRITEBYTECODE=1 python3 -m compileall -q client-python

  python3 -m venv "$venv_dir"
  PIP_NO_CACHE_DIR=1 "$venv_dir/bin/python" -m pip install --upgrade pip >/dev/null
  PIP_NO_CACHE_DIR=1 "$venv_dir/bin/python" -m pip install -e "client-python[dev]" >/dev/null
  "$venv_dir/bin/python" - <<'PY'
from slskr import BatchClient, SlskrClient, WebSocketClient

client = SlskrClient("http://localhost:8080", "test-token")
assert client.base_url == "http://localhost:8080"
assert BatchClient is not None
assert WebSocketClient is not None
PY
  "$venv_dir/bin/python" -m pytest -q client-python/tests
  "$venv_dir/bin/python" -m pip check
}

run_typescript_checks() {
  section "TypeScript client lifecycle/build checks"
  if ! command -v npm >/dev/null 2>&1; then
    printf 'npm is not installed; cannot check TypeScript client.\n' >&2
    exit 1
  fi

  (
    cd client-ts
    if [[ ! -d node_modules ]]; then
      npm ci
    fi
    npm test -- --runInBand
    npm run build
  )
}

run_go_tests
run_python_checks
run_typescript_checks

printf '\nClient SDK gates passed.\n'
