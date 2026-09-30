normalize_directory_suite() {
  local source="$1"
  local destination="$2"
  "$python_bin" - "$source" "$destination" <<'PY'
import json
import pathlib
import re
import shutil
import sys

source = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
destination.mkdir(parents=True, exist_ok=True)
timestamp = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,7})?Z$")

def normalize(value):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"createdAt", "modifiedAt"}:
                if not isinstance(child, str) or not timestamp.fullmatch(child):
                    raise SystemExit(f"invalid filesystem timestamp {child!r} in {source}")
                value[key] = "<TIMESTAMP>"
            elif key == "specTitle" and child in {
                "slskd API",
                "slskdN API",
                "slskR API",
                "slskr API",
            }:
                value[key] = "<DEFAULT_PRODUCT_IDENTITY> API"
            else:
                normalize(child)
    elif isinstance(value, list):
        for child in value:
            normalize(child)

def normalize_debug_runtime_ports(value):
    value = re.sub(
        r"(?m)^(\s*(?:dhtport|overlayport|effectiveoverlayport)=)\d+",
        r"\1<OVERLAY_PORT>",
        value,
    )
    value = re.sub(
        r"(?m)^(\s*(?:port|listenport)=)\d+ \(EnvironmentVariableConfigurationProvider\)$",
        r"\1<RUNTIME_PORT> (EnvironmentVariableConfigurationProvider)",
        value,
    )
    return re.sub(
        r"(?m)^(urls=http://127\.0\.0\.1:)\d+",
        r"\1<HTTP_PORT>",
        value,
    )

default_identity_values = (
    "A slskd user. https://github.com/slskd/slskd",
    "A slskdN user. Unofficial fork of slskd: https://github.com/snapetech/slskdn",
    "A slskR user. https://github.com/snapetech/slskr",
    "slskd/0.0.0 (https://github.com/slskd/slskd)",
    "slskd/0.0.0 (https://github.com/snapetech/slskdn)",
    "slskR/0.0.0 (https://github.com/snapetech/slskr)",
    "slskr",
    "From slskd:",
    "From slskdN:",
    "From slskR:",
    "Hi, I'm human and testing a slskd client. Shares may be temporarily unavailable while I validate the client.",
    "Hi, I'm human and testing a slskdN client. Shares may be temporarily unavailable while I validate the client.",
    "Hi, I'm human and testing an slskR client. Shares may be temporarily unavailable while I validate the client.",
    "Hi, I'm human and testing a slskr client. Shares may be temporarily unavailable while I validate the client.",
)

def normalize_default_identity_text(value):
    for candidate in default_identity_values:
        value = value.replace(candidate, "<DEFAULT_PRODUCT_IDENTITY>")
    value = re.sub(
        r"(?m)^(\s*notificationprefix=)(?:slskd|slskdN|slskR)( \(DefaultValueConfigurationProvider\))?$",
        r"\1<DEFAULT_PRODUCT_IDENTITY>\2",
        value,
    )
    value = re.sub(
        r"(?m)^(\s*username=)(?:slskd|slskdN|slskR|slskr)( \(DefaultValueConfigurationProvider\))?$",
        r"\1<DEFAULT_PRODUCT_IDENTITY>\2",
        value,
    )
    return re.sub(
        r"(?m)^    download:\n      exclude=\[\] \(DefaultValueConfigurationProvider\)\n",
        "",
        value,
    )

def normalize_default_metrics_identity(value):
    if isinstance(value, dict):
        for key, child in value.items():
            if key == "username" and child in {"slskd", "slskdN", "slskR", "slskr"}:
                value[key] = "<DEFAULT_PRODUCT_IDENTITY>"
            else:
                normalize_default_metrics_identity(child)
    elif isinstance(value, list):
        for child in value:
            normalize_default_metrics_identity(child)

for path in source.iterdir():
    output = destination / path.name
    if path.suffix == ".meta":
        shutil.copyfile(path, output)
        continue
    body = path.read_text(encoding="utf-8")
    if not body:
        output.write_text("", encoding="utf-8")
        continue
    try:
        value = json.loads(body)
    except json.JSONDecodeError:
        output.write_text(body, encoding="utf-8")
        continue
    if path.name.startswith("restart-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("management-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("management-options-"):
        value = {"remoteFileManagement": value["remoteFileManagement"]}
    elif path.name.startswith("configuration-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("configuration-options-"):
        value = {
            "remoteConfiguration": value["remoteConfiguration"],
            "debug": value["debug"],
        }
    elif path.name.startswith("configuration-location-"):
        if not isinstance(value, str) or not value.endswith("/slskd.yml"):
            raise SystemExit(f"invalid configuration location {value!r} in {source}")
        value = "<CONFIG_PATH>/slskd.yml"
    elif path.name.startswith("configuration-debug-"):
        if not isinstance(value, str) or not value:
            raise SystemExit(f"invalid configuration debug view in {source}")
        value = re.sub(
            r"/tmp/slskr-options-differential\.[^/]+/state-(?:slskd|slskdn)-configuration-(?:upstream|slskr)",
            "<STATE_DIR>",
            value,
        )
        value = re.sub(
            r"(?m)^(\s*key=)[A-Za-z0-9_+/-]{32,64}( \(DefaultValueConfigurationProvider\))$",
            r"\1<JWT_KEY>\2",
            value,
        )
        value = re.sub(
            r"(?m)^(\s*(?:dhtport|overlayport|effectiveoverlayport)=)\d+",
            r"\1<OVERLAY_PORT>",
            value,
        )
        value = normalize_debug_runtime_ports(value)
        value = normalize_default_identity_text(value)
    elif path.name.startswith("debug-options-") or path.name.startswith("debug-startup-"):
        value = {"debug": value["debug"]}
    elif path.name.startswith("debug-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("debug-view-") and isinstance(value, str):
        value = re.sub(
            r"/tmp/slskr-options-differential\.[^/]+/state-(?:slskd|slskdn)-debug-(?:upstream|slskr)",
            "<STATE_DIR>",
            value,
        )
        value = re.sub(
            r"(?m)^(\s*key=)[A-Za-z0-9_+/-]{32,64}( \(DefaultValueConfigurationProvider\))$",
            r"\1<JWT_KEY>\2",
            value,
        )
        value = normalize_debug_runtime_ports(value)
        value = normalize_default_identity_text(value)
    elif path.name == "metrics-default.body":
        normalize_default_metrics_identity(value)
    elif path.name.startswith("blacklist-options-") or path.name.startswith("blacklist-startup-"):
        blacklist = value["blacklist"]
        configured_file = blacklist.get("file", "")
        value = {
            "enabled": blacklist["enabled"],
            "file": pathlib.Path(configured_file).name if configured_file else "",
        }
    elif path.name.startswith("blacklist-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("blacklist-validation-") and isinstance(value, str):
        value = re.sub(
            r"/tmp/slskr-options-differential\.[^/]+/state-(?:slskd|slskdn)-blacklist-(?:upstream|slskr)",
            "<STATE_DIR>",
            value,
        )
    elif path.name.startswith("groups-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("groups-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("template-options-") or path.name.startswith("template-startup-"):
        value = {
            "completedPathTemplate": value["global"]["download"]["completedPathTemplate"],
        }
    elif path.name.startswith("template-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("template-debug-") and isinstance(value, str):
        lines = [
            line.strip()
            for line in value.splitlines()
            if line.strip().startswith("completedpathtemplate=")
        ]
        if len(lines) != 1:
            raise SystemExit(f"invalid completed-template debug view in {source}: {lines!r}")
        value = lines[0]
    elif path.name.startswith("completed-template-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("dht-options-") or path.name.startswith("dht-startup-"):
        value = {
            "enabled": value["dhtRendezvous"]["enabled"],
            "dhtPort": value["dhtRendezvous"]["dhtPort"],
        }
    elif path.name.startswith("dht-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("dht-status-"):
        for key in ("isEnabled", "isDhtRunning"):
            if not isinstance(value.get(key), bool):
                raise SystemExit(f"invalid DHT status {key} in {source}: {value!r}")
        # Public-DHT bootstrap readiness races independently across the two
        # processes. Controlled local testnet coverage proves that semantic;
        # this differential compares the deterministic configured lifecycle.
        value = {"isEnabled": value["isEnabled"]}
    elif path.name.startswith("dht-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("auto-options-") or path.name.startswith("auto-startup-"):
        value = value["soulseek"]["privateMessageAutoResponse"]
        if value.get("message") in default_identity_values:
            value["message"] = "<DEFAULT_PRODUCT_IDENTITY>"
    elif path.name.startswith("auto-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("auto-fixture-"):
        value = {
            "private_message_acks": value["private_message_acks"],
            "private_message_responses": value["private_message_responses"],
            "injected_private_message_ids": value["injected_private_message_ids"],
        }
        for response in value["private_message_responses"]:
            if response.get("message") in default_identity_values:
                response["message"] = "<DEFAULT_PRODUCT_IDENTITY>"
    elif path.name.startswith("auto-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("endpoint-options-") or path.name.startswith("endpoint-startup-"):
        value = {
            "address": value["soulseek"]["address"],
            "port": value["soulseek"]["port"],
        }
    elif path.name.startswith("endpoint-application-"):
        server = {
            "isConnected": value["server"]["isConnected"],
        }
        for key in ("address", "ipEndPoint"):
            if key in value["server"]:
                server[key] = value["server"][key]
        value = {
            "pendingReconnect": value["pendingReconnect"],
            "server": server,
        }
    elif path.name.startswith("endpoint-server-"):
        selected = {
            "isConnected": value["isConnected"],
        }
        for key in ("address", "ipEndPoint"):
            if key in value:
                selected[key] = value[key]
        value = selected
    elif path.name.startswith("endpoint-network-"):
        value = {
            "accepted": value["accepted"],
            "active": value["active"],
        }
    elif path.name.startswith("endpoint-debug-") and isinstance(value, str):
        soulseek_lines = []
        in_soulseek = False
        for line in value.splitlines():
            if line == "  soulseek:":
                in_soulseek = True
                continue
            if in_soulseek and line.startswith("  ") and not line.startswith("    "):
                break
            if in_soulseek and (line.startswith("    address=") or line.startswith("    port=")):
                soulseek_lines.append(line.strip())
        if len(soulseek_lines) != 2:
            raise SystemExit(f"invalid Soulseek endpoint debug view in {source}: {soulseek_lines!r}")
        value = soulseek_lines
    elif path.name.startswith("endpoint-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
        value = re.sub(
            r"(?m)^(\s*key=)[A-Za-z0-9_+/-]{32,64}( \(DefaultValueConfigurationProvider\))$",
            r"\1<JWT_KEY>\2",
            value,
        )
    elif path.name.startswith("credential-options-") or path.name.startswith("credential-startup-"):
        soulseek = value["soulseek"]
        value = {
            key: soulseek[key]
            for key in ("username", "password")
            if key in soulseek
        }
    elif path.name.startswith("credential-application-"):
        value = {"pendingReconnect": value["pendingReconnect"]}
    elif path.name.startswith("credential-network-"):
        value = {
            "accepted": value["accepted"],
            "active": value["active"],
            "loginUsernames": value.get("login_usernames", []),
            "loginPasswordDigest": value.get("login_password_digest", []),
        }
    elif path.name.startswith("credential-debug-") and isinstance(value, str):
        soulseek_lines = []
        in_soulseek = False
        for line in value.splitlines():
            if line == "  soulseek:":
                in_soulseek = True
                continue
            if in_soulseek and line.startswith("  ") and not line.startswith("    "):
                break
            if in_soulseek and (line.startswith("    username=") or line.startswith("    password=")):
                soulseek_lines.append(line.strip())
        if len(soulseek_lines) not in (0, 2):
            raise SystemExit(f"missing credential debug provider lines in {source}")
        value = "\n".join(soulseek_lines)
    elif path.name.startswith("obfuscation-options-") or path.name.startswith("obfuscation-startup-"):
        value = value["soulseek"]["obfuscation"]
    elif path.name.startswith("obfuscation-validation-"):
        if isinstance(value, dict) and "traceId" in value:
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("obfuscation-application-"):
        value = {"pendingReconnect": value["pendingReconnect"]}
    elif path.name.startswith("obfuscation-network-"):
        messages = value.get("set_wait_port_messages", [])
        value = {
            "advertisementCount": len(messages),
            "lastAdvertisement": messages[-1] if messages else None,
        }
    elif path.name.startswith("configuration-security-"):
        if isinstance(value, dict) and "traceId" in value:
            trace_id = value["traceId"]
            if not isinstance(trace_id, str) or not re.fullmatch(r"0H[A-Z0-9]{11}:[0-9]{8}", trace_id):
                raise SystemExit(f"invalid ASP.NET traceId {trace_id!r} in {source}")
            value["traceId"] = "<TRACE_ID>"
    elif path.name.startswith("no-connect-options-"):
        value = {"noConnect": value["flags"]["noConnect"]}
    elif path.name.startswith("no-connect-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("no-connect-network-"):
        value = {
            "connectionObserved": value.get("accepted", 0) > 0,
            "connectionActive": value.get("active", 0) > 0,
        }
    elif path.name == "no-connect-invalid-watch.body":
        if isinstance(value, dict) and "traceId" in value:
            trace_id = value["traceId"]
            if not isinstance(trace_id, str) or not re.fullmatch(r"0H[A-Z0-9]{11}:[0-9]{8}", trace_id):
                raise SystemExit(f"invalid ASP.NET traceId {trace_id!r} in {source}")
            value["traceId"] = "<TRACE_ID>"
        else:
            value = {"noConnect": value["flags"]["noConnect"]}
    elif path.name == "listener-invalid-watch.body":
        if isinstance(value, dict) and "traceId" in value:
            trace_id = value["traceId"]
            if not isinstance(trace_id, str) or not re.fullmatch(r"0H[A-Z0-9]{11}:[0-9]{8}", trace_id):
                raise SystemExit(f"invalid ASP.NET traceId {trace_id!r} in {source}")
            value["traceId"] = "<TRACE_ID>"
        elif value in (
            "A validation error has occurred.",
            "Collection was modified; enumeration operation may not execute.",
        ):
            value = "<FROZEN_LISTENER_VALIDATION_FAILURE>"
    elif path.name.startswith("config-watch-options-"):
        value = {"noConfigWatch": value["flags"]["noConfigWatch"]}
    elif path.name.startswith("config-watch-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("description-options-"):
        value = {"description": value["soulseek"]["description"]}
    elif path.name.startswith("description-application-"):
        value = {"pendingRestart": value["pendingRestart"]}
    elif path.name.startswith("application-"):
        value = value["shares"]
    elif path.name.startswith("storage-options-"):
        value = value["directories"]
    elif path.name.startswith("options-"):
        value = value["shares"]["directories"]
    normalize(value)
    output.write_text(json.dumps(value, sort_keys=True, separators=(",", ":")), encoding="utf-8")
PY
}

normalize_options() {
  local source="$1"
  local destination="$2"
  local normalize_directories="${3:-yes}"
  "$python_bin" - "$source" "$destination" "$normalize_directories" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    value = json.load(handle)
if sys.argv[3] == "yes":
    value["directories"]["downloads"] = "<APP_DIR>/downloads"
    value["directories"]["incomplete"] = "<APP_DIR>/incomplete"
value["web"]["port"] = "<HTTP_PORT>"
if isinstance(value["web"].get("https"), dict):
    value["web"]["https"]["port"] = "<HTTPS_PORT>"
if isinstance(value.get("dhtRendezvous"), dict):
    for key in ("dhtPort", "overlayPort", "effectiveOverlayPort"):
        if key in value["dhtRendezvous"]:
            value["dhtRendezvous"][key] = "<OVERLAY_PORT>"
# Native slskR identifies itself in the default Soulseek user description.
# The frozen controller profiles use different product identities, while the
# description field itself is covered by the dedicated watched/restart
# scenario below. Keep the default identity out of this structural comparison
# without weakening comparisons for configured descriptions.
default_descriptions = {
    "A slskd user. https://github.com/slskd/slskd",
    "A slskdN user. Unofficial fork of slskd: https://github.com/snapetech/slskdn",
    "A slskR user. https://github.com/snapetech/slskr",
}
if value.get("soulseek", {}).get("description") in default_descriptions:
    value["soulseek"]["description"] = "<DEFAULT_USER_DESCRIPTION>"

default_identity_values = {
    "slskd/0.0.0 (https://github.com/slskd/slskd)",
    "slskd/0.0.0 (https://github.com/snapetech/slskdn)",
    "slskR/0.0.0 (https://github.com/snapetech/slskr)",
    "slskd",
    "slskdN",
    "slskR",
    "slskr",
    "From slskd:",
    "From slskdN:",
    "From slskR:",
    "Hi, I'm human and testing a slskd client. Shares may be temporarily unavailable while I validate the client.",
    "Hi, I'm human and testing a slskdN client. Shares may be temporarily unavailable while I validate the client.",
    "Hi, I'm human and testing an slskR client. Shares may be temporarily unavailable while I validate the client.",
    "Hi, I'm human and testing a slskr client. Shares may be temporarily unavailable while I validate the client.",
}
# The frozen profiles retain their own default authentication username while
# native slskR uses its branded default. Normalize only these known product
# identity fields; configured usernames remain part of the strict comparison.
identity_keys = {"userAgent", "notificationPrefix", "message", "username"}

def normalize_default_identity(node):
    if isinstance(node, dict):
        for key, child in node.items():
            if key in identity_keys and child in default_identity_values:
                node[key] = "<DEFAULT_PRODUCT_IDENTITY>"
            else:
                normalize_default_identity(child)
    elif isinstance(node, list):
        for child in node:
            normalize_default_identity(child)

normalize_default_identity(value)
# The native options projection includes an explicit empty download-filter
# collection. Frozen profiles omit that empty compatibility-only shape; once
# configured, non-empty exclusions remain part of the strict comparison.
download_filter = value.get("filters", {}).get("download")
if isinstance(download_filter, dict) and download_filter.get("exclude") == [] and set(download_filter) == {"exclude"}:
    value["filters"].pop("download")
with open(sys.argv[2], "w", encoding="utf-8") as handle:
    json.dump(value, handle, indent=2, sort_keys=True)
    handle.write("\n")
PY
}

run_directory_scenario() {
  local target="$1"
  local root="$2"
  local port
  local https_port
  local listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local downloads="$work_dir/$target-custom-downloads"
  local incomplete="$work_dir/$target-custom-incomplete"
  local shared="$work_dir/$target-custom-shared"
  local upstream_state="$work_dir/state-$target-directories-upstream"
  local slskr_state="$work_dir/state-$target-directories-slskr"
  local upstream_json="$work_dir/$target-directories-upstream.json"
  local slskr_json="$work_dir/$target-directories-slskr.json"
  local upstream_restart_json="$work_dir/$target-directories-upstream-restart.json"
  local slskr_restart_json="$work_dir/$target-directories-slskr-restart.json"
  local upstream_normalized="$work_dir/$target-directories-upstream.normalized.json"
  local slskr_normalized="$work_dir/$target-directories-slskr.normalized.json"
  local upstream_suite="$work_dir/$target-directories-upstream-files"
  local slskr_suite="$work_dir/$target-directories-slskr-files"
  local upstream_delete_suite="$work_dir/$target-directories-upstream-deletes"
  local slskr_delete_suite="$work_dir/$target-directories-slskr-deletes"
  local upstream_normalized_suite="$work_dir/$target-directories-upstream-files.normalized"
  local slskr_normalized_suite="$work_dir/$target-directories-slskr-files.normalized"
  local upstream_log="$work_dir/$target-directories-upstream.log"
  local slskr_log="$work_dir/$target-directories-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$downloads" "$incomplete" "$shared" "$upstream_state" "$slskr_state"
  mkdir -p "$downloads/nested/deeper" "$incomplete/nested" "$shared/included" "$shared/excluded"
  printf 'completed fixture\n' >"$downloads/song.flac"
  printf 'nested fixture\n' >"$downloads/nested/nested.flac"
  printf 'deep fixture\n' >"$downloads/nested/deeper/deep.flac"
  printf 'incomplete fixture\n' >"$incomplete/incomplete.part"
  printf 'nested incomplete fixture\n' >"$incomplete/nested/nested.part"
  printf 'shared fixture\n' >"$shared/shared.flac"
  printf 'included fixture\n' >"$shared/included/public.flac"
  printf 'excluded fixture\n' >"$shared/excluded/secret.flac"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_IP_ADDRESS=127.0.0.1
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_REMOTE_FILE_MANAGEMENT=true
    export SLSKD_INSTANCE_NAME="$target-directory-proof"
    export SLSKD_DOWNLOADS_DIR="$downloads"
    export SLSKD_INCOMPLETE_DIR="$incomplete"
    export SLSKD_SHARED_DIR="[Library]$shared;!$shared/excluded"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$upstream_json" "$upstream_log"
  wait_for_share_files "$base_url" Library 2 "$upstream_log"
  capture_get "$upstream_suite" downloads "$base_url/api/v0/files/downloads/directories?limit=100&offset=0"
  capture_get "$upstream_suite" downloads-recursive "$base_url/api/v0/files/downloads/directories?recursive=true"
  capture_get "$upstream_suite" downloads-subdirectory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  capture_get "$upstream_suite" incomplete "$base_url/api/v0/files/incomplete/directories?limit=100&offset=0"
  capture_get "$upstream_suite" incomplete-recursive "$base_url/api/v0/files/incomplete/directories?recursive=true"
  capture_get "$upstream_suite" incomplete-subdirectory "$base_url/api/v0/files/incomplete/directories/bmVzdGVk"
  capture_get "$upstream_suite" shares "$base_url/api/v0/shares"
  capture_get "$upstream_suite" shares-contents "$base_url/api/v0/shares/contents"
  capture_get "$upstream_suite" share-library "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$upstream_suite" share-library-contents "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$upstream_suite" share-excluded "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  capture_get "$upstream_suite" share-excluded-contents "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B/contents"
  capture_get "$upstream_suite" share-alias-is-not-id "$base_url/api/v0/shares/Library"
  capture_get "$upstream_suite" share-lowercase-id "$base_url/api/v0/shares/b8100f5ba8bd048a7cf11d116fbbd73130c3c6f5"
  capture_delete "$upstream_delete_suite" existing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$upstream_delete_suite" missing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$upstream_delete_suite" existing-directory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  stop_daemon

  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_REMOTE_FILE_MANAGEMENT=true
    export SLSKD_INSTANCE_NAME="$target-directory-proof"
    export SLSKD_DOWNLOADS_DIR="$downloads"
    export SLSKD_INCOMPLETE_DIR="$incomplete"
    export SLSKD_SHARED_DIR="[Library]$shared;!$shared/excluded"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$upstream_restart_json" "$upstream_log"
  wait_for_share_files "$base_url" Library 2 "$upstream_log"
  capture_get "$upstream_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$upstream_suite" share-library-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$upstream_suite" share-library-contents-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$upstream_suite" share-excluded-restarted "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  stop_daemon

  mkdir -p "$downloads/nested/deeper"
  printf 'completed fixture\n' >"$downloads/song.flac"
  printf 'nested fixture\n' >"$downloads/nested/nested.flac"
  printf 'deep fixture\n' >"$downloads/nested/deeper/deep.flac"

  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-ip-address 127.0.0.1 \
      --slsk-listen-port "$listen_port" \
      --instance-name "$target-directory-proof" \
      --downloads "$downloads" \
      --incomplete "$incomplete" \
      --shared "[Library]$shared;!$shared/excluded" \
      --no-connect \
      --remote-file-management \
      --remote-configuration
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$slskr_json" "$slskr_log"
  wait_for_share_files "$base_url" Library 2 "$slskr_log"
  capture_get "$slskr_suite" downloads "$base_url/api/v0/files/downloads/directories?limit=100&offset=0"
  capture_get "$slskr_suite" downloads-recursive "$base_url/api/v0/files/downloads/directories?recursive=true"
  capture_get "$slskr_suite" downloads-subdirectory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  capture_get "$slskr_suite" incomplete "$base_url/api/v0/files/incomplete/directories?limit=100&offset=0"
  capture_get "$slskr_suite" incomplete-recursive "$base_url/api/v0/files/incomplete/directories?recursive=true"
  capture_get "$slskr_suite" incomplete-subdirectory "$base_url/api/v0/files/incomplete/directories/bmVzdGVk"
  capture_get "$slskr_suite" shares "$base_url/api/v0/shares"
  capture_get "$slskr_suite" shares-contents "$base_url/api/v0/shares/contents"
  capture_get "$slskr_suite" share-library "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$slskr_suite" share-library-contents "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$slskr_suite" share-excluded "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  capture_get "$slskr_suite" share-excluded-contents "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B/contents"
  capture_get "$slskr_suite" share-alias-is-not-id "$base_url/api/v0/shares/Library"
  capture_get "$slskr_suite" share-lowercase-id "$base_url/api/v0/shares/b8100f5ba8bd048a7cf11d116fbbd73130c3c6f5"
  capture_delete "$slskr_delete_suite" existing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$slskr_delete_suite" missing-file "$base_url/api/v0/files/downloads/files/c29uZy5mbGFj"
  capture_delete "$slskr_delete_suite" existing-directory "$base_url/api/v0/files/downloads/directories/bmVzdGVk"
  stop_daemon

  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --instance-name "$target-directory-proof" \
      --downloads "$downloads" \
      --incomplete "$incomplete" \
      --shared "[Library]$shared;!$shared/excluded" \
      --no-connect \
      --remote-file-management \
      --remote-configuration
  ) >>"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$slskr_restart_json" "$slskr_log"
  wait_for_share_files "$base_url" Library 2 "$slskr_log"
  capture_get "$slskr_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$slskr_suite" share-library-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5"
  capture_get "$slskr_suite" share-library-contents-restarted "$base_url/api/v0/shares/B8100F5BA8BD048A7CF11D116FBBD73130C3C6F5/contents"
  capture_get "$slskr_suite" share-excluded-restarted "$base_url/api/v0/shares/7471FCE8530D7BD0B0F7AD1269E277308456DA4B"
  stop_daemon

  normalize_options "$upstream_json" "$upstream_normalized" no
  normalize_options "$slskr_json" "$slskr_normalized" no
  if ! cmp --silent "$upstream_normalized" "$slskr_normalized"; then
    printf 'custom directory options differential failed for %s\n' "$target" >&2
    diff -u "$upstream_normalized" "$slskr_normalized" >&2 || true
    exit 1
  fi
  normalize_directory_suite "$upstream_suite" "$upstream_normalized_suite"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized_suite"
  if ! diff -ru "$upstream_normalized_suite" "$slskr_normalized_suite"; then
    printf 'custom directory file-management differential failed for %s\n' "$target" >&2
    exit 1
  fi
  if ! diff -ru "$upstream_delete_suite" "$slskr_delete_suite"; then
    printf 'custom directory delete differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s custom directory differential passed\n' "$target"
}

write_share_watch_yaml() {
  local path="$1"
  local alias="$2"
  local directory="$3"
  local temporary="$path.tmp"
  printf 'shares:\n  directories:\n    - "[%s]%s"\n' "$alias" "$directory" >"$temporary"
  mv "$temporary" "$path"
}

write_share_watch_yaml_in_place() {
  local path="$1"
  local alias="$2"
  local directory="$3"
  printf 'shares:\n  directories:\n    - "[%s]%s"\n' "$alias" "$directory" >"$path"
}

wait_for_share_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; values=json.load(sys.stdin)["shares"]["directories"]; print(values[0] if values else "")' 2>/dev/null || true)"
    if [[ "$current" == "$expected" ]]; then
      return
    fi
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'share watch differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'share watch differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

run_share_watch_scenario() {
  local target="$1"
  local root="$2"
  local port
  local https_port
  local listen_port
  port="$(pick_free_port)"
  https_port="$(pick_free_port)"
  listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local old_share="$work_dir/$target-watch-old"
  local new_share="$work_dir/$target-watch-new"
  local upstream_state="$work_dir/state-$target-watch-upstream"
  local slskr_state="$work_dir/state-$target-watch-slskr"
  local upstream_suite="$work_dir/$target-watch-upstream"
  local slskr_suite="$work_dir/$target-watch-slskr"
  local upstream_normalized="$work_dir/$target-watch-upstream.normalized"
  local slskr_normalized="$work_dir/$target-watch-slskr.normalized"
  local upstream_log="$work_dir/$target-watch-upstream.log"
  local slskr_log="$work_dir/$target-watch-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
  local old_raw="[Old]$old_share"
  local new_raw="[New]$new_share"

  mkdir -p "$old_share" "$new_share" "$upstream_state" "$slskr_state"
  printf 'old watched fixture\n' >"$old_share/old.flac"
  printf 'new watched fixture\n' >"$new_share/new.flac"
  write_share_watch_yaml "$upstream_state/slskd.yml" Old "$old_share"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-watch-upstream-options.json" "$upstream_log"
  wait_for_share_files "$base_url" Old 1 "$upstream_log"
  capture_get "$upstream_suite" options-before "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-before "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-before "$base_url/api/v0/application"
  printf 'shares: [unterminated' >"$upstream_state/slskd.yml.tmp"
  mv "$upstream_state/slskd.yml.tmp" "$upstream_state/slskd.yml"
  wait_for_share_option "$base_url" "" "$upstream_log"
  capture_get "$upstream_suite" options-invalid "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-invalid "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-invalid "$base_url/api/v0/application"
  write_share_watch_yaml "$upstream_state/slskd.yml" New "$new_share"
  wait_for_share_option "$base_url" "$new_raw" "$upstream_log"
  capture_get "$upstream_suite" options-watched "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-watched "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-watched "$base_url/api/v0/application"
  capture_put "$upstream_suite" shares-rescan-response "$base_url/api/v0/shares"
  wait_for_share_files "$base_url" New 1 "$upstream_log"
  capture_get "$upstream_suite" shares-rescanned "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-rescanned "$base_url/api/v0/application"
  stop_daemon

  write_share_watch_yaml "$slskr_state/slskd.yml" Old "$old_share"
  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    # The frozen slskdN profile enables its controller rate limiter by
    # default, but the reference fixture does not. This scenario intentionally
    # polls rapidly while waiting for watcher transitions; disable only the
    # slskR fixture's limiter so throttling cannot mask share-watch behavior.
    export SLSKD_WEB_RATE_LIMITING=false
    slskr_exec serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-watch-slskr-options.json" "$slskr_log"
  wait_for_share_files "$base_url" Old 1 "$slskr_log"
  capture_get "$slskr_suite" options-before "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-before "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-before "$base_url/api/v0/application"
  printf 'shares: [unterminated' >"$slskr_state/slskd.yml.tmp"
  mv "$slskr_state/slskd.yml.tmp" "$slskr_state/slskd.yml"
  wait_for_share_option "$base_url" "" "$slskr_log"
  capture_get "$slskr_suite" options-invalid "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-invalid "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-invalid "$base_url/api/v0/application"
  write_share_watch_yaml "$slskr_state/slskd.yml" New "$new_share"
  wait_for_share_option "$base_url" "$new_raw" "$slskr_log"
  capture_get "$slskr_suite" options-watched "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-watched "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-watched "$base_url/api/v0/application"
  capture_put "$slskr_suite" shares-rescan-response "$base_url/api/v0/shares"
  wait_for_share_files "$base_url" New 1 "$slskr_log"
  capture_get "$slskr_suite" shares-rescanned "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-rescanned "$base_url/api/v0/application"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'share watch differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s share watch differential passed\n' "$target"
}

run_no_watch_upload_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local old_share="$work_dir/$target-no-watch-old"
  local new_share="$work_dir/$target-no-watch-new"
  local upstream_state="$work_dir/state-$target-no-watch-upstream"
  local slskr_state="$work_dir/state-$target-no-watch-slskr"
  local upstream_suite="$work_dir/$target-no-watch-upstream"
  local slskr_suite="$work_dir/$target-no-watch-slskr"
  local upstream_normalized="$work_dir/$target-no-watch-upstream.normalized"
  local slskr_normalized="$work_dir/$target-no-watch-slskr.normalized"
  local upstream_log="$work_dir/$target-no-watch-upstream.log"
  local slskr_log="$work_dir/$target-no-watch-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
  local new_raw="[New]$new_share"
  local payload

  mkdir -p "$old_share" "$new_share" "$upstream_state" "$slskr_state"
  printf 'old no-watch fixture\n' >"$old_share/old.flac"
  printf 'new no-watch fixture\n' >"$new_share/new.flac"
  payload="$($python_bin -c 'import json,sys; print(json.dumps("shares:\n  directories:\n    - \"[New]" + sys.argv[1] + "\"\n"))' "$new_share")"

  write_share_watch_yaml "$upstream_state/slskd.yml" Old "$old_share"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONFIG_WATCH=true
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-upstream-options.json" "$upstream_log"
  wait_for_share_files "$base_url" Old 1 "$upstream_log"
  capture_request "$upstream_suite" yaml-put PUT "$base_url/api/v0/options/yaml" "$payload"
  wait_for_share_option "$base_url" "$new_raw" "$upstream_log"
  capture_get "$upstream_suite" options-uploaded "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-uploaded "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-uploaded "$base_url/api/v0/application"
  capture_get "$upstream_suite" yaml-uploaded "$base_url/api/v0/options/yaml"
  write_share_watch_yaml_in_place "$upstream_state/slskd.yml" Direct "$new_share"
  wait_for_share_option "$base_url" "[Direct]$new_share" "$upstream_log"
  capture_get "$upstream_suite" options-direct-write "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-direct-write "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-direct-write "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONFIG_WATCH=true
    export SLSKD_REMOTE_CONFIGURATION=true
    export SLSKD_NO_CONNECT=true
    export SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1
    export SLSKD_HTTP_PORT="$port"
    export SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-upstream-restart-options.json" "$upstream_log"
  wait_for_share_files "$base_url" Direct 1 "$upstream_log"
  capture_get "$upstream_suite" options-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$upstream_suite" application-restarted "$base_url/api/v0/application"
  stop_daemon

  write_share_watch_yaml "$slskr_state/slskd.yml" Old "$old_share"
  (
    export SLSKR_AUTH_DISABLED=true
    export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve \
      --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 \
      --http-port "$port" \
      --slsk-listen-port "$listen_port" \
      --no-connect \
      --no-config-watch \
      --remote-configuration
  ) >"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-slskr-options.json" "$slskr_log"
  wait_for_share_files "$base_url" Old 1 "$slskr_log"
  capture_request "$slskr_suite" yaml-put PUT "$base_url/api/v0/options/yaml" "$payload"
  wait_for_share_option "$base_url" "$new_raw" "$slskr_log"
  capture_get "$slskr_suite" options-uploaded "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-uploaded "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-uploaded "$base_url/api/v0/application"
  capture_get "$slskr_suite" yaml-uploaded "$base_url/api/v0/options/yaml"
  write_share_watch_yaml_in_place "$slskr_state/slskd.yml" Direct "$new_share"
  wait_for_share_option "$base_url" "[Direct]$new_share" "$slskr_log"
  capture_get "$slskr_suite" options-direct-write "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-direct-write "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-direct-write "$base_url/api/v0/application"
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
      --no-config-watch \
      --remote-configuration
  ) >>"$slskr_log" 2>&1 &
  daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-no-watch-slskr-restart-options.json" "$slskr_log"
  wait_for_share_files "$base_url" Direct 1 "$slskr_log"
  capture_get "$slskr_suite" options-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" shares-restarted "$base_url/api/v0/shares"
  capture_get "$slskr_suite" application-restarted "$base_url/api/v0/application"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'no-watch YAML upload differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s no-watch YAML upload differential passed\n' "$target"
}

write_storage_watch_yaml() {
  local path="$1"
  local downloads="$2"
  local incomplete="$3"
  local temporary="$path.tmp"
  printf 'directories:\n  downloads: "%s"\n  incomplete: "%s"\n' "$downloads" "$incomplete" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_download_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(json.load(sys.stdin)["directories"]["downloads"])' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'storage watch differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'storage watch differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

run_storage_restart_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local old_downloads="$work_dir/$target-restart-old-downloads"
  local old_incomplete="$work_dir/$target-restart-old-incomplete"
  local new_downloads="$work_dir/$target-restart-new-downloads"
  local new_incomplete="$work_dir/$target-restart-new-incomplete"
  local upstream_state="$work_dir/state-$target-restart-upstream"
  local slskr_state="$work_dir/state-$target-restart-slskr"
  local upstream_suite="$work_dir/$target-restart-upstream"
  local slskr_suite="$work_dir/$target-restart-slskr"
  local upstream_normalized="$work_dir/$target-restart-upstream.normalized"
  local slskr_normalized="$work_dir/$target-restart-slskr.normalized"
  local upstream_log="$work_dir/$target-restart-upstream.log"
  local slskr_log="$work_dir/$target-restart-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$old_downloads" "$old_incomplete" "$new_downloads" "$new_incomplete" "$upstream_state" "$slskr_state"
  printf 'old download\n' >"$old_downloads/old.flac"
  printf 'old incomplete\n' >"$old_incomplete/old.part"
  printf 'new download\n' >"$new_downloads/new.flac"
  printf 'new incomplete\n' >"$new_incomplete/new.part"

  write_storage_watch_yaml "$upstream_state/slskd.yml" "$old_downloads" "$old_incomplete"
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-upstream-options.json" "$upstream_log"
  capture_get "$upstream_suite" storage-options-before "$base_url/api/v0/options"
  capture_get "$upstream_suite" downloads-before "$base_url/api/v0/files/downloads/directories"
  capture_get "$upstream_suite" incomplete-before "$base_url/api/v0/files/incomplete/directories"
  capture_get "$upstream_suite" restart-application-before "$base_url/api/v0/application"
  write_storage_watch_yaml "$upstream_state/slskd.yml" "$new_downloads" "$new_incomplete"
  wait_for_download_option "$base_url" "$new_downloads" "$upstream_log"
  capture_get "$upstream_suite" storage-options-watched "$base_url/api/v0/options"
  capture_get "$upstream_suite" downloads-watched "$base_url/api/v0/files/downloads/directories"
  capture_get "$upstream_suite" incomplete-watched "$base_url/api/v0/files/incomplete/directories"
  capture_get "$upstream_suite" restart-application-watched "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state"
    export SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-upstream-reloaded-options.json" "$upstream_log"
  capture_get "$upstream_suite" storage-options-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" downloads-restarted "$base_url/api/v0/files/downloads/directories"
  capture_get "$upstream_suite" incomplete-restarted "$base_url/api/v0/files/incomplete/directories"
  capture_get "$upstream_suite" restart-application-restarted "$base_url/api/v0/application"
  stop_daemon

  write_storage_watch_yaml "$slskr_state/slskd.yml" "$old_downloads" "$old_incomplete"
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-slskr-options.json" "$slskr_log"
  capture_get "$slskr_suite" storage-options-before "$base_url/api/v0/options"
  capture_get "$slskr_suite" downloads-before "$base_url/api/v0/files/downloads/directories"
  capture_get "$slskr_suite" incomplete-before "$base_url/api/v0/files/incomplete/directories"
  capture_get "$slskr_suite" restart-application-before "$base_url/api/v0/application"
  write_storage_watch_yaml "$slskr_state/slskd.yml" "$new_downloads" "$new_incomplete"
  wait_for_download_option "$base_url" "$new_downloads" "$slskr_log"
  capture_get "$slskr_suite" storage-options-watched "$base_url/api/v0/options"
  capture_get "$slskr_suite" downloads-watched "$base_url/api/v0/files/downloads/directories"
  capture_get "$slskr_suite" incomplete-watched "$base_url/api/v0/files/incomplete/directories"
  capture_get "$slskr_suite" restart-application-watched "$base_url/api/v0/application"
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >>"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-restart-slskr-reloaded-options.json" "$slskr_log"
  capture_get "$slskr_suite" storage-options-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" downloads-restarted "$base_url/api/v0/files/downloads/directories"
  capture_get "$slskr_suite" incomplete-restarted "$base_url/api/v0/files/incomplete/directories"
  capture_get "$slskr_suite" restart-application-restarted "$base_url/api/v0/application"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'storage restart differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s storage restart differential passed\n' "$target"
}

write_management_watch_yaml() {
  local path="$1"
  local enabled="$2"
  local downloads="$3"
  local incomplete="$4"
  local temporary="$path.tmp"
  printf 'remote_file_management: %s\ndirectories:\n  downloads: "%s"\n  incomplete: "%s"\n' \
    "$enabled" "$downloads" "$incomplete" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_management_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteFileManagement"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'remote file-management differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'remote file-management differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

run_remote_file_management_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local downloads="$work_dir/$target-management-downloads"
  local incomplete="$work_dir/$target-management-incomplete"
  local upstream_state="$work_dir/state-$target-management-upstream"
  local slskr_state="$work_dir/state-$target-management-slskr"
  local upstream_suite="$work_dir/$target-management-upstream"
  local slskr_suite="$work_dir/$target-management-slskr"
  local upstream_normalized="$work_dir/$target-management-upstream.normalized"
  local slskr_normalized="$work_dir/$target-management-slskr.normalized"
  local upstream_log="$work_dir/$target-management-upstream.log"
  local slskr_log="$work_dir/$target-management-slskr.log"
  local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"

  mkdir -p "$downloads" "$incomplete" "$upstream_state" "$slskr_state"
  printf 'denied before\n' >"$downloads/denied-before.flac"
  printf 'allowed watched\n' >"$downloads/allowed-watched.flac"
  printf 'allowed restarted\n' >"$downloads/allowed-restarted.flac"
  printf 'denied watched\n' >"$downloads/denied-watched.flac"
  printf 'denied restarted\n' >"$downloads/denied-restarted.flac"

  write_management_watch_yaml "$upstream_state/slskd.yml" false "$downloads" "$incomplete"
  (
    export SLSKD_APP_DIR="$upstream_state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-upstream-options.json" "$upstream_log"
  capture_get "$upstream_suite" management-options-before "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-before "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-denied-before "$base_url/api/v0/files/downloads/files/ZGVuaWVkLWJlZm9yZS5mbGFj"
  write_management_watch_yaml "$upstream_state/slskd.yml" true "$downloads" "$incomplete"
  wait_for_management_option "$base_url" true "$upstream_log"
  capture_get "$upstream_suite" management-options-enabled "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-enabled "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-allowed-watched "$base_url/api/v0/files/downloads/files/YWxsb3dlZC13YXRjaGVkLmZsYWM="
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-upstream-restart-options.json" "$upstream_log"
  capture_get "$upstream_suite" management-options-enabled-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-enabled-restarted "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-allowed-restarted "$base_url/api/v0/files/downloads/files/YWxsb3dlZC1yZXN0YXJ0ZWQuZmxhYw=="
  write_management_watch_yaml "$upstream_state/slskd.yml" false "$downloads" "$incomplete"
  wait_for_management_option "$base_url" false "$upstream_log"
  capture_get "$upstream_suite" management-options-disabled "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-disabled "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-denied-watched "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXdhdGNoZWQuZmxhYw=="
  stop_daemon
  (
    export SLSKD_APP_DIR="$upstream_state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
    export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
    export SLSKD_SLSK_LISTEN_PORT="$listen_port"
    exec dotnet "$dll"
  ) >>"$upstream_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-upstream-disabled-restart-options.json" "$upstream_log"
  capture_get "$upstream_suite" management-options-disabled-restarted "$base_url/api/v0/options"
  capture_get "$upstream_suite" management-application-disabled-restarted "$base_url/api/v0/application"
  capture_delete "$upstream_suite" delete-denied-restarted "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXJlc3RhcnRlZC5mbGFj"
  stop_daemon

  printf 'denied before\n' >"$downloads/denied-before.flac"
  printf 'allowed watched\n' >"$downloads/allowed-watched.flac"
  printf 'allowed restarted\n' >"$downloads/allowed-restarted.flac"
  printf 'denied watched\n' >"$downloads/denied-watched.flac"
  printf 'denied restarted\n' >"$downloads/denied-restarted.flac"
  write_management_watch_yaml "$slskr_state/slskd.yml" false "$downloads" "$incomplete"
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-slskr-options.json" "$slskr_log"
  capture_get "$slskr_suite" management-options-before "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-before "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-denied-before "$base_url/api/v0/files/downloads/files/ZGVuaWVkLWJlZm9yZS5mbGFj"
  write_management_watch_yaml "$slskr_state/slskd.yml" true "$downloads" "$incomplete"
  wait_for_management_option "$base_url" true "$slskr_log"
  capture_get "$slskr_suite" management-options-enabled "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-enabled "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-allowed-watched "$base_url/api/v0/files/downloads/files/YWxsb3dlZC13YXRjaGVkLmZsYWM="
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >>"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-slskr-restart-options.json" "$slskr_log"
  capture_get "$slskr_suite" management-options-enabled-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-enabled-restarted "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-allowed-restarted "$base_url/api/v0/files/downloads/files/YWxsb3dlZC1yZXN0YXJ0ZWQuZmxhYw=="
  write_management_watch_yaml "$slskr_state/slskd.yml" false "$downloads" "$incomplete"
  wait_for_management_option "$base_url" false "$slskr_log"
  capture_get "$slskr_suite" management-options-disabled "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-disabled "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-denied-watched "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXdhdGNoZWQuZmxhYw=="
  stop_daemon
  (
    export SLSKR_AUTH_DISABLED=true SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
    slskr_exec serve --app-dir "$slskr_state" \
      --http-ip-address 127.0.0.1 --http-port "$port" --slsk-listen-port "$listen_port" --no-connect
  ) >>"$slskr_log" 2>&1 & daemon_pid="$!"
  wait_for_options "$base_url" "$work_dir/$target-management-slskr-disabled-restart-options.json" "$slskr_log"
  capture_get "$slskr_suite" management-options-disabled-restarted "$base_url/api/v0/options"
  capture_get "$slskr_suite" management-application-disabled-restarted "$base_url/api/v0/application"
  capture_delete "$slskr_suite" delete-denied-restarted "$base_url/api/v0/files/downloads/files/ZGVuaWVkLXJlc3RhcnRlZC5mbGFj"
  stop_daemon

  normalize_directory_suite "$upstream_suite" "$upstream_normalized"
  normalize_directory_suite "$slskr_suite" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'remote file-management differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s remote file-management differential passed\n' "$target"
}

write_remote_configuration_yaml() {
  local path="$1"
  local enabled="$2"
  local temporary="$path.tmp"
  printf 'debug: true\nremote_configuration: %s\n' "$enabled" >"$temporary"
  mv "$temporary" "$path"
}

wait_for_configuration_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["remoteConfiguration"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'remote configuration differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'remote configuration differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_remote_configuration_stage() {
  local target="$1"
  local base_url="$2"
  local suite="$3"
  local stage="$4"
  capture_get "$suite" "configuration-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "configuration-options-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "configuration-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "configuration-yaml-$stage" "$base_url/api/v0/options/yaml"
  capture_get "$suite" "configuration-location-$stage" "$base_url/api/v0/options/yaml/location"
  capture_get "$suite" "configuration-debug-$stage" "$base_url/api/v0/options/debug"
  capture_request "$suite" "configuration-validate-$stage" POST \
    "$base_url/api/v0/options/yaml/validate" '"debug: true\n"'
  capture_request "$suite" "configuration-patch-$stage" PATCH \
    "$base_url/api/v0/options" '{}'
  if [[ "$target" == "slskdn" && ("$stage" == disabled || "$stage" == enabled) ]]; then
    capture_get "$suite" "configuration-security-current-before-$stage" \
      "$base_url/api/v0/security/adversarial"
    capture_request "$suite" "configuration-security-$stage" PUT \
      "$base_url/api/v0/security/adversarial" '{}'
    if [[ "$stage" == enabled ]]; then
      capture_get "$suite" "configuration-security-current-default-$stage" \
        "$base_url/api/v0/security/adversarial"
      capture_get "$suite" "configuration-security-yaml-$stage" \
        "$base_url/api/v0/options/yaml"
      capture_request "$suite" "configuration-security-custom-$stage" PUT \
        "$base_url/api/v0/security/adversarial" \
        '{"enabled":true,"profile":"Custom","privacy":{"enabled":true,"padding":{"enabled":true,"bucketSizes":[256,512]}}}'
      capture_get "$suite" "configuration-security-current-custom-$stage" \
        "$base_url/api/v0/security/adversarial"
      capture_get "$suite" "configuration-security-yaml-custom-$stage" \
        "$base_url/api/v0/options/yaml"
      capture_request "$suite" "configuration-security-invalid-$stage" PUT \
        "$base_url/api/v0/security/adversarial" \
        '{"privacy":{"padding":{"bucketSizes":[256,0]}}}'
      capture_get "$suite" "configuration-security-current-after-invalid-$stage" \
        "$base_url/api/v0/security/adversarial"
      capture_get "$suite" "configuration-security-yaml-after-invalid-$stage" \
        "$base_url/api/v0/options/yaml"
    fi
  fi
}

start_remote_configuration_daemon() {
  local target="$1"
  local root="$2"
  local implementation="$3"
  local state="$4"
  local log="$5"
  local port="$6"
  local https_port="$7"
  local listen_port="$8"
  local append="${9:-false}"
  if [[ "$implementation" == upstream ]]; then
    local dll="$root/src/slskd/bin/Release/net10.0/linux-x64/slskd.dll"
    if [[ "$append" == true ]]; then
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >>"$log" 2>&1 &
    else
      (
        export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
        export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
        export SLSKD_SLSK_LISTEN_PORT="$listen_port"
        exec dotnet "$dll"
      ) >"$log" 2>&1 &
    fi
  elif [[ "$append" == true ]]; then
    (
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      slskr_exec serve
    ) >>"$log" 2>&1 &
  else
    (
      export SLSKR_CONTROLLER_PROFILE="$(runtime_profile_for_reference "$target")"
      export SLSKD_APP_DIR="$state" SLSKD_NO_CONNECT=true SLSKD_NO_AUTH=true
      export SLSKD_HTTP_IP_ADDRESS=127.0.0.1 SLSKD_HTTP_PORT="$port" SLSKD_HTTPS_PORT="$https_port"
      export SLSKD_SLSK_LISTEN_PORT="$listen_port"
      slskr_exec serve
    ) >"$log" 2>&1 &
  fi
  daemon_pid="$!"
}

run_remote_configuration_scenario() {
  local target="$1"
  local root="$2"
  local port="$(pick_free_port)"
  local https_port="$(pick_free_port)"
  local listen_port="$(pick_free_port)"
  local base_url="http://127.0.0.1:$port"
  local false_payload
  false_payload="$($python_bin -c 'import json; print(json.dumps("debug: true\nremote_configuration: false\n"))')"

  for implementation in upstream slskr; do
    local state="$work_dir/state-$target-configuration-$implementation"
    local suite="$work_dir/$target-configuration-$implementation"
    local log="$work_dir/$target-configuration-$implementation.log"
    mkdir -p "$state"
    write_remote_configuration_yaml "$state/slskd.yml" false
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port"
    wait_for_options "$base_url" "$work_dir/$target-configuration-$implementation-options.json" "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" disabled

    write_remote_configuration_yaml "$state/slskd.yml" true
    wait_for_configuration_option "$base_url" true "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" enabled
    capture_request "$suite" configuration-yaml-self-disable PUT \
      "$base_url/api/v0/options/yaml" "$false_payload"
    wait_for_configuration_option "$base_url" false "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" self-disabled
    # Allow the polling watcher to observe the uploaded false value before the
    # next direct write; the upload path itself applies synchronously.
    sleep 0.3

    write_remote_configuration_yaml "$state/slskd.yml" true
    wait_for_configuration_option "$base_url" true "$log"
    stop_daemon
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-configuration-$implementation-restart-options.json" "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" enabled-restarted
    stop_daemon

    write_remote_configuration_yaml "$state/slskd.yml" false
    start_remote_configuration_daemon "$target" "$root" "$implementation" "$state" "$log" \
      "$port" "$https_port" "$listen_port" true
    wait_for_options "$base_url" "$work_dir/$target-configuration-$implementation-disabled-restart-options.json" "$log"
    capture_remote_configuration_stage "$target" "$base_url" "$suite" disabled-restarted
    stop_daemon
  done

  local upstream_normalized="$work_dir/$target-configuration-upstream.normalized"
  local slskr_normalized="$work_dir/$target-configuration-slskr.normalized"
  normalize_directory_suite "$work_dir/$target-configuration-upstream" "$upstream_normalized"
  normalize_directory_suite "$work_dir/$target-configuration-slskr" "$slskr_normalized"
  if ! diff -ru "$upstream_normalized" "$slskr_normalized"; then
    printf 'remote configuration differential failed for %s\n' "$target" >&2
    exit 1
  fi
  printf '%s remote configuration differential passed\n' "$target"
}

write_debug_yaml() {
  local path="$1"
  local enabled="$2"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\ndebug: %s\n' "$enabled" >"$temporary"
  mv "$temporary" "$path"
}

write_debug_omitted_yaml() {
  local path="$1"
  local temporary="$path.tmp"
  printf 'remote_configuration: true\n' >"$temporary"
  mv "$temporary" "$path"
}

wait_for_debug_option() {
  local base_url="$1"
  local expected="$2"
  local log="$3"
  for _ in $(seq 1 600); do
    local current
    current="$(curl --fail --silent --max-time 1 "$base_url/api/v0/options" 2>/dev/null \
      | "$python_bin" -c 'import json,sys; print(str(json.load(sys.stdin)["debug"]).lower())' 2>/dev/null || true)"
    [[ "$current" == "$expected" ]] && return
    if ! kill -0 "$daemon_pid" 2>/dev/null; then
      printf 'debug differential failed: daemon exited while waiting for %s\n' "$expected" >&2
      tail -120 "$log" >&2 || true
      exit 1
    fi
    sleep 0.1
  done
  printf 'debug differential failed: timed out waiting for %s\n' "$expected" >&2
  tail -120 "$log" >&2 || true
  exit 1
}

capture_debug_log_state() {
  local suite="$1"
  local stage="$2"
  local log="$3"
  local observed=false
  if rg -q '\bDBG\]|\[Debug\]' "$log"; then
    observed=true
  fi
  printf '{"debugObserved":%s}' "$observed" >"$suite/debug-log-$stage.body"
  printf 'status=200\ncontent-type=application/json\n' >"$suite/debug-log-$stage.meta"
}

capture_debug_stage() {
  local base_url="$1"
  local suite="$2"
  local stage="$3"
  local log="$4"
  mkdir -p "$suite"
  capture_get "$suite" "debug-options-$stage" "$base_url/api/v0/options"
  capture_get "$suite" "debug-startup-$stage" "$base_url/api/v0/options/startup"
  capture_get "$suite" "debug-application-$stage" "$base_url/api/v0/application"
  capture_get "$suite" "debug-view-$stage" "$base_url/api/v0/options/debug"
  capture_debug_log_state "$suite" "$stage" "$log"
}

