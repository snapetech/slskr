#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn listening_party_directory_ticket_streams_local_audio_ranges() {
    run_controller_future_on_large_stack("listening-party-directory-stream", || {
        listening_party_directory_ticket_streams_local_audio_ranges_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn listening_party_directory_ticket_streams_local_audio_ranges_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-listening-party-stream-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("create listening-party share root");
    std::fs::write(root.join("party.flac"), b"party-audio-bytes")
        .expect("write listening-party fixture");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_CONTROLLER_PROFILE", "native")
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let (content_id, filename) = {
        let shares = state.shares.read().await;
        let entry = shares.entries.first().expect("share fixture entry");
        (
            super::stable_content_hash(&entry.filename, entry.size).to_string(),
            entry.filename.clone(),
        )
    };
    let party_id = "party:stream-audit";
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            "listening-party/pod:stream-audit/general".to_owned(),
            serde_json::json!({
                "partyId": party_id,
                "podId": "pod:stream-audit",
                "channelId": "general",
                "hostPeerId": "tester",
                "action": "play",
                "contentId": content_id,
                "title": "Party Track",
                "artist": "Party Artist",
                "serverTimeUnixMs": super::unix_timestamp_millis(),
                "listed": true,
                "allowMeshStreaming": true,
            }),
        )
        .expect("persist listening-party fixture");

    let directory = super::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
        .await
        .expect("list listening-party directory");
    assert_eq!(directory.status, "200 OK", "{}", directory.body);
    let stream_path = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap()[0]
        ["streamPath"]
        .as_str()
        .expect("directory stream path")
        .to_owned();
    assert!(stream_path.contains(&super::url_encode(&content_id)));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listening-party stream server");
    let address = listener
        .local_addr()
        .expect("listening-party stream address");
    let server_state = Arc::clone(&state);
    let server = std::thread::Builder::new()
        .name("listening-party-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create listening-party server runtime")
                .block_on(async move {
                    let (stream, _) = listener.accept().await.expect("accept radio request");
                    super::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve radio response");
                });
        })
        .expect("spawn listening-party server");
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect radio client");
    client
        .write_all(
            format!(
                "GET {stream_path} HTTP/1.1\r\nHost: localhost\r\nRange: bytes=2-6\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .expect("write radio request");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read radio response");
    server.join().expect("radio server task");
    std::fs::remove_dir_all(root).expect("remove listening-party fixture");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("radio response header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("radio response headers");
    assert!(
        headers.starts_with("HTTP/1.1 206 Partial Content\r\n"),
        "{headers}"
    );
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Range: bytes 2-6/17\r\n"));
    assert_eq!(&raw[split + 4..], b"rty-a");
    assert!(!filename.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn listening_party_stream_limits_match_frozen_caps() {
    let mut limits = super::ListeningPartyStreamLimits::default();
    let mut party_permits = Vec::new();
    for index in 0..super::LISTED_PARTY_MAX_CONCURRENT_STREAMS {
        party_permits.push(
            limits
                .try_acquire("party:limit-audit", &format!("ip-{index}"))
                .expect("party stream slot"),
        );
    }
    assert_eq!(
        limits
            .try_acquire("party:limit-audit", "ip-over-cap")
            .unwrap_err(),
        super::ListeningPartyStreamLimitRejection::Party
    );

    let mut ip_permits = Vec::new();
    for _ in 0..super::LISTED_PARTY_MAX_CONCURRENT_STREAMS_PER_IP {
        ip_permits.push(
            limits
                .try_acquire("party:ip-audit", "same-ip")
                .expect("per-IP stream slot"),
        );
    }
    assert_eq!(
        limits.try_acquire("party:ip-audit", "same-ip").unwrap_err(),
        super::ListeningPartyStreamLimitRejection::Ip
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn quarantine_jury_request_creation_validates_reason_evidence_and_jurors() {
    let (state, _receiver) = test_state();

    let missing_reason = super::route_http_request(
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
    let unsafe_evidence = super::route_http_request(
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
    let unsafe_juror = super::route_http_request(
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
    let valid = super::route_http_request(
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
async fn quarantine_jury_audit_report_reflects_real_status_not_hardcoded_zeros() {
    let (state, _receiver) = test_state();

    // Request A: reaches a real ReleaseCandidate quorum.
    let created_a = super::route_http_request(
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
        let verdict = super::route_http_request(
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
    let created_b = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
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
    let accept = super::route_http_request(
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
        super::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
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
            serde_json::json!(super::unix_timestamp().saturating_sub(100 * 3600));
        features.upsert(key, backdated).expect("backdate request b");
    }
    let stale = super::route_http_request("GET", "/api/v0/quarantine-jury/audit", None, "", &state)
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_member_affinities_reflect_real_activity_not_hardcoded_zeros() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000ba07";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Affinity Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "member-1".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add ordinary member");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            super::pods::PodChannel {
                channel_id: "general".to_owned(),
                kind: serde_json::json!(0),
                name: "general".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    let now_millis = super::unix_timestamp() * 1000;
    {
        let mut channels = state.pod_channels.write().await;
        for index in 0..3 {
            channels
                .append(
                    pod_id.to_owned(),
                    "general".to_owned(),
                    "member-1".to_owned(),
                    format!("message {index}"),
                    String::new(),
                    now_millis + index,
                )
                .expect("append message");
        }
    }
    let opinion = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"member-1","subjectType":"MeshPeer","subjectId":"track-1","kind":"Like","strength":0.8,"confidence":0.9}"#,
        &state,
    )
    .await
    .expect("submit opinion");
    assert_eq!(opinion.status, "200 OK", "{}", opinion.body);

    let affinities = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity"),
        None,
        "",
        &state,
    )
    .await
    .expect("member affinities");
    assert_eq!(affinities.status, "200 OK");
    let affinities = serde_json::from_str::<serde_json::Value>(&affinities.body).unwrap();

    let member = &affinities["member-1"];
    assert_eq!(member["messageCount"], 3);
    assert_eq!(member["opinionCount"], 1);
    assert_eq!(member["trustScore"], 0.7);
    assert!(member["affinityScore"].as_f64().unwrap() > 0.0, "{member}");
    assert!(!member["recentActivity"].as_array().unwrap().is_empty());

    let owner = &affinities["owner-peer"];
    assert_eq!(owner["messageCount"], 0);
    assert_eq!(owner["opinionCount"], 0);
    assert_eq!(owner["trustScore"], 1.0, "owner gets the +0.3 role bonus");
    // Owner has no messages/opinions, but `create()` sets joined_at
    // and last_seen to "now", so is_active is real too -- a nonzero
    // activity_bonus-only score (0.2 * trust_component 0.5 = 0.1),
    // not the flat 0.0 the old hardcoded handler always returned.
    let owner_affinity = owner["affinityScore"].as_f64().unwrap();
    assert!(
        (owner_affinity - 0.1).abs() < 1e-9,
        "expected ~0.1, got {owner_affinity}"
    );

    let update = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/members/affinity/update"),
        None,
        "",
        &state,
    )
    .await
    .expect("update affinities");
    assert_eq!(update.status, "200 OK");
    let update = serde_json::from_str::<serde_json::Value>(&update.body).unwrap();
    assert_eq!(update["success"], true);
    assert_eq!(update["membersUpdated"], 2, "real member count, not 0");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_opinion_refresh_reports_real_counts_and_new_delta() {
    let (state, _receiver) = test_state();
    let pod_id = "pod-refresh";
    {
        let mut features = state.controller_features.write_for_test().await;
        features
            .upsert(
                format!("pod/opinion/{pod_id}/content-a/opinion-1"),
                serde_json::json!({"contentId":"content-a","score":8}),
            )
            .expect("store first opinion");
        features
            .upsert(
                format!("pod/opinion/{pod_id}/content-b/opinion-2"),
                serde_json::json!({"contentId":"content-b","score":6}),
            )
            .expect("store second opinion");
    }

    let first = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("first opinion refresh");
    assert_eq!(first.status, "200 OK", "{}", first.body);
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first_json["success"], true);
    assert_eq!(first_json["podId"], pod_id);
    assert_eq!(first_json["opinionsRefreshed"], 2);
    assert_eq!(first_json["newOpinions"], 2);

    let second = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("second opinion refresh");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    assert_eq!(second_json["opinionsRefreshed"], 2);
    assert_eq!(second_json["newOpinions"], 0);

    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            format!("pod/opinion/{pod_id}/content-c/opinion-3"),
            serde_json::json!({"contentId":"content-c","score":9}),
        )
        .expect("store third opinion");
    let third = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions/refresh"),
        None,
        "",
        &state,
    )
    .await
    .expect("third opinion refresh");
    let third_json = serde_json::from_str::<serde_json::Value>(&third.body).unwrap();
    assert_eq!(third_json["opinionsRefreshed"], 3);
    assert_eq!(third_json["newOpinions"], 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn opinions_summary_requires_subject_and_computes_a_real_weighted_score() {
    let (state, _receiver) = test_state();

    // Matches the oracle's required-parameter 400, not just the
    // generic "query string is present" check.
    let missing_subject_id = super::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_subject_id.status, "400 Bad Request");

    for (issuer, kind, strength, confidence) in [
        ("peer-a", "Like", 1.0, 1.0),
        ("peer-b", "Like", 1.0, 0.5),
        ("peer-c", "Hate", 1.0, 1.0),
    ] {
        let created = super::route_http_request(
            "POST",
            "/api/v0/opinions",
            None,
            &format!(
                r#"{{"issuer":"{issuer}","subjectType":"Track","subjectId":"track-1","kind":"{kind}","strength":{strength},"confidence":{confidence}}}"#
            ),
            &state,
        )
        .await
        .unwrap();
        assert_eq!(created.status, "200 OK", "{}", created.body);
    }
    // A different subject must not be counted in track-1's summary.
    super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"peer-d","subjectType":"Track","subjectId":"track-2","kind":"Like","strength":1.0,"confidence":1.0}"#,
        &state,
    )
    .await
    .unwrap();

    let summary = super::route_http_request(
        "GET",
        "/api/v0/opinions/summary?subjectType=Track&subjectId=track-1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(summary.status, "200 OK", "{}", summary.body);
    let summary = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap();
    assert_eq!(summary["subjectType"], "Track");
    assert_eq!(summary["subjectId"], "track-1");
    assert_eq!(summary["scope"], "global");
    assert_eq!(summary["total"], 3);
    assert_eq!(summary["positive"], 2);
    assert_eq!(summary["negative"], 1);
    // weighted = (1*1*1.0) + (1*1*0.5) + (-1*1*1.0) = 0.5
    assert!(
        (summary["weightedScore"].as_f64().unwrap() - 0.5).abs() < 1e-9,
        "{summary}"
    );
    // confidence = average(1.0, 0.5, 1.0)
    assert!(
        (summary["confidence"].as_f64().unwrap() - (2.5 / 3.0)).abs() < 1e-9,
        "{summary}"
    );
    assert_eq!(summary["opinions"].as_array().unwrap().len(), 3);
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_content_metadata_requires_a_real_content_id() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let missing =
        super::route_http_request("GET", "/api/v0/podcore/content/metadata", None, "", &state)
            .await
            .unwrap();
    assert_eq!(missing.status, "400 Bad Request");

    let malformed = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=not-a-content-id",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(malformed.status, "404 Not Found");

    let valid_response = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:music:recording:route-audit",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(valid_response.status, "200 OK", "{}", valid_response.body);
    assert_eq!(valid_response.content_type, "application/json");
    let valid = serde_json::from_str::<serde_json::Value>(&valid_response.body).unwrap();
    assert_eq!(valid["contentId"], "content:music:recording:route-audit");
    assert_eq!(valid["title"], "recording: route-audit");
    assert_eq!(valid["artist"], "Unknown");
    assert_eq!(valid["type"], "recording");
    assert_eq!(valid["domain"], "music");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_content_metadata_nominal.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/metadata",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/metadata",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_content_metadata_uses_musicbrainz_recording_release_and_artist_shapes(
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MusicBrainz metadata fixture");
    let address = listener
        .local_addr()
        .expect("MusicBrainz metadata fixture address");
    let server = tokio::spawn(async move {
        let recording_request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "id": "recording-metadata-audit",
                "title": "Metadata Track",
                "length": 187000,
                "artist-credit": [{
                    "name": "Metadata Artist",
                    "artist": {"id": "artist-metadata-audit"}
                }]
            }),
        )
        .await;
        assert!(
            recording_request.starts_with(
                "GET /recording/recording-metadata-audit?fmt=json&inc=artists+isrcs HTTP/1.1"
            ),
            "{recording_request}"
        );

        let release_request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "id": "release-metadata-audit",
                "title": "Metadata Album",
                "date": "2024-05",
                "artist-credit": [{
                    "name": "Metadata Artist",
                    "artist": {"id": "artist-metadata-audit"}
                }],
                "label-info": [{"label": {"name": "Audit Label"}}],
                "media": [{
                    "tracks": [
                        {"position": "1", "recording": {"id": "track-1"}},
                        {"position": "2", "recording": {"id": "track-2"}}
                    ]
                }]
            }),
        )
        .await;
        assert!(
            release_request.starts_with(
                "GET /release/release-metadata-audit?fmt=json&inc=recordings+artists+labels+discids+isrcs HTTP/1.1"
            ),
            "{release_request}"
        );

        let artist_request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "recordings": [{
                    "id": "artist-track",
                    "title": "Artist Track",
                    "artist-credit": [{
                        "name": "Metadata Artist",
                        "artist": {"id": "artist-metadata-audit"}
                    }]
                }]
            }),
        )
        .await;
        assert!(
            artist_request.starts_with(
                "GET /recording?query=artist-metadata-audit&fmt=json&limit=10 HTTP/1.1"
            ),
            "{artist_request}"
        );
    });

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut settings = state.integration_settings.write().await;
        settings.musicbrainz.base_url = format!("http://{address}");
        settings.musicbrainz.retry_attempts = 1;
    }

    let track = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:audio:track:recording-metadata-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("track metadata response");
    assert_eq!(track.status, "200 OK", "{}", track.body);
    assert_eq!(track.content_type, "application/json");
    let track = serde_json::from_str::<serde_json::Value>(&track.body).unwrap();
    assert_eq!(
        track["contentId"],
        "content:audio:track:recording-metadata-audit"
    );
    assert_eq!(track["title"], "Metadata Track");
    assert_eq!(track["artist"], "Metadata Artist");
    assert_eq!(
        track["additionalInfo"]["musicbrainz_id"],
        "recording-metadata-audit"
    );
    assert_eq!(track["additionalInfo"]["duration_ms"], "187000");
    assert_eq!(track["additionalInfo"]["album"], "Unknown");
    assert_eq!(track["additionalInfo"]["position"], "0");

    let album = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:audio:album:release-metadata-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("album metadata response");
    assert_eq!(album.status, "200 OK", "{}", album.body);
    assert_eq!(album.content_type, "application/json");
    let album = serde_json::from_str::<serde_json::Value>(&album.body).unwrap();
    assert_eq!(album["title"], "Metadata Album");
    assert_eq!(album["artist"], "Metadata Artist");
    assert_eq!(album["additionalInfo"]["release_date"], "2024-05-01");
    assert_eq!(album["additionalInfo"]["track_count"], "2");
    assert_eq!(album["additionalInfo"]["label"], "Audit Label");

    let artist = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/metadata?contentId=content:audio:artist:artist-metadata-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("artist metadata response");
    assert_eq!(artist.status, "200 OK", "{}", artist.body);
    assert_eq!(artist.content_type, "application/json");
    let artist = serde_json::from_str::<serde_json::Value>(&artist.body).unwrap();
    assert_eq!(artist["title"], "Metadata Artist");
    assert_eq!(artist["artist"], "Metadata Artist");
    assert_eq!(
        artist["additionalInfo"]["musicbrainz_artist_id"],
        "artist-metadata-audit"
    );
    server.await.expect("MusicBrainz metadata fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_content_metadata_populated.json"),
        serde_json::to_string_pretty(&[serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/podcore/content/metadata",
            "case": "populated-dynamic-state",
            "pass": true,
        })])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_content_search_returns_musicbrainz_recording_results()
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind MusicBrainz content search fixture");
    let address = listener
        .local_addr()
        .expect("MusicBrainz content search fixture address");
    let server = tokio::spawn(async move {
        let request = serve_json_fixture(
            &listener,
            serde_json::json!({
                "recordings": [{
                    "id": "recording-search-audit",
                    "title": "Search Audit Track",
                    "artist-credit": [{
                        "name": "Audit Artist",
                        "artist": {"id": "artist-search-audit"}
                    }]
                }]
            }),
        )
        .await;
        assert!(
            request.starts_with("GET /recording?query=Search+Audit&fmt=json&limit=1 HTTP/1.1"),
            "{request}"
        );
        assert!(
            request.to_ascii_lowercase().contains("accept-language: en"),
            "{request}"
        );
    });
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut settings = state.integration_settings.write().await;
        settings.musicbrainz.base_url = format!("http://{address}");
        settings.musicbrainz.retry_attempts = 1;
    }

    let missing_query =
        super::route_http_request("GET", "/api/v0/podcore/content/search", None, "", &state)
            .await
            .unwrap();
    assert_eq!(missing_query.status, "400 Bad Request");

    // Only the "audio" domain is supported (matching the oracle's real
    // MusicBrainz-backed search, which only covers audio) -- any other
    // requested domain must come back empty, not error or ignore the
    // filter.
    let wrong_domain = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/search?query=Search%20Audit&domain=video",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(wrong_domain.status, "200 OK");
    assert_eq!(wrong_domain.content_type, "application/json");
    assert_eq!(wrong_domain.body, "[]");

    let found = super::route_http_request(
        "GET",
        "/api/v0/podcore/content/search?query=Search%20Audit&domain=audio&limit=1",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(found.status, "200 OK", "{}", found.body);
    assert_eq!(found.content_type, "application/json");
    let found_json = serde_json::from_str::<serde_json::Value>(&found.body).unwrap();
    let results = found_json
        .as_array()
        .expect("flat array, not a wrapper object");
    assert_eq!(results.len(), 1, "{found_json}");
    assert!(
        results[0]["contentId"]
            .as_str()
            .unwrap()
            .starts_with("content:audio:track:"),
        "{found_json}"
    );
    assert_eq!(
        results[0]["contentId"],
        "content:audio:track:recording-search-audit"
    );
    assert_eq!(results[0]["title"], "Search Audit Track");
    assert_eq!(results[0]["subtitle"], "Audit Artist");
    assert_eq!(results[0]["domain"], "audio");
    assert_eq!(results[0]["type"], "track");
    assert_eq!(
        results[0]["metadata"]["musicbrainz_recording_id"],
        "recording-search-audit"
    );
    assert_eq!(
        results[0]["metadata"]["musicbrainz_artist_id"],
        "artist-search-audit"
    );
    server
        .await
        .expect("MusicBrainz content search fixture task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_content_search.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/search",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/search",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/content/search",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn podcore_membership_and_message_stats_match_storage_contracts() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:stats-audit";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Stats audit",
                "isPublic": true,
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize stats pod"),
            "owner-peer".to_owned(),
        )
        .expect("create stats pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "moderator-peer".to_owned(),
                role: "mod".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: Some("2026-01-01T00:00:00+00:00".to_owned()),
                last_seen: Some("2026-01-02T00:00:00+00:00".to_owned()),
            },
        )
        .expect("add stats member");
    {
        let mut channels = state.pod_channels.write().await;
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "owner-peer".to_owned(),
                "one".to_owned(),
                String::new(),
                1_000,
            )
            .expect("append first stats message");
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "moderator-peer".to_owned(),
                "two".to_owned(),
                String::new(),
                2_000,
            )
            .expect("append second stats message");
    }

    let membership =
        super::route_http_request("GET", "/api/v0/podcore/membership/stats", None, "", &state)
            .await
            .expect("membership stats");
    let membership_json = serde_json::from_str::<serde_json::Value>(&membership.body).unwrap();
    assert_eq!(membership_json["totalMemberships"], 2);
    assert_eq!(membership_json["activeMemberships"], 2);
    assert_eq!(membership_json["membershipsByRole"]["owner"], 1);
    assert_eq!(membership_json["membershipsByRole"]["mod"], 1);
    assert_eq!(membership_json["membershipsByPod"][pod_id], 2);
    assert!(membership_json["lastOperation"].is_string());

    let messages =
        super::route_http_request("GET", "/api/v0/podcore/messages/stats", None, "", &state)
            .await
            .expect("message stats");
    let messages_json = serde_json::from_str::<serde_json::Value>(&messages.body).unwrap();
    assert_eq!(messages_json["totalMessages"], 2);
    assert_eq!(messages_json["totalSizeBytes"], 400);
    assert_eq!(messages_json["messagesPerPod"][pod_id], 2);
    assert_eq!(messages_json["messagesPerChannel"]["general"], 2);
    assert_eq!(messages_json["oldestMessage"], "1970-01-01T00:00:01+00:00");
    assert_eq!(messages_json["newestMessage"], "1970-01-01T00:00:02+00:00");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_dht_stats_reflect_real_publications_not_a_pod_count_proxy(
) {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": "dht-audit-pod-1",
                "name": "DHT Audit One",
                "visibility": "public",
                "focusContentId": "content:audio:artist:some-artist",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod one");
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": "dht-audit-pod-2",
                "name": "DHT Audit Two",
                "visibility": "private",
                "focusContentId": "content:video:movie:some-movie",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod two");

    // Two real pods exist but neither has been DHT-published yet --
    // pod_count and dht-publication-count are different things.
    let baseline = super::route_http_request("GET", "/api/v0/podcore/dht/stats", None, "", &state)
        .await
        .expect("baseline dht stats");
    assert_eq!(baseline.status, "200 OK", "{}", baseline.body);
    assert_eq!(baseline.content_type, "application/json");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["activePublications"], 0, "{baseline_json}");
    assert_eq!(baseline_json["totalPublished"], 0, "{baseline_json}");
    let malformed_stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/dht/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed dht stats query");
    let malformed_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_stats.body).unwrap();
    assert_eq!(malformed_stats.status, "200 OK", "{}", malformed_stats.body);
    assert_eq!(malformed_stats_json["activePublications"], 0);
    assert_eq!(malformed_stats_json["totalPublished"], 0);

    for path in [
        "/api/v0/podcore/dht/publish/dht-audit-pod-1",
        "/api/v0/podcore/dht/publish/dht-audit-pod-2",
    ] {
        let published = super::route_http_request("POST", path, None, "{}", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(published.status, "200 OK", "{path}");
    }
    // Republishing the same pod must still add to the real all-time
    // counter without inflating the currently-active count.
    let republished = super::route_http_request(
        "POST",
        "/api/v0/podcore/dht/update/dht-audit-pod-1",
        None,
        "{}",
        &state,
    )
    .await
    .expect("republish pod one");
    assert_eq!(republished.status, "200 OK");

    let malformed_refresh =
        super::route_http_request("POST", "/api/v0/podcore/dht/refresh/", None, "", &state)
            .await
            .expect("reject blank dht refresh pod id");
    assert_eq!(
        malformed_refresh.status, "400 Bad Request",
        "{}",
        malformed_refresh.body
    );

    let refresh = super::route_http_request(
        "POST",
        "/api/v0/podcore/dht/refresh/dht-audit-pod-1",
        None,
        "",
        &state,
    )
    .await
    .expect("refresh pod one");
    assert_eq!(refresh.status, "200 OK", "{}", refresh.body);
    assert_eq!(refresh.content_type, "application/json");
    let refresh_json = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap();
    assert_eq!(refresh_json["success"], true, "{refresh_json}");
    assert_eq!(refresh_json["podId"], "dht-audit-pod-1");
    assert_eq!(refresh_json["wasRepublished"], false);
    assert!(refresh_json["nextRefresh"].is_string());
    assert!(refresh_json.get("errorMessage").is_none());

    let stats = super::route_http_request("GET", "/api/v0/podcore/dht/stats", None, "", &state)
        .await
        .expect("dht stats");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["activePublications"], 2, "{stats_json}");
    assert_eq!(stats_json["totalPublished"], 3, "{stats_json}");
    assert_eq!(stats_json["expiredPublications"], 1, "{stats_json}");
    assert_eq!(
        stats_json["publicationsByDomain"]["audio"], 2,
        "{stats_json}"
    );
    assert_eq!(
        stats_json["publicationsByDomain"]["video"], 1,
        "{stats_json}"
    );
    assert_eq!(
        stats_json["publicationsByVisibility"]["Listed"], 2,
        "{stats_json}"
    );
    assert_eq!(
        stats_json["publicationsByVisibility"]["Private"], 1,
        "{stats_json}"
    );
    assert!(
        stats_json["lastPublishOperation"].is_string(),
        "{stats_json}"
    );

    let unpublished = super::route_http_request(
        "DELETE",
        "/api/v0/podcore/dht/unpublish/dht-audit-pod-2",
        None,
        "",
        &state,
    )
    .await
    .expect("unpublish pod two");
    assert_eq!(unpublished.status, "200 OK");
    let malformed_unpublish =
        super::route_http_request("DELETE", "/api/v0/podcore/dht/unpublish/", None, "", &state)
            .await
            .expect("reject blank dht unpublish pod id");
    assert_eq!(
        malformed_unpublish.status, "400 Bad Request",
        "{}",
        malformed_unpublish.body
    );

    let after_unpublish =
        super::route_http_request("GET", "/api/v0/podcore/dht/stats", None, "", &state)
            .await
            .expect("dht stats after unpublish");
    assert_eq!(after_unpublish.status, "200 OK", "{}", after_unpublish.body);
    assert_eq!(after_unpublish.content_type, "application/json");
    let after_unpublish_json =
        serde_json::from_str::<serde_json::Value>(&after_unpublish.body).unwrap();
    assert_eq!(
        after_unpublish_json["activePublications"], 1,
        "{after_unpublish_json}"
    );
    assert_eq!(
        after_unpublish_json["totalPublished"], 3,
        "{after_unpublish_json}"
    );
    assert_eq!(
        after_unpublish_json["expiredPublications"], 2,
        "{after_unpublish_json}"
    );
    assert!(
        after_unpublish_json["publicationsByDomain"]["video"] == 1,
        "{after_unpublish_json}"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_dht_stats.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/stats",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/stats",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/dht/refresh/{*podId}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/dht/refresh/{*podId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "DELETE",
                "route": "/api/v0/podcore/dht/unpublish/{*podId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_dht_metadata_reads_and_verifies_the_published_record()
{
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let pod_id = "metadata-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Published metadata",
                "visibility": "Listed",
                "isPublic": true,
            }))
            .expect("deserialize metadata pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create metadata pod");
    let published = super::route_http_request(
        "POST",
        "/api/v0/podcore/dht/publish",
        None,
        &serde_json::json!({
            "pod": {
                "podId": pod_id,
                "name": "Published metadata",
                "visibility": "public"
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("publish pod metadata");
    assert_eq!(published.status, "200 OK", "{}", published.body);
    let published_json = serde_json::from_str::<serde_json::Value>(&published.body).unwrap();
    assert_eq!(published_json["podId"], pod_id);
    assert!(published_json.get("publishedPod").is_none());

    let metadata = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/dht/metadata/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("get published metadata");
    assert_eq!(metadata.status, "200 OK", "{}", metadata.body);
    assert_eq!(metadata.content_type, "application/json");
    let metadata_json = serde_json::from_str::<serde_json::Value>(&metadata.body).unwrap();
    assert_eq!(metadata_json["found"], true);
    assert_eq!(metadata_json["podId"], pod_id);
    assert_eq!(metadata_json["publishedPod"]["name"], "Published metadata");
    assert_eq!(metadata_json["isValidSignature"], true);

    let unpublished = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/dht/unpublish/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("unpublish metadata");
    assert_eq!(unpublished.status, "200 OK");
    let missing = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/dht/metadata/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing published metadata");
    assert_eq!(missing.status, "404 Not Found");
    assert_eq!(missing.content_type, "application/json");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&missing.body).unwrap(),
        serde_json::json!({"found": false, "error": "Pod not found"})
    );
    let malformed =
        super::route_http_request("GET", "/api/v0/podcore/dht/metadata/", None, "", &state)
            .await
            .expect("reject blank published metadata pod id");
    assert_eq!(malformed.status, "400 Bad Request", "{}", malformed.body);
    assert_eq!(malformed.body, r#"{"error":"Pod ID is required"}"#);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_dht_metadata.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/dht/metadata/{*podId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_backfill_sync_and_sync_all_report_real_local_work() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));

    let empty_stats =
        super::route_http_request("GET", "/api/v0/podcore/backfill/stats", None, "", &state)
            .await
            .expect("empty backfill stats");
    assert_eq!(empty_stats.status, "200 OK", "{}", empty_stats.body);
    let empty_stats_json = serde_json::from_str::<serde_json::Value>(&empty_stats.body).unwrap();
    assert_eq!(empty_stats_json["totalBackfillRequestsSent"], 0);
    assert_eq!(empty_stats_json["totalBackfillRequestsReceived"], 0);
    assert_eq!(empty_stats_json["totalMessagesBackfilled"], 0);
    assert_eq!(empty_stats_json["totalBackfillBytesTransferred"], 0);
    assert_eq!(empty_stats_json["averageBackfillDurationMs"], 0.0);
    assert_eq!(
        empty_stats_json["backfillRequestsByPod"],
        serde_json::json!({})
    );
    assert_eq!(
        empty_stats_json["lastBackfillOperation"],
        super::PODCORE_MIN_DATETIME
    );
    let malformed_stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/backfill/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed backfill stats query");
    let malformed_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_stats.body).unwrap();
    assert_eq!(malformed_stats.status, "200 OK", "{}", malformed_stats.body);
    assert_eq!(malformed_stats_json["totalBackfillRequestsSent"], 0);
    assert_eq!(malformed_stats_json["totalMessagesBackfilled"], 0);
    let pod_id = "backfill-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Backfill audit",
                "isPublic": true,
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize backfill pod"),
            "tester".to_owned(),
        )
        .expect("create backfill pod");
    {
        let mut channels = state.pod_channels.write().await;
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "peer-1".to_owned(),
                "old".to_owned(),
                String::new(),
                1_000,
            )
            .expect("append old message");
        channels
            .append(
                pod_id.to_owned(),
                "general".to_owned(),
                "peer-1".to_owned(),
                "new".to_owned(),
                String::new(),
                1_001,
            )
            .expect("append new message");
    }

    let sync = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/backfill/{pod_id}/sync"),
        None,
        r#"{"general":1000}"#,
        &state,
    )
    .await
    .expect("sync backfill");
    assert_eq!(sync.status, "200 OK", "{}", sync.body);
    assert_eq!(sync.content_type, "application/json");
    let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap();
    assert_eq!(sync_json["totalMessagesReceived"], 1);
    assert!(sync_json.get("messagesReceived").is_none());
    assert!(sync_json.get("bytesTransferred").is_none());

    let last_seen = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/backfill/{pod_id}/general/last-seen"),
        None,
        "1",
        &state,
    )
    .await
    .expect("persist last-seen timestamp");
    assert_eq!(last_seen.status, "200 OK");
    assert!(last_seen.body.is_empty());

    let last_seen_readback = super::route_http_request(
        "GET",
        &format!("/api/v0/podcore/backfill/{pod_id}/last-seen"),
        None,
        "",
        &state,
    )
    .await
    .expect("read last-seen timestamp");
    assert_eq!(
        last_seen_readback.status, "200 OK",
        "{}",
        last_seen_readback.body
    );
    assert_eq!(last_seen_readback.content_type, "application/json");
    let last_seen_readback_json =
        serde_json::from_str::<serde_json::Value>(&last_seen_readback.body).unwrap();
    assert_eq!(last_seen_readback_json, serde_json::json!({"general": 1}));

    let sync_all = super::route_http_request(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        None,
        "",
        &state,
    )
    .await
    .expect("sync all backfill");
    assert_eq!(sync_all.status, "200 OK");
    assert_eq!(sync_all.content_type, "application/json");
    let sync_all_json = serde_json::from_str::<serde_json::Value>(&sync_all.body).unwrap();
    assert_eq!(sync_all_json.as_array().unwrap().len(), 1);
    assert_eq!(sync_all_json[0]["totalMessagesReceived"], 2);

    let stats =
        super::route_http_request("GET", "/api/v0/podcore/backfill/stats", None, "", &state)
            .await
            .expect("backfill stats");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalBackfillRequestsSent"], 2);
    assert_eq!(stats_json["totalBackfillRequestsReceived"], 0);
    assert_eq!(stats_json["totalMessagesBackfilled"], 3);
    assert_eq!(stats_json["totalBackfillBytesTransferred"], 609);
    assert_eq!(stats_json["backfillRequestsByPod"][pod_id], 2);
    assert!(stats_json["lastBackfillOperation"].is_string());

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_backfill.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/backfill/{podId}/sync",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/backfill/{podId}/sync",
                "case": "mutation-side-effects-and-readback",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "POST",
                "route": "/api/v0/podcore/backfill/sync-all",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/{podId}/last-seen",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/{podId}/last-seen",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/stats",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/stats",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/backfill/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn podcore_routing_matches_message_router_contract_and_updates_real_stats() {
    let (state, _receiver) = test_state();
    let pod_id = "routing-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Routing audit",
                "channels": [{"channelId": "general", "name": "General"}]
            }))
            .expect("deserialize routing pod"),
            "sender-peer".to_owned(),
        )
        .expect("create routing pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "target-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add routing target");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "banned-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: true,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add banned routing peer");

    let routed = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-message-1",
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "sender-peer",
            "body": "hello",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route message");
    assert_eq!(
        routed.status, "500 Internal Server Error",
        "{}",
        routed.body
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&routed.body).unwrap()["error"],
        "Failed to route message"
    );

    let direct = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route-to-peers",
        None,
        &serde_json::json!({
            "message": {
                "messageId": "routing-message-2",
                "podId": pod_id,
                "channelId": "general",
                "senderPeerId": "sender-peer",
            },
            "targetPeerIds": [" target-peer ", "target-peer"],
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route message to peers");
    assert_eq!(direct.status, "200 OK", "{}", direct.body);
    let direct_json = serde_json::from_str::<serde_json::Value>(&direct.body).unwrap();
    assert_eq!(direct_json["success"], false);
    assert_eq!(direct_json["targetPeerCount"], 1);
    assert_eq!(direct_json["successfullyRoutedCount"], 0);
    assert_eq!(direct_json["failedRoutingCount"], 1);
    assert_eq!(
        direct_json["failedPeerIds"],
        serde_json::json!(["target-peer"])
    );

    let stats = super::route_http_request("GET", "/api/v0/podcore/routing/stats", None, "", &state)
        .await
        .expect("routing stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalMessagesRouted"], 1);
    assert_eq!(stats_json["totalRoutingAttempts"], 1);
    assert_eq!(stats_json["successfulRoutingCount"], 0);
    assert_eq!(stats_json["failedRoutingCount"], 1);
    assert_eq!(stats_json["activeDeduplicationItems"], 1);
    assert_eq!(stats_json["routingStatsByPod"][pod_id], 1);
    assert!(stats_json["bloomFilterFillRatio"].as_f64().unwrap() > 0.0);
    assert!(stats_json["estimatedFalsePositiveRate"].as_f64().unwrap() > 0.0);
    assert!(stats_json["lastRoutingOperation"].is_string());

    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-message-1",
            "podId": pod_id,
            "channelId": "general",
            "senderPeerId": "sender-peer",
        })
        .to_string(),
        &state,
    )
    .await
    .expect("route duplicate message");
    assert_eq!(duplicate.status, "200 OK");
    let duplicate_json = serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap();
    assert_eq!(duplicate_json["targetPeerCount"], 0);
    assert_eq!(
        duplicate_json["errorMessage"],
        "Message already routed (duplicate)"
    );

    let missing_channel = super::route_http_request(
        "POST",
        "/api/v0/podcore/routing/route",
        None,
        &serde_json::json!({
            "messageId": "routing-message-invalid",
            "podId": pod_id,
        })
        .to_string(),
        &state,
    )
    .await
    .expect("validate route message");
    assert_eq!(missing_channel.status, "400 Bad Request");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_podcore_discovery_stats_use_registrations_and_search_activity()
{
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    let empty_stats =
        super::route_http_request("GET", "/api/v0/podcore/discovery/stats", None, "", &state)
            .await
            .expect("empty discovery stats");
    assert_eq!(empty_stats.status, "200 OK", "{}", empty_stats.body);
    assert_eq!(empty_stats.content_type, "application/json");
    let empty_stats_json = serde_json::from_str::<serde_json::Value>(&empty_stats.body).unwrap();
    assert_eq!(
        empty_stats_json["totalRegisteredPods"], 0,
        "{empty_stats_json}"
    );
    assert_eq!(
        empty_stats_json["activeDiscoveryEntries"], 0,
        "{empty_stats_json}"
    );
    assert_eq!(empty_stats_json["expiredEntries"], 0, "{empty_stats_json}");
    assert_eq!(
        empty_stats_json["registrationsByTag"],
        serde_json::json!({})
    );
    assert_eq!(empty_stats_json["searchesByType"], serde_json::json!({}));
    let malformed_stats = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/stats?unexpected=not-a-number",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed discovery stats query");
    let malformed_stats_json =
        serde_json::from_str::<serde_json::Value>(&malformed_stats.body).unwrap();
    assert_eq!(malformed_stats.status, "200 OK", "{}", malformed_stats.body);
    assert_eq!(malformed_stats_json["totalRegisteredPods"], 0);
    assert_eq!(malformed_stats_json["activeDiscoveryEntries"], 0);
    let empty_search = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/name/audit-pod",
        None,
        "",
        &state,
    )
    .await
    .expect("empty discovery search");
    assert_eq!(empty_search.status, "200 OK", "{}", empty_search.body);
    assert_eq!(empty_search.content_type, "application/json");
    let empty_search_json = serde_json::from_str::<serde_json::Value>(&empty_search.body).unwrap();
    assert_eq!(empty_search_json["pods"], serde_json::json!([]));
    assert_eq!(empty_search_json["searchTerm"], "audit-pod");
    assert_eq!(empty_search_json["searchType"], "name");
    assert_eq!(empty_search_json["totalFound"], 0);

    let empty_all = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/all?limit=50",
        None,
        "",
        &state,
    )
    .await
    .expect("empty all-pods discovery search");
    let empty_all_json = serde_json::from_str::<serde_json::Value>(&empty_all.body).unwrap();
    assert_eq!(empty_all.status, "200 OK", "{}", empty_all.body);
    assert_eq!(empty_all.content_type, "application/json");
    assert_eq!(empty_all_json["pods"], serde_json::json!([]));
    assert_eq!(empty_all_json["searchTerm"], "50");
    assert_eq!(empty_all_json["searchType"], "all");
    assert_eq!(empty_all_json["totalFound"], 0);

    let malformed_all = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/all?limit=0",
        None,
        "",
        &state,
    )
    .await
    .expect("reject invalid all-pods discovery limit");
    assert_eq!(
        malformed_all.status, "400 Bad Request",
        "{}",
        malformed_all.body
    );
    assert!(malformed_all
        .body
        .contains("Limit must be between 1 and 100"));

    for (path, expected) in [
        (
            "/api/v0/podcore/discovery/name/",
            "Name is required and must be within length limits",
        ),
        (
            "/api/v0/podcore/discovery/tag/",
            "Tag is required and must be within length limits",
        ),
        (
            "/api/v0/podcore/discovery/tags/",
            "Tags are required and must be within length limits",
        ),
        (
            "/api/v0/podcore/discovery/content/",
            "ContentId is required",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(
            response.status, "400 Bad Request",
            "{path}: {}",
            response.body
        );
        assert!(
            response.body.contains(expected),
            "{path}: {}",
            response.body
        );
    }

    let registered = super::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/register",
        None,
        r#"{"podId":"discovery-audit-pod","name":"Audit Pod","visibility":"Listed","tags":["music","live"],"focusContentId":"content:music:recording:discovery-audit"}"#,
        &state,
    )
    .await
    .expect("register discovery pod");
    assert_eq!(registered.status, "200 OK", "{}", registered.body);
    assert_eq!(registered.content_type, "application/json");

    let search = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/name/audit-pod",
        None,
        "",
        &state,
    )
    .await
    .expect("search discovery pods");
    assert_eq!(search.status, "200 OK", "{}", search.body);
    assert_eq!(search.content_type, "application/json");
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap();
    assert_eq!(
        search_json["pods"].as_array().unwrap().len(),
        1,
        "{search_json}"
    );
    assert_eq!(search_json["searchTerm"], "audit-pod");
    assert_eq!(search_json["searchType"], "name");
    assert_eq!(search_json["totalFound"], 1);

    let tag = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/tag/music",
        None,
        "",
        &state,
    )
    .await
    .expect("search discovery tag");
    let tag_json = serde_json::from_str::<serde_json::Value>(&tag.body).unwrap();
    assert_eq!(tag.status, "200 OK", "{}", tag.body);
    assert_eq!(tag.content_type, "application/json");
    assert_eq!(tag_json["pods"].as_array().unwrap().len(), 1, "{tag_json}");
    assert_eq!(tag_json["searchTerm"], "music");
    assert_eq!(tag_json["searchType"], "tag");
    assert_eq!(tag_json["totalFound"], 1);

    let tags = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/tags/music,live",
        None,
        "",
        &state,
    )
    .await
    .expect("search discovery tags");
    let tags_json = serde_json::from_str::<serde_json::Value>(&tags.body).unwrap();
    assert_eq!(tags.status, "200 OK", "{}", tags.body);
    assert_eq!(tags.content_type, "application/json");
    assert_eq!(
        tags_json["pods"].as_array().unwrap().len(),
        1,
        "{tags_json}"
    );
    assert_eq!(tags_json["searchTerm"], "music,live");
    assert_eq!(tags_json["searchType"], "tags");
    assert_eq!(tags_json["totalFound"], 1);

    for (path, search_type, search_term) in [
        (
            "/api/v0/podcore/discovery/name/not-present",
            "name",
            "not-present",
        ),
        (
            "/api/v0/podcore/discovery/tag/not-present",
            "tag",
            "not-present",
        ),
        (
            "/api/v0/podcore/discovery/tags/not-present,other",
            "tags",
            "not-present,other",
        ),
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        let response_json =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
        assert_eq!(response_json["pods"], serde_json::json!([]), "{path}");
        assert_eq!(response_json["searchType"], search_type, "{path}");
        assert_eq!(response_json["searchTerm"], search_term, "{path}");
        assert_eq!(response_json["totalFound"], 0, "{path}");
    }

    let all = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/all?limit=50",
        None,
        "",
        &state,
    )
    .await
    .expect("populated all-pods discovery search");
    let all_json = serde_json::from_str::<serde_json::Value>(&all.body).unwrap();
    assert_eq!(all.status, "200 OK", "{}", all.body);
    assert_eq!(all.content_type, "application/json");
    assert_eq!(all_json["pods"].as_array().unwrap().len(), 1, "{all_json}");
    assert_eq!(all_json["searchTerm"], "50");
    assert_eq!(all_json["searchType"], "all");
    assert_eq!(all_json["totalFound"], 1);

    let missing_content = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/content/content:music:recording:not-present",
        None,
        "",
        &state,
    )
    .await
    .expect("missing content discovery search");
    let missing_content_json =
        serde_json::from_str::<serde_json::Value>(&missing_content.body).unwrap();
    assert_eq!(missing_content.status, "200 OK", "{}", missing_content.body);
    assert_eq!(missing_content_json["pods"], serde_json::json!([]));
    assert_eq!(missing_content_json["searchType"], "content");
    assert_eq!(
        missing_content_json["searchTerm"],
        "content:music:recording:not-present"
    );
    assert_eq!(missing_content_json["totalFound"], 0);

    let content = super::route_http_request(
        "GET",
        "/api/v0/podcore/discovery/content/content:music:recording:discovery-audit",
        None,
        "",
        &state,
    )
    .await
    .expect("populated content discovery search");
    let content_json = serde_json::from_str::<serde_json::Value>(&content.body).unwrap();
    assert_eq!(content.status, "200 OK", "{}", content.body);
    assert_eq!(content.content_type, "application/json");
    assert_eq!(
        content_json["pods"].as_array().unwrap().len(),
        1,
        "{content_json}"
    );
    assert_eq!(content_json["searchType"], "content");
    assert_eq!(
        content_json["searchTerm"],
        "content:music:recording:discovery-audit"
    );
    assert_eq!(content_json["totalFound"], 1);

    let stats =
        super::route_http_request("GET", "/api/v0/podcore/discovery/stats", None, "", &state)
            .await
            .expect("discovery stats");
    assert_eq!(stats.status, "200 OK", "{}", stats.body);
    assert_eq!(stats.content_type, "application/json");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalRegisteredPods"], 1);
    assert_eq!(stats_json["activeDiscoveryEntries"], 5);
    assert_eq!(stats_json["expiredEntries"], 0);
    assert_eq!(stats_json["registrationsByTag"]["music"], 1);
    assert_eq!(stats_json["registrationsByTag"]["live"], 1);
    assert_eq!(stats_json["searchesByType"]["name"], 3);
    assert_eq!(stats_json["searchesByType"]["all"], 2);
    assert_eq!(stats_json["searchesByType"]["content"], 2);
    assert!(stats_json["lastDiscoveryOperation"].is_string());

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("podcore_discovery_stats.json"),
        serde_json::to_string_pretty(&[
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/stats",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/stats",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/name/{name}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tag/{tag}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/tags/{tags}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/all",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "nominal-status-headers-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "missing-empty-or-conflict-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "malformed-path-query-or-body",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/content/{*contentId}",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
            serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": "/api/v0/podcore/discovery/stats",
                "case": "populated-dynamic-state",
                "pass": true,
            }),
        ])
        .expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn library_bloom_preview_reflects_real_hashdb_contents_not_an_empty_filter() {
    let (state, _receiver) = test_state();

    // An empty store must not be reported as containing anything, but
    // the filter parameters should still reflect the real (empty) item
    // count rather than a canned placeholder.
    let empty = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"saltId":"audit-empty"}"#,
        &state,
    )
    .await
    .unwrap();
    let empty = serde_json::from_str::<serde_json::Value>(&empty.body).unwrap();
    assert_eq!(empty["itemCount"], 0);
    assert_eq!(empty["fillRatio"], 0.0);
    assert_eq!(empty["namespaceItemCounts"], serde_json::json!({}));

    // Seed two real hashdb entries with MusicBrainz recording ids.
    {
        let mut discovery = state.content_discovery.write().await;
        discovery
            .merge_hash_entries(vec![
                super::content_discovery::HashDbEntry {
                    flac_key: "bloom-key-1".to_owned(),
                    size: 111,
                    file_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                        .to_owned(),
                    music_brainz_id: "11111111-1111-1111-1111-111111111111".to_owned(),
                    ..Default::default()
                },
                super::content_discovery::HashDbEntry {
                    flac_key: "bloom-key-2".to_owned(),
                    size: 222,
                    file_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                        .to_owned(),
                    music_brainz_id: "22222222-2222-2222-2222-222222222222".to_owned(),
                    ..Default::default()
                },
            ])
            .expect("seed hash entries");
    }

    let populated = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"saltId":"audit-populated"}"#,
        &state,
    )
    .await
    .unwrap();
    let populated = serde_json::from_str::<serde_json::Value>(&populated.body).unwrap();
    assert_eq!(populated["itemCount"], 2);
    assert!(
        populated["fillRatio"].as_f64().unwrap() > 0.0,
        "a populated store must not report an all-zero bitset: {populated}"
    );
    assert_eq!(
        populated["namespaceItemCounts"],
        serde_json::json!({"musicbrainz:recording": 2})
    );
    let bits = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        populated["bitsBase64"].as_str().unwrap(),
    )
    .unwrap();
    assert!(
        bits.iter().any(|byte| *byte != 0),
        "expected at least one set bit in the populated filter"
    );

    // Different salts for the same underlying data must not produce the
    // same bit pattern -- that's the entire point of salting.
    let differently_salted = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"saltId":"audit-different-salt"}"#,
        &state,
    )
    .await
    .unwrap();
    let differently_salted =
        serde_json::from_str::<serde_json::Value>(&differently_salted.body).unwrap();
    assert_ne!(populated["bitsBase64"], differently_salted["bitsBase64"]);

    let oversized = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"expectedItems":1000000,"falsePositiveRate":1e-300,"saltId":"audit-oversized"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(oversized.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_membership_removal_requires_moderator_or_self() {
    // The acting peer for every podcore mutation is this instance's own
    // configured Soulseek identity (`pod_request_peer_id`), never a
    // client-supplied parameter -- so the fixture's local identity is
    // what stands in for "the caller" throughout this test.
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "operator-peer")
            .with("SLSK_PASSWORD", "operator-secret"),
    );

    let ordinary_pod = "pod:00000000000000000000000000000098";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": ordinary_pod,
                "name": "Membership Removal Audit (ordinary)",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");
    state
        .pods
        .write()
        .await
        .upsert_member(
            ordinary_pod,
            super::pods::PodMember {
                peer_id: "operator-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as an ordinary member");
    state
        .pods
        .write()
        .await
        .upsert_member(
            ordinary_pod,
            super::pods::PodMember {
                peer_id: "target-member".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add target member");

    // An ordinary (non-moderator) local peer may not remove another
    // member.
    let denied = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/membership/{ordinary_pod}/target-member"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(denied.status, "403 Forbidden");

    // The same ordinary local peer may remove their own membership.
    let self_removed = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/membership/{ordinary_pod}/operator-peer"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(self_removed.status, "200 OK", "{}", self_removed.body);

    let owned_pod = "pod:00000000000000000000000000000099";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": owned_pod,
                "name": "Membership Removal Audit (owned)",
            }))
            .expect("deserialize pod record fixture"),
            "operator-peer".to_owned(),
        )
        .expect("create pod owned by the local peer");
    state
        .pods
        .write()
        .await
        .upsert_member(
            owned_pod,
            super::pods::PodMember {
                peer_id: "target-member".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add target member to owned pod");

    // The pod owner (a moderator) may remove someone else's membership.
    let removed = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/membership/{owned_pod}/target-member"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(removed.status, "200 OK", "{}", removed.body);
}

async fn pod_fixture_with_local_role(
    local_peer: &str,
    creator: &str,
    pod_id: &str,
) -> (Arc<super::AppState>, mpsc::Receiver<super::SessionCommand>) {
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
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Authorization Audit",
            }))
            .expect("deserialize pod record fixture"),
            creator.to_owned(),
        )
        .expect("create pod");
    (state, receiver)
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_ban_unban_role_require_moderator() {
    let pod_id = "pod:0000000000000000000000000000ba01";
    let (state, _receiver) =
        pod_fixture_with_local_role("ordinary-peer", "owner-peer", pod_id).await;
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "ordinary-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as ordinary member");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "target-member".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add target member");

    for action in ["ban", "unban", "role"] {
        let body = if action == "role" {
            r#"{"role":"mod"}"#
        } else {
            ""
        };
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/membership/{pod_id}/target-member/{action}"),
            None,
            body,
            &state,
        )
        .await
        .unwrap();
        assert_eq!(
            response.status, "403 Forbidden",
            "non-moderator {action} must be denied"
        );
    }

    // The pod owner (a moderator) may perform all three actions.
    let owned_pod = "pod:0000000000000000000000000000ba02";
    let (state, _receiver) =
        pod_fixture_with_local_role("owner-peer", "owner-peer", owned_pod).await;
    for peer_id in ["target-member", "target-member-2"] {
        state
            .pods
            .write()
            .await
            .upsert_member(
                owned_pod,
                super::pods::PodMember {
                    peer_id: peer_id.to_owned(),
                    role: "member".to_owned(),
                    is_banned: false,
                    public_key: None,
                    joined_at: None,
                    last_seen: None,
                },
            )
            .expect("add target member");
    }
    let ban = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{owned_pod}/target-member/ban"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(ban.status, "200 OK", "{}", ban.body);
    // A banned member is no longer visible to member-scoped lookups
    // (matching the oracle), so exercise unban/role on a fresh member.
    let unban = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{owned_pod}/target-member/unban"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unban.status, "200 OK", "{}", unban.body);
    let role = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{owned_pod}/target-member-2/role"),
        None,
        r#"{"role":"mod"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(role.status, "200 OK", "{}", role.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_membership_add_requires_self() {
    let pod_id = "pod:0000000000000000000000000000ba03";
    let (state, _receiver) = pod_fixture_with_local_role("local-peer", "owner-peer", pod_id).await;

    // The local peer may not publish membership claiming to be someone
    // else, nor self-assign a moderator role or clear a ban via the
    // request body.
    let impersonation = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"someone-else","role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(impersonation.status, "403 Forbidden");

    let escalation = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"local-peer","role":"mod","isBanned":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(escalation.status, "200 OK", "{}", escalation.body);
    let escalation = serde_json::from_str::<serde_json::Value>(&escalation.body).unwrap();
    assert_eq!(escalation["success"], true);
    assert_eq!(escalation["podId"], pod_id);
    assert_eq!(escalation["peerId"], "local-peer");
    assert!(
        state
            .pods
            .read()
            .await
            .member_for_verification(pod_id, "local-peer")
            .is_some_and(|member| member.role == "member"),
        "self-publish must not grant a role from the request body"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_membership_update_requires_moderator_or_self_and_pins_role() {
    let pod_id = "pod:0000000000000000000000000000ba04";
    let (state, _receiver) = pod_fixture_with_local_role("local-peer", "owner-peer", pod_id).await;
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "local-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as ordinary member");
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "target-member".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add target member");

    // A non-moderator, non-self update must be denied.
    let denied = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/membership/{pod_id}/members/target-member"),
        None,
        r#"{"role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(denied.status, "403 Forbidden");

    // A self-update is allowed, but role/ban escalation attempts in the
    // body are ignored and pinned back to the existing record.
    let self_update = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/membership/{pod_id}/members/local-peer"),
        None,
        r#"{"role":"mod","isBanned":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(self_update.status, "200 OK", "{}", self_update.body);
    let self_update = serde_json::from_str::<serde_json::Value>(&self_update.body).unwrap();
    assert_eq!(self_update["success"], true);
    assert_eq!(self_update["podId"], pod_id);
    assert_eq!(self_update["peerId"], "local-peer");
    assert!(
        state
            .pods
            .read()
            .await
            .member_for_verification(pod_id, "local-peer")
            .is_some_and(|member| member.role == "member"),
        "self-update must not grant a role from the request body"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn pod_channel_mutations_require_moderator() {
    let pod_id = "pod:0000000000000000000000000000ba05";
    let (state, _receiver) =
        pod_fixture_with_local_role("ordinary-peer", "owner-peer", pod_id).await;
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            super::pods::PodMember {
                peer_id: "ordinary-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as ordinary member");

    let denied_create = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/channels"),
        None,
        r#"{"name":"general"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(denied_create.status, "403 Forbidden");

    let owned_pod = "pod:0000000000000000000000000000ba06";
    let (state, _receiver) =
        pod_fixture_with_local_role("owner-peer", "owner-peer", owned_pod).await;
    let created = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{owned_pod}/channels"),
        None,
        r#"{"channelId":"extra","name":"Extra"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let created = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    let channel_id = created["channelId"].as_str().unwrap().to_owned();

    let updated = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{owned_pod}/channels/{channel_id}"),
        None,
        r#"{"name":"Extra renamed"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "200 OK", "{}", updated.body);

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/{owned_pod}/channels/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK", "{}", deleted.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn deterministic_openapi_mutations_match_native_status_and_dto_contracts() {
    let (state, _receiver) = test_state();

    for (path, enabled) in [
        ("/api/v0/autoreplace/enable", true),
        ("/api/v0/autoreplace/disable", false),
    ] {
        let response = super::route_http_request("PUT", path, None, "", &state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        assert_eq!(value["enabled"], enabled);
        for key in [
            "lastRunAt",
            "lastRunProcessedCount",
            "lastRunReplacedCount",
            "intervalSeconds",
        ] {
            assert!(value.get(key).is_some(), "{path}: missing {key}");
        }
    }

    let destination = super::route_http_request(
        "POST",
        "/api/v0/destinations/validate",
        None,
        r#"{"path":"/tmp/slskdn-route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    let destination = serde_json::from_str::<serde_json::Value>(&destination.body).unwrap();
    assert_eq!(destination["path"], "/tmp/slskdn-route-audit");
    assert!(destination.get("exists").is_some());
    assert!(destination.get("writable").is_some());

    let announce = super::route_http_request("POST", "/api/v0/dht/announce", None, "", &state)
        .await
        .unwrap();
    assert_eq!(announce.status, "400 Bad Request");
    assert_eq!(announce.body, r#"{"error":"Not beacon capable"}"#);
    let discover = super::route_http_request("POST", "/api/v0/dht/discover", None, "", &state)
        .await
        .unwrap();
    let discover = serde_json::from_str::<serde_json::Value>(&discover.body).unwrap();
    assert!(discover.get("newConnectionsMade").is_some());
    assert!(discover.get("totalMeshConnections").is_some());

    for (path, message) in [
        (
            "/api/v0/hashdb/optimize/indexes",
            "Index optimization completed",
        ),
        (
            "/api/v0/hashdb/optimize/vacuum",
            "VACUUM and ANALYZE completed",
        ),
    ] {
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap()["message"],
            message
        );
    }
    let profile = super::route_http_request(
        "POST",
        "/api/v0/hashdb/optimize/profile",
        None,
        r#"{"query":"route-audit","parameters":{}}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(profile.status, "400 Bad Request");

    for (method, path) in [
        ("DELETE", "/api/v0/nowplaying"),
        ("DELETE", "/api/v0/integrations/spotify"),
        ("DELETE", "/api/v0/transfers/downloads/all/completed"),
        ("DELETE", "/api/v0/transfers/uploads/all/completed"),
        (
            "PATCH",
            "/api/v0/library/health/issues/00000000-0000-4000-8000-000000000001",
        ),
    ] {
        let response =
            super::route_http_request(method, path, None, r#"{"status":"Resolved"}"#, &state)
                .await
                .unwrap();
        assert_eq!(response.status, "204 No Content", "{method} {path}");
        assert!(response.body.is_empty(), "{method} {path}");
    }

    let blocked = super::route_http_request(
        "POST",
        "/api/v0/overlay/blocklist/username",
        None,
        r#"{"username":"route-audit-peer"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(blocked.status, "200 OK");
    let unblocked = super::route_http_request(
        "DELETE",
        "/api/v0/overlay/blocklist/username/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unblocked.body, r#"{"message":"Blocklist entry removed"}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_openapi_validation_and_large_dtos_match_native_contracts() {
    let (state, _receiver) = test_state();

    for (method, path, body, expected) in [
        (
            "POST",
            "/api/v0/searches",
            r#"{"searchText":"route-audit","acquisitionProfile":"route-audit"}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/mesh/sync/route-audit-peer",
            "{}",
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/mesh/message",
            "",
            "415 Unsupported Media Type",
        ),
        (
            "POST",
            "/api/v0/multisource/download",
            r#"{"filename":"Route Audit.flac","fileSize":1,"sources":[]}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/targets",
            r#"{"releaseId":"00000000-0000-4000-8000-000000000001"}"#,
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits",
            r#"{"id":"00000000-0000-4000-8000-000000000001","evidence":[]}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/00000000-0000-4000-8000-000000000001/approve-export",
            "{}",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/musicbrainz/overlays/edits/00000000-0000-4000-8000-000000000001/routes",
            "{}",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/share-grants",
            r#"{"collectionId":"00000000-0000-4000-8000-000000000001"}"#,
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/users/route-audit-peer/directory",
            r#"{"directory":"/tmp/slskdn-route-audit"}"#,
            "503 Service Unavailable",
        ),
        (
            "PUT",
            "/api/v0/conversations/route-audit-peer/1",
            "",
            "503 Service Unavailable",
        ),
        (
            "PUT",
            "/api/v0/conversations/route-audit-peer",
            "",
            "503 Service Unavailable",
        ),
        (
            "DELETE",
            "/api/v0/conversations/route-audit-peer",
            "",
            "404 Not Found",
        ),
        (
            "POST",
            "/api/v0/conversations/route-audit-peer",
            r#""route audit""#,
            "503 Service Unavailable",
        ),
        (
            "POST",
            "/api/v0/rooms/joined",
            r#""route-audit""#,
            "503 Service Unavailable",
        ),
        (
            "POST",
            "/api/v0/session",
            r#"{"username":"route-audit-peer","password":"route-audit"}"#,
            "401 Unauthorized",
        ),
        (
            "POST",
            "/api/v0/pods/pod%3A00000000000000000000000000000001/channels/general/bind",
            r#"{"roomName":"Route Audit","mode":"route-audit"}"#,
            "400 Bad Request",
        ),
        (
            "POST",
            "/api/v0/pods/pod%3A00000000000000000000000000000001/channels/general/unbind",
            "",
            "404 Not Found",
        ),
        (
            "PATCH",
            "/api/v0/options",
            "{}",
            "403 Forbidden",
        ),
        (
            "PUT",
            "/api/v0/options/yaml",
            r#""app: {}""#,
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/options/yaml/validate",
            r#""app: {}""#,
            "403 Forbidden",
        ),
        (
            "PUT",
            "/api/v0/relay/agent",
            "",
            "403 Forbidden",
        ),
        (
            "DELETE",
            "/api/v0/relay/agent",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/relay/controller/files/route-audit",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/relay/controller/shares/route-audit",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/soulseek/mesh-rendezvous/interest",
            "",
            "403 Forbidden",
        ),
        (
            "DELETE",
            "/api/v0/soulseek/mesh-rendezvous/interest",
            "",
            "403 Forbidden",
        ),
        (
            "POST",
            "/api/v0/streams/content%3Amusic%3Arecording%3Amissing/ticket",
            "{}",
            "404 Not Found",
        ),
    ] {
        let response = super::route_http_request(method, path, None, body, &state)
            .await
            .unwrap();
        assert_eq!(response.status, expected, "{method} {path}: {}", response.body);
    }

    let multisource_test = super::route_http_request(
        "POST",
        "/api/v0/multisource/test",
        None,
        r#"{"searchText":"route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(multisource_test.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&multisource_test.body).unwrap()["searchText"],
        "route-audit"
    );
    let multisource_test =
        serde_json::from_str::<serde_json::Value>(&multisource_test.body).unwrap();
    for key in [
        "downloadSuccess",
        "downloadTimeMs",
        "bytesDownloaded",
        "sourcesUsed",
        "outputPath",
        "finalHash",
        "averageSpeedMBps",
    ] {
        assert!(
            multisource_test.get(key).is_some(),
            "multisource test missing {key}"
        );
    }

    let bloom = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/library-bloom/snapshots/preview",
        None,
        r#"{"expectedItems":1,"falsePositiveRate":1,"saltId":"audit","rotatesAt":"2026-01-01T00:00:00Z"}"#,
        &state,
    )
    .await
    .unwrap();
    let bloom = serde_json::from_str::<serde_json::Value>(&bloom.body).unwrap();
    for key in [
        "snapshotId",
        "scope",
        "saltId",
        "createdAt",
        "rotatesAt",
        "expectedItems",
        "falsePositiveRate",
        "bitSize",
        "hashFunctionCount",
        "itemCount",
        "fillRatio",
        "bitsBase64",
        "namespaceItemCounts",
        "privacyNotes",
    ] {
        assert!(bloom.get(key).is_some(), "Bloom preview missing {key}");
    }

    let songid = super::route_http_request(
        "POST",
        "/api/v0/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    let songid = serde_json::from_str::<serde_json::Value>(&songid.body).unwrap();
    assert_eq!(songid["source"], "route-audit");
    for key in [
        "sourceType",
        "query",
        "createdAt",
        "summary",
        "currentStage",
        "percentComplete",
        "artifactDirectory",
        "evidence",
        "tracks",
        "albums",
        "artists",
        "plans",
        "options",
        "scorecard",
        "assessment",
        "metadata",
        "provenance",
        "perturbations",
        "stems",
        "corpusMatches",
        "clips",
        "transcripts",
        "ocr",
        "comments",
        "chapters",
        "segments",
        "mixGroups",
        "identityAssessment",
        "syntheticAssessment",
    ] {
        assert!(songid.get(key).is_some(), "SongID run missing {key}");
    }

    let taste = super::route_http_request(
        "POST",
        "/api/v0/taste-recommendations",
        None,
        r#"{"minimumTrustedSources":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(taste.status, "200 OK");
    let taste = serde_json::from_str::<serde_json::Value>(&taste.body).unwrap();
    for key in [
        "minimumTrustedSources",
        "trustedActorCount",
        "candidateCount",
        "recommendations",
    ] {
        assert!(taste.get(key).is_some(), "taste result missing {key}");
    }
    let invalid_work_ref =
        r#"{"workRef":{"@context":null,"domain":"music","title":"Route Audit"}}"#;
    for path in [
        "/api/v0/taste-recommendations/wishlist",
        "/api/v0/taste-recommendations/release-radar",
        "/api/v0/taste-recommendations/graph-preview",
    ] {
        let response = super::route_http_request("POST", path, None, invalid_work_ref, &state)
            .await
            .unwrap();
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }

    let start = super::route_http_request(
        "POST",
        "/api/v0/portforwarding/start",
        None,
        r#"{"localPort":1024,"podId":"pod:a","destinationHost":"example.invalid","destinationPort":1}"#,
        &state,
    )
    .await
    .unwrap();
    // The real handler (previously shadowed by a fake-success stub due
    // to a routing-table typo mapping this oracle-spelled path to the
    // wrong internal route) requires real Pod membership before
    // starting a forward -- "pod:a" was never created, so this must
    // fail closed, not return a canned success message.
    assert_eq!(start.status, "403 Forbidden", "{}", start.body);
    let stop = super::route_http_request("POST", "/api/v0/portforwarding/stop/1", None, "", &state)
        .await
        .unwrap();
    assert_eq!(stop.body, r#"{"message":"Port forwarding stopped"}"#);

    let index_sync = super::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        r#"{"records":[{"recordingId":"route-audit","peerIds":["peer-a"],"updatedAt":1}],"realmIndexes":[{"id":"index","realmId":"default-realm","subjectNamespace":"music","revision":1,"entries":[{"subjectId":"route-audit","workRef":{"domain":"music","title":"Route Audit","externalIds":{"musicbrainz:recording":"route-audit"}},"externalIds":{},"aliases":[]}],"signature":{"signer":"default-governance","value":"signature","payloadHash":"a890273abd9ae483659d6b08c9bc83dd82cda93bdefc9c940412c91f2ccddcc6"}}]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(index_sync.status, "200 OK", "{}", index_sync.body);

    // An unsafe decidedBy identifier (a local file path) must be
    // rejected by real validation, matching the oracle's
    // IsSafeOpaqueReference check.
    let unsafe_decision = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index/authority-decision",
        None,
        r#"{"enabled":true,"decidedBy":"/etc/passwd","note":"route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unsafe_decision.status, "400 Bad Request");
    let unsafe_decision = serde_json::from_str::<serde_json::Value>(&unsafe_decision.body).unwrap();
    assert_eq!(unsafe_decision["isAccepted"], false);
    assert!(unsafe_decision["errors"][0]
        .as_str()
        .unwrap()
        .contains("opaque and safe"));

    // A safe, well-formed decision must be genuinely accepted -- not
    // an unconditional 400 (versioned) or unconditional 200
    // (unversioned) regardless of input, as the old fake handlers did.
    let decision = super::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index/authority-decision",
        None,
        r#"{"enabled":true,"decidedBy":"route-audit","note":"route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(decision.status, "200 OK", "{}", decision.body);
    let decision = serde_json::from_str::<serde_json::Value>(&decision.body).unwrap();
    assert_eq!(decision["isAccepted"], true);
    assert_eq!(decision["enabled"], true);
    assert_eq!(decision["errors"], serde_json::json!([]));

    let pod_id = "pod:00000000000000000000000000000001";
    let created = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:00000000000000000000000000000001","name":"Route Audit","visibility":"Listed","contentId":"content:music:recording:route-audit","tags":[],"channels":[],"externalBindings":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let joined = super::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:00000000000000000000000000000001","peerId":"00000000-0000-4000-8000-000000000001","requestedRole":"route-audit","publicKey":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","message":"route audit","nonce":"route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(joined.status, "200 OK", "{}", joined.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&joined.body).unwrap()["podId"],
        pod_id
    );

    let empty_backfill = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/backfill/{pod_id}/sync"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(empty_backfill.status, "400 Bad Request");
    let last_seen = super::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/backfill/{pod_id}/general/last-seen"),
        None,
        "1",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(last_seen.status, "200 OK", "{}", last_seen.body);
    assert!(last_seen.body.is_empty());

    let verified = super::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        r#"{"messageId":"message","podId":"pod:00000000000000000000000000000001","channelId":"00000000-0000-4000-8000-000000000001","senderPeerId":"00000000-0000-4000-8000-000000000001","body":"route audit","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","sigVersion":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(verified.body, r#"{"isValid":true}"#);
    let evidence = super::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        r#"{"messageId":"message","podId":"pod:00000000000000000000000000000001","channelId":"00000000-0000-4000-8000-000000000001","senderPeerId":"00000000-0000-4000-8000-000000000001","body":"route audit","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","sigVersion":1}"#,
        &state,
    )
    .await
    .unwrap();
    let evidence = serde_json::from_str::<serde_json::Value>(&evidence.body).unwrap();
    for key in [
        "isValid",
        "isFromValidMember",
        "hasValidSignature",
        "isNotBanned",
        "errorMessage",
    ] {
        assert!(evidence.get(key).is_some(), "verification missing {key}");
    }
    let opinion = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/{pod_id}/opinions"),
        None,
        r#"{"contentId":"content:music:recording:route-audit","score":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(opinion.status, "400 Bad Request");

    // This state's local peer (the pod's creator/owner, since no
    // requestingPeerId was given to create-pod above) can moderate the
    // pod, so it may remove another member's membership. The deny/self
    // paths for a non-moderating actor are covered in the dedicated
    // `pod_membership_removal_requires_moderator_or_self` test, which
    // configures a distinct local identity for that purpose.
    let removed = super::route_http_request(
        "DELETE",
        &format!("/api/v0/podcore/membership/{pod_id}/00000000-0000-4000-8000-000000000001"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(removed.status, "200 OK");
    let removed = serde_json::from_str::<serde_json::Value>(&removed.body).unwrap();
    assert_eq!(removed["success"], true);
    assert!(removed.get("dhtKey").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_auxiliary_mutations_match_native_status_and_dto_contracts() {
    let (state, _receiver) = test_state();

    let unversioned_warm_cache = super::route_http_request(
        "POST",
        "/api/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[],"mb_artist_ids":[],"mb_label_ids":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unversioned_warm_cache.status, "400 Bad Request");
    assert_eq!(
        unversioned_warm_cache.body,
        r#"{"error":"Warm cache not enabled"}"#
    );
    let versioned_warm_cache = super::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[],"mb_artist_ids":[],"mb_label_ids":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(versioned_warm_cache.status, "400 Bad Request");
    assert_eq!(
        versioned_warm_cache.body,
        r#"{"error":"Warm cache not enabled"}"#
    );

    let event = super::route_http_request(
        "POST",
        "/api/v0/events/route-audit",
        None,
        r#""route-audit""#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(event.status, "400 Bad Request");
    assert_eq!(event.body, r#""Unknown event type""#);

    let shares = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .unwrap();
    assert_eq!(shares.status, "404 Not Found");
    let scan = super::route_http_request("PUT", "/api/v0/shares", None, "", &state)
        .await
        .unwrap();
    assert_eq!(scan.status, "200 OK");
    let cancelled = super::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .unwrap();
    assert_eq!(cancelled.status, "404 Not Found");

    let invite = super::route_http_request(
        "POST",
        "/api/v0/profile/invite",
        None,
        r#"{"expiresInHours":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invite.status, "200 OK", "{}", invite.body);
    let invite = serde_json::from_str::<serde_json::Value>(&invite.body).unwrap();
    assert!(invite["inviteLink"]
        .as_str()
        .unwrap()
        .starts_with("slskdn://invite/"));
    assert_eq!(
        invite["friendCode"]
            .as_str()
            .unwrap()
            .split('-')
            .map(str::len)
            .collect::<Vec<_>>(),
        vec![5, 4, 4, 3]
    );

    let csv_body = r#"{"csvText":"Artist,Track Title,Album\nRoute Audit Auxiliary,Parity Track,Contract Album","filter":"route-audit-auxiliary","enabled":true,"autoDownload":true,"maxResults":1,"includeAlbum":true}"#;
    let imported = super::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        csv_body,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(imported.status, "200 OK", "{}", imported.body);
    let imported = serde_json::from_str::<serde_json::Value>(&imported.body).unwrap();
    assert_eq!(imported["totalRows"], 1);
    assert_eq!(imported["createdCount"], 1);
    assert_eq!(imported["duplicateCount"], 0);
    assert_eq!(imported["skippedCount"], 0);
    assert_eq!(
        imported["createdItems"][0]["searchText"],
        "Route Audit Auxiliary Contract Album"
    );
    assert_eq!(imported["createdItems"][0]["maxResults"], 1);
    assert!(imported["createdItems"][0].get("artist").is_none());

    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/wishlist/import/csv",
        None,
        csv_body,
        &state,
    )
    .await
    .unwrap();
    let duplicate = serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap();
    assert_eq!(duplicate["createdCount"], 0);
    assert_eq!(duplicate["duplicateCount"], 1);

    let invalid_content = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/validate",
        None,
        r#""route-audit""#,
        &state,
    )
    .await
    .unwrap();
    let invalid_content = serde_json::from_str::<serde_json::Value>(&invalid_content.body).unwrap();
    assert_eq!(invalid_content["isValid"], false);
    assert_eq!(invalid_content["contentId"], "route-audit");
    assert_eq!(
        invalid_content["errorMessage"],
        "Invalid content ID format. Expected: content:<domain>:<type>:<id>"
    );
    let valid_content = super::route_http_request(
        "POST",
        "/api/v0/podcore/content/validate",
        None,
        r#""content:music:recording:test""#,
        &state,
    )
    .await
    .unwrap();
    let valid_content = serde_json::from_str::<serde_json::Value>(&valid_content.body).unwrap();
    assert_eq!(valid_content["isValid"], true);
    assert_eq!(valid_content["metadata"]["domain"], "music");
    assert_eq!(valid_content["metadata"]["type"], "recording");

    let group = super::route_http_request(
        "POST",
        "/api/v0/sharegroups",
        None,
        r#"{"name":"Route Audit Group"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(group.status, "201 Created");
    let group = serde_json::from_str::<serde_json::Value>(&group.body).unwrap();
    assert!(uuid::Uuid::parse_str(group["id"].as_str().unwrap()).is_ok());
    assert_eq!(group["name"], "Route Audit Group");
    assert_eq!(group["ownerUserId"], "Anonymous");
    assert!(group["createdAt"].as_str().is_some());
    assert!(group.get("members").is_none());

    let profile = super::route_http_request(
        "PUT",
        "/api/v0/profile/me",
        None,
        r#"{"displayName":"Route Audit","avatar":"route-audit","capabilities":1,"endpoints":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(profile.status, "200 OK", "{}", profile.body);
    let profile = serde_json::from_str::<serde_json::Value>(&profile.body).unwrap();
    for key in [
        "peerId",
        "publicKey",
        "displayName",
        "avatar",
        "capabilities",
        "endpoints",
        "createdAt",
        "expiresAt",
        "signature",
    ] {
        assert!(profile.get(key).is_some(), "missing profile key {key}");
    }
    assert_eq!(profile["displayName"], "Route Audit");
    assert_eq!(profile["capabilities"], 1);

    let verdict = super::route_http_request(
        "POST",
        "/api/v0/quarantine-jury/verdicts",
        None,
        r#"{"requestId":"00000000-0000-4000-8000-000000000001","juror":"route-audit","verdict":"NeedsManualReview"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(verdict.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&verdict.body).unwrap(),
        serde_json::json!({"isValid": false, "errors": ["Request not found."]})
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn enabled_warm_cache_hints_normalize_persist_and_bound_popularity() {
    let (state, _receiver) = test_state();
    std::fs::write(
        state.config.state_dir.join("slskd.yml"),
        "warmCache:\n  enabled: true\n",
    )
    .unwrap();

    let accepted = super::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[" rel-1 ","REL-1"],"mb_artist_ids":["artist-1"],"mb_label_ids":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(accepted.status, "200 OK", "{}", accepted.body);
    assert_eq!(accepted.body, r#"{"accepted":true}"#);

    let features = state.controller_features.read().await;
    assert_eq!(
        features
            .get("warm-cache/popularity/mb:release:rel-1")
            .unwrap()["hits"],
        1
    );
    assert_eq!(
        features
            .get("warm-cache/popularity/mb:artist:artist-1")
            .unwrap()["hits"],
        1
    );
    drop(features);

    let invalid = super::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        r#"{"mb_release_ids":[42]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");

    let oversized = super::route_http_request(
        "POST",
        "/api/v0/slskdn/warm-cache/hints",
        None,
        &serde_json::json!({"mb_release_ids": ["x".repeat(129)]}).to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(oversized.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_discovery_graph_and_opinions_match_native_contracts() {
    let (state, _receiver) = test_state();
    let graph = super::route_http_request(
        "POST",
        "/api/v0/discovery-graph",
        None,
        r#"{"scope":"route-audit","songIdRunId":"00000000-0000-4000-8000-000000000001","recordingId":"00000000-0000-4000-8000-000000000001","releaseId":"00000000-0000-4000-8000-000000000001","artistId":"00000000-0000-4000-8000-000000000001","title":"Route Audit","artist":"route-audit","album":"route-audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(graph.status, "200 OK", "{}", graph.body);
    let graph = serde_json::from_str::<serde_json::Value>(&graph.body).unwrap();
    assert_eq!(graph["title"], "Route Audit");
    assert_eq!(graph["seedNodeId"], "seed:route-audit");
    assert_eq!(graph["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(graph["edges"].as_array().unwrap().len(), 3);
    assert_eq!(graph["evidenceSummary"].as_array().unwrap().len(), 3);
    assert_eq!(graph["request"]["scope"], "route-audit");

    let invalid_opinion = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"id":"00000000-0000-4000-8000-000000000001","issuer":"route-audit","subjectType":"Unknown","subjectId":"subject","kind":"Unknown","strength":1,"confidence":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid_opinion.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&invalid_opinion.body).unwrap(),
        serde_json::json!(["subject type is required", "opinion kind is required"])
    );
    let missing_delete = super::route_http_request(
        "DELETE",
        "/api/v0/opinions/00000000-0000-4000-8000-000000000001",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_delete.status, "404 Not Found");

    let valid_opinion = super::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"route-audit","subjectType":"Track","subjectId":"track-1","kind":"Like","strength":1,"confidence":1,"scope":"global","source":"local","evidence":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(valid_opinion.status, "200 OK", "{}", valid_opinion.body);
    let valid_opinion = serde_json::from_str::<serde_json::Value>(&valid_opinion.body).unwrap();
    let opinion_id = valid_opinion["id"].as_str().unwrap();
    assert!(valid_opinion["updatedUnixMs"].as_i64().unwrap() > 0);
    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/v0/opinions/{opinion_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "204 No Content");

    let contact = super::route_http_request(
        "POST",
        "/api/v0/contacts/from-discovery",
        None,
        r#"{"peerId":"00000000-0000-4000-8000-000000000001","nickname":"Route Audit"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(contact.status, "404 Not Found");
    assert_eq!(contact.body, r#""Profile not found.""#);

    for action in ["download", "stream"] {
        let response = super::route_http_request(
            "POST",
            &format!(
                "/api/v0/searches/00000000-0000-4000-8000-000000000001/items/00000000-0000-4000-8000-000000000001/{action}"
            ),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "404 Not Found");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap(),
            serde_json::json!({
                "type": "search_not_found",
                "title": "Search not found",
                "status": 404,
                "detail": "Search not found",
            })
        );
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_discovery_graph_reads_songid_store_and_musicbrainz_release_graph() {
    let (state, _receiver) = test_state();
    {
        let mut library = state.library.write().await;
        library
            .create(
                "artist-1".to_owned(),
                "Release One".to_owned(),
                "Audio".to_owned(),
            )
            .expect("seed release-graph library record");
    }
    {
        let mut runtime = state.runtime.write().await;
        runtime.songid_runs = 17;
        runtime.songid_run_records.push(serde_json::json!({
            "id": "songid-17",
            "status": "completed",
            "query": "Route Audit",
            "metadata": {
                "title": "Route Audit",
                "artist": "Artist One",
                "album": "Release One"
            },
            "identityAssessment": {
                "verdict": "recognized_cataloged_track",
                "confidence": 0.92
            },
            "tracks": [{
                "candidateId": "track-1",
                "recordingId": "recording-1",
                "title": "Route Audit",
                "artist": "Artist One",
                "musicBrainzArtistId": "artist-1",
                "isExact": true,
                "identityScore": 0.92,
                "byzantineScore": 0.81,
                "actionScore": 0.88
            }],
            "albums": [{
                "candidateId": "album-1",
                "releaseId": "release-1",
                "title": "Release One",
                "artist": "Artist One",
                "musicBrainzArtistId": "artist-1",
                "identityScore": 0.86,
                "byzantineScore": 0.78,
                "actionScore": 0.82
            }],
            "artists": [{
                "candidateId": "artist-1",
                "artistId": "artist-1",
                "name": "Artist One",
                "identityScore": 0.84,
                "byzantineScore": 0.77,
                "actionScore": 0.81
            }],
            "segments": [{
                "segmentId": "segment-1",
                "label": "Opening",
                "decompositionLabel": "chapter",
                "confidence": 0.71,
                "candidates": [{
                    "candidateId": "segment-track-1",
                    "recordingId": "recording-2",
                    "title": "Adjacent Track",
                    "artist": "Artist Two",
                    "identityScore": 0.74,
                    "byzantineScore": 0.68,
                    "actionScore": 0.72
                }]
            }],
            "mixGroups": []
        }));
    }

    let run_graph = super::route_http_request(
        "POST",
        "/api/v0/discovery-graph",
        None,
        r#"{"scope":"songid_run","songIdRunId":"songid-17"}"#,
        &state,
    )
    .await
    .expect("build SongID-backed graph");
    assert_eq!(run_graph.status, "200 OK", "{}", run_graph.body);
    let run_graph = serde_json::from_str::<serde_json::Value>(&run_graph.body).unwrap();
    assert_eq!(run_graph["seedNodeId"], "songid:songid-17");
    assert!(run_graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["nodeId"] == "track:recording-1"));
    assert!(run_graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .all(|edge| edge["provenance"] != "fallback_request"));

    let artist_graph = super::route_http_request(
        "POST",
        "/api/v0/discovery-graph",
        None,
        r#"{"scope":"artist","songIdRunId":"songid-17","artistId":"artist-1","artist":"Artist One"}"#,
        &state,
    )
    .await
    .expect("build MusicBrainz-backed artist graph");
    assert_eq!(artist_graph.status, "200 OK", "{}", artist_graph.body);
    let artist_graph = serde_json::from_str::<serde_json::Value>(&artist_graph.body).unwrap();
    assert_eq!(artist_graph["seedNodeId"], "artist:artist-1");
    assert!(artist_graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["nodeId"] == "release-group:lib-1"));
    assert!(artist_graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["provenance"] == "musicbrainz_release_graph"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_release_radar_matches_native_state_and_result_contracts() {
    let (state, _receiver) = test_state();
    let artist_id = "00000000-0000-4000-8000-000000000101";
    let subscription = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        &format!(
            r#"{{"artistId":"{artist_id}","artistName":"Parity Artist","scope":"trusted","enabled":true,"mutedReleaseGroupIds":[],"createdAt":"2026-01-01T00:00:00Z"}}"#
        ),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(subscription.status, "200 OK");
    let subscription = serde_json::from_str::<serde_json::Value>(&subscription.body).unwrap();
    assert_eq!(subscription["id"], format!("artist-radar:{artist_id}"));
    assert_eq!(subscription["artistName"], "Parity Artist");
    assert_eq!(subscription["createdAt"], "2026-01-01T00:00:00+00:00");

    let subscriptions = super::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let subscriptions = serde_json::from_str::<serde_json::Value>(&subscriptions.body).unwrap();
    assert_eq!(subscriptions, serde_json::json!([subscription]));

    let rejected = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(rejected.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rejected.body).unwrap(),
        serde_json::json!({
            "accepted": false,
            "notifications": [],
            "rejectionReason": "Observation is not SongID-confirmed.",
        })
    );

    let observation_body = format!(
        r#"{{"artistId":"{artist_id}","recordingId":"00000000-0000-4000-8000-000000000102","releaseId":"00000000-0000-4000-8000-000000000103","releaseGroupId":"00000000-0000-4000-8000-000000000104","sourceRealm":"realm","sourceActor":"actor","songIdConfirmed":true,"confidence":1,"workRef":{{"id":"00000000-0000-4000-8000-000000000102","type":"recording","domain":"music","externalIds":{{}},"title":"Track","creator":"Artist","year":2026,"metadata":{{}},"attributedTo":"actor","published":"2026-01-01T00:00:00Z"}},"observedAt":"2026-01-01T00:00:00Z"}}"#
    );
    let observation = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        &observation_body,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(observation.status, "200 OK");
    let observation = serde_json::from_str::<serde_json::Value>(&observation.body).unwrap();
    assert_eq!(observation["accepted"], true);
    let notification = &observation["notifications"][0];
    assert_eq!(
        notification["subscriptionId"],
        format!("artist-radar:{artist_id}")
    );
    assert_eq!(notification["artistId"], artist_id);
    assert_eq!(notification["firstSeenAt"], "2026-01-01T00:00:00+00:00");
    assert_eq!(notification["read"], false);
    assert_eq!(
        notification["workRef"]["@context"],
        serde_json::json!([
            "https://www.w3.org/ns/activitystreams",
            "https://w3id.org/federation/workref#"
        ])
    );

    let duplicate = super::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        &observation_body,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap(),
        serde_json::json!({"accepted": true, "notifications": []})
    );

    let notification_id = notification["id"].as_str().unwrap();
    let route = super::route_http_request(
        "POST",
        &format!("/api/v0/musicbrainz/release-radar/notifications/{notification_id}/routes"),
        None,
        r#"{"targetPeerIds":[],"podId":"pod","channelId":"channel","senderPeerId":"sender"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(route.status, "400 Bad Request");
    let route = serde_json::from_str::<serde_json::Value>(&route.body).unwrap();
    assert_eq!(route["notificationId"], notification_id);
    assert_eq!(route["success"], false);
    assert_eq!(
        route["errorMessage"],
        "At least one target peer is required."
    );
    assert_eq!(route["targetPeerIds"], serde_json::json!([]));

    let routes = super::route_http_request(
        "GET",
        &format!("/api/v0/musicbrainz/release-radar/notifications/{notification_id}/routes"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let routes = serde_json::from_str::<serde_json::Value>(&routes.body).unwrap();
    assert_eq!(routes.as_array().unwrap().len(), 1);
    assert_eq!(routes[0]["id"], route["id"]);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn podcore_maintenance_mutations_match_native_result_contracts() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000001";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<super::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Route Audit",
                "visibility": "Listed",
                "isPublic": true,
            }))
            .expect("deserialize maintenance pod fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create maintenance pod");

    for action in ["publish", "update"] {
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/dht/{action}"),
            None,
            &serde_json::json!({"pod": {"podId": pod_id, "name": "Route Audit"}}).to_string(),
            &state,
        )
        .await
        .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        for key in ["success", "podId", "dhtKey", "publishedAt", "expiresAt"] {
            assert!(value.get(key).is_some(), "dht {action}: missing {key}");
        }
        assert_eq!(value["podId"], pod_id);
    }

    for action in ["register", "update"] {
        let response = super::route_http_request(
            "POST",
            &format!("/api/v0/podcore/discovery/{action}"),
            None,
            &serde_json::json!({"podId": pod_id, "name": "Route Audit", "visibility": "Listed"})
                .to_string(),
            &state,
        )
        .await
        .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        for key in [
            "success",
            "podId",
            "discoveryKeys",
            "registeredAt",
            "expiresAt",
        ] {
            assert!(
                value.get(key).is_some(),
                "discovery {action}: missing {key}"
            );
        }
    }

    let refresh = super::route_http_request(
        "POST",
        "/api/v0/podcore/discovery/refresh",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let refresh = serde_json::from_str::<serde_json::Value>(&refresh.body).unwrap();
    for key in ["success", "podId", "wasRepublished", "nextRefresh"] {
        assert!(
            refresh.get(key).is_some(),
            "discovery refresh: missing {key}"
        );
    }

    for (path, fields) in [
        (
            "/api/v0/podcore/membership/cleanup",
            vec!["recordsCleaned", "errorsEncountered", "completedAt"],
        ),
        (
            "/api/v0/podcore/routing/cleanup",
            vec![
                "messagesCleaned",
                "messagesRetained",
                "cleanupDuration",
                "completedAt",
            ],
        ),
    ] {
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        for field in fields {
            assert!(value.get(field).is_some(), "{path}: missing {field}");
        }
    }

    let seen = super::route_http_request(
        "POST",
        &format!("/api/v0/podcore/routing/seen/message-1/{pod_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&seen.body).unwrap()["wasNewlyRegistered"],
        true
    );

    for path in [
        "/api/v0/podcore/messages/rebuild-index",
        "/api/v0/podcore/messages/vacuum",
    ] {
        let response = super::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.body, "true", "{path}");
    }
    let sync_all = super::route_http_request(
        "POST",
        "/api/v0/podcore/backfill/sync-all",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(sync_all.body, "[]");

    for (section, action) in [("dht", "unpublish"), ("discovery", "unregister")] {
        let response = super::route_http_request(
            "DELETE",
            &format!("/api/v0/podcore/{section}/{action}/{pod_id}"),
            None,
            "",
            &state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "200 OK");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap()["success"],
            true
        );
    }
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_user_notes_lifecycle() {
    let (state, _receiver) = test_state();
    let empty_list = super::route_http_request("GET", "/api/v0/users/notes", None, "", &state)
        .await
        .unwrap();
    assert_eq!(empty_list.status, "200 OK");
    let empty_list_json = serde_json::from_str::<serde_json::Value>(&empty_list.body).unwrap();
    assert!(empty_list_json.as_array().is_some_and(Vec::is_empty));

    let malformed_create =
        super::route_http_request("POST", "/api/v0/users/notes", None, "{not-json", &state)
            .await
            .unwrap();
    assert_eq!(malformed_create.status, "400 Bad Request");
    let missing_create =
        super::route_http_request("POST", "/api/v0/users/notes", None, "{}", &state)
            .await
            .unwrap();
    assert_eq!(missing_create.status, "400 Bad Request");

    let malformed_list = super::route_http_request("GET", "/api/v0/users/notes/", None, "", &state)
        .await
        .unwrap();
    assert_eq!(malformed_list.status, "404 Not Found");

    let created = super::route_http_request(
        "POST",
        "/api/v0/users/notes",
        None,
        r#"{"username":"route-audit-peer","note":"contract","color":"red","icon":"star","isHighPriority":true}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(created.status, "200 OK");
    let created = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created["username"], "route-audit-peer");
    assert_eq!(created["color"], "red");
    assert_eq!(created["icon"], "star");
    assert_eq!(created["isHighPriority"], true);
    assert!(created.get("id").is_none());
    assert!(chrono::DateTime::parse_from_rfc3339(created["createdAt"].as_str().unwrap()).is_ok());

    let populated_list = super::route_http_request("GET", "/api/v0/users/notes", None, "", &state)
        .await
        .unwrap();
    assert_eq!(populated_list.status, "200 OK");
    let populated_list_json =
        serde_json::from_str::<serde_json::Value>(&populated_list.body).unwrap();
    assert!(populated_list_json.as_array().is_some_and(|records| {
        records
            .iter()
            .any(|record| record["username"] == "route-audit-peer")
    }));

    let fetched = super::route_http_request(
        "GET",
        "/api/v0/users/notes/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(fetched.status, "200 OK");
    let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
    assert_eq!(fetched_json["note"], "contract");

    let missing_fetch = super::route_http_request(
        "GET",
        "/api/v0/users/notes/no-such-route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_fetch.status, "404 Not Found");
    let malformed_fetch =
        super::route_http_request("GET", "/api/v0/users/notes/", None, "", &state)
            .await
            .unwrap();
    assert_eq!(malformed_fetch.status, "404 Not Found");

    let malformed_delete =
        super::route_http_request("DELETE", "/api/v0/users/notes/", None, "", &state)
            .await
            .unwrap();
    assert_eq!(malformed_delete.status, "404 Not Found");

    let deleted = super::route_http_request(
        "DELETE",
        "/api/v0/users/notes/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "204 No Content");
    let deleted_again = super::route_http_request(
        "DELETE",
        "/api/v0/users/notes/route-audit-peer",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted_again.status, "204 No Content");

    let post_runtime_pass = {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note runtime database");
        let (runtime_state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        db.close_for_test().await;
        let response = super::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-runtime","note":"not persisted"}"#,
            &runtime_state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "503 Service Unavailable");
        assert!(response.body.contains("user note persistence failed"));
        let notes_empty = runtime_state.user_notes.read().await.records.is_empty();
        notes_empty
    };
    assert!(post_runtime_pass);

    let delete_runtime_pass = {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note delete runtime database");
        let (runtime_state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let created = super::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-delete-runtime","note":"original"}"#,
            &runtime_state,
        )
        .await
        .unwrap();
        assert_eq!(created.status, "200 OK");
        db.close_for_test().await;
        let response = super::route_http_request(
            "DELETE",
            "/api/v0/users/notes/route-audit-delete-runtime",
            None,
            "",
            &runtime_state,
        )
        .await
        .unwrap();
        assert_eq!(response.status, "503 Service Unavailable");
        assert!(response
            .body
            .contains("user note deletion persistence failed"));
        let notes = runtime_state.user_notes.read().await;
        notes.records.len() == 1 && notes.records[0].note == "original"
    };
    assert!(delete_runtime_pass);

    let post_restart_pass = {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note restart database");
        let env = MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native");
        let (first_state, _receiver) =
            test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
        let created = super::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-restart","note":"survives restart"}"#,
            &first_state,
        )
        .await
        .unwrap();
        let persisted = db.list_user_notes(10, 0).await.unwrap();
        let (restarted_state, _receiver) =
            test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));
        *restarted_state.user_notes.write().await =
            super::UserNoteStore::from_persisted(persisted.clone());
        let fetched = super::route_http_request(
            "GET",
            "/api/v0/users/notes/route-audit-restart",
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        let fetched_json = serde_json::from_str::<serde_json::Value>(&fetched.body).unwrap();
        created.status == "200 OK"
            && persisted.len() == 1
            && persisted[0].note == "survives restart"
            && fetched.status == "200 OK"
            && fetched_json["note"] == "survives restart"
    };
    assert!(post_restart_pass);

    let post_concurrency_pass = {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note concurrency database");
        let (concurrent_state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_PERSISTENCE_ENABLED", "true")
                .with("SLSKR_CONTROLLER_PROFILE", "native"),
            super::SearchStore::new(),
            Some(db.clone()),
        );
        let bodies: Vec<String> = (0..4)
            .map(|index| {
                format!(r#"{{"username":"route-audit-concurrent-{index}","note":"note {index}"}}"#)
            })
            .collect();
        let responses = futures_util::future::join_all(bodies.iter().map(|body| {
            super::route_http_request("POST", "/api/v0/users/notes", None, body, &concurrent_state)
        }))
        .await;
        let persisted = db.list_user_notes(10, 0).await.unwrap_or_default();
        let expected: std::collections::BTreeSet<String> = (0..4)
            .map(|index| format!("route-audit-concurrent-{index}"))
            .collect();
        let persisted_usernames: std::collections::BTreeSet<String> = persisted
            .iter()
            .map(|record| record.username.clone())
            .collect();
        responses.iter().all(|response| {
            response
                .as_ref()
                .is_ok_and(|response| response.status == "200 OK")
        }) && persisted.len() == 4
            && persisted_usernames == expected
    };
    assert!(post_concurrency_pass);

    let delete_restart_pass = {
        let db = super::persistence::DatabaseManager::in_memory()
            .await
            .expect("user-note delete restart database");
        let env = MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "native");
        let (first_state, _receiver) =
            test_state_with_env_parts(env.clone(), super::SearchStore::new(), Some(db.clone()));
        let created = super::route_http_request(
            "POST",
            "/api/v0/users/notes",
            None,
            r#"{"username":"route-audit-delete-restart","note":"remove me"}"#,
            &first_state,
        )
        .await
        .unwrap();
        assert_eq!(created.status, "200 OK");
        let deleted = super::route_http_request(
            "DELETE",
            "/api/v0/users/notes/route-audit-delete-restart",
            None,
            "",
            &first_state,
        )
        .await
        .unwrap();
        let persisted_after_delete = db.list_user_notes(10, 0).await.unwrap();
        let (restarted_state, _receiver) =
            test_state_with_env_parts(env, super::SearchStore::new(), Some(db.clone()));
        *restarted_state.user_notes.write().await =
            super::UserNoteStore::from_persisted(persisted_after_delete.clone());
        let fetched = super::route_http_request(
            "GET",
            "/api/v0/users/notes/route-audit-delete-restart",
            None,
            "",
            &restarted_state,
        )
        .await
        .unwrap();
        deleted.status == "204 No Content"
            && persisted_after_delete.is_empty()
            && fetched.status == "404 Not Found"
    };
    assert!(delete_restart_pass);

    let ledger = [
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "mutation-side-effects-and-readback",
            "pass": created["username"] == "route-audit-peer"
                && created["note"] == "contract"
                && created["color"] == "red"
                && created["icon"] == "star"
                && created["isHighPriority"] == true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "nominal-status-headers-body",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "mutation-side-effects-and-readback",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "concurrency-and-idempotency",
            "pass": deleted_again.status == "204 No Content",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "nominal-status-headers-body",
            "pass": empty_list.status == "200 OK" && empty_list_json.as_array().is_some_and(Vec::is_empty),
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "missing-empty-or-conflict-state",
            "pass": empty_list.status == "200 OK" && empty_list_json.as_array().is_some_and(Vec::is_empty),
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "malformed-path-query-or-body",
            "pass": malformed_list.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes",
            "case": "populated-dynamic-state",
            "pass": populated_list.status == "200 OK"
                && populated_list_json.as_array().is_some_and(|records| {
                    records
                        .iter()
                        .any(|record| record["username"] == "route-audit-peer")
                }),
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "malformed-path-query-or-body",
            "pass": malformed_create.status == "400 Bad Request",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "missing-empty-or-conflict-state",
            "pass": missing_create.status == "400 Bad Request",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "runtime-failure-and-timeout",
            "pass": post_runtime_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "restart-persistence-or-reset",
            "pass": post_restart_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "POST",
            "route": "/api/v0/users/notes",
            "case": "concurrency-and-idempotency",
            "pass": post_concurrency_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "nominal-status-headers-body",
            "pass": fetched.status == "200 OK" && fetched_json["note"] == "contract",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "populated-dynamic-state",
            "pass": fetched.status == "200 OK" && fetched_json["username"] == "route-audit-peer",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "missing-empty-or-conflict-state",
            "pass": missing_fetch.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "GET",
            "route": "/api/v0/users/notes/{username}",
            "case": "malformed-path-query-or-body",
            "pass": malformed_fetch.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "malformed-path-query-or-body",
            "pass": malformed_delete.status == "404 Not Found",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "missing-empty-or-conflict-state",
            "pass": deleted_again.status == "204 No Content",
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "runtime-failure-and-timeout",
            "pass": delete_runtime_pass,
        }),
        serde_json::json!({
            "target": "slskdn",
            "method": "DELETE",
            "route": "/api/v0/users/notes/{username}",
            "case": "restart-persistence-or-reset",
            "pass": delete_restart_pass,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("user_notes_lifecycle.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}

#[cfg_attr(
    all(
        test,
        not(any(
            feature = "bounded-controller-api-tests",
            feature = "bounded-controller-api-tests-1",
            feature = "bounded-controller-api-tests-2",
            feature = "bounded-controller-api-tests-3",
            feature = "bounded-controller-api-tests-4",
            feature = "bounded-persistence-tests",
            feature = "bounded-file-lifecycle-tests",
            feature = "bounded-protocol-tests",
            feature = "bounded-security-control-tests",
            feature = "bounded-security-authorization-tests"
        ))
    ),
    tokio::test
)]
#[cfg(any(
    feature = "full-controller-tests",
    feature = "bounded-controller-api-tests",
    feature = "bounded-controller-api-tests-1"
))]
async fn controller_api_differential_mesh_http_disabled_shape() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("FEDERATION_ENABLED", "true")
            .with("FEDERATION_MODE", "Public")
            .with("FEDERATION_DOMAIN", "127.0.0.1"),
    );

    let webfinger = super::route_http_request("GET", "/.well-known/webfinger", None, "", &state)
        .await
        .expect("webfinger response");
    assert_eq!(webfinger.status, "400 Bad Request");
    assert_eq!(webfinger.content_type, "application/json");

    let missing_actor = super::route_http_request("GET", "/actors/library", None, "", &state)
        .await
        .expect("missing actor response");
    assert_eq!(missing_actor.status, "404 Not Found");
    assert_eq!(missing_actor.content_type, "application/json");

    let published = super::route_http_request(
        "POST",
        "/actors/music/outbox",
        None,
        r#"{"id":"activity-1","type":"Create","object":{"type":"Note","content":"hello"}}"#,
        &state,
    )
    .await
    .expect("publish activity");
    assert_eq!(published.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap()["type"],
        "Create"
    );

    for path in [
        "/actors/music",
        "/actors/music/inbox",
        "/actors/music/outbox",
        "/actors/music/followers",
        "/actors/music/following",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(response.status, "200 OK", "{path}");
        assert_eq!(response.content_type, "application/activity+json", "{path}");
        assert!(!response.body.to_ascii_lowercase().contains("<!doctype"));
    }

    let mesh = super::route_http_request("GET", "/mesh/http/services", None, "", &state)
        .await
        .expect("mesh services response");
    assert_eq!(mesh.status, "404 Not Found");
    assert_eq!(mesh.body, r#"{"error":"gateway_disabled"}"#);

    let ledger = [serde_json::json!({
        "target": "slskdn",
        "method": "GET",
        "route": "/mesh/http/services",
        "case": "missing-empty-or-conflict-state",
        "pass": true,
    })];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("mesh_http_disabled_shape.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}
