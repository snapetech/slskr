//! Controller full dispatch contracts ownership.

use super::*;

#[cfg_attr(test, tokio::test)]
#[cfg(feature = "full-controller-tests")]
pub(super) async fn dispatch_queued_downloads_after_login_sends_transfer_peer_commands() {
    let (state, mut receiver) = test_state();

    // Create a queued download
    {
        let mut queue = state.transfers.write().await;
        queue.create(
            0,
            Some("test-peer".to_owned()),
            "Remote/Test.flac".to_owned(),
            None,
            Some(1024),
        );
    }

    // Call dispatch function
    crate::session_runtime::dispatch_queued_downloads_after_login(&state).await;

    // Verify TransferPeer command was sent
    let command = receiver.recv().await.expect("should receive command");
    match command {
        crate::SessionCommand::TransferPeer { id, username } => {
            assert_eq!(id, 1);
            assert_eq!(username, "test-peer");
        }
        _ => panic!("Expected TransferPeer command, got {:?}", command),
    }
}
