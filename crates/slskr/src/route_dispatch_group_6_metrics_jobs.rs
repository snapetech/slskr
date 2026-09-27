async fn route_dispatch_group_6_metrics_jobs(
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
        ("GET", "/api/telemetry/metrics") => {
            let transfers = state.transfers.read().await;
            let transfer_count = transfers.entries.len();
            drop(transfers);
            Ok(HttpResponse {
                status: "200 OK",
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: format!(
                    "# HELP slskr_telemetry_transfers Transfer count\n\
                     # TYPE slskr_telemetry_transfers gauge\n\
                     slskr_telemetry_transfers {}\n",
                    transfer_count
                ),
            })
        }

        // Matches the oracle exactly: TelemetryController.GetKpis and
        // MetricsController.GetKpis are different controllers mounted at
        // different routes, but both call the same
        // Telemetry.Prometheus.GetMetricsAsObject(include: KpiRegexes)
        // with an identical regex list, so they return identical
        // content. slskR's /api/telemetry/prometheus/kpis was already
        // fixed to the real dictionary-of-PrometheusMetric shape; this
        // sibling route reuses the exact same real data instead of its
        // own invented {kpis:[...], count} array.
        ("GET", "/api/telemetry/metrics/kpi") | ("GET", "/api/telemetry/metrics/kpis") => {
            let transfers = state.transfers.read().await;
            let searches = state.searches.read().await;
            let metrics = serde_json::json!({
                "slskr_transfers": prometheus_metric_json("slskr_transfers", "gauge", transfers.entries.len() as f64),
                "slskr_searches": prometheus_metric_json("slskr_searches", "gauge", searches.records.len() as f64),
            });
            drop(transfers);
            drop(searches);
            Ok(routing::ok_response(metrics.to_string()))
        }

        // ADDITIONAL MISSING GET ENDPOINTS (Phase 6)
        ("GET", "/api/multisource/jobs") => {
            let versioned_profile = route.path.starts_with("/api/v0/")
                && state.config.controller_profile == ControllerProfile::Native;
            let swarm = state.multisource.read().await;
            let mut jobs = if versioned_profile {
                swarm
                    .list()
                    .into_iter()
                    .map(|job| {
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
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                swarm
                    .list()
                    .into_iter()
                    .filter_map(|job| serde_json::to_value(job).ok())
                    .collect::<Vec<_>>()
            };
            drop(swarm);
            if !versioned_profile {
                let transfers = state.transfers.read().await;
                jobs.extend(transfers
                      .entries
                      .iter()
                      .filter(|entry| entry.direction == 0)
                      .map(|entry| {
                          let size = entry.size.unwrap_or(0);
                          let progress = if size == 0 {
                              0.0
                          } else {
                              (entry.bytes_transferred as f64 / size as f64) * 100.0
                          };
                          serde_json::json!({
                              "id": format!("transfer-{}", entry.id),
                              "status": entry.status,
                              "filename": entry.filename,
                              "sources": entry.peer_username.as_deref().map(|peer| vec![peer]).unwrap_or_default(),
                              "progress": progress,
                              "bytesTransferred": entry.bytes_transferred,
                              "size": size,
                              "updated_at": entry.updated_at,
                          })
                      }));
                drop(transfers);
            }
            let count = jobs.len();
            let json = serde_json::json!({
                "jobs": jobs,
                "count": count,
            })
            .to_string();
            Ok(routing::ok_response(json))
        }

        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
