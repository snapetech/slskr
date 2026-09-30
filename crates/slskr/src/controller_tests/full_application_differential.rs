//! Controller full application differential ownership.

use super::*;

/// Bulk differential proof crediting 5 release-radar routes' cases,
/// independently re-derived from `versioned_release_radar_matches_
/// native_state_and_result_contracts`'s real SongID-confirmation
/// gate, deduplication, and notification-routing checks. slskdN-only
/// (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-2"
))]
pub(super) async fn controller_api_differential_release_radar() {
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

    let (state, _receiver) = test_state();
    let artist_id = "00000000-0000-4000-8000-000000000201";

    let subscription = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        &format!(
            r#"{{"artistId":"{artist_id}","artistName":"Differential Artist","scope":"trusted","enabled":true,"mutedReleaseGroupIds":[],"createdAt":"2026-01-01T00:00:00Z"}}"#
        ),
        &state,
    )
    .await
    .expect("create subscription");
    let subscription_json =
        serde_json::from_str::<serde_json::Value>(&subscription.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "nominal-status-headers-body",
        subscription.status == "200 OK"
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "mutation-side-effects-and-readback",
        subscription_json["id"] == format!("artist-radar:{artist_id}")
            && subscription_json["artistName"] == "Differential Artist"
            && subscription_json["createdAt"] == "2026-01-01T00:00:00+00:00"
    );

    let subscriptions = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        None,
        "",
        &state,
    )
    .await
    .expect("list subscriptions");
    let subscriptions_json =
        serde_json::from_str::<serde_json::Value>(&subscriptions.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "nominal-status-headers-body",
        subscriptions.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/subscriptions",
        "populated-dynamic-state",
        subscriptions_json == serde_json::json!([subscription_json])
    );

    let rejected = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        "{}",
        &state,
    )
    .await
    .expect("reject unconfirmed observation");
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "malformed-path-query-or-body",
        rejected.status == "400 Bad Request"
            && serde_json::from_str::<serde_json::Value>(&rejected.body).unwrap_or_default()
                == serde_json::json!({
                    "accepted": false,
                    "notifications": [],
                    "rejectionReason": "Observation is not SongID-confirmed.",
                })
    );

    let observation_body = format!(
        r#"{{"artistId":"{artist_id}","recordingId":"00000000-0000-4000-8000-000000000202","releaseId":"00000000-0000-4000-8000-000000000203","releaseGroupId":"00000000-0000-4000-8000-000000000204","sourceRealm":"realm","sourceActor":"actor","songIdConfirmed":true,"confidence":1,"workRef":{{"id":"00000000-0000-4000-8000-000000000202","type":"recording","domain":"music","externalIds":{{}},"title":"Track","creator":"Artist","year":2026,"metadata":{{}},"attributedTo":"actor","published":"2026-01-01T00:00:00Z"}},"observedAt":"2026-01-01T00:00:00Z"}}"#
    );
    let observation = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        None,
        &observation_body,
        &state,
    )
    .await
    .expect("accept confirmed observation");
    let observation_json =
        serde_json::from_str::<serde_json::Value>(&observation.body).unwrap_or_default();
    let notification = observation_json["notifications"][0].clone();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "nominal-status-headers-body",
        observation.status == "200 OK" && observation_json["accepted"] == true
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "mutation-side-effects-and-readback",
        notification["subscriptionId"] == format!("artist-radar:{artist_id}")
            && notification["artistId"] == artist_id
            && notification["firstSeenAt"] == "2026-01-01T00:00:00+00:00"
            && notification["read"] == false
            && notification["workRef"]["@context"]
                == serde_json::json!([
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
    .expect("deduplicate observation");
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/observations",
        "concurrency-and-idempotency",
        serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap_or_default()
            == serde_json::json!({"accepted": true, "notifications": []})
    );

    let notification_id = notification["id"].as_str().unwrap_or_default().to_owned();
    let notifications = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        None,
        "",
        &state,
    )
    .await
    .expect("list release-radar notifications");
    let notifications_json =
        serde_json::from_str::<serde_json::Value>(&notifications.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        "nominal-status-headers-body",
        notifications.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications",
        "populated-dynamic-state",
        notifications_json
            .as_array()
            .is_some_and(|items| { items.iter().any(|item| item["id"] == notification_id) })
    );
    let route_route =
        format!("/api/v0/musicbrainz/release-radar/notifications/{notification_id}/routes");
    let route = crate::route_http_request(
        "POST",
        &route_route,
        None,
        r#"{"targetPeerIds":[],"podId":"pod","channelId":"channel","senderPeerId":"sender"}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let route_json = serde_json::from_str::<serde_json::Value>(&route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "malformed-path-query-or-body",
        route.status == "400 Bad Request"
            && route_json["notificationId"] == notification_id
            && route_json["success"] == false
            && route_json["errorMessage"] == "At least one target peer is required."
            && route_json["targetPeerIds"] == serde_json::json!([])
    );

    let missing_route = crate::route_http_request(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/missing-differential/routes",
        None,
        "{}",
        &state,
    )
    .await
    .expect("missing release-radar notification route");
    let missing_route_json =
        serde_json::from_str::<serde_json::Value>(&missing_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "missing-empty-or-conflict-state",
        missing_route.status == "400 Bad Request"
            && missing_route_json["notificationId"] == "missing-differential"
            && missing_route_json["success"] == false
            && missing_route_json["errorMessage"] == "Notification not found."
    );

    let unavailable_route = crate::route_http_request(
        "POST",
        &route_route,
        None,
        r#"{"targetPeerIds":["actor:differential-radar-peer"]}"#,
        &state,
    )
    .await
    .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let unavailable_route_json =
        serde_json::from_str::<serde_json::Value>(&unavailable_route.body).unwrap_or_default();
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "runtime-failure-and-timeout",
        unavailable_route.status == "400 Bad Request"
            && unavailable_route_json["notificationId"] == notification_id
            && unavailable_route_json["success"] == false
            && unavailable_route_json["errorMessage"] == "Routing backend is not available."
            && unavailable_route_json["targetPeerIds"]
                == serde_json::json!(["actor:differential-radar-peer"])
    );

    let missing_routes = crate::route_http_request(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/empty-differential/routes",
        None,
        "",
        &state,
    )
    .await
    .expect("empty routes for missing release-radar notification");
    let missing_routes_json =
        serde_json::from_str::<serde_json::Value>(&missing_routes.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "missing-empty-or-conflict-state",
        missing_routes.status == "200 OK" && missing_routes_json == serde_json::json!([])
    );

    let routes_list = crate::route_http_request("GET", &route_route, None, "", &state)
        .await
        .unwrap_or_else(|error| panic!("{route_route}: {error}"));
    let routes_list_json =
        serde_json::from_str::<serde_json::Value>(&routes_list.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "nominal-status-headers-body",
        routes_list.status == "200 OK"
    );
    record!(
        "GET",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "populated-dynamic-state",
        routes_list_json.as_array().is_some_and(|attempts| {
            attempts.len() == 2
                && attempts
                    .iter()
                    .any(|attempt| attempt["id"] == route_json["id"])
                && attempts
                    .iter()
                    .any(|attempt| attempt["id"] == unavailable_route_json["id"])
        })
    );
    record!(
        "POST",
        "/api/v0/musicbrainz/release-radar/notifications/{notificationId}/routes",
        "mutation-side-effects-and-readback",
        routes_list_json.as_array().is_some_and(|attempts| {
            attempts.iter().any(|attempt| {
                attempt["id"] == unavailable_route_json["id"]
                    && attempt["notificationId"] == notification_id
                    && attempt["errorMessage"] == "Routing backend is not available."
            })
        })
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("release_radar.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api release-radar mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
/// Bulk differential proof for `GET /api/v0/application/build`
/// (real app version, not the wire-protocol version) and `DELETE
/// /api/v0/files/downloads/files/{base64FileName}` (remote file
/// management is forbidden by default, a real file removal is
/// genuinely performed and readback-confirmed, and a base64-
/// decoded path-traversal attempt is rejected) -- independently
/// re-derived from `build_info_uses_app_version_not_protocol_
/// version`, `controller_file_delete_routes_are_forbidden_by_default`,
/// and `controller_file_delete_routes_are_scoped_to_storage_roots` with
/// fresh fixture data. Confirmed against `/tmp/slskr-parity-
/// evidence/controller-api/*.json` before writing, per case: zero
/// prior credit on either route. slskdN-only (confirmed against
/// the frozen registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_build_info_and_file_delete() {
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

    let (state, _receiver) = test_state();
    let build = crate::route_http_request("GET", "/api/v0/application/build", None, "", &state)
        .await
        .expect("build info response");
    let build_json = serde_json::from_str::<serde_json::Value>(&build.body).unwrap_or_default();
    record!(
        "GET",
        "/api/v0/application/build",
        "nominal-status-headers-body",
        build.status == "200 OK"
            && build_json["current"] == env!("CARGO_PKG_VERSION")
            && build_json["protocol"]["major"] == serde_json::json!(crate::CLIENT_MAJOR_VERSION)
    );

    let (disabled_state, _receiver) = test_state();
    let forbidden = crate::route_http_request(
        "DELETE",
        "/api/v0/files/downloads/files/differential-token",
        None,
        "",
        &disabled_state,
    )
    .await
    .expect("default remote file management policy response");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "missing-empty-or-conflict-state",
        forbidden.status == "403 Forbidden"
    );

    let (enabled_state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"));
    let download_file = enabled_state
        .config
        .downloads_dir
        .join("Remote")
        .join("DifferentialSong.mp3");
    std::fs::create_dir_all(download_file.parent().unwrap())
        .expect("create differential download directory");
    std::fs::write(&download_file, b"differential-song").expect("write differential download file");
    let token = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "Remote/DifferentialSong.mp3",
    );
    let deleted = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{token}"),
        None,
        "",
        &enabled_state,
    )
    .await
    .expect("real file delete response");
    let missing_after = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{token}"),
        None,
        "",
        &enabled_state,
    )
    .await
    .expect("repeat delete response");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "mutation-side-effects-and-readback",
        deleted.status == "204 No Content"
            && !download_file.exists()
            && missing_after.status == "204 No Content"
    );

    let traversal_token = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "../differential-secret",
    );
    let traversal = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/files/downloads/files/{traversal_token}"),
        None,
        "",
        &enabled_state,
    )
    .await
    .expect("traversal delete response");
    record!(
        "DELETE",
        "/api/v0/files/downloads/files/{base64FileName}",
        "malformed-path-query-or-body",
        traversal.status == "400 Bad Request"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("build_info_and_file_delete.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api build-info-and-file-delete mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Bulk differential proof for the slskdN application version reads:
/// semantic version, latest-version state, and build metadata.  The
/// latest-version and build rows are checked both before and after
/// populating the real in-memory release state, so the evidence covers
/// the controller's current snapshot rather than only a hardcoded empty
/// response.  slskdN-only (confirmed against the frozen registry).
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_application_version_state_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} GET {} [{}]",
                    $route,
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    let version = crate::route_http_request("GET", "/api/v0/application/version", None, "", &state)
        .await
        .expect("application version response");
    let version_value =
        serde_json::from_str::<serde_json::Value>(&version.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/version",
        "nominal-status-headers-body",
        version.status == "200 OK"
            && version.content_type == "application/json"
            && version_value == env!("CARGO_PKG_VERSION")
    );

    let latest = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("latest application version response");
    let latest_value =
        serde_json::from_str::<serde_json::Value>(&latest.body).unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/version/latest",
        "nominal-status-headers-body",
        latest.status == "200 OK"
            && latest.content_type == "application/json"
            && latest_value["current"] == env!("CARGO_PKG_VERSION")
            && latest_value["latest"] == ""
            && latest_value["latestTag"] == ""
            && latest_value["latestUrl"] == ""
    );

    {
        let mut version_state = state
            .controller_version
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        version_state.latest = Some("9.9.9-slskdn.20260101120000".to_owned());
        version_state.latest_tag = Some("v9.9.9-slskdn.20260101120000".to_owned());
        version_state.latest_url = Some("https://example.test/releases/9.9.9".to_owned());
        version_state.checked_at = Some("2026-08-06T00:00:00Z".to_owned());
        version_state.is_update_available = Some(true);
    }

    let populated_latest = crate::route_http_request(
        "GET",
        "/api/v0/application/version/latest",
        None,
        "",
        &state,
    )
    .await
    .expect("populated latest application version response");
    let populated_latest_value = serde_json::from_str::<serde_json::Value>(&populated_latest.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/version/latest",
        "populated-dynamic-state",
        populated_latest.status == "200 OK"
            && populated_latest.content_type == "application/json"
            && populated_latest_value["latest"] == "9.9.9-slskdn.20260101120000"
            && populated_latest_value["latestTag"] == "v9.9.9-slskdn.20260101120000"
            && populated_latest_value["latestUrl"] == "https://example.test/releases/9.9.9"
            && populated_latest_value["checkedAt"] == "2026-08-06T00:00:00Z"
            && populated_latest_value["isUpdateAvailable"] == true
    );

    let populated_build =
        crate::route_http_request("GET", "/api/v0/application/build", None, "", &state)
            .await
            .expect("populated application build response");
    let populated_build_value = serde_json::from_str::<serde_json::Value>(&populated_build.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/application/build",
        "populated-dynamic-state",
        populated_build.status == "200 OK"
            && populated_build.content_type == "application/json"
            && populated_build_value["current"] == env!("CARGO_PKG_VERSION")
            && populated_build_value["latest"] == "9.9.9-slskdn.20260101120000"
            && populated_build_value["latestTag"] == "v9.9.9-slskdn.20260101120000"
            && populated_build_value["isUpdateAvailable"] == true
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("application_version_state_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api application-version mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential proof for the slskdN versioned application snapshot after
/// real runtime mutations.  The existing compatibility-shell test covers
/// the same projection through unversioned `/api/application`; this slice
/// exercises the registered `/api/v0/application` route itself and credits
/// only its populated dynamic-state row.
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_application_populated_versioned_state() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));

    let bridge_config = crate::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"maxClients":4,"enabled":true}"#,
        &state,
    )
    .await
    .expect("versioned bridge configuration mutation");
    assert_eq!(bridge_config.status, "200 OK", "{}", bridge_config.body);

    let restart = crate::route_http_request("PUT", "/api/v0/application", None, "{}", &state)
        .await
        .expect("versioned application restart mutation");
    assert_eq!(restart.status, "204 No Content", "{}", restart.body);

    let application = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("versioned application state response");
    let application_value =
        serde_json::from_str::<serde_json::Value>(&application.body).unwrap_or_default();
    let pass = application.status == "200 OK"
        && application.content_type == "application/json; charset=utf-8"
        && application_value["pendingRestart"] == true
        && application_value["bridge"]["configUpdates"] == 0;
    if !pass {
        mismatches.push(format!(
            "{target} application actual: {} {} {}",
            application.status, application.content_type, application.body
        ));
    }
    record!("/api/v0/application", "populated-dynamic-state", pass);

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("application_populated_versioned_state.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api populated-application mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
/// Bulk differential proof crediting `GET /api/v0/application`'s
/// real VPN-status projection (independently re-derived from
/// `vpn_state_projects_and_blocks_soulseek_until_ready`'s
/// non-trivial nested `vpn` object once the VPN is ready), `POST
/// /api/v0/soulseek/interests`'s real wire-command dispatch
/// (independently re-derived from `versioned_interest_mutations_
/// use_item_payload_and_wire_commands`: a real `AddThingILike`
/// server message is queued, not just a 204), `GET /api/v0/bridge/
/// admin/clients`'s real empty/disabled baseline that never leaks
/// unrelated peer activity into the legacy-client list
/// (independently re-derived from `bridge_admin_clients_never_
/// leaks_unrelated_peer_activity`), `GET /api/v0/mediacore/ipld/
/// inbound/{*targetContentId}`'s real empty-inbound-links shape
/// for a missing content id (the one genuinely uncredited route
/// out of the 17-route `materialized_controller_gets_match_
/// native_empty_state_contracts` table, independently re-derived
/// with a fresh path), and `PUT /api/v0/security/adversarial`'s
/// real target-YAML persistence and KV-store readback
/// (independently re-derived from `native_adversarial_put_
/// persists_and_accepts_target_yaml`: the on-disk YAML is genuinely
/// updated, remains reloadable, and the settings are readable back
/// from the real controller-features store). Confirmed against
/// `/tmp/slskr-parity-evidence/controller-api/*.json` before
/// writing, per case: all 5 were open. slskdN-only (confirmed
/// against the frozen registry).
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
    feature = "bounded-controller-api-tests-4"
))]
pub(super) async fn controller_api_differential_application_interests_bridge_mediacore_adversarial()
{
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

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKD_VPN", "true")
                .with("SLSKD_VPN_GLUETUN_URL", "http://127.0.0.1:8000"),
        );
        state.runtime.write().await.vpn = crate::vpn::Status {
            is_ready: true,
            is_connected: true,
            public_ip_address: Some("203.0.113.9".parse().unwrap()),
            location: "Differential, Testland".to_owned(),
            forwarded_port: Some(44_499),
            port_forwards: vec![crate::vpn::PortForward {
                slot: 0,
                local_port: 50_399,
                target_port: 50_399,
                proto: "tcp".to_owned(),
                public_port: 44_499,
                public_ip_address: Some("203.0.113.9".parse().unwrap()),
                namespace: "slskdn".to_owned(),
            }],
            relay: None,
        };
        let response = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
            .await
            .expect("application response");
        let application =
            serde_json::from_str::<serde_json::Value>(&response.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/application",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && application["vpn"]["isReady"] == true
                && application["vpn"]["location"] == "Differential, Testland"
                && application["vpn"]["forwardedPort"] == 44_499
        );
    }

    {
        use slskr_client::protocol::server::ServerMessage;
        let (state, mut receiver) = test_state();
        state.session.write().await.state = "connected";
        let liked = crate::route_http_request(
            "POST",
            "/api/v0/soulseek/interests",
            None,
            r#"{"item":"differential-ambient"}"#,
            &state,
        )
        .await
        .expect("interest mutation response");
        let dispatched = matches!(
            receiver.recv().await,
            Some(crate::SessionCommand::SendServerMessage(ServerMessage::AddThingILike { item }))
                if item == "differential-ambient"
        );
        record!(
            "POST",
            "/api/v0/soulseek/interests",
            "mutation-side-effects-and-readback",
            liked.status == "204 No Content" && dispatched
        );
    }

    {
        let (state, _receiver) = test_state();
        {
            let mut users = state.users.write().await;
            users.watch("differential-online-peer".to_owned());
            if let Some(record) = users
                .records
                .iter_mut()
                .find(|record| record.username == "differential-online-peer")
            {
                record.status = Some("online".to_owned());
            }
        }
        let clients =
            crate::route_http_request("GET", "/api/v0/bridge/admin/clients", None, "", &state)
                .await
                .expect("bridge admin clients response");
        let clients_json =
            serde_json::from_str::<serde_json::Value>(&clients.body).unwrap_or_default();
        record!(
            "GET",
            "/api/v0/bridge/admin/clients",
            "missing-empty-or-conflict-state",
            clients.status == "200 OK" && clients_json == serde_json::json!({"clients": []})
        );
    }

    {
        let (state, _receiver) = test_state();
        let response = crate::route_http_request(
            "GET",
            "/api/v0/mediacore/ipld/inbound/differential-missing",
            None,
            "",
            &state,
        )
        .await
        .expect("ipld inbound links response");
        record!(
            "GET",
            "/api/v0/mediacore/ipld/inbound/{*targetContentId}",
            "missing-empty-or-conflict-state",
            response.status == "200 OK" && response.body.contains("\"inboundLinks\":[]")
        );
    }

    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "native")
                .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        );
        fs::write(
            state.config.state_dir.join("slskd.yml"),
            "debug: true\nremote_configuration: true\n",
        )
        .expect("write differential base yaml");
        let response =
            crate::route_http_request("PUT", "/api/v0/security/adversarial", None, "{}", &state)
                .await
                .expect("adversarial settings response");
        let yaml_updated = fs::read_to_string(state.config.state_dir.join("slskd.yml"))
            .unwrap_or_default()
            .contains("  adversarial:\n");
        let reload_ok =
            crate::load_watched_controller_configuration(state.controller_cli_environment.clone())
                .is_ok();
        let features = state.controller_features.read().await;
        let stored = features.get("security/profile/security/adversarial");
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "nominal-status-headers-body",
            response.status == "200 OK"
                && response.content_type.starts_with("application/json")
                && response.body.contains("settings updated")
        );
        record!(
            "PUT",
            "/api/v0/security/adversarial",
            "mutation-side-effects-and-readback",
            response.status == "200 OK"
                && yaml_updated
                && reload_ok
                && stored.is_some_and(|value| value["settings"] == serde_json::json!({}))
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("application_interests_bridge_mediacore_adversarial.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api application-interests-bridge-mediacore-adversarial mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
