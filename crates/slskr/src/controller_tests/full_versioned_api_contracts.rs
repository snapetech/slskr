//! Controller full versioned api contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_session_bounds_failed_login_attempts() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "secret-token"),
    );
    for _ in 0..5 {
        let rejected = crate::route_http_request(
            "POST",
            "/api/v0/session",
            None,
            r#"{"username":"admin","password":"wrong"}"#,
            &state,
        )
        .await
        .expect("failed login");
        assert_eq!(rejected.status, "401 Unauthorized");
    }
    let locked = crate::route_http_request(
        "POST",
        "/api/v0/session",
        None,
        r#"{"username":"admin","password":"secret-token"}"#,
        &state,
    )
    .await
    .expect("locked login");
    assert_eq!(locked.status, "429 Too Many Requests", "{}", locked.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_get_validation_and_missing_resource_statuses_match_slskdn() {
    let (state, _receiver) = test_state();
    for path in [
        "/api/v0/collections/not-a-uuid",
        "/api/v0/share-grants/not-a-uuid",
        "/api/v0/multisource/search",
        "/api/v0/podcore/content/search",
        "/api/v0/telemetry/reports/transfers/leaderboard",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned GET validation");
        assert_eq!(
            response.status, "400 Bad Request",
            "{path}: {}",
            response.body
        );
    }
    for path in [
        "/api/v0/conversations/missing",
        "/api/v0/conversations/missing/messages",
        "/api/v0/jobs/missing",
        "/api/v0/profile/missing",
        "/api/v0/searches/missing/responses",
        "/api/v0/security/adversarial",
        "/api/v0/security/canaries",
        "/api/v0/security/tor/status",
        "/api/v0/shares/missing",
        "/api/v0/transfers/downloads/missing",
        "/api/v0/users/missing/browse/status",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("versioned missing GET");
        let expected_status = if path == "/api/v0/searches/missing/responses" {
            "400 Bad Request"
        } else {
            "404 Not Found"
        };
        assert_eq!(
            response.status, expected_status,
            "{path}: {}",
            response.body
        );
    }
    for path in [
        "/api/v0/relay/controller/downloads/token",
        "/api/v0/soulseek/mesh-rendezvous/discover",
        "/api/v0/soulseek/mesh-rendezvous/users",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("disabled versioned GET");
        assert_eq!(
            response.status, "403 Forbidden",
            "{path}: {}",
            response.body
        );
    }
    let response = crate::route_http_request("GET", "/api/v0/mesh/health", None, "", &state)
        .await
        .expect("native mesh health GET");
    assert_eq!(response.status, "200 OK", "{}", response.body);
    for path in ["/api/v0/signals/config", "/api/v0/signals/status"] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .expect("supported native versioned GET");
        assert_eq!(response.status, "200 OK", "{path}: {}", response.body);
    }
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn versioned_api_paths_map_to_current_handlers() {
    assert_eq!(normalize_api_path("/api/v0/health"), "/api/health");
    assert_eq!(normalize_api_path("/api/v0/metrics"), "/api/metrics");
    assert_eq!(normalize_api_path("/api/v0/telemetry"), "/api/telemetry");
    assert_eq!(
        normalize_api_path("/api/v0/capabilities/negotiate"),
        "/api/capabilities/negotiate"
    );
    assert_eq!(
        normalize_api_path("/api/v0/session/connect"),
        "/api/session/connect"
    );
    assert_eq!(normalize_api_path("/api/v0/dht/peers"), "/api/dht/peers");
    assert_eq!(normalize_api_path("/api/custom"), "/api/custom");
    assert_eq!(normalize_api_path("/api/info"), "/api/application");
    assert_eq!(
        normalize_api_path("/api/v0/portforwarding/available-ports"),
        "/api/port-forwarding/available-ports"
    );
    assert_eq!(
        normalize_api_path("/api/v0/portforwarding/start"),
        "/api/port-forwarding/start"
    );
    // Matches the oracle's real, distinct GetMeshPeers endpoint --
    // previously collapsed into the same (wrong) handler as
    // GET /api/capabilities/peers.
    assert_eq!(
        normalize_api_path("/api/v0/capabilities/mesh-peers"),
        "/api/v0/capabilities/mesh-peers"
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn versioned_public_routes_share_auth_policy_with_canonical_routes() {
    let env = MapEnv::default()
        .with(
            "SLSKR_STATE_DIR",
            &std::env::temp_dir().display().to_string(),
        )
        .with("SLSKR_API_TOKEN", "route-token");
    let config =
        crate::AppConfig::from_layers(None, FileConfig::default(), &env).expect("auth config");

    for path in ["/api/v0/health", "/api/v0/version", "/api/v0/capabilities"] {
        assert!(!crate::route_requires_auth(&config, path), "{path}");
    }
    assert!(crate::route_requires_auth(&config, "/api/v0/config"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_interest_mutations_use_item_payload_and_wire_commands() {
    use slskr_client::protocol::server::ServerMessage;
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";
    let liked = crate::route_http_request(
        "POST",
        "/api/v0/soulseek/interests",
        None,
        r#"{"item":"ambient"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(liked.status, "204 No Content");
    assert!(
        matches!(receiver.recv().await.unwrap(), crate::SessionCommand::SendServerMessage(ServerMessage::AddThingILike { item }) if item == "ambient")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_openapi_validation_and_large_dtos_match_native_contracts() {
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
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .unwrap();
        assert_eq!(response.status, expected, "{method} {path}: {}", response.body);
    }

    let multisource_test = crate::route_http_request(
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

    let bloom = crate::route_http_request(
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

    let songid = crate::route_http_request(
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

    let taste = crate::route_http_request(
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
        let response = crate::route_http_request("POST", path, None, invalid_work_ref, &state)
            .await
            .unwrap();
        assert_eq!(response.status, "400 Bad Request", "{path}");
    }

    let start = crate::route_http_request(
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
    let stop = crate::route_http_request("POST", "/api/v0/portforwarding/stop/1", None, "", &state)
        .await
        .unwrap();
    assert_eq!(stop.body, r#"{"message":"Port forwarding stopped"}"#);

    let index_sync = crate::route_http_request(
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
    let unsafe_decision = crate::route_http_request(
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
    let decision = crate::route_http_request(
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
    let created = crate::route_http_request(
        "POST",
        "/api/v0/podcore/content/create-pod",
        None,
        r#"{"podId":"pod:00000000000000000000000000000001","name":"Route Audit","visibility":"Listed","contentId":"content:music:recording:route-audit","tags":[],"channels":[],"externalBindings":[]}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(created.status, "201 Created", "{}", created.body);
    let joined = crate::route_http_request(
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

    let empty_backfill = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/backfill/{pod_id}/sync"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(empty_backfill.status, "400 Bad Request");
    let last_seen = crate::route_http_request(
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

    let verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        r#"{"messageId":"message","podId":"pod:00000000000000000000000000000001","channelId":"00000000-0000-4000-8000-000000000001","senderPeerId":"00000000-0000-4000-8000-000000000001","body":"route audit","timestampUnixMs":1,"signature":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=","sigVersion":1}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(verified.body, r#"{"isValid":true}"#);
    let evidence = crate::route_http_request(
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
    let opinion = crate::route_http_request(
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
    let removed = crate::route_http_request(
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
pub(super) async fn versioned_auxiliary_mutations_match_native_status_and_dto_contracts() {
    let (state, _receiver) = test_state();

    let unversioned_warm_cache = crate::route_http_request(
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
    let versioned_warm_cache = crate::route_http_request(
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

    let event = crate::route_http_request(
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

    let shares = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .unwrap();
    assert_eq!(shares.status, "404 Not Found");
    let scan = crate::route_http_request("PUT", "/api/v0/shares", None, "", &state)
        .await
        .unwrap();
    assert_eq!(scan.status, "200 OK");
    let cancelled = crate::route_http_request("DELETE", "/api/v0/shares", None, "", &state)
        .await
        .unwrap();
    assert_eq!(cancelled.status, "404 Not Found");

    let invite = crate::route_http_request(
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
    let imported = crate::route_http_request(
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

    let duplicate = crate::route_http_request(
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

    let invalid_content = crate::route_http_request(
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
    let valid_content = crate::route_http_request(
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

    let group = crate::route_http_request(
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

    let profile = crate::route_http_request(
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

    let verdict = crate::route_http_request(
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
pub(super) async fn versioned_discovery_graph_and_opinions_match_native_contracts() {
    let (state, _receiver) = test_state();
    let graph = crate::route_http_request(
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

    let invalid_opinion = crate::route_http_request(
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
    let missing_delete = crate::route_http_request(
        "DELETE",
        "/api/v0/opinions/00000000-0000-4000-8000-000000000001",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_delete.status, "404 Not Found");

    let valid_opinion = crate::route_http_request(
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
    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/opinions/{opinion_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "204 No Content");

    let contact = crate::route_http_request(
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
        let response = crate::route_http_request(
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
pub(super) async fn versioned_discovery_graph_reads_songid_store_and_musicbrainz_release_graph() {
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

    let run_graph = crate::route_http_request(
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

    let artist_graph = crate::route_http_request(
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
pub(super) async fn versioned_release_radar_matches_native_state_and_result_contracts() {
    let (state, _receiver) = test_state();
    let artist_id = "00000000-0000-4000-8000-000000000101";
    let subscription = crate::route_http_request(
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

    let subscriptions = crate::route_http_request(
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

    let rejected = crate::route_http_request(
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
    let observation = crate::route_http_request(
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

    let duplicate = crate::route_http_request(
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
    let route = crate::route_http_request(
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

    let routes = crate::route_http_request(
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
pub(super) async fn versioned_hashdb_paging_matches_sequence_controller_contract() {
    let (state, _receiver) = test_state();
    let hash_a = "a".repeat(64);
    let hash_b = "b".repeat(64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![
            crate::content_discovery::HashDbEntry {
                flac_key: hash_a.clone(),
                byte_hash: hash_a,
                size: 100,
                ..Default::default()
            },
            crate::content_discovery::HashDbEntry {
                flac_key: hash_b.clone(),
                byte_hash: hash_b,
                size: 200,
                ..Default::default()
            },
        ])
        .expect("seed hashdb sequence");

    let first =
        crate::route_http_request("GET", "/api/v0/hashdb/entries?limit=1", None, "", &state)
            .await
            .expect("first hashdb page");
    assert_eq!(first.status, "200 OK", "{}", first.body);
    let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first_json["latestSeq"], 2);
    assert_eq!(first_json["count"], 1);
    assert_eq!(first_json["entries"][0]["seqId"], 1);
    assert!(first_json.get("offset").is_none());
    assert!(first_json.get("limit").is_none());

    let second = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/entries?offset=1&limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("sequence-offset hashdb page");
    let second_json = serde_json::from_str::<serde_json::Value>(&second.body).unwrap();
    assert_eq!(second_json["entries"][0]["seqId"], 2);

    let sync = crate::route_http_request(
        "GET",
        "/api/v0/hashdb/sync/since/1?limit=1",
        None,
        "",
        &state,
    )
    .await
    .expect("hashdb sync page");
    let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap();
    assert_eq!(sync_json["latestSeq"], 2);
    assert_eq!(sync_json["count"], 1);
    assert_eq!(sync_json["entries"][0]["seqId"], 2);
    assert!(sync_json.get("fromSeqId").is_none());
    assert!(sync_json.get("hasMore").is_none());
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_relay_controller_download_binds_token_to_agent() {
    let (state, secret, now) = configured_relay_test_state().await;
    let download_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_download_tokens("Relay/Agent.txt", now)
        .into_iter()
        .next()
        .expect("registered agent download token")
        .1;
    let download_credential =
        crate::relay::credential_for_test(secret, "edge-one", &download_token);
    let root = crate::effective_downloads_dir(&state);
    fs::create_dir_all(root.join("Relay")).expect("relay download root");
    fs::write(root.join("Relay/Agent.txt"), b"relay payload").expect("relay fixture");
    let headers = crate::RequestSecurityHeaders {
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(download_credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let download = Box::pin(crate::route_http_request_with_headers(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &state,
        headers.clone(),
    ))
    .await
    .expect("relay download route");
    assert_eq!(download.status, "200 OK");
    assert_eq!(download.content_type, "application/octet-stream");
    assert!(download.body.is_empty());
    let mut stream = crate::open_relay_controller_download(&state, &download_token, &headers)
        .await
        .expect("relay download stream");
    let mut payload = Vec::new();
    std::io::Read::read_to_end(&mut stream.file, &mut payload).expect("read relay payload");
    assert_eq!(payload, b"relay payload");

    let invalid = Box::pin(crate::route_http_request_with_headers(
        "GET",
        "/api/v0/relay/controller/downloads/not-a-guid",
        None,
        "",
        &state,
        headers.clone(),
    ))
    .await
    .expect("invalid relay token route");
    assert_eq!(invalid.status, "400 Bad Request");
    let missing_credential = crate::route_http_request(
        "GET",
        &format!("/api/v0/relay/controller/downloads/{download_token}"),
        None,
        "",
        &state,
    )
    .await
    .expect("missing relay credential route");
    assert_eq!(missing_credential.status, "401 Unauthorized");
    let _ = fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test(flavor = "multi_thread", worker_threads = 2))]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_relay_controller_upload_tokens_are_one_use() {
    let (state, secret, now) = configured_relay_test_state().await;
    let (upload_token, upload_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Upload.flac", 0, now)
        .expect("relay upload stream");
    let upload_credential =
        crate::relay::credential_for_test(secret, "edge-one", &upload_token.to_string());
    let upload_body = "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Upload.flac\"\r\n\r\npayload\r\n--relay--\r\n";
    let upload_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(upload_credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let upload = Box::pin(crate::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay file upload route");
    assert_eq!(upload.status, "200 OK");
    let uploaded = upload_receiver
        .await
        .expect("relay upload receiver")
        .expect("relay upload result");
    assert_eq!(uploaded.filename, "Upload.flac");
    let stored_upload = state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", upload_token.simple()));
    assert_eq!(
        fs::read(&stored_upload).expect("stored relay upload"),
        b"payload"
    );
    let (abandoned_token, abandoned_receiver) = state
        .relay
        .write()
        .await
        .protocol
        .begin_file_stream("edge-one", "Abandoned.flac", 0, now)
        .expect("abandoned relay upload stream");
    drop(abandoned_receiver);
    let mut abandoned_headers = upload_headers.clone();
    abandoned_headers.x_relay_credential = Some(crate::relay::credential_for_test(
        secret,
        "edge-one",
        &abandoned_token.to_string(),
    ));
    let abandoned = Box::pin(crate::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{abandoned_token}"),
        None,
        "--relay\r\nContent-Disposition: form-data; name=\"file\"; filename=\"Abandoned.flac\"\r\n\r\nstranded\r\n--relay--\r\n",
        &state,
        abandoned_headers,
    ))
    .await
    .expect("abandoned relay upload route");
    assert_eq!(abandoned.status, "503 Service Unavailable");
    assert!(!state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!("file-{}.part", abandoned_token.simple()))
        .exists());
    let replay = Box::pin(crate::route_http_request_with_headers(
        "POST",
        &format!("/api/v0/relay/controller/files/{upload_token}"),
        None,
        upload_body,
        &state,
        upload_headers.clone(),
    ))
    .await
    .expect("relay upload replay route");
    assert_eq!(replay.status, "401 Unauthorized");

    let share_token = state
        .relay
        .write()
        .await
        .protocol
        .issue_share_upload_token("edge-one", now)
        .expect("share token");
    let share_credential = crate::relay::credential_for_test(secret, "edge-one", &share_token);
    let share_headers = crate::RequestSecurityHeaders {
        content_type: Some("multipart/form-data; boundary=relay".to_owned()),
        x_relay_agent: Some("edge-one".to_owned()),
        x_relay_credential: Some(share_credential),
        remote_addr: Some("127.0.0.1:1".parse().unwrap()),
        ..crate::RequestSecurityHeaders::default()
    };
    let database_source = state.config.state_dir.join("relay-test-source.db");
    crate::relay::write_share_database(
        &database_source,
        crate::ControllerProfile::Legacy,
        &[crate::relay::RemoteShare {
            filename: "Remote/Agent.flac".to_owned(),
            size: 6,
        }],
    )
    .await
    .expect("relay share database");
    let database_bytes = fs::read(&database_source).expect("read relay share database");
    let mut share_body = Vec::new();
    share_body.extend_from_slice(
        b"--relay\r\nContent-Disposition: form-data; name=\"shares\"\r\n\r\n[]\r\n--relay\r\nContent-Disposition: form-data; name=\"database\"; filename=\"shares.db\"\r\n\r\n",
    );
    share_body.extend_from_slice(&database_bytes);
    share_body.extend_from_slice(b"\r\n--relay--\r\n");
    let shares = crate::versioned_relay_request_bytes(
        "POST",
        &format!("/api/v0/relay/controller/shares/{share_token}"),
        &share_body,
        &share_headers,
        &state,
    )
    .await
    .expect("relay share upload route");
    assert_eq!(shares.status, "200 OK");
    assert_eq!(
        state
            .relay
            .read()
            .await
            .protocol
            .remote_file_for_agent("edge-one", "Remote/Agent.flac"),
        Some(("Remote/Agent.flac".to_owned(), 6))
    );
    let stored_share = state
        .config
        .state_dir
        .join("relay")
        .join("incoming")
        .join(format!(
            "share-{}.db",
            uuid::Uuid::parse_str(&share_token)
                .expect("share token UUID")
                .simple()
        ));
    assert_eq!(
        fs::read(&stored_share)
            .expect("stored relay database")
            .get(..16),
        Some(b"SQLite format 3\0".as_slice())
    );
    let _ = fs::remove_file(database_source);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn versioned_solid_resolution_uses_native_problem_details() {
    let (state, _receiver) = test_state();
    let invalid =
        crate::route_http_request("POST", "/api/v0/solid/resolve-webid", None, "{}", &state)
            .await
            .expect("invalid WebID response");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(invalid.content_type, "application/problem+json");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&invalid.body).unwrap(),
        serde_json::json!({
            "type": "about:blank",
            "title": "Invalid WebID",
            "status": 400,
            "detail": "WebId must be an absolute URI."
        })
    );

    let blocked = crate::route_http_request(
        "POST",
        "/api/v0/solid/resolve-webid",
        None,
        r#"{"webId":"https://example.com/profile#me"}"#,
        &state,
    )
    .await
    .expect("blocked WebID response");
    assert_eq!(blocked.status, "400 Bad Request");
    let blocked_json = serde_json::from_str::<serde_json::Value>(&blocked.body).unwrap();
    assert_eq!(blocked_json["title"], "Solid fetch blocked");
    assert_eq!(blocked_json["status"], 400);
    assert_eq!(
        blocked_json["detail"],
        "WebID resolution was blocked by policy."
    );
}
