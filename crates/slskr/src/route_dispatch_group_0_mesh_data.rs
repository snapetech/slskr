async fn route_dispatch_group_0_mesh_data(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let method = context.method;
    let normalized_path = context.normalized_path;
    let authorization = context.authorization;
    let body = context.body;
    let state = context.state;
    let route = context.route;
    let headers = context.headers;
    let extended_mutation = context.extended_mutation;
    let request_is_versioned_v0 = context.request_is_versioned_v0;
    match (method, normalized_path) {
        ("GET", "/api/capabilities/peers") => {
            // Matches the oracle's CapabilitiesController contract: known
            // Native capability peers, not the generic connected-user list.
            let mesh = state.mesh.read().await;
            let peers = mesh.capability_service_peers_json();
            let count = peers.len();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "peers": peers,
                    "count": count,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/capabilities/mesh-peers") => {
            let mesh = state.mesh.read().await;
            let peers = mesh.capability_service_mesh_peers_json();
            let count = peers.len();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "peers": peers,
                    "count": count,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/network/stats") => {
            let include_peers = query_params(route.query.unwrap_or_default())
                .into_iter()
                .find(|(key, _)| key == "includePeers")
                .and_then(|(_, value)| parse_bool_value(&value))
                .unwrap_or(false);
            Ok(routing::ok_response(
                network_stats_value(state, include_peers).await.to_string(),
            ))
        }
        ("GET", "/api/hashdb/stats") => {
            let discovery = state.content_discovery.read().await;
            let persisted_entries = discovery.hash_entries().len();
            let latest_seq = discovery.latest_seq();
            let database_size_bytes = discovery.database_size_bytes();
            let mut peer_ids = discovery
                .shadow_records()
                .iter()
                .flat_map(|record| record.peer_ids.iter().cloned())
                .collect::<HashSet<_>>();
            drop(discovery);
            let features = state.controller_features.read().await;
            peer_ids.extend(hashdb_inventory_peer_ids(&features));
            let inventory = hashdb_inventory_records(&features, usize::MAX, false);
            drop(features);
            let total_flac_entries = inventory.len();
            let hashed_flac_entries = inventory
                .iter()
                .filter(|entry| {
                    entry
                        .get("hashStatusStr")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|status| status.eq_ignore_ascii_case("known"))
                })
                .count();
            let native_peers = state.mesh.read().await.capability_records.len();
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalPeers": peer_ids.len(),
                    "capabilityPeers": native_peers,
                    "totalFlacEntries": total_flac_entries,
                    "hashedFlacEntries": hashed_flac_entries,
                    "totalHashEntries": persisted_entries,
                    "currentSeqId": latest_seq,
                    "databaseSizeBytes": database_size_bytes,
                })
                .to_string(),
            ))
        }
        ("GET", "/api/hashdb/entries") => {
            if route.path.starts_with("/api/v0/") {
                // Frozen HashDbController.GetEntries pages by sequence ID,
                // not by the local vector offset, and returns only HashDb
                // rows. Share-index projections belong to the unversioned
                // slskR compatibility helper below, not this controller.
                let limit = match query_parameter(route.query, "limit") {
                    Some(raw) => match raw.parse::<i64>() {
                        Ok(value) => usize::try_from(value.clamp(1, 1_000)).unwrap_or(1),
                        Err(_) => return Ok(routing::bad_request_response("limit is invalid")),
                    },
                    None => 100,
                };
                let offset = match query_parameter(route.query, "offset") {
                    Some(raw) => match raw.parse::<i64>() {
                        Ok(value) => u64::try_from(value.max(0)).unwrap_or(0),
                        Err(_) => return Ok(routing::bad_request_response("offset is invalid")),
                    },
                    None => 0,
                };
                let discovery = state.content_discovery.read().await;
                let (entries, _) = discovery.hash_entries_since_seq(offset, limit);
                let latest_seq = discovery.latest_seq();
                let count = entries.len();
                Ok(routing::ok_response(
                    serde_json::json!({
                        "latestSeq": latest_seq,
                        "entries": entries,
                        "count": count,
                    })
                    .to_string(),
                ))
            } else {
                let params = route.query.map(query_params).unwrap_or_default();
                let limit = params
                    .iter()
                    .find(|(key, _)| key == "limit")
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .unwrap_or(100)
                    .clamp(1, 1_000);
                let offset = params
                    .iter()
                    .find(|(key, _)| key == "offset")
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .unwrap_or(0);
                let discovery = state.content_discovery.read().await;
                let mut entries = discovery
                    .hash_entries()
                    .iter()
                    .map(|entry| {
                        serde_json::to_value(entry).unwrap_or_else(|_| serde_json::json!({}))
                    })
                    .collect::<Vec<_>>();
                let latest_seq = discovery.latest_seq();
                drop(discovery);
                let shares = state.shares.read().await;
                entries.extend(shares
                    .entries
                    .iter()
                    .filter(|entry| is_auto_retry_audio_file(&entry.filename))
                    .map(|entry| {
                        serde_json::json!({
                            "id": format!("share-{}-{}", entry.size, entry.filename),
                            "path": entry.filename,
                            "filename": entry.filename,
                            "extension": entry.extension,
                            "size": entry.size,
                            "hash": format!("{:016x}", stable_content_hash(&entry.filename, entry.size)),
                            "source": "share-index",
                        })
                    })
                    .collect::<Vec<_>>());
                let count = entries.len();
                let entries = entries
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .collect::<Vec<_>>();
                drop(shares);
                Ok(routing::ok_response(
                    serde_json::json!({
                        "latestSeq": latest_seq,
                        "entries": entries,
                        "count": count,
                        "offset": offset,
                        "limit": limit,
                    })
                    .to_string(),
                ))
            }
        }
        ("GET", path) if path.starts_with("/api/hashdb/hash/by-size/") => {
            let Some(raw_size) = path_segment_after(path, "/api/hashdb/hash/by-size/") else {
                return Ok(routing::not_found_response());
            };
            let Ok(size) = raw_size.parse::<u64>() else {
                return Ok(routing::bad_request_response(
                    "size must be a positive integer",
                ));
            };
            if size == 0 && !route.path.starts_with("/api/v0/") {
                return Ok(routing::bad_request_response(
                    "size must be a positive integer",
                ));
            }
            let discovery = state.content_discovery.read().await;
            let entries = discovery.hashes_by_size(size);
            Ok(routing::ok_response(
                serde_json::json!({
                    "count": entries.len(),
                    "entries": entries,
                })
                .to_string(),
            ))
        }
        ("GET", path) if path.starts_with("/api/hashdb/hash/") => {
            let Some(raw_key) = path_segment_after(path, "/api/hashdb/hash/") else {
                return Ok(routing::not_found_response());
            };
            let key = decoded_path_segment(raw_key).trim().to_owned();
            if route.path.starts_with("/api/v0/") && key.is_empty() {
                return Ok(routing::bad_request_response("flacKey is required"));
            }
            let discovery = state.content_discovery.read().await;
            Ok(discovery.lookup_hash(&key).map_or_else(
                || {
                    if route.path.starts_with("/api/v0/") {
                        HttpResponse {
                            status: "404 Not Found",
                            content_type: "application/json; charset=utf-8",
                            body: serde_json::json!({"error": "No hash found for key"}).to_string(),
                        }
                    } else {
                        routing::not_found_response()
                    }
                },
                |entry| {
                    routing::ok_response(
                        serde_json::to_string(entry).unwrap_or_else(|_| "{}".to_owned()),
                    )
                },
            ))
        }
        ("POST", "/api/hashdb/hash") => {
            let entry = if route.path.starts_with("/api/v0/") {
                match hashdb_verification_entry_from_body(body) {
                    Ok(entry) => entry,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                }
            } else {
                match serde_json::from_str::<content_discovery::HashDbEntry>(body) {
                    Ok(entry) => entry,
                    Err(_) => return Ok(routing::bad_request_response("invalid hash entry")),
                }
            };
            let persistence_turn = hash_db_persistence_turn().await;
            let (
                result,
                previous_entries,
                previous_latest_seq,
                mutated_entries,
                mutated_latest_seq,
            ) = {
                let mut discovery = state.content_discovery.write().await;
                let previous_entries = discovery.hash_entries().to_vec();
                let previous_latest_seq = discovery.latest_seq();
                let result = discovery
                    .merge_hash_entries(vec![entry])
                    .map(|_| (discovery.latest_seq(), discovery.hash_entries().to_vec()));
                let mutated_entries = discovery.hash_entries().to_vec();
                let mutated_latest_seq = discovery.latest_seq();
                (
                    result,
                    previous_entries,
                    previous_latest_seq,
                    mutated_entries,
                    mutated_latest_seq,
                )
            };
            match result {
                Ok((latest_seq, entries)) => {
                    if let Err(error) =
                        persist_hash_db_snapshot(state, &entries, latest_seq, &persistence_turn)
                            .await
                    {
                        rollback_hash_db_entries_if_unchanged(
                            state,
                            previous_entries,
                            previous_latest_seq,
                            &mutated_entries,
                            mutated_latest_seq,
                        )
                        .await;
                        drop(persistence_turn);
                        return Ok(routing::internal_server_error_response(&error));
                    }
                    if route.path.starts_with("/api/v0/") {
                        Ok(routing::ok_response(
                            serde_json::json!({"stored": true}).to_string(),
                        ))
                    } else {
                        Ok(routing::ok_response(
                            serde_json::json!({
                                "stored": true,
                                "latestSeq": latest_seq,
                            })
                            .to_string(),
                        ))
                    }
                }
                Err(error) => {
                    drop(persistence_turn);
                    Ok(content_discovery_error_response(state, error).await)
                }
            }
        }
        ("POST", "/api/hashdb/sync/merge") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(_) => return Ok(routing::bad_request_response("invalid hash merge request")),
            };
            let from_user = value
                .get("fromUser")
                .or_else(|| value.get("username"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("http-sync")
                .to_owned();
            let sync_settings = state
                .advanced_networking
                .read()
                .await
                .mesh_sync_security
                .clone();
            if state
                .mesh
                .write()
                .await
                .sync_is_quarantined(&from_user, unix_timestamp())
            {
                return Ok(HttpResponse {
                    status: "429 Too Many Requests",
                    content_type: "application/json; charset=utf-8",
                    body: serde_json::json!({"error":"mesh peer is quarantined"}).to_string(),
                });
            }
            let entries_value = match value.get("entries") {
                Some(entries) => entries,
                None => return Ok(routing::bad_request_response("entries are required")),
            };
            let max_entries = if route.path.starts_with("/api/v0/") {
                content_discovery::MAX_MESH_MERGE_ENTRIES
            } else {
                content_discovery::MAX_HASH_MERGE_ENTRIES
            };
            if json_array_exceeds_limit(entries_value, max_entries) {
                return Ok(routing::bad_request_response(&format!(
                    "entries must contain at most {max_entries} entries"
                )));
            }
            let entries = match serde_json::from_value::<Vec<content_discovery::HashDbEntry>>(
                entries_value.clone(),
            ) {
                Ok(entries) => entries,
                Err(_) => return Ok(routing::bad_request_response("entries are required")),
            };
            let received = entries.len();
            let persistence_turn = hash_db_persistence_turn().await;
            let (
                result,
                previous_entries,
                previous_latest_seq,
                mutated_entries,
                mutated_latest_seq,
            ) = {
                let mut discovery = state.content_discovery.write().await;
                let previous_entries = discovery.hash_entries().to_vec();
                let previous_latest_seq = discovery.latest_seq();
                let result = if route.path.starts_with("/api/v0/") {
                    discovery
                        .merge_hash_entries_from_mesh(entries)
                        .map(|(merged, _skipped)| {
                            (
                                merged,
                                discovery.latest_seq(),
                                discovery.hash_entries().to_vec(),
                            )
                        })
                } else {
                    discovery.merge_hash_entries(entries).map(|merged| {
                        (
                            merged,
                            discovery.latest_seq(),
                            discovery.hash_entries().to_vec(),
                        )
                    })
                };
                let mutated_entries = discovery.hash_entries().to_vec();
                let mutated_latest_seq = discovery.latest_seq();
                (
                    result,
                    previous_entries,
                    previous_latest_seq,
                    mutated_entries,
                    mutated_latest_seq,
                )
            };
            match result {
                Ok((merged, latest_seq, entries)) => {
                    if let Err(error) =
                        persist_hash_db_snapshot(state, &entries, latest_seq, &persistence_turn)
                            .await
                    {
                        rollback_hash_db_entries_if_unchanged(
                            state,
                            previous_entries,
                            previous_latest_seq,
                            &mutated_entries,
                            mutated_latest_seq,
                        )
                        .await;
                        drop(persistence_turn);
                        return Ok(routing::internal_server_error_response(&error));
                    }
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "received": received,
                            "merged": merged,
                            "latestSeq": latest_seq,
                        })
                        .to_string(),
                    ))
                }
                Err(error) => {
                    drop(persistence_turn);
                    let invalid = u32::try_from(received).unwrap_or(u32::MAX);
                    let rate_limited = state.mesh.write().await.record_invalid_sync_entries(
                        &from_user,
                        invalid,
                        &sync_settings,
                        unix_timestamp(),
                    );
                    if rate_limited {
                        record_peer_security_violation(state, &from_user).await;
                        Ok(HttpResponse {
                            status: "429 Too Many Requests",
                            content_type: "application/json; charset=utf-8",
                            body: serde_json::json!({"error":"mesh invalid-entry rate limit exceeded"}).to_string(),
                        })
                    } else {
                        Ok(content_discovery_error_response(state, error).await)
                    }
                }
            }
        }
        ("POST", "/api/virtualsoulfind/shadow-index/sync/merge") => {
            let value = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => value,
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "invalid shadow-index merge request",
                    ))
                }
            };
            let records_value = match value.get("records").or_else(|| value.get("entries")) {
                Some(records) => records,
                None => return Ok(routing::bad_request_response("records are required")),
            };
            if json_array_exceeds_limit(records_value, content_discovery::MAX_SHADOW_MERGE_RECORDS)
            {
                return Ok(routing::bad_request_response(&format!(
                    "records must contain at most {} records",
                    content_discovery::MAX_SHADOW_MERGE_RECORDS
                )));
            }
            let records = match serde_json::from_value::<Vec<content_discovery::ShadowIndexRecord>>(
                records_value.clone(),
            ) {
                Ok(records) => records,
                Err(_) => return Ok(routing::bad_request_response("records are required")),
            };
            let realm_indexes = match value
                .get("realmIndexes")
                .or_else(|| value.get("realm_indexes"))
            {
                Some(indexes) => {
                    if json_array_exceeds_limit(indexes, realm_subject_index::MAX_INDEXES) {
                        return Ok(routing::bad_request_response(&format!(
                            "realmIndexes must contain at most {} indexes",
                            realm_subject_index::MAX_INDEXES
                        )));
                    }
                    match serde_json::from_value::<Vec<serde_json::Value>>(indexes.clone()) {
                        Ok(indexes) => indexes,
                        Err(_) => {
                            return Ok(routing::bad_request_response(
                                "realmIndexes must be an array of objects",
                            ))
                        }
                    }
                }
                None => Vec::new(),
            };
            let received = records.len();
            let mut discovery = state.content_discovery.write().await;
            let mut realm_store = state.realm_subject_indexes.write().await;
            let previous_shadow_records = if realm_indexes.is_empty() {
                None
            } else {
                Some(discovery.shadow_records().to_vec())
            };
            if !realm_indexes.is_empty() {
                if let Err(error) = realm_store.validate_indexes(&realm_indexes) {
                    return Ok(routing::bad_request_response(&error));
                }
            }
            match discovery.merge_shadow_records(records) {
                Ok(merged) => {
                    let indexes_merged = if realm_indexes.is_empty() {
                        0
                    } else {
                        match realm_store.merge_indexes(realm_indexes) {
                            Ok(merged) => merged,
                            Err(error) => {
                                let error = match previous_shadow_records {
                                    Some(previous) => match
                                        discovery.restore_shadow_records(previous)
                                    {
                                        Ok(()) => error,
                                        Err(rollback_error) => format!(
                                            "{error}; shadow-record rollback failed: {rollback_error}"
                                        ),
                                    },
                                    None => error,
                                };
                                drop(realm_store);
                                drop(discovery);
                                update_session(state, |snapshot| {
                                    snapshot.last_error =
                                        Some(format!("realm subject-index merge failed: {error}"));
                                })
                                .await;
                                return Ok(routing::service_unavailable_response(
                                    "realm subject-index storage is unavailable",
                                ));
                            }
                        }
                    };
                    Ok(routing::ok_response(
                        serde_json::json!({
                            "received": received,
                            "merged": merged,
                            "realmIndexesMerged": indexes_merged,
                        })
                        .to_string(),
                    ))
                }
                Err(error) => {
                    drop(realm_store);
                    drop(discovery);
                    Ok(content_discovery_error_response(state, error).await)
                }
            }
        }
        (method, path) if path.starts_with("/api/virtualsoulfind/v2") => {
            Ok(route_virtual_soulfind_v2(method, path, route.query, body, state).await)
        }
        ("GET", path) if path.starts_with("/api/virtualsoulfind/shadow-index/") => {
            let Some(raw_recording_id) =
                path_segment_after(path, "/api/virtualsoulfind/shadow-index/")
            else {
                return Ok(routing::not_found_response());
            };
            let recording_id = decoded_path_segment(raw_recording_id);
            let discovery = state.content_discovery.read().await;
            if state.config.controller_profile == ControllerProfile::Native {
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "variants": virtual_soulfind_legacy_variants(&discovery, &recording_id),
                    })
                    .to_string(),
                ));
            }
            let record = discovery
                .shadow_records()
                .iter()
                .find(|record| record.recording_id.eq_ignore_ascii_case(&recording_id));
            Ok(routing::ok_response(
                record
                    .map_or_else(
                        || {
                            serde_json::json!({
                                "recordingId": recording_id,
                                "peerIds": [],
                                "totalPeerCount": 0,
                                "variants": [],
                            })
                        },
                        |record| {
                            serde_json::json!({
                                "recordingId": record.recording_id,
                                "peerIds": record.peer_ids,
                                "totalPeerCount": record.peer_ids.len(),
                                "variants": [],
                                "updatedAt": record.updated_at,
                            })
                        },
                    )
                    .to_string(),
            ))
        }
        ("POST", "/api/hashdb/backfill/from-history") => {
            Ok(hashdb_backfill_from_history_response(route.query, state).await)
        }
        ("POST", path) if path.starts_with("/api/hashdb/backfill/from-history") => {
            Ok(routing::not_found_response())
        }
        ("GET", "/api/mesh/stats") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let known_mesh_peers = mesh.candidate_usernames(&users).len();
            let rejected_messages = mesh.sync_rejected_messages;
            let quarantine_events = mesh.sync_quarantine_events;
            let quarantined_peers = mesh.sync_quarantined_until.len();
            let total_syncs = mesh.sync_merge_total;
            let successful_syncs = mesh.sync_merge_successful;
            let failed_syncs = mesh.sync_merge_failed;
            let total_entries_received = mesh.sync_entries_received;
            let total_entries_sent = mesh.sync_entries_sent;
            let skipped_entries = mesh.sync_skipped_entries;
            let rate_limit_violations = mesh.sync_rate_limit_violations;
            let total_entries_merged = mesh.sync_entries_merged;
            drop(mesh);
            drop(users);
            let current_seq_id = state.content_discovery.read().await.latest_seq();
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalSyncs": total_syncs,
                    "successfulSyncs": successful_syncs,
                    "failedSyncs": failed_syncs,
                    "totalEntriesReceived": total_entries_received,
                    "totalEntriesSent": total_entries_sent,
                    "totalEntriesMerged": total_entries_merged,
                    "rejectedMessages": rejected_messages,
                    "skippedEntries": skipped_entries,
                    "signatureVerificationFailures": 0,
                    "reputationBasedRejections": 0,
                    "rateLimitViolations": rate_limit_violations,
                    "quarantinedPeers": quarantined_peers,
                    "quarantineEvents": quarantine_events,
                    "proofOfPossessionFailures": 0,
                    "currentSeqId": current_seq_id,
                    "knownMeshPeers": known_mesh_peers,
                    "warnings": [],
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/peers") if route.path.starts_with("/api/v0/") => {
            let mesh = state.mesh.read().await;
            let peers = mesh
                .capability_records_json()
                .into_iter()
                .filter(|record| record["meshCapable"] == true)
                .collect::<Vec<_>>();
            drop(mesh);
            Ok(routing::ok_response(
                serde_json::json!({
                    "count": peers.len(),
                    "peers": peers,
                    "overlay": [],
                })
                .to_string(),
            ))
        }
        ("GET", "/api/mesh/peers") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(&mesh.users_json(&users))
                .unwrap_or_else(|_| serde_json::json!({}));
            value["peers"] = value["users"].clone();
            let active_connections = match state.private_gateway.as_ref() {
                Some(gateway) => gateway.active_connection_count().await,
                None => 0,
            };
            value["overlay"] = serde_json::json!({
                "enabled": state.private_gateway.is_some(),
                "activeConnections": active_connections,
            });
            let body = value.to_string();
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(body))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
