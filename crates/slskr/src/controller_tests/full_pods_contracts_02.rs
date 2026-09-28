//! Controller full pods contracts 02 ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn pod_membership_workflow_queues_accepts_lists_leaves_and_cancels() {
    let (state, _receiver) = test_state();
    {
        let mut rooms = state.rooms.write().await;
        rooms.join("pod:workflow".to_owned()).expect("workflow pod");
        rooms
            .add_member("pod:workflow", "owner-peer".to_owned())
            .expect("owner capacity")
            .expect("workflow pod");
        rooms
            .add_member("pod:workflow", "ordinary-peer".to_owned())
            .expect("ordinary member capacity")
            .expect("workflow pod");
        rooms.records[0].operated = true;
    }
    state.pod_membership_workflow.write().await.set_role(
        "pod:workflow",
        "owner-peer",
        "owner".to_owned(),
    );

    let join = crate::route_http_request(
        "POST",
        "/api/v0/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","requestedRole":"moderator"}"#,
        &state,
    )
    .await
    .expect("join request");
    assert_eq!(join.status, "200 OK");
    assert!(!state.rooms.read().await.records[0]
        .members
        .iter()
        .any(|member| member == "applicant"));

    let pending = crate::route_http_request(
        "GET",
        "/api/v0/podcore/membership/join/pending/pod%3Aworkflow",
        None,
        "",
        &state,
    )
    .await
    .expect("pending joins");
    assert_eq!(pending.status, "200 OK");
    let pending_json = serde_json::from_str::<serde_json::Value>(&pending.body).unwrap();
    assert_eq!(
        pending_json["pendingJoinRequests"][0]["peerId"],
        "applicant"
    );

    let unauthorized = crate::route_http_request(
        "POST",
        "/api/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","acceptedRole":"moderator","acceptorPeerId":"ordinary-peer"}"#,
        &state,
    )
    .await
    .expect("unauthorized join acceptance");
    assert_eq!(unauthorized.status, "400 Bad Request");
    assert!(unauthorized.body.contains("does not have permission"));
    assert_eq!(
        state
            .pod_membership_workflow
            .read()
            .await
            .pending_joins("pod:workflow")
            .len(),
        1
    );

    let accepted = crate::route_http_request(
        "POST",
        "/api/podcore/membership/join/accept",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","acceptedRole":"moderator","acceptorPeerId":"owner-peer"}"#,
        &state,
    )
    .await
    .expect("join acceptance");
    assert_eq!(accepted.status, "200 OK");
    assert!(state.rooms.read().await.records[0]
        .members
        .iter()
        .any(|member| member == "applicant"));

    let leave = crate::route_http_request(
        "POST",
        "/api/podcore/membership/leave",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant"}"#,
        &state,
    )
    .await
    .expect("leave request");
    assert_eq!(leave.status, "200 OK");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&leave.body).unwrap()["pending"],
        true
    );
    let pending_leave = crate::route_http_request(
        "GET",
        "/api/podcore/membership/leave/pending/pod%3Aworkflow",
        None,
        "",
        &state,
    )
    .await
    .expect("pending leaves");
    assert!(pending_leave.body.contains("applicant"));

    let accepted_leave = crate::route_http_request(
        "POST",
        "/api/podcore/membership/leave/accept",
        None,
        r#"{"podId":"pod:workflow","peerId":"applicant","acceptorPeerId":"owner-peer"}"#,
        &state,
    )
    .await
    .expect("leave acceptance");
    assert_eq!(accepted_leave.status, "200 OK");
    assert!(!state.rooms.read().await.records[0]
        .members
        .iter()
        .any(|member| member == "applicant"));

    let second_join = crate::route_http_request(
        "POST",
        "/api/podcore/membership/join",
        None,
        r#"{"podId":"pod:workflow","peerId":"peer-two"}"#,
        &state,
    )
    .await
    .expect("second join request");
    assert_eq!(second_join.status, "200 OK");
    let cancelled = crate::route_http_request(
        "DELETE",
        "/api/v0/podcore/membership/join/pod%3Aworkflow/peer-two",
        None,
        "",
        &state,
    )
    .await
    .expect("cancel join");
    assert_eq!(cancelled.status, "200 OK");
    assert_eq!(cancelled.body, r#"{"cancelled":true}"#);
}

#[cfg_attr(test, test)]
#[cfg(feature = "full-controller-tests")]
pub(super) fn pod_join_signatures_match_frozen_unambiguous_canonical_payload() {
    use ed25519_dalek::{Signer as _, SigningKey};

    let now_millis = crate::unix_timestamp().saturating_mul(1_000);
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let mut input = crate::PodJoinSignatureInput {
        pod_id: "pod-alpha".to_owned(),
        peer_id: "peer".to_owned(),
        requested_role: "member".to_owned(),
        timestamp_unix_ms: now_millis,
        message: "hello world".to_owned(),
        nonce: "nonce-one".to_owned(),
        signature: String::new(),
        public_key: crate::STANDARD.encode(signing_key.verifying_key().to_bytes()),
    };
    let signature = signing_key.sign(input.canonical_payload().as_bytes());
    input.signature = format!("ed25519:{}", crate::STANDARD.encode(signature.to_bytes()));

    assert!(
        crate::verify_pod_join_signature(crate::PodSignatureMode::Enforce, &input, now_millis,)
            .unwrap()
    );

    assert_eq!(
        input.canonical_payload(),
        format!(
            "[1,\"join-request\",\"pod-alpha\",\"peer\",\"member\",{now_millis},\"hello world\",\"nonce-one\"]"
        )
    );
    input.message = "hello|world".to_owned();
    let delimiter_signature = signing_key.sign(input.canonical_payload().as_bytes());
    input.signature = format!(
        "ed25519:{}",
        crate::STANDARD.encode(delimiter_signature.to_bytes())
    );
    assert!(
        crate::verify_pod_join_signature(crate::PodSignatureMode::Enforce, &input, now_millis,)
            .unwrap()
    );

    input.message = "hello".to_owned();
    input.nonce = "world|nonce-one".to_owned();
    assert!(
        crate::verify_pod_join_signature(crate::PodSignatureMode::Enforce, &input, now_millis,)
            .is_err()
    );
}
