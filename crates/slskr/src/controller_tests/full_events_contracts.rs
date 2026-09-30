//! Controller full events contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn event_log_persists_and_rehydrates_records() {
    let db = crate::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        crate::SearchStore::new(),
        Some(db.clone()),
    );

    crate::record_event(
        &state,
        "search.started",
        "42",
        Some("query=durable".to_owned()),
    )
    .await;

    let compatibility_event = crate::route_http_request(
        "POST",
        "/api/events/Noop",
        None,
        r#""durable compatibility event""#,
        &state,
    )
    .await
    .expect("record compatibility event");
    assert_eq!(compatibility_event.status, "200 OK");
    let nested_event = crate::route_http_request(
        "POST",
        "/api/events/Noop/extra",
        None,
        r#""must not persist""#,
        &state,
    )
    .await
    .expect("reject nested event route");
    assert_eq!(nested_event.status, "404 Not Found");

    let persisted = db.list_events(10, 0).await.expect("list events");
    assert_eq!(persisted.len(), 2);
    assert!(persisted.iter().any(|record| {
        record.kind == "search.started"
            && record.resource == "42"
            && record.detail.as_deref() == Some("query=durable")
    }));
    assert!(persisted.iter().any(|record| {
        record.kind == "compat.event"
            && record.resource == "Noop"
            && record.detail.as_deref() == Some("durable compatibility event")
    }));

    let rehydrated = crate::EventStore::from_persisted(persisted, crate::EVENT_HISTORY_LIMIT);
    assert_eq!(rehydrated.next_id, 3);
    assert!(rehydrated.json(None).contains("\"topic\":\"searches\""));
    assert!(rehydrated
        .controller_json(Some("topic=searches"))
        .contains("\"type\":\"search.started\""));

    let stats = crate::route_http_request("GET", "/api/admin/database/stats", None, "", &state)
        .await
        .expect("event database stats");
    assert_eq!(stats.status, "200 OK");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["events"], 2);
    assert_eq!(stats_json["persisted"]["events"], 2);
    assert_eq!(stats_json["projections"]["events"], 2);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn event_persistence_failures_roll_back_ingest_and_surface_internal_loss() {
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
    let previous = state.events.read().await.clone();
    db.close_for_test().await;

    let response =
        crate::route_http_request("POST", "/api/events/Noop", None, r#""must fail""#, &state)
            .await
            .expect("failed event persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("event persistence failed"));
    assert_eq!(*state.events.read().await, previous);

    crate::record_event(&state, "internal.test", "resource", None).await;
    assert_eq!(
        state.events.read().await.records.len(),
        previous.records.len() + 1
    );
    let session = state.session.read().await;
    assert!(session
        .last_error
        .as_deref()
        .unwrap_or_default()
        .contains("event persistence failed"));
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn event_store_bounds_live_and_rehydrated_text_fields() {
    let oversized_kind = "é".repeat(crate::MAX_EVENT_KIND_BYTES);
    let oversized_resource = "r".repeat(crate::MAX_EVENT_RESOURCE_BYTES + 1);
    let oversized_detail = "d".repeat(crate::MAX_EVENT_DETAIL_BYTES + 1);

    let mut live = crate::EventStore::new(2);
    let record = live.record(
        oversized_kind.clone(),
        oversized_resource.clone(),
        Some(oversized_detail.clone()),
    );
    assert!(record.kind.len() <= crate::MAX_EVENT_KIND_BYTES);
    assert!(record.kind.is_char_boundary(record.kind.len()));
    assert_eq!(record.resource.len(), crate::MAX_EVENT_RESOURCE_BYTES);
    let expected_detail = format!(
        "<omitted oversized event detail: {} bytes>",
        oversized_detail.len()
    );
    assert_eq!(record.detail.as_deref(), Some(expected_detail.as_str()));

    let rehydrated = crate::EventStore::from_persisted(
        vec![crate::persistence::EventRecord {
            id: 1,
            kind: oversized_kind,
            resource: oversized_resource,
            detail: Some(oversized_detail),
            created_at: 1,
        }],
        2,
    );
    assert!(rehydrated.records[0].kind.len() <= crate::MAX_EVENT_KIND_BYTES);
    assert_eq!(
        rehydrated.records[0].resource.len(),
        crate::MAX_EVENT_RESOURCE_BYTES
    );
    assert!(rehydrated.records[0]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.starts_with("<omitted oversized event detail:")));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn events_api_records_mutating_workflows() {
    let (state, mut receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_REMOTE_CONFIGURATION", "true"));
    state.session.write().await.state = "connected";

    crate::route_http_request(
        "POST",
        "/api/v0/searches",
        None,
        "{\"query\":\"event flac\"}",
        &state,
    )
    .await
    .unwrap();
    let _ = receiver.try_recv();
    crate::route_http_request(
        "POST",
        "/api/v0/messages/inbound",
        None,
        "{\"username\":\"friend\",\"body\":\"hi\"}",
        &state,
    )
    .await
    .unwrap();
    crate::route_http_request(
        "PATCH",
        "/api/options",
        None,
        "{\"theme\":\"dark\"}",
        &state,
    )
    .await
    .unwrap();

    let events = crate::route_http_request("GET", "/api/v0/events/records", None, "", &state)
        .await
        .expect("events response");
    assert_eq!(events.status, "200 OK");
    assert!(events.body.contains("\"kind\":\"search.started\""));
    assert!(events.body.contains("\"topic\":\"searches\""));
    assert!(events.body.contains("\"kind\":\"message.received\""));
    assert!(events.body.contains("\"topic\":\"messages\""));
    assert!(events.body.contains("\"kind\":\"options.updated\""));
    assert!(events.body.contains("\"topic\":\"settings\""));
    assert!(events.body.contains("\"count\":3"));

    let filtered = crate::route_http_request(
        "GET",
        "/api/v0/events/records?kind=search.started",
        None,
        "",
        &state,
    )
    .await
    .expect("filtered events");
    assert_eq!(filtered.status, "200 OK");
    assert!(filtered.body.contains("\"filtered_count\":1"));
    assert!(filtered.body.contains("\"resource\":\"1\""));
    assert!(!filtered.body.contains("message.received"));

    let topic_filtered = crate::route_http_request(
        "GET",
        "/api/v0/events/records?topic=messages",
        None,
        "",
        &state,
    )
    .await
    .expect("topic-filtered events");
    assert_eq!(topic_filtered.status, "200 OK");
    assert!(topic_filtered.body.contains("\"filtered_count\":1"));
    assert!(topic_filtered
        .body
        .contains("\"kind\":\"message.received\""));
    assert!(!topic_filtered.body.contains("search.started"));

    let settings_filtered = crate::route_http_request(
        "GET",
        "/api/v0/events/records?topic=settings",
        None,
        "",
        &state,
    )
    .await
    .expect("settings-filtered events");
    assert_eq!(settings_filtered.status, "200 OK");
    assert!(settings_filtered.body.contains("\"filtered_count\":1"));
    assert!(settings_filtered
        .body
        .contains("\"kind\":\"options.updated\""));
    assert!(settings_filtered.body.contains("volatile=true"));

    let controller_events = crate::route_http_request(
        "GET",
        "/api/v0/events?topic=searches&q=search",
        None,
        "",
        &state,
    )
    .await
    .expect("slskd events");
    assert_eq!(controller_events.status, "200 OK");
    let controller_json =
        serde_json::from_str::<serde_json::Value>(&controller_events.body).unwrap();
    assert_eq!(controller_json.as_array().unwrap().len(), 1);
    assert_eq!(controller_json[0]["topic"], "searches");
    assert_eq!(controller_json[0]["type"], "search.started");
    assert_eq!(controller_json[0]["payload"]["resource"], "1");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn events_controller_emits_filtered_total_count_header() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"));
    {
        let mut events = state.events.write().await;
        events.record("search.started", "one", Some("ambient".to_owned()));
        events.record("search.completed", "two", Some("ambient".to_owned()));
        events.record("transfer.completed", "three", Some("ambient".to_owned()));
    }

    let (mut client, server) = tokio::io::duplex(1024 * 1024);
    let task = tokio::spawn(crate::handle_http_stream(
        server,
        Some("127.0.0.1:1".parse().expect("local test peer")),
        false,
        state,
    ));
    client
        .write_all(
            b"GET /api/v0/events?topic=searches&limit=1 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write events pagination request");
    let mut response = Vec::new();
    client
        .read_to_end(&mut response)
        .await
        .expect("read events pagination response");
    task.await
        .expect("events pagination HTTP task")
        .expect("events pagination HTTP response");

    let response = String::from_utf8(response).expect("events response is UTF-8");
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .expect("events response header boundary");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("X-Total-Count: 2\r\n"));
    let events = serde_json::from_str::<serde_json::Value>(body).expect("events JSON");
    assert_eq!(events.as_array().map(Vec::len), Some(1));
}
