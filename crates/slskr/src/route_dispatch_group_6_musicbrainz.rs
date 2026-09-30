async fn route_dispatch_group_6_musicbrainz(
    context: &RouteDispatchContext<'_, '_>,
) -> RouteDispatchResult {
    let RouteDispatchContext {
        method,
        normalized_path,
        authorization,
        body,
        state,
        route,
        headers,
        state_arc,
        extended_mutation,
        request_is_versioned_v0,
    } = context.clone();
    match (method, normalized_path) {
        ("GET", "/api/musicbrainz/albums/completion") => {
            if route.path.starts_with("/api/v0/") {
                let targets = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/album-target/");
                if !targets.is_empty() {
                    let discovery = state.content_discovery.read().await;
                    let albums = targets
                        .iter()
                        .map(|target| musicbrainz_target_completion_value(target, &discovery))
                        .collect::<Vec<_>>();
                    drop(discovery);
                    return Ok(routing::ok_response(
                        serde_json::json!({"albums": albums}).to_string(),
                    ));
                }
            }
            let library = state.library.read().await;
            let mut value =
                serde_json::from_str::<serde_json::Value>(&library.musicbrainz_completion_json())
                    .unwrap_or_else(|_| serde_json::json!({}));
            let albums = value["completion_status"].clone();
            value["albums"] = albums;
            drop(library);
            Ok(routing::ok_response(value.to_string()))
        }

        ("GET", path)
            if path.starts_with("/api/musicbrainz/artist/")
                && path.ends_with("/discography-coverage") =>
        {
            let Some(artist) =
                path_segment_between(path, "/api/musicbrainz/artist/", "/discography-coverage")
            else {
                return Ok(routing::not_found_response());
            };
            let artist = decoded_path_segment(artist);
            if uuid::Uuid::parse_str(&artist).is_ok() {
                let profile = query_parameter(route.query, "profile")
                    .unwrap_or_else(|| "CoreDiscography".to_owned());
                let force_refresh = query_parameter(route.query, "forceRefresh")
                    .and_then(|value| parse_bool_value(&value))
                    .unwrap_or(false);
                let settings = state.integration_settings.read().await.musicbrainz.clone();
                match musicbrainz_discography_coverage_with_settings(
                    state,
                    &settings,
                    &artist,
                    &profile,
                    force_refresh,
                )
                .await
                {
                    Ok(Some(value)) => return Ok(routing::ok_response(value.to_string())),
                    Ok(None) => return Ok(routing::not_found_response()),
                    Err(error) => {
                        return Ok(routing::service_unavailable_response(&format!(
                            "MusicBrainz coverage lookup failed: {error}"
                        )))
                    }
                }
            }
            let library = state.library.read().await;
            let json = library.discography_coverage_json(&artist);
            drop(library);
            Ok(routing::ok_response(json))
        }

        ("GET", "/api/musicbrainz/release-radar/notifications") => {
            if route.path.starts_with("/api/v0/") {
                let unread_only = query_parameter(route.query, "unreadOnly")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"));
                let mut notifications = state
                    .controller_features
                    .read()
                    .await
                    .values_with_prefix("musicbrainz/radar/notification/");
                notifications.retain(|notification| {
                    !unread_only
                        || !notification
                            .get("read")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false)
                });
                notifications.sort_by(|left, right| {
                    right["firstSeenAt"]
                        .as_str()
                        .cmp(&left["firstSeenAt"].as_str())
                        .then_with(|| left["artistId"].as_str().cmp(&right["artistId"].as_str()))
                });
                return Ok(routing::ok_response(
                    serde_json::Value::Array(notifications).to_string(),
                ));
            }
            let wishlist = state.wishlist.read().await;
            let notifications = wishlist
                .records
                .iter()
                .flat_map(|record| record.items.iter())
                .map(|item| {
                    serde_json::json!({
                        "id": format!("release-radar-{}", item.id),
                        "artist": item.artist,
                        "title": item.title,
                        "searchText": item.search_text(),
                        "source": "wishlist",
                    })
                })
                .collect::<Vec<_>>();
            drop(wishlist);
            Ok(routing::ok_response(
                serde_json::Value::Array(notifications).to_string(),
            ))
        }
        _ => Err(ROUTE_NOT_HANDLED.to_owned()),
    }
}
