#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/slskr-auth-profiles.XXXXXX")"
daemon_pid=""

cleanup() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  rm -rf "$work_dir"
}
trap cleanup EXIT

pick_free_port() {
  python3 - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

stop_daemon() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid"
    wait "$daemon_pid" 2>/dev/null || true
  fi
  daemon_pid=""
}

start_daemon() {
  local target="$1"
  local port="$2"
  local state_dir="$work_dir/$target-state"
  mkdir -p "$state_dir"
  (
    export SLSKR_HTTP_BIND="127.0.0.1:$port"
    export SLSKR_STATE_DIR="$state_dir"
    export SLSKR_CONTROLLER_COMPATIBILITY_TARGET="$target"
    export SLSKR_AUTH_DISABLED=false
    export SLSKR_API_TOKEN=admin-token
    export SLSKR_API_READ_WRITE_TOKEN=write-token
    export SLSKR_API_READ_ONLY_TOKEN=read-token
    export SLSKR_AUTO_CONNECT=false
    export SLSKR_RECONNECT=false
    exec target/debug/slskr serve
  ) >"$work_dir/$target.log" 2>&1 &
  daemon_pid="$!"
  for _ in $(seq 1 100); do
    if curl --fail --silent --max-time 1 "http://127.0.0.1:$port/api/health" >/dev/null; then
      return 0
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      break
    fi
    sleep 0.1
  done
  tail -80 "$work_dir/$target.log" >&2 || true
  return 1
}

status() {
  local port="$1"
  local path="$2"
  local credential="${3:-}"
  local args=(--silent --output /dev/null --write-out '%{http_code}' --max-time 5)
  if [[ -n "$credential" ]]; then
    args+=(--header "Authorization: $credential")
  fi
  curl "${args[@]}" "http://127.0.0.1:$port$path"
}

expect_status() {
  local expected="$1"
  shift
  local actual
  actual="$(status "$@")"
  if [[ "$actual" != "$expected" ]]; then
    printf 'controller auth profile check failed: %s expected %s, got %s\n' "$*" "$expected" "$actual" >&2
    return 1
  fi
}

cargo build -q -p slskr

slskd_port="$(pick_free_port)"
start_daemon slskd "$slskd_port"
expect_status 401 "$slskd_port" /api/v0/logs
expect_status 200 "$slskd_port" /api/v0/logs 'Bearer read-token'
expect_status 401 "$slskd_port" /api/v0/application/dump
stop_daemon

slskdn_port="$(pick_free_port)"
start_daemon slskdn "$slskdn_port"
expect_status 401 "$slskdn_port" /api/v0/logs
expect_status 403 "$slskdn_port" /api/v0/logs 'Bearer read-token'
expect_status 200 "$slskdn_port" /api/v0/logs 'Bearer admin-token'
expect_status 401 "$slskdn_port" /api/v0/profile/me
expect_status 200 "$slskdn_port" /api/v0/profile/me 'Bearer read-token'
stop_daemon

printf 'controller auth profile check passed: frozen slskd and slskdN policies diverge externally as configured\n'
