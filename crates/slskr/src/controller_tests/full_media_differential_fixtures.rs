//! Controller full media differential fixtures ownership.

use super::*;

#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_primary_stream_ticket_lifecycle_impl() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let content_id = "Virtual/Test.flac";
    let encoded_content_id = crate::url_encode(content_id);
    let normal_route = format!("/api/v0/streams/{encoded_content_id}/ticket");
    let stream_route = format!("/api/v0/streams/{encoded_content_id}");
    let normal_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "differential-route-token");
    let normal_authorization = Some("Bearer differential-route-token");
    let (state, _receiver) = test_state_with_env(normal_env.clone());

    let normal_ticket_response =
        crate::route_http_request("POST", &normal_route, normal_authorization, "", &state)
            .await
            .expect("normal stream ticket response");
    let normal_ticket_json =
        serde_json::from_str::<serde_json::Value>(&normal_ticket_response.body)
            .unwrap_or_else(|_| serde_json::json!({}));
    let normal_ticket = normal_ticket_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "nominal-status-headers-body",
        normal_ticket_response.status == "200 OK"
            && normal_ticket_response.content_type == "application/json"
            && normal_ticket_json["expiresInSeconds"] == 120
            && !normal_ticket.is_empty()
    );

    let normal_stream = crate::route_http_request(
        "GET",
        &format!(
            "{stream_route}?ticket={}",
            crate::url_encode(&normal_ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("normal stream ticket readback");
    let normal_stream_json = serde_json::from_str::<serde_json::Value>(&normal_stream.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let normal_stored = state
        .stream_tickets
        .write()
        .await
        .get(&normal_ticket)
        .is_some_and(|ticket| {
            ticket.family == "share"
                && ticket.content_id == content_id
                && ticket.filename == content_id
        });
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "mutation-side-effects-and-readback",
        normal_stored
            && normal_stream.status == "200 OK"
            && normal_stream_json["status"] == "available"
            && normal_stream_json["ticket"] == "accepted"
    );

    let malformed_normal = crate::route_http_request(
        "POST",
        "/api/v0/streams/%20/ticket",
        normal_authorization,
        "",
        &state,
    )
    .await
    .expect("normal stream ticket malformed content id");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "malformed-path-query-or-body",
        malformed_normal.status == "400 Bad Request"
            && malformed_normal.body.contains("ContentId is required")
    );

    let missing_normal = crate::route_http_request(
        "POST",
        "/api/v0/streams/content%3Amusic%3Arecording%3Amissing/ticket",
        normal_authorization,
        "",
        &state,
    )
    .await
    .expect("normal stream ticket missing content");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "missing-empty-or-conflict-state",
        missing_normal.status == "404 Not Found"
    );

    let (capacity_state, _receiver) = test_state_with_env(normal_env.clone());
    {
        let mut tickets = capacity_state.stream_tickets.write().await;
        while tickets.records.len() < crate::MAX_PREVIEW_STREAM_TICKETS {
            let index = tickets.records.len();
            assert!(tickets
                .issue(
                    "share",
                    "local-share",
                    format!("stream-capacity-{index}"),
                    "Track.flac".to_owned(),
                    None,
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity_normal = crate::route_http_request(
        "POST",
        &normal_route,
        normal_authorization,
        "",
        &capacity_state,
    )
    .await
    .expect("normal stream ticket capacity response");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "runtime-failure-and-timeout",
        capacity_normal.status == "503 Service Unavailable"
            && capacity_normal
                .body
                .contains("stream ticket capacity is full")
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("normal stream runtime-failure database");
    let (failure_state, _failure_receiver) = test_state_with_env_parts(
        normal_env.clone(),
        crate::SearchStore::new(),
        Some(failure_db.clone()),
    );
    let failure_ticket = crate::route_http_request(
        "POST",
        &normal_route,
        normal_authorization,
        "",
        &failure_state,
    )
    .await
    .expect("create normal runtime-failure ticket");
    let failure_ticket_json = serde_json::from_str::<serde_json::Value>(&failure_ticket.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    failure_db.close_for_test().await;
    let failure_stream = crate::route_http_request(
        "GET",
        &format!(
            "{stream_route}?ticket={}",
            crate::url_encode(failure_ticket_json["ticket"].as_str().unwrap_or_default())
        ),
        None,
        "",
        &failure_state,
    )
    .await
    .expect("normal stream read with closed unrelated database");
    record!(
        "GET",
        "/api/v0/streams/{contentId}",
        "runtime-failure-and-timeout",
        failure_stream.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&failure_stream.body)
                .map(|value| value["status"] == "available")
                .unwrap_or(false)
    );

    let (restarted_state, _restarted_receiver) = test_state_with_env(normal_env.clone());
    let reset_get = crate::route_http_request(
        "GET",
        &format!(
            "{stream_route}?ticket={}",
            crate::url_encode(&normal_ticket)
        ),
        None,
        "",
        &restarted_state,
    )
    .await
    .expect("normal stream ticket after restart");
    let reset_create = crate::route_http_request(
        "POST",
        &normal_route,
        normal_authorization,
        "",
        &restarted_state,
    )
    .await
    .expect("normal stream ticket create after restart");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "restart-persistence-or-reset",
        reset_get.status == "401 Unauthorized" && reset_create.status == "200 OK"
    );

    let (concurrent_state, _concurrent_receiver) = test_state_with_env(normal_env.clone());
    let concurrent_normal = tokio::join!(
        crate::route_http_request(
            "POST",
            &normal_route,
            normal_authorization,
            "",
            &concurrent_state,
        ),
        crate::route_http_request(
            "POST",
            &normal_route,
            normal_authorization,
            "",
            &concurrent_state,
        ),
    );
    let concurrent_normal_pass = match concurrent_normal {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
                && concurrent_state.stream_tickets.read().await.records.len() == 2
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/streams/{contentId}/ticket",
        "concurrency-and-idempotency",
        concurrent_normal_pass
    );

    let api_env = MapEnv::default()
        .with("SLSKR_CONTROLLER_PROFILE", target)
        .with("SLSKR_AUTH_DISABLED", "false")
        .with("SLSKR_API_TOKEN", "differential-route-token");
    let (share_state, _share_receiver) = test_state_with_env(api_env.clone());
    let api_authorization = Some("Bearer differential-route-token");
    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        api_authorization,
        r#"{"name":"Primary stream residual"}"#,
        &share_state,
    )
    .await
    .expect("create stream share collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let share_content_id = "content/stream-residual";
    crate::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        api_authorization,
        &format!(r#"{{"content_id":"{share_content_id}","title":"Residual.flac","kind":"Audio"}}"#),
        &share_state,
    )
    .await
    .expect("create stream share item");
    let grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        api_authorization,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &share_state,
    )
    .await
    .expect("create stream share grant");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap_or_default()["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let issued = crate::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        api_authorization,
        r#"{"expiresInSeconds":600}"#,
        &share_state,
    )
    .await
    .expect("issue stream share token");
    let share_token = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap_or_default()
        ["token"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let share_headers = crate::RequestSecurityHeaders {
        x_share_token: Some(share_token.clone()),
        ..Default::default()
    };
    let share_encoded_content_id = crate::url_encode(share_content_id);
    let share_route = format!("/api/v0/streams/{share_encoded_content_id}/share-ticket");

    let share_ticket_response = crate::route_http_request_with_headers(
        "POST",
        &share_route,
        None,
        "",
        &share_state,
        share_headers.clone(),
    )
    .await
    .expect("share stream ticket response");
    let share_ticket_json = serde_json::from_str::<serde_json::Value>(&share_ticket_response.body)
        .unwrap_or_else(|_| serde_json::json!({}));
    let share_ticket = share_ticket_json["ticket"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "nominal-status-headers-body",
        share_ticket_response.status == "200 OK"
            && share_ticket_response.content_type == "application/json"
            && share_ticket_json["expiresInSeconds"] == 120
            && !share_ticket.is_empty()
            && !share_ticket_response.body.contains(&share_token)
    );
    let share_stored = share_state
        .stream_tickets
        .write()
        .await
        .get(&share_ticket)
        .is_some_and(|ticket| ticket.family == "share" && ticket.content_id == share_content_id);
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "mutation-side-effects-and-readback",
        share_stored
    );

    let malformed_share = crate::route_http_request_with_headers(
        "POST",
        &format!("{share_route}/extra"),
        api_authorization,
        "",
        &share_state,
        share_headers.clone(),
    )
    .await
    .expect("malformed share stream ticket path");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "malformed-path-query-or-body",
        malformed_share.status == "404 Not Found"
    );

    let missing_share = crate::route_http_request("POST", &share_route, None, "", &share_state)
        .await
        .expect("missing share stream token");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "missing-empty-or-conflict-state",
        missing_share.status == "401 Unauthorized"
    );

    let concurrent_share = tokio::join!(
        crate::route_http_request_with_headers(
            "POST",
            &share_route,
            None,
            "",
            &share_state,
            share_headers.clone(),
        ),
        crate::route_http_request_with_headers(
            "POST",
            &share_route,
            None,
            "",
            &share_state,
            share_headers.clone(),
        ),
    );
    let concurrent_share_pass = match concurrent_share {
        (Ok(left), Ok(right)) => {
            let left_json = serde_json::from_str::<serde_json::Value>(&left.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            let right_json = serde_json::from_str::<serde_json::Value>(&right.body)
                .unwrap_or_else(|_| serde_json::json!({}));
            left.status == "200 OK"
                && right.status == "200 OK"
                && left_json["ticket"].as_str().is_some_and(|ticket| {
                    !ticket.is_empty()
                        && ticket != right_json["ticket"].as_str().unwrap_or_default()
                })
        }
        _ => false,
    };
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "concurrency-and-idempotency",
        concurrent_share_pass
    );

    {
        let mut tickets = share_state.stream_tickets.write().await;
        while tickets.records.len() < crate::MAX_PREVIEW_STREAM_TICKETS {
            let index = tickets.records.len();
            assert!(tickets
                .issue(
                    "share",
                    "share:capacity",
                    format!("share-capacity-{index}"),
                    "Track.flac".to_owned(),
                    Some("friend".to_owned()),
                    0,
                    "audio/flac".to_owned(),
                    120,
                )
                .is_some());
        }
    }
    let capacity_share = crate::route_http_request_with_headers(
        "POST",
        &share_route,
        None,
        "",
        &share_state,
        share_headers.clone(),
    )
    .await
    .expect("share stream ticket capacity response");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "runtime-failure-and-timeout",
        capacity_share.status == "503 Service Unavailable"
            && capacity_share
                .body
                .contains("share stream ticket capacity is full")
    );

    let (restarted_share_state, _restarted_share_receiver) = test_state_with_env(api_env);
    let reset_share = crate::route_http_request_with_headers(
        "POST",
        &share_route,
        None,
        "",
        &restarted_share_state,
        share_headers,
    )
    .await
    .expect("share stream ticket after restart");
    record!(
        "POST",
        "/api/v0/streams/{contentId}/share-ticket",
        "restart-persistence-or-reset",
        reset_share.status == "401 Unauthorized"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("primary_stream_ticket_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
    assert!(
        mismatches.is_empty(),
        "{} primary stream ticket mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
