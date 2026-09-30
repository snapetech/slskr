//! Controller full federation differential fixtures ownership.

use super::*;

pub(super) async fn controller_api_differential_activitypub_open_cases_impl() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!("{target} {} {} [{}]", $method, $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let fixture = ActivityPubSignatureFixture::spawn().await;
    let runtime_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("ActivityPub runtime database");
    let (runtime_state, _runtime_receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", target)
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "social.example")
            .with("FEDERATION_BASE_URL", "https://social.example/")
            .with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(runtime_db.clone()),
    );
    runtime_db.close_for_test().await;

    for (path, route) in [
        ("/actors/music/extra", "/actors/{actorName}"),
        (
            "/actors/music/followers/extra",
            "/actors/{actorName}/followers",
        ),
        (
            "/actors/music/following/extra",
            "/actors/{actorName}/following",
        ),
        ("/actors/music/inbox/extra", "/actors/{actorName}/inbox"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("malformed ActivityPub path {path}: {error}"));
        record!(
            "GET",
            route,
            "malformed-path-query-or-body",
            response.status == "404 Not Found"
        );
    }

    for (path, route) in [
        ("/actors/music", "/actors/{actorName}"),
        ("/actors/music/followers", "/actors/{actorName}/followers"),
        ("/actors/music/following", "/actors/{actorName}/following"),
        ("/actors/music/inbox", "/actors/{actorName}/inbox"),
        ("/actors/music/outbox", "/actors/{actorName}/outbox"),
    ] {
        let response = crate::route_http_request("GET", path, None, "", &runtime_state)
            .await
            .unwrap_or_else(|error| panic!("runtime ActivityPub path {path}: {error}"));
        record!(
            "GET",
            route,
            "runtime-failure-and-timeout",
            response.status == "200 OK"
        );
    }

    let created = crate::unix_timestamp();
    let inbox_signature = fixture.sign(created);
    let inbox_headers = fixture.headers(&inbox_signature, created);
    let inbox_runtime = crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &runtime_state,
        inbox_headers,
    )
    .await
    .expect("runtime ActivityPub inbox");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "runtime-failure-and-timeout",
        inbox_runtime.status == "202 Accepted"
    );

    let (restart_state, _restart_receiver) = fixture.state();
    let restart_created = crate::unix_timestamp();
    let restart_signature = fixture.sign(restart_created);
    let inbox_restart = crate::route_http_request_with_headers(
        "POST",
        "/actors/music/inbox",
        None,
        &fixture.body,
        &restart_state,
        fixture.headers(&restart_signature, restart_created),
    )
    .await
    .expect("restart ActivityPub inbox");
    record!(
        "POST",
        "/actors/{actorName}/inbox",
        "restart-persistence-or-reset",
        inbox_restart.status == "202 Accepted"
    );

    let malformed_outbox = crate::route_http_request(
        "POST",
        "/actors/music/outbox/extra",
        None,
        "{}",
        &runtime_state,
    )
    .await
    .expect("malformed ActivityPub outbox");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "malformed-path-query-or-body",
        malformed_outbox.status == "404 Not Found"
    );
    let missing_outbox =
        crate::route_http_request("POST", "/actors/music/outbox", None, "{}", &runtime_state)
            .await
            .expect("missing ActivityPub outbox body");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "missing-empty-or-conflict-state",
        missing_outbox.status == "400 Bad Request"
    );

    let outbox_body = serde_json::json!({
        "id": "https://social.example/activities/open-outbox",
        "type": "Create",
        "actor": "https://social.example/actors/music",
        "object": {"type": "Note", "content": "runtime"},
    })
    .to_string();
    let outbox_runtime = crate::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &outbox_body,
        &runtime_state,
    )
    .await
    .expect("runtime ActivityPub outbox");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "runtime-failure-and-timeout",
        outbox_runtime.status == "200 OK"
    );

    let (outbox_restart_state, _outbox_restart_receiver) = fixture.state();
    let outbox_restart = crate::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        &outbox_body,
        &outbox_restart_state,
    )
    .await
    .expect("restart ActivityPub outbox");
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "restart-persistence-or-reset",
        outbox_restart.status == "200 OK"
    );

    let concurrent_bodies = [
        serde_json::json!({
            "id": "https://social.example/activities/open-concurrent-one",
            "type": "Create",
            "actor": "https://social.example/actors/music",
        })
        .to_string(),
        serde_json::json!({
            "id": "https://social.example/activities/open-concurrent-two",
            "type": "Create",
            "actor": "https://social.example/actors/music",
        })
        .to_string(),
    ];
    let concurrent_outbox =
        futures_util::future::join_all(concurrent_bodies.into_iter().map(|body| {
            let state = Arc::clone(&runtime_state);
            async move {
                crate::route_http_request("POST", "/actors/music/outbox", None, &body, &state).await
            }
        }))
        .await;
    record!(
        "POST",
        "/actors/{actorName}/outbox",
        "concurrency-and-idempotency",
        concurrent_outbox.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create ActivityPub open-case evidence directory");
    fs::write(
        evidence_dir.join("activitypub_open_cases.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize ActivityPub open-case ledger"),
    )
    .expect("write ActivityPub open-case evidence");
    assert!(
        mismatches.is_empty(),
        "{} controller-api ActivityPub mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
