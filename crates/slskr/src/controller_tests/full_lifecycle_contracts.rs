//! Controller full lifecycle contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn initiate_graceful_shutdown_disconnects_the_session() {
    // Matches the oracle's StopAsync teardown (Client.Disconnect before
    // exit) -- this is the logic a SIGTERM/SIGINT/SIGQUIT handler runs;
    // registering real OS signals isn't exercised here since raising
    // them would affect the whole shared test process.
    let (state, mut receiver) = test_state();
    {
        let mut session = state.session.write().await;
        session.state = "connected";
    }
    crate::initiate_graceful_shutdown(&state).await;
    let command = receiver.recv().await.expect("session command sent");
    assert!(matches!(command, crate::SessionCommand::Disconnect));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn transient_credential_stores_refuse_bursts_at_live_capacity() {
    let mut expired_oauth = crate::OAuthStateStore::with_max_records(1);
    let expired_state = expired_oauth
        .issue("spotify", "http://localhost/callback", 0)
        .unwrap();
    assert!(expired_oauth.consume("spotify", &expired_state).is_none());

    let mut oauth = crate::OAuthStateStore::with_max_records(1);
    let state = oauth
        .issue("spotify", "http://localhost/callback", 600)
        .unwrap();
    assert!(oauth
        .issue("spotify", "http://localhost/callback", 600)
        .is_none());
    assert!(oauth.consume("spotify", &state).is_some());
    assert!(oauth
        .issue("spotify", "http://localhost/callback", 600)
        .is_some());

    let mut expired_tickets = crate::PreviewStreamTicketStore::with_max_records(1);
    let (expired_ticket, _) = expired_tickets
        .issue(
            "peer",
            "test",
            "expired".to_owned(),
            "expired.flac".to_owned(),
            None,
            0,
            "audio/flac".to_owned(),
            0,
        )
        .unwrap();
    assert!(expired_tickets.get(&expired_ticket).is_none());

    let mut tickets = crate::PreviewStreamTicketStore::with_max_records(1);
    assert!(tickets
        .issue(
            "peer",
            "test",
            "content-1".to_owned(),
            "song.flac".to_owned(),
            Some("alice".to_owned()),
            123,
            "audio/flac".to_owned(),
            120,
        )
        .is_some());
    assert!(tickets
        .issue(
            "peer",
            "test",
            "content-2".to_owned(),
            "other.flac".to_owned(),
            Some("bob".to_owned()),
            456,
            "audio/flac".to_owned(),
            120,
        )
        .is_none());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn runtime_control_routes_roll_back_when_persistence_fails() {
    for (method, path, body, seed_runtime, seed_relay) in [
        ("PUT", "/api/application", "{}", "", false),
        ("DELETE", "/api/application", "", "restart", false),
        ("POST", "/api/application/gc", "", "", false),
        (
            "PUT",
            "/api/config/preferences",
            r#"{"autoreplace_enabled":true}"#,
            "",
            false,
        ),
        ("PUT", "/api/autoreplace/enable", "", "", false),
        ("PUT", "/api/autoreplace/disable", "", "autoreplace", false),
        ("PUT", "/api/relay", r#"{"enabled":true}"#, "", false),
        ("POST", "/api/relay", r#"{"enabled":true}"#, "", false),
        ("DELETE", "/api/relay", "", "", true),
        ("PUT", "/api/relay/agent", r#"{"enabled":true}"#, "", false),
        ("DELETE", "/api/relay/agent", "", "relay_agent", false),
        ("POST", "/api/v0/bridge/start", "", "", false),
        ("POST", "/api/v0/bridge/stop", "", "bridge", false),
        (
            "PUT",
            "/api/v0/bridge/admin/config",
            r#"{"enabled":true}"#,
            "",
            false,
        ),
        ("POST", "/api/options", r#"{}"#, "", false),
        (
            "POST",
            "/api/songid/runs",
            r#"{"source":"route-audit"}"#,
            "",
            false,
        ),
        (
            "POST",
            "/api/integrations/lidarr/wanted/sync",
            "",
            "",
            false,
        ),
        ("POST", "/api/profile/invite", "", "", false),
        ("POST", "/api/slskdn/warm-cache", "", "", false),
        ("POST", "/api/backfill", "", "", false),
    ] {
        let db = crate::persistence::DatabaseManager::in_memory()
            .await
            .expect("in-memory db");
        let (state, _receiver) = test_state_with_env_parts(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", "legacy")
                .with("SLSKR_PERSISTENCE_ENABLED", "true"),
            crate::SearchStore::new(),
            Some(db.clone()),
        );
        match seed_runtime {
            "restart" => {
                state.runtime.write().await.set_restart_requested(true);
            }
            "autoreplace" => {
                state.runtime.write().await.set_autoreplace(true);
            }
            "relay_agent" => {
                state.runtime.write().await.set_relay_agent(true);
            }
            "bridge" => {
                state.runtime.write().await.set_bridge_running(true, false);
            }
            _ => {}
        }
        if seed_relay {
            state.relay.write().await.set_enabled(true);
        }
        let previous_runtime = state.runtime.read().await.clone();
        let previous_relay = state.relay.read().await.clone();
        db.close_for_test().await;

        let response = crate::route_http_request(method, path, None, body, &state)
            .await
            .expect("failed runtime compatibility persistence response");
        assert_eq!(
            response.status, "503 Service Unavailable",
            "{method} {path}"
        );
        assert!(
            response
                .body
                .contains("runtime compatibility persistence failed"),
            "{method} {path}"
        );
        assert_eq!(
            *state.runtime.read().await,
            previous_runtime,
            "{method} {path}"
        );
        assert_eq!(*state.relay.read().await, previous_relay, "{method} {path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn runtime_compat_state_persists_and_rehydrates_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default()
            .with("SLSKR_PERSISTENCE_ENABLED", "true")
            .with("SLSKR_CONTROLLER_PROFILE", "legacy")
            .with("SLSKR_REMOTE_CONFIGURATION", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    let restart = crate::route_http_request("PUT", "/api/application", None, "{}", &state)
        .await
        .expect("restart request");
    assert_eq!(restart.status, "204 No Content");
    let gc = crate::route_http_request("POST", "/api/application/gc", None, "", &state)
        .await
        .expect("gc");
    assert_eq!(gc.status, "200 OK");
    let autoreplace = crate::route_http_request("PUT", "/api/autoreplace/enable", None, "", &state)
        .await
        .expect("autoreplace");
    assert_eq!(autoreplace.status, "200 OK");
    let relay =
        crate::route_http_request("POST", "/api/relay", None, r#"{"enabled":true}"#, &state)
            .await
            .expect("relay");
    assert_eq!(relay.status, "200 OK");
    let relay_agent = crate::route_http_request(
        "PUT",
        "/api/relay/agent",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("relay agent");
    assert_eq!(relay_agent.status, "200 OK");
    assert!(
        db.get_runtime_compat_state()
            .await
            .unwrap()
            .unwrap()
            .relay_agent_enabled
    );
    let relay_agent_disabled =
        crate::route_http_request("DELETE", "/api/relay/agent", None, "", &state)
            .await
            .expect("disable relay agent");
    assert_eq!(relay_agent_disabled.status, "200 OK");
    let bridge_config = crate::route_http_request(
        "PUT",
        "/api/v0/bridge/admin/config",
        None,
        r#"{"enabled":true}"#,
        &state,
    )
    .await
    .expect("bridge config");
    assert_eq!(bridge_config.status, "200 OK");
    let invite = crate::route_http_request("POST", "/api/profile/invite", None, "{}", &state)
        .await
        .expect("invite");
    assert_eq!(invite.status, "201 Created");
    let warm_cache =
        crate::route_http_request("POST", "/api/slskdn/warm-cache", None, "{}", &state)
            .await
            .expect("warm cache");
    assert_eq!(warm_cache.status, "202 Accepted");
    let songid = crate::route_http_request(
        "POST",
        "/api/songid/runs",
        None,
        r#"{"source":"route-audit"}"#,
        &state,
    )
    .await
    .expect("songid");
    assert_eq!(songid.status, "202 Accepted");
    let backfill = crate::route_http_request("POST", "/api/backfill", None, "{}", &state)
        .await
        .expect("backfill");
    assert_eq!(backfill.status, "202 Accepted");
    let options = crate::route_http_request(
        "PATCH",
        "/api/options",
        None,
        r#"{"soulseek":{"listenPort":50300}}"#,
        &state,
    )
    .await
    .expect("options patch");
    assert_eq!(options.status, "200 OK");
    let options_json = serde_json::from_str::<serde_json::Value>(&options.body).unwrap();
    assert_eq!(options_json["soulseek"]["listenPort"], 50300);
    let options_upload =
        crate::route_http_request("PUT", "/api/options/yaml", None, r#""app: {}""#, &state)
            .await
            .expect("options upload");
    assert_eq!(options_upload.status, "200 OK");
    assert!(options_upload.body.is_empty());
    let options_validate = crate::route_http_request(
        "POST",
        "/api/options/yaml/validate",
        None,
        r#""app: {}""#,
        &state,
    )
    .await
    .expect("options validate");
    assert_eq!(options_validate.status, "200 OK");
    assert!(options_validate.content_type.is_empty());
    assert!(options_validate.body.is_empty());
    let manual_import = crate::route_http_request(
        "POST",
        "/api/integrations/lidarr/manualimport",
        None,
        r#"{"artist":"Artist","title":"Album","kind":"Audio"}"#,
        &state,
    )
    .await
    .expect("Lidarr manual import");
    assert_eq!(manual_import.status, "202 Accepted");
    assert_eq!(db.list_library_items(10, 0).await.unwrap().len(), 1);

    let persisted = db
        .get_runtime_compat_state()
        .await
        .expect("get runtime state")
        .expect("runtime state persisted");
    assert!(!persisted.application_restart_requested);
    assert!(!persisted.bridge_running);
    assert!(state.runtime.read().await.application_restart_requested);
    assert_eq!(persisted.gc_runs, 1);
    assert!(persisted.autoreplace_enabled);
    assert!(persisted.relay_enabled);
    assert!(!persisted.relay_agent_enabled);
    assert_eq!(persisted.bridge_config_updates, 1);
    assert_eq!(persisted.options_updates, 0);
    assert_eq!(persisted.options_yaml_uploads, 0);
    assert_eq!(persisted.options_yaml_validations, 0);
    assert_eq!(persisted.profile_invites_created, 1);
    assert_eq!(persisted.cache_warm_runs, 1);
    assert_eq!(persisted.songid_runs, 1);
    assert_eq!(persisted.backfill_runs, 1);
    assert_eq!(persisted.lidarr_manual_imports, 1);

    let rehydrated_relay = crate::RelayState::from_persisted(&persisted);
    let rehydrated_runtime = crate::RuntimeCompatState::from_persisted(&persisted);
    assert!(rehydrated_relay.enabled);
    let runtime_json = rehydrated_runtime.json_value();
    assert_eq!(runtime_json["pendingRestart"], false);
    assert_eq!(runtime_json["autoreplaceEnabled"], true);
    assert_eq!(runtime_json["relayAgentEnabled"], false);
    assert_eq!(runtime_json["bridgeConfigUpdates"], 1);
    assert_eq!(runtime_json["optionsUpdates"], 0);
    assert_eq!(runtime_json["optionsYamlUploads"], 0);
    assert_eq!(runtime_json["optionsYamlValidations"], 0);
    assert_eq!(runtime_json["profileInvitesCreated"], 1);
    assert_eq!(runtime_json["cacheWarmRuns"], 1);
    assert_eq!(runtime_json["songidRuns"], 1);
    assert_eq!(runtime_json["backfillRuns"], 1);
    assert_eq!(runtime_json["lidarrManualImports"], 1);

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("database stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["runtimeState"], 1);
    assert_eq!(stats_json["persisted"]["runtimeState"], 1);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn bounded_store_ids_wrap_without_collisions() {
    let mut events = crate::EventStore::new(4);
    events.next_id = u64::MAX;
    let max_event = events.record("test", "max", None);
    let wrapped_event = events.record("test", "wrapped", None);
    assert_eq!((max_event.id, wrapped_event.id), (u64::MAX, 1));

    let mut messages = crate::MessageStore::with_max_records(4);
    messages.next_id = u64::MAX;
    let max_message = messages.add("peer".to_owned(), "inbound", "max".to_owned());
    let wrapped_message = messages.add("peer".to_owned(), "inbound", "wrapped".to_owned());
    assert_eq!((max_message.id, wrapped_message.id), (u64::MAX, 1));

    let mut contacts = crate::ContactStore::with_max_records(4);
    contacts.next_id = u64::MAX;
    let (max_contact, _) = contacts.create("Alice".to_owned()).unwrap();
    let (wrapped_contact, _) = contacts.create("Bob".to_owned()).unwrap();
    assert_eq!(max_contact.id, format!("contact-{}", u64::MAX));
    assert_eq!(wrapped_contact.id, "contact-1");

    let mut groups = crate::ShareGroupStore::with_limits(4, 4);
    groups.next_id = u64::MAX;
    let max_group = groups.create("Max".to_owned(), String::new()).unwrap();
    let wrapped_group = groups.create("Wrapped".to_owned(), String::new()).unwrap();
    assert_eq!(max_group.id, format!("sg-{}", u64::MAX));
    assert_eq!(wrapped_group.id, "sg-1");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn bounded_content_store_ids_wrap_without_collisions() {
    let mut notes = crate::UserNoteStore::new();
    notes.next_id = u64::MAX;
    assert_eq!(
        notes
            .create("alice".to_owned(), "max".to_owned())
            .unwrap()
            .id,
        format!("note-{}", u64::MAX)
    );
    assert_eq!(
        notes
            .create("bob".to_owned(), "wrapped".to_owned())
            .unwrap()
            .id,
        "note-1"
    );

    let mut interests = crate::InterestStore::new();
    interests.next_id = u64::MAX;
    assert_eq!(
        interests.add_liked("max".to_owned()).unwrap().0.id,
        format!("liked-{}", u64::MAX)
    );
    assert_eq!(
        interests.add_hated("wrapped".to_owned()).unwrap().0.id,
        "hated-1"
    );

    let mut grants = crate::ShareGrantStore::new();
    grants.next_id = u64::MAX;
    assert_eq!(
        grants
            .create_with_contract(None, "one".to_owned(), "alice".to_owned())
            .unwrap()
            .0
            .id,
        format!("grant-{}", u64::MAX)
    );
    assert_eq!(
        grants
            .create_with_contract(None, "two".to_owned(), "bob".to_owned())
            .unwrap()
            .0
            .id,
        "grant-1"
    );

    let mut library = crate::LibraryStore::new();
    library.next_id = u64::MAX;
    assert_eq!(
        library
            .create("artist".to_owned(), "max".to_owned(), "audio".to_owned())
            .unwrap()
            .id,
        format!("lib-{}", u64::MAX)
    );
    assert_eq!(
        library
            .create(
                "artist".to_owned(),
                "wrapped".to_owned(),
                "audio".to_owned()
            )
            .unwrap()
            .id,
        "lib-1"
    );
    library.next_health_scan_id = u64::MAX;
    assert_eq!(
        library.create_health_scan("/max".to_owned()).unwrap().id,
        format!("scan-{}", u64::MAX)
    );
    assert_eq!(
        library
            .create_health_scan("/wrapped".to_owned())
            .unwrap()
            .id,
        "scan-1"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn one_shot_user_requests_reject_before_mutation_when_dispatch_is_unavailable() {
    for (path, body) in [
        ("/api/v1/users/friend/stats/request", ""),
        ("/api/v0/users/friend/browse/request", ""),
        (
            "/api/v0/users/friend/browse/folder",
            r#"{"folder":"Remote/Album"}"#,
        ),
    ] {
        let (state, receiver) = test_state();
        state.session.write().await.state = "connected";
        drop(receiver);

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("unavailable dispatch response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("session manager is not running"),
            "{path}"
        );
        assert!(state.browse.read().await.records.is_empty(), "{path}");
    }
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn async_browse_projection_reports_persistence_failure() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );
    let record = state
        .browse
        .write()
        .await
        .request("friend".to_owned())
        .unwrap();
    db.close_for_test().await;

    crate::browse_runtime::persist_browse_projection(&state, &record).await;
    assert!(state
        .session
        .read()
        .await
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("browse persistence")));
    assert!(state.browse.read().await.get("friend").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn one_shot_room_routes_reject_before_mutation_when_dispatch_is_unavailable() {
    let (state, receiver) = test_state();
    drop(receiver);
    let refresh = crate::route_http_request("POST", "/api/v0/rooms/refresh", None, "", &state)
        .await
        .expect("unavailable refresh response");
    assert_eq!(refresh.status, "503 Service Unavailable");
    assert!(refresh.body.contains("session manager is not running"));

    for (path, body) in [
        (
            "/api/v0/rooms/music/messages",
            r#"{"username":"friend","body":"not sent"}"#,
        ),
        ("/api/v0/rooms/joined/music/messages", r#""also not sent""#),
    ] {
        let (state, receiver) = test_state();
        state.rooms.write().await.join("music".to_owned()).unwrap();
        drop(receiver);

        let response = crate::route_http_request("POST", path, None, body, &state)
            .await
            .expect("unavailable room message response");
        assert_eq!(response.status, "503 Service Unavailable", "{path}");
        assert!(
            response.body.contains("session manager is not running"),
            "{path}"
        );
        let rooms = state.rooms.read().await;
        assert!(rooms.records[0].messages.is_empty(), "{path}");
        drop(rooms);
        assert!(
            state
                .events
                .read()
                .await
                .records
                .iter()
                .all(|event| event.kind != "room.message"),
            "{path}"
        );
    }
}
