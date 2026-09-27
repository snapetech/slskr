async fn route_http_request_inner(
    request: routing::RouteRequest<'_>,
    state: &AppState,
    state_arc: Option<Arc<AppState>>,
) -> Result<HttpResponse, String> {
    route_http_request_inner_with_batch(request, state, state_arc, true).await
}

async fn route_http_request_inner_with_batch(
    request: routing::RouteRequest<'_>,
    state: &AppState,
    state_arc: Option<Arc<AppState>>,
    allow_batch: bool,
) -> Result<HttpResponse, String> {
    let routing::RouteRequest {
        method,
        path,
        authorization,
        body,
        headers,
    } = request;
    // Start request tracing
    let span = tracing::RequestSpan::new(
        method.to_string(),
        path.to_string(),
        None, // user_agent - would need to pass from connection
        None, // client_ip can be added from connection info
    );
    let _correlation_id = span.correlation_id.clone();
    tracing::set_request_span(span);

    let route = request.parsed();
    let request_is_versioned_v0 = route.path.starts_with("/api/v0/");

    let (raw_path, _) = crate::utils::split_request_target(path);
    let mut decoded_path = raw_path.to_owned();
    let mut traversal = false;
    for _ in 0..=2 {
        if crate::utils::contains_traversal_component(&decoded_path) {
            traversal = true;
            break;
        }
        let next = crate::utils::percent_decode(&decoded_path);
        if next == decoded_path {
            break;
        }
        decoded_path = next;
    }
    if traversal {
        return Ok(routing::bad_request_response(
            "One or more files in the request contain a dangerous path traversal segment",
        ));
    }

    // Normalize versioned paths before matching so static and dynamic routes
    // share the same dispatch behavior.
    let mut normalized_path = if let Some(versioned_path) = route
        .normalized_path
        .strip_prefix("/api/v0/")
        .or_else(|| route.normalized_path.strip_prefix("/api/v1/"))
        .or_else(|| route.normalized_path.strip_prefix("/api/v2/"))
    {
        format!("/api/{}", versioned_path)
    } else {
        route.normalized_path.to_string()
    };
    if route.path == "/api/server/status" {
        normalized_path = "/api/server/status".to_owned();
    }

    if !allow_batch && method == "POST" && normalized_path == "/api/batch" {
        return Ok(routing::bad_request_response(
            "nested batch operations are not supported",
        ));
    }

    // native profile's mesh-gateway middleware short-circuits every /mesh request
    // while the feature is disabled, before auth or controller fallback can
    // change the wire response.  Keep the same disabled contract here.
    if (normalized_path == "/mesh" || normalized_path.starts_with("/mesh/"))
        && !state.config.mesh_gateway.enabled
    {
        return Ok(mesh_gateway_disabled_response());
    }
    if normalized_path == "/mesh" || normalized_path.starts_with("/mesh/") {
        if let Some(response) = mesh_gateway_auth_failure(state, &headers) {
            return Ok(response);
        }
    }

    if route.path == "/api/v0/share-grants/announce" && method == "POST" {
        if !e2e_share_announce_enabled() {
            return Ok(routing::not_found_response());
        }
        if !is_authorized(&state.config, authorization, headers.cookie.as_deref()) {
            return Ok(routing::unauthorized_response());
        }
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
            return Ok(routing::bad_request_response("invalid JSON body"));
        };
        if payload
            .get("items")
            .is_some_and(|items| json_array_exceeds_limit(items, MAX_INCOMING_SHARE_ITEMS))
        {
            return Ok(routing::bad_request_response(
                "items must contain at most 10000 items",
            ));
        }
        let string_field = |field: &str| {
            payload
                .get(field)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_owned()
        };
        let share_grant_id = string_field("shareGrantId");
        let collection_id = string_field("collectionId");
        let recipient_user_id = string_field("recipientUserId");
        let owner_endpoint = string_field("ownerEndpoint");
        if share_grant_id.is_empty()
            || collection_id.is_empty()
            || recipient_user_id.is_empty()
            || owner_endpoint.is_empty()
        {
            return Ok(routing::bad_request_response(
                "shareGrantId, collectionId, recipientUserId, and ownerEndpoint are required",
            ));
        }
        let allow_download = payload
            .get("allowDownload")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let allow_stream = payload
            .get("allowStream")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let allow_reshare = payload
            .get("allowReshare")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let permissions = [
            allow_download.then_some("download"),
            allow_stream.then_some("stream"),
            allow_reshare.then_some("reshare"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(",");
        let items = payload
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(MAX_INCOMING_SHARE_ITEMS)
            .collect::<Vec<_>>();
        let record = IncomingShareRecord {
            id: share_grant_id,
            owner_endpoint,
            owner_user_id: string_field("ownerUserId"),
            recipient_user_id,
            collection_id,
            collection_title: string_field("collectionTitle"),
            collection_description: string_field("collectionDescription"),
            collection_type: string_field("collectionType"),
            permissions,
            token: string_field("token"),
            expiry_utc: string_field("expiryUtc"),
            max_bitrate_kbps: payload
                .get("maxBitrateKbps")
                .and_then(serde_json::Value::as_u64),
            max_concurrent_streams: payload
                .get("maxConcurrentStreams")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            items,
            received_at: unix_timestamp(),
        };
        let json = record.json();
        let mut incoming = state.incoming_shares.write().await;
        incoming.upsert(record);
        drop(incoming);
        return Ok(routing::created_response(json.to_string()));
    }
    if route.path == "/api/v0/share-grants/incoming" && method == "GET" {
        if !is_authorized(&state.config, authorization, headers.cookie.as_deref()) {
            return Ok(routing::unauthorized_response());
        }
        let incoming = state.incoming_shares.read().await;
        let records = incoming
            .list()
            .iter()
            .map(IncomingShareRecord::json)
            .collect::<Vec<_>>();
        drop(incoming);
        return Ok(routing::ok_response(
            serde_json::to_string(&records).unwrap_or_else(|_| "[]".to_owned()),
        ));
    }

    let controller_metrics_request =
        method == "GET" && route.path == controller_metrics_path(&state.config);
    if controller_metrics_request {
        if !state.config.controller_metrics_enabled {
            return Ok(controller_swagger_not_found_response());
        }
        if let Some(response) = controller_metrics_auth_failure(&state.config, authorization) {
            return Ok(response);
        }
        normalized_path = "/api/metrics".to_owned();
    } else {
        if request_uses_revoked_jwt(state, authorization).await {
            tracing::complete_request_span(401);
            return Ok(routing::unauthorized_response());
        }

        if let Err(err) =
            routing::check_route_auth(&state.config, method, route.path, authorization, &headers)
        {
            let status = if err == "unauthorized" { 401 } else { 403 };
            tracing::complete_request_span(status);
            return Ok(match err {
                "unauthorized" => routing::unauthorized_response(),
                "csrf" => routing::forbidden_response("cross-site mutating request rejected"),
                _ => routing::forbidden_response("insufficient permissions for this route"),
            });
        }
    }

    if state.config.controller_profile == ControllerProfile::Native {
        let feature = state.media_services.read().await.features.clone();
        let feature_disabled = (!feature.collections_sharing
            && (normalized_path.starts_with("/api/collections")
                || normalized_path.starts_with("/api/share-grants")
                || normalized_path.starts_with("/api/sharegroups")
                || normalized_path.starts_with("/api/sharing")))
            || (!feature.streaming
                && (normalized_path.starts_with("/api/streams")
                    || normalized_path.starts_with("/api/peer-streams")
                    || normalized_path.starts_with("/api/mesh-streams")
                    || normalized_path.starts_with("/api/listening-party/radio")))
            || (!feature.streaming_relay_fallback
                && normalized_path.starts_with("/api/relay/streams"))
            || (!feature.identity_friends
                && (normalized_path.starts_with("/api/profile")
                    || normalized_path.starts_with("/api/contacts")
                    || normalized_path.starts_with("/api/identity")))
            || (!feature.solid && normalized_path.starts_with("/api/solid"))
            || (!feature.song_id && normalized_path.starts_with("/api/songid"))
            || (!feature.mesh && normalized_path.starts_with("/api/mesh"))
            || (!feature.dht
                && (normalized_path.starts_with("/api/dht")
                    || normalized_path.starts_with("/api/overlay")))
            || (!feature.pods
                && (normalized_path.starts_with("/api/pods")
                    || normalized_path.starts_with("/api/podcore")))
            || (!feature.social_federation
                && (normalized_path.starts_with("/api/federation")
                    || normalized_path.starts_with("/api/activitypub")
                    || normalized_path.starts_with("/api/taste-recommendations")
                    || normalized_path.starts_with("/actors/")
                    || normalized_path.starts_with("/.well-known/webfinger")))
            || (!feature.virtual_soulfind
                && (normalized_path.starts_with("/api/virtualsoulfind")
                    || normalized_path.starts_with("/api/bridge")))
            || (!feature.multi_source_downloads
                && (normalized_path.starts_with("/api/multisource")
                    || normalized_path.starts_with("/api/swarm")));
        if feature_disabled {
            return Ok(controller_swagger_not_found_response());
        }
    }

    if route.path == "/api/v0/application/dump"
        && ((state.config.controller_profile == ControllerProfile::Legacy && method == "POST")
            || (state.config.controller_profile == ControllerProfile::Native && method == "GET"))
    {
        return Ok(routing::method_not_allowed_response());
    }

    if (normalized_path.starts_with("/actors/") || normalized_path == "/.well-known/webfinger")
        && !social_federation_is_active(&state.config)
    {
        return Ok(controller_swagger_not_found_response());
    }

    if normalized_path.starts_with("/api/mesh")
        || normalized_path.starts_with("/api/dht")
        || normalized_path.starts_with("/api/overlay")
        || normalized_path.starts_with("/api/pod")
    {
        let advanced = state.advanced_networking.read().await;
        let mesh_nat_detect = normalized_path == "/api/mesh/nat/detect";
        let disabled = (normalized_path.starts_with("/api/mesh")
            && !mesh_nat_detect
            && (!advanced.mesh.enabled || !advanced.mesh.enable_overlay))
            || (mesh_nat_detect && (!advanced.mesh.enabled || !advanced.mesh.enable_stun))
            || (normalized_path.starts_with("/api/dht")
                && normalized_path != "/api/dht/status"
                && (!advanced.dht.enabled || !advanced.mesh.enable_dht))
            || (normalized_path.starts_with("/api/overlay/data")
                && (!advanced.mesh.enable_overlay || !advanced.overlay_data.enable))
            || (normalized_path.starts_with("/api/overlay")
                && (!advanced.mesh.enable_overlay || !advanced.overlay.enable));
        if disabled {
            return Ok(controller_swagger_not_found_response());
        }
        let remote_limit = advanced.mesh.effective_max_remote_payload_size();
        let network_limit = if advanced.security.enabled && advanced.security.network_guard.enabled
        {
            advanced.security.network_guard.max_message_size
        } else {
            usize::MAX
        };
        if body.len() > remote_limit.min(network_limit) {
            return Ok(HttpResponse {
                status: "413 Payload Too Large",
                content_type: "application/json; charset=utf-8",
                body: r#"{"error":"remote payload exceeds configured security limit"}"#.to_owned(),
            });
        }
    }

    if state.config.controller_profile == ControllerProfile::Native
        && method == "POST"
        && normalized_path == "/api/application/dump"
    {
        if !*state.diagnostics_allow_memory_dump.read().await {
            return Ok(HttpResponse {
                status: "404 Not Found",
                content_type: "",
                body: String::new(),
            });
        }
        let is_loopback = headers
            .remote_addr
            .is_some_and(|address| match address.ip() {
                std::net::IpAddr::V4(address) => address.is_loopback(),
                std::net::IpAddr::V6(address) => address
                    .to_ipv4_mapped()
                    .map_or_else(|| address.is_loopback(), |address| address.is_loopback()),
            });
        if !*state.diagnostics_allow_remote_dump.read().await && !is_loopback {
            return Ok(HttpResponse {
                status: "403 Forbidden",
                content_type: "",
                body: String::new(),
            });
        }
        return Ok(HttpResponse {
            status: "200 OK",
            content_type: "application/octet-stream",
            body: String::new(),
        });
    }

    if unversioned_mutation_requires_api_version(method, route.path) {
        return Ok(HttpResponse {
            status: "400 Bad Request",
            content_type: "application/problem+json",
            body: serde_json::json!({
                "type": "https://docs.api-versioning.org/problems#unspecified",
                "title": "Unspecified API version",
                "status": 400,
                "detail": "An API version is required, but was not specified.",
                "code": "ApiVersionUnspecified",
            })
            .to_string(),
        });
    }

    if request_is_versioned_v0 {
        if let Some(response) = versioned_pods_blank_segment_response(method, route.path) {
            return Ok(response);
        }
        if let Some(response) = versioned_wishlist_invalid_id_response(method, route.path) {
            return Ok(response);
        }
    }

    if let Some(response) = virtual_soulfind_legacy_blank_id_response(&normalized_path) {
        return Ok(response);
    }

    if request_is_versioned_v0 {
        if let Some(response) = versioned_rooms_blank_segment_response(method, &normalized_path) {
            return Ok(response);
        }
    } else if let Some(response) =
        unversioned_rooms_compatibility_blank_id_response(method, &normalized_path)
    {
        return Ok(response);
    }

    if state.config.controller_profile == ControllerProfile::Native {
        if let Some(response) = bridge_transfer_blank_segment_response(method, route.path) {
            return Ok(response);
        }
    }

    if let Some(response) =
        controller_native_virtual_soulfind_read_failure_response(state, method, &normalized_path)
            .await
    {
        return Ok(response);
    }

    if let Some(response) =
        controller_native_wishlist_read_failure_response(state, method, route.path).await
    {
        return Ok(response);
    }

    if method == "GET" {
        if let Some(response) =
            controller_native_hashdb_read_failure_response(state, route.path).await
        {
            return Ok(response);
        }
        if let Some(response) =
            controller_native_backfill_candidates_read_failure_response(state, method, route.path)
                .await
        {
            return Ok(response);
        }
    }
    if method == "POST" {
        if let Some(response) =
            controller_native_hashdb_write_failure_response(state, route.path).await
        {
            return Ok(response);
        }
    }

    if let Some(response) =
        controller_native_transfer_storage_failure_response(state, method, route.path).await
    {
        return Ok(response);
    }
    if let Some(response) = controller_native_transfer_input_validation_response(
        state,
        method,
        route.path,
        route.query,
        body,
    ) {
        return Ok(response);
    }
    if let Some(response) =
        controller_native_transfer_auto_replace_status_response(state, method, route.path).await
    {
        return Ok(response);
    }
    if let Some(response) =
        controller_native_autoreplace_mutation_response(state, method, route.path).await
    {
        return Ok(response);
    }

    if method == "DELETE" && route.path == "/api/v0/session" {
        if let Some(token) = utils::bearer_authorization_token(authorization) {
            let now = unix_timestamp();
            if let Some(claims) = utils::verify_admin_jwt(&state.config, token, now) {
                let revoke_result = state
                    .revoked_jwts
                    .write()
                    .await
                    .revoke(claims.jti, claims.exp, now);
                if let Err(error) = revoke_result {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Error,
                        "security",
                        format!("JWT revocation persistence failed: {error}"),
                    )
                    .await;
                    return Ok(routing::service_unavailable_response(
                        "session revocation persistence failed",
                    ));
                }
            }
        }
        return Ok(routing::no_content_response());
    }

    if method == "GET" {
        if let Some(response) = versioned_get_failure_contract(route.path, route.query, state).await
        {
            return Ok(response);
        }
    }

    if state.config.controller_profile == ControllerProfile::Native {
        if let Some(response) =
            controller_native_search_query_validation(method, route.path, route.query)
        {
            return Ok(response);
        }
        if let Some(response) =
            controller_native_search_storage_failure_response(state, method, route.path).await
        {
            return Ok(response);
        }
    }

    if method == "DELETE" && state.config.controller_profile == ControllerProfile::Legacy {
        for prefix in ["/api/v0/transfers/downloads/", "/api/v0/transfers/uploads/"] {
            if let Some(value) = route.path.strip_prefix(prefix) {
                let segments = value.split('/').collect::<Vec<_>>();
                if segments.len() == 2
                    && segments[0] != "all"
                    && segments[1].parse::<u64>().is_err()
                {
                    return Ok(routing::bad_request_response("The request is invalid"));
                }
            }
        }
    }

    let relay_settings = state.advanced_networking.read().await.relay.clone();
    let relay_route = route.path.starts_with("/api/v0/relay/");
    if relay_route
        && relay_versioned_route_known(method, route.path)
        && !relay_versioned_route_allowed(&relay_settings, method, route.path)
    {
        return Ok(routing::forbidden_response(
            "feature is disabled by configuration",
        ));
    }
    if route.path.starts_with("/api/v0/")
        && matches!(
            (method, route.path),
            ("POST", "/api/v0/soulseek/mesh-rendezvous/interest")
                | ("DELETE", "/api/v0/soulseek/mesh-rendezvous/interest")
        )
    {
        return Ok(routing::forbidden_response(
            "feature is disabled by configuration",
        ));
    }

    if let Some(response) = versioned_relay_request(method, route.path, body, &headers, state).await
    {
        return Ok(response);
    }

    if method == "POST" && route.path == "/api/v0/podcore/signing/sign" {
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|payload| payload.get("message").cloned())
            .unwrap_or(serde_json::Value::Null);
        let sender_peer_id = message
            .get("senderPeerId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if !sender_peer_id.is_empty()
            && pod_request_peer_id(state).await.as_deref() != Some(sender_peer_id)
        {
            return Ok(routing::forbidden_response(
                "authenticated Pod membership is required",
            ));
        }
    }

    if method == "POST"
        && route.path.starts_with("/api/v0/podcore/membership/")
        && route.path.ends_with("/members")
    {
        let member_peer_id = extract_json_string_field(body, "peerId").unwrap_or_default();
        if pod_request_peer_id(state).await.as_deref() != Some(member_peer_id.as_str()) {
            return Ok(routing::forbidden_response(
                "authenticated Pod membership is required",
            ));
        }
    }

    // Keep the descriptor-unpublish path out of the large compatibility
    // mutation future.  The latter owns hundreds of unrelated branches and
    // can exceed the default Tokio worker stack before this small handler is
    // polled on a live HTTP request.
    if method == "DELETE" && normalized_path.starts_with("/api/mediacore/publish/descriptor/") {
        if let Some(response) = Box::pin(mediacore_mutation_response(
            method,
            &normalized_path,
            body,
            state,
        ))
        .await
        {
            return Ok(response);
        }
    }

    if method == "DELETE" && normalized_path.starts_with("/api/podcore/") {
        if let Some(response) = Box::pin(podcore_mutation_response(
            method,
            &normalized_path,
            route.query,
            body,
            state,
            route.path.starts_with("/api/v0/"),
        ))
        .await
        {
            return Ok(response);
        }
    }

    if method == "GET" && route.path == "/swagger/" {
        return Ok(controller_swagger_index_response(&state.config));
    }

    if state.config.controller_headless
        && matches!(method, "GET" | "HEAD")
        && (matches!(route.path, "/" | "/dashboard") || is_spa_navigation_path(route.path))
    {
        return Ok(controller_swagger_not_found_response());
    }

    if method == "GET" && audio_blank_recording_id_path(&normalized_path) {
        return Ok(routing::bad_request_response("RecordingId is required."));
    }

    if let Some(response) = versioned_conversation_mutation_validation_response(method, route.path)
    {
        return Ok(response);
    }

    if let Some(state_arc) = state_arc.as_ref() {
        if method == "PUT" && route.path == "/api/v0/shares" {
            return Ok(versioned_share_rescan_response(state, state_arc.clone()));
        }
    }

    if let Some(response) = route_dispatch_fast_read(method, &normalized_path, &route, state) {
        return complete_route_dispatch(response);
    }

    let extended_mutation = extended_controller_mutation_route(method, &normalized_path);
    let context = RouteDispatchContext {
        method,
        normalized_path: &normalized_path,
        authorization,
        body,
        state,
        route: &route,
        headers,
        state_arc: state_arc.clone(),
        extended_mutation,
        request_is_versioned_v0,
    };
    let mut response = route_dispatch_group_0(&context).await;
    if route_is_unhandled(&response) {
        response = route_dispatch_group_1(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_2(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_3(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_4(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_5(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_6(&context).await;
    }
    if route_is_unhandled(&response) {
        response = route_dispatch_group_7(&context).await;
    }
    if route_is_unhandled(&response) {
        response = Ok(routing::not_found_response());
    }
    complete_route_dispatch(response)
}

#[derive(Clone)]
struct RouteDispatchContext<'request, 'state> {
    method: &'request str,
    normalized_path: &'request str,
    authorization: Option<&'request str>,
    body: &'request str,
    state: &'state AppState,
    route: &'state routing::ParsedRoute<'request>,
    headers: &'state RequestSecurityHeaders,
    state_arc: Option<Arc<AppState>>,
    extended_mutation: bool,
    request_is_versioned_v0: bool,
}
