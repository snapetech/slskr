use super::*;

pub(super) fn decode_controller_base64_path_segment(encoded: &str) -> Result<String, String> {
    let encoded = percent_decode_component(encoded);
    let encoded = encoded
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    let bytes = STANDARD
        .decode(&encoded)
        .or_else(|_| STANDARD_NO_PAD.decode(&encoded))
        .map_err(|_| "path segment must be base64 encoded".to_owned())?;
    String::from_utf8(bytes).map_err(|_| "path segment must decode to UTF-8".to_owned())
}

fn scoped_relative_storage_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.to_path_buf();
    let mut appended = false;
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => {
                path.push(part);
                appended = true;
            }
            Component::CurDir => {}
            Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                return Err("path must be relative and stay within the storage root".to_owned());
            }
        }
    }
    if !appended {
        return Err("path is empty".to_owned());
    }
    if !path.starts_with(root) {
        return Err("path escapes the storage root".to_owned());
    }
    Ok(path)
}

pub(super) fn delete_scoped_file_storage_path(
    root: &Path,
    encoded_name: &str,
    directory: bool,
) -> Result<bool, String> {
    let decoded = decode_controller_base64_path_segment(encoded_name)?;
    let path = scoped_relative_storage_path(root, &decoded)?;
    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
    #[cfg(unix)]
    {
        delete_scoped_storage_path_unix(root, &path, directory)
    }
    #[cfg(not(unix))]
    {
        let canonical_root = root
            .canonicalize()
            .map_err(|error| format!("storage root canonicalize failed: {error}"))?;
        let canonical_parent = match path.parent().unwrap_or(root).canonicalize() {
            Ok(parent) => parent,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("storage parent canonicalize failed: {error}")),
        };
        if !canonical_parent.starts_with(&canonical_root) {
            return Err("path escapes the storage root".to_owned());
        }
        let metadata = match path.symlink_metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(format!("storage path metadata failed: {error}")),
        };
        if metadata.file_type().is_symlink() {
            return Err("storage path must not be a symlink".to_owned());
        }
        if directory {
            if !metadata.is_dir() {
                return Ok(false);
            }
            fs::remove_dir_all(&path)
                .map_err(|error| format!("directory delete failed: {error}"))?;
        } else {
            if !metadata.is_file() {
                return Ok(false);
            }
            fs::remove_file(&path).map_err(|error| format!("file delete failed: {error}"))?;
        }
        Ok(true)
    }
}

#[cfg(unix)]
pub(super) fn delete_scoped_storage_path_unix(
    root: &Path,
    path: &Path,
    directory: bool,
) -> Result<bool, String> {
    use rustix::fs::{open, openat, unlinkat, AtFlags, Mode, OFlags};

    let relative = path
        .strip_prefix(root)
        .map_err(|_| "storage path is outside the storage root".to_owned())?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err("storage path contains a non-relative component".to_owned()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (target, parents) = components
        .split_last()
        .ok_or_else(|| "storage path is empty".to_owned())?;
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut parent = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("storage root confined open failed: {error}"))?;
    for component in parents {
        parent = match openat(&parent, *component, directory_flags, Mode::empty()) {
            Ok(parent) => parent,
            Err(error) if error == rustix::io::Errno::NOENT => return Ok(false),
            Err(error) => return Err(format!("storage parent confined open failed: {error}")),
        };
    }
    if directory {
        let target_directory = match openat(&parent, *target, directory_flags, Mode::empty()) {
            Ok(target_directory) => target_directory,
            Err(error) if error == rustix::io::Errno::NOENT => return Ok(false),
            Err(error) => return Err(format!("storage directory confined open failed: {error}")),
        };
        let mut total = 0;
        remove_directory_contents_unix(&target_directory, 0, &mut total)?;
        unlinkat(&parent, *target, AtFlags::REMOVEDIR)
            .map_err(|error| format!("directory confined delete failed: {error}"))?;
    } else {
        match unlinkat(&parent, *target, AtFlags::empty()) {
            Ok(()) => {}
            Err(error) if error == rustix::io::Errno::NOENT => return Ok(false),
            Err(error) => return Err(format!("file confined delete failed: {error}")),
        }
    }
    Ok(true)
}

#[cfg(unix)]
fn remove_directory_contents_unix(
    directory: &impl std::os::fd::AsFd,
    depth: usize,
    total: &mut usize,
) -> Result<(), String> {
    use rustix::fs::{openat, unlinkat, AtFlags, Dir, Mode, OFlags};

    if depth >= SLSKD_STORAGE_MAX_DELETE_DEPTH {
        return Err(STORAGE_DIRECTORY_DELETE_DEPTH_ERROR.to_owned());
    }
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut entries = Dir::read_from(directory)
        .map_err(|error| format!("storage directory read failed: {error}"))?;
    let mut names = Vec::new();
    let mut scanned = 0;
    while let Some(entry) = entries.read() {
        let entry = entry.map_err(|error| format!("storage directory entry failed: {error}"))?;
        if !matches!(entry.file_name().to_bytes(), b"." | b"..") {
            reserve_storage_delete_entry(&mut scanned, total)?;
            names.push(entry.file_name().to_owned());
        }
    }
    for name in names {
        match openat(directory, &name, directory_flags, Mode::empty()) {
            Ok(child) => {
                remove_directory_contents_unix(&child, depth + 1, total)?;
                unlinkat(directory, &name, AtFlags::REMOVEDIR)
                    .map_err(|error| format!("storage child directory delete failed: {error}"))?;
            }
            Err(_) => unlinkat(directory, &name, AtFlags::empty())
                .map_err(|error| format!("storage child file delete failed: {error}"))?,
        }
    }
    Ok(())
}

pub(super) fn controller_user_file_json(entry: &BrowseEntry) -> serde_json::Value {
    serde_json::json!({
        "filename": entry.filename,
        "size": entry.size,
        "code": 1,
        "extension": entry.extension,
        "attributeCount": 0,
        "attributes": [],
    })
}

pub(super) fn controller_share_file_json(entry: &FileEntry) -> serde_json::Value {
    serde_json::json!({
        "filename": entry.filename,
        "size": entry.size,
        "code": 1,
        "extension": entry.extension,
        "attributeCount": 0,
        "attributes": [],
    })
}

fn share_entry_matches_prefix(entry: &FileEntry, share_id: &str) -> bool {
    share_id.is_empty()
        || share_id == "shares"
        || entry.filename == share_id
        || entry
            .filename
            .strip_prefix(share_id)
            .is_some_and(|rest| rest.starts_with('/'))
}

pub(super) fn controller_share_directories_json(
    entries: &[FileEntry],
    share_id: Option<&str>,
) -> String {
    let filtered = entries
        .iter()
        .filter(|entry| share_id.is_none_or(|share_id| share_entry_matches_prefix(entry, share_id)))
        .cloned()
        .collect::<Vec<_>>();
    let mut grouped = group_share_entries(&filtered);
    grouped.sort_by(|(left, _), (right, _)| right.cmp(left));
    let directories = grouped
        .into_iter()
        .map(|(name, mut files)| {
            files.sort_by(|left, right| left.filename.cmp(&right.filename));
            serde_json::json!({
                "name": name.replace('/', "\\"),
                "fileCount": files.len(),
                "files": files.iter().map(controller_share_file_json).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::Value::Array(directories).to_string()
}

pub(super) fn controller_user_directories_json(
    directory: &str,
    entries: &[BrowseEntry],
    query: Option<&str>,
) -> String {
    let filter = RecordListFilter::from_query(query);
    let files = entries
        .iter()
        .filter(|entry| virtual_folder(&entry.filename) == directory)
        .collect::<Vec<_>>();
    let file_count = files.len();
    let filtered_files = files
        .into_iter()
        .filter(|entry| {
            filter
                .q
                .as_deref()
                .is_none_or(|q| entry.filename.to_ascii_lowercase().contains(q))
        })
        .collect::<Vec<_>>();
    let filtered_file_count = filtered_files.len();
    let total_bytes = filtered_files.iter().map(|entry| entry.size).sum::<u64>();
    let files = filtered_files
        .into_iter()
        .skip(filter.offset)
        .take(filter.limit.unwrap_or(usize::MAX))
        .map(controller_user_file_json)
        .collect::<Vec<_>>();
    serde_json::Value::Array(vec![serde_json::json!({
        "name": directory,
        "fileCount": file_count,
        "filteredFileCount": filtered_file_count,
        "totalBytes": total_bytes,
        "files": files,
        "offset": filter.offset,
        "limit": filter.limit,
    })])
    .to_string()
}

pub(super) fn controller_user_root_json(entries: &[BrowseEntry], query: Option<&str>) -> String {
    let filter = RecordListFilter::from_query(query);
    let grouped = group_browse_entries(entries);
    let directory_count = grouped.len();
    let total_file_count = grouped.iter().map(|(_, files)| files.len()).sum::<usize>();
    let filtered = grouped
        .into_iter()
        .filter(|(name, files)| {
            filter.q.as_deref().is_none_or(|q| {
                name.to_ascii_lowercase().contains(q)
                    || files
                        .iter()
                        .any(|entry| entry.filename.to_ascii_lowercase().contains(q))
            })
        })
        .collect::<Vec<_>>();
    let filtered_directory_count = filtered.len();
    let filtered_file_count = filtered
        .iter()
        .map(|(name, files)| {
            if let Some(q) = filter.q.as_deref() {
                if name.to_ascii_lowercase().contains(q) {
                    return files.len();
                }
                files
                    .iter()
                    .filter(|entry| entry.filename.to_ascii_lowercase().contains(q))
                    .count()
            } else {
                files.len()
            }
        })
        .sum::<usize>();
    let filtered_total_bytes = filtered
        .iter()
        .map(|(name, files)| {
            if let Some(q) = filter.q.as_deref() {
                if name.to_ascii_lowercase().contains(q) {
                    return files.iter().map(|entry| entry.size).sum::<u64>();
                }
                files
                    .iter()
                    .filter(|entry| entry.filename.to_ascii_lowercase().contains(q))
                    .map(|entry| entry.size)
                    .sum::<u64>()
            } else {
                files.iter().map(|entry| entry.size).sum::<u64>()
            }
        })
        .sum::<u64>();
    let directories = filtered
        .into_iter()
        .skip(filter.offset)
        .take(filter.limit.unwrap_or(usize::MAX))
        .map(|(name, files)| {
            let directory_matched = filter
                .q
                .as_deref()
                .is_some_and(|q| name.to_ascii_lowercase().contains(q));
            let visible_files = files
                .iter()
                .filter(|entry| {
                    if directory_matched {
                        return true;
                    }
                    filter
                        .q
                        .as_deref()
                        .is_none_or(|q| entry.filename.to_ascii_lowercase().contains(q))
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "name": name,
                "fileCount": files.len(),
                "filteredFileCount": visible_files.len(),
                "totalBytes": visible_files.iter().map(|entry| entry.size).sum::<u64>(),
                "files": visible_files.iter().map(|entry| controller_user_file_json(entry)).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "directories": directories,
        "directoryCount": directory_count,
        "filteredDirectoryCount": filtered_directory_count,
        "fileCount": total_file_count,
        "filteredFileCount": filtered_file_count,
        "totalBytes": filtered_total_bytes,
        "offset": filter.offset,
        "limit": filter.limit,
        "lockedDirectories": [],
        "lockedDirectoryCount": 0,
    })
    .to_string()
}

pub(super) fn target_storage_directory_json(json: String, target: ControllerProfile) -> String {
    if target != ControllerProfile::Native {
        return json;
    }
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&json) else {
        return json;
    };
    if let Some(directories) = value
        .get_mut("directories")
        .and_then(serde_json::Value::as_array_mut)
    {
        for directory in directories {
            if let Some(directory) = directory.as_object_mut() {
                directory
                    .entry("files")
                    .or_insert_with(|| serde_json::json!([]));
                directory
                    .entry("directories")
                    .or_insert_with(|| serde_json::json!([]));
            }
        }
    }
    value.to_string()
}

#[cfg(not(unix))]
pub(super) fn controller_empty_directory_json(name: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "fullName": name,
        "attributes": "",
        "createdAt": "",
        "modifiedAt": "",
        "files": [],
        "directories": [],
    })
}

pub(super) fn controller_filesystem_timestamp(metadata: &fs::Metadata, created: bool) -> String {
    let time = if created {
        metadata.created().or_else(|_| metadata.modified())
    } else {
        metadata.modified()
    };
    let Ok(time) = time else {
        return "0001-01-01T00:00:00Z".to_owned();
    };
    let Ok(duration) = time.duration_since(std::time::UNIX_EPOCH) else {
        return "0001-01-01T00:00:00Z".to_owned();
    };
    let Some(timestamp) = chrono::DateTime::<chrono::Utc>::from_timestamp(
        i64::try_from(duration.as_secs()).unwrap_or(i64::MAX),
        duration.subsec_nanos(),
    ) else {
        return "0001-01-01T00:00:00Z".to_owned();
    };
    format!(
        "{}.{:07}Z",
        timestamp.format("%Y-%m-%dT%H:%M:%S"),
        timestamp.timestamp_subsec_nanos() / 100
    )
}

pub(super) const SLSKD_STORAGE_DIRECT_LIST_DEFAULT_ENTRIES: usize = 1_024;
pub(super) const SLSKD_STORAGE_RECURSIVE_LIST_DEFAULT_ENTRIES: usize = 256;
#[cfg(any(test, feature = "bounded-differential"))]
#[allow(dead_code)]
pub(super) const SLSKD_STORAGE_RECURSIVE_LIST_MAX_ENTRIES: usize = 1_024;
pub(super) const SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES: usize = 16_384;
pub(super) const SLSKD_STORAGE_MAX_RECURSION_DEPTH: usize = 24;
pub(super) const SLSKD_STORAGE_MAX_DELETE_DEPTH: usize = 64;
pub(super) const SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES: usize = 65_536;
pub(super) const STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR: &str =
    "storage directory entry limit exceeded";
pub(super) const STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR: &str =
    "storage directory delete entry limit exceeded";
pub(super) const STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR: &str =
    "storage directory delete total entry limit exceeded";
pub(super) const STORAGE_DIRECTORY_DELETE_DEPTH_ERROR: &str =
    "storage directory delete depth exceeded";
pub(super) const STORAGE_DIRECTORY_NOT_FOUND_ERROR: &str = "storage directory not found";

pub(super) fn reserve_storage_scan_entry(scanned: &mut usize) -> Result<(), String> {
    if *scanned >= SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES {
        return Err(STORAGE_DIRECTORY_ENTRY_LIMIT_ERROR.to_owned());
    }
    *scanned += 1;
    Ok(())
}

pub(super) fn reserve_storage_delete_entry(
    scanned: &mut usize,
    total: &mut usize,
) -> Result<(), String> {
    if *scanned >= SLSKD_STORAGE_MAX_SCANNED_DIRECTORY_ENTRIES {
        return Err(STORAGE_DIRECTORY_DELETE_ENTRY_LIMIT_ERROR.to_owned());
    }
    if *total >= SLSKD_STORAGE_MAX_DELETE_TOTAL_ENTRIES {
        return Err(STORAGE_DIRECTORY_DELETE_TOTAL_LIMIT_ERROR.to_owned());
    }
    *scanned += 1;
    *total += 1;
    Ok(())
}

pub(super) fn query_bool(query: Option<&str>, key: &str) -> Option<bool> {
    query_params(query.unwrap_or_default())
        .into_iter()
        .find_map(|(name, value)| (name == key).then(|| parse_bool_value(&value))?)
}

pub(super) fn query_bool_is_invalid(query: Option<&str>, key: &str) -> bool {
    query_params(query.unwrap_or_default())
        .into_iter()
        .any(|(name, value)| name == key && parse_bool_value(&value).is_none())
}

#[cfg(not(unix))]
pub(super) fn controller_storage_file_json(path: &Path, root: &Path) -> serde_json::Value {
    let relative = path
        .strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .unwrap_or_default()
        .replace('\\', "/");
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let metadata = path.metadata().ok();
    let length = metadata.as_ref().map(fs::Metadata::len).unwrap_or(0);
    serde_json::json!({
        "name": name,
        "fullName": relative,
        "length": length,
        "attributes": "Normal",
        "createdAt": metadata.as_ref().map_or_else(|| "0001-01-01T00:00:00Z".to_owned(), |metadata| controller_filesystem_timestamp(metadata, true)),
        "modifiedAt": metadata.as_ref().map_or_else(|| "0001-01-01T00:00:00Z".to_owned(), |metadata| controller_filesystem_timestamp(metadata, false)),
    })
}

#[cfg(not(unix))]
pub(super) fn controller_storage_directory_value(
    root: &Path,
    directory: &Path,
    state: &mut StorageDirectoryListState,
    _top_level: bool,
    depth: usize,
) -> Result<serde_json::Value, String> {
    let relative = directory
        .strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .unwrap_or_default()
        .replace('\\', "/");
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    if !directory.exists() {
        return Err(STORAGE_DIRECTORY_NOT_FOUND_ERROR.to_owned());
    }
    let metadata = directory
        .symlink_metadata()
        .map_err(|error| format!("directory metadata failed: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Err("storage path must not be a symlink".to_owned());
    }
    if !metadata.is_dir() {
        return Err("storage path is not a directory".to_owned());
    }
    let created_at = controller_filesystem_timestamp(&metadata, true);
    let modified_at = controller_filesystem_timestamp(&metadata, false);

    let mut entries = Vec::new();
    let mut scanned = 0;
    for entry in
        fs::read_dir(directory).map_err(|error| format!("directory read failed: {error}"))?
    {
        let entry = entry.map_err(|error| format!("directory entry read failed: {error}"))?;
        reserve_storage_scan_entry(&mut scanned)?;
        entries.push(entry.path());
    }
    entries.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

    let mut files = Vec::new();
    let mut directories = Vec::new();
    for path in entries {
        let metadata = path
            .symlink_metadata()
            .map_err(|error| format!("directory entry metadata failed: {error}"))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if !state.reserve_entry() {
            break;
        }
        if metadata.is_file() {
            files.push(controller_storage_file_json(&path, root));
        } else if metadata.is_dir() {
            if state.options.recursive && depth < SLSKD_STORAGE_MAX_RECURSION_DEPTH {
                let mut child =
                    controller_storage_directory_value(root, &path, state, false, depth + 1)?;
                if let Some(nested) = child
                    .get_mut("files")
                    .and_then(serde_json::Value::as_array_mut)
                {
                    files.append(nested);
                }
                if let Some(nested) = child
                    .get_mut("directories")
                    .and_then(serde_json::Value::as_array_mut)
                {
                    directories.append(nested);
                }
                child["files"] = serde_json::json!([]);
                child["directories"] = serde_json::json!([]);
                directories.push(child);
            } else {
                let child_relative = path
                    .strip_prefix(root)
                    .ok()
                    .and_then(|path| path.to_str())
                    .unwrap_or_default()
                    .replace('\\', "/");
                directories.push(controller_empty_directory_json(&child_relative));
                if state.options.recursive {
                    state.truncated = true;
                }
            }
        }
    }
    files.sort_by(|left, right| left["fullName"].as_str().cmp(&right["fullName"].as_str()));
    directories.sort_by(|left, right| left["fullName"].as_str().cmp(&right["fullName"].as_str()));
    Ok(serde_json::json!({
        "name": name,
        "fullName": relative,
        "attributes": "Directory",
        "createdAt": created_at,
        "modifiedAt": modified_at,
        "files": files,
        "directories": directories,
    }))
}

pub(super) fn controller_storage_directory_json(
    root: &Path,
    encoded_name: Option<&str>,
    options: StorageDirectoryListOptions,
) -> Result<String, String> {
    fs::create_dir_all(root).map_err(|error| format!("storage root create failed: {error}"))?;
    let path = if let Some(encoded_name) = encoded_name {
        let decoded = decode_controller_base64_path_segment(encoded_name)?;
        scoped_relative_storage_path(root, &decoded)?
    } else {
        root.to_path_buf()
    };
    #[cfg(unix)]
    {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "storage path is outside the storage root".to_owned())?;
        controller_storage_directory_json_unix(root, relative, options)
    }
    #[cfg(not(unix))]
    {
        let canonical_root = root
            .canonicalize()
            .map_err(|error| format!("storage root canonicalize failed: {error}"))?;
        let canonical_parent = if path == root {
            canonical_root.clone()
        } else if path.exists() {
            path.parent()
                .unwrap_or(root)
                .canonicalize()
                .map_err(|error| format!("storage parent canonicalize failed: {error}"))?
        } else {
            path.parent()
                .unwrap_or(root)
                .canonicalize()
                .map_err(|error| format!("storage parent canonicalize failed: {error}"))?
        };
        if !canonical_parent.starts_with(&canonical_root) {
            return Err("path escapes the storage root".to_owned());
        }
        let mut state = StorageDirectoryListState::new(options);
        controller_storage_directory_value(root, &path, &mut state, true, 0).map(|mut value| {
            value["fullName"] = serde_json::json!("");
            value.to_string()
        })
    }
}

#[cfg(unix)]
pub(super) fn controller_storage_directory_json_unix(
    root: &Path,
    relative: &Path,
    options: StorageDirectoryListOptions,
) -> Result<String, String> {
    use rustix::fs::{open, openat, Mode, OFlags};

    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("storage root confined open failed: {error}"))?;
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err("storage path contains a non-relative component".to_owned());
        };
        directory = match openat(&directory, component, directory_flags, Mode::empty()) {
            Ok(directory) => directory,
            Err(error) if error == rustix::io::Errno::NOENT => {
                return Err(STORAGE_DIRECTORY_NOT_FOUND_ERROR.to_owned());
            }
            Err(error) => return Err(format!("storage directory confined open failed: {error}")),
        };
    }
    let mut state = StorageDirectoryListState::new(options);
    controller_storage_directory_value_unix(root, relative, &directory, &mut state, true, 0)
        .map(|value| value.to_string())
}

#[cfg(unix)]
pub(super) fn controller_storage_directory_value_unix(
    root: &Path,
    relative: &Path,
    directory: &impl std::os::fd::AsFd,
    state: &mut StorageDirectoryListState,
    _top_level: bool,
    _depth: usize,
) -> Result<serde_json::Value, String> {
    let requested_root = root.join(relative);
    let directory_metadata = fs::metadata(&requested_root)
        .map_err(|error| format!("storage directory metadata failed: {error}"))?;
    let mut files = Vec::new();
    let mut directories = Vec::new();
    collect_controller_storage_entries_unix(
        &requested_root,
        Path::new(""),
        directory,
        state,
        0,
        &mut files,
        &mut directories,
    )?;
    let name = relative
        .file_name()
        .or_else(|| root.file_name())
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(serde_json::json!({
        "name": name,
        "fullName": "",
        "attributes": "Directory",
        "createdAt": controller_filesystem_timestamp(&directory_metadata, true),
        "modifiedAt": controller_filesystem_timestamp(&directory_metadata, false),
        "files": files,
        "directories": directories,
    }))
}

#[cfg(unix)]
fn collect_controller_storage_entries_unix(
    requested_root: &Path,
    relative: &Path,
    directory: &impl std::os::fd::AsFd,
    state: &mut StorageDirectoryListState,
    depth: usize,
    files: &mut Vec<serde_json::Value>,
    directories: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    use rustix::fs::{openat, Dir, Mode, OFlags};
    use std::os::unix::ffi::OsStrExt;

    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file_flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut reader = Dir::read_from(directory)
        .map_err(|error| format!("storage directory read failed: {error}"))?;
    let mut names = Vec::new();
    let mut scanned = 0;
    while let Some(entry) = reader.read() {
        let entry = entry.map_err(|error| format!("storage directory entry failed: {error}"))?;
        if !matches!(entry.file_name().to_bytes(), b"." | b"..") {
            reserve_storage_scan_entry(&mut scanned)?;
            names.push(entry.file_name().to_owned());
        }
    }
    names.sort_by(|left, right| left.to_bytes().cmp(right.to_bytes()));

    for name in &names {
        if openat(directory, name, directory_flags, Mode::empty()).is_ok() {
            continue;
        }
        let Some(metadata) = openat(directory, name, file_flags, Mode::empty())
            .ok()
            .and_then(|file| fs::File::from(file).metadata().ok())
            .filter(fs::Metadata::is_file)
        else {
            continue;
        };
        if !state.reserve_entry() {
            return Ok(());
        }
        let name_os = std::ffi::OsStr::from_bytes(name.to_bytes());
        let child_relative = relative.join(name_os);
        files.push(serde_json::json!({
            "name": name_os.to_string_lossy(),
            "fullName": child_relative.to_string_lossy().replace('\\', "/"),
            "length": metadata.len(),
            "attributes": "Normal",
            "createdAt": controller_filesystem_timestamp(&metadata, true),
            "modifiedAt": controller_filesystem_timestamp(&metadata, false),
        }));
    }

    for name in names {
        let Ok(child) = openat(directory, &name, directory_flags, Mode::empty()) else {
            continue;
        };
        if !state.reserve_entry() {
            return Ok(());
        }
        let name_os = std::ffi::OsStr::from_bytes(name.to_bytes());
        let child_relative = relative.join(name_os);
        let metadata = fs::metadata(requested_root.join(&child_relative))
            .map_err(|error| format!("storage directory metadata failed: {error}"))?;
        directories.push(serde_json::json!({
            "name": name_os.to_string_lossy(),
            "fullName": child_relative.to_string_lossy().replace('\\', "/"),
            "attributes": "Directory",
            "createdAt": controller_filesystem_timestamp(&metadata, true),
            "modifiedAt": controller_filesystem_timestamp(&metadata, false),
        }));
        if state.options.recursive && depth < SLSKD_STORAGE_MAX_RECURSION_DEPTH {
            collect_controller_storage_entries_unix(
                requested_root,
                &child_relative,
                &child,
                state,
                depth + 1,
                files,
                directories,
            )?;
        } else if state.options.recursive {
            state.truncated = true;
        }
    }
    Ok(())
}
