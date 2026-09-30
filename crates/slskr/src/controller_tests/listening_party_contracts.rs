use super::test_state;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listening_party_requires_membership_and_reports_a_real_event() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-audit";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Listening Party Audit",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod (tester is the owner/member)");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    // A pod that exists, where "tester" (the fixture's default
    // configured identity) is not a member, must reject both GET and
    // POST with a real 403 -- never leak state or accept an event.
    let outside_pod = "pod:listening-party-outsider";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": outside_pod,
                "name": "Not Tester's Pod",
            }))
            .expect("deserialize pod record fixture"),
            "someone-else".to_owned(),
        )
        .expect("create outsider pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            outside_pod,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create outsider channel");
    let forbidden_get = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("forbidden get");
    assert_eq!(
        forbidden_get.status, "403 Forbidden",
        "{}",
        forbidden_get.body
    );
    let forbidden_post = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{outside_pod}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:x"}"#,
        &state,
    )
    .await
    .expect("forbidden post");
    assert_eq!(
        forbidden_post.status, "403 Forbidden",
        "{}",
        forbidden_post.body
    );

    // An invalid action is a real 400, matching the oracle's real
    // play|pause|seek|stop vocabulary -- not slskR's old invented
    // {kind, action} pair.
    let invalid_action = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"resume","contentId":"content:audio:track:x"}"#,
        &state,
    )
    .await
    .expect("invalid action");
    assert_eq!(invalid_action.status, "400 Bad Request");

    // No prior state: a real 204, not a fabricated pod+chat-history
    // payload.
    let before = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("no state yet");
    assert_eq!(before.status, "204 No Content", "{}", before.body);

    let played = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:x","title":"Track","artist":"Artist","hostPeerId":"forged-peer"}"#,
        &state,
    )
    .await
    .expect("play event");
    assert_eq!(played.status, "200 OK", "{}", played.body);
    let played_json = serde_json::from_str::<serde_json::Value>(&played.body).unwrap();
    assert_eq!(played_json["action"], "play");
    assert_eq!(played_json["podId"], pod_id);
    assert_eq!(played_json["channelId"], channel_id);
    // hostPeerId is always the authenticated local identity, never
    // the client-supplied value -- a forged value must not survive.
    assert_eq!(played_json["hostPeerId"], "tester");
    assert_eq!(played_json["kind"], "slskdn.listenAlong.v1");
    assert!(played_json["partyId"]
        .as_str()
        .unwrap()
        .starts_with("party:"));

    // GET must now return the exact same real, persisted event.
    let polled = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("poll event");
    assert_eq!(polled.status, "200 OK", "{}", polled.body);
    assert_eq!(polled.body, played.body);

    // A "stop" event clears the stored state entirely.
    let stopped = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop event");
    assert_eq!(stopped.status, "200 OK", "{}", stopped.body);
    let after_stop = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("state after stop");
    assert_eq!(after_stop.status, "204 No Content", "{}", after_stop.body);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listening_party_directory_reflects_real_listed_events() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-directory-audit";
    let channel_id = "general";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Directory Audit",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    // An unlisted event must never appear in the public directory.
    let unlisted = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:unlisted","listed":false}"#,
        &state,
    )
    .await
    .expect("unlisted play event");
    assert_eq!(unlisted.status, "200 OK", "{}", unlisted.body);
    let after_unlisted =
        crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .expect("directory after unlisted event");
    assert_eq!(
        after_unlisted.body, "[]",
        "an unlisted event must not appear in the directory"
    );

    // A listed event must appear with the real event's data.
    let listed = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"play","contentId":"content:audio:track:listed","title":"Directory Track","artist":"Directory Artist","listed":true,"allowMeshStreaming":true}"#,
        &state,
    )
    .await
    .expect("listed play event");
    assert_eq!(listed.status, "200 OK", "{}", listed.body);
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    let party_id = listed_json["partyId"].as_str().unwrap().to_owned();

    let directory = crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
        .await
        .expect("directory after listed event");
    let directory_json = serde_json::from_str::<serde_json::Value>(&directory.body).unwrap();
    let entries = directory_json.as_array().unwrap();
    assert_eq!(entries.len(), 1, "{directory_json}");
    assert_eq!(entries[0]["partyId"], party_id);
    assert_eq!(entries[0]["podId"], pod_id);
    assert_eq!(entries[0]["channelId"], channel_id);
    assert_eq!(entries[0]["hostPeerId"], "tester");
    assert_eq!(entries[0]["title"], "Directory Track");
    assert_eq!(entries[0]["artist"], "Directory Artist");
    assert_eq!(entries[0]["contentId"], "content:audio:track:listed");
    assert_eq!(entries[0]["allowMeshStreaming"], true);
    assert_eq!(entries[0]["kind"], "slskdn.listeningParty.announce.v1");
    assert!(entries[0]["streamPath"].as_str().is_some_and(|path| path
        .starts_with("/api/v0/listening-party/radio/")
        && path.contains("?ticket=")));
    assert!(
        entries[0]["expiresAtUnixMs"].as_u64().unwrap()
            > entries[0]["startedAtUnixMs"].as_u64().unwrap()
    );
    assert_eq!(
        entries[0]["lastSeenUnixMs"], entries[0]["startedAtUnixMs"],
        "directory reads must not refresh a persisted party"
    );

    // A persisted announcement must age out from its event timestamp. If
    // the directory used the request time as last-seen, this stale record
    // would incorrectly appear beside the live party.
    let stale_timestamp = crate::unix_timestamp_millis().saturating_sub(
        crate::extended_controller::LISTENING_PARTY_ANNOUNCEMENT_TTL_MS.saturating_add(1),
    );
    state
        .controller_features
        .write_for_test()
        .await
        .upsert(
            "listening-party/stale-pod/stale-channel".to_owned(),
            serde_json::json!({
                "partyId": "party:stale",
                "podId": "pod:stale",
                "channelId": "stale-channel",
                "hostPeerId": "stale-peer",
                "action": "play",
                "contentId": "stale-content",
                "serverTimeUnixMs": stale_timestamp,
                "listed": true,
                "allowMeshStreaming": false,
            }),
        )
        .expect("persist stale listening-party fixture");
    let with_stale = crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
        .await
        .expect("directory with stale event");
    let with_stale_json = serde_json::from_str::<serde_json::Value>(&with_stale.body).unwrap();
    let with_stale_entries = with_stale_json.as_array().unwrap();
    assert_eq!(with_stale_entries.len(), 1, "{with_stale_json}");
    assert_eq!(with_stale_entries[0]["partyId"], party_id);

    // Stopping the party removes it from the directory entirely.
    let stopped = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop event");
    assert_eq!(stopped.status, "200 OK", "{}", stopped.body);
    let after_stop_directory =
        crate::route_http_request("GET", "/api/v0/listening-party", None, "", &state)
            .await
            .expect("directory after stop");
    assert_eq!(after_stop_directory.body, "[]");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn listening_party_radio_requires_a_real_ticket_matching_the_content_id() {
    let (state, _receiver) = test_state();
    let pod_id = "pod:listening-party-radio-audit";
    let channel_id = "general";
    let content_id = "radio-audit-content";
    state
        .pods
        .write()
        .await
        .create(
            serde_json::from_value::<crate::pods::PodRecord>(serde_json::json!({
                "podId": pod_id,
                "name": "Radio Audit",
            }))
            .expect("deserialize pod record fixture"),
            "tester".to_owned(),
        )
        .expect("create pod");
    state
        .pods
        .write()
        .await
        .upsert_channel(
            pod_id,
            crate::pods::PodChannel {
                channel_id: channel_id.to_owned(),
                kind: serde_json::json!(0),
                name: "General".to_owned(),
                binding_info: None,
                description: None,
            },
        )
        .expect("create channel");

    // A real, listed, mesh-streaming-enabled party actually playing
    // this exact contentId -- matches the oracle's real StreamListedParty
    // party-state gate, which is evaluated before any ticket at all.
    let played = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        &format!(
            r#"{{"action":"play","contentId":"{content_id}","title":"Radio Track","artist":"Radio Artist","listed":true,"allowMeshStreaming":true}}"#
        ),
        &state,
    )
    .await
    .expect("play event");
    assert_eq!(played.status, "200 OK", "{}", played.body);
    let party_id = serde_json::from_str::<serde_json::Value>(&played.body).unwrap()["partyId"]
        .as_str()
        .unwrap()
        .to_owned();

    // No ticket at all -- must not leak availability/peer data to an
    // unauthenticated probe.
    let no_ticket = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("no ticket request");
    assert_eq!(no_ticket.status, "401 Unauthorized", "{}", no_ticket.body);

    // A real ticket, but issued for a different piece of content --
    // must not authorize access to this contentId.
    let mismatched_ticket_response = crate::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        r#"{"contentId":"some-other-content","filename":"Other.flac","peerId":"mesh-peer"}"#,
        &state,
    )
    .await
    .expect("create mismatched ticket");
    assert_eq!(mismatched_ticket_response.status, "200 OK");
    let mismatched_ticket =
        serde_json::from_str::<serde_json::Value>(&mismatched_ticket_response.body).unwrap()
            ["ticket"]
            .as_str()
            .unwrap()
            .to_owned();
    let mismatched = crate::route_http_request(
        "GET",
        &format!(
            "/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={mismatched_ticket}"
        ),
        None,
        "",
        &state,
    )
    .await
    .expect("mismatched ticket request");
    assert_eq!(mismatched.status, "401 Unauthorized", "{}", mismatched.body);

    // The frozen directory service issues a ticket owned by this exact
    // party. That ticket authorizes the exact contentId, and the
    // response reflects the real content-discovery shadow record (not a
    // fake/empty stub).
    state
        .content_discovery
        .write()
        .await
        .merge_shadow_records(vec![crate::content_discovery::ShadowIndexRecord {
            recording_id: content_id.to_owned(),
            peer_ids: vec!["peer-a".to_owned(), "peer-b".to_owned()],
            updated_at: 0,
        }])
        .expect("seed shadow record");
    let valid_ticket = crate::issue_listening_party_stream_ticket(&state, &party_id, content_id)
        .await
        .expect("create valid listening-party ticket");
    let authorized = crate::route_http_request(
        "GET",
        &format!("/api/v0/listening-party/radio/{party_id}/{content_id}?ticket={valid_ticket}"),
        None,
        "",
        &state,
    )
    .await
    .expect("authorized request");
    assert_eq!(authorized.status, "200 OK", "{}", authorized.body);
    let authorized_json = serde_json::from_str::<serde_json::Value>(&authorized.body).unwrap();
    assert_eq!(authorized_json["partyId"], party_id);
    assert_eq!(authorized_json["contentId"], content_id);
    assert_eq!(authorized_json["available"], true);
    assert_eq!(
        authorized_json["peerIds"],
        serde_json::json!(["peer-a", "peer-b"])
    );

    let stopped = crate::route_http_request(
        "POST",
        &format!("/api/v0/listening-party/{pod_id}/{channel_id}"),
        None,
        r#"{"action":"stop"}"#,
        &state,
    )
    .await
    .expect("stop party and revoke ticket");
    assert_eq!(stopped.status, "200 OK", "{}", stopped.body);
    let mut tickets = state.stream_tickets.write().await;
    assert!(
        tickets.get(&valid_ticket).is_none(),
        "stopping a party must revoke tickets owned by its stored party id"
    );
}
