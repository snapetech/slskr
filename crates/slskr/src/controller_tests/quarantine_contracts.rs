use super::{quarantine_signed_verdict_json, test_state};

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn quarantine_jury_requires_real_quorum_before_accepting_or_releasing() {
    let (state, _receiver) = test_state();

    let created = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"route-audit","jurors":["juror-a","juror-b","juror-c"],"evidence":[{"type":"hash","reference":"opaque-ref"}],"minJurorVotes":2}"#,
        &state,
    )
    .await
    .expect("create quarantine request");
    assert_eq!(created.status, "200 OK", "{}", created.body);
    let created_json = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let request_id = created_json["request"]["requestId"]
        .as_str()
        .unwrap()
        .to_owned();

    // A verdict from a juror not on the request must be rejected.
    let unlisted = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &quarantine_signed_verdict_json(&request_id, "not-a-juror", "ReleaseCandidate").to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unlisted.status, "400 Bad Request");

    // A verdict without a real signature (or with a payload hash that
    // doesn't match its own contents) must be rejected too -- not
    // silently accepted the way the old handler took any verdict
    // shape it was given.
    let unsigned = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &format!(
            r#"{{"requestId":"{request_id}","juror":"juror-a","verdict":"ReleaseCandidate"}}"#
        ),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unsigned.status, "400 Bad Request");
    let unsigned_json = serde_json::from_str::<serde_json::Value>(&unsigned.body).unwrap();
    assert!(
        unsigned_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Signed juror verdict is required."),
        "{unsigned_json}"
    );

    let mut tampered = quarantine_signed_verdict_json(&request_id, "juror-a", "ReleaseCandidate");
    tampered["verdict"] = serde_json::json!("UpholdQuarantine");
    let tampered_result = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        &tampered.to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(tampered_result.status, "400 Bad Request");
    let tampered_json = serde_json::from_str::<serde_json::Value>(&tampered_result.body).unwrap();
    assert!(
        tampered_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Signature payload hash does not match verdict contents."),
        "{tampered_json}"
    );

    // Before quorum (minJurorVotes=2), acceptance and release must be
    // denied -- not silently accepted, as the old fake handlers did.
    let too_early = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(too_early.status, "400 Bad Request", "{}", too_early.body);
    let too_early_json = serde_json::from_str::<serde_json::Value>(&too_early.body).unwrap();
    assert_eq!(too_early_json["isAccepted"], false);

    let package_too_early = crate::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/release-package"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(package_too_early.status, "400 Bad Request");
    let package_too_early_json =
        serde_json::from_str::<serde_json::Value>(&package_too_early.body).unwrap();
    assert_eq!(package_too_early_json["isReady"], false);
    assert!(package_too_early_json["errors"][0]
        .as_str()
        .unwrap()
        .contains("has not been accepted"));

    // Two of three jurors vote ReleaseCandidate -- quorum (2) reached
    // and a real 2/3 supermajority.
    for juror in ["juror-a", "juror-b"] {
        let verdict = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(&request_id, juror, "ReleaseCandidate").to_string(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(verdict.status, "200 OK", "{}", verdict.body);
    }

    let aggregate = crate::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/aggregate"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(aggregate.status, "200 OK");
    let aggregate_json = serde_json::from_str::<serde_json::Value>(&aggregate.body).unwrap();
    assert_eq!(aggregate_json["recommendation"], "ReleaseCandidate");
    assert_eq!(aggregate_json["totalVerdicts"], 2);
    assert_eq!(aggregate_json["requiredVotes"], 2);
    assert_eq!(aggregate_json["quorumReached"], true);

    let review = crate::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/review"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let review_json = serde_json::from_str::<serde_json::Value>(&review.body).unwrap();
    assert_eq!(review_json["canAcceptReleaseCandidate"], true);
    assert_eq!(review_json["verdicts"].as_array().unwrap().len(), 2);

    // Real quorum reached: acceptance must now succeed.
    let accepted = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        r#"{"acceptedBy":"route-audit-operator"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(accepted.status, "200 OK", "{}", accepted.body);
    let accepted_json = serde_json::from_str::<serde_json::Value>(&accepted.body).unwrap();
    assert_eq!(accepted_json["isAccepted"], true);
    assert_eq!(
        accepted_json["decision"]["acceptedBy"],
        "route-audit-operator"
    );

    // Re-accepting is idempotent, not a second decision.
    let reaccepted = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(reaccepted.status, "200 OK");
    let reaccepted_json = serde_json::from_str::<serde_json::Value>(&reaccepted.body).unwrap();
    assert_eq!(
        reaccepted_json["decision"]["id"],
        accepted_json["decision"]["id"]
    );

    // Now that a real acceptance decision exists, the release package
    // must be built from real stored data.
    let package = crate::route_http_request(
        "GET",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/release-package"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(package.status, "200 OK", "{}", package.body);
    let package_json = serde_json::from_str::<serde_json::Value>(&package.body).unwrap();
    assert_eq!(package_json["isReady"], true);
    assert_eq!(package_json["package"]["requestId"], request_id);
    assert_eq!(
        package_json["package"]["currentAggregate"]["recommendation"],
        "ReleaseCandidate"
    );
    assert_eq!(
        package_json["package"]["verdicts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    // Routing to a juror not on the request must be rejected before
    // ever reaching the (currently unwired) routing backend.
    let bad_route = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/routes"),
        None,
        r#"{"targetJurors":["not-a-juror"]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(bad_route.status, "400 Bad Request");
    let bad_route_json = serde_json::from_str::<serde_json::Value>(&bad_route.body).unwrap();
    assert_eq!(bad_route_json["success"], false);
    assert!(bad_route_json["errorMessage"]
        .as_str()
        .unwrap()
        .contains("safe jurors"));

    // Valid jurors pass validation but hit the same "routing backend
    // unavailable" outcome the oracle itself reports when its router
    // isn't configured -- a real code path, not a fabricated success.
    let real_route = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_id}/routes"),
        None,
        r#"{"targetJurors":["juror-a"]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(real_route.status, "400 Bad Request");
    let real_route_json = serde_json::from_str::<serde_json::Value>(&real_route.body).unwrap();
    assert_eq!(real_route_json["success"], false);
    assert_eq!(
        real_route_json["errorMessage"],
        "Routing backend is not available."
    );
}
