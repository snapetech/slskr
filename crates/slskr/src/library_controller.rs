use super::*;

pub(super) fn library_media_kind(extension: &str) -> &'static str {
    match extension
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" | "flac" | "ogg" | "opus" | "aac" | "m4a" | "wav" => "Audio",
        "mp4" | "mkv" | "avi" | "mov" | "webm" => "Video",
        "txt" | "pdf" | "epub" | "mobi" => "Book",
        _ => "File",
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Resolve a player/library content identifier against the share index.
///
/// Native library projections expose a real sha256:<digest> identifier for
/// local files, while the Soulseek-facing share index historically addressed
/// the same files by virtual filename or its bounded compatibility hash. Keep
/// all three forms usable by the ticket and stream paths.
pub(super) async fn find_shared_entry_for_content(
    state: &AppState,
    filename: Option<&str>,
    content_id: Option<&str>,
) -> Option<FileEntry> {
    let expected_sha256 = content_id
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| is_sha256_hex(value));
    let (direct, candidates) = {
        let shares = state.shares.read().await;
        let direct = shares
            .entries
            .iter()
            .find(|entry| {
                filename.is_some_and(|value| entry.filename == value)
                    || content_id.is_some_and(|value| {
                        entry.filename == value
                            || stable_content_hash(&entry.filename, entry.size).to_string() == value
                    })
            })
            .cloned();
        let candidates = if expected_sha256.is_some() {
            shares
                .entries
                .iter()
                .filter_map(|entry| {
                    shares
                        .local_paths
                        .get(&entry.filename)
                        .map(|path| (entry.clone(), path.clone()))
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        (direct, candidates)
    };
    if direct.is_some() {
        return direct;
    }

    let expected_sha256 = expected_sha256?;
    for (entry, path) in candidates {
        if sha256_local_file_cached(&path)
            .await
            .is_some_and(|actual| actual.eq_ignore_ascii_case(expected_sha256))
        {
            return Some(entry);
        }
    }
    None
}

fn normalize_library_browser_path(raw_path: Option<String>) -> Result<String, String> {
    let raw_path = raw_path.unwrap_or_default();
    if raw_path.len() > 4_096 {
        return Err("library browser path is too long".to_owned());
    }
    let normalized = raw_path.replace('\\', "/");
    let mut segments = Vec::new();
    for segment in normalized.split('/') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." || segment.chars().any(char::is_control) {
            return Err("library browser path contains an invalid segment".to_owned());
        }
        segments.push(segment);
    }
    Ok(segments.join("/"))
}

fn library_browser_relative_path(filename: &str, path: &str) -> Option<String> {
    let filename = filename.replace('\\', "/");
    if path.is_empty() {
        return Some(filename);
    }
    filename
        .strip_prefix(path)
        .and_then(|relative| relative.strip_prefix('/'))
        .map(ToOwned::to_owned)
}

fn library_browser_breadcrumbs(path: &str) -> Vec<serde_json::Value> {
    let mut breadcrumbs = vec![serde_json::json!({
        "name": "Library",
        "path": "",
    })];
    let mut prefix = String::new();
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(segment);
        breadcrumbs.push(serde_json::json!({
            "name": segment,
            "path": prefix,
        }));
    }
    breadcrumbs
}

fn library_browser_query_terms(raw_query: Option<String>) -> Vec<String> {
    raw_query
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .filter(|term| !term.is_empty())
        .collect()
}

fn library_browser_kind_filter(raw_kinds: Option<String>) -> Option<HashSet<String>> {
    let kinds = raw_kinds
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<HashSet<_>>();
    (!kinds.is_empty()).then_some(kinds)
}

fn library_browser_entry_matches(
    entry: &FileEntry,
    query_terms: &[String],
    kinds: Option<&HashSet<String>>,
) -> bool {
    let kind_matches = kinds.is_none_or(|kinds| {
        kinds.contains(&library_media_kind(&entry.extension).to_ascii_lowercase())
    });
    let filename = entry.filename.replace('\\', "/");
    kind_matches
        && query_terms
            .iter()
            .all(|term| filename.to_ascii_lowercase().contains(term))
}

/// Return the real share-backed files behind the Web UI's local library
/// explorer. The old compatibility implementation returned metadata records
/// from LibraryStore, which have no streamable path and ignored path, query,
/// kinds, and directory navigation.
pub(super) async fn library_browser_response(
    state: &AppState,
    raw_query: Option<&str>,
) -> HttpResponse {
    let path = match normalize_library_browser_path(query_parameter(raw_query, "path")) {
        Ok(path) => path,
        Err(error) => return routing::bad_request_response(&error),
    };
    let query_terms = library_browser_query_terms(query_parameter(raw_query, "query"));
    let kinds = library_browser_kind_filter(query_parameter(raw_query, "kinds"));
    let limit = query_parameter(raw_query, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100)
        .clamp(1, 1_000);
    let offset = query_parameter(raw_query, "offset")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let (entries, local_paths, scan_failed) = {
        let shares = state.shares.read().await;
        (
            shares.entries.clone(),
            shares.local_paths.clone(),
            !shares.scan_errors.is_empty(),
        )
    };
    if scan_failed {
        return routing::service_unavailable_response("share browse unavailable");
    }

    let is_search = !query_terms.is_empty();
    let mut files = Vec::new();
    let mut directory_stats = BTreeMap::<String, (usize, HashSet<String>)>::new();
    for entry in entries
        .iter()
        .filter(|entry| library_browser_entry_matches(entry, &query_terms, kinds.as_ref()))
    {
        if is_search {
            files.push(entry.clone());
            continue;
        }
        let Some(relative) = library_browser_relative_path(&entry.filename, &path) else {
            continue;
        };
        let segments = relative
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>();
        if segments.len() == 1 {
            files.push(entry.clone());
        } else if segments.len() > 1 {
            let summary = directory_stats
                .entry(segments[0].to_owned())
                .or_insert_with(|| (0, HashSet::new()));
            summary.0 += 1;
            if segments.len() > 2 {
                summary.1.insert(segments[1].to_owned());
            }
        }
    }
    files.sort_by(|left, right| left.filename.cmp(&right.filename));

    let total_files = files.len();
    let directories = directory_stats
        .iter()
        .map(|(name, (file_count, children))| {
            let directory_path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}/{name}")
            };
            serde_json::json!({
                "name": name,
                "path": directory_path,
                "fileCount": file_count,
                "childDirectoryCount": children.len(),
            })
        })
        .collect::<Vec<_>>();
    let total_directories = directories.len();
    let page = files.iter().skip(offset).take(limit).collect::<Vec<_>>();
    let mut file_values = Vec::with_capacity(page.len());
    for entry in page {
        let local_path = local_paths.get(&entry.filename);
        file_values.push(
            native_library_item_value(entry, local_path.map(PathBuf::as_path), &entry.filename)
                .await,
        );
    }
    let returned_files = file_values.len();
    routing::ok_response(
        serde_json::json!({
            "path": path,
            "breadcrumbs": library_browser_breadcrumbs(&path),
            "directories": directories,
            "files": file_values,
            "totalFiles": total_files,
            "totalDirectories": total_directories,
            "offset": offset,
            "limit": limit,
            "hasMore": offset.saturating_add(returned_files) < total_files,
            "duplicatesRemoved": 0,
        })
        .to_string(),
    )
}

async fn native_library_item_value(
    entry: &FileEntry,
    local_path: Option<&Path>,
    display_path: &str,
) -> serde_json::Value {
    let sha256 = match local_path {
        Some(path) => sha256_local_file_cached(path).await,
        None => None,
    };
    let content_id = sha256.as_ref().map_or_else(
        || stable_content_hash(&entry.filename, entry.size).to_string(),
        |sha256| format!("sha256:{sha256}"),
    );
    let file_name = display_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(display_path);
    serde_json::json!({
        "contentId": content_id,
        "path": display_path,
        "fileName": file_name,
        "bytes": entry.size,
        "mediaKind": library_media_kind(&entry.extension),
        "sha256": sha256,
        "duplicateCount": 1,
    })
}

fn library_entry_matches(
    entry: &FileEntry,
    query: Option<&str>,
    kinds: Option<&HashSet<String>>,
) -> bool {
    let query_matches = query.is_none_or(|query| {
        entry
            .filename
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
    });
    let kind_matches = kinds.is_none_or(|kinds| {
        kinds.contains(&library_media_kind(&entry.extension).to_ascii_lowercase())
    });
    query_matches && kind_matches
}

pub(super) async fn native_library_items_search_json(
    state: &AppState,
    raw_query: Option<&str>,
) -> String {
    let query = query_parameter(raw_query, "query")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    let kinds = query_parameter(raw_query, "kinds")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|kind| !kind.is_empty())
                .map(str::to_ascii_lowercase)
                .collect::<HashSet<_>>()
        });
    let limit = query_parameter(raw_query, "limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100)
        .clamp(1, 100);

    let (entries, local_paths) = {
        let shares = state.shares.read().await;
        (shares.entries.clone(), shares.local_paths.clone())
    };
    let mut candidates = entries
        .into_iter()
        .filter(|entry| library_entry_matches(entry, query.as_deref(), kinds.as_ref()))
        .take(limit)
        .map(|entry| {
            let local_path = local_paths.get(&entry.filename).cloned();
            let display_path = entry.filename.clone();
            (entry, local_path, display_path)
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        let mut settings = state.share_settings.read().await.clone();
        let downloads = state
            .downloads_dir
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if downloads.is_dir()
            && !settings
                .directories
                .iter()
                .any(|directory| directory.local_path == downloads)
        {
            settings.directories.push(ShareDirectory {
                raw: downloads.display().to_string(),
                alias: "downloads".to_owned(),
                local_path: downloads,
                is_excluded: false,
            });
        }
        let case_sensitive = *state
            .controller_case_sensitive_regex
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let filters = compile_controller_regexes_for_request(
            settings.filters.clone(),
            case_sensitive,
            state.config.controller_profile,
        )
        .expect("validated share filters must compile");
        let scan = scan_share_dirs(
            &settings.directories,
            settings.follow_symlinks,
            settings.include_hidden,
            settings.max_files,
            settings.probe_media_attributes,
            settings.cache_workers,
            &filters,
        );
        candidates = scan
            .entries
            .into_iter()
            .filter(|entry| library_entry_matches(entry, query.as_deref(), kinds.as_ref()))
            .take(limit)
            .map(|entry| {
                let local_path = scan.local_paths.get(&entry.filename).cloned();
                let display_path = local_path
                    .as_deref()
                    .and_then(|path| {
                        settings
                            .directories
                            .iter()
                            .filter(|directory| !directory.is_excluded)
                            .find_map(|directory| {
                                path.strip_prefix(&directory.local_path)
                                    .ok()
                                    .map(virtual_share_path)
                            })
                    })
                    .unwrap_or_else(|| entry.filename.clone());
                (entry, local_path, display_path)
            })
            .collect();
    }

    let mut items = Vec::with_capacity(candidates.len());
    for (entry, local_path, display_path) in candidates {
        items.push(native_library_item_value(&entry, local_path.as_deref(), &display_path).await);
    }
    serde_json::json!({"items": items}).to_string()
}
