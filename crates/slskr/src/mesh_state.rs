use super::{
    capability_service_peer_json, persisted_capability_descriptor, unix_timestamp, MeshRendezvous,
    MeshRendezvousOptions, PeerCapabilityDescriptor, UserStore, MAX_MESH_SYNC_PEER_ID_BYTES,
    MAX_MESH_SYNC_SECURITY_PEERS, MAX_PEER_CAPABILITY_RECORDS, MESH_RENDEZVOUS_INTEREST_TAG,
    PEER_CAPABILITY_LEASE_SECONDS,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use std::collections::BTreeMap;
use std::time::SystemTime;

#[derive(Debug)]
pub(crate) struct MeshState {
    pub(crate) rendezvous: MeshRendezvous,
    pub(crate) enabled: bool,
    pub(crate) capability_handshake_enabled: bool,
    pub(crate) soulseek_rendezvous_enabled: bool,
    pub(crate) capability_records: Vec<PeerCapabilityDescriptor>,
    pub(crate) sync_invalid_entries: BTreeMap<String, (u64, u32)>,
    pub(crate) sync_invalid_messages: BTreeMap<String, (u64, u32)>,
    pub(crate) sync_rate_violations: BTreeMap<String, (u64, u32)>,
    pub(crate) sync_quarantined_until: BTreeMap<String, u64>,
    pub(crate) sync_rejected_messages: u64,
    pub(crate) sync_quarantine_events: u64,
    /// Real counters backing `/api/mesh/stats`'s `MeshSyncStats`
    /// contract, incremented at the one real merge call site
    /// (`POST /api/mesh/merge`) instead of being hardcoded to 0.
    pub(crate) sync_merge_total: u64,
    pub(crate) sync_merge_successful: u64,
    pub(crate) sync_merge_failed: u64,
    pub(crate) sync_entries_received: u64,
    pub(crate) sync_entries_sent: u64,
    pub(crate) sync_skipped_entries: u64,
    pub(crate) sync_rate_limit_violations: u64,
    pub(crate) sync_entries_merged: u64,
    pub(crate) updated_at: u64,
}

fn mesh_sync_peer_key(username: &str) -> Option<String> {
    let username = username.trim();
    if username.is_empty()
        || username.len() > MAX_MESH_SYNC_PEER_ID_BYTES
        || username.chars().any(char::is_control)
    {
        return None;
    }
    Some(username.to_ascii_lowercase())
}

impl MeshState {
    #[cfg(any(test, feature = "bounded-differential"))]
    pub(crate) fn new() -> Self {
        Self::with_settings(true, true, true, true)
    }

    pub(crate) fn from_settings(settings: &crate::config::MeshRuntimeSettings) -> Self {
        Self::with_settings(
            settings.enabled,
            settings.enable_soulseek_capability_handshake,
            settings.enable_soulseek_rendezvous,
            settings.probe_soulseek_rendezvous_capabilities,
        )
    }

    pub(crate) fn with_settings(
        enabled: bool,
        capability_handshake_enabled: bool,
        soulseek_rendezvous_enabled: bool,
        active_probe: bool,
    ) -> Self {
        Self {
            rendezvous: MeshRendezvous::new(MeshRendezvousOptions {
                interest_tag: MESH_RENDEZVOUS_INTEREST_TAG.to_owned(),
                active_probe: enabled && soulseek_rendezvous_enabled && active_probe,
            }),
            enabled,
            capability_handshake_enabled,
            soulseek_rendezvous_enabled,
            capability_records: Vec::new(),
            sync_invalid_entries: BTreeMap::new(),
            sync_invalid_messages: BTreeMap::new(),
            sync_rate_violations: BTreeMap::new(),
            sync_quarantined_until: BTreeMap::new(),
            sync_rejected_messages: 0,
            sync_quarantine_events: 0,
            sync_merge_total: 0,
            sync_merge_successful: 0,
            sync_merge_failed: 0,
            sync_entries_received: 0,
            sync_entries_sent: 0,
            sync_skipped_entries: 0,
            sync_rate_limit_violations: 0,
            sync_entries_merged: 0,
            updated_at: unix_timestamp(),
        }
    }

    pub(crate) fn sync_is_quarantined(&mut self, username: &str, now: u64) -> bool {
        let Some(key) = mesh_sync_peer_key(username) else {
            return false;
        };
        self.sync_quarantined_until
            .retain(|_, expires_at| *expires_at > now);
        self.sync_quarantined_until
            .get(&key)
            .is_some_and(|expires_at| *expires_at > now)
    }

    pub(crate) fn record_invalid_sync_entries(
        &mut self,
        username: &str,
        invalid: u32,
        settings: &crate::config::MeshSyncSecuritySettings,
        now: u64,
    ) -> bool {
        let Some(key) = mesh_sync_peer_key(username) else {
            return true;
        };
        let window = settings.rate_limit_window.as_secs().max(1);
        self.sync_invalid_entries
            .retain(|_, (started_at, _)| now.saturating_sub(*started_at) < window);
        self.sync_invalid_messages
            .retain(|_, (started_at, _)| now.saturating_sub(*started_at) < window);
        self.sync_rate_violations
            .retain(|_, (started_at, _)| now.saturating_sub(*started_at) < window);
        self.sync_quarantined_until
            .retain(|_, expires_at| *expires_at > now);
        if !self.sync_invalid_entries.contains_key(&key)
            && self.sync_invalid_entries.len() >= MAX_MESH_SYNC_SECURITY_PEERS
        {
            return true;
        }
        if !self.sync_invalid_messages.contains_key(&key)
            && self.sync_invalid_messages.len() >= MAX_MESH_SYNC_SECURITY_PEERS
        {
            return true;
        }
        let invalid_messages = self
            .sync_invalid_messages
            .entry(key.clone())
            .or_insert((now, 0));
        if now.saturating_sub(invalid_messages.0) >= window {
            *invalid_messages = (now, 0);
        }
        invalid_messages.1 = invalid_messages.1.saturating_add(1);
        let invalid_message_count = invalid_messages.1;
        let invalid_entries = self
            .sync_invalid_entries
            .entry(key.clone())
            .or_insert((now, 0));
        if now.saturating_sub(invalid_entries.0) >= window {
            *invalid_entries = (now, 0);
        }
        invalid_entries.1 = invalid_entries.1.saturating_add(invalid);
        let invalid_entry_count = invalid_entries.1;
        if invalid_entry_count <= settings.max_invalid_entries_per_window
            && invalid_message_count <= settings.max_invalid_messages_per_window
        {
            return false;
        }
        self.sync_rejected_messages = self.sync_rejected_messages.saturating_add(1);
        if !self.sync_rate_violations.contains_key(&key)
            && self.sync_rate_violations.len() >= MAX_MESH_SYNC_SECURITY_PEERS
        {
            return true;
        }
        let violations = self
            .sync_rate_violations
            .entry(key.clone())
            .or_insert((now, 0));
        if now.saturating_sub(violations.0) >= window {
            *violations = (now, 0);
        }
        violations.1 = violations.1.saturating_add(1);
        if violations.1 >= settings.quarantine_violation_threshold {
            if self.sync_quarantined_until.contains_key(&key)
                || self.sync_quarantined_until.len() < MAX_MESH_SYNC_SECURITY_PEERS
            {
                self.sync_quarantined_until.insert(
                    key,
                    now.saturating_add(settings.quarantine_duration.as_secs()),
                );
            }
            self.sync_quarantine_events = self.sync_quarantine_events.saturating_add(1);
            violations.1 = 0;
        }
        true
    }

    pub(crate) fn mesh_capability_usernames(&self) -> Vec<&str> {
        self.capability_records
            .iter()
            .filter(|descriptor| MeshRendezvous::accepts_descriptor(descriptor))
            .map(|descriptor| descriptor.username.as_str())
            .collect()
    }

    pub(crate) fn update_capability(
        &mut self,
        mut descriptor: PeerCapabilityDescriptor,
    ) -> Result<(), String> {
        descriptor
            .verify(std::time::SystemTime::now())
            .map_err(|error| format!("peer capability descriptor rejected: {error}"))?;
        let now = unix_timestamp();
        self.capability_records
            .retain(|record| record.expires_at_unix > now);
        if self.capability_records.iter().any(|record| {
            record.peer_id.eq_ignore_ascii_case(&descriptor.peer_id)
                && !record.username.eq_ignore_ascii_case(&descriptor.username)
        }) {
            return Err(
                "peer capability descriptor peer ID is already registered to another username"
                    .to_owned(),
            );
        }
        descriptor.issued_at_unix = now;
        descriptor.expires_at_unix = descriptor
            .expires_at_unix
            .min(now.saturating_add(PEER_CAPABILITY_LEASE_SECONDS));
        if let Some(existing) = self
            .capability_records
            .iter_mut()
            .find(|record| record.username.eq_ignore_ascii_case(&descriptor.username))
        {
            *existing = descriptor;
        } else {
            if self.capability_records.len() >= MAX_PEER_CAPABILITY_RECORDS {
                return Err(format!(
                    "peer capability registry is full (maximum {MAX_PEER_CAPABILITY_RECORDS} records)"
                ));
            }
            self.capability_records.push(descriptor);
        }
        self.updated_at = now;
        Ok(())
    }

    pub(crate) fn restore_capability(&mut self, descriptor: PeerCapabilityDescriptor) -> bool {
        if descriptor.verify(SystemTime::now()).is_err()
            || self.capability_records.len() >= MAX_PEER_CAPABILITY_RECORDS
            || self.capability_records.iter().any(|record| {
                record.peer_id.eq_ignore_ascii_case(&descriptor.peer_id)
                    || record.username.eq_ignore_ascii_case(&descriptor.username)
            })
        {
            return false;
        }
        self.capability_records.push(descriptor);
        true
    }

    pub(crate) fn restore_persisted_capabilities(&mut self, records: &[serde_json::Value]) {
        for record in records.iter().take(MAX_PEER_CAPABILITY_RECORDS) {
            if let Some(descriptor) = persisted_capability_descriptor(record) {
                let _ = self.restore_capability(descriptor);
            }
        }
    }

    pub(crate) fn persisted_capability_projection(&self) -> Vec<serde_json::Value> {
        self.capability_records
            .iter()
            .map(|descriptor| {
                serde_json::json!({
                    "peerId": descriptor.peer_id,
                    "username": descriptor.username,
                    "features": descriptor.features,
                    "endpoints": descriptor.endpoints,
                    "overlayPort": descriptor.overlay_port,
                    "maxPayloadLength": descriptor.max_payload_length,
                    "issuedAtUnix": descriptor.issued_at_unix,
                    "expiresAtUnix": descriptor.expires_at_unix,
                    "publicKey": STANDARD.encode(descriptor.public_key),
                    "signature": descriptor.signature.map(|value| STANDARD.encode(value)),
                })
            })
            .collect()
    }

    pub(crate) fn candidate_usernames(&self, users: &UserStore) -> Vec<String> {
        self.rendezvous.candidate_usernames(
            users.records.iter().map(|user| user.username.as_str()),
            self.mesh_capability_usernames(),
        )
    }

    pub(crate) fn status_json(&self, users: &UserStore) -> String {
        let candidates = self.candidate_usernames(users);
        serde_json::json!({
            "enabled": self.enabled,
            "capabilityHandshakeEnabled": self.capability_handshake_enabled,
            "soulseekRendezvousEnabled": self.soulseek_rendezvous_enabled,
            "activeProbe": self.rendezvous.active_probe_enabled(),
            "interestTag": self.rendezvous.interest_tag(),
            "publishedInterestTags": self.rendezvous.publish_interest_tags(),
            "candidateCount": candidates.len(),
            "capabilityRecords": self.capability_records.len(),
            "meshCapableRecords": self.mesh_capability_usernames().len(),
            "privacy": "Soulseek mesh rendezvous is passive unless active probing is explicitly enabled.",
            "updated_at": self.updated_at,
        })
        .to_string()
    }

    pub(crate) fn users_json(&self, users: &UserStore) -> String {
        let candidates = self
            .candidate_usernames(users)
            .into_iter()
            .map(|username| {
                let has_capability = self.capability_records.iter().any(|descriptor| {
                    descriptor.username.eq_ignore_ascii_case(&username)
                        && MeshRendezvous::accepts_descriptor(descriptor)
                });
                serde_json::json!({
                    "username": username,
                    "source": if has_capability { "capability" } else { "similar-user" },
                    "meshCapable": has_capability,
                })
            })
            .collect::<Vec<_>>();
        let count = candidates.len();
        serde_json::json!({
            "users": candidates,
            "count": count,
            "interestTag": self.rendezvous.interest_tag(),
            "updated_at": self.updated_at,
        })
        .to_string()
    }

    pub(crate) fn versioned_similar_users_json(&self, users: &UserStore) -> String {
        let candidates = self
            .candidate_usernames(users)
            .into_iter()
            .map(|username| {
                serde_json::json!({
                    "username": username,
                    "rating": 0,
                })
            })
            .collect::<Vec<_>>();
        serde_json::Value::Array(candidates).to_string()
    }

    pub(crate) fn capability_records_json(&self) -> Vec<serde_json::Value> {
        self.capability_records
            .iter()
            .map(|descriptor| {
                serde_json::json!({
                    "peerId": descriptor.peer_id,
                    "username": descriptor.username,
                    "features": descriptor.features,
                    "endpoints": descriptor.endpoints,
                    "overlayPort": descriptor.overlay_port,
                    "maxPayloadLength": descriptor.max_payload_length,
                    "issuedAt": descriptor.issued_at_unix,
                    "expiresAt": descriptor.expires_at_unix,
                    "meshCapable": MeshRendezvous::accepts_descriptor(descriptor),
                })
            })
            .collect()
    }

    pub(crate) fn capability_service_peers_json(&self) -> Vec<serde_json::Value> {
        let mut records = self.capability_records.iter().collect::<Vec<_>>();
        records.sort_by_key(|record| std::cmp::Reverse(record.issued_at_unix));
        records
            .into_iter()
            .map(capability_service_peer_json)
            .collect()
    }

    pub(crate) fn capability_service_mesh_peers_json(&self) -> Vec<serde_json::Value> {
        self.capability_service_peers_json()
            .into_iter()
            .filter(|record| record["canMeshSync"] == true)
            .map(|record| {
                serde_json::json!({
                    "username": record["username"],
                    "lastSeen": record["lastSeen"],
                    "meshSeqId": record["meshSeqId"],
                })
            })
            .collect()
    }

    pub(crate) fn discover_json(&self, users: &UserStore) -> String {
        let user_json = self
            .candidate_usernames(users)
            .into_iter()
            .map(|username| serde_json::json!({ "username": username }))
            .collect::<Vec<_>>();
        let capability_records = self.capability_records_json();
        let user_count = user_json.len();
        let capability_record_count = capability_records.len();
        serde_json::json!({
            "users": user_json,
            "count": user_count,
            "capabilityRecords": capability_records,
            "capabilityRecordCount": capability_record_count,
            "interestTag": self.rendezvous.interest_tag(),
            "activeProbe": self.rendezvous.active_probe_enabled(),
            "updated_at": self.updated_at,
        })
        .to_string()
    }
}
