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
mkdir -p "$work_dir" "$output_dir"
result_file="$output_dir/slskr-slskdn-cross-client-interop.tsv"

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

slskdn_binary="$(discover_slskdn_binary || true)"
if [[ -z "$slskdn_binary" ]]; then
  echo "slskdN binary not found; set SLSKDN_BINARY_PATH or build ../slskdn" >&2
  exit 2
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

slskr_fixture_name="slskr-to-slskdn-$(date -u +%Y%m%d%H%M%S).bin"
slskdn_fixture_name="slskdn-to-slskr-$(date -u +%Y%m%d%H%M%S).bin"
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
soulseek:
  address: ${SLSK_SERVER_ADDRESS:-vps.slsknet.org}
  port: ${SLSK_SERVER_PORT:-2271}
  username: "$slskdn_username"
  password: "$slskdn_password"
  listen_ip_address: 0.0.0.0
  listen_port: $slskdn_listen_port
flags:
  no_connect: false
YAML

(
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

printf 'timestamp\tcheck\tstatus\tdetail\n' >"$result_file"

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

status=0
run_slskr_to_slskdn_download || status=1
run_slskdn_to_slskr_download || status=1

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
