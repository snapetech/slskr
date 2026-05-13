use std::collections::HashSet;

use slskr_protocol::peer::PeerMessage;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    capabilities::{
        peer_capability_message, PeerCapabilityDescriptor, PeerCapabilityEnvelope,
        PeerCapabilityMessageType, FEATURE_MESH_V1,
    },
    peer_cache::PeerConnectionCache,
    ClientError,
};

pub const MESH_RENDEZVOUS_INTEREST_TAG: &str = "slskdn-mesh-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRendezvousOptions {
    pub interest_tag: String,
    pub active_probe: bool,
}

impl Default for MeshRendezvousOptions {
    fn default() -> Self {
        Self {
            interest_tag: MESH_RENDEZVOUS_INTEREST_TAG.to_owned(),
            active_probe: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshRendezvous {
    options: MeshRendezvousOptions,
}

impl MeshRendezvous {
    #[must_use]
    pub fn new(options: MeshRendezvousOptions) -> Self {
        Self { options }
    }

    #[must_use]
    pub fn disabled() -> Self {
        Self::new(MeshRendezvousOptions::default())
    }

    #[must_use]
    pub fn interest_tag(&self) -> &str {
        &self.options.interest_tag
    }

    #[must_use]
    pub fn active_probe_enabled(&self) -> bool {
        self.options.active_probe
    }

    #[must_use]
    pub fn publish_interest_tags(&self) -> Vec<String> {
        vec![self.options.interest_tag.clone()]
    }

    #[must_use]
    pub fn candidate_usernames<'a>(
        &self,
        similar_users: impl IntoIterator<Item = &'a str>,
        known_capability_users: impl IntoIterator<Item = &'a str>,
    ) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for username in similar_users.into_iter().chain(known_capability_users) {
            let username = username.trim();
            if username.is_empty() {
                continue;
            }
            if seen.insert(username.to_ascii_lowercase()) {
                candidates.push(username.to_owned());
            }
        }
        candidates
    }

    pub async fn probe_peer<S>(
        &self,
        cache: &PeerConnectionCache<S>,
        username: &str,
        local_descriptor: &PeerCapabilityDescriptor,
        nonce: [u8; 16],
    ) -> Result<bool, ClientError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        if !self.options.active_probe {
            return Ok(false);
        }
        let envelope = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Hello,
            nonce,
            local_descriptor.clone(),
        );
        let message = peer_capability_message(&envelope).map_err(|error| {
            ClientError::CapabilityExchange(format!("failed to build capability probe: {error}"))
        })?;
        cache.send_to(username, &message).await
    }

    #[must_use]
    pub fn accepts_descriptor(descriptor: &PeerCapabilityDescriptor) -> bool {
        descriptor
            .features
            .iter()
            .any(|feature| feature == FEATURE_MESH_V1)
    }
}

#[must_use]
pub fn is_capability_probe(message: &PeerMessage) -> bool {
    matches!(
        message,
        PeerMessage::Unknown {
            code: crate::capabilities::PEER_CAPABILITY_MESSAGE_CODE,
            ..
        }
    )
}
