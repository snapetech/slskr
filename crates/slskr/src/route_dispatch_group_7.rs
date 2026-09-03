async fn route_dispatch_group_7(context: &RouteDispatchContext<'_, '_>) -> RouteDispatchResult {
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
            const ANNOUNCEMENT_TTL_MS: u64 = 900_000;
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
                let started_at = event
                    .get("serverTimeUnixMs")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let last_seen = now_ms;
                let expires_at = last_seen.saturating_add(ANNOUNCEMENT_TTL_MS);
                if expires_at <= now_ms {
                    continue;
                }
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
                    match state.controller_features.write().await.upsert(
                        format!("musicbrainz/radar/subscription/{id}"),
                        subscription.clone(),
                    ) {
                        Ok(()) => routing::ok_response(subscription.to_string()),
                        Err(error) => routing::service_unavailable_response(&error),
                    },
                );
            }
            let artist = extract_json_string_field(body, "artist").unwrap_or_default();
            let title = extract_json_string_field(body, "title").unwrap_or_default();
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
                        if let Err(error) = state.controller_features.write().await.upsert(
                            format!("musicbrainz/album-target/{release_id}"),
                            album.clone(),
                        ) {
                            return Ok(routing::service_unavailable_response(&error));
                        }
                    }
                }
                if let (Some(recording_id), Some(track)) = (recording_id.as_deref(), track.as_ref())
                {
                    if let Err(error) = state.controller_features.write().await.upsert(
                        format!("musicbrainz/recording-target/{recording_id}"),
                        track.clone(),
                    ) {
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
            if let Err(error) = persist_expired_searches(state, &expired).await {
                rollback_searches_if_unchanged(state, previous_searches.clone(), &mutated_searches)
                    .await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            if let Err(error) = delete_persisted_searches(state, &evicted).await {
                rollback_searches_if_unchanged(state, previous_searches.clone(), &mutated_searches)
                    .await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
            }
            if let Err(error) = persist_search_record(state, &record).await {
                rollback_searches_if_unchanged(state, previous_searches, &mutated_searches).await;
                return Ok(wishlist_storage_error_response(
                    route.path.starts_with("/api/v0/"),
                    &error,
                ));
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
            let parsed_items = raw
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .enumerate()
                .filter_map(|(index, line)| {
                    let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
                    if index == 0
                        && parts
                            .first()
                            .is_some_and(|value| value.eq_ignore_ascii_case("artist"))
                    {
                        return None;
                    }
                    let (artist, title) = if parts.len() >= 2 {
                        (parts[0].to_owned(), parts[1].to_owned())
                    } else if let Some((artist, title)) = line.split_once(" - ") {
                        (artist.trim().to_owned(), title.trim().to_owned())
                    } else {
                        (String::new(), line.to_owned())
                    };
                    Some((artist, title, "Audio".to_owned()))
                })
                .collect::<Vec<_>>();
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

        ("GET", "/solid/clientid.jsonld") => Ok(solid_client_id_document_response(state).await),

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
            })
            .to_string();
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
                    _ => {
                        return Ok(routing::bad_request_response(
                            "limit must be between 1 and 250",
                        ))
                    }
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
            })
            .to_string();
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
            let content_id =
                share_stream_content_id(path).expect("guarded share stream ticket path");
            let Some(token) = request_share_token(authorization, &headers) else {
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
                || is_authorized(&state.config, authorization, headers.cookie.as_deref());
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
            let transfers = state.transfers.read().await;
            let shares = state.shares.read().await;
            let transfer = stream_id
                .strip_prefix("transfer-")
                .and_then(|id| id.parse::<u64>().ok())
                .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id));
            let share = shares.entries.iter().find(|entry| {
                entry.filename == stream_id
                    || stable_content_hash(&entry.filename, entry.size).to_string() == stream_id
            });
            if !api_authorized && ticket_record.is_none() {
                drop(shares);
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
            drop(shares);
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
            let (family, raw_ticket) = if let Some(ticket) = path.strip_prefix("/api/mesh-streams/")
            {
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
            let room =
                extract_json_string_field(body, "room").unwrap_or_else(|| "radio".to_owned());
            let title = extract_json_string_field(body, "title")
                .unwrap_or_else(|| "party content".to_owned());
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
                && state.config.controller_profile == ControllerProfile::Native =>
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
        | ("POST", "/api/multisource/download")
            if normalized_path != "/api/multisource/download"
                || serde_json::from_str::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("sources")
                            .and_then(serde_json::Value::as_array)
                            .cloned()
                    })
                    .is_some_and(|sources| !sources.is_empty()) =>
        {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                if normalized_path == "/api/multisource/download" {
                    return Ok(multisource_versioned_download_response(body));
                }
                return Ok(
                    multisource_versioned_swarm_response(normalized_path, body, state).await,
                );
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
                format!("multisource/{id}-{}", virtual_basename(&request.filename))
            });
            let downloads_dir = effective_downloads_dir(state);
            let output_path =
                match safe_download_path(&downloads_dir, &relative_path).and_then(|path| {
                    ensure_scoped_download_path(&downloads_dir, path.to_string_lossy().as_ref())
                }) {
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
            let result =
                multisource::execute(id, request, output_path, public_output_path, store).await;
            Ok(routing::ok_response(
                serde_json::to_string(&result)
                    .map_err(|error| format!("multisource result serialization failed: {error}"))?,
            ))
        }

        ("POST", "/api/multisource/download") => {
            if route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native
            {
                return Ok(multisource_versioned_download_response(body));
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
            let verified = match verify_pod_join_signature(mode, &input, now.saturating_mul(1_000))
            {
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
                if let Err(error) = reserve_pod_join_replay_key(state, &input, now).await {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        &error
                    }));
                }
            }
            let pod_exists_and_is_not_member = {
                let pods = state.pods.read().await;
                let pod_state = pods.get(&input.pod_id).map(|_| {
                    pods.members(&input.pod_id).is_none_or(|members| {
                        !members
                            .iter()
                            .any(|member| member.peer_id.eq_ignore_ascii_case(&input.peer_id))
                    })
                });
                drop(pods);
                if pod_state.is_some() {
                    pod_state
                } else {
                    let rooms = state.rooms.read().await;
                    rooms
                        .records
                        .iter()
                        .find(|room| room.name == input.pod_id)
                        .map(|room| {
                            !room
                                .members
                                .iter()
                                .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                        })
                }
            };
            match pod_exists_and_is_not_member {
                None => {
                    if mode == PodSignatureMode::Enforce {
                        release_pod_join_replay_key(state, &input).await;
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Some(false) => {
                    if mode == PodSignatureMode::Enforce {
                        release_pod_join_replay_key(state, &input).await;
                    }
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join request could not be processed"
                    } else {
                        "Already a member of this pod"
                    }));
                }
                Some(true) => {}
            }
            let mut workflow = state.pod_membership_workflow.write().await;
            if let Err(error) = workflow.add_join(input.clone()) {
                drop(workflow);
                if mode == PodSignatureMode::Enforce {
                    release_pod_join_replay_key(state, &input).await;
                }
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join request could not be processed"
                } else {
                    error
                }));
            }
            drop(workflow);
            let body = serde_json::json!({
                "success": true,
                "podId": input.pod_id,
                "peerId": input.peer_id,
                "joinRequest": input,
                "signatureMode": mode.as_str(),
                "signatureVerified": verified,
            })
            .to_string();
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/podcore/membership/join/accept") => {
            let input = match PodJoinAcceptanceInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.acceptor_public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod join acceptance",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join acceptance could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if !pod_acceptor_has_permission(state, &input.pod_id, &input.acceptor_peer_id).await {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept join requests"
                }));
            }
            let request = {
                state
                    .pod_membership_workflow
                    .write()
                    .await
                    .remove_join(&input.pod_id, &input.peer_id)
            };
            let Some(request) = request else {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Join acceptance could not be processed"
                } else {
                    "No pending join request found"
                }));
            };
            let added = state
                .rooms
                .write()
                .await
                .add_member(&input.pod_id, input.peer_id.clone());
            match added {
                Ok(Some(_)) => {}
                Ok(None) => {
                    let _ = state
                        .pod_membership_workflow
                        .write()
                        .await
                        .add_join(request);
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Join acceptance could not be processed"
                    } else {
                        "Pod not found"
                    }));
                }
                Err(()) => {
                    let _ = state
                        .pod_membership_workflow
                        .write()
                        .await
                        .add_join(request);
                    return Ok(if request_is_versioned_v0 {
                        routing::bad_request_response("Join acceptance could not be processed")
                    } else {
                        routing::service_unavailable_response("pod member capacity is full")
                    });
                }
            }
            state.pod_membership_workflow.write().await.set_role(
                &input.pod_id,
                &input.peer_id,
                input.accepted_role.clone(),
            );
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "operation": "join_acceptance",
                    "request": request,
                    "response": input,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/leave") => {
            let input = match PodLeaveRequestInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod leave",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave request could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            let is_member = state
                .rooms
                .read()
                .await
                .records
                .iter()
                .find(|room| room.name == input.pod_id)
                .is_some_and(|room| {
                    room.members
                        .iter()
                        .any(|member| member.eq_ignore_ascii_case(&input.peer_id))
                });
            if !is_member {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave request could not be processed"
                } else {
                    "Not a member of this pod"
                }));
            }
            let privileged = matches!(
                state
                    .pod_membership_workflow
                    .read()
                    .await
                    .role(&input.pod_id, &input.peer_id),
                "owner" | "moderator"
            );
            if privileged {
                if let Err(error) = state
                    .pod_membership_workflow
                    .write()
                    .await
                    .add_leave(input.clone())
                {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave request could not be processed"
                    } else {
                        error
                    }));
                }
            } else {
                state
                    .rooms
                    .write()
                    .await
                    .remove_member(&input.pod_id, &input.peer_id);
                state
                    .pod_membership_workflow
                    .write()
                    .await
                    .remove_role(&input.pod_id, &input.peer_id);
            }
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "leaveRequest": input,
                    "pending": privileged,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/podcore/membership/leave/accept") => {
            let input = match PodLeaveAcceptanceInput::from_json(body) {
                Ok(input) => input,
                Err(error) => return Ok(routing::bad_request_response(&error)),
            };
            let mode = state
                .advanced_networking
                .read()
                .await
                .pod_join_signature_mode;
            let verified = match verify_pod_signed_payload(
                mode,
                &input.signature,
                &input.acceptor_public_key,
                input.timestamp_unix_ms,
                unix_timestamp().saturating_mul(1_000),
                &input.canonical_payload(),
                "pod leave acceptance",
            ) {
                Ok(verified) => verified,
                Err(error) => {
                    return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                        "Leave acceptance could not be processed"
                    } else {
                        &error
                    }))
                }
            };
            if !pod_acceptor_has_permission(state, &input.pod_id, &input.acceptor_peer_id).await {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Acceptor does not have permission to accept leave requests"
                }));
            }
            let request = state
                .pod_membership_workflow
                .write()
                .await
                .remove_leave(&input.pod_id, &input.peer_id);
            let Some(request) = request else {
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "No pending leave request found"
                }));
            };
            if state
                .rooms
                .write()
                .await
                .remove_member(&input.pod_id, &input.peer_id)
                .is_none()
            {
                let _ = state
                    .pod_membership_workflow
                    .write()
                    .await
                    .add_leave(request);
                return Ok(routing::bad_request_response(if request_is_versioned_v0 {
                    "Leave acceptance could not be processed"
                } else {
                    "Not a member of this pod"
                }));
            }
            state
                .pod_membership_workflow
                .write()
                .await
                .remove_role(&input.pod_id, &input.peer_id);
            Ok(routing::ok_response(
                serde_json::json!({
                    "success": true,
                    "podId": input.pod_id,
                    "peerId": input.peer_id,
                    "operation": "leave_acceptance",
                    "request": request,
                    "response": input,
                    "signatureMode": mode.as_str(),
                    "signatureVerified": verified,
                })
                .to_string(),
            ))
        }

        ("GET", path) if pod_pending_request_has_blank_id(path, "join") => {
            Ok(routing::bad_request_response("PodId is required"))
        }

        ("GET", path) if pod_pending_request_has_blank_id(path, "leave") => {
            Ok(routing::bad_request_response("PodId is required"))
        }

        ("GET", path) if pod_pending_request_path(path, "join").is_some() => {
            let pod_id = pod_pending_request_path(path, "join").unwrap_or_default();
            let workflow = state.pod_membership_workflow.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "pendingJoinRequests": workflow.pending_joins(&pod_id),
                })
                .to_string(),
            ))
        }

        ("GET", path) if pod_pending_request_path(path, "leave").is_some() => {
            let pod_id = pod_pending_request_path(path, "leave").unwrap_or_default();
            let workflow = state.pod_membership_workflow.read().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "pendingLeaveRequests": workflow.pending_leaves(&pod_id),
                })
                .to_string(),
            ))
        }

        ("DELETE", path) if pod_cancel_request_path(path, "join").is_some() => {
            let (pod_id, peer_id) = pod_cancel_request_path(path, "join").unwrap_or_default();
            if state
                .pod_membership_workflow
                .write()
                .await
                .remove_join(&pod_id, &peer_id)
                .is_some()
            {
                Ok(routing::ok_response(r#"{"cancelled":true}"#.to_owned()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        ("DELETE", path) if pod_cancel_request_path(path, "leave").is_some() => {
            let (pod_id, peer_id) = pod_cancel_request_path(path, "leave").unwrap_or_default();
            if state
                .pod_membership_workflow
                .write()
                .await
                .remove_leave(&pod_id, &peer_id)
                .is_some()
            {
                Ok(routing::ok_response(r#"{"cancelled":true}"#.to_owned()))
            } else {
                Ok(routing::not_found_response())
            }
        }

        ("GET", "/api/playback/status") => {
            let now_playing = state.now_playing.read().await;
            let body = serde_json::json!({
                "status": if now_playing.records.is_empty() { "stopped" } else { "playing" },
                "nowPlaying": now_playing.records.iter().map(|record| {
                    serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_else(|_| serde_json::json!({}))
                }).collect::<Vec<_>>(),
                "count": now_playing.records.len(),
                "updated_at": now_playing.updated_at,
            }).to_string();
            drop(now_playing);
            Ok(routing::ok_response(body))
        }

        ("GET", "/api/traces") => {
            let events = state.events.read().await;
            let traces = events
                .records
                .iter()
                .rev()
                .take(100)
                .map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "kind": event.kind,
                        "resource": event.resource,
                        "detail": event.detail,
                        "created_at": event.created_at,
                    })
                })
                .collect::<Vec<_>>();
            let count = traces.len();
            drop(events);
            Ok(routing::ok_response(
                serde_json::json!({
                    "traces": traces,
                    "count": count,
                })
                .to_string(),
            ))
        }

        // FairnessController.GetSummary is a versioned native profile DTO, while the
        // unversioned /api/fairness route remains slskR's legacy ranking
        // projection.  With no recorded traffic, the frozen guard returns a
        // neutral upload/download ratio, a zero overlay/Soulseek ratio, and
        // an explicit within-constraints reason.
        ("GET", "/api/fairness")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/fairness/summary" =>
        {
            let totals = if let Some(db) = state.db.as_ref() {
                match db.get_traffic_totals().await {
                    Ok(totals) => totals,
                    Err(error) => {
                        return Ok(routing::internal_server_error_response(&format!(
                            "fairness storage unavailable: {error}"
                        )))
                    }
                }
            } else {
                persistence::TrafficTotalsRecord::default()
            };
            let overlay_upload = totals.overlay_upload_bytes.max(0) as f64;
            let overlay_download = totals.overlay_download_bytes.max(0) as f64;
            let soulseek_upload = totals.soulseek_upload_bytes.max(0) as f64;
            let upload_download_ratio = if overlay_download > 0.0 {
                overlay_upload / overlay_download
            } else {
                1.0
            };
            let overlay_to_soulseek_ratio = if soulseek_upload > 0.0 {
                overlay_upload / soulseek_upload
            } else if overlay_upload > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };
            let mut throttle = upload_download_ratio < 0.5;
            let mut reasons = Vec::new();
            if throttle {
                reasons.push(format!(
                    "overlay upload/download ratio {upload_download_ratio:.2} below minimum 0.50"
                ));
            }
            if overlay_to_soulseek_ratio > 3.0 {
                throttle = true;
                reasons.push(format!(
                    "overlay/Soulseek upload ratio {overlay_to_soulseek_ratio:.2} above maximum 3.00"
                ));
            }
            let reason = if reasons.is_empty() {
                "within fairness constraints".to_owned()
            } else {
                reasons.join("; ")
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "throttleOverlayDownloads": throttle,
                    "reason": reason,
                    "overlayUploadDownloadRatio": upload_download_ratio,
                    "overlayToSoulseekUploadRatio": if overlay_to_soulseek_ratio.is_finite() {
                        serde_json::json!(overlay_to_soulseek_ratio)
                    } else {
                        serde_json::json!("Infinity")
                    },
                    "totals": {
                        "overlayUploadBytes": totals.overlay_upload_bytes.max(0),
                        "overlayDownloadBytes": totals.overlay_download_bytes.max(0),
                        "soulseekUploadBytes": totals.soulseek_upload_bytes.max(0),
                        "soulseekDownloadBytes": totals.soulseek_download_bytes.max(0),
                    },
                })
                .to_string(),
            ))
        }
        ("GET", "/api/fairness") | ("GET", "/api/ranking") => {
            let mut response = native_compat_response(method, normalized_path, state).await;
            if normalized_path == "/api/fairness" {
                let transfers = state.transfers.read().await;
                let downloaded = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction == 0)
                    .map(|entry| entry.bytes_transferred)
                    .sum::<u64>();
                let uploaded = transfers
                    .entries
                    .iter()
                    .filter(|entry| entry.direction != 0)
                    .map(|entry| entry.bytes_transferred)
                    .sum::<u64>();
                let mut value = serde_json::from_str::<serde_json::Value>(&response.body)
                    .unwrap_or_else(|_| serde_json::json!({}));
                value["throttleOverlayDownloads"] = serde_json::json!(false);
                value["reason"] = serde_json::Value::Null;
                value["overlayUploadDownloadRatio"] = serde_json::json!(0.0);
                value["overlayToSoulseekUploadRatio"] = serde_json::json!(0.0);
                value["totals"] = serde_json::json!({
                    "downloadedBytes": downloaded,
                    "uploadedBytes": uploaded,
                });
                response.body = value.to_string();
            }
            Ok(response)
        }

        ("GET", "/api/port-forwarding/status") => Ok(routing::ok_response(
            serde_json::to_string(&state.port_forwarding.statuses().await)
                .unwrap_or_else(|_| "[]".to_owned()),
        )),

        ("GET", path) if path.starts_with("/api/port-forwarding/status/") => {
            let Some(local_port) = path_segment_after(path, "/api/port-forwarding/status/") else {
                return Ok(routing::not_found_response());
            };
            if local_port.parse::<u16>().is_err() || local_port == "0" {
                return Ok(routing::not_found_response());
            }
            let local_port = local_port.parse::<u16>().unwrap_or_default();
            match state.port_forwarding.status(local_port).await {
                Some(status) => Ok(routing::ok_response(
                    serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_owned()),
                )),
                None => Ok(routing::not_found_response()),
            }
        }

        ("GET", "/api/port-forwarding/available-ports") => {
            let start_port = match query_bounded_usize(route.query, "startPort", 1, 65_535) {
                Ok(value) => value.unwrap_or(1_024),
                Err(()) => {
                    return Ok(routing::bad_request_response("Invalid port range"));
                }
            };
            let end_port = match query_bounded_usize(route.query, "endPort", 1, 65_535) {
                Ok(value) => value.unwrap_or(65_535),
                Err(()) => {
                    return Ok(routing::bad_request_response("Invalid port range"));
                }
            };
            if start_port > end_port {
                return Ok(routing::bad_request_response("Invalid port range"));
            }
            let limit = match query_bounded_usize(route.query, "limit", 1, 1_000) {
                Ok(value) => value,
                Err(()) => {
                    return Ok(routing::bad_request_response(
                        "Limit must be between 1 and 1000",
                    ));
                }
            };
            let used_ports = state
                .port_forwarding
                .used_ports()
                .await
                .into_iter()
                .map(usize::from)
                .filter(|port| (start_port..=end_port).contains(port))
                .collect::<HashSet<_>>();
            let available_port_count = end_port - start_port + 1 - used_ports.len();
            let returned_port_count = limit.unwrap_or(1_000).min(available_port_count);
            let available_ports = (start_port..=end_port)
                .filter(|port| !used_ports.contains(port))
                .take(returned_port_count)
                .collect::<Vec<_>>();
            Ok(routing::ok_response(
                serde_json::json!({
                    "availablePortCount": available_port_count,
                    "availablePorts": available_ports,
                    "usedPortCount": used_ports.len(),
                })
                .to_string(),
            ))
        }

        ("GET", "/api/port-forwarding/stream-stats") => {
            let rules = state.port_forwarding.statuses().await;
            Ok(routing::ok_response(
                serde_json::json!({
                    "totalForwardingRules": rules.len(),
                    "activeRules": rules.iter().filter(|rule| rule.is_active).count(),
                    "totalConnections": rules.iter().map(|rule| rule.active_connections).sum::<usize>(),
                    "totalBytesForwarded": rules.iter().map(|rule| rule.bytes_forwarded).sum::<u64>(),
                    "rules": rules,
                })
                .to_string(),
            ))
        }

        ("POST", "/api/port-forwarding/start") => {
            let request = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) if value.is_object() => value,
                _ => return Ok(routing::bad_request_response("Request is required")),
            };
            let local_port = request.get("localPort").and_then(serde_json::Value::as_u64);
            let destination_port = request
                .get("destinationPort")
                .and_then(serde_json::Value::as_u64);
            let pod_id = request
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let destination_host = request
                .get("destinationHost")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            let service_name = request
                .get("serviceName")
                .and_then(serde_json::Value::as_str)
                .map(str::trim);
            if !matches!(local_port, Some(1_024..=65_535)) {
                return Ok(routing::bad_request_response(
                    "Local port must be between 1024 and 65535",
                ));
            }
            if pod_id.is_empty() || pod_id.len() > 100 {
                return Ok(routing::bad_request_response(
                    "PodId must be between 1 and 100 characters",
                ));
            }
            if destination_host.is_empty() || destination_host.len() > 253 {
                return Ok(routing::bad_request_response(
                    "Destination host must be between 1 and 253 characters",
                ));
            }
            if !matches!(destination_port, Some(1..=65_535)) {
                return Ok(routing::bad_request_response(
                    "Destination port must be between 1 and 65535",
                ));
            }
            if service_name.is_some_and(|value| value.len() > 100) {
                return Ok(routing::bad_request_response(
                    "Service name must be at most 100 characters",
                ));
            }
            let local_port = local_port.unwrap_or_default() as u16;
            let destination_port = destination_port.unwrap_or_default() as u16;
            let Some(local_username) = pod_request_peer_id(state).await else {
                return Ok(routing::forbidden_response(
                    "Authenticated peer identity is required",
                ));
            };
            let (pod, pod_gateway_certificate_sha256) = {
                let pods = state.pods.read().await;
                if !pods.is_member(pod_id, &local_username) {
                    return Ok(routing::forbidden_response(
                        "Only pod members can start port forwarding",
                    ));
                }
                if !pods.destination_allowed(pod_id, destination_host, destination_port) {
                    return Ok(routing::forbidden_response(
                        "Destination is not allowed by the Pod private-gateway policy",
                    ));
                }
                (pods.get(pod_id), pods.gateway_certificate_sha256(pod_id))
            };
            let Some(pod) = pod else {
                return Ok(routing::not_found_response());
            };
            let gateway_peer_id = pod
                .private_service_policy
                .as_ref()
                .and_then(serde_json::Value::as_object)
                .and_then(|policy| policy.get("gatewayPeerId"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty());
            let Some(gateway_peer_id) = gateway_peer_id else {
                return Ok(routing::service_unavailable_response(
                    "Pod private-gateway policy has no designated gateway",
                ));
            };
            let trusted_gateway = state
                .config
                .trusted_mesh_peers
                .iter()
                .find(|peer| peer.matches(gateway_peer_id));
            if let (Some(pod_pin), Some(trusted)) =
                (pod_gateway_certificate_sha256, trusted_gateway)
            {
                if pod_pin != trusted.certificate_sha256 {
                    return Ok(routing::service_unavailable_response(
                        "Pod and operator gateway certificate pins conflict",
                    ));
                }
            }
            let gateway_certificate_sha256 = pod_gateway_certificate_sha256
                .or_else(|| trusted_gateway.map(|peer| peer.certificate_sha256));
            let Some(gateway_certificate_sha256) = gateway_certificate_sha256 else {
                return Ok(routing::service_unavailable_response(
                    "Gateway has no authenticated TLS certificate pin",
                ));
            };
            let descriptor = state
                .mesh
                .read()
                .await
                .capability_records
                .iter()
                .find(|descriptor| {
                    descriptor.peer_id.eq_ignore_ascii_case(gateway_peer_id)
                        || descriptor.username.eq_ignore_ascii_case(gateway_peer_id)
                })
                .cloned();
            if descriptor.is_none() && trusted_gateway.is_none() {
                return Ok(routing::service_unavailable_response(
                    "Gateway peer capability record and operator trust entry are unavailable",
                ));
            }
            let gateway_username = descriptor
                .as_ref()
                .map(|descriptor| descriptor.username.clone())
                .or_else(|| trusted_gateway.map(|peer| peer.username.clone()))
                .expect("a descriptor or trusted gateway exists");
            let mut gateway_endpoints = Vec::new();
            if let Some(trusted) = trusted_gateway {
                gateway_endpoints.push(trusted.overlay_endpoint);
            }
            if let Some(descriptor) = descriptor.as_ref() {
                if let Some(overlay_port) = descriptor.overlay_port {
                    if let Some(peer_address) =
                        cached_peer_endpoint(state, &descriptor.username).await
                    {
                        let endpoint = SocketAddr::V4(SocketAddrV4::new(
                            peer_connect_ip(state, &peer_address),
                            overlay_port,
                        ));
                        if !gateway_endpoints.contains(&endpoint) {
                            gateway_endpoints.push(endpoint);
                        }
                    }
                }
            }
            if let Some(dht) = state.dht.as_ref() {
                for endpoint in dht.peers().await {
                    if gateway_endpoints.len() >= port_forwarding::MAX_GATEWAY_ENDPOINTS {
                        break;
                    }
                    if !gateway_endpoints.contains(&endpoint) {
                        gateway_endpoints.push(endpoint);
                    }
                }
            }
            if gateway_endpoints.is_empty() {
                if let Some(descriptor) = descriptor.as_ref() {
                    if let Some(overlay_port) = descriptor.overlay_port {
                        let peer_address =
                            match request_peer_endpoint(state, &descriptor.username).await {
                                Ok(address) => address,
                                Err(error) => {
                                    return Ok(routing::service_unavailable_response(&error));
                                }
                            };
                        gateway_endpoints.push(SocketAddr::V4(SocketAddrV4::new(
                            peer_connect_ip(state, &peer_address),
                            overlay_port,
                        )));
                    }
                }
            }
            if gateway_endpoints.is_empty() {
                return Ok(routing::service_unavailable_response(
                    "Gateway has no reachable overlay endpoint",
                ));
            }
            match state
                .port_forwarding
                .start(port_forwarding::StartRequest {
                    local_port,
                    pod_id: pod_id.to_owned(),
                    destination_host: destination_host.to_owned(),
                    destination_port,
                    service_name: service_name.map(str::to_owned),
                    gateway_username,
                    gateway_endpoints,
                    gateway_certificate_sha256,
                    local_username,
                    authentication_key: Arc::new(state.capability_signing_key.clone()),
                })
                .await
            {
                Ok(_) => Ok(routing::ok_response(
                    r#"{"message":"Port forwarding started"}"#.to_owned(),
                )),
                Err(error) if error.contains("already being forwarded") => {
                    Ok(routing::conflict_response(&error))
                }
                Err(error) => {
                    eprintln!("port forwarding start failed: {error}");
                    Ok(
                        if state.config.controller_profile == ControllerProfile::Native
                            && route.path.starts_with("/api/v0/")
                        {
                            routing::internal_server_error_response(
                                "Failed to start port forwarding",
                            )
                        } else {
                            routing::service_unavailable_response("port forwarding is unavailable")
                        },
                    )
                }
            }
        }

        ("POST", path) if path.starts_with("/api/port-forwarding/stop/") => {
            let Some(local_port) = path_segment_after(path, "/api/port-forwarding/stop/") else {
                return Ok(routing::not_found_response());
            };
            if local_port.parse::<u16>().is_err() || local_port == "0" {
                return Ok(routing::not_found_response());
            }
            state
                .port_forwarding
                .stop(local_port.parse().unwrap_or_default())
                .await;
            Ok(routing::ok_response(
                r#"{"message":"Port forwarding stopped"}"#.to_owned(),
            ))
        }

        ("GET", "/api/portforwarding/status") => Ok(routing::ok_response(
            serde_json::to_string(&state.port_forwarding.statuses().await)
                .unwrap_or_else(|_| "[]".to_owned()),
        )),

        ("GET", "/api/signals") => {
            let session = state.session.read().await;
            let events = state.events.read().await;
            let body = serde_json::json!({
                "signals": events.records.iter().rev().take(25).map(|event| {
                    serde_json::json!({
                        "id": event.id,
                        "kind": event.kind,
                        "resource": event.resource,
                        "created_at": event.created_at,
                    })
                }).collect::<Vec<_>>(),
                "connected": session.state == "connected",
                "count": events.records.len().min(25),
            })
            .to_string();
            drop(events);
            drop(session);
            Ok(routing::ok_response(body))
        }

        ("POST", "/api/backfill") => {
            let searches = state.searches.read().await;
            let shares = state.shares.read().await;
            let queued = searches.records.len() + shares.entries.len();
            drop(shares);
            drop(searches);
            let body = match mutate_runtime_compat_state(state, |runtime, _| {
                runtime.record_backfill(queued).to_string()
            })
            .await
            {
                Ok(body) => body,
                Err(error) => return Ok(routing::service_unavailable_response(&error)),
            };
            Ok(routing::accepted_response(body))
        }
        ("GET", path) if path.starts_with("/api/mediacore/") => {
            Ok(mediacore_extended_response(path, route.query, state).await)
        }
        ("GET", path) if extended_controller_get_route(path) => {
            Ok(extended_controller_get_response(
                path,
                route.query,
                state,
                route.path.starts_with("/api/v0/"),
            )
            .await)
        }
        ("GET", path) if extended_controller_dynamic_get_route(path) => {
            Ok(extended_controller_dynamic_get_response(
                path,
                route.query,
                state,
                route.path.starts_with("/api/v0/"),
            )
            .await)
        }
        (method, path) if extended_mutation => Ok(extended_controller_mutation_response(
            method,
            path,
            route.query,
            body,
            state,
            route.path.starts_with("/api/v0/"),
            &headers,
        )
        .await),
        (method, path) if native_compat_route(method, path) => {
            Ok(native_compat_response(method, path, state).await)
        }
        ("GET", path) if is_spa_navigation_path(path) => Ok(index_html_response()),
        ("HEAD", path) if is_spa_navigation_path(path) => Ok(head_response(index_html_response())),
        _ => {
            tracing::complete_request_span(404);
            Ok(routing::unmatched_route_response())
        }
    }
}
