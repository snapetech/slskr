async fn route_dispatch_group_6(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    } = *context;
    match (method, normalized_path) {
        ("PUT", path) if path.ends_with("/adversarial") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.contains("/disclosure/") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("PUT", path) if path.ends_with("/reputation") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("GET", "/api/config/shares") => {
            let shares = state.shares.read().await;
            let share_roots: Vec<String> = shares.roots
                .iter()
                .map(|root| format!(
                    "{{\"label\":\"{}\",\"files\":{},\"bytes\":{}}}",
                    json_escape(&root.label),
                    root.files,
                    root.bytes
                ))
                .collect();
            let json = format!(
                "{{\"roots\":[{}],\"count\":{}}}",
                share_roots.join(","),
                shares.roots.len()
            );
            drop(shares);
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/shares") => {
            let path = extract_json_string_field(body, "path").unwrap_or_default();
            if path.is_empty() {
                return Ok(routing::bad_request_response("path is required"));
            }
            let json = format!(
                "{{\"path\":\"{}\",\"added\":true,\"files\":0,\"bytes\":0}}",
                json_escape(&path)
            );
            Ok(routing::created_response(json))
        }

        ("GET", "/api/config/plugins") => {
            let integrations = state.integration_settings.read().await;
            let plugins = serde_json::json!([
                {
                    "id": "spotify",
                    "name": "Spotify",
                    "enabled": integrations.spotify.enabled,
                    "configured": integrations.spotify.configured(),
                },
                {
                    "id": "lidarr",
                    "name": "Lidarr",
                    "enabled": integrations.lidarr.enabled,
                    "configured": integrations.lidarr.configured(),
                },
                {
                    "id": "external-visualizer",
                    "name": "External Visualizer",
                    "enabled": state.config.integrations.external_visualizer.launch_enabled,
                    "configured": state.config.integrations.external_visualizer.configured(),
                },
                {
                    "id": "bridge",
                    "name": "Bridge",
                    "enabled": state.config.integrations.bridge.enabled,
                    "configured": state.config.integrations.bridge.enabled,
                }
            ]);
            let json = serde_json::json!({
                "plugins": plugins,
                "count": plugins.as_array().map_or(0, Vec::len),
            }).to_string();
            Ok(routing::ok_response(json))
        }

        ("POST", "/api/config/filters") => {
            let filter_type = extract_json_string_field(body, "type").unwrap_or_default();
            let pattern = extract_json_string_field(body, "pattern").unwrap_or_default();
            let json = format!(
                "{{\"type\":\"{}\",\"pattern\":\"{}\",\"created_at\":{}}}",
                json_escape(&filter_type),
                json_escape(&pattern),
                unix_timestamp()
            );
            Ok(routing::created_response(json))
        }

        // ADMIN/SYSTEM ENDPOINTS
        ("GET", "/api/admin/stats") => {
            let transfers = state.transfers.read().await;
            let total_bytes = transfers
                .entries
                .iter()
                .map(|entry| entry.bytes_transferred)
                .sum::<u64>();
            let active_transfers = transfers
                .entries
                .iter()
                .filter(|entry| matches!(entry.status.as_str(), "queued" | "requested" | "in_progress"))
                .count();
            let searches = state.searches.read().await;
            let users = state.users.read().await;
            let rooms = state.rooms.read().await;
            let shares = state.shares.read().await;
            let json = serde_json::json!({
                "total_transfers": transfers.entries.len(),
                "active_transfers": active_transfers,
                "total_bytes": total_bytes,
                "searches": searches.records.len(),
                "users": users.records.len(),
                "rooms": rooms.records.len(),
                "shared_files": shares.entries.len(),
            }).to_string();
            drop(shares);
            drop(rooms);
            drop(users);
            drop(searches);
            drop(transfers);
            Ok(routing::ok_response(json))
        }

        // RECOMMENDATIONS & ANALYTICS ENDPOINTS
        ("GET", "/api/soulseek/recommendations") => {
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_recommendations_json()
            } else {
                interests.recommendations_json("recommendations")
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/soulseek/recommendations/global") => {
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_recommendations_json()
            } else {
                interests.recommendations_json("global_recommendations")
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/items/") && path.ends_with("/recommendations") => {
            let Some(item_id) =
                path_segment_between(path, "/api/soulseek/items/", "/recommendations")
            else {
                return Ok(routing::not_found_response());
            };
            let item_id = decoded_path_segment(item_id).trim().to_owned();
            if item_id.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            let interests = state.interests.read().await;
            let json = if route.path.starts_with("/api/v0/") {
                interests.versioned_item_recommendations_json(&item_id)
            } else {
                interests.item_recommendations_json(&item_id)
            };
            drop(interests);
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/soulseek/items/") && path.ends_with("/similar-users") => {
            let Some(item_id) =
                path_segment_between(path, "/api/soulseek/items/", "/similar-users")
            else {
                return Ok(routing::not_found_response());
            };
            let item_id = decoded_path_segment(item_id).trim().to_owned();
            if item_id.is_empty() {
                return Ok(routing::bad_request_response("item is required"));
            }
            let users = state.users.read().await;
            if route.path.starts_with("/api/v0/") {
                let usernames = users
                    .records
                    .iter()
                    .filter(|user| user.watched || user.status.as_deref() == Some("online"))
                    .map(|user| user.username.clone())
                    .collect::<Vec<_>>();
                let json = serde_json::json!({
                    "item": item_id,
                    "usernames": usernames,
                })
                .to_string();
                drop(users);
                return Ok(routing::ok_response(json));
            }
            let similar_users = users
                .records
                .iter()
                .filter(|user| user.watched || user.status.as_deref() == Some("online"))
                .map(|user| {
                    serde_json::json!({
                        "username": user.username,
                        "status": user.status,
                        "watched": user.watched,
                        "score": 1.0,
                    })
                })
                .collect::<Vec<_>>();
            let count = similar_users.len();
            drop(users);
            let json = serde_json::json!({
                "item_id": item_id,
                "similar_users": similar_users,
                "count": count,
            })
            .to_string();
            Ok(routing::ok_response(json))
        }

        ("GET", path) if path.starts_with("/api/relay/controller/downloads/") => {
            let Some(_token) = path_segment_after(path, "/api/relay/controller/downloads/") else {
                return Ok(routing::not_found_response());
            };
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/octet-stream",
                body: String::new(),
            })
        }

        ("POST", path) if path.starts_with("/api/relay/controller/files/") => {
            let Some(token) = path_segment_after(path, "/api/relay/controller/files/") else {
                return Ok(routing::not_found_response());
            };
            let token = decoded_path_segment(token);
            let relay = state.relay.read().await;
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "accepted": true,
                "token": token,
                "relay_enabled": relay.enabled,
                "relayAgentEnabled": runtime.relay_agent_enabled,
                "kind": "files",
                "updated_at": runtime.updated_at.max(relay.updated_at),
            }).to_string();
            drop(runtime);
            drop(relay);
            Ok(routing::ok_response(body))
        }

        ("POST", path) if path.starts_with("/api/relay/controller/shares/") => {
            let Some(token) = path_segment_after(path, "/api/relay/controller/shares/") else {
                return Ok(routing::not_found_response());
            };
            let token = decoded_path_segment(token);
            let shares = state.shares.read().await;
            let relay = state.relay.read().await;
            let runtime = state.runtime.read().await;
            let body = serde_json::json!({
                "accepted": true,
                "token": token,
                "relay_enabled": relay.enabled,
                "relayAgentEnabled": runtime.relay_agent_enabled,
                "kind": "shares",
                "shareCount": shares.entries.len(),
                "rootCount": shares.roots.len(),
                "updated_at": runtime.updated_at.max(relay.updated_at),
            }).to_string();
            drop(runtime);
            drop(relay);
            drop(shares);
            Ok(routing::ok_response(body))
        }

         // ADDITIONAL MISSING GET ENDPOINTS (Phase 5)
         ("GET", "/api/source-providers") => Ok(routing::ok_response(
             source_provider_catalog_json(state.config.acquisition_planning_enabled),
         )),

         ("GET", "/api/discovery") => {
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches);
             let total_users = sources
                 .iter()
                 .filter_map(|source| source.get("username").and_then(serde_json::Value::as_str))
                 .map(str::to_ascii_lowercase)
                 .collect::<HashSet<_>>()
                 .len();
             Ok(routing::ok_response(serde_json::json!({
                 "isRunning": discovery.running,
                 "currentSearchTerm": discovery.search_term,
                 "stats": {
                     "totalFiles": sources.len(),
                     "totalUsers": total_users,
                     "searchCycles": discovery.search_cycles,
                     "lastCycleNewFiles": discovery.last_cycle_new_files,
                     "hashVerificationEnabled": discovery.hash_verification_enabled,
                     "filesWithHash": sources.iter().filter(|source| !source["hash"].is_null()).count(),
                 }
             }).to_string()))
         }
         ("POST", "/api/discovery/start") => {
             let search_term = extract_json_string_field(body, "searchTerm")
                 .unwrap_or_default()
                 .trim()
                 .to_owned();
             if search_term.is_empty() {
                 return Ok(routing::bad_request_response("SearchTerm is required"));
             }
            if state.source_discovery.read().await.running {
                let current = state.source_discovery.read().await.search_term.clone();
                return Ok(routing::HttpResponse {
                    status: "409 Conflict",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "error": "Discovery already running",
                        "currentSearchTerm": current,
                        "hint": "Call /api/v0/discovery/stop first",
                    })
                    .to_string(),
                });
            }
            let hash_verification_enabled =
                extract_json_bool_field(body, "enableHashVerification").unwrap_or(true);
            let search_term = truncate_utf8_bytes(search_term, MAX_SEARCH_QUERY_BYTES);
            let record = match dispatch_source_discovery_search(state, search_term.clone()).await {
                Ok(record) => record,
                Err(error) if error == "session manager is not running" => {
                    match create_rescue_search(state, search_term.clone()).await {
                        Ok(record) => record,
                        Err(error) => return Ok(routing::service_unavailable_response(&error)),
                    }
                }
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
             state.source_discovery.write().await.start(
                 search_term.clone(),
                 hash_verification_enabled,
                 record.token,
             );
             Ok(routing::ok_response(serde_json::json!({
                 "message": "Discovery started",
                 "searchTerm": search_term,
                 "hashVerificationEnabled": hash_verification_enabled,
             }).to_string()))
        }
        ("POST", "/api/discovery/stop") => {
            if !state.source_discovery.read().await.running {
                return Ok(routing::ok_response(
                    serde_json::json!({"message": "Discovery not running"}).to_string(),
                ));
            }
            let mut discovery = state.source_discovery.write().await;
            discovery.running = false;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches);
             discovery.last_cycle_new_files = sources.len();
             let total_users = sources
                 .iter()
                 .filter_map(|source| source.get("username").and_then(serde_json::Value::as_str))
                 .map(str::to_ascii_lowercase)
                 .collect::<HashSet<_>>()
                 .len();
             Ok(routing::ok_response(serde_json::json!({
                 "message": "Discovery stopped",
                 "stats": {
                     "totalFiles": sources.len(),
                     "totalUsers": total_users,
                     "searchCycles": discovery.search_cycles,
                     "lastCycleNewFiles": discovery.last_cycle_new_files,
                     "hashVerificationEnabled": discovery.hash_verification_enabled,
                     "filesWithHash": 0,
                 }
             }).to_string()))
         }
         ("GET", path) if path.starts_with("/api/discovery/sources/by-size/") => {
             let Some(size) = path_segment_after(path, "/api/discovery/sources/by-size/")
                 .and_then(|size| size.parse::<u64>().ok())
                 .filter(|size| *size > 0)
             else {
                 return Ok(routing::bad_request_response("size must be greater than zero"));
             };
             let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                 Ok(limit) => limit.unwrap_or(100),
                 Err(()) => return Ok(routing::bad_request_response("limit must be greater than zero")),
             };
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches)
                 .into_iter()
                 .filter(|source| source["size"].as_u64() == Some(size))
                 .take(limit)
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "size": size,
                 "sourceCount": sources.len(),
                 "sources": sources,
             }).to_string()))
         }
         ("GET", "/api/discovery/sources/by-filename") => {
             let pattern = query_parameter(route.query, "pattern")
                 .unwrap_or_default()
                 .trim()
                 .to_owned();
             if pattern.is_empty() {
                 return Ok(routing::bad_request_response("pattern query parameter is required"));
             }
             let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                 Ok(limit) => limit.unwrap_or(100),
                 Err(()) => return Ok(routing::bad_request_response("limit must be greater than zero")),
             };
             let needle = pattern.to_ascii_lowercase();
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&discovery, &searches)
                 .into_iter()
                 .filter(|source| source["filename"].as_str().is_some_and(|filename| filename.to_ascii_lowercase().contains(&needle)))
                 .take(limit)
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "pattern": pattern,
                 "sourceCount": sources.len(),
                 "sources": sources,
             }).to_string()))
         }
         ("GET", "/api/discovery/summaries") => {
             let min_sources = match query_bounded_usize(route.query, "minSources", 1, 1_000) {
                 Ok(minimum) => minimum.unwrap_or(2),
                 Err(()) => return Ok(routing::bad_request_response("minSources must be greater than zero")),
             };
             let discovery = state.source_discovery.read().await;
             let searches = state.searches.read().await;
             let mut grouped = BTreeMap::<u64, (HashSet<String>, String)>::new();
             for source in source_discovery_sources(&discovery, &searches) {
                 let size = source["size"].as_u64().unwrap_or(0);
                 let username = source["username"].as_str().unwrap_or_default().to_ascii_lowercase();
                 let filename = source["filename"].as_str().unwrap_or_default().to_owned();
                 let entry = grouped.entry(size).or_insert_with(|| (HashSet::new(), filename));
                 entry.0.insert(username);
             }
             let summaries = grouped
                 .into_iter()
                 .filter(|(_, (users, _))| users.len() >= min_sources)
                 .map(|(size, (users, filename))| serde_json::json!({
                     "size": size,
                     "sourceCount": users.len(),
                     "sampleFilename": filename,
                 }))
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "minSources": min_sources,
                 "count": summaries.len(),
                 "summaries": summaries,
             }).to_string()))
         }
         ("GET", "/api/discovery/no-partial-count") => Ok(routing::ok_response(
             serde_json::json!({
                 "usersWithoutPartialSupport": 0,
                 "message": "0 users are flagged as not supporting partial/chunked downloads",
             }).to_string(),
         )),
         ("POST", "/api/discovery/reset-partial-flags") => Ok(routing::ok_response(
             serde_json::json!({
                 "message": "Reset partial support flags for 0 users. They will be tried again on next swarm.",
             }).to_string(),
         )),

         ("GET", "/api/source-feeds") => {
             let wishlist = state.wishlist.read().await;
             let items = wishlist
                 .records
                 .iter()
                 .flat_map(|record| record.items.iter())
                 .map(|item| {
                     let item_json = serde_json::from_str::<serde_json::Value>(&item.json())
                         .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                     serde_json::json!({
                         "id": format!("wishlist-{}", item.id),
                         "name": item.search_text(),
                         "provider": "wishlist",
                         "enabled": true,
                         "items": [item_json],
                     })
                 })
                 .collect::<Vec<_>>();
             let count = items.len();
             drop(wishlist);
             Ok(routing::ok_response(serde_json::json!({
                 "feeds": items,
                 "count": count,
             }).to_string()))
         }

         ("POST", "/api/source-feeds") => {
             let name = extract_json_string_field(body, "name")
                 .or_else(|| extract_json_string_field(body, "title"))
                 .unwrap_or_else(|| "source feed".to_owned());
             let raw = extract_json_string_field(body, "text")
                 .or_else(|| extract_json_string_field(body, "content"))
                 .or_else(|| extract_json_string_field(body, "playlist"))
                 .unwrap_or_default();
             let parsed_items = raw
                 .lines()
                 .map(str::trim)
                 .filter(|line| !line.is_empty())
                 .map(|line| {
                     let (artist, title) = line
                         .split_once(" - ")
                         .map(|(artist, title)| (artist.trim().to_owned(), title.trim().to_owned()))
                         .unwrap_or_else(|| (String::new(), line.to_owned()));
                     (artist, title, "SourceFeed".to_owned())
             })
             .collect::<Vec<_>>();
             let mut wishlist = state.wishlist.write().await;
             let previous = wishlist.clone();
             if !wishlist.can_add_items(parsed_items.len()) {
                 return Ok(routing::service_unavailable_response("wishlist item capacity is full"));
             }
             let mut items = Vec::new();
             let mut persisted_items = Vec::new();
             for (artist, title, kind) in parsed_items {
                 let item = wishlist
                     .add_item(artist, title, kind)
                     .map_err(|_| "wishlist capacity changed unexpectedly".to_owned())?;
                 let value = serde_json::from_str::<serde_json::Value>(&item.json())
                     .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                 persisted_items.push(item);
                 items.push(value);
             }
             let count = items.len();
             let mutated = wishlist.clone();
             drop(wishlist);
             if let Err(error) = persist_wishlist_items_checked(state, &persisted_items).await {
                 rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             Ok(routing::created_response(serde_json::json!({
                 "id": format!("source-feed-{}", unix_timestamp()),
                 "name": name,
                 "enabled": true,
                 "items": items,
                 "count": count,
                 "provider": "manual",
                 "persisted": true,
             }).to_string()))
         }

         ("GET", "/api/songid/runs") => {
             // Matches the oracle's real ListRuns(limit=10): newest-first,
             // bounded by the real `limit` query param -- not the full,
             // unbounded, oldest-first storage order.
             let limit = route
                 .query
                 .map(query_params)
                 .unwrap_or_default()
                 .into_iter()
                 .find(|(key, _)| key == "limit")
                 .and_then(|(_, value)| value.parse::<i64>().ok())
                 .filter(|limit| *limit > 0)
                 .unwrap_or(10) as usize;
             let runtime = state.runtime.read().await;
             let mut runs = runtime.songid_run_records.clone();
             drop(runtime);
             runs.reverse();
             runs.truncate(limit);
             Ok(routing::ok_response(serde_json::Value::Array(runs).to_string()))
         }

         ("GET", "/api/songid/runs/queue") => {
             // Matches the oracle's real GetQueueSummary(activeLimit):
             // counts are windowed over the most recent
             // max(activeLimit, 100) runs (newest-first), and activeRuns
             // is bounded by the real activeLimit query param -- not the
             // entire unbounded, unordered history.
             let active_limit = route
                 .query
                 .map(query_params)
                 .unwrap_or_default()
                 .into_iter()
                 .find(|(key, _)| key == "activeLimit")
                 .and_then(|(_, value)| value.parse::<i64>().ok())
                 .filter(|limit| *limit > 0)
                 .unwrap_or(25) as usize;
             let max_concurrent_runs = state
                 .media_services
                 .read()
                 .await
                 .song_id_max_concurrent_runs;
             let runtime = state.runtime.read().await;
             let mut recent_runs = runtime.songid_run_records.clone();
             drop(runtime);
             recent_runs.reverse();
             recent_runs.truncate(active_limit.max(100));
             let queued = recent_runs.iter().filter(|record| record["status"] == "queued").count();
             let running = recent_runs.iter().filter(|record| record["status"] == "running").count();
             let completed = recent_runs.iter().filter(|record| record["status"] == "completed").count();
             let failed = recent_runs.iter().filter(|record| record["status"] == "failed").count();
             let active_runs = recent_runs
                 .iter()
                 .filter(|record| matches!(record["status"].as_str(), Some("queued" | "running")))
                 .take(active_limit)
                 .cloned()
                 .collect::<Vec<_>>();
             Ok(routing::ok_response(serde_json::json!({
                 "queuedCount": queued,
                 "runningCount": running,
                 "completedCount": completed,
                 "failedCount": failed,
                 "maxConcurrentRuns": max_concurrent_runs,
                 "activeRuns": active_runs,
             }).to_string()))
         }

         ("POST", "/api/songid/runs") => {
             // Matches the oracle's SongIdController.CreateRun: a request
             // with no real source is rejected before ever consuming a
             // concurrency slot, not silently accepted as an empty-source
             // run.
             let source = extract_json_string_field(body, "source")
                 .unwrap_or_default()
                 .trim()
                 .to_owned();
             if source.is_empty() {
                 return Ok(routing::bad_request_response("SongID source is required."));
             }
             let source_type = songid_source_type(&source);
             if source_type == "local_file"
                 && !songid_local_file_is_allowed(&state.config, &source)
             {
                 return Ok(routing::bad_request_response(
                     "SongID analysis could not be queued.",
                 ));
             }
             if let Some(response) = enqueue_songid_job(
                 state,
                 source.clone(),
                 source_type,
                 extract_json_string_field(body, "query")
                     .filter(|query| !query.trim().is_empty()),
                 state.config.controller_profile != ControllerProfile::Native,
             )
             .await?
             {
                 return Ok(response);
             }
             let Ok(_songid_permit) = Arc::clone(&state.songid_run_slots).try_acquire_owned()
             else {
                 return Ok(routing::service_unavailable_response(
                     "SongID run concurrency limit reached",
                 ));
             };
             let integrations = state.integration_settings.read().await.clone();
             let (fallback_query, metadata, evidence, full_source_fingerprint, acoustid_finding) =
                 songid_source_analysis(&source, source_type, &integrations).await;
             let query = extract_json_string_field(body, "query")
                 .filter(|query| !query.trim().is_empty())
                 .unwrap_or(fallback_query);
             let spotify_metadata_loaded = metadata
                 .pointer("/extra/analysisAudioSource")
                 .and_then(serde_json::Value::as_str)
                 == Some("spotify_page");
             let library = state.library.read().await;
             let shares = state.shares.read().await;
             let runs = songid_runs_value(&library, &shares, None);
             let matches = runs
                 .iter()
                 .flat_map(|run| run.get("matches").and_then(serde_json::Value::as_array).cloned().unwrap_or_default())
                 .collect::<Vec<_>>();
             let library_items = library.records.len();
             let shared_files = shares.entries.len();
             drop(shares);
             drop(library);
             // Unlike the oracle's real async queue+worker pipeline
             // (SongIdService.QueueAnalyzeAsync enqueues a "queued" run
             // that a background worker progresses through "running" to
             // "completed" over real time), slskR analyzes synchronously
             // right here -- record_songid_run has already computed the
             // real, final status by the time this response is built. It
             // must not be overwritten with a fake "queued" placeholder
             // that nothing would ever advance past, permanently hiding
             // the real (already-available) result from every later poll.
             let run = match mutate_runtime_compat_state(state, |runtime, _| {
                 let mut run = runtime.record_songid_run(matches, library_items, shared_files)?;
                 run["source"] = serde_json::json!(source);
                 run["sourceType"] = serde_json::json!(source_type);
                 run["query"] = serde_json::json!(query);
                 run["summary"] = serde_json::json!(match source_type {
                     "local_file" if full_source_fingerprint.is_some() && acoustid_finding.is_some() => {
                         "Analyzed local file with Chromaprint and AcoustID metadata."
                     }
                     "local_file" if full_source_fingerprint.is_some() => {
                         "Analyzed local file with Chromaprint and filename metadata."
                     }
                     "local_file" => "Analyzed local file with filename fallback metadata.",
                     "youtube_url" => {
                         "Classified YouTube URL; optional metadata tools may enrich the run."
                     }
                     "spotify_url" if spotify_metadata_loaded => {
                         "Analyzed Spotify page metadata for SongID query generation."
                     }
                     "spotify_url" => "Spotify metadata fetch failed; using source query fallback.",
                     "url" => "Classified URL; optional source metadata may enrich the run.",
                     _ => "Using free-text SongID query.",
                 });
                 run["evidence"] = serde_json::Value::Array(
                     evidence.iter().cloned().map(serde_json::Value::String).collect(),
                 );
                 run["metadata"] = metadata.clone();
                 if let Some(fingerprint) = full_source_fingerprint.clone() {
                     run["fullSourceFingerprint"] = fingerprint;
                 }
                 if let Some(finding) = acoustid_finding.clone() {
                     run["clips"] = serde_json::json!([{
                         "clipId": "full-source",
                         "acoustId": finding,
                     }]);
                     run["scorecard"] = serde_json::json!({
                         "acoustIdHitCount": 1,
                         "rawAcoustIdHitCount": 1,
                     });
                 }
                 if let Some(stored) = runtime.songid_run_records.last_mut() {
                     *stored = run.clone();
                 }
                 Some(run)
             }).await {
                 Ok(Some(run)) => run,
                 Ok(None) => {
                     return Ok(routing::service_unavailable_response("song id run space exhausted"));
                 }
                 Err(error) => return Ok(routing::service_unavailable_response(&error)),
             };
             publish_songid_hub_event(state, "create", &run);
             Ok(routing::accepted_response(run.to_string()))
         }

         ("GET", path) if path.starts_with("/api/songid/runs/") && path.contains("/evidence-package") => {
             let Some(run_id) =
                 path_segment_between(path, "/api/songid/runs/", "/evidence-package")
             else {
                 return Ok(routing::not_found_response());
             };
             let runtime = state.runtime.read().await;
             let run = runtime.songid_run(run_id);
             drop(runtime);
             Ok(run
                 .map(|run| routing::ok_response(songid_evidence_package_json(&run).to_string()))
                 .unwrap_or_else(routing::not_found_response))
         }

         ("GET", path)
             if path.starts_with("/api/songid/runs/")
                 && !path.contains("/forensic-matrix")
                 && !path.contains("/evidence-package") =>
         {
             let Some(run_id) = path_segment_after(path, "/api/songid/runs/") else {
                 return Ok(routing::not_found_response());
             };
             let runtime = state.runtime.read().await;
             let run = runtime.songid_run(run_id);
             drop(runtime);
             Ok(run
                 .map(|run| routing::ok_response(run.to_string()))
                 .unwrap_or_else(routing::not_found_response))
         }

         ("GET", path) if path.starts_with("/api/songid/runs/") && path.contains("/forensic-matrix") => {
             let Some(run_id) =
                 path_segment_between(path, "/api/songid/runs/", "/forensic-matrix")
             else {
                 return Ok(routing::not_found_response());
             };
             let runtime = state.runtime.read().await;
            let Some(run) = runtime.songid_run(run_id) else {
                return Ok(routing::not_found_response());
            };
            drop(runtime);
            let matrix = run
                .get("forensicMatrix")
                .or_else(|| run.get("matches"))
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|match_value| {
                    serde_json::json!({
                        "libraryItemId": match_value.get("libraryItemId").cloned().unwrap_or(serde_json::Value::Null),
                        "filename": match_value.get("filename").cloned().unwrap_or(serde_json::Value::Null),
                        "score": match_value.get("score").or_else(|| match_value.get("identityScore")).cloned().unwrap_or_else(|| serde_json::json!(0.0)),
                        "signals": match_value.get("signals").cloned().unwrap_or_else(|| serde_json::json!([])),
                    })
                })
                 .collect::<Vec<_>>();
             let count = matrix.len();
             Ok(routing::ok_response(serde_json::json!({
                 "run_id": run_id,
                 "matrix": matrix,
                 "count": count,
             }).to_string()))
         }

         ("GET", path) if path.starts_with("/api/soulseek/users/") && path.contains("/interests") && path.len() > 20 => {
             let username = path.split('/').nth(4).unwrap_or("unknown");
             let json = format!(
                 "{{\"username\":\"{}\",\"interests\":[],\"count\":0}}",
                 json_escape(username)
             );
             Ok(routing::ok_response(json))
         }

         ("GET", "/api/swarm/analytics/dashboard") => {
             let time_window_hours = match query_bounded_usize(
                 route.query,
                 "timeWindowHours",
                 1,
                 168,
             ) {
                 Ok(value) => value.unwrap_or(24) as u64,
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Time window must be between 1 and 168 hours (7 days)",
                     ));
                 }
             };
             let ranking_limit = match query_bounded_usize(route.query, "rankingLimit", 1, 100) {
                 Ok(value) => value.unwrap_or(20),
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Ranking limit must be between 1 and 100",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, time_window_hours, ranking_limit);
             drop(swarms);
             Ok(routing::ok_response(dashboard.to_string()))
         }

         ("GET", "/api/swarm/analytics/performance") => {
             let time_window_hours = match query_bounded_usize(
                 route.query,
                 "timeWindowHours",
                 1,
                 168,
             ) {
                 Ok(value) => value.unwrap_or(24) as u64,
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Time window must be between 1 and 168 hours (7 days)",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, time_window_hours, 20);
             drop(swarms);
             Ok(routing::ok_response(
                 dashboard["performanceMetrics"].to_string(),
             ))
         }

         ("GET", "/api/swarm/analytics/peers/rankings") => {
             let limit = match query_bounded_usize(route.query, "limit", 1, 100) {
                 Ok(value) => value.unwrap_or(20),
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Limit must be between 1 and 100",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, 24, limit);
             drop(swarms);
             Ok(routing::ok_response(dashboard["peerRankings"].to_string()))
         }

         ("GET", "/api/swarm/analytics/efficiency") => {
             let time_window_hours = match query_bounded_usize(
                 route.query,
                 "timeWindowHours",
                 1,
                 168,
             ) {
                 Ok(value) => value.unwrap_or(24) as u64,
                 Err(()) => {
                     return Ok(routing::bad_request_response(
                         "Time window must be between 1 and 168 hours (7 days)",
                     ));
                 }
             };
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, time_window_hours, 100);
             drop(swarms);
             Ok(routing::ok_response(
                 dashboard["efficiencyMetrics"].to_string(),
             ))
         }

         ("GET", "/api/swarm/analytics/trends") => {
             if query_bounded_usize(route.query, "timeWindowHours", 1, 168).is_err() {
                 return Ok(routing::bad_request_response(
                     "Time window must be between 1 and 168 hours (7 days)",
                 ));
             }
             if query_bounded_usize(route.query, "dataPoints", 2, 168).is_err() {
                 return Ok(routing::bad_request_response(
                     "Data points must be between 2 and 168",
                 ));
             }
             Ok(routing::ok_response(
                 serde_json::json!({
                     "timePoints": [],
                     "successRates": [],
                     "averageSpeeds": [],
                     "averageDurations": [],
                     "averageSourcesUsed": [],
                     "downloadCounts": [],
                 })
                 .to_string(),
             ))
         }

         ("GET", "/api/swarm/analytics/recommendations") => {
             let swarms = state.multisource.read().await;
             let dashboard = swarm_analytics_dashboard(&swarms, 24, 10);
             drop(swarms);
             Ok(routing::ok_response(
                 dashboard["recommendations"].to_string(),
             ))
         }

        ("GET", "/api/telemetry/metrics") => {
            let transfers = state.transfers.read().await;
            let transfer_count = transfers.entries.len();
            drop(transfers);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: format!(
                    "# HELP slskr_telemetry_transfers Transfer count\n\
                     # TYPE slskr_telemetry_transfers gauge\n\
                     slskr_telemetry_transfers {}\n",
                    transfer_count
                ),
            })
        }

          // Matches the oracle exactly: TelemetryController.GetKpis and
          // MetricsController.GetKpis are different controllers mounted at
          // different routes, but both call the same
          // Telemetry.Prometheus.GetMetricsAsObject(include: KpiRegexes)
          // with an identical regex list, so they return identical
          // content. slskR's /api/telemetry/prometheus/kpis was already
          // fixed to the real dictionary-of-PrometheusMetric shape; this
          // sibling route reuses the exact same real data instead of its
          // own invented {kpis:[...], count} array.
          ("GET", "/api/telemetry/metrics/kpi") | ("GET", "/api/telemetry/metrics/kpis") => {
              let transfers = state.transfers.read().await;
              let searches = state.searches.read().await;
              let metrics = serde_json::json!({
                  "slskr_transfers": prometheus_metric_json("slskr_transfers", "gauge", transfers.entries.len() as f64),
                  "slskr_searches": prometheus_metric_json("slskr_searches", "gauge", searches.records.len() as f64),
              });
              drop(transfers);
              drop(searches);
              Ok(routing::ok_response(metrics.to_string()))
          }

          // ADDITIONAL MISSING GET ENDPOINTS (Phase 6)
          ("GET", "/api/multisource/jobs") => {
            let versioned_profile = route.path.starts_with("/api/v0/")
                && state.config.controller_profile
                    == ControllerProfile::Native;
            let swarm = state.multisource.read().await;
            let mut jobs = if versioned_profile {
                  swarm
                      .list()
                      .into_iter()
                      .map(|job| {
                          serde_json::json!({
                              "jobId": job.id,
                              "state": job.status,
                              "totalChunks": job.total_chunks,
                              "completedChunks": job.completed_chunks,
                              "percentComplete": if job.total_chunks > 0 {
                                  job.completed_chunks as f64 * 100.0 / job.total_chunks as f64
                              } else {
                                  0.0
                              },
                              "activeWorkers": 0,
                              "chunksPerSecond": 0.0,
                          })
                      })
                      .collect::<Vec<_>>()
              } else {
                  swarm
                      .list()
                      .into_iter()
                      .filter_map(|job| serde_json::to_value(job).ok())
                      .collect::<Vec<_>>()
              };
              drop(swarm);
            if !versioned_profile {
                  let transfers = state.transfers.read().await;
                  jobs.extend(transfers
                      .entries
                      .iter()
                      .filter(|entry| entry.direction == 0)
                      .map(|entry| {
                          let size = entry.size.unwrap_or(0);
                          let progress = if size == 0 {
                              0.0
                          } else {
                              (entry.bytes_transferred as f64 / size as f64) * 100.0
                          };
                          serde_json::json!({
                              "id": format!("transfer-{}", entry.id),
                              "status": entry.status,
                              "filename": entry.filename,
                              "sources": entry.peer_username.as_deref().map(|peer| vec![peer]).unwrap_or_default(),
                              "progress": progress,
                              "bytesTransferred": entry.bytes_transferred,
                              "size": size,
                              "updated_at": entry.updated_at,
                          })
                      }));
                  drop(transfers);
              }
              let count = jobs.len();
              let json = serde_json::json!({
                  "jobs": jobs,
                  "count": count,
              }).to_string();
              Ok(routing::ok_response(json))
          }

          ("GET", path) if pod_channel_messages_path(path).is_some() => {
              let (pod_id, channel_id) = pod_channel_messages_path(path).unwrap_or_default();
              if pods::is_gold_star_club(&pod_id) && !gold_star_club_available(state) {
                  return Ok(routing::not_found_response());
              }
              let since = match query_millis_parameter(route.query, "since") {
                  Ok(value) => value,
                  Err(error) => return Ok(routing::bad_request_response(&error)),
              };
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if pods.get(&pod_id).is_none() || !pods.channel_exists(&pod_id, &channel_id) {
                  return Ok(routing::ok_response("[]".to_owned()));
              }
              if peer_id
                  .as_deref()
                  .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              if let Err(error) = state.pod_channels.read().await.validate_storage() {
                  eprintln!("pod channel message storage failed: {error}");
                  return Ok(routing::internal_server_error_response(
                      "Failed to get messages",
                  ));
              }
              let binding = pods.soulseek_binding(&pod_id, &channel_id);
              drop(pods);
              if let Some(binding) = binding.filter(|binding| binding.kind == "dm") {
                  let local_peer_id = peer_id.unwrap_or_default();
                  let messages = state.messages.read().await;
                  let projected = messages
                      .records
                      .iter()
                      .filter(|message| {
                          message.username.eq_ignore_ascii_case(&binding.identifier)
                              && since.is_none_or(|since| message.created_at_ms > since)
                      })
                      .map(|message| pod_channels::PodChannelMessage {
                          message_id: message.id.to_string(),
                          pod_id: pod_id.clone(),
                          channel_id: channel_id.clone(),
                          sender_peer_id: if message.direction == "inbound" {
                              format!("bridge:{}", message.username)
                          } else {
                              local_peer_id.clone()
                          },
                          body: message.body.clone(),
                          timestamp_unix_ms: message.created_at_ms,
                          signature: String::new(),
                          sig_version: 1,
                      })
                      .collect::<Vec<_>>();
                  return Ok(routing::ok_response(
                      serde_json::to_string(&projected)
                          .map_err(|error| format!("pod message serialization failed: {error}"))?,
                  ));
              }
              let channels = state.pod_channels.read().await;
              let messages = channels.list(&pod_id, &channel_id, since);
              drop(channels);
              Ok(routing::ok_response(
                  serde_json::to_string(&messages)
                      .map_err(|error| format!("pod message serialization failed: {error}"))?,
              ))
          }

          ("POST", path) if pod_channel_messages_path(path).is_some() => {
              let (pod_id, channel_id) = pod_channel_messages_path(path).unwrap_or_default();
              let body_text = extract_json_string_field(body, "body")
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let sender_peer_id = extract_json_string_field(body, "senderPeerId")
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              if body_text.is_empty() {
                  return Ok(routing::bad_request_response("Message body is required"));
              }
              if sender_peer_id.is_empty() {
                  return Ok(routing::bad_request_response("SenderPeerId is required"));
              }
              let authenticated_peer_id = pod_request_peer_id(state).await;
              let Some(authenticated_peer_id) = authenticated_peer_id else {
                  return Ok(routing::forbidden_response(
                      "Authenticated peer identity is required",
                  ));
              };
              if sender_peer_id != authenticated_peer_id {
                  return Ok(routing::forbidden_response(
                      "SenderPeerId must match the authenticated peer identity",
                  ));
              }
              let signature = extract_json_string_field(body, "signature")
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let signature_mode = state
                  .advanced_networking
                  .read()
                  .await
                  .pod_security_signature_mode;
              if signature_mode == PodSignatureMode::Enforce && signature.is_empty() {
                  return Ok(routing::bad_request_response(
                      "Message signature is required when PodCore.Security.SignatureMode is Enforce",
                  ));
              }
              if signature_mode == PodSignatureMode::Warn && signature.is_empty() {
                  record_daemon_log(
                      state,
                      logging::LogLevel::Warn,
                      "podcore",
                      "accepted unsigned pod message in warn mode".to_owned(),
                  )
                  .await;
              }
              let pods = state.pods.read().await;
              if pods.get(&pod_id).is_none() || !pods.channel_exists(&pod_id, &channel_id) {
                  return Ok(routing::not_found_response());
              }
              if !pods.is_member(&pod_id, &authenticated_peer_id) {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              let binding = pods.soulseek_binding(&pod_id, &channel_id);
              drop(pods);
              if let Some(binding) = binding.as_ref().filter(|binding| binding.kind == "dm") {
                  let session_command_permit = match state.session_commands.reserve().await {
                      Ok(permit) => permit,
                      Err(_) => {
                          return Ok(routing::service_unavailable_response(
                              "session manager is not running",
                          ));
                      }
                  };
                  let mut messages = state.messages.write().await;
                  let previous = messages.clone();
                  let record = messages.add(
                      binding.identifier.clone(),
                      "outbound",
                      body_text.clone(),
                  );
                  let mutated = messages.clone();
                  drop(messages);
                  if let Err(error) = persist_message_record_checked(state, &record).await {
                      rollback_messages_if_unchanged(state, previous, &mutated).await;
                      return Ok(routing::service_unavailable_response(&error));
                  }
                  session_command_permit.send(SessionCommand::MessageUser {
                      username: binding.identifier.clone(),
                      body: body_text,
                  });
                  return Ok(routing::ok_response(
                      serde_json::json!({
                          "messageId": record.id.to_string(),
                          "sent": true,
                      })
                      .to_string(),
                  ));
              }
              let result = state.pod_channels.write().await.append(
                  pod_id,
                  channel_id,
                  authenticated_peer_id,
                  body_text,
                  signature,
                  unix_timestamp_millis(),
              );
              match result {
                  Ok(message) => {
                      if let Some(binding) = binding.filter(|binding| {
                          binding.kind == "room" && binding.mode == "mirror"
                      }) {
                          let _ = try_send_session_command(
                              state,
                              SessionCommand::SayRoom {
                                  room: binding.identifier,
                                  body: format!(
                                      "[Pod:{}] {}",
                                      message.sender_peer_id, message.body
                                  ),
                              },
                          );
                      }
                      Ok(routing::ok_response(
                          serde_json::json!({
                              "messageId": message.message_id,
                              "sent": true,
                          })
                          .to_string(),
                      ))
                  }
                  Err(error)
                      if error.contains("required") || error.contains("must be at most") =>
                  {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) => {
                      eprintln!("pod channel message persistence failed: {error}");
                      Ok(routing::internal_server_error_response("Failed to send message"))
                  }
              }
          }

          ("POST", "/api/pods") => {
              let value = match serde_json::from_str::<serde_json::Value>(body) {
                  Ok(value) => value,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              let Some(pod_value) = value
                  .get("pod")
                  .filter(|pod| !pod.is_null())
                  .cloned()
              else {
                  return Ok(routing::bad_request_response("Pod data is required"));
              };
              let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                  Ok(pod) => pod,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              let Some(creator) = pod_request_peer_id(state).await else {
                  return Ok(routing::forbidden_response("Authenticated peer identity is required"));
              };
              match state.pods.write().await.create(pod, creator) {
                  Ok(pod) => Ok(routing::created_response(
                      serde_json::to_string(&pod)
                          .map_err(|error| format!("pod serialization failed: {error}"))?,
                  )),
                  Err(error) if error == "Pod already exists" => {
                      Ok(routing::conflict_response(&error))
                  }
                  Err(error) if error.contains("capacity is full") => {
                      Ok(routing::conflict_response(&error))
                  }
                  Err(error) if error.starts_with("pod state write failed") => {
                      eprintln!("pod persistence failed: {error}");
                      Ok(routing::internal_server_error_response("Failed to create pod"))
                  }
                  Err(error) => Ok(routing::bad_request_response(&error)),
              }
          }

          ("GET", path)
              if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to get pod"));
              }
              if pods.get(&pod_id).is_some()
                  && !pods.is_public(&pod_id)
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              Ok(pods
                  .get(&pod_id)
                  .map(|pod| {
                      routing::ok_response(
                          serde_json::to_string(&pod).unwrap_or_else(|_| "{}".to_owned()),
                      )
                  })
                  .unwrap_or_else(routing::not_found_response))
          }

          ("PUT", path)
              if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let value = match serde_json::from_str::<serde_json::Value>(body) {
                  Ok(value) => value,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              let Some(pod_value) = value
                  .get("pod")
                  .filter(|pod| !pod.is_null())
                  .cloned()
              else {
                  return Ok(routing::bad_request_response("Pod data is required"));
              };
              let pod = match serde_json::from_value::<pods::PodRecord>(pod_value) {
                  Ok(pod) => pod,
                  Err(error) => {
                      return Ok(routing::bad_request_response(&format!(
                          "Invalid pod request: {error}"
                      )));
                  }
              };
              if pod.pod_id != pod_id {
                  return Ok(routing::bad_request_response(
                      "PodId in URL must match PodId in body",
                  ));
              }
              let peer_id = pod_request_peer_id(state).await;
              let mut pods = state.pods.write().await;
              if pods.get(&pod_id).is_some()
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              if let Some(gateway_peer_id) = pods.gateway_peer_for_update(&pod_id, &pod) {
                  if peer_id.as_deref() != Some(gateway_peer_id.as_str()) {
                      return Ok(routing::forbidden_response(
                          "Only the designated gateway peer can modify private service policy",
                      ));
                  }
              }
              let proposed_channel_ids = pod
                  .channels
                  .iter()
                  .map(|channel| channel.channel_id.trim().to_owned())
                  .collect::<HashSet<_>>();
              let removed_channel_ids = pods
                  .get(&pod_id)
                  .map(|existing| {
                      existing
                          .channels
                          .into_iter()
                          .map(|channel| channel.channel_id)
                          .filter(|channel_id| !proposed_channel_ids.contains(channel_id))
                          .collect::<HashSet<_>>()
                  })
                  .unwrap_or_default();
              let mut channels = state.pod_channels.write().await;
              let removed_messages = match channels.delete_channels(&pod_id, &removed_channel_ids) {
                  Ok(messages) => messages,
                  Err(error) => {
                      eprintln!("pod channel cleanup failed: {error}");
                      return Ok(routing::internal_server_error_response(
                          "Failed to update pod",
                      ));
                  }
              };
              let update_result = pods.update(&pod_id, pod);
              match update_result {
                  Ok(Some(pod)) => Ok(routing::ok_response(
                      serde_json::to_string(&pod)
                          .map_err(|error| format!("pod serialization failed: {error}"))?,
                  )),
                  Ok(None) => {
                      if let Err(error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {error}");
                          return Ok(routing::internal_server_error_response(
                              "Failed to update pod",
                          ));
                      }
                      Ok(routing::not_found_response())
                  }
                  Err(error) => {
                      if let Err(rollback_error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {rollback_error}");
                          return Ok(routing::internal_server_error_response(
                              "Failed to update pod",
                          ));
                      }
                      if error.starts_with("pod state write failed") {
                          eprintln!("pod persistence failed: {error}");
                          Ok(routing::internal_server_error_response("Failed to update pod"))
                      } else {
                          Ok(routing::bad_request_response(&error))
                    }
                }
            }
        }

          ("DELETE", path)
              if pod_resource_segments(path).is_some_and(|segments| segments.len() == 1) =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to delete pod"));
              }
              if pods.get(&pod_id).is_some()
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              drop(pods);
              let mut pods = state.pods.write().await;
              if pods.get(&pod_id).is_none() {
                  return Ok(routing::not_found_response());
              }
              let mut channels = state.pod_channels.write().await;
              let removed_messages = match channels.delete_pod(&pod_id) {
                  Ok(messages) => messages,
                  Err(error) => {
                      eprintln!("pod channel cleanup failed: {error}");
                      return Ok(routing::internal_server_error_response(
                          "Failed to delete pod",
                      ));
                  }
              };
              match pods.delete(&pod_id) {
                  Ok(true) => Ok(routing::no_content_response()),
                  Ok(false) => {
                      if let Err(rollback_error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {rollback_error}");
                      }
                      Ok(routing::not_found_response())
                  }
                  Err(error) => {
                      if let Err(rollback_error) = channels.restore(removed_messages) {
                          eprintln!("pod channel cleanup rollback failed: {rollback_error}");
                      }
                      eprintln!("pod persistence failed: {error}");
                      Ok(routing::internal_server_error_response("Failed to delete pod"))
                  }
              }
          }

          ("GET", path)
              if pod_resource_segments(path)
                  .is_some_and(|segments| segments.len() == 2 && segments[1] == "members") =>
          {
              let pod_id = pod_resource_segments(path).unwrap_or_default().remove(0);
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response(
                      "Failed to get pod members",
                  ));
              }
              if pods.get(&pod_id).is_some()
                  && !pods.is_public(&pod_id)
                  && peer_id
                      .as_deref()
                      .is_none_or(|peer_id| !pods.is_member(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response("Pod membership is required"));
              }
              Ok(pods
                  .members(&pod_id)
                  .map(|members| {
                      routing::ok_response(
                          serde_json::to_string(&members).unwrap_or_else(|_| "[]".to_owned()),
                      )
                  })
                  .unwrap_or_else(routing::not_found_response))
          }

          ("POST", path)
              if pod_resource_segments(path).is_some_and(|segments| {
                  segments.len() == 2 && matches!(segments[1].as_str(), "join" | "leave" | "ban")
              }) =>
          {
              let segments = pod_resource_segments(path).unwrap_or_default();
              let pod_id = &segments[0];
              let action = &segments[1];
              if pods::is_gold_star_club(pod_id) && !gold_star_club_available(state) {
                  return Ok(routing::not_found_response());
              }
              let peer_id = if action == "ban" {
                  extract_json_string_field(body, "peerId")
                      .unwrap_or_default()
                      .trim()
                      .to_owned()
              } else {
                  pod_request_peer_id(state).await.unwrap_or_default()
              };
              if peer_id.is_empty() {
                  return Ok(routing::bad_request_response("PeerId is required"));
              }
              let moderator = if action == "ban" {
                  pod_request_peer_id(state).await
              } else {
                  None
              };
              let mut pods = state.pods.write().await;
              if action == "ban"
                  && moderator
                      .as_deref()
                      .is_none_or(|moderator| !pods.can_moderate(pod_id, moderator))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              let result = match action.as_str() {
                  "join" => pods.join(pod_id, peer_id.clone()),
                  "leave" => pods.leave(pod_id, &peer_id),
                  _ => pods.ban(pod_id, &peer_id),
              };
              match result {
                  Ok(Some(true)) => {
                      if action == "leave" && pods::is_gold_star_club(pod_id) {
                          if let Err(error) = pods::record_gold_star_club_revocation(
                              &state.config.state_dir,
                              &peer_id,
                          ) {
                              eprintln!("Gold Star Club revocation persistence failed: {error}");
                              return Ok(routing::service_unavailable_response(
                                  "pod revocation storage is unavailable",
                              ));
                          }
                      }
                      let response_key = match action.as_str() {
                          "join" => "joined",
                          "leave" => "left",
                          _ => "banned",
                      };
                      Ok(routing::ok_response(
                          serde_json::json!({ (response_key): true }).to_string(),
                      ))
                  }
                  Ok(Some(false)) if action == "join" => Ok(routing::bad_request_response(
                      "Failed to join pod (may already be a member)",
                  )),
                  Ok(Some(false)) => Ok(routing::not_found_response()),
                  Ok(None) => Ok(routing::not_found_response()),
                  Err(error)
                      if error.contains("capacity")
                          || error.contains("banned")
                          || error.contains("approval")
                          || error.contains("last Pod moderator") =>
                  {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) if error.contains("required") || error.contains("at most") => {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) => {
                      eprintln!("pod persistence failed: {error}");
                      let message = match action.as_str() {
                          "join" => "Failed to join pod",
                          "leave" => "Failed to leave pod",
                          _ => "Failed to ban member",
                      };
                      Ok(routing::internal_server_error_response(message))
                  }
              }
          }

          ("POST", path)
              if pod_resource_segments(path).is_some_and(|segments| {
                  segments.len() == 4
                      && segments[1] == "channels"
                      && matches!(segments[3].as_str(), "bind" | "unbind")
              }) =>
          {
              let segments = pod_resource_segments(path).unwrap_or_default();
              let pod_id = &segments[0];
              let channel_id = &segments[2];
              let action = &segments[3];
              if action == "bind" {
                  let mode = extract_json_string_field(body, "mode")
                      .unwrap_or_else(|| "readonly".to_owned())
                      .trim()
                      .to_ascii_lowercase();
                  if !matches!(mode.as_str(), "readonly" | "mirror") {
                      return Ok(routing::bad_request_response(
                          "Mode must be 'readonly' or 'mirror'",
                      ));
                  }
              } else if state
                  .pods
                  .read()
                  .await
                  .soulseek_binding(pod_id, channel_id)
                  .is_none()
              {
                  return Ok(routing::not_found_response());
              }
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              let can_moderate = peer_id
                  .as_deref()
                  .is_some_and(|peer_id| pods.can_moderate(pod_id, peer_id));
              drop(pods);
              if !can_moderate {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              let result = if action == "bind" {
                  let room_name = extract_json_string_field(body, "roomName")
                      .unwrap_or_default()
                      .trim()
                      .to_owned();
                  let mode = extract_json_string_field(body, "mode")
                      .unwrap_or_else(|| "readonly".to_owned())
                      .trim()
                      .to_ascii_lowercase();
                  state
                      .pods
                      .write()
                      .await
                      .bind_room(pod_id, channel_id, room_name, mode)
              } else {
                  state.pods.write().await.unbind_room(pod_id, channel_id)
              };
              match result {
                  Ok(Some(true)) => {
                      if action == "bind" {
                          if let Some(binding) = state
                              .pods
                              .read()
                              .await
                              .soulseek_binding(pod_id, channel_id)
                          {
                              let _ = try_send_session_command(
                                  state,
                                  SessionCommand::JoinRoom(binding.identifier),
                              );
                          }
                      }
                      let response_key = if action == "bind" { "bound" } else { "unbound" };
                      Ok(routing::ok_response(
                          serde_json::json!({ (response_key): true }).to_string(),
                      ))
                  }
                  Ok(Some(false)) | Ok(None) => Ok(routing::not_found_response()),
                  Err(error) if error.contains("required") || error.starts_with("Mode must") => {
                      Ok(routing::bad_request_response(&error))
                  }
                  Err(error) => {
                      eprintln!("pod binding persistence failed: {error}");
                      Ok(routing::internal_server_error_response(if action == "bind" {
                          "Failed to bind room"
                      } else {
                          "Failed to unbind room"
                      }))
                  }
              }
          }

          ("GET", "/api/pods") => {
              let peer_id = pod_request_peer_id(state).await;
              let pods = state.pods.read().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to list pods"));
              }
              Ok(routing::ok_response(
                  serde_json::to_string(&pods.list_visible(peer_id.as_deref()))
                      .map_err(|error| format!("pod serialization failed: {error}"))?,
              ))
          }

          ("GET", "/api/solid/status") => {
              let media = state.media_services.read().await;
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": true,
                  "clientId": media
                      .solid
                      .client_id_url
                      .clone()
                      .unwrap_or_else(|| "/solid/clientid.jsonld".to_owned()),
                  "redirectPath": media.solid.redirect_path,
              }).to_string()))
          }

          ("GET", "/api/federation/diagnostics")
              if route.path == "/api/v0/federation/diagnostics" =>
          {
              Ok(federation_diagnostics_response(&state.config))
          }

          ("GET", "/api/federation/diagnostics") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let user_count = users.records.len();
              let watched_users = users.records.iter().filter(|user| user.watched).count();
              let mesh_capabilities = mesh.capability_records.len();
              let items = mesh
                  .capability_records
                  .iter()
                  .map(|record| {
                      serde_json::json!({
                          "username": record.username,
                          "issuedAt": record.issued_at_unix,
                          "expiresAt": record.expires_at_unix,
                          "features": record.features.clone(),
                          "endpoints": record.endpoints.clone(),
                          "source": "peer-capability",
                      })
                  })
                  .chain(users.records.iter().filter(|user| user.watched).map(|user| {
                      serde_json::json!({
                          "username": user.username,
                          "status": user.status,
                          "source": "watched-user",
                      })
                  }))
                  .collect::<Vec<_>>();
              let item_count = items.len();
              let checks = vec![
                  serde_json::json!({
                      "id": "watched-users",
                      "status": if users.records.is_empty() { "empty" } else { "ready" },
                      "count": users.records.len(),
                  }),
                  serde_json::json!({
                      "id": "mesh-capabilities",
                      "status": if mesh.capability_records.is_empty() { "empty" } else { "ready" },
                      "count": mesh.capability_records.len(),
                  }),
              ];
              let ready = checks
                  .iter()
                  .any(|check| check.get("status").and_then(serde_json::Value::as_str) == Some("ready"));
              drop(mesh);
              drop(users);
              Ok(routing::ok_response(serde_json::json!({
                  "federation": {"enabled": ready, "watchedUsers": watched_users},
                  "publishing": {"enabled": false},
                  "pods": {"enabled": true},
                  "mesh": {"capabilityRecords": mesh_capabilities},
                  "status": if ready { "ready" } else { "empty" },
                  "checks": checks,
                  "items": items,
                  "itemCount": item_count,
                  "counts": {
                      "users": user_count,
                      "watchedUsers": watched_users,
                      "meshCapabilities": mesh_capabilities,
                  },
                  "warnings": [],
                  "errors": [],
              }).to_string()))
          }

          ("GET", "/api/security/dashboard") => {
              let users = state.users.read().await;
              let webhooks = state.webhooks.read().await;
              let events = state.events.read().await;
              let security = state.security.read().await;
              let watched = users.records.iter().filter(|user| user.watched).count();
              let webhook_count = webhooks.get_all().len();
              let event_count = events.records.len();
              let ban_count = security.active_bans();
              let bans = security
                  .json_value()
                  .get("bans")
                  .cloned()
                  .unwrap_or_else(|| serde_json::json!([]));
              // Matches the oracle's real PeerReputation.GetStats(): real
              // per-peer scores and violation counts, not watch/online
              // status (which has nothing to do with reputation).
              let mut peer_keys = security
                  .reputation
                  .keys()
                  .chain(security.reputation_profiles.keys())
                  .cloned()
                  .collect::<Vec<_>>();
              peer_keys.sort_unstable();
              peer_keys.dedup();
              let total_peers = peer_keys.len();
              let mut total_score = 0_i64;
              let mut trusted_peers = 0;
              let mut untrusted_peers = 0;
              let mut total_successful_transfers = 0_u64;
              let mut total_failed_transfers = 0_u64;
              let mut total_protocol_violations = 0_u64;
              for username in &peer_keys {
                  let score = security
                      .reputation
                      .get(username)
                      .copied()
                      .unwrap_or(SECURITY_REPUTATION_DEFAULT_SCORE);
                  total_score += i64::from(score);
                  if score >= SECURITY_REPUTATION_TRUSTED_THRESHOLD {
                      trusted_peers += 1;
                  }
                  if score <= SECURITY_REPUTATION_UNTRUSTED_THRESHOLD {
                      untrusted_peers += 1;
                  }
                  if let Some(profile) = security.reputation_profiles.get(username) {
                      total_successful_transfers = total_successful_transfers
                          .saturating_add(profile.successful_transfers);
                      total_failed_transfers = total_failed_transfers
                          .saturating_add(profile.failed_transfers);
                      total_protocol_violations = total_protocol_violations
                          .saturating_add(profile.protocol_violations);
                  } else {
                      total_protocol_violations = total_protocol_violations.saturating_add(
                          u64::from(security.violations.get(username).copied().unwrap_or(0)),
                      );
                  }
              }
              let average_score = if total_peers > 0 {
                  total_score as f64 / total_peers as f64
              } else {
                  f64::from(SECURITY_REPUTATION_DEFAULT_SCORE)
              };
              let reputation_stats = serde_json::json!({
                  "totalPeers": total_peers,
                  "trustedPeers": trusted_peers,
                  "untrustedPeers": untrusted_peers,
                  "averageScore": average_score,
                  "totalSuccessfulTransfers": total_successful_transfers,
                  "totalFailedTransfers": total_failed_transfers,
                  "totalProtocolViolations": total_protocol_violations,
              });
              drop(security);
              drop(events);
              drop(webhooks);
              drop(users);
              Ok(routing::ok_response(serde_json::json!({
                  "eventStats": {"totalEvents": event_count},
                  "networkGuardStats": {"globalConnections": watched},
                  "violationStats": {"activeBans": ban_count},
                  "reputationStats": reputation_stats.clone(),
                  "paranoidStats": {"enabled": false},
                  "fingerprintStats": {"knownFingerprints": 0},
                  "entropyStats": {"checks": 0},
                  "consensusStats": {"decisions": 0},
                  "verificationStats": {"verified": 0},
                  "disclosureStats": {"disclosures": 0},
                  "temporalStats": {"events": event_count},
                  "enabled": true,
                  "status": "local",
                  "stats": {
                      "networkGuardStats": { "globalConnections": watched },
                      "reputationStats": reputation_stats,
                      "threatStats": { "activeThreats": untrusted_peers },
                      "banStats": { "activeBans": ban_count }
                  },
                  "events": event_count,
                  "webhooks": webhook_count,
                  "bans": bans
              }).to_string()))
          }

          ("GET", "/api/security/status") => {
              let users = state.users.read().await;
              let events = state.events.read().await;
              let security = state.security.read().await;
              let watched = users.records.iter().filter(|user| user.watched).count();
              let offline_watched = users
                  .records
                  .iter()
                  .filter(|user| user.watched && user.status.as_deref() == Some("offline"))
                  .count();
              let event_count = events.records.len();
              let ban_count = security.active_bans();
              drop(security);
              drop(events);
              drop(users);
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": true,
                  "status": "local",
                  "watchedPeers": watched,
                  "suspiciousPeers": offline_watched,
                  "activeBans": ban_count,
                  "events": event_count,
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/security/") => {
              Ok(security_extended_response(path, route.query, state).await)
          }

          ("GET", "/api/soulseek/mesh-rendezvous/status") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let body = mesh.status_json(&users);
              drop(mesh);
              drop(users);
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/users") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let body = mesh.users_json(&users);
              drop(mesh);
              drop(users);
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/soulseek/mesh-rendezvous/discover") => {
              let users = state.users.read().await;
              let mesh = state.mesh.read().await;
              let candidates = if mesh.rendezvous.active_probe_enabled() {
                  mesh.candidate_usernames(&users)
                      .into_iter()
                      .take(MAX_ACTIVE_MESH_DISCOVERY_PROBES)
                      .collect::<Vec<_>>()
              } else {
                  Vec::new()
              };
              let body = mesh.discover_json(&users);
              drop(mesh);
              drop(users);
              for username in candidates {
                  if let Err(error) = send_session_command(
                      state,
                      SessionCommand::ProbePeerCapability(username),
                  )
                  .await
                  {
                      return Ok(routing::service_unavailable_response(&error));
                  }
              }
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/soulseek/peer-capabilities") => {
              let mesh = state.mesh.read().await;
              let body = serde_json::Value::Array(mesh.capability_records_json()).to_string();
              drop(mesh);
              Ok(routing::ok_response(body))
          }

          ("GET", "/api/mesh/transport") if route.path.starts_with("/api/v0/") => {
              let dht_sessions = match state.dht.as_ref() {
                  Some(dht) => dht.peers().await.len(),
                  None => 0,
              };
              let overlay_sessions = match state.private_gateway.as_ref() {
                  Some(gateway) => gateway.active_connection_count().await,
                  None => 0,
              };
              Ok(routing::ok_response(serde_json::json!({
                  "dht": dht_sessions,
                  "overlay": overlay_sessions,
                  "natType": "Unknown",
              }).to_string()))
          }
          ("GET", "/api/mesh/transport") => {
              let gateway = state.private_gateway.as_ref();
              let enabled = gateway.is_some();
              let connected_peers = match gateway {
                  Some(gateway) => gateway.active_connection_count().await,
                  None => 0,
              };
              Ok(routing::ok_response(serde_json::json!({
                  "dht": {
                      "enabled": state.dht.is_some(),
                      "running": state.dht.is_some(),
                  },
                  "overlay": {
                      "enabled": enabled,
                      "endpoint": gateway.map(|gateway| gateway.bind().to_string()),
                  },
                  "status": if enabled { "Healthy" } else { "Disabled" },
                  "health": if enabled { "Healthy" } else { "Disabled" },
                  "description": if enabled {
                      "TLS mesh service transport is listening"
                  } else {
                      "Mesh service transport is disabled"
                  },
                  "transportPreference": "Auto",
                  "overlayBind": gateway.map(|gateway| gateway.bind()),
                  "overlayPort": gateway.map(|gateway| gateway.bind().port()),
                  "certificateSha256": gateway.map(|gateway| hex::encode(gateway.certificate_sha256())),
                  "dhtEnabled": state.dht.is_some(),
                  "connectedPeers": connected_peers,
                  "totalPeers": connected_peers,
                  "activeCircuits": 0,
                  "activeStreams": 0,
                  "bootstrapPeers": [],
                  "isolatedPeers": 0,
                  "quorumPeers": 0,
                  "relayedPeers": 0,
                  "natType": "Unknown",
                  "publicEndpoint": null,
                  "lastDhtError": null,
                  "lastDhtPublishUtc": null
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/multisource/jobs/") => {
              let Some(job_id) = path_segment_after(path, "/api/multisource/jobs/") else {
                  return Ok(routing::not_found_response());
              };
              let job_id = decoded_path_segment(job_id);
              let versioned_profile = route.path.starts_with("/api/v0/")
                  && state.config.controller_profile
                      == ControllerProfile::Native;
              let swarm = state.multisource.read().await;
              if let Some(job) = swarm.get(&job_id) {
                  let body = if versioned_profile {
                      serde_json::json!({
                          "jobId": job.id,
                          "state": job.status,
                          "totalChunks": job.total_chunks,
                          "completedChunks": job.completed_chunks,
                          "percentComplete": if job.total_chunks > 0 {
                              job.completed_chunks as f64 * 100.0 / job.total_chunks as f64
                          } else {
                              0.0
                          },
                          "activeWorkers": 0,
                          "chunksPerSecond": 0.0,
                          "estimatedSecondsRemaining": 0.0,
                          "bytesDownloaded": job.bytes_downloaded,
                          "bytesDownloadedMB": job.bytes_downloaded as f64 / 1024.0 / 1024.0,
                      })
                      .to_string()
                  } else {
                      serde_json::to_string(job)
                          .map_err(|error| format!("multisource job serialization failed: {error}"))?
                  };
                  drop(swarm);
                  return Ok(routing::ok_response(body));
              }
              drop(swarm);
              let transfer_id = job_id.strip_prefix("transfer-").unwrap_or(&job_id);
              let transfers = state.transfers.read().await;
              let body = transfer_id
                  .parse::<u64>()
                  .ok()
                  .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
                  .map(|entry| {
                      let size = entry.size.unwrap_or(0);
                      let progress = if size == 0 {
                          0.0
                      } else {
                          (entry.bytes_transferred as f64 / size as f64) * 100.0
                      };
                      serde_json::json!({
                          "id": job_id,
                          "status": entry.status,
                          "filename": entry.filename,
                          "sources": entry.peer_username.as_deref().map(|peer| vec![peer]).unwrap_or_default(),
                          "progress": progress,
                          "bytesTransferred": entry.bytes_transferred,
                          "size": size,
                          "updated_at": entry.updated_at,
                      })
                  });
              drop(transfers);
              // Matches the oracle's real GetJobStatus: an unknown job id
              // is a real 404, not a fabricated 200 with an invented
              // "not_found" status string.
              match body {
                  Some(body) => Ok(routing::ok_response(body.to_string())),
                  None => Ok(HttpResponse {
                      status: "404 Not Found",
                      content_type: "application/json",
                      body: r#"{"error":"Job not found. It may have completed or been cancelled."}"#.to_owned(),
                  }),
              }
          }

          ("GET", "/api/player/external-visualizer") => {
              let visualizer = state
                  .media_services
                  .read()
                  .await
                  .external_visualizer
                  .clone();
              let resolved = resolve_external_visualizer_path(visualizer.command.as_deref());
              let working_directory = resolve_external_visualizer_working_directory(
                  visualizer.working_directory.as_deref(),
                  resolved.as_deref(),
              );
              let name = if visualizer.name.trim().is_empty() {
                  "External visualizer"
              } else {
                  visualizer.name.trim()
              };
              Ok(routing::ok_response(serde_json::json!({
                  "enabled": visualizer.launch_enabled,
                  "configured": visualizer.configured(),
                  "available": resolved.is_some(),
                  "name": name,
                  "path": visualizer.command.as_deref().unwrap_or_default().trim(),
                  "resolvedPath": resolved,
                  "workingDirectory": working_directory,
                  "arguments": visualizer.arguments,
              }).to_string()))
          }

          ("GET", path) if path.starts_with("/api/realm-subject-indexes/") && path.ends_with("/conflicts") => {
              let Some(realm) = path_segment_between(
                  path,
                  "/api/realm-subject-indexes/",
                  "/conflicts",
              ) else {
                  return Ok(routing::not_found_response());
              };
              let realm = decoded_path_segment(realm);
              let report = state.realm_subject_indexes.read().await.conflict_report(&realm);
              Ok(routing::ok_response(report.to_string()))
          }

          ("POST", "/api/discovery-graph") => {
              if route.path.starts_with("/api/v0/") {
                  return Ok(discovery_graph::build_response(body, state).await);
              }
              let interests = state.interests.read().await;
              let wishlist = state.wishlist.read().await;
              let mut nodes = interests
                  .liked
                  .iter()
                  .map(|interest| serde_json::json!({
                      "id": interest.id,
                      "label": interest.name,
                      "kind": "interest",
                  }))
                  .collect::<Vec<_>>();
              nodes.extend(wishlist.records.iter().flat_map(|record| {
                  record.items.iter().map(|item| serde_json::json!({
                      "id": item.id,
                      "label": item.search_text(),
                      "kind": "wishlist",
                  }))
              }));
              let count = nodes.len();
              drop(wishlist);
              drop(interests);
              Ok(routing::accepted_response(serde_json::json!({
                  "nodes": nodes,
                  "edges": [],
                  "count": count,
                  "status": if count == 0 { "empty" } else { "ready" },
              }).to_string()))
          }

        ("POST", "/api/jobs/discography") => {
              if route.path.starts_with("/api/v0/") {
                  let payload = match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(payload @ serde_json::Value::Object(_)) => payload,
                      _ => {
                          return Ok(routing::bad_request_response(
                              "discography job body must be an object",
                          ));
                      }
                  };
                  let has_artist = ["artist", "artist_id", "query"].iter().any(|field| {
                      payload
                          .get(*field)
                          .and_then(serde_json::Value::as_str)
                          .is_some_and(|value| !value.trim().is_empty())
                  });
                  if !has_artist {
                      return Ok(routing::bad_request_response("artist_id is required"));
                  }
              }
              let artist = extract_json_string_field(body, "artist")
                  .or_else(|| extract_json_string_field(body, "artist_id"))
                  .or_else(|| extract_json_string_field(body, "query"))
                  .unwrap_or_else(|| "discography".to_owned());
              let query = format!("{} discography", artist.trim()).trim().to_owned();
              let (previous_searches, mutated_searches, record, evicted, expired) = {
                  let mut searches = state.searches.write().await;
                  let previous_searches = searches.clone();
                  let outcome = match searches.create(None, query, "global", None, Vec::new(), DEFAULT_SEARCH_TTL_SECONDS) {
                      Ok(outcome) => outcome,
                      Err(error) => return Ok(search_create_error_response(error)),
                  };
                  let record = outcome.record;
                  let evicted = outcome.evicted;
                  let expired = outcome.expired;
                  let mutated_searches = searches.clone();
                  (previous_searches, mutated_searches, record, evicted, expired)
              };
              let job_projection = serde_json::json!({
                  "jobId": record.id,
                  "type": "discography",
                  "artistId": artist,
                  "artistName": artist,
                  "profile": "CoreDiscography",
                  "targetDirectory": "",
                  "releaseJobIds": [],
                  "releaseIds": [],
                  "totalReleases": 0,
                  "completedReleases": 0,
                  "failedReleases": 0,
                  "status": "Pending",
                  "createdAt": record.created_at.to_string(),
              });
              let job_store_result = state
                  .controller_features
                  .write()
                  .await
                  .upsert(format!("job/discography/{}", record.id), job_projection);
              if let Err(error) = job_store_result {
                  rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                  return Ok(routing::service_unavailable_response(&error));
              }
              let response = serde_json::json!({
                  "id": record.id,
                  "search_id": record.id,
                  "token": record.token,
                  "status": "queued",
                  "kind": "discography",
                  "artist": artist,
                  "query": record.query,
                  "results": [],
              }).to_string();
              let mut upserts = expired.clone();
              upserts.push(record.clone());
              if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                  rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                  let _ = state
                      .controller_features
                      .write()
                      .await
                      .remove(&format!("job/discography/{}", record.id));
                  return Ok(routing::service_unavailable_response(&error));
              }
              for expired_record in &expired {
                  publish_search_hub_event(state, "update", expired_record);
              }
              Ok(routing::accepted_response(response))
          }

          ("POST", "/api/jobs/mb-release") => {
              if route.path.starts_with("/api/v0/") {
                  let payload = match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(payload @ serde_json::Value::Object(_)) => payload,
                      _ => {
                          return Ok(routing::bad_request_response(
                              "MusicBrainz release job body must be an object",
                          ));
                      }
                  };
                  let has_release_or_search = [
                      "mb_release_id",
                      "mbReleaseId",
                      "artist",
                      "title",
                      "release",
                      "query",
                  ]
                  .iter()
                  .any(|field| {
                      payload
                          .get(*field)
                          .and_then(serde_json::Value::as_str)
                          .is_some_and(|value| !value.trim().is_empty())
                  });
                  if !has_release_or_search {
                      return Ok(routing::bad_request_response("mb_release_id is required"));
                  }
              }
              let release_id = extract_json_string_field(body, "mb_release_id")
                  .or_else(|| extract_json_string_field(body, "mbReleaseId"))
                  .filter(|id| !id.trim().is_empty());
              let (artist, title) = if let Some(release_id) =
                  release_id.filter(|_| route.path.starts_with("/api/v0/"))
              {
                  let musicbrainz = state.integration_settings.read().await.musicbrainz.clone();
                  match musicbrainz_release_target_with_settings(&musicbrainz, &release_id).await
                  {
                      Ok(Some(target)) => target,
                      Ok(None) => {
                          return Ok(HttpResponse {
                              status: "404 Not Found",
                              content_type: "application/json",
                              body: serde_json::json!(
                                  "Unable to resolve release into a SongID-ready MusicBrainz target."
                              )
                              .to_string(),
                          });
                      }
                      Err(error) => {
                          return Ok(routing::service_unavailable_response(&format!(
                              "MusicBrainz lookup failed: {error}"
                          )));
                      }
                  }
              } else {
                  let artist = extract_json_string_field(body, "artist").unwrap_or_default();
                  let title = extract_json_string_field(body, "title")
                      .or_else(|| extract_json_string_field(body, "release"))
                      .or_else(|| extract_json_string_field(body, "query"))
                      .unwrap_or_else(|| "release".to_owned());
                  (artist, title)
              };
              let query = [artist.as_str(), title.as_str()]
                  .into_iter()
                  .filter(|value| !value.trim().is_empty())
                  .collect::<Vec<_>>()
                  .join(" ");
              let (previous_searches, mutated_searches, record, evicted, expired) = {
                  let mut searches = state.searches.write().await;
                  let previous_searches = searches.clone();
                  let outcome = match searches.create(None, query, "global", None, Vec::new(), DEFAULT_SEARCH_TTL_SECONDS) {
                      Ok(outcome) => outcome,
                      Err(error) => return Ok(search_create_error_response(error)),
                  };
                  let record = outcome.record;
                  let evicted = outcome.evicted;
                  let expired = outcome.expired;
                  let mutated_searches = searches.clone();
                  (previous_searches, mutated_searches, record, evicted, expired)
              };
              let job_projection = serde_json::json!({
                  "jobId": record.id,
                  "type": "mb-release",
                  "artistId": artist,
                  "artistName": artist,
                  "profile": "AllReleases",
                  "targetDirectory": "",
                  "releaseJobIds": [],
                  "releaseIds": [],
                  "totalReleases": 0,
                  "completedReleases": 0,
                  "failedReleases": 0,
                  "status": "Pending",
                  "createdAt": record.created_at.to_string(),
              });
              let job_store_result = state
                  .controller_features
                  .write()
                  .await
                  .upsert(format!("job/mb-release/{}", record.id), job_projection);
              if let Err(error) = job_store_result {
                  rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                  return Ok(routing::service_unavailable_response(&error));
              }
              let response = serde_json::json!({
                  "id": record.id,
                  "search_id": record.id,
                  "token": record.token,
                  "status": "queued",
                  "kind": "mb-release",
                  "artist": artist,
                  "title": title,
                  "query": record.query,
                  "results": [],
              }).to_string();
              let mut upserts = expired.clone();
              upserts.push(record.clone());
              if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                  rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                  let _ = state
                      .controller_features
                      .write()
                      .await
                      .remove(&format!("job/mb-release/{}", record.id));
                  return Ok(routing::service_unavailable_response(&error));
              }
              for expired_record in &expired {
                  publish_search_hub_event(state, "update", expired_record);
              }
              Ok(routing::accepted_response(response))
          }

          ("POST", "/api/options/yaml/validate") => {
              if let Some(response) = controller_options_validation_failure_response(state) {
                  return Ok(response);
              }
              if !effective_remote_configuration(state) {
                  return Ok(controller_forbidden_response());
              }
              match controller_options_config_validate_response(
                  body,
                  state.config.controller_profile,
              ) {
                  Ok(response) => Ok(response),
                  Err(error) => Ok(routing::bad_request_response(&error)),
              }
          }

          ("POST", "/api/source-feed-imports/preview") => {
              if route.path.starts_with("/api/v0/") {
                  let request = serde_json::from_str::<serde_json::Value>(body)
                      .unwrap_or(serde_json::Value::Null);
                  let raw = extract_json_string_field(body, "sourceText")
                      .or_else(|| extract_json_string_field(body, "text"))
                      .unwrap_or_default();
                  if raw.trim().is_empty() {
                      return Ok(routing::bad_request_response("SourceText is required"));
                  }
                  let requested_limit = extract_json_u64_field(body, "limit").unwrap_or(500);
                  if requested_limit == 0 {
                      return Ok(routing::bad_request_response("Limit must be greater than 0"));
                  }
                  let maximum = state
                      .integration_settings
                      .read()
                      .await
                      .spotify
                      .max_items_per_import;
                  let safe_limit = requested_limit.min(maximum).max(1);
                  let requested_kind = extract_json_string_field(body, "sourceKind")
                      .unwrap_or_else(|| "auto".to_owned())
                      .trim()
                      .to_ascii_lowercase();
                  let fetch_provider_urls = request
                      .get("fetchProviderUrls")
                      .and_then(serde_json::Value::as_bool)
                      .unwrap_or(true);
                  if fetch_provider_urls && looks_like_spotify_source(&raw, &requested_kind) {
                      let provider_access_token = extract_json_string_field(
                          body,
                          "providerAccessToken",
                      )
                      .unwrap_or_default();
                      let result = preview_spotify_source_feed(
                          state,
                          &raw,
                          &provider_access_token,
                          usize::try_from(safe_limit).unwrap_or(usize::MAX),
                          "https://accounts.spotify.com/api/token",
                          "https://api.spotify.com/v1",
                      )
                      .await?;
                      let mut history = state.source_feed_import_history.write().await;
                      let previous = history.clone();
                      history.record(&request, &raw, &result);
                      if let Err(error) = history.persist(&state.config.state_dir) {
                          *history = previous;
                          return Ok(routing::service_unavailable_response(&error));
                      }
                      drop(history);
                      return Ok(HttpResponse {
                          status: "200 OK",
                          content_type: "application/json; charset=utf-8",
                          body: result.to_string(),
                      });
                  }
                  if fetch_provider_urls {
                      if let Some(result) = preview_configured_provider_source_feed(
                          state,
                          &raw,
                          &requested_kind,
                          usize::try_from(safe_limit).unwrap_or(usize::MAX),
                          "https://www.googleapis.com/youtube/v3/playlistItems",
                          "https://ws.audioscrobbler.com/2.0/",
                          "https://itunes.apple.com/lookup",
                          "https://api.listenbrainz.org",
                      )
                      .await?
                      {
                          let mut history = state.source_feed_import_history.write().await;
                          let previous = history.clone();
                          history.record(&request, &raw, &result);
                          if let Err(error) = history.persist(&state.config.state_dir) {
                              *history = previous;
                              return Ok(routing::service_unavailable_response(&error));
                          }
                          drop(history);
                          return Ok(HttpResponse {
                              status: "200 OK",
                              content_type: "application/json; charset=utf-8",
                              body: result.to_string(),
                          });
                      }
                  }
                  let include_album = request
                      .get("includeAlbum")
                      .and_then(serde_json::Value::as_bool)
                      .unwrap_or(false);
                  let result = preview_local_source_feed(
                      &raw,
                      &requested_kind,
                      include_album,
                      usize::try_from(safe_limit).unwrap_or(usize::MAX),
                  );
                  let mut history = state.source_feed_import_history.write().await;
                  let previous = history.clone();
                  history.record(&request, &raw, &result);
                  if let Err(error) = history.persist(&state.config.state_dir) {
                      *history = previous;
                      return Ok(routing::service_unavailable_response(&error));
                  }
                  drop(history);
                  return Ok(HttpResponse {
                      status: "200 OK",
                      content_type: "application/json; charset=utf-8",
                      body: result.to_string(),
                  });
              }
              let raw = extract_json_string_field(body, "sourceText")
                  .or_else(|| extract_json_string_field(body, "text"))
                  .or_else(|| extract_json_string_field(body, "content"))
                  .or_else(|| extract_json_string_field(body, "playlist"))
                  .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
              let items = raw
                  .lines()
                  .map(str::trim)
                  .filter(|line| !line.is_empty())
                  .enumerate()
                  .map(|(index, line)| {
                      let (artist, title) = line
                          .split_once(" - ")
                          .map(|(artist, title)| (artist.trim(), title.trim()))
                          .unwrap_or(("", line));
                      serde_json::json!({
                          "id": format!("preview-{}", index + 1),
                          "artist": artist,
                          "title": title,
                          "searchText": line,
                          "valid": !line.is_empty(),
                      })
                  })
                  .collect::<Vec<_>>();
              let count = items.len();
              Ok(routing::ok_response(serde_json::json!({
                  "items": items,
                  "count": count,
                  "valid": count > 0,
              }).to_string()))
          }

          // TASTE RECOMMENDATIONS POST ENDPOINTS (Phase 6)
          ("POST", "/api/taste-recommendations") => {
              if route.path.starts_with("/api/v0/") && !body.trim().is_empty() {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Null | serde_json::Value::Object(_)) => {}
                      Ok(_) | Err(_) => {
                          return Ok(routing::bad_request_response("The request body is invalid"));
                      }
                  }
              }
              let interests = state.interests.read().await;
              let mut value = serde_json::from_str::<serde_json::Value>(
                  &interests.recommendations_json("recommendations"),
              )
              .unwrap_or_else(|_| serde_json::json!({ "recommendations": [], "count": 0 }));
              drop(interests);
              if route.path.starts_with("/api/v0/") {
                  let recommendations = value
                      .get("recommendations")
                      .and_then(serde_json::Value::as_array)
                      .cloned()
                      .unwrap_or_default();
                  let minimum_trusted_sources = extract_json_u64_field(body, "minimumTrustedSources")
                      .unwrap_or(2)
                      .max(2);
                  return Ok(routing::ok_response(serde_json::json!({
                      "minimumTrustedSources": minimum_trusted_sources,
                      "trustedActorCount": 0,
                      "candidateCount": recommendations.len(),
                      "recommendations": recommendations,
                  }).to_string()));
              }
              value["status"] = serde_json::Value::String("analyzing".to_owned());
              let json = value.to_string();
              Ok(routing::accepted_response(json))
          }

          ("POST", "/api/taste-recommendations/graph-preview") => {
              if route.path.starts_with("/api/v0/") {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Object(_)) => {}
                      Ok(serde_json::Value::Null) | Err(_) => {
                          return Ok(routing::bad_request_response(
                              "graph preview request is required",
                          ));
                      }
                      Ok(_) => return Ok(routing::bad_request_response("The request body is invalid")),
                  }
              }
              if route.path.starts_with("/api/v0/")
                  && serde_json::from_str::<serde_json::Value>(body)
                      .ok()
                      .and_then(|value| value.get("workRef").cloned())
                      .and_then(|value| value.get("@context").cloned())
                      .is_some_and(|value| value.is_null())
              {
                  return Ok(native_model_validation_response());
              }
              let interests = state.interests.read().await;
              let graph_data = interests
                  .liked
                  .iter()
                  .map(|interest| {
                      serde_json::json!({
                          "id": interest.id,
                          "label": interest.name,
                          "kind": "interest",
                      })
                  })
                  .collect::<Vec<_>>();
              let nodes = graph_data.len();
              drop(interests);
              let json = serde_json::json!({
                  "graph_data": graph_data,
                  "nodes": nodes,
                  "edges": 0,
              }).to_string();
              Ok(routing::ok_response(json))
          }

          ("POST", "/api/taste-recommendations/release-radar") => {
              if route.path.starts_with("/api/v0/") {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Object(_)) => {}
                      Ok(serde_json::Value::Null) | Err(_) => {
                          return Ok(routing::bad_request_response(
                              "radar subscription request is required",
                          ));
                      }
                      Ok(_) => return Ok(routing::bad_request_response("The request body is invalid")),
                  }
              }
              if route.path.starts_with("/api/v0/")
                  && serde_json::from_str::<serde_json::Value>(body)
                      .ok()
                      .and_then(|value| value.get("workRef").cloned())
                      .and_then(|value| value.get("@context").cloned())
                      .is_some_and(|value| value.is_null())
              {
                  return Ok(native_model_validation_response());
              }
              let wishlist = state.wishlist.read().await;
              let recommendations = wishlist
                  .records
                  .iter()
                  .flat_map(|record| record.items.iter())
                  .map(|item| {
                      serde_json::json!({
                          "id": item.id,
                          "artist": item.artist,
                          "title": item.title,
                          "searchText": item.search_text(),
                          "source": "wishlist",
                      })
                  })
                  .collect::<Vec<_>>();
              let count = recommendations.len();
              drop(wishlist);
              let json = serde_json::json!({
                  "recommendations": recommendations,
                  "count": count,
                  "status": "processing",
              }).to_string();
              Ok(if route.path.starts_with("/api/v0/") {
                  routing::ok_response(json)
              } else {
                  routing::accepted_response(json)
              })
          }

          ("POST", "/api/taste-recommendations/wishlist") => {
              if route.path.starts_with("/api/v0/") {
                  match serde_json::from_str::<serde_json::Value>(body) {
                      Ok(serde_json::Value::Object(_)) => {}
                      Ok(serde_json::Value::Null) | Err(_) => {
                          return Ok(routing::bad_request_response(
                              "promotion request is required",
                          ));
                      }
                      Ok(_) => return Ok(routing::bad_request_response("The request body is invalid")),
                  }
              }
              let request = serde_json::from_str::<serde_json::Value>(body)
                  .unwrap_or_else(|_| serde_json::json!({}));
              let work_ref = request.get("workRef").cloned().unwrap_or_default();
              if route.path.starts_with("/api/v0/")
                  && work_ref.get("@context").is_some_and(serde_json::Value::is_null)
              {
                  return Ok(native_model_validation_response());
              }
              // Matches the oracle's real PromoteToWishlistAsync: a
              // recommendable WorkRef either promotes to a real, honest
              // review-only (disabled, no auto-download) Wishlist seed,
              // or -- if a wishlist item with the same search text
              // already exists -- reports the existing one, rather than
              // just echoing the caller's own current wishlist back as
              // if a promotion had happened.
              if !work_ref_is_recommendable(&work_ref) {
                  return Ok(HttpResponse {
                      status: "400 Bad Request",
                      content_type: "application/json",
                      body: serde_json::json!({
                          "created": false,
                          "message": "WorkRef is not a safe music recommendation.",
                      })
                      .to_string(),
                  });
              }
              let search_text = work_ref_search_text(&work_ref);
              let mut wishlist = state.wishlist.write().await;
              if let Some(existing_id) = wishlist.item_id_for_search_text(&search_text) {
                  drop(wishlist);
                  return Ok(routing::ok_response(
                      serde_json::json!({
                          "created": false,
                          "wishlistItemId": existing_id,
                          "searchText": search_text,
                          "message": "Wishlist already has this recommendation seed.",
                      })
                      .to_string(),
                  ));
              }
              let previous = wishlist.clone();
              let creator = work_ref
                  .get("creator")
                  .and_then(serde_json::Value::as_str)
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let title = work_ref
                  .get("title")
                  .and_then(serde_json::Value::as_str)
                  .unwrap_or_default()
                  .trim()
                  .to_owned();
              let note = request.get("note").and_then(serde_json::Value::as_str);
              let filter = work_ref_wishlist_filter(&work_ref, note);
              let Ok(item) = wishlist.add_item_with_settings(
                  creator,
                  title,
                  "TasteRecommendation".to_owned(),
                  filter,
                  false,
                  false,
                  25,
                  None,
              ) else {
                  drop(wishlist);
                  return Ok(routing::service_unavailable_response(
                      "wishlist item capacity is full",
                  ));
              };
              let mutated = wishlist.clone();
              drop(wishlist);
              if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                  rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                  return Ok(routing::service_unavailable_response(&error));
              }
              Ok(routing::ok_response(
                  serde_json::json!({
                      "created": true,
                      "wishlistItemId": item.id,
                      "searchText": search_text,
                      "message": "Created review-only Wishlist seed.",
                  })
                  .to_string(),
              ))
          }

          // PLAYER LAUNCH ENDPOINT (Phase 6)
        ("POST", "/api/player/external-visualizer/launch") => {
            let visualizer = state
                .media_services
                .read()
                .await
                .external_visualizer
                .clone();
            if !visualizer.launch_enabled {
                record_event(
                    state,
                    "external_visualizer.launch.blocked",
                    "external_visualizer",
                    Some("launch is disabled by configuration".to_owned()),
                )
                .await;
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "started": false,
                        "name": serde_json::Value::Null,
                        "processId": serde_json::Value::Null,
                        "error": "External visualizer launching is disabled in configuration.",
                    })
                    .to_string(),
                });
            }
            if let Some(command) = resolve_external_visualizer_path(visualizer.command.as_deref()) {
                let Ok(process_permit) = Arc::clone(&state.external_visualizer_processes)
                    .try_acquire_owned()
                else {
                    return Ok(routing::HttpResponse {
                        status: "503 Service Unavailable",
                        content_type: "application/json",
                        body: "{\"error\":\"external visualizer process limit reached\"}".to_owned(),
                    });
                };
                let mut process = std::process::Command::new(&command);
                process.args(
                    visualizer
                        .arguments
                        .iter()
                        .filter(|argument| !argument.trim().is_empty()),
                );
                if let Some(directory) = resolve_external_visualizer_working_directory(
                    visualizer.working_directory.as_deref(),
                    Some(&command),
                ) {
                    process.current_dir(directory);
                }
                match process.spawn() {
                    Ok(mut child) => {
                        let process_id = child.id();
                        let name = if visualizer.name.trim().is_empty() {
                            "External visualizer".to_owned()
                        } else {
                            visualizer.name.trim().to_owned()
                        };
                        tokio::task::spawn_blocking(move || {
                            let _process_permit = process_permit;
                            let _ = child.wait();
                        });
                        record_event(
                            state,
                            "external_visualizer.launch",
                            "external_visualizer".to_owned(),
                            Some("launch requested".to_owned()),
                        )
                        .await;
                        Ok(routing::ok_response(serde_json::json!({
                            "started": true,
                            "name": name,
                            "processId": process_id,
                            "error": serde_json::Value::Null,
                        }).to_string()))
                    }
                    Err(error) => {
                        drop(process_permit);
                        record_event(
                            state,
                            "external_visualizer.launch.failed",
                            "external_visualizer".to_owned(),
                            Some("launch failed".to_owned()),
                        )
                        .await;
                        Ok(HttpResponse {
                            status: "400 Bad Request",
                            content_type: "application/json",
                            body: serde_json::json!({
                                "started": false,
                                "name": serde_json::Value::Null,
                                "processId": serde_json::Value::Null,
                                "error": error.to_string(),
                            })
                            .to_string(),
                        })
                    }
                }
            } else {
                record_event(
                    state,
                    "external_visualizer.launch.failed",
                    "external_visualizer".to_owned(),
                    Some("launch failed".to_owned()),
                )
                .await;
                Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "started": false,
                        "name": serde_json::Value::Null,
                        "processId": serde_json::Value::Null,
                        "error": "External visualizer path is not configured or does not exist.",
                    })
                    .to_string(),
                })
            }
        }

          // BANS & BLOCKING ENDPOINTS
        ("GET", path) if security_ban_route_tail(path).is_some_and(|tail| tail.is_empty()) => {
            let security = state.security.read().await;
            let json = security.json_value().to_string();
            drop(security);
            Ok(routing::ok_response(json))
        }

        ("POST", path)
            if security_ban_route_tail(path).as_deref() == Some(["username"].as_slice()) =>
        {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let Some(username) = normalize_security_ban_value("username", &username) else {
                return Ok(routing::bad_request_response("username is required"));
            };
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let mut security = state.security.write().await;
            let previous = security.clone();
            let Some(record) = security.ban_with_options(
                "username",
                username.clone(),
                reason,
                duration_seconds,
                is_permanent,
            ) else {
                return Ok(routing::service_unavailable_response("security ban capacity is full"));
            };
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    *security = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(String::new()));
            }
            Ok(routing::ok_response(serde_json::json!({
                "username": username,
                "banned": true,
                "persisted": persisted,
                "kind": record.kind,
                "created_at": record.created_at,
                "activeBans": active_bans,
            }).to_string()))
        }

        ("DELETE", path)
            if security_ban_route_tail(path)
                .is_some_and(|tail| tail.len() == 2 && tail[0] == "username") =>
        {
            let username = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let Some(username) = normalize_security_ban_value("username", &username) else {
                return Ok(routing::bad_request_response("username is required"));
            };
            let mut security = state.security.write().await;
            let previous = security.clone();
            let removed = security.unban("username", &username);
            let persisted = if removed {
                match persist_security_unban(state, "username", &username).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        *security = previous;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(if removed {
                    routing::ok_response(String::new())
                } else {
                    routing::not_found_response()
                });
            }
            Ok(routing::ok_response(serde_json::json!({
                "username": username,
                "banned": false,
                "removed": removed,
                "persisted": persisted,
                "activeBans": active_bans,
            }).to_string()))
        }

        ("POST", path)
            if security_ban_route_tail(path).as_deref() == Some(["ip"].as_slice()) =>
        {
            let ip = extract_json_string_field(body, "ipAddress")
                .or_else(|| extract_json_string_field(body, "ip"))
                .unwrap_or_default();
            let Some(ip) = normalize_security_ban_value("ip", &ip) else {
                return Ok(routing::bad_request_response("valid ip is required"));
            };
            let (reason, duration_seconds, is_permanent) = security_ban_options(body);
            let mut security = state.security.write().await;
            let previous = security.clone();
            let Some(record) = security.ban_with_options(
                "ip",
                ip.clone(),
                reason,
                duration_seconds,
                is_permanent,
            ) else {
                return Ok(routing::service_unavailable_response("security ban capacity is full"));
            };
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    *security = previous;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(routing::ok_response(String::new()));
            }
            Ok(routing::ok_response(serde_json::json!({
                "ip": ip,
                "banned": true,
                "persisted": persisted,
                "kind": record.kind,
                "created_at": record.created_at,
                "activeBans": active_bans,
            }).to_string()))
        }

        ("DELETE", path)
            if security_ban_route_tail(path)
                .is_some_and(|tail| tail.len() == 2 && tail[0] == "ip") =>
        {
            let ip = decoded_path_segment(path.rsplit('/').next().unwrap_or(""));
            let Some(ip) = normalize_security_ban_value("ip", &ip) else {
                return Ok(routing::bad_request_response("valid ip is required"));
            };
            let mut security = state.security.write().await;
            let previous = security.clone();
            let removed = security.unban("ip", &ip);
            let persisted = if removed {
                match persist_security_unban(state, "ip", &ip).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        *security = previous;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = security.active_bans();
            drop(security);
            if route.path.starts_with("/api/v0/") {
                return Ok(if removed {
                    routing::ok_response(String::new())
                } else {
                    routing::not_found_response()
                });
            }
            Ok(routing::ok_response(serde_json::json!({
                "ip": ip,
                "banned": false,
                "removed": removed,
                "persisted": persisted,
                "activeBans": active_bans,
            }).to_string()))
        }

        // ADDITIONAL MISSING DELETE ENDPOINTS (Phase 5)
        ("DELETE", path) if path_segment_after(path, "/api/conversations/").is_some() => {
            let Some(username) = path_segment_after(path, "/api/conversations/") else {
                return Ok(routing::not_found_response());
            };
            let username = decoded_path_segment(username).trim().to_owned();
            if username.is_empty() {
                return Ok(routing::bad_request_response("username is required"));
            }
            let mut messages = state.messages.write().await;
            let previous = messages.clone();
            let before = messages.records.len();
            messages.records.retain(|record| record.username != username);
            let removed = before.saturating_sub(messages.records.len());
            let mutated = messages.clone();
            drop(messages);
            let persisted_removed = match persist_conversation_delete_checked(state, &username).await {
                Ok(removed) => removed.unwrap_or(0),
                Err(error) => {
                    rollback_messages_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            if route.path.starts_with("/api/v0/") {
                Ok(if removed > 0 || persisted_removed > 0 {
                    routing::no_content_response()
                } else {
                    routing::not_found_response()
                })
            } else {
                Ok(routing::ok_response(
                    (removed > 0 || persisted_removed > 0).to_string(),
                ))
            }
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/directories/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", path) if path.starts_with("/api/files/") && path.contains("/files/") => {
            Ok(routing::ok_response("false".to_owned()))
        }

        ("DELETE", "/api/integrations/spotify") => {
            if let Err(error) = delete_spotify_connection_store(state) {
                return Ok(routing::service_unavailable_response(&error));
            }
            *state.spotify_connection.write().await = SpotifyConnectionStore::default();
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response("{\"connected\":false,\"removed\":true}".to_owned()))
            }
        }

        ("DELETE", "/api/nowplaying") => {
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let cleared = now_playing.clear();
            let mutated = now_playing.clone();
            drop(now_playing);
            if let Err(error) = persist_now_playing_clear_checked(state).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            if route.path.starts_with("/api/v0/") {
                Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                })
            } else {
                Ok(routing::ok_response(format!(
                    "{{\"now_playing\":[],\"count\":0,\"cleared\":true,\"cleared_count\":{}}}",
                    cleared
                )))
            }
        }

        ("DELETE", "/api/relay") => {
            let json = match mutate_runtime_compat_state(state, |_, relay| relay.set_enabled(false))
                .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json.to_string()))
        }

        ("DELETE", "/api/relay/agent") => {
            let json = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.set_relay_agent(false).to_string()
            })
            .await
            {
                Ok(json) => json,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(json))
        }

        ("DELETE", "/api/shares") => {
            if route.path.starts_with("/api/v0/") {
                if state.share_scans.available_permits() == 0 {
                    state.share_scans.add_permits(1);
                    Ok(routing::no_content_response())
                } else {
                    Ok(routing::not_found_response())
                }
            } else {
                Ok(routing::ok_response("true".to_owned()))
            }
        }

        ("GET", "/api/shares/contents") => {
            let shares = state.shares.read().await;
            if !shares.scan_errors.is_empty() {
                drop(shares);
                return Ok(routing::internal_server_error_response(
                    "share browse unavailable",
                ));
            }
            let json = controller_share_directories_json(&shares.entries, None);
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("GET", path) if path.starts_with("/api/shares/") && path.ends_with("/contents") => {
            let Some(share_id) = share_contents_id(path) else {
                return Ok(routing::not_found_response());
            };
            let shares = state.shares.read().await;
            let Some(root) = shares
                .roots
                .iter()
                .find(|root| share_root_id(&root.label) == share_id)
            else {
                return Ok(routing::not_found_response());
            };
            if !shares.scan_errors.is_empty() {
                drop(shares);
                return Ok(routing::internal_server_error_response(
                    "share browse unavailable",
                ));
            }
            let json = controller_share_directories_json(&shares.entries, Some(&root.label));
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("GET", path) if path.starts_with("/api/shares/") => {
            let Some(share_id) = share_resource_id(path) else {
                return Ok(routing::not_found_response());
            };
            let shares = state.shares.read().await;
            let Some(root) = shares
                .roots
                .iter()
                .find(|root| share_root_id(&root.label) == share_id)
            else {
                return Ok(routing::not_found_response());
            };
            let json = controller_share_value(root).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }

        ("DELETE", path) if path.starts_with("/api/transfers/") && path.ends_with("/all/completed") => {
            let mut transfers = state.transfers.write().await;
            let before = transfers.entries.len();
            transfers.entries.retain(|entry| {
                !matches!(
                    entry.status.as_str(),
                    "succeeded" | "completed" | "cancelled" | "failed" | "rejected"
                )
            });
            let pruned = before.saturating_sub(transfers.entries.len());
            transfers.persist_state();
            drop(transfers);
            Ok(routing::ok_response(format!("{{\"pruned\":{}}}", pruned)))
        }

        // Generic :var pattern endpoints for mesh/network cleanup & channels (Phase 5)
        ("DELETE", path) if path.contains("/cleanup") && path.matches('/').count() == 3 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/unpublish") && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        ("DELETE", path) if path.contains("/channels/") && path.matches('/').count() == 4 && !path.contains("/api/") => {
            Ok(routing::not_found_response())
        }

        // ADDITIONAL MISSING INTEGRATION & PLATFORM ENDPOINTS (Phase 5)
        ("GET", "/api/integrations/spotify/status") => {
            let spotify = state.integration_settings.read().await.spotify.clone();
            let connection = state.spotify_connection.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: connection.status_json(spotify.configured()).to_string(),
            })
        }

        ("POST", "/api/integrations/spotify/authorize") => {
            let spotify = state.integration_settings.read().await.spotify.clone();
            if !spotify.configured() {
                return Ok(routing::bad_request_response(
                    "Spotify authorization is not configured.",
                ));
            }
            let client_id = spotify.client_id.as_deref().unwrap_or_default();
            let redirect_uri = spotify_redirect_uri(state, &spotify);
            let (state_token, oauth_record, previous, mutated) = {
                let mut oauth_states = state.oauth_states.write().await;
                let previous = oauth_states.clone();
                let Some(state_token) = oauth_states.issue("spotify", &redirect_uri, 600) else {
                    return Ok(routing::service_unavailable_response(
                        "OAuth state capacity is full",
                    ));
                };
                let oauth_record = oauth_states.records.get(&state_token).cloned();
                let mutated = oauth_states.clone();
                (state_token, oauth_record, previous, mutated)
            };
            if let Some(oauth_record) = oauth_record.as_ref() {
                if let Err(error) =
                    persist_oauth_state_checked(state, &state_token, oauth_record).await
                {
                    rollback_oauth_states_if_unchanged(state, previous, &mutated).await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            }
            let code_verifier = oauth_record
                .as_ref()
                .and_then(|record| record.code_verifier.as_deref())
                .ok_or_else(|| "Spotify PKCE verifier generation failed".to_owned())?;
            let code_challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
            let authorization_url = format!(
                "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&state={}&code_challenge_method=S256&code_challenge={}",
                url_encode(client_id),
                url_encode(&spotify.scopes),
                url_encode(&redirect_uri),
                url_encode(&state_token),
                url_encode(&code_challenge),
            );
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: serde_json::json!({
                    "authorizationUrl": authorization_url,
                    "redirectUri": redirect_uri,
                    "scope": spotify.scopes,
                })
                .to_string(),
            })
        }

        ("GET", "/api/integrations/spotify/callback") => {
            let params = route.query.map(query_params).unwrap_or_default();
            let code = params
                .iter()
                .find(|(key, _)| key == "code")
                .map(|(_, value)| value.as_str());
            let error = params
                .iter()
                .find(|(key, _)| key == "error")
                .map(|(_, value)| value.as_str());
            let state_value = params
                .iter()
                .find(|(key, _)| key == "state")
                .map(|(_, value)| value.as_str())
                .unwrap_or("");

            if error.is_some_and(|value| !value.trim().is_empty()) {
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/html; charset=utf-8",
                    body: spotify_callback_html("Spotify authorization failed."),
                });
            }
            let Some(code) = code.filter(|value| !value.trim().is_empty()) else {
                return Ok(routing::bad_request_response(
                    "Missing Spotify authorization code or state.",
                ));
            };
            if state_value.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "Missing Spotify authorization code or state.",
                ));
            }
            let pending = match consume_oauth_state(state, "spotify", state_value).await {
                Ok(Some(record)) if record.code_verifier.is_some() => record,
                Ok(_) => {
                    return Ok(routing::bad_request_response(
                        "Spotify authorization could not be completed.",
                    ))
                }
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let spotify = state.integration_settings.read().await.spotify.clone();
            match complete_spotify_authorization(
                state,
                &spotify,
                &pending,
                code,
                "https://accounts.spotify.com/api/token",
                "https://api.spotify.com/v1/me",
            )
            .await
            {
                Ok(_) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "text/html; charset=utf-8",
                    body: spotify_callback_html(
                        "Spotify account connected. You can close this window.",
                    ),
                }),
                Err(error) => {
                    record_daemon_log(state, logging::LogLevel::Warn, "spotify", error).await;
                    Ok(routing::internal_server_error_response(
                        "Spotify authorization could not be completed.",
                    ))
                }
            }
        }

        ("GET", "/api/integrations/lidarr/sync/status") => {
            let sync = state.lidarr_sync_state.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: sync.json().to_string(),
            })
        }

        ("GET", "/api/integrations/lidarr/status") => {
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            match fetch_lidarr_system_status(&lidarr).await {
                Ok(value) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: value.to_string(),
                }),
                Err(error) => Ok(routing::service_unavailable_response(&error)),
            }
        }

        ("GET", "/api/integrations/lidarr/wanted/missing") => {
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            if !lidarr.configured() {
                let library = state.library.read().await;
                let missing_albums = lidarr_missing_albums_value(&library);
                let count = missing_albums.len();
                let updated_at = library.updated_at;
                drop(library);
                return Ok(routing::ok_response(serde_json::json!({
                    "missing_albums": missing_albums,
                    "count": count,
                    "status": if count == 0 { "local_clean" } else { "local" },
                    "source": "library-health",
                    "configured": false,
                    "next_action": if count == 0 { "library metadata is complete" } else { "fix library health issues or configure Lidarr URL and API key" },
                    "updated_at": updated_at,
                }).to_string()));
            }
            let page = query_parameter(route.query, "page")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1)
                .max(1);
            let page_size = query_parameter(route.query, "pageSize")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(50)
                .clamp(1, 250);
            match fetch_lidarr_wanted_missing(&lidarr, page, page_size).await {
                Ok(value) if route.path.starts_with("/api/v0/") => {
                    let mut records = value
                        .get("records")
                        .and_then(serde_json::Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    for record in &mut records {
                        let artist = record
                            .pointer("/artist/artistName")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        let title = record
                            .get("title")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default();
                        record["searchText"] = serde_json::Value::String(
                            [artist, title]
                                .into_iter()
                                .filter(|value| !value.trim().is_empty())
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                    }
                    let total = value
                        .get("totalRecords")
                        .cloned()
                        .unwrap_or_else(|| serde_json::json!(0));
                    Ok(HttpResponse {
                        status: "200 OK",
                        content_type: "application/json; charset=utf-8",
                        body: serde_json::json!({
                            "records": records,
                            "totalRecords": total,
                            "page": page,
                            "pageSize": page_size,
                        })
                        .to_string(),
                    })
                }
                Ok(value) => Ok(routing::ok_response(value.to_string())),
                Err(_) => Ok(routing::ok_response(
                    "{\"missing_albums\":[],\"count\":0,\"status\":\"connection_failed\",\"error\":\"Lidarr connection failed\"}".to_owned(),
                )),
            }
        }

        ("POST", "/api/integrations/lidarr/wanted/sync") => {
            if route.path.starts_with("/api/v0/") {
                let lidarr = state.integration_settings.read().await.lidarr.clone();
                let result = match sync_lidarr_wanted_to_wishlist(state, &lidarr).await {
                    Ok(result) => result,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: result.to_string(),
                });
            }
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            if !lidarr.configured() {
                let library = state.library.read().await;
                let missing_albums = lidarr_missing_albums_value(&library);
                let missing_count = missing_albums.len();
                drop(library);
                let body = match mutate_runtime_compat_state(state, |runtime, _| {
                    let mut value = runtime.record_lidarr_sync(missing_count, false);
                    if let Some(object) = value.as_object_mut() {
                        object.insert("missing_albums".to_owned(), serde_json::json!(missing_albums));
                        object.insert("source".to_owned(), serde_json::json!("library-health"));
                        object.insert(
                            "next_action".to_owned(),
                            serde_json::json!(if missing_count == 0 {
                                "library metadata is complete"
                            } else {
                                "fix library health issues or configure Lidarr URL and API key"
                            }),
                        );
                    }
                    value.to_string()
                }).await {
                    Ok(body) => body,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                return Ok(routing::accepted_response(body));
            }
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value = runtime.record_lidarr_sync(0, true);
                if let Some(object) = value.as_object_mut() {
                    object.insert("missing_albums".to_owned(), serde_json::json!([]));
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!("poll wanted/missing"),
                    );
                }
                value.to_string()
            }).await {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("GET", "/api/integrations/lidarr/manualimport/history")
        | ("GET", "/api/v0/integrations/lidarr/manualimport/history") => {
            let limit = match query_bounded_usize(route.query, "limit", 1, 500) {
                Ok(value) => value.unwrap_or(50),
                Err(_) => {
                    return Ok(routing::bad_request_response(
                        "limit must be between 1 and 500",
                    ))
                }
            };
            Ok(routing::ok_response(
                serde_json::Value::Array(list_lidarr_import_history(state, limit).await)
                    .to_string(),
            ))
        }

        ("POST", path)
            if (path.starts_with("/api/integrations/lidarr/manualimport/history/")
                || path.starts_with("/api/v0/integrations/lidarr/manualimport/history/"))
                && path.ends_with("/retry") =>
        {
            let prefix = if path.starts_with("/api/v0/") {
                "/api/v0/integrations/lidarr/manualimport/history/"
            } else {
                "/api/integrations/lidarr/manualimport/history/"
            };
            let Some(history_id) = path
                .strip_prefix(prefix)
                .and_then(|value| value.strip_suffix("/retry"))
                .filter(|value| uuid::Uuid::parse_str(value).is_ok())
            else {
                return Ok(routing::bad_request_response("HistoryId is required"));
            };
            let Some(history) = state
                .controller_features
                .read()
                .await
                .get(&lidarr_import_history_key(history_id))
                .cloned()
            else {
                return Ok(routing::not_found_response());
            };
            let Some(directory) = history
                .get("sourceDirectory")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
            else {
                return Ok(routing::bad_request_response(
                    "Import history record has no source directory",
                ));
            };
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            let result = match run_lidarr_import_with_history(
                state,
                &lidarr,
                directory,
                Some(history_id),
            )
            .await
            {
                Ok(result) => result,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::ok_response(result.to_string()))
        }

        ("POST", "/api/integrations/lidarr/manualimport") => {
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if route.path.starts_with("/api/v0/") {
                if directory.trim().is_empty() {
                    return Ok(routing::bad_request_response("Directory is required"));
                }
                let lidarr = state.integration_settings.read().await.lidarr.clone();
                let mut result = match run_lidarr_import_with_history(state, &lidarr, &directory, None).await
                {
                    Ok(result) => result,
                    Err(error) => return Ok(routing::service_unavailable_response(&error)),
                };
                // Keep rejected filenames available to the internal completed
                // download policy, but match the frozen controller's public
                // manual-import response shape.
                if let Some(object) = result.as_object_mut() {
                    object.remove("rejectedFilenames");
                }
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: result.to_string(),
                });
            }
            let lidarr = state.integration_settings.read().await.lidarr.clone();
            if !lidarr.configured() {
                let artist = extract_json_string_field(body, "artist")
                    .or_else(|| extract_json_string_field(body, "albumArtist"))
                    .unwrap_or_default();
                let title = extract_json_string_field(body, "title")
                    .or_else(|| extract_json_string_field(body, "album"))
                    .or_else(|| {
                        (!directory.trim().is_empty())
                            .then(|| directory.rsplit('/').next().unwrap_or(&directory).to_owned())
                    })
                    .unwrap_or_else(|| "Manual Import".to_owned());
                let kind =
                    extract_json_string_field(body, "kind").unwrap_or_else(|| "Audio".to_owned());
                let mut library = state.library.write().await;
                let mut runtime = state.runtime.write().await;
                let relay = state.relay.read().await;
                let previous_library = library.clone();
                let previous_runtime = runtime.clone();
                let Some(record) = library.create(artist, title, kind) else {
                    return Ok(routing::service_unavailable_response("library item capacity is full"));
                };
                let item = serde_json::from_str::<serde_json::Value>(&record.json())
                    .unwrap_or_else(|_| serde_json::json!({ "id": record.id }));
                let body = runtime
                    .record_lidarr_manual_import(1, false, directory, vec![item])
                    .to_string();
                if let Some(db) = state.db.as_ref() {
                    let runtime_record = runtime.persistence_record(&relay);
                    if let Err(error) = db
                        .upsert_library_item_and_runtime_compat_state(
                            &persisted_library_item(&record),
                            &runtime_record,
                        )
                        .await
                    {
                        *library = previous_library;
                        *runtime = previous_runtime;
                        return Ok(routing::service_unavailable_response(&format!(
                            "library persistence failed: Lidarr manual import transaction failed: {error}"
                        )));
                    }
                }
                drop(relay);
                drop(runtime);
                drop(library);
                return Ok(routing::accepted_response(body));
            }
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                let mut value =
                    runtime.record_lidarr_manual_import(0, true, directory, Vec::new());
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "next_action".to_owned(),
                        serde_json::json!("trigger Lidarr manual import from configured UI"),
                    );
                }
                value.to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("GET", "/api/musicbrainz/albums/completion") => {
            if route.path.starts_with("/api/v0/") {
                let targets = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/album-target/");
                if !targets.is_empty() {
                    let discovery = state.content_discovery.read().await;
                    let albums = targets
                        .iter()
                        .map(|target| musicbrainz_target_completion_value(target, &discovery))
                        .collect::<Vec<_>>();
                    drop(discovery);
                    return Ok(routing::ok_response(
                        serde_json::json!({"albums": albums}).to_string(),
                    ));
                }
            }
            let library = state.library.read().await;
            let mut value = serde_json::from_str::<serde_json::Value>(
                &library.musicbrainz_completion_json(),
            )
            .unwrap_or_else(|_| serde_json::json!({}));
            let albums = value["completion_status"].clone();
            value["albums"] = albums;
            drop(library);
            Ok(routing::ok_response(value.to_string()))
        }

        ("GET", path)
            if path.starts_with("/api/musicbrainz/artist/")
                && path.ends_with("/discography-coverage") =>
        {
            let Some(artist) = path_segment_between(
                path,
                "/api/musicbrainz/artist/",
                "/discography-coverage",
            ) else {
                return Ok(routing::not_found_response());
            };
            let artist = decoded_path_segment(artist);
            if uuid::Uuid::parse_str(&artist).is_ok() {
                let profile = query_parameter(route.query, "profile")
                    .unwrap_or_else(|| "CoreDiscography".to_owned());
                let force_refresh = query_parameter(route.query, "forceRefresh")
                    .and_then(|value| parse_bool_value(&value))
                    .unwrap_or(false);
                let settings = state.integration_settings.read().await.musicbrainz.clone();
                match musicbrainz_discography_coverage_with_settings(
                    state,
                    &settings,
                    &artist,
                    &profile,
                    force_refresh,
                )
                .await
                {
                    Ok(Some(value)) => return Ok(routing::ok_response(value.to_string())),
                    Ok(None) => return Ok(routing::not_found_response()),
                    Err(error) => {
                        return Ok(routing::service_unavailable_response(&format!(
                            "MusicBrainz coverage lookup failed: {error}"
                        )))
                    }
                }
            }
            let library = state.library.read().await;
            let json = library.discography_coverage_json(&artist);
            drop(library);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/musicbrainz/release-radar/notifications") => {
            if route.path.starts_with("/api/v0/") {
                let unread_only = query_parameter(route.query, "unreadOnly")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"));
                let mut notifications = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/radar/notification/");
                notifications.retain(|notification| {
                    !unread_only
                        || !notification
                            .get("read")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                });
                notifications.sort_by(|left, right| {
                    right["firstSeenAt"]
                        .as_str()
                        .cmp(&left["firstSeenAt"].as_str())
                        .then_with(|| left["artistId"].as_str().cmp(&right["artistId"].as_str()))
                });
                return Ok(routing::ok_response(
                    serde_json::Value::Array(notifications).to_string(),
                ));
            }
            let wishlist = state.wishlist.read().await;
            let notifications = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .map(|item| {
                    serde_json::json!({
                        "id": format!("release-radar-{}", item.id),
                        "artist": item.artist,
                        "title": item.title,
                        "searchText": item.search_text(),
                        "source": "wishlist",
                    })
                })
                .collect::<Vec<_>>();
            drop(wishlist);
            Ok(routing::ok_response(
                serde_json::Value::Array(notifications).to_string(),
            ))
        },
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
