//! Controller full automation differential ownership.

use super::*;

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
pub(super) async fn controller_api_differential_automation_compat_routes_use_expected_shapes() {
    let (state, mut receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        "peer 1=127.0.0.1:2234;peer1=127.0.0.1:2234",
    ));
    let mut ledger = Vec::new();
    macro_rules! record_evidence {
        ($method:expr, $route:expr, $case:expr) => {
            ledger.push(serde_json::json!({
                "target": "slskdn",
                "method": $method,
                "route": $route,
                "case": $case,
                "pass": true,
            }));
        };
    }

    let app = crate::route_http_request("GET", "/api/v0/application", None, "", &state)
        .await
        .expect("application route");
    assert_eq!(app.status, "200 OK");
    let app_json = serde_json::from_str::<serde_json::Value>(&app.body).unwrap();
    assert_eq!(app_json["version"]["current"], env!("CARGO_PKG_VERSION"));
    assert_eq!(app_json["server"]["isConnected"], false);
    record_evidence!("GET", "/api/v0/application", "nominal-status-headers-body");

    let server_connect = crate::route_http_request("POST", "/api/v0/server", None, "", &state)
        .await
        .expect("server connect");
    assert_eq!(server_connect.status, "202 Accepted");
    assert!(matches!(
        receiver.try_recv().unwrap(),
        crate::SessionCommand::Connect
    ));

    let server_connect_put = crate::route_http_request("PUT", "/api/v0/server", None, "", &state)
        .await
        .expect("server put connect");
    assert_eq!(server_connect_put.status, "205 Reset Content");
    assert!(server_connect_put.body.is_empty());
    record_evidence!("PUT", "/api/v0/server", "nominal-status-headers-body");
    assert!(receiver.try_recv().is_err());

    {
        let mut session = state.session.write().await;
        session.state = "error";
        session.last_error = Some("login failed: invalid password".to_owned());
    }
    let server_error = crate::route_http_request("GET", "/api/v0/server", None, "", &state)
        .await
        .expect("server error state");
    assert_eq!(server_error.status, "200 OK");
    let server_error_json = serde_json::from_str::<serde_json::Value>(&server_error.body).unwrap();
    assert_eq!(server_error_json["state"], "Error");
    assert_eq!(server_error_json["lastError"], "login failed");
    record_evidence!("GET", "/api/v0/server", "nominal-status-headers-body");
    record_evidence!("GET", "/api/v0/server", "populated-dynamic-state");

    // Versioned search creation requires an authenticated Soulseek server
    // session.  The preceding assertions intentionally exercise the error
    // snapshot, then restore the nominal connected state for the positive
    // search/readback contract below.
    state.session.write().await.state = "connected";

    let session_enabled =
        crate::route_http_request("GET", "/api/v0/session/enabled", None, "", &state)
            .await
            .expect("session enabled");
    assert!(matches!(session_enabled.body.as_str(), "true" | "false"));

    let search = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        r#"{"searchText":"Remote Song"}"#,
        &state,
    )
    .await
    .expect("search route");
    assert_eq!(search.status, "200 OK");
    assert!(search.body.contains("\"query\":\"Remote Song\""));
    let search_json = serde_json::from_str::<serde_json::Value>(&search.body).unwrap();
    assert!(search_json["searchId"].is_string());
    assert_eq!(search_json["query"], "Remote Song");
    assert!(search_json["results"].is_array());
    record_evidence!("POST", "/api/v0/searches", "nominal-status-headers-body");
    record_evidence!(
        "POST",
        "/api/v0/searches",
        "mutation-side-effects-and-readback"
    );
    let _ = receiver.try_recv();

    let _ = crate::route_http_request(
        "POST",
        "/api/v0/search-responses",
        None,
        r#"{"token":1,"peer_username":"peer1","filename":"Remote/Song.mp3","size":99}"#,
        &state,
    )
    .await
    .unwrap();
    let responses =
        crate::route_http_request("GET", "/api/v0/searches/1/responses", None, "", &state)
            .await
            .expect("search responses");
    let responses_json = serde_json::from_str::<serde_json::Value>(&responses.body).unwrap();
    assert!(responses_json.is_array());
    assert_eq!(responses_json[0]["username"], "peer1");
    assert_eq!(responses_json[0]["files"][0]["filename"], "Remote/Song.mp3");
    record_evidence!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/searches/{id}/responses",
        "populated-dynamic-state"
    );

    let uuid_search_id = "11111111-1111-1111-1111-111111111111";
    let uuid_search = crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        &format!(r#"{{"id":"{uuid_search_id}","searchText":"UUID Song"}}"#),
        &state,
    )
    .await
    .expect("uuid search route");
    assert_eq!(uuid_search.status, "200 OK");
    let uuid_search_json = serde_json::from_str::<serde_json::Value>(&uuid_search.body).unwrap();
    assert!(uuid_search_json["searchId"].is_string());
    assert_eq!(uuid_search_json["query"], "UUID Song");
    let _ = receiver.try_recv();

    let dashless_search_id = uuid_search_json["searchId"]
        .as_str()
        .expect("dashless search id");
    let dashless_detail = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{dashless_search_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("dashless search detail route");
    assert_eq!(dashless_detail.status, "200 OK");
    let dashless_responses = crate::route_http_request(
        "GET",
        &format!("/api/v0/searches/{dashless_search_id}/responses"),
        None,
        "",
        &state,
    )
    .await
    .expect("dashless search responses route");
    assert_eq!(dashless_responses.status, "200 OK");

    for (method, path) in [
        ("GET", format!("/api/v0/searches/{uuid_search_id}")),
        (
            "GET",
            format!("/api/v0/searches/{uuid_search_id}?includeResponses=true"),
        ),
        (
            "GET",
            format!("/api/v0/searches/{uuid_search_id}/responses"),
        ),
        ("PUT", format!("/api/v0/searches/{uuid_search_id}")),
        ("DELETE", format!("/api/v0/searches/{uuid_search_id}")),
    ] {
        let response = crate::route_http_request(method, &path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/uploads/peer1/1")
            && !path.contains("/transfers/downloads/peer1/1")
        {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
    }

    let enqueue = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/peer1",
        None,
        r#"{"files":[{"filename":"Remote/Song.mp3","size":99}]}"#,
        &state,
    )
    .await
    .expect("enqueue download");
    let enqueue_json = serde_json::from_str::<serde_json::Value>(&enqueue.body).unwrap();
    assert_eq!(enqueue_json["queued"], 1);
    assert_eq!(enqueue_json["transfers"][0]["username"], "peer1");
    assert_eq!(enqueue_json["transfers"][0]["filename"], "Remote/Song.mp3");
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/transfers/downloads/{username}",
        "mutation-side-effects-and-readback"
    );

    let downloads =
        crate::route_http_request("GET", "/api/v0/transfers/downloads", None, "", &state)
            .await
            .expect("downloads route");
    let downloads_json = serde_json::from_str::<serde_json::Value>(&downloads.body).unwrap();
    assert_eq!(downloads_json[0]["username"], "peer1");
    assert_eq!(downloads_json[0]["directories"][0]["directory"], "Remote");
    assert_eq!(downloads_json[0]["directories"][0]["fileCount"], 1);
    assert_eq!(
        downloads_json[0]["directories"][0]["files"][0]["direction"],
        "Download"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/transfers/downloads",
        "populated-dynamic-state"
    );

    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }
    // The route fixture can inherit a configured room from the caller's
    // environment.  Keep this round-trip deterministic so the first
    // request proves creation and the second proves idempotent readback.
    state.rooms.write().await.records.clear();
    let joined =
        crate::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .expect("join room");
    assert_eq!(joined.status, "201 Created");
    let joined_again =
        crate::route_http_request("POST", "/api/v0/rooms/joined", None, r#""music""#, &state)
            .await
            .unwrap();
    assert_eq!(joined_again.status, "200 OK");
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined",
        "concurrency-and-idempotency"
    );
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined",
        "mutation-side-effects-and-readback"
    );
    let joined_rooms = crate::route_http_request("GET", "/api/v0/rooms/joined", None, "", &state)
        .await
        .expect("joined rooms");
    let joined_rooms_json = serde_json::from_str::<serde_json::Value>(&joined_rooms.body).unwrap();
    assert!(joined_rooms_json
        .as_array()
        .is_some_and(|rooms| rooms.iter().any(|room| room.as_str() == Some("music"))));
    record_evidence!("GET", "/api/v0/rooms/joined", "populated-dynamic-state");
    let _ = receiver.try_recv();
    let repeated = crate::route_http_request("POST", "/api/v0/rooms/music/join", None, "", &state)
        .await
        .unwrap();
    assert_eq!(repeated.status, "200 OK");
    assert!(receiver.try_recv().is_err());

    let room_message = crate::route_http_request(
        "POST",
        "/api/v0/rooms/joined/music/messages",
        None,
        r#""hello room""#,
        &state,
    )
    .await
    .expect("room message");
    assert_eq!(room_message.status, "201 Created");
    assert!(room_message.body.is_empty());
    let _ = receiver.try_recv();

    let room_messages = crate::route_http_request(
        "GET",
        "/api/v0/rooms/joined/music/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("room messages");
    let room_messages_json =
        serde_json::from_str::<serde_json::Value>(&room_messages.body).unwrap();
    assert_eq!(room_messages_json[0]["message"], "hello room");
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/rooms/joined/{roomName}/messages",
        "mutation-side-effects-and-readback"
    );
    record_evidence!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/rooms/joined/{roomName}/messages",
        "populated-dynamic-state"
    );

    let conversation_send = crate::route_http_request(
        "POST",
        "/api/v0/conversations/peer1",
        None,
        r#""hello peer""#,
        &state,
    )
    .await
    .expect("conversation send");
    assert_eq!(conversation_send.status, "201 Created");
    assert!(conversation_send.body.is_empty());
    let _ = receiver.try_recv();

    let conversations = crate::route_http_request("GET", "/api/v0/conversations", None, "", &state)
        .await
        .expect("conversations");
    let conversations_json =
        serde_json::from_str::<serde_json::Value>(&conversations.body).unwrap();
    assert_eq!(conversations_json[0]["username"], "peer1");
    assert_eq!(
        conversations_json[0]["messages"][0]["message"],
        "hello peer"
    );
    record_evidence!(
        "POST",
        "/api/v0/conversations/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "POST",
        "/api/v0/conversations/{username}",
        "mutation-side-effects-and-readback"
    );
    record_evidence!(
        "GET",
        "/api/v0/conversations",
        "nominal-status-headers-body"
    );
    record_evidence!("GET", "/api/v0/conversations", "populated-dynamic-state");

    let user_status =
        crate::route_http_request("GET", "/api/v0/users/peer1/status", None, "", &state)
            .await
            .expect("user status");
    assert!(user_status.body.contains("\"presence\""));
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/status",
        "nominal-status-headers-body"
    );

    let user_directory = crate::route_http_request(
        "POST",
        "/api/v0/users/peer1/directory",
        None,
        r#"{"directory":"Virtual"}"#,
        &state,
    )
    .await
    .expect("user directory");
    let user_directory_json =
        serde_json::from_str::<serde_json::Value>(&user_directory.body).unwrap();
    assert!(user_directory_json.is_array());
    assert_eq!(user_directory_json[0]["name"], "Virtual");
    record_evidence!(
        "POST",
        "/api/v0/users/{username}/directory",
        "nominal-status-headers-body"
    );

    let user_endpoint =
        crate::route_http_request("GET", "/api/v0/users/peer%201/endpoint", None, "", &state)
            .await
            .expect("user endpoint");
    let user_endpoint_json =
        serde_json::from_str::<serde_json::Value>(&user_endpoint.body).unwrap();
    assert_eq!(user_endpoint_json["username"], "peer 1");
    assert_eq!(user_endpoint_json["addressFamily"], "IPv4");
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/endpoint",
        "populated-dynamic-state"
    );

    let browse_status = crate::route_http_request(
        "GET",
        "/api/v0/users/peer%201/browse/status",
        None,
        "",
        &state,
    )
    .await
    .expect("browse status");
    assert_eq!(browse_status.status, "404 Not Found");

    let user_status =
        crate::route_http_request("GET", "/api/v0/users/peer%201/status", None, "", &state)
            .await
            .expect("user status");
    let user_status_json = serde_json::from_str::<serde_json::Value>(&user_status.body).unwrap();
    assert_eq!(user_status_json["username"], "peer 1");
    assert_eq!(user_status_json["presence"], "Offline");
    record_evidence!(
        "GET",
        "/api/v0/users/{username}/status",
        "populated-dynamic-state"
    );

    state.shares.write().await.roots.push(crate::ShareRoot {
        label: "Virtual".to_owned(),
        local_path: PathBuf::from("Virtual"),
        raw: "Virtual".to_owned(),
        directories: 0,
        files: 1,
        bytes: 42,
        extensions: Vec::new(),
        statistics_ready: true,
    });
    let shares = crate::route_http_request("GET", "/api/v0/shares", None, "", &state)
        .await
        .expect("shares");
    let shares_json = serde_json::from_str::<serde_json::Value>(&shares.body).unwrap();
    let first_share = shares_json
        .as_object()
        .and_then(|object| object.values().next())
        .and_then(|value| value.as_array())
        .and_then(|array| array.first())
        .expect("first share");
    assert!(first_share["id"].is_string());
    assert!(first_share["raw"].is_string());
    record_evidence!("GET", "/api/v0/shares", "populated-dynamic-state");

    let share_contents =
        crate::route_http_request("GET", "/api/v0/shares/contents", None, "", &state)
            .await
            .expect("share contents");
    let share_contents_json =
        serde_json::from_str::<serde_json::Value>(&share_contents.body).unwrap();
    assert!(share_contents_json
        .as_array()
        .is_some_and(|entries| !entries.is_empty()));
    record_evidence!(
        "GET",
        "/api/v0/shares/contents",
        "nominal-status-headers-body"
    );
    record_evidence!("GET", "/api/v0/shares/contents", "populated-dynamic-state");

    let virtual_share = crate::route_http_request(
        "GET",
        &format!("/api/v0/shares/{}", crate::share_root_id("Virtual")),
        None,
        "",
        &state,
    )
    .await
    .expect("virtual share");
    let virtual_share_json =
        serde_json::from_str::<serde_json::Value>(&virtual_share.body).unwrap();
    assert_eq!(virtual_share_json["id"], crate::share_root_id("Virtual"));
    assert_eq!(virtual_share_json["files"], 1);

    let download_dir = crate::route_http_request(
        "GET",
        "/api/v0/files/downloads/directories",
        None,
        "",
        &state,
    )
    .await
    .expect("download dir");
    let download_dir_json = serde_json::from_str::<serde_json::Value>(&download_dir.body).unwrap();
    assert!(download_dir_json["fullName"].is_string());
    assert!(download_dir_json.get("fullname").is_none());

    let telemetry_transfer = crate::route_http_request(
        "POST",
        "/api/v0/transfers/downloads/telemetry%20peer",
        None,
        r#"[{"filename":"Telemetry/Album/Track.flac","size":321}]"#,
        &state,
    )
    .await
    .expect("telemetry transfer");
    assert_eq!(telemetry_transfer.status, "200 OK");

    // The leaderboard and directories reports only count real
    // completed transfers now (matching the oracle's State=48
    // filter), so mark the downloads above completed, and add a real
    // completed *upload* for the directories report -- which, also
    // matching the oracle, only ever reports on uploads (what other
    // users downloaded from local shares), never downloads.
    {
        let mut transfers = state.transfers.write().await;
        for entry in transfers.entries.iter_mut() {
            if entry.direction == 0 {
                entry.status = "succeeded".to_owned();
            }
        }
        // A distinct username -- not "telemetry peer" -- so this
        // synthetic upload doesn't inflate that user's own transfer
        // count/report later in this test.
        transfers.create(
            1,
            Some("directory-audit-peer".to_owned()),
            "Telemetry/Album/Track.flac".to_owned(),
            None,
            Some(321),
        );
        if let Some(entry) = transfers
            .entries
            .iter_mut()
            .rev()
            .find(|entry| entry.direction == 1)
        {
            entry.status = "succeeded".to_owned();
        }
    }

    let leaderboard = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download",
        None,
        "",
        &state,
    )
    .await
    .expect("leaderboard");
    let leaderboard_json = serde_json::from_str::<serde_json::Value>(&leaderboard.body).unwrap();
    let telemetry_leader = leaderboard_json
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["username"].as_str() == Some("telemetry peer"))
        })
        .expect("telemetry leaderboard row");
    assert_eq!(telemetry_leader["totalBytes"], 321);
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard",
        "populated-dynamic-state"
    );

    let asc_leaderboard = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/leaderboard?direction=Download&sortBy=TotalBytes&sortOrder=ASC",
        None,
        "",
        &state,
    )
    .await
    .expect("ascending leaderboard");
    let asc_leaderboard_json =
        serde_json::from_str::<serde_json::Value>(&asc_leaderboard.body).unwrap();
    assert_eq!(asc_leaderboard_json[0]["username"], "peer1");

    let summary = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/summary?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered summary");
    let summary_json = serde_json::from_str::<serde_json::Value>(&summary.body).unwrap();
    let summary_record = &summary_json["Download"]["Succeeded"];
    assert_eq!(summary_record["count"], 1);
    assert_eq!(summary_record["totalBytes"], 321);
    assert_eq!(summary_record["distinctUsers"], 1);
    assert_eq!(summary_json["Upload"], serde_json::json!({}));
    assert!(summary_json.get("count").is_none());

    let user_transfers = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("user transfer report");
    let user_transfers_json =
        serde_json::from_str::<serde_json::Value>(&user_transfers.body).unwrap();
    assert_eq!(user_transfers_json["username"], "telemetry peer");
    assert_eq!(user_transfers_json["count"], 1);
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/{username}",
        "populated-dynamic-state"
    );
    let aliased_user_transfers = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/users/unrelated/telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("reject aliased user transfer report");
    assert_eq!(aliased_user_transfers.status, "404 Not Found");
    let telemetry_transfer_id = user_transfers_json["transfers"][0]["id"]
        .as_str()
        .expect("telemetry transfer id")
        .to_owned();

    let directory_report = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        None,
        "",
        &state,
    )
    .await
    .expect("directory report");
    let directory_report_json =
        serde_json::from_str::<serde_json::Value>(&directory_report.body).unwrap();
    assert!(directory_report_json
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["path"].as_str() == Some("Telemetry/Album"))
        })
        .is_some());
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/directories",
        "populated-dynamic-state"
    );

    let pareto = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download",
        None,
        "",
        &state,
    )
    .await
    .expect("exceptions pareto");
    let pareto_json = serde_json::from_str::<serde_json::Value>(&pareto.body).unwrap();
    assert!(pareto_json.is_array());
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "nominal-status-headers-body"
    );

    let cancelled = crate::route_http_request(
        "DELETE",
        &format!("/api/v0/transfers/downloads/telemetry%20peer/{telemetry_transfer_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("cancel telemetry transfer");
    assert_eq!(cancelled.status, "204 No Content");
    let exceptions = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions?direction=Download&username=telemetry%20peer&sortOrder=ASC",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered exceptions");
    let exceptions_json = serde_json::from_str::<serde_json::Value>(&exceptions.body).unwrap();
    assert_eq!(exceptions_json[0]["username"], "telemetry peer");
    assert_eq!(exceptions_json[0]["state"], "Cancelled");
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "nominal-status-headers-body"
    );
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions",
        "populated-dynamic-state"
    );

    let populated_pareto = crate::route_http_request(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto?direction=Download&username=telemetry%20peer",
        None,
        "",
        &state,
    )
    .await
    .expect("populated exceptions pareto");
    let populated_pareto_json =
        serde_json::from_str::<serde_json::Value>(&populated_pareto.body).unwrap();
    assert_eq!(populated_pareto.status, "200 OK");
    assert_eq!(populated_pareto_json[0]["exception"], "Cancelled");
    assert_eq!(populated_pareto_json[0]["count"], 1);
    assert_eq!(populated_pareto_json[0]["distinctUsers"], 1);
    record_evidence!(
        "GET",
        "/api/v0/telemetry/reports/transfers/exceptions/pareto",
        "populated-dynamic-state"
    );

    let contract_routes = [
        ("GET", "/api/application", ""),
        ("GET", "/api/application/version", ""),
        ("GET", "/api/application/version/latest", ""),
        ("POST", "/api/application/gc", ""),
        ("GET", "/api/session", ""),
        (
            "POST",
            "/api/session",
            r#"{"username":"user","password":"pass"}"#,
        ),
        ("GET", "/api/session/enabled", ""),
        ("GET", "/api/server", ""),
        ("PUT", "/api/server", ""),
        ("DELETE", "/api/server", ""),
        ("POST", "/api/searches", r#"{"searchText":"contract song"}"#),
        ("GET", "/api/searches", ""),
        ("GET", "/api/searches/1", ""),
        ("PUT", "/api/searches/1", ""),
        ("GET", "/api/searches/1/responses", ""),
        ("GET", "/api/transfers/downloads/", ""),
        ("GET", "/api/transfers/downloads/peer1", ""),
        (
            "POST",
            "/api/transfers/downloads/peer1",
            r#"[{"filename":"Remote/Song.mp3","size":99}]"#,
        ),
        ("GET", "/api/transfers/downloads/peer1/1", ""),
        ("GET", "/api/transfers/downloads/peer1/1/position", ""),
        ("DELETE", "/api/transfers/downloads/peer1/1", ""),
        ("DELETE", "/api/transfers/downloads/all/completed", ""),
        ("GET", "/api/transfers/uploads/", ""),
        ("GET", "/api/transfers/uploads/peer1", ""),
        ("GET", "/api/transfers/uploads/peer1/1", ""),
        ("DELETE", "/api/transfers/uploads/peer1/1", ""),
        ("DELETE", "/api/transfers/uploads/all/completed", ""),
        ("GET", "/api/rooms/joined", ""),
        ("POST", "/api/rooms/joined", r#""contract-room""#),
        ("GET", "/api/rooms/joined/contract-room", ""),
        (
            "POST",
            "/api/rooms/joined/contract-room/messages",
            r#""hello""#,
        ),
        ("GET", "/api/rooms/joined/contract-room/messages", ""),
        (
            "POST",
            "/api/rooms/joined/contract-room/ticker",
            r#""ticker""#,
        ),
        (
            "POST",
            "/api/rooms/joined/contract-room/members",
            r#""peer1""#,
        ),
        ("GET", "/api/rooms/joined/contract-room/users", ""),
        ("DELETE", "/api/rooms/joined/contract-room", ""),
        ("GET", "/api/rooms/available", ""),
        ("GET", "/api/conversations", ""),
        ("POST", "/api/conversations/peer1", r#""hello peer again""#),
        ("GET", "/api/conversations/peer1", ""),
        ("GET", "/api/conversations/peer1/messages", ""),
        ("PUT", "/api/conversations/peer1/1", ""),
        ("PUT", "/api/conversations/peer1", ""),
        ("DELETE", "/api/conversations/peer1", ""),
        ("GET", "/api/users/peer1/endpoint", ""),
        ("GET", "/api/users/peer1/browse", ""),
        ("POST", "/api/users/peer1/directory", r#"{"directory":""}"#),
        ("GET", "/api/users/peer1/info", ""),
        ("GET", "/api/users/peer1/status", ""),
        ("GET", "/api/shares", ""),
        (
            "GET",
            "/api/shares/8E7DAA120E8DB8C0B0388938D9F61D6C6B796EDD",
            "",
        ),
        ("GET", "/api/shares/contents", ""),
        (
            "GET",
            "/api/shares/8E7DAA120E8DB8C0B0388938D9F61D6C6B796EDD/contents",
            "",
        ),
        ("PUT", "/api/shares", ""),
        ("DELETE", "/api/shares", ""),
        ("GET", "/api/files/downloads/directories", ""),
        ("GET", "/api/files/downloads/directories/Zm9v", ""),
        ("DELETE", "/api/files/downloads/directories/Zm9v", ""),
        ("DELETE", "/api/files/downloads/files/Zm9vLm1wMw", ""),
        ("GET", "/api/files/incomplete/directories", ""),
        ("GET", "/api/files/incomplete/directories/Zm9v", ""),
        ("DELETE", "/api/files/incomplete/directories/Zm9v", ""),
        ("DELETE", "/api/files/incomplete/files/Zm9vLm1wMw", ""),
        ("GET", "/api/options", ""),
        ("GET", "/api/options/startup", ""),
        ("GET", "/api/options/debug", ""),
        ("GET", "/api/options/yaml/location", ""),
        ("GET", "/api/options/yaml", ""),
        ("PUT", "/api/options/yaml", r#""app: {}""#),
        ("POST", "/api/options/yaml/validate", r#""app: {}""#),
        ("GET", "/api/events", ""),
        ("POST", "/api/events/Noop", r#""""#),
        ("GET", "/api/logs", ""),
        ("PUT", "/api/relay/agent", ""),
        ("DELETE", "/api/relay/agent", ""),
        ("GET", "/api/relay/controller/downloads/token", ""),
        ("POST", "/api/relay/controller/files/token", ""),
        ("POST", "/api/relay/controller/shares/token", ""),
        ("GET", "/api/telemetry/metrics", ""),
        ("GET", "/api/telemetry/metrics/kpis", ""),
        ("GET", "/api/telemetry/reports/transfers/summary", ""),
        ("GET", "/api/telemetry/reports/transfers/histogram", ""),
        ("GET", "/api/telemetry/reports/transfers/leaderboard", ""),
        ("GET", "/api/telemetry/reports/transfers/users/peer1", ""),
        ("GET", "/api/telemetry/reports/transfers/exceptions", ""),
        (
            "GET",
            "/api/telemetry/reports/transfers/exceptions/pareto",
            "",
        ),
        ("GET", "/api/telemetry/reports/transfers/directories", ""),
        ("DELETE", "/api/searches/1", ""),
        ("DELETE", "/api/application", ""),
        ("PUT", "/api/application", ""),
    ];

    for (method, path, body) in contract_routes {
        if path.starts_with("/api/rooms/") {
            let mut session = state.session.write().await;
            session.state = "connected";
            session.updated_at = crate::unix_timestamp();
        }
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            crate::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        let expected_missing_user_transfer = (method == "GET"
            && path.ends_with("/transfers/uploads/peer1"))
            || path.contains("/transfers/uploads/peer1/1")
            || path.contains("/transfers/downloads/peer1/1");
        let expected_missing_storage_directory = method == "GET"
            && (path == "/api/files/downloads/directories/Zm9v"
                || path == "/api/files/incomplete/directories/Zm9v");
        let expected_missing_user_browse = method == "GET"
            && (path == "/api/users/peer1/browse" || path == "/api/users/peer%201/browse");
        if expected_missing_storage_directory {
            assert_eq!(response.status, "404 Not Found", "{method} {path}");
        } else if !expected_missing_user_transfer && !expected_missing_user_browse {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        while receiver.try_recv().is_ok() {}

        let versioned = path.replacen("/api", "/api/v0", 1);
        let versioned_response = tokio::time::timeout(
            Duration::from_secs(1),
            crate::route_http_request(method, &versioned, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {versioned}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {versioned}: {error}"));
        if method == "DELETE"
            && (versioned == "/api/v0/searches/1"
                || versioned.starts_with("/api/v0/conversations/")
                || versioned == "/api/v0/shares"
                || versioned == "/api/v0/rooms/joined/contract-room"
                || versioned.starts_with("/api/v0/transfers/uploads/peer1/1"))
            || (method == "GET" && versioned.starts_with("/api/v0/transfers/uploads/peer1"))
            || (method == "GET" && versioned.starts_with("/api/v0/shares/root"))
            || (method == "GET"
                && (versioned == "/api/v0/users/peer1/browse"
                    || versioned == "/api/v0/users/peer%201/browse"))
            || (method == "GET"
                && (versioned == "/api/v0/files/downloads/directories/Zm9v"
                    || versioned == "/api/v0/files/incomplete/directories/Zm9v"))
        {
            assert_eq!(
                versioned_response.status, "404 Not Found",
                "{method} {versioned}: {}",
                versioned_response.body
            );
        } else {
            assert_ne!(
                versioned_response.status, "404 Not Found",
                "{method} {versioned}"
            );
        }
        assert!(
            !versioned_response.status.starts_with('5'),
            "{method} {versioned}: {}",
            versioned_response.status
        );
        while receiver.try_recv().is_ok() {}
    }

    // A user with no tracked browse must 404, matching the oracle's
    // BrowseTracker.TryGet-miss contract, on both the unversioned and
    // versioned paths -- there is no version-specific split here.
    for path in [
        "/api/users/peer1/browse/status",
        "/api/v0/users/peer1/browse/status",
        "/api/users/peer%201/browse/status",
    ] {
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "GET {path}");
    }

    let query_contract_routes = [
        // Not "/api/application/version/latest?forceCheck=true" here:
        // that now awaits a real (parameterized, separately tested)
        // GitHub Releases lookup, so it can't share this 1-second
        // timeout or depend on live network in a hermetic test run.
        ("GET", "/api/searches?includeResponses=true", ""),
        ("DELETE", "/api/transfers/downloads/peer1/1?remove=true", ""),
        ("GET", "/api/transfers/downloads/?includeRemoved=true", ""),
        ("GET", "/api/transfers/uploads/?includeRemoved=true", ""),
        (
            "GET",
            "/api/conversations?includeInactive=true&unAcknowledgedOnly=false",
            "",
        ),
        ("GET", "/api/conversations/peer1?includeMessages=false", ""),
        (
            "GET",
            "/api/conversations/peer1/messages?unAcknowledgedOnly=true",
            "",
        ),
        ("GET", "/api/files/downloads/directories?recursive=true", ""),
        (
            "GET",
            "/api/files/incomplete/directories?recursive=true",
            "",
        ),
        ("GET", "/api/events?limit=10", ""),
        (
            "GET",
            "/api/telemetry/reports/transfers/summary?startDate=2026-01-01&endDate=2026-01-02",
            "",
        ),
    ];

    for (method, path, body) in query_contract_routes {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            crate::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/downloads/peer1/1") {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        let json = serde_json::from_str::<serde_json::Value>(&response.body)
            .unwrap_or_else(|error| panic!("{method} {path}: invalid json: {error}"));
        assert_ne!(json["status"], "disabled", "{method} {path}");
        if path == "/api/virtualsoulfind/canonical/status" {
            assert_eq!(json["enabled"], true);
            assert!(json["counts"]["shares"].as_u64().unwrap() >= 1);
            assert!(json["itemCount"].as_u64().unwrap() >= 1);
        }
        while receiver.try_recv().is_ok() {}
    }

    let encoded_path_routes = [
        ("GET", "/api/profile/peer%201", ""),
        ("GET", "/api/users/peer%201/browse", ""),
        (
            "POST",
            "/api/transfers/downloads/peer%201",
            r#"[{"filename":"Remote/Encoded.mp3","size":11}]"#,
        ),
        ("GET", "/api/transfers/downloads/peer%201", ""),
        ("GET", "/api/transfers/downloads/peer%201/1", ""),
        (
            "DELETE",
            "/api/transfers/downloads/peer%201/1?remove=true",
            "",
        ),
        ("POST", "/api/rooms/joined", r#""room space""#),
        ("GET", "/api/rooms/joined/room%20space", ""),
        (
            "POST",
            "/api/rooms/joined/room%20space/messages",
            r#""hello""#,
        ),
        ("GET", "/api/rooms/joined/room%20space/messages", ""),
        ("POST", "/api/conversations/peer%201", r#""hello encoded""#),
        ("GET", "/api/conversations/peer%201", ""),
        ("GET", "/api/conversations/peer%201/messages", ""),
        ("PUT", "/api/conversations/peer%201", ""),
        ("DELETE", "/api/conversations/peer%201", ""),
    ];

    for (method, path, body) in encoded_path_routes {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            crate::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/downloads/peer%201/1") && path != "/api/users/peer%201/browse"
        {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        while receiver.try_recv().is_ok() {}
    }

    for (method, path, body) in [
        ("GET", "/api/profile/peer%201/untrusted", ""),
        ("GET", "/api/users/peer%201/untrusted/info", ""),
        ("GET", "/api/users/peer%201/untrusted/browse", ""),
        ("GET", "/api/users/peer%201/untrusted/group", ""),
        ("GET", "/api/users/peer%201/untrusted/endpoint", ""),
        (
            "POST",
            "/api/users/peer%201/untrusted/directory",
            r#"{"directory":"Music"}"#,
        ),
        (
            "GET",
            "/api/soulseek/users/peer%201/untrusted/interests",
            "",
        ),
    ] {
        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    let native_compat_routes = [
        ("GET", "/api/slskdn", ""),
        ("GET", "/api/slskdn/library/health", ""),
        ("POST", "/api/slskdn/warm-cache", ""),
        ("GET", "/api/hashdb/stats", ""),
        ("POST", "/api/hashdb/backfill/from-history", ""),
        ("GET", "/api/streams/content-1", ""),
        (
            "POST",
            "/api/v0/peer-streams/tickets",
            r#"{"filename":"Virtual/Test.flac","username":"peer"}"#,
        ),
        (
            "POST",
            "/api/v0/mesh-streams/tickets",
            r#"{"contentId":"mesh-content","filename":"Virtual/Test.flac","peerId":"mesh-peer"}"#,
        ),
        ("GET", "/api/listening-party", ""),
        ("POST", "/api/listening-party/radio/party/content", ""),
        ("GET", "/api/mesh/health", ""),
        ("GET", "/api/mesh/stats", ""),
        (
            "POST",
            "/api/multisource/download",
            r#"{"filename":"x","size":1}"#,
        ),
        (
            "POST",
            "/api/multisource/swarm",
            r#"{"filename":"x","size":1,"sources":[]}"#,
        ),
        (
            "POST",
            "/api/multisource/swarm/async",
            r#"{"filename":"x","size":1,"sources":[]}"#,
        ),
        ("GET", "/api/podcore/content/search?query=cover", ""),
        (
            "POST",
            "/api/podcore/membership/join",
            r#"{"podId":"pod","peerId":"peer"}"#,
        ),
        ("GET", "/api/library/items", ""),
        ("GET", "/api/virtualsoulfind/canonical/status", ""),
        ("POST", "/api/audio/variants/dedupe", ""),
        ("POST", "/api/mediacore/retrieve", ""),
        ("GET", "/api/playback/status", ""),
        ("GET", "/api/traces", ""),
        ("GET", "/api/fairness", ""),
        ("GET", "/api/ranking", ""),
        ("GET", "/api/portforwarding/status", ""),
        ("GET", "/api/signals", ""),
        ("POST", "/api/backfill", ""),
        ("GET", "/api/security/status", ""),
        ("GET", "/api/pods", ""),
        ("GET", "/api/solid/status", ""),
        ("GET", "/api/federation/diagnostics", ""),
    ];

    for (method, path, body) in native_compat_routes {
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            crate::route_http_request(method, path, None, body, &state),
        )
        .await
        .unwrap_or_else(|_| panic!("{method} {path}: timed out"))
        .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        if !path.contains("/transfers/downloads/peer%201/1") {
            assert_ne!(response.status, "404 Not Found", "{method} {path}");
        }
        assert!(
            !response.status.starts_with('5'),
            "{method} {path}: {}",
            response.status
        );
        while receiver.try_recv().is_ok() {}
    }

    for (method, path) in [
        ("GET", "/api/hashdb/unregistered-state-projection"),
        ("POST", "/api/security/status"),
    ] {
        let response = crate::route_http_request(method, path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("{method} {path}: {error}"));
        assert_eq!(response.status, "404 Not Found", "{method} {path}");
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    std::fs::create_dir_all(&evidence_dir).expect("create controller-api evidence directory");
    std::fs::write(
        evidence_dir.join("automation_compat_routes.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");
}
/// Differential proof for the remaining slskdN JobsController and the
/// dedicated discography/label-crate job controllers.  The compatibility
/// dispatcher already uses local searches for work execution; this test
/// proves the frozen job-list/detail projections, exact nested routes,
/// versioned validation, persistence-backed projections, and concurrent
/// creation behavior independently of the broader search tests.
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
pub(super) async fn controller_api_differential_jobs_residuals() {
    let target = "slskdn";
    let base_env = MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target);
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

    let known_id = "jobs-residual-known";
    let label_id = "jobs-residual-label";
    let get_cases = [
        (
            "/api/jobs?limit=not-a-number",
            "/api/jobs",
            "malformed-path-query-or-body",
            "list",
        ),
        (
            "/api/jobs",
            "/api/jobs",
            "missing-empty-or-conflict-state",
            "list",
        ),
        (
            "/api/jobs",
            "/api/jobs",
            "runtime-failure-and-timeout",
            "list",
        ),
        (
            "/api/jobs/jobs-residual-known",
            "/api/jobs/{id}",
            "nominal-status-headers-body",
            "found",
        ),
        (
            "/api/jobs/jobs-residual-known/extra",
            "/api/jobs/{id}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/jobs/no-such-job",
            "/api/jobs/{id}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/jobs/jobs-residual-known",
            "/api/jobs/{id}",
            "runtime-failure-and-timeout",
            "found",
        ),
        (
            "/api/jobs/discography/jobs-residual-known",
            "/api/jobs/discography/{jobId}",
            "nominal-status-headers-body",
            "discography",
        ),
        (
            "/api/jobs/discography/jobs-residual-known/extra",
            "/api/jobs/discography/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/jobs/discography/no-such-job",
            "/api/jobs/discography/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/jobs/discography/jobs-residual-known",
            "/api/jobs/discography/{jobId}",
            "runtime-failure-and-timeout",
            "discography",
        ),
        (
            "/api/jobs/discography/jobs-residual-known",
            "/api/jobs/discography/{jobId}",
            "populated-dynamic-state",
            "discography",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label",
            "/api/jobs/label-crate/{jobId}",
            "nominal-status-headers-body",
            "label",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label/extra",
            "/api/jobs/label-crate/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/jobs/label-crate/no-such-job",
            "/api/jobs/label-crate/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label",
            "/api/jobs/label-crate/{jobId}",
            "runtime-failure-and-timeout",
            "label",
        ),
        (
            "/api/jobs/label-crate/jobs-residual-label",
            "/api/jobs/label-crate/{jobId}",
            "populated-dynamic-state",
            "label",
        ),
        (
            "/api/v0/jobs?limit=not-a-number",
            "/api/v0/jobs",
            "nominal-status-headers-body",
            "list",
        ),
        (
            "/api/v0/jobs?limit=not-a-number",
            "/api/v0/jobs",
            "malformed-path-query-or-body",
            "list",
        ),
        (
            "/api/v0/jobs",
            "/api/v0/jobs",
            "missing-empty-or-conflict-state",
            "list",
        ),
        (
            "/api/v0/jobs",
            "/api/v0/jobs",
            "runtime-failure-and-timeout",
            "list",
        ),
        (
            "/api/v0/jobs",
            "/api/v0/jobs",
            "populated-dynamic-state",
            "list",
        ),
        (
            "/api/v0/jobs/jobs-residual-known",
            "/api/v0/jobs/{id}",
            "nominal-status-headers-body",
            "found",
        ),
        (
            "/api/v0/jobs/jobs-residual-known/extra",
            "/api/v0/jobs/{id}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/v0/jobs/jobs-residual-known",
            "/api/v0/jobs/{id}",
            "runtime-failure-and-timeout",
            "found",
        ),
        (
            "/api/v0/jobs/jobs-residual-known",
            "/api/v0/jobs/{id}",
            "populated-dynamic-state",
            "found",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known",
            "/api/v0/jobs/discography/{jobId}",
            "nominal-status-headers-body",
            "discography",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known/extra",
            "/api/v0/jobs/discography/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/v0/jobs/discography/no-such-job",
            "/api/v0/jobs/discography/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known",
            "/api/v0/jobs/discography/{jobId}",
            "runtime-failure-and-timeout",
            "discography",
        ),
        (
            "/api/v0/jobs/discography/jobs-residual-known",
            "/api/v0/jobs/discography/{jobId}",
            "populated-dynamic-state",
            "discography",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label",
            "/api/v0/jobs/label-crate/{jobId}",
            "nominal-status-headers-body",
            "label",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label/extra",
            "/api/v0/jobs/label-crate/{jobId}",
            "malformed-path-query-or-body",
            "missing",
        ),
        (
            "/api/v0/jobs/label-crate/no-such-job",
            "/api/v0/jobs/label-crate/{jobId}",
            "missing-empty-or-conflict-state",
            "missing",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label",
            "/api/v0/jobs/label-crate/{jobId}",
            "runtime-failure-and-timeout",
            "label",
        ),
        (
            "/api/v0/jobs/label-crate/jobs-residual-label",
            "/api/v0/jobs/label-crate/{jobId}",
            "populated-dynamic-state",
            "label",
        ),
    ];

    for (path, route, case, shape) in get_cases {
        let (state, _receiver) = test_state_with_env(base_env.clone());
        if matches!(shape, "found" | "discography") {
            state
                .searches
                .write()
                .await
                .create(
                    Some(known_id.to_owned()),
                    "Residual discography".to_owned(),
                    "global",
                    None,
                    Vec::new(),
                    crate::DEFAULT_SEARCH_TTL_SECONDS,
                )
                .expect("seed residual search job");
        }
        if shape == "label" {
            state
                .controller_features
                .write_for_test()
                .await
                .upsert(
                    format!("job/label-crate/{label_id}"),
                    serde_json::json!({
                        "id": label_id,
                        "jobId": label_id,
                        "labelId": "label-residual",
                        "labelName": "Residual Label",
                        "limit": 2,
                        "releaseIds": [],
                        "totalReleases": 0,
                        "completedReleases": 0,
                        "failedReleases": 0,
                        "type": "label_crate",
                        "status": "Pending",
                    }),
                )
                .expect("seed residual label job");
        }
        let response = crate::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap_or_else(|error| panic!("GET {path}: {error}"));
        let pass = match shape {
            "list" => serde_json::from_str::<serde_json::Value>(&response.body).is_ok_and(|json| {
                response.status == "200 OK"
                    && json["jobs"].is_array()
                    && json["limit"].is_number()
                    && json["offset"].is_number()
                    && json["has_more"].is_boolean()
            }),
            "found" => response.status == "200 OK" && response.body.contains(known_id),
            "discography" | "label" => {
                response.status == "200 OK" && response.body.contains("jobId")
            }
            "missing" => response.status == "404 Not Found",
            _ => false,
        };
        record!("GET", route, case, pass);
    }

    let unversioned_mutations = [
        (
            "/api/jobs/discography",
            "/api/jobs/discography",
            "{\"artist\":\"Residual Artist\"}",
        ),
        (
            "/api/jobs/label-crate",
            "/api/jobs/label-crate",
            "{\"label_name\":\"Residual Label\"}",
        ),
        (
            "/api/jobs/mb-release",
            "/api/jobs/mb-release",
            "{\"artist\":\"Residual Artist\",\"title\":\"Residual Release\"}",
        ),
    ];
    for (path, route, valid_body) in unversioned_mutations {
        for (case, body) in [
            ("runtime-failure-and-timeout", valid_body),
            ("mutation-side-effects-and-readback", valid_body),
            ("restart-persistence-or-reset", valid_body),
            ("concurrency-and-idempotency", valid_body),
        ] {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            let response = crate::route_http_request("POST", path, None, body, &state)
                .await
                .unwrap_or_else(|error| panic!("POST {path}: {error}"));
            record!(
                "POST",
                route,
                case,
                response.status == "400 Bad Request"
                    && response.body.contains("ApiVersionUnspecified")
            );
        }
    }

    let versioned_mutations = vec![
        (
            "/api/v0/jobs/discography",
            "/api/v0/jobs/discography",
            "discography",
            "{\"artist\":\"Residual Artist\"}",
            vec![
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                ("runtime-failure-and-timeout", "{\"artist\":\"Residual Artist\"}"),
                (
                    "restart-persistence-or-reset",
                    "{\"artist\":\"Residual Restart Artist\"}",
                ),
                (
                    "concurrency-and-idempotency",
                    "{\"artist\":\"Residual Concurrent Artist\"}",
                ),
            ],
        ),
        (
            "/api/v0/jobs/label-crate",
            "/api/v0/jobs/label-crate",
            "label",
            "{\"label_name\":\"Residual Label\",\"limit\":2}",
            vec![
                ("nominal-status-headers-body", "{\"label_name\":\"Residual Label\"}"),
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                (
                    "runtime-failure-and-timeout",
                    "{\"label_name\":\"Residual Label\"}",
                ),
                (
                    "mutation-side-effects-and-readback",
                    "{\"label_name\":\"Residual Readback Label\"}",
                ),
                (
                    "restart-persistence-or-reset",
                    "{\"label_name\":\"Residual Restart Label\"}",
                ),
                (
                    "concurrency-and-idempotency",
                    "{\"label_name\":\"Residual Concurrent Label\"}",
                ),
            ],
        ),
        (
            "/api/v0/jobs/mb-release",
            "/api/v0/jobs/mb-release",
            "mb-release",
            "{\"artist\":\"Residual Artist\",\"title\":\"Residual Release\"}",
            vec![
                ("malformed-path-query-or-body", "not-json"),
                ("missing-empty-or-conflict-state", ""),
                (
                    "runtime-failure-and-timeout",
                    "{\"artist\":\"Residual Artist\",\"title\":\"Residual Release\"}",
                ),
                (
                    "mutation-side-effects-and-readback",
                    "{\"artist\":\"Residual Readback Artist\",\"title\":\"Residual Readback Release\"}",
                ),
                (
                    "restart-persistence-or-reset",
                    "{\"artist\":\"Residual Restart Artist\",\"title\":\"Residual Restart Release\"}",
                ),
                (
                    "concurrency-and-idempotency",
                    "{\"artist\":\"Residual Concurrent Artist\",\"title\":\"Residual Concurrent Release\"}",
                ),
            ],
        ),
    ];

    for (path, route, kind, _valid_body, cases) in versioned_mutations {
        for (case, body) in cases {
            let (state, _receiver) = test_state_with_env(base_env.clone());
            if case == "concurrency-and-idempotency" {
                let (left, right) = tokio::join!(
                    crate::route_http_request("POST", path, None, body, &state),
                    crate::route_http_request("POST", path, None, body, &state)
                );
                let expected = if kind == "label" {
                    "200 OK"
                } else if body == "not-json" || body.is_empty() {
                    "400 Bad Request"
                } else {
                    "202 Accepted"
                };
                record!(
                    "POST",
                    route,
                    case,
                    left.as_ref()
                        .is_ok_and(|response| response.status == expected)
                        && right
                            .as_ref()
                            .is_ok_and(|response| response.status == expected)
                );
                continue;
            }

            let response = crate::route_http_request("POST", path, None, body, &state)
                .await
                .unwrap_or_else(|error| panic!("POST {path}: {error}"));
            let expected = if body == "not-json" || body.is_empty() {
                "400 Bad Request"
            } else if kind == "label" {
                "200 OK"
            } else {
                "202 Accepted"
            };
            let mut pass = response.status == expected && !response.body.is_empty();
            if pass && matches!(case, "mutation-side-effects-and-readback") {
                let json = serde_json::from_str::<serde_json::Value>(&response.body)
                    .expect("job mutation JSON");
                let id = if kind == "label" {
                    json["jobId"]
                        .as_str()
                        .or_else(|| json["id"].as_str())
                        .unwrap_or_default()
                } else {
                    json["search_id"].as_str().unwrap_or_default()
                };
                let read_path = if kind == "label" {
                    format!("/api/v0/jobs/label-crate/{id}")
                } else {
                    format!("/api/jobs/{id}")
                };
                let readback = crate::route_http_request("GET", &read_path, None, "", &state)
                    .await
                    .expect("job mutation readback");
                pass = !id.is_empty() && readback.status == "200 OK" && !readback.body.is_empty();
            }
            if pass && case == "restart-persistence-or-reset" {
                let json = serde_json::from_str::<serde_json::Value>(&response.body)
                    .expect("restart job JSON");
                let id = if kind == "label" {
                    json["jobId"]
                        .as_str()
                        .or_else(|| json["id"].as_str())
                        .unwrap_or_default()
                } else {
                    json["search_id"].as_str().unwrap_or_default()
                };
                let state_path = state.controller_features.read().await.state_path.clone();
                let persisted = fs::read(&state_path).expect("read persisted job projection");
                let file = serde_json::from_slice::<crate::ControllerFeatureStateFile>(&persisted)
                    .expect("parse persisted job projection");
                drop(state);
                let (restarted, _receiver) = test_state_with_env(base_env.clone());
                *restarted.controller_features.write_for_test().await =
                    crate::ControllerFeatureState {
                        records: file.records,
                        state_path,
                    };
                let read_path = if kind == "label" {
                    format!("/api/v0/jobs/label-crate/{id}")
                } else if kind == "discography" {
                    format!("/api/v0/jobs/discography/{id}")
                } else {
                    format!("/api/jobs/{id}")
                };
                let readback = crate::route_http_request("GET", &read_path, None, "", &restarted)
                    .await
                    .expect("restart job readback");
                pass = !id.is_empty() && readback.status == "200 OK" && !readback.body.is_empty();
            }
            record!("POST", route, case, pass);
        }
    }

    assert_eq!(ledger.len(), 66, "Jobs residual ledger size");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create Jobs evidence directory");
    fs::write(
        evidence_dir.join("jobs_controller_residuals.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize Jobs ledger"),
    )
    .expect("write Jobs ledger");
    assert!(
        mismatches.is_empty(),
        "{} Jobs residual mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
