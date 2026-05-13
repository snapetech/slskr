#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

env_files=(
  "${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
  "${SLSKR_SLSKDN_ENV_FILE:-$repo_root/../slskdn/.env}"
  "${SLSKR_SLSKDN_ACCOUNT_POOL_FILE:-$repo_root/../slskdn/tests/slskd.Tests.Integration/local-mesh-account-pool.env}"
)

for env_file in "${env_files[@]}"; do
  if [[ -f "$env_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$env_file"
    set +a
  fi
done

api_token="${SLSKR_CROSS_CLIENT_API_TOKEN:-slskr-cross-client-interop}"
timeout_seconds="${SLSKR_CROSS_CLIENT_TIMEOUT_SECONDS:-240}"
soak_seconds="${SLSKR_CROSS_CLIENT_SOAK_SECONDS:-30}"
work_dir="${SLSKR_CROSS_CLIENT_WORK_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-slskdn-interop.XXXXXX")}"
output_dir="${SLSKR_INTEROP_OUTPUT_DIR:-$repo_root/target/live-interop}"
server_host="${SLSKR_CROSS_CLIENT_SERVER_HOST:-${SLSK_SERVER_ADDRESS:-vps.slsknet.org}}"
server_port="${SLSKR_CROSS_CLIENT_SERVER_PORT:-${SLSK_SERVER_PORT:-2271}}"
server_endpoint="${SLSKR_CROSS_CLIENT_SERVER:-$server_host:$server_port}"
mkdir -p "$work_dir" "$output_dir"
result_file="$output_dir/slskr-slskdn-cross-client-interop.tsv"
diag_file="$work_dir/diagnostics.log"

pick_free_port() {
  node -e "const net=require('net'); const s=net.createServer(); s.listen(0,'127.0.0.1',()=>{console.log(s.address().port); s.close();});"
}

picked_ports=()
pick_port() {
  local port
  while true; do
    port="$(pick_free_port)"
    if [[ " ${picked_ports[*]} " != *" ${port} "* ]]; then
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

json_get() {
  local expression="$1"
  node -e "
const expression = process.argv[1];
let data = '';
process.stdin.on('data', chunk => data += chunk);
process.stdin.on('end', () => {
  const root = JSON.parse(data);
  const parts = expression.split('.').filter(Boolean);
  let value = root;
  for (const part of parts) {
    if (value === undefined || value === null) process.exit(1);
    value = Array.isArray(value) && /^[0-9]+$/.test(part) ? value[Number(part)] : value[part];
  }
  if (value === undefined || value === null) process.exit(1);
  process.stdout.write(String(value));
});
" "$expression"
}

json_find_string() {
  local needle="$1"
  node -e "
const needle = process.argv[1];
let data = '';
process.stdin.on('data', chunk => data += chunk);
process.stdin.on('end', () => {
  const root = JSON.parse(data);
  const visit = value => {
    if (typeof value === 'string') return value.includes(needle);
    if (Array.isArray(value)) return value.some(visit);
    if (value && typeof value === 'object') return Object.values(value).some(visit);
    return false;
  };
  process.exit(visit(root) ? 0 : 1);
});
" "$needle"
}

auth_get() {
  local url="$1"
  curl -fsS -H "Authorization: Bearer $api_token" -H "X-API-Key: integration-test" "$url"
}

auth_post_json() {
  local url="$1"
  local payload="$2"
  curl -fsS -H "Authorization: Bearer $api_token" -H "X-API-Key: integration-test" -H "Content-Type: application/json" -d "$payload" "$url"
}

auth_put_empty() {
  local url="$1"
  curl -fsS -X PUT -H "Authorization: Bearer $api_token" -H "X-API-Key: integration-test" "$url"
}

try_request() {
  local label="$1"
  shift
  local output status
  set +e
  output="$("$@" 2>&1)"
  status=$?
  set -e
  if [[ $status -ne 0 ]]; then
    echo "$label failed: $output" >&2
  fi
  return "$status"
}

record_check() {
  local check="$1"
  local status="$2"
  local detail="$3"
  printf '%s\t%s\t%s\t%s\n' "$(date -Is)" "$check" "$status" "$detail" | tee -a "$result_file"
}

wait_json_contains() {
  local label="$1"
  local url="$2"
  local needle="$3"
  local deadline=$((SECONDS + timeout_seconds))
  local body=""
  while ((SECONDS < deadline)); do
    if body="$(auth_get "$url" 2>/dev/null)" && printf '%s' "$body" | json_find_string "$needle" 2>/dev/null; then
      record_check "$label" ok "matched=$needle"
      return 0
    fi
    sleep 2
  done
  record_check "$label" fail "timeout waiting for $needle last=${body:-none}"
  return 1
}

wait_raw_contains() {
  local label="$1"
  local url="$2"
  local needle="$3"
  local deadline=$((SECONDS + timeout_seconds))
  local body=""
  while ((SECONDS < deadline)); do
    if body="$(auth_get "$url" 2>/dev/null)" && [[ "$body" == *"$needle"* ]]; then
      record_check "$label" ok "matched=$needle"
      return 0
    fi
    sleep 2
  done
  record_check "$label" fail "timeout waiting for $needle last=${body:-none}"
  return 1
}

url_escape() {
  node -e "process.stdout.write(encodeURIComponent(process.argv[1]));" "$1"
}

account_username() {
  local index="$1"
  local slskr_user="SLSKR_TEST_${index}_USERNAME"
  local slskdn_user="SLSKDN_MESH_ACCOUNT_${index}_USERNAME"
  local suffixes=(A B C D E F)
  if [[ -n "${!slskr_user:-}" ]]; then
    printf '%s' "${!slskr_user}"
    return 0
  fi
  if [[ "$index" =~ ^[0-9]+$ && "$index" -ge 1 && "$index" -le "${#suffixes[@]}" ]]; then
    slskdn_user="SLSKDN_MESH_ACCOUNT_${suffixes[$((index - 1))]}_USERNAME"
  fi
  printf '%s' "${!slskdn_user:-}"
}

account_password() {
  local index="$1"
  local slskr_pass="SLSKR_TEST_${index}_PASSWORD"
  local slskdn_pass="SLSKDN_MESH_ACCOUNT_${index}_PASSWORD"
  local suffixes=(A B C D E F)
  if [[ -n "${!slskr_pass:-}" ]]; then
    printf '%s' "${!slskr_pass}"
    return 0
  fi
  if [[ "$index" =~ ^[0-9]+$ && "$index" -ge 1 && "$index" -le "${#suffixes[@]}" ]]; then
    slskdn_pass="SLSKDN_MESH_ACCOUNT_${suffixes[$((index - 1))]}_PASSWORD"
  fi
  printf '%s' "${!slskdn_pass:-}"
}

slskr_index="${SLSKR_CROSS_CLIENT_SLSKR_INDEX:-1}"
slskdn_index="${SLSKR_CROSS_CLIENT_SLSKDN_INDEX:-2}"
upstream_index="${SLSKR_CROSS_CLIENT_UPSTREAM_INDEX:-3}"
slskr_username="${SLSKR_CROSS_CLIENT_SLSKR_USERNAME:-$(account_username "$slskr_index")}"
slskr_password="${SLSKR_CROSS_CLIENT_SLSKR_PASSWORD:-$(account_password "$slskr_index")}"
slskdn_username="${SLSKR_CROSS_CLIENT_SLSKDN_USERNAME:-$(account_username "$slskdn_index")}"
slskdn_password="${SLSKR_CROSS_CLIENT_SLSKDN_PASSWORD:-$(account_password "$slskdn_index")}"
upstream_username="${SLSKR_CROSS_CLIENT_UPSTREAM_USERNAME:-$(account_username "$upstream_index")}"
upstream_password="${SLSKR_CROSS_CLIENT_UPSTREAM_PASSWORD:-$(account_password "$upstream_index")}"

if [[ -z "$slskr_username" || -z "$slskr_password" || -z "$slskdn_username" || -z "$slskdn_password" ]]; then
  echo "missing cross-client credentials; set SLSKR_TEST_1/2_USERNAME/PASSWORD or slskdN local mesh account pool credentials" >&2
  exit 2
fi
if [[ "$slskr_username" == "$slskdn_username" ]]; then
  echo "slskr and slskdN users must be distinct" >&2
  exit 2
fi

discover_slskdn_binary() {
  local candidates=()
  if [[ -n "${SLSKDN_BINARY_PATH:-}" ]]; then
    candidates+=("$SLSKDN_BINARY_PATH")
  fi
  candidates+=(
    "$repo_root/../slskdn/src/slskd/bin/Release/net10.0/slskd"
    "$repo_root/../slskdn/src/slskd/bin/Debug/net10.0/slskd"
    "$repo_root/../slskdn/dist/linux-x64/slskd"
    "$repo_root/../slskdn/publish/slskd"
  )
  for candidate in "${candidates[@]}"; do
    if [[ -x "$candidate" ]]; then
      printf '%s' "$candidate"
      return 0
    fi
  done
  return 1
}

slskdn_binary_has_endpoint_overrides() {
  local binary="$1"
  local dll="${binary}.dll"
  if grep -Fq "SLSKDN_TEST_USER_ENDPOINT_OVERRIDES" < <(strings "$binary" 2>/dev/null); then
    return 0
  fi
  if grep -Fq "SLSKDN_TEST_USER_ENDPOINT_OVERRIDES" < <(strings -el "$binary" 2>/dev/null); then
    return 0
  fi
  if [[ -f "$dll" ]] && grep -Fq "SLSKDN_TEST_USER_ENDPOINT_OVERRIDES" < <(strings "$dll" 2>/dev/null); then
    return 0
  fi
  if [[ -f "$dll" ]] && grep -Fq "SLSKDN_TEST_USER_ENDPOINT_OVERRIDES" < <(strings -el "$dll" 2>/dev/null); then
    return 0
  fi
  return 1
}

build_slskdn_binary() {
  local slskdn_root="$repo_root/../slskdn"
  local project="$slskdn_root/src/slskd/slskd.csproj"
  if [[ ! -f "$project" ]]; then
    return 1
  fi
  echo "building slskdN interop binary with endpoint override support" >&2
  dotnet build "$project" -c Release >/dev/null
}

slskdn_binary="$(discover_slskdn_binary || true)"
if [[ -z "$slskdn_binary" ]]; then
  echo "slskdN binary not found; set SLSKDN_BINARY_PATH or build ../slskdn" >&2
  exit 2
fi
if ! slskdn_binary_has_endpoint_overrides "$slskdn_binary"; then
  build_slskdn_binary || {
    echo "slskdN binary lacks SLSKDN_TEST_USER_ENDPOINT_OVERRIDES support and rebuild failed" >&2
    exit 2
  }
  slskdn_binary="$(discover_slskdn_binary || true)"
  if [[ -z "$slskdn_binary" ]] || ! slskdn_binary_has_endpoint_overrides "$slskdn_binary"; then
    echo "rebuilt slskdN binary still lacks SLSKDN_TEST_USER_ENDPOINT_OVERRIDES support" >&2
    exit 2
  fi
fi

slskr_http_port="$(pick_port)"
slskr_listen_port="${SLSKR_CROSS_CLIENT_SLSKR_LISTEN_PORT:-$(pick_port)}"
slskdn_http_port="$(pick_port)"
slskdn_listen_port="${SLSKR_CROSS_CLIENT_SLSKDN_LISTEN_PORT:-$(pick_port)}"

slskr_state="$work_dir/slskr-state"
slskr_share="$work_dir/slskr-share"
slskdn_app="$work_dir/slskdn-app"
slskdn_share="$slskdn_app/shares"
mkdir -p "$slskr_state" "$slskr_share" "$slskdn_app/config" "$slskdn_app/downloads" "$slskdn_app/incomplete" "$slskdn_share"

slskr_fixture_name="slskr-to-slskdn-$(date -u +%Y%m%d%H%M%S).flac"
slskdn_fixture_name="slskdn-to-slskr-$(date -u +%Y%m%d%H%M%S).flac"
printf 'slskr fixture %s\n' "$(date -u +%FT%TZ)" >"$slskr_share/$slskr_fixture_name"
printf 'slskdn fixture %s\n' "$(date -u +%FT%TZ)" >"$slskdn_share/$slskdn_fixture_name"
slskr_fixture_size="$(wc -c <"$slskr_share/$slskr_fixture_name" | tr -d ' ')"
slskdn_fixture_size="$(wc -c <"$slskdn_share/$slskdn_fixture_name" | tr -d ' ')"
slskr_fixture_sha="$(sha256sum "$slskr_share/$slskr_fixture_name" | awk '{print $1}')"
slskdn_fixture_sha="$(sha256sum "$slskdn_share/$slskdn_fixture_name" | awk '{print $1}')"
slskr_remote_filename="$(basename "$slskr_share")/$slskr_fixture_name"
slskdn_remote_filename="shares\\\\$slskdn_fixture_name"

slskr_log="$work_dir/slskr.log"
slskdn_log="$work_dir/slskdn.log"
slskr_pid=""
slskdn_pid=""

cleanup() {
  for pid in "$slskr_pid" "$slskdn_pid"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}
trap cleanup EXIT

cat >"$slskdn_app/config/slskd.yml" <<YAML
debug: true
web:
  port: $slskdn_http_port
  address: 127.0.0.1
  https:
    disabled: true
    force: false
  authentication:
    disabled: true
    username: admin
    password: admin
directories:
  downloads: $slskdn_app/downloads
  incomplete: $slskdn_app/incomplete
shares:
  directories:
    - $slskdn_share
  cache:
    storage_mode: disk
feature:
  identityFriends: true
  collectionsSharing: true
  streaming: true
soulseek:
  address: $server_host
  port: $server_port
  diagnostic_level: debug
  username: "$slskdn_username"
  password: "$slskdn_password"
  listen_ip_address: 0.0.0.0
  listen_port: $slskdn_listen_port
flags:
  no_connect: false
YAML

(
  export SLSK_SERVER="$server_endpoint"
  export SLSKR_HTTP_BIND="127.0.0.1:$slskr_http_port"
  export SLSKR_STATE_DIR="$slskr_state"
  export SLSK_USERNAME="$slskr_username"
  export SLSK_PASSWORD="$slskr_password"
  export SLSKR_AUTO_CONNECT=true
  export SLSKR_RECONNECT=false
  export SLSKR_AUTH_DISABLED=false
  export SLSKR_API_TOKEN="$api_token"
  export SLSKR_SHARE_DIRS="$slskr_share"
  export SLSKR_LISTENER_BIND="127.0.0.1:$slskr_listen_port"
  export SLSK_LISTEN_PORT="$slskr_listen_port"
  export SLSKR_ADVERTISED_PORT="$slskr_listen_port"
  export SLSKR_PEER_HOST_OVERRIDE=127.0.0.1
  export SLSKR_TEST_USER_ENDPOINT_OVERRIDES="$slskdn_username=127.0.0.1:$slskdn_listen_port"
  export SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS=15
  exec cargo run -q -p slskr -- serve
) >"$slskr_log" 2>&1 &
slskr_pid="$!"

(
  export APP_DIR="$slskdn_app"
  export SLSKDN_TEST_USER_ENDPOINT_OVERRIDES="$slskr_username=127.0.0.1:$slskr_listen_port"
  exec "$slskdn_binary" --config "$slskdn_app/config/slskd.yml" --app-dir "$slskdn_app"
) >"$slskdn_log" 2>&1 &
slskdn_pid="$!"

{
  printf 'server_endpoint=%s\n' "$server_endpoint"
  printf 'slskr_http=127.0.0.1:%s slskr_listen=127.0.0.1:%s\n' "$slskr_http_port" "$slskr_listen_port"
  printf 'slskdn_http=127.0.0.1:%s slskdn_listen=127.0.0.1:%s\n' "$slskdn_http_port" "$slskdn_listen_port"
  printf 'slskr_endpoint_override=%s=127.0.0.1:%s\n' "$slskdn_username" "$slskdn_listen_port"
  printf 'slskdn_endpoint_override=%s=127.0.0.1:%s\n' "$slskr_username" "$slskr_listen_port"
} >"$diag_file"

wait_slskr_connected() {
  local deadline=$((SECONDS + timeout_seconds))
  local session
  while ((SECONDS < deadline)); do
    if session="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" 2>/dev/null)"; then
      if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
        echo "slskr connected"
        return 0
      fi
    fi
    sleep 2
  done
  echo "slskr did not connect" >&2
  tail -n 120 "$slskr_log" >&2 || true
  return 1
}

wait_slskdn_connected() {
  local deadline=$((SECONDS + timeout_seconds))
  local app
  while ((SECONDS < deadline)); do
    if app="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null)"; then
      if [[ "$(printf '%s' "$app" | json_get server.isLoggedIn 2>/dev/null || true)" == "true" ]]; then
        echo "slskdN connected"
        return 0
      fi
    fi
    sleep 2
  done
  echo "slskdN did not connect" >&2
  tail -n 120 "$slskdn_log" >&2 || true
  return 1
}

wait_slskr_connected
wait_slskdn_connected

{
  printf '\n[session]\n'
  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
  printf '\n[listeners]\n'
  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
  printf '\n[slskdn-application]\n'
  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" || true
  printf '\n[slskdn-endpoint:slskr]\n'
  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
} >>"$diag_file" 2>&1

try_request slskr-share-rescan auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/shares/rescan" '{}' >/dev/null || true
try_request slskdn-share-rescan auth_put_empty "http://127.0.0.1:$slskdn_http_port/api/v0/shares" >/dev/null \
  || try_request slskdn-share-rescan-post auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/shares" '{}' >/dev/null \
  || true
sleep "${SLSKR_CROSS_CLIENT_PEER_SETTLE_SECONDS:-8}"

wait_for_file() {
  local path="$1"
  local expected_sha="$2"
  local deadline=$((SECONDS + timeout_seconds))
  while ((SECONDS < deadline)); do
    if [[ -f "$path" ]]; then
      local actual_sha
      actual_sha="$(sha256sum "$path" | awk '{print $1}')"
      if [[ "$actual_sha" == "$expected_sha" ]]; then
        return 0
      fi
    fi
    sleep 2
  done
  return 1
}

probe_peer_address() {
  local label="$1"
  local peer_username="$2"
  if [[ -z "$upstream_username" || -z "$upstream_password" ]]; then
    printf '[peer-address:%s] skipped: no upstream probe credentials\n' "$label" >>"$diag_file"
    return 0
  fi
  {
    printf '\n[peer-address:%s]\n' "$label"
    SLSK_USERNAME="$upstream_username" \
    SLSK_PASSWORD="$upstream_password" \
    SLSK_SERVER="$server_endpoint" \
    SLSK_PEER_USERNAME="$peer_username" \
    SLSK_PEER_ADDRESS_PROBE_ATTEMPTS=1 \
    SLSK_PEER_ADDRESS_PROBE_TIMEOUT_SECONDS=15 \
      timeout 45 cargo run -q -p slskr -- probe peer-address
  } >>"$diag_file" 2>&1 || {
    printf '[peer-address:%s] failed\n' "$label" >>"$diag_file"
    return 1
  }
}

printf 'timestamp\tcheck\tstatus\tdetail\n' >"$result_file"

run_runtime_protocol_checks() {
  local session listeners app endpoint escaped_slskr escaped_slskdn
  session="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session")"
  if [[ "$(printf '%s' "$session" | json_get state 2>/dev/null || true)" == "connected" ]]; then
    record_check runtime-slskr-session ok "state=connected"
  else
    record_check runtime-slskr-session fail "$session"
    return 1
  fi

  listeners="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners")"
  if [[ "$listeners" == *"$slskr_listen_port"* ]]; then
    record_check network-slskr-listener ok "port=$slskr_listen_port"
  else
    record_check network-slskr-listener fail "$listeners"
    return 1
  fi

  app="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application")"
  if [[ "$(printf '%s' "$app" | json_get server.isLoggedIn 2>/dev/null || true)" == "true" ]]; then
    record_check runtime-slskdn-session ok "server.isLoggedIn=true"
  else
    record_check runtime-slskdn-session fail "$app"
    return 1
  fi
  if [[ "$app" == *"$slskdn_fixture_name"* || "$app" == *"\"files\":1"* ]]; then
    record_check runtime-slskdn-shares ok "fixture=$slskdn_fixture_name"
  else
    record_check runtime-slskdn-shares fail "$app"
    return 1
  fi

  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"
  endpoint="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/endpoint")"
  if [[ "$endpoint" == *"$slskr_listen_port"* ]]; then
    record_check network-slskdn-resolves-slskr ok "endpoint=$endpoint"
  else
    record_check network-slskdn-resolves-slskr fail "$endpoint"
    return 1
  fi

  endpoint="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/endpoint")"
  if [[ "$endpoint" == *"$slskdn_listen_port"* ]]; then
    record_check network-slskr-resolves-slskdn ok "endpoint=$endpoint"
  else
    record_check network-slskr-resolves-slskdn fail "$endpoint"
    return 1
  fi
}

run_browse_interop_checks() {
  local escaped_slskr escaped_slskdn body
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"

  body="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse")"
  if printf '%s' "$body" | json_find_string "$slskr_fixture_name" 2>/dev/null; then
    record_check protocol-slskdn-browses-slskr ok "fixture=$slskr_fixture_name"
  else
    record_check protocol-slskdn-browses-slskr fail "$body"
    return 1
  fi

  auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/request" '{}' >/dev/null
  wait_json_contains protocol-slskr-browses-slskdn "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse" "$slskdn_fixture_name"
}

run_search_interop_checks() {
  local escaped_slskr escaped_slskdn
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"

  if SLSK_USERNAME="$slskr_username" \
    SLSK_PASSWORD="$slskr_password" \
    SLSK_SERVER="$server_endpoint" \
    SLSK_PEER_USERNAME="$slskdn_username" \
    SLSK_SEARCH_QUERY="$slskdn_fixture_name" \
    SLSK_SEARCH_EXPECTED="$slskdn_fixture_name" \
    SLSK_SEARCH_HOST_OVERRIDE=127.0.0.1 \
    SLSK_SEARCH_PROBE_TIMEOUT_SECONDS=20 \
      timeout 45 cargo run -q -p slskr -- probe search-peer >>"$diag_file" 2>&1; then
    record_check protocol-slskr-searches-slskdn ok "query=$slskdn_fixture_name"
  else
    record_check protocol-slskr-searches-slskdn fail "$(tail -n 1 "$diag_file")"
    return 1
  fi

  if SLSK_USERNAME="$slskdn_username" \
    SLSK_PASSWORD="$slskdn_password" \
    SLSK_SERVER="$server_endpoint" \
    SLSK_PEER_USERNAME="$slskr_username" \
    SLSK_SEARCH_QUERY="$slskr_fixture_name" \
    SLSK_SEARCH_EXPECTED="$slskr_fixture_name" \
    SLSK_SEARCH_HOST_OVERRIDE=127.0.0.1 \
    SLSK_SEARCH_PROBE_TIMEOUT_SECONDS=20 \
      timeout 45 cargo run -q -p slskr -- probe search-peer >>"$diag_file" 2>&1; then
    record_check protocol-slskdn-searches-slskr ok "query=$slskr_fixture_name"
  else
    record_check protocol-slskdn-searches-slskr fail "$(tail -n 1 "$diag_file")"
    return 1
  fi

  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/status" >>"$diag_file" 2>&1 || true
  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse/status" >>"$diag_file" 2>&1 || true
}

run_message_interop_checks() {
  local escaped_slskr escaped_slskdn slskr_message slskdn_message
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"
  slskr_message="slskr-to-slskdn-message-$(date -u +%Y%m%d%H%M%S)"
  slskdn_message="slskdn-to-slskr-message-$(date -u +%Y%m%d%H%M%S)"

  auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/messages" "{\"username\":\"$slskdn_username\",\"body\":\"$slskr_message\"}" >/dev/null
  wait_json_contains protocol-slskr-message-dispatch "http://127.0.0.1:$slskr_http_port/api/v0/messages/$escaped_slskdn" "$slskr_message"

  auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/conversations/$escaped_slskr" "\"$slskdn_message\"" >/dev/null
  record_check protocol-slskdn-message-dispatch ok "target=$slskr_username"

  SLSK_USERNAME="$slskr_username" \
  SLSK_PASSWORD="$slskr_password" \
  SLSK_MESSAGE_USERNAME="$slskdn_username" \
  SLSK_MESSAGE_PASSWORD="$slskdn_password" \
  SLSK_SERVER="$server_endpoint" \
  SLSK_MESSAGE_PROBE_TIMEOUT_SECONDS=30 \
    timeout 60 cargo run -q -p slskr -- probe private-message >>"$diag_file" 2>&1
  record_check protocol-private-message-server-roundtrip ok "sender=$slskr_username receiver=$slskdn_username"
}

run_mesh_runtime_checks() {
  local health stats transport ticket
  health="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/health")"
  record_check runtime-slskdn-mesh-health ok "$(printf '%s' "$health" | tr '\n\t' '  ')"

  stats="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/stats")"
  record_check runtime-slskdn-mesh-stats ok "$(printf '%s' "$stats" | tr '\n\t' '  ')"

  transport="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/transport")"
  record_check network-slskdn-mesh-transport ok "$(printf '%s' "$transport" | tr '\n\t' '  ')"

  ticket="$(auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/mesh-streams/tickets" "{\"contentId\":\"interop-content\",\"peerId\":\"$slskr_username\",\"filename\":\"Interop/Test.flac\",\"expectedSize\":0}")"
  if [[ "$ticket" == *"\"source\":\"mesh\""* && "$ticket" == *"streamUrl"* ]]; then
    record_check runtime-slskdn-mesh-stream-ticket ok "$ticket"
  else
    record_check runtime-slskdn-mesh-stream-ticket fail "$ticket"
    return 1
  fi

  ticket="$(auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/mesh-streams/tickets" "{\"contentId\":\"interop-content\",\"filename\":\"Interop/Test.flac\",\"peerId\":\"$slskdn_username\"}")"
  if [[ "$ticket" == *"streamUrl"* ]]; then
    record_check runtime-slskr-mesh-stream-ticket ok "$ticket"
  else
    record_check runtime-slskr-mesh-stream-ticket fail "$ticket"
    return 1
  fi
}

probe_peer_address slskr "$slskr_username" || true
probe_peer_address slskdn "$slskdn_username" || true

run_slskdn_to_slskr_download() {
  local created transfer_id status bytes transfer_json download_path
  if ! created="$(auth_post_json \
      "http://127.0.0.1:$slskr_http_port/api/v0/transfers" \
      "{\"peer_username\":\"$slskdn_username\",\"filename\":\"$slskdn_remote_filename\",\"size\":$slskdn_fixture_size}" 2>&1)"; then
    printf '%s\tslskdn-to-slskr-download\tfail\tcreate failed: %s\n' "$(date -Is)" "$created" | tee -a "$result_file"
    return 1
  fi
  transfer_id="$(printf '%s' "$created" | json_get id)"
  auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/transfers/$transfer_id/start" '{}' >/dev/null
  local deadline=$((SECONDS + timeout_seconds))
  while ((SECONDS < deadline)); do
    transfer_json="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/transfers/$transfer_id")"
    status="$(printf '%s' "$transfer_json" | json_get status 2>/dev/null || true)"
    bytes="$(printf '%s' "$transfer_json" | json_get bytes_transferred 2>/dev/null || true)"
    if [[ "$status" == "succeeded" && "$bytes" == "$slskdn_fixture_size" ]]; then
      download_path="$slskr_state/downloads/shares\\$slskdn_fixture_name"
      wait_for_file "$download_path" "$slskdn_fixture_sha"
      printf '%s\tslskdn-to-slskr-download\tok\tbytes=%s sha256=%s\n' "$(date -Is)" "$bytes" "$slskdn_fixture_sha" | tee -a "$result_file"
      return 0
    fi
    if [[ "$status" == "failed" || "$status" == "cancelled" ]]; then
      printf '%s\tslskdn-to-slskr-download\tfail\t%s\n' "$(date -Is)" "$transfer_json" | tee -a "$result_file"
      return 1
    fi
    sleep 2
  done
  printf '%s\tslskdn-to-slskr-download\tfail\ttimeout last=%s\n' "$(date -Is)" "${transfer_json:-none}" | tee -a "$result_file"
  return 1
}

run_slskr_to_slskdn_download() {
  local escaped_user response download_path
  escaped_user="$(url_escape "$slskr_username")"
  response="$(auth_post_json \
    "http://127.0.0.1:$slskdn_http_port/api/v0/transfers/downloads/$escaped_user" \
    "[{\"filename\":\"$slskr_remote_filename\",\"size\":$slskr_fixture_size}]")"
  download_path="$slskdn_app/downloads/$slskr_remote_filename"
  if wait_for_file "$download_path" "$slskr_fixture_sha"; then
    printf '%s\tslskr-to-slskdn-download\tok\tbytes=%s sha256=%s response=%s\n' "$(date -Is)" "$slskr_fixture_size" "$slskr_fixture_sha" "$response" | tee -a "$result_file"
    return 0
  fi
  printf '%s\tslskr-to-slskdn-download\tfail\tdownload missing path=%s response=%s\n' "$(date -Is)" "$download_path" "$response" | tee -a "$result_file"
  return 1
}

record_final_diagnostics() {
  {
    printf '\n[final-session]\n'
    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session" || true
    printf '\n[final-listeners]\n'
    auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners" || true
    printf '\n[final-slskdn-endpoint:slskr]\n'
    auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$slskr_username/endpoint" || true
  } >>"$diag_file" 2>&1
}

status=0
run_runtime_protocol_checks || status=1
run_browse_interop_checks || status=1
run_slskr_to_slskdn_download || status=1
run_slskdn_to_slskr_download || status=1
run_search_interop_checks || status=1
run_message_interop_checks || status=1
run_mesh_runtime_checks || status=1
record_final_diagnostics

if ((soak_seconds > 0)); then
  sleep "$soak_seconds"
  slskr_session="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/session")"
  slskdn_app_json="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application")"
  [[ "$(printf '%s' "$slskr_session" | json_get state)" == "connected" ]]
  [[ "$(printf '%s' "$slskdn_app_json" | json_get server.isLoggedIn)" == "true" ]]
  printf '%s\tpost-transfer-soak\tok\tseconds=%s\n' "$(date -Is)" "$soak_seconds" | tee -a "$result_file"
fi

if [[ "$status" -ne 0 ]]; then
  echo "cross-client interop failed"
  echo "result_file=$result_file"
  echo "work_dir=$work_dir"
  exit "$status"
fi

echo "cross-client interop ok"
echo "result_file=$result_file"
echo "work_dir=$work_dir"
echo "slskr_user=$(redact "$slskr_username") slskdn_user=$(redact "$slskdn_username")"
