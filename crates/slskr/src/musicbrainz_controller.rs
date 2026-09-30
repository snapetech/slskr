use super::*;

fn safe_radar_reference(value: &str) -> bool {
    value.len() <= 256
        && value.chars().all(|character| {
            character.is_alphanumeric() || matches!(character, '-' | '_' | ':' | '.' | '@')
        })
}

pub(super) async fn musicbrainz_mutation_response(
    path: &str,
    body: &str,
    state: &AppState,
    is_versioned_v0: bool,
) -> HttpResponse {
    if path == "/api/musicbrainz/library-bloom/snapshots/preview" {
        let request = serde_json::from_str::<serde_json::Value>(body)
            .unwrap_or_else(|_| serde_json::json!({}));
        // Matches native profile's real item source: locally-held hashdb entries
        // that carry a MusicBrainz recording id, not the generic library
        // catalog (which has no MusicBrainz identifiers to key membership
        // on). slskR's hashdb doesn't separately track release ids, so
        // every real item is tagged under the recording namespace.
        const RECORDING_NAMESPACE: &str = "musicbrainz:recording";
        let discovery = state.content_discovery.read().await;
        let mbids: Vec<String> = discovery
            .hash_entries()
            .iter()
            .filter(|entry| !entry.music_brainz_id.is_empty())
            .map(|entry| entry.music_brainz_id.clone())
            .collect();
        drop(discovery);
        let digest = hex::encode(Sha256::digest(mbids.join("\n").as_bytes()));
        let salt_id = request
            .get("saltId")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("salt:{}", uuid::Uuid::new_v4().simple()));
        let expected_items = request
            .get("expectedItems")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1_024)
            .clamp(16, 1_000_000)
            .max(mbids.len() as u64);
        let false_positive_rate = request
            .get("falsePositiveRate")
            .and_then(serde_json::Value::as_f64)
            .filter(|value| (0.0..1.0).contains(value))
            .unwrap_or(0.01);
        let mut filter =
            match bloom_filter::SaltedBloomFilter::try_new(expected_items, false_positive_rate) {
                Ok(filter) => filter,
                Err(error) => return routing::bad_request_response(&error.to_string()),
            };
        for mbid in &mbids {
            filter.add(&bloom_filter::build_salted_item(
                &salt_id,
                RECORDING_NAMESPACE,
                mbid,
            ));
        }
        let namespace_item_counts = if mbids.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({ RECORDING_NAMESPACE: mbids.len() })
        };
        return routing::ok_response(
            serde_json::json!({
                "version": 1,
                "snapshotId": format!("library-bloom:{}", uuid::Uuid::new_v4().simple()),
                "scope": "manual-preview",
                "saltId": salt_id,
                "createdAt": chrono::Utc::now().to_rfc3339(),
                "rotatesAt": request.get("rotatesAt").cloned().unwrap_or(serde_json::Value::Null),
                "expectedItems": filter.expected_items(),
                "falsePositiveRate": filter.false_positive_rate(),
                "bitSize": filter.bit_size(),
                "hashFunctionCount": filter.hash_function_count(),
                "itemCount": filter.item_count(),
                "fillRatio": filter.fill_ratio(),
                "bitsBase64": filter.to_base64(),
                "namespaceItemCounts": namespace_item_counts,
                "privacyNotes": [
                    "Snapshot contains salted Bloom-filter membership only; it does not include filenames, paths, file hashes, or exact item identifiers.",
                    "Bloom matches are probabilistic and must be treated as likely suggestions, not proof of remote holdings.",
                    "Rotate SaltId before long-lived publication to reduce cross-snapshot correlation."
                ],
                "digestSha256": digest,
                "generatedAt": unix_timestamp(),
            })
            .to_string(),
        );
    }
    if path == "/api/musicbrainz/library-bloom/diffs" {
        let payload = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(payload @ serde_json::Value::Object(_)) => payload,
            Ok(_) => return routing::bad_request_response("diff request must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        let remote = payload
            .get("recordingIds")
            .or_else(|| payload.get("items"))
            .and_then(serde_json::Value::as_array);
        if remote.is_some_and(|items| items.len() > MAX_LIBRARY_ITEMS) {
            return routing::bad_request_response(&format!(
                "recordingIds must contain at most {MAX_LIBRARY_ITEMS} items"
            ));
        }
        let remote = remote.cloned().unwrap_or_default();
        let local = state
            .content_discovery
            .read()
            .await
            .hash_entries()
            .iter()
            .map(|entry| entry.music_brainz_id.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let missing = remote
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .filter(|id| !local.contains(&id.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({
                "isCompatible": true,
                "missingRecordingIds": missing,
                "missingCount": missing.len(),
            })
            .to_string(),
        );
    }
    if path == "/api/musicbrainz/library-bloom/wishlist" {
        let payload = serde_json::from_str::<serde_json::Value>(body).ok();
        let suggestions = payload.as_ref().and_then(|value| {
            value
                .get("suggestions")
                .or_else(|| value.get("items"))
                .and_then(serde_json::Value::as_array)
        });
        if suggestions.is_some_and(|items| items.len() > MAX_WISHLIST_ITEMS) {
            return routing::bad_request_response(&format!(
                "suggestions must contain at most {MAX_WISHLIST_ITEMS} items"
            ));
        }
        let suggestions = suggestions.cloned().unwrap_or_default();
        let mut created = Vec::new();
        for suggestion in suggestions.into_iter().take(MAX_WISHLIST_ITEMS) {
            let artist = suggestion
                .get("artist")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let title = suggestion
                .get("title")
                .or_else(|| suggestion.get("releaseTitle"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if artist.is_empty() && title.is_empty() {
                continue;
            }
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let item = {
                let mut wishlist = state.wishlist.write().await;
                match wishlist.add_item(artist.to_owned(), title.to_owned(), "release".to_owned()) {
                    Ok(item) => item,
                    Err(()) => break,
                }
            };
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                return routing::service_unavailable_response(&error);
            }
            created
                .push(serde_json::from_str::<serde_json::Value>(&item.json()).unwrap_or_default());
        }
        return routing::ok_response(
            serde_json::json!({"promoted": created.len(), "items": created}).to_string(),
        );
    }
    if let Some(artist_id) = path_segment_between(
        path,
        "/api/musicbrainz/artist/",
        "/discography-coverage/wishlist",
    ) {
        let payload = if is_versioned_v0 {
            match serde_json::from_str::<serde_json::Value>(body) {
                Ok(payload @ serde_json::Value::Object(_)) => payload,
                _ => return routing::bad_request_response("request body is required"),
            }
        } else {
            serde_json::from_str::<serde_json::Value>(body)
                .unwrap_or_else(|_| serde_json::json!({}))
        };
        if is_versioned_v0 {
            let artist = decoded_path_segment(artist_id);
            let profile = payload
                .get("profile")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("CoreDiscography")
                .to_owned();
            let filter = payload
                .get("filter")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("flac")
                .trim()
                .to_owned();
            let max_results = match payload.get("maxResults") {
                None => 100,
                Some(value) => {
                    let Some(value) = value.as_i64() else {
                        return routing::bad_request_response("MaxResults must be greater than 0");
                    };
                    if value <= 0 {
                        return routing::bad_request_response("MaxResults must be greater than 0");
                    }
                    usize::try_from(value)
                        .unwrap_or(MAX_WISHLIST_RESULTS)
                        .clamp(1, MAX_WISHLIST_RESULTS)
                }
            };

            // Reuse the coverage map when the caller already loaded it. If
            // promotion is invoked directly, build and cache the map here so
            // the endpoint remains a complete operation on its own.
            let cache_key = format!("musicbrainz/discography/{artist}/{profile}");
            let coverage = state
                .controller_features
                .read()
                .await
                .get(&cache_key)
                .cloned();
            let coverage = if let Some(coverage) = coverage {
                coverage
            } else {
                let settings = state.integration_settings.read().await.musicbrainz.clone();
                match crate::route_dispatch::musicbrainz_discography_coverage_with_settings(
                    state, &settings, &artist, &profile, false,
                )
                .await
                {
                    Ok(Some(coverage)) => coverage,
                    Ok(None) => return routing::not_found_response(),
                    Err(error) => {
                        return routing::internal_server_error_response(&format!(
                            "MusicBrainz coverage lookup failed: {error}"
                        ))
                    }
                }
            };

            let artist_name = coverage
                .get("artistName")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&artist)
                .to_owned();
            let mut candidates = Vec::new();
            for release in coverage
                .get("releases")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                for track in release
                    .get("tracks")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if track.get("status").and_then(serde_json::Value::as_str) != Some("Absent") {
                        continue;
                    }
                    let title = track
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .trim();
                    if title.is_empty() {
                        continue;
                    }
                    let track_artist = track
                        .get("artist")
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or(&artist_name)
                        .trim();
                    candidates.push((track_artist.to_owned(), title.to_owned()));
                }
            }

            let mut existing = {
                let wishlist = state.wishlist.read().await;
                wishlist
                    .records
                    .iter()
                    .flat_map(|record| record.items.iter())
                    .map(|item| {
                        (
                            item.search_text()
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .to_ascii_lowercase(),
                            item.filter.trim().to_ascii_lowercase(),
                        )
                    })
                    .collect::<HashSet<_>>()
            };
            let mut created_item_ids = Vec::new();
            let mut already_seeded_count = 0_u64;
            for (track_artist, title) in candidates {
                let search_text = format!("{track_artist} {title}")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_lowercase();
                let key = (search_text, filter.to_ascii_lowercase());
                if !existing.insert(key) {
                    already_seeded_count = already_seeded_count.saturating_add(1);
                    continue;
                }
                if created_item_ids.len() >= max_results {
                    break;
                }
                let _wishlist_search_persistence =
                    state.wishlist_search_persistence_lock.lock().await;
                let item = {
                    let mut wishlist = state.wishlist.write().await;
                    match wishlist.add_item_with_settings(
                        track_artist,
                        title,
                        "MusicBrainzDiscography".to_owned(),
                        filter.clone(),
                        true,
                        false,
                        max_results,
                        None,
                    ) {
                        Ok(item) => item,
                        Err(()) => break,
                    }
                };
                if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                    return routing::service_unavailable_response(&error);
                }
                created_item_ids.push(item.id);
            }
            return routing::ok_response(
                serde_json::json!({
                    "artistId": artist,
                    "createdCount": created_item_ids.len(),
                    "alreadySeededCount": already_seeded_count,
                    "createdItemIds": created_item_ids,
                })
                .to_string(),
            );
        }
        let titles = payload
            .get("missingReleases")
            .or_else(|| payload.get("releases"))
            .and_then(serde_json::Value::as_array);
        if titles.is_some_and(|items| items.len() > MAX_WISHLIST_ITEMS) {
            return routing::bad_request_response(&format!(
                "missingReleases must contain at most {MAX_WISHLIST_ITEMS} items"
            ));
        }
        let titles = titles.cloned().unwrap_or_default();
        if titles.is_empty() {
            return routing::bad_request_response("missingReleases are required");
        }
        let artist = decoded_path_segment(artist_id);
        let mut promoted = 0;
        for title in titles.into_iter().filter_map(|value| {
            value.as_str().map(str::to_owned).or_else(|| {
                value
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
        }) {
            let _wishlist_search_persistence = state.wishlist_search_persistence_lock.lock().await;
            let item = {
                let mut wishlist = state.wishlist.write().await;
                match wishlist.add_item(artist.clone(), title, "release".to_owned()) {
                    Ok(item) => item,
                    Err(()) => break,
                }
            };
            if let Err(error) = persist_wishlist_item_checked(state, &item).await {
                return routing::service_unavailable_response(&error);
            }
            promoted += 1;
        }
        return routing::ok_response(
            serde_json::json!({"artistId": artist, "promoted": promoted}).to_string(),
        );
    }
    if path == "/api/musicbrainz/overlays/edits" {
        let mut edit = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(edit @ serde_json::Value::Object(_)) => edit,
            Ok(_) => return routing::bad_request_response("edit must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if edit.as_object().is_none_or(serde_json::Map::is_empty) {
            return routing::bad_request_response("edit fields are required");
        }
        if is_versioned_v0
            && edit
                .get("evidence")
                .and_then(serde_json::Value::as_array)
                .is_none_or(Vec::is_empty)
        {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: serde_json::json!({
                    "isValid": false,
                    "errors": [
                        "At least one evidence item is required.",
                        "Signature payload hash does not match edit contents.",
                        "Edit type, target, and field combination is not supported."
                    ],
                    "edit": edit,
                })
                .to_string(),
            };
        }
        let id = edit
            .get("editId")
            .or_else(|| edit.get("id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        edit["editId"] = serde_json::json!(id);
        edit["storedAt"] = serde_json::json!(unix_timestamp());
        return match state
            .controller_features
            .upsert(format!("musicbrainz/edit/{id}"), edit.clone())
            .await
        {
            Ok(()) => routing::ok_response(
                serde_json::json!({"isValid": true, "edit": edit, "errors": []}).to_string(),
            ),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if let Some(edit_id) =
        path_segment_between(path, "/api/musicbrainz/overlays/edits/", "/approve-export")
    {
        let edit_id = decoded_path_segment(edit_id);
        let features = state.controller_features.read().await;
        let Some(edit) = features
            .get(&format!("musicbrainz/edit/{edit_id}"))
            .cloned()
        else {
            drop(features);
            return HttpResponse {
                status: "404 Not Found",
                content_type: "application/json",
                body: serde_json::json!({"errors": ["Edit not found."], "decision": null})
                    .to_string(),
            };
        };
        // Matches the oracle's real ApproveExportAsync: approving an
        // already-approved edit is idempotent -- it returns the existing
        // decision unchanged rather than re-validating or overwriting it.
        let existing_decision = features
            .get(&format!("musicbrainz/approval/{edit_id}"))
            .cloned();
        drop(features);
        if let Some(existing_decision) = existing_decision {
            return routing::ok_response(
                serde_json::json!({"errors": [], "decision": existing_decision}).to_string(),
            );
        }

        let request = serde_json::from_str::<serde_json::Value>(body)
            .unwrap_or_else(|_| serde_json::json!({}));
        let approved_by = request
            .get("approvedBy")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("local-user")
            .to_owned();
        let note = request
            .get("note")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .to_owned();

        let mut errors = Vec::new();
        if !is_safe_opaque_reference(&approved_by) {
            errors.push("Approved-by identifier must be opaque and safe.");
        }
        if note.len() > 512 {
            errors.push("Approval note must be 512 characters or fewer.");
        }
        if !musicbrainz_edit_is_exportable(&edit) {
            errors.push("Overlay edit type is not exportable.");
        }
        if !errors.is_empty() {
            return HttpResponse {
                status: "400 Bad Request",
                content_type: "application/json",
                body: serde_json::json!({"errors": errors, "decision": null}).to_string(),
            };
        }

        let decision = serde_json::json!({
            "id": format!("musicbrainz-overlay-export:{}", uuid::Uuid::new_v4().simple()),
            "editId": edit_id,
            "approvedBy": approved_by,
            "note": note,
            "upstreamTarget": musicbrainz_edit_upstream_target(&edit),
            "proposedChange": musicbrainz_edit_proposed_change(&edit),
            "createdAt": unix_timestamp(),
        });
        return match state
            .controller_features
            .upsert(format!("musicbrainz/approval/{edit_id}"), decision.clone())
            .await
        {
            Ok(()) => routing::ok_response(
                serde_json::json!({"errors": [], "decision": decision}).to_string(),
            ),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if let Some(edit_id) = path_segment_between(path, "/api/musicbrainz/overlays/edits/", "/routes")
    {
        let edit_id = decoded_path_segment(edit_id);
        if is_versioned_v0 {
            let edit_exists = state
                .controller_features
                .read()
                .await
                .get(&format!("musicbrainz/edit/{edit_id}"))
                .is_some();
            if !edit_exists {
                let id = format!(
                    "musicbrainz-overlay-route:{}",
                    uuid::Uuid::new_v4().simple()
                );
                let attempt = serde_json::json!({
                    "id": id,
                    "editId": edit_id,
                    "messageId": "",
                    "podId": "musicbrainz-overlay",
                    "channelId": format!("edit:{edit_id}"),
                    "targetPeerIds": [],
                    "routedPeerIds": [],
                    "failedPeerIds": [],
                    "success": false,
                    "errorMessage": "Edit not found.",
                    "createdAt": chrono::Utc::now().to_rfc3339(),
                });
                return match state
                    .controller_features
                    .upsert(
                        format!("musicbrainz/edit-route/{edit_id}/{id}"),
                        attempt.clone(),
                    )
                    .await
                {
                    Ok(()) => HttpResponse {
                        status: "404 Not Found",
                        content_type: "application/json",
                        body: attempt.to_string(),
                    },
                    Err(error) => routing::service_unavailable_response(&error),
                };
            }

            let route_request = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(request)) => request,
                Ok(_) => return routing::bad_request_response("route request must be an object"),
                Err(_) => return routing::bad_request_response("invalid JSON body"),
            };
            if route_request
                .get("targetPeerIds")
                .is_some_and(|peers| json_array_exceeds_limit(peers, MAX_ROUTING_TARGET_PEERS))
            {
                return routing::bad_request_response(
                    "targetPeerIds must contain at most 256 items",
                );
            }
            let mut target_peer_ids = match route_request.get("targetPeerIds") {
                None | Some(serde_json::Value::Null) => Vec::new(),
                Some(serde_json::Value::Array(values)) => values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                Some(_) => return routing::bad_request_response("targetPeerIds must be an array"),
            };
            target_peer_ids.sort_by_key(|value| value.to_ascii_lowercase());
            target_peer_ids.dedup_by(|left, right| left.eq_ignore_ascii_case(right));

            let pod_id = route_request
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .unwrap_or("musicbrainz-overlay")
                .to_owned();
            let channel_id = route_request
                .get("channelId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("edit:{edit_id}"));
            let sender_peer_id = route_request
                .get("senderPeerId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim);
            let error_message = if !safe_radar_reference(&pod_id)
                || !safe_radar_reference(&channel_id)
                || sender_peer_id.is_some_and(|value| !safe_radar_reference(value))
            {
                "Route metadata must be opaque and safe."
            } else if target_peer_ids.is_empty() {
                "At least one target peer is required."
            } else if target_peer_ids
                .iter()
                .any(|value| !safe_radar_reference(value))
            {
                "Route targets must be opaque and safe."
            } else {
                "Routing backend is not available."
            };
            let id = format!(
                "musicbrainz-overlay-route:{}",
                uuid::Uuid::new_v4().simple()
            );
            let attempt = serde_json::json!({
                "id": id,
                "editId": edit_id,
                "messageId": "",
                "podId": pod_id,
                "channelId": channel_id,
                "targetPeerIds": target_peer_ids,
                "routedPeerIds": [],
                "failedPeerIds": [],
                "success": false,
                "errorMessage": error_message,
                "createdAt": chrono::Utc::now().to_rfc3339(),
            });
            return match state
                .controller_features
                .upsert(
                    format!("musicbrainz/edit-route/{edit_id}/{id}"),
                    attempt.clone(),
                )
                .await
            {
                Ok(()) => HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: attempt.to_string(),
                },
                Err(error) => routing::service_unavailable_response(&error),
            };
        }
        if state
            .controller_features
            .read()
            .await
            .get(&format!("musicbrainz/edit/{edit_id}"))
            .is_none()
        {
            return routing::not_found_response();
        }
        let id = uuid::Uuid::new_v4().to_string();
        let route = serde_json::json!({
            "id": id, "editId": edit_id, "success": true, "routedAt": unix_timestamp()
        });
        return match state
            .controller_features
            .upsert(
                format!("musicbrainz/edit-route/{edit_id}/{id}"),
                route.clone(),
            )
            .await
        {
            Ok(()) => routing::ok_response(route.to_string()),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if path == "/api/musicbrainz/release-radar/observations" {
        if is_versioned_v0 {
            let observation = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(observation)) => observation,
                Ok(_) => return routing::bad_request_response("observation must be an object"),
                Err(_) => return routing::bad_request_response("invalid JSON body"),
            };
            if observation
                .get("workRef")
                .and_then(serde_json::Value::as_object)
                .and_then(|work_ref| work_ref.get("@context"))
                .is_some_and(serde_json::Value::is_null)
            {
                return HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/problem+json",
                    body: serde_json::json!({
                        "title": "One or more validation errors occurred.",
                        "status": 400,
                        "detail": "The request is invalid.",
                        "errors": {},
                    })
                    .to_string(),
                };
            }
            let work_ref = match radar_work_ref(observation.get("workRef")) {
                Ok(work_ref) => work_ref,
                Err(error) => return routing::bad_request_response(&error),
            };
            let artist_id = observation
                .get("artistId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            let recording_id = observation
                .get("recordingId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            let confidence = observation
                .get("confidence")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let rejection_reason = if !observation
                .get("songIdConfirmed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                Some("Observation is not SongID-confirmed.")
            } else if confidence < 0.8 {
                Some("Observation confidence is below the release radar threshold.")
            } else if artist_id.is_empty() {
                Some("Artist MBID is required.")
            } else if recording_id.is_empty() {
                Some("Recording MBID is required.")
            } else if !work_ref["domain"]
                .as_str()
                .is_some_and(|domain| domain.eq_ignore_ascii_case("music"))
            {
                Some("WorkRef domain must be music.")
            } else if work_ref["title"]
                .as_str()
                .is_none_or(|title| title.trim().is_empty())
            {
                Some("WorkRef title is required.")
            } else {
                None
            };
            if let Some(reason) = rejection_reason {
                return HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: serde_json::json!({
                        "accepted": false,
                        "notifications": [],
                        "rejectionReason": reason,
                    })
                    .to_string(),
                };
            }

            let source_realm = observation
                .get("sourceRealm")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            let source_actor = observation
                .get("sourceActor")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            let release_id = observation
                .get("releaseId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim);
            let release_group_id = observation
                .get("releaseGroupId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim);
            let first_seen_at = match normalized_radar_timestamp(
                observation
                    .get("observedAt")
                    .and_then(serde_json::Value::as_str),
            ) {
                Ok(timestamp) => timestamp,
                Err(error) => return routing::bad_request_response(&error),
            };
            let features = state.controller_features.read().await;
            let subscriptions = features.values_with_prefix("musicbrainz/radar/subscription/");
            let existing_notifications =
                features.values_with_prefix("musicbrainz/radar/notification/");
            drop(features);
            let mut notifications = Vec::new();
            for subscription in subscriptions {
                let subscription_id = subscription["id"].as_str().unwrap_or_default();
                let scope = subscription["scope"].as_str().unwrap_or("trusted");
                let scope_matches = scope.eq_ignore_ascii_case("trusted")
                    || scope
                        .strip_prefix("realm:")
                        .is_some_and(|realm| realm.eq_ignore_ascii_case(source_realm))
                    || scope
                        .strip_prefix("actor:")
                        .is_some_and(|actor| actor.eq_ignore_ascii_case(source_actor));
                let muted = subscription["mutedReleaseGroupIds"]
                    .as_array()
                    .is_some_and(|values| {
                        release_group_id.is_some_and(|release_group_id| {
                            values.iter().any(|value| {
                                value.as_str().is_some_and(|value| {
                                    value.eq_ignore_ascii_case(release_group_id)
                                })
                            })
                        })
                    });
                let duplicate = existing_notifications.iter().any(|notification| {
                    notification["subscriptionId"].as_str() == Some(subscription_id)
                        && notification["artistId"]
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(artist_id))
                        && notification["recordingId"]
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(recording_id))
                        && notification["sourceRealm"]
                            .as_str()
                            .is_some_and(|value| value.eq_ignore_ascii_case(source_realm))
                });
                if !subscription["enabled"].as_bool().unwrap_or(true)
                    || !subscription["artistId"]
                        .as_str()
                        .is_some_and(|value| value.eq_ignore_ascii_case(artist_id))
                    || !scope_matches
                    || muted
                    || duplicate
                {
                    continue;
                }
                let id = format!(
                    "artist-radar-notification:{}",
                    uuid::Uuid::new_v4().simple()
                );
                let notification = serde_json::json!({
                    "id": id,
                    "subscriptionId": subscription_id,
                    "artistId": artist_id,
                    "recordingId": recording_id,
                    "releaseId": release_id,
                    "releaseGroupId": release_group_id,
                    "sourceRealm": source_realm,
                    "sourceActor": source_actor,
                    "confidence": confidence,
                    "workRef": work_ref,
                    "firstSeenAt": first_seen_at,
                    "read": false,
                });
                if let Err(error) = state
                    .controller_features
                    .upsert(
                        format!("musicbrainz/radar/notification/{id}"),
                        notification.clone(),
                    )
                    .await
                {
                    return routing::service_unavailable_response(&error);
                }
                notifications.push(notification);
            }
            return routing::ok_response(
                serde_json::json!({
                    "accepted": true,
                    "notifications": notifications,
                })
                .to_string(),
            );
        }
        let mut observation = match serde_json::from_str::<serde_json::Value>(body) {
            Ok(observation @ serde_json::Value::Object(_)) => observation,
            Ok(_) => return routing::bad_request_response("observation must be an object"),
            Err(_) => return routing::bad_request_response("invalid JSON body"),
        };
        if observation
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        {
            return routing::bad_request_response("observation fields are required");
        }
        let id = uuid::Uuid::new_v4().to_string();
        observation["observationId"] = serde_json::json!(id);
        observation["observedAt"] = serde_json::json!(unix_timestamp());
        return match state
            .controller_features
            .upsert(format!("musicbrainz/observation/{id}"), observation.clone())
            .await
        {
            Ok(()) => routing::created_response(observation.to_string()),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    if let Some(notification_id) = path_segment_between(
        path,
        "/api/musicbrainz/release-radar/notifications/",
        "/routes",
    ) {
        let notification_id = decoded_path_segment(notification_id);
        if is_versioned_v0 {
            let route_request = match serde_json::from_str::<serde_json::Value>(body) {
                Ok(serde_json::Value::Object(request)) => request,
                Ok(_) => return routing::bad_request_response("route request must be an object"),
                Err(_) => return routing::bad_request_response("invalid JSON body"),
            };
            if route_request
                .get("targetPeerIds")
                .is_some_and(|peers| json_array_exceeds_limit(peers, MAX_ROUTING_TARGET_PEERS))
            {
                return routing::bad_request_response(
                    "targetPeerIds must contain at most 256 items",
                );
            }
            let mut target_peer_ids = match route_request.get("targetPeerIds") {
                None | Some(serde_json::Value::Null) => Vec::new(),
                Some(serde_json::Value::Array(values)) => values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                Some(_) => return routing::bad_request_response("targetPeerIds must be an array"),
            };
            target_peer_ids.sort_by_key(|value| value.to_ascii_lowercase());
            target_peer_ids.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
            let pod_id = route_request
                .get("podId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .unwrap_or("artist-release-radar");
            let channel_id = route_request
                .get("channelId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("notification:{notification_id}"));
            let sender_peer_id = route_request
                .get("senderPeerId")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(str::trim);
            let notification_exists = state
                .controller_features
                .read()
                .await
                .get(&format!("musicbrainz/radar/notification/{notification_id}"))
                .is_some();
            let error_message = if !notification_exists {
                "Notification not found."
            } else if !safe_radar_reference(pod_id)
                || !safe_radar_reference(&channel_id)
                || sender_peer_id.is_some_and(|value| !safe_radar_reference(value))
            {
                "Route metadata must be opaque and safe."
            } else if target_peer_ids.is_empty() {
                "At least one target peer is required."
            } else if target_peer_ids
                .iter()
                .any(|value| !safe_radar_reference(value))
            {
                "Route targets must be opaque and safe."
            } else {
                "Routing backend is not available."
            };
            let id = format!("artist-radar-route:{}", uuid::Uuid::new_v4().simple());
            let attempt = serde_json::json!({
                "id": id,
                "notificationId": notification_id,
                "messageId": "",
                "podId": pod_id,
                "channelId": channel_id,
                "targetPeerIds": target_peer_ids,
                "routedPeerIds": [],
                "failedPeerIds": [],
                "success": false,
                "errorMessage": error_message,
                "createdAt": chrono::Utc::now().to_rfc3339(),
            });
            return match state
                .controller_features
                .upsert(
                    format!("musicbrainz/notification-route/{notification_id}/{id}"),
                    attempt.clone(),
                )
                .await
            {
                Ok(()) => HttpResponse {
                    status: "400 Bad Request",
                    content_type: "application/json",
                    body: attempt.to_string(),
                },
                Err(error) => routing::service_unavailable_response(&error),
            };
        }
        let id = uuid::Uuid::new_v4().to_string();
        let route = serde_json::json!({
            "id": id, "notificationId": notification_id, "success": true, "routedAt": unix_timestamp()
        });
        return match state
            .controller_features
            .upsert(
                format!("musicbrainz/notification-route/{notification_id}/{id}"),
                route.clone(),
            )
            .await
        {
            Ok(()) => routing::ok_response(route.to_string()),
            Err(error) => routing::service_unavailable_response(&error),
        };
    }
    routing::not_found_response()
}

/// Matches the oracle's real `TasteRecommendationService.BuildWishlistFilter`.
pub(super) fn work_ref_wishlist_filter(work_ref: &serde_json::Value, note: Option<&str>) -> String {
    let mut metadata = vec![
        "source:taste-recommendation".to_owned(),
        "review-only".to_owned(),
    ];
    if let Some(mbid) = work_ref
        .get("externalIds")
        .and_then(|value| value.get("musicbrainz"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        metadata.push(format!("mbid:{mbid}"));
    }
    if let Some(artist_mbid) = work_ref
        .get("externalIds")
        .and_then(|value| value.get("musicbrainz_artist"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        metadata.push(format!("artist-mbid:{artist_mbid}"));
    }
    if let Some(note) = note.map(str::trim).filter(|value| !value.is_empty()) {
        metadata.push(format!("note:{note}"));
    }
    metadata.join("; ")
}

pub(super) fn radar_subscription_from_body(body: &str) -> Result<serde_json::Value, String> {
    let value = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|_| "invalid JSON body".to_owned())?;
    let object = value
        .as_object()
        .ok_or_else(|| "subscription must be an object".to_owned())?;
    let artist_id = object
        .get("artistId")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    if artist_id.is_empty() {
        return Err("artistId is required".to_owned());
    }
    let id = object
        .get("id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| format!("artist-radar:{}", artist_id.to_ascii_lowercase()));
    let artist_name = object
        .get("artistName")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let scope = object
        .get("scope")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::trim)
        .unwrap_or("trusted");
    let muted_release_group_ids = match object.get("mutedReleaseGroupIds") {
        None | Some(serde_json::Value::Null) => Vec::new(),
        Some(serde_json::Value::Array(values)) => {
            if values.len() > MAX_RADAR_MUTED_RELEASE_GROUPS {
                return Err(format!(
                    "mutedReleaseGroupIds must contain at most {MAX_RADAR_MUTED_RELEASE_GROUPS} items"
                ));
            }
            values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "mutedReleaseGroupIds must contain strings".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        Some(_) => return Err("mutedReleaseGroupIds must be an array".to_owned()),
    };
    let created_at =
        normalized_radar_timestamp(object.get("createdAt").and_then(serde_json::Value::as_str))?;
    Ok(serde_json::json!({
        "id": id,
        "artistId": artist_id,
        "artistName": artist_name,
        "scope": scope,
        "enabled": object.get("enabled").and_then(serde_json::Value::as_bool).unwrap_or(true),
        "mutedReleaseGroupIds": muted_release_group_ids,
        "createdAt": created_at,
    }))
}

fn radar_work_ref(value: Option<&serde_json::Value>) -> Result<serde_json::Value, String> {
    let mut work_ref = value
        .and_then(serde_json::Value::as_object)
        .cloned()
        .unwrap_or_default();
    work_ref.entry("@context").or_insert_with(|| {
        serde_json::json!([
            "https://www.w3.org/ns/activitystreams",
            "https://w3id.org/federation/workref#"
        ])
    });
    work_ref
        .entry("id")
        .or_insert_with(|| serde_json::json!(""));
    work_ref
        .entry("type")
        .or_insert_with(|| serde_json::json!("WorkRef"));
    work_ref
        .entry("domain")
        .or_insert_with(|| serde_json::json!(""));
    work_ref
        .entry("externalIds")
        .or_insert_with(|| serde_json::json!({}));
    work_ref
        .entry("title")
        .or_insert_with(|| serde_json::json!(""));
    work_ref.entry("creator").or_insert(serde_json::Value::Null);
    work_ref.entry("year").or_insert(serde_json::Value::Null);
    work_ref
        .entry("metadata")
        .or_insert_with(|| serde_json::json!({}));
    work_ref
        .entry("attributedTo")
        .or_insert(serde_json::Value::Null);
    if let Some(published) = work_ref
        .get("published")
        .and_then(serde_json::Value::as_str)
    {
        work_ref.insert(
            "published".to_owned(),
            serde_json::json!(normalized_radar_timestamp(Some(published))?),
        );
    } else {
        work_ref
            .entry("published")
            .or_insert(serde_json::Value::Null);
    }
    Ok(serde_json::Value::Object(work_ref))
}

/// Matches the oracle's real `MusicBrainzOverlayService.IsExportableEdit`.
const MUSICBRAINZ_EXPORTABLE_EDIT_TYPES: [&str; 7] = [
    "TitleCorrection",
    "ArtistCorrection",
    "Alias",
    "MissingAltTitle",
    "DuplicateMarker",
    "ReleaseGrouping",
    "RecordingLinkage",
];

fn musicbrainz_edit_is_exportable(edit: &serde_json::Value) -> bool {
    edit.get("type")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| MUSICBRAINZ_EXPORTABLE_EDIT_TYPES.contains(&value))
}

fn musicbrainz_edit_upstream_target(edit: &serde_json::Value) -> String {
    format!(
        "{}:{}",
        edit.get("targetType")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default(),
        edit.get("targetId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default(),
    )
}

fn musicbrainz_edit_proposed_change(edit: &serde_json::Value) -> String {
    format!(
        "{} => {}",
        edit.get("field")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim(),
        edit.get("value")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim(),
    )
}

/// Matches the oracle's real `MusicBrainzOverlayService.BuildExportReview`,
/// used both by `GET .../export-review` directly and by
/// `POST .../approve-export` to decide `CanApproveExport`/`ReviewReason`.
fn musicbrainz_export_review_json(
    edit: &serde_json::Value,
    decision: Option<&serde_json::Value>,
) -> serde_json::Value {
    let exportable = musicbrainz_edit_is_exportable(edit);
    let review_reason = if decision.is_some() {
        "Upstream export has already been approved locally."
    } else if exportable {
        "Overlay edit can be reviewed for manual upstream MusicBrainz submission."
    } else {
        "Overlay edit type is not exportable."
    };
    serde_json::json!({
        "edit": edit,
        "upstreamTarget": musicbrainz_edit_upstream_target(edit),
        "proposedChange": musicbrainz_edit_proposed_change(edit),
        "evidence": edit.get("evidence").cloned().unwrap_or_else(|| serde_json::json!([])),
        "canApproveExport": decision.is_none() && exportable,
        "reviewReason": review_reason,
        "decision": decision,
    })
}

pub(super) async fn musicbrainz_dynamic_get_response(path: &str, state: &AppState) -> HttpResponse {
    if let Some(artist_id) =
        path_segment_between(path, "/api/musicbrainz/artist/", "/release-graph")
    {
        let artist_id = decoded_path_segment(artist_id);
        let library = state.library.read().await;
        let records = library
            .records
            .iter()
            .filter(|item| item.artist.eq_ignore_ascii_case(&artist_id))
            .collect::<Vec<_>>();
        if records.is_empty() {
            return routing::not_found_response();
        }
        let release_groups = records
            .iter()
            .map(|item| {
                let release_type = match item.kind.to_ascii_lowercase().as_str() {
                    "album" => "Album",
                    "single" => "Single",
                    "ep" => "EP",
                    "compilation" => "Compilation",
                    "soundtrack" => "Soundtrack",
                    "live" => "Live",
                    "remix" => "Remix",
                    _ => "Other",
                };
                serde_json::json!({
                    "releaseGroupId": item.id,
                    "title": item.title,
                    "type": release_type,
                    "firstReleaseDate": "",
                    "releases": [{
                        "releaseId": item.id,
                        "title": item.title,
                        "country": "",
                        "status": "",
                        "releaseDate": "",
                    }],
                })
            })
            .collect::<Vec<_>>();
        drop(library);
        let now = chrono::Utc::now();
        return routing::ok_response(
            serde_json::json!({
                "artistId": artist_id,
                "name": artist_id,
                "sortName": artist_id,
                "releaseGroups": release_groups,
                "cachedAt": now.to_rfc3339(),
                "expiresAt": (now + chrono::Duration::days(7)).to_rfc3339(),
            })
            .to_string(),
        );
    }
    if let Some(artist_id) =
        path_segment_between(path, "/api/musicbrainz/overlays/artist/", "/release-graph")
    {
        let artist_id = decoded_path_segment(artist_id);
        let library = state.library.read().await;
        let releases = library
            .records
            .iter()
            .filter(|item| item.artist.eq_ignore_ascii_case(&artist_id))
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "title": item.title,
                    "kind": item.kind,
                })
            })
            .collect::<Vec<_>>();
        return routing::ok_response(
            serde_json::json!({
                "artistId": artist_id,
                "releases": releases,
                "edges": [],
            })
            .to_string(),
        );
    }
    if let Some(edit_id) =
        path_segment_between(path, "/api/musicbrainz/overlays/edits/", "/export-review")
    {
        let edit_id = decoded_path_segment(edit_id);
        let features = state.controller_features.read().await;
        let Some(edit) = features
            .get(&format!("musicbrainz/edit/{edit_id}"))
            .cloned()
        else {
            return routing::not_found_response();
        };
        let decision = features
            .get(&format!("musicbrainz/approval/{edit_id}"))
            .cloned();
        drop(features);
        // Matches the oracle's real GetExportReview: the same
        // BuildExportReview shape the approve-export flow validates
        // against (upstreamTarget/proposedChange/evidence/
        // canApproveExport/reviewReason/decision), not an invented
        // {editId, edit, approval, readyForExport} wrapper.
        return routing::ok_response(
            musicbrainz_export_review_json(&edit, decision.as_ref()).to_string(),
        );
    }
    if let Some(edit_id) = path_segment_between(path, "/api/musicbrainz/overlays/edits/", "/routes")
    {
        let edit_id = decoded_path_segment(edit_id);
        let routes = state
            .controller_features
            .read()
            .await
            .values_with_prefix(&format!("musicbrainz/edit-route/{edit_id}/"));
        return routing::ok_response(serde_json::Value::Array(routes).to_string());
    }
    if let Some(notification_id) = path_segment_between(
        path,
        "/api/musicbrainz/release-radar/notifications/",
        "/routes",
    ) {
        let notification_id = decoded_path_segment(notification_id);
        let routes = state
            .controller_features
            .read()
            .await
            .values_with_prefix(&format!(
                "musicbrainz/notification-route/{notification_id}/"
            ));
        return routing::ok_response(serde_json::Value::Array(routes).to_string());
    }
    routing::not_found_response()
}
