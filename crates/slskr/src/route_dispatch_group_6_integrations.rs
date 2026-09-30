async fn route_dispatch_group_6_integrations(
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
                        object.insert(
                            "missing_albums".to_owned(),
                            serde_json::json!(missing_albums),
                        );
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
                })
                .await
                {
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
            })
            .await
            {
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
            let result =
                match run_lidarr_import_with_history(state, &lidarr, directory, Some(history_id))
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
                let mut result =
                    match run_lidarr_import_with_history(state, &lidarr, &directory, None).await {
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
                        (!directory.trim().is_empty()).then(|| {
                            directory
                                .rsplit('/')
                                .next()
                                .unwrap_or(&directory)
                                .to_owned()
                        })
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
                    return Ok(routing::service_unavailable_response(
                        "library item capacity is full",
                    ));
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
                    let persistence_result = db
                        .create_library_item_and_record_manual_import(
                            &persisted_library_item(&record),
                            runtime_record,
                        )
                        .await
                        .map_err(|error| error.to_string());
                    if let Err(error_message) = persistence_result {
                        rollback_library_runtime_if_unchanged(
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
                let mut value = runtime.record_lidarr_manual_import(0, true, directory, Vec::new());
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

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
