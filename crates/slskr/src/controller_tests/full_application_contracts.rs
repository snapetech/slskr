//! Controller full application contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn build_info_uses_app_version_not_protocol_version() {
    let (state, _receiver) = test_state();

    let response = crate::route_http_request("GET", "/api/application/build", None, "", &state)
        .await
        .unwrap();
    assert_eq!(response.status, "200 OK");
    let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();

    assert_eq!(body["current"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        body["full"],
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_VERSION")
        )
    );
    assert_eq!(body["latestTag"], "");
    assert_eq!(
        body["protocol"]["major"],
        serde_json::json!(crate::CLIENT_MAJOR_VERSION)
    );
    assert_eq!(
        body["protocol"]["minor"],
        serde_json::json!(crate::CLIENT_MINOR_VERSION)
    );
    assert_ne!(
        body["current"],
        serde_json::json!(format!(
            "{}.{}.{}",
            crate::CLIENT_NAME,
            crate::CLIENT_MAJOR_VERSION,
            crate::CLIENT_MINOR_VERSION
        ))
    );
}

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn embedded_distributed_search_is_unwrapped_and_broadcast_to_children() {
    let (state, _session_commands) = test_state();
    let (child_sender, mut child_messages) = mpsc::channel(4);
    {
        let mut runtime = state.distributed_network.write().await;
        runtime.parent = Some("parent".to_owned());
        runtime.children.insert("child".to_owned(), child_sender);
    }

    let search = crate::DistributedSearch {
        identifier: 0,
        username: "requester".to_owned(),
        token: 77,
        query: "embedded-search-that-is-not-shared".to_owned(),
    };
    let search_frame = crate::DistributedMessage::Search(search.clone())
        .encode()
        .expect("encode distributed search");

    crate::handle_embedded_distributed_search(
        &state,
        Some(crate::DistributedConnectionRole::Parent),
        Some("parent"),
        search_frame.code,
        &search_frame.payload,
    )
    .await;

    assert_eq!(
        child_messages.recv().await,
        Some(crate::DistributedMessage::Search(search))
    );
}
