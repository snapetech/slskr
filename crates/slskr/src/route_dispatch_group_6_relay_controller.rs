async fn route_dispatch_group_6_relay_controller(
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
                 "isRunning": discovery.is_running(),
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
            let hash_verification_enabled =
                extract_json_bool_field(body, "enableHashVerification").unwrap_or(true);
            let search_term = truncate_utf8_bytes(search_term, MAX_SEARCH_QUERY_BYTES);
            let generation = {
                let mut discovery = state.source_discovery.write().await;
                let Some(generation) = discovery.begin_start(
                    search_term.clone(),
                    hash_verification_enabled,
                ) else {
                    return Ok(routing::HttpResponse {
                        status: "409 Conflict",
                        content_type: "application/json",
                        body: serde_json::json!({
                            "error": "Discovery already running",
                            "currentSearchTerm": discovery.search_term.clone(),
                            "hint": "Call /api/v0/discovery/stop first",
                        })
                        .to_string(),
                    });
                };
                generation
            };
            let record = match dispatch_source_discovery_search(state, search_term.clone()).await {
                Ok(record) => record,
                Err(error) if error == "session manager is not running" => {
                    match create_rescue_search(state, search_term.clone()).await {
                        Ok(record) => record,
                        Err(error) => {
                            state.source_discovery.write().await.fail_start(generation);
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                }
                Err(error) => {
                    state.source_discovery.write().await.fail_start(generation);
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
             if !state
                 .source_discovery
                 .write()
                 .await
                 .finish_start(generation, record.token)
             {
                 return Ok(routing::HttpResponse {
                     status: "409 Conflict",
                     content_type: "application/json",
                     body: serde_json::json!({
                         "error": "Discovery start was superseded",
                         "hint": "Retry the discovery start request",
                     })
                     .to_string(),
                 });
             }
             Ok(routing::ok_response(serde_json::json!({
                 "message": "Discovery started",
                 "searchTerm": search_term,
                 "hashVerificationEnabled": hash_verification_enabled,
             }).to_string()))
         }
         ("POST", "/api/discovery/stop") => {
             let mut discovery = state.source_discovery.write().await;
             if !discovery.stop() {
                 return Ok(routing::ok_response(
                     serde_json::json!({"message": "Discovery not running"}).to_string(),
                 ));
             }
             let stopped = discovery.clone();
             drop(discovery);
             let searches = state.searches.read().await;
             let sources = source_discovery_sources(&stopped, &searches);
             drop(searches);
             let mut response_discovery = stopped.clone();
             response_discovery.last_cycle_new_files = sources.len();
             let mut discovery = state.source_discovery.write().await;
             if source_discovery_state_matches(&discovery, &stopped) {
                 discovery.last_cycle_new_files = sources.len();
             }
             drop(discovery);
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
                      "searchCycles": response_discovery.search_cycles,
                      "lastCycleNewFiles": response_discovery.last_cycle_new_files,
                      "hashVerificationEnabled": response_discovery.hash_verification_enabled,
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
             let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
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

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}

fn source_discovery_state_matches(
    current: &SourceDiscoveryState,
    expected: &SourceDiscoveryState,
) -> bool {
    current.running == expected.running
        && current.generation == expected.generation
        && current.starting_generation == expected.starting_generation
        && current.search_term == expected.search_term
        && current.hash_verification_enabled == expected.hash_verification_enabled
        && current.search_tokens == expected.search_tokens
        && current.search_cycles == expected.search_cycles
        && current.last_cycle_new_files == expected.last_cycle_new_files
        && current.last_dispatch_at == expected.last_dispatch_at
}
