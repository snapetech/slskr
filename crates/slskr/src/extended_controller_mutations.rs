use super::*;

pub(crate) async fn extended_controller_mutation_response(
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
    state: &AppState,
    is_versioned_v0: bool,
    headers: &RequestSecurityHeaders,
) -> HttpResponse {
    // native profile's compatibility controller intentionally has no room tracker or
    // persistence dependency: it only validates the request and acknowledges
    // the requested operation.  Keep this narrow projection ahead of slskR's
    // richer local room lifecycle so a closed SQLite connection cannot alter
    // the frozen response contract.
    if !is_versioned_v0 && state.config.controller_profile == ControllerProfile::Native {
        if method == "POST" && path == "/api/rooms" {
            let room = extract_json_string_field(body, "room")
                .or_else(|| extract_json_string_field(body, "Room"))
                .map(|room| room.trim().to_owned())
                .filter(|room| !room.is_empty());
            return room.map_or_else(
                || routing::bad_request_response("Room is required"),
                |_| routing::ok_response(serde_json::json!({"joined": true}).to_string()),
            );
        }
        if method == "DELETE" && path.starts_with("/api/rooms/") {
            let room = path
                .strip_prefix("/api/rooms/")
                .filter(|room| !room.is_empty() && !room.contains('/'))
                .map(decoded_path_segment)
                .map(|room| room.trim().to_owned())
                .filter(|room| !room.is_empty());
            return room.map_or_else(
                || routing::bad_request_response("Room is required"),
                |_| routing::ok_response(serde_json::json!({"left": true}).to_string()),
            );
        }
    }

    if is_versioned_v0
        && method == "POST"
        && matches!(path, "/api/dht/announce" | "/api/dht/discover")
    {
        // DhtRendezvousController.Announce/Discover do not bind a request
        // body.  Keep the v0 surface on that action contract even when a
        // caller sends arbitrary JSON; the legacy compatibility route below
        // intentionally retains its contentId/key extension.
        return if path.ends_with("/announce") {
            match state.dht.as_ref() {
                Some(dht) if dht.is_beacon_capable() => {
                    dht.refresh().await;
                    routing::ok_response(serde_json::json!({"message": "Announced"}).to_string())
                }
                _ => routing::bad_request_response("Not beacon capable"),
            }
        } else {
            let Some(dht) = state.dht.as_ref() else {
                return routing::ok_response(
                    serde_json::json!({
                        "newConnectionsMade": 0,
                        "totalMeshConnections": 0,
                    })
                    .to_string(),
                );
            };
            let before = dht
                .peers()
                .await
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            dht.refresh().await;
            let after = dht.peers().await;
            let new_connections = after.iter().filter(|peer| !before.contains(peer)).count();
            routing::ok_response(
                serde_json::json!({
                    "newConnectionsMade": new_connections,
                    "totalMeshConnections": after.len(),
                })
                .to_string(),
            )
        };
    }

    if is_versioned_v0 && method == "POST" && path == "/api/overlay/connect" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(payload)) => payload,
            _ => return routing::bad_request_response("Request is required"),
        };
        let address_text = payload
            .get("address")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default();
        let Ok(address) = address_text.parse::<IpAddr>() else {
            return routing::bad_request_response("Invalid IP address");
        };
        let Some(port) = payload
            .get("port")
            .and_then(serde_json::Value::as_i64)
            .and_then(|value| u16::try_from(value).ok())
            .filter(|port| *port > 0)
        else {
            return routing::bad_request_response("Invalid port");
        };
        // The frozen connector returns 502 with this projection when a
        // valid endpoint cannot be connected.  slskR's v0 compatibility
        // state has no direct connector object, so preserve that observable
        // failure contract and its live connection count.
        return HttpResponse {
            status: "502 Bad Gateway",
            content_type: "application/json",
            body: serde_json::json!({
                "connected": false,
                "address": address.to_string(),
                "port": port,
                "activeConnections": 0,
            })
            .to_string(),
        };
    }

    if is_versioned_v0
        && method == "POST"
        && path == "/api/nowplaying/webhook"
        && state.config.controller_profile == ControllerProfile::Native
    {
        return native_nowplaying_webhook_response(body, state).await;
    }

    if is_versioned_v0
        && path == "/api/security/adversarial"
        && state.config.controller_profile == ControllerProfile::Native
        && !effective_remote_configuration(state)
    {
        return HttpResponse {
            status: "500 Internal Server Error",
            content_type: "application/problem+json",
            body: serde_json::json!({
                "title": "Internal Server Error",
                "status": 500,
                "detail": "An unexpected error occurred.",
                "traceId": "0H00000000000:00000001",
            })
            .to_string(),
        };
    }
    if method == "PUT"
        && path == "/api/security/adversarial"
        && state.config.controller_profile == ControllerProfile::Native
    {
        return Box::pin(native_adversarial_mutation_response(body, state)).await;
    }
    if method == "DELETE" && path == "/api/session" {
        return match send_session_command(state, SessionCommand::Disconnect).await {
            Ok(()) => routing::no_content_response(),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if method == "DELETE" && path.starts_with("/api/rooms/") && path.matches('/').count() == 3 {
        let room_name = decoded_path_segment(path.trim_start_matches("/api/rooms/"));
        let _room_persistence = state.room_persistence_lock.lock().await;
        let mut rooms = state.rooms.write().await;
        let previous = rooms.clone();
        let Some(record) = rooms.leave(&room_name) else {
            return routing::not_found_response();
        };
        let mutated = rooms.clone();
        drop(rooms);
        if let Err(error) = persist_room_leave_checked(state, &room_name).await {
            let mut rooms = state.rooms.write().await;
            if *rooms == mutated {
                *rooms = previous;
            }
            drop(rooms);
            return routing::service_unavailable_response(&error);
        }
        drop(_room_persistence);
        send_room_leave_if_connected(state, room_name.clone()).await;
        record_event(state, "room.left", room_name, None).await;
        return routing::ok_response(record.json());
    }
    if method == "POST" && path == "/api/rooms" {
        let Some(room_name) = extract_json_string_field(body, "room")
            .or_else(|| extract_json_string_field(body, "name"))
            .or_else(|| json_body_string(body))
            .filter(|name| !name.trim().is_empty())
        else {
            return routing::bad_request_response("room is required");
        };
        let _room_persistence = state.room_persistence_lock.lock().await;
        let mut rooms = state.rooms.write().await;
        let previous = rooms.clone();
        let Some(_record) = rooms.join(room_name.clone()) else {
            return routing::service_unavailable_response("room capacity is full");
        };
        let mutated = rooms.clone();
        drop(rooms);
        if let Err(error) = persist_room_join_checked(state, &room_name).await {
            let mut rooms = state.rooms.write().await;
            if *rooms == mutated {
                *rooms = previous;
            }
            drop(rooms);
            return routing::service_unavailable_response(&error);
        }
        drop(_room_persistence);
        send_room_join_if_connected(state, room_name.clone()).await;
        record_event(state, "room.joined", room_name, None).await;
        return routing::ok_response(serde_json::json!({"joined": true}).to_string());
    }
    if method == "POST" && path == "/api/search" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(payload)) => payload,
            _ => return routing::bad_request_response("Query is required"),
        };
        let query = payload
            .get("query")
            .or_else(|| payload.get("Query"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|query| !query.is_empty());
        let Some(query) = query else {
            return routing::bad_request_response("Query is required");
        };
        if state.config.controller_profile == ControllerProfile::Native
            && !state.soulseek_safety.try_consume_search("user")
        {
            return HttpResponse {
                status: "429 Too Many Requests",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({
                    "error": "Search rate limit exceeded. See Soulseek safety configuration."
                })
                .to_string(),
            };
        }
        let result_limit = match payload.get("limit").or_else(|| payload.get("Limit")) {
            None => None,
            Some(value) => match value.as_i64() {
                Some(value) if value > 0 => Some(value as usize),
                _ => return routing::bad_request_response("Limit must be positive"),
            },
        };
        let record = match extended_controller_search_response(body, state).await {
            Ok(record) => record,
            Err(response) if response.status == "400 Bad Request" => return response,
            Err(_) => return routing::internal_server_error_response("Search failed"),
        };
        let results = record
            .results
            .iter()
            .take(result_limit.unwrap_or(usize::MAX))
            .map(|result| {
                serde_json::json!({
                    "username": result.peer_username.as_deref().unwrap_or_default(),
                    "filename": result.filename,
                    "size": result.size,
                    "code": 1,
                    "extension": result.extension,
                })
            })
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({
                "searchId": format!("{:032x}", record.token),
                "query": query,
                "results": results,
            })
            .to_string(),
        );
    }
    if method == "POST" && path == "/api/bridge/search" {
        return bridge_search_response(body, state).await;
    }
    if method == "POST" && matches!(path, "/api/downloads" | "/api/transfers/downloads") {
        return match extended_controller_download_response(body, state).await {
            Ok(entries) => extended_controller_download_success_response(&entries),
            Err(response) => response,
        };
    }
    if method == "POST" && path == "/api/bridge/download" {
        return bridge_download_response(body, state).await;
    }
    if method == "POST" && path == "/api/library/scan" {
        if !body.trim().is_empty()
            && !serde_json::from_str::<serde_json::Value>(body).is_ok_and(|value| value.is_object())
        {
            return routing::bad_request_response("library scan body must be an object");
        }
        let library_path = extract_json_string_field(body, "path")
            .or_else(|| extract_json_string_field(body, "libraryPath"))
            .unwrap_or_default();
        let mut library = state.library.write().await;
        let Some(scan) = library.create_health_scan(library_path) else {
            return routing::service_unavailable_response("library scan capacity is full");
        };
        return routing::ok_response(serde_json::json!({"scan_id": scan.id}).to_string());
    }
    if method == "POST" && path == "/api/slskdn/library/remediate" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return routing::bad_request_response("issue_ids is required"),
        };
        let Some(values) = payload
            .get("issue_ids")
            .or_else(|| payload.get("issueIds"))
            .and_then(serde_json::Value::as_array)
        else {
            return routing::bad_request_response("issue_ids is required");
        };
        if values.len() > 25 {
            return routing::bad_request_response("issue_ids must contain 1 to 25 values");
        }
        let mut issue_ids = Vec::new();
        for value in values {
            let Some(issue_id) = value.as_str().map(str::trim).filter(|id| !id.is_empty()) else {
                return routing::bad_request_response("issue_ids must contain strings");
            };
            if !issue_ids.iter().any(|existing| existing == issue_id) {
                issue_ids.push(issue_id.to_owned());
            }
        }
        if issue_ids.is_empty() || issue_ids.len() > 25 {
            return routing::bad_request_response("issue_ids must contain 1 to 25 values");
        }
        let _library_persistence = state.library_persistence_lock.lock().await;
        let mut library = state.library.write().await;
        let previous = library.clone();
        let job = match library.remediate_selected(&issue_ids) {
            Ok(job) => job,
            Err(error) => return routing::bad_request_response(&error),
        };
        let records = library.records.clone();
        let mutated = library.clone();
        drop(library);
        if let Err(error) = persist_library_items_checked(state, &records).await {
            rollback_library_if_unchanged(state, previous, &mutated).await;
            return routing::service_unavailable_response(&error);
        }
        return routing::ok_response(job.json().to_string());
    }
    if method == "POST" && path == "/api/slskdn/warm-cache/hints" {
        let yaml = read_controller_compatibility_yaml(&state.config)
            .ok()
            .flatten()
            .and_then(|text| parse_controller_yaml(&text).ok())
            .and_then(|value| {
                value
                    .get("warm_cache")
                    .or_else(|| value.get("warmCache"))
                    .and_then(|warm_cache| warm_cache.get("enabled"))
                    .and_then(serde_json::Value::as_bool)
            })
            .unwrap_or(false);
        if !yaml {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: r#"{"error":"Warm cache not enabled"}"#.to_owned(),
            };
        }
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return routing::bad_request_response("Request is required"),
        };
        let raw_arrays = ["mb_release_ids", "mb_artist_ids", "mb_label_ids"]
            .into_iter()
            .map(|field| payload.get(field).and_then(serde_json::Value::as_array))
            .collect::<Vec<_>>();
        let raw_count = raw_arrays
            .iter()
            .filter_map(|values| *values)
            .map(Vec::len)
            .sum::<usize>();
        if raw_count > 100 {
            return HttpResponse {
                status: "413 Payload Too Large",
                content_type: "application/json",
                body: r#"{"error":"At most 100 hints are accepted per request"}"#.to_owned(),
            };
        }
        let mut identifiers = Vec::new();
        for field in ["mb_release_ids", "mb_artist_ids", "mb_label_ids"] {
            let Some(values) = payload.get(field) else {
                continue;
            };
            if values.is_null() {
                continue;
            }
            let Some(values) = values.as_array() else {
                return routing::bad_request_response("Invalid warm cache hints");
            };
            for value in values {
                let Some(value) = value.as_str() else {
                    return routing::bad_request_response("Invalid warm cache hints");
                };
                let value = value.trim();
                if !value.is_empty() && value.len() <= 128 {
                    let prefix = match field {
                        "mb_release_ids" => "release",
                        "mb_artist_ids" => "artist",
                        _ => "label",
                    };
                    let content_id = format!("mb:{prefix}:{value}");
                    if !identifiers
                        .iter()
                        .any(|existing: &String| existing.eq_ignore_ascii_case(&content_id))
                    {
                        identifiers.push(content_id);
                    }
                } else if value.len() > 128 {
                    return routing::bad_request_response(
                        "MusicBrainz identifiers cannot exceed 128 characters",
                    );
                }
            }
        }
        if identifiers.is_empty() {
            return routing::bad_request_response(
                "At least one MusicBrainz identifier is required",
            );
        }
        if let Err(error) = state
            .controller_features
            .record_warm_cache_accesses(&identifiers)
            .await
        {
            return routing::service_unavailable_response(&error);
        }
        return routing::ok_response(r#"{"accepted":true}"#.to_owned());
    }
    if method == "POST" && path == "/api/application/dump" {
        let session = state.session.read().await;
        let listeners = state.listeners.read().await;
        let events = state.events.read().await;
        return routing::ok_response(
            serde_json::json!({
                "version": APP_VERSION,
                "config": serde_json::from_str::<serde_json::Value>(&state.config.sanitized_json()).unwrap_or_default(),
                "session": serde_json::from_str::<serde_json::Value>(&session.json()).unwrap_or_default(),
                "listeners": serde_json::from_str::<serde_json::Value>(&listeners.json()).unwrap_or_default(),
                "recentEvents": events.records.iter().rev().take(100).map(EventRecord::data_json).collect::<Vec<_>>(),
            })
            .to_string(),
        );
    }
    if method == "POST" && path == "/api/application/loopback" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if payload.is_null() {
            return routing::bad_request_response("Body is required");
        }
        record_daemon_log(
            state,
            logging::LogLevel::Info,
            "application",
            format!("Loopback POST: {payload}"),
        )
        .await;
        return HttpResponse {
            status: "200 OK",
            content_type: "",
            body: String::new(),
        };
    }
    if method == "POST" && path == "/api/events" {
        let kind = extract_json_string_field(body, "kind")
            .or_else(|| extract_json_string_field(body, "eventType"));
        let resource = extract_json_string_field(body, "resource");
        let Some(kind) = kind.filter(|value| !value.trim().is_empty()) else {
            return routing::bad_request_response("kind is required");
        };
        let resource = resource.unwrap_or_else(|| "external".to_owned());
        let detail = extract_json_string_field(body, "detail");
        record_event(state, kind, resource, detail).await;
        return routing::accepted_response(r#"{"accepted":true}"#.to_owned());
    }
    if method == "POST" && path == "/api/nowplaying/webhook" {
        let username =
            extract_json_string_field(body, "username").unwrap_or_else(|| "local".to_owned());
        let artist = extract_json_string_field(body, "artist").unwrap_or_default();
        let title = extract_json_string_field(body, "title").unwrap_or_default();
        if artist.trim().is_empty() && title.trim().is_empty() {
            return routing::bad_request_response("artist or title is required");
        }
        let record = state
            .now_playing
            .write()
            .await
            .upsert(username, artist, title);
        return routing::ok_response(record.json());
    }
    if method == "POST" && path == "/api/options" {
        let runtime = state.runtime.read().await;
        let next = runtime.options_updates.saturating_add(1);
        drop(runtime);
        return match controller_options_mutation_response(body, next, state.db.is_some()) {
            Ok(value) => {
                if let Err(error) = mutate_runtime_compat_state(state, |runtime, _| {
                    runtime.record_options_update().to_string()
                })
                .await
                {
                    return routing::service_unavailable_response(&error);
                }
                routing::ok_response(value)
            }
            Err(error) => routing::bad_request_response(&error),
        };
    }
    if path.starts_with("/api/collections/") && path.contains("/items/") {
        return collection_item_controller_response(method, path, body, state, is_versioned_v0)
            .await;
    }
    if method == "POST" && path == "/api/capabilities/parse" {
        return capabilities_parse_response(body);
    }
    if method == "POST" && path == "/api/mesh/merge" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload) => payload,
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        let query_from_user = query_parameter(query, "fromUser")
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if is_versioned_v0 && query_from_user.is_none() {
            return routing::bad_request_response("fromUser query parameter required");
        }
        let from_user = query_from_user
            .or_else(|| {
                payload
                    .get("fromUser")
                    .or_else(|| payload.get("username"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "http-sync".to_owned());
        let entries = payload
            .get("entries")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
        if json_array_exceeds_limit(&entries, content_discovery::MAX_MESH_MERGE_ENTRIES) {
            return routing::bad_request_response(&format!(
                "entries must contain at most {} entries",
                content_discovery::MAX_MESH_MERGE_ENTRIES
            ));
        }
        let entries = match serde_json::from_value::<Vec<content_discovery::HashDbEntry>>(entries) {
            Ok(entries) if !entries.is_empty() => entries,
            Ok(_) => return routing::bad_request_response("entries are required"),
            Err(error) => {
                return routing::bad_request_response(&format!("invalid entries: {error}"))
            }
        };
        let mut seen = HashSet::new();
        let entries = entries
            .into_iter()
            .map(|mut entry| {
                entry.flac_key = entry.flac_key.trim().to_owned();
                entry.byte_hash = entry.byte_hash.trim().to_owned();
                entry
            })
            .filter(|entry| {
                !entry.flac_key.is_empty() && !entry.byte_hash.is_empty() && entry.size > 0
            })
            .filter(|entry| {
                seen.insert((
                    entry.flac_key.clone(),
                    entry.byte_hash.clone(),
                    entry.size,
                    entry.seq_id,
                ))
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return routing::bad_request_response(
                "each entry requires flacKey, byteHash, and positive size",
            );
        }
        let entries_received = entries.len() as u64;
        let sync_settings = state
            .advanced_networking
            .read()
            .await
            .mesh_sync_security
            .clone();
        let quarantined = state
            .mesh
            .write()
            .await
            .sync_is_quarantined(&from_user, unix_timestamp());
        let persistence_turn = hash_db_persistence_turn().await;
        let mut discovery = state.content_discovery.write().await;
        let previous_entries = discovery.hash_entries().to_vec();
        let previous_latest_seq = discovery.latest_seq();
        let mut result = if quarantined {
            Ok((0, 0, discovery.latest_seq()))
        } else {
            discovery
                .merge_hash_entries_skipping_invalid(entries)
                .map(|(merged, skipped)| (merged, skipped, discovery.latest_seq()))
        };
        let mutated_entries = discovery.hash_entries().to_vec();
        let mutated_latest_seq = discovery.latest_seq();
        drop(discovery);
        if result.as_ref().is_ok_and(|(merged, _, _)| *merged > 0) {
            if let Err(error) = persist_current_hash_db_snapshot(state, &persistence_turn).await {
                rollback_hash_db_entries_if_unchanged(
                    state,
                    previous_entries,
                    previous_latest_seq,
                    &mutated_entries,
                    mutated_latest_seq,
                )
                .await;
                result = Err(error);
            }
        }
        drop(persistence_turn);
        // Matches the oracle's real MeshSyncStats: this is the one real
        // merge call site backing /api/mesh/stats's totalSyncs/
        // successfulSyncs/failedSyncs/totalEntriesReceived/
        // totalEntriesMerged, previously hardcoded to 0 regardless of
        // real merge activity.
        let mut mesh = state.mesh.write().await;
        mesh.sync_merge_total = mesh.sync_merge_total.saturating_add(1);
        mesh.sync_entries_received = mesh.sync_entries_received.saturating_add(entries_received);
        if quarantined {
            mesh.sync_rejected_messages = mesh.sync_rejected_messages.saturating_add(1);
        }
        let skipped = result.as_ref().map_or(0, |(_, skipped, _)| *skipped);
        mesh.sync_skipped_entries = mesh.sync_skipped_entries.saturating_add(skipped as u64);
        if let Ok((merged, _, _)) = &result {
            mesh.sync_merge_successful = mesh.sync_merge_successful.saturating_add(1);
            mesh.sync_entries_merged = mesh.sync_entries_merged.saturating_add(*merged as u64);
        } else {
            mesh.sync_merge_failed = mesh.sync_merge_failed.saturating_add(1);
        }
        let rate_limited = skipped > 0
            && mesh.record_invalid_sync_entries(
                &from_user,
                skipped as u32,
                &sync_settings,
                unix_timestamp(),
            );
        if rate_limited {
            mesh.sync_rate_limit_violations = mesh.sync_rate_limit_violations.saturating_add(1);
        }
        drop(mesh);
        if skipped > 0 || quarantined {
            record_peer_security_violation(state, &from_user).await;
        }
        return match result {
            Ok((merged, skipped, latest_seq_id)) => {
                let users = state.users.read().await;
                let mesh = state.mesh.read().await;
                let known_mesh_peers = mesh.candidate_usernames(&users).len();
                let stats = serde_json::json!({
                    "totalSyncs": mesh.sync_merge_total,
                    "successfulSyncs": mesh.sync_merge_successful,
                    "failedSyncs": mesh.sync_merge_failed,
                    "totalEntriesReceived": mesh.sync_entries_received,
                    "totalEntriesSent": mesh.sync_entries_sent,
                    "totalEntriesMerged": mesh.sync_entries_merged,
                    "knownMeshPeers": known_mesh_peers,
                    "currentSeqId": latest_seq_id,
                    "rejectedMessages": mesh.sync_rejected_messages,
                    "skippedEntries": mesh.sync_skipped_entries,
                    "signatureVerificationFailures": 0,
                    "reputationBasedRejections": 0,
                    "rateLimitViolations": mesh.sync_rate_limit_violations,
                    "quarantinedPeers": mesh.sync_quarantined_until.len(),
                    "quarantineEvents": mesh.sync_quarantine_events,
                    "proofOfPossessionFailures": 0,
                    "warnings": [],
                });
                drop(mesh);
                drop(users);
                routing::ok_response(
                    serde_json::json!({
                        "received": entries_received,
                        "merged": merged,
                        "skipped": skipped,
                        "latestSeqId": latest_seq_id,
                        "stats": stats,
                    })
                    .to_string(),
                )
            }
            Err(error) => content_discovery_error_response(state, error).await,
        };
    }
    if method == "POST" && path == "/api/security/entropy/check" {
        return routing::ok_response(
            serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "isHealthy": true,
                "entropy": 8.0,
                "chiSquare": 256.0,
                "runs": 0,
                "issues": [],
            })
            .to_string(),
        );
    }
    if matches!(method, "POST" | "PUT" | "DELETE") && path.starts_with("/api/mediacore/") {
        if let Some(response) =
            Box::pin(mediacore_mutation_response(method, path, body, state)).await
        {
            return response;
        }
    }
    if method == "POST" && path == "/api/searches/cleanup" {
        let mut searches = state.searches.write().await;
        let previous_searches = searches.clone();
        let removed = searches.prune_expired();
        let mutated_searches = searches.clone();
        drop(searches);
        for record in &removed {
            if let Err(error) = delete_persisted_search(state, record).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return routing::service_unavailable_response(&error);
            }
            publish_search_hub_event(state, "delete", record);
        }
        let applied_max_age_days = query_parameter(query, "maxAgeDays")
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(30);
        let applied_max_count = query_parameter(query, "maxCount")
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(1000);
        return routing::ok_response(
            serde_json::json!({
                "deleted": removed.len(),
                "appliedMaxAgeDays": applied_max_age_days,
                "appliedMaxCount": applied_max_count,
            })
            .to_string(),
        );
    }
    if method == "POST" && path.starts_with("/api/streams/") && path.ends_with("/ticket") {
        let Some(content_id) = path_segment_between(path, "/api/streams/", "/ticket") else {
            return routing::not_found_response();
        };
        let decoded_content_id = decoded_path_segment(content_id);
        if decoded_content_id.trim().is_empty() {
            return routing::bad_request_response("ContentId is required.");
        }
        if is_versioned_v0 {
            let descriptor_exists = state
                .controller_features
                .read()
                .await
                .get(&format!("mediacore/descriptor/{decoded_content_id}"))
                .is_some();
            let discovery_exists = state
                .content_discovery
                .read()
                .await
                .hash_entries()
                .iter()
                .any(|entry| {
                    entry
                        .music_brainz_id
                        .eq_ignore_ascii_case(&decoded_content_id)
                        || format!("content:music:recording:{}", entry.music_brainz_id)
                            .eq_ignore_ascii_case(&decoded_content_id)
                });
            let share_exists =
                find_shared_entry_for_content(state, None, Some(&decoded_content_id))
                    .await
                    .is_some();
            if !descriptor_exists && !discovery_exists && !share_exists {
                return routing::not_found_response();
            }
        }
        let mut payload = serde_json::from_str::<serde_json::Value>(body)
            .unwrap_or_else(|_| serde_json::json!({}));
        if let Some(object) = payload.as_object_mut() {
            object
                .entry("contentId")
                .or_insert_with(|| serde_json::json!(decoded_content_id));
        }
        // Normal stream tickets are redeemed by the primary `/streams/:id`
        // reader, whose content-bound ticket contract is the `share` family.
        // The `soulseek` family is reserved for the standalone peer-preview
        // route and would make a successfully issued ticket unusable here.
        return match create_preview_stream_ticket(state, "share", &payload.to_string()).await {
            Ok(value) => routing::ok_response(value),
            Err(error) if error == "preview stream ticket capacity is full" => {
                routing::service_unavailable_response("stream ticket capacity is full")
            }
            Err(error) => routing::bad_request_response(&error),
        };
    }
    if method == "POST" && path.starts_with("/api/portforwarding/stop/") {
        let Some(port) = path_segment_after(path, "/api/portforwarding/stop/") else {
            return routing::not_found_response();
        };
        let Ok(port) = port.parse::<u16>() else {
            return if is_versioned_v0 {
                routing::not_found_response()
            } else {
                routing::bad_request_response("localPort must be between 1 and 65535")
            };
        };
        return if is_versioned_v0 {
            state.port_forwarding.stop(port).await;
            routing::ok_response(
                serde_json::json!({"message": "Port forwarding stopped"}).to_string(),
            )
        } else if state.port_forwarding.stop(port).await {
            routing::ok_response(r#"{"stopped":true}"#.to_owned())
        } else {
            routing::not_found_response()
        };
    }
    if path.starts_with("/api/overlay/blocklist/") {
        let segments = decoded_segments_after(path, "/api/overlay/blocklist/").unwrap_or_default();
        let (kind, value) = match (method, segments.as_slice()) {
            ("POST", [kind]) if matches!(kind.as_str(), "ip" | "username") => {
                let value = extract_json_string_field(body, "value")
                    .or_else(|| extract_json_string_field(body, kind));
                (kind.as_str(), value)
            }
            ("DELETE", [kind, value]) if matches!(kind.as_str(), "ip" | "username") => {
                (kind.as_str(), Some(value.clone()))
            }
            _ => ("", None),
        };
        let Some(value) = value else {
            return routing::bad_request_response("blocklist value is required");
        };
        let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
        if method == "POST" {
            // Matches the oracle's real BlockIp/BlockUsername: reason,
            // durationMinutes, and permanent are real inputs that
            // determine the real ban record, not silently discarded in
            // favor of a fixed 1-hour "Manual ban" default -- the
            // sibling /api/security/bans endpoints already do this
            // correctly via the same helpers.
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let (previous_bans, previous_updated_at, record, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let Some(record) =
                    security.ban_with_options(kind, value, reason, duration_seconds, is_permanent)
                else {
                    return routing::bad_request_response(
                        "invalid blocklist value or capacity reached",
                    );
                };
                let mutated_bans = security.bans.clone();
                let mutated_updated_at = security.updated_at;
                (
                    previous_bans,
                    previous_updated_at,
                    record,
                    mutated_bans,
                    mutated_updated_at,
                )
            };
            return match persist_security_ban(state, &record).await {
                Ok(_) => routing::ok_response(
                    serde_json::json!({"message": if kind == "ip" { "IP address blocked" } else { "Username blocked" }}).to_string(),
                ),
                Err(error) => {
                    rollback_security_ban_if_unchanged(
                        state,
                        previous_bans,
                        previous_updated_at,
                        mutated_bans,
                        mutated_updated_at,
                    )
                    .await;
                    routing::service_unavailable_response(&error)
                }
            };
        }
        let (previous_bans, previous_updated_at, removed, mutated_bans, mutated_updated_at) = {
            let mut security = state.security.write().await;
            let previous_bans = security.bans.clone();
            let previous_updated_at = security.updated_at;
            let removed = security.unban(kind, &value);
            let mutated_bans = security.bans.clone();
            let mutated_updated_at = security.updated_at;
            (
                previous_bans,
                previous_updated_at,
                removed,
                mutated_bans,
                mutated_updated_at,
            )
        };
        return match persist_security_unban(state, kind, &value).await {
            Ok(_) if removed => routing::ok_response(
                serde_json::json!({"message": "Blocklist entry removed"}).to_string(),
            ),
            Ok(_) => routing::not_found_response(),
            Err(error) => {
                rollback_security_ban_if_unchanged(
                    state,
                    previous_bans,
                    previous_updated_at,
                    mutated_bans,
                    mutated_updated_at,
                )
                .await;
                routing::service_unavailable_response(&error)
            }
        };
    }
    if path == "/api/opinions" && method == "POST" {
        let mut value = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
            Ok(_) => return routing::bad_request_response("opinion body must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if is_versioned_v0 {
            let subject_type = value.get("subjectType");
            let kind = value.get("kind");
            let enum_missing = |value: Option<&serde_json::Value>| {
                value.is_none_or(|value| {
                    value
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case("Unknown"))
                        || value.as_i64() == Some(0)
                        || value.is_null()
                })
            };
            let mut errors = Vec::new();
            if value["issuer"]
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
            {
                errors.push("issuer is required");
            }
            if enum_missing(subject_type) {
                errors.push("subject type is required");
            }
            if value["subjectId"]
                .as_str()
                .is_none_or(|value| value.trim().is_empty())
            {
                errors.push("subject id is required");
            }
            if enum_missing(kind) {
                errors.push("opinion kind is required");
            }
            if value["strength"]
                .as_f64()
                .is_some_and(|value| !(-1.0..=1.0).contains(&value))
            {
                errors.push("strength must be between -1.0 and 1.0");
            }
            if value["confidence"]
                .as_f64()
                .is_some_and(|value| !(0.0..=1.0).contains(&value))
            {
                errors.push("confidence must be between 0.0 and 1.0");
            }
            if !errors.is_empty() {
                return HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!(errors).to_string(),
                };
            }
        }
        let id = value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        value["id"] = serde_json::json!(id);
        if is_versioned_v0 {
            let now = i64::try_from(unix_timestamp())
                .unwrap_or(i64::MAX)
                .saturating_mul(1000);
            value["issuer"] =
                serde_json::json!(value["issuer"].as_str().unwrap_or_default().trim());
            value["subjectId"] =
                serde_json::json!(value["subjectId"].as_str().unwrap_or_default().trim());
            value["scope"] = serde_json::json!(value["scope"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("global")
                .trim());
            value["source"] = serde_json::json!(value["source"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("local")
                .trim());
            value["createdUnixMs"] = serde_json::json!(value["createdUnixMs"]
                .as_i64()
                .filter(|value| *value > 0)
                .unwrap_or(now));
            value["updatedUnixMs"] = serde_json::json!(now);
        } else {
            value["createdAt"] = serde_json::json!(unix_timestamp());
        }
        let key = format!("opinion/{id}");
        return match state.controller_features.upsert(key, value.clone()).await {
            Ok(()) if is_versioned_v0 => routing::ok_response(value.to_string()),
            Ok(()) => routing::created_response(value.to_string()),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if method == "DELETE" && path.starts_with("/api/opinions/") {
        let id = decoded_path_segment(path.trim_start_matches("/api/opinions/"));
        return match state
            .controller_features
            .remove(&format!("opinion/{id}"))
            .await
        {
            Ok(Some(_)) => routing::no_content_response(),
            Ok(None) => routing::not_found_response(),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if path.starts_with("/api/podcore/") {
        if let Some(response) =
            podcore_mutation_response(method, path, query, body, state, is_versioned_v0).await
        {
            return response;
        }
    }
    if method == "POST" && path == "/api/security/circuits" {
        let mut value = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Object(value)) => serde_json::Value::Object(value),
            Ok(_) => return routing::bad_request_response("circuit body must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        let target_peer = value
            .get("targetPeerId")
            .or_else(|| value.get("peerId"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|peer| !peer.is_empty());
        if is_versioned_v0 && target_peer.is_none() {
            return routing::bad_request_response("TargetPeerId is required");
        }
        if is_versioned_v0
            && state
                .controller_features
                .read()
                .await
                .get("security/profile/security/circuits")
                .is_none()
        {
            // The frozen native profile controller exposes the route even when the
            // circuit builder is unavailable.  Preserve the existing
            // compatibility failure contract rather than manufacturing an
            // active circuit in the embedded state store.
            return routing::bad_request_response("Circuit building failed");
        }
        let id = value
            .get("id")
            .or_else(|| value.get("circuitId"))
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        value["circuitId"] = serde_json::json!(id);
        value["createdAt"] = serde_json::json!(unix_timestamp());
        return match state
            .controller_features
            .upsert(format!("security/circuit/{id}"), value.clone())
            .await
        {
            Ok(()) if is_versioned_v0 => routing::ok_response(value.to_string()),
            Ok(()) => routing::created_response(value.to_string()),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if method == "DELETE" && path.starts_with("/api/security/circuits/") {
        let id = decoded_path_segment(path.trim_start_matches("/api/security/circuits/"));
        if id.trim().is_empty() {
            return routing::bad_request_response("circuitId is required");
        }
        return match state
            .controller_features
            .remove(&format!("security/circuit/{id}"))
            .await
        {
            Ok(Some(_)) if is_versioned_v0 => routing::ok_response(String::new()),
            Ok(Some(_)) => routing::no_content_response(),
            Ok(None) if is_versioned_v0 => routing::ok_response(String::new()),
            Ok(None) => routing::not_found_response(),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if path == "/api/soulseek/mesh-rendezvous/interest" {
        let _interest_persistence = state.interest_persistence_lock.lock().await;
        let mut interests = state.interests.write().await;
        if method == "POST" {
            let previous = interests.clone();
            let Some((record, _)) = interests.add_liked(MESH_RENDEZVOUS_INTEREST_TAG.to_owned())
            else {
                return routing::service_unavailable_response("interest capacity is full");
            };
            let mutated = interests.clone();
            drop(interests);
            return match persist_interest_checked(state, &record).await {
                Ok(_) => routing::ok_response(record.json()),
                Err(error) => {
                    let mut interests = state.interests.write().await;
                    if *interests == mutated {
                        *interests = previous;
                    }
                    drop(interests);
                    routing::service_unavailable_response(&error)
                }
            };
        }
        let previous = interests.clone();
        let record = interests
            .liked
            .iter()
            .find(|record| record.name == MESH_RENDEZVOUS_INTEREST_TAG)
            .cloned();
        let Some(record) = record else {
            return routing::not_found_response();
        };
        interests.remove_liked(&record.id);
        let mutated = interests.clone();
        drop(interests);
        return match persist_interest_delete_checked(state, &record.id).await {
            Ok(_) => routing::no_content_response(),
            Err(error) => {
                let mut interests = state.interests.write().await;
                if *interests == mutated {
                    *interests = previous;
                }
                drop(interests);
                routing::service_unavailable_response(&error)
            }
        };
    }
    if let Some(response) = Box::pin(feature_controller_mutation_response(
        method,
        path,
        body,
        state,
        is_versioned_v0,
    ))
    .await
    {
        return response;
    }
    if let Some(response) = Box::pin(misc_controller_mutation_response(
        method,
        path,
        query,
        body,
        state,
        is_versioned_v0,
        headers,
    ))
    .await
    {
        return response;
    }

    let operation_id = uuid::Uuid::new_v4().to_string();
    let payload_sha256 = hex::encode(Sha256::digest(body.as_bytes()));
    record_event(
        state,
        format!("controller.{}.accepted", method.to_ascii_lowercase()),
        path.to_owned(),
        Some(format!(
            "operation_id={operation_id};payload_sha256={payload_sha256};query={}",
            query.unwrap_or_default()
        )),
    )
    .await;
    routing::accepted_response(
        serde_json::json!({
            "operationId": operation_id,
            "accepted": true,
            "method": method,
            "path": path,
            "payloadSha256": payload_sha256,
        })
        .to_string(),
    )
}
