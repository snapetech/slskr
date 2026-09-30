//! Controller full bridge pod fixtures ownership.

use super::*;

pub(super) async fn virtual_soulfind_bridge_timeout_and_reconnect(
    message_type: i32,
    payload: Vec<u8>,
) -> bool {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge timeout listener");
    let address = listener.local_addr().expect("bridge timeout address");
    let server_payload = payload.clone();
    let server = tokio::spawn(async move {
        let (mut first, _) = listener.accept().await.expect("accept first bridge client");
        let request = tokio::time::timeout(
            Duration::from_secs(1),
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut first),
        )
        .await;
        if !matches!(
            request,
            Ok(Ok(Some((actual_type, actual_payload))))
                if actual_type == message_type && actual_payload == vec![0xA1]
        ) {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(75)).await;
        drop(first);

        let (mut second, _) = listener
            .accept()
            .await
            .expect("accept reconnected bridge client");
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut second,
            message_type,
            &server_payload,
        )
        .await
        .is_ok()
    });

    let mut first_client = match tokio::net::TcpStream::connect(address).await {
        Ok(client) => client,
        Err(_) => {
            server.abort();
            let _ = server.await;
            return false;
        }
    };
    if crate::soulfind_bridge_runtime::bridge_write_frame(&mut first_client, message_type, &[0xA1])
        .await
        .is_err()
    {
        server.abort();
        let _ = server.await;
        return false;
    }
    let timed_out = tokio::time::timeout(
        Duration::from_millis(25),
        crate::soulfind_bridge_runtime::bridge_read_frame(&mut first_client),
    )
    .await
    .is_err();
    drop(first_client);

    let mut second_client = match tokio::net::TcpStream::connect(address).await {
        Ok(client) => client,
        Err(_) => {
            server.abort();
            let _ = server.await;
            return false;
        }
    };
    let reconnected = match tokio::time::timeout(
        Duration::from_secs(1),
        crate::soulfind_bridge_runtime::bridge_read_frame(&mut second_client),
    )
    .await
    {
        Ok(Ok(Some((actual_type, actual_payload)))) => {
            actual_type == message_type && actual_payload == payload
        }
        _ => false,
    };
    let server_pass = server.await.is_ok_and(|result| result);
    timed_out && reconnected && server_pass
}

pub(super) async fn pod_fixture_with_local_role(
    local_peer: &str,
    creator: &str,
    pod_id: &str,
) -> (Arc<crate::AppState>, mpsc::Receiver<crate::SessionCommand>) {
    let (state, receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", local_peer)
            .with("SLSK_PASSWORD", "test-secret"),
    );
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Authorization Audit",
            }))
            .expect("deserialize pod record fixture"),
            creator.to_owned(),
        )
        .expect("create pod");
    (state, receiver)
}

pub(super) async fn virtual_soulfind_v2_target_negative_ledger() -> Vec<serde_json::Value> {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let expected_body = serde_json::to_string("VirtualSoulfind v2 is disabled")
        .expect("serialize disabled VirtualSoulfind v2 body");
    let routes = [
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/artist-1",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/%20",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/{artistId}/releases",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/artist-1/releases",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/%20/releases",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/missing/releases",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search?query=artist&limit=10",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search?limit=invalid",
            "/api/v1/virtualsoulfind/v2/catalogue/artists/search",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/{releaseId}/tracks",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/release-1/tracks",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/%20/tracks",
            "/api/v1/virtualsoulfind/v2/catalogue/releases/missing/tracks",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/executions/{executionId}",
            "/api/v1/virtualsoulfind/v2/executions/execution-1",
            "/api/v1/virtualsoulfind/v2/executions/%20",
            "/api/v1/virtualsoulfind/v2/executions/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/intents/releases/{intentId}",
            "/api/v1/virtualsoulfind/v2/intents/releases/release-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/releases/%20",
            "/api/v1/virtualsoulfind/v2/intents/releases/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/tracks/%20",
            "/api/v1/virtualsoulfind/v2/intents/tracks/missing",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=10",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=invalid",
            "/api/v1/virtualsoulfind/v2/intents/tracks/pending?limit=0",
            "",
            "",
            "",
        ),
        (
            "GET",
            "/api/v1/virtualsoulfind/v2/stats",
            "/api/v1/virtualsoulfind/v2/stats",
            "/api/v1/virtualsoulfind/v2/stats?unexpected=%7B",
            "/api/v1/virtualsoulfind/v2/stats",
            "",
            "",
            "",
        ),
        (
            "PATCH",
            "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1",
            "/api/v1/virtualsoulfind/v2/intents/tracks/missing",
            r#"{"status":"Planned"}"#,
            "{",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            "/api/v1/virtualsoulfind/v2/intents/releases",
            r#"{"releaseId":"release-1"}"#,
            "{",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            "/api/v1/virtualsoulfind/v2/intents/tracks",
            r#"{"domain":"Music","trackId":"track-1"}"#,
            "{",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/intents/tracks/{intentId}/process",
            "/api/v1/virtualsoulfind/v2/intents/tracks/track-intent-1/process",
            "/api/v1/virtualsoulfind/v2/intents/tracks/%20/process",
            "/api/v1/virtualsoulfind/v2/intents/tracks/missing/process",
            "",
            "",
            "",
        ),
        (
            "POST",
            "/api/v1/virtualsoulfind/v2/plans",
            "/api/v1/virtualsoulfind/v2/plans",
            "/api/v1/virtualsoulfind/v2/plans",
            "/api/v1/virtualsoulfind/v2/plans",
            r#"{"domain":"Music","trackId":"track-1"}"#,
            "{",
            "",
        ),
    ];

    let mut ledger = Vec::new();
    for (
        method,
        route,
        nominal_path,
        malformed_path,
        missing_path,
        valid_body,
        malformed_body,
        missing_body,
    ) in routes
    {
        let cases = if method == "GET" {
            vec![
                ("nominal-status-headers-body", nominal_path, valid_body),
                (
                    "malformed-path-query-or-body",
                    malformed_path,
                    malformed_body,
                ),
                (
                    "missing-empty-or-conflict-state",
                    missing_path,
                    missing_body,
                ),
                ("runtime-failure-and-timeout", nominal_path, valid_body),
                ("populated-dynamic-state", nominal_path, valid_body),
            ]
        } else {
            vec![
                ("nominal-status-headers-body", nominal_path, valid_body),
                (
                    "malformed-path-query-or-body",
                    malformed_path,
                    malformed_body,
                ),
                (
                    "missing-empty-or-conflict-state",
                    missing_path,
                    missing_body,
                ),
                ("runtime-failure-and-timeout", nominal_path, valid_body),
                (
                    "mutation-side-effects-and-readback",
                    nominal_path,
                    valid_body,
                ),
                ("restart-persistence-or-reset", nominal_path, valid_body),
                ("concurrency-and-idempotency", nominal_path, valid_body),
            ]
        };

        for (case, path, request_body) in cases {
            let pass = if case == "concurrency-and-idempotency" {
                let (first, second) = tokio::join!(
                    crate::route_http_request(method, path, None, request_body, &state),
                    crate::route_http_request(method, path, None, request_body, &state),
                );
                [first, second].into_iter().all(|response| {
                    response.is_ok_and(|response| {
                        response.status == "503 Service Unavailable"
                            && response.body == expected_body
                    })
                })
            } else {
                let response = crate::route_http_request(method, path, None, request_body, &state)
                    .await
                    .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
                response.status == "503 Service Unavailable" && response.body == expected_body
            };
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": method,
                "route": route,
                "case": case,
                "pass": pass,
            }));
            assert!(pass, "{method} {path} did not match disabled v2 contract");
        }
    }
    ledger
}
