write_download_auto_retry_yaml() {
  local path="$1"
  local no_connect="$2"
  local server_port="$3"
  local listen_port="$4"
  local enabled="$5"
  local retry_delay="$6"
  local check_interval="$7"
  local max_attempts="$8"
  local max_files="$9"
  local max_files_per_peer="${10}"
  local peer_cooldown="${11}"
  local alternate_sources="${12}"
  local max_searches="${13}"
  local tolerance="${14}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\ntransfers:\n  download:\n    auto_retry:\n      enabled: %s\n      retry_delay_seconds: %s\n      check_interval_seconds: %s\n      max_attempts: %s\n      max_files_per_cycle: %s\n      max_files_per_peer_per_cycle: %s\n      peer_cooldown_seconds: %s\n      alternate_sources_enabled: %s\n      max_alternate_source_searches_per_cycle: %s\n      alternate_source_size_tolerance_percent: %s\n' \
    "$no_connect" "$server_port" "$listen_port" "$enabled" "$retry_delay" \
    "$check_interval" "$max_attempts" "$max_files" "$max_files_per_peer" \
    "$peer_cooldown" "$alternate_sources" "$max_searches" "$tolerance" \
    >"$temporary"
  mv "$temporary" "$path"
}

seed_download_auto_retry_failures() {
  local implementation="$1"
  local state="$2"
  if [[ "$implementation" == upstream ]]; then
    sqlite3 "$state/data/transfers.db" <<'SQL'
INSERT INTO Transfers (Id, RequestId, Username, Direction, Filename, Size, StartOffset, BatchId, DestinationDirectory, LocalFilename, Attempts, NextAttemptAt, State, StateDescription, RequestedAt, UpdatedAt, EnqueuedAt, StartedAt, EndedAt, BytesTransferred, AverageSpeed, PlaceInQueue, Exception, BitRate, SampleRate, BitDepth, Length, Artist, Album, Title, TrackNumber, Year, Removed)
VALUES
('00000000-0000-0000-0000-000000000001', NULL, 'fixture-peer-a', 'Download', 'Remote/A-First.flac', 1000, 0, NULL, NULL, NULL, 1, NULL, 144, 'Completed, TimedOut', datetime('now','-40 seconds'), datetime('now','-40 seconds'), NULL, NULL, datetime('now','-40 seconds'), 0, 0, NULL, 'timed out', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0),
('00000000-0000-0000-0000-000000000002', NULL, 'fixture-peer-a', 'Download', 'Remote/A-Second.flac', 1000, 0, NULL, NULL, NULL, 1, NULL, 144, 'Completed, TimedOut', datetime('now','-39 seconds'), datetime('now','-39 seconds'), NULL, NULL, datetime('now','-39 seconds'), 0, 0, NULL, 'timed out', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0),
('00000000-0000-0000-0000-000000000003', NULL, 'fixture-peer-b', 'Download', 'Remote/B-First.flac', 1000, 0, NULL, NULL, NULL, 1, NULL, 144, 'Completed, TimedOut', datetime('now','-38 seconds'), datetime('now','-38 seconds'), NULL, NULL, datetime('now','-38 seconds'), 0, 0, NULL, 'timed out', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 0);
SQL
  else
    "$python_bin" - "$state/transfer-state.json" <<'PY'
import json,sys,time
now=int(time.time())
entries=[]
for ident,peer,name,age in [
    (1,"fixture-peer-a","Remote/A-First.flac",40),
    (2,"fixture-peer-a","Remote/A-Second.flac",39),
    (3,"fixture-peer-b","Remote/B-First.flac",38),
]:
    entries.append({
        "id":ident,"direction":0,"token":ident,"peer_username":peer,
        "filename":name,"local_path":None,"batch_id":None,"request_id":None,
        "request_name":None,"destination_directory":None,"bit_rate":None,
        "sample_rate":None,"bit_depth":None,"length_seconds":None,"artist":None,
        "album":None,"title":None,"track_number":None,"year":None,"size":1000,
        "bytes_transferred":0,"status":"failed","reason":"timed out",
        "requested_at":now-age,"started_at":None,"start_offset":0,
        "updated_at":now-age,"updated_at_ms":0,
    })
with open(sys.argv[1],"w",encoding="utf-8") as handle:
    json.dump({"version":1,"entries":entries},handle,separators=(",",":"))
PY
  fi
}

wait_for_download_auto_retry_options() {
  local base_url="$1"
  local enabled="$2"
  local tolerance="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["global"]["download"]["autoRetry"]; expected={"enabled":sys.argv[1]=="true","retryDelaySeconds":10,"checkIntervalSeconds":10,"maxAttempts":1,"maxFilesPerCycle":2,"maxFilesPerPeerPerCycle":1,"peerCooldownSeconds":60,"alternateSourcesEnabled":False,"maxAlternateSourceSearchesPerCycle":0,"alternateSourceSizeTolerancePercent":float(sys.argv[2])}; raise SystemExit(0 if value==expected else 1)' \
        "$enabled" "$tolerance" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for watched options\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_download_auto_retry_defaults() {
  local base_url="$1"
  local log="$2"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["global"]["download"]["autoRetry"]; expected={"enabled":True,"retryDelaySeconds":1800,"checkIntervalSeconds":300,"maxAttempts":5,"maxFilesPerCycle":10,"maxFilesPerPeerPerCycle":1,"peerCooldownSeconds":900,"alternateSourcesEnabled":True,"maxAlternateSourceSearchesPerCycle":1,"alternateSourceSizeTolerancePercent":5}; raise SystemExit(0 if value==expected else 1)' 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for null/default binding\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for null/default binding\n' >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_download_auto_retry_count() {
  local implementation="$1"
  local state="$2"
  local expected="$3"
  local log="$4"
  for _ in $(seq 1 300); do
    local count=''
    if [[ "$implementation" == upstream ]]; then
      count="$(sqlite3 "$state/data/transfers.db" 'SELECT COUNT(*) FROM Transfers;' 2>/dev/null || true)"
    else
      count="$($python_bin - "$state/transfer-state.json" 2>/dev/null <<'PY' || true
import json,sys
print(len(json.load(open(sys.argv[1],encoding="utf-8"))["entries"]))
PY
)"
    fi
    [[ "$count" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for %s transfers\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for %s transfers\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_download_auto_retry_requests() {
  local fixture_status="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 300); do
    if "$python_bin" - "$fixture_status" "$expected" 2>/dev/null <<'PY'
import json,sys
fixture=json.load(open(sys.argv[1],encoding="utf-8"))
requests=fixture.get("peer_address_requests",[])
expected=int(sys.argv[2])
expected_peers=["fixture-peer-a","fixture-peer-b"]
raise SystemExit(0 if len(requests)==expected and sorted(requests)==expected_peers else 1)
PY
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'download auto-retry daemon exited while waiting for %s peer requests\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'download auto-retry timed out waiting for %s peer requests\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_download_auto_retry_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local current="$suite/auto-retry-$stage-current.raw"
  local startup="$suite/auto-retry-$stage-startup.raw"
  local application="$suite/auto-retry-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application"
  "$python_bin" - "$current" "$startup" "$application" >"$suite/auto-retry-$stage.body" <<'PY'
import json,sys
current=json.load(open(sys.argv[1],encoding="utf-8"))["global"]["download"]["autoRetry"]
startup=json.load(open(sys.argv[2],encoding="utf-8"))["global"]["download"]["autoRetry"]
application=json.load(open(sys.argv[3],encoding="utf-8"))
print(json.dumps({"current":current,"startup":startup,"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/auto-retry-$stage.meta"
  rm -f "$current" "$startup" "$application"
}

capture_download_auto_retry_behavior() {
  local fixture_status="$1"
  local suite="$2"
  "$python_bin" - "$fixture_status" >"$suite/auto-retry-behavior.body" <<'PY'
import json,sys
fixture=json.load(open(sys.argv[1],encoding="utf-8"))
all_requests=fixture.get("peer_address_requests",[])
requests=sorted(set(all_requests))
print(json.dumps({
    "initialFailures":3,
    "retryCount":len(all_requests),
    "boundedToTwo":len(all_requests)==2,
    "contactedPeers":requests,
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/auto-retry-behavior.meta"
}

run_download_auto_retry_scenario() {
  local root="$1"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local peer_regular_port="$(pick_free_port)"
    local peer_obfuscated_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-auto-retry-$implementation"
    local suite="$work_dir/slskdn-auto-retry-$implementation"
    local log="$work_dir/slskdn-auto-retry-$implementation.log"
    local fixture_status="$work_dir/slskdn-auto-retry-$implementation-fixture.json"
    local fixture_log="$work_dir/slskdn-auto-retry-$implementation-fixture.log"
    mkdir -p "$state" "$suite"

    write_download_auto_retry_yaml "$state/slskd.yml" true "$server_port" "$listen_port" \
      false 10 10 1 2 1 60 false 0 5.5
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-auto-retry-$implementation-initialize.json" "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" startup-disabled
    stop_daemon
    seed_download_auto_retry_failures "$implementation" "$state"

    start_soulseek_peer_fixture "$server_port" "$fixture_status" "$fixture_log" \
      "$peer_regular_port" "$peer_obfuscated_port" regular-only
    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      false 10 10 1 2 1 60 false 0 5.5
    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/slskdn-auto-retry-$implementation-lifecycle.json" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    sleep 1
    wait_for_download_auto_retry_count "$implementation" "$state" 3 "$log"

    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      true 10 10 1 2 1 60 false 0 5.5
    wait_for_download_auto_retry_options "$base_url" true 5.5 "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" watched-enabled
    wait_for_download_auto_retry_requests "$fixture_status" 2 "$log"
    sleep 1
    wait_for_download_auto_retry_requests "$fixture_status" 2 "$log"
    sleep 11
    wait_for_download_auto_retry_requests "$fixture_status" 2 "$log"
    capture_download_auto_retry_behavior "$fixture_status" "$suite"

    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      null null null null null null null null null null
    wait_for_download_auto_retry_defaults "$base_url" "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" watched-null-defaults

    for validation in \
      'auto-retry-validation-valid|transfers:\n  download:\n    auto_retry:\n      enabled: true\n      retry_delay_seconds: 10\n      check_interval_seconds: 10\n      max_attempts: 0\n      max_files_per_cycle: 1\n      max_files_per_peer_per_cycle: 1\n      peer_cooldown_seconds: 60\n      alternate_sources_enabled: false\n      max_alternate_source_searches_per_cycle: 0\n      alternate_source_size_tolerance_percent: 5.5\n' \
      'auto-retry-validation-strings|transfers:\n  download:\n    auto_retry:\n      enabled: "true"\n      retry_delay_seconds: "10"\n      alternate_source_size_tolerance_percent: "5.5"\n' \
      'auto-retry-validation-null-defaults|transfers:\n  download:\n    auto_retry:\n      enabled: null\n      max_attempts: null\n      max_alternate_source_searches_per_cycle: null\n      alternate_source_size_tolerance_percent: null\n' \
      'auto-retry-validation-null-invalid|transfers:\n  download:\n    auto_retry:\n      retry_delay_seconds: null\n' \
      'auto-retry-validation-object-null|transfers:\n  download:\n    auto_retry: null\n' \
      'auto-retry-validation-object-scalar|transfers:\n  download:\n    auto_retry: true\n' \
      'auto-retry-validation-delay-low|transfers:\n  download:\n    auto_retry:\n      retry_delay_seconds: 9\n' \
      'auto-retry-validation-interval-high|transfers:\n  download:\n    auto_retry:\n      check_interval_seconds: 3601\n' \
      'auto-retry-validation-attempts-high|transfers:\n  download:\n    auto_retry:\n      max_attempts: 101\n' \
      'auto-retry-validation-files-zero|transfers:\n  download:\n    auto_retry:\n      max_files_per_cycle: 0\n' \
      'auto-retry-validation-peer-files-high|transfers:\n  download:\n    auto_retry:\n      max_files_per_peer_per_cycle: 21\n' \
      'auto-retry-validation-cooldown-low|transfers:\n  download:\n    auto_retry:\n      peer_cooldown_seconds: 59\n' \
      'auto-retry-validation-searches-high|transfers:\n  download:\n    auto_retry:\n      max_alternate_source_searches_per_cycle: 11\n' \
      'auto-retry-validation-enabled-invalid|transfers:\n  download:\n    auto_retry:\n      enabled: nope\n' \
      'auto-retry-validation-tolerance-frozen-low|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: -0.5\n' \
      'auto-retry-validation-tolerance-frozen-high|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: 100.5\n' \
      'auto-retry-validation-tolerance-low|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: -0.5001\n' \
      'auto-retry-validation-tolerance-high|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: 100.5001\n' \
      'auto-retry-validation-tolerance-text|transfers:\n  download:\n    auto_retry:\n      alternate_source_size_tolerance_percent: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done

    write_download_auto_retry_yaml "$state/slskd.yml" false "$server_port" "$listen_port" \
      false 10 10 1 2 1 60 false 0 5.5
    wait_for_download_auto_retry_options "$base_url" false 5.5 "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" watched-disabled
    stop_daemon
    stop_soulseek_fixture

    start_no_connect_daemon slskdn "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true
    wait_for_options "$base_url" "$work_dir/slskdn-auto-retry-$implementation-restarted.json" "$log"
    wait_for_download_auto_retry_options "$base_url" false 5.5 "$log"
    capture_download_auto_retry_stage "$base_url" "$suite" restarted-disabled
    stop_daemon
  done

  local upstream_normalized="$work_dir/slskdn-auto-retry-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-auto-retry-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-retry-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-auto-retry-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'slskdn download auto-retry differential failed\n' >&2
    exit 1
  fi
  printf 'slskdn download auto-retry differential passed\n'
}

write_blacklist_yaml() {
  local path="$1"
  local server_port="$2"
  local listen_port="$3"
  local share_dir="$4"
  local enabled="$5"
  local blacklist_file="$6"
  local no_connect="${7:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: %s\nblacklist:\n  enabled: %s\n  file: "%s"\nshares:\n  directories:\n    - "%s"\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$no_connect" "$enabled" "$blacklist_file" "$share_dir" "$server_port" "$listen_port" \
    >"$temporary"
  mv "$temporary" "$path"
}

start_blacklist_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local http_port="$6"
  local https_port="$7"
  local environment_enabled="${8:-}"
  local environment_file="${9:-}"
  local cli_enabled="${10:-false}"
  local cli_file="${11:-}"
  local append="${12:-false}"
  local cli_args=()
  [[ "$cli_enabled" == true ]] && cli_args+=(--enable-blacklist)
  [[ -n "$cli_file" ]] && cli_args+=(--blacklist-file "$cli_file")
  local redirect='>'
  [[ "$append" == true ]] && redirect='>>'
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$redirect" == '>>' ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
        [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
        export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
        [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
        exec dotnet "$dll" "${cli_args[@]}"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$redirect" == '>>' ]]; then
    (
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
      [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
      slskr_exec serve "${cli_args[@]}"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_BLACKLIST="$environment_enabled"
      [[ -n "$environment_file" ]] && export SLSKD_BLACKLIST_FILE="$environment_file"
      slskr_exec serve "${cli_args[@]}"
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_for_blacklist_option() {
  local base_url="$1"
  local enabled="$2"
  local file="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["blacklist"]; raise SystemExit(0 if value.get("enabled") == (sys.argv[1] == "true") and value.get("file", "") == sys.argv[2] else 1)' "$enabled" "$file" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'blacklist differential failed: daemon exited while waiting for options\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'blacklist differential failed: timed out waiting for enabled=%s file=%s\n' "$enabled" "$file" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_blacklist_listener() {
  local listen_port="$1"
  local log="$2"
  local consecutive=0
  for _ in $(seq 1 600); do
    if ss -H -ltn "sport = :$listen_port" | rg -q '^LISTEN'; then
      consecutive=$((consecutive + 1))
      [[ "$consecutive" -ge 10 ]] && return
    else
      consecutive=0
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'blacklist differential failed: daemon exited while waiting for peer listener\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'blacklist differential failed: peer listener did not stabilize on 127.0.0.1:%s\n' "$listen_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_blacklist_peer_suite() {
  local suite="$1"
  local label="$2"
  local listen_port="$3"
  local expected_blacklisted="$4"
  mkdir -p "$suite"
  local captured=false
  for _ in $(seq 1 300); do
    if "$python_bin" - "$listen_port" "$expected_blacklisted" \
      >"$suite/$label.body.tmp" 2>"$suite/$label.error" <<'PY'
import json
import socket
import struct
import sys
import time
import zlib

port = int(sys.argv[1])
expected = sys.argv[2] == "true"
probe_username = f"blacklist-probe-{time.time_ns()}"

def string(value):
    encoded = value.encode("utf-8")
    return struct.pack("<I", len(encoded)) + encoded

def receive_exact(connection, length):
    value = b""
    while len(value) < length:
        chunk = connection.recv(length - len(value))
        if not chunk:
            raise EOFError
        value += chunk
    return value

def request(code, payload=b"", timeout=1.5):
    connection = socket.create_connection(("127.0.0.1", port), timeout=2)
    connection.settimeout(timeout)
    initialization = string(probe_username) + string("P") + struct.pack("<I", 0)
    connection.sendall(struct.pack("<I", len(initialization) + 1) + b"\x01" + initialization)
    connection.sendall(struct.pack("<II", len(payload) + 4, code) + payload)
    try:
        length = struct.unpack("<I", receive_exact(connection, 4))[0]
        frame = receive_exact(connection, length)
        return struct.unpack("<I", frame[:4])[0], frame[4:]
    except (socket.timeout, EOFError):
        return None
    finally:
        connection.close()

def read_string(payload, offset):
    length = struct.unpack_from("<I", payload, offset)[0]
    offset += 4
    value = payload[offset:offset + length].decode("utf-8")
    return value, offset + length

user_info = request(15)
restricted_user_info = False
if user_info and user_info[0] == 16:
    payload = user_info[1]
    _, offset = read_string(payload, 0)
    has_picture = payload[offset] != 0
    offset += 1
    if has_picture:
        picture_length = struct.unpack_from("<I", payload, offset)[0]
        offset += 4 + picture_length
    _, queue_size = struct.unpack_from("<II", payload, offset)
    slots_free = payload[offset + 8] != 0
    restricted_user_info = queue_size == 2_147_483_647 and not slots_free

browse = request(4)
browse_empty = bool(
    browse and browse[0] == 5 and zlib.decompress(browse[1]) == bytes(12)
)

search = request(8, struct.pack("<I", 123) + string("fixture")) if expected else None
search_suppressed = expected and search is None

folder = request(36, struct.pack("<I", 124) + string("share"))
folder_empty = False
if folder and folder[0] == 37:
    try:
        payload = zlib.decompress(folder[1])
        token = struct.unpack_from("<I", payload, 0)[0]
        outer, offset = read_string(payload, 4)
        directory_count = struct.unpack_from("<I", payload, offset)[0]
        offset += 4
        inner, offset = read_string(payload, offset)
        file_count = struct.unpack_from("<I", payload, offset)[0]
        folder_empty = token == 124 and outer == "share" and directory_count == 1 and inner == "share" and file_count == 0
    except (IndexError, UnicodeDecodeError, struct.error, zlib.error):
        folder_empty = False

transfer = request(40, struct.pack("<II", 0, 125) + string("share\\fixture.txt"))
file_not_shared = False
if transfer and transfer[0] == 41:
    payload = transfer[1]
    allowed = payload[4] != 0
    if not allowed:
        reason, _ = read_string(payload, 5)
        file_not_shared = reason == "File not shared."

result = {
    "browseEmpty": browse_empty,
    "fileNotShared": file_not_shared,
    "folderEmpty": folder_empty,
    "restrictedUserInfo": restricted_user_info,
    "searchSuppressed": search_suppressed,
}
peer_checks = {key: value for key, value in result.items() if key != "searchSuppressed"}
if any(value != expected for value in peer_checks.values()) or (expected and not search_suppressed):
    raise SystemExit(f"blacklist peer behavior mismatch: expected={expected} actual={result}")
print(json.dumps(result, sort_keys=True, separators=(",", ":")))
PY
    then
      mv "$suite/$label.body.tmp" "$suite/$label.body"
      captured=true
      break
    fi
    sleep 0.1
  done
  if [[ "$captured" != true ]]; then
    printf '%s: ' "$label" >&2
    cat "$suite/$label.error" >&2
    rm -f "$suite/$label.body.tmp" "$suite/$label.error"
    return 1
  fi
  rm -f "$suite/$label.error"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/$label.meta"
}

capture_blacklist_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "blacklist-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "blacklist-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "blacklist-application-$stage" "$base_url/api/v0/application"
}

run_blacklist_scenario() {
  local target="$1"
  local root="$2"
  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local server_port="$(pick_free_port)"
    local listen_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-$target-blacklist-$implementation"
    local suite="$work_dir/$target-blacklist-$implementation"
    local log="$work_dir/$target-blacklist-$implementation.log"
    local fixture_status="$work_dir/$target-blacklist-$implementation-fixture.json"
    local fixture_log="$work_dir/$target-blacklist-$implementation-fixture.log"
    local share_dir="$state/share"
    local cidr_file="$state/blacklist-cidr.txt"
    local p2p_file="$state/blacklist-p2p.txt"
    local dat_file="$state/blacklist-dat.txt"
    mkdir -p "$state" "$suite" "$share_dir"
    printf 'fixture-data' >"$share_dir/fixture.txt"
    printf '127.0.0.1/32\n' >"$cidr_file"
    printf 'loopback:127.0.0.1-127.0.0.1\n' >"$p2p_file"
    printf '127.000.000.001 - 127.000.000.001 , 000 , loopback\n' >"$dat_file"

    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success
    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" false "$cidr_file"
    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-startup.json" "$log"
    wait_for_blacklist_option "$base_url" false "$cidr_file" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    wait_for_share_files "$base_url" share 1 "$log"
    capture_blacklist_stage "$base_url" "$suite" startup-disabled
    capture_blacklist_peer_suite "$suite" peer-startup-disabled "$listen_port" false

    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" true "$cidr_file"
    wait_for_blacklist_option "$base_url" true "$cidr_file" "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_stage "$base_url" "$suite" watched-cidr
    capture_blacklist_peer_suite "$suite" peer-watched-cidr "$listen_port" true

    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" false "$p2p_file"
    wait_for_blacklist_option "$base_url" false "$p2p_file" "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_peer_suite "$suite" peer-watched-disabled "$listen_port" false
    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" true "$p2p_file"
    wait_for_blacklist_option "$base_url" true "$p2p_file" "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_stage "$base_url" "$suite" watched-p2p
    capture_blacklist_peer_suite "$suite" peer-watched-p2p "$listen_port" true

    for validation in \
      "blacklist-validation-valid-cidr|blacklist:\n  enabled: true\n  file: '$cidr_file'\n" \
      "blacklist-validation-valid-p2p|blacklist:\n  enabled: true\n  file: '$p2p_file'\n" \
      "blacklist-validation-valid-dat|blacklist:\n  enabled: true\n  file: '$dat_file'\n" \
      "blacklist-validation-disabled|blacklist:\n  enabled: false\n" \
      "blacklist-validation-enabled-null|blacklist:\n  enabled: null\n" \
      "blacklist-validation-file-null|blacklist:\n  enabled: true\n  file: null\n" \
      "blacklist-validation-enabled-text|blacklist:\n  enabled: nope\n  file: '$cidr_file'\n" \
      "blacklist-validation-parent-null|blacklist: null\n" \
      "blacklist-validation-parent-array|blacklist: []\n" \
      "blacklist-validation-missing|blacklist:\n  enabled: true\n  file: '$state/missing.txt'\n"
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon
    stop_soulseek_fixture

    rm -f "$fixture_status"
    start_soulseek_fixture "$server_port" "$fixture_status" "$fixture_log" login-success
    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" false "" true
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-restart.json" "$log"
    wait_for_blacklist_option "$base_url" true "$p2p_file" "$log"
    wait_for_fixture_active "$fixture_status" 1 "$log"
    wait_for_blacklist_listener "$listen_port" "$log"
    capture_blacklist_stage "$base_url" "$suite" restarted
    capture_blacklist_peer_suite "$suite" peer-restarted "$listen_port" true
    stop_daemon
    stop_soulseek_fixture

    write_blacklist_yaml "$state/slskd.yml" "$server_port" "$listen_port" "$share_dir" false "$cidr_file" true
    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true "$p2p_file"
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-precedence-yaml.json" "$log"
    wait_for_blacklist_option "$base_url" false "$cidr_file" "$log"
    capture_blacklist_stage "$base_url" "$suite" yaml-over-environment
    stop_daemon

    start_blacklist_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false "$cidr_file" true "$p2p_file" true
    wait_for_options "$base_url" "$work_dir/$target-blacklist-$implementation-precedence-cli.json" "$log"
    wait_for_blacklist_option "$base_url" true "$p2p_file" "$log"
    capture_blacklist_stage "$base_url" "$suite" cli-over-yaml
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-blacklist-upstream.normalized"
  local slskr_normalized="$work_dir/$target-blacklist-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-blacklist-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-blacklist-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'blacklist differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s blacklist differential passed\n' "$target"
}

write_transfer_groups_yaml() {
  local path="$1"
  local upload_slots="$2"
  local default_priority="$3"
  local friend_priority="$4"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ntransfers:\n  upload:\n    slots: %s\n    speed_limit: 1200\n    limits:\n      queued:\n        files: 50\n        megabytes: 500\n      daily: ~\n      weekly:\n        failures: 8\n  groups:\n    default:\n      upload:\n        priority: %s\n        strategy: firstinfirstout\n        slots: 9\n        limits:\n          queued:\n            files: 25\n    leechers:\n      thresholds:\n        files: 4\n        directories: 2\n      upload:\n        priority: 90\n        strategy: roundrobin\n        slots: 1\n        speed_limit: 100\n    blacklisted:\n      members: [blocked-user]\n      patterns: ["^evil-"]\n      cidrs: [192.0.2.0/24]\n    user_defined:\n      friends:\n        upload:\n          priority: %s\n          strategy: firstinfirstout\n          slots: 7\n          limits:\n            queued:\n              files: 100\n        members: [friend, ally]\n' \
    "$upload_slots" "$default_priority" "$friend_priority" >"$temporary"
  mv "$temporary" "$path"
}

capture_transfer_groups_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local current="$work_dir/$target-transfer-groups-$stage-current-$$.json"
  local startup="$work_dir/$target-transfer-groups-$stage-startup-$$.json"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$current"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options/startup" >"$startup"
  "$python_bin" - "$target" "$current" "$startup" >"$suite/groups-$stage.body" <<'PY'
import json,sys
target,current_path,startup_path=sys.argv[1:]
current=json.load(open(current_path,encoding="utf-8"))
startup=json.load(open(startup_path,encoding="utf-8"))
def project(value):
    if target == "slskdn":
        return {"upload":value["global"]["upload"],"limits":value["global"]["limits"],"groups":value["groups"]}
    return {"upload":value["transfers"]["upload"],"groups":value["transfers"]["groups"]}
print(json.dumps({"current":project(current),"startup":project(startup)},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/groups-$stage.meta"
  rm -f "$current" "$startup"
  capture_get "$suite" "group-member-$stage" "$base_url/api/v0/users/friend/group"
  capture_get "$suite" "group-blacklisted-$stage" "$base_url/api/v0/users/blocked-user/group"
  capture_get "$suite" "groups-application-$stage" "$base_url/api/v0/application"
}

run_transfer_groups_scenario() {
  local target="$1"
  local root="$2"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-transfer-groups-$implementation"
    local suite="$work_dir/$target-transfer-groups-$implementation"
    local log="$work_dir/$target-transfer-groups-$implementation.log"
    mkdir -p "$state" "$suite"
    write_transfer_groups_yaml "$state/slskd.yml" 20 20 5
    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")" SLSKD_NO_AUTH=true
        slskr_exec serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port"
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-transfer-groups-$implementation-options.json" "$log"
    capture_transfer_groups_stage "$target" "$base_url" "$suite" startup

    for validation in \
      $'valid|transfers:\n  upload:\n    slots: 10\n    speed_limit: 100\n  groups:\n    default:\n      upload:\n        priority: 1\n        strategy: roundrobin\n        slots: 2\n' \
      $'daily-null|transfers:\n  upload:\n    limits:\n      daily: null\n' \
      $'invalid-upload-slots|transfers:\n  upload:\n    slots: 0\n' \
      $'invalid-priority|transfers:\n  groups:\n    default:\n      upload:\n        priority: 0\n' \
      $'invalid-strategy|transfers:\n  groups:\n    default:\n      upload:\n        strategy: invalid\n' \
      $'invalid-limit|transfers:\n  groups:\n    default:\n      upload:\n        limits:\n          queued:\n            files: 0\n' \
      $'duplicate-membership|transfers:\n  groups:\n    user_defined:\n      first:\n        members: [same-user]\n      second:\n        members: [same-user]\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "groups-validation-$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$("$python_bin" -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml")"
    done

    write_transfer_groups_yaml "$state/slskd.yml" 21 30 4
    local watched=false
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
        | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); target=sys.argv[1]; slots=value["global"]["upload"]["slots"] if target == "slskdn" else value["transfers"]["upload"]["slots"]; raise SystemExit(0 if slots == 21 else 1)' "$target" 2>/dev/null
      then
        watched=true
        break
      fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then
        printf 'transfer groups differential failed: daemon exited during watched reload\n' >&2
        tail -120 "$log" >&2 || true
        exit 1
      fi
      sleep 0.1
    done
    if [[ "$watched" != true ]]; then
      printf 'transfer groups differential failed: watched options did not update\n' >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    capture_transfer_groups_stage "$target" "$base_url" "$suite" watched
    stop_daemon

    if [[ "$implementation" == upstream ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")" SLSKD_NO_AUTH=true
        slskr_exec serve --app-dir "$state" \
          --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port"
      ) >>"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/$target-transfer-groups-$implementation-restarted-options.json" "$log"
    capture_transfer_groups_stage "$target" "$base_url" "$suite" restarted
    stop_daemon
  done

  normalize_directory_suite "$work_dir/$target-transfer-groups-upstream" "$work_dir/$target-transfer-groups-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-transfer-groups-slskr" "$work_dir/$target-transfer-groups-slskr.normalized"
  if ! diff -ru "$work_dir/$target-transfer-groups-upstream.normalized" "$work_dir/$target-transfer-groups-slskr.normalized"; then
    printf 'transfer groups differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s transfer groups differential passed\n' "$target"
}

write_transfer_download_yaml() {
  local path="$1" target="$2" slots="$3" speed="$4" variant="$5"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\ntransfers:\n  download:\n    slots: %s\n    speed_limit: %s\n' "$slots" "$speed" >"$temporary"
  if [[ "$target" == slskd ]]; then
    printf '    retry:\n      partial: %s\n      attempts: %s\n      delay: %s\n      max_delay: %s\n    destination:\n      subdirectory: "Music/${SOURCE_USERNAME}"\n      exists: overwrite\n      permissions:\n        mode: "0750"\n' \
      "$( [[ "$variant" == startup ]] && printf overwrite || printf resume )" \
      "$( [[ "$variant" == startup ]] && printf 4 || printf 5 )" \
      "$( [[ "$variant" == startup ]] && printf 1200 || printf 1300 )" \
      "$( [[ "$variant" == startup ]] && printf 31000 || printf 32000 )" >>"$temporary"
  else
    printf '    retry:\n      incomplete: %s\n      attempts: %s\n      delay: %s\n      max_delay: %s\n    completed_layout: %s\n    auto_replace_stuck: %s\n    auto_replace_threshold: %s\n    auto_replace_interval: %s\n' \
      "$( [[ "$variant" == startup ]] && printf overwrite || printf resume )" \
      "$( [[ "$variant" == startup ]] && printf 4 || printf 5 )" \
      "$( [[ "$variant" == startup ]] && printf 1200 || printf 1300 )" \
      "$( [[ "$variant" == startup ]] && printf 31000 || printf 32000 )" \
      "$( [[ "$variant" == startup ]] && printf uploader_folder || printf batch_id )" \
      "$( [[ "$variant" == startup ]] && printf true || printf false )" \
      "$( [[ "$variant" == startup ]] && printf 7.5 || printf 8.5 )" \
      "$( [[ "$variant" == startup ]] && printf 90 || printf 100 )" >>"$temporary"
  fi
  mv "$temporary" "$path"
}

start_transfer_download_daemon() {
  local target="$1" root="$2" implementation="$3" state="$4" log="$5"
  local http_port="$6" https_port="$7" listen_port="$8"
  if [[ "$implementation" == upstream ]]; then
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_LOGO=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_ADDRESS=127.0.0.1
      export SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
      exec dotnet "$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    ) >"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")" SLSKD_NO_AUTH=true
      slskr_exec serve --app-dir "$state" \
        --http-ip-address 127.0.0.1 --http-port "$http_port" --slsk-listen-port "$listen_port" --no-logo
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

capture_transfer_download_stage() {
  local target="$1" base_url="$2" suite="$3" stage="$4"
  "$python_bin" - "$target" "$base_url" >"$suite/download-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
target=sys.argv[1]; url=urllib.parse.urlsplit(sys.argv[2])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200: raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
current=get("/api/v0/options"); startup=get("/api/v0/options/startup"); application=get("/api/v0/application")
def project(value):
    return value["global"]["download"] if target == "slskdn" else value["transfers"]["download"]
print(json.dumps({"current":project(current),"startup":project(startup),"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/download-$stage.meta"
}

run_transfer_download_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-transfer-download-$implementation"
    local suite="$work_dir/$target-transfer-download-$implementation"
    local log="$work_dir/$target-transfer-download-$implementation.log"
    mkdir -p "$state" "$suite"
    write_transfer_download_yaml "$state/slskd.yml" "$target" 4 777 startup
    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-transfer-download-$implementation-options.json" "$log"
    capture_transfer_download_stage "$target" "$base_url" "$suite" startup

    local common_valid=$'transfers:\n  download:\n    slots: 2\n    speed_limit: 3\n    retry:\n      attempts: 2\n      delay: 5000\n      max_delay: 60000\n'
    capture_request "$suite" download-validation-valid POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$common_valid")"
    capture_request "$suite" download-validation-slots-zero POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    slots: 0\n')"
    capture_request "$suite" download-validation-speed-zero POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    speed_limit: 0\n')"
    capture_request "$suite" download-validation-attempts-zero POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    retry:\n      attempts: 0\n')"
    if [[ "$target" == slskd ]]; then
      capture_request "$suite" download-validation-strategy POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    retry:\n      partial: invalid\n')"
      capture_request "$suite" download-validation-mode POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    destination:\n      permissions:\n        mode: "999"\n')"
      capture_request "$suite" download-validation-traversal POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    destination:\n      subdirectory: ../escape\n')"
    else
      capture_request "$suite" download-validation-strategy POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    retry:\n      incomplete: invalid\n')"
      capture_request "$suite" download-validation-threshold POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    auto_replace_threshold: 0\n')"
      capture_request "$suite" download-validation-interval POST "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload $'transfers:\n  download:\n    auto_replace_interval: 9\n')"
    fi

    write_transfer_download_yaml "$state/slskd.yml" "$target" 5 778 watched
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); target=sys.argv[1]; value=value["global"]["download"] if target == "slskdn" else value["transfers"]["download"]; raise SystemExit(0 if value["speedLimit"] == 778 else 1)' "$target" 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/application" | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["pendingRestart"] else 1)' 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    capture_transfer_download_stage "$target" "$base_url" "$suite" watched
    stop_daemon

    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-transfer-download-$implementation-restarted-options.json" "$log"
    capture_transfer_download_stage "$target" "$base_url" "$suite" restarted
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-transfer-download-upstream" "$work_dir/$target-transfer-download-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-transfer-download-slskr" "$work_dir/$target-transfer-download-slskr.normalized"
  if ! diff -ru "$work_dir/$target-transfer-download-upstream.normalized" "$work_dir/$target-transfer-download-slskr.normalized"; then
    printf 'transfer download differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s transfer download differential passed\n' "$target"
}

write_soulseek_connection_yaml() {
  local path="$1" variant="$2"
  local read write transfer queue connect inactivity transfer_timeout proxy_port username password
  if [[ "$variant" == startup ]]; then
    read=2048; write=3072; transfer=81920; queue=5
    connect=1000; inactivity=2000; transfer_timeout=30000
    proxy_port=1080; username=proxy-user; password=proxy-secret
  else
    read=4096; write=5120; transfer=98304; queue=6
    connect=1100; inactivity=2100; transfer_timeout=31000
    proxy_port=1081; username=watched-user; password=watched-secret
  fi
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\nsoulseek:\n  connection:\n    buffer:\n      read: %s\n      write: %s\n      transfer: %s\n      write_queue: %s\n    timeout:\n      connect: %s\n      inactivity: %s\n      transfer: %s\n    proxy:\n      enabled: true\n      address: 127.0.0.1\n      port: %s\n      username: %s\n      password: %s\n' \
    "$read" "$write" "$transfer" "$queue" "$connect" "$inactivity" "$transfer_timeout" \
    "$proxy_port" "$username" "$password" >"$temporary"
  mv "$temporary" "$path"
}

capture_soulseek_connection_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/connection-$stage.body" <<'PY'
import http.client,json,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200: raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
current=get("/api/v0/options"); startup=get("/api/v0/options/startup"); application=get("/api/v0/application")
print(json.dumps({"current":current["soulseek"]["connection"],"startup":startup["soulseek"]["connection"],"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/connection-$stage.meta"
}

run_soulseek_connection_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-soulseek-connection-$implementation"
    local suite="$work_dir/$target-soulseek-connection-$implementation"
    local log="$work_dir/$target-soulseek-connection-$implementation.log"
    mkdir -p "$state" "$suite"
    write_soulseek_connection_yaml "$state/slskd.yml" startup
    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-connection-$implementation-options.json" "$log"
    capture_soulseek_connection_stage "$base_url" "$suite" startup

    for validation in \
      $'valid|soulseek:\n  connection:\n    buffer:\n      read: 1024\n      write: 1024\n      transfer: 81920\n      write_queue: 5\n    timeout:\n      connect: 1000\n      inactivity: 1000\n      transfer: 30000\n' \
      $'read-low|soulseek:\n  connection:\n    buffer:\n      read: 1023\n' \
      $'write-low|soulseek:\n  connection:\n    buffer:\n      write: 1023\n' \
      $'transfer-buffer-low|soulseek:\n  connection:\n    buffer:\n      transfer: 81919\n' \
      $'write-queue-low|soulseek:\n  connection:\n    buffer:\n      write_queue: 4\n' \
      $'connect-timeout-low|soulseek:\n  connection:\n    timeout:\n      connect: 999\n' \
      $'inactivity-timeout-low|soulseek:\n  connection:\n    timeout:\n      inactivity: 999\n' \
      $'transfer-timeout-low|soulseek:\n  connection:\n    timeout:\n      transfer: 29999\n' \
      $'proxy-missing-endpoint|soulseek:\n  connection:\n    proxy:\n      enabled: true\n' \
      $'proxy-valid|soulseek:\n  connection:\n    proxy:\n      enabled: true\n      address: 127.0.0.1\n      port: 1080\n      username: user\n      password: secret\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "connection-validation-$label" POST \
        "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$yaml")"
    done

    write_soulseek_connection_yaml "$state/slskd.yml" watched
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
        | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["soulseek"]["connection"]["buffer"]["read"] == 4096 else 1)' 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    capture_soulseek_connection_stage "$base_url" "$suite" watched
    stop_daemon

    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-connection-$implementation-restarted-options.json" "$log"
    capture_soulseek_connection_stage "$base_url" "$suite" restarted
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-soulseek-connection-upstream" "$work_dir/$target-soulseek-connection-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-soulseek-connection-slskr" "$work_dir/$target-soulseek-connection-slskr.normalized"
  if ! diff -ru "$work_dir/$target-soulseek-connection-upstream.normalized" "$work_dir/$target-soulseek-connection-slskr.normalized"; then
    printf 'Soulseek connection differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s Soulseek connection differential passed\n' "$target"
}

write_soulseek_profile_distributed_yaml() {
  local path="$1" picture="$2" variant="$3"
  local diagnostic disabled disable_children child_limit logging
  if [[ "$variant" == startup ]]; then
    diagnostic=trace; disabled=false; disable_children=false; child_limit=3; logging=true
  else
    diagnostic=debug; disabled=false; disable_children=true; child_limit=2; logging=false
  fi
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\nsoulseek:\n  picture: "%s"\n  diagnostic_level: %s\n  distributed_network:\n    disabled: %s\n    disable_children: %s\n    child_limit: %s\n    logging: %s\n' \
    "$picture" "$diagnostic" "$disabled" "$disable_children" "$child_limit" "$logging" >"$temporary"
  mv "$temporary" "$path"
}

capture_soulseek_profile_distributed_stage() {
  local base_url="$1" suite="$2" stage="$3"
  "$python_bin" - "$base_url" >"$suite/profile-distributed-$stage.body" <<'PY'
import http.client,json,os,sys,urllib.parse
url=urllib.parse.urlsplit(sys.argv[1])
def get(path):
    connection=http.client.HTTPConnection(url.hostname,url.port,timeout=5)
    connection.request("GET",path)
    response=connection.getresponse(); body=response.read(); connection.close()
    if response.status != 200: raise SystemExit(f"GET {path} returned {response.status}: {body!r}")
    return json.loads(body)
current=get("/api/v0/options"); startup=get("/api/v0/options/startup"); application=get("/api/v0/application")
def project(value):
    soulseek=value["soulseek"]
    picture=soulseek.get("picture")
    return {"picture":os.path.basename(picture) if picture else picture,"diagnosticLevel":soulseek["diagnosticLevel"],"distributedNetwork":soulseek["distributedNetwork"]}
print(json.dumps({"current":project(current),"startup":project(startup),"application":application["distributedNetwork"],"pendingRestart":application["pendingRestart"]},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/profile-distributed-$stage.meta"
}

run_soulseek_profile_distributed_scenario() {
  local target="$1" root="$2"
  local http_port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-soulseek-profile-distributed-$implementation"
    local suite="$work_dir/$target-soulseek-profile-distributed-$implementation"
    local log="$work_dir/$target-soulseek-profile-distributed-$implementation.log"
    local picture_startup="$state/picture-startup.bin"
    local picture_watched="$state/picture-watched.bin"
    mkdir -p "$state" "$suite"
    printf '\x00\x01\x02\xff' >"$picture_startup"
    printf '\x09\x08\x07' >"$picture_watched"
    write_soulseek_profile_distributed_yaml "$state/slskd.yml" "$picture_startup" startup
    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-profile-distributed-$implementation-options.json" "$log"
    capture_soulseek_profile_distributed_stage "$base_url" "$suite" startup

    for validation in \
      $'diagnostic-trace|soulseek:\n  diagnostic_level: trace\n' \
      $'diagnostic-invalid|soulseek:\n  diagnostic_level: verbose\n' \
      $'picture-missing|soulseek:\n  picture: /tmp/slskr-picture-that-does-not-exist\n' \
      $'distributed-valid|soulseek:\n  distributed_network:\n    disabled: false\n    disable_children: false\n    child_limit: 1\n    logging: true\n' \
      $'distributed-zero|soulseek:\n  distributed_network:\n    child_limit: 0\n' \
      $'distributed-overflow|soulseek:\n  distributed_network:\n    child_limit: 2147483648\n' \
      $'distributed-bool|soulseek:\n  distributed_network:\n    disabled: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "profile-distributed-validation-$label" POST \
        "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$yaml")"
    done

    write_soulseek_profile_distributed_yaml "$state/slskd.yml" "$picture_watched" watched
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
        | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["soulseek"]; raise SystemExit(0 if value["diagnosticLevel"] == "debug" and value["distributedNetwork"]["childLimit"] == 2 else 1)' 2>/dev/null; then break; fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    # The reference watcher can publish the new current projection a tick
    # before it records the restart boundary. Wait for that state to settle so
    # the parity comparison does not sample an intermediate event ordering.
    local watched_restart_settled=false
    for _ in $(seq 1 600); do
      if curl --fail --silent --max-time 1 "$base_url/api/v0/application" \
        | "$python_bin" -c 'import json,sys; raise SystemExit(0 if json.load(sys.stdin)["pendingRestart"] else 1)' 2>/dev/null; then
        watched_restart_settled=true
        break
      fi
      if ! kill -0 "$daemon_pid" 2>/dev/null; then tail -120 "$log" >&2 || true; exit 1; fi
      sleep 0.1
    done
    if [[ "$watched_restart_settled" != true ]]; then
      printf 'Soulseek profile/distributed differential failed: watched restart state did not settle for %s\n' "$implementation" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    capture_soulseek_profile_distributed_stage "$base_url" "$suite" watched
    stop_daemon

    start_transfer_download_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-soulseek-profile-distributed-$implementation-restarted-options.json" "$log"
    capture_soulseek_profile_distributed_stage "$base_url" "$suite" restarted
    stop_daemon
  done
  normalize_directory_suite "$work_dir/$target-soulseek-profile-distributed-upstream" "$work_dir/$target-soulseek-profile-distributed-upstream.normalized"
  normalize_directory_suite "$work_dir/$target-soulseek-profile-distributed-slskr" "$work_dir/$target-soulseek-profile-distributed-slskr.normalized"
  if ! diff -ru "$work_dir/$target-soulseek-profile-distributed-upstream.normalized" "$work_dir/$target-soulseek-profile-distributed-slskr.normalized"; then
    printf 'Soulseek profile/distributed differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s Soulseek profile/distributed differential passed\n' "$target"
}

write_dht_yaml() {
  local path="$1"
  local enabled="$2"
  local dht_port="$3"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\nflags:\n  no_connect: true\nweb:\n  rate_limiting:\n    enabled: false\ndht:\n  enabled: %s\n  dht_port: %s\n' \
    "$enabled" "$dht_port" >"$temporary"
  mv "$temporary" "$path"
}

start_dht_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    (
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      exec dotnet "$dll"
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE=native
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      slskr_exec serve
    ) >>"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

wait_for_dht_options() {
  local base_url="$1"
  local expected_enabled="$2"
  local expected_port="$3"
  local log="$4"
  for _ in $(seq 1 600); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin)["dhtRendezvous"]; raise SystemExit(0 if value["enabled"] == (sys.argv[1] == "true") and value["dhtPort"] == int(sys.argv[2]) else 1)' \
        "$expected_enabled" "$expected_port" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'DHT differential failed: daemon exited while waiting for enabled=%s port=%s\n' \
        "$expected_enabled" "$expected_port" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'DHT differential failed: timed out waiting for enabled=%s port=%s\n' \
    "$expected_enabled" "$expected_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_udp_state() {
  local port="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local state
    state="$($python_bin - "$port" <<'PY'
import socket,sys
sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
try:
    sock.bind(("0.0.0.0", int(sys.argv[1])))
except OSError:
    print("bound")
else:
    print("free")
finally:
    sock.close()
PY
)"
    if [[ "$state" == "$expected" ]]; then
      return
    fi
    if [[ -n "$daemon_pid" ]] && ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'DHT differential failed: daemon exited while waiting for UDP %s to be %s\n' \
        "$port" "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'DHT differential failed: timed out waiting for UDP %s to be %s\n' \
    "$port" "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_dht_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "dht-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "dht-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "dht-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "dht-status-$stage" "$base_url/api/v0/dht/status"
}

run_dht_scenario() {
  local root="$1"
  local first_port="$(pick_free_udp_port)"
  local second_port="$(pick_free_udp_port)"
  while [[ "$second_port" == "$first_port" ]]; do
    second_port="$(pick_free_udp_port)"
  done

  for implementation in upstream slskr; do
    local http_port="$(pick_free_port)"
    local https_port="$(pick_free_port)"
    local base_url="http://127.0.0.1:$http_port"
    local state="$work_dir/state-slskdn-dht-$implementation"
    local suite="$work_dir/slskdn-dht-$implementation"
    local log="$work_dir/slskdn-dht-$implementation.log"
    mkdir -p "$state" "$suite"

    write_dht_yaml "$state/slskd.yml" true "$first_port"
    start_dht_daemon "$root" "$implementation" "$state" "$log" "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-dht-$implementation-enabled.json" "$log"
    wait_for_dht_options "$base_url" true "$first_port" "$log"
    wait_for_udp_state "$first_port" bound "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" enabled-startup

    write_dht_yaml "$state/slskd.yml" false "$second_port"
    wait_for_dht_options "$base_url" false "$second_port" "$log"
    wait_for_udp_state "$first_port" bound "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" disabled-watched

    for validation in \
      'dht-validation-enabled-zero|dht:\n  enabled: true\n  dht_port: 0\n' \
      'dht-validation-enabled-high|dht:\n  enabled: true\n  dht_port: 65536\n' \
      'dht-validation-disabled-zero|dht:\n  enabled: false\n  dht_port: 0\n' \
      'dht-validation-invalid-enabled|dht:\n  enabled: nope\n  dht_port: 50305\n' \
      'dht-validation-invalid-port|dht:\n  enabled: true\n  dht_port: nope\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "$label" POST "$base_url/api/v0/options/yaml/validate" \
        "$($python_bin -c 'import json,sys; print(json.dumps(bytes(sys.argv[1], "utf-8").decode("unicode_escape")))' "$yaml")"
    done
    stop_daemon
    wait_for_udp_state "$first_port" free "$log"

    start_dht_daemon "$root" "$implementation" "$state" "$log" "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-dht-$implementation-disabled.json" "$log"
    wait_for_dht_options "$base_url" false "$second_port" "$log"
    wait_for_udp_state "$first_port" free "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" disabled-restarted

    write_dht_yaml "$state/slskd.yml" true "$second_port"
    wait_for_dht_options "$base_url" true "$second_port" "$log"
    wait_for_udp_state "$second_port" free "$log"
    capture_dht_stage "$base_url" "$suite" enabled-watched
    stop_daemon

    start_dht_daemon "$root" "$implementation" "$state" "$log" "$http_port" "$https_port"
    wait_for_options "$base_url" "$work_dir/slskdn-dht-$implementation-reenabled.json" "$log"
    wait_for_dht_options "$base_url" true "$second_port" "$log"
    wait_for_udp_state "$second_port" bound "$log"
    capture_dht_stage "$base_url" "$suite" enabled-restarted
    stop_daemon
    wait_for_udp_state "$second_port" free "$log"
  done

  local upstream_normalized="$work_dir/slskdn-dht-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-dht-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-dht-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-dht-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'DHT differential failed for slskdN\n' >&2
    exit 1
  fi
  printf 'slskdn DHT differential passed\n'
}

write_listener_yaml() {
  local path="$1"
  local server_port="$2"
  local listen_ip="$3"
  local listen_port="$4"
  local obfuscation_enabled="${5:-false}"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndht:\n  enabled: false\nflags:\n  no_connect: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  description: listener differential\n  listen_ip_address: %s\n  listen_port: %s\n  obfuscation:\n    enabled: %s\n' \
    "$server_port" "$listen_ip" "$listen_port" "$obfuscation_enabled" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_listener_option() {
  local base_url="$1"
  local expected_ip="$2"
  local expected_port="$3"
  local log="$4"
  for _ in $(seq 1 400); do
    if curl --fail --silent --max-time 1 "$base_url/api/v0/options" \
      | "$python_bin" -c 'import json,sys; value=json.load(sys.stdin); raise SystemExit(0 if value["soulseek"]["listenIpAddress"] == sys.argv[1] and value["soulseek"]["listenPort"] == int(sys.argv[2]) else 1)' "$expected_ip" "$expected_port" 2>/dev/null
    then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'listener differential failed: daemon exited while waiting for %s:%s\n' "$expected_ip" "$expected_port" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'listener differential failed: timed out waiting for %s:%s\n' "$expected_ip" "$expected_port" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

direct_user_info_is_open() {
  local host="$1"
  local port="$2"
  "$python_bin" - "$host" "$port" <<'PY'
import socket,sys
try:
    connection=socket.create_connection((sys.argv[1], int(sys.argv[2])), timeout=0.5)
except OSError:
    raise SystemExit(1)
connection.close()
PY
}

wait_for_direct_user_info_state() {
  local host="$1"
  local port="$2"
  local expected="$3"
  local log="$4"
  for _ in $(seq 1 120); do
    local actual=closed
    if direct_user_info_is_open "$host" "$port"; then
      actual=open
    fi
    [[ "$actual" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'listener differential failed: daemon exited while waiting for %s:%s to be %s\n' "$host" "$port" "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.05
  done
  printf 'listener differential failed: %s:%s did not become %s\n' "$host" "$port" "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

wait_for_advertised_port() {
  local status="$1"
  local expected_port="$2"
  local minimum_count="$3"
  local log="$4"
  for _ in $(seq 1 300); do
    if "$python_bin" - "$status" "$expected_port" "$minimum_count" <<'PY'
import json,sys
try:
    value=json.load(open(sys.argv[1], encoding="utf-8"))
except (FileNotFoundError, json.JSONDecodeError):
    raise SystemExit(1)
ports=value.get("set_wait_ports", [])
raise SystemExit(0 if len(ports) >= int(sys.argv[3]) and ports[-1] == int(sys.argv[2]) else 1)
PY
    then
      return
    fi
    sleep 0.05
  done
  printf 'listener differential failed: server advertisement did not reach port %s at count %s\n' "$expected_port" "$minimum_count" >&2
  cat "$status" >&2 || true
  tail -120 "$log" >&2 || true
  exit 1
}

advertisement_count() {
  "$python_bin" -c 'import json,sys; print(len(json.load(open(sys.argv[1], encoding="utf-8")).get("set_wait_ports", [])))' "$1"
}

wait_for_advertisement_count() {
  local status="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 300); do
    local actual
    actual="$(advertisement_count "$status" 2>/dev/null || true)"
    if [[ "$actual" == "$expected" ]]; then
      sleep 0.2
      actual="$(advertisement_count "$status" 2>/dev/null || true)"
      [[ "$actual" == "$expected" ]] && return
    fi
    if [[ "$actual" =~ ^[0-9]+$ && "$actual" -gt "$expected" ]]; then
      break
    fi
    sleep 0.05
  done
  printf 'listener differential failed: expected %s SetListenPort messages\n' "$expected" >&2
  cat "$status" >&2 || true
  tail -120 "$log" >&2 || true
  exit 1
}

capture_listener_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  local expected_ip="$5"
  local expected_port="$6"
  local old_port="$7"
  local new_port="$8"
  local host_ip="$9"
  local fixture_status="${10}"
  local options_file="$suite/listener-$stage-options.raw"
  local application_file="$suite/listener-$stage-application.raw"
  mkdir -p "$suite"
  curl --fail --silent --max-time 2 "$base_url/api/v0/options" >"$options_file"
  curl --fail --silent --max-time 2 "$base_url/api/v0/application" >"$application_file"
  local local_old=false local_new=false host_new=false
  direct_user_info_is_open 127.0.0.1 "$old_port" && local_old=true
  direct_user_info_is_open 127.0.0.1 "$new_port" && local_new=true
  direct_user_info_is_open "$host_ip" "$new_port" && host_new=true
  local local_obfuscated=false host_obfuscated=false
  if [[ "$expected_port" -lt 65535 ]]; then
    direct_user_info_is_open 127.0.0.1 "$((expected_port + 1))" && local_obfuscated=true
    direct_user_info_is_open "$host_ip" "$((expected_port + 1))" && host_obfuscated=true
  fi
  "$python_bin" - "$options_file" "$application_file" "$fixture_status" \
    "$target" "$expected_ip" "$expected_port" "$local_old" "$local_new" "$host_new" \
    "$local_obfuscated" "$host_obfuscated" "$stage" \
    >"$suite/listener-$stage.body" <<'PY'
import json,sys
options=json.load(open(sys.argv[1], encoding="utf-8"))
application=json.load(open(sys.argv[2], encoding="utf-8"))
fixture=json.load(open(sys.argv[3], encoding="utf-8"))
target=sys.argv[4]
expected_ip=sys.argv[5]
expected_port=int(sys.argv[6])
ports=fixture.get("set_wait_ports", [])
messages=fixture.get("set_wait_port_messages", [])
message=messages[-1] if messages else {}
expects_obfuscation=target == "slskdn" and expected_port < 65535
expected_message_count={
    "startup": 2,
    "port-watched": 3,
    "port-restarted": 5,
    "ip-watched": 5,
    "ip-restarted": 7,
}[sys.argv[12]]
print(json.dumps({
    "optionsIpMatches": options["soulseek"]["listenIpAddress"] == expected_ip,
    "optionsPortMatches": options["soulseek"]["listenPort"] == expected_port,
    "pendingReconnect": application["pendingReconnect"],
    "pendingRestart": application["pendingRestart"],
    "serverState": application["server"]["state"],
    "localOldOpen": sys.argv[7] == "true",
    "localNewOpen": sys.argv[8] == "true",
    "hostNewOpen": sys.argv[9] == "true",
    "localObfuscatedOpen": sys.argv[10] == "true",
    "hostObfuscatedOpen": sys.argv[11] == "true",
    "advertisedPortMatches": bool(ports) and ports[-1] == expected_port,
    "advertisementCountMatches": len(messages) == expected_message_count,
    "advertisementMetadataMatches": bool(message) and (
        message.get("payload_length") == (12 if expects_obfuscation else 4)
        and message.get("obfuscation_type") == (1 if expects_obfuscation else None)
        and message.get("obfuscated_port") == (expected_port + 1 if expects_obfuscation else None)
    ),
}, sort_keys=True, separators=(",", ":")))
PY
  rm -f "$options_file" "$application_file"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/listener-$stage.meta"
}

listener_validation_payload() {
  local yaml="$1"
  "$python_bin" -c 'import json,sys; print(json.dumps(sys.argv[1]))' "$yaml"
}

write_obfuscation_yaml() {
  local path="$1"
  local listen_port="$2"
  local enabled="$3"
  local mode="$4"
  local obfuscated_port="$5"
  local advertise_regular_port="$6"
  local prefer_outbound="$7"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n  obfuscation:\n    enabled: %s\n    mode: %s\n    listen_port: %s\n    advertise_regular_port: %s\n    prefer_outbound: %s\n' \
    "$listen_port" "$enabled" "$mode" "$obfuscated_port" \
    "$advertise_regular_port" "$prefer_outbound" >"$temporary"
  mv "$temporary" "$path"
}

write_obfuscation_omitted_yaml() {
  local path="$1"
  local listen_port="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: true\nsoulseek:\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n' \
    "$listen_port" >"$temporary"
  mv "$temporary" "$path"
}

start_obfuscation_daemon() {
  local root="$1"
  local implementation="$2"
  local state="$3"
  local log="$4"
  local http_port="$5"
  local https_port="$6"
  local environment_enabled="${7:-}"
  local environment_mode="${8:-}"
  local environment_listen_port="${9:-}"
  local environment_advertise_regular="${10:-}"
  local environment_prefer_outbound="${11:-}"
  local cli_enabled="${12:-false}"
  local cli_mode="${13:-}"
  local cli_listen_port="${14:-}"
  local cli_advertise_regular="${15:-false}"
  local cli_prefer_outbound="${16:-false}"
  local append="${17:-false}"
  local cli_args=()
  [[ "$cli_enabled" == true ]] && cli_args+=(--slsk-obfuscation)
  [[ -n "$cli_mode" ]] && cli_args+=(--slsk-obfuscation-mode "$cli_mode")
  [[ -n "$cli_listen_port" ]] && cli_args+=(--slsk-obfuscation-listen-port "$cli_listen_port")
  [[ "$cli_advertise_regular" == true ]] && cli_args+=(--slsk-obfuscation-advertise-regular-port)
  [[ "$cli_prefer_outbound" == true ]] && cli_args+=(--slsk-obfuscation-prefer-outbound)
  run_obfuscation_process() {
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_OBFUSCATION="$environment_enabled"
      [[ -n "$environment_mode" ]] && export SLSKD_SLSK_OBFUSCATION_MODE="$environment_mode"
      [[ -n "$environment_listen_port" ]] && export SLSKD_SLSK_OBFUSCATION_LISTEN_PORT="$environment_listen_port"
      [[ -n "$environment_advertise_regular" ]] && export SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT="$environment_advertise_regular"
      [[ -n "$environment_prefer_outbound" ]] && export SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND="$environment_prefer_outbound"
      exec dotnet "$dll" "${cli_args[@]}"
    else
      export SLSKR_CONTROLLER_PROFILE=native
      export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$http_port" SLSKD_HTTPS_PORT="$https_port"
      [[ -n "$environment_enabled" ]] && export SLSKD_SLSK_OBFUSCATION="$environment_enabled"
      [[ -n "$environment_mode" ]] && export SLSKD_SLSK_OBFUSCATION_MODE="$environment_mode"
      [[ -n "$environment_listen_port" ]] && export SLSKD_SLSK_OBFUSCATION_LISTEN_PORT="$environment_listen_port"
      [[ -n "$environment_advertise_regular" ]] && export SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT="$environment_advertise_regular"
      [[ -n "$environment_prefer_outbound" ]] && export SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND="$environment_prefer_outbound"
      slskr_exec serve "${cli_args[@]}"
    fi
  }
  if [[ "$append" == true ]]; then
    run_obfuscation_process >>"$log" 2>&1 &
  else
    run_obfuscation_process >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

capture_obfuscation_options() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  capture_get "$suite" "obfuscation-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "obfuscation-startup-$stage" "$base_url/api/v0/options/startup"
}

capture_obfuscation_invalid_startup() {
  local root="$1" implementation="$2" state="$3" suite="$4" log="$5"
  local http_port="$6" https_port="$7" regular_port="$8" obfuscated_port="$9"
  write_obfuscation_yaml "$state/slskd.yml" "$regular_port" true compatibility \
    "$obfuscated_port" false true
  start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
    "$http_port" "$https_port"
  for _ in $(seq 1 300); do
    if ! kill -0 "$daemon_pid" 2>/dev/null; then break; fi
    sleep 0.05
  done
  if kill -0 "$daemon_pid" 2>/dev/null; then
    printf 'obfuscation invalid-startup differential: daemon did not exit\n' >&2
    stop_daemon
    exit 1
  fi
  local exit_code=0
  wait "$daemon_pid" || exit_code="$?"
  daemon_pid=""
  local matched=false
  if grep -q 'regular peer port must be advertised' "$log"; then matched=true; fi
  "$python_bin" - "$exit_code" "$matched" >"$suite/obfuscation-invalid-startup.body" <<'PY'
import json,sys
print(json.dumps({
    "exitCode":int(sys.argv[1]),
    "regularAdvertisementFailure":sys.argv[2] == "true",
},sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/obfuscation-invalid-startup.meta"
}

run_obfuscation_options_scenario() {
  local root="$1"
  local http_port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local regular_port="$(pick_free_port)"
  local yaml_obfuscated_port="$(pick_free_port)"
  local environment_obfuscated_port="$(pick_free_port)"
  local cli_obfuscated_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$http_port"

  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-obfuscation-$implementation"
    local suite="$work_dir/slskdn-obfuscation-$implementation"
    local log="$work_dir/slskdn-obfuscation-$implementation.log"
    mkdir -p "$state" "$suite"

    write_obfuscation_yaml "$state/slskd.yml" "$regular_port" false prefer \
      "$yaml_obfuscated_port" false false
    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true compatibility "$environment_obfuscated_port" true true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-yaml.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" yaml-over-environment

    for validation in \
      $'mode-invalid|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    mode: invalid\n' \
      $'mode-only|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    enabled: true\n    mode: only\n' \
      $'listen-low|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    listen_port: 1023\n' \
      $'listen-high|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    listen_port: 65536\n' \
      $'listen-same|flags:\n  no_connect: true\nsoulseek:\n  listen_port: 50300\n  obfuscation:\n    enabled: true\n    listen_port: 50300\n' \
      $'listen-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    listen_port: nope\n' \
      $'enabled-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    enabled: nope\n' \
      $'advertise-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    advertise_regular_port: nope\n' \
      $'prefer-text|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    prefer_outbound: nope\n' \
      $'advertise-disabled|flags:\n  no_connect: true\nsoulseek:\n  obfuscation:\n    enabled: true\n    advertise_regular_port: false\n'
    do
      local label="${validation%%|*}"
      local yaml="${validation#*|}"
      capture_request "$suite" "obfuscation-validation-$label" POST \
        "$base_url/api/v0/options/yaml/validate" "$(listener_validation_payload "$yaml")"
    done
    stop_daemon

    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" true compatibility "$environment_obfuscated_port" true true \
      true compatibility "$cli_obfuscated_port" true true true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-cli.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" command-line-over-yaml
    stop_daemon

    write_obfuscation_omitted_yaml "$state/slskd.yml" "$regular_port"
    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" false prefer "$environment_obfuscated_port" false false \
      false "" "" false false true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-environment.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" environment-with-yaml-omitted
    stop_daemon

    start_obfuscation_daemon "$root" "$implementation" "$state" "$log" \
      "$http_port" "$https_port" "" "" "" "" "" false "" "" false false true
    wait_for_options "$base_url" "$work_dir/slskdn-obfuscation-$implementation-defaults.json" "$log"
    capture_obfuscation_options "$base_url" "$suite" defaults
    stop_daemon

    capture_obfuscation_invalid_startup "$root" "$implementation" "$state" "$suite" "$log" \
      "$http_port" "$https_port" "$regular_port" "$yaml_obfuscated_port"
  done

  local upstream_normalized="$work_dir/slskdn-obfuscation-upstream.normalized"
  local slskr_normalized="$work_dir/slskdn-obfuscation-slskr.normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/slskdn-obfuscation-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'obfuscation options differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn obfuscation options differential passed\n'
}

write_obfuscation_runtime_yaml() {
  local path="$1"
  local server_port="$2"
  local regular_port="$3"
  local enabled="$4"
  local mode="$5"
  local obfuscated_port="$6"
  local advertise_regular_port="$7"
  local prefer_outbound="$8"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: true\nflags:\n  no_connect: false\ndht:\n  enabled: false\nsoulseek:\n  address: 127.0.0.1\n  port: %s\n  username: fixture-user\n  password: fixture-password\n  listen_ip_address: 0.0.0.0\n  listen_port: %s\n  obfuscation:\n    enabled: %s\n    mode: %s\n    listen_port: %s\n    advertise_regular_port: %s\n    prefer_outbound: %s\n' \
    "$server_port" "$regular_port" "$enabled" "$mode" "$obfuscated_port" \
    "$advertise_regular_port" "$prefer_outbound" >"$temporary"
  mv "$temporary" "$path"
}

