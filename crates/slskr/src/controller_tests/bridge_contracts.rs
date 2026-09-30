use std::path::Path;

use super::{add_test_share, test_state_with_env, MapEnv};

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_admin_clients_never_leaks_unrelated_peer_activity() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    // Real, unrelated peer activity: an online watched user and a
    // real peer capability record. Neither is a legacy client
    // connected to the embedded Soulfind bridge listener, so neither
    // must appear in the bridge client list.
    {
        let mut users = state.users.write().await;
        users.watch("online-peer".to_owned());
        if let Some(record) = users
            .records
            .iter_mut()
            .find(|record| record.username == "online-peer")
        {
            record.status = Some("online".to_owned());
        }
    }
    let clients = crate::route_http_request("GET", "/api/bridge/admin/clients", None, "", &state)
        .await
        .expect("bridge clients");
    assert_eq!(clients.status, "200 OK", "{}", clients.body);
    let clients_json = serde_json::from_str::<serde_json::Value>(&clients.body).unwrap();
    assert_eq!(
        clients_json,
        serde_json::json!({"clients": [], "count": 0, "status": "disabled", "ready": false}),
        "{clients_json}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_search_and_download_use_real_oracle_shapes() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));
    add_test_share(
        &state,
        "Virtual/Bridge Track.flac",
        Path::new("/nonexistent/bridge-track.flac"),
        4096,
    )
    .await;

    // Matches the oracle's real BridgeSearchResult{Query,
    // Users[{PeerId, Username, Files[...]}]} contract, not the
    // generic /api/search record shape.
    let search = crate::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"Bridge Track"}"#,
        &state,
    )
    .await
    .expect("bridge search");
    assert_eq!(search.status, "201 Created", "{}", search.body);
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap();
    assert_eq!(search_json["query"], "Bridge Track");
    let users = search_json["users"].as_array().unwrap();
    assert_eq!(users.len(), 1, "{search_json}");
    assert!(users[0].get("peerId").is_some(), "{search_json}");
    assert!(users[0].get("username").is_some(), "{search_json}");
    let files = users[0]["files"].as_array().unwrap();
    assert_eq!(files.len(), 1, "{search_json}");
    assert_eq!(files[0]["path"], "Virtual/Bridge Track.flac");
    assert_eq!(files[0]["sizeBytes"], 4096);
    assert_eq!(files[0]["codec"], "flac");

    // A query with no matches must report a real empty users list,
    // not a synthetic empty-file peer entry.
    let empty_search = crate::route_http_request(
        "POST",
        "/api/v0/bridge/search",
        None,
        r#"{"query":"nothing-matches-this"}"#,
        &state,
    )
    .await
    .expect("bridge search with no matches");
    let empty_search_json = serde_json::from_str::<serde_json::Value>(&empty_search.body).unwrap();
    assert_eq!(empty_search_json["users"], serde_json::json!([]));

    // Matches the oracle's real single-item BridgeDownloadRequest /
    // {transfer_id} contract, not the generic /api/downloads
    // batch shape.
    let download = crate::route_http_request(
        "POST",
        "/api/v0/bridge/download",
        None,
        r#"{"username":"peer","filename":"Virtual/Bridge Track.flac","targetPath":"/tmp/out.flac"}"#,
        &state,
    )
    .await
    .expect("bridge download");
    assert_eq!(download.status, "200 OK", "{}", download.body);
    let download_json = serde_json::from_str::<serde_json::Value>(&download.body).unwrap();
    assert!(download_json["transfer_id"].is_string(), "{download_json}");
    assert!(
        download_json.get("downloadIds").is_none(),
        "{download_json}"
    );
    assert!(download_json.get("enqueued").is_none(), "{download_json}");
}
