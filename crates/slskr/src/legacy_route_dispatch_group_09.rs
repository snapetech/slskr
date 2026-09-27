use super::*;

#[cfg(feature = "legacy-route-dispatch")]
pub(super) async fn legacy_route_dispatch_group_09(
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

        ("POST", "/api/integrations/lidarr/manualimport") => {
            let directory = extract_json_string_field(body, "directory").unwrap_or_default();
            if route.path.starts_with("/api/v0/") {
                if directory.trim().is_empty() {
                    return Ok(routing::bad_request_response("Directory is required"));
                }
                let lidarr = state.integration_settings.read().await.lidarr.clone();
                let mut result = match import_lidarr_completed_directory(state, &lidarr, &directory).await
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
                let _library_persistence = state.library_persistence_lock.lock().await;
                let _runtime_persistence = state.runtime_persistence_lock.lock().await;
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
                let runtime_record = state
                    .db
                    .as_ref()
                    .map(|_| runtime.persistence_record(&relay));
                let mutated_library = library.clone();
                let mutated_runtime = runtime.clone();
                drop(relay);
                drop(runtime);
                drop(library);
                if let (Some(db), Some(runtime_record)) =
                    (state.db.as_ref(), runtime_record.as_ref())
                {
                    let persistence_error = db
                        .upsert_library_item_and_runtime_compat_state(
                            &persisted_library_item(&record),
                            runtime_record,
                        )
                        .await
                        .err()
                        .map(|error| error.to_string());
                    if let Some(error_message) = persistence_error {
                        route_dispatch::rollback_library_runtime_if_unchanged(
                            state,
                            previous_library,
                            mutated_library,
                            previous_runtime,
                            mutated_runtime,
                            record,
                        )
                        .await;
                        return Ok(routing::service_unavailable_response(&format!(
                            "library persistence failed: Lidarr manual import transaction failed: {error_message}"
                        )));
                    }
                }
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
        }

        ("GET", "/api/musicbrainz/release-radar/subscriptions") => {
            if route.path.starts_with("/api/v0/") {
                let mut subscriptions = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/radar/subscription/");
                subscriptions.sort_by(|left, right| {
                    left["artistName"]
                        .as_str()
                        .map(str::to_ascii_lowercase)
                        .cmp(&right["artistName"].as_str().map(str::to_ascii_lowercase))
                        .then_with(|| {
                            left["artistId"]
                                .as_str()
                                .map(str::to_ascii_lowercase)
                                .cmp(&right["artistId"].as_str().map(str::to_ascii_lowercase))
                        })
                });
                return Ok(routing::ok_response(
                    serde_json::Value::Array(subscriptions).to_string(),
                ));
            }
            let wishlist = state.wishlist.read().await;
            let subscriptions = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .map(|item| {
                    serde_json::json!({
                        "id": format!("wishlist-{}", item.id),
                        "artist": item.artist,
                        "title": item.title,
                        "source": "wishlist",
                    })
                })
                .collect::<Vec<_>>();
            drop(wishlist);
            Ok(routing::ok_response(
                serde_json::Value::Array(subscriptions).to_string(),
            ))
        }

        ("GET", "/api/listening-party") => {
            // Matches the oracle's real ListeningPartyController.List
            // response shape (ListeningPartyAnnouncement[]), but built
            // from slskR's own locally-stored listening-party events
            // rather than a real DHT-backed cross-peer directory --
            // slskR is single-peer-per-instance and has no such mesh
            // discovery wired in for this feature yet. This is an
            // honest, local-only simplification (every entry reflects a
            // real, currently-active local listen-along event), not the
            // previous behavior of listing unrelated joined chat rooms.
            let now_ms = unix_timestamp_millis();
            let events = state
                .controller_features
                .read()
                .await
                .values_with_prefix("listening-party/");
            let mut announcements = Vec::new();
            for event in events {
                if !event
                    .get("listed")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false)
                {
                    continue;
                }
                let Some((started_at, last_seen, expires_at)) =
                    listening_party_event_window(&event, now_ms)
                else {
                    continue;
                };
                let party_id = event
                    .get("partyId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let content_id = event
                    .get("contentId")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let allow_mesh_streaming = event
                    .get("allowMeshStreaming")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let stream_path = if allow_mesh_streaming {
                    issue_listening_party_stream_ticket(state, party_id, content_id)
                        .await
                        .map(|ticket| {
                            format!(
                                "/api/v0/listening-party/radio/{}/{}?ticket={}",
                                url_encode(party_id),
                                url_encode(content_id),
                                url_encode(&ticket)
                            )
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                announcements.push(serde_json::json!({
                    "kind": "slskdn.listeningParty.announce.v1",
                    "partyId": event.get("partyId").cloned().unwrap_or_default(),
                    "podId": event.get("podId").cloned().unwrap_or_default(),
                    "channelId": event.get("channelId").cloned().unwrap_or_default(),
                    "hostPeerId": event.get("hostPeerId").cloned().unwrap_or_default(),
                    "title": event.get("title").cloned().unwrap_or_default(),
                    "artist": event.get("artist").cloned().unwrap_or_default(),
                    "album": event.get("album").cloned().unwrap_or(serde_json::Value::Null),
                    "contentId": event.get("contentId").cloned().unwrap_or_default(),
                    "description": event.get("description").cloned().unwrap_or_default(),
                    "tags": event.get("tags").cloned().unwrap_or_else(|| serde_json::json!([])),
                    "allowMeshStreaming": allow_mesh_streaming,
                    "streamPath": stream_path,
                    "startedAtUnixMs": started_at,
                    "expiresAtUnixMs": expires_at,
                    "lastSeenUnixMs": last_seen,
                }));
            }
            announcements.sort_by(|left, right| {
                right["lastSeenUnixMs"]
                    .as_u64()
                    .cmp(&left["lastSeenUnixMs"].as_u64())
            });
            Ok(routing::ok_response(
                serde_json::Value::Array(announcements).to_string(),
            ))
        }

         ("POST", "/api/nowplaying") => {
            let username = extract_json_string_field(body, "username").unwrap_or_default();
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let _now_playing_persistence = state.now_playing_persistence_lock.lock().await;
            let mut now_playing = state.now_playing.write().await;
            let previous = now_playing.clone();
            let record = now_playing.upsert(username, artist, title);
            let mutated = now_playing.clone();
            let json = record.json();
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &record).await {
                let mut now_playing = state.now_playing.write().await;
                if *now_playing == mutated {
                    *now_playing = previous;
                }
                drop(now_playing);
                return Ok(routing::service_unavailable_response(&error));
            }
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/nowplaying") if route.path.starts_with("/api/v0/") => {
            let now_playing = state.now_playing.read().await;
            let Some(record) = now_playing.records.iter().max_by_key(|record| record.updated_at) else {
                return Ok(routing::no_content_response());
            };
            let started_at = chrono::DateTime::<chrono::Utc>::from_timestamp(
                i64::try_from(record.updated_at).unwrap_or(i64::MAX),
                0,
            )
            .map(|timestamp| timestamp.to_rfc3339());
            Ok(routing::ok_response(
                serde_json::json!({
                    "artist": record.artist,
                    "title": record.title,
                    "album": null,
                    "startedAt": started_at,
                })
                .to_string(),
            ))
        }

        ("GET", "/api/nowplaying") => {
            let now_playing = state.now_playing.read().await;
            let json = now_playing.json();
            drop(now_playing);
            Ok(routing::ok_response(json))
        }

         ("POST", "/api/relay") => {
             let relay_enabled = extract_json_bool_field(body, "enabled").unwrap_or(false);
             let json = match mutate_runtime_compat_state(state, |_, relay| {
                 relay.set_enabled(relay_enabled).to_string()
             })
             .await
             {
                 Ok(json) => json,
                 Err(error) => return Ok(routing::service_unavailable_response(&error)),
             };
             Ok(routing::ok_response(json))
         }

         // ADDITIONAL MISSING POST ENDPOINTS (Phase 6)
         ("POST", "/api/destinations/validate") => {
             let destination = if route.path.starts_with("/api/v0/") {
                 let path = serde_json::from_str::<serde_json::Value>(body)
                     .ok()
                     .and_then(|value| {
                         value
                             .as_object()
                             .and_then(|object| json_object_field_ci(object, "path"))
                             .and_then(serde_json::Value::as_str)
                             .map(str::trim)
                             .filter(|path| !path.is_empty())
                             .map(ToOwned::to_owned)
                     });
                 let Some(path) = path else {
                     return Ok(routing::bad_request_response("Path is required"));
                 };
                 path
             } else {
                 extract_json_string_field(body, "destination")
                     .or_else(|| extract_json_string_field(body, "path"))
                     .or_else(|| extract_json_string_field(body, "url"))
                     .unwrap_or_default()
             };
             let destinations = state.destinations.read().await;
             let normalized_path = if destinations.records.is_empty() {
                 DestinationStore::from_config(
                     &state.config.downloads_dir,
                     &state.config.core_workflow.destinations,
                 )
                 .normalize_explicit_path(&destination)
             } else {
                 destinations.normalize_explicit_path(&destination)
             };
             let matched = destinations
                 .records
                 .iter()
                 .find(|record| {
                     record.path == destination
                         || record.name.eq_ignore_ascii_case(destination.trim())
                         || record.id.eq_ignore_ascii_case(destination.trim())
                 })
                 .cloned();
             let default = destinations.records.iter().find(|record| record.is_default).cloned();
             let known_count = destinations.records.len();
             drop(destinations);
             if route.path.starts_with("/api/v0/") {
                 let path = normalized_path
                     .as_ref()
                     .map(|path| path.display().to_string())
                     .unwrap_or_else(|| destination.clone());
                 let exists = normalized_path
                     .as_deref()
                     .is_some_and(Path::is_dir);
                 return Ok(routing::ok_response(serde_json::json!({
                     "path": path,
                     "exists": exists,
                     "writable": exists
                         && normalized_path
                             .as_deref()
                             .is_some_and(directory_is_writable),
                 }).to_string()));
             }
             Ok(routing::ok_response(serde_json::json!({
                 "destination": destination,
                 "valid": !destination.trim().is_empty(),
                 "known": matched.is_some(),
                 "knownCount": known_count,
                 "matched": matched.map(|record| serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({ "id": record.id }))),
                 "default": default.map(|record| serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({ "id": record.id }))),
             }).to_string()))
         }

        ("POST", "/api/profile/invite") => {
            if route.path.starts_with("/api/v0/") {
                match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Object(_)) => {}
                    Ok(serde_json::Value::Null) | Err(_) => {
                        return Ok(routing::bad_request_response("Request is required."));
                    }
                    Ok(_) => return Ok(routing::bad_request_response("Request is required.")),
                }
                let expires_in_hours = extract_json_i32_field(body, "expiresInHours")
                    .filter(|value| *value > 0)
                    .unwrap_or(24);
                let descriptor = match local_capability_descriptor(state).await {
                    Ok(descriptor) => descriptor,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let session = state.session.read().await;
                let display_name = session
                    .username
                    .clone()
                    .or_else(|| state.config.username.clone())
                    .unwrap_or_else(|| "local".to_owned());
                drop(session);
                let expires_at = chrono::Utc::now()
                    + chrono::Duration::hours(i64::from(expires_in_hours));
                let profile_peer_id = local_profile_peer_id(state);
                let invite = serde_json::json!({
                    "InviteVersion": 1,
                    "Profile": {
                        "PeerId": profile_peer_id.clone(),
                        "PublicKey": STANDARD.encode(descriptor.public_key),
                        "DisplayName": display_name,
                        "Avatar": null,
                        "Capabilities": 0,
                        "Endpoints": descriptor.endpoints,
                        "CreatedAt": unix_seconds_rfc3339(descriptor.issued_at_unix),
                        "ExpiresAt": unix_seconds_rfc3339(descriptor.expires_at_unix),
                        "Signature": descriptor.signature.map(|signature| STANDARD.encode(signature)),
                    },
                    "Nonce": uuid::Uuid::new_v4().simple().to_string(),
                    "ExpiresAt": expires_at.to_rfc3339(),
                    "InviteSignature": null,
                });
                let encoded = STANDARD_NO_PAD
                    .encode(invite.to_string())
                        .replace('+', "-")
                        .replace('/', "_");
                let friend_code = profile_friend_code(&profile_peer_id);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "inviteLink": format!("slskdn://invite/{encoded}"),
                        "friendCode": friend_code,
                    })
                    .to_string(),
                ));
            }
            let invite_state = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_profile_invite()
            })
            .await
            {
                Ok(invite_state) => invite_state,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            let count = invite_state
                .get("count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let updated_at = invite_state
                .get("updated_at")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_else(unix_timestamp);
            Ok(routing::created_response(serde_json::json!({
                "invite": format!("local-{count}"),
                "created_at": updated_at,
                "count": count,
                "persisted": true,
            }).to_string()))
        }

         ("POST", "/api/musicbrainz/release-radar/subscriptions") => {
             if route.path.starts_with("/api/v0/") {
                 let subscription = match radar_subscription_from_body(body) {
                     Ok(subscription) => subscription,
                     Err(error) => return Ok(routing::bad_request_response(&error)),
                 };
                 let id = subscription["id"].as_str().unwrap_or_default().to_owned();
                 return Ok(match state.controller_features.upsert(
                     format!("musicbrainz/radar/subscription/{id}"),
                     subscription.clone(),
                 ).await {
                     Ok(()) => routing::ok_response(subscription.to_string()),
                     Err(error) => routing::service_unavailable_response(&error),
                 });
             }
             let artist = extract_json_string_field(body, "artist").unwrap_or_default();
             let title = extract_json_string_field(body, "title").unwrap_or_default();
             let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
             let mut wishlist = state.wishlist.write().await;
             let previous = wishlist.clone();
             let item = match wishlist.add_item(
                 artist,
                 title,
                 "MusicBrainzReleaseRadar".to_owned(),
             ) {
                 Ok(item) => item,
                 Err(()) => {
                     return Ok(routing::service_unavailable_response(
                         "wishlist item capacity is full",
                     ));
                 }
             };
             let json = item.json();
             let count = wishlist
                 .records
                 .iter()
                 .flat_map(|record| record.items.iter())
                 .count();
             let mutated = wishlist.clone();
             drop(wishlist);
             if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                 rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             Ok(routing::created_response(serde_json::json!({
                 "subscriptions": [serde_json::from_str::<serde_json::Value>(&json).unwrap_or_else(|_| serde_json::json!({}))],
                 "created": true,
                 "persisted": true,
                 "status": "local",
                 "count": count,
             }).to_string()))
         }

         ("POST", "/api/musicbrainz/targets") => {
             if route.path.starts_with("/api/v0/") {
                 match serde_json::from_str::<serde_json::Value>(body) {
                     Ok(serde_json::Value::Object(_)) => {}
                     _ => return Ok(routing::bad_request_response("request body is required")),
                 }
                 if extract_json_string_field(body, "target").is_none()
                     && extract_json_string_field(body, "mbid").is_none()
                     && extract_json_string_field(body, "artist").is_none()
                     && extract_json_string_field(body, "title").is_none()
                     && extract_json_string_field(body, "release").is_none()
                 {
                     return Ok(routing::not_found_response());
                 }
             }
             let target = extract_json_string_field(body, "target")
                 .or_else(|| extract_json_string_field(body, "mbid"))
                 .or_else(|| extract_json_string_field(body, "artist"))
                 .unwrap_or_default();
             let title = extract_json_string_field(body, "title")
                 .or_else(|| extract_json_string_field(body, "release"))
                 .unwrap_or_default();
             if target.trim().is_empty() && title.trim().is_empty() {
                 return Ok(routing::bad_request_response("target/artist or title is required"));
             }
             let _library_persistence = state.library_persistence_lock.lock().await;
             let mut library = state.library.write().await;
             let previous = library.clone();
             let Some(record) = library.create(target.clone(), title, "MusicBrainzTarget".to_owned()) else {
                 return Ok(routing::service_unavailable_response("library item capacity is full"));
             };
             let target_projection = library.target_json(&target);
             let mutated = library.clone();
             drop(library);
             if let Err(error) = persist_library_item_checked(state, &record).await {
                 rollback_library_if_unchanged(state, previous, &mutated).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             Ok(routing::created_response(serde_json::json!({
                 "target": target,
                 "created": true,
                 "item": serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({})),
                 "projection": serde_json::from_str::<serde_json::Value>(&target_projection).unwrap_or_else(|_| serde_json::json!({})),
             }).to_string()))
         }

         ("POST", path)
             if path.starts_with("/api/wishlist/")
                 && path.ends_with("/search")
                 && wishlist_search_item_id(path).is_some() =>
         {
             let item_id = wishlist_search_item_id(path)
                 .expect("guarded wishlist search path");
             let wishlist = state.wishlist.read().await;
             let Some(item) = wishlist.get_item(item_id) else {
                 drop(wishlist);
                 return Ok(routing::not_found_response());
             };
             let query = item.search_text();
             drop(wishlist);
             if query.trim().is_empty() {
                 return Ok(routing::bad_request_response("wishlist item has no search text"));
             }
             let session_command_permit = match state.session_commands.reserve().await {
                 Ok(permit) => permit,
                 Err(_) => {
                     return Ok(routing::service_unavailable_response(
                         "session manager is not running",
                     ));
                 }
             };
             let mut searches = state.searches.write().await;
             let previous_searches = searches.clone();
             let outcome = match searches.create_scheduled_wishlist_for_item(
                 query,
                 Some(item_id.to_owned()),
                 DEFAULT_SEARCH_TTL_SECONDS,
             ) {
                 Ok(outcome) => outcome,
                 Err(error) => return Ok(search_create_error_response(error)),
             };
             let record = outcome.record;
             let evicted = outcome.evicted;
             let expired = outcome.expired;
             let response = serde_json::json!({
                 "item_id": item_id,
                 "search_started": true,
                 "status": "searching",
                 "search_id": record.id,
                 "token": record.token,
                 "query": record.query,
                 "target": record.target,
             }).to_string();
             let mutated_searches = searches.clone();
             drop(searches);
             let mut upserts = expired.clone();
             upserts.push(record.clone());
             if let Err(error) = persist_search_transition(state, &upserts, &evicted).await {
                 rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                 return Ok(wishlist_storage_error_response(
                     route.path.starts_with("/api/v0/"),
                     &error,
                 ));
             }
             for expired_record in &expired {
                 publish_search_hub_event(state, "update", expired_record);
             }
             session_command_permit.send(SessionCommand::Search {
                 token: record.token,
                 query: record.query.clone(),
                 target: SearchDispatchTarget::Wishlist,
             });
             record_event(state, "search.started", record.token.to_string(), None).await;
             Ok(if route.path.starts_with("/api/v0/") {
                 routing::ok_response(response)
             } else {
                 routing::accepted_response(response)
             })
         }

         ("POST", "/api/wishlist/import/csv") => {
             if route.path.starts_with("/api/v0/") {
                 return Ok(versioned_wishlist_csv_import_response(body, state).await);
             }
             let raw = extract_json_string_field(body, "csv")
                 .or_else(|| extract_json_string_field(body, "text"))
                 .or_else(|| extract_json_string_field(body, "content"))
                 .unwrap_or_else(|| body.trim().trim_matches('"').to_owned());
             let parsed_items = match parse_simple_wishlist_import_rows(&raw) {
                 Ok(parsed_items) => parsed_items,
                 Err(error) => return Ok(routing::bad_request_response(error)),
             };
             let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
             let mut wishlist = state.wishlist.write().await;
             let previous = wishlist.clone();
             if !wishlist.can_add_items(parsed_items.len()) {
                 return Ok(routing::service_unavailable_response("wishlist item capacity is full"));
             }
             let mut imported = Vec::new();
             let mut persisted_items = Vec::new();
             for (artist, title, kind) in parsed_items {
                 let item = wishlist
                     .add_item(artist, title, kind)
                     .map_err(|_| "wishlist capacity changed unexpectedly".to_owned())?;
                 let value = serde_json::from_str::<serde_json::Value>(&item.json())
                     .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                 persisted_items.push(item);
                 imported.push(value);
             }
             let mutated = wishlist.clone();
             drop(wishlist);
             if let Err(error) = persist_wishlist_items_checked(state, &persisted_items).await {
                 rollback_wishlist_if_unchanged(state, previous, &mutated).await;
                 return Ok(routing::service_unavailable_response(&error));
             }
             Ok(routing::created_response(serde_json::json!({
                 "imported": imported.len(),
                 "items": imported,
             }).to_string()))
         }

         ("POST", path)
             if path.starts_with("/api/share-grants/")
                 && path.ends_with("/backfill")
                 && share_grant_helper_id(path, "backfill").is_some() =>
         {
             let grant_id = share_grant_helper_id(path, "backfill")
                 .expect("guarded share-grant backfill path");
             let versioned = route.path.starts_with("/api/v0/");
             if versioned {
                let delegated_authorized = if let Some(token) = request_share_token(authorization, headers) {
                     let mut tokens = state.share_access_tokens.write().await;
                     let valid = tokens
                         .validate(&token)
                         .is_some_and(|record| record.grant_id == grant_id);
                     drop(tokens);
                     valid
                 } else {
                     !state.config.auth_required
                         || is_authorized(
                             &state.config,
                             authorization,
                             headers.cookie.as_deref(),
                         )
                 };
                 if !delegated_authorized {
                     return Ok(routing::unauthorized_response());
                 }
             }
             if versioned && state.share_grants.read().await.get(grant_id).is_none() {
                 return Ok(routing::not_found_response());
             }
             let share_grants = state.share_grants.read().await;
             let collections = state.collections.read().await;
             let grant = share_grants.get(grant_id);
             if versioned {
                 let Some(grant_record) = grant.as_ref() else {
                     drop(collections);
                     drop(share_grants);
                     return Ok(routing::not_found_response());
                 };
                 if !share_grant_allows_download(&grant_record.permissions) {
                     drop(collections);
                     drop(share_grants);
                     return Ok(routing::forbidden_response(
                         "Download not allowed for this share",
                     ));
                 }
                 let Some(collection) = collections.get(&grant_record.collection_id) else {
                     drop(collections);
                     drop(share_grants);
                     return Ok(routing::not_found_response());
                 };
                 if collection.items.is_empty() {
                     drop(collections);
                     drop(share_grants);
                     return Ok(routing::ok_response(
                         serde_json::json!({
                             "enqueued": 0,
                             "failed": 0,
                             "total": 0,
                             "message": "No items to backfill",
                         })
                         .to_string(),
                     ));
                 }
             }
             let backfilled = grant
                 .as_ref()
                 .and_then(|grant| collections.get(&grant.collection_id))
                 .map(|collection| collection.items.len())
                 .unwrap_or(0);
             let persisted = grant.is_some();
             drop(collections);
             drop(share_grants);
             Ok(routing::accepted_response(serde_json::json!({
                 "grant_id": grant_id,
                 "backfilled": backfilled,
                 "persisted": persisted,
                 "status": if persisted { "local" } else { "compatibility_acknowledgement" },
             }).to_string()))
         }

         ("POST", path)
             if path.starts_with("/api/share-grants/")
                 && path.ends_with("/token")
                 && share_grant_helper_id(path, "token").is_some() =>
         {
             let grant_id = share_grant_helper_id(path, "token")
                 .expect("guarded share-grant token path");
             let _collection_grant_persistence =
                 state.collection_grant_persistence_lock.lock().await;
             if route.path.starts_with("/api/v0/")
                 && state.share_grants.read().await.get(grant_id).is_none()
             {
                 return Ok(routing::not_found_response());
             }
             if route.path.starts_with("/api/v0/") && !body.trim().is_empty() {
                 match serde_json::from_str::<serde_json::Value>(body) {
                     Ok(value) if value.is_object() || value.is_null() => {}
                     _ => {
                         return Ok(routing::bad_request_response(
                             "The request body is invalid",
                         ));
                     }
                 }
             }
             let share_grants = state.share_grants.read().await;
             let grant = share_grants.get(grant_id);
             drop(share_grants);
             let caller_id = utils::authenticated_caller_id(
                 &state.config,
                 authorization,
                 headers.cookie.as_deref(),
                 headers.remote_addr,
             );
             // Matches the oracle's real CreateToken: "Caller must own
             // the collection."
             if let Some(collection_id) = grant.as_ref().map(|grant| grant.collection_id.as_str()) {
                 if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await {
                     return Ok(routing::not_found_response());
                 }
             }
             let ttl_seconds = extract_json_u64_field(body, "expiresInSeconds")
                 .or_else(|| extract_json_u64_field(body, "expires_in_seconds"))
                .unwrap_or(DEFAULT_SHARE_ACCESS_TOKEN_TTL_SECONDS)
                .clamp(1, MAX_SHARE_ACCESS_TOKEN_TTL_SECONDS);
             let issued = if grant.is_some() {
                 let mut tokens = state.share_access_tokens.write().await;
                 let issued = tokens.issue(grant_id.to_owned(), ttl_seconds);
                 drop(tokens);
                 issued
             } else {
                 None
             };
             if grant.is_some() && issued.is_none() {
                 return Ok(routing::service_unavailable_response(
                     "share access token capacity is full or secure token generation failed",
                 ));
             }
             let created = issued.is_some();
             let mut persisted = false;
             if let Some((token, expires_at)) = issued.as_ref() {
                 let digest = share_access_token_digest(token);
                 let record = ShareAccessTokenRecord {
                     grant_id: grant_id.to_owned(),
                     expires_at: *expires_at,
                 };
                 match persist_share_access_token(state, &digest, &record).await {
                     Ok(was_persisted) => persisted = was_persisted,
                     Err(error) => {
                         state
                             .share_access_tokens
                             .write()
                             .await
                             .remove_if_unchanged(&digest, &record);
                         return Ok(routing::service_unavailable_response(&error));
                     }
                 }
             }
             let (token, expires_at) = issued
                 .map(|(token, expires_at)| (Some(token), Some(expires_at)))
                 .unwrap_or((None, None));
             Ok(routing::created_response(serde_json::json!({
                 "grant_id": grant_id,
                 "token": token,
                 "expiresAt": expires_at,
                 "expiresInSeconds": if created { Some(ttl_seconds) } else { None },
                 "created": created,
                 "persisted": persisted,
                 "status": if persisted { "persistent_token" } else if created { "ephemeral_compatibility_token" } else { "compatibility_acknowledgement" },
             }).to_string()))
         }

        ("GET", "/solid/clientid.jsonld") => {
            Ok(solid_client_id_document_response(state).await)
        }

        ("GET", "/api/slskdn") => {
            let session = state.session.read().await;
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let transfers = state.transfers.read().await;
            let users = state.users.read().await;
            let rooms = state.rooms.read().await;
            let library = state.library.read().await;
            let body = serde_json::json!({
                "status": "local",
                "enabled": true,
                "connected": session.state == "connected",
                "shares": shares.entries.len(),
                "searches": searches.records.len(),
                "transfers": transfers.entries.len(),
                "users": users.records.len(),
                "rooms": rooms.records.len(),
                "libraryItems": library.records.len(),
            }).to_string();
            drop(library);
            drop(rooms);
            drop(users);
            drop(transfers);
            drop(searches);
            drop(shares);
            drop(session);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/slskdn/library/health") => {
            let limit = match query_parameter(route.query, "limit") {
                None => 100,
                Some(value) => match value.parse::<i64>() {
                    Ok(value) if (1..=250).contains(&value) => value as usize,
                    _ => return Ok(routing::bad_request_response("limit must be between 1 and 250")),
                },
            };
            let path_filter = query_parameter(route.query, "path")
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.trim().to_owned());
            let library = state.library.read().await;
            let all_issues = library.health_issues();
            let total_issues = all_issues.len();
            let issues = all_issues
                .into_iter()
                .take(limit)
                .map(|issue| {
                    serde_json::json!({
                        "type": "MissingMetadata",
                        "file": "",
                        "mb_recording_id": "",
                        "reason": issue
                            .get("message")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("Library metadata is incomplete"),
                        "severity": "Medium",
                    })
                })
                .collect::<Vec<_>>();
            let body = serde_json::json!({
                "path": path_filter.unwrap_or_else(|| "(all)".to_owned()),
                "summary": {
                    "total_issues": total_issues,
                    "issues_open": total_issues,
                    "issues_resolved": 0,
                },
                "issues": issues,
            }).to_string();
            drop(library);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/slskdn/warm-cache") => {
            let shares = state.shares.read().await;
            let searches = state.searches.read().await;
            let library = state.library.read().await;
            let warmed = shares.entries.len() + searches.records.len() + library.records.len();
            drop(library);
            drop(searches);
            drop(shares);
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_cache_warm(warmed).to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }

        ("POST", path)
            if path.starts_with("/api/streams/")
                && path.ends_with("/share-ticket")
                && share_stream_content_id(path).is_some() =>
        {
            let content_id = share_stream_content_id(path)
                .expect("guarded share stream ticket path");
            let Some(token) = request_share_token(authorization, headers) else {
                return Ok(routing::unauthorized_response());
            };
            let mut tokens = state.share_access_tokens.write().await;
            let token_record = tokens.validate(&token);
            drop(tokens);
            let Some(token_record) = token_record else {
                return Ok(routing::unauthorized_response());
            };
            let grants = state.share_grants.read().await;
            let Some(grant) = grants.get(&token_record.grant_id) else {
                drop(grants);
                return Ok(routing::unauthorized_response());
            };
            drop(grants);
            if !share_grant_allows_stream(&grant.permissions) {
                return Ok(routing::forbidden_response(
                    "Streaming not allowed for this share",
                ));
            }
            let collections = state.collections.read().await;
            let Some(collection) = collections.get(&grant.collection_id) else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            let Some(item) = collection
                .items
                .iter()
                .find(|item| item.content_id == content_id)
                .cloned()
            else {
                drop(collections);
                return Ok(routing::not_found_response());
            };
            drop(collections);
            let filename = if item.title.trim().is_empty() {
                item.content_id.clone()
            } else {
                item.title.clone()
            };
            let mut tickets = state.stream_tickets.write().await;
            let Some((ticket, _)) = tickets.issue(
                "share",
                &format!("share:{}", grant.id),
                item.content_id,
                filename.clone(),
                Some(grant.username),
                0,
                preview_stream_content_type(&filename).to_owned(),
                SHARE_STREAM_TICKET_TTL_SECONDS,
            ) else {
                return Ok(routing::service_unavailable_response(
                    "share stream ticket capacity is full",
                ));
            };
            drop(tickets);
            Ok(routing::ok_response(
                serde_json::json!({
                    "ticket": ticket,
                    "expiresInSeconds": SHARE_STREAM_TICKET_TTL_SECONDS,
                })
                .to_string(),
            ))
        }

        ("GET" | "HEAD", path) if path.starts_with("/api/streams/") && path.len() > 13 => {
            let stream_id = decoded_path_segment(&path[13..]);
            if query_parameter(route.query, "token").is_some() {
                return Ok(routing::bad_request_response(
                    "share tokens must be exchanged for stream tickets",
                ));
            }
            let api_authorized = !state.config.auth_required
                || is_authorized(
                    &state.config,
                    authorization,
                    headers.cookie.as_deref(),
                );
            let ticket = query_parameter(route.query, "ticket");
            let ticket_record = if let Some(ticket) = ticket.as_deref() {
                let mut tickets = state.stream_tickets.write().await;
                let record = tickets.get(ticket);
                drop(tickets);
                record.filter(|record| record.family == "share" && record.content_id == stream_id)
            } else {
                None
            };
            if ticket.is_some() && ticket_record.is_none() {
                return Ok(routing::unauthorized_response());
            }
            let share = find_shared_entry_for_content(state, None, Some(&stream_id)).await;
            let transfers = state.transfers.read().await;
            let transfer = stream_id
                .strip_prefix("transfer-")
                .and_then(|id| id.parse::<u64>().ok())
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id));
            if !api_authorized && ticket_record.is_none() {
                drop(transfers);
                return Ok(routing::unauthorized_response());
            }
            let body = serde_json::json!({
                "id": stream_id,
                "status": if transfer.is_some() || share.is_some() || ticket_record.is_some() { "available" } else { "not_found" },
                "ticket": ticket.as_ref().map(|_| "accepted"),
                "transfer": transfer.map(|entry| serde_json::json!({
                    "id": entry.id,
                    "filename": entry.filename,
                    "bytesTransferred": entry.bytes_transferred,
                    "size": entry.size,
                    "state": entry.status,
                })),
                "share": share.map(|entry| serde_json::json!({
                    "filename": entry.filename,
                    "size": entry.size,
                    "extension": entry.extension,
                })),
            }).to_string();
            drop(transfers);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/peer-streams/tickets") | ("POST", "/api/mesh-streams/tickets") => {
            let family = if normalized_path.starts_with("/api/mesh-streams") {
                "mesh"
            } else {
                "peer"
            };
            match create_preview_stream_ticket(state, family, body).await {
                Ok(ticket) => Ok(routing::ok_response(ticket)),
                Err(error) if error == "preview stream ticket capacity is full" => {
                    Ok(HttpResponse {
                        status: "429 Too Many Requests",
                        content_type: "text/plain; charset=utf-8",
                        body: if family == "mesh" {
                            "Mesh stream limit reached.".to_owned()
                        } else {
                            "Peer stream limit reached.".to_owned()
                        },
                    })
                }
                Err(error) => Ok(routing::bad_request_response(&error)),
            }
        }

        ("GET", path)
            if path.starts_with("/api/peer-streams/") || path.starts_with("/api/mesh-streams/") =>
        {
            let (family, raw_ticket) = if let Some(ticket) = path.strip_prefix("/api/mesh-streams/") {
                ("mesh", ticket)
            } else if let Some(ticket) = path.strip_prefix("/api/peer-streams/") {
                ("peer", ticket)
            } else {
                unreachable!()
            };
            let ticket = decoded_path_segment(raw_ticket);
            match open_preview_stream_ticket(state, family, &ticket).await {
                Some(body) => Ok(routing::ok_response(body)),
                None => Ok(routing::not_found_response()),
            }
        }

        ("POST", "/api/listening-party/radio/party/content") => {
            let room = extract_json_string_field(body, "room").unwrap_or_else(|| "radio".to_owned());
            let title = extract_json_string_field(body, "title").unwrap_or_else(|| "party content".to_owned());
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let mut rooms = state.rooms.write().await;
            let Some(room_record) = rooms.join(room.clone()) else {
                return Ok(routing::service_unavailable_response(
                    "room capacity is full",
                ));
            };
            let room_record = rooms
                .add_message(
                    &room,
                    "local".to_owned(),
                    if artist.trim().is_empty() {
                        title.clone()
                    } else {
                        format!("{artist} - {title}")
                    },
                )
                .unwrap_or(room_record);
            let active_count = rooms.records.iter().filter(|room| room.joined).count();
            drop(rooms);
            let _now_playing_persistence = state.now_playing_persistence_lock.lock().await;
            let mut now_playing = state.now_playing.write().await;
            let playing = now_playing.upsert(room.clone(), artist, title);
            drop(now_playing);
            if let Err(error) = persist_now_playing_checked(state, &playing).await {
                update_session(state, |snapshot| snapshot.last_error = Some(error)).await;
            }
            Ok(routing::accepted_response(serde_json::json!({
                "status": "queued",
                "room": room,
                "activePartyCount": active_count,
                "party": serde_json::from_str::<serde_json::Value>(&room_record.json()).unwrap_or_else(|_| serde_json::json!({})),
                "nowPlaying": serde_json::from_str::<serde_json::Value>(&playing.json()).unwrap_or_else(|_| serde_json::json!({})),
            }).to_string()))
        }

        ("GET", "/api/mesh/health")
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile
                    == ControllerProfile::Native =>
        {
            let routing_nodes = if let Some(dht) = state.dht.as_ref() {
                serde_json::from_str::<serde_json::Value>(&dht.status_json().await)
                    .ok()
                    .and_then(|value| value["dhtNodeCount"].as_u64())
                    .unwrap_or(0)
            } else {
                0
            };
            let discovery = state.content_discovery.read().await;
            let stored_keys = discovery.hash_entries().len();
            let content_peer_hints = discovery
                .shadow_records()
                .iter()
                .map(|record| record.peer_ids.len())
                .sum::<usize>();
            drop(discovery);
            Ok(routing::ok_response(
                serde_json::json!({
                    "routingNodes": routing_nodes,
                    "storedKeys": stored_keys,
                    "contentPeerHints": content_peer_hints,
                    "generatedAt": chrono::Utc::now().to_rfc3339(),
                })
                .to_string(),
            ))
        }

        ("GET", "/api/mesh/health") => {
            let users = state.users.read().await;
            let mesh = state.mesh.read().await;
            let candidate_count = mesh.candidate_usernames(&users).len();
            let capability_count = mesh.capability_records.len();
            drop(mesh);
            drop(users);
            Ok(routing::ok_response(serde_json::json!({
                "status": if candidate_count > 0 || capability_count > 0 { "ready" } else { "empty" },
                "healthy": true,
                "candidates": candidate_count,
                "capabilities": capability_count,
                "interestTag": MESH_RENDEZVOUS_INTEREST_TAG,
            }).to_string()))
        }

        ("POST", "/api/multisource/swarm")
        | ("POST", "/api/multisource/swarm/async")
        | ("POST", "/api/multisource/download") if normalized_path != "/api/multisource/download"
            || serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .is_some_and(|value| {
                    value
                        .get("sources")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|sources| !sources.is_empty())
                }) =>
        {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile
                    == ControllerProfile::Native
            {
                if normalized_path == "/api/multisource/download" {
                    return Ok(multisource_versioned_download_response(body, state).await);
                }
                return Ok(multisource_versioned_swarm_response(
                    normalized_path.as_str(),
                    body,
                    state,
                )
                .await);
            }
            let mut request = match serde_json::from_str::<multisource::SwarmRequest>(body) {
                Ok(request) => request,
                Err(_) => return Ok(routing::bad_request_response("invalid swarm request")),
            };
            if request.sources.is_empty() {
                let expected_hash = request.expected_hash.clone().unwrap_or_default();
                request.sources =
                    discover_mesh_range_sources(state, &expected_hash, request.file_size).await;
            }
            if let Err(error) = multisource::validate_request(&mut request) {
                return Ok(routing::bad_request_response(&error));
            }
            let id = uuid::Uuid::new_v4().to_string();
            let relative_path = request.output_path.clone().unwrap_or_else(|| {
                format!(
                    "multisource/{id}-{}",
                    virtual_basename(&request.filename)
                )
            });
            let downloads_dir = effective_downloads_dir(state);
            let output_path = match safe_download_path(&downloads_dir, &relative_path)
                .and_then(|path| {
                    ensure_scoped_download_path(
                        &downloads_dir,
                        path.to_string_lossy().as_ref(),
                    )
                })
            {
                Ok(path) => path,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let public_output_path = output_path
                .strip_prefix(&downloads_dir)
                .map(|path| path.to_string_lossy().replace('\\', "/"))
                .map_err(|_| "multisource output path escaped the download root".to_owned())?;
            let job = multisource::new_job(
                id.clone(),
                &request,
                public_output_path.clone(),
                unix_timestamp(),
            );
            state.multisource.write().await.insert(job.clone());
            let store = Arc::clone(&state.multisource);
            if normalized_path == "/api/multisource/swarm/async" {
                tokio::spawn(multisource::execute(
                    id.clone(),
                    request,
                    output_path,
                    public_output_path,
                    store,
                ));
                return Ok(routing::accepted_response(
                    serde_json::json!({
                        "id": id,
                        "status": "queued",
                        "job": job,
                    })
                    .to_string(),
                ));
            }
            let result = multisource::execute(
                id,
                request,
                output_path,
                public_output_path,
                store,
            )
            .await;
            Ok(routing::ok_response(
                serde_json::to_string(&result)
                    .map_err(|error| format!("multisource result serialization failed: {error}"))?,
            ))
        }

        ("POST", "/api/multisource/download") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile
                    == ControllerProfile::Native
            {
                return Ok(multisource_versioned_download_response(body, state).await);
            }
            if route.path.starts_with("/api/v0/")
                && serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("sources")
                            .and_then(serde_json::Value::as_array)
                            .map(Vec::len)
                    })
                    .is_some_and(|count| count < 2)
            {
                return Ok(HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!("At least 2 verified sources are required").to_string(),
                });
            }
            let filename = extract_json_string_field(body, "filename")
                .or_else(|| extract_json_string_field(body, "path"))
                .unwrap_or_else(|| "multisource-download".to_owned());
            let size = extract_json_u64_field(body, "size");
            let peer = extract_json_string_field(body, "username")
                .or_else(|| extract_json_string_field(body, "peer"));
            let mut transfers = state.transfers.write().await;
            let entry = transfers.create(0, peer, filename, None, size);
            let body = serde_json::json!({
                "id": format!("transfer-{}", entry.id),
                "transfer_id": entry.id,
                "status": "queued",
                "job": serde_json::from_str::<serde_json::Value>(&entry.json()).unwrap_or_else(|_| serde_json::json!({})),
            }).to_string();
            drop(transfers);
            persist_transfer_durability(state).await;
            Ok(routing::accepted_response(body))
        }

        ("GET", "/api/podcore/content/search") => {
            let params = route.query.map(query_params).unwrap_or_default();
            let query = params
                .iter()
                .find(|(key, _)| key == "query" || key == "q")
                .map(|(_, value)| value.trim().to_owned())
                .unwrap_or_default();
            if query.is_empty() {
                return Ok(routing::bad_request_response("Search query is required"));
            }
            // The oracle's real backend is a live MusicBrainz recording
            // search. Keep the result as the flat `ContentSearchResult[]`
            // contract rather than the old {query, results, count} wrapper.
            let domain = params
                .iter()
                .find(|(key, _)| key == "domain")
                .map(|(_, value)| value.trim().to_owned())
                .filter(|value| !value.is_empty());
            if domain.is_some_and(|domain| !domain.eq_ignore_ascii_case("audio")) {
                return Ok(routing::ok_response("[]".to_owned()));
            }
            let limit = params
                .iter()
                .find(|(key, _)| key == "limit")
                .and_then(|(_, value)| value.parse::<i64>().ok())
                .unwrap_or(20)
                .clamp(1, 100) as usize;
            let settings = state.integration_settings.read().await.musicbrainz.clone();
            let mut hits = musicbrainz_search_recordings(&settings, &query, limit)
                .await
                .unwrap_or_default();
            if hits.is_empty() {
                // A disconnected or empty MusicBrainz backend must not erase
                // local content search results. The compatibility controller
                // has a real local library, so use it as the bounded fallback
                // while preserving the MusicBrainz-backed result shape.
                let query = query.to_ascii_lowercase();
                let library = state.library.read().await;
                hits = library
                    .records
                    .iter()
                    .filter(|record| {
                        record.kind.eq_ignore_ascii_case("audio")
                            && (record.title.to_ascii_lowercase().contains(&query)
                                || record.artist.to_ascii_lowercase().contains(&query))
                    })
                    .take(limit)
                    .map(|record| MusicBrainzRecordingHit {
                        recording_id: record.id.clone(),
                        title: record.title.clone(),
                        artist: record.artist.clone(),
                        artist_id: None,
                    })
                    .collect();
            }
            let results = hits
                .into_iter()
                .filter(|hit| !hit.recording_id.is_empty())
                .map(|hit| {
                    serde_json::json!({
                        "contentId": format!("content:audio:track:{}", hit.recording_id),
                        "title": hit.title,
                        "subtitle": hit.artist,
                        "type": "track",
                        "domain": "audio",
                        "metadata": {
                            "musicbrainz_recording_id": hit.recording_id,
                            "artist": hit.artist,
                            "title": hit.title,
                            "musicbrainz_artist_id": hit.artist_id.unwrap_or_default(),
                        },
                    })
                })
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::Value::Array(results).to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/join") => {
            let input = match PodJoinSignatureInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let now = unix_timestamp();
            let verified = match verify_pod_join_signature(
                mode,
                &input,
                now.saturating_mul(1_000),
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if mode == PodSignatureMode::Warn && !verified {
                record_daemon_log(
                    state,
                    logging::LogLevel::Warn,
                    "podcore",
                    "accepted unsigned or legacy pod join in warn mode".to_owned(),
                )
                .await;
            }
            if mode == PodSignatureMode::Enforce {
                if let Err(error) = state.pod_join_replays.write().await.reserve(&input, now) {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }));
                }
            }
            let (pod_exists_and_is_not_member, add_result) = {
                let rooms = state.rooms.read().await;
                let pods = state.pods.read().await;
                let mut workflow = state.pod_membership_workflow.write().await;
                let pod_state = pods.get(&input.pod_id).map(|_| {
                    pods.members(&input.pod_id).is_none_or(|members| {
                        !members.iter().any(|member| {
                            member.peer_id.eq_ignore_ascii_case(&input.peer_id)
                        })
                    })
                });
                let pod_state = pod_state.or_else(|| {
                    rooms.records.iter().find(|room| room.name == input.pod_id).map(|room| {
                        !room
                            .members
                            .iter()
                            .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                    })
                });
                let add_result = if pod_state == Some(true) {
                    workflow.add_join(input.clone())
                } else {
                    Ok(())
                };
                (pod_state, add_result)
            };
            match pod_exists_and_is_not_member {
                None => {
                    if mode == PodSignatureMode::Enforce {
                        state.pod_join_replays.write().await.release(&input);
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Some(false) => {
                    if mode == PodSignatureMode::Enforce {
                        state.pod_join_replays.write().await.release(&input);
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Already a member of this pod"
                    }));
                }
                Some(true) => {
                    if let Err(error) = add_result {
                        if mode == PodSignatureMode::Enforce {
                            state.pod_join_replays.write().await.release(&input);
                        }
                        return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                            "Join request could not be processed"
                        } else {
                            error
                        }));
                    }
                }
            }
            let body = serde_json::json!({
                "success": true,
                "podId": input.pod_id,
                "peerId": input.peer_id,
                "joinRequest": input,
                "signatureMode": mode.as_str(),
                "signatureVerified": verified,
            }).to_string();
            Ok(routing::ok_response(body))
        }

        _ => Err(LEGACY_ROUTE_NOT_HANDLED.to_owned()),
    }
    .inspect(complete_legacy_request_span)
}
