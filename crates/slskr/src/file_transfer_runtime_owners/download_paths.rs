use super::*;

pub(crate) async fn configured_download_destination_path(
    state: &AppState,
    relative: &str,
) -> Result<PathBuf, String> {
    let configured_default = state
        .destinations
        .read()
        .await
        .records
        .iter()
        .find(|destination| destination.is_default)
        .map(|destination| PathBuf::from(&destination.path));
    let root = configured_default.unwrap_or_else(|| effective_downloads_dir(state));
    let mut path = safe_download_path(&root, relative)?;
    let settings = state.transfer_download_settings.read().await;
    let rename_existing = state.config.controller_profile == ControllerProfile::Native
        || settings.destination.exists.eq_ignore_ascii_case("rename");
    drop(settings);
    if !rename_existing || !path.exists() {
        return Ok(path);
    }
    let parent = path.parent().unwrap_or(root.as_path()).to_path_buf();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download")
        .to_owned();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let base_ticks =
        621_355_968_000_000_000_u64.saturating_add(unix_timestamp_millis().saturating_mul(10_000));
    for increment in 0..10_000_u64 {
        let candidate = parent.join(format!(
            "{stem}_{}{extension}",
            base_ticks.saturating_add(increment)
        ));
        if !candidate.exists() {
            path = candidate;
            break;
        }
    }
    Ok(path)
}

pub(crate) async fn prepare_transfer_local_path(
    state: &AppState,
    direction: u32,
    peer_username: Option<&str>,
    filename: &str,
    batch_id: Option<&str>,
    details: &TransferRequestDetails,
    supplied_local_path: Option<String>,
) -> Result<Option<String>, String> {
    if direction == 1 {
        if supplied_local_path.is_some() {
            return Err("local_path is not accepted for uploads; use a shared filename".to_owned());
        }
        return find_shared_local_file(state, filename)
            .await
            .map(|shared_file| Some(shared_file.local_path.display().to_string()))
            .ok_or_else(|| "upload filename is not available from local shares".to_owned());
    }

    let relative = if let Some(destination) = details
        .destination_directory
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        format!(
            "{}/{}",
            destination.trim_matches(['/', '\\']),
            virtual_basename(filename)
        )
    } else {
        render_configured_completed_download_path(
            state,
            peer_username.unwrap_or_default(),
            filename,
            batch_id,
            details.request_name.as_deref(),
            unix_timestamp(),
        )
        .await?
    };
    let path = configured_download_destination_path(state, &relative).await?;
    Ok(Some(path.display().to_string()))
}

pub(crate) async fn render_configured_completed_download_path(
    state: &AppState,
    uploader: &str,
    remote_filename: &str,
    batch_id: Option<&str>,
    request_name: Option<&str>,
    requested_at: u64,
) -> Result<String, String> {
    let template = effective_download_completed_path_template(state);
    if !template.trim().is_empty() {
        return render_completed_download_path(
            &template,
            uploader,
            remote_filename,
            batch_id,
            request_name,
            requested_at,
        );
    }
    let settings = state.transfer_download_settings.read().await.clone();
    if state.config.controller_profile == ControllerProfile::Native {
        return match settings.completed_layout.as_str() {
            "flat" => Ok(virtual_basename(remote_filename).to_owned()),
            "uploader_folder" => render_completed_download_path(
                "{uploader}/{remote_parent}",
                uploader,
                remote_filename,
                batch_id,
                request_name,
                requested_at,
            ),
            "batch_id" if batch_id.is_some() => render_completed_download_path(
                "{batch_id}",
                uploader,
                remote_filename,
                batch_id,
                request_name,
                requested_at,
            ),
            _ => render_completed_download_path(
                "",
                uploader,
                remote_filename,
                batch_id,
                request_name,
                requested_at,
            ),
        };
    }

    let pattern = settings
        .destination
        .subdirectory
        .as_deref()
        .unwrap_or("${SOURCE_DIRECTORY}");
    if pattern.eq_ignore_ascii_case("{}") {
        return Ok(virtual_basename(remote_filename).to_owned());
    }
    let normalized = remote_filename.replace('\\', "/");
    let source_path = normalized
        .rsplit_once('/')
        .map_or("", |(directory, _)| directory)
        .trim_end_matches('/');
    let source_directory = source_path.rsplit('/').next().unwrap_or_default();
    let mut destination = String::with_capacity(pattern.len().saturating_add(64));
    let mut rest = pattern;
    while let Some(open) = rest.find("${") {
        destination.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find('}') else {
            destination.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let token = &after[..close];
        let replacement = match token.to_ascii_uppercase().as_str() {
            "SOURCE_USERNAME" => uploader,
            "SOURCE_PATH" => source_path,
            "SOURCE_DIRECTORY" => source_directory,
            "BATCH_ID" => batch_id.unwrap_or("unknown_batch_id"),
            "BATCH_EXTERNAL_ID" => "unknown_batch_external_id",
            "SEARCH_ID" => "unknown_search_id",
            "SEARCH_TEXT" => request_name.unwrap_or("unknown_search_text"),
            _ => &rest[open..open + close + 3],
        };
        destination.push_str(replacement);
        rest = &after[close + 1..];
    }
    destination.push_str(rest);
    render_completed_download_path(
        &destination,
        uploader,
        remote_filename,
        batch_id,
        request_name,
        requested_at,
    )
}

pub(crate) fn render_completed_download_path(
    template: &str,
    uploader: &str,
    remote_filename: &str,
    batch_id: Option<&str>,
    request_name: Option<&str>,
    requested_at: u64,
) -> Result<String, String> {
    if template.trim().is_empty() {
        return Ok(remote_filename.to_owned());
    }
    let normalized = remote_filename.replace('\\', "/");
    let mut remote_parts = normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let basename = remote_parts
        .pop()
        .ok_or_else(|| "download filename is empty".to_owned())?;
    let stem = Path::new(basename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(basename);
    let remote_folder = if remote_parts.is_empty() {
        "_singles".to_owned()
    } else {
        remote_parts.join("/")
    };
    let remote_parent = remote_parts.last().copied().unwrap_or("_singles");
    let requested_at = i64::try_from(requested_at)
        .ok()
        .and_then(|seconds| chrono::DateTime::from_timestamp(seconds, 0))
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    let mut rendered = String::with_capacity(template.len().saturating_add(64));
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        rendered.push_str(&rest[..open]);
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find('}') else {
            return Err("download completed path template has an unclosed token".to_owned());
        };
        let token = &after_open[..close];
        let (name, format) = token.split_once(':').unwrap_or((token, ""));
        let value = match name.to_ascii_lowercase().as_str() {
            "uploader" => uploader.to_owned(),
            "remote_folder" => remote_folder.clone(),
            "remote_parent" => remote_parent.to_owned(),
            "remote_filename" => stem.to_owned(),
            "batch_id" => batch_id.unwrap_or("_no-batch").to_owned(),
            "request_name" => request_name.unwrap_or("_").to_owned(),
            "search_text" => String::new(),
            "date" => {
                let format = if format.is_empty() {
                    "%Y-%m-%d"
                } else {
                    format
                };
                if chrono::format::StrftimeItems::new(format)
                    .any(|item| matches!(item, chrono::format::Item::Error))
                {
                    return Err(
                        "download completed path template has an invalid date format".to_owned(),
                    );
                }
                requested_at.format(format).to_string()
            }
            _ => String::new(),
        };
        if rendered.len().saturating_add(value.len()) > MAX_TRANSFER_LOCAL_PATH_BYTES {
            return Err("download completed path exceeds the local path limit".to_owned());
        }
        rendered.push_str(&value);
        rest = &after_open[close + 1..];
    }
    if rendered.len().saturating_add(rest.len()) > MAX_TRANSFER_LOCAL_PATH_BYTES {
        return Err("download completed path exceeds the local path limit".to_owned());
    }
    rendered.push_str(rest);

    let mut output = Vec::new();
    for segment in rendered.split(['/', '\\']) {
        let segment = segment.trim();
        if segment.is_empty() || matches!(segment, "." | "..") {
            continue;
        }
        let sanitized = segment
            .chars()
            .map(|ch| {
                if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
                    '_'
                } else {
                    ch
                }
            })
            .collect::<String>();
        if !sanitized.is_empty() {
            output.push(sanitized);
        }
    }
    output.push(basename.to_owned());
    let output = output.join("/");
    if output.len() > MAX_TRANSFER_LOCAL_PATH_BYTES {
        return Err("download completed path exceeds the local path limit".to_owned());
    }
    Ok(output)
}

#[cfg(any(test, feature = "bounded-differential"))]
#[allow(dead_code)]
pub(crate) fn download_root(state_dir: &Path) -> PathBuf {
    state_dir.join("downloads")
}

pub(crate) fn safe_download_path(root: &Path, filename: &str) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    let mut appended = false;
    if filename.starts_with(['/', '\\']) {
        return Err(
            "download filename must be relative and stay within the download root".to_owned(),
        );
    }
    for part in filename.split(['/', '\\']) {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(
                "download filename must be relative and stay within the download root".to_owned(),
            );
        }
        let mut components = Path::new(part).components();
        let Some(Component::Normal(component)) = components.next() else {
            return Err(
                "download filename must be relative and stay within the download root".to_owned(),
            );
        };
        if components.next().is_some() {
            return Err(
                "download filename must be relative and stay within the download root".to_owned(),
            );
        }
        path.push(component);
        appended = true;
    }
    if !appended {
        return Err("download filename is empty".to_owned());
    }
    if !path.starts_with(root) {
        return Err("download path escapes the download root".to_owned());
    }
    Ok(path)
}

pub(crate) fn ensure_scoped_download_path(
    root: &Path,
    local_path: &str,
) -> Result<PathBuf, String> {
    let root = root.to_path_buf();
    let path = PathBuf::from(local_path);
    if !path.starts_with(&root) {
        return Err("download path is outside the download root".to_owned());
    }
    fs::create_dir_all(&root).map_err(|error| format!("download root create failed: {error}"))?;
    #[cfg(unix)]
    ensure_scoped_download_parent_unix(&root, &path)?;
    #[cfg(not(unix))]
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("download directory create failed: {error}"))?;
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("download root canonicalize failed: {error}"))?;
    let canonical_parent = path
        .parent()
        .unwrap_or(root.as_path())
        .canonicalize()
        .map_err(|error| format!("download parent canonicalize failed: {error}"))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err("download path escapes the download root".to_owned());
    }
    if path
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("download path must not be a symlink".to_owned());
    }
    Ok(path)
}

#[cfg(unix)]
pub(super) fn ensure_scoped_download_parent_unix(root: &Path, path: &Path) -> Result<(), String> {
    use rustix::fs::{mkdirat, open, openat, Mode, OFlags};

    let relative = path
        .strip_prefix(root)
        .map_err(|_| "download path is outside the download root".to_owned())?;
    let mut components = relative.components().peekable();
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("download root confined open failed: {error}"))?;
    while let Some(component) = components.next() {
        let Component::Normal(component) = component else {
            return Err("download path contains a non-relative component".to_owned());
        };
        if components.peek().is_none() {
            break;
        }
        directory = match openat(&directory, component, directory_flags, Mode::empty()) {
            Ok(child) => child,
            Err(error) if error == rustix::io::Errno::NOENT => {
                match mkdirat(&directory, component, Mode::from_raw_mode(0o700)) {
                    Ok(()) => {}
                    Err(error) if error == rustix::io::Errno::EXIST => {}
                    Err(error) => {
                        return Err(format!(
                            "download directory confined create failed: {error}"
                        ));
                    }
                }
                openat(&directory, component, directory_flags, Mode::empty())
                    .map_err(|error| format!("download directory confined open failed: {error}"))?
            }
            Err(error) => {
                return Err(format!("download directory confined open failed: {error}"));
            }
        };
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn open_download_file(root: &Path, path: &Path) -> Result<fs::File, String> {
    use rustix::fs::{open, openat, Mode, OFlags};

    let relative = path
        .strip_prefix(root)
        .map_err(|_| "download path is outside the download root".to_owned())?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err("download path contains a non-relative component".to_owned()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (filename, parents) = components
        .split_last()
        .ok_or_else(|| "download filename is empty".to_owned())?;
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("download root open failed: {error}"))?;
    for component in parents {
        directory = openat(&directory, *component, directory_flags, Mode::empty())
            .map_err(|error| format!("download directory confined open failed: {error}"))?;
    }
    let fd = openat(
        &directory,
        *filename,
        OFlags::WRONLY | OFlags::CREATE | OFlags::APPEND | OFlags::CLOEXEC | OFlags::NOFOLLOW,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| format!("download file confined open failed: {error}"))?;
    Ok(fs::File::from(fd))
}

#[cfg(not(unix))]
pub(crate) fn open_download_file(_root: &Path, path: &Path) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    // Windows rejects SetEndOfFile on an append-only handle; retain append
    // semantics while requesting read/write access for retry truncation.
    options.read(true).write(true).create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options
        .open(path)
        .map_err(|error| format!("download file open failed: {error}"))
}

#[cfg(unix)]
pub(crate) fn open_download_file_for_read(root: &Path, path: &Path) -> Result<fs::File, String> {
    use rustix::fs::{open, openat, Mode, OFlags};

    let relative = path
        .strip_prefix(root)
        .map_err(|_| "download path is outside the download root".to_owned())?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err("download path contains a non-relative component".to_owned()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (filename, parents) = components
        .split_last()
        .ok_or_else(|| "download filename is empty".to_owned())?;
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("download root confined open failed: {error}"))?;
    for component in parents {
        directory = openat(&directory, *component, directory_flags, Mode::empty())
            .map_err(|error| format!("download directory confined open failed: {error}"))?;
    }
    let fd = openat(
        &directory,
        *filename,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| format!("download file confined open failed: {error}"))?;
    Ok(fs::File::from(fd))
}

#[cfg(not(unix))]
pub(crate) fn open_download_file_for_read(root: &Path, path: &Path) -> Result<fs::File, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("download root canonicalize failed: {error}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("download file canonicalize failed: {error}"))?;
    if !canonical_path.starts_with(canonical_root) {
        return Err("download path is outside the download root".to_owned());
    }
    fs::OpenOptions::new()
        .read(true)
        .open(canonical_path)
        .map_err(|error| format!("download file open failed: {error}"))
}
