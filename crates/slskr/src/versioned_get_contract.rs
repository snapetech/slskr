use super::*;

pub(super) async fn versioned_get_failure_contract(
    path: &str,
    query: Option<&str>,
    state: &AppState,
) -> Option<HttpResponse> {
    if !path.starts_with("/api/v0/") {
        return None;
    }

    if state.config.controller_profile == ControllerProfile::Legacy
        && matches!(
            path,
            "/api/v0/mesh/health" | "/api/v0/signals/config" | "/api/v0/signals/status"
        )
    {
        return Some(HttpResponse {
            status: "400 Bad Request",
            content_type: "application/problem+json",
            body: serde_json::json!({
                "type": "https://docs.api-versioning.org/problems#unsupported",
                "title": "Unsupported API version",
                "status": 400,
                "detail": "The HTTP resource does not support API version 0.",
                "code": "UnsupportedApiVersion",
            })
            .to_string(),
        });
    }

    // FairnessController evaluates its durable TrafficStats row before it
    // builds the decision DTO.  Preserve the frozen native profile 500 contract when
    // that SQLite dependency is present but unavailable; in-memory/no-DB
    // deployments retain the neutral zero-total response.
    if state.config.controller_profile == ControllerProfile::Native
        && path == "/api/v0/fairness/summary"
    {
        if let Some(db) = state.db.as_ref() {
            if db.get_traffic_totals().await.is_err() {
                return Some(routing::internal_server_error_response(
                    "fairness storage unavailable",
                ));
            }
        }
    }

    if matches!(
        state.config.controller_profile,
        ControllerProfile::Legacy | ControllerProfile::Native
    ) && path == "/api/v0/conversations"
        && (query_bool_is_invalid(query, "includeInactive")
            || query_bool_is_invalid(query, "unAcknowledgedOnly"))
    {
        return Some(routing::bad_request_response(
            "The query value must be a boolean",
        ));
    }

    if matches!(
        state.config.controller_profile,
        ControllerProfile::Legacy | ControllerProfile::Native
    ) && ((path == "/api/v0/application/build"
        && query_bool_is_invalid(query, "checkForUpdates"))
        || (path == "/api/v0/application/version/latest"
            && query_bool_is_invalid(query, "forceCheck")))
    {
        return Some(routing::bad_request_response(
            "The query value must be a boolean",
        ));
    }

    if state.config.controller_profile == ControllerProfile::Native
        && path == "/api/v0/network/stats"
        && query_bool_is_invalid(query, "includePeers")
    {
        return Some(routing::bad_request_response(
            "The query value must be a boolean",
        ));
    }

    if state.config.controller_profile == ControllerProfile::Native
        && matches!(path, "/api/v0/opinions" | "/api/v0/opinions/summary")
    {
        let enum_query_is_invalid = |name: &str, max_numeric: i64, names: &[&str]| {
            query_parameter(query, name).is_some_and(|value| {
                let value = value.trim();
                value.parse::<i64>().map_or_else(
                    |_| !names.iter().any(|name| value.eq_ignore_ascii_case(name)),
                    |numeric| !(0..=max_numeric).contains(&numeric),
                )
            })
        };
        if enum_query_is_invalid(
            "subjectType",
            11,
            &[
                "Unknown",
                "User",
                "File",
                "ContentHash",
                "Artist",
                "Album",
                "Track",
                "Pod",
                "Source",
                "MeshPeer",
                "SearchTerm",
                "Other",
            ],
        ) || enum_query_is_invalid(
            "kind",
            9,
            &[
                "Unknown",
                "Like",
                "Hate",
                "Trust",
                "Distrust",
                "Block",
                "Recommend",
                "Quarantine",
                "VerifiedGood",
                "VerifiedBad",
            ],
        ) {
            return Some(routing::bad_request_response(
                "The query value is not valid.",
            ));
        }
        if path == "/api/v0/opinions"
            && (query_bool_is_invalid(query, "includeExpired")
                || query_parameter(query, "limit")
                    .is_some_and(|value| value.parse::<i32>().is_err()))
        {
            return Some(routing::bad_request_response(
                "The query value is not valid.",
            ));
        }
    }

    let relay_settings = state.advanced_networking.read().await.relay.clone();
    if (relay_versioned_route_known("GET", path)
        && !relay_versioned_route_allowed(&relay_settings, "GET", path))
        || matches!(
            path,
            "/api/v0/soulseek/mesh-rendezvous/discover" | "/api/v0/soulseek/mesh-rendezvous/users"
        )
    {
        return Some(routing::forbidden_response(
            "feature is disabled by configuration",
        ));
    }

    let uuid_prefixes = [
        "/api/v0/collections/",
        "/api/v0/contacts/",
        // Must precede the shorter "/api/v0/share-grants/" prefix below --
        // strip_prefix matches whichever prefix is checked first, so a
        // more specific nested prefix must always come before a shorter
        // one it is contained within, or the nested route's real id
        // segment (e.g. a collection UUID) never reaches its own check
        // and instead gets validated against the wrong path segment
        // ("by-collection").
        "/api/v0/share-grants/by-collection/",
        "/api/v0/share-grants/",
        "/api/v0/sharegroups/",
        "/api/v0/wishlist/",
        "/api/v0/multisource/jobs/",
    ];
    for prefix in uuid_prefixes {
        if let Some(value) = path.strip_prefix(prefix) {
            // `nearby` is a literal sibling action on ContactsController,
            // not a contact id.  It must reach the controller's nearby
            // handler instead of being rejected by the Guid route guard.
            if prefix == "/api/v0/contacts/"
                && value.split('/').next().unwrap_or_default() == "nearby"
            {
                continue;
            }
            if prefix == "/api/v0/share-grants/"
                && value.split('/').next().unwrap_or_default() == "announce"
            {
                continue;
            }
            let value = decoded_path_segment(value.split('/').next().unwrap_or_default());
            if prefix == "/api/v0/wishlist/" && is_legacy_wishlist_item_id(&value) {
                continue;
            }
            if uuid::Uuid::parse_str(&value).is_err() {
                return Some(routing::bad_request_response("The request is invalid"));
            }
            // Stop at the first (most specific, since nested prefixes are
            // ordered before the shorter prefixes they're contained
            // within) matching prefix -- otherwise a path validated
            // successfully here would keep matching shorter prefixes
            // later in the list and get re-validated against the wrong
            // path segment.
            break;
        }
    }

    for prefix in ["/api/v0/transfers/downloads/", "/api/v0/transfers/uploads/"] {
        if let Some(value) = path.strip_prefix(prefix) {
            let segments = value.split('/').collect::<Vec<_>>();
            let first = segments.first().copied().unwrap_or_default();
            let special = matches!(
                first,
                "accelerated" | "auto-replace" | "stuck" | "user-stats" | "diagnostics" | "stats"
            );
            if !special && segments.len() >= 2 {
                let valid_id = if first == "batches" {
                    uuid::Uuid::parse_str(segments[1]).is_ok()
                } else {
                    segments[1].parse::<u64>().is_ok()
                };
                if !valid_id {
                    return Some(routing::bad_request_response("The request is invalid"));
                }
            }
        }
    }

    if path.starts_with("/api/v0/mediacore/ipld/traverse/") {
        let content_id =
            decoded_path_segment(path.trim_start_matches("/api/v0/mediacore/ipld/traverse/"));
        if !content_id.starts_with("content:") {
            return Some(routing::bad_request_response("The request is invalid"));
        }
    }

    let has_only_api_version_query = query.is_none_or(|query| {
        query.split('&').all(|pair| {
            pair.split_once('=')
                .map(|(name, _)| name.eq_ignore_ascii_case("api-version"))
                .unwrap_or(false)
        })
    });
    let missing_required_query = matches!(
        path,
        "/api/v0/library/health/issues/by-type"
            | "/api/v0/multisource/search"
            | "/api/v0/multisource/users"
            | "/api/v0/opinions/summary"
            | "/api/v0/podcore/content/metadata"
            | "/api/v0/podcore/content/search"
            | "/api/v0/telemetry/reports/transfers/exceptions"
            | "/api/v0/telemetry/reports/transfers/exceptions/pareto"
            | "/api/v0/telemetry/reports/transfers/leaderboard"
    ) && has_only_api_version_query;
    if missing_required_query {
        return Some(routing::bad_request_response(
            "A required query value is missing",
        ));
    }

    if let Some(username) = path
        .strip_prefix("/api/v0/conversations/")
        .and_then(|value| value.split('/').next())
    {
        let username = decoded_path_segment(username);
        if username != "activity"
            && !username.trim().is_empty()
            && !state
                .messages
                .read()
                .await
                .records
                .iter()
                .any(|record| record.username.eq_ignore_ascii_case(&username))
        {
            return Some(routing::not_found_response());
        }
    }

    if let Some(job_id) = path.strip_prefix("/api/v0/jobs/") {
        if !job_id.contains('/')
            && state
                .searches
                .read()
                .await
                .get_by_identifier(&decoded_path_segment(job_id))
                .is_none()
        {
            return Some(routing::not_found_response());
        }
    }

    if let Some(search_id) = path
        .strip_prefix("/api/v0/searches/")
        .and_then(|value| value.strip_suffix("/responses"))
    {
        if state.config.controller_profile == ControllerProfile::Native
            && !controller_search_identifier_is_valid(search_id)
        {
            // SearchResponsesController binds this route segment as a Guid.
            // Validate it before the missing-record check so malformed input
            // remains a 400 instead of being mistaken for an unknown search.
            return Some(routing::bad_request_response("The request is invalid"));
        }
        if state
            .searches
            .read()
            .await
            .get_by_identifier(&decoded_path_segment(search_id))
            .is_none()
        {
            return Some(routing::not_found_response());
        }
    }

    if let Some(segments) = decoded_segments_after(path, "/api/v0/listening-party/radio/") {
        if let [party_id, content_id] = segments.as_slice() {
            // Matches the oracle's real StreamListedParty gate: the party
            // referenced by partyId must actually be listed, must allow
            // mesh streaming, and must currently be playing exactly this
            // contentId -- otherwise NotFound(), evaluated before any
            // stream ticket is even considered. Previously this checked
            // the unrelated global content-discovery shadow-record index,
            // which neither matched the oracle's real per-party gate nor
            // scoped the check to the specific party being queried.
            let listed = state
                .controller_features
                .read()
                .await
                .values_with_prefix("listening-party/")
                .into_iter()
                .any(|event| {
                    event.get("partyId").and_then(serde_json::Value::as_str) == Some(party_id)
                        && event.get("listed").and_then(serde_json::Value::as_bool) == Some(true)
                        && event
                            .get("allowMeshStreaming")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                        && event.get("contentId").and_then(serde_json::Value::as_str)
                            == Some(content_id)
                });
            if !listed {
                return Some(routing::not_found_response());
            }
        }
    }

    for (prefix, suffix) in [
        ("/api/v0/musicbrainz/artist/", "/discography-coverage"),
        ("/api/v0/musicbrainz/overlays/artist/", "/release-graph"),
    ] {
        if let Some(artist_id) = path_segment_between(path, prefix, suffix) {
            let artist_id = decoded_path_segment(artist_id);
            let is_musicbrainz_artist = prefix == "/api/v0/musicbrainz/artist/";
            let known_artist = state
                .library
                .read()
                .await
                .records
                .iter()
                .any(|record| record.artist.eq_ignore_ascii_case(&artist_id));
            let valid_musicbrainz_id =
                is_musicbrainz_artist && uuid::Uuid::parse_str(&artist_id).is_ok();
            if !(known_artist || valid_musicbrainz_id) {
                return Some(routing::not_found_response());
            }
        }
    }

    if let Some(peer_id) = path.strip_prefix("/api/v0/profile/") {
        if peer_id != "me" && !peer_id.contains('/') {
            let peer_id = decoded_path_segment(peer_id).trim().to_owned();
            if peer_id.is_empty() {
                return Some(routing::bad_request_response("PeerId is required."));
            }
            let local_peer_id = local_profile_peer_id(state);
            let known_user = state
                .users
                .read()
                .await
                .records
                .iter()
                .any(|user| user.username.eq_ignore_ascii_case(&peer_id));
            if !peer_id.eq_ignore_ascii_case(&local_peer_id) && !known_user {
                return Some(routing::not_found_response());
            }
        }
    }

    if path == "/api/v0/security/adversarial"
        && state.config.controller_profile == ControllerProfile::Native
    {
        // The frozen native profile controller's direct settings GET remains
        // unregistered even after a successful PUT and persisted YAML update.
        // Keep the mutation/readback store available to its own management
        // flow, but preserve the externally observable GET contract.
        return Some(HttpResponse {
            status: "404 Not Found",
            content_type: "text/plain; charset=utf-8",
            body: "Adversarial features are not configured".to_owned(),
        });
    }

    if matches!(
        path,
        "/api/v0/security/canaries" | "/api/v0/security/tor/status"
    ) {
        let feature_key = if path.ends_with("/canaries") {
            "security/profile/security/canaries"
        } else {
            "security/profile/security/tor"
        };
        let configured = state
            .controller_features
            .read()
            .await
            .get(feature_key)
            .is_some();
        if !configured {
            return Some(routing::not_found_response());
        }
    }

    if let Some(share_id) = path.strip_prefix("/api/v0/shares/") {
        let share_id = decoded_path_segment(share_id.split('/').next().unwrap_or_default());
        if !matches!(share_id.as_str(), "catalog" | "contents") && {
            let shares = state.shares.read().await;
            !shares
                .roots
                .iter()
                .any(|root| share_root_id(&root.label) == share_id)
        } {
            return Some(HttpResponse {
                status: "404 Not Found",
                content_type: "",
                body: String::new(),
            });
        }
    }

    for (prefix, direction) in [
        ("/api/v0/transfers/downloads/", 0_u32),
        ("/api/v0/transfers/uploads/", 1_u32),
    ] {
        if let Some(value) = path.strip_prefix(prefix) {
            let raw_username = value.split('/').next().unwrap_or_default();
            if matches!(
                raw_username,
                "" | "accelerated"
                    | "auto-replace"
                    | "stuck"
                    | "user-stats"
                    | "diagnostics"
                    | "stats"
                    | "batches"
            ) {
                continue;
            }
            let username = decoded_path_segment(raw_username);
            if state.config.controller_profile == ControllerProfile::Native
                && username.trim().is_empty()
            {
                return Some(routing::bad_request_response("The request is invalid"));
            }
            if !state.transfers.read().await.entries.iter().any(|entry| {
                entry.direction == direction
                    && entry
                        .peer_username
                        .as_deref()
                        .is_some_and(|peer| peer.eq_ignore_ascii_case(&username))
            }) {
                return Some(routing::not_found_response());
            }
        }
    }

    if let Some(username) = path
        .strip_prefix("/api/v0/users/")
        .and_then(|value| value.strip_suffix("/browse/status"))
    {
        if state
            .browse
            .read()
            .await
            .get(&decoded_path_segment(username))
            .is_none()
        {
            return Some(routing::not_found_response());
        }
    }

    if let Some(username) = path
        .strip_prefix("/api/v0/multisource/users/")
        .and_then(|value| value.strip_suffix("/files"))
    {
        let username = decoded_path_segment(username).trim().to_owned();
        if username.is_empty() {
            return Some(routing::bad_request_response("Username is required"));
        }
        if !state
            .searches
            .read()
            .await
            .records
            .iter()
            .flat_map(|search| &search.results)
            .any(|result| {
                result
                    .peer_username
                    .as_deref()
                    .is_some_and(|peer| peer.eq_ignore_ascii_case(&username))
            })
        {
            return Some(routing::bad_request_response(
                "No search results. Call /users?searchText=... first",
            ));
        }
    }

    None
}
