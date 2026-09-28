use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn live_response_identifier(
    responses: &[EndpointBody],
    endpoint: &str,
    keys: &[&str],
) -> Option<String> {
    endpoint_array(Some(responses), endpoint)
        .into_iter()
        .find_map(|item| value_text(&item, keys).filter(|value| safe_route_segment(value)))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn live_endpoint_segment(
    route_path: &str,
    endpoint: ApiEndpoint,
    responses: &[EndpointBody],
) -> Option<(String, String)> {
    let route_id = normalize_route_path(route_path)
        .contains(":id")
        .then(|| route_param_value_optional(route_path))
        .flatten();

    if endpoint.path.contains(":id") {
        let id = route_id.or_else(|| {
            if endpoint.path == "/searches/:id/responses" {
                live_response_identifier(
                    responses,
                    "/searches",
                    &["id", "searchId", "search.id", "token"],
                )
                .or_else(|| {
                    live_response_identifier(
                        responses,
                        "/searches/records",
                        &["id", "searchId", "search.id", "token"],
                    )
                })
            } else if endpoint.path.starts_with("/downloads/requests/") {
                live_response_identifier(
                    responses,
                    "/downloads/requests",
                    &["id", "requestId", "request.id", "request.requestId"],
                )
            } else if endpoint.path == "/share-grants/by-collection/:id" {
                live_response_identifier(
                    responses,
                    "/collections",
                    &["id", "collectionId", "collection.id"],
                )
            } else if endpoint.path == "/collections/:id/items" {
                live_response_identifier(
                    responses,
                    "/collections",
                    &["id", "collectionId", "collection.id"],
                )
            } else if endpoint.path.starts_with("/share-grants/") {
                live_response_identifier(
                    responses,
                    "/share-grants",
                    &["id", "grantId", "shareGrantId", "grant.id"],
                )
            } else if endpoint.path.starts_with("/sharegroups/") {
                live_response_identifier(
                    responses,
                    "/sharegroups",
                    &["id", "groupId", "shareGroupId", "group.id"],
                )
            } else if endpoint.path.starts_with("/wishlist/") {
                live_response_identifier(
                    responses,
                    "/wishlist",
                    &["id", "wishlistId", "searchId", "item.id"],
                )
            } else {
                None
            }
        })?;
        return Some((":id".to_string(), id));
    }

    if endpoint.path.contains(":username") {
        let username = if endpoint.path.starts_with("/conversations/") {
            live_response_identifier(
                responses,
                "/conversations",
                &["username", "user", "peer", "name"],
            )
        } else if endpoint.path.starts_with("/users/") {
            live_response_identifier(responses, "/users", &["username", "name", "user"])
        } else {
            None
        }?;
        return Some((":username".to_string(), username));
    }

    if endpoint.path.contains(":roomName") {
        let room =
            live_response_identifier(responses, "/rooms/joined", &["name", "roomName", "room"])?;
        return Some((":roomName".to_string(), room));
    }

    None
}

#[cfg(target_arch = "wasm32")]
pub(super) fn concrete_live_endpoint_path(
    route_path: &str,
    endpoint: ApiEndpoint,
    responses: &[EndpointBody],
) -> Option<String> {
    if endpoint.path == "/library/health/issues/by-type" {
        let library_path = endpoint_body(responses, "/shares")
            .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
            .and_then(|value| {
                value
                    .get("local")
                    .and_then(serde_json::Value::as_array)
                    .or_else(|| value.as_array())
                    .and_then(|entries| entries.first())
                    .and_then(|entry| {
                        ["localPath", "raw", "path", "directory"]
                            .iter()
                            .find_map(|key| entry.get(*key).map(json_scalar_preview))
                    })
            })
            .filter(|path| !path.trim().is_empty())?;
        return Some(endpoint_url(&format!(
            "/library/health/issues/by-type?libraryPath={}",
            percent_encode_query(&library_path)
        )));
    }
    let path = endpoint.path;
    if !path.contains(':') {
        return Some(endpoint_url(path));
    }

    let (placeholder, value) = live_endpoint_segment(route_path, endpoint, responses)?;
    Some(endpoint_url(&path.replace(&placeholder, &value)))
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn refresh_route_data(window: &web_sys::Window) -> Result<(), JsValue> {
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-route-data") else {
        return Ok(());
    };
    let summary = document.get_element_by_id("slskr-route-summary");
    let page_data = document.get_element_by_id("slskr-page-data");
    let path = window.location().pathname()?;
    let Some(page) = route_page(&path) else {
        return Ok(());
    };
    set_live_status(&document, "Refreshing live data");
    if let Some(page_data) = page_data.as_ref() {
        page_data.set_attribute("data-slskr-live-state", "pending")?;
    }
    if let Some(route_data) = document.get_element_by_id("slskr-route-data") {
        route_data.set_attribute("data-slskr-live-state", "pending")?;
    }

    let mut rendered = String::new();
    let mut responses = Vec::new();
    let mut errors = 0;
    for endpoint in route_endpoints(page.surface)
        .into_iter()
        .filter(|endpoint| endpoint.method == "GET")
    {
        // Parameterized probes are only valid after their collection endpoint
        // has supplied a real identifier.  Skipping an unavailable dependent
        // probe is preferable to sending a fabricated ID and turning an
        // empty/disconnected workspace into a stream of false errors.
        let Some(url) = concrete_live_endpoint_path(&path, endpoint, &responses) else {
            continue;
        };
        let row = match fetch_text(window, &url).await {
            Ok(body) => {
                responses.push(EndpointBody {
                    endpoint,
                    body: body.clone(),
                });
                runtime_probe_result_html(&[(endpoint.method, &url, Ok(body.as_str()))])
            }
            Err(error) => {
                errors += 1;
                let message = error
                    .as_string()
                    .unwrap_or_else(|| "request failed".to_string());
                runtime_probe_result_html(&[(endpoint.method, &url, Err(message.as_str()))])
            }
        };
        rendered.push_str(&row);
        status.set_inner_html(&rendered);
        if let Some(summary) = summary.as_ref() {
            summary.set_inner_html(&route_workflow_stats_html(
                route_kind(&path),
                Some(&responses),
            ));
        }
        if let Some(page_data) = page_data.as_ref() {
            page_data.set_inner_html(&route_workspace_result_html(&path, &responses));
            mount_workspace_tabs(&document)?;
            mount_data_cards(&document)?;
            mount_reference_actions(window, &document)?;
            mount_native_tables(&document)?;
            mount_native_subviews(&document)?;
            mount_native_actions(&document)?;
            mount_transfer_columns(&document)?;
            mount_download_policy_controls(&document)?;
            mount_native_filters(&document)?;
            mount_native_sorters(&document)?;
            mount_browser_local_panels(window, &document)?;
        }
    }
    if let Some(page_data) = page_data.as_ref() {
        page_data.set_inner_html(&route_workspace_result_html(&path, &responses));
        mount_workspace_tabs(&document)?;
        mount_data_cards(&document)?;
        mount_reference_actions(window, &document)?;
        mount_native_tables(&document)?;
        mount_native_subviews(&document)?;
        mount_native_actions(&document)?;
        mount_transfer_columns(&document)?;
        mount_download_policy_controls(&document)?;
        mount_native_filters(&document)?;
        mount_native_sorters(&document)?;
        mount_browser_local_panels(window, &document)?;
        page_data.set_attribute(
            "data-slskr-live-state",
            if errors == 0 { "ready" } else { "error" },
        )?;
    }
    if let Some(route_data) = document.get_element_by_id("slskr-route-data") {
        route_data.set_attribute(
            "data-slskr-live-state",
            if errors == 0 { "ready" } else { "error" },
        )?;
    }
    let message = if errors == 0 {
        format!("Updated {} live probes", responses.len())
    } else {
        format!("Updated {} live probes, {} errors", responses.len(), errors)
    };
    set_live_status(&document, &message);

    Ok(())
}
