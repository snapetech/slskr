use super::*;

pub(crate) async fn extended_controller_dynamic_get_response(
    path: &str,
    query: Option<&str>,
    state: &AppState,
    versioned_v0: bool,
) -> HttpResponse {
    if path.starts_with("/api/audio/canonical/") || path.starts_with("/api/audio/variants/dedupe/")
    {
        if let Some(db) = state.db.as_ref() {
            if db.get_traffic_totals().await.is_err() {
                return routing::service_unavailable_response("audio storage unavailable");
            }
        }
    }
    if path.starts_with("/api/bridge/transfer/") && path.ends_with("/progress") {
        return bridge_transfer_progress_response(path, state).await;
    }
    if let Some(recording_id) = path_segment_after(path, "/api/audio/canonical/") {
        let recording_id = decoded_path_segment(recording_id);
        let discovery = state.content_discovery.read().await;
        let candidates = discovery
            .hash_entries()
            .iter()
            .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(&recording_id))
            .map(|entry| {
                serde_json::json!({
                    "codecProfile": "flac",
                    "flacKey": entry.flac_key,
                    "byteHash": entry.byte_hash,
                    "fullFileHash": entry.full_file_hash,
                    "fileSha256": entry.file_sha256,
                    "size": entry.size,
                    "useCount": entry.use_count,
                })
            })
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({"recordingId": recording_id, "candidates": candidates}).to_string(),
        );
    }
    if let Some(recording_id) = path_segment_after(path, "/api/audio/variants/dedupe/") {
        let recording_id = decoded_path_segment(recording_id);
        let discovery = state.content_discovery.read().await;
        let entries = discovery
            .hash_entries()
            .iter()
            .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(&recording_id))
            .collect::<Vec<_>>();
        let mut groups = BTreeMap::<String, Vec<serde_json::Value>>::new();
        for entry in entries {
            let key = [&entry.file_sha256, &entry.full_file_hash, &entry.byte_hash]
                .into_iter()
                .find(|value| !value.is_empty())
                .cloned()
                .unwrap_or_else(|| entry.flac_key.clone());
            groups
                .entry(key)
                .or_default()
                .push(serde_json::json!(entry));
        }
        let groups = groups
            .into_iter()
            .map(|(hash, variants)| serde_json::json!({"hash": hash, "variants": variants}))
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({"recordingId": recording_id, "groups": groups}).to_string(),
        );
    }
    if let Some(username) = path_segment_between(path, "/api/compatibility/users/", "/browse") {
        let username = decoded_path_segment(username);
        let browse = state.browse.read().await;
        let Some(record) = browse.get(&username) else {
            return routing::not_found_response();
        };
        let directories = group_browse_entries(&record.entries)
            .into_iter()
            .map(|(name, files)| {
                serde_json::json!({
                    "name": name,
                    "files": files.into_iter().map(|file| serde_json::json!({
                        "filename": file.filename,
                        "size": file.size,
                        "attributes": [file.extension],
                    })).collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({"username": username, "directories": directories}).to_string(),
        );
    }
    if let Some(id) = path_segment_after(path, "/api/downloads/") {
        if path.starts_with("/api/downloads/requests/") {
            return routing::not_found_response();
        }
        let id = decoded_path_segment(id);
        if state.config.controller_profile == ControllerProfile::Native
            && uuid::Uuid::parse_str(&id).is_err()
        {
            return routing::bad_request_response("Invalid download ID format");
        }
        let transfers = state.transfers.read().await;
        let transfer = transfers.entries.iter().find(|entry| {
            entry.direction == 0
                && (entry.id.to_string() == id
                    || entry.request_id.as_deref().is_some_and(|request_id| {
                        request_id.eq_ignore_ascii_case(&id)
                            || (uuid::Uuid::parse_str(request_id).ok()
                                == uuid::Uuid::parse_str(&id).ok())
                    }))
        });
        let Some(transfer) = transfer else {
            return routing::not_found_response();
        };
        let size = transfer.size.unwrap_or(0);
        return routing::ok_response(
            serde_json::json!({
                "Id": transfer.request_id.as_deref().unwrap_or(&id),
                "User": transfer.peer_username,
                "RemotePath": transfer.filename,
                "LocalPath": transfer.local_path,
                "Status": native_download_status(&transfer.status),
                "Progress": if size == 0 { 0.0 } else { transfer.bytes_transferred as f64 / size as f64 },
                "Size": size,
                "Remaining": size.saturating_sub(transfer.bytes_transferred),
                "Speed": transfer.average_speed_at(unix_timestamp()),
            })
            .to_string(),
        );
    }
    if let Some(import_id) = path_segment_after(path, "/api/source-feed-imports/history/") {
        let import_id = decoded_path_segment(import_id);
        let history = state.source_feed_import_history.read().await;
        return history
            .history
            .iter()
            .find(|entry| entry["importId"].as_str() == Some(import_id.as_str()))
            .map_or_else(routing::not_found_response, |entry| HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: entry.to_string(),
            });
    }
    if let Some(username) = path_segment_after(path, "/api/capabilities/peers/") {
        let username = decoded_path_segment(username).trim().to_owned();
        if versioned_v0 && username.is_empty() {
            return routing::bad_request_response("Username is required");
        }
        let mesh = state.mesh.read().await;
        let record = mesh
            .capability_service_peers_json()
            .into_iter()
            .find(|record| {
                record["username"]
                    .as_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case(&username))
            });
        return record.map_or_else(
            || {
                if versioned_v0 {
                    HttpResponse {
                        status: "404 Not Found",
                        content_type: "application/json",
                        body: serde_json::json!({
                            "error": "No capabilities known for peer",
                        })
                        .to_string(),
                    }
                } else {
                    routing::not_found_response()
                }
            },
            |record| routing::ok_response(record.to_string()),
        );
    }
    if let Some(size) = path_segment_after(path, "/api/hashdb/inventory/by-size/") {
        let size = if versioned_v0 {
            let Ok(size) = size.parse::<i64>() else {
                return routing::bad_request_response("size must be an integer");
            };
            size
        } else {
            let Ok(size) = size.parse::<u64>() else {
                return routing::bad_request_response("size must be an unsigned integer");
            };
            i64::try_from(size).unwrap_or(i64::MAX)
        };
        let limit = match query_parameter(query, "limit") {
            None => 100,
            Some(value) => match value.parse::<i64>() {
                Ok(value) => usize::try_from(value.max(1))
                    .unwrap_or(1_000)
                    .clamp(1, 1_000),
                Err(_) if versioned_v0 => return routing::bad_request_response("limit is invalid"),
                Err(_) => 100,
            },
        };
        let entries = if versioned_v0 {
            let features = state.controller_features.read().await;
            features
                .values_with_prefix(HASHDB_FLAC_INVENTORY_PREFIX)
                .into_iter()
                .filter(|entry| entry.get("size").and_then(serde_json::Value::as_i64) == Some(size))
                .take(limit)
                .collect::<Vec<_>>()
        } else {
            let discovery = state.content_discovery.read().await;
            discovery
                .hashes_by_size(u64::try_from(size).unwrap_or(0))
                .into_iter()
                .take(limit)
                .map(|entry| serde_json::to_value(entry).unwrap_or_else(|_| serde_json::json!({})))
                .collect::<Vec<_>>()
        };
        return routing::ok_response(
            serde_json::json!({"count": entries.len(), "entries": entries}).to_string(),
        );
    }
    if let Some(since) = path_segment_after(path, "/api/hashdb/sync/since/") {
        let since = match since.parse::<i64>() {
            Ok(value) => u64::try_from(value.max(0)).unwrap_or(0),
            Err(_) => return routing::bad_request_response("sinceSeq must be an integer"),
        };
        let limit = match query_parameter(query, "limit") {
            Some(raw) => match raw.parse::<i64>() {
                Ok(value) => usize::try_from(value.max(0)).unwrap_or(usize::MAX),
                Err(_) => return routing::bad_request_response("limit is invalid"),
            },
            None => 1_000,
        };
        let discovery = state.content_discovery.read().await;
        let (entries, _) = discovery.hash_entries_since_seq(since, limit);
        let latest_seq = discovery.latest_seq();
        let count = entries.len();
        return routing::ok_response(
            serde_json::json!({
                "latestSeq": latest_seq,
                "count": count,
                "entries": entries,
            })
            .to_string(),
        );
    }
    if let Some(segments) = decoded_segments_after(path, "/api/listening-party/radio/") {
        if let [party_id, content_id] = segments.as_slice() {
            // Matches the oracle's real StreamListedParty gate: streaming
            // must be enabled, the party must be listed and currently
            // playing this exact content, and the ticket must be owned by
            // this exact party. The HTTP server upgrades this successful
            // controller result to the real range-capable file response.
            if !state.media_services.read().await.features.streaming {
                return routing::not_found_response();
            }
            let now_ms = unix_timestamp_millis();
            let listed = state
                .controller_features
                .read()
                .await
                .values_with_prefix("listening-party/")
                .into_iter()
                .any(|event| {
                    listening_party_event_window(&event, now_ms).is_some()
                        && event.get("partyId").and_then(serde_json::Value::as_str)
                            == Some(party_id)
                        && event.get("listed").and_then(serde_json::Value::as_bool) == Some(true)
                        && event
                            .get("allowMeshStreaming")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                        && event.get("contentId").and_then(serde_json::Value::as_str)
                            == Some(content_id)
                });
            if !listed {
                return routing::not_found_response();
            }
            let ticket_token = query_parameter(query, "ticket").unwrap_or_default();
            let ticket = if ticket_token.trim().is_empty() {
                None
            } else {
                state.stream_tickets.write().await.get(&ticket_token)
            };
            let owner_key = format!("listening-party:{party_id}");
            if !ticket.is_some_and(|ticket| {
                ticket.family == "listening-party"
                    && ticket.source == owner_key
                    && ticket.content_id == *content_id
            }) {
                return routing::unauthorized_response();
            }
            let discovery = state.content_discovery.read().await;
            let record = discovery
                .shadow_records()
                .iter()
                .find(|record| record.recording_id.eq_ignore_ascii_case(content_id));
            return routing::ok_response(
                serde_json::json!({
                    "partyId": party_id,
                    "contentId": content_id,
                    "available": record.is_some(),
                    "peerIds": record.map(|record| record.peer_ids.clone()).unwrap_or_default(),
                })
                .to_string(),
            );
        }
    }
    if let Some(segments) = decoded_segments_after(path, "/api/listening-party/") {
        if let [pod_id, channel_id] = segments.as_slice() {
            let pods = state.pods.read().await;
            let pod_exists = pods.get(pod_id).is_some();
            if pod_exists {
                if !pods.channel_exists(pod_id, channel_id) {
                    return routing::not_found_response();
                }
                // Matches the oracle's ListeningPartyController.Get: requires
                // real pod membership, then returns the real stored
                // ListeningPartyEvent that the sibling POST handler writes
                // (or 204 if there is none) -- never the pod object plus raw
                // chat history, and never without a membership check. This
                // check also applies in the local no-auth test mode: the
                // configured Soulseek identity is still the acting peer.
                let Some(peer_id) = pod_request_peer_id(state).await else {
                    drop(pods);
                    return routing::forbidden_response("Authenticated peer identity is required");
                };
                if !pods.is_member(pod_id, &peer_id) {
                    drop(pods);
                    return routing::forbidden_response("Pod membership is required");
                }
            } else if state.config.auth_required || !pod_id.starts_with("pod:") {
                // An authenticated request for an unknown pod is a real
                // missing-resource response. In the no-auth compatibility
                // mode, retain the empty-state behavior for a syntactically
                // valid pod identifier used by route probes.
                drop(pods);
                return routing::not_found_response();
            }
            drop(pods);
            let event = state
                .controller_features
                .read()
                .await
                .get(&format!("listening-party/{pod_id}/{channel_id}"))
                .cloned();
            return match event {
                Some(event) => routing::ok_response(event.to_string()),
                None => routing::no_content_response(),
            };
        }
    }
    if let Some(flac_key) = path_segment_after(path, "/api/mesh/lookup/") {
        let flac_key = decoded_path_segment(flac_key).trim().to_owned();
        if flac_key.is_empty() {
            return routing::bad_request_response("flacKey required");
        }
        let discovery = state.content_discovery.read().await;
        return discovery.lookup_hash(&flac_key).map_or_else(
            || HttpResponse {
                status: "404 Not Found",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({"found": false}).to_string(),
            },
            |entry| {
                routing::ok_response(serde_json::json!({"found": true, "entry": entry}).to_string())
            },
        );
    }
    if let Some(username) = path_segment_between(path, "/api/multisource/users/", "/files") {
        let username = decoded_path_segment(username);
        if versioned_v0 && state.config.controller_profile == ControllerProfile::Native {
            let username = username.trim().to_owned();
            if username.is_empty() {
                return routing::bad_request_response("Username is required");
            }
            let filter = query_parameter(query, "filter")
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty());
            let searches = state.searches.read().await;
            let mut files = searches
                .records
                .iter()
                .flat_map(|record| record.results.iter().map(move |result| (record, result)))
                .filter(|(_, result)| {
                    result
                        .peer_username
                        .as_deref()
                        .is_some_and(|peer| peer.eq_ignore_ascii_case(&username))
                })
                .filter(|(_, result)| {
                    filter.as_deref().is_none_or(|filter| {
                        result
                            .filename
                            .to_ascii_lowercase()
                            .contains(&filter.to_ascii_lowercase())
                    })
                })
                .map(|(record, result)| {
                    let full_path = result.filename.clone();
                    let filename = virtual_basename(&full_path).to_owned();
                    let extension = filename
                        .rsplit_once('.')
                        .map(|(_, extension)| format!(".{extension}").to_ascii_lowercase())
                        .unwrap_or_default();
                    (
                        record.query.clone(),
                        serde_json::json!({
                            "filename": filename,
                            "fullPath": full_path,
                            "size": result.size,
                            "sizeMB": format!("{:.1} MB", result.size as f64 / 1024.0 / 1024.0),
                            "bitRate": null,
                            "sampleRate": null,
                            "bitDepth": null,
                            "extension": extension,
                        }),
                    )
                })
                .collect::<Vec<_>>();
            files.sort_by(|left, right| {
                left.1["fullPath"]
                    .as_str()
                    .cmp(&right.1["fullPath"].as_str())
            });
            let last_query = files
                .first()
                .map(|(query, _)| query.clone())
                .unwrap_or_default();
            let file_values = files
                .into_iter()
                .enumerate()
                .map(|(index, (_, mut file))| {
                    file["index"] = serde_json::json!(index + 1);
                    file
                })
                .collect::<Vec<_>>();
            let mut directories = BTreeMap::<String, Vec<serde_json::Value>>::new();
            for file in &file_values {
                let full_path = file["fullPath"].as_str().unwrap_or_default();
                let directory = full_path
                    .rsplit_once(['/', '\\'])
                    .map(|(directory, _)| directory.replace('\\', "/"))
                    .unwrap_or_default();
                directories.entry(directory).or_default().push(file.clone());
            }
            let directories = directories
                .into_iter()
                .map(|(full_directory, files)| {
                    serde_json::json!({
                        "directory": full_directory.rsplit('/').next().unwrap_or_default(),
                        "fullDirectory": full_directory,
                        "fileCount": files.len(),
                        "files": files.into_iter().take(20).collect::<Vec<_>>(),
                    })
                })
                .collect::<Vec<_>>();
            return routing::ok_response(
                serde_json::json!({
                    "username": username,
                    "lastQuery": last_query,
                    "filter": filter,
                    "totalFiles": file_values.len(),
                    "directories": directories,
                    "hint": "Pick a file and use POST /api/v0/multisource/file-sources with {filename, size} to find other sources",
                })
                .to_string(),
            );
        }
        let searches = state.searches.read().await;
        let files = searches
            .records
            .iter()
            .flat_map(|record| record.results.iter())
            .filter(|entry| {
                entry
                    .peer_username
                    .as_deref()
                    .is_some_and(|peer| peer.eq_ignore_ascii_case(&username))
            })
            .map(SearchResultEntry::controller_file_json)
            .take(1_000)
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({"username": username, "files": files, "count": files.len()})
                .to_string(),
        );
    }
    if path.starts_with("/api/musicbrainz/") {
        return musicbrainz_dynamic_get_response(path, state).await;
    }
    if let Some(job_id) = path_segment_between(path, "/api/playback/", "/diagnostics") {
        let job_id = decoded_path_segment(job_id).trim().to_owned();
        if job_id.is_empty() {
            return routing::bad_request_response("jobId is required");
        }
        // Matches the oracle's real GetDiagnostics: this is the job's
        // recorded playback-feedback state (position/buffer/priority), not
        // multisource swarm-download progress -- 404 when no feedback has
        // ever been posted for this jobId, regardless of whether a
        // download job with that id happens to exist.
        let feedback = state
            .controller_features
            .read()
            .await
            .values_with_prefix(&format!("playback/feedback/{job_id}/"));
        let Some(latest) = latest_playback_feedback(&feedback) else {
            return routing::not_found_response();
        };
        return routing::ok_response(
            serde_json::json!({
                "jobId": job_id,
                "trackId": latest.get("trackId").cloned().unwrap_or(serde_json::Value::Null),
                "positionMs": latest.get("positionMs").and_then(serde_json::Value::as_i64).unwrap_or(0),
                "bufferAheadMs": latest.get("bufferAheadMs").and_then(serde_json::Value::as_i64).unwrap_or(0),
                "priority": playback_priority_for_latest_feedback(Some(latest)),
            })
            .to_string(),
        );
    }
    if path.starts_with("/api/podcore/") {
        return podcore_dynamic_get_response(path, query, state, versioned_v0).await;
    }
    if let Some(local_port) = path_segment_after(path, "/api/portforwarding/status/") {
        let Ok(local_port) = local_port.parse::<u16>() else {
            return if versioned_v0 {
                routing::not_found_response()
            } else {
                routing::bad_request_response("localPort must be between 1 and 65535")
            };
        };
        return state.port_forwarding.status(local_port).await.map_or_else(
            routing::not_found_response,
            |status| {
                routing::ok_response(
                    serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_owned()),
                )
            },
        );
    }
    if path.starts_with("/api/quarantine-jury/requests/") {
        return quarantine_dynamic_get_response(path, state).await;
    }
    if let Some(username) = path_segment_after(path, "/api/ranking/history/") {
        if let Some(response) = ranking_storage_failure_response(state).await {
            return response;
        }
        let username = decoded_path_segment(username).trim().to_owned();
        if username.is_empty() {
            return ranking_bad_request_response("Username is required");
        }
        let transfers = state.transfers.read().await;
        let (successes, failures) = ranking_history_counts(&transfers, &username);
        return routing::ok_response(
            ranking_history_json(&username, successes, failures).to_string(),
        );
    }
    if path.starts_with("/api/realm-subject-indexes/") {
        return realm_subject_dynamic_get_response(path, state).await;
    }
    if let Some(content_id) = path_segment_after(path, "/api/relay/streams/") {
        let content_id = decoded_path_segment(content_id);
        let transfers = state.transfers.read().await;
        let transfer = transfers.entries.iter().find(|entry| {
            entry.filename == content_id
                || stable_content_hash(&entry.filename, entry.size.unwrap_or(0)).to_string()
                    == content_id
        });
        return transfer.map_or_else(routing::not_found_response, |entry| {
            routing::ok_response(serde_json::json!({
                "contentId": content_id,
                "transfer": serde_json::from_str::<serde_json::Value>(&entry.json()).unwrap_or_default(),
            }).to_string())
        });
    }
    if let Some(job_id) = path_segment_between(path, "/api/traces/", "/summary") {
        let job_id = decoded_path_segment(job_id);
        let jobs = state.multisource.read().await;
        let job = jobs.get(&job_id);
        let total_events = job
            .and_then(|job| job.result.as_ref())
            .map(|result| result.chunks.len())
            .unwrap_or(0);
        return routing::ok_response(
            serde_json::json!({
                "jobId": job_id,
                "firstEventAt": job.map(|job| job.created_at),
                "lastEventAt": job.map(|job| job.updated_at),
                "duration": job.map(|job| job.updated_at.saturating_sub(job.created_at)),
                "totalEvents": total_events,
                "eventCounts": {},
                "bytesBySource": {},
                "bytesByBackend": {},
                "peers": [],
                "rescueInvoked": false,
            })
            .to_string(),
        );
    }
    if let Some(mbid) = path_segment_after(path, "/api/virtualsoulfind/canonical/") {
        let mbid = decoded_path_segment(mbid);
        let discovery = state.content_discovery.read().await;
        if state.config.controller_profile == ControllerProfile::Native {
            let variants = virtual_soulfind_legacy_variants(&discovery, &mbid);
            let canonical_variant = variants.first().cloned();
            let has_canonical_variant = canonical_variant.is_some();
            let available_variants = variants.len();
            return routing::ok_response(
                serde_json::json!({
                    "canonical_variant": canonical_variant,
                    "available_variants": available_variants,
                    "selection_reason": if has_canonical_variant {
                        "Selected highest quality variant from shadow index"
                    } else {
                        "No variants found in shadow index"
                    },
                })
                .to_string(),
            );
        }
        let hashes = discovery
            .hash_entries()
            .iter()
            .filter(|entry| entry.music_brainz_id.eq_ignore_ascii_case(&mbid))
            .collect::<Vec<_>>();
        let peers = discovery.peer_ids_for_recordings(std::slice::from_ref(&mbid));
        return routing::ok_response(
            serde_json::json!({
                "mbid": mbid,
                "canonical": hashes.first(),
                "variants": hashes.clone(),
                "peerIds": peers,
                "available_variants": hashes,
                "selection_reason": if hashes.is_empty() { "no_variants" } else { "highest_local_confidence" },
            })
            .to_string(),
        );
    }
    routing::not_found_response()
}
