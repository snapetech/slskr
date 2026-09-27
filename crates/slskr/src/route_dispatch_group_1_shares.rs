async fn route_dispatch_group_1_shares(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let method = context.method;
    let normalized_path = context.normalized_path;
    let authorization = context.authorization;
    let body = context.body;
    let state = context.state;
    let route = context.route;
    let headers = context.headers;
    let extended_mutation = context.extended_mutation;
    let request_is_versioned_v0 = context.request_is_versioned_v0;
    match (method, normalized_path) {
        ("GET", "/api/shares") => {
            let shares = state.shares.read().await;
            let mut roots = shares
                .roots
                .iter()
                .map(controller_share_value)
                .collect::<Vec<_>>();
            if roots.is_empty() && !shares.entries.is_empty() {
                roots.push(serde_json::json!({
                    "localPath": "shares",
                    "id": "shares",
                    "alias": "shares",
                    "raw": "shares",
                    "remotePath": "shares",
                    "directories": 0,
                    "files": shares.entries.len(),
                    "bytes": shares.entries.iter().map(|entry| entry.size).sum::<u64>(),
                    "isExcluded": false,
                }));
            }
            let json = serde_json::json!({ "local": roots }).to_string();
            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json; charset=utf-8",
                body: json,
            })
        }
        ("GET", "/api/shares/catalog") => {
            let shares = state.shares.read().await;
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: shares.catalog_json(route.query),
            })
        }
        ("PUT", "/api/shares") => {
            let rebuilt = match rebuild_share_index(state).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(share_rebuild_error_response(&error)),
            };
            let json = rebuilt.json();
            record_event(state, "share.scan.completed", "shares", None).await;
            if route.path.starts_with("/api/v0/") {
                return Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "",
                    body: String::new(),
                });
            }
            Ok(routing::ok_response((!json.is_empty()).to_string()))
        }
        ("GET", "/api/files/downloads/directories")
        | ("GET", "/api/files/incomplete/directories")
        | ("GET", "/api/v0/files/downloads/directories")
        | ("GET", "/api/v0/files/incomplete/directories") => {
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            ) && query_bool_is_invalid(route.query, "recursive")
            {
                return Ok(routing::bad_request_response(
                    "The recursive query value must be a boolean",
                ));
            }
            let root = if normalized_path.contains("/files/downloads/") {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            if state.config.controller_profile == ControllerProfile::Legacy && !root.is_dir() {
                return Ok(file_storage_error_response(
                    STORAGE_DIRECTORY_NOT_FOUND_ERROR,
                ));
            }
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, None, options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(json, state.config.controller_profile),
                }),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("GET", path)
            if (path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/v0/files/downloads/directories/")
                || path.starts_with("/api/v0/files/incomplete/directories/")) =>
        {
            let Some((storage, resource, encoded_name)) =
                controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            if resource != "directories" {
                return Ok(routing::not_found_response());
            }
            if matches!(
                state.config.controller_profile,
                ControllerProfile::Legacy | ControllerProfile::Native
            ) && query_bool_is_invalid(route.query, "recursive")
            {
                return Ok(routing::bad_request_response(
                    "The recursive query value must be a boolean",
                ));
            }
            let root = if storage == "downloads" {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            let options = StorageDirectoryListOptions::from_query(route.query);
            match controller_storage_directory_json(&root, Some(encoded_name), options) {
                Ok(json) => Ok(HttpResponse {
                    status: "200 OK",
                    content_type: "application/json; charset=utf-8",
                    body: target_storage_directory_json(json, state.config.controller_profile),
                }),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("DELETE", path)
            if path.starts_with("/api/files/downloads/directories/")
                || path.starts_with("/api/files/downloads/files/")
                || path.starts_with("/api/files/incomplete/directories/")
                || path.starts_with("/api/files/incomplete/files/")
                || path.starts_with("/api/v0/files/downloads/directories/")
                || path.starts_with("/api/v0/files/downloads/files/")
                || path.starts_with("/api/v0/files/incomplete/directories/")
                || path.starts_with("/api/v0/files/incomplete/files/") =>
        {
            if !effective_remote_file_management(state) {
                return Ok(HttpResponse {
                    status: "403 Forbidden",
                    content_type: "",
                    body: String::new(),
                });
            }
            let Some((storage, resource, encoded_name)) =
                controller_file_storage_resource_path(path)
            else {
                return Ok(routing::not_found_response());
            };
            let root = if storage == "downloads" {
                effective_downloads_dir(state)
            } else {
                effective_incomplete_dir(state)
            };
            let delete_result = if resource == "directories" {
                delete_scoped_file_storage_path(&root, encoded_name, true)
            } else {
                delete_scoped_file_storage_path(&root, encoded_name, false)
            };
            match delete_result {
                Ok(true) => Ok(HttpResponse {
                    status: "204 No Content",
                    content_type: "",
                    body: String::new(),
                }),
                Ok(false)
                    if resource == "files"
                        && matches!(
                            state.config.controller_profile,
                            ControllerProfile::Legacy | ControllerProfile::Native
                        ) =>
                {
                    Ok(HttpResponse {
                        status: "204 No Content",
                        content_type: "",
                        body: String::new(),
                    })
                }
                Ok(false) => Ok(routing::not_found_response()),
                Err(error) => Ok(file_storage_error_response(&error)),
            }
        }
        ("GET", path) if path.starts_with("/api/files/") || path.starts_with("/api/v0/files/") => {
            let root_label = path
                .strip_prefix("/api/v0/files/")
                .or_else(|| path.strip_prefix("/api/files/"))
                .unwrap_or("");

            if root_label.is_empty() {
                return Ok(routing::not_found_response());
            }

            let mut extension_filter: Option<String> = None;
            let mut selected_folder = String::new();
            let mut folder_requested = false;
            let mut recursive = false;
            for (name, value) in query_params(route.query.unwrap_or_default()) {
                match name.as_str() {
                    "extension" => extension_filter = non_empty(value),
                    "folder" | "path" | "prefix" => {
                        folder_requested = true;
                        selected_folder = value.trim_matches('/').to_owned();
                    }
                    "recursive" => recursive = parse_bool_value(&value).unwrap_or(false),
                    _ => {}
                }
            }

            let filter = RecordListFilter::from_query(route.query);
            let shares = state.shares.read().await;

            let Some(root) = shares.roots.iter().find(|r| r.label == root_label) else {
                drop(shares);
                return Ok(routing::not_found_response());
            };

            let base_prefix = if selected_folder.is_empty() {
                root_label.to_owned()
            } else {
                format!("{}/{}", root_label, selected_folder)
            };
            let root_prefix = format!("{root_label}/");
            let base_child_prefix = format!("{base_prefix}/");
            let q = filter.q.as_deref();
            let folder_mode = folder_requested || recursive;

            let root_entries = shares
                .entries
                .iter()
                .filter(|entry| entry.filename.starts_with(&root_prefix))
                .collect::<Vec<_>>();

            let mut directory_summaries = BTreeMap::<String, (usize, u64)>::new();
            for entry in &root_entries {
                let Some(relative_to_base) = entry.filename.strip_prefix(&base_child_prefix) else {
                    continue;
                };
                let Some((child, _)) = relative_to_base.split_once('/') else {
                    continue;
                };
                if child.is_empty() {
                    continue;
                }
                let directory_path = if selected_folder.is_empty() {
                    child.to_owned()
                } else {
                    format!("{selected_folder}/{child}")
                };
                if q.is_some_and(|q| {
                    !directory_path.to_ascii_lowercase().contains(q)
                        && !format!("{root_label}/{directory_path}")
                            .to_ascii_lowercase()
                            .contains(q)
                }) {
                    continue;
                }
                if extension_filter
                    .as_deref()
                    .is_some_and(|ext| entry.extension != ext)
                {
                    continue;
                }
                let summary = directory_summaries.entry(directory_path).or_default();
                summary.0 += 1;
                summary.1 += entry.size;
            }

            let mut entries: Vec<_> = root_entries
                .into_iter()
                .filter(|entry| {
                    if folder_mode {
                        if recursive {
                            entry.filename.starts_with(&base_child_prefix)
                        } else {
                            virtual_folder(&entry.filename) == base_prefix
                        }
                    } else {
                        entry.filename.starts_with(&root_prefix)
                    }
                })
                .filter(|e| {
                    extension_filter
                        .as_deref()
                        .is_none_or(|ext| e.extension == ext)
                })
                .filter(|entry| {
                    q.is_none_or(|q| {
                        entry
                            .filename
                            .strip_prefix(&root_prefix)
                            .unwrap_or(&entry.filename)
                            .to_ascii_lowercase()
                            .contains(q)
                            || entry.filename.to_ascii_lowercase().contains(q)
                    })
                })
                .collect();

            let filtered_count = entries.len();
            let directory_count = directory_summaries.len();
            let total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();

            entries = entries
                .into_iter()
                .skip(filter.offset)
                .take(filter.limit.unwrap_or(usize::MAX))
                .collect();

            let entries_json = entries
                .iter()
                .map(|entry| {
                    let path = if folder_mode {
                        entry
                            .filename
                            .strip_prefix(&base_child_prefix)
                            .unwrap_or("")
                    } else {
                        entry.filename.strip_prefix(&root_prefix).unwrap_or("")
                    };
                    format!(
                        "{{\"type\":\"file\",\"path\":\"{}\",\"virtual_path\":\"{}\",\"size\":{},\"extension\":\"{}\"}}",
                        json_escape(path),
                        json_escape(&entry.filename),
                        entry.size,
                        json_escape(&entry.extension)
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let directories_json = directory_summaries
                .iter()
                .map(|(directory, (file_count, total_bytes))| {
                    let path = if selected_folder.is_empty() {
                        directory.as_str()
                    } else {
                        directory
                            .strip_prefix(&format!("{selected_folder}/"))
                            .unwrap_or(directory)
                    };
                    format!(
                        "{{\"type\":\"directory\",\"name\":\"{}\",\"path\":\"{}\",\"virtual_path\":\"{}/{}\",\"file_count\":{},\"total_bytes\":{}}}",
                        json_escape(path),
                        json_escape(path),
                        json_escape(root_label),
                        json_escape(directory),
                        file_count,
                        total_bytes
                    )
                })
                .collect::<Vec<_>>()
                .join(",");

            let response_body = format!(
                "{{\"label\":\"{}\",\"folder\":{},\"recursive\":{},\"entries\":[{}],\"directories\":[{}],\"count\":{},\"filtered_count\":{},\"directory_count\":{},\"total_bytes\":{},\"offset\":{},\"limit\":{}}}",
                json_escape(&root.label),
                json_option((!selected_folder.is_empty()).then_some(selected_folder.as_str())),
                recursive,
                entries_json,
                directories_json,
                root.files,
                filtered_count,
                directory_count,
                total_bytes,
                filter.offset,
                json_usize_option(filter.limit)
            );

            drop(shares);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body: response_body,
            })
        }
        ("POST", "/api/shares/rescan") => {
            let snapshot = match rebuild_share_index(state).await {
                Ok(snapshot) => snapshot,
                Err(error) => return Ok(share_rebuild_error_response(&error)),
            };
            record_event(
                state,
                "share.scan.completed",
                "shares",
                Some(format!("{} files", snapshot.entries.len())),
            )
            .await;
            Ok(HttpResponse {
                status: "202 Accepted",
                content_type: "application/json",
                body: snapshot.json(),
            })
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
