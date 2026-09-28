#![allow(
    unused_variables,
    clippy::needless_borrow,
    clippy::needless_return,
    clippy::too_many_arguments,
    reason = "route groups share one compatibility-dispatch signature across frozen target profiles"
)]

use super::*;

pub(super) fn share_grant_store_matches(
    current: &ShareGrantStore,
    expected: &ShareGrantStore,
) -> bool {
    current.next_id == expected.next_id
        && current.updated_at == expected.updated_at
        && current.records.len() == expected.records.len()
        && current
            .records
            .iter()
            .zip(&expected.records)
            .all(|(current, expected)| {
                current.id == expected.id
                    && current.collection_id == expected.collection_id
                    && current.username == expected.username
                    && current.shared_at == expected.shared_at
                    && current.permissions == expected.permissions
            })
}

/// Roll back only stores that still contain this transaction's candidate.
/// A concurrent update to runtime state must not prevent an unchanged library
/// mutation from being restored (and vice versa).
pub(super) async fn rollback_library_runtime_if_unchanged(
    state: &AppState,
    previous_library: LibraryStore,
    mutated_library: LibraryStore,
    previous_runtime: RuntimeCompatState,
    mutated_runtime: RuntimeCompatState,
    failed_library_item: LibraryItemRecord,
) {
    let mut library = state.library.write().await;
    let mut runtime = state.runtime.write().await;
    if *library == mutated_library {
        *library = previous_library;
    } else if let Some(index) = library
        .records
        .iter()
        .position(|record| record == &failed_library_item)
    {
        library.records.remove(index);
    }
    if *runtime == mutated_runtime {
        *runtime = previous_runtime;
    } else if runtime.lidarr_manual_imports == mutated_runtime.lidarr_manual_imports {
        runtime.lidarr_manual_imports = previous_runtime.lidarr_manual_imports;
    }
}

const ROUTE_NOT_HANDLED: &str = "\0slskr-route-not-handled\0";
type RouteDispatchResult = Result<HttpResponse, String>;

fn route_is_unhandled(result: &RouteDispatchResult) -> bool {
    matches!(result, Err(error) if error == ROUTE_NOT_HANDLED)
}

fn complete_route_dispatch(response: RouteDispatchResult) -> RouteDispatchResult {
    response.inspect(|response| {
        let status_code: u16 = response
            .status
            .split(' ')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500);
        tracing::complete_request_span(status_code);
    })
}

/// Keep the most frequent immutable controller reads out of the large route
/// group futures. Constructing a future for the full compatibility match is
/// measurable on sub-millisecond responses, while these routes do not depend
/// on the rest of the dispatch context.
fn route_dispatch_fast_read(
    method: &str,
    normalized_path: &str,
    route: &routing::ParsedRoute<'_>,
    state: &AppState,
) -> Option<RouteDispatchResult> {
    match (method, normalized_path) {
        ("GET", "/") => Some(Ok(index_html_response())),
        ("HEAD", "/") => Some(Ok(head_response(index_html_response()))),
        ("GET", "/dashboard") => Some(Ok(fallback_dashboard_response())),
        ("HEAD", "/dashboard") => Some(Ok(head_response(fallback_dashboard_response()))),
        ("GET", "/api/health") => Some(Ok(health_response(&state.config))),
        ("GET", "/health") => Some(Ok(health_response(&state.config))),
        ("HEAD", "/health") => Some(Ok(head_response(health_response(&state.config)))),
        ("GET", "/health/mesh") => Some(Ok(mesh_health_response(&state.config))),
        ("HEAD", "/health/mesh") => Some(Ok(head_response(mesh_health_response(&state.config)))),
        ("GET", "/api/version") => Some(Ok(version_response())),
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && matches!(
                    route.path,
                    "/api/slskdn/capabilities" | "/api/v0/slskdn/capabilities"
                ) =>
        {
            None
        }
        ("GET", "/api/capabilities")
            if state.config.controller_profile == ControllerProfile::Native
                && route.path == "/api/v0/capabilities" =>
        {
            None
        }
        ("GET", "/api/capabilities") => Some(Ok(capabilities_response())),
        _ => None,
    }
}

fn parse_download_filter_update(body: &str) -> Result<Vec<String>, String> {
    let payload = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|_| "Download filter payload must be valid JSON".to_owned())?;
    let terms = payload
        .get("exclude")
        .or_else(|| payload.get("terms"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Download filter payload must contain an exclude array".to_owned())?;
    if terms.len() > 100 {
        return Err("Download filter supports at most 100 terms".to_owned());
    }
    let mut normalized = Vec::with_capacity(terms.len());
    for term in terms {
        let term = term
            .as_str()
            .ok_or_else(|| "Download filter terms must be strings".to_owned())?
            .trim();
        if term.is_empty() {
            return Err("Download filter terms cannot be blank".to_owned());
        }
        if term.chars().count() > 256 {
            return Err("Download filter terms must be at most 256 characters".to_owned());
        }
        if !normalized.iter().any(|existing| existing == term) {
            normalized.push(term.to_owned());
        }
    }
    Ok(normalized)
}

fn share_grant_permissions_from_request(body: &str, versioned: bool) -> String {
    let payload = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
    if !versioned {
        return extract_json_string_field(body, "permissions")
            .unwrap_or_else(|| "download,stream".to_owned());
    }

    let bool_field = |camel: &str, snake: &str, default: bool| {
        payload
            .get(camel)
            .or_else(|| payload.get(snake))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default)
    };
    let permissions = [
        bool_field("allowDownload", "allow_download", true).then_some("download"),
        bool_field("allowStream", "allow_stream", true).then_some("stream"),
        bool_field("allowReshare", "allow_reshare", false).then_some("reshare"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if permissions.is_empty() {
        "none".to_owned()
    } else {
        permissions.join(",")
    }
}

pub(super) async fn update_download_filter(state: &AppState, body: &str) -> HttpResponse {
    if !effective_remote_configuration(state) {
        return controller_forbidden_response();
    }
    let terms = match parse_download_filter_update(body) {
        Ok(terms) => terms,
        Err(error) => return routing::bad_request_response(&error),
    };
    let existing = match crate::read_controller_compatibility_yaml(&state.config) {
        Ok(Some(text)) => text,
        Ok(None) => String::new(),
        Err(error) => return routing::service_unavailable_response(&error),
    };
    let mut yaml = if existing.trim().is_empty() {
        serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
    } else {
        match serde_yaml::from_str::<serde_yaml::Value>(&existing) {
            Ok(value @ serde_yaml::Value::Mapping(_)) => value,
            Ok(serde_yaml::Value::Null) => serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
            Ok(_) | Err(_) => {
                return routing::bad_request_response(
                    "Existing configuration is not a YAML mapping",
                )
            }
        }
    };
    let serde_yaml::Value::Mapping(root) = &mut yaml else {
        return routing::bad_request_response("Existing configuration is not a YAML mapping");
    };
    let filters_key = serde_yaml::Value::String("filters".to_owned());
    let download_key = serde_yaml::Value::String("download".to_owned());
    let exclude_key = serde_yaml::Value::String("exclude".to_owned());
    let filters = root
        .entry(filters_key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let serde_yaml::Value::Mapping(filters) = filters else {
        return routing::bad_request_response("filters must be a YAML mapping");
    };
    let download = filters
        .entry(download_key)
        .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let serde_yaml::Value::Mapping(download) = download else {
        return routing::bad_request_response("filters.download must be a YAML mapping");
    };
    download.insert(
        exclude_key,
        serde_yaml::Value::Sequence(terms.into_iter().map(serde_yaml::Value::String).collect()),
    );
    let yaml_text = match serde_yaml::to_string(&yaml) {
        Ok(text) => text,
        Err(error) => {
            return routing::service_unavailable_response(&format!(
                "failed to serialize configuration: {error}"
            ))
        }
    };
    // The shared controller YAML writer accepts the target's JSON-string
    // envelope (`"<yaml>"`), not an object wrapper.  Keep the download
    // policy endpoint on that same validated persistence path.
    apply_controller_yaml_upload(&serde_json::Value::String(yaml_text).to_string(), state).await
}

pub(super) async fn download_policy_response(
    state: &AppState,
    filenames: &[String],
) -> Option<HttpResponse> {
    let exclusions = effective_download_exclusions(state).await;
    let blocked = filenames
        .iter()
        .filter_map(|filename| {
            crate::download_filter::matching_exclusion(filename, &exclusions).map(|exclusion| {
                serde_json::json!({
                    "filename": filename,
                    "exclusion": exclusion,
                })
            })
        })
        .collect::<Vec<_>>();
    if blocked.is_empty() {
        return None;
    }
    Some(HttpResponse {
        status: "403 Forbidden",
        content_type: "application/json",
        body: serde_json::json!({
            "type": "download_blocked",
            "title": "Download blocked",
            "detail": "Every requested file matched a configured global download exclusion.",
            "blocked": blocked,
        })
        .to_string(),
    })
}

#[allow(
    dead_code,
    reason = "used by focused and bounded in-process route tests"
)]
pub(super) async fn route_http_request_with_headers(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: &AppState,
    headers: &RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    let request = routing::RouteRequest::new(method, path, authorization, body, headers);
    route_http_request_inner(request, state, None).await
}

#[allow(
    dead_code,
    reason = "the historical dispatcher owns the legacy server call path"
)]
pub(super) async fn route_http_request_with_state(
    method: &str,
    path: &str,
    authorization: Option<&str>,
    body: &str,
    state: Arc<AppState>,
    headers: &RequestSecurityHeaders,
) -> Result<HttpResponse, String> {
    let state_arc = state.clone();
    let request = routing::RouteRequest::new(method, path, authorization, body, headers);
    route_http_request_inner(request, &state, Some(state_arc)).await
}

fn versioned_share_rescan_response(state: &AppState, state_arc: Arc<AppState>) -> HttpResponse {
    let permit = match Arc::clone(&state.share_scans).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            // The legacy and native profile controllers expose PUT /api/v0/shares
            // as an asynchronous, idempotent trigger.  A request that arrives
            // while the scan is already running still returns an empty 200; the
            // separate /shares/rescan route retains the explicit busy error.
            return HttpResponse {
                status: "200 OK",
                content_type: "",
                body: String::new(),
            };
        }
    };
    state.spawn_managed_task(async move {
        match rebuild_share_index_with_permit(&state_arc, permit).await {
            Ok(snapshot) => {
                record_event(
                    &state_arc,
                    "share.scan.completed",
                    "shares",
                    Some(format!("{} files", snapshot.entries.len())),
                )
                .await;
            }
            Err(error) => {
                if error == super::SHARE_SCAN_CANCELLED_ERROR {
                    return;
                }
                record_daemon_log(
                    &state_arc,
                    crate::logging::LogLevel::Warn,
                    "shares",
                    format!("versioned share scan failed: {error}"),
                )
                .await;
                record_event(
                    &state_arc,
                    "share.scan.failed",
                    "shares",
                    Some("share index unavailable".to_owned()),
                )
                .await;
            }
        }
    });
    HttpResponse {
        status: "200 OK",
        content_type: "",
        body: String::new(),
    }
}

fn disconnected_search_conflict_response(state: &AppState, display_state: &str) -> HttpResponse {
    let message = if state.config.controller_profile == ControllerProfile::Native {
        "Search could not be started".to_owned()
    } else {
        format!(
            "The server connection must be connected and logged in to perform a search (currently: {display_state})"
        )
    };
    HttpResponse {
        status: "409 Conflict",
        content_type: "application/json",
        body: serde_json::to_string(&message)
            .unwrap_or_else(|_| "\"Search could not be started\"".to_owned()),
    }
}

fn json_string_field(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

include!("musicbrainz_targets.rs");

include!("route_dispatch_request_flow.rs");

include!("route_dispatch_group_0.rs");
include!("route_dispatch_group_0_control_plane.rs");
include!("route_dispatch_group_0_mesh_data.rs");
include!("route_dispatch_group_1.rs");
include!("route_dispatch_group_1_discovery.rs");
include!("route_dispatch_group_1_telemetry.rs");
include!("route_dispatch_group_1_events.rs");
include!("route_dispatch_group_1_shares.rs");
include!("route_dispatch_group_2.rs");
include!("route_dispatch_group_2_session_search.rs");
include!("route_dispatch_group_2_downloads.rs");
include!("route_dispatch_group_2_transfer_status.rs");
include!("route_dispatch_group_2_transfer_files.rs");
include!("route_dispatch_group_2_search_rooms.rs");
include!("route_dispatch_group_3.rs");
include!("route_dispatch_group_3_people_browse.rs");
include!("route_dispatch_group_3_admin_controls.rs");
include!("route_dispatch_group_3_room_membership.rs");
include!("route_dispatch_group_3_options_diagnostics.rs");
include!("route_dispatch_group_4.rs");
include!("route_dispatch_group_4_collections.rs");
include!("route_dispatch_group_4_wishlist.rs");
include!("route_dispatch_group_4_contacts_sharegroups.rs");
include!("route_dispatch_group_4_notes_interests_grants.rs");
include!("route_dispatch_group_5.rs");
include!("route_dispatch_group_5_library_profile.rs");
include!("route_dispatch_group_5_conversations_jobs.rs");
include!("route_dispatch_group_5_configuration_bridge.rs");
include!("route_dispatch_group_5_mutations.rs");
include!("route_dispatch_group_6.rs");
include!("route_dispatch_group_6_admin_discovery.rs");
include!("route_dispatch_group_6_admin_routes.rs");
include!("route_dispatch_group_6_relay_controller.rs");
include!("route_dispatch_group_6_telemetry.rs");
include!("route_dispatch_group_6_metrics_jobs.rs");
include!("route_dispatch_group_6_pods.rs");
include!("route_dispatch_group_6_federation_security.rs");
include!("route_dispatch_group_6_multisource_graph.rs");
include!("route_dispatch_group_6_media_jobs.rs");
include!("route_dispatch_group_6_security_shares.rs");
include!("route_dispatch_group_6_integrations.rs");
include!("route_dispatch_group_6_musicbrainz.rs");
include!("route_dispatch_group_7.rs");
include!("route_dispatch_group_7_media_profile_import.rs");
include!("route_dispatch_group_7_stream_services.rs");
include!("route_dispatch_group_7_podcore.rs");
include!("route_dispatch_group_7_network_admin.rs");
