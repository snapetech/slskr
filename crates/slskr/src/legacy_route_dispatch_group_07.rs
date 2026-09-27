use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_07(
    context: &super::LegacyRouteDispatchContext<'_, '_>,
) -> Result<HttpResponse, String> {
    let super::LegacyRouteDispatchContext {
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
    let _ = (
        authorization,
        headers,
        extended_mutation,
        request_is_versioned_v0,
    );
    match (method, normalized_path.as_str()) {
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
            let runs = songid_runs_value(&library, &shares, Some(&query));
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
              // Use the same channel-then-pod order as parent mutations. The
              // checks and append share a turn so a queued pod update cannot
              // remove the channel and then have this request recreate an
              // orphaned message from an earlier check.
              let (binding, append_result) = {
                  let mut channels = state.pod_channels.write().await;
                  let pods = state.pods.read().await;
                  if pods.get(&pod_id).is_none()
                      || !pods.channel_exists(&pod_id, &channel_id)
                  {
                      return Ok(routing::not_found_response());
                  }
                  if !pods.is_member(&pod_id, &authenticated_peer_id) {
                      return Ok(routing::forbidden_response("Pod membership is required"));
                  }
                  let binding = pods.soulseek_binding(&pod_id, &channel_id);
                  let append_result = if binding
                      .as_ref()
                      .is_some_and(|binding| binding.kind == "dm")
                  {
                      None
                  } else {
                      Some(channels.append(
                          pod_id.clone(),
                          channel_id.clone(),
                          authenticated_peer_id.clone(),
                          body_text.clone(),
                          signature.clone(),
                          unix_timestamp_millis(),
                      ))
                  };
                  (binding, append_result)
              };
              if let Some(binding) = binding.as_ref().filter(|binding| binding.kind == "dm") {
                  let session_command_permit = match state.session_commands.reserve().await {
                      Ok(permit) => permit,
                      Err(_) => {
                          return Ok(routing::service_unavailable_response(
                              "session manager is not running",
                          ));
                      }
                  };
                  let _message_persistence = state.message_persistence_lock.lock().await;
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
                  drop(_message_persistence);
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
              let result = append_result.expect("non-DM pod message is appended under the lock");
              match result {
                  Ok(message) => {
                      if let Some(binding) = binding.filter(|binding| {
                          binding.kind == "room" && binding.mode == "mirror"
                      }) {
                          let room = binding.identifier;
                          if let Err(error) = try_send_session_command(
                              state,
                              SessionCommand::SayRoom {
                                  room: room.clone(),
                                  body: format!(
                                      "[Pod:{}] {}",
                                      message.sender_peer_id, message.body
                                  ),
                              },
                          ) {
                              record_pod_room_mirror_failure(state, &room, &error).await;
                          }
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
              {
                  let pods = state.pods.read().await;
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
              }
              let mut channels = state.pod_channels.write().await;
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
              let previous_pods = pods.clone();
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
              let update_result = pods.update(&pod_id, pod);
              match update_result {
                  Ok(Some(pod)) => {
                      let response_body = serde_json::to_string(&pod)
                          .map_err(|error| format!("pod serialization failed: {error}"))?;
                      match channels.delete_channels(&pod_id, &removed_channel_ids) {
                          Ok(_) => Ok(routing::ok_response(response_body)),
                          Err(error) => {
                              if let Err(rollback_error) = pods.restore_snapshot(&previous_pods) {
                                  eprintln!("pod update rollback failed: {rollback_error}");
                              }
                              eprintln!("pod channel cleanup failed: {error}");
                              Ok(routing::internal_server_error_response(
                                  "Failed to update pod",
                              ))
                          }
                      }
                  }
                  Ok(None) => Ok(routing::not_found_response()),
                  Err(error) => {
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
              if pods.get(&pod_id).is_none() {
                  return Ok(routing::not_found_response());
              }
              if peer_id
                  .as_deref()
                  .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              let mut channels = state.pod_channels.write().await;
              let mut pods = state.pods.write().await;
              if let Err(error) = pods.validate_storage() {
                  eprintln!("pod storage failed: {error}");
                  return Ok(routing::internal_server_error_response("Failed to delete pod"));
              }
              if pods.get(&pod_id).is_none() {
                  return Ok(routing::not_found_response());
              }
              if peer_id
                  .as_deref()
                  .is_none_or(|peer_id| !pods.can_moderate(&pod_id, peer_id))
              {
                  return Ok(routing::forbidden_response(
                      "Pod moderator membership is required",
                  ));
              }
              let previous_pods = pods.clone();
              match pods.delete(&pod_id) {
                  Ok(true) => match channels.delete_pod(&pod_id) {
                      Ok(_) => Ok(routing::no_content_response()),
                      Err(error) => {
                          if let Err(rollback_error) = pods.restore_snapshot(&previous_pods) {
                              eprintln!("pod delete rollback failed: {rollback_error}");
                          }
                          eprintln!("pod channel cleanup failed: {error}");
                          Ok(routing::internal_server_error_response(
                              "Failed to delete pod",
                          ))
                      }
                  },
                  Ok(false) => Ok(routing::not_found_response()),
                  Err(error) => {
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
                              let room = binding.identifier;
                              if let Err(error) = try_send_session_command(
                                  state,
                                  SessionCommand::JoinRoom(room.clone()),
                              ) {
                                  record_room_dispatch_failure(state, "join", &room, &error).await;
                              }
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

        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
