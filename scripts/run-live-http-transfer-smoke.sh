#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -f .env ]]; then
  set -a
  # shellcheck disable=SC1091
  source ./.env
  set +a
fi

source_username="${SLSKR_SOURCE_USERNAME:-${SLSKR_TEST_4_USERNAME:-${SLSKR_TEST_2_USERNAME:-}}}"
source_password="${SLSKR_SOURCE_PASSWORD:-${SLSKR_TEST_4_PASSWORD:-${SLSKR_TEST_2_PASSWORD:-}}}"
target_username="${SLSKR_TARGET_USERNAME:-${SLSKR_TEST_2_USERNAME:-${SLSKR_TEST_1_USERNAME:-}}}"
target_password="${SLSKR_TARGET_PASSWORD:-${SLSKR_TEST_2_PASSWORD:-${SLSKR_TEST_1_PASSWORD:-}}}"
probe_username="${SLSKR_PROBE_USERNAME:-${SLSKR_TEST_3_USERNAME:-}}"
probe_password="${SLSKR_PROBE_PASSWORD:-${SLSKR_TEST_3_PASSWORD:-}}"

if [[ -z "${SLSKR_TARGET_USERNAME:-}" && "$target_username" == "$source_username" && -n "${SLSKR_TEST_1_USERNAME:-}" ]]; then
  target_username="$SLSKR_TEST_1_USERNAME"
  target_password="$SLSKR_TEST_1_PASSWORD"
fi
if [[ -z "${SLSKR_PROBE_USERNAME:-}" && ( "$probe_username" == "$source_username" || "$probe_username" == "$target_username" ) && -n "${SLSKR_TEST_1_USERNAME:-}" && "$SLSKR_TEST_1_USERNAME" != "$source_username" && "$SLSKR_TEST_1_USERNAME" != "$target_username" ]]; then
  probe_username="$SLSKR_TEST_1_USERNAME"
  probe_password="$SLSKR_TEST_1_PASSWORD"
fi

if [[ -z "$source_username" || -z "$source_password" || -z "$target_username" || -z "$target_password" || -z "$probe_username" || -z "$probe_password" ]]; then
  echo "missing credentials: set SLSKR_TEST_1/2/3_USERNAME/PASSWORD, or SLSKR_SOURCE_*, SLSKR_TARGET_*, and SLSKR_PROBE_*" >&2
  exit 2
fi
if [[ "$source_username" == "$target_username" || "$source_username" == "$probe_username" || "$target_username" == "$probe_username" ]]; then
  echo "source, target, and probe users must be three distinct accounts" >&2
  exit 2
fi

api_token="${SLSKR_LIVE_SMOKE_API_TOKEN:-live-smoke-token}"
soak_seconds="${SLSKR_LIVE_HTTP_SOAK_SECONDS:-60}"
timeout_seconds="${SLSKR_LIVE_HTTP_TIMEOUT_SECONDS:-180}"
work_dir="${SLSKR_LIVE_HTTP_WORK_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-live-http-transfer.XXXXXX")}"

pick_free_port() {
  node -e "const net=require('net'); const s=net.createServer(); s.listen(0,'127.0.0.1',()=>{console.log(s.address().port); s.close();});"
}

picked_ports=()
pick_port() {
  local port
  while true; do
    port="$(pick_free_port)"
    if [[ ! " ${picked_ports[*]} " =~ " ${port} " ]]; then
      picked_ports+=("$port")
      printf '%s\n' "$port"
      return 0
    fi
  done
}

redact() {
  local value="$1"
  if ((${#value} <= 2)); then
    printf '%s' '***'
  else
    printf '%s***%s' "${value:0:1}" "${value: -1}"
  fi
}

json_field() {
  local field="$1"
  node -e "let data='';process.stdin.on('data',c=>data+=c);process.stdin.on('end',()=>{const value=JSON.parse(data)[process.argv[1]]; if (value === undefined || value === null) process.exit(1); process.stdout.write(String(value));});" "$field"
}

auth_get() {
  local url="$1"
  curl -fsS -H "Authorization: Bearer $api_token" "$url"
}

auth_post_json() {
  local url="$1"
  local payload="$2"
  curl -fsS -H "Authorization: Bearer $api_token" -H "Content-Type: application/json" -d "$payload" "$url"
}

source_http_port="$(pick_port)"
target_http_port="$(pick_port)"
source_listen_port="${SLSKR_LIVE_SOURCE_LISTEN_PORT:-46102}"
target_listen_port="${SLSKR_LIVE_TARGET_LISTEN_PORT:-46104}"

ensure_port_free() {
  local port="$1"
  if (echo >"/dev/tcp/127.0.0.1/$port") >/dev/null 2>&1; then
    echo "port is already in use: $port" >&2
    exit 2
  fi
}

ensure_port_free "$source_listen_port"
ensure_port_free "$target_listen_port"

source_state="$work_dir/source-state"
target_state="$work_dir/target-state"
share_dir="$work_dir/slskr-live-source-share"
mkdir -p "$source_state" "$target_state" "$share_dir"

fixture_name="api-transfer-smoke.bin"
fixture_path="$share_dir/$fixture_name"
fixture_payload="slskr live HTTP transfer smoke $(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf '%s\n' "$fixture_payload" >"$fixture_path"
fixture_size="$(wc -c <"$fixture_path" | tr -d ' ')"
fixture_sha="$(sha256sum "$fixture_path" | awk '{print $1}')"
remote_filename="$(basename "$share_dir")/$fixture_name"

source_log="$work_dir/source.log"
target_log="$work_dir/target.log"
source_pid=""
target_pid=""

cleanup() {
  for pid in "$source_pid" "$target_pid"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}
trap cleanup EXIT

echo "live-http-transfer work_dir=$work_dir"
echo "source_user=$(redact "$source_username") target_user=$(redact "$target_username")"

(
  export SLSKR_HTTP_BIND="127.0.0.1:$source_http_port"
  export SLSKR_STATE_DIR="$source_state"
  export SLSK_USERNAME="$source_username"
  export SLSK_PASSWORD="$source_password"
  export SLSKR_AUTO_CONNECT=true
  export SLSKR_RECONNECT=false
  export SLSKR_AUTH_DISABLED=false
  export SLSKR_API_TOKEN="$api_token"
  export SLSKR_SHARE_DIRS="$share_dir"
  export SLSKR_LISTENER_BIND="127.0.0.1:$source_listen_port"
  export SLSK_LISTEN_PORT="$source_listen_port"
  export SLSKR_ADVERTISED_PORT="$source_listen_port"
  export SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS=10
  exec cargo run -q -p slskr -- serve
) >"$source_log" 2>&1 &
source_pid="$!"

(
  export SLSKR_HTTP_BIND="127.0.0.1:$target_http_port"
  export SLSKR_STATE_DIR="$target_state"
  export SLSK_USERNAME="$target_username"
  export SLSK_PASSWORD="$target_password"
  export SLSKR_AUTO_CONNECT=true
  export SLSKR_RECONNECT=false
  export SLSKR_AUTH_DISABLED=false
  export SLSKR_API_TOKEN="$api_token"
  export SLSKR_LISTENER_BIND="127.0.0.1:$target_listen_port"
  export SLSK_LISTEN_PORT="$target_listen_port"
  export SLSKR_ADVERTISED_PORT="$target_listen_port"
  export SLSKR_PEER_HOST_OVERRIDE=127.0.0.1
  export SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS=10
  exec cargo run -q -p slskr -- serve
) >"$target_log" 2>&1 &
target_pid="$!"

wait_connected() {
  local name="$1"
  local port="$2"
  local deadline=$((SECONDS + timeout_seconds))
  while ((SECONDS < deadline)); do
    if session="$(auth_get "http://127.0.0.1:$port/api/v0/session" 2>/dev/null)"; then
      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" ]]; then
        echo "$name connected"
        return 0
      fi
    fi
    sleep 2
  done
  echo "$name did not connect before timeout" >&2
  tail -n 80 "$work_dir/$name.log" >&2 || true
  return 1
}

wait_connected source "$source_http_port"
wait_connected target "$target_http_port"

wait_session_settled() {
  local name="$1"
  local port="$2"
  local deadline=$((SECONDS + timeout_seconds))
  local session seen
  while ((SECONDS < deadline)); do
    if session="$(auth_get "http://127.0.0.1:$port/api/v0/session" 2>/dev/null)"; then
      seen="$(printf '%s' "$session" | json_field server_messages_seen 2>/dev/null || echo 0)"
      if [[ "$(printf '%s' "$session" | json_field state 2>/dev/null || true)" == "connected" && "${seen:-0}" -ge 6 ]]; then
        echo "$name session settled messages=$seen"
        return 0
      fi
    fi
    sleep 1
  done
  echo "$name session did not settle: ${session:-no session response}" >&2
  return 1
}

wait_session_settled source "$source_http_port"
wait_session_settled target "$target_http_port"

wait_listener_ready() {
  local name="$1"
  local port="$2"
  local expected="$3"
  local deadline=$((SECONDS + timeout_seconds))
  local listeners
  while ((SECONDS < deadline)); do
    if listeners="$(auth_get "http://127.0.0.1:$port/api/v0/listeners" 2>/dev/null)"; then
      if [[ "$listeners" == *"127.0.0.1:$expected"* && "$listeners" == *'"errors":0'* ]]; then
        echo "$name listener ready port=$expected"
        return 0
      fi
    fi
    sleep 1
  done
  echo "$name listener did not become ready: ${listeners:-no listener response}" >&2
  return 1
}

wait_listener_ready source "$source_http_port" "$source_listen_port"
wait_listener_ready target "$target_http_port" "$target_listen_port"

# The public server can briefly return port=0 immediately after login even after
# SetWaitPort succeeds. Give that metadata a moment to settle before probing.
sleep "${SLSKR_LIVE_HTTP_PEER_PUBLISH_SETTLE_SECONDS:-5}"

wait_peer_address() {
  local deadline=$((SECONDS + timeout_seconds))
  local stdout_file="$work_dir/peer-address.out"
  local stderr_file="$work_dir/peer-address.err"
  while ((SECONDS < deadline)); do
    if (
      export SLSK_USERNAME="$probe_username"
      export SLSK_PASSWORD="$probe_password"
      export SLSK_PEER_USERNAME="$source_username"
      exec cargo run -q -p slskr -- probe peer-address
    ) >"$stdout_file" 2>"$stderr_file"; then
      if rg -q 'port=[1-9][0-9]*' "$stdout_file"; then
        echo "source peer address advertised"
        return 0
      fi
    fi
    sleep 5
  done
  echo "source peer address was not advertised before timeout" >&2
  tail -n 40 "$stdout_file" >&2 || true
  tail -n 40 "$stderr_file" >&2 || true
  return 1
}

wait_target_browse() {
  local deadline=$((SECONDS + timeout_seconds))
  local browse_json status count
  while ((SECONDS < deadline)); do
    auth_post_json "http://127.0.0.1:$target_http_port/api/v0/users/$source_username/browse/request" '{}' >/dev/null || true
    local attempt_deadline=$((SECONDS + 20))
    while ((SECONDS < attempt_deadline)); do
      if browse_json="$(auth_get "http://127.0.0.1:$target_http_port/api/v0/users/$source_username/browse" 2>/dev/null)"; then
        status="$(printf '%s' "$browse_json" | json_field status 2>/dev/null || true)"
        count="$(printf '%s' "$browse_json" | json_field count 2>/dev/null || true)"
        if [[ "$status" == "ready" && "${count:-0}" -gt 0 ]]; then
          echo "target browse path ready files=$count"
          return 0
        fi
        if [[ "$status" == "failed" ]]; then
          echo "target browse failed: $browse_json" >&2
          return 1
        fi
      fi
      sleep 2
    done
  done
  echo "target browse path did not become ready before timeout" >&2
  [[ -n "${browse_json:-}" ]] && echo "$browse_json" >&2
  tail -n 80 "$target_log" >&2 || true
  return 1
}

wait_peer_address
wait_target_browse

missing_auth_status="$(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:$target_http_port/api/v0/config")"
if [[ "$missing_auth_status" != "401" ]]; then
  echo "expected unauthenticated config request to return 401, got $missing_auth_status" >&2
  exit 1
fi
auth_get "http://127.0.0.1:$target_http_port/api/v0/config" >/dev/null
echo "api auth enforced"

created="$(auth_post_json "http://127.0.0.1:$target_http_port/api/v0/transfers" "{\"peer_username\":\"$source_username\",\"filename\":\"$remote_filename\",\"size\":$fixture_size}")"
transfer_id="$(printf '%s' "$created" | json_field id)"
auth_post_json "http://127.0.0.1:$target_http_port/api/v0/transfers/$transfer_id/start" '{}' >/dev/null
echo "transfer queued id=$transfer_id filename=$remote_filename size=$fixture_size"

deadline=$((SECONDS + timeout_seconds))
last_transfer=""
while ((SECONDS < deadline)); do
  last_transfer="$(auth_get "http://127.0.0.1:$target_http_port/api/v0/transfers/$transfer_id")"
  status="$(printf '%s' "$last_transfer" | json_field status 2>/dev/null || true)"
  bytes="$(printf '%s' "$last_transfer" | json_field bytes_transferred 2>/dev/null || true)"
  if [[ "$status" == "succeeded" && "$bytes" == "$fixture_size" ]]; then
    break
  fi
  if [[ "$status" == "failed" || "$status" == "cancelled" ]]; then
    echo "transfer ended unexpectedly: $last_transfer" >&2
    tail -n 80 "$source_log" >&2 || true
    tail -n 80 "$target_log" >&2 || true
    exit 1
  fi
  sleep 2
done

status="$(printf '%s' "$last_transfer" | json_field status 2>/dev/null || true)"
bytes="$(printf '%s' "$last_transfer" | json_field bytes_transferred 2>/dev/null || true)"
if [[ "$status" != "succeeded" || "$bytes" != "$fixture_size" ]]; then
  echo "transfer did not succeed before timeout: $last_transfer" >&2
  tail -n 80 "$source_log" >&2 || true
  tail -n 80 "$target_log" >&2 || true
  exit 1
fi

download_path="$target_state/downloads/$remote_filename"
if [[ ! -f "$download_path" ]]; then
  echo "downloaded file missing: $download_path" >&2
  exit 1
fi
download_sha="$(sha256sum "$download_path" | awk '{print $1}')"
if [[ "$download_sha" != "$fixture_sha" ]]; then
  echo "download sha mismatch: expected=$fixture_sha actual=$download_sha" >&2
  exit 1
fi
echo "http api transfer completed bytes=$bytes sha256=$download_sha"

if ((soak_seconds > 0)); then
  echo "soak seconds=$soak_seconds"
  sleep "$soak_seconds"
  source_state_json="$(auth_get "http://127.0.0.1:$source_http_port/api/v0/session")"
  target_state_json="$(auth_get "http://127.0.0.1:$target_http_port/api/v0/session")"
  if [[ "$(printf '%s' "$source_state_json" | json_field state)" != "connected" ]]; then
    echo "source disconnected during soak: $source_state_json" >&2
    exit 1
  fi
  if [[ "$(printf '%s' "$target_state_json" | json_field state)" != "connected" ]]; then
    echo "target disconnected during soak: $target_state_json" >&2
    exit 1
  fi
fi

auth_get "http://127.0.0.1:$source_http_port/api/v0/stats" >/dev/null
auth_get "http://127.0.0.1:$target_http_port/api/v0/stats" >/dev/null
echo "live http transfer smoke ok"
