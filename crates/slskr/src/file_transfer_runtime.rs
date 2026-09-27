use super::transfer_queue::AudioTechnicalMetadata;
use super::*;
use std::io::Read;

pub(super) async fn configured_download_destination_path(
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

pub(super) async fn prepare_transfer_local_path(
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

pub(super) async fn render_configured_completed_download_path(
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

pub(super) fn render_completed_download_path(
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
pub(super) fn download_root(state_dir: &Path) -> PathBuf {
    state_dir.join("downloads")
}

pub(super) fn safe_download_path(root: &Path, filename: &str) -> Result<PathBuf, String> {
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

pub(super) fn ensure_scoped_download_path(
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
fn ensure_scoped_download_parent_unix(root: &Path, path: &Path) -> Result<(), String> {
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
pub(super) fn open_download_file(root: &Path, path: &Path) -> Result<fs::File, String> {
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
pub(super) fn open_download_file(_root: &Path, path: &Path) -> Result<fs::File, String> {
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
pub(super) fn open_download_file_for_read(root: &Path, path: &Path) -> Result<fs::File, String> {
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
pub(super) fn open_download_file_for_read(root: &Path, path: &Path) -> Result<fs::File, String> {
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

pub(super) async fn transfer_capacity_available(
    state: &AppState,
    excluding_id: Option<u64>,
) -> bool {
    if state.config.transfer_max_active == 0 {
        return false;
    }
    let transfers = state.transfers.read().await;
    transfers.active_count_excluding(excluding_id) < state.config.transfer_max_active
}

pub(super) async fn download_capacity_available(
    state: &AppState,
    excluding_id: Option<u64>,
) -> bool {
    if !transfer_capacity_available(state, excluding_id).await {
        return false;
    }
    let slots = state.transfer_download_settings.read().await.slots;
    let transfers = state.transfers.read().await;
    let active = transfers
        .entries
        .iter()
        .filter(|entry| entry.id != excluding_id.unwrap_or(u64::MAX))
        .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
        .count();
    active < usize::try_from(slots).unwrap_or(usize::MAX)
}

pub(super) fn effective_transfer_group_from(
    settings: &crate::config::TransferGroupsSettings,
    users: &UserStore,
    username: &str,
) -> String {
    // Matches the oracle's real UserService.GetGroup: a blacklist decision is
    // only made for a username already present in the user cache. Unknown
    // usernames return the default group, even when the configured blacklist
    // would match them; once cached, the blacklist wins over every other
    // classification, including privileged status.
    if users
        .records
        .iter()
        .any(|record| record.username == username)
        && settings
            .blacklisted
            .members
            .iter()
            .any(|member| member.eq_ignore_ascii_case(username))
    {
        return "blacklisted".to_owned();
    }
    if users
        .records
        .iter()
        .find(|record| record.username == username)
        .is_some_and(|record| record.privileged)
    {
        return "privileged".to_owned();
    }
    if let Some((name, _)) = settings
        .user_defined
        .iter()
        .filter(|(_, group)| group.members.iter().any(|member| member == username))
        .min_by(|(left_name, left), (right_name, right)| {
            left.upload
                .priority
                .cmp(&right.upload.priority)
                .then_with(|| left_name.cmp(right_name))
        })
    {
        return name.clone();
    }
    if users
        .records
        .iter()
        .find(|record| record.username == username)
        .is_some_and(|record| {
            record
                .file_count
                .is_some_and(|count| count < settings.leechers.threshold_files)
                || record
                    .directory_count
                    .is_some_and(|count| count < settings.leechers.threshold_directories)
        })
    {
        "leechers".to_owned()
    } else {
        "default".to_owned()
    }
}

pub(super) async fn effective_transfer_group(state: &AppState, username: &str) -> String {
    let settings = state.transfer_groups_settings.read().await;
    let users = state.users.read().await;
    effective_transfer_group_from(&settings, &users, username)
}

pub(super) fn transfer_group_upload_settings<'a>(
    settings: &'a crate::config::TransferGroupsSettings,
    group: &str,
) -> Option<&'a crate::config::TransferGroupUploadSettings> {
    match group {
        "privileged" => None,
        "default" => Some(&settings.default.upload),
        "leechers" => Some(&settings.leechers.upload),
        name => settings.user_defined.get(name).map(|group| &group.upload),
    }
}

#[derive(Default)]
struct UserUploadLimitStatistics {
    queued_files: u64,
    queued_bytes: u64,
    weekly_failed_files: u64,
    weekly_succeeded_files: u64,
    weekly_succeeded_bytes: u64,
    daily_failed_files: u64,
    daily_succeeded_files: u64,
    daily_succeeded_bytes: u64,
}

fn user_upload_limit_statistics(
    transfers: &TransferQueue,
    username: &str,
    now: u64,
) -> UserUploadLimitStatistics {
    let mut stats = UserUploadLimitStatistics::default();
    let daily_cutoff = now.saturating_sub(24 * 60 * 60);
    let weekly_cutoff = now.saturating_sub(7 * 24 * 60 * 60);
    for entry in transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && entry.peer_username.as_deref() == Some(username))
    {
        if !is_terminal_transfer_status(&entry.status) {
            stats.queued_files = stats.queued_files.saturating_add(1);
            stats.queued_bytes = stats.queued_bytes.saturating_add(entry.size.unwrap_or(0));
        }
        let Some(started_at) = entry.started_at else {
            continue;
        };
        let failed = is_failed_transfer_status(&entry.status) || entry.status == "cancelled";
        let succeeded = is_successful_transfer_status(&entry.status);
        if started_at >= weekly_cutoff {
            if failed {
                stats.weekly_failed_files = stats.weekly_failed_files.saturating_add(1);
            } else if succeeded {
                stats.weekly_succeeded_files = stats.weekly_succeeded_files.saturating_add(1);
                stats.weekly_succeeded_bytes = stats
                    .weekly_succeeded_bytes
                    .saturating_add(entry.size.unwrap_or(0));
            }
        }
        if started_at >= daily_cutoff {
            if failed {
                stats.daily_failed_files = stats.daily_failed_files.saturating_add(1);
            } else if succeeded {
                stats.daily_succeeded_files = stats.daily_succeeded_files.saturating_add(1);
                stats.daily_succeeded_bytes = stats
                    .daily_succeeded_bytes
                    .saturating_add(entry.size.unwrap_or(0));
            }
        }
    }
    stats
}

fn effective_limit_value(
    group: Option<&crate::config::TransferLimitSettings>,
    global: Option<&crate::config::TransferLimitSettings>,
    field: fn(&crate::config::TransferLimitSettings) -> Option<u32>,
) -> Option<u32> {
    group.and_then(field).or_else(|| global.and_then(field))
}

fn transfer_window_limit_error(
    group: Option<&crate::config::TransferLimitSettings>,
    global: Option<&crate::config::TransferLimitSettings>,
    files: u64,
    bytes: u64,
    requested_size: u64,
    suffix: &str,
    megabytes_first: bool,
) -> Option<String> {
    let file_over = effective_limit_value(group, global, |limit| limit.files)
        .is_some_and(|limit| files.saturating_add(1) > u64::from(limit));
    let megabytes_over =
        effective_limit_value(group, global, |limit| limit.megabytes).is_some_and(|limit| {
            bytes.saturating_add(requested_size) > u64::from(limit).saturating_mul(1_000_000)
        });
    if !file_over && !megabytes_over {
        return None;
    }
    let unit = if (megabytes_first && megabytes_over) || (!megabytes_first && !file_over) {
        "megabytes"
    } else {
        "files"
    };
    Some(format!("Too many {unit}{suffix}"))
}

pub(super) async fn inbound_upload_policy(
    state: &AppState,
    username: &str,
    filename: &str,
    requested_size: u64,
) -> Result<String, String> {
    if state
        .failed_upload_peer_cooldowns
        .write()
        .await
        .remaining(username, unix_timestamp())
        .is_some()
    {
        return Err("Recent transfer failed; retry later.".to_owned());
    }
    if !state.config.transfer_allow_inbound {
        return Err("inbound transfers are disabled".to_owned());
    }
    if !transfer_capacity_available(state, None).await {
        return Err("transfer limit reached".to_owned());
    }
    let group_name = effective_transfer_group(state, username).await;
    let privileged = group_name == "privileged";
    let groups = state.transfer_groups_settings.read().await;
    let upload = state.transfer_upload_settings.read().await;
    let group =
        transfer_group_upload_settings(&groups, &group_name).unwrap_or(&groups.default.upload);
    if !privileged
        && state.config.controller_profile == ControllerProfile::Native
        && !group.allowed_file_types.is_empty()
    {
        let extension = Path::new(filename)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| format!(".{extension}"))
            .unwrap_or_default();
        if !group
            .allowed_file_types
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(&extension))
        {
            return Err(format!("File type {extension} is not permitted."));
        }
    }
    let transfers = state.transfers.read().await;
    let stats = user_upload_limit_statistics(&transfers, username, unix_timestamp());
    if !privileged {
        if let Some(error) = transfer_window_limit_error(
            group.limits.queued.as_ref(),
            upload.limits.queued.as_ref(),
            stats.queued_files,
            stats.queued_bytes,
            requested_size,
            "",
            true,
        ) {
            return Err(error);
        }
        if effective_limit_value(
            group.limits.weekly.as_ref(),
            upload.limits.weekly.as_ref(),
            |limit| limit.failures,
        )
        .is_some_and(|limit| stats.weekly_failed_files >= u64::from(limit))
        {
            return Err("Too many failed transfers this week".to_owned());
        }
        if let Some(error) = transfer_window_limit_error(
            group.limits.weekly.as_ref(),
            upload.limits.weekly.as_ref(),
            stats.weekly_succeeded_files,
            stats.weekly_succeeded_bytes,
            requested_size,
            " this week",
            false,
        ) {
            return Err(error);
        }
        if effective_limit_value(
            group.limits.daily.as_ref(),
            upload.limits.daily.as_ref(),
            |limit| limit.failures,
        )
        .is_some_and(|limit| stats.daily_failed_files >= u64::from(limit))
        {
            return Err("Too many failed transfers today".to_owned());
        }
        if let Some(error) = transfer_window_limit_error(
            group.limits.daily.as_ref(),
            upload.limits.daily.as_ref(),
            stats.daily_succeeded_files,
            stats.daily_succeeded_bytes,
            requested_size,
            " today",
            false,
        ) {
            return Err(error);
        }
    }
    let active_uploads = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .count();
    if active_uploads >= usize::try_from(upload.slots).unwrap_or(usize::MAX) {
        return Err("Queued".to_owned());
    }
    let users = state.users.read().await;
    let group_active = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .filter(|entry| {
            entry.peer_username.as_deref().is_some_and(|peer| {
                effective_transfer_group_from(&groups, &users, peer) == group_name
            })
        })
        .count();
    let group_slots = if privileged {
        upload.slots
    } else {
        group.slots.min(upload.slots)
    };
    if group_active >= usize::try_from(group_slots).unwrap_or(usize::MAX) {
        return Err("Queued".to_owned());
    }
    Ok(group_name)
}

pub(super) async fn probe_peer_capability(
    state: &AppState,
    username: &str,
) -> Result<PeerCapabilityDescriptor, String> {
    let mesh_settings = state.advanced_networking.read().await.mesh.clone();
    if !mesh_settings.enabled
        || !mesh_settings.enable_soulseek_rendezvous
        || !mesh_settings.probe_soulseek_rendezvous_capabilities
        || !mesh_settings.enable_soulseek_capability_handshake
    {
        return Err("Soulseek mesh capability probing is disabled by configuration".to_owned());
    }
    let username = username.trim();
    if username.is_empty() {
        return Err("peer capability username is required".to_owned());
    }
    let address = request_peer_endpoint(state, username).await?;
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let hello = PeerCapabilityEnvelope::new(
        PeerCapabilityMessageType::Hello,
        nonce.clone(),
        local_capability_descriptor(state).await?,
    );
    let message = peer_capability_message(&hello)
        .map_err(|error| format!("peer capability hello failed: {error}"))?;
    let response = send_peer_message_request(state, &address, message).await?;
    let Some(mut acknowledgement) = decode_peer_capability_message(&response)
        .map_err(|error| format!("peer capability acknowledgement rejected: {error}"))?
    else {
        return Err(format!(
            "expected peer capability acknowledgement, got {}",
            peer_message_name(&response)
        ));
    };
    if acknowledgement.message_type != PeerCapabilityMessageType::Acknowledge {
        return Err("peer capability response was not an acknowledgement".to_owned());
    }
    if acknowledgement.nonce != nonce {
        return Err("peer capability acknowledgement nonce did not match".to_owned());
    }
    acknowledgement.descriptor.username = username.to_owned();
    let descriptor = acknowledgement.descriptor;
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor.clone())?;
    update_listeners(state, |snapshot| {
        snapshot.last_event = Some(format!(
            "peer_capability_acknowledge:{}",
            redact_username(username)
        ));
        snapshot.last_error = None;
    })
    .await;
    Ok(descriptor)
}

pub(super) async fn execute_accepted_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) {
    if cancel_download_if_blocked_by_policy(state, transfer)
        .await
        .is_some()
    {
        return;
    }
    if transfer
        .local_path
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return;
    }

    {
        let mut transfers = state.transfers.write().await;
        if transfers
            .entries
            .iter()
            .find(|entry| entry.id == transfer.id)
            .is_some_and(|entry| entry.status == "cancelled")
        {
            return;
        }
        transfers.update_status(transfer.id, "in_progress", None, None);
    }
    persist_transfer_durability(state).await;

    let result = if transfer.direction == 1 {
        upload_file_transfer(state, address, transfer).await
    } else {
        download_file_transfer_with_retry(state, address, transfer).await
    };
    if transfer_is_cancelled(state, transfer.id).await {
        return;
    }
    if let Err(error) = &result {
        record_expected_upload_failure(state, transfer, error).await;
    }
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => {
            if let Some(username) = transfer.peer_username.clone() {
                let indirect_pending = {
                    let mut transfers = state.transfers.write().await;
                    transfers.update_status(
                        transfer.id,
                        "indirect_pending",
                        None,
                        Some(format!("direct file-transfer failed: {error}")),
                    )
                };
                if let Some(indirect_pending) = indirect_pending {
                    persist_transfer_projection(state, &indirect_pending).await;
                }
                if let Err(error) = try_send_session_command(
                    state,
                    SessionCommand::IndirectTransfer {
                        id: transfer.id,
                        username,
                        token: transfer.token,
                    },
                ) {
                    let failed = {
                        let mut transfers = state.transfers.write().await;
                        transfers.update_status(
                            transfer.id,
                            "failed",
                            None,
                            Some(format!("indirect transfer dispatch failed: {error}")),
                        )
                    };
                    if let Some(failed) = failed {
                        persist_transfer_projection(state, &failed).await;
                    }
                    update_session(state, |snapshot| {
                        snapshot.last_error = Some(format!(
                            "indirect transfer {} dispatch failed: {error}",
                            transfer.id
                        ));
                    })
                    .await;
                }
                return;
            }
            (
                "failed",
                transfer.bytes_transferred,
                transfer.size,
                Some(error),
            )
        }
    };

    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason)
    };
    if let Some(updated) = updated {
        let updated = apply_completed_download_permissions(state, updated).await;
        let updated = enrich_completed_audio_metadata(state, updated).await;
        persist_transfer_projection(state, &updated).await;
        issue_relay_download_tokens(state, &updated).await;
        maybe_import_lidarr_completed_download(state, &updated).await;
        maybe_upload_ftp_completed_download(state, &updated).await;
    }
    if transfer.direction == 1 {
        schedule_queued_uploads(state).await;
    } else {
        schedule_queued_downloads(state).await;
    }
}

pub(super) async fn project_indirect_transfer_response(
    state: &AppState,
    response: &ConnectToPeerResponse,
) {
    let Ok(kind) = ConnectionKind::try_from_connection_type(&response.connection_type) else {
        return;
    };
    if kind != ConnectionKind::FileTransfer {
        return;
    }
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_indirect_transfer(&response.username, response.token)
    };
    let Some(transfer) = transfer else {
        return;
    };
    let in_progress = {
        let mut transfers = state.transfers.write().await;
        if transfers
            .entries
            .iter()
            .find(|entry| entry.id == transfer.id)
            .is_some_and(|entry| entry.status == "cancelled")
        {
            return;
        }
        transfers.update_status(transfer.id, "in_progress", None, None)
    };
    if let Some(in_progress) = in_progress {
        persist_transfer_projection(state, &in_progress).await;
    }

    let result = execute_indirect_file_transfer(state, response, &transfer).await;
    if transfer_is_cancelled(state, transfer.id).await {
        return;
    }
    if let Err(error) = &result {
        record_expected_upload_failure(state, &transfer, error).await;
    }
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => (
            "failed",
            transfer.bytes_transferred,
            transfer.size,
            Some(error),
        ),
    };
    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason)
    };
    if let Some(updated) = updated {
        let updated = enrich_completed_audio_metadata(state, updated).await;
        persist_transfer_projection(state, &updated).await;
        issue_relay_download_tokens(state, &updated).await;
        maybe_import_lidarr_completed_download(state, &updated).await;
        maybe_upload_ftp_completed_download(state, &updated).await;
    }
    if transfer.direction == 1 {
        schedule_queued_uploads(state).await;
    } else {
        schedule_queued_downloads(state).await;
    }
}

pub(super) async fn project_peer_transfer_response(state: &AppState, address: &PeerAddress) {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_peer_transfer(&address.username)
    };
    let Some(transfer) = transfer else {
        return;
    };
    if cancel_download_if_blocked_by_policy(state, &transfer)
        .await
        .is_some()
    {
        return;
    }
    if transfer.direction == 0 && !download_capacity_available(state, Some(transfer.id)).await {
        let queued = {
            let mut transfers = state.transfers.write().await;
            transfers.update_status(
                transfer.id,
                "queued",
                None,
                Some("Local download slots exhausted".to_owned()),
            )
        };
        if let Some(queued) = queued {
            persist_transfer_projection(state, &queued).await;
        }
        return;
    }

    let negotiating = {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "peer_negotiating", None, None)
    };
    if let Some(negotiating) = negotiating {
        persist_transfer_projection(state, &negotiating).await;
    }

    let result = negotiate_peer_transfer(state, address, &transfer).await;
    let (status, bytes_transferred, reason) = match result {
        Ok(PeerTransferNegotiation::Allowed { token, size }) if token == transfer.token => {
            let transferred = transfer.bytes_transferred;
            let size = size.or(transfer.size);
            let accepted = {
                let mut transfers = state.transfers.write().await;
                transfers.update_local_execution(transfer.id, "accepted", transferred, size, None)
            };
            if let Some(accepted) = accepted {
                persist_transfer_projection(state, &accepted).await;
                execute_accepted_file_transfer(state, address, &accepted).await;
            }
            return;
        }
        Ok(PeerTransferNegotiation::QueuedInbound { token, size }) => {
            let accepted_result = {
                let mut transfers = state.transfers.write().await;
                transfers.accept_queued_inbound_negotiation(transfer.id, token, size)
            };
            let accepted = match accepted_result {
                Ok(accepted) => accepted,
                Err(error) => {
                    let failed = {
                        let mut transfers = state.transfers.write().await;
                        transfers.update_status(transfer.id, "failed", None, Some(error))
                    };
                    if let Some(failed) = failed {
                        persist_transfer_projection(state, &failed).await;
                    }
                    return;
                }
            };
            if let Some(accepted) = accepted {
                persist_transfer_projection(state, &accepted).await;
            } else {
                let failed = {
                    let mut transfers = state.transfers.write().await;
                    transfers.update_status(
                        transfer.id,
                        "failed",
                        None,
                        Some("queued transfer is no longer pending".to_owned()),
                    )
                };
                if let Some(failed) = failed {
                    persist_transfer_projection(state, &failed).await;
                }
            }
            return;
        }
        Ok(PeerTransferNegotiation::Allowed { token, .. }) => (
            "failed",
            None,
            Some(format!(
                "transfer token mismatch: expected {}, received {token}",
                transfer.token
            )),
        ),
        Ok(PeerTransferNegotiation::Rejected { token, reason }) if token == transfer.token => {
            if is_remote_queue_response(&reason) {
                ("queued", None, Some(reason))
            } else {
                ("failed", None, Some(reason))
            }
        }
        Ok(PeerTransferNegotiation::Rejected { token, .. }) => (
            "failed",
            None,
            Some(format!(
                "transfer token mismatch: expected {}, received {token}",
                transfer.token
            )),
        ),
        Err(error) => ("failed", None, Some(error)),
    };

    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, status, bytes_transferred, reason)
    };
    if let Some(updated) = updated {
        persist_transfer_projection(state, &updated).await;
    }
    if transfer.direction == 1 && status != "queued" {
        schedule_queued_uploads(state).await;
    } else if transfer.direction == 0 && status != "queued" {
        schedule_queued_downloads(state).await;
    }
}

pub(super) async fn schedule_queued_downloads(state: &AppState) {
    loop {
        if !download_capacity_available(state, None).await {
            return;
        }
        let queued = {
            let mut transfers = state.transfers.write().await;
            let next = transfers
                .entries
                .iter()
                .filter(|entry| {
                    entry.direction == 0
                        && entry.status == "queued"
                        && entry.reason.as_deref() == Some("Local download slots exhausted")
                })
                .min_by_key(|entry| (entry.requested_at, entry.id))
                .cloned();
            next.and_then(|entry| transfers.update_status(entry.id, "peer_lookup", None, None))
        };
        let Some(queued) = queued else {
            return;
        };
        persist_transfer_projection(state, &queued).await;
        let Some(username) = queued.peer_username.as_deref() else {
            continue;
        };
        if let Some(address) = cached_peer_endpoint(state, username).await {
            Box::pin(project_peer_transfer_response(state, &address)).await;
        } else if try_send_session_command(
            state,
            SessionCommand::TransferPeer {
                id: queued.id,
                username: username.to_owned(),
            },
        )
        .is_err()
        {
            let failed = state.transfers.write().await.update_status(
                queued.id,
                "failed",
                None,
                Some("download peer lookup dispatch failed".to_owned()),
            );
            if let Some(failed) = failed {
                persist_transfer_projection(state, &failed).await;
            }
        }
    }
}

pub(super) async fn apply_completed_download_permissions(
    state: &AppState,
    transfer: TransferEntry,
) -> TransferEntry {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return transfer;
    }
    let mode = state
        .transfer_download_settings
        .read()
        .await
        .destination
        .permissions_mode
        .clone()
        .or_else(|| state.config.permissions_file_mode.clone());
    let (Some(mode), Some(local_path)) = (mode, transfer.local_path.as_deref()) else {
        return transfer;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(mode) = u32::from_str_radix(&mode, 8) else {
            return transfer;
        };
        let root = effective_downloads_dir(state);
        let Ok(path) = ensure_scoped_download_path(&root, local_path) else {
            return transfer;
        };
        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
            let failed = state.transfers.write().await.update_status(
                transfer.id,
                "failed",
                None,
                Some(format!("download destination permissions failed: {error}")),
            );
            return failed.unwrap_or(transfer);
        }
        if let Some(parent) = path.parent().filter(|parent| *parent != root) {
            let directory_mode =
                mode | ((mode & 0o400) >> 2) | ((mode & 0o040) >> 2) | ((mode & 0o004) >> 2);
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(directory_mode));
        }
    }
    transfer
}

pub(super) async fn enrich_completed_audio_metadata(
    state: &AppState,
    transfer: TransferEntry,
) -> TransferEntry {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return transfer;
    }
    let Some(local_path) = transfer.local_path.as_deref() else {
        return transfer;
    };
    let root = effective_downloads_dir(state);
    let Ok(path) = ensure_scoped_download_path(&root, local_path) else {
        return transfer;
    };

    // Matches native profile's completed-download HashDb pipeline: hash the first
    // 32 KiB of supported audio files, store the real byte hash under the
    // shared FLAC key, and expose each stage through the metadata activity
    // endpoint. Unsupported and undersized files are recorded as skipped,
    // not silently presented as an empty pipeline.
    let filename = transfer.filename.clone();
    let display_filename = Path::new(&filename)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(filename.as_str())
        .to_owned();
    let hash_stage = {
        let discovery = state.content_discovery.read().await;
        discovery.begin_metadata_stage(&display_filename, "hash")
    };
    if !is_audio_hash_candidate(&filename) {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "skipped",
            Some("Not a supported audio file".to_owned()),
        );
        return transfer;
    }
    let Ok(hash_file) = open_download_file_for_read(&root, &path) else {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "failed",
            Some("Completed file could not be opened".to_owned()),
        );
        return transfer;
    };
    let file_size = hash_file.metadata().map(|metadata| metadata.len()).ok();
    let Some(file_size) = file_size else {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "failed",
            Some("Completed file metadata could not be read".to_owned()),
        );
        return transfer;
    };
    if file_size < METADATA_HASH_CHUNK_SIZE as u64 {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "skipped",
            Some("File is too small for hashing".to_owned()),
        );
        return transfer;
    }
    let byte_hash = tokio::task::spawn_blocking(move || read_file_prefix_hash(hash_file))
        .await
        .ok()
        .flatten();
    let Some(byte_hash) = byte_hash else {
        let discovery = state.content_discovery.read().await;
        discovery.finish_metadata_stage(
            hash_stage,
            "failed",
            Some("Hash computation failed".to_owned()),
        );
        return transfer;
    };
    let flac_key = content_discovery::generate_flac_key(&filename, file_size);
    let persistence_turn = hash_db_persistence_turn().await;
    let (merge_result, previous_entries, previous_latest_seq, mutated_entries, mutated_latest_seq) = {
        let mut discovery = state.content_discovery.write().await;
        let previous_entries = discovery.hash_entries().to_vec();
        let previous_latest_seq = discovery.latest_seq();
        let merge_result = discovery.merge_hash_entries(vec![content_discovery::HashDbEntry {
            flac_key,
            byte_hash,
            size: file_size,
            ..content_discovery::HashDbEntry::default()
        }]);
        let mutated_entries = discovery.hash_entries().to_vec();
        let mutated_latest_seq = discovery.latest_seq();
        (
            merge_result,
            previous_entries,
            previous_latest_seq,
            mutated_entries,
            mutated_latest_seq,
        )
    };
    let merge_error = match merge_result {
        Ok(_) => match persist_current_hash_db_snapshot(state, &persistence_turn).await {
            Ok(()) => None,
            Err(error) => {
                rollback_hash_db_entries_if_unchanged(
                    state,
                    previous_entries,
                    previous_latest_seq,
                    &mutated_entries,
                    mutated_latest_seq,
                )
                .await;
                Some(error)
            }
        },
        Err(error) => Some(error),
    };
    drop(persistence_turn);
    let discovery = state.content_discovery.read().await;
    if let Some(error) = merge_error {
        discovery.finish_metadata_stage(hash_stage, "failed", Some(error));
        return transfer;
    }
    discovery.finish_metadata_stage(hash_stage, "complete", Some("Hash stored".to_owned()));
    let chromaprint_stage = discovery.begin_metadata_stage(&display_filename, "chromaprint");
    discovery.finish_metadata_stage(
        chromaprint_stage,
        "skipped",
        Some("Chromaprint is not configured".to_owned()),
    );
    drop(discovery);

    let Ok(file) = open_download_file_for_read(&root, &path) else {
        return transfer;
    };
    let metadata =
        tokio::task::spawn_blocking(move || read_audio_technical_metadata(file, &filename))
            .await
            .ok()
            .flatten();
    let Some(metadata) = metadata else {
        return transfer;
    };
    let mut transfers = state.transfers.write().await;
    transfers
        .update_audio_metadata(transfer.id, metadata)
        .unwrap_or(transfer)
}

const METADATA_HASH_CHUNK_SIZE: usize = 32 * 1024;

fn is_audio_hash_candidate(filename: &str) -> bool {
    matches!(
        Path::new(filename)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "aac" | "flac" | "m4a" | "mp3" | "ogg" | "opus" | "wav" | "wave"
    )
}

fn read_file_prefix_hash(mut file: fs::File) -> Option<String> {
    use sha2::Digest as _;
    use std::io::Read;

    let mut prefix = vec![0_u8; METADATA_HASH_CHUNK_SIZE];
    let bytes_read = file.read(&mut prefix).ok()?;
    (bytes_read > 0).then(|| hex::encode(Sha256::digest(&prefix[..bytes_read])))
}

fn read_audio_technical_metadata(
    mut file: fs::File,
    filename: &str,
) -> Option<AudioTechnicalMetadata> {
    use std::io::Read;

    const MAX_AUDIO_HEADER_BYTES: u64 = 1024 * 1024;
    let file_size = file.metadata().ok()?.len();
    let mut header = Vec::new();
    file.by_ref()
        .take(MAX_AUDIO_HEADER_BYTES)
        .read_to_end(&mut header)
        .ok()?;
    let extension = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "flac" => flac_technical_metadata(&header, file_size),
        "mp3" => mp3_technical_metadata(&header, file_size),
        "wav" | "wave" => wav_technical_metadata(&header),
        _ => None,
    }
}

fn flac_technical_metadata(header: &[u8], file_size: u64) -> Option<AudioTechnicalMetadata> {
    if header.get(..4)? != b"fLaC" {
        return None;
    }
    let block_header = header.get(4..8)?;
    if block_header[0] & 0x7f != 0 {
        return None;
    }
    let length = usize::from(block_header[1]) << 16
        | usize::from(block_header[2]) << 8
        | usize::from(block_header[3]);
    if length < 34 {
        return None;
    }
    let stream_info = header.get(8..8 + length)?;
    let packed = u64::from_be_bytes(stream_info.get(10..18)?.try_into().ok()?);
    let sample_rate = u32::try_from((packed >> 44) & 0x000f_ffff).ok()?;
    let bit_depth = u32::try_from(((packed >> 36) & 0x1f) + 1).ok()?;
    let total_samples = packed & 0x0000_000f_ffff_ffff;
    if sample_rate == 0 || total_samples == 0 {
        return None;
    }
    let length_seconds = u32::try_from(total_samples / u64::from(sample_rate)).ok()?;
    let bit_rate = (length_seconds > 0)
        .then(|| file_size.saturating_mul(8) / u64::from(length_seconds) / 1000)
        .and_then(|value| u32::try_from(value).ok());
    Some(AudioTechnicalMetadata {
        bit_rate,
        sample_rate: Some(sample_rate),
        bit_depth: Some(bit_depth),
        length_seconds: Some(length_seconds),
    })
}

pub(super) fn mp3_technical_metadata(
    header: &[u8],
    file_size: u64,
) -> Option<AudioTechnicalMetadata> {
    const MPEG1_LAYER3: [u32; 16] = [
        0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
    ];
    const MPEG2_LAYER3: [u32; 16] = [
        0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
    ];
    let frame = header.windows(4).find(|frame| {
        frame[0] == 0xff
            && frame[1] & 0xe0 == 0xe0
            && (frame[1] >> 1) & 0x03 == 0x01
            && frame[2] >> 4 != 0
            && frame[2] >> 4 != 0x0f
            && (frame[2] >> 2) & 0x03 != 0x03
    })?;
    let version = (frame[1] >> 3) & 0x03;
    if version == 1 {
        return None;
    }
    let bitrate_index = usize::from(frame[2] >> 4);
    let bit_rate = if version == 3 {
        MPEG1_LAYER3[bitrate_index]
    } else {
        MPEG2_LAYER3[bitrate_index]
    };
    let sample_index = usize::from((frame[2] >> 2) & 0x03);
    let base_sample_rate = [44_100_u32, 48_000, 32_000][sample_index];
    let sample_rate = match version {
        3 => base_sample_rate,
        2 => base_sample_rate / 2,
        0 => base_sample_rate / 4,
        _ => return None,
    };
    if bit_rate == 0 || sample_rate == 0 {
        return None;
    }
    let length_seconds = u32::try_from(file_size.saturating_mul(8) / (u64::from(bit_rate) * 1000))
        .ok()
        .filter(|value| *value > 0);
    Some(AudioTechnicalMetadata {
        bit_rate: Some(bit_rate),
        sample_rate: Some(sample_rate),
        bit_depth: None,
        length_seconds,
    })
}

pub(super) fn wav_technical_metadata(header: &[u8]) -> Option<AudioTechnicalMetadata> {
    if header.get(..4)? != b"RIFF" || header.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut offset = 12_usize;
    let mut sample_rate = None;
    let mut bit_depth = None;
    let mut byte_rate = None;
    let mut data_length = None;
    while offset.saturating_add(8) <= header.len() {
        let kind = header.get(offset..offset + 4)?;
        let length = u32::from_le_bytes(header.get(offset + 4..offset + 8)?.try_into().ok()?);
        let length_usize = usize::try_from(length).ok()?;
        if kind == b"data" {
            data_length = Some(length);
        } else if kind == b"fmt " {
            let payload = header.get(offset + 8..offset + 8 + length_usize)?;
            if payload.len() < 16 {
                return None;
            }
            sample_rate = Some(u32::from_le_bytes(payload.get(4..8)?.try_into().ok()?));
            byte_rate = Some(u32::from_le_bytes(payload.get(8..12)?.try_into().ok()?));
            bit_depth = Some(u32::from(u16::from_le_bytes(
                payload.get(14..16)?.try_into().ok()?,
            )));
        }
        if sample_rate.is_some() && data_length.is_some() {
            break;
        }
        offset = offset.saturating_add(8 + length_usize + (length_usize % 2));
    }
    let byte_rate = byte_rate.filter(|value| *value > 0)?;
    let length_seconds = data_length
        .map(|length| length / byte_rate)
        .filter(|value| *value > 0);
    Some(AudioTechnicalMetadata {
        bit_rate: Some(byte_rate.saturating_mul(8) / 1000),
        sample_rate,
        bit_depth,
        length_seconds,
    })
}

pub(super) async fn fail_indirect_transfer(state: &AppState, token: u32, reason: String) {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| entry.token == token && entry.status == "indirect_pending")
            .cloned()
    };
    if let Some(transfer) = transfer {
        let failed = {
            let mut transfers = state.transfers.write().await;
            transfers.update_status(transfer.id, "failed", None, Some(reason))
        };
        if let Some(failed) = failed {
            persist_transfer_projection(state, &failed).await;
        }
    }
}

pub(super) async fn fail_indirect_browse(state: &AppState, token: u32, reason: String) {
    let _browse_persistence = state.browse_persistence_lock.lock().await;
    let failed = {
        let mut browse = state.browse.write().await;
        browse.fail_indirect(token, reason.clone())
    };
    let persistence_failure = if let Some(record) = failed.as_ref() {
        persist_browse_record_checked(state, record)
            .await
            .err()
            .map(|error| (record.clone(), error))
    } else {
        None
    };
    drop(_browse_persistence);

    if let Some(record) = failed {
        if let Some((failed_record, error)) = persistence_failure {
            super::browse_runtime::report_browse_projection_failure(state, &failed_record, error)
                .await;
        }
        record_event(
            state,
            "browse.failed",
            record.username.clone(),
            Some(reason),
        )
        .await;
    }
}

async fn execute_indirect_file_transfer(
    state: &AppState,
    response: &ConnectToPeerResponse,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    if let Some(reason) = cancel_download_if_blocked_by_policy(state, transfer).await {
        return Err(reason);
    }
    let mut connection = connect_indirect_file_transfer(state, response).await?;
    if transfer.direction == 1 {
        upload_file_transfer_with_connection(state, transfer, &mut connection, true).await
    } else {
        download_file_transfer_with_connection(state, transfer, &mut connection).await
    }
}

async fn upload_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_file_transfer_preferred(state, address).await?;
    upload_file_transfer_with_connection(state, transfer, &mut connection, true).await
}

pub(super) async fn upload_file_transfer_with_connection(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    send_token: bool,
) -> Result<(u64, u64), String> {
    let local_path = transfer
        .local_path
        .as_deref()
        .ok_or_else(|| "local path is required".to_owned())?;
    let shared_file = find_shared_local_file(state, &transfer.filename)
        .await
        .ok_or_else(|| "upload filename is not available from local shares".to_owned())?;
    if Path::new(local_path) != shared_file.local_path {
        return Err("upload local path does not match the share index".to_owned());
    }
    let mut file = open_shared_local_file(state, &shared_file.local_path).await?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("local file metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("local path is not a file".to_owned());
    }
    if let Some(expected_size) = transfer.size.or(Some(shared_file.size)) {
        if metadata.len() != expected_size {
            return Err(format!(
                "local file size {} does not match expected transfer size {expected_size}",
                metadata.len()
            ));
        }
    }
    let size = metadata.len();
    let bytes_transferred =
        upload_file_with_progress(state, transfer, connection, &mut file, size, send_token).await?;
    Ok((bytes_transferred, size))
}

async fn upload_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    file: &mut fs::File,
    size: u64,
    send_token: bool,
) -> Result<u64, String> {
    use std::io::{Read, Seek, SeekFrom};

    if send_token {
        time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            connection.send_token(transfer.token),
        )
        .await
        .map_err(|_| "file upload token send timed out".to_owned())?
        .map_err(|error| format!("file upload token send failed: {error}"))?;
    }
    let offset = time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.receive_offset(),
    )
    .await
    .map_err(|_| "file upload offset receive timed out".to_owned())?
    .map_err(|error| format!("file upload offset receive failed: {error}"))?;
    let start = usize::try_from(offset)
        .map_err(|_| format!("transfer offset {offset} exceeds local file size {size}"))?;
    if u64::try_from(start).unwrap_or(u64::MAX) > size {
        return Err(format!(
            "transfer offset {offset} exceeds local file size {size}"
        ));
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| format!("local file seek failed: {error}"))?;

    // The peer's offset is already present on the remote side.  Report the
    // complete remote position to the transfer projection, while pacing only
    // the bytes sent by this attempt.
    let mut sent = offset;
    update_transfer_progress(state, transfer.id, sent).await;
    let pacing_started = Instant::now();
    let buffer_len = state
        .config
        .soulseek_connection
        .buffer_transfer
        .min(connection.max_write_chunk_len());
    let mut buffer = vec![0_u8; buffer_len];
    loop {
        if transfer_is_cancelled(state, transfer.id).await {
            return Err("transfer cancelled".to_owned());
        }
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("local file read failed: {error}"))?;
        if read == 0 {
            break;
        }
        let chunk = &buffer[..read];
        time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            connection.write_chunk(chunk),
        )
        .await
        .map_err(|_| "file upload chunk send timed out".to_owned())?
        .map_err(|error| format!("file upload chunk send failed: {error}"))?;
        sent = sent.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        update_transfer_progress(state, transfer.id, sent).await;
        let speed_limit_kib = effective_upload_pacing_limit(
            state,
            transfer.peer_username.as_deref().unwrap_or_default(),
        )
        .await;
        if speed_limit_kib < i32::MAX as u32 {
            let bytes_per_second = u64::from(speed_limit_kib).saturating_mul(1024).max(1);
            let expected_nanos = u128::from(sent.saturating_sub(offset))
                .saturating_mul(1_000_000_000)
                .checked_div(u128::from(bytes_per_second))
                .unwrap_or(u128::MAX)
                .min(u128::from(u64::MAX));
            let expected = Duration::from_nanos(expected_nanos as u64);
            let elapsed = pacing_started.elapsed();
            if expected > elapsed {
                time::sleep(expected - elapsed).await;
            }
        }
    }
    Ok(sent)
}

pub(super) async fn effective_upload_speed_limit(state: &AppState, username: &str) -> u32 {
    let global = state.transfer_upload_settings.read().await.speed_limit_kib;
    if username.is_empty() {
        return global;
    }
    let group_name = effective_transfer_group(state, username).await;
    if group_name == "privileged" {
        return global;
    }
    let groups = state.transfer_groups_settings.read().await;
    transfer_group_upload_settings(&groups, &group_name)
        .map_or(global, |group| global.min(group.speed_limit_kib))
}

async fn effective_upload_pacing_limit(state: &AppState, username: &str) -> u32 {
    let configured = effective_upload_speed_limit(state, username).await;
    let global = state.transfer_upload_settings.read().await.speed_limit_kib;
    if configured == i32::MAX as u32 && global == i32::MAX as u32 {
        return configured;
    }
    let group_name = effective_transfer_group(state, username).await;
    let groups = state.transfer_groups_settings.read().await;
    let users = state.users.read().await;
    let transfers = state.transfers.read().await;
    let active = transfers
        .entries
        .iter()
        .filter(|entry| entry.direction == 1 && is_active_transfer_status(&entry.status))
        .collect::<Vec<_>>();
    let global_share = if global == i32::MAX as u32 {
        global
    } else {
        global
            .checked_div(u32::try_from(active.len().max(1)).unwrap_or(u32::MAX))
            .unwrap_or(0)
            .max(1)
    };
    let group_active = active
        .iter()
        .filter(|entry| {
            entry.peer_username.as_deref().is_some_and(|peer| {
                effective_transfer_group_from(&groups, &users, peer) == group_name
            })
        })
        .count();
    let group_share = if configured == i32::MAX as u32 {
        configured
    } else {
        configured
            .checked_div(u32::try_from(group_active.max(1)).unwrap_or(u32::MAX))
            .unwrap_or(0)
            .max(1)
    };
    global_share.min(group_share)
}

async fn download_file_transfer(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    let mut connection = connect_file_transfer_preferred(state, address).await?;
    download_file_transfer_with_connection(state, transfer, &mut connection).await
}

async fn download_file_transfer_with_retry(
    state: &AppState,
    address: &PeerAddress,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    if let Some(reason) = cancel_download_if_blocked_by_policy(state, transfer).await {
        return Err(reason);
    }
    if transfer_is_cancelled(state, transfer.id).await {
        return Err("transfer cancelled".to_owned());
    }
    let retry = state.transfer_download_settings.read().await.retry.clone();
    let mut last_error = None;
    for attempt in 0..retry.attempts.max(1) {
        if attempt > 0 {
            let attempt_number = attempt.saturating_add(1);
            let delay = download_retry_delay(&retry, attempt);
            let next_attempt_at = unix_timestamp().saturating_add(delay.as_secs());
            let scheduled = {
                let mut transfers = state.transfers.write().await;
                transfers.update_retry_state(
                    transfer.id,
                    attempt_number,
                    Some(next_attempt_at),
                    "queued",
                    Some(format!("retry scheduled for attempt {attempt_number}")),
                )
            };
            if let Some(scheduled) = scheduled {
                persist_transfer_projection(state, &scheduled).await;
            }
            time::sleep(delay).await;
        }
        if transfer_is_cancelled(state, transfer.id).await {
            return Err("transfer cancelled".to_owned());
        }
        if attempt > 0 {
            let attempt_number = attempt.saturating_add(1);
            let started = {
                let mut transfers = state.transfers.write().await;
                transfers.update_retry_state(transfer.id, attempt_number, None, "in_progress", None)
            };
            if let Some(started) = started {
                persist_transfer_projection(state, &started).await;
            }
        }
        match download_file_transfer(state, address, transfer).await {
            Ok(completed) => return Ok(completed),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "download retry attempts exhausted".to_owned()))
}

pub(super) fn download_retry_delay(
    retry: &crate::config::TransferDownloadRetrySettings,
    attempt: u32,
) -> Duration {
    let multiplier = 1_u32
        .checked_shl(attempt.saturating_sub(1))
        .unwrap_or(u32::MAX);
    retry
        .delay
        .checked_mul(multiplier)
        .unwrap_or(retry.max_delay)
        .min(retry.max_delay)
}

pub(super) fn prepare_incomplete_download_file(
    root: &Path,
    path: &Path,
    strategy: &str,
    size: u64,
) -> Result<(fs::File, u64), String> {
    let file = open_download_file(root, path)?;
    let file = if strategy.eq_ignore_ascii_case("overwrite") {
        #[cfg(not(unix))]
        {
            drop(file);
            fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(path)
                .map_err(|error| format!("download partial overwrite failed: {error}"))?
        }
        #[cfg(unix)]
        {
            file.set_len(0)
                .map_err(|error| format!("download partial overwrite failed: {error}"))?;
            file
        }
    } else {
        file
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("download file metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("download path is not a regular file".to_owned());
    }
    let offset = metadata.len();
    if offset > size {
        return Err(format!(
            "local resume offset {offset} exceeds transfer size {size}"
        ));
    }
    Ok((file, offset))
}

pub(super) async fn download_file_transfer_with_connection(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
) -> Result<(u64, u64), String> {
    let local_path = transfer
        .local_path
        .as_deref()
        .ok_or_else(|| "local path is required".to_owned())?;
    let size = transfer
        .size
        .ok_or_else(|| "download size is required before file transfer".to_owned())?;
    validate_configured_path_policy(state, local_path).await?;
    let downloads_dir = effective_downloads_dir(state);
    let final_path = ensure_scoped_download_path(&downloads_dir, local_path)?;
    let incomplete_dir = effective_incomplete_dir(state);
    let incomplete_path = safe_download_path(
        &incomplete_dir,
        &format!("{}-{}.part", transfer.id, transfer.token),
    )?;
    let incomplete_path =
        ensure_scoped_download_path(&incomplete_dir, incomplete_path.to_string_lossy().as_ref())?;
    let overwrite_destination = state.config.controller_profile == ControllerProfile::Legacy
        && state
            .transfer_download_settings
            .read()
            .await
            .destination
            .exists
            .eq_ignore_ascii_case("overwrite");
    if !incomplete_path.exists()
        && final_path.exists()
        && !overwrite_destination
        && transfer.bytes_transferred > 0
    {
        fs::rename(&final_path, &incomplete_path)
            .map_err(|error| format!("download partial migration failed: {error}"))?;
    }
    let retry_strategy = state
        .transfer_download_settings
        .read()
        .await
        .retry
        .incomplete
        .clone();
    let (mut file, offset) =
        prepare_incomplete_download_file(&incomplete_dir, &incomplete_path, &retry_strategy, size)?;
    let remaining = usize::try_from(size - offset)
        .map_err(|_| "download remaining size is too large".to_owned())?;
    let bytes_received =
        download_file_with_progress(state, transfer, connection, offset, remaining, &mut file)
            .await?;
    file.sync_all()
        .map_err(|error| format!("download file sync failed: {error}"))?;
    drop(file);
    enforce_completed_download_content_safety(state, &incomplete_path, &final_path).await?;
    let relative_final = final_path
        .strip_prefix(&downloads_dir)
        .map_err(|_| "download destination is outside the download root".to_owned())?;
    let completed_path =
        configured_download_destination_path(state, relative_final.to_string_lossy().as_ref())
            .await?;
    if completed_path.exists() && overwrite_destination {
        fs::remove_file(&completed_path)
            .map_err(|error| format!("download destination overwrite failed: {error}"))?;
    }
    match fs::rename(&incomplete_path, &completed_path) {
        Ok(()) => {}
        Err(rename_error) => {
            fs::copy(&incomplete_path, &completed_path).map_err(|copy_error| {
                format!(
                    "download completion move failed: {rename_error}; copy failed: {copy_error}"
                )
            })?;
            fs::remove_file(&incomplete_path)
                .map_err(|error| format!("download incomplete cleanup failed: {error}"))?;
        }
    }
    if completed_path != final_path {
        {
            let mut transfers = state.transfers.write().await;
            transfers.update_local_path(transfer.id, completed_path.display().to_string());
        }
        persist_transfer_durability(state).await;
    }
    Ok((offset + bytes_received, size))
}

async fn validate_configured_path_policy(state: &AppState, path: &str) -> Result<(), String> {
    let policy = state
        .advanced_networking
        .read()
        .await
        .security
        .path_guard
        .clone();
    // Traversal confinement is unconditional; the switch controls only the
    // configurable length/depth limits, matching frozen native profile.
    if path
        .split(['/', '\\'])
        .any(|segment| segment == ".." || segment == ".")
    {
        return Err("download path contains a traversal segment".to_owned());
    }
    if policy.enabled {
        let depth = path
            .split(['/', '\\'])
            .filter(|segment| !segment.is_empty())
            .count();
        if path.len() > policy.max_path_length || depth > policy.max_path_depth {
            return Err("download path exceeds configured path guard limits".to_owned());
        }
    }
    Ok(())
}

pub(super) async fn enforce_completed_download_content_safety(
    state: &AppState,
    path: &Path,
    expected_path: &Path,
) -> Result<(), String> {
    let security = state.advanced_networking.read().await.security.clone();
    if !security.enabled || !security.content_safety.enabled {
        return Ok(());
    }
    let extension = expected_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable = matches!(
        extension.as_str(),
        "exe"
            | "bat"
            | "cmd"
            | "com"
            | "pif"
            | "scr"
            | "ps1"
            | "vbs"
            | "vbe"
            | "js"
            | "jse"
            | "wsf"
            | "msi"
            | "msp"
            | "hta"
            | "cpl"
            | "jar"
            | "app"
    );
    let mut suspicious = executable && security.content_safety.block_executables;
    if security.content_safety.verify_magic_bytes && !suspicious {
        let mut header = [0_u8; 12];
        let read = fs::File::open(path)
            .and_then(|mut file| file.read(&mut header))
            .map_err(|error| format!("download content-safety read failed: {error}"))?;
        let header = &header[..read];
        suspicious = match extension.as_str() {
            "flac" => !header.starts_with(b"fLaC"),
            "wav" => !(header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WAVE")),
            "ogg" | "opus" => !header.starts_with(b"OggS"),
            "mp3" => {
                !(header.starts_with(b"ID3")
                    || header
                        .first()
                        .zip(header.get(1))
                        .is_some_and(|(a, b)| *a == 0xff && (*b & 0xe0) == 0xe0))
            }
            _ => false,
        };
    }
    if !suspicious {
        return Ok(());
    }
    if security.content_safety.quarantine_suspicious {
        let root = if security
            .content_safety
            .quarantine_directory
            .as_os_str()
            .is_empty()
        {
            state.config.state_dir.join("quarantine")
        } else {
            security.content_safety.quarantine_directory
        };
        fs::create_dir_all(&root)
            .map_err(|error| format!("download quarantine creation failed: {error}"))?;
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("suspicious-download");
        let destination = root.join(format!("{}-{name}", uuid::Uuid::new_v4().simple()));
        fs::rename(path, destination)
            .map_err(|error| format!("download quarantine move failed: {error}"))?;
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("suspicious download removal failed: {error}"))?;
    }
    Err("download rejected by configured content safety policy".to_owned())
}

async fn download_file_with_progress(
    state: &AppState,
    transfer: &TransferEntry,
    connection: &mut slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    offset: u64,
    remaining: usize,
    file: &mut fs::File,
) -> Result<u64, String> {
    let token = time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.receive_token(),
    )
    .await
    .map_err(|_| "file download token receive timed out".to_owned())?
    .map_err(|error| format!("file download token receive failed: {error}"))?;
    if token != transfer.token {
        return Err(format!(
            "transfer token mismatch: expected {}, received {token}",
            transfer.token
        ));
    }
    time::timeout(
        state.config.soulseek_connection.timeout_transfer,
        connection.send_offset(offset),
    )
    .await
    .map_err(|_| "file download offset send timed out".to_owned())?
    .map_err(|error| format!("file download offset send failed: {error}"))?;

    let mut bytes_received = 0_usize;
    let pacing_started = Instant::now();
    while bytes_received < remaining {
        if transfer_is_cancelled(state, transfer.id).await {
            return Err("transfer cancelled".to_owned());
        }
        let next_len =
            (remaining - bytes_received).min(state.config.soulseek_connection.buffer_transfer);
        let chunk = time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            connection.read_chunk(next_len),
        )
        .await
        .map_err(|_| "file download chunk receive timed out".to_owned())?
        .map_err(|error| format!("file download chunk receive failed: {error}"))?;
        bytes_received += chunk.len();
        let transferred = offset.saturating_add(u64::try_from(bytes_received).unwrap_or(u64::MAX));
        write_download_chunk_if_active(state, transfer.id, file, &chunk, transferred).await?;
        let speed_limit_kib = effective_download_pacing_limit(state).await;
        if speed_limit_kib < i32::MAX as u32 {
            let bytes_per_second = u64::from(speed_limit_kib).saturating_mul(1024).max(1);
            let expected_nanos = u128::from(bytes_received as u64)
                .saturating_mul(1_000_000_000)
                .checked_div(u128::from(bytes_per_second))
                .unwrap_or(u128::MAX)
                .min(u128::from(u64::MAX));
            let expected = Duration::from_nanos(expected_nanos as u64);
            let elapsed = pacing_started.elapsed();
            if expected > elapsed {
                time::sleep(expected - elapsed).await;
            }
        }
    }
    Ok(u64::try_from(bytes_received).unwrap_or(u64::MAX))
}

pub(super) async fn effective_download_pacing_limit(state: &AppState) -> u32 {
    let configured = state
        .transfer_download_settings
        .read()
        .await
        .speed_limit_kib;
    if configured == i32::MAX as u32 {
        return configured;
    }
    let active = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .filter(|entry| entry.direction == 0 && is_active_transfer_status(&entry.status))
        .count()
        .max(1);
    configured
        .checked_div(u32::try_from(active).unwrap_or(u32::MAX))
        .unwrap_or(0)
        .max(1)
}

pub(super) async fn write_download_chunk_if_active(
    state: &AppState,
    transfer_id: u64,
    file: &mut fs::File,
    chunk: &[u8],
    bytes_transferred: u64,
) -> Result<(), String> {
    use std::io::Write;

    let mut transfers = state.transfers.write().await;
    if transfers
        .entries
        .iter()
        .find(|entry| entry.id == transfer_id)
        .is_none_or(|entry| entry.status != "in_progress")
    {
        return Err("transfer cancelled".to_owned());
    }
    file.write_all(chunk)
        .map_err(|error| format!("download file write failed: {error}"))?;
    let updated = transfers.update_progress(transfer_id, bytes_transferred);
    let should_persist = updated
        .as_ref()
        .is_some_and(|entry| transfers.should_persist_progress(entry));
    drop(transfers);
    if let Some(entry) = updated {
        if should_persist {
            persist_transfer_progress_projection(state, &entry).await;
        } else {
            persist_transfer_durability(state).await;
            publish_transfer_hub_event(state, "progress", &entry);
        }
    }
    Ok(())
}

pub(super) async fn update_transfer_progress(
    state: &AppState,
    transfer_id: u64,
    bytes_transferred: u64,
) {
    let mut transfers = state.transfers.write().await;
    if transfers
        .entries
        .iter()
        .find(|entry| entry.id == transfer_id)
        .is_some_and(|entry| entry.status == "cancelled")
    {
        return;
    }
    let updated = transfers.update_progress(transfer_id, bytes_transferred);
    let should_persist = updated
        .as_ref()
        .is_some_and(|entry| transfers.should_persist_progress(entry));
    drop(transfers);
    if let Some(entry) = updated {
        if should_persist {
            persist_transfer_progress_projection(state, &entry).await;
        } else {
            persist_transfer_durability(state).await;
            publish_transfer_hub_event(state, "progress", &entry);
        }
    }
}

async fn persist_transfer_progress_projection(state: &AppState, entry: &TransferEntry) {
    persist_transfer_durability(state).await;
    if let Some(db) = state.db.as_ref() {
        let persistence_error = db
            .update_transfer_progress(
                &entry.id.to_string(),
                entry.bytes_transferred,
                entry.updated_at_ms,
            )
            .await
            .err()
            .map(|error| error.to_string());
        if let Some(error) = persistence_error {
            update_session(state, |snapshot| {
                snapshot.last_error = Some(format!(
                    "transfer {} progress persistence failed: {error}",
                    entry.id
                ));
            })
            .await;
        }
    }
    publish_transfer_hub_event(state, "progress", entry);
}

pub(super) async fn transfer_is_cancelled(state: &AppState, transfer_id: u64) -> bool {
    state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .find(|entry| entry.id == transfer_id)
        .is_some_and(|entry| entry.status == "cancelled")
}

pub(super) async fn handle_inbound_file_transfer(
    state: &AppState,
    mut file: slskr_client::file_transfer::FileTransferConnection<TcpStream>,
    token: Option<u32>,
) -> Result<(), String> {
    if let Some(token) = token {
        let pending = state
            .pending_backfill_transfers
            .write()
            .await
            .remove(&token);
        if let Some(pending) = pending {
            let result =
                receive_backfill_header(state, &mut file, token, pending.expected_size).await;
            let public_result = result.clone();
            let _ = pending.response.send(public_result);
            return result.map(|_| ());
        }
    }
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers.pending_inbound_file_transfer(token)
    }
    .ok_or_else(|| {
        token.map_or_else(
            || "no accepted inbound file transfer is pending".to_owned(),
            |token| format!("no accepted inbound file transfer is pending for token {token}"),
        )
    })?;

    // native profile's type-1 transfer manager sends the negotiated remote token
    // immediately after PeerInit.  The regular compatibility path retains
    // the historical local-token send behavior for clients that do not send
    // that framing on an incoming transfer socket.
    let remote_token_received = if file.is_obfuscated() && transfer.direction == 1 {
        let remote_token = time::timeout(
            state.config.soulseek_connection.timeout_transfer,
            file.receive_token(),
        )
        .await
        .map_err(|_| "obfuscated inbound transfer token receive timed out".to_owned())?
        .map_err(|error| format!("obfuscated inbound transfer token receive failed: {error}"))?;
        if remote_token != transfer.token {
            return Err(format!(
                "obfuscated inbound transfer token mismatch: expected {}, received {remote_token}",
                transfer.token
            ));
        }
        true
    } else {
        false
    };

    let in_progress = {
        let mut transfers = state.transfers.write().await;
        transfers.update_status(transfer.id, "in_progress", None, None)
    };
    if let Some(in_progress) = in_progress {
        persist_transfer_projection(state, &in_progress).await;
    }

    let result = if transfer.direction == 0 {
        download_file_transfer_with_connection(state, &transfer, &mut file).await
    } else {
        upload_file_transfer_with_connection(state, &transfer, &mut file, !remote_token_received)
            .await
    };
    if let Err(error) = &result {
        record_expected_upload_failure(state, &transfer, error).await;
    }
    let (status, bytes_transferred, size, reason) = match result {
        Ok((bytes_transferred, size)) => ("succeeded", bytes_transferred, Some(size), None),
        Err(error) => (
            "failed",
            transfer.bytes_transferred,
            transfer.size,
            Some(error),
        ),
    };

    let updated = {
        let mut transfers = state.transfers.write().await;
        transfers.update_local_execution(transfer.id, status, bytes_transferred, size, reason)
    };
    if let Some(updated) = updated {
        let updated = apply_completed_download_permissions(state, updated).await;
        let updated = enrich_completed_audio_metadata(state, updated).await;
        persist_transfer_projection(state, &updated).await;
        issue_relay_download_tokens(state, &updated).await;
        maybe_import_lidarr_completed_download(state, &updated).await;
        maybe_upload_ftp_completed_download(state, &updated).await;
    }
    if transfer.direction == 1 {
        schedule_queued_uploads(state).await;
    } else {
        schedule_queued_downloads(state).await;
    }
    Ok(())
}
