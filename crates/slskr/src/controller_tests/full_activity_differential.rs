//! Controller full activity differential ownership.

use super::*;

/// Bulk differential proof crediting 7 more slskdN-only GET routes'
/// `nominal-status-headers-body` cases, independently re-derived from
/// `bounded_activity_and_network_polling_routes_project_local_state`,
/// `native_versioned_extended_gets_match_empty_state_contracts`, and
/// `versioned_hashdb_paging_matches_sequence_controller_contract`.
/// All confirmed present in the frozen slskdN registry (slskd declares
/// none of these).
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
pub(super) async fn controller_api_differential_activity_hashdb_and_transport_status_gets() {
    let target = "slskdn";
    let mut ledger = Vec::new();
    let mut mismatches = Vec::new();

    macro_rules! record {
        ($route:expr, $pass:expr) => {
            if !$pass {
                mismatches.push(format!("{target} GET {} ", $route));
            }
            ledger.push(serde_json::json!({
                "target": target,
                "method": "GET",
                "route": $route,
                "case": "nominal-status-headers-body",
                "pass": $pass,
            }));
        };
    }

    // Activity/network polling routes.
    {
        let (state, _receiver) = test_state_with_env(
            MapEnv::default()
                .with("SLSKR_CONTROLLER_PROFILE", target)
                .with("SLSKR_REMOTE_FILE_MANAGEMENT", "true"),
        );
        let message_id = {
            let mut messages = state.messages.write().await;
            messages
                .add("peer-unread".to_owned(), "inbound", "hello".to_owned())
                .id
        };
        let unread = crate::route_http_request(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            None,
            "",
            &state,
        )
        .await
        .expect("unacknowledged activity");
        let unread_pass = unread.status == "200 OK" && unread.body == "true";
        state.messages.write().await.ack(message_id);
        let acknowledged = crate::route_http_request(
            "GET",
            "/api/v0/conversations/activity/unacknowledged",
            None,
            "",
            &state,
        )
        .await
        .expect("acknowledged activity");
        record!(
            "/api/v0/conversations/activity/unacknowledged",
            unread_pass && acknowledged.status == "200 OK" && acknowledged.body == "false"
        );

        let mut rooms = state.rooms.write().await;
        rooms.join("active-room".to_owned()).expect("room capacity");
        rooms
            .add_message("active-room", "local".to_owned(), "outbound".to_owned())
            .expect("joined room");
        drop(rooms);
        let activity = crate::route_http_request("GET", "/api/v0/rooms/activity", None, "", &state)
            .await
            .expect("room activity");
        let activity_json =
            serde_json::from_str::<serde_json::Value>(&activity.body).unwrap_or_default();
        record!(
            "/api/v0/rooms/activity",
            activity.status == "200 OK"
                && activity_json["active-room"].as_u64().unwrap_or_default() > 0
        );

        let network = crate::route_http_request("GET", "/api/v0/network/stats", None, "", &state)
            .await
            .expect("network stats");
        let network_json =
            serde_json::from_str::<serde_json::Value>(&network.body).unwrap_or_default();
        record!(
            "/api/v0/network/stats",
            network.status == "200 OK" && network_json.get("dht").is_some()
        );
    }

    // Transport/listening-party empty-state routes.
    {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        let transports = crate::route_http_request(
            "GET",
            "/api/v0/security/transports/status",
            None,
            "",
            &state,
        )
        .await
        .expect("transport selector status");
        let transports_json =
            serde_json::from_str::<serde_json::Value>(&transports.body).unwrap_or_default();
        record!(
            "/api/v0/security/transports/status",
            transports.status == "200 OK" && transports_json["selectedMode"] == "Direct"
        );

        let listening_party = crate::route_http_request(
            "GET",
            "/api/v0/listening-party/pod:route-audit/route-audit-channel",
            None,
            "",
            &state,
        )
        .await
        .expect("empty listening-party state");
        record!(
            "/api/v0/listening-party/{podId}/{channelId}",
            listening_party.status == "204 No Content"
        );
    }

    // HashDb paging routes.
    {
        let (state, _receiver) =
            test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", target));
        let hash_by_size_empty =
            crate::route_http_request("GET", "/api/v0/hashdb/hash/by-size/4096", None, "", &state)
                .await
                .expect("empty hashdb size lookup");
        let hash_by_size_empty_json =
            serde_json::from_str::<serde_json::Value>(&hash_by_size_empty.body).unwrap_or_default();
        record!(
            "/api/v0/hashdb/hash/by-size/{size}",
            hash_by_size_empty.status == "200 OK"
                && hash_by_size_empty
                    .content_type
                    .starts_with("application/json")
                && hash_by_size_empty_json["count"] == 0
                && hash_by_size_empty_json["entries"] == serde_json::json!([])
        );

        let inventory_by_size_empty = crate::route_http_request(
            "GET",
            "/api/v0/hashdb/inventory/by-size/4096",
            None,
            "",
            &state,
        )
        .await
        .expect("empty hashdb inventory size lookup");
        let inventory_by_size_empty_json =
            serde_json::from_str::<serde_json::Value>(&inventory_by_size_empty.body)
                .unwrap_or_default();
        record!(
            "/api/v0/hashdb/inventory/by-size/{size}",
            inventory_by_size_empty.status == "200 OK"
                && inventory_by_size_empty
                    .content_type
                    .starts_with("application/json")
                && inventory_by_size_empty_json["count"] == 0
                && inventory_by_size_empty_json["entries"] == serde_json::json!([])
        );

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
        let first_json = serde_json::from_str::<serde_json::Value>(&first.body).unwrap_or_default();
        record!(
            "/api/v0/hashdb/entries",
            first.status == "200 OK"
                && first_json["latestSeq"] == 2
                && first_json["entries"][0]["seqId"] == 1
        );

        let sync = crate::route_http_request(
            "GET",
            "/api/v0/hashdb/sync/since/1?limit=1",
            None,
            "",
            &state,
        )
        .await
        .expect("hashdb sync page");
        let sync_json = serde_json::from_str::<serde_json::Value>(&sync.body).unwrap_or_default();
        record!(
            "/api/v0/hashdb/sync/since/{sinceSeq}",
            sync.status == "200 OK" && sync_json["entries"][0]["seqId"] == 2
        );
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("controller-api");
    fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    fs::write(
        evidence_dir.join("activity_hashdb_and_transport_status_gets.json"),
        serde_json::to_string_pretty(&ledger).expect("serialize controller-api ledger"),
    )
    .expect("write controller-api ledger");

    assert!(
        mismatches.is_empty(),
        "{} controller-api activity/hashdb/transport mismatches:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
