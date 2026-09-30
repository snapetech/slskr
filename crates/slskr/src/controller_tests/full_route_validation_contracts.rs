//! Controller full route validation contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn parse_route_reads_method_and_path() {
    assert_eq!(
        parse_route("POST /api/session/connect HTTP/1.1\r\nhost: localhost\r\n\r\n"),
        ("POST", "/api/session/connect")
    );
    assert_eq!(
        split_request_target("/api/v0/shares/catalog?q=test"),
        ("/api/v0/shares/catalog", Some("q=test"))
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn read_only_api_routes_return_contract_shapes() {
    let (state, _receiver) = test_state();

    let cases = [
        ("/api/v0/health", "\"status\":\"ok\""),
        ("/api/v0/version", "\"name\":\"slskr\""),
        (
            "/api/v0/capabilities",
            "\"version\":\"slskdn/1.0.0+dht+mesh+swarm\"",
        ),
        ("/api/v0/config", "\"credentials_configured\":true"),
        ("/api/v0/stats", "\"session\":"),
        ("/api/v0/telemetry", "\"health\":"),
        ("/api/v0/events", "[]"),
        ("/api/v0/events/records", "\"entries\":"),
        ("/api/v0/logs", "[]"),
        ("/api/v0/session/enabled", "false"),
        ("/api/v0/listeners", "\"regular_accepts\":0"),
        ("/api/v0/users", "\"count\":0"),
        ("/api/v0/rooms", "\"count\":0"),
        ("/api/v0/shares", "\"files\":1"),
        ("/api/v0/shares/catalog", "\"total_bytes\":42"),
        ("/api/v0/searches", "[]"),
        ("/api/v0/searches/records", "\"count\":0"),
        ("/api/v0/transfers", "[]"),
        ("/api/v0/transfers/stats", "\"total\":0"),
    ];

    for (path, expected_body) in cases {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("route response");
        assert_eq!(response.status, "200 OK", "{path}");
        assert_eq!(
            response.content_type,
            if path == "/api/v0/shares" {
                "application/json; charset=utf-8"
            } else {
                "application/json"
            },
            "{path}"
        );
        assert!(
            response.body.contains(expected_body),
            "{path}: {}",
            response.body
        );
        assert!(
            !response.body.contains("test-password")
                && !response.body.contains("api-token")
                && !response.body.contains("client-secret"),
            "{path}: {}",
            response.body
        );
    }
    let session_check = crate::route_http_request("GET", "/api/v0/session", None, "", &state)
        .await
        .expect("versioned session check");
    assert_eq!(session_check.status, "200 OK");
    assert_eq!(session_check.content_type, "application/json");
    assert!(
        session_check.body.contains("\"state\":"),
        "{}",
        session_check.body
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn unknown_api_route_returns_json_404() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("GET", "/api/v0/missing", None, "", &state)
        .await
        .expect("route response");

    assert_eq!(response.status, "404 Not Found");
    assert_eq!(response.content_type, "application/json");
    assert_eq!(response.body, "{\"error\":\"route not found\"}");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn json_escape_handles_control_characters() {
    assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn query_params_decode_percent_encoding() {
    assert_eq!(
        percent_decode("Virtual%2FTest+File.flac"),
        "Virtual/Test File.flac"
    );
    assert_eq!(
        query_params("q=test+file&extension=flac"),
        vec![
            ("q".to_owned(), "test file".to_owned()),
            ("extension".to_owned(), "flac".to_owned()),
        ]
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn list_limits_are_bounded_by_default() {
    let default_filter = crate::RecordListFilter::from_query(None);
    assert_eq!(default_filter.limit, Some(crate::DEFAULT_LIST_LIMIT));

    let huge_filter = crate::RecordListFilter::from_query(Some("limit=999999"));
    assert_eq!(huge_filter.limit, Some(crate::DEFAULT_LIST_LIMIT));

    let zero_filter = crate::CatalogFilter::from_query(Some("limit=0"));
    assert_eq!(zero_filter.limit, Some(crate::DEFAULT_LIST_LIMIT));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn extracts_simple_json_string_fields() {
    assert_eq!(
        crate::extract_json_string_field(r#"{"query":"artist \"song\""}"#, "query"),
        Some("artist \"song\"".to_owned())
    );
    assert_eq!(
        crate::extract_json_string_field(
            r#"{"a":"\"query\":\"hijacked\"","query":"real"}"#,
            "query"
        ),
        Some("real".to_owned())
    );
    assert_eq!(
        crate::extract_json_string_array_field(
            r#"{"capabilities":["shares","telemetry","quoted \" item"]}"#,
            "capabilities"
        ),
        Some(vec![
            "shares".to_owned(),
            "telemetry".to_owned(),
            "quoted \" item".to_owned()
        ])
    );
    assert_eq!(
        crate::extract_json_string_field(r#"{"other":"value"}"#, "query"),
        None
    );
    assert_eq!(
        crate::extract_json_u32_field(r#"{"token":42}"#, "token"),
        Some(42)
    );
    assert_eq!(
        crate::extract_json_bool_field(r#"{"slot_free":false}"#, "slot_free"),
        Some(false)
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn null_options_overlay_matches_the_selected_frozen_target() {
    for (target, expected, expected_content_type, expected_body) in [
        ("slskd", "204 No Content", "", ""),
        (
            "slskdn",
            "400 Bad Request",
            "application/json; charset=utf-8",
            r#"{"title":"One or more validation errors occurred.","status":400,"detail":"The request is invalid.","errors":{}}"#,
        ),
    ] {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_REMOTE_CONFIGURATION", "true")
                .with("SLSKR_CONTROLLER_PROFILE", target),
        );
        assert!(state.config.remote_configuration, "{target}");
        for body in ["null", "[]"] {
            let response =
                crate::route_http_request("PATCH", "/api/v0/options", None, body, &state)
                    .await
                    .expect("non-object options overlay response");
            assert_eq!(response.status, expected, "{target}: {}", response.body);
            assert_eq!(
                response.content_type, expected_content_type,
                "{target} {body}"
            );
            assert_eq!(response.body, expected_body, "{target} {body}");
        }
    }
}
