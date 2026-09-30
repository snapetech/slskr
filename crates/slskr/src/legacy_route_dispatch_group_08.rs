use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_08(
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
              let job_store_result = state.controller_features.upsert(format!("job/discography/{}", record.id), job_projection).await;
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
                  let job_key = format!("job/discography/{}", record.id);
                  if let Err(cleanup_error) = state.controller_features.remove(&job_key).await
                  {
                      record_daemon_log(
                          state,
                          logging::LogLevel::Error,
                          "jobs",
                          format!("failed to remove rolled-back job projection {job_key}: {cleanup_error}"),
                      )
                      .await;
                  }
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
              let job_store_result = state.controller_features.upsert(format!("job/mb-release/{}", record.id), job_projection).await;
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
                  let job_key = format!("job/mb-release/{}", record.id);
                  if let Err(cleanup_error) = state.controller_features.remove(&job_key).await
                  {
                      record_daemon_log(
                          state,
                          logging::LogLevel::Error,
                          "jobs",
                          format!("failed to remove rolled-back job projection {job_key}: {cleanup_error}"),
                      )
                      .await;
                  }
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
                      if let Err(error) = SourceFeedImportHistoryStore::record_and_persist(
                          &state.source_feed_import_history,
                          &state.source_feed_import_history_persistence_lock,
                          &state.config.state_dir,
                          &request,
                          &raw,
                          &result,
                      )
                      .await
                      {
                          return Ok(routing::service_unavailable_response(&error));
                      }
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
                          if let Err(error) = SourceFeedImportHistoryStore::record_and_persist(
                              &state.source_feed_import_history,
                              &state.source_feed_import_history_persistence_lock,
                              &state.config.state_dir,
                              &request,
                              &raw,
                              &result,
                          )
                          .await
                          {
                              return Ok(routing::service_unavailable_response(&error));
                          }
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
                  let result = match preview_local_source_feed(
                      &raw,
                      &requested_kind,
                      include_album,
                      usize::try_from(safe_limit).unwrap_or(usize::MAX),
                  ) {
                      Ok(result) => result,
                      Err(error) => return Ok(routing::bad_request_response(error)),
                  };
                  if let Err(error) = SourceFeedImportHistoryStore::record_and_persist(
                      &state.source_feed_import_history,
                      &state.source_feed_import_history_persistence_lock,
                      &state.config.state_dir,
                      &request,
                      &raw,
                      &result,
                  )
                  .await
                  {
                      return Ok(routing::service_unavailable_response(&error));
                  }
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
              let items = match parse_simple_source_preview_items(&raw) {
                  Ok(items) => items,
                  Err(error) => return Ok(routing::bad_request_response(error)),
              };
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
              let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
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
                let mut process = tokio::process::Command::new(&command);
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
                match state.visualizer_children.launch(&mut process, &state.external_visualizer_processes) {
                    Ok(process_id) => {
                        let name = if visualizer.name.trim().is_empty() {
                            "External visualizer".to_owned()
                        } else {
                            visualizer.name.trim().to_owned()
                        };
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
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        Ok(routing::service_unavailable_response(&error.to_string()))
                    }
                    Err(error) => {
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
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, record, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let Some(record) = security.ban_with_options(
                    "username",
                    username.clone(),
                    reason,
                    duration_seconds,
                    is_permanent,
                ) else {
                    return Ok(routing::service_unavailable_response(
                        "security ban capacity is full",
                    ));
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
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    rollback_security_ban_if_unchanged(
                        state,
                        previous_bans,
                        previous_updated_at,
                        mutated_bans,
                        mutated_updated_at,
                    )
                    .await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = state.security.read().await.active_bans();
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
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, removed, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let removed = security.unban("username", &username);
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
            let persisted = if removed {
                match persist_security_unban(state, "username", &username).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        rollback_security_ban_if_unchanged(
                            state,
                            previous_bans,
                            previous_updated_at,
                            mutated_bans,
                            mutated_updated_at,
                        )
                        .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = state.security.read().await.active_bans();
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
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, record, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let Some(record) = security.ban_with_options(
                    "ip",
                    ip.clone(),
                    reason,
                    duration_seconds,
                    is_permanent,
                ) else {
                    return Ok(routing::service_unavailable_response(
                        "security ban capacity is full",
                    ));
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
            let persisted = match persist_security_ban(state, &record).await {
                Ok(persisted) => persisted,
                Err(error) => {
                    rollback_security_ban_if_unchanged(
                        state,
                        previous_bans,
                        previous_updated_at,
                        mutated_bans,
                        mutated_updated_at,
                    )
                    .await;
                    return Ok(routing::service_unavailable_response(&error));
                }
            };
            let active_bans = state.security.read().await.active_bans();
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
            let _security_ban_persistence = state.security_ban_persistence_lock.lock().await;
            let (previous_bans, previous_updated_at, removed, mutated_bans, mutated_updated_at) = {
                let mut security = state.security.write().await;
                let previous_bans = security.bans.clone();
                let previous_updated_at = security.updated_at;
                let removed = security.unban("ip", &ip);
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
            let persisted = if removed {
                match persist_security_unban(state, "ip", &ip).await {
                    Ok(persisted) => persisted,
                    Err(error) => {
                        rollback_security_ban_if_unchanged(
                            state,
                            previous_bans,
                            previous_updated_at,
                            mutated_bans,
                            mutated_updated_at,
                        )
                        .await;
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }
            } else {
                state.db.is_some()
            };
            let active_bans = state.security.read().await.active_bans();
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
            let _message_persistence = state.message_persistence_lock.lock().await;
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
            drop(_message_persistence);
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
            if let Err(error) = disconnect_spotify_connection(state).await {
                return Ok(routing::service_unavailable_response(&error));
            }
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
            let _now_playing_persistence = state.now_playing_persistence_lock.lock().await;
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
                    "succeeded"
                        | "completed"
                        | "cancelled"
                        | "failed"
                        | "rejected"
                        | "errored"
                )
            });
            let pruned = before.saturating_sub(transfers.entries.len());
            transfers.persist_state();
            let retained_ids = transfers
                .entries
                .iter()
                .map(|entry| entry.id)
                .collect::<HashSet<_>>();
            transfers
                .progress_persisted_at
                .retain(|id, _| retained_ids.contains(id));
            drop(transfers);
            persist_transfer_durability(state).await;
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
            let _oauth_persistence = state.oauth_persistence_lock.lock().await;
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
            drop(_oauth_persistence);
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

        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
