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
  node -e "const net=require('net'); const s=net.createServer(); s.listen(0,'0.0.0.0',()=>{console.log(s.address().port); s.close();});"
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
  if [[ "$url" == "http://127.0.0.1:$slskdn_http_port/"* ]]; then
    curl -fsS "$url"
  else
    curl -fsS -H "Authorization: Bearer $api_token" "$url"
  fi
}

auth_post_json() {
  local url="$1"
  local payload="$2"
  if [[ "$url" == "http://127.0.0.1:$slskdn_http_port/"* ]]; then
    curl -fsS -H "Content-Type: application/json" -d "$payload" "$url"
  else
    curl -fsS -H "Authorization: Bearer $api_token" -H "Content-Type: application/json" -d "$payload" "$url"
  fi
}

auth_put_empty() {
  local url="$1"
  if [[ "$url" == "http://127.0.0.1:$slskdn_http_port/"* ]]; then
    curl -fsS -X PUT "$url"
  else
    curl -fsS -X PUT -H "Authorization: Bearer $api_token" "$url"
  fi
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
slskdn_overlay_port="${SLSKR_CROSS_CLIENT_SLSKDN_OVERLAY_PORT:-$(pick_port)}"
gateway_echo_port="$(pick_port)"
gateway_echo_host="$(ip -4 route get 1.1.1.1 | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }')"

slskr_state="$work_dir/slskr-state"
slskr_share="$work_dir/slskr-share"
slskdn_app="$work_dir/slskdn-app"
slskdn_share="$slskdn_app/shares"
mkdir -p "$slskr_state" "$slskr_share" "$slskdn_app/config" "$slskdn_app/downloads" "$slskdn_app/incomplete" "$slskdn_share"

slskr_fixture_name="slskr-to-slskdn-$(date -u +%Y%m%d%H%M%S).flac"
slskdn_fixture_name="slskdn-to-slskr-$(date -u +%Y%m%d%H%M%S).flac"
printf 'slskr fixture %s\n' "$(date -u +%FT%TZ)" >"$slskr_share/$slskr_fixture_name"
printf 'fLaC\000\000\000\042' >"$slskdn_share/$slskdn_fixture_name"
dd if=/dev/zero bs=34 count=1 >>"$slskdn_share/$slskdn_fixture_name" 2>/dev/null
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
gateway_echo_pid=""

# Build before either daemon starts. slskdN's test endpoint overrides live in its
# bounded endpoint cache, so compiling through `cargo run` after launch can
# consume their useful lifetime before the cross-client checks begin.
cargo build -q -p slskr
slskr_binary="$repo_root/target/debug/slskr"

node -e '
const net = require("net");
const port = Number(process.argv[1]);
net.createServer(socket => socket.pipe(socket)).listen(port, "0.0.0.0");
' "$gateway_echo_port" >"$work_dir/gateway-echo.log" 2>&1 &
gateway_echo_pid="$!"

cleanup() {
  for pid in "$slskr_pid" "$slskdn_pid" "$gateway_echo_pid"; do
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
dht:
  enabled: true
  lan_only: true
  overlay_port: $slskdn_overlay_port
  advertised_overlay_port: $slskdn_overlay_port
  dht_port: $slskdn_overlay_port
overlay:
  enable: true
  listen_port: $slskdn_overlay_port
  quic_listen_port: $slskdn_overlay_port
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
  export SLSKR_RECONNECT=true
  export SLSKR_AUTH_DISABLED=false
  export SLSKR_API_TOKEN="$api_token"
  export SLSKR_SHARE_DIRS="$slskr_share"
  export SLSKR_LISTENER_BIND="127.0.0.1:$slskr_listen_port"
  export SLSK_LISTEN_PORT="$slskr_listen_port"
  export SLSKR_ADVERTISED_PORT="$slskr_listen_port"
  export SLSKR_PEER_HOST_OVERRIDE=127.0.0.1
  export SLSKR_TEST_USER_ENDPOINT_OVERRIDES="$slskdn_username=127.0.0.1:$slskdn_listen_port;$upstream_username=127.0.0.1:$slskdn_listen_port"
  export SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS=60
  exec "$slskr_binary" serve
) >"$slskr_log" 2>&1 &
slskr_pid="$!"

(
  export APP_DIR="$slskdn_app"
  export SLSKDN_TEST_USER_ENDPOINT_OVERRIDES="$slskr_username=127.0.0.1:$slskr_listen_port;$upstream_username=127.0.0.1:$slskr_listen_port"
  exec "$slskdn_binary" --config "$slskdn_app/config/slskd.yml" --app-dir "$slskdn_app"
) >"$slskdn_log" 2>&1 &
slskdn_pid="$!"

{
  printf 'server_endpoint=%s\n' "$server_endpoint"
  printf 'slskr_http=127.0.0.1:%s slskr_listen=127.0.0.1:%s\n' "$slskr_http_port" "$slskr_listen_port"
  printf 'slskdn_http=127.0.0.1:%s slskdn_listen=127.0.0.1:%s\n' "$slskdn_http_port" "$slskdn_listen_port"
  printf 'slskdn_overlay=127.0.0.1:%s\n' "$slskdn_overlay_port"
  printf 'slskr_endpoint_override=%s=127.0.0.1:%s\n' "$slskdn_username" "$slskdn_listen_port"
  printf 'slskr_upstream_endpoint_override=%s=127.0.0.1:%s\n' "$upstream_username" "$slskdn_listen_port"
  printf 'slskdn_endpoint_override=%s=127.0.0.1:%s\n' "$slskr_username" "$slskr_listen_port"
  printf 'slskdn_upstream_endpoint_override=%s=127.0.0.1:%s\n' "$upstream_username" "$slskr_listen_port"
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

run_virtual_soulfind_v2_checks() {
  local label port base_url track_id track_payload created intent_id pending plan release_created release_id status intent stats
  for label in slskr slskdn; do
    if [[ "$label" == "slskr" ]]; then
      port="$slskr_http_port"
    else
      port="$slskdn_http_port"
    fi
    base_url="http://127.0.0.1:$port/api/v1/virtualsoulfind/v2"
    track_id="$(node -e 'process.stdout.write(require("crypto").randomUUID())')"
    track_payload="$(node -e '
const trackId = process.argv[1];
process.stdout.write(JSON.stringify({ domain: "Music", trackId, priority: "High" }));
' "$track_id")"
    if ! created="$(auth_post_json "$base_url/intents/tracks?api-version=1" "$track_payload")" \
      || ! intent_id="$(printf '%s' "$created" | json_get desiredTrackId)" \
      || [[ -z "$intent_id" ]] \
      || [[ "$(printf '%s' "$created" | json_get status)" != "Pending" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-create" fail "${created:-request failed}"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-create" ok "intent=$intent_id"

    pending="$(auth_get "$base_url/intents/tracks/pending?api-version=1&limit=10")"
    if [[ "$pending" != *"$intent_id"* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-pending" fail "$pending"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-pending" ok "pending intent listed"

    plan="$(auth_post_json "$base_url/plans?api-version=1" "$track_payload")"
    if [[ "$plan" != *"$track_id"* || "$plan" != *'"steps":[]'* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-plan" fail "$plan"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-plan" ok "empty plan returned for unknown catalogue track"

    release_created="$(auth_post_json \
      "$base_url/intents/releases?api-version=1" \
      '{"releaseId":"release:interop","priority":"Normal","mode":"Wanted","notes":"interop"}')"
    release_id="$(printf '%s' "$release_created" | json_get desiredReleaseId 2>/dev/null || true)"
    if [[ -z "$release_id" || "$release_created" != *'"status":"Pending"'* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-release" fail "$release_created"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-release" ok "release intent=$release_id"

    auth_post_json \
      "$base_url/intents/tracks/$intent_id/process?api-version=1" \
      '{}' >/dev/null
    status=""
    for _ in $(seq 1 20); do
      intent="$(auth_get "$base_url/intents/tracks/$intent_id?api-version=1")"
      status="$(printf '%s' "$intent" | json_get status 2>/dev/null || true)"
      [[ "$status" == "Failed" ]] && break
      sleep 0.1
    done
    stats="$(auth_get "$base_url/stats?api-version=1")"
    if [[ "$status" != "Failed" ]] \
      || [[ "$(printf '%s' "$stats" | json_get totalProcessed 2>/dev/null || true)" != "1" ]] \
      || [[ "$(printf '%s' "$stats" | json_get failureCount 2>/dev/null || true)" != "1" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-process" fail "intent=$intent stats=$stats"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-process" ok "atomic claim processed unknown track once"
  done
}

run_browse_interop_checks() {
  local escaped_slskr escaped_slskdn body
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"

  wait_slskr_connected
  wait_slskdn_connected

  body="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse")"
  if printf '%s' "$body" | json_find_string "$slskr_fixture_name" 2>/dev/null; then
    record_check protocol-slskdn-browses-slskr ok "fixture=$slskr_fixture_name"
  else
    record_check protocol-slskdn-browses-slskr fail "$body"
    return 1
  fi

  wait_slskr_connected
  wait_slskdn_connected

  auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse/request" '{}' >/dev/null
  wait_json_contains protocol-slskr-browses-slskdn "http://127.0.0.1:$slskr_http_port/api/v0/users/$escaped_slskdn/browse" "$slskdn_fixture_name"
}

run_search_interop_checks() {
  local escaped_slskr escaped_slskdn
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"

  if SLSK_USERNAME="${upstream_username:-$slskr_username}" \
    SLSK_PASSWORD="${upstream_password:-$slskr_password}" \
    SLSK_SERVER="$server_endpoint" \
    SLSK_PEER_USERNAME="$slskdn_username" \
    SLSK_SEARCH_QUERY="slskdn" \
    SLSK_SEARCH_EXPECTED="$slskdn_fixture_name" \
    SLSK_SEARCH_HOST_OVERRIDE=127.0.0.1 \
    SLSK_SEARCH_PORT_OVERRIDE="$slskdn_listen_port" \
    SLSK_SEARCH_WAIT_PORT="$slskr_listen_port" \
    SLSK_SEARCH_FORCE_LOGIN=true \
    SLSK_SEARCH_PROBE_ATTEMPTS=3 \
    SLSK_SEARCH_PROBE_TIMEOUT_SECONDS=20 \
      timeout 75 cargo run -q -p slskr -- probe search-peer >>"$diag_file" 2>&1; then
    record_check protocol-slskr-searches-slskdn ok "query=slskdn expected=$slskdn_fixture_name"
  else
    record_check protocol-slskr-searches-slskdn fail "$(tail -n 1 "$diag_file")"
    return 1
  fi

  if SLSK_USERNAME="${upstream_username:-$slskdn_username}" \
    SLSK_PASSWORD="${upstream_password:-$slskdn_password}" \
    SLSK_SERVER="$server_endpoint" \
    SLSK_PEER_USERNAME="$slskr_username" \
    SLSK_SEARCH_QUERY="slskr" \
    SLSK_SEARCH_EXPECTED="$slskr_fixture_name" \
    SLSK_SEARCH_HOST_OVERRIDE=127.0.0.1 \
    SLSK_SEARCH_PORT_OVERRIDE="$slskr_listen_port" \
    SLSK_SEARCH_WAIT_PORT="$slskdn_listen_port" \
    SLSK_SEARCH_FORCE_LOGIN=true \
    SLSK_SEARCH_PROBE_ATTEMPTS=3 \
    SLSK_SEARCH_PROBE_TIMEOUT_SECONDS=20 \
      timeout 75 cargo run -q -p slskr -- probe search-peer >>"$diag_file" 2>&1; then
    record_check protocol-slskdn-searches-slskr ok "query=slskr expected=$slskr_fixture_name"
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

  if auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/messages" "{\"username\":\"$slskdn_username\",\"body\":\"$slskr_message\"}" >/dev/null; then
    wait_json_contains protocol-slskr-message-dispatch "http://127.0.0.1:$slskdn_http_port/api/v0/conversations/$escaped_slskr" "$slskr_message" || return 1
  else
    record_check protocol-slskr-message-dispatch fail "send failed"
    return 1
  fi

  if auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/conversations/$escaped_slskr" "\"$slskdn_message\"" >/dev/null; then
    wait_json_contains protocol-slskdn-message-dispatch "http://127.0.0.1:$slskr_http_port/api/v0/messages/$escaped_slskdn" "$slskdn_message" || return 1
  else
    record_check protocol-slskdn-message-dispatch fail "send failed"
    return 1
  fi

  record_check protocol-private-message-server-roundtrip ok "sender=$slskr_username receiver=$slskdn_username"
}

run_mesh_runtime_checks() {
  local escaped_slskr escaped_slskdn capability_probe slskr_capabilities slskdn_capabilities overlay_pin overlay_output health stats transport ticket
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"

  capability_probe="$(auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/mesh/sync/$escaped_slskdn" '{}')"
  if [[ "$capability_probe" != *'"probeQueued":true'* ]]; then
    record_check protocol-ksdn-probe-dispatch fail "$capability_probe"
    return 1
  fi
  record_check protocol-ksdn-probe-dispatch ok "slskr hello queued"

  wait_json_contains protocol-ksdn-slskr-receives-ack \
    "http://127.0.0.1:$slskr_http_port/api/v0/soulseek/peer-capabilities" \
    "$slskdn_username" || return 1
  slskr_capabilities="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/soulseek/peer-capabilities")"
  if [[ "$slskr_capabilities" != *'"mesh_sync"'* || "$slskr_capabilities" != *'"overlayPort"'* ]]; then
    record_check protocol-ksdn-slskr-verifies-slskdn-descriptor fail "$slskr_capabilities"
    return 1
  fi
  record_check protocol-ksdn-slskr-verifies-slskdn-descriptor ok "signed mesh_sync descriptor persisted"

  wait_json_contains protocol-ksdn-slskdn-receives-hello \
    "http://127.0.0.1:$slskdn_http_port/api/v0/capabilities/peers/$escaped_slskr" \
    "$slskr_username" || return 1
  slskdn_capabilities="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/capabilities/peers/$escaped_slskr")"
  if [[ "$slskdn_capabilities" != *'"slskdn/runtime-capability-v1"'* ]]; then
    record_check protocol-ksdn-slskdn-persists-slskr-descriptor fail "$slskdn_capabilities"
    return 1
  fi
  record_check protocol-ksdn-slskdn-persists-slskr-descriptor ok "runtime capability record persisted"

  if [[ ! -s "$slskdn_app/overlay_cert.pfx" ]]; then
    node -e "const net=require('net'); const s=net.createConnection({host:'127.0.0.1',port:Number(process.argv[1])},()=>s.destroy()); s.on('error',()=>process.exit(1)); setTimeout(()=>process.exit(1),5000);" \
      "$slskdn_overlay_port" || true
    local certificate_deadline=$((SECONDS + 15))
    while [[ ! -s "$slskdn_app/overlay_cert.pfx" ]] && ((SECONDS < certificate_deadline)); do
      sleep 1
    done
  fi
  if [[ ! -s "$slskdn_app/overlay_cert.pfx" ]]; then
    record_check protocol-pinned-overlay-certificate fail "overlay certificate was not created"
    return 1
  fi
  overlay_pin="$(
    openssl pkcs12 -in "$slskdn_app/overlay_cert.pfx" -passin pass: -clcerts -nokeys 2>/dev/null \
      | openssl x509 -outform der 2>/dev/null \
      | sha256sum \
      | awk '{print $1}'
  )"
  if [[ ! "$overlay_pin" =~ ^[0-9a-f]{64}$ ]]; then
    record_check protocol-pinned-overlay-certificate fail "certificate fingerprint unavailable"
    return 1
  fi
  record_check protocol-pinned-overlay-certificate ok "sha256 fingerprint loaded"

  pod_id="pod:$(printf '%s' "slskr-pod-interop-$(date +%s%N)" | sha256sum | cut -c1-32)"
  pod_message="slskr-pod-message-$(date +%s%N)"
  pod_create_payload="$(node -e '
const podId = process.argv[1];
process.stdout.write(JSON.stringify({
  pod: {
    podId,
    name: "slskr live interop",
    description: "Pinned overlay workflow fixture",
    visibility: 0,
    isPublic: true,
    maxMembers: 8,
    allowGuests: false,
    requireApproval: false,
    tags: ["interop"],
    channels: [{ channelId: "general", kind: 0, name: "General" }],
    externalBindings: [],
    capabilities: []
  },
  requestingPeerId: "ignored"
}));
' "$pod_id")"
  if pod_create="$(auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/pods" "$pod_create_payload")" \
    && [[ "$pod_create" == *"$pod_id"* ]]; then
    record_check runtime-slskdn-pod-create ok "pod=$pod_id"
  else
    record_check runtime-slskdn-pod-create fail "${pod_create:-request failed}"
    return 1
  fi

  overlay_service_call() {
    local method="$1"
    local payload="$2"
    local expected="$3"
    local service="${4:-pods}"
    local expected_sha256="${5:-}"
    SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_port" \
    SLSKR_OVERLAY_CERTIFICATE_SHA256="$overlay_pin" \
    SLSKR_OVERLAY_SERVICE="$service" \
    SLSKR_OVERLAY_METHOD="$method" \
    SLSKR_OVERLAY_PAYLOAD="$payload" \
    SLSKR_OVERLAY_EXPECTED="$expected" \
    SLSKR_OVERLAY_EXPECTED_SHA256="$expected_sha256" \
    SLSK_USERNAME="$slskr_username" \
    SLSK_PEER_USERNAME="$slskdn_username" \
      "$slskr_binary" probe overlay-service 2>&1
  }

  if overlay_output="$(
    SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_port" \
    SLSKR_OVERLAY_CERTIFICATE_SHA256="$overlay_pin" \
    SLSKR_OVERLAY_SERVICE=dht \
    SLSKR_OVERLAY_METHOD=Ping \
    SLSKR_OVERLAY_PAYLOAD='{"RequesterId":"AAAAAAAAAAAAAAAAAAAAAAAAAAA="}' \
    SLSKR_OVERLAY_EXPECTED=Timestamp \
    SLSK_USERNAME="$slskr_username" \
    SLSK_PEER_USERNAME="$slskdn_username" \
      "$slskr_binary" probe overlay-service 2>&1
  )"; then
    printf '\n[pinned-overlay-service]\n%s\n' "$overlay_output" >>"$diag_file"
    record_check protocol-pinned-overlay-service ok "dht.Ping returned a timestamp"
  else
    printf '\n[pinned-overlay-service-failed]\n%s\n' "$overlay_output" >>"$diag_file"
    record_check protocol-pinned-overlay-service fail "$overlay_output"
    return 1
  fi

  library_items="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/library/items?query=$(url_escape "$slskdn_fixture_name")&limit=10")"
  slskdn_content_id="$(printf '%s' "$library_items" | node -e '
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  const filename = process.argv[1];
  const body = JSON.parse(input);
  const item = (body.items || []).find(candidate => candidate.fileName === filename);
  if (item?.contentId) process.stdout.write(item.contentId);
});
' "$slskdn_fixture_name")"
  if [[ -z "$slskdn_content_id" ]]; then
    record_check runtime-slskdn-mesh-content-id fail "$library_items"
    return 1
  fi
  record_check runtime-slskdn-mesh-content-id ok "contentId=$slskdn_content_id"

  mesh_content_payload="$(node -e '
const [contentId, lengthText] = process.argv.slice(1);
process.stdout.write(JSON.stringify({ contentId, range: { offset: 0, length: Number(lengthText) } }));
' "$slskdn_content_id" "$slskdn_fixture_size")"
  if mesh_content_output="$(overlay_service_call GetByContentId "$mesh_content_payload" '' MeshContent "$slskdn_fixture_sha")"; then
    printf '\n[mesh-content-exact-bytes]\n%s\n' "$mesh_content_output" >>"$diag_file"
    record_check protocol-slskr-mesh-content-slskdn ok "bytes=$slskdn_fixture_size sha256=$slskdn_fixture_sha"
  else
    printf '\n[mesh-content-exact-bytes-failed]\n%s\n' "$mesh_content_output" >>"$diag_file"
    record_check protocol-slskr-mesh-content-slskdn fail "$mesh_content_output"
    return 1
  fi

  if pod_list_output="$(overlay_service_call List '{}' "$pod_id")"; then
    printf '\n[pods-list]\n%s\n' "$pod_list_output" >>"$diag_file"
    record_check protocol-slskr-pods-list-slskdn ok "listed pod discovered over pinned overlay"
  else
    printf '\n[pods-list-failed]\n%s\n' "$pod_list_output" >>"$diag_file"
    record_check protocol-slskr-pods-list-slskdn fail "$pod_list_output"
    return 1
  fi

  if pod_get_output="$(overlay_service_call Get "{\"PodId\":\"$pod_id\"}" "$pod_id")"; then
    printf '\n[pods-get]\n%s\n' "$pod_get_output" >>"$diag_file"
    record_check protocol-slskr-pods-get-slskdn ok "pod metadata fetched over pinned overlay"
  else
    printf '\n[pods-get-failed]\n%s\n' "$pod_get_output" >>"$diag_file"
    record_check protocol-slskr-pods-get-slskdn fail "$pod_get_output"
    return 1
  fi

  if pod_join_output="$(overlay_service_call Join "{\"PodId\":\"$pod_id\",\"Role\":\"member\"}" '"Success":true')"; then
    printf '\n[pods-join]\n%s\n' "$pod_join_output" >>"$diag_file"
    record_check protocol-slskr-pods-join-slskdn ok "remote overlay identity joined pod"
  else
    printf '\n[pods-join-failed]\n%s\n' "$pod_join_output" >>"$diag_file"
    record_check protocol-slskr-pods-join-slskdn fail "$pod_join_output"
    return 1
  fi

  if pod_post_output="$(overlay_service_call PostMessage "{\"PodId\":\"$pod_id\",\"ChannelId\":\"general\",\"Body\":\"$pod_message\"}" '"Success":true')"; then
    printf '\n[pods-post-message]\n%s\n' "$pod_post_output" >>"$diag_file"
    record_check protocol-slskr-pods-post-slskdn ok "member message stored over pinned overlay"
  else
    printf '\n[pods-post-message-failed]\n%s\n' "$pod_post_output" >>"$diag_file"
    record_check protocol-slskr-pods-post-slskdn fail "$pod_post_output"
    return 1
  fi

  if pod_messages_output="$(overlay_service_call GetMessages "{\"PodId\":\"$pod_id\",\"ChannelId\":\"general\"}" "$pod_message")"; then
    printf '\n[pods-get-messages]\n%s\n' "$pod_messages_output" >>"$diag_file"
    record_check protocol-slskr-pods-messages-slskdn ok "stored member message polled over pinned overlay"
  else
    printf '\n[pods-get-messages-failed]\n%s\n' "$pod_messages_output" >>"$diag_file"
    record_check protocol-slskr-pods-messages-slskdn fail "$pod_messages_output"
    return 1
  fi

  if pod_leave_output="$(overlay_service_call Leave "{\"PodId\":\"$pod_id\"}" '"Success":true')"; then
    printf '\n[pods-leave]\n%s\n' "$pod_leave_output" >>"$diag_file"
    record_check protocol-slskr-pods-leave-slskdn ok "remote overlay identity left pod"
  else
    printf '\n[pods-leave-failed]\n%s\n' "$pod_leave_output" >>"$diag_file"
    record_check protocol-slskr-pods-leave-slskdn fail "$pod_leave_output"
    return 1
  fi

  if ! local_profile="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/profile/me")" \
    || ! gateway_peer_id="$(printf '%s' "$local_profile" | json_get peerId)" \
    || [[ -z "$gateway_peer_id" ]]; then
    record_check runtime-slskdn-gateway-identity fail "local signed profile unavailable"
    return 1
  fi
  record_check runtime-slskdn-gateway-identity ok "signed local gateway identity loaded"

  gateway_pod_id="pod:$(printf '%s' "slskr-gateway-interop-$(date +%s%N)" | sha256sum | cut -c1-32)"
  gateway_pod_payload="$(node -e '
const [podId, gatewayPeerId, host, portText] = process.argv.slice(1);
const port = Number(portText);
process.stdout.write(JSON.stringify({
  pod: {
    podId,
    name: "slskr gateway interop",
    visibility: 2,
    isPublic: false,
    maxMembers: 3,
    allowGuests: false,
    requireApproval: false,
    tags: ["interop"],
    channels: [{ channelId: "general", kind: 0, name: "General" }],
    externalBindings: [],
    capabilities: [0],
    privateServicePolicy: {
      enabled: true,
      maxMembers: 3,
      gatewayPeerId,
      registeredServices: [],
      allowedDestinations: [{ hostPattern: host, port, protocol: "tcp", allowPublic: false, kind: 0 }],
      allowPrivateRanges: true,
      allowPublicDestinations: false,
      maxConcurrentTunnelsPerPeer: 2,
      maxConcurrentTunnelsPod: 3,
      maxNewTunnelsPerMinutePerPeer: 3,
      maxBytesPerDayPerPeer: 1048576,
      maxBufferedBytesPerTunnel: 65536,
      maxFrameSize: 8192
    }
  },
  requestingPeerId: "ignored"
}));
' "$gateway_pod_id" "$gateway_peer_id" "$gateway_echo_host" "$gateway_echo_port")"
  if gateway_pod_create="$(auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/pods" "$gateway_pod_payload")" \
    && [[ "$gateway_pod_create" == *"$gateway_pod_id"* ]]; then
    record_check runtime-slskdn-gateway-pod-create ok "pod=$gateway_pod_id"
  else
    record_check runtime-slskdn-gateway-pod-create fail "${gateway_pod_create:-request failed}"
    return 1
  fi

  if gateway_join_output="$(overlay_service_call Join "{\"PodId\":\"$gateway_pod_id\",\"Role\":\"member\"}" '"Success":true')"; then
    printf '\n[gateway-pod-join]\n%s\n' "$gateway_join_output" >>"$diag_file"
    record_check protocol-slskr-gateway-pod-join-slskdn ok "remote overlay identity joined gateway pod"
  else
    printf '\n[gateway-pod-join-failed]\n%s\n' "$gateway_join_output" >>"$diag_file"
    record_check protocol-slskr-gateway-pod-join-slskdn fail "$gateway_join_output"
    return 1
  fi

  gateway_nonce="$(openssl rand -hex 16)"
  gateway_timestamp="$(date +%s)"
  gateway_open_payload="$(node -e '
const [podId, host, portText, nonce, timestampText] = process.argv.slice(1);
process.stdout.write(JSON.stringify({
  PodId: podId,
  DestinationHost: host,
  DestinationPort: Number(portText),
  RequestNonce: nonce,
  RequestTimestamp: Number(timestampText)
}));
' "$gateway_pod_id" "$gateway_echo_host" "$gateway_echo_port" "$gateway_nonce" "$gateway_timestamp")"
  if gateway_open_output="$(overlay_service_call OpenTunnel "$gateway_open_payload" '"Accepted":true' private-gateway)" \
    && gateway_tunnel_id="$(printf '%s' "$gateway_open_output" | sed -nE 's/.*"TunnelId":"([^"]+)".*/\1/p' | head -n 1)" \
    && [[ -n "$gateway_tunnel_id" ]]; then
    printf '\n[gateway-open-tunnel]\n%s\n' "$gateway_open_output" >>"$diag_file"
    record_check protocol-slskr-gateway-open-slskdn ok "private TCP tunnel opened"
  else
    printf '\n[gateway-open-tunnel-failed]\n%s\n' "${gateway_open_output:-no response}" >>"$diag_file"
    record_check protocol-slskr-gateway-open-slskdn fail "${gateway_open_output:-tunnel id unavailable}"
    return 1
  fi

  gateway_echo_message="slskr-private-gateway-$(date +%s%N)"
  gateway_echo_base64="$(printf '%s' "$gateway_echo_message" | base64 -w0)"
  if gateway_send_output="$(overlay_service_call TunnelData "{\"TunnelId\":\"$gateway_tunnel_id\",\"Data\":\"$gateway_echo_base64\"}" '"Sent":' private-gateway)"; then
    printf '\n[gateway-tunnel-data]\n%s\n' "$gateway_send_output" >>"$diag_file"
    record_check protocol-slskr-gateway-send-slskdn ok "tunnel payload accepted"
  else
    printf '\n[gateway-tunnel-data-failed]\n%s\n' "$gateway_send_output" >>"$diag_file"
    record_check protocol-slskr-gateway-send-slskdn fail "$gateway_send_output"
    return 1
  fi

  gateway_receive_output=""
  gateway_received=0
  for _ in $(seq 1 20); do
    if gateway_receive_output="$(overlay_service_call GetTunnelData "{\"TunnelId\":\"$gateway_tunnel_id\"}" "$gateway_echo_base64" private-gateway)"; then
      gateway_received=1
      break
    fi
    sleep 0.25
  done
  if [[ "$gateway_received" == "1" ]]; then
    printf '\n[gateway-get-tunnel-data]\n%s\n' "$gateway_receive_output" >>"$diag_file"
    record_check protocol-slskr-gateway-receive-slskdn ok "exact echo payload returned"
  else
    printf '\n[gateway-get-tunnel-data-failed]\n%s\n' "$gateway_receive_output" >>"$diag_file"
    record_check protocol-slskr-gateway-receive-slskdn fail "echo payload unavailable"
    return 1
  fi

  if gateway_close_output="$(overlay_service_call CloseTunnel "{\"TunnelId\":\"$gateway_tunnel_id\"}" '"Closed":true' private-gateway)"; then
    printf '\n[gateway-close-tunnel]\n%s\n' "$gateway_close_output" >>"$diag_file"
    record_check protocol-slskr-gateway-close-slskdn ok "private TCP tunnel closed"
  else
    printf '\n[gateway-close-tunnel-failed]\n%s\n' "$gateway_close_output" >>"$diag_file"
    record_check protocol-slskr-gateway-close-slskdn fail "$gateway_close_output"
    return 1
  fi

  if dht_store_output="$(
    SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_port" \
    SLSKR_OVERLAY_CERTIFICATE_SHA256="$overlay_pin" \
    SLSK_USERNAME="$slskr_username" \
    SLSK_PEER_USERNAME="$slskdn_username" \
      "$slskr_binary" probe dht-store 2>&1
  )"; then
    printf '\n[signed-dht-store]\n%s\n' "$dht_store_output" >>"$diag_file"
    record_check protocol-slskr-dht-store-slskdn ok "authenticated signed Store accepted"
  else
    printf '\n[signed-dht-store-failed]\n%s\n' "$dht_store_output" >>"$diag_file"
    record_check protocol-slskr-dht-store-slskdn fail "$dht_store_output"
    return 1
  fi

  if ! health="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/health?api-version=1.0")" \
    || ! printf '%s' "$health" | json_get routingNodes >/dev/null 2>&1; then
    record_check runtime-slskdn-mesh-health fail "${health:-request failed}"
    return 1
  fi
  record_check runtime-slskdn-mesh-health ok "$(printf '%s' "$health" | tr '\n\t' '  ')"

  if ! stats="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/stats")" \
    || ! printf '%s' "$stats" | json_get totalSyncs >/dev/null 2>&1; then
    record_check runtime-slskdn-mesh-stats fail "${stats:-request failed}"
    return 1
  fi
  record_check runtime-slskdn-mesh-stats ok "$(printf '%s' "$stats" | tr '\n\t' '  ')"

  if ! transport="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/transport")" \
    || ! printf '%s' "$transport" | json_get natType >/dev/null 2>&1; then
    record_check network-slskdn-mesh-transport fail "${transport:-request failed}"
    return 1
  fi
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
      download_path="$slskr_state/downloads/shares/$slskdn_fixture_name"
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

run_slskr_backfill_probe() {
  local response success hash
  if ! response="$(auth_post_json \
      "http://127.0.0.1:$slskr_http_port/api/v0/backfill/file" \
      "{\"peerId\":\"$slskdn_username\",\"path\":\"$slskdn_remote_filename\",\"size\":$slskdn_fixture_size}" 2>&1)"; then
    record_check protocol-slskr-backfill-slskdn fail "$response"
    return 1
  fi
  success="$(printf '%s' "$response" | json_get success 2>/dev/null || true)"
  hash="$(printf '%s' "$response" | json_get hash 2>/dev/null || true)"
  if [[ "$success" == "true" && "$hash" == "$slskdn_fixture_sha" ]]; then
    record_check protocol-slskr-backfill-slskdn ok "bytes=$slskdn_fixture_size byteHash=$hash"
    return 0
  fi
  record_check protocol-slskr-backfill-slskdn fail "$response"
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
run_virtual_soulfind_v2_checks || status=1
run_browse_interop_checks || status=1
run_search_interop_checks || status=1
run_slskr_backfill_probe || status=1
run_slskr_to_slskdn_download || status=1
run_slskdn_to_slskr_download || status=1
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
