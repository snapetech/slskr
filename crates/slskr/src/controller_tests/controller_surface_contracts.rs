use super::{test_state, test_state_with_env, MapEnv};

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn transfer_reports_require_a_real_direction_not_a_silent_default() {
    let (state, _receiver) = test_state();
    for path in [
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
    ] {
        // Present but nonsense: matches the oracle's real
        // Enum.TryParse<TransferDirection> failure path -- must be a
        // real 400, not silently treated as "no filter" and mixing
        // Upload/Download rows into one report.
        let invalid = crate::route_http_request(
            "GET",
            &format!("{path}?direction=sideways"),
            None,
            "",
            &state,
        )
        .await
        .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(invalid.status, "400 Bad Request", "{path}");
        assert!(
            invalid.body.contains("Invalid direction"),
            "{path}: {}",
            invalid.body
        );

        // Absent, but with an unrelated query param present (so the
        // generic "no query at all" gate doesn't short-circuit it):
        // still a real 400, matching the oracle's required Direction.
        let missing =
            crate::route_http_request("GET", &format!("{path}?limit=5"), None, "", &state)
                .await
                .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(missing.status, "400 Bad Request", "{path}");
        assert!(
            missing.body.contains("Direction is required"),
            "{path}: {}",
            missing.body
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn telemetry_kpis_are_always_json_unlike_the_base_prometheus_route() {
    let (state, _receiver) = test_state();

    let base = crate::route_http_request("GET", "/api/telemetry/prometheus", None, "", &state)
        .await
        .expect("base prometheus route");
    assert!(base.content_type.starts_with("text/plain"), "{base:?}");
    assert!(base.body.contains("slskr_transfers"));

    let kpis = crate::route_http_request("GET", "/api/telemetry/prometheus/kpis", None, "", &state)
        .await
        .expect("kpis route");
    // Matches the oracle's GetKpis: always application/json, a
    // dictionary keyed by metric name -- never the base route's
    // text/plain Prometheus exposition format.
    assert!(
        kpis.content_type.starts_with("application/json"),
        "{kpis:?}"
    );
    let kpis_json = serde_json::from_str::<serde_json::Value>(&kpis.body).unwrap();
    assert_eq!(kpis_json["slskr_transfers"]["type"], "gauge");
    assert_eq!(kpis_json["slskr_transfers"]["samples"][0]["value"], 0.0);
    assert_eq!(kpis_json["slskr_searches"]["type"], "gauge");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn native_versioned_extended_gets_match_empty_state_contracts() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let metadata = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/metadata-processing?limit=0",
        None,
        "",
        &state,
    )
    .await
    .expect("metadata processing status");
    assert_eq!(metadata.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&metadata.body).unwrap(),
        serde_json::json!({"active": [], "history": []})
    );

    let transports = crate::route_http_request(
        "GET",
        "/api/v0/security/transports/status",
        None,
        "",
        &state,
    )
    .await
    .expect("transport selector status");
    assert_eq!(transports.status, "200 OK");
    let transports = serde_json::from_str::<serde_json::Value>(&transports.body).unwrap();
    assert_eq!(transports["selectedMode"], "Direct");
    assert_eq!(transports["totalTransports"], 1);
    assert_eq!(transports["availableTransports"], 0);
    assert_eq!(transports["availableTransportTypes"], serde_json::json!([]));
    assert!(transports["lastConnectivityTest"].is_string());
    assert_eq!(transports["primaryTransportAvailable"], false);
    assert_eq!(transports["fallbackAvailable"], false);

    let listening_party = crate::route_http_request(
        "GET",
        "/api/v0/listening-party/pod:route-audit/route-audit-channel",
        None,
        "",
        &state,
    )
    .await
    .expect("empty listening-party state");
    assert_eq!(listening_party.status, "204 No Content");
}
