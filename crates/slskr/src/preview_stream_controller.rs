use super::*;

pub(super) async fn create_preview_stream_ticket(
    state: &AppState,
    family: &str,
    body: &str,
) -> Result<String, String> {
    let payload = serde_json::from_str::<serde_json::Value>(body).ok();
    let field = |name: &str| {
        payload
            .as_ref()
            .and_then(|value| value.get(name))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    };
    let mesh_contract = family == "mesh";
    let content_id = if mesh_contract {
        let value = field("contentId").ok_or_else(|| "ContentId is required.".to_owned())?;
        let value = normalize_preview_identifier(value, "ContentId", 512)?;
        Some(value)
    } else {
        field("contentId")
            .or_else(|| field("content_id"))
            .or_else(|| field("id"))
    };
    let filename = if mesh_contract {
        let value = field("filename").ok_or_else(|| "Filename is required.".to_owned())?;
        normalize_preview_filename(value)?
    } else {
        let value = field("filename")
            .or_else(|| field("path"))
            .or_else(|| content_id.clone())
            .ok_or_else(|| "Filename is required.".to_owned())?;
        normalize_preview_filename(value)?
    };
    let peer_username = if mesh_contract {
        field("peerId")
            .filter(|value| !value.trim().is_empty())
            .map(|value| normalize_preview_identifier(value, "PeerId", 512))
            .transpose()?
    } else if matches!(family, "peer" | "soulseek") {
        let value = field("username")
            .or_else(|| field("peerUsername"))
            .or_else(|| field("peer_username"))
            .or_else(|| field("peerId"))
            .ok_or_else(|| "Username is required.".to_owned())?;
        let value = value.trim();
        if value.is_empty()
            || value.encode_utf16().count() > 256
            || value.chars().any(char::is_control)
        {
            return Err("Username is required.".to_owned());
        }
        Some(value.to_owned())
    } else {
        field("username")
            .or_else(|| field("peerUsername"))
            .or_else(|| field("peer_username"))
            .or_else(|| field("peerId"))
    };
    let requested_size = if mesh_contract {
        let value = payload
            .as_ref()
            .and_then(|payload| payload.get("expectedSize").or_else(|| payload.get("size")));
        match value {
            None | Some(serde_json::Value::Null) => 0,
            Some(value) => value
                .as_i64()
                .filter(|value| *value >= 0)
                .and_then(|value| u64::try_from(value).ok())
                .or_else(|| value.as_u64())
                .ok_or_else(|| "Expected size must be greater than or equal to zero.".to_owned())?,
        }
    } else {
        match payload.as_ref().and_then(|payload| payload.get("size")) {
            None | Some(serde_json::Value::Null) => 0,
            Some(value) => value
                .as_i64()
                .filter(|value| *value >= 0)
                .and_then(|value| u64::try_from(value).ok())
                .or_else(|| value.as_u64())
                .ok_or_else(|| "Size must be greater than or equal to zero.".to_owned())?,
        }
    };
    if requested_size > MAX_PREVIEW_STREAM_BYTES {
        return Err("preview stream size exceeds the 2 GiB limit".to_owned());
    }
    let source_url = extract_json_string_field(body, "sourceUrl")
        .or_else(|| extract_json_string_field(body, "source_url"));
    let source_authorization = extract_json_string_field(body, "sourceAuthorization")
        .or_else(|| extract_json_string_field(body, "source_authorization"));
    let expected_hash = field("expectedHash").or_else(|| field("expected_hash"));
    let expected_hash = expected_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if mesh_contract
        && expected_hash.as_deref().is_some_and(|hash| {
            hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err("Expected hash must be a SHA-256 hex digest.".to_owned());
    }
    if mesh_contract && content_id.as_deref().is_none_or(str::is_empty) {
        return Err("contentId is required".to_owned());
    }
    let share = find_shared_entry_for_content(state, Some(&filename), content_id.as_deref()).await;
    let content_type_filename = share
        .as_ref()
        .map(|entry| entry.filename.as_str())
        .unwrap_or(filename.as_str());
    let content_type = if mesh_contract {
        mesh_preview_stream_content_type(content_type_filename)
            .unwrap_or("application/octet-stream")
    } else {
        preview_stream_content_type(content_type_filename)
    };
    if content_type == "application/octet-stream" {
        return Err(if mesh_contract {
            "Only audio files can be preview streamed from mesh peers.".to_owned()
        } else {
            "Only audio files can be preview streamed from peers.".to_owned()
        });
    }
    let content_type = content_type.to_owned();
    let remote_mesh = match source_url {
        Some(_) if family != "mesh" => {
            return Err("sourceUrl is supported only for mesh preview streams".to_owned());
        }
        Some(source_url) => {
            if requested_size == 0 {
                return Err("size is required for a remote mesh preview".to_owned());
            }
            if source_authorization
                .as_ref()
                .is_some_and(|value| value.contains(['\r', '\n']) || value.len() > 8 * 1024)
            {
                return Err("sourceAuthorization is invalid".to_owned());
            }
            let expected_hash = expected_hash
                .as_deref()
                .map(str::trim)
                .filter(|hash| {
                    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
                .ok_or_else(|| {
                    "expectedHash must be a SHA-256 digest for a remote mesh preview".to_owned()
                })?
                .to_ascii_lowercase();
            Some((source_url, source_authorization, expected_hash))
        }
        None => None,
    };

    let transfers = state.transfers.read().await;
    let searches = state.searches.read().await;
    let transfer = transfers.entries.iter().find(|entry| {
        entry.filename == filename
            || content_id
                .as_deref()
                .and_then(|id| id.strip_prefix("transfer-"))
                .and_then(|id| id.parse::<u64>().ok())
                == Some(entry.id)
    });
    let search_result = searches.records.iter().find_map(|record| {
        record.results.iter().find(|result| {
            (result.filename == filename || content_id.as_deref() == Some(result.filename.as_str()))
                && peer_username
                    .as_deref()
                    .is_none_or(|peer| result.peer_username.as_deref() == Some(peer))
        })
    });

    let resolved_filename = share
        .as_ref()
        .map(|entry| entry.filename.clone())
        .or_else(|| transfer.map(|entry| entry.filename.clone()))
        .or_else(|| search_result.map(|entry| entry.filename.clone()))
        .unwrap_or(filename);
    let resolved_size = share
        .as_ref()
        .map(|entry| entry.size)
        .or_else(|| transfer.and_then(|entry| entry.size))
        .or_else(|| search_result.map(|entry| entry.size))
        .unwrap_or(requested_size);
    let resolved_content_id = content_id
        .unwrap_or_else(|| stable_content_hash(&resolved_filename, resolved_size).to_string());
    let resolved_peer_username =
        peer_username.or_else(|| search_result.and_then(|entry| entry.peer_username.clone()));
    let source = if share.is_some() {
        "local-share"
    } else if transfer.is_some() {
        "transfer"
    } else if search_result.is_some() && family == "mesh" {
        "mesh-preview"
    } else if search_result.is_some() {
        "peer-preview"
    } else if family == "mesh" {
        "mesh-unresolved"
    } else {
        "peer-unresolved"
    };
    drop(searches);
    drop(transfers);

    let remote_overlay = if family == "mesh"
        && remote_mesh.is_none()
        && !matches!(source, "local-share" | "transfer")
    {
        if let Some(identity) = resolved_peer_username.as_deref() {
            let trusted = state
                .config
                .trusted_mesh_peers
                .iter()
                .find(|peer| peer.matches(identity));
            trusted
                .map(|peer| {
                    if resolved_size == 0 {
                        return Err("size is required for a trusted mesh preview".to_owned());
                    }
                    let expected_hash = expected_hash
                        .as_deref()
                        .map(str::trim)
                        .filter(|hash| {
                            hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                        })
                        .ok_or_else(|| {
                            "expectedHash must be a SHA-256 digest for a trusted mesh preview"
                                .to_owned()
                        })?;
                    Ok((peer.peer_id.clone(), expected_hash.to_ascii_lowercase()))
                })
                .transpose()?
        } else {
            None
        }
    } else {
        None
    };

    let mut tickets = state.stream_tickets.write().await;
    let Some((token, ticket)) = tickets.issue(
        family,
        source,
        resolved_content_id,
        resolved_filename,
        resolved_peer_username,
        resolved_size,
        content_type,
        120,
    ) else {
        return Err("preview stream ticket capacity is full".to_owned());
    };
    if let Some((source_url, source_authorization, expected_hash)) = remote_mesh {
        if !tickets.configure_remote_mesh(&token, source_url, source_authorization, expected_hash) {
            return Err("preview stream ticket could not be configured".to_owned());
        }
    } else if let Some((peer_identity, expected_hash)) = remote_overlay {
        if !tickets.configure_remote_overlay(&token, peer_identity, expected_hash) {
            return Err("preview stream ticket could not be configured".to_owned());
        }
    }
    drop(tickets);

    let body = if mesh_contract {
        serde_json::json!({
            "ticket": token,
            "streamUrl": format!("/api/v0/mesh-streams/{}", url_encode(&token)),
            "expiresInSeconds": 120,
            "contentType": ticket.content_type,
            "source": "mesh",
        })
        .to_string()
    } else {
        serde_json::json!({
            "ticket": token,
            "streamUrl": format!("/api/v0/{family}-streams/{}", url_encode(&token)),
            "stream_url": format!("/api/v0/{family}-streams/{}", url_encode(&token)),
            "expiresInSeconds": 120,
            "contentType": ticket.content_type,
            "content_id": ticket.content_id,
            "filename": ticket.filename,
            "size": ticket.size,
            "source": ticket.source,
        })
        .to_string()
    };
    Ok(body)
}

fn normalize_preview_identifier(
    value: String,
    field_name: &str,
    max_utf16_units: usize,
) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.encode_utf16().count() > max_utf16_units
        || value.chars().any(char::is_control)
    {
        return Err(format!("{field_name} is required."));
    }
    Ok(value.to_owned())
}

fn normalize_preview_filename(value: String) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty()
        || value.encode_utf16().count() > 4_096
        || value.chars().any(char::is_control)
        || preview_filename_contains_traversal(value)
        || Path::new(value).is_absolute()
        || (value
            .as_bytes()
            .first()
            .is_some_and(|character| character.is_ascii_alphabetic())
            && value
                .as_bytes()
                .get(1)
                .is_some_and(|separator| *separator == b':'))
    {
        return Err("Filename is required.".to_owned());
    }
    Ok(value.to_owned())
}

fn preview_filename_contains_traversal(value: &str) -> bool {
    let mut decoded = value.to_owned();
    for _ in 0..5 {
        let next = percent_decode_component(&decoded);
        if next == decoded {
            break;
        }
        decoded = next;
    }
    value.contains("..") || decoded.contains("..")
}

pub(super) async fn find_shared_local_file(
    state: &AppState,
    filename: &str,
) -> Option<SharedLocalFile> {
    let local_path = {
        let shares = state.shares.read().await;
        let normalized = filename.replace('\\', "/");
        shares
            .local_paths
            .get(filename)
            .or_else(|| shares.local_paths.get(&normalized))
            .cloned()
    }?;
    let settings = state.share_settings.read().await;
    let metadata = shared_local_file_metadata(&settings, &local_path)?;
    metadata.is_file().then_some(SharedLocalFile {
        local_path,
        size: metadata.len(),
    })
}

/// Issue the frozen listening-party stream ticket for a local content id.
///
/// native profile binds these tickets to `listening-party:{partyId}` and the radio
/// controller checks that owner before opening the resolved local file.  The
/// ticket is cached for its lifetime so repeated directory reads do not fill
/// the bounded preview-ticket store with duplicate announcements.
pub(super) async fn issue_listening_party_stream_ticket(
    state: &AppState,
    party_id: &str,
    content_id: &str,
) -> Option<String> {
    let content_id = content_id.trim();
    let party_id = party_id.trim();
    if content_id.is_empty() || party_id.is_empty() {
        return None;
    }

    let (filename, size) = {
        let shares = state.shares.read().await;
        let transfers = state.transfers.read().await;
        let share = shares.entries.iter().find(|entry| {
            entry.filename == content_id
                || stable_content_hash(&entry.filename, entry.size).to_string() == content_id
        });
        let transfer = transfers.entries.iter().find(|entry| {
            entry.direction == 0
                && is_successful_transfer_status(&entry.status)
                && (entry.filename == content_id
                    || entry.id.to_string()
                        == content_id.strip_prefix("transfer-").unwrap_or_default())
        });
        let filename = share
            .map(|entry| entry.filename.clone())
            .or_else(|| transfer.map(|entry| entry.filename.clone()))
            .unwrap_or_else(|| content_id.to_owned());
        let size = share
            .map(|entry| entry.size)
            .or_else(|| transfer.and_then(|entry| entry.size))
            .unwrap_or(0);
        (filename, size)
    };

    let source = format!("listening-party:{party_id}");
    let mut tickets = state.stream_tickets.write().await;
    if let Some(token) = tickets.find_active_token("listening-party", &source, content_id) {
        return Some(token);
    }
    let content_type = preview_stream_content_type(&filename).to_owned();
    let (token, _) = tickets.issue(
        "listening-party",
        &source,
        content_id.to_owned(),
        filename,
        None,
        size,
        content_type,
        900,
    )?;
    Some(token)
}

fn shared_local_file_metadata(
    settings: &crate::config::ShareSettings,
    local_path: &Path,
) -> Option<fs::Metadata> {
    let symlink_metadata = fs::symlink_metadata(local_path).ok()?;
    if symlink_metadata.file_type().is_symlink() && !settings.follow_symlinks {
        return None;
    }
    let metadata = fs::metadata(local_path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    if !settings.roots.is_empty() {
        let canonical_path = local_path.canonicalize().ok()?;
        let inside_share_root = settings
            .roots
            .iter()
            .filter_map(|root| root.canonicalize().ok())
            .any(|root| canonical_path.starts_with(root));
        if !inside_share_root {
            return None;
        }
    }
    Some(metadata)
}

pub(super) async fn open_shared_local_file(
    state: &AppState,
    local_path: &Path,
) -> Result<fs::File, String> {
    let settings = state.share_settings.read().await;
    #[cfg(unix)]
    if !settings.follow_symlinks && !settings.roots.is_empty() {
        return open_shared_local_file_unix(&settings.roots, local_path);
    }

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        if !settings.follow_symlinks {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW);
        }
    }
    options
        .open(local_path)
        .map_err(|error| format!("local file open failed: {error}"))
}

pub(super) fn application_dump_path(path: &str) -> bool {
    let route = routing::parse_route("GET", path);
    route.path == "/api/v0/application/dump"
}

pub(super) fn application_dump_request(method: &str, path: &str, config: &AppConfig) -> bool {
    application_dump_path(path)
        && match config.controller_profile {
            ControllerProfile::Legacy => method == "GET",
            ControllerProfile::Native => method == "POST",
        }
}

pub(super) async fn open_application_dump_file(
    state: &AppState,
) -> Result<LocalStreamFile, String> {
    let state_dir = state.config.state_dir.clone();
    run_application_dump_worker(move || create_application_dump_file(&state_dir)).await
}

async fn run_application_dump_worker<T: Send + 'static>(
    worker: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    static DUMP_PERMITS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
    let permit = DUMP_PERMITS
        .try_acquire()
        .map_err(|_| "application dump is already running".to_owned())?;
    tokio::task::spawn_blocking(move || {
        // Retain admission through the blocking work after requester cancellation.
        let _permit = permit;
        worker()
    })
    .await
    .map_err(|error| format!("application dump task failed: {error}"))?
}

fn create_application_dump_file(state_dir: &Path) -> Result<LocalStreamFile, String> {
    let directory = state_dir.join("diagnostics");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("application dump directory create failed: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("application dump directory permissions failed: {error}"))?;
    }
    let basename = directory.join(format!("slskr-{}", uuid::Uuid::new_v4().simple()));
    let output_path = if std::env::var_os("SLSKR_CONTROLLER_AUDIT_MODE").is_some() {
        let path = basename.with_extension("audit.dmp");
        fs::write(
            &path,
            format!(
                "\x7fELF\nslskr controller-audit diagnostic dump\npid={}\n",
                std::process::id()
            ),
        )
        .map_err(|error| format!("application audit dump write failed: {error}"))?;
        path
    } else {
        create_platform_core_dump(&basename)?
    };
    let file = fs::OpenOptions::new()
        .read(true)
        .open(&output_path)
        .map_err(|error| format!("application dump open failed: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("application dump metadata failed: {error}"))?;
    if !metadata.is_file() || metadata.len() == 0 {
        let _ = fs::remove_file(&output_path);
        return Err("application dump is empty".to_owned());
    }
    Ok(LocalStreamFile {
        file,
        length: metadata.len(),
        content_type: "application/octet-stream".to_owned(),
        cleanup_path: Some(output_path.into()),
    })
}

#[cfg(target_os = "linux")]
fn create_platform_core_dump(basename: &Path) -> Result<PathBuf, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let pid = std::process::id();
    let output_path = PathBuf::from(format!("{}.{pid}", basename.display()));
    // Start a shell blocked on stdin so Linux Yama can authorize only this
    // child PID before it execs gcore and attaches to this process.
    let mut command = Command::new("/bin/sh");
    command
        .arg("-c")
        .arg("read _; exec gcore \"$@\"")
        .arg("slskr-gcore")
        .arg("-o")
        .arg(basename)
        .arg(pid.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::script_process_group::configure_blocking(&mut command);
    let child = command
        .spawn()
        .map_err(|error| format!("application dump requires gcore: {error}"))?;
    let mut owner = crate::core_dump_process::CoreDumpProcess::new(child, output_path.clone());
    owner.authorize_ptrace()?;
    owner
        .child
        .stdin
        .take()
        .ok_or_else(|| "application dump child stdin unavailable".to_owned())
        .and_then(|mut stdin| {
            writeln!(stdin)
                .map_err(|error| format!("application dump child release failed: {error}"))
        })?;
    let status = owner.wait_until(std::time::Instant::now() + Duration::from_secs(60))?;
    if !status.success() {
        return Err("gcore failed to create the application dump".to_owned());
    }
    owner.keep_output();
    Ok(output_path)
}

#[cfg(not(target_os = "linux"))]
fn create_platform_core_dump(_basename: &Path) -> Result<PathBuf, String> {
    Err("application dumps are not supported on this platform".to_owned())
}

pub(super) struct PeerPreviewStream {
    connection: slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    token: u32,
    length: u64,
    content_type: String,
}

pub(super) fn primary_stream_id(path: &str) -> Option<String> {
    let route = routing::parse_route("GET", path);
    let normalized = if let Some(versioned) = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
    {
        format!("/api/{versioned}")
    } else {
        route.normalized_path.to_owned()
    };
    let raw = normalized.strip_prefix("/api/streams/")?;
    (!raw.is_empty() && !raw.ends_with("/share-ticket")).then(|| decoded_path_segment(raw))
}

fn preview_stream_ticket_path(path: &str) -> Option<(&'static str, String)> {
    let route = routing::parse_route("GET", path);
    let normalized = if let Some(versioned) = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
    {
        format!("/api/{versioned}")
    } else {
        route.normalized_path.to_owned()
    };
    let (family, token) = normalized
        .strip_prefix("/api/peer-streams/")
        .map(|token| ("peer", token))
        .or_else(|| {
            normalized
                .strip_prefix("/api/mesh-streams/")
                .map(|token| ("mesh", token))
        })?;
    (!token.is_empty() && !token.contains('/')).then(|| (family, decoded_path_segment(token)))
}

pub(super) fn listening_party_stream_path(path: &str) -> Option<(String, String)> {
    let route = routing::parse_route("GET", path);
    let normalized = if let Some(versioned) = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
    {
        format!("/api/{versioned}")
    } else {
        route.normalized_path.to_owned()
    };
    let segments = decoded_segments_after(&normalized, "/api/listening-party/radio/")?;
    let [party_id, content_id] = segments.as_slice() else {
        return None;
    };
    Some((party_id.clone(), content_id.clone()))
}

pub(super) fn http_stream_ticket_path(
    path: &str,
    query: Option<&str>,
) -> Option<(&'static str, String)> {
    preview_stream_ticket_path(path).or_else(|| {
        listening_party_stream_path(path)?;
        query_parameter(query, "ticket")
            .filter(|token| !token.trim().is_empty() && !token.contains('/'))
            .map(|token| ("listening-party", token))
    })
}

pub(super) async fn open_local_preview_stream_file(
    state: &AppState,
    family: &str,
    token: &str,
) -> Result<Option<LocalStreamFile>, String> {
    let ticket = {
        let mut tickets = state.stream_tickets.write().await;
        tickets.get(token)
    };
    let Some(ticket) = ticket.filter(|ticket| ticket.family == family) else {
        return Ok(None);
    };

    if let Some(shared) = find_shared_local_file(state, &ticket.filename).await {
        let file = open_shared_local_file(state, &shared.local_path).await?;
        let metadata = file
            .metadata()
            .map_err(|error| format!("shared preview metadata failed: {error}"))?;
        if metadata.is_file() {
            return Ok(Some(LocalStreamFile {
                file,
                length: metadata.len(),
                content_type: ticket.content_type,
                cleanup_path: None,
            }));
        }
    }

    let transfer = {
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| {
                entry.direction == 0
                    && entry.filename == ticket.filename
                    && is_successful_transfer_status(&entry.status)
            })
            .cloned()
    };
    let Some(local_path) = transfer.and_then(|transfer| transfer.local_path) else {
        return Ok(None);
    };
    let root = effective_downloads_dir(state);
    let file = open_download_file_for_read(&root, Path::new(&local_path))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("download preview metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Ok(None);
    }
    Ok(Some(LocalStreamFile {
        file,
        length: metadata.len(),
        content_type: ticket.content_type,
        cleanup_path: None,
    }))
}

pub(super) async fn open_remote_peer_preview_stream(
    state: &AppState,
    family: &str,
    token: &str,
) -> Result<Option<PeerPreviewStream>, String> {
    if family != "peer" || !state.config.transfer_allow_outbound {
        return Ok(None);
    }
    let ticket = {
        let mut tickets = state.stream_tickets.write().await;
        tickets.get(token)
    };
    let Some(ticket) = ticket.filter(|ticket| ticket.family == "peer") else {
        return Ok(None);
    };
    if matches!(ticket.source.as_str(), "local-share" | "transfer") {
        return Ok(None);
    }
    let username = ticket
        .peer_username
        .clone()
        .filter(|username| !username.trim().is_empty())
        .ok_or_else(|| "peer preview ticket has no source username".to_owned())?;
    let address = request_peer_endpoint(state, &username).await?;
    let transfer_token = state.transfers.write().await.allocate_token();
    let now = unix_timestamp();
    let transfer = TransferEntry {
        id: 0,
        direction: 0,
        token: transfer_token,
        peer_username: Some(username),
        filename: ticket.filename.clone(),
        local_path: None,
        batch_id: None,
        request_id: None,
        wishlist_item_id: None,
        request_name: None,
        destination_directory: None,
        bit_rate: None,
        sample_rate: None,
        bit_depth: None,
        length_seconds: None,
        artist: None,
        album: None,
        title: None,
        track_number: None,
        year: None,
        attempts: 1,
        auto_replace_attempts: 0,
        next_attempt_at: None,
        size: (ticket.size > 0).then_some(ticket.size),
        bytes_transferred: 0,
        status: "peer_negotiating".to_owned(),
        reason: None,
        requested_at: now,
        started_at: None,
        start_offset: 0,
        updated_at: now,
        updated_at_ms: unix_timestamp_millis(),
        previous_status: None,
    };
    let length = match negotiate_peer_transfer(state, &address, &transfer).await? {
        PeerTransferNegotiation::Allowed { token, size } if token == transfer_token => size
            .or(transfer.size)
            .filter(|size| *size > 0)
            .ok_or_else(|| "peer preview size is unavailable".to_owned())?,
        PeerTransferNegotiation::Rejected { .. }
        | PeerTransferNegotiation::QueuedInbound { .. } => {
            return Err("peer preview source is not immediately available".to_owned());
        }
        PeerTransferNegotiation::Allowed { .. } => {
            return Err("peer preview negotiation token did not match".to_owned());
        }
    };
    if length > MAX_PREVIEW_STREAM_BYTES {
        return Err("peer preview size exceeds the 2 GiB limit".to_owned());
    }
    let connection = connect_file_transfer_preferred(state, &address).await?;
    Ok(Some(PeerPreviewStream {
        connection,
        token: transfer_token,
        length,
        content_type: ticket.content_type,
    }))
}

pub(super) async fn open_remote_mesh_preview_file(
    state: &AppState,
    family: &str,
    token: &str,
) -> Result<Option<LocalStreamFile>, String> {
    if family != "mesh" {
        return Ok(None);
    }
    let ticket = {
        let mut tickets = state.stream_tickets.write().await;
        tickets.get(token)
    };
    let Some(ticket) = ticket.filter(|ticket| ticket.family == "mesh") else {
        return Ok(None);
    };
    if matches!(ticket.source.as_str(), "local-share" | "transfer") {
        return Ok(None);
    }
    let expected_hash = ticket
        .expected_hash
        .as_deref()
        .ok_or_else(|| "mesh preview ticket has no expected hash".to_owned())?;
    let relative = format!(".preview/{}.part", uuid::Uuid::new_v4());
    let downloads_dir = effective_downloads_dir(state);
    let path = safe_download_path(&downloads_dir, &relative).and_then(|path| {
        ensure_scoped_download_path(&downloads_dir, path.to_string_lossy().as_ref())
    })?;
    if let Some(source_url) = ticket.source_url.clone() {
        multisource::fetch_single_verified_source(
            multisource::RangeSource {
                username: ticket
                    .peer_username
                    .clone()
                    .unwrap_or_else(|| "mesh-peer".to_owned()),
                url: source_url,
                authorization: ticket.source_authorization.clone(),
            },
            ticket.size,
            expected_hash,
            &path,
        )
        .await?;
    } else if let Some(peer_identity) = ticket.overlay_peer_identity.as_deref() {
        let peer = state
            .config
            .trusted_mesh_peers
            .iter()
            .find(|peer| peer.matches(peer_identity))
            .ok_or_else(|| "trusted mesh preview peer is no longer configured".to_owned())?;
        let local_username = pod_request_peer_id(state)
            .await
            .ok_or_else(|| "local mesh identity is unavailable".to_owned())?;
        mesh_services::fetch_content(
            state.private_gateway.as_ref(),
            peer,
            &local_username,
            &state.capability_signing_key,
            &ticket.content_id,
            ticket.size,
            expected_hash,
            &path,
        )
        .await?;
    } else {
        return Err("mesh preview ticket has no executable source endpoint".to_owned());
    }
    let file = match open_download_file_for_read(&downloads_dir, &path) {
        Ok(file) => file,
        Err(error) => {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
    };
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = fs::remove_file(&path);
            return Err(format!("mesh preview metadata failed: {error}"));
        }
    };
    Ok(Some(LocalStreamFile {
        file,
        length: metadata.len(),
        content_type: ticket.content_type,
        cleanup_path: Some(path.into()),
    }))
}

pub(super) async fn write_peer_preview_response<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    mut preview: PeerPreviewStream,
    include_body: bool,
    keep_alive: bool,
    extra_headers: &str,
    io_timeout: Duration,
) -> Result<http_server::FileResponseResult, String> {
    if include_body {
        let received_token = time::timeout(io_timeout, preview.connection.receive_token())
            .await
            .map_err(|_| "peer preview token receive timed out".to_owned())?
            .map_err(|error| format!("peer preview token receive failed: {error}"))?;
        if received_token != preview.token {
            return Err("peer preview transfer token did not match".to_owned());
        }
        time::timeout(io_timeout, preview.connection.send_offset(0))
            .await
            .map_err(|_| "peer preview offset send timed out".to_owned())?
            .map_err(|error| format!("peer preview offset send failed: {error}"))?;
    }
    let connection = if keep_alive { "keep-alive" } else { "close" };
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccept-Ranges: none\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nStrict-Transport-Security: max-age=31536000; includeSubDomains\r\nConnection: {connection}\r\n{extra_headers}\r\n",
        preview.content_type, preview.length
    );
    time::timeout(io_timeout, writer.write_all(headers.as_bytes()))
        .await
        .map_err(|_| "peer preview response header timed out".to_owned())?
        .map_err(|error| format!("peer preview response header failed: {error}"))?;
    if include_body {
        let mut remaining = preview.length;
        while remaining > 0 {
            let wanted = usize::try_from(remaining.min(TRANSFER_PROGRESS_CHUNK_BYTES as u64))
                .map_err(|_| "peer preview chunk size is invalid".to_owned())?;
            let chunk = time::timeout(io_timeout, preview.connection.read_chunk(wanted))
                .await
                .map_err(|_| "peer preview chunk receive timed out".to_owned())?
                .map_err(|error| format!("peer preview chunk receive failed: {error}"))?;
            if chunk.is_empty() {
                return Err("peer preview source closed before completion".to_owned());
            }
            time::timeout(io_timeout, writer.write_all(&chunk))
                .await
                .map_err(|_| "peer preview response write timed out".to_owned())?
                .map_err(|error| format!("peer preview response write failed: {error}"))?;
            remaining = remaining.saturating_sub(chunk.len() as u64);
        }
    }
    time::timeout(io_timeout, writer.flush())
        .await
        .map_err(|_| "peer preview response flush timed out".to_owned())?
        .map_err(|error| format!("peer preview response flush failed: {error}"))?;
    Ok(http_server::FileResponseResult {
        status_code: 200,
        content_length: preview.length,
    })
}

pub(super) async fn write_remote_preview_head_response<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    ticket: &PreviewStreamTicket,
    keep_alive: bool,
    extra_headers: &str,
    io_timeout: Duration,
) -> Result<http_server::FileResponseResult, String> {
    let connection = if keep_alive { "keep-alive" } else { "close" };
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccept-Ranges: none\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nStrict-Transport-Security: max-age=31536000; includeSubDomains\r\nConnection: {connection}\r\n{extra_headers}\r\n",
        ticket.content_type, ticket.size
    );
    time::timeout(io_timeout, async {
        writer
            .write_all(headers.as_bytes())
            .await
            .map_err(|error| format!("remote preview HEAD response failed: {error}"))?;
        writer
            .flush()
            .await
            .map_err(|error| format!("remote preview HEAD flush failed: {error}"))
    })
    .await
    .map_err(|_| "remote preview HEAD response timed out".to_owned())??;
    Ok(http_server::FileResponseResult {
        status_code: 200,
        content_length: ticket.size,
    })
}

pub(super) fn remote_preview_head_ticket(
    ticket: Option<PreviewStreamTicket>,
    family: &str,
) -> Option<PreviewStreamTicket> {
    ticket.filter(|ticket| ticket.family == family && ticket.size > 0)
}

pub(super) async fn open_primary_stream_file(
    state: &AppState,
    stream_id: &str,
    query: Option<&str>,
) -> Result<Option<LocalStreamFile>, String> {
    let ticket_filename = if let Some(token) = query_parameter(query, "ticket") {
        let mut tickets = state.stream_tickets.write().await;
        let ticket = tickets.get(&token);
        drop(tickets);
        ticket
            .filter(|ticket| ticket.family == "share" && ticket.content_id == stream_id)
            .map(|ticket| ticket.filename)
    } else {
        None
    };

    let shared_filename =
        find_shared_entry_for_content(state, ticket_filename.as_deref(), Some(stream_id))
            .await
            .map(|entry| entry.filename);
    if let Some(filename) = shared_filename {
        if let Some(shared) = find_shared_local_file(state, &filename).await {
            let file = open_shared_local_file(state, &shared.local_path).await?;
            let metadata = file
                .metadata()
                .map_err(|error| format!("shared stream metadata failed: {error}"))?;
            if metadata.is_file() {
                return Ok(Some(LocalStreamFile {
                    file,
                    length: metadata.len(),
                    content_type: primary_stream_content_type(&filename).to_owned(),
                    cleanup_path: None,
                }));
            }
        }
    }

    let transfer = {
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| {
                let matches_id = stream_id
                    .strip_prefix("transfer-")
                    .and_then(|id| id.parse::<u64>().ok())
                    == Some(entry.id);
                entry.direction == 0
                    && is_successful_transfer_status(&entry.status)
                    && (matches_id || ticket_filename.as_deref() == Some(entry.filename.as_str()))
            })
            .cloned()
    };
    let Some(transfer) = transfer else {
        return Ok(None);
    };
    let Some(local_path) = transfer.local_path.as_deref() else {
        return Ok(None);
    };
    let root = effective_downloads_dir(state);
    let path = PathBuf::from(local_path);
    let file = open_download_file_for_read(&root, &path)?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("download stream metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Ok(None);
    }
    Ok(Some(LocalStreamFile {
        file,
        length: metadata.len(),
        content_type: primary_stream_content_type(&transfer.filename).to_owned(),
        cleanup_path: None,
    }))
}

#[cfg(unix)]
pub(super) fn open_shared_local_file_unix(
    roots: &[PathBuf],
    local_path: &Path,
) -> Result<fs::File, String> {
    use rustix::fs::{open, openat, Mode, OFlags};

    let (root, relative) = roots
        .iter()
        .find_map(|root| {
            local_path
                .strip_prefix(root)
                .ok()
                .map(|relative| (root, relative))
        })
        .ok_or_else(|| "local file is outside configured share roots".to_owned())?;
    let mut components = relative.components().peekable();
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("share root confined open failed: {error}"))?;
    while let Some(component) = components.next() {
        let Component::Normal(component) = component else {
            return Err("shared file path contains a non-relative component".to_owned());
        };
        if components.peek().is_none() {
            let file = openat(
                &directory,
                component,
                OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| format!("shared file confined open failed: {error}"))?;
            return Ok(fs::File::from(file));
        }
        directory = openat(&directory, component, directory_flags, Mode::empty())
            .map_err(|error| format!("shared directory confined open failed: {error}"))?;
    }
    Err("shared file path is empty".to_owned())
}

pub(super) fn primary_stream_content_type(path: &str) -> &'static str {
    match path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
    {
        Some(extension) if extension == "mp4" => "video/mp4",
        Some(extension) if extension == "webm" => "video/webm",
        Some(extension) if extension == "mkv" => "video/x-matroska",
        _ => preview_stream_content_type(path),
    }
}

pub(super) fn preview_stream_content_type(path: &str) -> &'static str {
    match path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
    {
        Some(extension) if extension == "flac" => "audio/flac",
        Some(extension) if extension == "mp3" => "audio/mpeg",
        Some(extension) if extension == "ogg" || extension == "oga" => "audio/ogg",
        Some(extension) if extension == "opus" => "audio/opus",
        Some(extension) if extension == "aac" => "audio/aac",
        Some(extension) if extension == "wav" => "audio/wav",
        Some(extension) if extension == "m4a" || extension == "mp4" => "audio/mp4",
        _ => "application/octet-stream",
    }
}

/// The frozen MeshStreamTicketService intentionally accepts a narrower
/// extension set than the compatibility preview routes.  Keep that contract
/// separate so mesh tickets do not inherit peer/listening-party aliases such
/// as `.oga` or `.mp4`.
fn mesh_preview_stream_content_type(path: &str) -> Option<&'static str> {
    match path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
    {
        Some(extension) if extension == "flac" => Some("audio/flac"),
        Some(extension) if extension == "mp3" => Some("audio/mpeg"),
        Some(extension) if extension == "m4a" => Some("audio/mp4"),
        Some(extension) if extension == "aac" => Some("audio/aac"),
        Some(extension) if extension == "ogg" => Some("audio/ogg"),
        Some(extension) if extension == "opus" => Some("audio/opus"),
        Some(extension) if extension == "wav" => Some("audio/wav"),
        _ => None,
    }
}

#[cfg(test)]
mod dump_worker_tests {
    use super::run_application_dump_worker;
    use std::time::Duration;

    #[tokio::test]
    async fn canceled_request_retains_dump_admission_until_blocking_worker_finishes() {
        let (started, started_rx) = tokio::sync::oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let request = tokio::spawn(run_application_dump_worker(move || {
            let _ = started.send(());
            release_rx
                .recv_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())?;
            Ok(())
        }));
        tokio::time::timeout(Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(run_application_dump_worker(|| Ok(()))
            .await
            .unwrap_err()
            .contains("already running"));
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match run_application_dump_worker(|| Ok(())).await {
                    Ok(()) => break,
                    Err(error) if error.contains("already running") => {
                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                    Err(error) => panic!("unexpected dump worker failure: {error}"),
                }
            }
        })
        .await
        .unwrap();
    }
}
