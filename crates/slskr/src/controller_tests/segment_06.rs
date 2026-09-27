#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn now_playing_and_security_state_bound_remote_keys() {
    let mut now_playing = super::NowPlayingStore::new();
    let bounded = now_playing.upsert(
        "é".repeat(super::MAX_USER_USERNAME_BYTES),
        "a".repeat(super::MAX_NOW_PLAYING_ARTIST_BYTES + 1),
        "t".repeat(super::MAX_NOW_PLAYING_TITLE_BYTES + 1),
    );
    assert!(bounded.username.len() <= super::MAX_USER_USERNAME_BYTES);
    assert_eq!(bounded.artist.len(), super::MAX_NOW_PLAYING_ARTIST_BYTES);
    assert_eq!(bounded.title.len(), super::MAX_NOW_PLAYING_TITLE_BYTES);
    now_playing.records.clear();
    for index in 0..super::MAX_NOW_PLAYING_RECORDS {
        now_playing.upsert(
            format!("user-{index}"),
            "Artist".to_owned(),
            "Track".to_owned(),
        );
    }
    now_playing.upsert(
        "overflow".to_owned(),
        "Artist".to_owned(),
        "Track".to_owned(),
    );
    assert_eq!(now_playing.records.len(), super::MAX_NOW_PLAYING_RECORDS);
    assert!(now_playing
        .records
        .iter()
        .any(|record| record.username == "overflow"));
    let count = now_playing.records.len();
    now_playing.upsert(
        "OVERFLOW".to_owned(),
        "Updated".to_owned(),
        "Track".to_owned(),
    );
    assert_eq!(now_playing.records.len(), count);

    let mut security = super::SecurityState::new();
    let first = security.ban("username", "Spammer".to_owned()).unwrap();
    let duplicate = security.ban("username", "spammer".to_owned()).unwrap();
    assert_eq!(first.value, duplicate.value);
    assert_eq!(security.bans.len(), 1);
    assert!(security.unban("username", "SPAMMER"));
    let literal_pattern = security.ban("username", "user.*".to_owned()).unwrap();
    assert_eq!(literal_pattern.value, "user.*");
    assert!(!security.unban("username", "user123"));
    assert!(security.unban("username", "USER.*"));
    for index in 0..super::MAX_SECURITY_BANS {
        security.ban("ip", format!("2001:db8::{index:x}")).unwrap();
    }
    assert!(security.ban("ip", "198.51.100.1".to_owned()).is_none());
    assert_eq!(security.bans.len(), super::MAX_SECURITY_BANS);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn configured_mesh_sync_and_violation_thresholds_enforce_quarantine_and_bans() {
    let config = crate::config::AppConfig::from_layers(
        None,
        crate::config::FileConfig::default(),
        &MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"),
    )
    .unwrap();
    let mut sync = config.advanced_networking.mesh_sync_security.clone();
    sync.max_invalid_entries_per_window = 2;
    sync.quarantine_violation_threshold = 2;
    sync.rate_limit_window = std::time::Duration::from_secs(60);
    sync.quarantine_duration = std::time::Duration::from_secs(120);
    let mut mesh = super::MeshState::new();
    assert!(mesh.record_invalid_sync_entries("Peer", 3, &sync, 1_000));
    assert!(!mesh.sync_is_quarantined("peer", 1_000));
    assert!(mesh.record_invalid_sync_entries("PEER", 1, &sync, 1_001));
    assert!(mesh.sync_is_quarantined("peer", 1_001));
    assert_eq!(mesh.sync_rejected_messages, 2);
    assert_eq!(mesh.sync_quarantine_events, 1);
    assert!(!mesh.sync_is_quarantined("peer", 1_122));

    let mut security_settings = config.advanced_networking.security.clone();
    security_settings.enabled = true;
    security_settings.violation_tracker.enabled = true;
    security_settings
        .violation_tracker
        .violations_before_auto_ban = 2;
    security_settings.violation_tracker.base_ban_duration = std::time::Duration::from_secs(900);
    let mut security = super::SecurityState::new();
    assert!(!security.record_peer_violation("BadPeer", &security_settings));
    assert_eq!(security.reputation["badpeer"], 35);
    assert!(security.record_peer_violation("badpeer", &security_settings));
    assert_eq!(security.reputation["badpeer"], 20);
    assert_eq!(security.active_bans(), 1);
    assert_eq!(security.bans[0].value, "badpeer");
    assert_eq!(security.bans[0].reason, "Automatic security-policy ban");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn private_message_auto_response_classifier_and_cooldown_are_bounded() {
    for candidate in [
        "Are you human?",
        "Please prove you are not a bot",
        "Human verification challenge",
        "You are not sharing anything; share more files",
    ] {
        assert!(
            super::is_private_message_auto_response_candidate(candidate),
            "{candidate}"
        );
    }
    for ordinary in [
        "hello",
        "what music do you like?",
        "the robots album is good",
    ] {
        assert!(
            !super::is_private_message_auto_response_candidate(ordinary),
            "{ordinary}"
        );
    }
    assert!(!super::is_private_message_auto_response_candidate(
        &"x".repeat(super::MAX_MESSAGE_BODY_BYTES + 1)
    ));

    let mut tracker = super::PrivateMessageAutoResponseTracker::default();
    assert!(tracker.should_respond("Peer", 1_000, 600));
    assert!(!tracker.should_respond("peer", 1_599, 600));
    tracker.release(" PEER ");
    assert!(tracker.should_respond("peer", 1_599, 600));
    assert!(!tracker.should_respond("PEER", 1_600, 600));
    assert!(tracker.should_respond("PEER", 2_199, 600));
    for index in 0..=super::MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS {
        assert!(tracker.should_respond(&format!("peer-{index}"), 2_000, 600));
    }
    assert_eq!(
        tracker.len(),
        super::MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn private_message_auto_response_settings_are_runtime_mutable_and_redacted() {
    let (state, _receiver) = test_state();
    let initial = super::route_http_request(
        "GET",
        "/api/private-message-auto-response",
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(initial.status, "200 OK");
    assert!(initial.body.contains(r#""enabled":false"#));
    assert!(!initial.body.contains("temporarily unavailable"));

    let updated = super::route_http_request(
        "PUT",
        "/api/private-message-auto-response",
        None,
        r#"{"enabled":true,"message":"Runtime human check","cooldownMinutes":15}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "200 OK");
    assert!(updated.body.contains(r#""enabled":true"#));
    assert!(updated.body.contains(r#""cooldown_minutes":15"#));
    assert!(!updated.body.contains("Runtime human check"));
    let settings = state.private_message_auto_response_settings.read().await;
    assert!(settings.enabled);
    assert_eq!(settings.message, "Runtime human check");
    assert_eq!(settings.cooldown_minutes, 15);
    drop(settings);

    assert!(state
        .private_message_auto_responses
        .write()
        .await
        .should_respond("peer", super::unix_timestamp(), 60));
    let disabled = super::route_http_request(
        "PUT",
        "/api/private-message-auto-response",
        None,
        r#"{"enabled":false}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(disabled.status, "200 OK");
    assert!(state.private_message_auto_responses.read().await.is_empty());

    let invalid = super::route_http_request(
        "PUT",
        "/api/private-message-auto-response",
        None,
        r#"{"cooldownMinutes":0}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_grants_bound_and_deduplicate_collection_users() {
    let mut grants = super::ShareGrantStore::new();
    assert!(grants
        .create_with_contract(None, "collection".to_owned(), "   ".to_owned())
        .is_none());
    let (first, created) = grants
        .create_with_contract(None, "collection".to_owned(), "  Alice  ".to_owned())
        .unwrap();
    assert!(created);
    assert_eq!(first.username, "Alice");
    let (duplicate, created) = grants
        .create_with_contract(None, "collection".to_owned(), "alice".to_owned())
        .unwrap();
    assert!(!created);
    assert_eq!(duplicate.id, first.id);
    for index in 1..super::MAX_SHARE_GRANTS {
        grants
            .create_with_contract(None, format!("collection-{index}"), format!("user-{index}"))
            .unwrap();
    }
    assert!(grants
        .create_with_contract(None, "overflow".to_owned(), "user".to_owned())
        .is_none());
    assert_eq!(grants.records.len(), super::MAX_SHARE_GRANTS);
    let mut exhausted = super::ShareGrantStore::new();
    exhausted.next_id = u64::MAX;
    assert_eq!(
        exhausted
            .create_with_contract(None, "collection".to_owned(), "user".to_owned())
            .unwrap()
            .0
            .id,
        format!("grant-{}", u64::MAX)
    );

    let permissions = "p".repeat(super::MAX_SHARE_GRANT_PERMISSIONS_BYTES + 1);
    let updated = exhausted
        .update(&format!("grant-{}", u64::MAX), permissions)
        .unwrap();
    assert_eq!(
        updated.permissions.len(),
        super::MAX_SHARE_GRANT_PERMISSIONS_BYTES
    );
    assert_eq!(
        exhausted
            .update(&format!("grant-{}", u64::MAX), "   ".to_owned())
            .unwrap()
            .permissions,
        "read"
    );

    let hydrated = super::ShareGrantStore::from_persisted(vec![
        crate::persistence::ShareGrantRecord {
            id: "grant-1".to_owned(),
            collection_id: "collection".to_owned(),
            username: format!("  {}  ", "é".repeat(super::MAX_USER_USERNAME_BYTES)),
            shared_at: 1,
            permissions: "x".repeat(super::MAX_SHARE_GRANT_PERMISSIONS_BYTES + 1),
        },
        crate::persistence::ShareGrantRecord {
            id: "grant-2".to_owned(),
            collection_id: "collection".to_owned(),
            username: "   ".to_owned(),
            shared_at: 2,
            permissions: "write".to_owned(),
        },
    ]);
    assert_eq!(hydrated.records.len(), 1);
    assert!(hydrated.records[0].username.len() <= super::MAX_USER_USERNAME_BYTES);
    assert_eq!(
        hydrated.records[0].permissions.len(),
        super::MAX_SHARE_GRANT_PERMISSIONS_BYTES
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn security_bans_normalize_and_reject_malformed_persisted_values() {
    let oversized_username = format!("  {}  ", "é".repeat(super::MAX_USER_USERNAME_BYTES));
    let mut security = super::SecurityState::new();
    let username = security
        .ban("username", oversized_username)
        .expect("valid username ban");
    assert!(username.value.len() <= super::MAX_SECURITY_BAN_USERNAME_BYTES);
    assert!(!username.value.starts_with(char::is_whitespace));
    assert!(!username.value.ends_with(char::is_whitespace));
    assert!(security
        .ban(
            "username",
            format!(" {} ", username.value.to_ascii_uppercase())
        )
        .is_some());
    assert_eq!(security.active_bans(), 1);
    assert!(security.ban("ip", "not-an-ip".to_owned()).is_none());
    assert!(security.ban("unknown", "value".to_owned()).is_none());

    let hydrated = super::SecurityState::from_persisted(vec![
        crate::persistence::SecurityBanRecord {
            kind: "ip".to_owned(),
            value: "2001:0db8:0:0:0:0:0:1".to_owned(),
            created_at: 1,
            reason: "Manual ban".to_owned(),
            expires_at: 3_601,
            is_permanent: false,
        },
        crate::persistence::SecurityBanRecord {
            kind: "ip".to_owned(),
            value: "2001:db8::1".to_owned(),
            created_at: 2,
            reason: "Manual ban".to_owned(),
            expires_at: 3_602,
            is_permanent: false,
        },
        crate::persistence::SecurityBanRecord {
            kind: "ip".to_owned(),
            value: "malformed".to_owned(),
            created_at: 3,
            reason: "Manual ban".to_owned(),
            expires_at: 3_603,
            is_permanent: false,
        },
        crate::persistence::SecurityBanRecord {
            kind: "other".to_owned(),
            value: "ignored".to_owned(),
            created_at: 4,
            reason: "Manual ban".to_owned(),
            expires_at: 3_604,
            is_permanent: false,
        },
    ]);
    assert_eq!(hydrated.active_bans(), 1);
    assert_eq!(hydrated.bans[0].value, "2001:db8::1");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn security_reputation_routes_reflect_real_score_and_violations() {
    // Matches the oracle's real PeerReputation-backed
    // SecurityController: reputation endpoints previously read/wrote
    // a disconnected, admin-only settings blob that never reflected
    // real peer behavior (violations always showed as 0, and
    // suspicious/trusted lists were derived from watch/online status
    // instead of reputation at all).
    let (state, _receiver) = test_state();

    // The profile counters must reconcile the real retained transfer
    // history rather than remain at an unconditional zero.
    {
        let mut transfers = state.transfers.write().await;
        let successful = transfers.create(
            0,
            Some("cleanpeer".to_owned()),
            "Music/clean.flac".to_owned(),
            None,
            Some(100),
        );
        transfers.update_status(successful.id, "succeeded", Some(100), None);
        let failed = transfers.create(
            0,
            Some("cleanpeer".to_owned()),
            "Music/failed.flac".to_owned(),
            None,
            Some(20),
        );
        transfers.update_status(
            failed.id,
            "failed",
            Some(20),
            Some("content hash mismatch".to_owned()),
        );
        let aborted = transfers.create(
            0,
            Some("cleanpeer".to_owned()),
            "Music/aborted.flac".to_owned(),
            None,
            Some(3),
        );
        transfers.update_status(aborted.id, "cancelled", Some(3), None);
    }

    // A peer with no recorded violations reports the real,
    // never-decremented default score.
    let clean = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/cleanpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("clean peer reputation");
    let clean_json = serde_json::from_str::<serde_json::Value>(&clean.body).unwrap();
    assert_eq!(clean_json["score"], 50);
    assert_eq!(clean_json["protocolViolations"], 0);
    assert_eq!(clean_json["successfulTransfers"], 1);
    assert_eq!(clean_json["failedTransfers"], 1);
    assert_eq!(clean_json["abortedTransfers"], 1);
    assert_eq!(clean_json["totalBytesTransferred"], 123);
    assert_eq!(clean_json["contentMismatches"], 1);
    assert_eq!(clean_json["successRate"], 1.0 / 3.0);
    assert_eq!(clean_json["trustLevel"], "Neutral");

    // Seed a real violation-driven score drop via the same
    // SecurityState used by the real violation tracker.
    {
        let mut security = state.security.write().await;
        security.reputation.insert("badpeer".to_owned(), 15);
        security.violations.insert("badpeer".to_owned(), 3);
        security.reputation.insert("neutral-low".to_owned(), 30);
    }
    let bad = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/badpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("bad peer reputation");
    let bad_json = serde_json::from_str::<serde_json::Value>(&bad.body).unwrap();
    assert_eq!(bad_json["score"], 15);
    assert_eq!(bad_json["protocolViolations"], 3);
    assert_eq!(bad_json["trustLevel"], "Untrusted");

    // The suspicious list reflects the real low score, not
    // watch/online status.
    let suspicious = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/suspicious",
        None,
        "",
        &state,
    )
    .await
    .expect("suspicious peers");
    let suspicious_json = serde_json::from_str::<serde_json::Value>(&suspicious.body).unwrap();
    let suspicious_usernames = suspicious_json
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["username"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(suspicious_usernames, vec!["badpeer", "neutral-low"]);
    assert_eq!(suspicious_json[0]["firstSeen"].as_str().unwrap().len(), 20);
    assert_eq!(suspicious_json[0]["lastSeen"].as_str().unwrap().len(), 20);
    assert_eq!(suspicious_json[0]["successfulTransfers"], 0);
    assert_eq!(suspicious_json[0]["failedTransfers"], 0);
    assert_eq!(suspicious_json[0]["successRate"], 0.5);
    assert_eq!(suspicious_json[0]["trustLevel"], "Untrusted");

    let dashboard =
        super::route_http_request("GET", "/api/v0/security/dashboard", None, "", &state)
            .await
            .expect("security dashboard");
    let dashboard_json = serde_json::from_str::<serde_json::Value>(&dashboard.body).unwrap();
    assert_eq!(dashboard_json["reputationStats"]["totalPeers"], 3);
    assert_eq!(dashboard_json["reputationStats"]["untrustedPeers"], 1);
    let average_score = dashboard_json["reputationStats"]["averageScore"]
        .as_f64()
        .unwrap();
    assert!((average_score - (95.0 / 3.0)).abs() < 1e-12);
    assert_eq!(
        dashboard_json["reputationStats"]["totalProtocolViolations"],
        3
    );

    // The trusted list never includes the suspicious peer.
    let trusted = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/trusted",
        None,
        "",
        &state,
    )
    .await
    .expect("trusted peers");
    let trusted_json = serde_json::from_str::<serde_json::Value>(&trusted.body).unwrap();
    assert!(trusted_json
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["username"] != "badpeer"));

    // A manual PUT override rejects out-of-range scores...
    let invalid = super::route_http_request(
        "PUT",
        "/api/v0/security/reputation/badpeer",
        None,
        r#"{"score":150}"#,
        &state,
    )
    .await
    .expect("reject out-of-range score");
    assert_eq!(invalid.status, "400 Bad Request");

    // ...and a valid override writes directly into the same real
    // score the automatic violation tracker reads and adjusts.
    let overridden = super::route_http_request(
        "PUT",
        "/api/v0/security/reputation/badpeer",
        None,
        r#"{"score":80,"reason":"manual review"}"#,
        &state,
    )
    .await
    .expect("override reputation score");
    assert_eq!(overridden.status, "200 OK");
    assert_eq!(state.security.read().await.reputation["badpeer"], 80);
    let after_override = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/badpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("reputation after override");
    let after_override_json =
        serde_json::from_str::<serde_json::Value>(&after_override.body).unwrap();
    assert_eq!(after_override_json["score"], 80);
    assert_eq!(after_override_json["trustLevel"], "Trusted");

    let first_seen = bad_json["firstSeen"].as_str().unwrap().to_owned();
    let reread = super::route_http_request(
        "GET",
        "/api/v0/security/reputation/badpeer",
        None,
        "",
        &state,
    )
    .await
    .expect("re-read peer reputation");
    let reread_json = serde_json::from_str::<serde_json::Value>(&reread.body).unwrap();
    assert_eq!(reread_json["firstSeen"], first_seen);
    assert_eq!(reread_json["protocolViolations"], 3);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn taste_recommendation_wishlist_promotion_creates_a_real_review_only_seed() {
    // Matches the oracle's real PromoteToWishlistAsync: a
    // recommendable WorkRef actually creates a disabled,
    // no-auto-download Wishlist seed (or reports the existing one
    // if already promoted), rather than just echoing the caller's
    // current wishlist back as if a promotion had occurred.
    let (state, _receiver) = test_state();

    let work_ref = r#"{"workRef":{"domain":"music","title":"Selected Ambient Works","creator":"Aphex Twin","externalIds":{"musicbrainz":"abcd1234-1234-4a12-9a12-0a1234567890"}},"note":"from a trusted follower"}"#;
    let promoted = super::route_http_request(
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
    let duplicate = super::route_http_request(
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
    let rejected = super::route_http_request(
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
async fn overlay_pin_rotation_uses_the_real_thumbprint_field_and_no_content_response() {
    // Matches the oracle's real RotateCertificatePin
    // (DhtRendezvousController.cs): the wire field is "thumbprint",
    // not "certificateSha256"/"pin", the pin is stored
    // upper-invariant, and a successful rotation is a real 204 No
    // Content, not a 200 with a JSON body.
    let (state, _receiver) = test_state();

    let missing_field = super::route_http_request(
        "PUT",
        "/api/overlay/pins/peer1",
        None,
        r#"{"certificateSha256":"07070707070707070707070707070707070707070707070707070707070707"}"#,
        &state,
    )
    .await
    .expect("reject the wrong field name");
    assert_eq!(missing_field.status, "400 Bad Request");

    let too_short = super::route_http_request(
        "PUT",
        "/api/overlay/pins/peer1",
        None,
        r#"{"thumbprint":"abc123"}"#,
        &state,
    )
    .await
    .expect("reject a too-short thumbprint");
    assert_eq!(too_short.status, "400 Bad Request");

    let rotated = super::route_http_request(
        "PUT",
        "/api/overlay/pins/peer1",
        None,
        r#"{"thumbprint":"0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a"}"#,
        &state,
    )
    .await
    .expect("rotate the pin");
    assert_eq!(rotated.status, "204 No Content", "{}", rotated.body);
    assert!(rotated.body.is_empty());

    let stored = state
        .controller_features
        .read()
        .await
        .get("overlay/pin/peer1")
        .cloned()
        .expect("pin was really stored");
    assert_eq!(
        stored["certificateSha256"],
        "0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A0A"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn overlay_blocklist_honors_real_reason_duration_and_permanent_fields() {
    // Matches the oracle's real BlockIp/BlockUsername/GetBlocklist
    // (DhtRendezvousController.cs): reason/durationMinutes/permanent
    // are real inputs, and the listing reflects the oracle's real
    // BlockedEntryResponse shape (target/type/reason/blockedAt/
    // expiresAt/isPermanent) -- previously these fields were
    // silently discarded in favor of a fixed 1-hour "Manual ban"
    // default, and the listing used an unrelated shape.
    let (state, _receiver) = test_state();

    let blocked = super::route_http_request(
        "POST",
        "/api/overlay/blocklist/username",
        None,
        r#"{"value":"forever-banned","reason":"repeated abuse","permanent":true}"#,
        &state,
    )
    .await
    .expect("block a username permanently");
    assert_eq!(blocked.status, "200 OK", "{}", blocked.body);

    let listed = super::route_http_request("GET", "/api/overlay/blocklist", None, "", &state)
        .await
        .expect("list the blocklist");
    assert_eq!(listed.status, "200 OK");
    let listed_json = serde_json::from_str::<serde_json::Value>(&listed.body).unwrap();
    let entries = listed_json["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["target"], "forever-banned");
    assert_eq!(entries[0]["type"], "username");
    assert_eq!(entries[0]["reason"], "repeated abuse");
    assert_eq!(entries[0]["isPermanent"], true);
    assert!(entries[0]["blockedAt"].as_str().is_some());
    assert!(entries[0]["expiresAt"].as_str().is_some());

    let timed = super::route_http_request(
        "POST",
        "/api/overlay/blocklist/ip",
        None,
        r#"{"value":"203.0.113.5","durationMinutes":5}"#,
        &state,
    )
    .await
    .expect("block an ip for a real, non-default duration");
    assert_eq!(timed.status, "200 OK");
    let ban = state
        .security
        .read()
        .await
        .bans
        .iter()
        .find(|record| record.kind == "ip")
        .cloned()
        .expect("ip ban recorded");
    assert!(!ban.is_permanent);
    assert_eq!(ban.expires_at - ban.created_at, 5 * 60);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn mesh_publish_and_lookup_round_trip_through_the_real_hash_database() {
    // Matches the oracle's real PublishHashAsync/LookupHashAsync,
    // which both operate on the exact same hash database. Previously
    // POST /api/mesh/publish wrote into an unrelated
    // "mesh/published/{key}" feature-state key that
    // GET /api/mesh/lookup/{flacKey} never read, so a publish then
    // immediate lookup always reported "not found".
    let (state, _receiver) = test_state();

    let missing =
        super::route_http_request("GET", "/api/mesh/lookup/round-trip-key", None, "", &state)
            .await
            .expect("lookup before publish");
    assert_eq!(missing.status, "404 Not Found");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&missing.body).unwrap()["found"],
        false
    );

    let byte_hash = "a".repeat(64);
    let published = super::route_http_request(
        "POST",
        "/api/mesh/publish",
        None,
        &format!(r#"{{"flacKey":"round-trip-key","byteHash":"{byte_hash}","size":4096}}"#),
        &state,
    )
    .await
    .expect("publish a real hash");
    assert_eq!(published.status, "200 OK", "{}", published.body);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&published.body).unwrap()["published"],
        true
    );

    let found =
        super::route_http_request("GET", "/api/mesh/lookup/round-trip-key", None, "", &state)
            .await
            .expect("lookup after publish");
    assert_eq!(found.status, "200 OK", "{}", found.body);
    let found_json = serde_json::from_str::<serde_json::Value>(&found.body).unwrap();
    assert_eq!(found_json["found"], true);
    assert_eq!(found_json["entry"]["flacKey"], "round-trip-key");
    assert_eq!(found_json["entry"]["byteHash"], byte_hash);
    assert_eq!(found_json["entry"]["size"], 4096);
    assert!(found_json.get("flacKey").is_none());
    assert!(found_json.get("peers").is_none());

    // The "key"/"contentId" aliases the previous implementation
    // accepted (and used to bypass byteHash/size validation) are no
    // longer accepted at all -- only the oracle's real "flacKey".
    let alias_rejected = super::route_http_request(
        "POST",
        "/api/mesh/publish",
        None,
        r#"{"key":"alias-key"}"#,
        &state,
    )
    .await
    .expect("reject the key/contentId alias with no real validation");
    assert_eq!(alias_rejected.status, "400 Bad Request");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn capabilities_peers_routes_use_the_real_capability_store() {
    // Matches the oracle's real GetPeers/GetMeshPeers: known peer
    // *capability* records (mesh-capable subset for mesh-peers), not
    // the generic connected-Soulseek-user list. Previously both
    // "/api/capabilities/peers" and its "/api/v0/capabilities/
    // mesh-peers" alias collapsed into the same wrong handler.
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "native"));
    {
        let mut mesh = state.mesh.write().await;
        mesh.capability_records.push(test_capability_descriptor(
            "mesh-capable-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        ));
        mesh.capability_records
            .push(test_capability_descriptor("plain-peer", vec![]));
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
        let persisted_descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
            "persisted-peer",
            vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
            Vec::new(),
            std::time::Duration::from_secs(300),
            &signing_key,
            std::time::SystemTime::now(),
        )
        .unwrap()
        .sign(&signing_key)
        .unwrap();
        let projection = vec![serde_json::json!({
            "peerId": persisted_descriptor.peer_id,
            "username": persisted_descriptor.username,
            "features": persisted_descriptor.features,
            "endpoints": persisted_descriptor.endpoints,
            "overlayPort": persisted_descriptor.overlay_port,
            "maxPayloadLength": persisted_descriptor.max_payload_length,
            "issuedAtUnix": persisted_descriptor.issued_at_unix,
            "expiresAtUnix": persisted_descriptor.expires_at_unix,
            "publicKey": base64::engine::general_purpose::STANDARD.encode(persisted_descriptor.public_key),
            "signature": persisted_descriptor.signature.map(|value| base64::engine::general_purpose::STANDARD.encode(value)),
        })];
        state
            .controller_features
            .write_for_test()
            .await
            .upsert(
                "hashdb/peers".to_owned(),
                serde_json::json!({"peers": projection}),
            )
            .unwrap();
    }
    // A watched Soulseek user with no capability record at all must
    // not appear in either capability listing.
    state.users.write().await.apply_status(&super::UserStatus {
        username: "unrelated-soulseek-user".to_owned(),
        status: 2,
        privileged: false,
    });

    let peers = super::route_http_request("GET", "/api/capabilities/peers", None, "", &state)
        .await
        .expect("list all known peer capability records");
    assert_eq!(peers.status, "200 OK");
    let peers_json = serde_json::from_str::<serde_json::Value>(&peers.body).unwrap();
    let usernames = peers_json["peers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["username"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(peers_json["count"], 2);
    assert!(usernames.contains(&"mesh-capable-peer"));
    assert!(usernames.contains(&"plain-peer"));
    assert!(!usernames.contains(&"unrelated-soulseek-user"));
    assert_eq!(peers_json["peers"][0]["protocolVersion"], 1);
    assert_eq!(
        peers_json["peers"][0]["clientVersion"],
        "slskdn/runtime-capability-v1"
    );
    assert_eq!(peers_json["peers"][0]["flagsValue"], 8);
    assert_eq!(peers_json["peers"][0]["flags"], "SupportsMeshSync");
    assert_eq!(peers_json["peers"][0]["canMeshSync"], true);
    assert!(peers_json["peers"][0]["lastSeen"].is_string());
    assert_eq!(peers_json["peers"][0]["meshSeqId"], 0);
    assert!(peers_json["peers"][0].get("peerId").is_none());

    let mesh_peers =
        super::route_http_request("GET", "/api/v0/capabilities/mesh-peers", None, "", &state)
            .await
            .expect("list only mesh-capable peer capability records");
    assert_eq!(mesh_peers.status, "200 OK");
    let mesh_peers_json = serde_json::from_str::<serde_json::Value>(&mesh_peers.body).unwrap();
    assert_eq!(mesh_peers_json["count"], 1);
    assert_eq!(mesh_peers_json["peers"][0]["username"], "mesh-capable-peer");
    assert!(mesh_peers_json["peers"][0].get("flags").is_none());

    let peer = super::route_http_request(
        "GET",
        "/api/capabilities/peers/mesh-capable-peer",
        None,
        "",
        &state,
    )
    .await
    .expect("get one capability peer");
    let peer_json = serde_json::from_str::<serde_json::Value>(&peer.body).unwrap();
    assert_eq!(peer.status, "200 OK");
    assert_eq!(peer_json["username"], "mesh-capable-peer");
    assert_eq!(peer_json["flagsValue"], 8);

    let hash_peers = super::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("list hashdb capability peers");
    assert_eq!(hash_peers.status, "200 OK", "{}", hash_peers.body);
    let hash_peers_json = serde_json::from_str::<serde_json::Value>(&hash_peers.body).unwrap();
    assert_eq!(hash_peers_json["count"], 1);
    assert_eq!(hash_peers_json["peers"][0]["peerId"], "mesh-capable-peer");
    assert_eq!(hash_peers_json["peers"][0]["caps"], 8);
    assert_eq!(hash_peers_json["peers"][0]["capsFlags"], "SupportsMeshSync");
    assert_eq!(
        hash_peers_json["peers"][0]["clientVersion"],
        "slskdn/runtime-capability-v1"
    );
    assert!(hash_peers_json["peers"][0]["lastSeen"].is_string());
    assert_eq!(hash_peers_json["peers"][0]["backfillsToday"], 0);

    state
        .backfill
        .write()
        .await
        .record_peer_success("mesh-capable-peer", super::unix_timestamp());
    let hash_peers = super::route_http_request("GET", "/api/v0/hashdb/peers", None, "", &state)
        .await
        .expect("list hashdb peers after a backfill");
    let hash_peers_json = serde_json::from_str::<serde_json::Value>(&hash_peers.body).unwrap();
    assert_eq!(hash_peers_json["peers"][0]["backfillsToday"], 1);

    let persisted = state
        .controller_features
        .read()
        .await
        .get("hashdb/peers")
        .cloned()
        .unwrap();
    let mut restored = super::MeshState::new();
    restored.restore_persisted_capabilities(persisted["peers"].as_array().unwrap());
    assert_eq!(restored.capability_records.len(), 1);
    assert!(restored
        .capability_records
        .iter()
        .any(|record| record.username == "persisted-peer"));
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn security_ban_routes_reject_invalid_ips_and_canonicalize_values() {
    let (state, _receiver) = test_state();
    let invalid = super::route_http_request(
        "POST",
        "/api/security/bans/ip",
        None,
        r#"{"ip":"attacker-controlled"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(invalid.status, "400 Bad Request");
    assert!(state.security.read().await.bans.is_empty());

    let ip = super::route_http_request(
        "POST",
        "/api/security/bans/ip",
        None,
        r#"{"ip":" 2001:0db8:0:0:0:0:0:1 "}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(ip.status, "200 OK");
    let ip_json = serde_json::from_str::<serde_json::Value>(&ip.body).unwrap();
    assert_eq!(ip_json["ip"], "2001:db8::1");
    assert_eq!(ip_json["persisted"], false);

    let username = super::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"  Peer One  "}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(username.status, "200 OK");
    let username_json = serde_json::from_str::<serde_json::Value>(&username.body).unwrap();
    assert_eq!(username_json["username"], "Peer One");
    assert_eq!(username_json["persisted"], false);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn security_ban_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    db.close_for_test().await;

    let response = super::route_http_request(
        "POST",
        "/api/security/bans/username",
        None,
        r#"{"username":"must-persist"}"#,
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("security ban persistence failed"));
    assert!(state.security.read().await.bans.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn security_ban_routes_require_exact_documented_segments() {
    let (state, _receiver) = test_state();
    let security_body = state.security.read().await.json_value().to_string();
    for path in [
        "/not-api/bans",
        "/api/x/y/bans",
        "/api/security/bans/username",
    ] {
        let response = super::route_http_request("GET", path, None, "", &state)
            .await
            .unwrap();
        assert_ne!(response.body, security_body, "unexpected GET match: {path}");
    }
    for path in [
        "/api/security/bans/ip/extra",
        "/api/x/y/bans/ip",
        "/not-api/bans/ip",
    ] {
        super::route_http_request("POST", path, None, r#"{"ip":"192.0.2.1"}"#, &state)
            .await
            .unwrap();
    }
    assert!(state.security.read().await.bans.is_empty());

    let root = super::route_http_request("GET", "/api/bans", None, "", &state)
        .await
        .unwrap();
    assert_eq!(root.body, security_body);
    let namespaced = super::route_http_request("GET", "/api/security/bans", None, "", &state)
        .await
        .unwrap();
    assert_eq!(namespaced.body, security_body);
    let versioned = super::route_http_request("GET", "/api/v0/security/bans", None, "", &state)
        .await
        .unwrap();
    assert_eq!(versioned.body, "[]");

    assert_eq!(super::security_ban_route_tail("/api/bans"), Some(vec![]));
    assert_eq!(
        super::security_ban_route_tail("/api/security/bans/ip/192.0.2.1"),
        Some(vec!["ip", "192.0.2.1"])
    );
    assert_eq!(super::security_ban_route_tail("/not-api/bans"), None);
    assert_eq!(super::security_ban_route_tail("/api/x/y/bans"), None);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn security_unban_removes_legacy_noncanonical_persistence_keys() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    db.upsert_security_ban(&crate::persistence::SecurityBanRecord {
        kind: "username".to_owned(),
        value: "  Peer One  ".to_owned(),
        created_at: 1,
        reason: "Manual ban".to_owned(),
        expires_at: 3_601,
        is_permanent: false,
    })
    .await
    .unwrap();
    db.upsert_security_ban(&crate::persistence::SecurityBanRecord {
        kind: "ip".to_owned(),
        value: "2001:0db8:0:0:0:0:0:1".to_owned(),
        created_at: 1,
        reason: "Manual ban".to_owned(),
        expires_at: 3_601,
        is_permanent: false,
    })
    .await
    .unwrap();
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );

    assert!(
        super::persist_security_unban(&state, "username", "peer one")
            .await
            .unwrap()
    );
    assert!(super::persist_security_unban(&state, "ip", "2001:db8::1")
        .await
        .unwrap());
    assert!(db.list_security_bans().await.unwrap().is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn security_unban_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .security
        .write()
        .await
        .ban("username", "must-remain".to_owned())
        .expect("ban");
    db.close_for_test().await;

    let response = super::route_http_request(
        "DELETE",
        "/api/security/bans/username/must-remain",
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("security unban persistence failed"));
    assert_eq!(state.security.read().await.active_bans(), 1);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_grants_require_live_collections_and_are_revoked_on_delete() {
    let (state, _receiver) = test_state();
    let missing = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        r#"{"collection_id":"col-1","username":"friend"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(missing.status, "404 Not Found");
    assert!(state.share_grants.read().await.records.is_empty());

    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let granted = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}"),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(granted.status, "201 Created");
    assert_eq!(state.share_grants.read().await.records.len(), 1);
    let grant_id = state.share_grants.read().await.records[0].id.clone();
    state
        .stream_tickets
        .write()
        .await
        .issue(
            "share",
            &format!("share:{grant_id}"),
            "content".to_owned(),
            "track.flac".to_owned(),
            Some("friend".to_owned()),
            1,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue grant stream ticket");

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state.share_grants.read().await.records.is_empty());
    assert!(state.stream_tickets.read().await.records.is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn collection_delete_rolls_back_grant_revocation_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let collection_id = state
        .collections
        .write()
        .await
        .create(String::new(), "Private".to_owned(), String::new())
        .expect("collection")
        .id;
    state
        .share_grants
        .write()
        .await
        .create_with_contract(None, collection_id.clone(), "friend".to_owned())
        .expect("share grant");
    db.close_for_test().await;

    let response = super::route_http_request(
        "DELETE",
        &format!("/api/collections/{collection_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("collection deletion persistence failed"));
    assert!(state.collections.read().await.get(&collection_id).is_some());
    assert!(state.share_grants.read().await.get("grant-1").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(all(
    feature = "full-controller-tests",
    not(feature = "legacy-route-dispatch")
))]
async fn collection_delete_releases_store_guards_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-collection-delete-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = super::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create collection deletion database");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .expect("create persisted collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .expect("collection JSON")["id"]
        .as_str()
        .expect("collection id")
        .to_owned();
    let grant_body = format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}");
    let grant = super::route_http_request("POST", "/api/share-grants", None, &grant_body, &state)
        .await
        .expect("create persisted share grant");
    assert_eq!(grant.status, "201 Created");

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let path = format!("/api/collections/{collection_id}");
    let deletion = tokio::spawn(async move {
        super::route_http_request("DELETE", &path, None, "", &task_state).await
    });
    let stores_visible_during_sqlite_wait = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let collection_removed = state
                .collections
                .try_read()
                .is_ok_and(|collections| collections.get(&collection_id).is_none());
            let grants_removed = state
                .share_grants
                .try_read()
                .is_ok_and(|grants| grants.records.is_empty());
            if collection_removed && grants_removed {
                break true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let request_waited_for_sqlite = !deletion.is_finished();
    let collection_reader_responsive = state.collections.try_read().is_ok();
    let grant_reader_responsive = state.share_grants.try_read().is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let response = tokio::time::timeout(Duration::from_secs(3), deletion)
        .await
        .expect("collection delete should finish after releasing SQLite")
        .expect("collection delete route task should join")
        .expect("collection delete response");

    assert!(
        stores_visible_during_sqlite_wait,
        "deleted collection and grants should be readable while SQLite is blocked"
    );
    assert!(
        request_waited_for_sqlite,
        "the request should wait for SQLite"
    );
    assert!(
        collection_reader_responsive,
        "collection readers should not wait for SQLite"
    );
    assert!(
        grant_reader_responsive,
        "share grant readers should not wait for SQLite"
    );
    assert_eq!(response.status, "200 OK");
    assert!(db
        .list_collections(10, 0)
        .await
        .expect("list persisted collections")
        .is_empty());
    assert!(db
        .list_share_grants(10, 0)
        .await
        .expect("list persisted share grants")
        .is_empty());

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_grant_revocation_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .share_grants
        .write()
        .await
        .create_with_contract(None, "private-collection".to_owned(), "friend".to_owned())
        .expect("grant");
    db.close_for_test().await;

    let response =
        super::route_http_request("DELETE", "/api/share-grants/grant-1", None, "", &state)
            .await
            .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response
        .body
        .contains("share grant revocation persistence failed"));
    assert!(state.share_grants.read().await.get("grant-1").is_some());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_grant_create_and_update_roll_back_when_persistence_fails() {
    let create_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (create_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(create_db.clone()),
    );
    create_state
        .collections
        .write()
        .await
        .create(String::new(), "Private".to_owned(), String::new())
        .expect("collection");
    create_db.close_for_test().await;

    let create = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        r#"{"collection_id":"col-1","username":"friend"}"#,
        &create_state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(create.status, "503 Service Unavailable");
    assert!(create.body.contains("share grant persistence failed"));
    assert!(create_state.share_grants.read().await.records.is_empty());

    let update_db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (update_state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(update_db.clone()),
    );
    update_state
        .share_grants
        .write()
        .await
        .create_with_contract(None, "private-collection".to_owned(), "friend".to_owned())
        .expect("grant");
    update_db.close_for_test().await;

    let update = super::route_http_request(
        "PUT",
        "/api/share-grants/grant-1",
        None,
        r#"{"permissions":"restricted"}"#,
        &update_state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(update.status, "503 Service Unavailable");
    assert!(update.body.contains("share grant persistence failed"));
    assert_eq!(
        update_state
            .share_grants
            .read()
            .await
            .get("grant-1")
            .expect("rolled-back grant")
            .permissions,
        "read"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(all(
    feature = "full-controller-tests",
    not(feature = "legacy-route-dispatch")
))]
async fn share_grant_create_releases_store_readers_during_sqlite_io() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let db_path = std::env::temp_dir().join(format!(
        "slskr-legacy-share-grant-lock-test-{}-{unique}.db",
        std::process::id()
    ));
    let db = super::persistence::DatabaseManager::new(
        db_path.to_str().expect("database path should be UTF-8"),
    )
    .await
    .expect("create share grant database");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .expect("create persisted collection");
    assert_eq!(collection.status, "201 Created");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body)
        .expect("collection JSON")["id"]
        .as_str()
        .expect("collection id")
        .to_owned();

    let blocker_pool = sqlx_sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            sqlx_sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await
        .expect("open SQLite lock connection");
    let mut blocker = blocker_pool
        .acquire()
        .await
        .expect("acquire SQLite lock connection");
    sqlx_core::query::query("BEGIN IMMEDIATE")
        .execute(&mut *blocker)
        .await
        .expect("hold SQLite write lock");

    let task_state = Arc::clone(&state);
    let body = format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}");
    let mutation = tokio::spawn(async move {
        super::route_http_request("POST", "/api/share-grants", None, &body, &task_state).await
    });
    let grant_visible_during_sqlite_wait = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .share_grants
                .try_read()
                .is_ok_and(|grants| grants.get("grant-1").is_some())
            {
                break true;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .is_ok();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let request_waited_for_sqlite = !mutation.is_finished();
    let grant_reader_responsive = state.share_grants.try_read().is_ok();

    sqlx_core::query::query("COMMIT")
        .execute(&mut *blocker)
        .await
        .expect("release SQLite write lock");
    drop(blocker);
    let response = tokio::time::timeout(Duration::from_secs(3), mutation)
        .await
        .expect("share grant create should finish after releasing SQLite")
        .expect("share grant route task should join")
        .expect("share grant response");

    assert!(
        grant_visible_during_sqlite_wait,
        "the candidate grant should be readable while SQLite is blocked"
    );
    assert!(
        request_waited_for_sqlite,
        "the request should wait for SQLite"
    );
    assert!(
        grant_reader_responsive,
        "share grant readers should not wait for SQLite"
    );
    assert_eq!(response.status, "201 Created");
    let persisted_grants = db
        .list_share_grants(10, 0)
        .await
        .expect("list persisted share grants");
    assert!(persisted_grants
        .iter()
        .any(|grant| grant.collection_id == collection_id && grant.username == "friend"));

    blocker_pool.close().await;
    db.close_for_test().await;
    let _ = fs::remove_dir_all(&state.config.state_dir);
    let _ = fs::remove_file(&db_path);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_grant_routes_bound_fields_and_require_exact_helper_paths() {
    let (state, _receiver) = test_state();
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Shared"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let oversized_username = "é".repeat(super::MAX_USER_USERNAME_BYTES);
    let grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(
            "{{\"collection_id\":\"{collection_id}\",\"username\":\"  {oversized_username}  \"}}"
        ),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(grant.status, "201 Created");
    let grant_json = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap();
    let grant_id = grant_json["id"].as_str().unwrap().to_owned();
    assert!(grant_json["username"].as_str().unwrap().len() <= super::MAX_USER_USERNAME_BYTES);

    let oversized_permissions = "p".repeat(super::MAX_SHARE_GRANT_PERMISSIONS_BYTES + 1);
    let updated = super::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}"),
        None,
        &format!("{{\"permissions\":\"{oversized_permissions}\"}}"),
        &state,
    )
    .await
    .unwrap();
    assert_eq!(updated.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.body).unwrap()["permissions"]
            .as_str()
            .unwrap()
            .len(),
        super::MAX_SHARE_GRANT_PERMISSIONS_BYTES
    );

    let reset = super::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}"),
        None,
        r#"{"permissions":"   "}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reset.body).unwrap()["permissions"],
        "read"
    );
    super::route_http_request(
        "PUT",
        &format!("/api/share-grants/{grant_id}/extra"),
        None,
        r#"{"permissions":"corrupted"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(
        state
            .share_grants
            .read()
            .await
            .get(&grant_id)
            .unwrap()
            .permissions,
        "read"
    );

    let token = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    let token_json = serde_json::from_str::<serde_json::Value>(&token.body).unwrap();
    assert_eq!(token_json["created"], true);
    assert_eq!(token_json["persisted"], false);
    assert_eq!(token_json["status"], "ephemeral_compatibility_token");
    assert_eq!(token_json["expiresInSeconds"], 2_592_000);
    let token_value = token_json["token"].as_str().unwrap();
    assert_eq!(token_value.len(), "share-".len() + 64);
    assert!(!token_value.contains(&grant_id));
    let second_token = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    let second_token_json = serde_json::from_str::<serde_json::Value>(&second_token.body).unwrap();
    assert_ne!(second_token_json["token"], token_json["token"]);
    assert!(!state
        .share_access_tokens
        .read()
        .await
        .records
        .contains_key(token_value));
    let malformed_token = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/extra/token"),
        None,
        "{}",
        &state,
    )
    .await
    .unwrap();
    assert_ne!(malformed_token.body, token.body);
    assert_eq!(
        super::share_grant_helper_id(
            &format!("/api/share-grants/{grant_id}/backfill"),
            "backfill"
        ),
        Some(grant_id.as_str())
    );
    assert_eq!(
        super::share_grant_helper_id(
            &format!("/api/share-grants/{grant_id}/extra/backfill"),
            "backfill"
        ),
        None
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn share_tokens_use_headers_and_content_bound_stream_tickets() {
    run_controller_future_on_large_stack("share-token-stream-tickets", || {
        share_tokens_use_headers_and_content_bound_stream_tickets_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn share_tokens_use_headers_and_content_bound_stream_tickets_impl() {
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "route-token"),
    );
    let api_authorization = Some("Bearer route-token");
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        api_authorization,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .unwrap();
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let item = super::route_http_request(
        "POST",
        &format!("/api/collections/{collection_id}/items"),
        api_authorization,
        r#"{"content_id":"content/one","title":"track.flac","kind":"Audio"}"#,
        &state,
    )
    .await
    .unwrap();
    assert_eq!(item.status, "201 Created");
    let grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        api_authorization,
        &format!("{{\"collection_id\":\"{collection_id}\",\"username\":\"friend\"}}"),
        &state,
    )
    .await
    .unwrap();
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let issued = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        api_authorization,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .unwrap();
    let issued_json = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap();
    let token = issued_json["token"].as_str().unwrap().to_owned();
    assert_eq!(issued_json["expiresInSeconds"], 600);
    assert!(!issued.body.contains("?token="));

    let query_token = super::route_http_request(
        "GET",
        &format!(
            "/api/share-grants/{grant_id}/manifest?token={}",
            super::url_encode(&token)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(query_token.status, "400 Bad Request");

    let share_headers = super::RequestSecurityHeaders {
        x_share_token: Some(token.clone()),
        ..Default::default()
    };
    let manifest = super::route_http_request_with_headers(
        "GET",
        &format!("/api/share-grants/{grant_id}/manifest"),
        None,
        "",
        &state,
        share_headers.clone(),
    )
    .await
    .unwrap();
    assert_eq!(manifest.status, "200 OK");
    let manifest_json = serde_json::from_str::<serde_json::Value>(&manifest.body).unwrap();
    assert_eq!(manifest_json["itemCount"], 1);
    assert_eq!(manifest_json["items"][0]["contentId"], "content/one");
    assert!(!manifest.body.contains(&token));

    let query_stream = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fone?token={}",
            super::url_encode(&token)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(query_stream.status, "400 Bad Request");

    let ticket_response = super::route_http_request_with_headers(
        "POST",
        "/api/v0/streams/content%2Fone/share-ticket",
        None,
        "",
        &state,
        share_headers,
    )
    .await
    .unwrap();
    assert_eq!(ticket_response.status, "200 OK");
    let ticket = serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap()
        ["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(!ticket_response.body.contains(&token));

    let stream = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fone?ticket={}",
            super::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(stream.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stream.body).unwrap()["status"],
        "available"
    );

    let wrong_content = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/different?ticket={}",
            super::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(wrong_content.status, "401 Unauthorized");

    let deleted = super::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        api_authorization,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(deleted.status, "200 OK");
    assert!(state.share_access_tokens.read().await.records.is_empty());
    assert!(state.stream_tickets.read().await.records.is_empty());
    let revoked_stream = super::route_http_request(
        "GET",
        &format!(
            "/api/v0/streams/content%2Fone?ticket={}",
            super::url_encode(&ticket)
        ),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(revoked_stream.status, "401 Unauthorized");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_access_tokens_persist_only_digests_and_rehydrate() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    let collection = super::route_http_request(
        "POST",
        "/api/collections",
        None,
        r#"{"name":"Private"}"#,
        &state,
    )
    .await
    .expect("create collection");
    let collection_id = serde_json::from_str::<serde_json::Value>(&collection.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let grant = super::route_http_request(
        "POST",
        "/api/share-grants",
        None,
        &format!(r#"{{"collection_id":"{collection_id}","username":"friend"}}"#),
        &state,
    )
    .await
    .expect("create grant");
    let grant_id = serde_json::from_str::<serde_json::Value>(&grant.body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let issued = super::route_http_request(
        "POST",
        &format!("/api/share-grants/{grant_id}/token"),
        None,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("issue token");
    assert_eq!(issued.status, "201 Created");
    let issued_json = serde_json::from_str::<serde_json::Value>(&issued.body).unwrap();
    assert_eq!(issued_json["persisted"], true);
    assert_eq!(issued_json["status"], "persistent_token");
    let raw_token = issued_json["token"].as_str().unwrap();
    let digest = super::share_access_token_digest(raw_token);

    let persisted = db
        .list_share_access_tokens(0, 10, 0)
        .await
        .expect("list persisted token digests");
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].token_digest, digest);
    assert_ne!(persisted[0].token_digest, raw_token);
    assert_eq!(persisted[0].grant_id, grant_id);
    let valid_grants = [grant_id.as_str()].into_iter().collect::<HashSet<_>>();
    let mut rehydrated = super::ShareAccessTokenStore::from_persisted(persisted, &valid_grants);
    assert_eq!(
        rehydrated.validate(raw_token).map(|record| record.grant_id),
        Some(grant_id.clone())
    );

    let revoked = super::route_http_request(
        "DELETE",
        &format!("/api/share-grants/{grant_id}"),
        None,
        "",
        &state,
    )
    .await
    .expect("revoke grant");
    assert_eq!(revoked.status, "200 OK");
    assert!(db
        .list_share_access_tokens(0, 10, 0)
        .await
        .expect("list revoked token digests")
        .is_empty());
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn share_access_token_issue_rolls_back_when_persistence_fails() {
    let db = super::persistence::DatabaseManager::in_memory()
        .await
        .expect("in-memory db");
    let (state, _receiver) = test_state_with_env_parts(
        MapEnv::default().with("SLSKR_PERSISTENCE_ENABLED", "true"),
        super::SearchStore::new(),
        Some(db.clone()),
    );
    state
        .collections
        .write()
        .await
        .create(String::new(), "Private".to_owned(), String::new())
        .expect("collection");
    state
        .share_grants
        .write()
        .await
        .create_with_contract(None, "col-1".to_owned(), "friend".to_owned())
        .expect("grant");
    db.close_for_test().await;

    let response = super::route_http_request(
        "POST",
        "/api/share-grants/grant-1/token",
        None,
        r#"{"expiresInSeconds":600}"#,
        &state,
    )
    .await
    .expect("failed persistence response");
    assert_eq!(response.status, "503 Service Unavailable");
    assert!(response.body.contains("share access token cleanup failed"));
    assert!(state.share_access_tokens.read().await.records.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn primary_stream_route_serves_authenticated_file_ranges() {
    run_controller_future_on_large_stack("primary-stream-file-ranges", || {
        primary_stream_route_serves_authenticated_file_ranges_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn primary_stream_route_serves_authenticated_file_ranges_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "slskr-stream-share-{}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("create stream share root");
    std::fs::write(root.join("track.flac"), b"0123456789").expect("write stream fixture");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "stream-token")
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let virtual_filename = {
        let shares = state.shares.read().await;
        shares.entries[0].filename.clone()
    };
    let ticket = state
        .stream_tickets
        .write()
        .await
        .issue(
            "share",
            "share:test",
            virtual_filename.clone(),
            virtual_filename.clone(),
            Some("friend".to_owned()),
            10,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue stream ticket")
        .0;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stream server");
    let address = listener.local_addr().expect("stream server address");
    let server_state = Arc::clone(&state);
    let server = std::thread::Builder::new()
        .name("primary-stream-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create primary stream server runtime")
                .block_on(async move {
                    let (stream, _) = listener.accept().await.expect("accept stream request");
                    super::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve stream response");
                });
        })
        .expect("spawn primary stream server");
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect stream client");
    client
        .write_all(
            format!(
                "GET /api/v0/streams/{}?ticket={} HTTP/1.1\r\nHost: localhost\r\nRange: bytes=3-6\r\nConnection: close\r\n\r\n",
                super::url_encode(&virtual_filename),
                super::url_encode(&ticket)
            )
            .as_bytes(),
        )
        .await
        .expect("write stream request");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read stream response");
    server.join().expect("stream server task");
    std::fs::remove_dir_all(root).expect("remove stream fixture");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("stream response header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("stream response headers");
    assert!(
        headers.starts_with("HTTP/1.1 206 Partial Content\r\n"),
        "unexpected stream response: {headers}"
    );
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Length: 4\r\n"));
    assert!(headers.contains("Content-Range: bytes 3-6/10\r\n"));
    assert!(headers.contains("Accept-Ranges: bytes\r\n"));
    assert_eq!(&raw[split + 4..], b"3456");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn preview_ticket_get_is_anonymous_and_streams_local_audio_without_ranges() {
    run_controller_future_on_large_stack("local-preview-ticket-stream", || {
        preview_ticket_get_is_anonymous_and_streams_local_audio_without_ranges_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn preview_ticket_get_is_anonymous_and_streams_local_audio_without_ranges_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let root = std::env::temp_dir().join(format!(
        "slskr-peer-preview-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&root).expect("create preview share root");
    std::fs::write(root.join("preview.flac"), b"preview-bytes").expect("write preview fixture");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSKR_AUTH_DISABLED", "false")
            .with("SLSKR_API_TOKEN", "stream-token")
            .with("SLSKR_SHARE_FIXTURE", "")
            .with("SLSKR_SHARE_DIRS", &root.display().to_string()),
    );
    let virtual_filename = state.shares.read().await.entries[0].filename.clone();
    let ticket = state
        .stream_tickets
        .write()
        .await
        .issue(
            "peer",
            "local-share",
            virtual_filename.clone(),
            virtual_filename,
            Some("peer".to_owned()),
            13,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue peer preview ticket")
        .0;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind preview server");
    let address = listener.local_addr().expect("preview server address");
    let server_state = Arc::clone(&state);
    let server = std::thread::Builder::new()
        .name("local-preview-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create local preview server runtime")
                .block_on(async move {
                    let (stream, _) = listener.accept().await.expect("accept preview request");
                    super::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve preview response");
                });
        })
        .expect("spawn local preview server");
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect preview client");
    client
        .write_all(
            format!(
                "GET /api/v0/peer-streams/{} HTTP/1.1\r\nHost: localhost\r\nRange: bytes=2-4\r\nConnection: close\r\n\r\n",
                super::url_encode(&ticket)
            )
            .as_bytes(),
        )
        .await
        .expect("write preview request");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read preview response");
    server.join().expect("preview server task");
    std::fs::remove_dir_all(root).expect("remove preview fixture");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("preview response header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("preview response headers");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Length: 13\r\n"));
    assert!(headers.contains("Accept-Ranges: none\r\n"));
    assert_eq!(&raw[split + 4..], b"preview-bytes");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn peer_preview_ticket_streams_remote_soulseek_bytes_without_transfer_record() {
    run_controller_future_on_large_stack("peer-preview-ticket-stream", || {
        peer_preview_ticket_streams_remote_soulseek_bytes_without_transfer_record_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn peer_preview_ticket_streams_remote_soulseek_bytes_without_transfer_record_impl() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let peer_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind peer preview source");
    let peer_address = peer_listener.local_addr().expect("peer preview address");
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_TEST_USER_ENDPOINT_OVERRIDES",
        &format!("friend={peer_address}"),
    ));
    let ticket = state
        .stream_tickets
        .write()
        .await
        .issue(
            "peer",
            "peer-preview",
            "Remote/Song.flac".to_owned(),
            "Remote/Song.flac".to_owned(),
            Some("friend".to_owned()),
            4,
            "audio/flac".to_owned(),
            120,
        )
        .expect("issue remote peer preview ticket")
        .0;
    let peer = tokio::spawn(async move {
        let (stream, _) = peer_listener
            .accept()
            .await
            .expect("accept preview negotiation");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("preview negotiation init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "P".to_owned(),
                token: 0,
            }
        );
        let mut messages = slskr_client::stream::PeerMessageConnection::new(init.into_inner());
        assert_eq!(
            messages.receive().await.expect("preview transfer request"),
            super::PeerMessage::TransferRequest(super::TransferRequest {
                filename_encoding: Default::default(),
                direction: 0,
                token: 1,
                filename: "Remote/Song.flac".to_owned(),
                size: None,
            })
        );
        messages
            .send(&super::PeerMessage::TransferResponse(
                super::TransferResponse::Allowed {
                    token: 1,
                    size: Some(4),
                },
            ))
            .await
            .expect("allow preview transfer");

        let (stream, _) = peer_listener
            .accept()
            .await
            .expect("accept preview file transfer");
        let mut init = slskr_client::stream::InitConnection::new(stream);
        assert_eq!(
            init.receive().await.expect("preview file init"),
            slskr_client::protocol::init::InitMessage::PeerInit {
                username: "tester".to_owned(),
                connection_type: "F".to_owned(),
                token: 0,
            }
        );
        let mut file = slskr_client::file_transfer::FileTransferConnection::new(init.into_inner());
        file.send_token(1).await.expect("send preview token");
        assert_eq!(file.receive_offset().await.expect("preview offset"), 0);
        file.write_chunk(b"song")
            .await
            .expect("write preview bytes");
    });

    let http_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind peer preview HTTP server");
    let http_address = http_listener.local_addr().expect("preview HTTP address");
    let server_state = Arc::clone(&state);
    let http = std::thread::Builder::new()
        .name("peer-preview-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create peer preview server runtime")
                .block_on(async move {
                    let (stream, _) = http_listener
                        .accept()
                        .await
                        .expect("accept preview HTTP request");
                    super::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve remote preview response");
                });
        })
        .expect("spawn peer preview server");
    let mut client = tokio::net::TcpStream::connect(http_address)
        .await
        .expect("connect remote preview client");
    client
        .write_all(
            format!(
                "GET /api/v0/peer-streams/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                super::url_encode(&ticket)
            )
            .as_bytes(),
        )
        .await
        .expect("request remote preview");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read remote preview");
    http.join().expect("preview HTTP task");
    peer.await.expect("preview peer task");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("remote preview header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("remote preview headers");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Content-Length: 4\r\n"));
    assert!(headers.contains("Accept-Ranges: none\r\n"));
    assert_eq!(&raw[split + 4..], b"song");
    assert!(state.transfers.read().await.entries.is_empty());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn mesh_preview_ticket_fetches_verifies_streams_and_removes_staging_file() {
    run_controller_future_on_large_stack("mesh-preview-ticket-stream", || {
        mesh_preview_ticket_fetches_verifies_streams_and_removes_staging_file_impl()
    });
}

#[cfg(feature = "full-controller-tests")]
async fn mesh_preview_ticket_fetches_verifies_streams_and_removes_staging_file_impl() {
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let content = Arc::new(b"mesh-preview".to_vec());
    let expected_hash = hex::encode(Sha256::digest(content.as_slice()));
    let source_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh preview source");
    let source_address = source_listener.local_addr().expect("mesh source address");
    let source_content = Arc::clone(&content);
    let source = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = source_listener
                .accept()
                .await
                .expect("accept mesh range request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let count = stream.read(&mut buffer).await.expect("read mesh request");
                request.extend_from_slice(&buffer[..count]);
                if count == 0 || request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).expect("mesh request UTF-8");
            let range = request
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                .expect("mesh range header");
            let (start, end) = range.split_once('-').expect("mesh range bounds");
            let start = start.parse::<usize>().expect("mesh range start");
            let end = end.parse::<usize>().expect("mesh range end");
            let body = &source_content[start..=end];
            stream
                .write_all(
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                        body.len(),
                        source_content.len()
                    )
                    .as_bytes(),
                )
                .await
                .expect("write mesh range headers");
            stream.write_all(body).await.expect("write mesh range body");
        }
    });

    let (state, _receiver) = test_state();
    let ticket_response = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"mesh-content","filename":"Remote/Mesh.flac","peerId":"mesh-peer","size":{},"expectedHash":"{expected_hash}","sourceUrl":"http://{source_address}/content"}}"#,
            content.len()
        ),
        &state,
    )
    .await
    .expect("create executable mesh ticket");
    assert_eq!(ticket_response.status, "200 OK");
    let stream_url = serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap()
        ["streamUrl"]
        .as_str()
        .unwrap()
        .to_owned();
    let http_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh preview HTTP server");
    let http_address = http_listener
        .local_addr()
        .expect("mesh preview HTTP address");
    let server_state = Arc::clone(&state);
    let http = std::thread::Builder::new()
        .name("mesh-preview-http-server".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Runtime::new()
                .expect("create mesh preview server runtime")
                .block_on(async move {
                    let (stream, _) = http_listener
                        .accept()
                        .await
                        .expect("accept mesh preview request");
                    super::handle_http_connection(stream, server_state)
                        .await
                        .expect("serve mesh preview response");
                });
        })
        .expect("spawn mesh preview server");
    let mut client = tokio::net::TcpStream::connect(http_address)
        .await
        .expect("connect mesh preview client");
    client
        .write_all(
            format!("GET {stream_url} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .expect("request mesh preview");
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .expect("read mesh preview");
    http.join().expect("mesh preview HTTP task");
    source.await.expect("mesh preview source task");

    let split = raw
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("mesh preview header boundary");
    let headers = String::from_utf8(raw[..split].to_vec()).expect("mesh preview headers");
    assert!(headers.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(headers.contains("Content-Type: audio/flac\r\n"));
    assert!(headers.contains("Accept-Ranges: none\r\n"));
    assert_eq!(&raw[split + 4..], content.as_slice());
    let preview_dir = state.config.downloads_dir.join(".preview");
    assert_eq!(
        std::fs::read_dir(preview_dir)
            .expect("read preview staging directory")
            .count(),
        0
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn trusted_mesh_preview_fetches_frozen_overlay_content_ranges() {
    use sha2::{Digest, Sha256};
    use std::io::Read as _;

    let root = std::env::temp_dir().join(format!(
        "slskr-trusted-mesh-preview-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(&root).expect("trusted mesh preview root");
    let content = b"trusted frozen overlay mesh preview";
    let content_path = root.join("trusted.flac");
    std::fs::write(&content_path, content).expect("write trusted mesh content");

    let (remote_state, _remote_receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "member=127.0.0.1:1"),
    );
    add_test_share(
        &remote_state,
        "Virtual/Trusted.flac",
        &content_path,
        content.len() as u64,
    )
    .await;
    let gateway = Arc::new(
        super::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("trusted mesh gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    let trusted_peers = serde_json::json!([{
        "peerId": "remote-peer",
        "username": "tester",
        "overlayEndpoint": endpoint.to_string(),
        "certificateSha256": hex::encode(certificate_pin)
    }]);
    let (local_state, _local_receiver) = test_state_with_env(
        MapEnv::default()
            .with("SLSK_USERNAME", "member")
            .with("SLSK_PASSWORD", "secret")
            .with("SLSKR_TRUSTED_MESH_PEERS", &trusted_peers.to_string()),
    );
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "member",
        vec![slskr_client::capabilities::FEATURE_MESH_V1.to_owned()],
        Vec::new(),
        std::time::Duration::from_secs(300),
        &local_state.capability_signing_key,
        std::time::SystemTime::now(),
    )
    .and_then(|descriptor| descriptor.sign(&local_state.capability_signing_key))
    .expect("local trusted mesh capability");
    remote_state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register trusted mesh caller capability");
    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&remote_state)));

    let expected_hash = hex::encode(Sha256::digest(content));
    let ticket_response = super::route_http_request(
        "POST",
        "/api/v0/mesh-streams/tickets",
        None,
        &format!(
            r#"{{"contentId":"Virtual/Trusted.flac","filename":"Remote/Trusted.flac","peerId":"remote-peer","size":{},"expectedHash":"{expected_hash}"}}"#,
            content.len()
        ),
        &local_state,
    )
    .await
    .expect("create trusted overlay mesh ticket");
    assert_eq!(ticket_response.status, "200 OK", "{}", ticket_response.body);
    let ticket = serde_json::from_str::<serde_json::Value>(&ticket_response.body).unwrap()
        ["ticket"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut preview = super::open_remote_mesh_preview_file(&local_state, "mesh", &ticket)
        .await
        .expect("fetch trusted overlay mesh content")
        .expect("trusted overlay preview file");
    let mut received = Vec::new();
    preview
        .file
        .read_to_end(&mut received)
        .expect("read trusted overlay preview");
    assert_eq!(received, content);
    assert_eq!(preview.length, content.len() as u64);
    let cleanup = preview.cleanup_path.take().expect("preview cleanup path");
    drop(preview);
    std::fs::remove_file(cleanup).expect("remove trusted preview staging file");

    gateway_server.abort();
    let _ = std::fs::remove_dir_all(&remote_state.config.state_dir);
    let _ = std::fs::remove_dir_all(&local_state.config.state_dir);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn remote_preview_head_uses_ticket_metadata_without_a_source_connection() {
    use tokio::io::AsyncReadExt as _;

    let ticket = super::PreviewStreamTicket {
        family: "mesh".to_owned(),
        source: "mesh-unresolved".to_owned(),
        content_id: "content".to_owned(),
        filename: "Remote.flac".to_owned(),
        peer_username: Some("remote".to_owned()),
        size: 1_234,
        content_type: "audio/flac".to_owned(),
        source_url: Some("https://127.0.0.1:9/content".to_owned()),
        source_authorization: None,
        overlay_peer_identity: None,
        expected_hash: Some("a".repeat(64)),
        created_at: 1,
        expires_at: u64::MAX,
    };
    let (mut client, mut server) = tokio::io::duplex(4 * 1024);
    let write = tokio::spawn(async move {
        super::write_remote_preview_head_response(
            &mut server,
            &ticket,
            false,
            "X-Request-ID: test\r\n",
            std::time::Duration::from_secs(1),
        )
        .await
    });
    let mut response = Vec::new();
    client.read_to_end(&mut response).await.unwrap();
    let written = write.await.unwrap().unwrap();
    let response = String::from_utf8(response).unwrap();

    assert_eq!(written.content_length, 1_234);
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Length: 1234\r\n"));
    assert!(response.contains("Content-Type: audio/flac\r\n"));
    assert!(response.ends_with("X-Request-ID: test\r\n\r\n"));

    let mut unknown_length = super::PreviewStreamTicket {
        family: "peer".to_owned(),
        source: "peer-unresolved".to_owned(),
        content_id: "content".to_owned(),
        filename: "Remote.flac".to_owned(),
        peer_username: Some("remote".to_owned()),
        size: 0,
        content_type: "audio/flac".to_owned(),
        source_url: None,
        source_authorization: None,
        overlay_peer_identity: None,
        expected_hash: None,
        created_at: 1,
        expires_at: u64::MAX,
    };
    assert!(super::remote_preview_head_ticket(Some(unknown_length.clone()), "peer").is_none());
    unknown_length.size = 1;
    assert!(super::remote_preview_head_ticket(Some(unknown_length), "peer").is_some());
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn library_items_bound_growth_and_checked_ids() {
    let mut library = super::LibraryStore::new();
    for index in 0..super::MAX_LIBRARY_ITEMS {
        library
            .create(
                "Artist".to_owned(),
                format!("Track {index}"),
                "Audio".to_owned(),
            )
            .unwrap();
    }
    assert!(library
        .create(String::new(), "Overflow".to_owned(), "Audio".to_owned())
        .is_none());
    assert_eq!(library.records.len(), super::MAX_LIBRARY_ITEMS);
    let mut exhausted = super::LibraryStore::new();
    exhausted.next_id = u64::MAX;
    assert_eq!(
        exhausted
            .create(String::new(), "Track".to_owned(), "Audio".to_owned())
            .unwrap()
            .id,
        format!("lib-{}", u64::MAX)
    );

    let mut persisted = (1..=super::MAX_LIBRARY_ITEMS + 1)
        .map(|index| crate::persistence::LibraryItemRecord {
            id: format!("lib-{index}"),
            artist: "Artist".to_owned(),
            title: format!("Track {index}"),
            kind: "Audio".to_owned(),
            created_at: 1,
        })
        .collect::<Vec<_>>();
    persisted.push(crate::persistence::LibraryItemRecord {
        id: "lib-1".to_owned(),
        artist: "Duplicate".to_owned(),
        title: "Duplicate".to_owned(),
        kind: "Audio".to_owned(),
        created_at: 2,
    });
    let hydrated = super::LibraryStore::from_persisted(persisted);
    assert_eq!(hydrated.records.len(), super::MAX_LIBRARY_ITEMS);
    assert_eq!(
        hydrated
            .records
            .iter()
            .filter(|item| item.id == "lib-1")
            .count(),
        1
    );
    assert_eq!(hydrated.next_id, super::MAX_LIBRARY_ITEMS as u64 + 2);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn destinations_bound_deduplicate_and_select_one_default() {
    let mut persisted = (0..super::MAX_DESTINATIONS + 2)
        .map(|index| crate::persistence::DestinationRecord {
            id: format!("destination-{index}"),
            name: format!("Destination {index}"),
            path: format!("/downloads/{index}"),
            is_default: index < 2,
            created_at: 1,
            updated_at: 1,
        })
        .collect::<Vec<_>>();
    persisted.push(crate::persistence::DestinationRecord {
        id: "destination-0".to_owned(),
        name: "Duplicate".to_owned(),
        path: "/duplicate".to_owned(),
        is_default: true,
        created_at: 2,
        updated_at: 2,
    });
    let destinations = super::DestinationStore::from_persisted(persisted);
    assert_eq!(destinations.records.len(), super::MAX_DESTINATIONS);
    assert_eq!(
        destinations
            .records
            .iter()
            .filter(|record| record.is_default)
            .count(),
        1
    );
    assert_eq!(
        destinations
            .records
            .iter()
            .filter(|record| record.id == "destination-0")
            .count(),
        1
    );
    assert!(destinations.records[0].is_default);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn destination_validation_allows_children_but_rejects_escape_paths() {
    let root = std::env::temp_dir().join(format!(
        "slskr-destination-validation-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let child = root.join("nested");
    std::fs::create_dir_all(&child).unwrap();
    let destinations = super::DestinationStore::from_config(&root, &[]);

    assert_eq!(
        destinations.normalize_explicit_path(&child.display().to_string()),
        Some(child.clone())
    );
    assert!(destinations
        .normalize_explicit_path(&root.join("../outside").display().to_string())
        .is_none());
    assert!(destinations
        .normalize_explicit_path("relative/nested")
        .is_none());

    let missing = root.join("new").join("nested");
    assert_eq!(
        destinations.normalize_explicit_path(&missing.display().to_string()),
        Some(missing)
    );

    #[cfg(unix)]
    {
        let outside = std::env::temp_dir().join(format!(
            "slskr-destination-validation-outside-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&outside).unwrap();
        let link = root.join("escape");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let escaped_missing = link.join("not-yet-created");
        assert!(destinations
            .normalize_explicit_path(&escaped_missing.display().to_string())
            .is_none());
        std::fs::remove_dir_all(outside).unwrap();
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn searches_bound_active_records_and_avoid_identity_collisions() {
    let mut searches = super::SearchStore::new();
    for index in 0..super::MAX_SEARCH_RECORDS {
        searches
            .create(
                None,
                format!("query-{index}"),
                "global",
                None,
                Vec::new(),
                super::MAX_SEARCH_TTL_SECONDS,
            )
            .unwrap();
    }
    assert_eq!(
        searches
            .create(
                None,
                "overflow".to_owned(),
                "global",
                None,
                Vec::new(),
                super::MAX_SEARCH_TTL_SECONDS,
            )
            .unwrap_err(),
        super::SearchCreateError::CapacityFull
    );
    searches.records[0].status = "completed";
    let evicted = searches
        .create(
            None,
            "replacement".to_owned(),
            "global",
            None,
            Vec::new(),
            super::MAX_SEARCH_TTL_SECONDS,
        )
        .unwrap();
    assert_eq!(evicted.evicted.len(), 1);
    assert_eq!(searches.records.len(), super::MAX_SEARCH_RECORDS);

    let mut wrapped = super::SearchStore::new();
    wrapped.next_token = u32::MAX;
    let external = wrapped
        .create(
            Some("2".to_owned()),
            "external".to_owned(),
            "global",
            None,
            Vec::new(),
            300,
        )
        .unwrap();
    assert_eq!(external.record.token, u32::MAX);
    let after_wrap = wrapped
        .create(None, "wrapped".to_owned(), "global", None, Vec::new(), 300)
        .unwrap();
    assert_eq!(after_wrap.record.token, 1);
    let skips_string_id = wrapped
        .create(None, "skip".to_owned(), "global", None, Vec::new(), 300)
        .unwrap();
    assert_eq!(skips_string_id.record.token, 3);
    assert_eq!(
        wrapped
            .create(
                Some("2".to_owned()),
                "duplicate".to_owned(),
                "global",
                None,
                Vec::new(),
                300,
            )
            .unwrap_err(),
        super::SearchCreateError::DuplicateId
    );

    let mut persisted = (1..=super::MAX_SEARCH_RECORDS + 1)
        .map(|index| crate::persistence::SearchRecord {
            id: index.to_string(),
            query: format!("persisted-{index}"),
            status: "completed".to_owned(),
            result_count: 0,
            created_at: index as i64,
            completed_at: Some(index as i64),
            room: None,
            target: Some("global".to_owned()),
            fallback_attempts: 0,
        })
        .collect::<Vec<_>>();
    persisted.push(crate::persistence::SearchRecord {
        id: "1".to_owned(),
        query: "duplicate".to_owned(),
        status: "completed".to_owned(),
        result_count: 0,
        created_at: 0,
        completed_at: Some(0),
        room: None,
        target: Some("global".to_owned()),
        fallback_attempts: 0,
    });
    let hydrated = super::SearchStore::from_persisted(persisted);
    assert_eq!(hydrated.records.len(), super::MAX_SEARCH_RECORDS);
    assert_eq!(
        hydrated
            .records
            .iter()
            .filter(|record| record.id == "1")
            .count(),
        1
    );
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn transfer_ids_and_tokens_wrap_without_collisions() {
    let mut queue = super::TransferQueue::new_in_memory(8);
    let first = queue.create(0, Some("peer".to_owned()), "first".to_owned(), None, None);
    assert_eq!((first.id, first.token), (1, 1));
    queue.next_id = u64::MAX;
    queue.next_token = u32::MAX;
    let maximum = queue.create(0, Some("peer".to_owned()), "max".to_owned(), None, None);
    assert_eq!((maximum.id, maximum.token), (u64::MAX, u32::MAX));
    let wrapped = queue.create(0, Some("peer".to_owned()), "wrapped".to_owned(), None, None);
    assert_eq!((wrapped.id, wrapped.token), (2, 2));
    let rejected =
        queue.record_rejected_request(0, 99, "rejected".to_owned(), None, "test".to_owned());
    assert_eq!(rejected.id, 3);
    assert_eq!(
        queue
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        queue.entries.len()
    );

    let mut duplicate = wrapped.clone();
    duplicate.filename = "newest".to_owned();
    super::write_transfer_state(&queue.state_path, &[wrapped, duplicate]).unwrap();
    let loaded = super::load_transfer_state(&queue.state_path, 8).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].filename, "newest");
    let _ = std::fs::remove_file(&queue.state_path);
    let _ = std::fs::remove_file(&queue.events_path);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn bounded_store_ids_wrap_without_collisions() {
    let mut events = super::EventStore::new(4);
    events.next_id = u64::MAX;
    let max_event = events.record("test", "max", None);
    let wrapped_event = events.record("test", "wrapped", None);
    assert_eq!((max_event.id, wrapped_event.id), (u64::MAX, 1));

    let mut messages = super::MessageStore::with_max_records(4);
    messages.next_id = u64::MAX;
    let max_message = messages.add("peer".to_owned(), "inbound", "max".to_owned());
    let wrapped_message = messages.add("peer".to_owned(), "inbound", "wrapped".to_owned());
    assert_eq!((max_message.id, wrapped_message.id), (u64::MAX, 1));

    let mut contacts = super::ContactStore::with_max_records(4);
    contacts.next_id = u64::MAX;
    let (max_contact, _) = contacts.create("Alice".to_owned()).unwrap();
    let (wrapped_contact, _) = contacts.create("Bob".to_owned()).unwrap();
    assert_eq!(max_contact.id, format!("contact-{}", u64::MAX));
    assert_eq!(wrapped_contact.id, "contact-1");

    let mut groups = super::ShareGroupStore::with_limits(4, 4);
    groups.next_id = u64::MAX;
    let max_group = groups.create("Max".to_owned(), String::new()).unwrap();
    let wrapped_group = groups.create("Wrapped".to_owned(), String::new()).unwrap();
    assert_eq!(max_group.id, format!("sg-{}", u64::MAX));
    assert_eq!(wrapped_group.id, "sg-1");
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn bounded_content_store_ids_wrap_without_collisions() {
    let mut notes = super::UserNoteStore::new();
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

    let mut interests = super::InterestStore::new();
    interests.next_id = u64::MAX;
    assert_eq!(
        interests.add_liked("max".to_owned()).unwrap().0.id,
        format!("liked-{}", u64::MAX)
    );
    assert_eq!(
        interests.add_hated("wrapped".to_owned()).unwrap().0.id,
        "hated-1"
    );

    let mut grants = super::ShareGrantStore::new();
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

    let mut library = super::LibraryStore::new();
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

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
fn browse_indirect_tokens_wrap_without_aliasing_pending_records() {
    let mut browse = super::BrowseStore::with_limits(4, 4);
    browse.request("alice".to_owned()).unwrap();
    browse.next_indirect_token = u32::MAX;
    assert_eq!(
        browse.mark_indirect_pending("alice", "fallback".to_owned()),
        Some(u32::MAX)
    );
    browse.request("bob".to_owned()).unwrap();
    assert_eq!(
        browse.mark_indirect_pending("bob", "fallback".to_owned()),
        Some(1)
    );
    browse.request("carol".to_owned()).unwrap();
    browse.next_indirect_token = u32::MAX;
    assert_eq!(
        browse.mark_indirect_pending("carol", "fallback".to_owned()),
        Some(2)
    );
    let next_token = browse.next_indirect_token;
    assert_eq!(browse.mark_indirect_pending("missing", String::new()), None);
    assert_eq!(browse.next_indirect_token, next_token);
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn compatibility_projections_use_local_state_for_recommendations_and_activity() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_CONTROLLER_PROFILE", "legacy"));

    let liked = super::route_http_request(
        "POST",
        "/api/soulseek/interests",
        None,
        r#"{"name":"ambient"}"#,
        &state,
    )
    .await
    .expect("add liked interest");
    assert_eq!(liked.status, "201 Created");

    super::route_http_request(
        "POST",
        "/api/soulseek/hated-interests",
        None,
        r#"{"name":"low bitrate"}"#,
        &state,
    )
    .await
    .expect("add hated interest");

    let user_interests = super::route_http_request(
        "GET",
        "/api/soulseek/users/peer/interests",
        None,
        "",
        &state,
    )
    .await
    .expect("user interests");
    let user_interests_json =
        serde_json::from_str::<serde_json::Value>(&user_interests.body).unwrap();
    assert_eq!(user_interests.status, "503 Service Unavailable");
    assert!(user_interests_json["error"]
        .as_str()
        .is_some_and(|error| error.contains("session")));

    let recommendations =
        super::route_http_request("GET", "/api/soulseek/recommendations", None, "", &state)
            .await
            .expect("recommendations");
    let recommendations_json =
        serde_json::from_str::<serde_json::Value>(&recommendations.body).unwrap();
    assert_eq!(
        recommendations_json["recommendations"][0]["query"],
        "ambient"
    );

    let item_recommendations = super::route_http_request(
        "GET",
        "/api/soulseek/items/lib-1/recommendations",
        None,
        "",
        &state,
    )
    .await
    .expect("item recommendations");
    let item_recommendations_json =
        serde_json::from_str::<serde_json::Value>(&item_recommendations.body).unwrap();
    assert_eq!(item_recommendations_json["item_id"], "lib-1");
    assert_eq!(
        item_recommendations_json["recommendations"][0]["interest"],
        "ambient"
    );

    {
        let mut users = state.users.write().await;
        users.watch("near-peer".to_owned()).unwrap();
        if let Some(record) = users
            .records
            .iter_mut()
            .find(|record| record.username == "near-peer")
        {
            record.status = Some("online".to_owned());
        }
    }
    let similar = super::route_http_request(
        "GET",
        "/api/soulseek/items/lib-1/similar-users",
        None,
        "",
        &state,
    )
    .await
    .expect("similar users");
    let similar_json = serde_json::from_str::<serde_json::Value>(&similar.body).unwrap();
    assert_eq!(similar_json["similar_users"][0]["username"], "near-peer");

    super::route_http_request(
        "POST",
        "/api/nowplaying",
        None,
        r#"{"username":"peer","artist":"A","title":"Track"}"#,
        &state,
    )
    .await
    .expect("now playing post");
    let now_playing = super::route_http_request("GET", "/api/nowplaying", None, "", &state)
        .await
        .expect("now playing list");
    let now_playing_json = serde_json::from_str::<serde_json::Value>(&now_playing.body).unwrap();
    assert_eq!(now_playing_json["now_playing"][0]["username"], "peer");
    assert_eq!(now_playing_json["now_playing"][0]["title"], "Track");

    let source_preview = super::route_http_request(
        "POST",
        "/api/v0/source-feed-imports/preview",
        None,
        r#"{"text":"Artist - One\nTwo"}"#,
        &state,
    )
    .await
    .expect("source preview");
    let source_preview_json =
        serde_json::from_str::<serde_json::Value>(&source_preview.body).unwrap();
    assert_eq!(source_preview_json["suggestionCount"], 2);
    assert_eq!(source_preview_json["suggestions"][0]["artist"], "Artist");
    assert_eq!(source_preview_json["suggestions"][1]["title"], "Two");

    super::route_http_request(
        "POST",
        "/api/wishlist",
        None,
        r#"{"artist":"Wish","title":"Song"}"#,
        &state,
    )
    .await
    .expect("wishlist feed seed");
    let source_feeds = super::route_http_request("GET", "/api/source-feeds", None, "", &state)
        .await
        .expect("source feeds");
    let source_feeds_json = serde_json::from_str::<serde_json::Value>(&source_feeds.body).unwrap();
    assert_eq!(source_feeds_json["feeds"][0]["provider"], "wishlist");
    let created_feed = super::route_http_request(
        "POST",
        "/api/source-feeds",
        None,
        r#"{"name":"Manual feed","text":"Feed Artist - Feed Track"}"#,
        &state,
    )
    .await
    .expect("create source feed");
    let created_feed_json = serde_json::from_str::<serde_json::Value>(&created_feed.body).unwrap();
    assert_eq!(created_feed_json["provider"], "manual");
    assert_eq!(created_feed_json["count"], 1);
    let source_feeds = super::route_http_request("GET", "/api/source-feeds", None, "", &state)
        .await
        .expect("source feeds after create");
    let source_feeds_json = serde_json::from_str::<serde_json::Value>(&source_feeds.body).unwrap();
    assert!(source_feeds_json["count"].as_u64().unwrap() >= 2);

    super::route_http_request(
        "POST",
        "/api/v0/transfers",
        None,
        r#"{"direction":0,"peer_username":"peer","filename":"Remote/Song.flac","size":100}"#,
        &state,
    )
    .await
    .expect("create bridge transfer");
    super::route_http_request(
        "POST",
        "/api/v0/transfers/1/progress",
        None,
        r#"{"bytes_transferred":25}"#,
        &state,
    )
    .await
    .expect("progress bridge transfer");
    let bridge_stats =
        super::route_http_request("GET", "/api/bridge/admin/stats", None, "", &state)
            .await
            .expect("bridge stats");
    let bridge_stats_json = serde_json::from_str::<serde_json::Value>(&bridge_stats.body).unwrap();
    assert_eq!(bridge_stats_json["total_requests"], 1);
    assert_eq!(bridge_stats_json["total_bytes"], 25);
    // Matches the oracle's real BridgeStatistics contract: with no
    // embedded Soulfind bridge server, these must stay honest zeros
    // even though a real (unrelated) local transfer just happened --
    // they must never be backfilled from slskR's own transfer queue.
    assert_eq!(
        bridge_stats_json["totalConnections"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(
        bridge_stats_json["currentConnections"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(
        bridge_stats_json["totalDownloads"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(bridge_stats_json["totalSearches"], 0, "{bridge_stats_json}");
    assert_eq!(
        bridge_stats_json["totalRoomJoins"], 0,
        "{bridge_stats_json}"
    );
    assert_eq!(
        bridge_stats_json["totalBytesProxied"], 0,
        "{bridge_stats_json}"
    );

    let bridge_dashboard =
        super::route_http_request("GET", "/api/bridge/admin/dashboard", None, "", &state)
            .await
            .expect("bridge dashboard");
    let bridge_dashboard_json =
        serde_json::from_str::<serde_json::Value>(&bridge_dashboard.body).unwrap();
    assert_eq!(
        bridge_dashboard_json["connectedClients"], 0,
        "{bridge_dashboard_json}"
    );
    assert_eq!(
        bridge_dashboard_json["stats"]["totalConnections"], 0,
        "{bridge_dashboard_json}"
    );
    assert_eq!(
        bridge_dashboard_json["stats"]["totalBytesProxied"], 0,
        "{bridge_dashboard_json}"
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn versioned_interest_mutations_use_item_payload_and_wire_commands() {
    use slskr_client::protocol::server::ServerMessage;
    let (state, mut receiver) = test_state();
    state.session.write().await.state = "connected";
    let liked = super::route_http_request(
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
        matches!(receiver.recv().await.unwrap(), super::SessionCommand::SendServerMessage(ServerMessage::AddThingILike { item }) if item == "ambient")
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn soulfind_bridge_wire_frames_match_target_parser_contract() {
    use tokio::io::AsyncWriteExt as _;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge test listener");
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read bridge frame")
                .expect("bridge client frame");
        assert_eq!(message_type, super::BRIDGE_LOGIN);
        let mut cursor = 0;
        assert_eq!(
            super::bridge_read_string(&payload, &mut cursor).as_deref(),
            Some("legacy-client")
        );
        assert_eq!(
            super::bridge_read_string(&payload, &mut cursor).as_deref(),
            Some("secret")
        );
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            super::BRIDGE_LOGIN_RESPONSE,
            &super::bridge_login_response(true, "Login successful"),
        )
        .await
        .expect("write bridge response");
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("bridge test client");
    let mut login = Vec::new();
    super::bridge_write_string(&mut login, "legacy-client");
    super::bridge_write_string(&mut login, "secret");
    let frame_length = u32::try_from(4 + login.len()).unwrap();
    client.write_all(&frame_length.to_le_bytes()).await.unwrap();
    client
        .write_all(&super::BRIDGE_LOGIN.to_le_bytes())
        .await
        .unwrap();
    client.write_all(&login).await.unwrap();

    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read login response")
        .expect("login response frame");
    assert_eq!(message_type, super::BRIDGE_LOGIN_RESPONSE);
    assert_eq!(payload.first(), Some(&1));
    let mut cursor = 1;
    assert_eq!(
        super::bridge_read_string(&payload, &mut cursor).as_deref(),
        Some("Login successful")
    );
    server.await.unwrap();
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
async fn protocol_behaviors_differential_overlay_gateway_mesh_search() {
    use slskr_client::overlay::{
        MeshHello, MeshSearchRequestMessage, FEATURE_MESH_SEARCH, FEATURE_MESH_SERVICE,
    };

    let root = std::env::temp_dir().join(format!(
        "slskr-overlay-search-differential-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create overlay search state directory");
    let (state, _receiver) = test_state_with_env(
        MapEnv::default().with("SLSKR_TEST_USER_ENDPOINT_OVERRIDES", "member=127.0.0.1:1"),
    );
    let gateway = Arc::new(
        super::private_gateway::Gateway::load_or_create_with_quic(
            "127.0.0.1:0".parse().unwrap(),
            &root,
            None,
        )
        .await
        .expect("load overlay gateway"),
    );
    let endpoint = gateway.bind();
    let certificate_pin = gateway.certificate_sha256();
    *state.runtime_credentials.write().await =
        Some(super::LoginCredentials::default_client("slskR", "secret"));

    add_test_share(
        &state,
        "Virtual/Zed.flac",
        Path::new("/nonexistent/virtual-zed.flac"),
        9,
    )
    .await;
    add_test_share(
        &state,
        "Virtual/Alpha.flac",
        Path::new("/nonexistent/virtual-alpha.flac"),
        7,
    )
    .await;
    add_test_share(
        &state,
        "Virtual/Mid.mp3",
        Path::new("/nonexistent/virtual-mid.mp3"),
        8,
    )
    .await;

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[42; 32]);
    let descriptor = slskr_client::capabilities::PeerCapabilityDescriptor::unsigned(
        "member",
        vec![FEATURE_MESH_SEARCH.to_owned()],
        Vec::new(),
        Duration::from_secs(300),
        &remote_key,
        SystemTime::now(),
    )
    .map(|descriptor| descriptor.with_overlay_port(Some(endpoint.port())))
    .and_then(|descriptor| descriptor.sign(&remote_key))
    .expect("member capability");
    state
        .mesh
        .write()
        .await
        .update_capability(descriptor)
        .expect("register member capability");
    super::remember_peer_endpoint(
        &state,
        super::PeerAddress {
            username: "member".to_owned(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            port: 1,
            obfuscation_type: 0,
            obfuscated_port: 0,
        },
    )
    .await;

    let gateway_server = tokio::spawn(gateway.run(Arc::clone(&state)));
    let mut hello = MeshHello::new(
        "member",
        vec![
            FEATURE_MESH_SERVICE.to_owned(),
            FEATURE_MESH_SEARCH.to_owned(),
        ],
        None,
        None,
        "mesh-search-differential-nonce",
    )
    .expect("mesh search hello");
    hello
        .authenticate(&remote_key, &certificate_pin)
        .expect("authenticate mesh search hello");
    let mut client = slskr_client::overlay::connect_tls_overlay(endpoint, certificate_pin, hello)
        .await
        .expect("connect mesh search gateway");

    let request_id = uuid::Uuid::new_v4().to_string();
    let request = MeshSearchRequestMessage::new(request_id.clone(), "virtual", 2, None)
        .expect("mesh search request");
    let response = client.search(&request).await.expect("mesh search response");
    assert_eq!(response.request_id, request_id);
    assert!(response.truncated);
    assert_eq!(response.files.len(), 2);
    assert_eq!(response.files[0].filename, "Virtual/Alpha.flac");
    assert_eq!(response.files[0].size, 7);
    assert_eq!(response.files[0].codec.as_deref(), Some("FLAC"));
    assert_eq!(
        response.files[0].media_kinds,
        Some(vec!["Music".to_owned()])
    );
    assert_eq!(response.files[1].filename, "Virtual/Mid.mp3");
    assert_eq!(response.files[1].size, 8);
    assert_eq!(response.files[1].codec.as_deref(), Some("MP3"));

    let rows = [
        serde_json::json!({
            "target": "slskdn",
            "subject": "rendezvous-overlay:MeshSearchReq:mesh_search_req",
            "case": "live-bidirectional-exchange",
            "pass": true,
        }),
        serde_json::json!({
            "target": "slskdn",
            "subject": "rendezvous-overlay:MeshSearchResp:mesh_search_resp",
            "case": "live-bidirectional-exchange",
            "pass": true,
        }),
    ];
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create overlay protocol evidence directory");
    fs::write(
        evidence_dir.join("overlay_server_mesh_search.json"),
        serde_json::to_string_pretty(&rows).expect("serialize overlay protocol evidence"),
    )
    .expect("write overlay protocol evidence");

    drop(client);
    gateway_server.abort();
    let _ = gateway_server.await;
    let _ = fs::remove_dir_all(root);
}

async fn send_mesh_sync_fixture_message(
    state: &std::sync::Arc<super::AppState>,
    session: &mut slskr_client::server::ServerSession<tokio::net::TcpStream>,
    fixture: &mut slskr_client::stream::ServerConnection<tokio::net::TcpStream>,
    remote_key: &ed25519_dalek::SigningKey,
    timestamp: i64,
    mut message: slskr_client::mesh_sync::MeshSyncMessage,
) -> String {
    message
        .sign_at(remote_key, timestamp)
        .expect("sign mesh-sync request");
    let body = message
        .encode_private_message()
        .expect("encode mesh-sync request");
    crate::session_runtime::project_server_message(
        state,
        session,
        &slskr_client::protocol::server::ServerMessage::MessageUserResponse(
            slskr_client::protocol::server::PrivateMessage {
                id: 1,
                timestamp: 1,
                username: "mesh-peer".to_owned(),
                message: body,
                is_new: true,
                was_replayed: false,
            },
        ),
    )
    .await;
    match fixture
        .receive_with_direction(slskr_client::protocol::server::Direction::ClientToServer)
        .await
        .expect("receive mesh-sync response")
    {
        slskr_client::protocol::server::ServerMessage::MessageUserRequest { username, message } => {
            assert_eq!(username, "mesh-peer");
            message
        }
        other => panic!("unexpected mesh-sync response: {other:?}"),
    }
}

async fn send_mesh_sync_fixture_without_response(
    state: &std::sync::Arc<super::AppState>,
    session: &mut slskr_client::server::ServerSession<tokio::net::TcpStream>,
    fixture: &mut slskr_client::stream::ServerConnection<tokio::net::TcpStream>,
    remote_key: &ed25519_dalek::SigningKey,
    timestamp: i64,
    mut message: slskr_client::mesh_sync::MeshSyncMessage,
) {
    message
        .sign_at(remote_key, timestamp)
        .expect("sign mesh-sync response fixture");
    let body = message
        .encode_private_message()
        .expect("encode mesh-sync response fixture");
    crate::session_runtime::project_server_message(
        state,
        session,
        &slskr_client::protocol::server::ServerMessage::MessageUserResponse(
            slskr_client::protocol::server::PrivateMessage {
                id: 2,
                timestamp: 2,
                username: "mesh-peer".to_owned(),
                message: body,
                is_new: true,
                was_replayed: false,
            },
        ),
    )
    .await;
    match tokio::time::timeout(
        std::time::Duration::from_millis(100),
        fixture.receive_with_direction(slskr_client::protocol::server::Direction::ClientToServer),
    )
    .await
    {
        Err(_) => {}
        Ok(Ok(response)) => panic!("unexpected mesh-sync response: {response:?}"),
        Ok(Err(error)) => panic!("mesh-sync response channel failed: {error}"),
    }
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
async fn protocol_behaviors_differential_mesh_sync_private_runtime() {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use slskr_client::{
        mesh_sync::{
            DhtStoreMessage, MeshAckMessage, MeshHashEntry, MeshHelloMessage, MeshMessageType,
            MeshPushDeltaMessage, MeshReqChunkMessage, MeshReqDeltaMessage, MeshReqKeyMessage,
            MeshRespChunkMessage, MeshRespKeyMessage, MeshSyncBase, MeshSyncMessage,
        },
        server::ServerSession,
        stream::ServerConnection,
    };

    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh-sync fixture");
    let address = listener.local_addr().expect("mesh-sync fixture address");
    let client = tokio::net::TcpStream::connect(address);
    let server = listener.accept();
    let (client, server) = tokio::join!(client, server);
    let (server, _) = server.expect("accept mesh-sync fixture");
    let mut session = ServerSession::new(ServerConnection::new(server));
    let mut fixture = ServerConnection::new(client.expect("mesh-sync client fixture"));

    let root = std::env::temp_dir().join(format!(
        "slskr-mesh-sync-runtime-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).expect("create mesh-sync fixture directory");
    let local_path = root.join("fixture.flac");
    let local_bytes = b"mesh-sync-proof-fixture";
    fs::write(&local_path, local_bytes).expect("write mesh-sync fixture");
    let filename = "Virtual/fixture.flac";
    add_test_share(&state, filename, &local_path, local_bytes.len() as u64).await;
    let local_key = super::content_discovery::generate_flac_key(filename, local_bytes.len() as u64);
    state
        .content_discovery
        .write()
        .await
        .merge_hash_entries(vec![super::content_discovery::HashDbEntry {
            flac_key: local_key.clone(),
            byte_hash: "a".repeat(64),
            size: local_bytes.len() as u64,
            ..super::content_discovery::HashDbEntry::default()
        }])
        .expect("seed mesh-sync hash database");

    let remote_key = ed25519_dalek::SigningKey::from_bytes(&[53; 32]);
    let now = super::unix_timestamp_millis() as i64;
    let hello = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::Hello(MeshHelloMessage {
            message_type: MeshMessageType::Hello,
            base: MeshSyncBase::default(),
            client_id: "mesh-peer".to_owned(),
            client_version: "fixture".to_owned(),
            latest_sequence_id: 0,
            hash_count: 0,
        }),
    )
    .await;
    let hello = MeshSyncMessage::decode_private_message(&hello).expect("decode hello response");
    assert!(matches!(hello, MeshSyncMessage::Hello(_)));
    hello.verify_signature().expect("verify hello response");

    let delta = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
            message_type: MeshMessageType::ReqDelta,
            base: MeshSyncBase::default(),
            since_sequence_id: 0,
            max_entries: 10,
        }),
    )
    .await;
    let delta = MeshSyncMessage::decode_private_message(&delta).expect("decode delta response");
    delta.verify_signature().expect("verify delta response");
    assert!(matches!(delta, MeshSyncMessage::PushDelta(_)));
    if let MeshSyncMessage::PushDelta(message) = &delta {
        assert_eq!(message.entries.len(), 1);
        for entry in &message.entries {
            slskr_client::mesh_sync::verify_mesh_hash_entry_signature(entry)
                .expect("verify delta entry signature");
        }
    }

    let key_response = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::ReqKey(MeshReqKeyMessage {
            message_type: MeshMessageType::ReqKey,
            base: MeshSyncBase::default(),
            flac_key: local_key.clone(),
        }),
    )
    .await;
    let key_response =
        MeshSyncMessage::decode_private_message(&key_response).expect("decode key response");
    key_response
        .verify_signature()
        .expect("verify key response");
    assert!(matches!(
        key_response,
        MeshSyncMessage::RespKey(MeshRespKeyMessage { found: true, .. })
    ));

    let chunk_response = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
            message_type: MeshMessageType::ReqChunk,
            base: MeshSyncBase::default(),
            flac_key: local_key,
            offset: 0,
            length: 8,
        }),
    )
    .await;
    let chunk_response =
        MeshSyncMessage::decode_private_message(&chunk_response).expect("decode chunk response");
    chunk_response
        .verify_signature()
        .expect("verify chunk response");
    match chunk_response {
        MeshSyncMessage::RespChunk(MeshRespChunkMessage {
            success: true,
            data_base64,
            ..
        }) => assert_eq!(
            BASE64.decode(data_base64).expect("decode chunk"),
            &local_bytes[..8]
        ),
        other => panic!("unexpected chunk response: {other:?}"),
    }

    let pushed_key = "1122334455667788".to_owned();
    let pushed = send_mesh_sync_fixture_message(
        &state,
        &mut session,
        &mut fixture,
        &remote_key,
        now,
        MeshSyncMessage::PushDelta(MeshPushDeltaMessage {
            message_type: MeshMessageType::PushDelta,
            base: MeshSyncBase::default(),
            entries: vec![MeshHashEntry {
                sequence_id: 7,
                flac_key: pushed_key.clone(),
                byte_hash: "b".repeat(64),
                size: 12,
                metadata_flags: None,
                signer_public_key: None,
                signature: None,
            }],
            latest_sequence_id: 7,
            has_more: false,
        }),
    )
    .await;
    let pushed =
        MeshSyncMessage::decode_private_message(&pushed).expect("decode push acknowledgement");
    pushed
        .verify_signature()
        .expect("verify push acknowledgement");
    assert!(matches!(
        pushed,
        MeshSyncMessage::Ack(MeshAckMessage {
            merged_count: 1,
            ..
        })
    ));
    assert!(state
        .content_discovery
        .read()
        .await
        .lookup_hash(&pushed_key)
        .is_some());

    for message in [
        MeshSyncMessage::RespKey(MeshRespKeyMessage {
            message_type: MeshMessageType::RespKey,
            base: MeshSyncBase::default(),
            flac_key: pushed_key.clone(),
            found: false,
            entry: None,
        }),
        MeshSyncMessage::Ack(MeshAckMessage {
            message_type: MeshMessageType::Ack,
            base: MeshSyncBase::default(),
            merged_count: 0,
            latest_sequence_id: 0,
        }),
        MeshSyncMessage::RespChunk(MeshRespChunkMessage {
            message_type: MeshMessageType::RespChunk,
            base: MeshSyncBase::default(),
            flac_key: pushed_key.clone(),
            offset: 0,
            data_base64: String::new(),
            success: false,
        }),
        MeshSyncMessage::DhtStore(DhtStoreMessage {
            message_type: MeshMessageType::DhtStore,
            base: MeshSyncBase::default(),
            key: BASE64.encode(b"key"),
            value: BASE64.encode(b"value"),
            requester_id: "mesh-peer".to_owned(),
            ttl_seconds: 60,
        }),
    ] {
        send_mesh_sync_fixture_without_response(
            &state,
            &mut session,
            &mut fixture,
            &remote_key,
            now,
            message,
        )
        .await;
    }

    let subjects = [
        ("Hello", 1),
        ("ReqDelta", 2),
        ("PushDelta", 3),
        ("ReqKey", 4),
        ("RespKey", 5),
        ("Ack", 6),
        ("ReqChunk", 7),
        ("RespChunk", 8),
    ];
    let mut rows = Vec::new();
    for (name, value) in subjects {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("mesh-sync:{name}:{value}"),
            "case": "live-bidirectional-exchange",
            "pass": true,
        }));
    }
    for (name, value) in [
        ("Hello", 1),
        ("ReqDelta", 2),
        ("PushDelta", 3),
        ("ReqKey", 4),
        ("ReqChunk", 7),
        ("RespKey", 5),
        ("Ack", 6),
        ("RespChunk", 8),
        ("DhtStore", 9),
    ] {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("mesh-sync:{name}:{value}"),
            "case": "decode-dispatch-and-side-effects",
            "pass": true,
        }));
    }
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create mesh-sync evidence directory");
    fs::write(
        evidence_dir.join("mesh_sync_private_runtime.json"),
        serde_json::to_string_pretty(&rows).expect("serialize mesh-sync evidence"),
    )
    .expect("write mesh-sync evidence");
    let _ = fs::remove_dir_all(root);
}

async fn virtual_soulfind_bridge_timeout_and_reconnect(
    message_type: i32,
    payload: Vec<u8>,
) -> bool {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge timeout listener");
    let address = listener.local_addr().expect("bridge timeout address");
    let server_payload = payload.clone();
    let server = tokio::spawn(async move {
        let (mut first, _) = listener.accept().await.expect("accept first bridge client");
        let request = tokio::time::timeout(
            Duration::from_secs(1),
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut first),
        )
        .await;
        if !matches!(
            request,
            Ok(Ok(Some((actual_type, actual_payload))))
                if actual_type == message_type && actual_payload == vec![0xA1]
        ) {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(75)).await;
        drop(first);

        let (mut second, _) = listener
            .accept()
            .await
            .expect("accept reconnected bridge client");
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut second,
            message_type,
            &server_payload,
        )
        .await
        .is_ok()
    });

    let mut first_client = match tokio::net::TcpStream::connect(address).await {
        Ok(client) => client,
        Err(_) => {
            server.abort();
            let _ = server.await;
            return false;
        }
    };
    if crate::soulfind_bridge_runtime::bridge_write_frame(&mut first_client, message_type, &[0xA1])
        .await
        .is_err()
    {
        server.abort();
        let _ = server.await;
        return false;
    }
    let timed_out = tokio::time::timeout(
        Duration::from_millis(25),
        crate::soulfind_bridge_runtime::bridge_read_frame(&mut first_client),
    )
    .await
    .is_err();
    drop(first_client);

    let mut second_client = match tokio::net::TcpStream::connect(address).await {
        Ok(client) => client,
        Err(_) => {
            server.abort();
            let _ = server.await;
            return false;
        }
    };
    let reconnected = match tokio::time::timeout(
        Duration::from_secs(1),
        crate::soulfind_bridge_runtime::bridge_read_frame(&mut second_client),
    )
    .await
    {
        Ok(Ok(Some((actual_type, actual_payload)))) => {
            actual_type == message_type && actual_payload == payload
        }
        _ => false,
    };
    let server_pass = server.await.is_ok_and(|result| result);
    timed_out && reconnected && server_pass
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
async fn protocol_behaviors_differential_virtual_soulfind_bridge_round_trips() {
    // These fields mirror BinaryReader/BinaryWriter in the frozen
    // SoulseekProtocolParser.cs.  Build the expected payloads separately
    // from slskR's bridge helpers so this test catches layout drift, not
    // merely helper-to-helper agreement.
    fn oracle_string(payload: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        payload.extend_from_slice(&(i32::try_from(bytes.len()).unwrap()).to_le_bytes());
        payload.extend_from_slice(bytes);
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge differential listener");
    let address = listener.local_addr().expect("bridge differential address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept bridge differential");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read login request")
                .expect("login request frame");
        let mut expected = Vec::new();
        oracle_string(&mut expected, "legacy-client");
        oracle_string(&mut expected, "secret");
        assert_eq!(message_type, super::BRIDGE_LOGIN);
        assert_eq!(payload, expected);
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            super::BRIDGE_LOGIN_RESPONSE,
            &super::bridge_login_response(true, "Login successful"),
        )
        .await
        .expect("write login response");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read search request")
                .expect("search request frame");
        let mut expected = Vec::new();
        oracle_string(&mut expected, "ambient");
        expected.extend_from_slice(&41_i32.to_le_bytes());
        assert_eq!(message_type, super::BRIDGE_SEARCH_REQUEST);
        assert_eq!(payload, expected);
        let mut response = Vec::new();
        response.extend_from_slice(&41_i32.to_le_bytes());
        response.extend_from_slice(&1_i32.to_le_bytes());
        oracle_string(&mut response, "slskR");
        oracle_string(&mut response, "Ambient/Track.flac");
        response.extend_from_slice(&4_096_i64.to_le_bytes());
        response.extend_from_slice(&0_i32.to_le_bytes());
        oracle_string(&mut response, "flac");
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            super::BRIDGE_SEARCH_RESPONSE,
            &response,
        )
        .await
        .expect("write search response");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read download request")
                .expect("download request frame");
        let mut expected = Vec::new();
        oracle_string(&mut expected, "peer");
        oracle_string(&mut expected, "Ambient/Track.flac");
        expected.extend_from_slice(&42_i32.to_le_bytes());
        assert_eq!(message_type, super::BRIDGE_DOWNLOAD_REQUEST);
        assert_eq!(payload, expected);
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            super::BRIDGE_DOWNLOAD_RESPONSE,
            &super::bridge_download_wire_response(true, "transfer-id", 42),
        )
        .await
        .expect("write download response");

        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .expect("read room list request")
                .expect("room list request frame");
        assert_eq!(message_type, super::BRIDGE_ROOM_LIST_REQUEST);
        assert!(payload.is_empty());
        let mut response = Vec::new();
        response.extend_from_slice(&1_i32.to_le_bytes());
        oracle_string(&mut response, "ambient");
        response.extend_from_slice(&7_i32.to_le_bytes());
        crate::soulfind_bridge_runtime::bridge_write_frame(
            &mut stream,
            super::BRIDGE_ROOM_LIST_RESPONSE,
            &response,
        )
        .await
        .expect("write room list response");
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect bridge differential client");
    let mut rows = Vec::new();
    macro_rules! record {
        ($name:literal, $value:literal) => {
            rows.push(serde_json::json!({
                "target": "slskdn",
                "subject": concat!("virtual-soulfind-bridge:", $name, ":", $value),
                "case": "exact-frame-and-encoding",
                "pass": true,
            }));
            rows.push(serde_json::json!({
                "target": "slskdn",
                "subject": concat!("virtual-soulfind-bridge:", $name, ":", $value),
                "case": "live-bidirectional-exchange",
                "pass": true,
            }));
        };
    }

    let mut login = Vec::new();
    oracle_string(&mut login, "legacy-client");
    oracle_string(&mut login, "secret");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, super::BRIDGE_LOGIN, &login)
        .await
        .expect("send login request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read login response")
        .expect("login response frame");
    assert_eq!(message_type, super::BRIDGE_LOGIN_RESPONSE);
    let mut expected = vec![1_u8];
    oracle_string(&mut expected, "Login successful");
    assert_eq!(payload, expected);
    record!("Login", "1");
    record!("LoginResponse", "2");

    let mut search = Vec::new();
    oracle_string(&mut search, "ambient");
    search.extend_from_slice(&41_i32.to_le_bytes());
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_SEARCH_REQUEST,
        &search,
    )
    .await
    .expect("send search request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read search response")
        .expect("search response frame");
    assert_eq!(message_type, super::BRIDGE_SEARCH_RESPONSE);
    let mut expected = Vec::new();
    expected.extend_from_slice(&41_i32.to_le_bytes());
    expected.extend_from_slice(&1_i32.to_le_bytes());
    oracle_string(&mut expected, "slskR");
    oracle_string(&mut expected, "Ambient/Track.flac");
    expected.extend_from_slice(&4_096_i64.to_le_bytes());
    expected.extend_from_slice(&0_i32.to_le_bytes());
    oracle_string(&mut expected, "flac");
    assert_eq!(payload, expected);
    record!("SearchRequest", "3");
    record!("SearchResponse", "4");

    let mut download = Vec::new();
    oracle_string(&mut download, "peer");
    oracle_string(&mut download, "Ambient/Track.flac");
    download.extend_from_slice(&42_i32.to_le_bytes());
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_DOWNLOAD_REQUEST,
        &download,
    )
    .await
    .expect("send download request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read download response")
        .expect("download response frame");
    assert_eq!(message_type, super::BRIDGE_DOWNLOAD_RESPONSE);
    let mut expected = vec![1_u8];
    oracle_string(&mut expected, "transfer-id");
    expected.extend_from_slice(&42_i32.to_le_bytes());
    assert_eq!(payload, expected);
    record!("DownloadRequest", "5");
    record!("DownloadResponse", "6");

    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_ROOM_LIST_REQUEST,
        &[],
    )
    .await
    .expect("send room list request");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read room list response")
        .expect("room list response frame");
    assert_eq!(message_type, super::BRIDGE_ROOM_LIST_RESPONSE);
    let mut expected = Vec::new();
    expected.extend_from_slice(&1_i32.to_le_bytes());
    oracle_string(&mut expected, "ambient");
    expected.extend_from_slice(&7_i32.to_le_bytes());
    assert_eq!(payload, expected);
    record!("RoomListRequest", "7");
    record!("RoomListResponse", "8");

    server.await.expect("bridge differential server task");

    for (name, message_type) in [
        ("Login", super::BRIDGE_LOGIN),
        ("LoginResponse", super::BRIDGE_LOGIN_RESPONSE),
        ("SearchRequest", super::BRIDGE_SEARCH_REQUEST),
        ("SearchResponse", super::BRIDGE_SEARCH_RESPONSE),
        ("DownloadRequest", super::BRIDGE_DOWNLOAD_REQUEST),
        ("DownloadResponse", super::BRIDGE_DOWNLOAD_RESPONSE),
        ("RoomListRequest", super::BRIDGE_ROOM_LIST_REQUEST),
        ("RoomListResponse", super::BRIDGE_ROOM_LIST_RESPONSE),
    ] {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "timeout-cancel-reconnect-and-failure",
            "pass": virtual_soulfind_bridge_timeout_and_reconnect(
                message_type,
                vec![0xB2, message_type as u8],
            )
            .await,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_round_trips.json"),
        serde_json::to_string_pretty(&rows).expect("serialize bridge protocol evidence"),
    )
    .expect("write bridge protocol evidence");
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
async fn protocol_behaviors_differential_virtual_soulfind_bridge_raw_frames() {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    // The frozen bridge parser accepts message types 9..15 at the generic
    // frame boundary, while its handler intentionally dispatches only
    // types 1..8.  Prove the wire framing for these opaque messages, but
    // do not claim payload decoding or handler side effects that neither
    // runtime implements.
    const CASES: [(&str, i32, &[u8]); 7] = [
        ("RoomJoinRequest", 9, b"room-join"),
        ("RoomJoinResponse", 10, b"room-joined"),
        ("RoomLeaveRequest", 11, b"room-leave"),
        ("RoomMessage", 12, b"room-message"),
        ("UserStatus", 13, b"user-status"),
        ("PeerInfo", 14, b"peer-info"),
        ("FileTransfer", 15, b"file-transfer"),
    ];

    fn oracle_frame(message_type: i32, payload: &[u8]) -> Vec<u8> {
        let length = u32::try_from(4_usize + payload.len()).expect("fixture frame length");
        let mut frame = Vec::with_capacity(4 + length as usize);
        frame.extend_from_slice(&length.to_le_bytes());
        frame.extend_from_slice(&message_type.to_le_bytes());
        frame.extend_from_slice(payload);
        frame
    }

    async fn truncated_frame_is_rejected(message_type: i32) -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind truncated bridge frame listener");
        let address = listener
            .local_addr()
            .expect("truncated bridge frame listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("accept truncated bridge frame client");
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .is_err()
        });
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect truncated bridge frame client");
        client
            .write_all(&7_u32.to_le_bytes())
            .await
            .expect("write truncated bridge frame length");
        client
            .write_all(&message_type.to_le_bytes())
            .await
            .expect("write truncated bridge frame type");
        client
            .write_all(&[0xA0, 0xB0])
            .await
            .expect("write truncated bridge frame payload");
        drop(client);
        server.await.expect("truncated bridge frame server task")
    }

    async fn oversized_frame_is_rejected(message_type: i32) -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind oversized bridge frame listener");
        let address = listener
            .local_addr()
            .expect("oversized bridge frame listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("accept oversized bridge frame client");
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream)
                .await
                .is_err()
        });
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect oversized bridge frame client");
        let length = u32::try_from(super::BRIDGE_MAX_FRAME_BYTES + 1)
            .expect("oversized bridge frame length");
        client
            .write_all(&length.to_le_bytes())
            .await
            .expect("write oversized bridge frame length");
        client
            .write_all(&message_type.to_le_bytes())
            .await
            .expect("write oversized bridge frame type");
        server.await.expect("oversized bridge frame server task")
    }

    async fn unknown_type_is_preserved() -> bool {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind unknown bridge frame listener");
        let address = listener
            .local_addr()
            .expect("unknown bridge frame listener address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener
                .accept()
                .await
                .expect("accept unknown bridge frame client");
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream).await
                == Ok(Some((999, vec![0xCC])))
        });
        let mut client = tokio::net::TcpStream::connect(address)
            .await
            .expect("connect unknown bridge frame client");
        client
            .write_all(&5_u32.to_le_bytes())
            .await
            .expect("write unknown bridge frame length");
        client
            .write_all(&999_i32.to_le_bytes())
            .await
            .expect("write unknown bridge frame type");
        client
            .write_all(&[0xCC])
            .await
            .expect("write unknown bridge frame payload");
        server.await.expect("unknown bridge frame server task")
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind raw bridge frame listener");
    let address = listener
        .local_addr()
        .expect("raw bridge frame listener address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener
            .accept()
            .await
            .expect("accept raw bridge frame client");
        for (_, message_type, payload) in CASES {
            let expected = oracle_frame(message_type, payload);
            let mut actual = vec![0_u8; expected.len()];
            stream
                .read_exact(&mut actual)
                .await
                .expect("read raw bridge request frame");
            assert_eq!(actual, expected, "raw bridge request frame");
            stream
                .write_all(&expected)
                .await
                .expect("write raw bridge response frame");
        }
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect raw bridge frame client");
    let mut rows = Vec::new();
    for (name, message_type, payload) in CASES {
        crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, message_type, payload)
            .await
            .expect("write raw bridge request frame");
        let (actual_type, actual_payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
                .await
                .expect("read raw bridge response frame")
                .expect("raw bridge response frame");
        assert_eq!(actual_type, message_type, "raw bridge response type");
        assert_eq!(actual_payload, payload, "raw bridge response payload");
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "exact-frame-and-encoding",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "live-bidirectional-exchange",
            "pass": true,
        }));
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "timeout-cancel-reconnect-and-failure",
            "pass": virtual_soulfind_bridge_timeout_and_reconnect(
                message_type,
                payload.to_vec(),
            )
            .await,
        }));
    }

    server.await.expect("raw bridge frame server task");

    const ALL_TYPES: [(&str, i32); 15] = [
        ("Login", 1),
        ("LoginResponse", 2),
        ("SearchRequest", 3),
        ("SearchResponse", 4),
        ("DownloadRequest", 5),
        ("DownloadResponse", 6),
        ("RoomListRequest", 7),
        ("RoomListResponse", 8),
        ("RoomJoinRequest", 9),
        ("RoomJoinResponse", 10),
        ("RoomLeaveRequest", 11),
        ("RoomMessage", 12),
        ("UserStatus", 13),
        ("PeerInfo", 14),
        ("FileTransfer", 15),
    ];
    let unknown_type_pass = unknown_type_is_preserved().await;
    for (name, message_type) in ALL_TYPES {
        let malformed_pass = truncated_frame_is_rejected(message_type).await
            && oversized_frame_is_rejected(message_type).await
            && unknown_type_pass;
        assert!(
            malformed_pass,
            "bridge frame boundary handling failed for {name}:{message_type}"
        );
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{message_type}"),
            "case": "malformed-truncated-oversize-and-unknown",
            "pass": malformed_pass,
        }));
    }

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create protocol evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_raw_frames.json"),
        serde_json::to_string_pretty(&rows).expect("serialize raw bridge frame evidence"),
    )
    .expect("write raw bridge frame evidence");
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
async fn protocol_behaviors_differential_virtual_soulfind_bridge_dispatch() {
    let (mut state, mut receiver) = test_state();
    let expected_username = state
        .config
        .username
        .clone()
        .unwrap_or_else(|| "slskR".to_owned());
    Arc::get_mut(&mut state)
        .expect("test state has one owner")
        .config
        .media_services
        .virtual_soulfind
        .bridge
        .require_auth = false;
    add_test_share(
        &state,
        "Ambient/Track.flac",
        Path::new("/nonexistent/ambient-track.flac"),
        4_096,
    )
    .await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge dispatch listener");
    let address = listener.local_addr().expect("bridge dispatch address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept bridge dispatch client");
        super::bridge_handle_client(
            "protocol-differential-client".to_owned(),
            stream,
            server_state,
        )
        .await;
    });

    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect bridge dispatch client");
    let mut rows = Vec::new();
    let mut record = |name: &str, value: i32, pass: bool| {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{value}"),
            "case": "decode-dispatch-and-side-effects",
            "pass": pass,
        }));
    };

    let mut login = Vec::new();
    super::bridge_write_string(&mut login, "legacy-client");
    super::bridge_write_string(&mut login, "ignored-with-auth-disabled");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, super::BRIDGE_LOGIN, &login)
        .await
        .expect("send bridge dispatch login");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch login response")
        .expect("bridge dispatch login response frame");
    let mut cursor = 1;
    let login_pass = message_type == super::BRIDGE_LOGIN_RESPONSE
        && payload.first() == Some(&1)
        && super::bridge_read_string(&payload, &mut cursor).as_deref() == Some("Login successful");

    let mut search = Vec::new();
    super::bridge_write_string(&mut search, "ambient");
    super::bridge_write_i32(&mut search, 41);
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_SEARCH_REQUEST,
        &search,
    )
    .await
    .expect("send bridge dispatch search");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch search response")
        .expect("bridge dispatch search response frame");
    let mut cursor = 0;
    let search_pass = message_type == super::BRIDGE_SEARCH_RESPONSE
        && super::bridge_read_i32(&payload, &mut cursor) == Some(41)
        && super::bridge_read_i32(&payload, &mut cursor) == Some(1)
        && super::bridge_read_string(&payload, &mut cursor).as_deref()
            == Some(expected_username.as_str())
        && super::bridge_read_string(&payload, &mut cursor).as_deref()
            == Some("Ambient/Track.flac");

    let mut download = Vec::new();
    super::bridge_write_string(&mut download, "peer");
    super::bridge_write_string(&mut download, "Ambient/Track.flac");
    super::bridge_write_i32(&mut download, 42);
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_DOWNLOAD_REQUEST,
        &download,
    )
    .await
    .expect("send bridge dispatch download");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch download response")
        .expect("bridge dispatch download response frame");
    let mut cursor = 1;
    let transfer_id = super::bridge_read_string(&payload, &mut cursor).unwrap_or_default();
    let download_pass = message_type == super::BRIDGE_DOWNLOAD_RESPONSE
        && payload.first() == Some(&1)
        && !transfer_id.is_empty()
        && super::bridge_read_i32(&payload, &mut cursor) == Some(42);
    let queued_download = matches!(
        receiver.recv().await,
        Some(super::SessionCommand::TransferPeer { username, .. }) if username == "peer"
    );
    let download_pass = download_pass && queued_download;

    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_ROOM_LIST_REQUEST,
        &[],
    )
    .await
    .expect("send bridge dispatch room list");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read bridge dispatch room list response")
        .expect("bridge dispatch room list response frame");
    let mut cursor = 0;
    let room_list_pass = message_type == super::BRIDGE_ROOM_LIST_RESPONSE
        && super::bridge_read_i32(&payload, &mut cursor) == Some(0);

    // The frozen bridge parser declares these message types, but the
    // frozen proxy has no handlers for them.  Its observable contract is
    // therefore the generic LoginResponse-shaped error, not a fabricated
    // typed payload or side effect.
    for (name, value) in [
        ("RoomJoinRequest", 9),
        ("RoomJoinResponse", 10),
        ("RoomLeaveRequest", 11),
        ("RoomMessage", 12),
        ("UserStatus", 13),
        ("PeerInfo", 14),
        ("FileTransfer", 15),
    ] {
        crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, value, &[0x01, 0x02, 0x03])
            .await
            .expect("send unsupported bridge message type");
        let (message_type, payload) =
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
                .await
                .expect("read unsupported bridge error")
                .expect("unsupported bridge error frame");
        let mut cursor = 1;
        let unsupported_pass = message_type == super::BRIDGE_LOGIN_RESPONSE
            && payload.first() == Some(&0)
            && super::bridge_read_string(&payload, &mut cursor).as_deref()
                == Some("Unknown message type");
        record(name, value, unsupported_pass);
    }

    record("Login", 1, login_pass);
    record("LoginResponse", 2, login_pass);
    record("SearchRequest", 3, search_pass);
    record("SearchResponse", 4, search_pass);
    record("DownloadRequest", 5, download_pass);
    record("DownloadResponse", 6, download_pass);
    record("RoomListRequest", 7, room_list_pass);
    record("RoomListResponse", 8, room_list_pass);

    assert!(rows.iter().all(|row| row["pass"] == true), "{rows:?}");
    drop(client);
    server.await.expect("bridge dispatch server task");

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create bridge dispatch evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_dispatch.json"),
        serde_json::to_string_pretty(&rows).expect("serialize bridge dispatch evidence"),
    )
    .expect("write bridge dispatch evidence");
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
#[cfg(any(feature = "full-controller-tests", feature = "bounded-protocol-tests"))]
async fn protocol_behaviors_differential_virtual_soulfind_bridge_malformed_frames() {
    let (mut state, _receiver) = test_state();
    Arc::get_mut(&mut state)
        .expect("test state has one owner")
        .config
        .media_services
        .virtual_soulfind
        .bridge
        .require_auth = false;

    let mut rows = Vec::new();
    let mut record = |name: &str, value: i32, pass: bool| {
        rows.push(serde_json::json!({
            "target": "slskdn",
            "subject": format!("virtual-soulfind-bridge:{name}:{value}"),
            "case": "malformed-truncated-oversize-and-unknown",
            "pass": pass,
        }));
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("malformed login listener");
    let address = listener.local_addr().expect("malformed login address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept malformed login client");
        super::bridge_handle_client("malformed-login".to_owned(), stream, server_state).await;
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect malformed login client");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, super::BRIDGE_LOGIN, &[])
        .await
        .expect("send malformed login");
    let (message_type, payload) = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read malformed login response")
        .expect("malformed login response frame");
    let mut cursor = 1;
    let login_pass = message_type == super::BRIDGE_LOGIN_RESPONSE
        && payload.first() == Some(&0)
        && super::bridge_read_string(&payload, &mut cursor).is_some();
    drop(client);
    server.await.expect("malformed login server task");
    record("Login", 1, login_pass);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("malformed search listener");
    let address = listener.local_addr().expect("malformed search address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept malformed search client");
        super::bridge_handle_client("malformed-search".to_owned(), stream, server_state).await;
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect malformed search client");
    let mut login = Vec::new();
    super::bridge_write_string(&mut login, "legacy-client");
    super::bridge_write_string(&mut login, "ignored");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, super::BRIDGE_LOGIN, &login)
        .await
        .expect("send valid login before malformed search");
    let _ = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read valid login response");
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_SEARCH_REQUEST,
        &[],
    )
    .await
    .expect("send malformed search");
    let search_pass = matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        )
        .await,
        Ok(Ok(None))
    );
    drop(client);
    server.await.expect("malformed search server task");
    record("SearchRequest", 3, search_pass);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("malformed download listener");
    let address = listener.local_addr().expect("malformed download address");
    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        let (stream, _) = listener
            .accept()
            .await
            .expect("accept malformed download client");
        super::bridge_handle_client("malformed-download".to_owned(), stream, server_state).await;
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect malformed download client");
    let mut login = Vec::new();
    super::bridge_write_string(&mut login, "legacy-client");
    super::bridge_write_string(&mut login, "ignored");
    crate::soulfind_bridge_runtime::bridge_write_frame(&mut client, super::BRIDGE_LOGIN, &login)
        .await
        .expect("send valid login before malformed download");
    let _ = crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        .await
        .expect("read valid login response");
    crate::soulfind_bridge_runtime::bridge_write_frame(
        &mut client,
        super::BRIDGE_DOWNLOAD_REQUEST,
        &[],
    )
    .await
    .expect("send malformed download");
    let download_pass = matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut client)
        )
        .await,
        Ok(Ok(None))
    );
    drop(client);
    server.await.expect("malformed download server task");
    record("DownloadRequest", 5, download_pass);

    assert!(rows.iter().all(|row| row["pass"] == true), "{rows:?}");
    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    fs::create_dir_all(&evidence_dir).expect("create bridge malformed evidence directory");
    fs::write(
        evidence_dir.join("virtual_soulfind_bridge_malformed.json"),
        serde_json::to_string_pretty(&rows).expect("serialize bridge malformed evidence"),
    )
    .expect("write bridge malformed evidence");
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_rejects_oversized_wire_frames() {
    use tokio::io::AsyncWriteExt as _;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge test listener");
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        assert_eq!(
            crate::soulfind_bridge_runtime::bridge_read_frame(&mut stream).await,
            Err("invalid bridge message length")
        );
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("bridge test client");
    let oversized = u32::try_from(super::BRIDGE_MAX_FRAME_BYTES + 1).unwrap();
    client.write_all(&oversized.to_le_bytes()).await.unwrap();
    server.await.unwrap();
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
async fn bridge_frame_reads_have_a_deadline() {
    use tokio::io::AsyncWriteExt as _;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bridge timeout listener");
    let address = listener.local_addr().expect("bridge timeout address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener
            .accept()
            .await
            .expect("accept bridge timeout client");
        crate::soulfind_bridge_runtime::bridge_read_frame_with_timeout(
            &mut stream,
            Duration::from_millis(20),
        )
        .await
    });
    let mut client = tokio::net::TcpStream::connect(address)
        .await
        .expect("connect bridge timeout client");
    client
        .write_all(&5_u32.to_le_bytes())
        .await
        .expect("write partial bridge frame");
    assert_eq!(
        server.await.expect("bridge timeout server task"),
        Err("bridge read timed out")
    );
}
