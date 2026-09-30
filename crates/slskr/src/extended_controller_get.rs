use super::*;

pub(crate) async fn extended_controller_get_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
    versioned_v0: bool,
) -> HttpResponse {
    match path {
        "/api/bridge/rooms" => {
            let rooms = state.rooms.read().await;
            let rows = rooms
                .records
                .iter()
                .filter(|room| room.joined)
                .map(|room| {
                    if state.config.controller_profile == ControllerProfile::Native {
                        serde_json::json!({
                            "name": room.name,
                            "memberCount": room.user_count.unwrap_or_else(|| {
                                u32::try_from(room.members.len()).unwrap_or(u32::MAX)
                            }),
                        })
                    } else {
                        serde_json::json!({
                            "room": room.name,
                            "joined": room.joined,
                            "userCount": room.user_count,
                            "messageCount": room.messages.len(),
                        })
                    }
                })
                .collect::<Vec<_>>();
            let body = if state.config.controller_profile == ControllerProfile::Native {
                serde_json::json!({"rooms": rows}).to_string()
            } else {
                serde_json::Value::Array(rows).to_string()
            };
            HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body,
            }
        }
        "/api/source-feed-imports/history" => {
            let limit = query_parameter(query, "limit")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(50)
                .clamp(
                    1,
                    i64::try_from(MAX_SOURCE_FEED_IMPORT_HISTORY).unwrap_or(100),
                );
            let history = state.source_feed_import_history.read().await;
            let rows = history
                .history
                .iter()
                .take(usize::try_from(limit).unwrap_or(MAX_SOURCE_FEED_IMPORT_HISTORY))
                .cloned()
                .collect::<Vec<_>>();
            HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::Value::Array(rows).to_string(),
            }
        }
        "/api/hashdb/backfill/candidates" => {
            let limit = match query_parameter(query, "limit") {
                None => 10,
                Some(value) => match value.parse::<i64>() {
                    Ok(value) => usize::try_from(value.max(1))
                        .unwrap_or(1_000)
                        .clamp(1, 1_000),
                    Err(_) if versioned_v0 => {
                        return routing::bad_request_response("limit is invalid")
                    }
                    Err(_) => 10,
                },
            };
            let entries =
                hashdb_inventory_records(&*state.controller_features.read().await, limit, true);
            routing::ok_response(
                serde_json::json!({"count": entries.len(), "entries": entries}).to_string(),
            )
        }
        "/api/hashdb/inventory/unhashed" => {
            if versioned_v0
                && query_parameter(query, "limit")
                    .is_some_and(|value| value.parse::<i64>().is_err())
            {
                return routing::bad_request_response("limit is invalid");
            }
            let mut rows =
                hashdb_inventory_records(&*state.controller_features.read().await, 1_000, true);
            if !versioned_v0 {
                let discovery = state.content_discovery.read().await;
                let hashed_sizes = discovery
                    .hash_entries()
                    .iter()
                    .map(|entry| entry.size)
                    .collect::<HashSet<_>>();
                drop(discovery);
                let shares = state.shares.read().await;
                rows.extend(
                    shares
                        .entries
                        .iter()
                        .filter(|entry| !hashed_sizes.contains(&entry.size))
                        .take(1_000)
                        .map(|entry| {
                            serde_json::json!({
                                "filename": entry.filename,
                                "size": entry.size,
                                "extension": entry.extension,
                            })
                        })
                        .collect::<Vec<_>>(),
                );
            }
            rows.truncate(1_000);
            let count = rows.len();
            routing::ok_response(
                serde_json::json!({"entries": rows.clone(), "items": rows, "count": count})
                    .to_string(),
            )
        }
        "/api/hashdb/metadata-processing" => {
            // slskdn exposes the metadata pipeline's active and completed
            // stages. Return the real bounded in-memory stage tracker.
            let limit = query_parameter(query, "limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(50);
            let discovery = state.content_discovery.read().await;
            let (active, history) = discovery.metadata_processing_status(limit);
            routing::ok_response(
                serde_json::json!({"active": active, "history": history}).to_string(),
            )
        }
        "/api/hashdb/key" => {
            let filename = query_parameter(query, "filename").unwrap_or_default();
            let size = query_parameter(query, "size").and_then(|size| size.parse::<u64>().ok());
            if filename.trim().is_empty() || size.is_none_or(|size| size == 0) {
                return routing::bad_request_response("filename and positive size are required");
            }
            let size = size.unwrap_or_default();
            routing::ok_response(
                serde_json::json!({
                    "flacKey": content_discovery::generate_flac_key(&filename, size),
                })
                .to_string(),
            )
        }
        "/api/hashdb/optimize/analyze" => {
            let discovery = state.content_discovery.read().await;
            let hash_db_entry_count = discovery.hash_entries().len();
            let mut peer_ids = discovery
                .shadow_records()
                .iter()
                .flat_map(|record| record.peer_ids.iter().cloned())
                .collect::<HashSet<_>>();
            let database_size_bytes = discovery.database_size_bytes();
            drop(discovery);
            let features = state.controller_features.read().await;
            peer_ids.extend(hashdb_inventory_peer_ids(&features));
            let flac_inventory_entry_count =
                hashdb_inventory_records(&features, usize::MAX, false).len();
            drop(features);
            let peer_count = peer_ids.len();
            let mut recommendations: Vec<&str> = Vec::new();
            if hash_db_entry_count > 100_000 {
                recommendations
                    .push("Large HashDb table detected. Consider running VACUUM to reclaim space.");
            }
            if database_size_bytes > 100 * 1024 * 1024 {
                recommendations
                    .push("Database size exceeds 100MB. Consider running VACUUM to optimize.");
            }
            routing::ok_response(
                serde_json::json!({
                    "analyzed": true,
                    "entries": hash_db_entry_count,
                    "hashDbEntryCount": hash_db_entry_count,
                    "flacInventoryEntryCount": flac_inventory_entry_count,
                    "peerCount": peer_count,
                    "databaseSizeBytes": database_size_bytes,
                    // The store is a single JSON file, not SQLite -- there
                    // are no named indexes that can be "missing".
                    "missingIndexes": Vec::<String>::new(),
                    "recommendations": recommendations,
                })
                .to_string(),
            )
        }
        "/api/hashdb/optimize/slow-queries" => {
            let limit = match query_parameter(query, "limit") {
                None => 20,
                Some(value) => match value.parse::<i64>() {
                    Ok(value) => usize::try_from(value.max(0)).unwrap_or(usize::MAX),
                    Err(_) if versioned_v0 => {
                        return routing::bad_request_response("limit is invalid")
                    }
                    Err(_) => 20,
                },
            };
            let discovery = state.content_discovery.read().await;
            let slow_queries = discovery.slow_queries(limit);
            let total_queries = discovery.total_queries();
            drop(discovery);
            routing::ok_response(
                serde_json::json!({
                    "totalQueries": total_queries,
                    "slowQueries": slow_queries,
                })
                .to_string(),
            )
        }
        "/api/hashdb/peers" => {
            if versioned_v0 {
                // Frozen HashDbController reads the capability-backed Peer
                // projection and includes only peers with non-zero HashDb
                // capability flags. The unversioned route remains the
                // historical local inventory projection below.
                let mesh = state.mesh.read().await;
                let peer_records = mesh
                    .capability_service_peers_json()
                    .into_iter()
                    .filter(|peer| peer["flagsValue"].as_i64().unwrap_or(0) > 0)
                    .collect::<Vec<_>>();
                drop(mesh);
                let mut peers = Vec::with_capacity(peer_records.len());
                for peer in peer_records {
                    let last_seen = peer["lastSeen"]
                        .as_u64()
                        .map(unix_seconds_rfc3339)
                        .unwrap_or_else(|| unix_seconds_rfc3339(0));
                    let backfills_today = state.backfill.write().await.peer_count_today(
                        peer["username"].as_str().unwrap_or_default(),
                        unix_timestamp(),
                    );
                    peers.push(serde_json::json!({
                        "peerId": peer["username"],
                        "caps": peer["flagsValue"],
                        "capsFlags": peer["flags"],
                        "clientVersion": peer["clientVersion"],
                        "lastSeen": last_seen,
                        "backfillsToday": backfills_today,
                    }));
                }
                peers.sort_by(|left, right| {
                    right["lastSeen"].as_str().cmp(&left["lastSeen"].as_str())
                });
                let count = peers.len();
                routing::ok_response(
                    serde_json::json!({"count": count, "peers": peers}).to_string(),
                )
            } else {
                let discovery = state.content_discovery.read().await;
                let mut peer_ids = discovery
                    .shadow_records()
                    .iter()
                    .flat_map(|record| record.peer_ids.iter().cloned())
                    .collect::<HashSet<_>>();
                drop(discovery);
                peer_ids.extend(hashdb_inventory_peer_ids(
                    &*state.controller_features.read().await,
                ));
                let mut peers = peer_ids
                    .into_iter()
                    .map(|peer_id| serde_json::json!({"peerId": peer_id}))
                    .collect::<Vec<_>>();
                peers.sort_by(|left, right| left["peerId"].as_str().cmp(&right["peerId"].as_str()));
                let count = peers.len();
                routing::ok_response(
                    serde_json::json!({"peers": peers, "count": count}).to_string(),
                )
            }
        }
        "/api/hashdb/schema" => {
            let discovery = state.content_discovery.read().await;
            routing::ok_response(
                serde_json::json!({
                    "currentVersion": HASHDB_SCHEMA_VERSION,
                    "targetVersion": HASHDB_SCHEMA_VERSION,
                    "isUpToDate": true,
                    "message": "Schema is up to date",
                    "latestSeqId": discovery.latest_seq(),
                    "tables": ["hashEntries", "shadowRecords"],
                })
                .to_string(),
            )
        }
        "/api/mesh/delta" => {
            let discovery = state.content_discovery.read().await;
            let since_seq = match query_parameter(query, "sinceSeq")
                .or_else(|| query_parameter(query, "since_seq_id"))
                .or_else(|| query_parameter(query, "fromSeqId"))
            {
                Some(value) => match value.parse::<i64>() {
                    Ok(value) => value.max(0) as u64,
                    Err(_) if versioned_v0 => {
                        return routing::bad_request_response("The value 'sinceSeq' is not valid.")
                    }
                    Err(_) => 0,
                },
                None => 0,
            };
            let max_entries = match query_parameter(query, "maxEntries")
                .or_else(|| query_parameter(query, "max_entries"))
                .or_else(|| query_parameter(query, "limit"))
            {
                Some(value) => match value.parse::<i32>() {
                    Ok(value) => value.max(0) as usize,
                    Err(_) if versioned_v0 => {
                        return routing::bad_request_response(
                            "The value 'maxEntries' is not valid.",
                        )
                    }
                    Err(_) => 1_000,
                },
                None => 1_000,
            }
            .min(10_000);
            let (entries, has_more) = discovery.hash_entries_since_seq(since_seq, max_entries);
            let latest_seq_id = discovery.latest_seq();
            drop(discovery);
            let sent = entries.len() as u64;
            if sent > 0 {
                let mut mesh = state.mesh.write().await;
                mesh.sync_entries_sent = mesh.sync_entries_sent.saturating_add(sent);
            }
            let wire_entries = entries
                .into_iter()
                .map(|entry| {
                    serde_json::json!({
                        "seq_id": entry.seq_id,
                        "flac_key": entry.flac_key,
                        "byte_hash": entry.byte_hash,
                        "size": entry.size,
                    })
                })
                .collect::<Vec<_>>();
            routing::ok_response(
                serde_json::json!({
                    "type": 3,
                    "proto_version": 1,
                    "public_key": "",
                    "signature": "",
                    "timestamp_ms": 0,
                    "latest_seq_id": latest_seq_id,
                    "entries": wire_entries,
                    "has_more": has_more,
                })
                .to_string(),
            )
        }
        "/api/mesh/hello" => {
            let discovery = state.content_discovery.read().await;
            let latest_seq_id = discovery.latest_seq();
            let hash_count = discovery.hash_entries().len();
            drop(discovery);
            let client_id = state
                .config
                .username
                .clone()
                .unwrap_or_else(|| "slskr".to_owned());
            routing::ok_response(
                serde_json::json!({
                    "type": 1,
                    "proto_version": 1,
                    "public_key": "",
                    "signature": "",
                    "timestamp_ms": 0,
                    "client_id": client_id,
                    "client_version": APP_VERSION,
                    "latest_seq_id": latest_seq_id,
                    "hash_count": hash_count,
                })
                .to_string(),
            )
        }
        "/api/multisource/search" => {
            if versioned_v0 && state.config.controller_profile == ControllerProfile::Native {
                let search_text = query_parameter(query, "searchText")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if search_text.is_empty() {
                    return routing::bad_request_response("Search text is required");
                }
                let searches = state.searches.read().await;
                let mut groups = BTreeMap::<
                    (String, u64),
                    (String, String, String, Vec<serde_json::Value>),
                >::new();
                for record in searches.records.iter().filter(|record| {
                    record.query.eq_ignore_ascii_case(&search_text)
                        || record
                            .query
                            .to_ascii_lowercase()
                            .contains(&search_text.to_ascii_lowercase())
                }) {
                    for result in &record.results {
                        let filename = virtual_basename(&result.filename).to_owned();
                        let key = (filename.to_ascii_lowercase(), result.size);
                        let extension = filename
                            .rsplit_once('.')
                            .map(|(_, extension)| format!(".{extension}").to_ascii_lowercase())
                            .unwrap_or_default();
                        let group = groups.entry(key).or_insert_with(|| {
                            (
                                filename.clone(),
                                result.filename.clone(),
                                extension,
                                Vec::new(),
                            )
                        });
                        if let Some(username) = result.peer_username.as_deref() {
                            group.3.push(serde_json::json!({
                                "username": username,
                                "fullPath": result.filename,
                                "hasFreeUploadSlot": result.slot_free.unwrap_or(false),
                                "queueLength": result.queue_length.unwrap_or(0),
                                "uploadSpeed": result.average_speed.unwrap_or(0),
                                "bitRate": null,
                                "sampleRate": null,
                                "bitDepth": null,
                            }));
                        }
                    }
                }
                let total_files = groups.len();
                let mut candidates = groups
                    .into_iter()
                    .filter(|(_, (_, _, _, sources))| sources.len() >= 2)
                    .map(|((_, size), (filename, full_path, extension, sources))| {
                        serde_json::json!({
                            "filename": filename,
                            "fullPath": full_path,
                            "size": size,
                            "extension": extension,
                            "sources": sources,
                        })
                    })
                    .collect::<Vec<_>>();
                candidates.truncate(50);
                return routing::ok_response(
                    serde_json::json!({
                        "query": search_text,
                        "totalFiles": total_files,
                        "multiSourceCandidates": candidates.len(),
                        "candidates": candidates,
                    })
                    .to_string(),
                );
            }
            let searches = state.searches.read().await;
            let rows = searches.records.iter().take(100).map(|record| serde_json::json!({
                "id": record.id,
                "query": record.query,
                "status": record.status,
                "sourceCount": record.results.iter().filter_map(|result| result.peer_username.as_deref()).collect::<HashSet<_>>().len(),
                "fileCount": record.results.len(),
            })).collect::<Vec<_>>();
            routing::ok_response(serde_json::Value::Array(rows).to_string())
        }
        "/api/multisource/users" => {
            if versioned_v0 && state.config.controller_profile == ControllerProfile::Native {
                let search_text = query_parameter(query, "searchText")
                    .unwrap_or_default()
                    .trim()
                    .to_owned();
                if search_text.is_empty() {
                    return routing::bad_request_response("Search text is required");
                }
                let search_text_lower = search_text.to_ascii_lowercase();
                let searches = state.searches.read().await;
                let mut users = BTreeMap::<String, (usize, usize, bool, u32, u32)>::new();
                let mut total_responses = 0_usize;
                for record in searches.records.iter().filter(|record| {
                    record.query.eq_ignore_ascii_case(&search_text)
                        || record
                            .query
                            .to_ascii_lowercase()
                            .contains(&search_text_lower)
                }) {
                    total_responses = total_responses.saturating_add(1);
                    for result in &record.results {
                        let Some(username) = result.peer_username.as_deref() else {
                            continue;
                        };
                        let entry = users.entry(username.to_owned()).or_default();
                        entry.0 = entry.0.saturating_add(1);
                        if result.extension.eq_ignore_ascii_case("flac") {
                            entry.1 = entry.1.saturating_add(1);
                        }
                        entry.2 |= result.slot_free.unwrap_or(false);
                        entry.3 = entry.3.max(result.average_speed.unwrap_or(0));
                        entry.4 = entry.4.max(result.queue_length.unwrap_or(0));
                    }
                }
                let users = users
                    .into_iter()
                    .take(10)
                    .enumerate()
                    .map(
                        |(
                            index,
                            (username, (file_count, flac_count, has_free_slot, speed, queue)),
                        )| {
                            let score = if has_free_slot { 1_000.0 } else { 0.0 }
                                + speed as f64 / 1_000.0
                                + (100_u32.saturating_sub(queue.min(100))) as f64
                                + flac_count as f64;
                            serde_json::json!({
                                "index": index + 1,
                                "username": username,
                                "fileCount": file_count,
                                "flacCount": flac_count,
                                "hasFreeSlot": has_free_slot,
                                "speed": format!("{} KB/s", speed / 1024),
                                "queue": queue,
                                "score": score,
                            })
                        },
                    )
                    .collect::<Vec<_>>();
                return routing::ok_response(
                    serde_json::json!({
                        "query": search_text,
                        "totalResponses": total_responses,
                        "users": users,
                        "hint": "Use /api/v0/multisource/users/{username}/files to list files from a user",
                    })
                    .to_string(),
                );
            }
            let searches = state.searches.read().await;
            let mut users = BTreeMap::<String, (usize, u32)>::new();
            for result in searches
                .records
                .iter()
                .flat_map(|record| record.results.iter())
            {
                if let Some(username) = result.peer_username.as_ref() {
                    let entry = users.entry(username.clone()).or_default();
                    entry.0 = entry.0.saturating_add(1);
                    entry.1 = entry.1.max(result.average_speed.unwrap_or(0));
                }
            }
            routing::ok_response(
                serde_json::Value::Array(
                    users
                        .into_iter()
                        .map(|(username, (files, speed))| {
                            serde_json::json!({
                                "username": username,
                                "fileCount": files,
                                "uploadSpeed": speed,
                            })
                        })
                        .collect(),
                )
                .to_string(),
            )
        }
        "/api/opinions" => {
            let opinions = state
                .controller_features
                .read()
                .await
                .values_with_prefix("opinion/");
            routing::ok_response(serde_json::Value::Array(opinions).to_string())
        }
        "/api/opinions/summary" => {
            // Matches the oracle's OpinionController.Summary +
            // OpinionService.SummarizeAsync: requires a real subjectType
            // and subjectId (not just a non-empty query string), filters
            // to that subject/scope, and returns a real weighted-score
            // summary -- not a global aggregate with an invented "neutral"
            // bucket the oracle's DTO doesn't have.
            let subject_type = query_parameter(query, "subjectType").unwrap_or_default();
            let subject_id = query_parameter(query, "subjectId").unwrap_or_default();
            if subject_type.trim().is_empty()
                || subject_type.eq_ignore_ascii_case("Unknown")
                || subject_id.trim().is_empty()
            {
                return routing::bad_request_response("subjectType and subjectId are required");
            }
            let subject_id = subject_id.trim().to_owned();
            let scope = query_parameter(query, "scope")
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "global".to_owned());

            fn opinion_polarity(opinion: &serde_json::Value) -> i64 {
                match opinion
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                {
                    "Like" | "Trust" | "Recommend" | "VerifiedGood" => 1,
                    "Hate" | "Distrust" | "Block" | "Quarantine" | "VerifiedBad" => -1,
                    _ => 0,
                }
            }

            let matching = state
                .controller_features
                .read()
                .await
                .values_with_prefix("opinion/")
                .into_iter()
                .filter(|opinion| {
                    opinion
                        .get("subjectType")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|value| value.eq_ignore_ascii_case(&subject_type))
                        && opinion
                            .get("subjectId")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|value| value.trim() == subject_id)
                        && opinion
                            .get("scope")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("global")
                            .eq_ignore_ascii_case(&scope)
                })
                .collect::<Vec<_>>();
            let total = matching.len();
            let weighted: f64 = matching
                .iter()
                .map(|opinion| {
                    let strength = opinion
                        .get("strength")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0);
                    let confidence = opinion
                        .get("confidence")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(0.0);
                    opinion_polarity(opinion) as f64 * strength.abs() * confidence
                })
                .sum();
            let weighted_score = weighted.clamp(-(total as f64), total as f64);
            let confidence = if total == 0 {
                0.0
            } else {
                matching
                    .iter()
                    .map(|opinion| {
                        opinion
                            .get("confidence")
                            .and_then(serde_json::Value::as_f64)
                            .unwrap_or(0.0)
                    })
                    .sum::<f64>()
                    / total as f64
            };
            let positive = matching
                .iter()
                .filter(|opinion| opinion_polarity(opinion) > 0)
                .count();
            let negative = matching
                .iter()
                .filter(|opinion| opinion_polarity(opinion) < 0)
                .count();

            routing::ok_response(
                serde_json::json!({
                    "subjectType": subject_type,
                    "subjectId": subject_id,
                    "scope": scope,
                    "total": total,
                    "positive": positive,
                    "negative": negative,
                    "weightedScore": weighted_score,
                    "confidence": confidence,
                    "opinions": matching,
                })
                .to_string(),
            )
        }
        "/api/overlay/blocklist" => {
            // Matches the oracle's real GetBlocklist/BlockedEntryResponse
            // shape (target/type/reason/blockedAt/expiresAt/isPermanent),
            // not the generic {kind,type,value,created_at} ban-listing
            // shape used elsewhere.
            let security = state.security.read().await;
            let entries = security
                .bans
                .iter()
                .map(|record| {
                    serde_json::json!({
                        "target": record.value,
                        "type": record.kind,
                        "reason": record.reason,
                        "blockedAt": unix_seconds_rfc3339(record.created_at),
                        "expiresAt": unix_seconds_rfc3339(record.expires_at),
                        "isPermanent": record.is_permanent,
                    })
                })
                .collect::<Vec<_>>();
            drop(security);
            routing::ok_response(serde_json::json!({"entries": entries}).to_string())
        }
        "/api/overlay/connections" => {
            let connections = match state.private_gateway.as_ref() {
                Some(gateway) => gateway.active_overlay_connections().await,
                None => Vec::new(),
            };
            routing::ok_response(
                serde_json::to_string(&connections).unwrap_or_else(|_| "[]".to_owned()),
            )
        }
        "/api/overlay/stats" => {
            let peers = state.peer_endpoints.read().await;
            let listeners = state.listeners.read().await;
            let active_connections = match state.private_gateway.as_ref() {
                Some(gateway) => gateway.active_connection_count().await,
                None => 0,
            };
            routing::ok_response(
                serde_json::json!({
                    "server": {"activeConnections": active_connections, "acceptedConnections": listeners.regular_accepts + listeners.obfuscated_accepts},
                    "connector": {"knownPeers": peers.len()},
                    "rateLimiter": {"rejected": 0},
                    "blocklist": {"entries": state.security.read().await.active_bans()},
                    "activeConnections": active_connections,
                    "knownPeers": peers.len(),
                    "acceptedConnections": listeners.regular_accepts + listeners.obfuscated_accepts,
                    "errors": listeners.errors,
                })
                .to_string(),
            )
        }
        path if path.starts_with("/api/podcore/") => {
            podcore_stats_response(path, query, state, versioned_v0).await
        }
        "/api/quarantine-jury/audit" | "/api/quarantine-jury/requests" => {
            let requests = state
                .controller_features
                .read()
                .await
                .values_with_prefix("quarantine/request/");
            let request_count = requests.len();
            if path.ends_with("/audit") {
                let stale_after_hours = query
                    .map(query_params)
                    .unwrap_or_default()
                    .into_iter()
                    .find(|(key, _)| key == "staleAfterHours")
                    .and_then(|(_, value)| value.parse::<u64>().ok())
                    .unwrap_or(72)
                    .max(1);
                let generated_at = unix_timestamp();
                let mut entries = Vec::with_capacity(requests.len());
                for request in &requests {
                    entries.push(
                        quarantine_build_audit_entry(
                            state,
                            request,
                            generated_at,
                            stale_after_hours.saturating_mul(3600),
                        )
                        .await,
                    );
                }
                let count_with_status = |status: &str| {
                    entries
                        .iter()
                        .filter(|entry| entry["status"] == status)
                        .count()
                };
                routing::ok_response(
                    serde_json::json!({
                        "entries": entries,
                        "requestCount": request_count,
                        "acceptedReleaseCandidateCount": count_with_status("accepted-release-candidate"),
                        "pendingReleaseCandidateCount": count_with_status("pending-release-acceptance"),
                        "pendingManualReviewCount": count_with_status("manual-review"),
                        "upholdQuarantineCount": count_with_status("uphold-quarantine"),
                        "staleRequestCount": entries.iter().filter(|entry| entry["isStale"] == true).count(),
                        "count": request_count,
                        "generatedAt": generated_at,
                    })
                    .to_string(),
                )
            } else {
                routing::ok_response(serde_json::Value::Array(requests).to_string())
            }
        }
        "/api/signals/config"
            if versioned_v0 && state.config.controller_profile == ControllerProfile::Native =>
        {
            let advanced = state.advanced_networking.read().await;
            let signal_system = &advanced.signal_system;
            HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({
                    "enabled": signal_system.enabled,
                    "deduplication_cache_size": signal_system.deduplication_cache_size,
                    "default_ttl_seconds": signal_system.default_ttl.as_secs(),
                    "mesh_channel": {
                        "enabled": signal_system.mesh_channel.enabled,
                        "priority": signal_system.mesh_channel.priority,
                        "require_active_session": signal_system.mesh_channel.require_active_session,
                    },
                    "bt_extension_channel": {
                        "enabled": signal_system.bt_extension_channel.enabled,
                        "priority": signal_system.bt_extension_channel.priority,
                        "require_active_session": signal_system.bt_extension_channel.require_active_session,
                    },
                })
                .to_string(),
            }
        }
        "/api/signals/config" => {
            let advanced = state.advanced_networking.read().await;
            let signal_system = &advanced.signal_system;
            routing::ok_response(
                serde_json::json!({
                    "enabled": signal_system.enabled,
                    "deduplicationCacheSize": signal_system.deduplication_cache_size,
                    "defaultTtlSeconds": signal_system.default_ttl.as_secs(),
                    "retentionSeconds": signal_system.default_ttl.as_secs(),
                    "channelCapacity": signal_system.deduplication_cache_size,
                    "meshChannel": {
                        "enabled": signal_system.mesh_channel.enabled,
                        "priority": signal_system.mesh_channel.priority,
                        "requireActiveSession": signal_system.mesh_channel.require_active_session,
                    },
                    "btExtensionChannel": {
                        "enabled": signal_system.bt_extension_channel.enabled,
                        "priority": signal_system.bt_extension_channel.priority,
                        "requireActiveSession": signal_system.bt_extension_channel.require_active_session,
                    },
                })
                .to_string(),
            )
        }
        "/api/signals/status"
            if versioned_v0 && state.config.controller_profile == ControllerProfile::Native =>
        {
            let advanced = state.advanced_networking.read().await;
            let signal_system = &advanced.signal_system;
            let active_channels = if signal_system.enabled {
                [
                    signal_system.mesh_channel.enabled.then_some("mesh"),
                    signal_system
                        .bt_extension_channel
                        .enabled
                        .then_some("bt_extension"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({
                    "enabled": signal_system.enabled,
                    "active_channels": active_channels,
                    "statistics": {
                        "signals_sent": 0,
                        "signals_received": 0,
                        "duplicate_signals_dropped": 0,
                        "expired_signals_dropped": 0,
                    },
                })
                .to_string(),
            }
        }
        "/api/signals/status" => {
            let events = state.events.read().await;
            let advanced = state.advanced_networking.read().await;
            let signal_system = &advanced.signal_system;
            let active_channels = [
                signal_system.mesh_channel.enabled.then_some("mesh"),
                signal_system
                    .bt_extension_channel
                    .enabled
                    .then_some("btExtension"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
            routing::ok_response(
                serde_json::json!({
                    "enabled": signal_system.enabled,
                    "signalsReceived": events.records.len(),
                    "signalsPublished": events.records.len(),
                    "duplicateSignalsDropped": 0,
                    "expiredSignalsDropped": 0,
                    "subscriberCount": state.event_tx.receiver_count(),
                    "activeChannels": active_channels,
                })
                .to_string(),
            )
        }
        "/api/songid/capabilities" => {
            let integrations = state.integration_settings.read().await.clone();
            routing::ok_response(songid_capabilities_json(Some(&integrations)).to_string())
        }
        "/api/telemetry/prometheus" => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;
            HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: format!(
                    "# TYPE slskr_transfers gauge\nslskr_transfers {}\n# TYPE slskr_searches gauge\nslskr_searches {}\n",
                    transfers.entries.len(), searches.records.len()
                ),
            }
        }
        // Matches the oracle's GetKpis: always a JSON dictionary keyed by
        // metric name, never text/plain (unlike the base prometheus route,
        // which supports both via content negotiation).
        "/api/telemetry/prometheus/kpis" => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;
            let metrics = serde_json::json!({
                "slskr_transfers": prometheus_metric_json("slskr_transfers", "gauge", transfers.entries.len() as f64),
                "slskr_searches": prometheus_metric_json("slskr_searches", "gauge", searches.records.len() as f64),
            });
            drop(transfers);
            drop(searches);
            routing::ok_response(metrics.to_string())
        }
        "/api/virtualsoulfind/disaster-mode/status" => {
            // The frozen controller reports the coordinator's current level,
            // not a level inferred from configuration or the transport
            // connection snapshot. Its coordinator starts in Normal mode;
            // this compatibility runtime has no coordinator transition yet,
            // so --no-connect and disasterMode.force must not fabricate a
            // FullFallback status.
            let level = virtual_soulfind_disaster_mode_level();
            let (level_name, description) = match level {
                1 => (
                    "SoulseekDegraded",
                    "Legacy fallback assisting: Soulseek degraded, mesh assisting",
                ),
                2 => (
                    "SoulseekUnavailable",
                    "Legacy fallback active: Soulseek unavailable, mesh primary",
                ),
                3 => (
                    "FullFallback",
                    "Legacy full fallback: shadow-index, relay, swarm-only",
                ),
                _ => (
                    "Normal",
                    "Normal default mode: Soulseek + mesh operating together",
                ),
            };
            routing::ok_response(
                serde_json::json!({
                    "level": level,
                    "level_name": level_name,
                    "description": description,
                    "is_active": level != 0,
                    "mode_family": "legacy_fallback",
                    "networks": {
                        "soulseek_available": level <= 1,
                        "mesh_assisting": level >= 1,
                        "mesh_primary": level >= 2,
                        "full_fallback": level >= 3,
                    },
                })
                .to_string(),
            )
        }
        _ => routing::not_found_response(),
    }
}
