use super::*;

pub(super) fn relay_versioned_route_allowed(
    settings: &config::RelaySettings,
    method: &str,
    path: &str,
) -> bool {
    if !relay_versioned_route_known(method, path) {
        return false;
    }
    if !settings.enabled {
        return false;
    }
    let mode = settings.mode.as_str();
    match method {
        "PUT" | "DELETE" => path == "/api/v0/relay/agent" && matches!(mode, "agent" | "debug"),
        "GET" => {
            (path.starts_with("/api/v0/relay/controller/downloads/")
                || path.starts_with("/api/v0/relay/streams/"))
                && matches!(mode, "controller" | "debug")
        }
        "POST" => {
            (path.starts_with("/api/v0/relay/controller/files/")
                || path.starts_with("/api/v0/relay/controller/shares/"))
                && matches!(mode, "controller" | "debug")
        }
        _ => false,
    }
}

pub(super) fn relay_versioned_route_known(method: &str, path: &str) -> bool {
    let has_one_segment = |prefix: &str| {
        path.strip_prefix(prefix)
            .is_some_and(|value| !value.is_empty() && !value.contains('/'))
    };
    match method {
        "PUT" | "DELETE" => path == "/api/v0/relay/agent",
        "GET" => {
            has_one_segment("/api/v0/relay/controller/downloads/")
                || has_one_segment("/api/v0/relay/streams/")
        }
        "POST" => {
            has_one_segment("/api/v0/relay/controller/files/")
                || has_one_segment("/api/v0/relay/controller/shares/")
        }
        _ => false,
    }
}

fn relay_multipart_boundary(content_type: Option<&str>) -> Option<String> {
    let content_type = content_type?.trim();
    let mut parts = content_type.split(';');
    let media_type = parts.next()?.trim();
    if !media_type.eq_ignore_ascii_case("multipart/form-data") {
        return None;
    }
    parts.find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case("boundary") {
            return None;
        }
        let value = value.trim().trim_matches('"');
        (!value.is_empty()).then(|| value.to_owned())
    })
}

pub(super) async fn versioned_relay_request(
    method: &str,
    path: &str,
    body: &str,
    headers: &RequestSecurityHeaders,
    state: &AppState,
) -> Option<HttpResponse> {
    versioned_relay_request_bytes(method, path, body.as_bytes(), headers, state).await
}

pub(super) async fn versioned_relay_request_bytes(
    method: &str,
    path: &str,
    body: &[u8],
    headers: &RequestSecurityHeaders,
    state: &AppState,
) -> Option<HttpResponse> {
    if !path.starts_with("/api/v0/relay/") {
        return None;
    }

    if !relay_versioned_route_known(method, path) {
        return None;
    }

    let settings = state.advanced_networking.read().await.relay.clone();
    if !relay_versioned_route_allowed(&settings, method, path) {
        return Some(routing::forbidden_response(
            "feature is disabled by configuration",
        ));
    }

    if path == "/api/v0/relay/agent" && method == "PUT" {
        let result = mutate_runtime_compat_state(state, |runtime, relay| {
            relay.set_enabled(true);
            runtime.set_relay_agent(true).to_string()
        })
        .await;
        return Some(match result {
            Ok(_) => HttpResponse {
                status: "200 OK",
                content_type: "",
                body: String::new(),
            },
            Err(error) => routing::service_unavailable_response(&error),
        });
    }

    if path == "/api/v0/relay/agent" && method == "DELETE" {
        let result = mutate_runtime_compat_state(state, |runtime, _| {
            runtime.set_relay_agent(false).to_string()
        })
        .await;
        return Some(match result {
            Ok(_) => routing::no_content_response(),
            Err(error) => routing::service_unavailable_response(&error),
        });
    }

    if method == "GET" && path.starts_with("/api/v0/relay/controller/downloads/") {
        let token = path_segment_after(path, "/api/v0/relay/controller/downloads/")
            .map(decoded_path_segment)
            .unwrap_or_default();
        let Ok(token) = uuid::Uuid::parse_str(token.trim()) else {
            return Some(routing::bad_request_response(
                "Token is not in a valid format",
            ));
        };
        let Some(credential) = headers
            .x_relay_credential
            .as_deref()
            .map(str::trim)
            .filter(|credential| !credential.is_empty())
        else {
            return Some(routing::unauthorized_response());
        };
        let authorized = state.relay.write().await.protocol.validate_download(
            &settings,
            relay::credential_scheme(state.config.controller_profile),
            token,
            credential,
            unix_timestamp(),
        );
        return Some(match authorized {
            Some(_) => HttpResponse {
                status: "200 OK",
                content_type: "application/octet-stream",
                body: String::new(),
            },
            None => routing::unauthorized_response(),
        });
    }

    if method == "POST" && path.starts_with("/api/v0/relay/controller/files/") {
        let token = path_segment_after(path, "/api/v0/relay/controller/files/")
            .map(decoded_path_segment)
            .unwrap_or_default();
        let Ok(token) = uuid::Uuid::parse_str(token.trim()) else {
            return Some(routing::bad_request_response(
                "Token is not in a valid format",
            ));
        };
        if relay_multipart_boundary(headers.content_type.as_deref()).is_none() {
            return Some(HttpResponse {
                status: "415 Unsupported Media Type",
                content_type: "application/json",
                body: String::new(),
            });
        }
        let Some(credential) = headers
            .x_relay_credential
            .as_deref()
            .map(str::trim)
            .filter(|credential| !credential.is_empty())
        else {
            return Some(routing::unauthorized_response());
        };
        let parts = match relay::parse_multipart(body, headers.content_type.as_deref()) {
            Ok(parts) => parts,
            Err(_) => {
                return Some(routing::bad_request_response(
                    "Upload file section is missing",
                ));
            }
        };
        let Some(file_part) = parts.iter().find(|part| part.filename.is_some()) else {
            return Some(routing::bad_request_response(
                "Upload file section is missing",
            ));
        };
        let Some(filename) = file_part.filename.as_deref() else {
            return Some(routing::bad_request_response("Upload filename is missing"));
        };
        if filename.is_empty()
            || filename.len() > 4 * 1024
            || filename.contains("..")
            || filename.chars().any(char::is_control)
        {
            return Some(routing::bad_request_response("Invalid filename"));
        }
        let authorized = state.relay.write().await.protocol.validate_file_upload(
            &settings,
            relay::credential_scheme(state.config.controller_profile),
            token,
            filename,
            credential,
            unix_timestamp(),
        );
        return Some(match authorized {
            Some(_) => match persist_relay_upload_part(state, token, filename, file_part.data) {
                Ok(upload) => {
                    let upload_path = upload.path.clone();
                    if relay::RuntimeState::complete_file_stream(token, upload) {
                        HttpResponse {
                            status: "200 OK",
                            content_type: "",
                            body: String::new(),
                        }
                    } else {
                        cleanup_relay_upload(&upload_path);
                        relay_upload_error_response("relay file stream request is no longer active")
                    }
                }
                Err(error) => {
                    let _ = relay::RuntimeState::fail_file_stream_token(token, error.clone());
                    relay_upload_error_response(&error)
                }
            },
            None => routing::unauthorized_response(),
        });
    }

    if method == "POST" && path.starts_with("/api/v0/relay/controller/shares/") {
        let token = path_segment_after(path, "/api/v0/relay/controller/shares/")
            .map(decoded_path_segment)
            .unwrap_or_default();
        let Ok(token) = uuid::Uuid::parse_str(token.trim()) else {
            return Some(routing::bad_request_response("Token is not a valid Guid"));
        };
        if relay_multipart_boundary(headers.content_type.as_deref()).is_none() {
            return Some(HttpResponse {
                status: "415 Unsupported Media Type",
                content_type: "application/json",
                body: String::new(),
            });
        }
        let Some(credential) = headers
            .x_relay_credential
            .as_deref()
            .map(str::trim)
            .filter(|credential| !credential.is_empty())
        else {
            return Some(routing::unauthorized_response());
        };
        let parts = match relay::parse_multipart(body, headers.content_type.as_deref()) {
            Ok(parts) => parts,
            Err(_) => {
                return Some(routing::bad_request_response(
                    "Share multipart sections are missing",
                ));
            }
        };
        let authorized = state.relay.write().await.protocol.validate_share_upload(
            &settings,
            relay::credential_scheme(state.config.controller_profile),
            token,
            credential,
            unix_timestamp(),
        );
        return Some(match authorized {
            Some(authorized) => {
                let share_part = parts.iter().find(|part| part.name == "shares");
                let database_part = parts.iter().find(|part| part.filename.is_some());
                let Some(share_part) = share_part else {
                    return Some(routing::bad_request_response(
                        "Share metadata section is missing",
                    ));
                };
                if share_part.data.len() > relay::MAX_RELAY_SHARE_METADATA_BYTES {
                    return Some(routing::bad_request_response(
                        "Share metadata exceeds the maximum size",
                    ));
                }
                let shares = match serde_json::from_slice::<serde_json::Value>(share_part.data) {
                    Ok(value) if value.is_array() => value,
                    _ => {
                        return Some(routing::bad_request_response(
                            "Share metadata is not a JSON array",
                        ));
                    }
                };
                if shares
                    .as_array()
                    .is_some_and(|entries| entries.len() > relay::MAX_RELAY_SHARE_ENTRIES)
                {
                    return Some(routing::bad_request_response(
                        "Share metadata contains too many entries",
                    ));
                }
                let metadata_shares = shares
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|share| {
                        let filename = share.get("filename")?.as_str()?.trim();
                        let size = share.get("size").and_then(serde_json::Value::as_u64)?;
                        (!filename.is_empty()).then(|| relay::RemoteShare {
                            filename: filename.to_owned(),
                            size,
                        })
                    })
                    .collect::<Vec<_>>();
                let Some(database_part) = database_part else {
                    return Some(routing::bad_request_response(
                        "Share database section is missing",
                    ));
                };
                match persist_relay_share_database(
                    state,
                    token,
                    database_part.filename.as_deref().unwrap_or("shares.db"),
                    database_part.data,
                ) {
                    Ok(database_path) => match relay::read_share_database(
                        &database_path,
                        state.config.controller_profile,
                    )
                    .await
                    {
                        Ok(mut remote_shares) => {
                            // Older slskR agents sent file-shaped metadata while the
                            // frozen agents send share-root metadata.  Keep the former
                            // as a compatibility fallback, but always validate and
                            // extract the uploaded SQLite repository first.
                            if remote_shares.is_empty() && !metadata_shares.is_empty() {
                                remote_shares = metadata_shares;
                            }
                            match state.relay.write().await.protocol.record_share_upload(
                                token,
                                authorized.agent_name,
                                remote_shares.len(),
                                remote_shares,
                                database_path.clone(),
                                unix_timestamp(),
                            ) {
                                Ok(()) => HttpResponse {
                                    status: "200 OK",
                                    content_type: "",
                                    body: String::new(),
                                },
                                Err(error) => {
                                    let _ = fs::remove_file(&database_path);
                                    relay_upload_error_response(&error)
                                }
                            }
                        }
                        Err(error) => {
                            let _ = fs::remove_file(&database_path);
                            routing::bad_request_response(&error)
                        }
                    },
                    Err(error) => relay_upload_error_response(&error),
                }
            }
            None => routing::unauthorized_response(),
        });
    }

    None
}

fn relay_upload_directory(state: &AppState) -> Result<PathBuf, String> {
    let directory = state.config.state_dir.join("relay").join("incoming");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("relay upload directory create failed: {error}"))?;
    Ok(directory)
}

fn relay_upload_error_response(error: &str) -> HttpResponse {
    eprintln!("relay upload operation failed: {error}");
    routing::service_unavailable_response("relay upload unavailable")
}

pub(super) fn cleanup_relay_upload(path: &Path) {
    if fs::remove_file(path).is_err() {
        eprintln!("relay upload cleanup failed");
    }
}

fn persist_relay_upload_part(
    state: &AppState,
    token: uuid::Uuid,
    filename: &str,
    data: &[u8],
) -> Result<relay::UploadedFile, String> {
    let directory = relay_upload_directory(state)?;
    let path = directory.join(format!("file-{}.part", token.simple()));
    let file = write_private_relay_staging_file(&path, data, "relay file upload")?;
    Ok(relay::UploadedFile {
        filename: filename.to_owned(),
        file,
        path,
        length: data.len() as u64,
    })
}

pub(super) fn persist_relay_share_database(
    state: &AppState,
    token: uuid::Uuid,
    _filename: &str,
    data: &[u8],
) -> Result<PathBuf, String> {
    let directory = relay_upload_directory(state)?;
    let path = directory.join(format!("share-{}.db", token.simple()));
    let file = write_private_relay_staging_file(&path, data, "relay share database")?;
    drop(file);
    Ok(path)
}

fn write_private_relay_staging_file(
    path: &Path,
    data: &[u8],
    label: &str,
) -> Result<fs::File, String> {
    use std::io::{Seek, Write};

    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("{label} staging create failed: {error}"))?;
    let result = file
        .write_all(data)
        .and_then(|()| file.sync_all())
        .and_then(|()| file.rewind())
        .map_err(|error| format!("{label} staging write failed: {error}"));
    if let Err(error) = result {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error);
    }
    Ok(file)
}

pub(super) fn relay_versioned_stream_content_id(method: &str, path: &str) -> Option<String> {
    if method != "GET" {
        return None;
    }
    let content_id = path_segment_after(path, "/api/v0/relay/streams/")?;
    let content_id = decoded_path_segment(content_id);
    (!content_id.is_empty() && !content_id.contains('/')).then_some(content_id)
}

pub(super) async fn open_relay_controller_stream(
    state: &AppState,
    content_id: &str,
    query: Option<&str>,
) -> Result<LocalStreamFile, String> {
    let agent_name = query_parameter(query, "agentName")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "agentName query parameter is required".to_owned())?;

    let local_file = {
        let shares = state.shares.read().await;
        shares
            .entries
            .iter()
            .find(|entry| {
                entry.filename == content_id
                    || stable_content_hash(&entry.filename, entry.size).to_string() == content_id
            })
            .map(|entry| (entry.filename.clone(), entry.size))
    };
    let (filename, expected_size) = if let Some(file) = local_file {
        file
    } else if let Some((filename, size)) = state
        .relay
        .read()
        .await
        .protocol
        .remote_file_for_agent(&agent_name, content_id)
    {
        (filename, size)
    } else {
        // Completed local transfers are also valid ContentLocator sources in
        // the frozen controller.  Keep the lookup bounded to terminal
        // downloads whose local filename is already known.
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| {
                entry.direction == 0
                    && is_successful_transfer_status(&entry.status)
                    && (entry.filename == content_id
                        || stable_content_hash(&entry.filename, entry.size.unwrap_or(0))
                            .to_string()
                            == content_id)
            })
            .map(|entry| (entry.filename.clone(), entry.size.unwrap_or(0)))
            .ok_or_else(|| "relay content was not found".to_owned())?
    };

    let (info_token, info_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_info(&agent_name, &filename, unix_timestamp())
        .ok_or_else(|| "relay agent is not registered".to_owned())?;
    let info_sent = {
        let relay = state.relay.read().await;
        relay::send_hub_invocation(
            &relay.protocol,
            &agent_name,
            "RequestFileInfo",
            vec![
                serde_json::Value::String(filename.clone()),
                serde_json::Value::String(info_token.to_string()),
            ],
        )
    };
    if !info_sent {
        state
            .relay
            .write()
            .await
            .protocol
            .cancel_file_info(info_token);
        return Err("relay agent connection is unavailable".to_owned());
    }
    let file_info = match time::timeout(Duration::from_secs(30), info_receiver).await {
        Ok(Ok(Ok(info))) => info,
        Ok(Ok(Err(error))) => {
            state
                .relay
                .write()
                .await
                .protocol
                .cancel_file_info(info_token);
            return Err(error);
        }
        Ok(Err(_)) => {
            state
                .relay
                .write()
                .await
                .protocol
                .cancel_file_info(info_token);
            return Err("relay agent closed the file-info request".to_owned());
        }
        Err(_) => {
            state
                .relay
                .write()
                .await
                .protocol
                .cancel_file_info(info_token);
            return Err("relay agent file-info request timed out".to_owned());
        }
    };
    if !file_info.exists {
        return Err("relay file was not found on the agent".to_owned());
    }
    if expected_size != 0 && file_info.length != expected_size {
        return Err("relay agent file-info length does not match the share index".to_owned());
    }

    let (token, receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream(&agent_name, &filename, 0, unix_timestamp())
        .ok_or_else(|| "relay agent is not registered".to_owned())?;
    let sent = {
        let relay = state.relay.read().await;
        relay::send_hub_invocation(
            &relay.protocol,
            &agent_name,
            "RequestFileUpload",
            vec![
                serde_json::Value::String(filename.clone()),
                serde_json::json!(0_u64),
                serde_json::Value::String(token.to_string()),
            ],
        )
    };
    if !sent {
        state.relay.write().await.protocol.cancel_file_stream(token);
        return Err("relay agent connection is unavailable".to_owned());
    }

    let uploaded = match time::timeout(Duration::from_secs(30), receiver).await {
        Ok(Ok(Ok(uploaded))) => uploaded,
        Ok(Ok(Err(error))) => {
            state.relay.write().await.protocol.cancel_file_stream(token);
            return Err(error);
        }
        Ok(Err(_)) => {
            state.relay.write().await.protocol.cancel_file_stream(token);
            return Err("relay agent closed the file stream".to_owned());
        }
        Err(_) => {
            state.relay.write().await.protocol.cancel_file_stream(token);
            return Err("relay agent file stream timed out".to_owned());
        }
    };
    let relay::UploadedFile {
        filename,
        file,
        path,
        length,
    } = uploaded;
    if file_info.length != length || (expected_size != 0 && length != expected_size) {
        cleanup_relay_upload(&path);
        return Err("relay agent upload length does not match file-info".to_owned());
    }
    let metadata = file.metadata().map_err(|error| {
        cleanup_relay_upload(&path);
        format!("relay stream file metadata failed: {error}")
    })?;
    if !metadata.is_file() {
        cleanup_relay_upload(&path);
        return Err("relay stream upload is not a file".to_owned());
    }
    if metadata.len() != length {
        cleanup_relay_upload(&path);
        return Err("relay stream upload length changed during transfer".to_owned());
    }
    Ok(LocalStreamFile {
        file,
        length: metadata.len(),
        content_type: preview_stream_content_type(&filename).to_owned(),
        cleanup_path: Some(path.into()),
    })
}

pub(super) fn relay_versioned_download_token(method: &str, path: &str) -> Option<String> {
    (method == "GET" && path.starts_with("/api/v0/relay/controller/downloads/")).then(|| {
        path_segment_after(path, "/api/v0/relay/controller/downloads/")
            .map(decoded_path_segment)
            .unwrap_or_default()
    })
}

pub(super) async fn open_relay_controller_download(
    state: &AppState,
    token: &str,
    headers: &RequestSecurityHeaders,
) -> Result<LocalStreamFile, String> {
    let token = uuid::Uuid::parse_str(token.trim())
        .map_err(|_| "relay download token is not in a valid format".to_owned())?;
    let credential = headers
        .x_relay_credential
        .as_deref()
        .map(str::trim)
        .filter(|credential| !credential.is_empty())
        .ok_or_else(|| "relay download credential is missing".to_owned())?;
    let settings = state.advanced_networking.read().await.relay.clone();
    let authorized = state
        .relay
        .write()
        .await
        .protocol
        .validate_download(
            &settings,
            relay::credential_scheme(state.config.controller_profile),
            token,
            credential,
            unix_timestamp(),
        )
        .ok_or_else(|| "relay download credential is invalid".to_owned())?;
    let root = effective_downloads_dir(state);
    let path = safe_download_path(&root, &authorized.filename)
        .map_err(|_| "Invalid filename".to_owned())?;
    let file = open_download_file_for_read(&root, &path)?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("relay download metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("relay download source is not a file".to_owned());
    }
    Ok(LocalStreamFile {
        file,
        length: metadata.len(),
        content_type: "application/octet-stream".to_owned(),
        cleanup_path: None,
    })
}

#[cfg(test)]
mod staging_tests {
    use super::write_private_relay_staging_file;
    use std::io::Read;

    #[test]
    fn relay_staging_handle_is_readable_rewound_and_exclusive() {
        let directory =
            std::env::temp_dir().join(format!("slskr-relay-staging-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).expect("create relay staging fixture");
        let path = directory.join("upload.part");
        let mut file = write_private_relay_staging_file(&path, b"stream", "fixture").unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .expect("read returned relay handle");
        assert_eq!(bytes, b"stream");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert!(write_private_relay_staging_file(&path, b"replacement", "fixture").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"stream");
        drop(file);
        std::fs::remove_dir_all(directory).expect("remove relay staging fixture");
    }
}
