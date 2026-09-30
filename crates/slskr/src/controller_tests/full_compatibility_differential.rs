//! Controller full compatibility differential ownership.

use super::*;

/// Bulk differential proof for compatibility-tail controller projections
/// that already have direct contract tests but were not yet linked to the
/// behavioral ledger: profile projection, share-grant token acknowledgement,
/// and the default-disabled relay, mesh-rendezvous, and STUN guards.
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
pub(super) async fn controller_api_differential_compatibility_projection_tail() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($method:expr, $route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!(
                    "{target} {} {} [{}]",
                    $method, $route, $case
                ));
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

    let advanced = serde_json::json!({
        "mesh": {
            "enabled": true,
            "enableOverlay": false,
            "enableDht": false,
            "enableStun": false
        }
    });
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_ADVANCED_NETWORKING_JSON", &advanced.to_string()),
    );

    let profile = crate::route_http_request("GET", "/api/v0/profile/me", None, "", &state)
        .await
        .expect("versioned profile projection");
    let profile_json =
        serde_json::from_str::<serde_json::Value>(&profile.body).unwrap_or(serde_json::Value::Null);
    record!(
        "GET",
        "/api/v0/profile/me",
        "nominal-status-headers-body",
        profile.status == "200 OK"
            && profile.content_type.starts_with("application/json")
            && profile_json["peerId"]
                .as_str()
                .is_some_and(|peer_id| !peer_id.is_empty())
            && profile_json["displayName"].is_string()
            && profile_json["capabilities"].is_i64()
            && profile_json["endpoints"].is_array()
    );

    let collection = crate::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Compatibility Tail"}"#,
        &state,
    )
    .await
    .expect("compatibility-tail collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .unwrap_or(serde_json::Value::Null)["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let grant = crate::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"tail-peer"}}"#),
        &state,
    )
    .await
    .expect("compatibility-tail share grant");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body)
        .unwrap_or(serde_json::Value::Null)["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let share_token = crate::route_http_request(
        "POST",
        &format!("/api/v0/share-grants/{grant_id}/token"),
        None,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("versioned share-grant token acknowledgement");
    let share_token_json = serde_json::from_str::<serde_json::Value>(&share_token.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "POST",
        "/api/v0/share-grants/{id}/token",
        "nominal-status-headers-body",
        share_token.status == "201 Created"
            && share_token_json["token"].is_string()
            && share_token_json["created"] == true
            && share_token_json["persisted"] == false
            && share_token_json["status"] == "ephemeral_compatibility_token"
    );

    let relay_download = crate::route_http_request(
        "GET",
        "/api/v0/relay/controller/downloads/token",
        None,
        "",
        &state,
    )
    .await
    .expect("disabled relay controller download");
    record!(
        "GET",
        "/api/v0/relay/controller/downloads/{token}",
        "missing-empty-or-conflict-state",
        relay_download.status == "403 Forbidden"
            && relay_download.body.contains("feature is disabled")
    );

    let mesh_users = crate::route_http_request(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/users",
        None,
        "",
        &state,
    )
    .await
    .expect("disabled mesh-rendezvous users");
    record!(
        "GET",
        "/api/v0/soulseek/mesh-rendezvous/users",
        "missing-empty-or-conflict-state",
        mesh_users.status == "403 Forbidden" && mesh_users.body.contains("feature is disabled")
    );

    let nat_detect = crate::route_http_request("POST", "/api/v0/mesh/nat/detect", None, "", &state)
        .await
        .expect("disabled mesh NAT detection");
    record!(
        "POST",
        "/api/v0/mesh/nat/detect",
        "missing-empty-or-conflict-state",
        nat_detect.status == "404 Not Found"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("compatibility_projection_tail.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api compatibility-tail mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

/// Differential evidence for the two small slskdN compatibility/fairness
/// projections that previously only had route-presence proof.  The
/// fairness cases use the same durable TrafficStats boundary as the
/// frozen FairnessGuard, including its closed-database failure contract.
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
pub(super) async fn controller_api_differential_compatibility_info_and_fairness_contracts() {
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($route:expr, $case:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("GET {} [{}]", $route, $case));
            }
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": "GET",
                "route": $route,
                "case": $case,
                "pass": $pass,
            }));
        };
    }

    let (info_state, _receiver) = test_state();
    let malformed_info = crate::route_http_request("GET", "/api/info/extra", None, "", &info_state)
        .await
        .expect("malformed compatibility info route");
    record!(
        "/api/info",
        "malformed-path-query-or-body",
        malformed_info.status == "404 Not Found"
    );

    let empty_info = crate::route_http_request("GET", "/api/info", None, "", &info_state)
        .await
        .expect("empty compatibility info route");
    let empty_info_json = serde_json::from_str::<serde_json::Value>(&empty_info.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/info",
        "missing-empty-or-conflict-state",
        empty_info.status == "200 OK"
            && empty_info.content_type == "application/json"
            && empty_info_json["impl"] == "slskdn"
            && empty_info_json["compat"] == "slskd"
            && empty_info_json["version"].is_string()
            && empty_info_json["soulseek"]["connected"] == false
            && empty_info_json["soulseek"]["user"] == "tester"
    );

    {
        let mut session = info_state.session.write().await;
        session.state = "connected";
        session.username = Some("connected-user".to_owned());
    }
    let populated_info = crate::route_http_request("GET", "/api/info", None, "", &info_state)
        .await
        .expect("populated compatibility info route");
    let populated_info_json = serde_json::from_str::<serde_json::Value>(&populated_info.body)
        .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/info",
        "populated-dynamic-state",
        populated_info.status == "200 OK"
            && populated_info_json["soulseek"]["connected"] == true
            && populated_info_json["soulseek"]["user"] == "connected-user"
    );

    let info_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("compatibility info database");
    let (info_failure_state, _receiver) = test_state_with_env_parts(
        MapEnv::default(),
        crate::SearchStore::new(),
        Some(info_db.clone()),
    );
    info_db.close_for_test().await;
    let info_failure = crate::route_http_request("GET", "/api/info", None, "", &info_failure_state)
        .await
        .expect("compatibility info route with closed unrelated database");
    record!(
        "/api/info",
        "runtime-failure-and-timeout",
        info_failure.status == "200 OK"
            && serde_json::from_str::<serde_json::Value>(&info_failure.body)
                .map(|value| value["impl"] == "slskdn")
                .unwrap_or(false)
    );

    let fairness_empty = test_state().0;
    let fairness_empty_response =
        crate::route_http_request("GET", "/api/v0/fairness/summary", None, "", &fairness_empty)
            .await
            .expect("empty fairness summary");
    let fairness_empty_json =
        serde_json::from_str::<serde_json::Value>(&fairness_empty_response.body)
            .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/fairness/summary",
        "missing-empty-or-conflict-state",
        fairness_empty_response.status == "200 OK"
            && fairness_empty_response.content_type == "application/json"
            && fairness_empty_json["throttleOverlayDownloads"] == false
            && fairness_empty_json["reason"] == "within fairness constraints"
            && fairness_empty_json["overlayUploadDownloadRatio"] == 1.0
            && fairness_empty_json["overlayToSoulseekUploadRatio"] == 0.0
            && fairness_empty_json["totals"]
                == serde_json::json!({
                    "overlayUploadBytes": 0,
                    "overlayDownloadBytes": 0,
                    "soulseekUploadBytes": 0,
                    "soulseekDownloadBytes": 0,
                })
    );

    let malformed_fairness = crate::route_http_request(
        "GET",
        "/api/v0/fairness/summary/extra",
        None,
        "",
        &fairness_empty,
    )
    .await
    .expect("malformed fairness summary route");
    record!(
        "/api/v0/fairness/summary",
        "malformed-path-query-or-body",
        malformed_fairness.status == "404 Not Found"
    );

    let fairness_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("fairness database");
    fairness_db
        .add_traffic(120, 200, 60, 300)
        .await
        .expect("seed fairness totals");
    let (fairness_populated, _receiver) = test_state_with_env_parts(
        MapEnv::default(),
        crate::SearchStore::new(),
        Some(fairness_db.clone()),
    );
    let populated_fairness = crate::route_http_request(
        "GET",
        "/api/v0/fairness/summary",
        None,
        "",
        &fairness_populated,
    )
    .await
    .expect("populated fairness summary");
    let populated_fairness_json =
        serde_json::from_str::<serde_json::Value>(&populated_fairness.body)
            .unwrap_or(serde_json::Value::Null);
    record!(
        "/api/v0/fairness/summary",
        "populated-dynamic-state",
        populated_fairness.status == "200 OK"
            && populated_fairness_json["throttleOverlayDownloads"] == false
            && populated_fairness_json["reason"] == "within fairness constraints"
            && populated_fairness_json["overlayUploadDownloadRatio"] == 0.6
            && populated_fairness_json["overlayToSoulseekUploadRatio"] == 2.0
            && populated_fairness_json["totals"]
                == serde_json::json!({
                    "overlayUploadBytes": 120,
                    "overlayDownloadBytes": 200,
                    "soulseekUploadBytes": 60,
                    "soulseekDownloadBytes": 300,
                })
    );

    let fairness_failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("fairness failure database");
    let (fairness_failure, _receiver) = test_state_with_env_parts(
        MapEnv::default(),
        crate::SearchStore::new(),
        Some(fairness_failure_db.clone()),
    );
    fairness_failure_db.close_for_test().await;
    let fairness_failure_response = crate::route_http_request(
        "GET",
        "/api/v0/fairness/summary",
        None,
        "",
        &fairness_failure,
    )
    .await
    .expect("fairness summary with closed database");
    record!(
        "/api/v0/fairness/summary",
        "runtime-failure-and-timeout",
        fairness_failure_response.status == "500 Internal Server Error"
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create compatibility evidence directory");
    fs::write(
        evidence_dir.join("compatibility_info_and_fairness_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize compatibility evidence"),
    )
    .expect("write compatibility evidence");
    assert!(
        mismatches.is_empty(),
        "{} compatibility info/fairness mismatches: {:?}",
        mismatches.len(),
        mismatches
    );
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
    feature = "bounded-controller-api-tests-3"
))]
pub(super) async fn controller_api_differential_compatibility_user_browse_contracts() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();
    macro_rules! record {
        ($case:expr, $pass:expr) => {{
            let pass = $pass;
            if !pass {
                mismatches.push(format!(
                    "{target} GET /api/compatibility/users/{{username}}/browse [{}]",
                    $case
                ));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": "/api/compatibility/users/{username}/browse",
                "case": $case,
                "pass": pass,
            }));
        }};
    }

    let env = || MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
    let (state, _receiver) = test_state_with_env(env());
    {
        let mut browse = state.browse.write().await;
        browse.add_entries(
            "compat-peer".to_owned(),
            vec![crate::BrowseEntry {
                filename: "Music/Track.flac".to_owned(),
                size: 321,
                extension: "flac".to_owned(),
                path_encoding: crate::ProtocolTextEncoding::Utf8,
            }],
            true,
        );
    }
    let nominal = crate::route_http_request(
        "GET",
        "/api/compatibility/users/compat-peer/browse",
        None,
        "",
        &state,
    )
    .await
    .expect("compatibility browse nominal response");
    let nominal_json = serde_json::from_str::<serde_json::Value>(&nominal.body).unwrap_or_default();
    record!(
        "nominal-status-headers-body",
        nominal.status == "200 OK"
            && nominal.content_type.starts_with("application/json")
            && nominal_json["username"] == "compat-peer"
            && nominal_json["directories"]
                .as_array()
                .is_some_and(|directories| {
                    directories.iter().any(|directory| {
                        directory["files"].as_array().is_some_and(|files| {
                            files.iter().any(|file| {
                                file["filename"] == "Track.flac"
                                    && file["size"] == 321
                                    && file["attributes"] == serde_json::json!(["flac"])
                            })
                        })
                    })
                })
    );
    record!(
        "populated-dynamic-state",
        nominal.status == "200 OK"
            && nominal_json["directories"]
                .as_array()
                .is_some_and(|directories| !directories.is_empty())
    );

    let malformed = crate::route_http_request(
        "GET",
        "/api/compatibility/users/compat-peer/browse/extra",
        None,
        "",
        &state,
    )
    .await
    .expect("malformed compatibility browse path");
    record!(
        "malformed-path-query-or-body",
        malformed.status == "404 Not Found"
    );

    let missing = crate::route_http_request(
        "GET",
        "/api/compatibility/users/missing-peer/browse",
        None,
        "",
        &state,
    )
    .await
    .expect("missing compatibility browse user");
    record!(
        "missing-empty-or-conflict-state",
        missing.status == "404 Not Found"
    );

    let failure_db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("compatibility browse failure database");
    let (failure_state, _receiver) =
        test_state_with_env_parts(env(), crate::SearchStore::new(), Some(failure_db.clone()));
    {
        let mut browse = failure_state.browse.write().await;
        browse.add_entries(
            "compat-peer".to_owned(),
            vec![crate::BrowseEntry {
                filename: "Music/Closed.flac".to_owned(),
                size: 123,
                extension: "flac".to_owned(),
                path_encoding: crate::ProtocolTextEncoding::Utf8,
            }],
            true,
        );
    }
    failure_db.close_for_test().await;
    let runtime = crate::route_http_request(
        "GET",
        "/api/compatibility/users/compat-peer/browse",
        None,
        "",
        &failure_state,
    )
    .await
    .expect("compatibility browse after database close");
    record!(
        "runtime-failure-and-timeout",
        runtime.status == "200 OK" && runtime.body.contains("Closed.flac")
    );

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    fs::write(
        evidence_dir.join("compatibility_user_browse_contracts.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize compatibility browse ledger"),
    )
    .expect("write compatibility browse ledger");
    assert!(
        mismatches.is_empty(),
        "{} compatibility browse mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
