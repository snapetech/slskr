#!/usr/bin/env bash
set -euo pipefail

# Enter the repository-wide resident-memory guard even when this runner is
# invoked directly.  The held marker prevents recursion when the guard
# re-executes this script.
runner_repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if ! "$runner_repo_root/scripts/process-memory-guard-active.sh"; then
  exec "$runner_repo_root/scripts/with-process-memory-guard.sh" "${BASH_SOURCE[0]}" "$@"
fi

# Keep every live interop child bounded by the stricter virtual-memory ceiling
# as well.  A stricter parent limit is preserved rather than raised.
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

curl_failure_args=(-f)
if curl --help all 2>/dev/null | grep -Fq -- '--fail-with-body'; then
  curl_failure_args=(--fail-with-body)
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

env_files=(
  "${SLSKR_LIVE_ENV_FILE:-$repo_root/.env}"
  "${SLSKR_SLSKDN_ENV_FILE:-$repo_root/../slskdn/.env}"
  "${SLSKR_LIVE_ACCOUNT_ENV_FILE:-$repo_root/.secrets/generated-soulseek-accounts.env}"
  "${SLSKR_SLSKDN_ACCOUNT_POOL_FILE:-$repo_root/../slskdn/tests/slskd.Tests.Integration/local-mesh-account-pool.env}"
)

explicit_slskdn_binary="${SLSKDN_BINARY_PATH:-}"

for env_file in "${env_files[@]}"; do
  if [[ -f "$env_file" ]]; then
    set -a
    # shellcheck disable=SC1090
    source "$env_file"
    set +a
  fi
done

# A caller-supplied frozen executable is authoritative. The credential/env
# files may contain a developer-local build path, but must not silently replace
# an explicit oracle selected for a certification run.
if [[ -n "$explicit_slskdn_binary" ]]; then
  export SLSKDN_BINARY_PATH="$explicit_slskdn_binary"
fi

api_token="${SLSKR_CROSS_CLIENT_API_TOKEN:-slskr-cross-client-interop}"
timeout_seconds="${SLSKR_CROSS_CLIENT_TIMEOUT_SECONDS:-240}"
soak_seconds="${SLSKR_CROSS_CLIENT_SOAK_SECONDS:-30}"
work_dir="${SLSKR_CROSS_CLIENT_WORK_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/slskr-slskdn-interop.XXXXXX")}"
output_dir="${SLSKR_INTEROP_OUTPUT_DIR:-$repo_root/target/live-interop}"
server_host="${SLSKR_CROSS_CLIENT_SERVER_HOST:-${SLSK_SERVER_ADDRESS:-vps.slsknet.org}}"
server_port="${SLSKR_CROSS_CLIENT_SERVER_PORT:-${SLSK_SERVER_PORT:-2271}}"
server_endpoint="${SLSKR_CROSS_CLIENT_SERVER:-$server_host:$server_port}"
mkdir -p "$work_dir" "$output_dir"
final_result_file="$output_dir/slskr-slskdn-cross-client-interop.tsv"
result_file="$output_dir/.slskr-slskdn-cross-client-interop.$$.tsv"
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
    curl -sS "${curl_failure_args[@]}" "$url"
  else
    curl -sS "${curl_failure_args[@]}" -H "Authorization: Bearer $api_token" "$url"
  fi
}

auth_post_json() {
  local url="$1"
  local payload="$2"
  if [[ "$url" == "http://127.0.0.1:$slskdn_http_port/"* ]]; then
    curl -sS "${curl_failure_args[@]}" -H "Content-Type: application/json" -d "$payload" "$url"
  else
    curl -sS "${curl_failure_args[@]}" -H "Authorization: Bearer $api_token" -H "Content-Type: application/json" -d "$payload" "$url"
  fi
}

auth_post_json_with_status() {
  local url="$1"
  local payload="$2"
  if [[ "$url" == "http://127.0.0.1:$slskdn_http_port/"* ]]; then
    curl -sS -H "Content-Type: application/json" -d "$payload" -w $'\n%{http_code}' "$url"
  else
    curl -sS -H "Authorization: Bearer $api_token" -H "Content-Type: application/json" -d "$payload" -w $'\n%{http_code}' "$url"
  fi
}

auth_patch_json() {
  local url="$1"
  local payload="$2"
  if [[ "$url" == "http://127.0.0.1:$slskdn_http_port/"* ]]; then
    curl -fsS -X PATCH -H "Content-Type: application/json" -d "$payload" "$url"
  else
    curl -fsS -X PATCH -H "Authorization: Bearer $api_token" -H "Content-Type: application/json" -d "$payload" "$url"
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

v2_url() {
  local url="$1"
  if [[ "${v2_api_version_required:-0}" == "1" ]]; then
    if [[ "$url" == *'?'* ]]; then
      printf '%s&api-version=1' "$url"
    else
      printf '%s?api-version=1' "$url"
    fi
  else
    printf '%s' "$url"
  fi
}

rust_search_diagnostics() {
  auth_get "http://127.0.0.1:$slskr_http_port/api/v0/logs" 2>/dev/null |
    node -e '
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  let entries;
  try { entries = JSON.parse(input); } catch { process.exit(0); }
  const selected = (Array.isArray(entries) ? entries : [])
    .filter(entry => entry.category === "search" || /incoming public search/i.test(entry.message || ""))
    .slice(0, 8)
    .map(entry => `${entry.level || "?"}:${entry.message || ""}`)
    .join(" | ");
  process.stdout.write(selected.replace(/[\r\n\t]+/g, " ").slice(0, 900));
});
' 2>/dev/null || true
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

target_log_count() {
  local needle="$1"
  grep -cF -- "$needle" "$slskdn_log" 2>/dev/null || true
}

wait_target_log_delta() {
  local needle="$1"
  local before="$2"
  local deadline=$((SECONDS + timeout_seconds))
  local after
  while ((SECONDS < deadline)); do
    after="$(target_log_count "$needle")"
    if [[ "$before" =~ ^[0-9]+$ && "$after" =~ ^[0-9]+$ ]] && ((after > before)); then
      return 0
    fi
    sleep 1
  done
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
    local explicit_binary="$SLSKDN_BINARY_PATH"
    if [[ "$explicit_binary" != /* ]]; then
      explicit_binary="$repo_root/$explicit_binary"
    fi
    candidates+=("$explicit_binary")
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
slskr_obfuscated_listen_port="${SLSKR_CROSS_CLIENT_SLSKR_OBFUSCATED_LISTEN_PORT:-$(pick_port)}"
slskdn_http_port="$(pick_port)"
slskdn_listen_port="${SLSKR_CROSS_CLIENT_SLSKDN_LISTEN_PORT:-$(pick_port)}"
slskdn_obfuscated_port="${SLSKR_CROSS_CLIENT_SLSKDN_OBFUSCATED_LISTEN_PORT:-$(pick_port)}"
slskdn_overlay_port="${SLSKR_CROSS_CLIENT_SLSKDN_OVERLAY_PORT:-$(pick_port)}"
slskdn_overlay_endpoint_port="$slskdn_overlay_port"
slskdn_quic_backend_port="${SLSKR_CROSS_CLIENT_SLSKDN_QUIC_BACKEND_PORT:-$(pick_port)}"
slskdn_quic_data_port="${SLSKR_CROSS_CLIENT_SLSKDN_QUIC_DATA_PORT:-$(pick_port)}"
slskr_overlay_port_override="${SLSKR_CROSS_CLIENT_SLSKR_OVERLAY_PORT:-}"
slskr_shared_tcp="${SLSKR_CROSS_CLIENT_SHARED_TCP:-}"
if [[ -z "$slskr_shared_tcp" ]]; then
  # The current upstream profile shares the public TCP listener. Preserve the
  # old isolated profile when a caller explicitly supplies a different overlay
  # port, so frozen/diagnostic runs do not change topology accidentally.
  if [[ -n "$slskr_overlay_port_override" && "$slskr_overlay_port_override" != "$slskr_listen_port" ]]; then
    slskr_shared_tcp=0
  else
    slskr_shared_tcp=1
  fi
fi
case "$slskr_shared_tcp" in
  0|1) ;;
  *)
    echo "SLSKR_CROSS_CLIENT_SHARED_TCP must be 0 or 1" >&2
    exit 2
    ;;
esac
if [[ "$slskr_shared_tcp" == 1 ]]; then
  if [[ -n "$slskr_overlay_port_override" && "$slskr_overlay_port_override" != "$slskr_listen_port" ]]; then
    echo "SLSKR_CROSS_CLIENT_SLSKR_OVERLAY_PORT must equal SLSKR_CROSS_CLIENT_SLSKR_LISTEN_PORT when SLSKR_CROSS_CLIENT_SHARED_TCP=1; use SHARED_TCP=0 for a dedicated compatibility listener" >&2
    exit 2
  fi
  slskr_overlay_tcp_port="$slskr_listen_port"
  slskr_overlay_port="${slskr_overlay_port_override:-$(pick_port)}"
else
  slskr_overlay_port="${slskr_overlay_port_override:-$(pick_port)}"
  if [[ "$slskr_overlay_port" == "$slskr_listen_port" ]]; then
    echo "a dedicated slskR overlay port must differ from the Soulseek listen port" >&2
    exit 2
  fi
  slskr_overlay_tcp_port="$slskr_overlay_port"
fi
slskr_dht_port="${SLSKR_CROSS_CLIENT_SLSKR_DHT_PORT:-$slskr_overlay_port}"
slskr_quic_backend_port="${SLSKR_CROSS_CLIENT_SLSKR_QUIC_BACKEND_PORT:-$(pick_port)}"
slskr_quic_data_port="${SLSKR_CROSS_CLIENT_SLSKR_QUIC_DATA_PORT:-$(pick_port)}"
gateway_echo_port="$(pick_port)"
gateway_echo_host="$(ip -4 route get 1.1.1.1 | awk '{ for (i = 1; i <= NF; i++) if ($i == "src") { print $(i + 1); exit } }')"

slskr_state="$work_dir/slskr-state"
slskr_config="$work_dir/slskr.toml"
slskr_share="$work_dir/slskr-share"
slskdn_app="$work_dir/slskdn-app"
slskdn_share="$slskdn_app/shares"
slskdn_web="$slskdn_app/web"
mkdir -p "$slskr_state" "$slskr_share" "$slskdn_app/config" "$slskdn_app/downloads" "$slskdn_app/incomplete" "$slskdn_share" "$slskdn_web"

if [[ "${SLSKR_CROSS_CLIENT_TARGET_OBFUSCATED_LOOPBACK_OVERRIDE:-0}" == "1" ]]; then
  # Test-only route forcing: make the target's regular replacement dial fail
  # while the temporary target hook translates its selected obfuscated dial to
  # the replacement's loopback obfuscated listener.
  target_endpoint_overrides="$slskr_username=127.0.0.1:1;$upstream_username=127.0.0.1:$slskr_listen_port"
elif [[ "${SLSKR_CROSS_CLIENT_OMIT_REPLACEMENT_ENDPOINT_OVERRIDE:-0}" == "1" ]]; then
  target_endpoint_overrides="$upstream_username=127.0.0.1:$slskr_listen_port"
else
  target_endpoint_overrides="${SLSKR_CROSS_CLIENT_TARGET_ENDPOINT_OVERRIDES:-$slskr_username=127.0.0.1:$slskr_listen_port;$upstream_username=127.0.0.1:$slskr_listen_port}"
fi

cat >"$slskr_config" <<'TOML'
[mesh]
enabled = true
enable_soulseek_capability_handshake = true
enable_soulseek_rendezvous = true
probe_soulseek_rendezvous_capabilities = true
TOML

slskr_fixture_name="slskr-to-slskdn-$(date -u +%Y%m%d%H%M%S).flac"
slskdn_fixture_name="slskdn-to-slskr-$(date -u +%Y%m%d%H%M%S).flac"
printf 'slskr fixture %s\n' "$(date -u +%FT%TZ)" >"$slskr_share/$slskr_fixture_name"
printf 'fLaC\000\000\000\042' >"$slskdn_share/$slskdn_fixture_name"
dd if=/dev/zero bs=34 count=1 >>"$slskdn_share/$slskdn_fixture_name" 2>/dev/null
mkdir -p "$slskr_share/Interop Artist" "$slskdn_share/Interop Artist"
printf 'slskr v2 fixture %s\n' "$(date -u +%FT%TZ)" >"$slskr_share/Interop Artist/Interop Track.flac"
printf 'fLaC\000\000\000\042' >"$slskdn_share/Interop Artist/Interop Track.flac"
dd if=/dev/zero bs=34 count=1 >>"$slskdn_share/Interop Artist/Interop Track.flac" 2>/dev/null
slskr_fixture_size="$(wc -c <"$slskr_share/$slskr_fixture_name" | tr -d ' ')"
slskdn_fixture_size="$(wc -c <"$slskdn_share/$slskdn_fixture_name" | tr -d ' ')"
slskr_fixture_sha="$(sha256sum "$slskr_share/$slskr_fixture_name" | awk '{print $1}')"
slskdn_fixture_sha="$(sha256sum "$slskdn_share/$slskdn_fixture_name" | awk '{print $1}')"
slskdn_v2_fixture_name="Interop Track.flac"
slskdn_v2_fixture_size="$(wc -c <"$slskdn_share/Interop Artist/$slskdn_v2_fixture_name" | tr -d ' ')"
slskdn_v2_fixture_sha="$(sha256sum "$slskdn_share/Interop Artist/$slskdn_v2_fixture_name" | awk '{print $1}')"
slskr_remote_filename="$(basename "$slskr_share")/$slskr_fixture_name"
slskdn_remote_filename="shares\\\\$slskdn_fixture_name"

slskr_log="$work_dir/slskr.log"
slskdn_log="$work_dir/slskdn.log"
slskr_pid=""
slskdn_pid=""
gateway_echo_pid=""

wait_slskr_connected() {
  local startup_timeout_seconds="${SLSKR_CROSS_CLIENT_STARTUP_TIMEOUT_SECONDS:-120}"
  if [[ ! "$startup_timeout_seconds" =~ ^[1-9][0-9]*$ ]]; then
    echo "SLSKR_CROSS_CLIENT_STARTUP_TIMEOUT_SECONDS must be a positive integer" >&2
    return 2
  fi
  local deadline=$((SECONDS + startup_timeout_seconds))
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

# Build before either daemon starts. slskdN's test endpoint overrides live in its
# bounded endpoint cache, so compiling either daemon after launch can consume
# their useful lifetime before the cross-client checks begin. A pre-existing
# executable is only reusable when the slskR package inputs are older than it;
# this prevents a live parity run from silently testing stale Rust code.
slskr_binary="$repo_root/target/debug/slskr"
slskr_build_required=0
if [[ ! -x "$slskr_binary" ]]; then
  slskr_build_required=1
elif find \
  "$repo_root/crates/slskr" \
  "$repo_root/crates/slskr-client" \
  "$repo_root/Cargo.toml" \
  "$repo_root/Cargo.lock" \
  -type f -newer "$slskr_binary" -print -quit | grep -q .; then
  slskr_build_required=1
fi
if [[ "$slskr_build_required" -eq 1 ]]; then
  echo "building slskR interop binary from current package inputs" >&2
  export CARGO_BUILD_JOBS=1
  export RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}"
  export CARGO_NET_OFFLINE="${CARGO_NET_OFFLINE:-true}"
  scripts/with-build-guard.sh cargo build -q -p slskr
fi

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
  if [[ -n "${result_file:-}" && -f "$result_file" ]]; then
    # Preserve failed evidence for diagnosis. A failed run is never promoted
    # to the canonical all-green artifact, but deleting the only row-level
    # record makes transient network failures impossible to distinguish from
    # implementation regressions after the child daemons are cleaned up.
    failed_result_file="$output_dir/slskr-slskdn-cross-client-interop.failed-$$.tsv"
    cp -- "$result_file" "$failed_result_file"
    rm -f -- "$result_file"
  fi
}
trap cleanup EXIT

cat >"$slskdn_app/config/slskd.yml" <<YAML
debug: true
web:
  port: $slskdn_http_port
  address: 127.0.0.1
  content_path: .
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
  pods: true
  virtualSoulfind: true
virtualSoulfindV2:
  enabled: true
soulseek:
  address: $server_host
  port: $server_port
  diagnostic_level: debug
  distributed_network:
    logging: true
  username: "$slskdn_username"
  password: "$slskdn_password"
  listen_ip_address: 0.0.0.0
  listen_port: $slskdn_listen_port
  obfuscation:
    enabled: true
    mode: prefer
    listen_port: $slskdn_obfuscated_port
    advertise_regular_port: true
    prefer_outbound: true
dht:
  enabled: true
  lan_only: true
  overlay_port: $slskdn_overlay_port
  advertised_overlay_port: $slskdn_overlay_port
  dht_port: $slskdn_overlay_port
overlay:
  enable: true
  listen_port: $slskdn_overlay_port
  enable_quic: true
  quic_listen_port: $slskdn_overlay_port
  share_quic_with_dht_port: true
  quic_backend_listen_port: $slskdn_quic_backend_port
overlay_data:
  enable: true
  listen_port: $slskdn_overlay_port
  share_with_dht_port: true
  backend_listen_port: $slskdn_quic_data_port
flags:
  no_connect: false
YAML

(
  export SLSK_SERVER="$server_endpoint"
  export SLSKR_HTTP_BIND="127.0.0.1:$slskr_http_port"
  # The matrix uses the plain local controller endpoint.  Disable the unused
  # HTTPS listener so the default 5031 port cannot collide with another local
  # slskR instance.
  export SLSKD_NO_HTTPS=true
  export SLSKR_CONFIG="$slskr_config"
  export SLSKR_STATE_DIR="$slskr_state"
  export SLSK_USERNAME="$slskr_username"
  export SLSK_PASSWORD="$slskr_password"
  export SLSKR_AUTO_CONNECT=true
  export SLSKR_RECONNECT=true
  export SLSKR_SLSK_DIAG_LEVEL=debug
  export SLSKR_SLSK_DNET_LOGGING=true
  export SLSKR_AUTH_DISABLED=false
  export SLSKR_API_TOKEN="$api_token"
  export SLSKR_SHARE_DIRS="$slskr_share"
  export SLSKR_LISTENER_BIND="0.0.0.0:$slskr_listen_port"
  export SLSKR_OVERLAY_BIND="0.0.0.0:$slskr_overlay_tcp_port"
  export SLSKR_ADVANCED_NETWORKING_JSON="{\"dht\":{\"enabled\":true,\"dht_port\":$slskr_dht_port,\"overlay_port\":$slskr_overlay_tcp_port,\"advertised_overlay_port\":$slskr_overlay_tcp_port,\"lan_only\":true},\"overlay\":{\"enable\":true,\"listen_port\":$slskr_dht_port,\"enable_quic\":true,\"quic_listen_port\":$slskr_dht_port,\"share_quic_with_dht_port\":true,\"quic_backend_listen_port\":$slskr_quic_backend_port},\"overlay_data\":{\"enable\":true,\"listen_port\":$slskr_dht_port}}"
  export SLSKR_OBFUSCATED_LISTENER_BIND="0.0.0.0:$slskr_obfuscated_listen_port"
  export SLSK_LISTEN_PORT="$slskr_listen_port"
  export SLSKR_ADVERTISED_PORT="$slskr_listen_port"
  export SLSKR_OBFUSCATED_ADVERTISED_PORT="$slskr_obfuscated_listen_port"
  export SLSKD_SLSK_OBFUSCATION_LISTEN_PORT="$slskr_obfuscated_listen_port"
  export SLSK_OBFUSCATION=true
  export SLSK_OBFUSCATION_MODE=prefer
  export SLSK_OBFUSCATION_PREFER_OUTBOUND=true
  export SLSKR_PEER_HOST_OVERRIDE=127.0.0.1
  # Keep a real distributed link alive so the target-originated branch
  # metadata is observable on the replacement daemon.  The disposable probe
  # below still covers the direct ping transaction; this override covers the
  # reverse direction on the daemon's long-lived distributed connection.
  export SLSKR_DISTRIBUTED_PARENT_OVERRIDE="127.0.0.1:$slskdn_listen_port"
  export SLSKR_TEST_USER_ENDPOINT_OVERRIDES="$slskdn_username=127.0.0.1:$slskdn_listen_port;$upstream_username=127.0.0.1:$slskdn_listen_port"
  export SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS=60
  if [[ "${SLSKR_CROSS_CLIENT_DELAY_TARGET_START_UNTIL_SLSKR_CONNECTED:-0}" != "1" ]]; then
    distributed_parent_wait_deadline=$((SECONDS + ${SLSKR_CROSS_CLIENT_DISTRIBUTED_PARENT_WAIT_SECONDS:-120}))
    while ((SECONDS < distributed_parent_wait_deadline)); do
      distributed_parent_target_state="$(curl -sS "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null || true)"
      # Start slskR after the target has a logged-in Soulseek session and a
      # stable distributed role. Waiting specifically for canAcceptChildren
      # made this gate circular on target builds that briefly report child
      # acceptance before becoming a branch root; the target rejects inbound
      # children while it has neither a parent nor branch-root status.
      if [[ "$distributed_parent_target_state" == *'"isLoggedIn":true'* \
        && ("$distributed_parent_target_state" == *'"isBranchRoot":true'* \
          || "$distributed_parent_target_state" == *'"hasParent":true'*) ]]; then
        break
      fi
      sleep 1
    done
  fi
  exec "$slskr_binary" serve
) >"$slskr_log" 2>&1 &
slskr_pid="$!"

if [[ "${SLSKR_CROSS_CLIENT_DELAY_TARGET_START_UNTIL_SLSKR_CONNECTED:-0}" == "1" ]]; then
  wait_slskr_connected
fi

(
  export APP_DIR="$slskdn_app"
  export SLSKDN_TEST_USER_ENDPOINT_OVERRIDES="$target_endpoint_overrides"
  if [[ "${SLSKR_CROSS_CLIENT_TARGET_OBFUSCATED_LOOPBACK_OVERRIDE:-0}" == "1" ]]; then
    export SLSKDN_TEST_OBFUSCATED_ENDPOINT_OVERRIDES="$slskr_username=127.0.0.1:$slskr_obfuscated_listen_port"
  fi
  cd "$slskdn_app"
  exec "$slskdn_binary" --config config/slskd.yml --app-dir "$slskdn_app"
) >"$slskdn_log" 2>&1 &
slskdn_pid="$!"

{
  printf 'server_endpoint=%s\n' "$server_endpoint"
  printf 'slskr_http=127.0.0.1:%s slskr_listen=127.0.0.1:%s slskr_obfuscated=127.0.0.1:%s\n' "$slskr_http_port" "$slskr_listen_port" "$slskr_obfuscated_listen_port"
  printf 'slskdn_http=127.0.0.1:%s slskdn_listen=127.0.0.1:%s slskdn_obfuscated=127.0.0.1:%s\n' "$slskdn_http_port" "$slskdn_listen_port" "$slskdn_obfuscated_port"
  printf 'slskr_shared_tcp=%s slskr_overlay_tcp=127.0.0.1:%s slskr_dht_udp=127.0.0.1:%s\n' "$slskr_shared_tcp" "$slskr_overlay_tcp_port" "$slskr_dht_port"
  printf 'slskr_overlay=127.0.0.1:%s\n' "$slskr_overlay_port"
  printf 'slskdn_overlay=127.0.0.1:%s\n' "$slskdn_overlay_port"
  printf 'slskdn_quic_backend=127.0.0.1:%s slskdn_quic_data=127.0.0.1:%s\n' "$slskdn_quic_backend_port" "$slskdn_quic_data_port"
  printf 'slskr_quic_backend=127.0.0.1:%s slskr_quic_data=127.0.0.1:%s\n' "$slskr_quic_backend_port" "$slskr_quic_data_port"
  printf 'slskr_endpoint_override=%s=127.0.0.1:%s\n' "$slskdn_username" "$slskdn_listen_port"
  printf 'slskr_upstream_endpoint_override=%s=127.0.0.1:%s\n' "$upstream_username" "$slskdn_listen_port"
  printf 'slskdn_endpoint_override=%s=127.0.0.1:%s\n' "$slskr_username" "$slskr_listen_port"
  printf 'slskdn_upstream_endpoint_override=%s=127.0.0.1:%s\n' "$upstream_username" "$slskr_listen_port"
} >"$diag_file"

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
  printf '\n[slskdn-options]\n'
  auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/options" || true
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
      timeout 45 "$slskr_binary" probe peer-address
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
  if [[ "$app" == *"$slskdn_fixture_name"* || "$app" == *"\"files\":2"* ]]; then
    record_check runtime-slskdn-shares ok "fixture=$slskdn_fixture_name"
  else
    record_check runtime-slskdn-shares fail "$app"
    return 1
  fi

  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"
  endpoint="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/endpoint")"
  local expected_slskr_endpoint_port="$slskr_listen_port"
  if [[ "${SLSKR_CROSS_CLIENT_TARGET_OBFUSCATED_LOOPBACK_OVERRIDE:-0}" == "1" ]]; then
    expected_slskr_endpoint_port=1
  fi
  if [[ "${SLSKR_CROSS_CLIENT_OMIT_REPLACEMENT_ENDPOINT_OVERRIDE:-0}" == "1" ]]; then
    if [[ "$endpoint" == *":$expected_slskr_endpoint_port"* && "$endpoint" != *'"address":"127.0.0.1"'* ]]; then
      record_check network-slskdn-resolves-slskr ok "public-endpoint=$endpoint"
    else
      record_check network-slskdn-resolves-slskr fail "$endpoint"
      return 1
    fi
  elif [[ "$endpoint" == *":$expected_slskr_endpoint_port"* ]]; then
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
  local label port base_url track_id track_payload response response_body response_status
  local intent_id intent_body pending_body release_payload release_response release_body release_id
  local process_track_id process_response process_body process_status stats_body plan_body artists_body
  local artist_id release_group_id tracks_body positive_track_id positive_intent_id positive_response positive_status
  for label in slskr slskdn; do
    if [[ "$label" == "slskr" ]]; then
      port="$slskr_http_port"
    else
      port="$slskdn_http_port"
      v2_api_version_required=1
    fi
    if [[ "$label" == "slskr" ]]; then
      v2_api_version_required=0
    fi
    base_url="http://127.0.0.1:$port/api/v1/virtualsoulfind/v2"
    track_id="$(node -e 'process.stdout.write(require("crypto").randomUUID())')"
    track_payload="$(node -e '
const trackId = process.argv[1];
process.stdout.write(JSON.stringify({ domain: "Music", trackId, priority: "High" }));
    ' "$track_id")"
    if ! response="$(auth_post_json_with_status \
      "$(v2_url "$base_url/intents/tracks")" "$track_payload")"; then
      record_check "runtime-$label-virtualsoulfind-v2-create" fail "request failed: ${response:-no response}"
      return 1
    fi
    response_status="$(printf '%s\n' "$response" | tail -n 1)"
    response_body="$(printf '%s\n' "$response" | sed '$d')"
    if [[ "$response_status" != "201" ]] \
      || [[ "$response_body" != *'"desiredTrackId"'* ]] \
      || ! printf '%s' "$response_body" | json_find_string 'Pending' 2>/dev/null; then
      record_check "runtime-$label-virtualsoulfind-v2-create" fail \
        "status=$response_status body=$response_body"
      return 1
    fi
    intent_id="$(printf '%s' "$response_body" | json_get desiredTrackId 2>/dev/null || true)"
    if [[ -z "$intent_id" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-create" fail "created intent has no desiredTrackId body=$response_body"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-create" ok \
      "status=201 desiredTrackId=$intent_id"

    intent_body="$(auth_get "$(v2_url "$base_url/intents/tracks/$intent_id")")"
    if [[ "$intent_body" == *"$intent_id"* && "$intent_body" == *'"status":"Pending"'* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-track-readback" ok "status=200 pending intent read back"
    else
      record_check "runtime-$label-virtualsoulfind-v2-track-readback" fail "$intent_body"
      return 1
    fi

    pending_body="$(auth_get "$(v2_url "$base_url/intents/tracks/pending?limit=100")")"
    if printf '%s' "$pending_body" | json_find_string "$intent_id" 2>/dev/null; then
      record_check "runtime-$label-virtualsoulfind-v2-pending" ok "pending intent listed"
    else
      record_check "runtime-$label-virtualsoulfind-v2-pending" fail "$pending_body"
      return 1
    fi

    if auth_patch_json "$(v2_url "$base_url/intents/tracks/$intent_id")" '{"status":"Planned"}' >/dev/null; then
      intent_body="$(auth_get "$(v2_url "$base_url/intents/tracks/$intent_id")")"
      if [[ "$intent_body" == *'"status":"Planned"'* ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-status-update" ok "status=204 Planned persisted"
      else
        record_check "runtime-$label-virtualsoulfind-v2-status-update" fail "$intent_body"
        return 1
      fi
    else
      record_check "runtime-$label-virtualsoulfind-v2-status-update" fail "PATCH status update failed"
      return 1
    fi

    release_payload="$(node -e 'process.stdout.write(JSON.stringify({ releaseId: "release-interop", priority: "Normal", mode: "Wanted", notes: "live v2 route proof" }))')"
    if ! release_response="$(auth_post_json_with_status "$(v2_url "$base_url/intents/releases")" "$release_payload")"; then
      record_check "runtime-$label-virtualsoulfind-v2-release-create" fail "request failed"
      return 1
    fi
    response_status="$(printf '%s\n' "$release_response" | tail -n 1)"
    release_body="$(printf '%s\n' "$release_response" | sed '$d')"
    release_id="$(printf '%s' "$release_body" | json_get desiredReleaseId 2>/dev/null || true)"
    if [[ "$response_status" == "201" && -n "$release_id" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-release-create" ok "status=201 desiredReleaseId=$release_id"
      release_body="$(auth_get "$(v2_url "$base_url/intents/releases/$release_id")")"
      if [[ "$release_body" == *"$release_id"* && "$release_body" == *'"status":"Pending"'* ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-release-readback" ok "status=200 release read back"
      else
        record_check "runtime-$label-virtualsoulfind-v2-release-readback" fail "$release_body"
        return 1
      fi
    else
      record_check "runtime-$label-virtualsoulfind-v2-release-create" fail "status=$response_status body=$release_body"
      return 1
    fi

    track_payload="$(node -e 'process.stdout.write(JSON.stringify({ domain: "Music", trackId: require("crypto").randomUUID(), priority: "Normal" }))')"
    process_response="$(auth_post_json_with_status "$(v2_url "$base_url/intents/tracks")" "$track_payload")"
    process_status="$(printf '%s\n' "$process_response" | tail -n 1)"
    process_body="$(printf '%s\n' "$process_response" | sed '$d')"
    process_track_id="$(printf '%s' "$process_body" | json_get desiredTrackId 2>/dev/null || true)"
    if [[ "$process_status" != "201" || -z "$process_track_id" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-process-create" fail "status=$process_status body=$process_body"
      return 1
    fi
    if ! auth_post_json "$(v2_url "$base_url/intents/tracks/$process_track_id/process")" '{}' >/dev/null 2>&1; then
      record_check "runtime-$label-virtualsoulfind-v2-process" fail "POST process failed"
      return 1
    fi
    record_check "runtime-$label-virtualsoulfind-v2-process" ok "status=202 processing requested"

    process_status="Pending"
    process_body=""
    local process_deadline=$((SECONDS + timeout_seconds))
    while ((SECONDS < process_deadline)); do
      process_body="$(auth_get "$(v2_url "$base_url/intents/tracks/$process_track_id")" 2>/dev/null || true)"
      process_status="$(printf '%s' "$process_body" | json_get status 2>/dev/null || true)"
      if [[ "$process_status" == "Completed" || "$process_status" == "Failed" ]]; then
        break
      fi
      sleep 1
    done
    if [[ "$process_status" == "Failed" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-process-terminal" ok "status=Failed no-catalogue negative path is explicit"
    elif [[ "$process_status" == "Completed" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-process-terminal" ok "status=Completed"
    else
      record_check "runtime-$label-virtualsoulfind-v2-process-terminal" fail "timeout last=$process_body"
      return 1
    fi

    plan_body="$(auth_post_json "$(v2_url "$base_url/plans")" "{\"domain\":\"Music\",\"trackId\":\"$track_id\"}")"
    if [[ "$plan_body" == *"$track_id"* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-plan" ok "plan response references track"
    else
      record_check "runtime-$label-virtualsoulfind-v2-plan" fail "$plan_body"
      return 1
    fi

    artists_body="$(auth_get "$(v2_url "$base_url/catalogue/artists/search?query=interop&limit=10")")"
    if [[ "$artists_body" == \[* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-catalogue-search" ok "artist search returned JSON array"
    else
      record_check "runtime-$label-virtualsoulfind-v2-catalogue-search" fail "$artists_body"
      return 1
    fi

    artist_id="$(printf '%s' "$artists_body" | json_get '0.artistId' 2>/dev/null || true)"
    if [[ -n "$artist_id" ]]; then
      release_body="$(auth_get "$(v2_url "$base_url/catalogue/artists/$artist_id/releases?limit=10")")"
      release_group_id="$(printf '%s' "$release_body" | json_get '0.releaseGroupId' 2>/dev/null || true)"
      tracks_body=""
      if [[ -n "$release_group_id" ]]; then
        tracks_body="$(auth_get "$(v2_url "$base_url/catalogue/releases/$release_group_id/tracks")")"
      fi
      positive_track_id="$(printf '%s' "$tracks_body" | json_get '0.trackId' 2>/dev/null || true)"
      if [[ -z "$positive_track_id" ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-workflow" fail \
          "artist=$artist_id release=$release_group_id tracks=$tracks_body"
        return 1
      fi
      positive_response="$(auth_post_json_with_status \
        "$(v2_url "$base_url/plans")" \
        "{\"domain\":\"Music\",\"trackId\":\"$positive_track_id\",\"mode\":\"SoulseekFriendly\",\"priority\":\"High\"}")"
      response_status="$(printf '%s\n' "$positive_response" | tail -n 1)"
      response_body="$(printf '%s\n' "$positive_response" | sed '$d')"
      if [[ "$response_status" != "200" || "$response_body" != *'"trackId"'* ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-plan" fail \
          "status=$response_status body=$response_body"
        return 1
      fi
      if [[ "$label" == "slskr" && "$response_body" != *'"status":"Ready"'* ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-plan" fail \
          "local catalogue plan was not executable: $response_body"
        return 1
      fi
      record_check "runtime-$label-virtualsoulfind-v2-catalogue-plan" ok \
        "track=$positive_track_id status=$(printf '%s' "$response_body" | json_get status 2>/dev/null || true)"

      positive_response="$(auth_post_json_with_status \
        "$(v2_url "$base_url/intents/tracks")" \
        "{\"domain\":\"Music\",\"trackId\":\"$positive_track_id\",\"priority\":\"High\"}")"
      response_status="$(printf '%s\n' "$positive_response" | tail -n 1)"
      response_body="$(printf '%s\n' "$positive_response" | sed '$d')"
      positive_intent_id="$(printf '%s' "$response_body" | json_get desiredTrackId 2>/dev/null || true)"
      if [[ "$response_status" != "201" || -z "$positive_intent_id" ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-intent" fail \
          "status=$response_status body=$response_body"
        return 1
      fi
      positive_response="$(auth_post_json_with_status \
        "$(v2_url "$base_url/intents/tracks/$positive_intent_id/process")" '{}')"
      response_status="$(printf '%s\n' "$positive_response" | tail -n 1)"
      if [[ "$response_status" != "202" ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-process" fail \
          "status=$response_status body=$(printf '%s\n' "$positive_response" | sed '$d')"
        return 1
      fi
      positive_status="Pending"
      process_body=""
      local positive_deadline=$((SECONDS + timeout_seconds))
      while ((SECONDS < positive_deadline)); do
        process_body="$(auth_get "$(v2_url "$base_url/intents/tracks/$positive_intent_id")" 2>/dev/null || true)"
        positive_status="$(printf '%s' "$process_body" | json_get status 2>/dev/null || true)"
        if [[ "$positive_status" == "Completed" || "$positive_status" == "Failed" ]]; then
          break
        fi
        sleep 1
      done
      if [[ "$positive_status" == "Completed" ]]; then
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-process" ok \
          "track=$positive_track_id status=Completed source=$(printf '%s' "$process_body" | json_get plannedSources 2>/dev/null || true)"
      else
        record_check "runtime-$label-virtualsoulfind-v2-catalogue-process" fail \
          "track=$positive_track_id status=$positive_status body=$process_body"
        return 1
      fi
    elif [[ "$label" == "slskr" ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-catalogue-workflow" fail \
        "live shared Interop Artist fixture was not indexed: $artists_body"
      return 1
    else
      record_check "runtime-$label-virtualsoulfind-v2-catalogue-workflow" ok \
        "current target returned an empty catalogue for the clean fixture"
    fi

    stats_body="$(auth_get "$(v2_url "$base_url/stats")")"
    if [[ "$stats_body" == *'"totalProcessed"'* && "$stats_body" == *'"pendingCount"'* ]]; then
      record_check "runtime-$label-virtualsoulfind-v2-stats" ok "processor counters exposed"
    else
      record_check "runtime-$label-virtualsoulfind-v2-stats" fail "$stats_body"
      return 1
    fi
  done
}

run_obfuscated_peer_interop_checks() {
  local obfuscated_allow_plain_response obfuscated_peer_log obfuscated_peer_output obfuscated_response_contract
  obfuscated_peer_log="$work_dir/slskr-obfuscated-peer.log"
  obfuscated_allow_plain_response="${SLSKR_CROSS_CLIENT_OBFUSCATED_ALLOW_PLAIN_RESPONSE:-true}"
  if [[ -z "$upstream_username" || -z "$upstream_password" ]]; then
    record_check protocol-slskr-obfuscated-peer-slskdn fail "upstream probe credentials are required"
    return 1
  fi

  if obfuscated_peer_output="$(
    SLSK_SERVER="$server_endpoint" \
    SLSK_USERNAME="$upstream_username" \
    SLSK_PASSWORD="$upstream_password" \
    SLSK_OBFUSCATED_PEER_USERNAME="$slskdn_username" \
    SLSK_OBFUSCATED_HOST_OVERRIDE=127.0.0.1 \
    SLSK_OBFUSCATED_PORT_OVERRIDE="$slskdn_obfuscated_port" \
    SLSK_OBFUSCATED_ALLOW_PLAIN_RESPONSE="$obfuscated_allow_plain_response" \
    SLSK_OBFUSCATED_PEER_ADDRESS_ATTEMPTS=3 \
    SLSK_OBFUSCATED_PROBE_TIMEOUT_SECONDS=20 \
      timeout 90 "$slskr_binary" probe obfuscated-peer 2>&1
  )"; then
    printf '%s\n' "$obfuscated_peer_output" >"$obfuscated_peer_log"
    if [[ "$obfuscated_peer_output" == *"plain-response fallback"* ]]; then
      obfuscated_response_contract="plain-fallback"
    else
      obfuscated_response_contract="obfuscated"
    fi
    record_check protocol-slskr-obfuscated-peer-slskdn ok \
      "peer=$slskdn_username port=$slskdn_obfuscated_port probe_contract=obfuscated-peer-v1 response_contract=$obfuscated_response_contract"
  else
    printf '%s\n' "${obfuscated_peer_output:-no response}" >"$obfuscated_peer_log"
    record_check protocol-slskr-obfuscated-peer-slskdn fail \
      "detail=$(tail -n 6 "$obfuscated_peer_log" 2>/dev/null | tr '\n\t' ' ')"
    return 1
  fi
}

run_browse_interop_checks() {
  local escaped_slskr body before_listeners after_listeners
  local before_obfuscated_accepts before_obfuscated_messages after_obfuscated_accepts
  local after_obfuscated_messages expect_reverse_obfuscated
  escaped_slskr="$(url_escape "$slskr_username")"
  expect_reverse_obfuscated="${SLSKR_CROSS_CLIENT_EXPECT_REVERSE_OBFUSCATED:-0}"
  if [[ "${SLSKR_CROSS_CLIENT_TARGET_OBFUSCATED_LOOPBACK_OVERRIDE:-0}" == "1" ]]; then
    expect_reverse_obfuscated=1
  fi

  wait_slskr_connected
  wait_slskdn_connected

  if [[ "$expect_reverse_obfuscated" == "1" ]]; then
    before_listeners="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners")"
    before_obfuscated_accepts="$(printf '%s' "$before_listeners" | json_get obfuscated_accepts 2>/dev/null || true)"
    before_obfuscated_messages="$(printf '%s' "$before_listeners" | json_get obfuscated_peer_messages 2>/dev/null || true)"
  fi

  body="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/browse")"
  if printf '%s' "$body" | json_find_string "$slskr_fixture_name" 2>/dev/null; then
    record_check protocol-slskdn-browses-slskr ok "fixture=$slskr_fixture_name"
  else
    record_check protocol-slskdn-browses-slskr fail "$body"
    return 1
  fi

  if [[ "$expect_reverse_obfuscated" == "1" ]]; then
    after_listeners="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/listeners")"
    after_obfuscated_accepts="$(printf '%s' "$after_listeners" | json_get obfuscated_accepts 2>/dev/null || true)"
    after_obfuscated_messages="$(printf '%s' "$after_listeners" | json_get obfuscated_peer_messages 2>/dev/null || true)"
    if [[ "$before_obfuscated_accepts" =~ ^[0-9]+$ ]] \
      && [[ "$before_obfuscated_messages" =~ ^[0-9]+$ ]] \
      && [[ "$after_obfuscated_accepts" =~ ^[0-9]+$ ]] \
      && [[ "$after_obfuscated_messages" =~ ^[0-9]+$ ]] \
      && ((after_obfuscated_accepts > before_obfuscated_accepts)) \
      && ((after_obfuscated_messages > before_obfuscated_messages)); then
      record_check protocol-slskdn-obfuscated-peer-slskr ok \
        "transport=type1-obfuscated request=browse accepts=${before_obfuscated_accepts}->${after_obfuscated_accepts} messages=${before_obfuscated_messages}->${after_obfuscated_messages}"
    else
      record_check protocol-slskdn-obfuscated-peer-slskr fail \
        "transport=type1-obfuscated counters did not advance before=${before_obfuscated_accepts}/${before_obfuscated_messages} after=${after_obfuscated_accepts}/${after_obfuscated_messages}"
      return 1
    fi
  fi

}

run_reverse_browse_interop_check() {
  local escaped_slskdn
  escaped_slskdn="$(url_escape "$slskdn_username")"

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
      timeout 75 "$slskr_binary" probe search-peer >>"$diag_file" 2>&1; then
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
      timeout 75 "$slskr_binary" probe search-peer >>"$diag_file" 2>&1; then
    record_check protocol-slskdn-searches-slskr ok "query=slskr expected=$slskr_fixture_name"
  else
    search_diagnostics="$(rust_search_diagnostics)"
    record_check protocol-slskdn-searches-slskr fail "detail=$(tail -n 1 "$diag_file") rust_diagnostics=${search_diagnostics:-none}"
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

run_room_interop_checks() {
  local room target_message rust_message status=0
  room="${SLSKR_CROSS_CLIENT_ROOM_NAME:-slskr-live-interop}"
  if ! auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/rooms/joined" "\"$room\"" >/dev/null; then
    record_check protocol-slskdn-public-room fail "target room join failed"
    record_check protocol-slskr-public-room-slskdn fail "target room join failed"
    return 1
  fi
  if ! auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/rooms/$room/join" '{}' >/dev/null; then
    record_check protocol-slskdn-public-room fail "slskr room join failed"
    record_check protocol-slskr-public-room-slskdn fail "slskr room join failed"
    return 1
  fi
  sleep 3

  target_message="slskdn-room-to-slskr-$(date +%s%N)"
  if ! auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/rooms/joined/$room/messages" "\"$target_message\"" >/dev/null; then
    record_check protocol-slskdn-public-room fail "room=$room target_message=$target_message"
    status=1
  elif ! wait_json_contains protocol-slskdn-public-room "http://127.0.0.1:$slskr_http_port/api/rooms/joined/$room/messages" "$target_message"; then
    status=1
  fi

  rust_message="slskr-room-to-slskdn-$(date +%s%N)"
  if ! auth_post_json "http://127.0.0.1:$slskr_http_port/api/rooms/joined/$room/messages" "{\"body\":\"$rust_message\"}" >/dev/null; then
    record_check protocol-slskr-public-room-slskdn fail "room=$room rust_message=$rust_message"
    status=1
  elif ! wait_json_contains protocol-slskr-public-room-slskdn "http://127.0.0.1:$slskdn_http_port/api/v0/rooms/joined/$room/messages" "$rust_message"; then
    status=1
  fi
  return "$status"
}

run_mesh_runtime_checks() {
  local escaped_slskr escaped_slskdn capability_probe slskr_capabilities slskdn_capabilities overlay_pin overlay_output health stats transport ticket mesh_status=0
  local mesh_sync_target_first mesh_sync_target_second mesh_sync_replacement_first mesh_sync_replacement_second
  local mesh_sync_target_first_status mesh_sync_target_second_status mesh_sync_replacement_first_status mesh_sync_replacement_second_status
  local mesh_sync_target_first_body mesh_sync_target_second_body mesh_sync_replacement_first_body mesh_sync_replacement_second_body
  escaped_slskr="$(url_escape "$slskr_username")"
  escaped_slskdn="$(url_escape "$slskdn_username")"

  capability_probe="$(auth_post_json "http://127.0.0.1:$slskr_http_port/api/mesh/sync/$escaped_slskdn" '{}')"
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

  # The exact frozen slskdN build exposes the mesh-sync controller, but its
  # outbound MeshSyncService transport is intentionally unavailable.  The
  # replacement's slskdn compatibility profile preserves that observable
  # target contract at the versioned route.  Exercise two attempts against
  # each live daemon so a retry cannot turn the target's stable failure into a
  # false positive or silently mutate state.
  mesh_sync_target_first="$(auth_post_json_with_status \
    "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/sync/$escaped_slskr" '{}' 2>/dev/null || true)"
  mesh_sync_target_second="$(auth_post_json_with_status \
    "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/sync/$escaped_slskr" '{}' 2>/dev/null || true)"
  mesh_sync_replacement_first="$(auth_post_json_with_status \
    "http://127.0.0.1:$slskr_http_port/api/v0/mesh/sync/$escaped_slskdn" '{}' 2>/dev/null || true)"
  mesh_sync_replacement_second="$(auth_post_json_with_status \
    "http://127.0.0.1:$slskr_http_port/api/v0/mesh/sync/$escaped_slskdn" '{}' 2>/dev/null || true)"
  mesh_sync_target_first_status="${mesh_sync_target_first##*$'\n'}"
  mesh_sync_target_second_status="${mesh_sync_target_second##*$'\n'}"
  mesh_sync_replacement_first_status="${mesh_sync_replacement_first##*$'\n'}"
  mesh_sync_replacement_second_status="${mesh_sync_replacement_second##*$'\n'}"
  mesh_sync_target_first_body="${mesh_sync_target_first%$'\n'*}"
  mesh_sync_target_second_body="${mesh_sync_target_second%$'\n'*}"
  mesh_sync_replacement_first_body="${mesh_sync_replacement_first%$'\n'*}"
  mesh_sync_replacement_second_body="${mesh_sync_replacement_second%$'\n'*}"
  if [[ "$mesh_sync_target_first_status" == "400" \
    && "$mesh_sync_target_second_status" == "400" \
    && "$mesh_sync_replacement_first_status" == "400" \
    && "$mesh_sync_replacement_second_status" == "400" \
    && "$mesh_sync_target_first_body" == '{"error":"Failed to sync with peer"}' \
    && "$mesh_sync_target_second_body" == "$mesh_sync_target_first_body" \
    && "$mesh_sync_replacement_first_body" == "$mesh_sync_target_first_body" \
    && "$mesh_sync_replacement_second_body" == "$mesh_sync_target_first_body" ]]; then
    # The current target has no usable outbound mesh transport for this clean
    # fixture. Matching its stable 400 response on both attempts and both
    # daemons is the compatibility contract; it is not a claim that positive
    # mesh synchronization was exercised.
    record_check protocol-ksdn-mesh-sync-reconnect-retry ok \
      'expected-target-negative status=400 body={"error":"Failed to sync with peer"} target_attempts=400,400 replacement_attempts=400,400'
  else
    record_check protocol-ksdn-mesh-sync-reconnect-retry fail \
      "mesh-sync retry contract mismatch target=${mesh_sync_target_first_status}/${mesh_sync_target_second_status} replacement=${mesh_sync_replacement_first_status}/${mesh_sync_replacement_second_status}"
  fi

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
    SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_endpoint_port" \
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

  probe_overlay_endpoint() {
    local candidate output last_output=""
    local candidates=("$slskdn_overlay_port")
    if [[ "$slskdn_listen_port" != "$slskdn_overlay_port" ]]; then
      # Recent slskdN builds demultiplex TLS mesh overlay connections on the
      # Soulseek TCP listener instead of opening a second TCP port. Older
      # builds retain a standalone overlay listener on dht.overlay_port.
      candidates+=("$slskdn_listen_port")
    fi
    for candidate in "${candidates[@]}"; do
      if output="$(
        SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$candidate" \
        SLSKR_OVERLAY_CERTIFICATE_SHA256="$overlay_pin" \
        SLSKR_OVERLAY_SERVICE=dht \
        SLSKR_OVERLAY_METHOD=Ping \
        SLSKR_OVERLAY_PAYLOAD='{"RequesterId":"AAAAAAAAAAAAAAAAAAAAAAAAAAA="}' \
        SLSKR_OVERLAY_EXPECTED=Timestamp \
        SLSK_USERNAME="$slskr_username" \
        SLSK_PEER_USERNAME="$slskdn_username" \
          "$slskr_binary" probe overlay-service 2>&1
      )"; then
        slskdn_overlay_endpoint_port="$candidate"
        overlay_probe_output="$output"
        return 0
      fi
      last_output="endpoint=$candidate $output"
    done
    overlay_probe_output="$last_output"
    return 1
  }

  overlay_probe_output=""
  if probe_overlay_endpoint; then
    overlay_output="$overlay_probe_output"
    printf '\n[pinned-overlay-service]\n%s\n' "$overlay_output" >>"$diag_file"
    record_check protocol-pinned-overlay-service ok "endpoint=127.0.0.1:$slskdn_overlay_endpoint_port dht.Ping returned a timestamp"
  else
    printf '\n[pinned-overlay-service-failed]\n%s\n' "$overlay_probe_output" >>"$diag_file"
    record_check protocol-pinned-overlay-service fail "$overlay_probe_output"
    return 1
  fi

  library_items="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/library/items?query=$(url_escape "$slskdn_v2_fixture_name")&limit=10")"
  slskdn_content_id="$(printf '%s' "$library_items" | node -e '
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  const filename = process.argv[1];
  const body = JSON.parse(input);
  const item = (body.items || []).find(candidate => candidate.fileName === filename);
  if (item?.contentId) process.stdout.write(item.contentId);
});
' "$slskdn_v2_fixture_name")"
  if [[ -z "$slskdn_content_id" ]]; then
    record_check runtime-slskdn-mesh-content-id fail "$library_items"
    return 1
  fi
  record_check runtime-slskdn-mesh-content-id ok "contentId=$slskdn_content_id"

  mesh_content_payload="$(node -e '
const [contentId, lengthText] = process.argv.slice(1);
process.stdout.write(JSON.stringify({ contentId, range: { offset: 0, length: Number(lengthText) } }));
' "$slskdn_content_id" "$slskdn_v2_fixture_size")"
  if mesh_content_output="$(overlay_service_call GetByContentId "$mesh_content_payload" '' MeshContent "$slskdn_v2_fixture_sha")"; then
    printf '\n[mesh-content-exact-bytes]\n%s\n' "$mesh_content_output" >>"$diag_file"
    record_check protocol-slskr-mesh-content-slskdn ok "bytes=$slskdn_v2_fixture_size sha256=$slskdn_v2_fixture_sha"
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
    mesh_status=1
  fi

  if pod_get_output="$(overlay_service_call Get "{\"PodId\":\"$pod_id\"}" "$pod_id")"; then
    printf '\n[pods-get]\n%s\n' "$pod_get_output" >>"$diag_file"
    record_check protocol-slskr-pods-get-slskdn ok "pod metadata fetched over pinned overlay"
  else
    printf '\n[pods-get-failed]\n%s\n' "$pod_get_output" >>"$diag_file"
    record_check protocol-slskr-pods-get-slskdn fail "$pod_get_output"
    mesh_status=1
  fi

  if pod_join_output="$(overlay_service_call Join "{\"PodId\":\"$pod_id\",\"Role\":\"member\"}" '"Success":true')"; then
    printf '\n[pods-join]\n%s\n' "$pod_join_output" >>"$diag_file"
    record_check protocol-slskr-pods-join-slskdn ok "remote overlay identity joined pod"
  else
    printf '\n[pods-join-failed]\n%s\n' "$pod_join_output" >>"$diag_file"
    record_check protocol-slskr-pods-join-slskdn fail "$pod_join_output"
    mesh_status=1
  fi

  if pod_post_output="$(overlay_service_call PostMessage "{\"PodId\":\"$pod_id\",\"ChannelId\":\"general\",\"Body\":\"$pod_message\"}" '"Success":true')"; then
    printf '\n[pods-post-message]\n%s\n' "$pod_post_output" >>"$diag_file"
    record_check protocol-slskr-pods-post-slskdn ok "member message stored over pinned overlay"
  else
    printf '\n[pods-post-message-failed]\n%s\n' "$pod_post_output" >>"$diag_file"
    record_check protocol-slskr-pods-post-slskdn fail "$pod_post_output"
    mesh_status=1
  fi

  if pod_messages_output="$(overlay_service_call GetMessages "{\"PodId\":\"$pod_id\",\"ChannelId\":\"general\"}" "$pod_message")"; then
    printf '\n[pods-get-messages]\n%s\n' "$pod_messages_output" >>"$diag_file"
    record_check protocol-slskr-pods-messages-slskdn ok "stored member message polled over pinned overlay"
  else
    printf '\n[pods-get-messages-failed]\n%s\n' "$pod_messages_output" >>"$diag_file"
    record_check protocol-slskr-pods-messages-slskdn fail "$pod_messages_output"
    mesh_status=1
  fi

  if pod_leave_output="$(overlay_service_call Leave "{\"PodId\":\"$pod_id\"}" '"Success":true')"; then
    printf '\n[pods-leave]\n%s\n' "$pod_leave_output" >>"$diag_file"
    record_check protocol-slskr-pods-leave-slskdn ok "remote overlay identity left pod"
  else
    printf '\n[pods-leave-failed]\n%s\n' "$pod_leave_output" >>"$diag_file"
    record_check protocol-slskr-pods-leave-slskdn fail "$pod_leave_output"
    mesh_status=1
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
  requestingPeerId: gatewayPeerId
}));
' "$gateway_pod_id" "$gateway_peer_id" "$gateway_echo_host" "$gateway_echo_port")"
  if gateway_pod_create="$(auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/pods" "$gateway_pod_payload")" \
    && [[ "$gateway_pod_create" == *"$gateway_pod_id"* ]]; then
    record_check runtime-slskdn-gateway-pod-create ok "pod=$gateway_pod_id"
  else
    record_check runtime-slskdn-gateway-pod-create fail "${gateway_pod_create:-request failed}"
    mesh_status=1
  fi

  if gateway_join_output="$(overlay_service_call Join "{\"PodId\":\"$gateway_pod_id\",\"Role\":\"member\"}" '"Success":true')"; then
    printf '\n[gateway-pod-join]\n%s\n' "$gateway_join_output" >>"$diag_file"
    record_check protocol-slskr-gateway-pod-join-slskdn ok "remote overlay identity joined gateway pod"
  else
    printf '\n[gateway-pod-join-failed]\n%s\n' "$gateway_join_output" >>"$diag_file"
    record_check protocol-slskr-gateway-pod-join-slskdn fail "$gateway_join_output"
    mesh_status=1
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
    mesh_status=1
    gateway_tunnel_id=""
  fi

  gateway_echo_message="slskr-private-gateway-$(date +%s%N)"
  gateway_echo_base64="$(printf '%s' "$gateway_echo_message" | base64 -w0)"
  if [[ -z "${gateway_tunnel_id:-}" ]]; then
    gateway_send_output="gateway tunnel was not opened"
    record_check protocol-slskr-gateway-send-slskdn fail "$gateway_send_output"
    mesh_status=1
  elif gateway_send_output="$(overlay_service_call TunnelData "{\"TunnelId\":\"$gateway_tunnel_id\",\"Data\":\"$gateway_echo_base64\"}" '"Sent":' private-gateway)"; then
    printf '\n[gateway-tunnel-data]\n%s\n' "$gateway_send_output" >>"$diag_file"
    record_check protocol-slskr-gateway-send-slskdn ok "tunnel payload accepted"
  else
    printf '\n[gateway-tunnel-data-failed]\n%s\n' "$gateway_send_output" >>"$diag_file"
    record_check protocol-slskr-gateway-send-slskdn fail "$gateway_send_output"
    mesh_status=1
  fi

  gateway_receive_output=""
  gateway_received=0
  if [[ -n "${gateway_tunnel_id:-}" ]]; then
    for _ in $(seq 1 20); do
      if gateway_receive_output="$(overlay_service_call GetTunnelData "{\"TunnelId\":\"$gateway_tunnel_id\"}" "$gateway_echo_base64" private-gateway)"; then
        gateway_received=1
        break
      fi
      sleep 0.25
    done
  else
    gateway_receive_output="gateway tunnel was not opened"
  fi
  if [[ "$gateway_received" == "1" ]]; then
    printf '\n[gateway-get-tunnel-data]\n%s\n' "$gateway_receive_output" >>"$diag_file"
    record_check protocol-slskr-gateway-receive-slskdn ok "exact echo payload returned"
  else
    printf '\n[gateway-get-tunnel-data-failed]\n%s\n' "$gateway_receive_output" >>"$diag_file"
    record_check protocol-slskr-gateway-receive-slskdn fail "echo payload unavailable"
    mesh_status=1
  fi

  if [[ -z "${gateway_tunnel_id:-}" ]]; then
    gateway_close_output="gateway tunnel was not opened"
    record_check protocol-slskr-gateway-close-slskdn fail "$gateway_close_output"
    mesh_status=1
  elif gateway_close_output="$(overlay_service_call CloseTunnel "{\"TunnelId\":\"$gateway_tunnel_id\"}" '"Closed":true' private-gateway)"; then
    printf '\n[gateway-close-tunnel]\n%s\n' "$gateway_close_output" >>"$diag_file"
    record_check protocol-slskr-gateway-close-slskdn ok "private TCP tunnel closed"
  else
    printf '\n[gateway-close-tunnel-failed]\n%s\n' "$gateway_close_output" >>"$diag_file"
    record_check protocol-slskr-gateway-close-slskdn fail "$gateway_close_output"
    mesh_status=1
  fi

  if dht_store_output="$(
    SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_endpoint_port" \
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
    mesh_status=1
  fi

  if ! health="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/health?api-version=1.0")" \
    || ! printf '%s' "$health" | json_get routingNodes >/dev/null 2>&1; then
    record_check runtime-slskdn-mesh-health fail "${health:-request failed}"
    mesh_status=1
  else
    record_check runtime-slskdn-mesh-health ok "$(printf '%s' "$health" | tr '\n\t' '  ')"
  fi

  if ! stats="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/stats")" \
    || ! printf '%s' "$stats" | json_get totalSyncs >/dev/null 2>&1; then
    record_check runtime-slskdn-mesh-stats fail "${stats:-request failed}"
    mesh_status=1
  else
    record_check runtime-slskdn-mesh-stats ok "$(printf '%s' "$stats" | tr '\n\t' '  ')"
  fi

  if ! transport="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/mesh/transport")" \
    || ! printf '%s' "$transport" | json_get natType >/dev/null 2>&1; then
    record_check network-slskdn-mesh-transport fail "${transport:-request failed}"
    mesh_status=1
  else
    record_check network-slskdn-mesh-transport ok "$(printf '%s' "$transport" | tr '\n\t' '  ')"
  fi

  ticket="$(auth_post_json "http://127.0.0.1:$slskdn_http_port/api/v0/mesh-streams/tickets" "{\"contentId\":\"interop-content\",\"peerId\":\"$slskr_username\",\"filename\":\"Interop/Test.flac\",\"expectedSize\":0}")"
  if [[ "$ticket" == *"\"source\":\"mesh\""* && "$ticket" == *"streamUrl"* ]]; then
    record_check runtime-slskdn-mesh-stream-ticket ok "$ticket"
  else
    record_check runtime-slskdn-mesh-stream-ticket fail "$ticket"
    mesh_status=1
  fi

  ticket="$(auth_post_json "http://127.0.0.1:$slskr_http_port/api/v0/mesh-streams/tickets" "{\"contentId\":\"interop-content\",\"filename\":\"Interop/Test.flac\",\"peerId\":\"$slskdn_username\"}")"
  if [[ "$ticket" == *"streamUrl"* ]]; then
    record_check runtime-slskr-mesh-stream-ticket ok "$ticket"
  else
    record_check runtime-slskr-mesh-stream-ticket fail "$ticket"
    mesh_status=1
  fi

  return "$mesh_status"
}

run_advanced_transport_interop_checks() {
  local output before detail transport_status=0
  local udp_needle='[Overlay] Using legacy envelope handling without peer validation - security reduced'
  local quic_control_needle='[Overlay-QUIC] Received control probe'
  local quic_data_needle='[Overlay-QUIC-DATA] Received '
  local quic_control_unavailable='[DI] QUIC overlay requested but runtime/platform support is unavailable'
  local quic_data_unavailable='[DI] QUIC data overlay requested but runtime/platform support is unavailable'

  before="$(target_log_count "$udp_needle")"
  if output="$(
    SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_port" \
      "$slskr_binary" probe overlay-udp 2>&1
  )" && wait_target_log_delta "$udp_needle" "$before"; then
    detail="target_log=legacy-control-dispatch accepted endpoint=127.0.0.1:$slskdn_overlay_port probe=$(printf '%s' "$output" | tr '\n\t' ' ')"
    record_check protocol-slskr-overlay-udp-slskdn ok "$detail"
  else
    record_check protocol-slskr-overlay-udp-slskdn fail \
      "target_log=legacy-control-dispatch not observed output=$(printf '%s' "${output:-none}" | tr '\n\t' ' ')"
    transport_status=1
  fi

  if grep -Fq -- "$quic_control_unavailable" "$slskdn_log"; then
    record_check protocol-slskr-overlay-quic-control-slskdn skip \
      "target runtime reports QUIC control unavailable; no MsQuic listener is available for this environment"
  else
    before="$(target_log_count "$quic_control_needle")"
    if output="$(
      SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_quic_backend_port" \
        "$slskr_binary" probe overlay-quic-control 2>&1
    )" && wait_target_log_delta "$quic_control_needle" "$before"; then
      detail="target_log=quic-control-dispatch accepted endpoint=127.0.0.1:$slskdn_quic_backend_port probe=$(printf '%s' "$output" | tr '\n\t' ' ')"
      record_check protocol-slskr-overlay-quic-control-slskdn ok "$detail"
    else
      record_check protocol-slskr-overlay-quic-control-slskdn fail \
        "target_log=quic-control-receipt not observed output=$(printf '%s' "${output:-none}" | tr '\n\t' ' ')"
      transport_status=1
    fi
  fi

  if grep -Fq -- "$quic_data_unavailable" "$slskdn_log"; then
    record_check protocol-slskr-quic-data-slskdn skip \
      "target runtime reports QUIC data unavailable; no MsQuic listener is available for this environment"
  else
    before="$(target_log_count "$quic_data_needle")"
    if output="$(
      SLSKR_OVERLAY_ENDPOINT="127.0.0.1:$slskdn_overlay_port" \
        "$slskr_binary" probe quic-data 2>&1
    )" && wait_target_log_delta "$quic_data_needle" "$before"; then
      detail="target_log=quic-data-server accepted shared-endpoint=127.0.0.1:$slskdn_overlay_port backend=127.0.0.1:$slskdn_quic_data_port probe=$(printf '%s' "$output" | tr '\n\t' ' ')"
      record_check protocol-slskr-quic-data-slskdn ok "$detail"
    else
      record_check protocol-slskr-quic-data-slskdn fail \
        "target_log=quic-data-receipt not observed output=$(printf '%s' "${output:-none}" | tr '\n\t' ' ')"
      transport_status=1
    fi
  fi

  local route_payload route_response route_ok
  route_payload="$(node -e '
const peer = process.argv[1];
const sender = process.argv[2];
process.stdout.write(JSON.stringify({
  message: {
    messageId: `slskr-reverse-overlay-negative-${Date.now()}`,
    podId: "probe",
    channelId: "general",
    senderPeerId: sender,
    body: "strict reverse overlay route probe",
    timestampUnixMs: Date.now(),
    signature: "",
    sigVersion: 1
  },
  targetPeerIds: [peer]
}));
' "$slskr_username" "$slskdn_username")"
  for check in \
    protocol-slskdn-overlay-udp-slskr \
    protocol-slskdn-overlay-quic-control-slskr \
    protocol-slskdn-quic-data-slskr; do
    route_response=""
    if route_response="$(auth_post_json \
      "http://127.0.0.1:$slskdn_http_port/api/v0/podcore/routing/route-to-peers" \
      "$route_payload" 2>&1)"; then
      route_ok="$(printf '%s' "$route_response" | node -e '
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  try {
    const body = JSON.parse(input);
    const failedPeer = Array.isArray(body.failedPeerIds) && body.failedPeerIds.includes(process.argv[1]);
    process.exit(body.success === false && body.failedRoutingCount === 1 && failedPeer ? 0 : 1);
  } catch {
    process.exit(1);
  }
});
' "$slskr_username" 2>/dev/null && printf true || printf false)"
    else
      route_ok=false
    fi
    if [[ "$route_ok" == true ]]; then
      record_check "$check" ok \
        "expected-target-negative endpoint-resolution-unavailable success=false failedRoutingCount=1"
    else
      record_check "$check" fail \
        "target reverse route did not return expected endpoint-resolution negative response body=$(printf '%s' "${route_response:-none}" | tr '\n\t' ' ')"
      transport_status=1
    fi
  done

  return "$transport_status"
}

probe_peer_address slskr "$slskr_username" || true
probe_peer_address slskdn "$slskdn_username" || true

run_user_watch_interop_checks() {
  local escaped_slskr user_watch_log target_user_status target_user_info
  escaped_slskr="$(url_escape "$slskr_username")"
  user_watch_log="$work_dir/slskr-user-watch.log"
  if [[ "$upstream_username" != "$slskr_username" && "$upstream_username" != "$slskdn_username" ]] \
    && SLSK_SERVER="$server_endpoint" \
    SLSK_USERNAME="$upstream_username" \
    SLSK_PASSWORD="$upstream_password" \
    SLSK_PEER_USERNAME="$slskdn_username" \
      "$slskr_binary" probe user-watch >"$user_watch_log" 2>&1; then
    record_check protocol-slskr-user-watch-slskdn ok "watched=$slskdn_username stats=received"
  else
    record_check protocol-slskr-user-watch-slskdn fail "detail=$(tail -n 4 "$user_watch_log" 2>/dev/null | tr '\n\t' '  ')"
    return 1
  fi

  target_user_status="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/status" 2>/dev/null || true)"
  target_user_info="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/users/$escaped_slskr/info" 2>/dev/null || true)"
  if [[ -n "$target_user_status" && -n "$target_user_info" ]]; then
    record_check protocol-slskdn-user-watch-slskr ok "status-and-info=$slskr_username"
  else
    record_check protocol-slskdn-user-watch-slskr fail "status=$target_user_status info=$target_user_info"
    return 1
  fi
}

run_distributed_peer_interop_checks() {
  local distributed_peer_log distributed_target_state distributed_target_summary distributed_target_ready
  local distributed_reverse_state distributed_reverse_ready
  distributed_peer_log="$work_dir/slskr-distributed-peer.log"
  distributed_target_state=""
  distributed_target_summary=""
  distributed_target_ready=false
  distributed_reverse_state=""
  distributed_reverse_ready=false
  if [[ "$upstream_username" != "$slskr_username" && "$upstream_username" != "$slskdn_username" ]]; then
    for _ in $(seq 1 "${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_ATTEMPTS:-60}"); do
      distributed_target_state="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null || true)"
      if [[ "$(printf '%s' "$distributed_target_state" | json_get distributedNetwork.canAcceptChildren 2>/dev/null || true)" == "true" ]]; then
        distributed_target_ready=true
        break
      fi
      sleep "${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_DELAY_SECONDS:-1}"
    done
  fi
  printf '\n[distributed-target-state]\n%s\n' "$distributed_target_state" >>"$diag_file"
  distributed_target_summary="$(printf '%s' "$distributed_target_state" | node -e '
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  try {
    const state = JSON.parse(input);
    const distributed = state.distributedNetwork || {};
    const server = state.server || {};
    process.stdout.write(JSON.stringify({
      isLoggedIn: server.isLoggedIn,
      canAcceptChildren: distributed.canAcceptChildren,
      hasParent: distributed.hasParent,
      branchLevel: distributed.branchLevel,
      childCount: Array.isArray(distributed.children) ? distributed.children.length : undefined,
      childLimit: distributed.childLimit,
      state: distributed.state,
    }));
  } catch {
    process.exit(1);
  }
});
' 2>/dev/null || true)"
  if [[ "$distributed_target_ready" == true ]] \
    && SLSK_SERVER="$server_endpoint" \
    SLSK_USERNAME="$upstream_username" \
    SLSK_PASSWORD="$upstream_password" \
    SLSK_PEER_USERNAME="$slskdn_username" \
    SLSK_DISTRIBUTED_PEER_USERNAME="$slskdn_username" \
    SLSK_DISTRIBUTED_HOST_OVERRIDE=127.0.0.1 \
    SLSK_DISTRIBUTED_PORT_OVERRIDE="$slskdn_listen_port" \
      "$slskr_binary" probe distributed-peer >"$distributed_peer_log" 2>&1; then
    record_check protocol-slskr-distributed-peer-slskdn ok "peer=$slskdn_username ping=received probe_contract=distributed-ping-response-v2"
  elif [[ "$distributed_target_ready" != true ]]; then
    record_check protocol-slskr-distributed-peer-slskdn fail "detail=target distributed network not ready after ${SLSKR_CROSS_CLIENT_DISTRIBUTED_READY_ATTEMPTS:-60}s summary=${distributed_target_summary:-unparseable}"
    return 1
  else
    record_check protocol-slskr-distributed-peer-slskdn fail "detail=$(tail -n 4 "$distributed_peer_log" 2>/dev/null | tr '\n\t' '  ')"
    return 1
  fi

  # The daemon-level override above makes the replacement a live child of the
  # frozen target.  A successful child registration sends BranchLevel and
  # BranchRoot from target to replacement, which is the missing reverse
  # direction.  Require the target to report the replacement child and the
  # replacement to report a non-zero branch before recording the row.
  # Target branch status is asynchronous: a successful distributed probe can
  # return before the target has committed the child and sent BranchLevel and
  # BranchRoot back to the replacement. Allow the normal parent-selection and
  # child-registration work to settle before declaring reverse propagation
  # broken.
  for _ in $(seq 1 "${SLSKR_CROSS_CLIENT_DISTRIBUTED_REVERSE_ATTEMPTS:-90}"); do
    distributed_target_state="$(auth_get "http://127.0.0.1:$slskdn_http_port/api/v0/application" 2>/dev/null || true)"
    distributed_reverse_state="$(auth_get "http://127.0.0.1:$slskr_http_port/api/v0/application" 2>/dev/null || true)"
    if printf '%s' "$distributed_target_state" | node -e '
const username = process.argv[1];
let input = "";
process.stdin.on("data", chunk => input += chunk);
process.stdin.on("end", () => {
  try {
    const state = JSON.parse(input);
    const children = state.distributedNetwork?.children;
    process.exit(Array.isArray(children) && children.some(child => String(child).toLowerCase() === username.toLowerCase()) ? 0 : 1);
  } catch {
    process.exit(1);
  }
});
' "$slskr_username" 2>/dev/null \
      && [[ "$(printf '%s' "$distributed_reverse_state" | json_get distributedNetwork.branchLevel 2>/dev/null || true)" =~ ^[1-9][0-9]*$ ]]; then
      distributed_reverse_ready=true
      break
    fi
    sleep 1
  done
  if [[ "$distributed_reverse_ready" == true ]]; then
    record_check protocol-slskdn-distributed-peer-slskr ok \
      "target-child=$slskr_username response=branch-info branch-level=$(printf '%s' "$distributed_reverse_state" | json_get distributedNetwork.branchLevel)"
  else
    record_check protocol-slskdn-distributed-peer-slskr fail \
      "target-child-or-branch-info-not-observed target=$(printf '%s' "$distributed_target_state" | tr '\n\t' ' ' | cut -c1-700) replacement=$(printf '%s' "$distributed_reverse_state" | tr '\n\t' ' ' | cut -c1-700)"
    return 1
  fi
}

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
# slskdN's test endpoint overrides intentionally use the same 60-second
# bounded cache as its normal user endpoint cache. Exercise every target-
# initiated route that depends on the replacement endpoint before running the
# longer upstream obfuscated probe; otherwise the target silently falls back
# to the public server endpoint and reports a false peer-availability failure.
run_browse_interop_checks || status=1
run_slskdn_to_slskr_download || status=1
run_message_interop_checks || status=1
run_slskr_to_slskdn_download || status=1
run_slskr_backfill_probe || status=1
run_obfuscated_peer_interop_checks || status=1
run_search_interop_checks || status=1
run_user_watch_interop_checks || status=1
run_reverse_browse_interop_check || status=1
run_distributed_peer_interop_checks || status=1
run_room_interop_checks || status=1
run_mesh_runtime_checks || status=1
run_advanced_transport_interop_checks || status=1
record_final_diagnostics

failed_checks="$(awk -F '\t' 'NR > 1 && $3 == "fail" { print }' "$result_file")"
if [[ -n "$failed_checks" ]]; then
  status=1
  echo "cross-client interop failed checks:" >&2
  printf '%s\n' "$failed_checks" >&2
fi

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

mv "$result_file" "$final_result_file"
echo "cross-client interop ok"
echo "result_file=$final_result_file"
echo "work_dir=$work_dir"
echo "slskr_user=$(redact "$slskr_username") slskdn_user=$(redact "$slskdn_username")"
