#!/usr/bin/env bash
set -euo pipefail

# The live adapter launches .NET, Rust, and short-lived Node helpers. Keep the
# entire harness bounded even when it is invoked directly instead of through a
# parent guard. The lower parent limit, if any, remains authoritative.
interop_virtual_memory_kib="${SLSKR_INTEROP_VIRTUAL_MEMORY_KIB:-12582912}"
if [[ ! "$interop_virtual_memory_kib" =~ ^[1-9][0-9]{0,7}$ || "$interop_virtual_memory_kib" -gt 12582912 ]]; then
  echo "SLSKR_INTEROP_VIRTUAL_MEMORY_KIB must be between 1 and 12582912" >&2
  exit 2
fi
parent_virtual_memory_kib="$(ulimit -v)"
if [[ "$parent_virtual_memory_kib" =~ ^[0-9]+$ && "$parent_virtual_memory_kib" -lt "$interop_virtual_memory_kib" ]]; then
  interop_virtual_memory_kib="$parent_virtual_memory_kib"
fi
ulimit -v "$interop_virtual_memory_kib"
export NODE_OPTIONS='--max-old-space-size=1024'
export DOTNET_GCHeapHardLimit=1073741824
export COMPlus_GCHeapHardLimit=1073741824
export DOTNET_PROCESSOR_COUNT=2
export DOTNET_ThreadPool_MinThreads=2
export DOTNET_ThreadPool_MaxThreads=16

# Run the common Soulseek wire checks against a pinned slskd build.  The
# frozen slskd source does not contain the test endpoint override used by the
# slskdN harness, so callers must provide a test-only adapter build exposing
# SLSKD_TEST_USER_ENDPOINT_OVERRIDES.  The adapter changes endpoint lookup
# only; it does not change protocol or product behavior.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
output_dir="${SLSKR_INTEROP_OUTPUT_DIR:-$repo_root/target/live-interop}"
work_dir="${SLSKR_SLSKD_INTEROP_WORK_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-slskd-interop.XXXXXX")}"
binary="${SLSKR_FROZEN_SLSKD_BINARY:-}"
server_endpoint="${SLSKR_CROSS_CLIENT_SERVER:-${SLSK_SERVER:-vps.slsknet.org:2271}}"
api_token="${SLSKR_CROSS_CLIENT_API_TOKEN:-slskr-cross-client-interop}"
timeout_seconds="${SLSKR_CROSS_CLIENT_TIMEOUT_SECONDS:-90}"
slskd_http_port="${SLSKR_SLSKD_INTEROP_HTTP_PORT:-$(node -e 'const net=require("net");const s=net.createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})')}"
slskr_http_port="${SLSKR_SLSKD_INTEROP_SLSKR_HTTP_PORT:-$(node -e 'const net=require("net");const s=net.createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})')}"
slskd_listen_port="${SLSKR_SLSKD_INTEROP_LISTEN_PORT:-$(node -e 'const net=require("net");const s=net.createServer();s.listen(0,"0.0.0.0",()=>{console.log(s.address().port);s.close()})')}"
slskr_listen_port="${SLSKR_SLSKD_INTEROP_SLSKR_LISTEN_PORT:-$(node -e 'const net=require("net");const s=net.createServer();s.listen(0,"0.0.0.0",()=>{console.log(s.address().port);s.close()})')}"
slskr_probe_listen_port="${SLSKR_SLSKD_INTEROP_PROBE_LISTEN_PORT:-$(node -e 'const net=require("net");const s=net.createServer();s.listen(0,"0.0.0.0",()=>{console.log(s.address().port);s.close()})')}"

if [[ -z "$binary" || ! -x "$binary" ]]; then
  echo "set SLSKR_FROZEN_SLSKD_BINARY to the pinned slskd test-adapter binary" >&2
  exit 2
fi

slskd_binary_has_endpoint_overrides() {
  local candidate="$1"
  local marker='SLSKD_TEST_USER_ENDPOINT_OVERRIDES'
  local file
  for file in "$candidate" "${candidate}.dll"; do
    [[ -f "$file" ]] || continue
    # .NET assemblies can store user-facing strings in layouts that the
    # platform `strings` utility does not recognize. Scan the binary bytes
    # first, then retain the plain and UTF-16 scans for native/portable builds.
    if rg -a -Fq -- "$marker" "$file" 2>/dev/null; then
      return 0
    fi
    if grep -Fq -- "$marker" < <(strings "$file" 2>/dev/null); then
      return 0
    fi
    if grep -Fq -- "$marker" < <(strings -el "$file" 2>/dev/null); then
      return 0
    fi
  done
  return 1
}

if ! slskd_binary_has_endpoint_overrides "$binary"; then
  echo "slskd binary lacks SLSKD_TEST_USER_ENDPOINT_OVERRIDES test-adapter support" >&2
  echo "refusing live parity evidence because endpoint overrides would be ignored" >&2
  exit 2
fi

for env_file in "$repo_root/.env" "$repo_root/.secrets/generated-soulseek-accounts.env"; do
  if [[ -f "$env_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$env_file"
    set +a
  fi
done

slskr_username="${SLSKR_CROSS_CLIENT_SLSKR_USERNAME:-${SLSKR_TEST_2_USERNAME:-}}"
slskr_password="${SLSKR_CROSS_CLIENT_SLSKR_PASSWORD:-${SLSKR_TEST_2_PASSWORD:-}}"
slskd_username="${SLSKR_CROSS_CLIENT_SLSKD_USERNAME:-${SLSKR_TEST_1_USERNAME:-}}"
slskd_password="${SLSKR_CROSS_CLIENT_SLSKD_PASSWORD:-${SLSKR_TEST_1_PASSWORD:-}}"
upstream_username="${SLSKR_CROSS_CLIENT_UPSTREAM_USERNAME:-${SLSKR_TEST_3_USERNAME:-$slskr_username}}"
upstream_password="${SLSKR_CROSS_CLIENT_UPSTREAM_PASSWORD:-${SLSKR_TEST_3_PASSWORD:-$slskr_password}}"

if [[ -z "$slskr_username" || -z "$slskr_password" || -z "$slskd_username" || -z "$slskd_password" ]]; then
  echo "missing slskd interop credentials" >&2
  exit 2
fi
if [[ "$slskr_username" == "$slskd_username" ]]; then
  echo "slskr and slskd interop users must be distinct" >&2
  exit 2
fi

mkdir -p "$output_dir" "$work_dir"
final_result_file="$output_dir/slskr-slskd-cross-client-interop.tsv"
result_file="$output_dir/.slskr-slskd-cross-client-interop.$$.tsv"
slskd_app="$work_dir/slskd-app"
slskr_state="$work_dir/slskr-state"
slskr_share="$repo_root/target/open-commons-fixtures"
slskd_share="$repo_root/target/open-commons-fixtures"
slskd_log="$work_dir/slskd.log"
slskr_log="$work_dir/slskr.log"
fixture_file="$slskr_share/commons-click-track.ogg"
fixture_size="$(stat -c '%s' "$fixture_file")"
fixture_sha256="$(sha256sum "$fixture_file" | awk '{print $1}')"
fixture_remote_path='open-commons-fixtures\commons-click-track.ogg'
fixture_remote_path_json="${fixture_remote_path//\\/\\\\}"
slskd_endpoint_overrides="$slskr_username=127.0.0.1:$slskr_listen_port"
if [[ "$upstream_username" != "$slskr_username" && "$upstream_username" != "$slskd_username" ]]; then
  slskd_endpoint_overrides+=";$upstream_username=127.0.0.1:$slskr_probe_listen_port"
fi
mkdir -p "$slskd_app/incomplete" "$slskd_app/downloads" "$slskr_state" "$work_dir/rust-downloads"

if [[ ! -x "$repo_root/target/debug/slskr" ]]; then
  export CARGO_BUILD_JOBS=1
  export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
  export CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}"
  scripts/with-build-guard.sh cargo build -q -p slskr
fi
slskr_binary="$repo_root/target/debug/slskr"

record_check() {
  local check="$1" status="$2" detail="$3"
  printf '%s\t%s\t%s\t%s\n' "$(date -Is)" "$check" "$status" "$detail" | tee -a "$result_file"
}

auth_rust() {
  curl -fsS --max-time 10 -H "Authorization: Bearer $api_token" "$@"
}

auth_slskd() {
  curl -fsS --max-time 10 "$@"
}

rust_search_diagnostics() {
  local query="${1:-}"
  auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/logs" 2>/dev/null |
    node -e '
const query = process.argv[1];
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  let entries;
  try { entries = JSON.parse(input); } catch { process.exit(0); }
  const selected = (Array.isArray(entries) ? entries : [])
    .filter(entry => entry.category === "search" || /incoming public search/i.test(entry.message || ""))
    .filter(entry => !query || (entry.message || "").includes(query))
    .slice(0, 20)
    .map(entry => `${entry.level || "?"}:${entry.message || ""}`)
    .join(" | ");
  process.stdout.write(selected.replace(/[\r\n\t]+/g, " ").slice(0, 1800));
});
' "$query" 2>/dev/null || true
}

json_field() {
  local field="$1"
  node -e '
const field = process.argv[1];
let data = "";
process.stdin.on("data", chunk => data += chunk);
process.stdin.on("end", () => {
  const value = JSON.parse(data)[field];
  if (value === undefined || value === null) process.exit(1);
  process.stdout.write(String(value));
});
' "$field"
}

cleanup() {
  for pid in "${slskr_pid:-}" "${slskd_pid:-}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
  if [[ -n "${result_file:-}" && -f "$result_file" ]]; then
    rm -f -- "$result_file"
  fi
}
trap cleanup EXIT

printf 'timestamp\tcheck\tstatus\tdetail\n' >"$result_file"

SLSKD_APP_DIR="$slskd_app" \
SLSKD_HTTP_PORT="$slskd_http_port" \
SLSKD_HTTP_IP_ADDRESS=127.0.0.1 \
SLSKD_NO_HTTPS=true \
SLSKD_NO_AUTH=true \
SLSKD_NO_LOGO=true \
SLSKD_NO_VERSION_CHECK=true \
SLSKD_INCOMPLETE_DIR="$slskd_app/incomplete" \
SLSKD_DOWNLOADS_DIR="$slskd_app/downloads" \
SLSKD_SHARED_DIR="$slskd_share" \
SLSKD_SLSK_ADDRESS="${server_endpoint%:*}" \
SLSKD_SLSK_PORT="${server_endpoint##*:}" \
SLSKD_SLSK_USERNAME="$slskd_username" \
SLSKD_SLSK_PASSWORD="$slskd_password" \
SLSKD_SLSK_LISTEN_IP_ADDRESS=0.0.0.0 \
SLSKD_SLSK_LISTEN_PORT="$slskd_listen_port" \
SLSKD_SLSK_DNET_LOGGING=true \
SLSKD_SLSK_DIAG_LEVEL=debug \
SLSKD_TEST_USER_ENDPOINT_OVERRIDES="$slskd_endpoint_overrides" \
  "$binary" >"$slskd_log" 2>&1 &
slskd_pid="$!"

for _ in $(seq 1 "${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_ATTEMPTS:-30}"); do
  target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
  if [[ "$target_state" == *'Connected, LoggedIn'* \
    && ( "$target_state" == *'"canAcceptChildren":true'* \
      || "$target_state" == *'"CanAcceptChildren":true'* ) ]]; then
    break
  fi
  sleep "${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_DELAY_SECONDS:-1}"
done

SLSK_SERVER="$server_endpoint" \
SLSKR_HTTP_BIND="127.0.0.1:$slskr_http_port" \
SLSKR_STATE_DIR="$slskr_state" \
SLSK_USERNAME="$slskr_username" \
SLSK_PASSWORD="$slskr_password" \
SLSKR_AUTO_CONNECT=true \
SLSKR_RECONNECT=true \
SLSKR_AUTH_DISABLED=false \
SLSKR_SLSK_DIAG_LEVEL=debug \
SLSKR_SLSK_DNET_LOGGING=true \
SLSKR_API_TOKEN="$api_token" \
SLSKR_SHARE_DIRS="$slskr_share" \
SLSKR_DOWNLOADS_DIR="$work_dir/rust-downloads" \
SLSKR_LISTENER_BIND="0.0.0.0:$slskr_listen_port" \
SLSK_LISTEN_PORT="$slskr_listen_port" \
SLSKR_ADVERTISED_PORT="$slskr_listen_port" \
SLSKR_PEER_HOST_OVERRIDE=127.0.0.1 \
SLSKR_DISTRIBUTED_PARENT_OVERRIDE="127.0.0.1:$slskd_listen_port" \
SLSKR_TEST_USER_ENDPOINT_OVERRIDES="$slskd_username=127.0.0.1:$slskd_listen_port" \
  "$slskr_binary" serve >"$slskr_log" 2>&1 &
slskr_pid="$!"

deadline=$((SECONDS + timeout_seconds))
target_state=""
rust_state=""
while ((SECONDS < deadline)); do
  target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
  rust_state="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/session" 2>/dev/null || true)"
  if [[ "$target_state" == *'Connected, LoggedIn'* && "$rust_state" == *'"state":"connected"'* ]]; then
    break
  fi
  sleep 2
done
if [[ "$target_state" == *'Connected, LoggedIn'* ]]; then
  record_check runtime-slskd-session ok "state=connected"
else
  record_check runtime-slskd-session fail "application=${target_state:0:240}"
  exit 1
fi
if [[ "$rust_state" == *'"state":"connected"'* ]]; then
  record_check runtime-slskr-session-slskd ok "state=connected"
else
  record_check runtime-slskr-session-slskd fail "session=${rust_state:0:240}"
  exit 1
fi

for _ in $(seq 1 "${SLSKR_CROSS_CLIENT_SLSKR_DISTRIBUTED_READY_ATTEMPTS:-120}"); do
  rust_distributed_state="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/application" 2>/dev/null || true)"
  if [[ "$rust_distributed_state" == *'"hasParent":true'* \
    || "$rust_distributed_state" == *'"HasParent":true'* ]]; then
    break
  fi
  sleep "${SLSKR_CROSS_CLIENT_SLSKR_DISTRIBUTED_READY_DELAY_SECONDS:-1}"
done

# Startup share indexing is synchronous in a fresh state directory, but a
# bounded rescan makes the live proof independent of cache timing and verifies
# the exact fixture is present before public-search checks begin.
auth_rust -X POST -H 'Content-Type: application/json' -d '{}' \
  "http://127.0.0.1:$slskr_http_port/api/v0/shares/rescan" >/dev/null 2>&1 || true
slskr_share_ready=false
for _ in $(seq 1 "${SLSKR_CROSS_CLIENT_SHARE_READY_ATTEMPTS:-30}"); do
  rust_share_catalog="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/shares/catalog?q=commons-click-track.ogg" 2>/dev/null || true)"
  if [[ "$rust_share_catalog" == *"commons-click-track.ogg"* ]]; then
    slskr_share_ready=true
    break
  fi
  sleep "${SLSKR_CROSS_CLIENT_SHARE_READY_DELAY_SECONDS:-1}"
done
if [[ "$slskr_share_ready" == true ]]; then
  record_check runtime-slskr-share-index ok "fixture=commons-click-track.ogg"
else
  record_check runtime-slskr-share-index fail "catalog=${rust_share_catalog:0:320}"
fi

target_endpoint="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/endpoint")"
rust_endpoint="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/endpoint")"
if [[ "$target_endpoint" == *"$slskr_listen_port"* ]]; then
  record_check network-slskd-resolves-slskr ok "endpoint=$slskr_listen_port"
else
  record_check network-slskd-resolves-slskr fail "$target_endpoint"
fi
if [[ "$rust_endpoint" == *"$slskd_listen_port"* ]]; then
  record_check network-slskr-resolves-slskd ok "endpoint=$slskd_listen_port"
else
  record_check network-slskr-resolves-slskd fail "$rust_endpoint"
fi

escaped_slskr_username="$(node -e 'process.stdout.write(encodeURIComponent(process.argv[1]))' "$slskr_username")"
user_watch_log="$work_dir/slskr-user-watch.log"
if [[ "$upstream_username" != "$slskr_username" && "$upstream_username" != "$slskd_username" ]] \
  && SLSK_SERVER="$server_endpoint" \
  SLSK_USERNAME="$upstream_username" \
  SLSK_PASSWORD="$upstream_password" \
  SLSK_PEER_USERNAME="$slskd_username" \
    "$slskr_binary" probe user-watch >"$user_watch_log" 2>&1; then
  record_check protocol-slskr-user-watch-slskd ok "watched=$slskd_username stats=received"
else
  record_check protocol-slskr-user-watch-slskd fail "detail=$(tail -n 4 "$user_watch_log" 2>/dev/null | tr '\n\t' '  ')"
fi

target_user_status="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$escaped_slskr_username/status" 2>/dev/null || true)"
target_user_info="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$escaped_slskr_username/info" 2>/dev/null || true)"
if [[ -n "$target_user_status" && -n "$target_user_info" ]]; then
  record_check protocol-slskd-user-watch-slskr ok "status-and-info=$slskr_username"
else
  record_check protocol-slskd-user-watch-slskr fail "status=$target_user_status info=$target_user_info"
fi

distributed_peer_log="$work_dir/slskr-distributed-peer.log"
distributed_target_state=""
distributed_target_ready=false
if [[ "$upstream_username" != "$slskr_username" && "$upstream_username" != "$slskd_username" ]]; then
  for _ in $(seq 1 "${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_ATTEMPTS:-30}"); do
    distributed_target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
    if [[ "$distributed_target_state" == *'"canAcceptChildren":true'* || "$distributed_target_state" == *'"CanAcceptChildren":true'* ]]; then
      distributed_target_ready=true
      break
    fi
    sleep "${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_DELAY_SECONDS:-1}"
  done
fi
if [[ "$distributed_target_ready" == true ]] \
  && SLSK_SERVER="$server_endpoint" \
  SLSK_USERNAME="$upstream_username" \
  SLSK_PASSWORD="$upstream_password" \
  SLSK_PEER_USERNAME="$slskd_username" \
  SLSK_DISTRIBUTED_PEER_USERNAME="$slskd_username" \
  SLSK_DISTRIBUTED_HOST_OVERRIDE=127.0.0.1 \
  SLSK_DISTRIBUTED_PORT_OVERRIDE="$slskd_listen_port" \
    "$slskr_binary" probe distributed-peer >"$distributed_peer_log" 2>&1; then
  record_check protocol-slskr-distributed-peer-slskd ok "peer=$slskd_username ping=received probe_contract=distributed-ping-response-v2"
elif [[ "$distributed_target_ready" != true ]]; then
  record_check protocol-slskr-distributed-peer-slskd fail "detail=target distributed network not ready after ${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_ATTEMPTS:-30}s state=${distributed_target_state:0:240}"
else
  record_check protocol-slskr-distributed-peer-slskd fail "detail=$(tail -n 4 "$distributed_peer_log" 2>/dev/null | tr '\n\t' '  ')"
fi

target_browse="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/browse" 2>/dev/null || true)"
if [[ "$target_browse" == *"commons-click-track.ogg"* ]]; then
  record_check protocol-slskd-browses-slskr ok "fixture=commons-click-track.ogg"
else
  record_check protocol-slskd-browses-slskr fail "${target_browse:0:320}"
fi

target_directory="$(auth_slskd -X POST -H 'Content-Type: application/json' \
  -d '{"directory":"open-commons-fixtures"}' \
  "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/directory" 2>/dev/null || true)"
if [[ "$target_directory" == *"commons-click-track.ogg"* ]]; then
  record_check protocol-slskd-folder-contents-slskr ok "folder=open-commons-fixtures fixture=commons-click-track.ogg"
else
  record_check protocol-slskd-folder-contents-slskr fail "${target_directory:0:320}"
fi

auth_rust -H 'Content-Type: application/json' -d '{}' \
  "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse/request" >/dev/null
sleep 3
rust_browse="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse" 2>/dev/null || true)"
if [[ "$rust_browse" == *"commons-click-track.ogg"* ]]; then
  record_check protocol-slskr-browses-slskd ok "fixture=commons-click-track.ogg"
else
  record_check protocol-slskr-browses-slskd fail "${rust_browse:0:320}"
fi

auth_rust -H 'Content-Type: application/json' -d '{"folder":"open-commons-fixtures"}' \
  "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse/folder" >/dev/null
rust_folder_status=""
rust_folder_entries=""
for _ in $(seq 1 15); do
  sleep 1
  rust_folder_status="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse/status" 2>/dev/null || true)"
  rust_folder_entries="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/users/$slskd_username/browse" 2>/dev/null || true)"
  if [[ "$rust_folder_status" == *'"status":"ready"'* \
    && "$rust_folder_status" == *'"folder":"open-commons-fixtures"'* \
    && "$rust_folder_entries" == *"commons-click-track.ogg"* ]]; then
    break
  fi
done
if [[ "$rust_folder_status" == *'"status":"ready"'* \
  && "$rust_folder_status" == *'"folder":"open-commons-fixtures"'* \
  && "$rust_folder_entries" == *"commons-click-track.ogg"* ]]; then
  record_check protocol-slskr-folder-contents-slskd ok "folder=open-commons-fixtures fixture=commons-click-track.ogg"
else
  record_check protocol-slskr-folder-contents-slskd fail "status=${rust_folder_status:0:220} entries=${rust_folder_entries:0:220}"
fi

search_log="$work_dir/slskr-search.log"
set +e
SLSK_SERVER="$server_endpoint" \
SLSK_USERNAME="$upstream_username" \
SLSK_PASSWORD="$upstream_password" \
SLSK_PEER_USERNAME="$slskd_username" \
SLSK_SEARCH_QUERY="commons-click-track.ogg" \
SLSK_SEARCH_EXPECTED="commons-click-track.ogg" \
SLSK_SEARCH_HOST_OVERRIDE=127.0.0.1 \
SLSK_SEARCH_PORT_OVERRIDE="$slskd_listen_port" \
SLSK_SEARCH_FORCE_LOGIN=true \
SLSK_SEARCH_PROBE_ATTEMPTS=2 \
SLSK_SEARCH_PROBE_TIMEOUT_SECONDS=20 \
  "$slskr_binary" probe search-peer >"$search_log" 2>&1
search_status=$?
set -e
if [[ "$search_status" -eq 0 ]]; then
  record_check protocol-slskr-searches-slskd ok "query=commons-click-track.ogg expected=commons-click-track.ogg"
else
  record_check protocol-slskr-searches-slskd fail "status=$search_status detail=$(tail -n 4 "$search_log" | tr '\n\t' '  ')"
fi

target_search_created="$(auth_slskd -X POST -H 'Content-Type: application/json' \
  -d '{"searchText":"commons-click-track.ogg","searchTimeout":15000,"responseLimit":100}' \
  "http://127.0.0.1:$slskd_http_port/api/v0/searches" 2>/dev/null || true)"
target_search_id="$(printf '%s' "$target_search_created" | json_field id 2>/dev/null || true)"
target_search_ok=false
target_search_body=""
if [[ -n "$target_search_id" ]]; then
  for _ in $(seq 1 20); do
    target_search_body="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/searches/$target_search_id?includeResponses=true" 2>/dev/null || true)"
    if [[ "$target_search_body" == *"$slskr_username"* && "$target_search_body" == *"commons-click-track.ogg"* ]]; then
      target_search_ok=true
      break
    fi
    sleep 1
  done
fi
if [[ "$target_search_ok" == true ]]; then
  record_check protocol-slskd-searches-slskr ok "query=commons-click-track.ogg peer=$slskr_username"
else
  search_diagnostics="$(rust_search_diagnostics 'commons-click-track.ogg')"
  record_check protocol-slskd-searches-slskr fail "search_id=${target_search_id:-missing} response=${target_search_body:0:320} rust_diagnostics=${search_diagnostics:-none}"
fi

target_message="slskd-to-slskr-$(date +%s%N)"
target_status="$(curl -sS --max-time 10 -o "$work_dir/target-message.json" -w '%{http_code}' \
  -H 'Content-Type: application/json' -d "\"$target_message\"" \
  "http://127.0.0.1:$slskd_http_port/api/v0/conversations/$slskr_username" || true)"
sleep 2
rust_messages="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/v0/messages/$slskd_username" 2>/dev/null || true)"
if [[ "$target_status" == 2* && "$rust_messages" == *"$target_message"* ]]; then
  record_check protocol-slskd-message-dispatch ok "matched=$target_message"
else
  record_check protocol-slskd-message-dispatch fail "status=$target_status"
fi

rust_message="slskr-to-slskd-$(date +%s%N)"
rust_status="$(curl -sS --max-time 10 -o "$work_dir/rust-message.json" -w '%{http_code}' \
  -H "Authorization: Bearer $api_token" -H 'Content-Type: application/json' \
  -d "{\"username\":\"$slskd_username\",\"body\":\"$rust_message\"}" \
  "http://127.0.0.1:$slskr_http_port/api/v0/messages" || true)"
sleep 2
target_messages="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/conversations/$slskr_username/messages" 2>/dev/null || true)"
if [[ "$rust_status" == 2* && "$target_messages" == *"$rust_message"* ]]; then
  record_check protocol-slskr-message-dispatch-slskd ok "matched=$rust_message"
else
  record_check protocol-slskr-message-dispatch-slskd fail "status=$rust_status"
fi

if kill -0 "$slskd_pid" 2>/dev/null; then
  kill "$slskd_pid" 2>/dev/null || true
  wait "$slskd_pid" 2>/dev/null || true
fi
SLSKD_APP_DIR="$slskd_app" \
SLSKD_HTTP_PORT="$slskd_http_port" \
SLSKD_HTTP_IP_ADDRESS=127.0.0.1 \
SLSKD_NO_HTTPS=true \
SLSKD_NO_AUTH=true \
SLSKD_NO_LOGO=true \
SLSKD_NO_VERSION_CHECK=true \
SLSKD_INCOMPLETE_DIR="$slskd_app/incomplete" \
SLSKD_DOWNLOADS_DIR="$slskd_app/downloads" \
SLSKD_SHARED_DIR="$slskd_share" \
SLSKD_SLSK_ADDRESS="${server_endpoint%:*}" \
SLSKD_SLSK_PORT="${server_endpoint##*:}" \
SLSKD_SLSK_USERNAME="$slskd_username" \
SLSKD_SLSK_PASSWORD="$slskd_password" \
SLSKD_SLSK_LISTEN_IP_ADDRESS=0.0.0.0 \
SLSKD_SLSK_LISTEN_PORT="$slskd_listen_port" \
SLSKD_SLSK_DNET_LOGGING=true \
SLSKD_TEST_USER_ENDPOINT_OVERRIDES="$slskd_endpoint_overrides" \
  "$binary" >>"$slskd_log" 2>&1 &
slskd_pid="$!"

restart_target_state=""
for _ in $(seq 1 30); do
  restart_target_state="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/application" 2>/dev/null || true)"
  if [[ "$restart_target_state" == *'Connected, LoggedIn'* ]]; then
    break
  fi
  sleep 1
done
if [[ "$restart_target_state" == *'Connected, LoggedIn'* ]]; then
  record_check runtime-slskd-restart-session ok "state=connected persisted_app=$slskd_app"
else
  record_check runtime-slskd-restart-session fail "application=${restart_target_state:0:240}"
fi
restart_browse="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/browse" 2>/dev/null || true)"
if [[ "$restart_browse" == *"commons-click-track.ogg"* ]]; then
  record_check protocol-slskd-restart-browse ok "fixture=commons-click-track.ogg persisted_share_cache=true"
else
  record_check protocol-slskd-restart-browse fail "${restart_browse:0:320}"
fi
restart_directory="$(auth_slskd -X POST -H 'Content-Type: application/json' \
  -d '{"directory":"open-commons-fixtures"}' \
  "http://127.0.0.1:$slskd_http_port/api/v0/users/$slskr_username/directory" 2>/dev/null || true)"
if [[ "$restart_directory" == *"commons-click-track.ogg"* ]]; then
  record_check protocol-slskd-restart-folder ok "folder=open-commons-fixtures fixture=commons-click-track.ogg persisted_share_cache=true"
else
  record_check protocol-slskd-restart-folder fail "${restart_directory:0:320}"
fi

room_name="${SLSKR_CROSS_CLIENT_ROOM_NAME:-slskr-live-interop}"
room_target_join_status="$(curl -sS --max-time 20 -o "$work_dir/target-room-join.json" -w '%{http_code}' \
  -H 'Content-Type: application/json' -d "\"$room_name\"" \
  "http://127.0.0.1:$slskd_http_port/api/v0/rooms/joined" || true)"
room_rust_join_status="$(auth_rust -X POST -o "$work_dir/rust-room-join.json" -w '%{http_code}' -d '{}' \
  "http://127.0.0.1:$slskr_http_port/api/v0/rooms/$room_name/join" 2>/dev/null || true)"
sleep 3
target_room_message="slskd-room-to-slskr-$(date +%s%N)"
target_room_message_status="$(curl -sS --max-time 20 -o "$work_dir/target-room-message.json" -w '%{http_code}' \
  -H 'Content-Type: application/json' -d "\"$target_room_message\"" \
  "http://127.0.0.1:$slskd_http_port/api/v0/rooms/joined/$room_name/messages" || true)"
rust_room_messages=""
for _ in $(seq 1 15); do
  rust_room_messages="$(auth_rust "http://127.0.0.1:$slskr_http_port/api/rooms/joined/$room_name/messages" 2>/dev/null || true)"
  if [[ "$rust_room_messages" == *"$target_room_message"* ]]; then
    break
  fi
  sleep 1
done
if [[ "$room_target_join_status" == 2* && "$room_rust_join_status" == 2* && "$target_room_message_status" == 2* && "$rust_room_messages" == *"$target_room_message"* ]]; then
  record_check protocol-slskd-public-room ok "room=$room_name matched=$target_room_message"
else
  record_check protocol-slskd-public-room fail "join_target=$room_target_join_status join_rust=$room_rust_join_status message=$target_room_message_status"
fi

rust_room_message="slskr-room-to-slskd-$(date +%s%N)"
rust_room_message_status="$(auth_rust -X POST -H 'Content-Type: application/json' -d "{\"body\":\"$rust_room_message\"}" -o "$work_dir/rust-room-message.json" -w '%{http_code}' \
  "http://127.0.0.1:$slskr_http_port/api/rooms/joined/$room_name/messages" 2>/dev/null || true)"
target_room_messages=""
for _ in $(seq 1 15); do
  target_room_messages="$(auth_slskd "http://127.0.0.1:$slskd_http_port/api/v0/rooms/joined/$room_name/messages" 2>/dev/null || true)"
  if [[ "$target_room_messages" == *"$rust_room_message"* ]]; then
    break
  fi
  sleep 1
done
if [[ "$rust_room_message_status" == 2* && "$target_room_messages" == *"$rust_room_message"* ]]; then
  record_check protocol-slskr-public-room ok "room=$room_name matched=$rust_room_message"
else
  record_check protocol-slskr-public-room fail "status=$rust_room_message_status target_messages=${target_room_messages:0:320}"
fi

if [[ "$upstream_username" != "$slskr_username" && "$upstream_username" != "$slskd_username" ]]; then
  download_log="$work_dir/slskr-download.log"
  set +e
  SLSK_SERVER="$server_endpoint" \
  SLSK_USERNAME="$upstream_username" \
  SLSK_PASSWORD="$upstream_password" \
  SLSK_PEER_USERNAME="$slskd_username" \
  SLSK_DOWNLOAD_FILENAME="$fixture_remote_path" \
  SLSK_DOWNLOAD_SHA256="$fixture_sha256" \
  SLSK_DOWNLOAD_HOST_OVERRIDE=127.0.0.1 \
  SLSK_DOWNLOAD_LISTENER_BIND="0.0.0.0:$slskr_probe_listen_port" \
  SLSK_DOWNLOAD_ADVERTISED_PORT="$slskr_probe_listen_port" \
    "$slskr_binary" probe download-peer >"$download_log" 2>&1
  download_status=$?
  set -e
  if [[ "$download_status" -eq 0 ]]; then
    record_check slskr-to-slskd-download ok "fixture=commons-click-track.ogg sha256=$fixture_sha256"
  else
    record_check slskr-to-slskd-download fail "status=$download_status detail=$(tail -n 4 "$download_log" | tr '\n\t' '  ')"
  fi
else
  record_check slskr-to-slskd-download fail "distinct probe credentials are required"
fi

target_download_file=""
target_download_status="$(curl -sS --max-time 10 -o "$work_dir/target-download.json" -w '%{http_code}' \
  -H 'Content-Type: application/json' -d "[{\"filename\":\"$fixture_remote_path_json\",\"size\":$fixture_size}]" \
  "http://127.0.0.1:$slskd_http_port/api/v0/transfers/downloads/$slskr_username" || true)"
target_download_ok=false
target_download_sha256=""
for _ in $(seq 1 30); do
  target_download_file="$(find "$slskd_app/downloads" -type f -name 'commons-click-track.ogg' -print -quit)"
  if [[ -n "$target_download_file" ]]; then
    target_download_sha256="$(sha256sum "$target_download_file" | awk '{print $1}')"
    if [[ "$target_download_sha256" == "$fixture_sha256" ]]; then
      target_download_ok=true
      break
    fi
  fi
  sleep 1
done
if [[ "$target_download_status" == 2* && "$target_download_ok" == true ]]; then
  record_check slskd-to-slskr-download ok "fixture=$fixture_remote_path sha256=$target_download_sha256"
else
  record_check slskd-to-slskr-download fail "status=$target_download_status path=${target_download_file:-missing} sha256=${target_download_sha256:-missing}"
fi

record_check interop-adapter-slskd ok "endpoint_override=bounded-test-only"
mv "$result_file" "$final_result_file"
echo "result_file=$final_result_file"
