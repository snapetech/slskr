//! Controller full pods contracts ownership.

use super::*;

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn podcore_timespan_stats_preserve_target_millisecond_precision() {
    assert_eq!(crate::format_timespan_millis(0), "00:00:00");
    assert_eq!(crate::format_timespan_millis(1), "00:00:00.0010000");
    assert_eq!(crate::format_timespan_millis(1_234), "00:00:01.2340000");
    assert_eq!(
        crate::format_timespan_millis(86_400_001),
        "1.00:00:00.0010000"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_management_routes_persist_crud_members_and_bindings() {
    let (state, _receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod:api","name":"API Pod","isPublic":true,"maxMembers":4,"tags":["music"],"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"ignored-by-auth"}"#,
        &state,
    )
    .await
    .expect("create pod");
    assert_eq!(created.status, "201 Created");
    let created = serde_json::from_str::<serde_json::Value>(&created.body).unwrap();
    assert_eq!(created["podId"], "pod:api");
    assert_eq!(created["name"], "API Pod");
    assert!(created.get("members").is_none());

    let listed = crate::route_http_request("GET", "/api/pods", None, "", &state)
        .await
        .expect("list pods");
    let listed = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["podId"], "pod:api");

    let detail = crate::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("pod detail");
    assert_eq!(detail.status, "200 OK");

    let members = crate::route_http_request("GET", "/api/pods/pod%3Aapi/members", None, "", &state)
        .await
        .expect("pod members");
    let members = serde_json::from_str::<serde_json::Value>(&members.body).unwrap();
    assert_eq!(members.as_array().unwrap().len(), 1);
    assert_eq!(members[0]["peerId"], "tester");
    assert_eq!(members[0]["role"], "owner");

    *state.runtime_credentials.write().await =
        Some(crate::LoginCredentials::default_client("member", "secret"));
    let joined = crate::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/join",
        None,
        r#"{"peerId":"member"}"#,
        &state,
    )
    .await
    .expect("join pod");
    assert_eq!(joined.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&joined.body).unwrap()["joined"],
        true
    );
    *state.runtime_credentials.write().await = None;

    let bound = crate::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/channels/general/bind",
        None,
        r#"{"roomName":"ambient","mode":"mirror"}"#,
        &state,
    )
    .await
    .expect("bind pod channel");
    assert_eq!(bound.status, "200 OK");
    let detail = crate::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("bound pod detail");
    let detail = serde_json::from_str::<serde_json::Value>(&detail.body).unwrap();
    assert_eq!(
        detail["channels"][0]["bindingInfo"],
        "soulseek-room:ambient"
    );

    let banned = crate::route_http_request(
        "POST",
        "/api/pods/pod%3Aapi/ban",
        None,
        r#"{"peerId":"member"}"#,
        &state,
    )
    .await
    .expect("ban pod member");
    assert_eq!(banned.status, "200 OK");
    let members = crate::route_http_request("GET", "/api/pods/pod%3Aapi/members", None, "", &state)
        .await
        .expect("members after ban");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&members.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let updated = crate::route_http_request(
        "PUT",
        "/api/pods/pod%3Aapi",
        None,
        r#"{"pod":{"podId":"pod:api","name":"Renamed Pod","isPublic":true,"maxMembers":4,"channels":[{"channelId":"general","kind":0,"name":"General"}]}}"#,
        &state,
    )
    .await
    .expect("update pod");
    assert_eq!(updated.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.body).unwrap()["name"],
        "Renamed Pod"
    );

    let message = crate::route_http_request(
        "POST",
        "/api/v0/pods/pod%3Aapi/channels/general/messages",
        None,
        r#"{"body":"delete me","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("pod message before delete");
    assert_eq!(message.status, "200 OK");

    let deleted = crate::route_http_request("DELETE", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("delete pod");
    assert_eq!(deleted.status, "204 No Content");
    let missing = crate::route_http_request("GET", "/api/pods/pod%3Aapi", None, "", &state)
        .await
        .expect("deleted pod");
    assert_eq!(missing.status, "404 Not Found");
    assert!(state
        .pod_channels
        .read()
        .await
        .list("pod:api", "general", None)
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_channel_messages_are_durable_shaped_and_incremental() {
    let (state, _receiver) = test_state();
    let path = "/api/v0/pods/pod-1/channels/general/messages";
    let created = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        None,
        r#"{"pod":{"podId":"pod-1","name":"Pod One","isPublic":true,"channels":[{"channelId":"general","kind":0,"name":"General"}]},"requestingPeerId":"peer-1"}"#,
        &state,
    )
    .await
    .expect("create pod for messages");
    assert_eq!(created.status, "201 Created");

    state
        .pods
        .write()
        .await
        .join("pod-1", "peer-1".to_owned())
        .unwrap();
    let spoofed = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"spoofed","senderPeerId":"peer-1"}"#,
        &state,
    )
    .await
    .expect("spoofed pod message");
    assert_eq!(spoofed.status, "403 Forbidden");

    *state.runtime_credentials.write().await = Some(crate::LoginCredentials::default_client(
        "public-intruder",
        "secret",
    ));
    let forbidden = crate::route_http_request("GET", path, None, "", &state)
        .await
        .expect("public pod history still requires membership");
    assert_eq!(forbidden.status, "403 Forbidden");
    *state.runtime_credentials.write().await = None;

    let first = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"first","senderPeerId":"tester","signature":"sig"}"#,
        &state,
    )
    .await
    .expect("first pod message");
    assert_eq!(first.status, "200 OK");
    let first = serde_json::from_str::<serde_json::Value>(&first.body).unwrap();
    assert_eq!(first["sent"], true);
    assert_eq!(first["messageId"].as_str().unwrap().len(), 32);

    let initial = crate::route_http_request("GET", path, None, "", &state)
        .await
        .expect("initial pod messages");
    let initial = serde_json::from_str::<serde_json::Value>(&initial.body).unwrap();
    assert_eq!(initial.as_array().unwrap().len(), 1);
    assert_eq!(initial[0]["podId"], "pod-1");
    assert_eq!(initial[0]["channelId"], "general");
    assert_eq!(initial[0]["senderPeerId"], "tester");
    assert_eq!(initial[0]["body"], "first");
    assert_eq!(initial[0]["sigVersion"], 1);
    let cursor = initial[0]["timestampUnixMs"].as_u64().unwrap();

    let second = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"second","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("second pod message");
    assert_eq!(second.status, "200 OK");
    {
        let mut channels = state.pod_channels.write().await;
        let latest = channels.list("pod-1", "general", None).pop().unwrap();
        if latest.timestamp_unix_ms == cursor {
            channels
                .append(
                    "pod-1".to_owned(),
                    "general".to_owned(),
                    "tester".to_owned(),
                    "third".to_owned(),
                    String::new(),
                    cursor + 1,
                )
                .unwrap();
        }
    }
    let incremental =
        crate::route_http_request("GET", &format!("{path}?since={cursor}"), None, "", &state)
            .await
            .expect("incremental pod messages");
    let incremental = serde_json::from_str::<serde_json::Value>(&incremental.body).unwrap();
    assert!(!incremental.as_array().unwrap().is_empty());
    assert!(incremental
        .as_array()
        .unwrap()
        .iter()
        .all(|message| message["timestampUnixMs"].as_u64().unwrap() > cursor));

    let invalid = crate::route_http_request("GET", &format!("{path}?since=-1"), None, "", &state)
        .await
        .expect("invalid pod message cursor");
    assert_eq!(invalid.status, "400 Bad Request");

    let invalid = crate::route_http_request(
        "POST",
        path,
        None,
        r#"{"body":"","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("invalid pod message");
    assert_eq!(invalid.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_channel_bindings_bridge_room_and_private_messages() {
    let (state, mut receiver) = test_state();
    let created = crate::route_http_request(
        "POST",
        "/api/pods",
        None,
        r#"{"pod":{"podId":"pod-bridge","name":"Bridge","isPublic":true,"channels":[{"channelId":"room","kind":0,"name":"Room"},{"channelId":"dm","kind":1,"name":"DM","bindingInfo":"soulseek-dm:bob"}]}}"#,
        &state,
    )
    .await
    .expect("create bridge pod");
    assert_eq!(created.status, "201 Created");

    let bound = crate::route_http_request(
        "POST",
        "/api/pods/pod-bridge/channels/room/bind",
        None,
        r#"{"roomName":"ambient","mode":"mirror"}"#,
        &state,
    )
    .await
    .expect("bind bridge room");
    assert_eq!(bound.status, "200 OK");
    assert_eq!(
        receiver.recv().await,
        Some(crate::SessionCommand::JoinRoom("ambient".to_owned()))
    );

    let sent = crate::route_http_request(
        "POST",
        "/api/pods/pod-bridge/channels/room/messages",
        None,
        r#"{"body":"from pod","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("send mirrored pod message");
    assert_eq!(sent.status, "200 OK");
    assert_eq!(
        receiver.recv().await,
        Some(crate::SessionCommand::SayRoom {
            room: "ambient".to_owned(),
            body: "[Pod:tester] from pod".to_owned(),
        })
    );

    crate::bridge_soulseek_room_message_to_pods(&state, "AMBIENT", "alice", "from room").await;
    let room_history = crate::route_http_request(
        "GET",
        "/api/pods/pod-bridge/channels/room/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("read bridged room history");
    let room_history = serde_json::from_str::<serde_json::Value>(&room_history.body).unwrap();
    assert!(room_history.as_array().unwrap().iter().any(|message| {
        message["senderPeerId"] == "bridge:alice" && message["body"] == "[Soulseek:alice] from room"
    }));

    let inbound = crate::route_http_request(
        "POST",
        "/api/messages/inbound",
        None,
        r#"{"username":"bob","body":"from dm"}"#,
        &state,
    )
    .await
    .expect("record inbound private message");
    assert_eq!(inbound.status, "201 Created");
    let dm_history = crate::route_http_request(
        "GET",
        "/api/pods/pod-bridge/channels/dm/messages",
        None,
        "",
        &state,
    )
    .await
    .expect("read bridged dm history");
    let dm_history = serde_json::from_str::<serde_json::Value>(&dm_history.body).unwrap();
    assert_eq!(dm_history[0]["senderPeerId"], "bridge:bob");
    assert_eq!(dm_history[0]["body"], "from dm");

    let dm_sent = crate::route_http_request(
        "POST",
        "/api/pods/pod-bridge/channels/dm/messages",
        None,
        r#"{"body":"to dm","senderPeerId":"tester"}"#,
        &state,
    )
    .await
    .expect("send bridged dm");
    assert_eq!(dm_sent.status, "200 OK");
    assert_eq!(
        receiver.recv().await,
        Some(crate::SessionCommand::MessageUser {
            username: "bob".to_owned(),
            body: "to dm".to_owned(),
        })
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_creation_never_trusts_a_caller_supplied_peer_identity() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "")
            .with("SLSK_PASSWORD", "")
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "test-token"),
    );
    let response = crate::route_http_request(
        "POST",
        "/api/v0/pods",
        Some("Bearer test-token"),
        r#"{"pod":{"podId":"pod:spoofed","name":"Spoofed"},"requestingPeerId":"caller-controlled"}"#,
        &state,
    )
    .await
    .expect("pod create without local identity");
    assert_eq!(response.status, "403 Forbidden");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn realm_subject_indexes_persist_authority_and_compute_conflicts() {
    let (state, _receiver) = test_state();
    let index = |id: &str, subject: &str, title: &str, discogs: &str| {
        let mut index = serde_json::json!({
            "id": id,
            "realmId": crate::realm_subject_index::DEFAULT_REALM_ID,
            "subjectNamespace": "music",
            "revision": 1,
            "publishedAt": "2026-08-01T00:00:00Z",
            "entries": [{
                "subjectId": subject,
                "workRef": {
                    "domain": "music",
                    "title": title,
                    "creator": "Artist",
                },
                "externalIds": {
                    "musicbrainz:recording": "recording-1",
                    "discogs": discogs,
                },
                "aliases": ["shared-alias"],
            }],
            "signature": {
                "signer": crate::realm_subject_index::DEFAULT_GOVERNANCE_ROOT,
                "algorithm": "realm-governance-sha256",
                "payloadHash": "",
                "value": "signature",
            },
        });
        index["signature"]["payloadHash"] =
            serde_json::json!(crate::realm_subject_index::compute_payload_hash(&index));
        index
    };
    let merged = crate::route_http_request(
        "POST",
        "/api/v0/virtualsoulfind/shadow-index/sync/merge",
        None,
        &serde_json::json!({
            "records": [{"recordingId":"recording-1","peerIds":["peer-a"],"updatedAt":1}],
            "realmIndexes": [
                index("index-a", "subject-a", "Title A", "discogs-a"),
                index("index-b", "subject-b", "Title B", "discogs-b"),
                index("index-c", "subject-a", "Title C", "discogs-c"),
            ],
        })
        .to_string(),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(merged.status, "200 OK", "{}", merged.body);

    let indexes = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&indexes.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let conflicts = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let conflicts = serde_json::from_str::<serde_json::Value>(&conflicts.body).unwrap();
    assert_eq!(conflicts["indexCount"], 3);
    assert_eq!(conflicts["entryCount"], 3);
    assert_eq!(conflicts["hasConflicts"], true);
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "external-id"));
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "recording-subject"));
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "workref-identity"));
    assert!(conflicts["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|conflict| conflict["type"] == "alias-subject"));

    let resolutions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/recordings/recording-1/resolutions",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&resolutions.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let disabled = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-b/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"operator","note":"conflicting authority"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(disabled.status, "200 OK", "{}", disabled.body);
    let disabled = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/index-c/authority-decision",
        None,
        r#"{"enabled":false,"decidedBy":"operator","note":"conflicting authority"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(disabled.status, "200 OK", "{}", disabled.body);
    let decisions = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/authority-decisions",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&decisions.body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let after_disable = crate::route_http_request(
        "GET",
        "/api/v0/realm-subject-indexes/default-realm/conflicts",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    let after_disable = serde_json::from_str::<serde_json::Value>(&after_disable.body).unwrap();
    assert_eq!(after_disable["disabledAuthorityCount"], 2);
    assert_eq!(after_disable["hasConflicts"], false);

    let missing_decision = crate::route_http_request(
        "POST",
        "/api/v0/realm-subject-indexes/default-realm/missing/authority-decision",
        None,
        r#"{"enabled":true,"decidedBy":"operator"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing_decision.status, "400 Bad Request");
    assert!(missing_decision
        .body
        .contains("Index authority was not found"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn taste_recommendation_wishlist_promotion_creates_a_real_review_only_seed() {
    // Matches the oracle's real PromoteToWishlistAsync: a
    // recommendable WorkRef actually creates a disabled,
    // no-auto-download Wishlist seed (or reports the existing one
    // if already promoted), rather than just echoing the caller's
    // current wishlist back as if a promotion had occurred.
    let (state, _receiver) = test_state();

    let work_ref = r#"{"workRef":{"domain":"music","title":"Selected Ambient Works","creator":"Aphex Twin","externalIds":{"musicbrainz":"abcd1234-1234-4a12-9a12-0a1234567890"}},"note":"from a trusted follower"}"#;
    let promoted = crate::route_http_request(
        "POST",
        "/api/v0/taste-recommendations/wishlist",
        None,
        work_ref,
        &state,
    )
    .await
    .expect("promote a recommendable WorkRef");
    assert_eq!(promoted.status, "200 OK", "{}", promoted.body);
    let promoted_json = serde_json::from_str::<serde_json::Value>(&promoted.body).unwrap();
    assert_eq!(promoted_json["created"], true);
    assert_eq!(
        promoted_json["searchText"],
        "Aphex Twin Selected Ambient Works"
    );
    let item_id = promoted_json["wishlistItemId"].as_str().unwrap().to_owned();

    let wishlist = state.wishlist.read().await;
    let item = wishlist
        .records
        .iter()
        .flat_map(|record| record.items.iter())
        .find(|item| item.id == item_id)
        .expect("the promoted item is really stored in the wishlist")
        .clone();
    drop(wishlist);
    assert_eq!(item.artist, "Aphex Twin");
    assert_eq!(item.title, "Selected Ambient Works");
    assert!(!item.enabled, "a promoted seed must be review-only");
    assert!(!item.auto_download);
    assert!(item.filter.contains("source:taste-recommendation"));
    assert!(item.filter.contains("review-only"));
    assert!(item.filter.contains("note:from a trusted follower"));

    // Promoting the exact same WorkRef again reports the existing
    // seed instead of creating a duplicate.
    let duplicate = crate::route_http_request(
        "POST",
        "/api/v0/taste-recommendations/wishlist",
        None,
        work_ref,
        &state,
    )
    .await
    .expect("promote the same WorkRef again");
    assert_eq!(duplicate.status, "200 OK");
    let duplicate_json = serde_json::from_str::<serde_json::Value>(&duplicate.body).unwrap();
    assert_eq!(duplicate_json["created"], false);
    assert_eq!(duplicate_json["wishlistItemId"], item_id);
    assert_eq!(
        state
            .wishlist
            .read()
            .await
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .filter(|item| item.id == item_id)
            .count(),
        1,
        "no duplicate wishlist item was created"
    );

    // A non-music WorkRef is not recommendable at all.
    let rejected = crate::route_http_request(
        "POST",
        "/api/v0/taste-recommendations/wishlist",
        None,
        r#"{"workRef":{"domain":"books","title":"Not Music"}}"#,
        &state,
    )
    .await
    .expect("reject a non-music WorkRef");
    assert_eq!(rejected.status, "400 Bad Request");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&rejected.body).unwrap()["created"],
        false
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_member_affinities_reflect_real_activity_not_hardcoded_zeros() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000ba07";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
            crate::pods::PodMember {
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
            crate::pods::PodChannel {
                channel_id: "general".to_owned(),
                kind: serde_json::json!(0),
                name: "general".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    let now_millis = crate::unix_timestamp() * 1000;
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
    let opinion = crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"member-1","subjectType":"MeshPeer","subjectId":"track-1","kind":"Like","strength":0.8,"confidence":0.9}"#,
        &state,
    )
    .await
    .expect("submit opinion");
    assert_eq!(opinion.status, "200 OK", "{}", opinion.body);

    let affinities = crate::route_http_request(
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

    let update = crate::route_http_request(
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
pub(super) async fn pod_opinion_refresh_reports_real_counts_and_new_delta() {
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

    let first = crate::route_http_request(
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

    let second = crate::route_http_request(
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
    let third = crate::route_http_request(
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
pub(super) async fn opinions_summary_requires_subject_and_computes_a_real_weighted_score() {
    let (state, _receiver) = test_state();

    // Matches the oracle's required-parameter 400, not just the
    // generic "query string is present" check.
    let missing_subject_id = crate::route_http_request(
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
        let created = crate::route_http_request(
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
    crate::route_http_request(
        "POST",
        "/api/v0/opinions",
        None,
        r#"{"issuer":"peer-d","subjectType":"Track","subjectId":"track-2","kind":"Like","strength":1.0,"confidence":1.0}"#,
        &state,
    )
    .await
    .unwrap();

    let summary = crate::route_http_request(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn podcore_membership_and_message_stats_match_storage_contracts() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:stats-audit";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
            crate::pods::PodMember {
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
        crate::route_http_request("GET", "/api/v0/podcore/membership/stats", None, "", &state)
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
        crate::route_http_request("GET", "/api/v0/podcore/messages/stats", None, "", &state)
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn podcore_routing_matches_message_router_contract_and_updates_real_stats() {
    let (state, _receiver) = test_state();
    let pod_id = "routing-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
            crate::pods::PodMember {
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
            crate::pods::PodMember {
                peer_id: "banned-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: true,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add banned routing peer");

    let routed = crate::route_http_request(
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

    let direct = crate::route_http_request(
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

    let stats = crate::route_http_request("GET", "/api/v0/podcore/routing/stats", None, "", &state)
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

    let duplicate = crate::route_http_request(
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

    let missing_channel = crate::route_http_request(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_membership_removal_requires_moderator_or_self() {
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
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
            crate::pods::PodMember {
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
            crate::pods::PodMember {
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
    let denied = crate::route_http_request(
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
    let self_removed = crate::route_http_request(
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
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
            crate::pods::PodMember {
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
    let removed = crate::route_http_request(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_ban_unban_role_require_moderator() {
    let pod_id = "pod:0000000000000000000000000000ba01";
    let (state, _receiver) =
        pod_fixture_with_local_role("ordinary-peer", "owner-peer", pod_id).await;
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
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
            crate::pods::PodMember {
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
        let response = crate::route_http_request(
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
                crate::pods::PodMember {
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
    let ban = crate::route_http_request(
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
    let unban = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{owned_pod}/target-member/unban"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(unban.status, "200 OK", "{}", unban.body);
    let role = crate::route_http_request(
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
pub(super) async fn pod_membership_add_requires_self() {
    let pod_id = "pod:0000000000000000000000000000ba03";
    let (state, _receiver) = pod_fixture_with_local_role("local-peer", "owner-peer", pod_id).await;

    // The local peer may not publish membership claiming to be someone
    // else, nor self-assign a moderator role or clear a ban via the
    // request body.
    let impersonation = crate::route_http_request(
        "POST",
        &format!("/api/v0/podcore/membership/{pod_id}/members"),
        None,
        r#"{"peerId":"someone-else","role":"member","isBanned":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(impersonation.status, "403 Forbidden");

    let escalation = crate::route_http_request(
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
pub(super) async fn pod_membership_update_requires_moderator_or_self_and_pins_role() {
    let pod_id = "pod:0000000000000000000000000000ba04";
    let (state, _receiver) = pod_fixture_with_local_role("local-peer", "owner-peer", pod_id).await;
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
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
            crate::pods::PodMember {
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
    let denied = crate::route_http_request(
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
    let self_update = crate::route_http_request(
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
pub(super) async fn pod_channel_mutations_require_moderator() {
    let pod_id = "pod:0000000000000000000000000000ba05";
    let (state, _receiver) =
        pod_fixture_with_local_role("ordinary-peer", "owner-peer", pod_id).await;
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "ordinary-peer".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add local peer as ordinary member");

    let denied_create = crate::route_http_request(
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
    let created = crate::route_http_request(
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

    let updated = crate::route_http_request(
        "PUT",
        &format!("/api/v0/podcore/{owned_pod}/channels/{channel_id}"),
        None,
        r#"{"name":"Extra renamed"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "200 OK", "{}", updated.body);

    let deleted = crate::route_http_request(
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
pub(super) async fn podcore_maintenance_mutations_match_native_result_contracts() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000001";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
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
        let response = crate::route_http_request(
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
        let response = crate::route_http_request(
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

    let refresh = crate::route_http_request(
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
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap();
        let value = serde_json::from_str::<serde_json::Value>(&response.body).unwrap();
        for field in fields {
            assert!(value.get(field).is_some(), "{path}: missing {field}");
        }
    }

    let seen = crate::route_http_request(
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
        let response = crate::route_http_request("POST", path, None, "", &state)
            .await
            .unwrap();
        assert_eq!(response.body, "true", "{path}");
    }
    let sync_all = crate::route_http_request(
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
        let response = crate::route_http_request(
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

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_signature_verification_fails_for_a_sender_with_no_registered_key() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:00000000000000000000000000000ba08";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Signing Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");
    // "tester" (the fixture's default configured Soulseek identity,
    // required by the sign endpoint's own auth check) is a real pod
    // member, but has never registered a public key -- a well-formed
    // signature from them must still be rejected, since there is
    // nothing real to verify it against.
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add member with no public key");

    let keypair = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    let signed = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId": "message-1",
                "podId": pod_id,
                "senderPeerId": "tester",
                "body": "hello",
                "timestampUnixMs": crate::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign message");
    assert_eq!(signed.status, "200 OK");

    let verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signed.body,
        &state,
    )
    .await
    .expect("verify message");
    assert_eq!(verified.body, r#"{"isValid":false}"#);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_signing_stats_reflect_real_activity_not_hardcoded_zeros() {
    let (state, _receiver) = test_state();
    let baseline =
        crate::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
            .await
            .expect("baseline signing stats");
    let baseline_json = serde_json::from_str::<serde_json::Value>(&baseline.body).unwrap();
    assert_eq!(baseline_json["totalSignaturesCreated"], 0);
    assert_eq!(baseline_json["totalSignaturesVerified"], 0);
    assert_eq!(
        baseline_json["lastSignatureOperation"],
        crate::PODCORE_MIN_DATETIME
    );

    let pod_id = "pod:00000000000000000000000000000ba09";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Signing Stats Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");

    let keypair = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let signed = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({
            "privateKey": keys["privateKey"],
            "message": {
                "messageId": "message-1",
                "podId": pod_id,
                "senderPeerId": "tester",
                "body": "hello",
                "timestampUnixMs": crate::unix_timestamp() * 1000,
            }
        })
        .to_string(),
        &state,
    )
    .await
    .expect("sign message");
    assert_eq!(signed.status, "200 OK");

    let verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &signed.body,
        &state,
    )
    .await
    .expect("verify message");
    assert_eq!(verified.body, r#"{"isValid":true}"#);

    // A forged sender fails verification -- this must also count as a
    // real (failed) verification, not be silently dropped from stats.
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap();
    let mut forged = signed_json.clone();
    forged["message"]["senderPeerId"] = serde_json::json!("someone-else");
    let forged_verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/verify",
        None,
        &forged.to_string(),
        &state,
    )
    .await
    .expect("verify forged sender");
    assert_eq!(forged_verified.body, r#"{"isValid":false}"#);

    let stats = crate::route_http_request("GET", "/api/v0/podcore/signing/stats", None, "", &state)
        .await
        .expect("signing stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    assert_eq!(stats_json["totalSignaturesCreated"], 1, "{stats_json}");
    assert_eq!(stats_json["totalSignaturesVerified"], 2, "{stats_json}");
    assert_eq!(stats_json["successfulVerifications"], 1, "{stats_json}");
    assert_eq!(stats_json["failedVerifications"], 1, "{stats_json}");
    assert!(
        stats_json["lastSignatureOperation"].is_string(),
        "{stats_json}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_verification_message_checks_real_membership_and_signature() {
    let (state, _receiver) = test_state();
    // Unlike other pod fixtures in this suite, this podId deliberately
    // contains no colon: verification/message derives the membership
    // podId by splitting channelId on the first ':' as "podId:channel",
    // a convention that wouldn't round-trip through this test if the
    // podId itself contained a colon.
    let pod_id = "verification-audit-pod";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Verification Audit",
            }))
            .expect("deserialize pod record fixture"),
            "owner-peer".to_owned(),
        )
        .expect("create pod");

    let keypair = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/generate-keypair",
        None,
        "{}",
        &state,
    )
    .await
    .expect("generate keypair");
    let keys = serde_json::from_str::<serde_json::Value>(&keypair.body).unwrap();
    state
        .pods
        .write()
        .await
        .upsert_member(
            pod_id,
            crate::pods::PodMember {
                peer_id: "tester".to_owned(),
                role: "member".to_owned(),
                is_banned: false,
                public_key: keys["publicKey"].as_str().map(str::to_owned),
                joined_at: None,
                last_seen: None,
            },
        )
        .expect("add tester as a real pod member with a signing public key");

    let message = serde_json::json!({
        "messageId": "message-1",
        "podId": pod_id,
        "channelId": format!("{pod_id}:general"),
        "senderPeerId": "tester",
        "body": "hello",
        "timestampUnixMs": crate::unix_timestamp() * 1000,
    });
    let signed = crate::route_http_request(
        "POST",
        "/api/v0/podcore/signing/sign",
        None,
        &serde_json::json!({"privateKey": keys["privateKey"], "message": message}).to_string(),
        &state,
    )
    .await
    .expect("sign message");
    assert_eq!(signed.status, "200 OK");
    let signed_json = serde_json::from_str::<serde_json::Value>(&signed.body).unwrap();
    let mut verified_message = message.clone();
    verified_message["signature"] = signed_json["signature"].clone();

    let verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &verified_message.to_string(),
        &state,
    )
    .await
    .expect("verify message");
    assert_eq!(verified.status, "200 OK");
    let verified_json = serde_json::from_str::<serde_json::Value>(&verified.body).unwrap();
    assert_eq!(
        verified_json,
        serde_json::json!({
            "isValid": true,
            "isFromValidMember": true,
            "hasValidSignature": true,
            "isNotBanned": true,
        }),
        "{verified_json}"
    );

    // A sender who was never registered as a real pod member fails
    // the membership check, even with a structurally valid message.
    let mut unknown_sender = verified_message.clone();
    unknown_sender["senderPeerId"] = serde_json::json!("stranger");
    let unknown_verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &unknown_sender.to_string(),
        &state,
    )
    .await
    .expect("verify unknown sender");
    let unknown_json = serde_json::from_str::<serde_json::Value>(&unknown_verified.body).unwrap();
    assert_eq!(unknown_json["isValid"], false, "{unknown_json}");
    assert_eq!(unknown_json["isFromValidMember"], false, "{unknown_json}");
    assert_eq!(unknown_json["isNotBanned"], true, "{unknown_json}");

    // A channelId with no "podId:channel" separator is a structurally
    // invalid request, reported honestly rather than treated as some
    // other kind of failure.
    let mut bad_channel = verified_message.clone();
    bad_channel["channelId"] = serde_json::json!("no-colon-here");
    let bad_channel_verified = crate::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        &bad_channel.to_string(),
        &state,
    )
    .await
    .expect("verify bad channel id");
    assert_eq!(
        bad_channel_verified.body,
        serde_json::json!({
            "isValid": false,
            "isFromValidMember": false,
            "hasValidSignature": false,
            "isNotBanned": false,
            "errorMessage": "Invalid channel ID format",
        })
        .to_string()
    );

    // Missing required fields is a real 400, not a silently-accepted
    // always-false result.
    let missing_pod_id = crate::route_http_request(
        "POST",
        "/api/v0/podcore/verification/message",
        None,
        r#"{"messageId":"message-1"}"#,
        &state,
    )
    .await
    .expect("verify missing podId");
    assert_eq!(missing_pod_id.status, "400 Bad Request");

    let stats = crate::route_http_request(
        "GET",
        "/api/v0/podcore/verification/stats",
        None,
        "",
        &state,
    )
    .await
    .expect("verification stats");
    let stats_json = serde_json::from_str::<serde_json::Value>(&stats.body).unwrap();
    // Only 2 real verification attempts are recorded: the successful
    // one and the unknown-sender failure. Matching the oracle exactly,
    // the invalid-channel-format and missing-field cases never reach
    // the verifier (they fail before any membership/signature check
    // runs), so neither counts as a verification attempt.
    assert_eq!(stats_json["totalVerifications"], 2, "{stats_json}");
    assert_eq!(stats_json["successfulVerifications"], 1, "{stats_json}");
    assert_eq!(stats_json["failedMembershipChecks"], 1, "{stats_json}");
    assert!(stats_json["lastVerification"].is_string(), "{stats_json}");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_join_enforce_mode_verifies_ed25519_and_rejects_replay() {
    use ed25519_dalek::{Signer, SigningKey};

    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "enforce"),
        crate::SearchStore::new(),
        None,
    );
    state
        .rooms
        .write()
        .await
        .join("pod:ambient".to_owned())
        .expect("test pod");
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let timestamp = crate::unix_timestamp().saturating_mul(1_000);
    let payload = serde_json::to_string(&(
        1,
        "join-request",
        "pod:ambient",
        "peer-one",
        "member",
        timestamp,
        "Please add me",
        "nonce-one",
    ))
    .unwrap();
    let signature = base64::engine::general_purpose::STANDARD
        .encode(signing_key.sign(payload.as_bytes()).to_bytes());
    let public_key =
        base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().to_bytes());
    let body = serde_json::json!({
        "podId": "pod:ambient",
        "peerId": "peer-one",
        "requestedRole": "member",
        "timestampUnixMs": timestamp,
        "message": "Please add me",
        "nonce": "nonce-one",
        "signature": format!("ed25519:{signature}"),
        "publicKey": public_key,
    })
    .to_string();

    let joined =
        crate::route_http_request("POST", "/api/podcore/membership/join", None, &body, &state)
            .await
            .expect("verified pod join");
    assert_eq!(joined.status, "200 OK", "{}", joined.body);
    let joined_json = serde_json::from_str::<serde_json::Value>(&joined.body).unwrap();
    assert_eq!(joined_json["signatureMode"], "enforce");
    assert_eq!(joined_json["signatureVerified"], true);
    assert_eq!(joined_json["success"], true);
    assert_eq!(joined_json["joinRequest"]["peerId"], "peer-one");
    assert_eq!(state.rooms.read().await.records.len(), 1);
    assert!(state.rooms.read().await.records[0].members.is_empty());

    let replay =
        crate::route_http_request("POST", "/api/podcore/membership/join", None, &body, &state)
            .await
            .expect("replayed pod join");
    assert_eq!(replay.status, "400 Bad Request");
    assert!(replay.body.contains("nonce has already been used"));
    assert_eq!(state.rooms.read().await.records.len(), 1);

    let mut tampered = serde_json::from_str::<serde_json::Value>(&body).unwrap();
    tampered["podId"] = serde_json::json!("pod:tampered");
    let rejected = crate::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        &tampered.to_string(),
        &state,
    )
    .await
    .expect("tampered pod join");
    assert_eq!(rejected.status, "400 Bad Request");
    assert!(rejected.body.contains("signature is invalid"));
    assert_eq!(state.rooms.read().await.records.len(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_join_warn_mode_accepts_legacy_but_rejects_invalid_ed25519() {
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "warn"),
        crate::SearchStore::new(),
        None,
    );
    {
        let mut rooms = state.rooms.write().await;
        rooms.join("legacy-pod".to_owned()).expect("legacy pod");
        rooms.join("invalid-pod".to_owned()).expect("invalid pod");
    }
    let legacy = crate::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        r#"{"podId":"legacy-pod","peerId":"legacy-peer"}"#,
        &state,
    )
    .await
    .expect("legacy pod join");
    assert_eq!(legacy.status, "200 OK", "{}", legacy.body);
    let legacy_json = serde_json::from_str::<serde_json::Value>(&legacy.body).unwrap();
    assert_eq!(legacy_json["signatureMode"], "warn");
    assert_eq!(legacy_json["signatureVerified"], false);

    let invalid = crate::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        &serde_json::json!({
            "podId": "invalid-pod",
            "peerId": "peer",
            "timestampUnixMs": crate::unix_timestamp().saturating_mul(1_000),
            "signature": format!("ed25519:{}", base64::engine::general_purpose::STANDARD.encode([0_u8; 64])),
            "publicKey": base64::engine::general_purpose::STANDARD.encode([0_u8; 32]),
        }).to_string(),
        &state,
    )
    .await
    .expect("invalid signed pod join");
    assert_eq!(invalid.status, "400 Bad Request");
    assert_eq!(state.rooms.read().await.records.len(), 2);
}
