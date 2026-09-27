run_advanced_networking_security_scenario() {
  local root="$1"
  local port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local dht_port="$(pick_free_udp_port)" mesh_udp_port="$(pick_free_udp_port)" mesh_quic_port="$(pick_free_udp_port)"
  local base_url="http://127.0.0.1:$port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-advanced-networking-security-$implementation"
    local suite="$work_dir/slskdn-advanced-networking-security-$implementation"
    local log="$work_dir/slskdn-advanced-networking-security-$implementation.log"
    mkdir -p "$state" "$suite"
    write_advanced_networking_security_yaml "$state/slskd.yml" "$dht_port" "$mesh_udp_port" "$mesh_quic_port"
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_PROFILE=native
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        slskr_exec serve
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-advanced-networking-security-$implementation-options.json" "$log"
    capture_advanced_networking_security "$base_url" "$suite"
    stop_daemon
  done
  normalize_directory_suite "$work_dir/slskdn-advanced-networking-security-upstream" "$work_dir/slskdn-advanced-networking-security-upstream.normalized"
  normalize_directory_suite "$work_dir/slskdn-advanced-networking-security-slskr" "$work_dir/slskdn-advanced-networking-security-slskr.normalized"
  if ! diff -ru "$work_dir/slskdn-advanced-networking-security-upstream.normalized" "$work_dir/slskdn-advanced-networking-security-slskr.normalized"; then
    printf 'advanced networking/security differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn advanced networking/security differential passed\n'
}

write_media_advanced_service_yaml() {
  local path="$1"
  printf '%s' "flags:
  no_connect: true
feature:
  collectionsSharing: false
  streaming: false
  streamingRelayFallback: false
  meshParallelSearch: false
  meshPublishAvailability: false
  identityFriends: false
  solid: true
  scenePodBridge: true
  scenePodBridgeOptions: {proxyTransfers: true, exportPodAvailability: true}
  songId: true
  mesh: false
  dht: false
  pods: false
  socialFederation: false
  virtualSoulfind: true
  multiSourceDownloads: false
player:
  external_visualizer:
    enabled: true
    path: /bin/echo
    arguments: [visualizer, --fixture]
    working_directory: /tmp
    name: Fixture Visualizer
solid:
  allowInsecureHttp: true
  maxFetchBytes: 7654321
  timeoutSeconds: 23
  allowedHosts: [pod.example, identity.example]
  redirectPath: /fixture/callback
song_id:
  max_concurrent_runs: 7
virtualSoulfind:
  bridge:
    enabled: false
    port: 4322
    bindAddress: 127.0.0.2
    maxClients: 17
    requireAuth: true
    password: fixture-secret
    maxRequestsPerMinute: 71
    maxTransfersPerSession: 19
  disasterMode:
    auto: true
    force: true
    unavailableThresholdMinutes: 13
    enableGracefulDegradation: false
    recoveryCheckIntervalMinutes: 11
    recoveryHealthyChecksRequired: 5
web:
  authentication:
    disabled: true
" >"$path"
}

capture_media_advanced_service() {
  local base_url="$1" suite="$2"
  mkdir -p "$suite"
  "$python_bin" - "$base_url" >"$suite/media.body" <<'PY'
import json,os,sys,urllib.request
base=sys.argv[1]
def get(path):
    with urllib.request.urlopen(base + path, timeout=5) as response:
        return json.load(response)
value=get("/api/v0/options")
feature=value["feature"]
visualizer=get("/api/v0/player/external-visualizer")
result={
 "feature":{key:feature[key] for key in [
   "collectionsSharing","streaming","streamingRelayFallback","meshParallelSearch",
   "meshPublishAvailability","identityFriends","solid","scenePodBridge","songId",
   "mesh","dht","pods","socialFederation","virtualSoulfind","multiSourceDownloads"]},
 "scenePodBridgeOptions":feature["scenePodBridgeOptions"],
 "player":value["player"]["externalVisualizer"],
 "solid":value["solid"],
 "songId":value["songId"],
 "bridge":value["virtualSoulfind"]["bridge"],
 "disasterMode":value["virtualSoulfind"]["disasterMode"],
 "visualizerStatus":visualizer,
 "solidStatus":get("/api/v0/solid/status"),
 "bridgeAdmin":get("/api/v0/bridge/admin/config"),
 "disasterStatus":get("/api/v0/virtualsoulfind/disaster-mode/status"),
 "songQueue":get("/api/v0/songid/runs/queue"),
}
for key in ("path","resolvedPath"):
    if result["visualizerStatus"].get(key):
        result["visualizerStatus"][key]=os.path.basename(result["visualizerStatus"][key])
for key in ("path","workingDirectory"):
    if result["player"].get(key):
        result["player"][key]=os.path.basename(result["player"][key])
if result["visualizerStatus"].get("workingDirectory"):
    result["visualizerStatus"]["workingDirectory"]=os.path.basename(result["visualizerStatus"]["workingDirectory"])
print(json.dumps(result,sort_keys=True,separators=(",",":")))
PY
  printf 'status=200\ncontent-type=application/json\n' >"$suite/media.meta"
}

run_media_advanced_service_scenario() {
  local root="$1"
  local port="$(pick_free_port)" https_port="$(pick_free_port)" listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  for implementation in upstream slskr; do
    local state="$work_dir/state-slskdn-media-advanced-service-$implementation"
    local suite="$work_dir/slskdn-media-advanced-service-$implementation"
    local log="$work_dir/slskdn-media-advanced-service-$implementation.log"
    mkdir -p "$state" "$suite"
    write_media_advanced_service_yaml "$state/slskd.yml"
    if [[ "$implementation" == upstream ]]; then
      local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    else
      (
        export SLSKR_CONTROLLER_PROFILE=native
        export SLSKD_APP_DIR="$state" SLSKD_NO_AUTH=true SLSKD_NO_CONNECT=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port"
        export SLSKD_HTTPS_PORT="$https_port" SLSKD_SLSK_LISTEN_PORT="$listen_port"
        slskr_exec serve
      ) >"$log" 2>&1 &
    fi
    daemon_pid="$!"
    wait_for_options "$base_url" "$work_dir/slskdn-media-advanced-service-$implementation-options.json" "$log"
    capture_media_advanced_service "$base_url" "$suite"
    stop_daemon
  done
  normalize_directory_suite "$work_dir/slskdn-media-advanced-service-upstream" "$work_dir/slskdn-media-advanced-service-upstream.normalized"
  normalize_directory_suite "$work_dir/slskdn-media-advanced-service-slskr" "$work_dir/slskdn-media-advanced-service-slskr.normalized"
  if ! diff -ru "$work_dir/slskdn-media-advanced-service-upstream.normalized" "$work_dir/slskdn-media-advanced-service-slskr.normalized"; then
    printf 'media/advanced-service differential failed for slskdn\n' >&2
    exit 1
  fi
  printf 'slskdn media/advanced-service differential passed\n'
}

run_target() {
  local target="$1"
  local root="$2"
  local port
  local https_port
  local listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local upstream_state="$work_dir/state-$target-upstream"
  local slskr_state="$work_dir/state-$target-slskr"
  local upstream_json="$work_dir/$target-upstream.json"
  local slskr_json="$work_dir/$target-slskr.json"
  local upstream_normalized="$work_dir/$target-upstream.normalized.json"
  local slskr_normalized="$work_dir/$target-slskr.normalized.json"
  local upstream_mutations="$work_dir/$target-upstream-mutations"
  local slskr_mutations="$work_dir/$target-slskr-mutations"
  local upstream_log="$work_dir/$target-upstream.log"
  local slskr_log="$work_dir/$target-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$upstream_state" "$slskr_state"
  if [[ "$target" == "slskdn" ]]; then
    mkdir -p "$(dirname "$dll")/wwwroot"
  fi
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    export SLSKD_REMOTE_CONFIGURATION=true
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$upstream_json" "$upstream_log"
  capture_mutation_suite "$base_url" "$upstream_mutations"
  stop_daemon

  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect \
      --remote-configuration
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$slskr_json" "$slskr_log"
  capture_mutation_suite "$base_url" "$slskr_mutations"
  stop_daemon

  normalize_options "$upstream_json" "$upstream_normalized"
  normalize_options "$slskr_json" "$slskr_normalized"
  if ! cmp --silent "$upstream_normalized" "$slskr_normalized"; then
    printf 'options differential failed for %s\n' "$target" >&2
    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
    exit 1
  fi
  compare_mutation_suites "$target" "$upstream_mutations" "$slskr_mutations"
  printf '%s options differential passed\n' "$target"
}

mkdir -p "$work_dir"
