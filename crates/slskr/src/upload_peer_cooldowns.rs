use super::{unix_timestamp, AppState, TransferEntry};
use std::collections::BTreeMap;

const FAILED_UPLOAD_PEER_COOLDOWN_SECONDS: u64 = 30;
const MAX_FAILED_UPLOAD_PEER_COOLDOWNS: usize = 4_096;
const MAX_FAILED_UPLOAD_PEER_USERNAME_BYTES: usize = 1_024;

#[derive(Debug, Default)]
pub(crate) struct UploadPeerCooldowns {
    retry_after: BTreeMap<String, u64>,
}

impl UploadPeerCooldowns {
    pub(crate) fn record_failure(&mut self, username: &str, now: u64) {
        let Some(key) = upload_peer_cooldown_key(username) else {
            return;
        };
        self.retry_after.retain(|_, expires_at| *expires_at > now);
        if !self.retry_after.contains_key(&key)
            && self.retry_after.len() >= MAX_FAILED_UPLOAD_PEER_COOLDOWNS
        {
            if let Some(oldest) = self
                .retry_after
                .iter()
                .min_by_key(|(_, expires_at)| *expires_at)
                .map(|(username, _)| username.clone())
            {
                self.retry_after.remove(&oldest);
            }
        }
        self.retry_after
            .insert(key, now.saturating_add(FAILED_UPLOAD_PEER_COOLDOWN_SECONDS));
    }

    pub(crate) fn remaining(&mut self, username: &str, now: u64) -> Option<u64> {
        let key = upload_peer_cooldown_key(username)?;
        let retry_after = self.retry_after.get(&key).copied()?;
        if retry_after > now {
            return Some(retry_after.saturating_sub(now));
        }
        self.retry_after.remove(&key);
        None
    }
}

fn upload_peer_cooldown_key(username: &str) -> Option<String> {
    let username = username.trim();
    if username.is_empty()
        || username.len() > MAX_FAILED_UPLOAD_PEER_USERNAME_BYTES
        || username.chars().any(char::is_control)
    {
        return None;
    }
    Some(username.to_ascii_lowercase())
}

pub(crate) fn is_expected_remote_upload_failure(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    if error.contains("local") || error.contains("shared file") {
        return false;
    }
    [
        "timed out",
        "timeout",
        "connection",
        "broken pipe",
        "unexpected end",
        "transport",
        "peer",
        "file-transfer connect",
    ]
    .iter()
    .any(|marker| error.contains(marker))
}

pub(super) async fn record_expected_upload_failure(
    state: &AppState,
    transfer: &TransferEntry,
    error: &str,
) {
    if transfer.direction != 1 || !is_expected_remote_upload_failure(error) {
        return;
    }
    let Some(username) = transfer.peer_username.as_deref() else {
        return;
    };
    state
        .failed_upload_peer_cooldowns
        .write()
        .await
        .record_failure(username, unix_timestamp());
}

#[cfg(test)]
mod tests {
    use super::{
        is_expected_remote_upload_failure, UploadPeerCooldowns, MAX_FAILED_UPLOAD_PEER_COOLDOWNS,
        MAX_FAILED_UPLOAD_PEER_USERNAME_BYTES,
    };

    #[test]
    fn cooldown_is_case_insensitive_and_expires() {
        let mut cooldowns = UploadPeerCooldowns::default();
        cooldowns.record_failure(" PeerOne ", 100);

        assert_eq!(cooldowns.remaining("peerone", 100), Some(30));
        assert_eq!(cooldowns.remaining("PEERONE", 129), Some(1));
        assert_eq!(cooldowns.remaining("peerone", 130), None);
        assert_eq!(cooldowns.remaining("peerone", 131), None);
    }

    #[test]
    fn only_expected_remote_failures_start_a_cooldown() {
        assert!(is_expected_remote_upload_failure(
            "file-transfer connect failed: connection refused"
        ));
        assert!(is_expected_remote_upload_failure(
            "file upload chunk send timed out"
        ));
        assert!(!is_expected_remote_upload_failure(
            "local file read failed: permission denied"
        ));
        assert!(!is_expected_remote_upload_failure(
            "upload filename is not available from local shares"
        ));
    }

    #[test]
    fn cooldown_state_bounds_peer_keys_and_reclaims_capacity() {
        let mut cooldowns = UploadPeerCooldowns::default();
        cooldowns.record_failure(&"x".repeat(MAX_FAILED_UPLOAD_PEER_USERNAME_BYTES + 1), 100);
        assert!(cooldowns.retry_after.is_empty());

        for index in 0..MAX_FAILED_UPLOAD_PEER_COOLDOWNS {
            cooldowns.record_failure(&format!("peer-{index}"), 100);
        }
        cooldowns.record_failure("new-peer", 100);
        assert_eq!(
            cooldowns.retry_after.len(),
            MAX_FAILED_UPLOAD_PEER_COOLDOWNS
        );
        assert!(cooldowns.retry_after.contains_key("new-peer"));

        cooldowns.record_failure("after-expiry", 131);
        assert_eq!(cooldowns.retry_after.len(), 1);
        assert!(cooldowns.retry_after.contains_key("after-expiry"));
    }
}
