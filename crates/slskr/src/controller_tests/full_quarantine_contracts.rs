//! Controller full quarantine contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn quarantine_jury_request_creation_validates_reason_evidence_and_jurors() {
    let (state, _receiver) = test_state();

    let missing_reason = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"jurors":["juror-a"],"evidence":[{"type":"hash","reference":"opaque-ref"}]}"#,
        &state,
    )
    .await
    .expect("missing local reason");
    assert_eq!(missing_reason.status, "400 Bad Request");
    let missing_reason_json =
        serde_json::from_str::<serde_json::Value>(&missing_reason.body).unwrap();
    assert!(
        missing_reason_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Local quarantine reason is required."),
        "{missing_reason_json}"
    );

    // An evidence reference shaped like a filesystem path is real,
    // concrete leakage risk -- must be rejected, not merely counted.
    let unsafe_evidence = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"audit","jurors":["juror-a"],"evidence":[{"type":"hash","reference":"/etc/passwd"}]}"#,
        &state,
    )
    .await
    .expect("unsafe evidence reference");
    assert_eq!(unsafe_evidence.status, "400 Bad Request");
    let unsafe_evidence_json =
        serde_json::from_str::<serde_json::Value>(&unsafe_evidence.body).unwrap();
    assert!(
        unsafe_evidence_json["errors"].as_array().unwrap().iter().any(
            |error| error
                == "Evidence references must not include paths, raw hashes, endpoints, or private identifiers."
        ),
        "{unsafe_evidence_json}"
    );

    // A juror identifier that looks like a filesystem path is the
    // same class of leakage risk on the juror side.
    let unsafe_juror = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"audit","jurors":["/etc/shadow"],"evidence":[{"type":"hash","reference":"opaque-ref"}]}"#,
        &state,
    )
    .await
    .expect("unsafe juror identifier");
    assert_eq!(unsafe_juror.status, "400 Bad Request");
    let unsafe_juror_json = serde_json::from_str::<serde_json::Value>(&unsafe_juror.body).unwrap();
    assert!(
        unsafe_juror_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|error| error == "Juror identifiers must be opaque and safe."),
        "{unsafe_juror_json}"
    );

    // A genuinely safe request must still succeed.
    let valid = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"  audit  ","jurors":["juror-a"],"evidence":[{"type":"hash","reference":"opaque-ref"}]}"#,
        &state,
    )
    .await
    .expect("valid request");
    assert_eq!(valid.status, "200 OK", "{}", valid.body);
    let valid_json = serde_json::from_str::<serde_json::Value>(&valid.body).unwrap();
    assert_eq!(valid_json["request"]["localReason"], "audit");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn quarantine_jury_audit_report_reflects_real_status_not_hardcoded_zeros() {
    let (state, _receiver) = test_state();

    // Request A: reaches a real ReleaseCandidate quorum.
    let created_a = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"audit-a","jurors":["juror-a","juror-b"],"evidence":[{"type":"hash","reference":"opaque-ref-a"}],"minJurorVotes":2}"#,
        &state,
    )
    .await
    .expect("create request a");
    let request_a = serde_json::from_str::<serde_json::Value>(&created_a.body).unwrap()["request"]
        ["requestId"]
        .as_str()
        .unwrap()
        .to_owned();
    for juror in ["juror-a", "juror-b"] {
        let verdict = crate::route_http_request(
            "POST",
            "/api/v0/quarantine-jury/verdicts",
            None,
            &quarantine_signed_verdict_json(&request_a, juror, "ReleaseCandidate").to_string(),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(verdict.status, "200 OK", "{}", verdict.body);
    }

    // Request B: no verdicts at all -- stuck at real manual-review.
    let created_b = crate::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/requests",
        None,
        r#"{"localReason":"audit-b","jurors":["juror-c"],"evidence":[{"type":"hash","reference":"opaque-ref-b"}],"minJurorVotes":1}"#,
        &state,
    )
    .await
    .expect("create request b");
    let request_b = serde_json::from_str::<serde_json::Value>(&created_b.body).unwrap()["request"]
        ["requestId"]
        .as_str()
        .unwrap()
        .to_owned();

    let baseline =
        crate::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
            .await
            .expect("audit report before acceptance");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["requestCount"], 2, "{baseline_json}");
    // Real per-status counts, not hardcoded zeros: A is pending
    // release acceptance (quorum reached, not yet accepted), B is a
    // real manual-review (no verdicts at all).
    assert_eq!(
        baseline_json["pendingReleaseCandidateCount"], 1,
        "{baseline_json}"
    );
    assert_eq!(
        baseline_json["pendingManualReviewCount"], 1,
        "{baseline_json}"
    );
    assert_eq!(
        baseline_json["acceptedReleaseCandidateCount"], 0,
        "{baseline_json}"
    );
    assert_eq!(baseline_json["upholdQuarantineCount"], 0, "{baseline_json}");
    let entries = baseline_json["entries"].as_array().unwrap();
    let entry_a = entries
        .iter()
        .find(|entry| entry["requestId"] == request_a)
        .expect("entry for request a");
    assert_eq!(entry_a["status"], "pending-release-acceptance");
    assert_eq!(entry_a["verdictCount"], 2);
    assert_eq!(entry_a["quorumReached"], true);
    assert_eq!(entry_a["canAcceptReleaseCandidate"], true);
    let entry_b = entries
        .iter()
        .find(|entry| entry["requestId"] == request_b)
        .expect("entry for request b");
    assert_eq!(entry_b["status"], "manual-review");
    assert_eq!(entry_b["verdictCount"], 0);
    assert_eq!(entry_b["quorumReached"], false);

    // Accepting request A must move it out of "pending" and into a
    // real "accepted" bucket in a fresh report.
    let accept = crate::route_http_request(
        "POST",
        &format!("/api/v0/quarantine-jury/requests/{request_a}/accept-release-candidate"),
        None,
        "{}",
        &state,
    )
    .await
    .expect("accept request a");
    assert_eq!(accept.status, "200 OK", "{}", accept.body);

    let after_accept =
        crate::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
            .await
            .expect("audit report after acceptance");
    let after_accept_json = serde_json::from_str::<serde_json::Value>(&after_accept.body).unwrap();
    assert_eq!(
        after_accept_json["acceptedReleaseCandidateCount"], 1,
        "{after_accept_json}"
    );
    assert_eq!(
        after_accept_json["pendingReleaseCandidateCount"], 0,
        "{after_accept_json}"
    );

    // Backdate request B (still not accepted) well past the default
    // 72-hour staleness window -- proving isStale reflects real
    // request age and the real staleAfterHours floor (the oracle
    // clamps it to a minimum of 1 hour, so a "staleAfterHours=0"
    // query can't be used to fake staleness on a brand-new request).
    {
        let key = format!("quarantine/request/{request_b}");
        let mut features = state.controller_features.write_for_test().await;
        let mut backdated = features.get(&key).cloned().expect("request b exists");
        backdated["createdAt"] =
            serde_json::json!(crate::unix_timestamp().saturating_sub(100 * 3600));
        features.upsert(key, backdated).expect("backdate request b");
    }
    let stale = crate::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
        .await
        .expect("audit report after backdating request b");
    let stale_json = serde_json::from_str::<serde_json::Value>(&stale.body).unwrap();
    assert_eq!(stale_json["staleRequestCount"], 1, "{stale_json}");
    let stale_entries = stale_json["entries"].as_array().unwrap();
    let stale_entry_b = stale_entries
        .iter()
        .find(|entry| entry["requestId"] == request_b)
        .unwrap();
    assert_eq!(stale_entry_b["isStale"], true, "{stale_entry_b}");
    let stale_entry_a = stale_entries
        .iter()
        .find(|entry| entry["requestId"] == request_a)
        .unwrap();
    // Already-accepted requests are never stale, regardless of age.
    assert_eq!(stale_entry_a["isStale"], false, "{stale_entry_a}");
}
