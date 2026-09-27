use super::*;

pub(super) async fn virtual_soulfind_catalogue(
    state: &AppState,
) -> Vec<virtual_soulfind_v2::CatalogueItem> {
    let shares = state.shares.read().await;
    let mut catalogue = shares
        .entries
        .iter()
        .filter(|entry| matches!(library_media_kind(&entry.extension), "Audio"))
        .map(|entry| {
            let (artist, title) = virtual_soulfind_file_metadata(&entry.filename);
            let local_path = shares
                .local_paths
                .get(&entry.filename)
                .map(|path| path.display().to_string());
            let source_id = format!(
                "share:{}",
                hex::encode(Sha256::digest(
                    format!("{}|{}", entry.filename, entry.size).as_bytes(),
                ))
            );
            virtual_soulfind_v2::CatalogueItem {
                source_id,
                artist,
                title,
                kind: "Album".to_owned(),
                created_at: unix_timestamp(),
                local_path,
                size: entry.size,
            }
        })
        .collect::<Vec<_>>();
    drop(shares);

    // Library metadata records are also local catalogue sources. They may not
    // have a share-backed path, but they still provide the stable artist,
    // release, and track identities required by the bounded planning API.
    let library = state.library.read().await;
    catalogue.extend(
        library
            .records
            .iter()
            .map(|record| virtual_soulfind_v2::CatalogueItem {
                source_id: format!("library:{}", record.id),
                artist: record.artist.clone(),
                title: record.title.clone(),
                kind: record.kind.clone(),
                created_at: record.created_at,
                local_path: None,
                size: 0,
            }),
    );
    catalogue
}

fn virtual_soulfind_file_metadata(filename: &str) -> (String, String) {
    let relative = filename.split_once('/').map_or(filename, |(_, path)| path);
    let components = relative
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let basename = components.last().copied().unwrap_or(relative);
    let title = basename
        .rsplit_once('.')
        .map_or(basename, |(stem, _)| stem)
        .trim();
    let (artist, title) = title.split_once(" - ").map_or_else(
        || {
            (
                components
                    .get(components.len().saturating_sub(2))
                    .copied()
                    .unwrap_or("Unknown Artist"),
                title,
            )
        },
        |(artist, title)| (artist.trim(), title.trim()),
    );
    (
        if artist.is_empty() {
            "Unknown Artist".to_owned()
        } else {
            artist.to_owned()
        },
        if title.is_empty() {
            "Unknown Track".to_owned()
        } else {
            title.to_owned()
        },
    )
}

pub(super) async fn route_virtual_soulfind_v2(
    method: &str,
    path: &str,
    query: Option<&str>,
    body: &str,
    state: &AppState,
) -> HttpResponse {
    if state.config.controller_profile == ControllerProfile::Legacy {
        return routing::not_found_response();
    }
    if !state.config.virtual_soulfind_v2_enabled {
        return routing::service_unavailable_string_response("VirtualSoulfind v2 is disabled");
    }
    let Some(suffix) = path.strip_prefix("/api/virtualsoulfind/v2") else {
        return routing::not_found_response();
    };
    let value = || {
        serde_json::from_str::<serde_json::Value>(body)
            .map_err(|_| routing::bad_request_response("invalid request body"))
    };

    // ASP.NET binds a present-but-blank route parameter and the controller
    // returns BadRequest after trimming it.  The generic route helpers reject
    // that shape as an unmatched route, which would incorrectly turn the
    // frozen 400 contract into 404.  Keep this correction local to the
    // VirtualSoulfind v2 dispatcher.
    if [
        ("/intents/tracks/", ""),
        ("/intents/releases/", ""),
        ("/catalogue/artists/", ""),
        ("/catalogue/releases/", ""),
        ("/executions/", ""),
    ]
    .iter()
    .any(|(prefix, _)| virtual_soulfind_blank_single_segment(suffix, prefix))
        || virtual_soulfind_blank_action_segment(suffix, "/intents/tracks/", "/process")
        || virtual_soulfind_blank_action_segment(suffix, "/catalogue/artists/", "/releases")
        || virtual_soulfind_blank_action_segment(suffix, "/catalogue/releases/", "/tracks")
    {
        return routing::bad_request_response("");
    }

    match (method, suffix) {
        ("POST", "/intents/tracks") => {
            let request = match value() {
                Ok(value) => value,
                Err(response) => return response,
            };
            let track_id = request
                .get("trackId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let domain = request
                .get("domain")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("Music"));
            let priority = request
                .get("priority")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("Normal"));
            let parent = request
                .get("parentDesiredReleaseId")
                .and_then(serde_json::Value::as_str);
            match state
                .virtual_soulfind_v2
                .write()
                .await
                .enqueue_track(&domain, track_id, &priority, parent)
            {
                Ok(intent) => routing::created_response(intent.to_string()),
                Err(error) if error.contains("capacity") => {
                    routing::service_unavailable_response(&error)
                }
                Err(error) => routing::bad_request_response(&error),
            }
        }
        ("POST", "/intents/releases") => {
            let request = match value() {
                Ok(value) => value,
                Err(response) => return response,
            };
            let release_id = request
                .get("releaseId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let priority = request
                .get("priority")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("Normal"));
            let mode = request
                .get("mode")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("Wanted"));
            let notes = request.get("notes").and_then(serde_json::Value::as_str);
            match state
                .virtual_soulfind_v2
                .write()
                .await
                .enqueue_release(release_id, &priority, &mode, notes)
            {
                Ok(intent) => routing::created_response(intent.to_string()),
                Err(error) if error.contains("capacity") => {
                    routing::service_unavailable_response(&error)
                }
                Err(error) => routing::bad_request_response(&error),
            }
        }
        ("GET", "/intents/tracks/pending") => {
            let limit = match query_bounded_usize(query, "limit", 0, 1_024) {
                Ok(limit) => limit.unwrap_or(100),
                Err(()) => return routing::bad_request_response("limit is invalid"),
            };
            routing::ok_response(
                state
                    .virtual_soulfind_v2
                    .read()
                    .await
                    .pending_tracks(limit)
                    .to_string(),
            )
        }
        ("GET", "/stats") => {
            routing::ok_response(state.virtual_soulfind_v2.read().await.stats().to_string())
        }
        ("GET", "/catalogue/artists/search") => {
            let Some(query_text) = query_parameter(query, "query")
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
            else {
                return routing::bad_request_response("Query is required");
            };
            if query_text.len() > 512 || query_text.chars().any(char::is_control) {
                return routing::bad_request_response("Query is invalid");
            }
            let limit = match query_bounded_usize(query, "limit", 0, 100) {
                Ok(limit) => limit.unwrap_or(50),
                Err(()) => return routing::bad_request_response("limit is invalid"),
            };
            routing::ok_response(
                virtual_soulfind_v2::search_artists(
                    &virtual_soulfind_catalogue(state).await,
                    &query_text,
                    limit,
                )
                .to_string(),
            )
        }
        ("POST", "/plans") => {
            let request = match value() {
                Ok(value) => value,
                Err(response) => return response,
            };
            let track_id = request
                .get("trackId")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let domain = request
                .get("domain")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("Music"));
            let mode = request
                .get("mode")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("SoulseekFriendly"));
            let priority = request
                .get("priority")
                .cloned()
                .unwrap_or_else(|| serde_json::json!("Normal"));
            match virtual_soulfind_v2::create_plan(
                &virtual_soulfind_catalogue(state).await,
                &domain,
                track_id,
                &mode,
                &priority,
            ) {
                Ok(plan) => routing::ok_response(plan.to_string()),
                Err(error) => routing::bad_request_response(&error),
            }
        }
        _ => {
            if let Some(id) = single_route_segment(suffix, "/intents/tracks/") {
                return match method {
                    "GET" => state
                        .virtual_soulfind_v2
                        .read()
                        .await
                        .track(&decoded_path_segment(id))
                        .map_or_else(routing::not_found_response, |intent| {
                            routing::ok_response(intent.to_string())
                        }),
                    "PATCH" => {
                        let request = match value() {
                            Ok(value) => value,
                            Err(response) => return response,
                        };
                        let status = request
                            .get("status")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        match state
                            .virtual_soulfind_v2
                            .write()
                            .await
                            .update_track_status(&decoded_path_segment(id), &status)
                        {
                            Ok(true) => routing::no_content_response(),
                            Ok(false) => routing::not_found_response(),
                            Err(error) => routing::bad_request_response(&error),
                        }
                    }
                    _ => routing::not_found_response(),
                };
            }
            if let Some(id) = single_route_segment(suffix, "/intents/releases/") {
                return if method == "GET" {
                    state
                        .virtual_soulfind_v2
                        .read()
                        .await
                        .release(&decoded_path_segment(id))
                        .map_or_else(routing::not_found_response, |intent| {
                            routing::ok_response(intent.to_string())
                        })
                } else {
                    routing::not_found_response()
                };
            }
            if let Some(id) = action_route_segment(suffix, "/intents/tracks/", "/process") {
                if method != "POST" {
                    return routing::not_found_response();
                }
                let id = decoded_path_segment(id);
                if state.virtual_soulfind_v2.read().await.track(&id).is_none() {
                    return routing::not_found_response();
                }
                let catalogue = virtual_soulfind_catalogue(state).await;
                let intents = Arc::clone(&state.virtual_soulfind_v2);
                let process_id = id.clone();
                tokio::spawn(async move {
                    intents.write().await.process_track(&process_id, &catalogue);
                });
                return routing::accepted_response(
                    serde_json::json!({
                        "message": "Processing started",
                        "intentId": id,
                    })
                    .to_string(),
                );
            }
            if let Some(id) = single_route_segment(suffix, "/catalogue/artists/") {
                if method != "GET" {
                    return routing::not_found_response();
                }
                return virtual_soulfind_v2::artist(
                    &virtual_soulfind_catalogue(state).await,
                    &decoded_path_segment(id),
                )
                .map_or_else(routing::not_found_response, |artist| {
                    routing::ok_response(artist.to_string())
                });
            }
            if let Some(id) = action_route_segment(suffix, "/catalogue/artists/", "/releases") {
                if method != "GET" {
                    return routing::not_found_response();
                }
                let limit = match query_bounded_usize(query, "limit", 0, 100) {
                    Ok(limit) => limit.unwrap_or(100),
                    Err(()) => return routing::bad_request_response("limit is invalid"),
                };
                return routing::ok_response(
                    virtual_soulfind_v2::artist_releases(
                        &virtual_soulfind_catalogue(state).await,
                        &decoded_path_segment(id),
                        limit,
                    )
                    .to_string(),
                );
            }
            if let Some(id) = action_route_segment(suffix, "/catalogue/releases/", "/tracks") {
                if method != "GET" {
                    return routing::not_found_response();
                }
                return routing::ok_response(
                    virtual_soulfind_v2::release_tracks(
                        &virtual_soulfind_catalogue(state).await,
                        &decoded_path_segment(id),
                    )
                    .to_string(),
                );
            }
            if let Some(id) = single_route_segment(suffix, "/executions/") {
                return if method == "GET" {
                    state
                        .virtual_soulfind_v2
                        .read()
                        .await
                        .execution(&decoded_path_segment(id))
                        .map_or_else(routing::not_found_response, |execution| {
                            routing::ok_response(execution.to_string())
                        })
                } else {
                    routing::not_found_response()
                };
            }
            routing::not_found_response()
        }
    }
}

fn single_route_segment<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    let segment = path.strip_prefix(prefix)?;
    (!segment.is_empty() && !segment.contains('/')).then_some(segment)
}

fn action_route_segment<'a>(path: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let segment = path.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!segment.is_empty() && !segment.contains('/')).then_some(segment)
}

fn virtual_soulfind_blank_single_segment(path: &str, prefix: &str) -> bool {
    let Some(segment) = path.strip_prefix(prefix) else {
        return false;
    };
    if segment.contains('/') {
        return false;
    }
    segment.is_empty() || decoded_path_segment(segment).trim().is_empty()
}

fn virtual_soulfind_blank_action_segment(path: &str, prefix: &str, suffix: &str) -> bool {
    let Some(segment) = path
        .strip_prefix(prefix)
        .and_then(|path| path.strip_suffix(suffix))
    else {
        return false;
    };
    segment.is_empty()
        || (!segment.contains('/') && decoded_path_segment(segment).trim().is_empty())
}
