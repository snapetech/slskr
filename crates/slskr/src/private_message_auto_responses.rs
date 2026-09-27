use super::{truncate_utf8_bytes, MAX_MESSAGE_BODY_BYTES, MAX_MESSAGE_USERNAME_BYTES};
use std::collections::{BTreeMap, HashSet};

pub(crate) const MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS: usize = 4_096;

pub(crate) fn is_private_message_auto_response_candidate(message: &str) -> bool {
    if message.trim().is_empty() || message.len() > MAX_MESSAGE_BODY_BYTES {
        return false;
    }
    let normalized = message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    let direct_human_check = [
        "are you human",
        "are you a human",
        "are u human",
        "are u a human",
        "r u human",
        "are you real",
        "are u real",
        "are you a bot",
        "are u a bot",
        "you human?",
        "u human?",
        "bot?",
        "human?",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));
    if direct_human_check {
        return true;
    }

    let tokens = normalized
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<HashSet<_>>();
    let has_intent = [
        "prove", "verify", "confirm", "show", "tell", "say", "reply", "respond", "answer", "type",
        "write", "pass", "solve",
    ]
    .iter()
    .any(|token| tokens.contains(token));
    let has_identity = [
        "human",
        "person",
        "real",
        "bot",
        "robot",
        "automated",
        "automation",
        "script",
        "client",
        "captcha",
    ]
    .iter()
    .any(|token| tokens.contains(token));
    let named_check = [
        "check",
        "test",
        "verification",
        "gate",
        "screen",
        "challenge",
    ]
    .iter()
    .any(|token| tokens.contains(token))
        && has_identity;
    let share_gate = [
        "not sharing",
        "no sharing",
        "share something",
        "share more",
        "share files",
        "empty share",
        "empty shares",
        "no leechers",
        "anti-leech",
        "anti leech",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase));
    named_check || (has_intent && has_identity) || share_gate
}

#[derive(Debug, Default)]
pub(crate) struct PrivateMessageAutoResponseTracker {
    sent_at: BTreeMap<String, u64>,
}

impl PrivateMessageAutoResponseTracker {
    pub(crate) fn should_respond(
        &mut self,
        username: &str,
        now: u64,
        cooldown_seconds: u64,
    ) -> bool {
        self.sent_at
            .retain(|_, sent_at| now.saturating_sub(*sent_at) < cooldown_seconds);
        let key = truncate_utf8_bytes(
            username.trim().to_ascii_lowercase(),
            MAX_MESSAGE_USERNAME_BYTES,
        );
        if key.is_empty()
            || self
                .sent_at
                .get(&key)
                .is_some_and(|sent_at| now.saturating_sub(*sent_at) < cooldown_seconds)
        {
            return false;
        }
        if self.sent_at.len() >= MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS {
            if let Some(oldest) = self
                .sent_at
                .iter()
                .min_by_key(|(_, sent_at)| **sent_at)
                .map(|(username, _)| username.clone())
            {
                self.sent_at.remove(&oldest);
            }
        }
        self.sent_at.insert(key, now);
        true
    }

    pub(crate) fn release(&mut self, username: &str) {
        let key = truncate_utf8_bytes(
            username.trim().to_ascii_lowercase(),
            MAX_MESSAGE_USERNAME_BYTES,
        );
        self.sent_at.remove(&key);
    }

    #[cfg(feature = "full-controller-tests")]
    pub(crate) fn len(&self) -> usize {
        self.sent_at.len()
    }

    #[cfg(feature = "full-controller-tests")]
    pub(crate) fn is_empty(&self) -> bool {
        self.sent_at.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{PrivateMessageAutoResponseTracker, MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS};

    #[test]
    fn response_cooldown_is_case_insensitive_and_expires() {
        let mut tracker = PrivateMessageAutoResponseTracker::default();

        assert!(tracker.should_respond(" PeerOne ", 100, 30));
        assert!(!tracker.should_respond("peerone", 129, 30));
        assert!(tracker.should_respond("PEERONE", 130, 30));
    }

    #[test]
    fn response_release_and_peer_capacity_are_preserved() {
        let mut tracker = PrivateMessageAutoResponseTracker::default();
        assert!(tracker.should_respond("peer-1", 100, 30));
        tracker.release(" PEER-1 ");
        assert!(tracker.should_respond("peer-1", 101, 30));

        for index in 0..MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS {
            tracker.should_respond(&format!("peer-{index}"), 200, 30);
        }
        tracker.should_respond("new-peer", 200, 30);

        assert_eq!(
            tracker.sent_at.len(),
            MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_PEERS
        );
        assert!(tracker.sent_at.contains_key("new-peer"));
    }
}
