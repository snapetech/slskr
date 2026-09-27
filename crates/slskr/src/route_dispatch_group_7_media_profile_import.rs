async fn route_dispatch_group_7_media_profile_import(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        state_arc,
        extended_mutation,
        request_is_versioned_v0,
    } = context.clone();
    match (method, normalized_path) {
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
            let Some(record) = now_playing
                .records
                .iter()
                .max_by_key(|record| record.updated_at)
            else {
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
            let default = destinations
                .records
                .iter()
                .find(|record| record.is_default)
                .cloned();
            let known_count = destinations.records.len();
            drop(destinations);
            if route.path.starts_with("/api/v0/") {
                let path = normalized_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| destination.clone());
                let exists = normalized_path.as_deref().is_some_and(Path::is_dir);
                return Ok(routing::ok_response(
                    serde_json::json!({
                        "path": path,
                        "exists": exists,
                        "writable": exists
                            && normalized_path
                                .as_deref()
                                .is_some_and(directory_is_writable),
                    })
                    .to_string(),
                ));
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
                let expires_at =
                    chrono::Utc::now() + chrono::Duration::hours(i64::from(expires_in_hours));
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
            Ok(routing::created_response(
                serde_json::json!({
                    "invite": format!("local-{count}"),
                    "created_at": updated_at,
                    "count": count,
                    "persisted": true,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/musicbrainz/release-radar/subscriptions") => {
            if route.path.starts_with("/api/v0/") {
                let subscription = match radar_subscription_from_body(body) {
                    Ok(subscription) => subscription,
                    Err(error) => return Ok(routing::bad_request_response(&error)),
                };
                let id = subscription["id"].as_str().unwrap_or_default().to_owned();
                return Ok(
                    match state
                        .controller_features
                        .upsert(
                            format!("musicbrainz/radar/subscription/{id}"),
                            subscription.clone(),
                        )
                        .await
                    {
                        Ok(()) => routing::ok_response(subscription.to_string()),
                        Err(error) => routing::service_unavailable_response(&error),
                    },
                );
            }
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let mut wishlist = state.wishlist.write().await;
            let previous = wishlist.clone();
            let item = match wishlist.add_item(artist, title, "MusicBrainzReleaseRadar".to_owned())
            {
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
                let request = match serde_json::from_str::<serde_json::Value>(body) {
                    Ok(serde_json::Value::Object(request)) => request,
                    _ => return Ok(routing::bad_request_response("request body is required")),
                };
                let release_id = request
                    .get("releaseId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                let recording_id = request
                    .get("recordingId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                let discogs_release_id = request
                    .get("discogsReleaseId")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned);
                if release_id.is_none() && recording_id.is_none() && discogs_release_id.is_none() {
                    return Ok(routing::bad_request_response(
                        "Provide at least one identifier (release, recording, or discogs).",
                    ));
                }
                let settings = state.integration_settings.read().await.musicbrainz.clone();

                let mut album = None;
                if let Some(release_id) = release_id.as_deref() {
                    let cached_album = state
                        .controller_features
                        .read()
                        .await
                        .get(&format!("musicbrainz/album-target/{release_id}"))
                        .cloned();
                    album = match musicbrainz_album_target_with_settings(&settings, release_id)
                        .await
                    {
                        Ok(album) => album,
                        Err(error) => {
                            if cached_album.is_some() {
                                ::tracing::warn!(
                                    release_id,
                                    error = %error,
                                    "MusicBrainz release lookup failed; serving cached album target"
                                );
                                cached_album.clone()
                            } else {
                                ::tracing::warn!(
                                    release_id,
                                    error = %error,
                                    "MusicBrainz release lookup failed; treating target as unresolved"
                                );
                                None
                            }
                        }
                    };
                    if album.is_none() {
                        album = cached_album;
                    }
                } else if let Some(discogs_release_id) = discogs_release_id.as_deref() {
                    album = match musicbrainz_discogs_album_target_with_settings(
                        &settings,
                        discogs_release_id,
                    )
                    .await
                    {
                        Ok(album) => album,
                        Err(error) => {
                            return Ok(routing::service_unavailable_response(&format!(
                                "MusicBrainz lookup failed: {error}"
                            )))
                        }
                    };
                }

                let track = if album.is_none() {
                    if let Some(recording_id) = recording_id.as_deref() {
                        let cached_track = state
                            .controller_features
                            .read()
                            .await
                            .get(&format!("musicbrainz/recording-target/{recording_id}"))
                            .cloned();
                        match musicbrainz_recording_target_with_settings(&settings, recording_id).await {
                            Ok(track) => track,
                            Err(error) => {
                                if cached_track.is_some() {
                                    ::tracing::warn!(
                                        recording_id,
                                        error = %error,
                                        "MusicBrainz recording lookup failed; serving cached track target"
                                    );
                                    cached_track.clone()
                                } else {
                                    ::tracing::warn!(
                                        recording_id,
                                        error = %error,
                                        "MusicBrainz recording lookup failed; treating target as unresolved"
                                    );
                                    None
                                }
                            }
                        }
                        .or(cached_track)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if album.is_none() && track.is_none() {
                    return Ok(routing::not_found_response());
                }

                if let Some(album) = album.as_ref() {
                    let release_id = album
                        .get("musicBrainzReleaseId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    if !release_id.is_empty() {
                        if let Err(error) = state
                            .controller_features
                            .upsert(
                                format!("musicbrainz/album-target/{release_id}"),
                                album.clone(),
                            )
                            .await
                        {
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                }
                if let (Some(recording_id), Some(track)) = (recording_id.as_deref(), track.as_ref())
                {
                    if let Err(error) = state
                        .controller_features
                        .upsert(
                            format!("musicbrainz/recording-target/{recording_id}"),
                            track.clone(),
                        )
                        .await
                    {
                        return Ok(routing::service_unavailable_response(&error));
                    }
                }

                return Ok(routing::ok_response(
                    serde_json::json!({"album": album, "track": track}).to_string(),
                ));
            }
            let target = extract_json_string_field(body, "target")
                .or_else(|| extract_json_string_field(body, "mbid"))
                .or_else(|| extract_json_string_field(body, "artist"))
                .unwrap_or_default();
            let title = extract_json_string_field(body, "title")
                .or_else(|| extract_json_string_field(body, "release"))
                .unwrap_or_default();
            if target.trim().is_empty() && title.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "target/artist or title is required",
                ));
            }
            let _library_persistence = state.library_persistence_lock.lock().await;
            let mut library = state.library.write().await;
            let previous = library.clone();
            let Some(record) =
                library.create(target.clone(), title, "MusicBrainzTarget".to_owned())
            else {
                return Ok(routing::service_unavailable_response(
                    "library item capacity is full",
                ));
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
            let requested_item_id =
                wishlist_search_item_id(path).expect("guarded wishlist search path");
            let native = route.path.starts_with("/api/v0/");
            let wishlist = state.wishlist.read().await;
            let Some(item_id) = wishlist.resolve_item_id(requested_item_id, native) else {
                return Ok(routing::not_found_response());
            };
            let Some(item) = wishlist.get_item(&item_id) else {
                drop(wishlist);
                return Ok(routing::not_found_response());
            };
            let query = item.search_text();
            drop(wishlist);
            if query.trim().is_empty() {
                return Ok(routing::bad_request_response(
                    "wishlist item has no search text",
                ));
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
                Some(item_id.clone()),
                DEFAULT_SEARCH_TTL_SECONDS,
            ) {
                Ok(outcome) => outcome,
                Err(error) => return Ok(search_create_error_response(error)),
            };
            let record = outcome.record;
            let evicted = outcome.evicted;
            let expired = outcome.expired;
            let response = serde_json::json!({
                "item_id": if native {
                    native_wishlist_item_id(&item_id)
                } else {
                    item_id.clone()
                },
                "search_started": true,
                "status": "searching",
                "search_id": record.id,
                "token": record.token,
                "query": record.query,
                "target": record.target,
            })
            .to_string();
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
                return Ok(routing::service_unavailable_response(
                    "wishlist item capacity is full",
                ));
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
            Ok(routing::created_response(
                serde_json::json!({
                    "imported": imported.len(),
                    "items": imported,
                })
                .to_string(),
            ))
        }

        ("POST", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/backfill")
                && share_grant_helper_id(path, "backfill").is_some() =>
        {
            let grant_id =
                share_grant_helper_id(path, "backfill").expect("guarded share-grant backfill path");
            let versioned = route.path.starts_with("/api/v0/");
            if versioned {
                let delegated_authorized = if let Some(token) =
                    request_share_token(authorization, headers)
                {
                    let mut tokens = state.share_access_tokens.write().await;
                    let valid = tokens
                        .validate(&token)
                        .is_some_and(|record| record.grant_id == grant_id);
                    drop(tokens);
                    valid
                } else {
                    !state.config.auth_required
                        || is_authorized(&state.config, authorization, headers.cookie.as_deref())
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
            Ok(routing::accepted_response(
                serde_json::json!({
                    "grant_id": grant_id,
                    "backfilled": backfilled,
                    "persisted": persisted,
                    "status": if persisted { "local" } else { "compatibility_acknowledgement" },
                })
                .to_string(),
            ))
        }

        ("POST", path)
            if path.starts_with("/api/share-grants/")
                && path.ends_with("/token")
                && share_grant_helper_id(path, "token").is_some() =>
        {
            let grant_id =
                share_grant_helper_id(path, "token").expect("guarded share-grant token path");
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
                        return Ok(routing::bad_request_response("The request body is invalid"));
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
                if share_grant_collection_forbids(state, collection_id, caller_id.as_deref()).await
                {
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

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
