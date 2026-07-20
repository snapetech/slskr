#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python_bin="${PYTHON:-python3}"
work_dir="${SLSKR_CONTROLLER_AUDIT_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-controller-audit.XXXXXX")}"
state_dir="$work_dir/state"
log_file="$work_dir/slskr.log"
report_file="$work_dir/controller-audit.json"
slskd_report_file="$work_dir/slskd-controller-audit.json"
daemon_pid=""
keep_artifacts="${SLSKR_CONTROLLER_AUDIT_KEEP:-0}"
reference_json="${SLSKR_CONTROLLER_REFERENCE_JSON:-}"
reference_base="${SLSKR_CONTROLLER_REFERENCE_BASE:-}"
upstream_repo="${SLSKR_UPSTREAM_GIT_REPO:-$repo_root/../slskdn}"
slskd_ref="${SLSKR_SLSKD_REF:-16e5d86ec9a91120f3ef40b85cb22036566b788a}"
slskdn_ref="${SLSKR_SLSKDN_REF:-65a14a8b821de4df4ab7ef3ab3b156d7206837a3}"
slskd_root="$work_dir/upstream-slskd"
slskdn_root="$work_dir/upstream-slskdn"

pick_free_port() {
  "$python_bin" - <<'PY'
import socket
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    sock.bind(("127.0.0.1", 0))
    print(sock.getsockname()[1])
PY
}

cleanup() {
  if [[ -n "$daemon_pid" ]] && kill -0 "$daemon_pid" 2>/dev/null; then
    kill "$daemon_pid" 2>/dev/null || true
    wait "$daemon_pid" 2>/dev/null || true
  fi
  if [[ "$keep_artifacts" != "1" ]]; then
    rm -rf "$work_dir"
  fi
}
trap cleanup EXIT

mkdir -p "$state_dir"
mkdir -p "$slskd_root" "$slskdn_root"
git -C "$upstream_repo" archive "$slskd_ref" src/slskd | tar -x -C "$slskd_root"
git -C "$upstream_repo" archive "$slskdn_ref" src/slskd | tar -x -C "$slskdn_root"
http_port="${SLSKR_CONTROLLER_AUDIT_PORT:-$(pick_free_port)}"
base_url="http://127.0.0.1:$http_port"

node scripts/check-slskdn-controller-auth-registry.mjs \
  --target slskd \
  --slskdn-root "$slskd_root"
node scripts/check-slskdn-controller-auth-registry.mjs \
  --target slskdn \
  --slskdn-root "$slskdn_root"
cargo build -q -p slskr

(
  export SLSKR_HTTP_BIND="127.0.0.1:$http_port"
  export SLSKR_STATE_DIR="$state_dir"
  export SLSKR_AUTH_DISABLED=true
  export SLSKR_AUTO_CONNECT=false
  export SLSKR_RECONNECT=false
  # Route materialization does not exercise DHT I/O. Keep this gate isolated
  # from another slskdN-compatible process using the frozen default port 50305.
  export SLSKR_DHT_ENABLED=false
  export SLSKR_API_RATE_LIMIT_ANONYMOUS=10000
  export SLSKR_CONTROLLER_AUDIT_MODE=1
  exec target/debug/slskr serve
) >"$log_file" 2>&1 &
daemon_pid="$!"

healthy=0
for _ in $(seq 1 100); do
  if curl --fail --silent --max-time 1 "$base_url/api/health" >/dev/null; then
    healthy=1
    break
  fi
  if ! kill -0 "$daemon_pid" 2>/dev/null; then
    break
  fi
  sleep 0.1
done

if [[ "$healthy" != "1" ]]; then
  printf 'slskdN controller parity failed: slskr did not become healthy at %s\n' "$base_url" >&2
  tail -80 "$log_file" >&2 || true
  exit 1
fi

if ! node scripts/audit-slskdn-controller-routes.mjs \
  --slskdn-root "$slskdn_root" \
  --materialize \
  --include-response \
  --probe-base "$base_url" \
  --fail-on-unmatched \
  --fail-on-fallback \
  --json >"$report_file"; then
  printf 'slskdN controller parity failed: generic router fallthrough or compatibility fallback detected\n' >&2
  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$report_file" >&2 || true
  tail -80 "$log_file" >&2 || true
  exit 1
fi

if ! node scripts/audit-slskdn-controller-routes.mjs \
  --slskdn-root "$slskd_root" \
  --materialize \
  --include-response \
  --probe-base "$base_url" \
  --fail-on-unmatched \
  --fail-on-fallback \
  --json >"$slskd_report_file"; then
  printf 'slskd controller parity failed: generic router fallthrough or compatibility fallback detected\n' >&2
  rg -n 'generic_404|compatibility_fallback|AbortError|probe_error' "$slskd_report_file" >&2 || true
  tail -80 "$log_file" >&2 || true
  exit 1
fi

if [[ -n "$reference_json" && -n "$reference_base" ]]; then
  printf 'slskdN controller parity failed: select only one reference input\n' >&2
  exit 1
fi
if [[ -n "$reference_json" ]]; then
  node scripts/check-slskdn-get-contract-shapes.mjs \
    --reference-json "$reference_json" \
    --candidate-json "$report_file"
elif [[ -n "$reference_base" ]]; then
  node scripts/check-slskdn-get-contract-shapes.mjs \
    --reference-base "$reference_base" \
    --candidate-json "$report_file"
fi

printf 'slskd + slskdN controller parity check passed: all 91 + 678 materialized controller routes are handled without compatibility fallback\n'
if [[ "$keep_artifacts" == "1" ]]; then
  printf 'controller parity artifacts retained at %s\n' "$work_dir"
fi
