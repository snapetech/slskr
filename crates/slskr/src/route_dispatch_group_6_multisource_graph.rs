async fn route_dispatch_group_6_multisource_graph(
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
        ("GET", path) if path.starts_with("/api/multisource/jobs/") => {
            let Some(job_id) = path_segment_after(path, "/api/multisource/jobs/") else {
                return Ok(routing::not_found_response());
            };
            let job_id = decoded_path_segment(job_id);
            let versioned_profile = route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native;
            let swarm = state.multisource.read().await;
            if let Some(job) = swarm.get(&job_id) {
                let body = if versioned_profile {
                    serde_json::json!({
                        "jobId": job.id,
                        "state": job.status,
                        "totalChunks": job.total_chunks,
                        "completedChunks": job.completed_chunks,
                        "percentComplete": if job.total_chunks > 0 {
                            job.completed_chunks as f64 * 100.0 / job.total_chunks as f64
                        } else {
                            0.0
                        },
                        "activeWorkers": 0,
                        "chunksPerSecond": 0.0,
                        "estimatedSecondsRemaining": 0.0,
                        "bytesDownloaded": job.bytes_downloaded,
                        "bytesDownloadedMB": job.bytes_downloaded as f64 / 1024.0 / 1024.0,
                    })
                    .to_string()
                } else {
                    serde_json::to_string(job)
                        .map_err(|error| format!("multisource job serialization failed: {error}"))?
                };
                drop(swarm);
                return Ok(routing::ok_response(body));
            }
            drop(swarm);
            let transfer_id = job_id.strip_prefix("transfer-").unwrap_or(&job_id);
            let transfers = state.transfers.read().await;
            let body = transfer_id
                  .parse::<u64>()
                  .ok()
                  .and_then(|id| transfers.entries.iter().find(|entry| entry.id == id))
                  .map(|entry| {
                      let size = entry.size.unwrap_or(0);
                      let progress = if size == 0 {
                          0.0
                      } else {
                          (entry.bytes_transferred as f64 / size as f64) * 100.0
                      };
                      serde_json::json!({
                          "id": job_id,
                          "status": entry.status,
                          "filename": entry.filename,
                          "sources": entry.peer_username.as_deref().map(|peer| vec![peer]).unwrap_or_default(),
                          "progress": progress,
                          "bytesTransferred": entry.bytes_transferred,
                          "size": size,
                          "updated_at": entry.updated_at,
                      })
                  });
            drop(transfers);
            // Matches the oracle's real GetJobStatus: an unknown job id
            // is a real 404, not a fabricated 200 with an invented
            // "not_found" status string.
            match body {
                Some(body) => Ok(routing::ok_response(body.to_string())),
                None => Ok(HttpResponse {
                    status: "404 Not Found",
                    content_type: "application/json",
                    body: r#"{"error":"Job not found. It may have completed or been cancelled."}"#
                        .to_owned(),
                }),
            }
        }

        ("GET", "/api/player/external-visualizer") => {
            let visualizer = state
                .media_services
                .read()
                .await
                .external_visualizer
                .clone();
            let resolved = resolve_external_visualizer_path(visualizer.command.as_deref());
            let working_directory = resolve_external_visualizer_working_directory(
                visualizer.working_directory.as_deref(),
                resolved.as_deref(),
            );
            let name = if visualizer.name.trim().is_empty() {
                "External visualizer"
            } else {
                visualizer.name.trim()
            };
            Ok(routing::ok_response(
                serde_json::json!({
                    "enabled": visualizer.launch_enabled,
                    "configured": visualizer.configured(),
                    "available": resolved.is_some(),
                    "name": name,
                    "path": visualizer.command.as_deref().unwrap_or_default().trim(),
                    "resolvedPath": resolved,
                    "workingDirectory": working_directory,
                    "arguments": visualizer.arguments,
                })
                .to_string(),
            ))
        }

        ("GET", path)
            if path.starts_with("/api/realm-subject-indexes/") && path.ends_with("/conflicts") =>
        {
            let Some(realm) =
                path_segment_between(path, "/api/realm-subject-indexes/", "/conflicts")
            else {
                return Ok(routing::not_found_response());
            };
            let realm = decoded_path_segment(realm);
            let report = state
                .realm_subject_indexes
                .read()
                .await
                .conflict_report(&realm);
            Ok(routing::ok_response(report.to_string()))
        }

        ("POST", "/api/discovery-graph") => {
            if route.path.starts_with("/api/v0/") {
                return Ok(discovery_graph::build_response(body, state).await);
            }
            let interests = state.interests.read().await;
            let wishlist = state.wishlist.read().await;
            let mut nodes = interests
                .liked
                .iter()
                .map(|interest| {
                    serde_json::json!({
                        "id": interest.id,
                        "label": interest.name,
                        "kind": "interest",
                    })
                })
                .collect::<Vec<_>>();
            nodes.extend(wishlist.records.iter().flat_map(|record| {
                record.items.iter().map(|item| {
                    serde_json::json!({
                        "id": item.id,
                        "label": item.search_text(),
                        "kind": "wishlist",
                    })
                })
            }));
            let count = nodes.len();
            drop(wishlist);
            drop(interests);
            Ok(routing::accepted_response(
                serde_json::json!({
                    "nodes": nodes,
                    "edges": [],
                    "count": count,
                    "status": if count == 0 { "empty" } else { "ready" },
                })
                .to_string(),
            ))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
