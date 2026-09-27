use super::*;

pub(super) async fn native_capabilities_response(state: &AppState) -> HttpResponse {
    let media = state.media_services.read().await;
    let mut features = vec![
        "mbid_jobs",
        "discography_jobs",
        "label_crate_jobs",
        "canonical_scoring",
        "rescue_mode",
        "library_health",
        "warm_cache",
        "job_manifests",
        "session_traces",
        "playback_aware",
        "soulseek_type1_obfuscation_options",
        "soulseek_type1_obfuscated_distributed_messages",
        "soulseek_type1_obfuscated_file_transfers",
    ];
    if media.features.scene_pod_bridge {
        features.push("scene_pod_bridge");
    }
    let config = &state.config;
    let requested_listen_port =
        (config.obfuscation_listen_port > 0).then_some(config.obfuscation_listen_port);
    let effective_listen_port = config.obfuscated_advertised_port;
    let mode = config.obfuscation_mode.as_str();
    let limitations = vec![
        "Type-1 obfuscation is a compatibility/privacy posture, not transport security or meaningful encryption.",
        "Current runtime support covers peer-message (P), distributed-message (D), and file-transfer (F) streams.",
        "Regular transfer paths remain advertised and available for legacy-client compatibility.",
    ];
    let summary = if config.obfuscation_enabled {
        if mode == "prefer" && config.obfuscation_prefer_outbound {
            "Prefer mode uses obfuscated peer/distributed/transfer dials when peers advertise type-1 metadata and keeps regular fallback."
        } else {
            "Compatibility mode keeps regular outbound peer/distributed/transfer dials first and adds obfuscated reachability."
        }
    } else {
        "Soulseek type-1 peer/distributed/transfer obfuscation is disabled."
    };
    let body = serde_json::json!({
        "impl": "slskdn",
        "compat": "slskd",
        "version": APP_VERSION,
        "features": features,
        "obfuscation": {
            "enabled": config.obfuscation_enabled,
            "mode": mode,
            "type": 1,
            "regularListenPort": config.listen_port,
            "requestedListenPort": requested_listen_port,
            "effectiveListenPort": effective_listen_port,
            "advertiseRegularPort": config.obfuscation_advertise_regular_port,
            "preferOutbound": mode == "prefer" && config.obfuscation_prefer_outbound,
            "supportedConnectionTypes": ["P", "D", "F"],
            "runtimeSupported": true,
            "runtimeState": if config.obfuscation_enabled { "active" } else { "disabled" },
            "summary": summary,
            "limitations": limitations,
        },
        "feature": {
            "scenePodBridge": media.features.scene_pod_bridge,
        },
    });
    drop(media);
    routing::ok_response(body.to_string())
}

pub(super) async fn native_capability_controller_response(state: &AppState) -> HttpResponse {
    #[derive(Serialize)]
    struct CapabilityFile {
        client: &'static str,
        version: &'static str,
        features: [&'static str; 6],
        protocol_version: i32,
        capabilities: i32,
        mesh_seq_id: u64,
    }

    #[derive(Serialize)]
    struct CapabilityResponse {
        version: &'static str,
        tag: &'static str,
        json: String,
    }

    let mesh_seq_id = state.content_discovery.read().await.latest_seq();
    let capability_json = CapabilityFile {
        client: "slskdn",
        version: "1.0.0",
        features: [
            "dht",
            "hash_exchange",
            "mesh_sync",
            "flac_hash_db",
            "swarm_download",
            "partial_download",
        ],
        protocol_version: 1,
        capabilities: 63,
        mesh_seq_id,
    };
    let response = CapabilityResponse {
        version: "slskdn/1.0.0+dht+mesh+swarm",
        tag: "slskdn_caps:v1;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1",
        json: serde_json::to_string_pretty(&capability_json).unwrap_or_else(|_| "{}".to_owned()),
    };
    routing::ok_response(serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_owned()))
}

pub(super) fn capabilities_response() -> HttpResponse {
    let capability_json = serde_json::json!({
        "client": "slskr",
        "version": APP_VERSION,
        "features": ["dht", "hash_exchange", "mesh_sync", "flac_hash_db", "swarm_download", "partial_download"],
        "protocol_version": 1,
        "capabilities": 63,
        "mesh_seq_id": 0,
    });
    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: serde_json::json!({
        "version": format!("slskr/{APP_VERSION}+dht+mesh+swarm"),
            "impl": "slskr",
            "compat": "slskr-v1",
            "features": capability_json["features"].clone(),
            "obfuscation": {"type1": true, "mode": "compatibility"},
            "feature": {"dht": true, "mesh": true, "swarm": true},
            "tag": "slskr_caps:v1;dht=1;mesh=1;swarm=1;hashx=1;flacdb=1",
            "json": serde_json::to_string_pretty(&capability_json).unwrap_or_else(|_| "{}".to_owned()),
            "api_version": "v0",
            "client_version": APP_VERSION,
            "supports": ["login","peers","shares","searches","transfers","users","messages","rooms","room-list-sync","browser-session-auth"],
        }).to_string(),
    }
}

pub(super) async fn network_stats_value(
    state: &AppState,
    include_peers: bool,
) -> serde_json::Value {
    let searches = state.searches.read().await;
    let search_count = searches.records.len();
    let search_result_count = searches
        .records
        .iter()
        .map(|record| record.results.len())
        .sum::<usize>();
    drop(searches);

    let shares = state.shares.read().await;
    let shared_file_count = shares.entries.len();
    let shared_bytes = shares.entries.iter().map(|entry| entry.size).sum::<u64>();
    drop(shares);

    let events = state.events.read().await;
    let completed_backfills = events
        .records
        .iter()
        .filter(|event| event.kind.contains("backfill") || event.kind.contains("hashdb"))
        .count();
    drop(events);

    let users = state.users.read().await;
    let mesh = state.mesh.read().await;
    let candidate_usernames = mesh.candidate_usernames(&users);
    let discovered_peers = if include_peers {
        mesh.capability_records_json()
    } else {
        Vec::new()
    };
    let mesh_peers = if include_peers {
        candidate_usernames
            .iter()
            .map(|username| {
                serde_json::json!({
                    "username": username,
                    "meshCapable": mesh.capability_records.iter().any(|descriptor| {
                        descriptor.username.eq_ignore_ascii_case(username)
                            && MeshRendezvous::accepts_descriptor(descriptor)
                    }),
                })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let capability_record_count = mesh.capability_records.len();
    drop(mesh);
    drop(users);

    let transfers = state.transfers.read().await;
    let swarm_jobs = transfers
        .entries
        .iter()
        .filter(|entry| entry.status == "in_progress")
        .map(|entry| {
            let total_bytes = entry.size.unwrap_or_default();
            let progress_percent = if total_bytes == 0 {
                0.0
            } else {
                entry.bytes_transferred as f64 * 100.0 / total_bytes as f64
            };
            serde_json::json!({
                "activeSources": usize::from(entry.peer_username.is_some()),
                "downloadedBytes": entry.bytes_transferred,
                "filename": entry.filename,
                "jobId": format!("transfer-{}", entry.id),
                "progressPercent": progress_percent,
                "totalBytes": total_bytes,
            })
        })
        .collect::<Vec<_>>();
    drop(transfers);

    let queued_backfills = search_count + search_result_count + shared_file_count;
    let now = unix_timestamp();
    serde_json::json!({
        "backfill": {
            "isActive": queued_backfills > 0,
            "isRunning": false,
            "queued": queued_backfills,
            "completed": completed_backfills,
            "searches": search_count,
            "searchResults": search_result_count,
            "sharedFiles": shared_file_count,
        },
        "capabilitiesJson": capabilities_response().body,
        "capabilitiesVersion": APP_VERSION,
        "dht": {
            "dhtNodeCount": 0,
            "isLanOnly": false,
            "lanOnly": false,
            "isBeaconCapable": false,
            "isDhtRunning": false,
            "verifiedBeaconCount": 0,
        },
        "discoveredPeers": discovered_peers,
        "hashDb": {
            "currentSeqId": now,
            "totalHashEntries": shared_file_count,
            "totalEntries": shared_file_count,
            "totalBytes": shared_bytes,
        },
        "mesh": {
            "currentSeqId": now,
            "localSeqId": now,
            "isSyncing": false,
            "knownMeshPeers": candidate_usernames.len(),
            "connectedPeerCount": 0,
            "capabilityRecords": capability_record_count,
            "interestTag": MESH_RENDEZVOUS_INTEREST_TAG,
            "warnings": [],
        },
        "meshPeers": mesh_peers,
        "swarmJobs": swarm_jobs,
        "transport": {
            "status": "Healthy",
            "health": "Healthy",
            "description": "Mesh transport unavailable in this runtime",
            "transportPreference": "Auto",
            "connectedPeers": 0,
            "totalPeers": 0,
            "activeCircuits": 0,
            "activeStreams": 0,
            "bootstrapPeers": [],
            "isolatedPeers": 0,
            "quorumPeers": 0,
            "relayedPeers": 0,
            "natType": "Unknown",
            "publicEndpoint": null,
            "lastDhtError": null,
            "lastDhtPublishUtc": null,
        },
    })
}

pub(super) fn capabilities_negotiate_response(body: &str) -> HttpResponse {
    if json_array_field_exceeds_limit(body, "capabilities", MAX_CAPABILITY_NEGOTIATION_ITEMS) {
        return routing::bad_request_response("capabilities negotiation exceeds item limits");
    }
    let server_capabilities = ["shares", "telemetry"];

    // Parse requested capabilities from body
    let requested = extract_json_string_array_field(body, "capabilities").unwrap_or_default();

    // Compute intersection
    let mut accepted = Vec::new();
    let mut unsupported = Vec::new();

    for req_cap in requested {
        if server_capabilities.contains(&req_cap.as_str()) {
            accepted.push(req_cap);
        } else {
            unsupported.push(req_cap);
        }
    }

    let response_body = serde_json::json!({
        "accepted": accepted,
        "unsupported": unsupported,
        "server_capabilities": server_capabilities,
    })
    .to_string();

    HttpResponse {
        status: "200 OK",
        content_type: "application/json",
        body: response_body,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedCapabilityDescription {
    flags: i32,
    client_version: String,
    protocol_version: i32,
}

const CAPABILITY_SUPPORTS_DHT: i32 = 1 << 0;
const CAPABILITY_SUPPORTS_HASH_EXCHANGE: i32 = 1 << 1;
const CAPABILITY_SUPPORTS_PARTIAL_DOWNLOAD: i32 = 1 << 2;
const CAPABILITY_SUPPORTS_MESH_SYNC: i32 = 1 << 3;
const CAPABILITY_SUPPORTS_FLAC_HASH_DB: i32 = 1 << 4;
const CAPABILITY_SUPPORTS_SWARM: i32 = 1 << 5;

fn capability_flag_value(name: &str) -> i32 {
    match name.to_ascii_lowercase().as_str() {
        "dht" => CAPABILITY_SUPPORTS_DHT,
        "hashx" => CAPABILITY_SUPPORTS_HASH_EXCHANGE,
        "partial" => CAPABILITY_SUPPORTS_PARTIAL_DOWNLOAD,
        "mesh" => CAPABILITY_SUPPORTS_MESH_SYNC,
        "flacdb" => CAPABILITY_SUPPORTS_FLAC_HASH_DB,
        "swarm" => CAPABILITY_SUPPORTS_SWARM,
        _ => 0,
    }
}

fn capability_flags_from_features(features: &[String]) -> i32 {
    features.iter().fold(0, |flags, feature| {
        let flag = match feature.to_ascii_lowercase().as_str() {
            "dht" | "dht_hash_db" => CAPABILITY_SUPPORTS_DHT,
            "hashx" | "hash_exchange" => CAPABILITY_SUPPORTS_HASH_EXCHANGE,
            "partial" | "partial_download" => CAPABILITY_SUPPORTS_PARTIAL_DOWNLOAD,
            "mesh" | "mesh_sync" | slskr_client::capabilities::FEATURE_MESH_V1 => {
                CAPABILITY_SUPPORTS_MESH_SYNC
            }
            "flacdb" | "flac_hash_db" => CAPABILITY_SUPPORTS_FLAC_HASH_DB,
            "swarm" | "swarm_download" => CAPABILITY_SUPPORTS_SWARM,
            _ => 0,
        };
        flags | flag
    })
}

fn capability_flags_from_tag(flags: &str) -> i32 {
    flags
        .split(';')
        .filter_map(|part| {
            let mut fields = part.split('=');
            let name = fields.next()?;
            let value = fields.next()?;
            (fields.next().is_none() && value == "1").then(|| capability_flag_value(name))
        })
        .fold(0, |flags, flag| flags | flag)
}

fn capability_flags_from_version(tokens: &str) -> i32 {
    tokens
        .split('+')
        .filter(|token| !token.is_empty())
        .map(capability_flag_value)
        .fold(0, |flags, flag| flags | flag)
}

fn capability_flags_string(flags: i32) -> String {
    let names = [
        (CAPABILITY_SUPPORTS_DHT, "SupportsDHT"),
        (CAPABILITY_SUPPORTS_HASH_EXCHANGE, "SupportsHashExchange"),
        (
            CAPABILITY_SUPPORTS_PARTIAL_DOWNLOAD,
            "SupportsPartialDownload",
        ),
        (CAPABILITY_SUPPORTS_MESH_SYNC, "SupportsMeshSync"),
        (CAPABILITY_SUPPORTS_FLAC_HASH_DB, "SupportsFlacHashDb"),
        (CAPABILITY_SUPPORTS_SWARM, "SupportsSwarm"),
    ];
    if flags == 0 {
        return "None".to_owned();
    }
    names
        .into_iter()
        .filter_map(|(flag, name)| (flags & flag != 0).then_some(name))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn capability_service_peer_json(
    descriptor: &slskr_client::capabilities::PeerCapabilityDescriptor,
) -> serde_json::Value {
    let flags = capability_flags_from_features(&descriptor.features);
    serde_json::json!({
        "username": descriptor.username,
        "flags": capability_flags_string(flags),
        "flagsValue": flags,
        // The capability controller describes the native slskdN-compatible
        // capability envelope, not the unrelated UserInfo version-string
        // parser or this replacement's internal package name.
        "clientVersion": "slskdn/runtime-capability-v1",
        "protocolVersion": 1,
        "canSwarm": flags & CAPABILITY_SUPPORTS_SWARM != 0,
        "canMeshSync": flags & CAPABILITY_SUPPORTS_MESH_SYNC != 0,
        "lastSeen": unix_seconds_rfc3339(descriptor.issued_at_unix),
        "meshSeqId": 0,
    })
}

pub(super) fn persisted_capability_descriptor(
    value: &serde_json::Value,
) -> Option<PeerCapabilityDescriptor> {
    let decode = |name: &str| STANDARD.decode(value.get(name)?.as_str()?.as_bytes()).ok();
    let public_key = <[u8; 32]>::try_from(decode("publicKey")?.as_slice()).ok()?;
    let signature = value
        .get("signature")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .and_then(|_| <[u8; 64]>::try_from(decode("signature")?.as_slice()).ok());
    let features = value
        .get("features")?
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    let endpoints = value
        .get("endpoints")?
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    Some(PeerCapabilityDescriptor {
        peer_id: value.get("peerId")?.as_str()?.to_owned(),
        username: value.get("username")?.as_str()?.to_owned(),
        features,
        endpoints,
        overlay_port: value
            .get("overlayPort")
            .and_then(serde_json::Value::as_u64)
            .and_then(|port| u16::try_from(port).ok()),
        max_payload_length: value
            .get("maxPayloadLength")
            .and_then(serde_json::Value::as_u64)
            .and_then(|length| u32::try_from(length).ok())?,
        issued_at_unix: value.get("issuedAtUnix")?.as_u64()?,
        expires_at_unix: value.get("expiresAtUnix")?.as_u64()?,
        public_key,
        signature,
    })
}

fn parse_capability_tag(description: &str) -> Option<ParsedCapabilityDescription> {
    let matcher = fancy_regex::Regex::new(r"(?i)slskr_caps:v(\d+);?(.*)").ok()?;
    let captures = matcher.captures(description).ok()??;
    let protocol_version = captures.get(1)?.as_str().parse::<i32>().ok()?;
    let flags = captures
        .get(2)
        .map(|value| capability_flags_from_tag(value.as_str()))
        .unwrap_or_default();
    Some(ParsedCapabilityDescription {
        flags,
        client_version: String::new(),
        protocol_version,
    })
}

fn parse_capability_version(version: &str) -> Option<ParsedCapabilityDescription> {
    let matcher = fancy_regex::Regex::new(r"(?i)(?:slskr|slskdn)/([^+\s]+)(\+.*)?").ok()?;
    let captures = matcher.captures(version).ok()??;
    let client_version = captures.get(1)?.as_str().to_owned();
    let flags = captures
        .get(2)
        .map(|value| capability_flags_from_version(value.as_str()))
        .unwrap_or_default();
    Some(ParsedCapabilityDescription {
        flags,
        client_version,
        protocol_version: 1,
    })
}

pub(super) fn capabilities_parse_response(body: &str) -> HttpResponse {
    let payload = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(payload) => payload,
        Err(_) => return routing::bad_request_response("invalid JSON body"),
    };
    let description = payload
        .get("description")
        .or_else(|| payload.get("Description"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let version = payload
        .get("versionString")
        .or_else(|| payload.get("VersionString"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let parsed = (!description.is_empty())
        .then(|| parse_capability_tag(description))
        .flatten()
        .or_else(|| {
            (!version.is_empty())
                .then(|| parse_capability_version(version))
                .flatten()
        });
    let Some(parsed) = parsed else {
        return routing::ok_response(serde_json::json!({"isSlskdn": false}).to_string());
    };
    routing::ok_response(
        serde_json::json!({
            "isSlskdn": true,
            "flags": capability_flags_string(parsed.flags),
            "flagsValue": parsed.flags,
            "protocolVersion": parsed.protocol_version,
            "clientVersion": parsed.client_version,
            "canSwarm": parsed.flags & CAPABILITY_SUPPORTS_SWARM != 0,
            "canMeshSync": parsed.flags & CAPABILITY_SUPPORTS_MESH_SYNC != 0,
        })
        .to_string(),
    )
}

pub(super) fn new_capability_signing_key() -> Result<SigningKey, String> {
    let mut secret = [0_u8; 32];
    SysRng
        .try_fill_bytes(&mut secret)
        .map_err(|_| "secure randomness unavailable for peer capability identity".to_owned())?;
    Ok(SigningKey::from_bytes(&secret))
}

pub(super) fn load_or_create_capability_signing_key(
    state_dir: &Path,
) -> Result<SigningKey, String> {
    let path = state_dir.join("peer-capability-key.bin");
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err("peer capability key must be a regular file".to_owned());
            }
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
            }
            let mut file = options
                .open(&path)
                .map_err(|error| format!("peer capability key read failed: {error}"))?;
            let opened_metadata = file
                .metadata()
                .map_err(|error| format!("peer capability key metadata failed: {error}"))?;
            if !opened_metadata.is_file() {
                return Err("peer capability key must be a regular file".to_owned());
            }
            let mut bytes = Vec::with_capacity(33);
            let mut limited = std::io::Read::take(&mut file, 33);
            std::io::Read::read_to_end(&mut limited, &mut bytes)
                .map_err(|error| format!("peer capability key read failed: {error}"))?;
            let secret: [u8; 32] = bytes
                .try_into()
                .map_err(|_| "peer capability key must contain exactly 32 bytes".to_owned())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))
                    .map_err(|error| format!("peer capability key permissions failed: {error}"))?;
            }
            Ok(SigningKey::from_bytes(&secret))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let signing_key = new_capability_signing_key()?;
            let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4().simple()));
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .map_err(|error| format!("peer capability key creation failed: {error}"))?;
            std::io::Write::write_all(&mut file, &signing_key.to_bytes())
                .map_err(|error| format!("peer capability key write failed: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("peer capability key sync failed: {error}"))?;
            fs::rename(&temporary, &path)
                .map_err(|error| format!("peer capability key publish failed: {error}"))?;
            Ok(signing_key)
        }
        Err(error) => Err(format!("peer capability key metadata failed: {error}")),
    }
}

pub(super) async fn local_capability_descriptor(
    state: &AppState,
) -> Result<PeerCapabilityDescriptor, String> {
    let mut features = vec![FEATURE_CAPABILITIES_V1.to_owned()];
    let publish_availability = state
        .media_services
        .read()
        .await
        .features
        .mesh_publish_availability;
    if publish_availability && state.private_gateway.is_some() {
        features.push("mesh_sync".to_owned());
    }
    PeerCapabilityDescriptor::unsigned(
        pod_request_peer_id(state)
            .await
            .unwrap_or_else(|| "slskr".to_owned()),
        features,
        Vec::new(),
        std::time::Duration::from_secs(24 * 60 * 60),
        &state.capability_signing_key,
        std::time::SystemTime::now(),
    )
    .map(|descriptor| {
        descriptor.with_overlay_port(if publish_availability {
            state
                .private_gateway
                .as_ref()
                .map(|gateway| gateway.bind().port())
        } else {
            None
        })
    })
    .and_then(|descriptor| descriptor.sign(&state.capability_signing_key))
    .map_err(|error| format!("local peer capability descriptor failed: {error}"))
}

pub(super) fn local_profile_peer_id(state: &AppState) -> String {
    mesh_dht::peer_id(&state.capability_signing_key)
}

pub(super) fn profile_friend_code(peer_id: &str) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let digest = Sha256::digest(peer_id.as_bytes());
    let mut encoded = String::with_capacity(16);
    let mut bits = 0_u16;
    let mut bit_count = 0_u8;
    for byte in digest.iter().take(10) {
        bits = (bits << 8) | u16::from(*byte);
        bit_count += 8;
        while bit_count >= 5 {
            bit_count -= 5;
            encoded.push(ALPHABET[((bits >> bit_count) & 31) as usize] as char);
        }
    }
    if bit_count > 0 {
        encoded.push(ALPHABET[((bits << (5 - bit_count)) & 31) as usize] as char);
    }
    format!(
        "{}-{}-{}-{}",
        &encoded[..5],
        &encoded[5..9],
        &encoded[9..13],
        &encoded[13..16]
    )
}
