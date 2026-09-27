use super::{
    share_grant_allows_download, share_grant_allows_reshare, share_grant_allows_stream,
    MAX_INCOMING_SHARES,
};

/// A share announced by another node's owner. The real Soulseek/mesh
/// transport is expected to deliver these; `/api/v0/share-grants/announce`
/// is an HTTP stand-in for that push, gated behind
/// `SLSKDN_E2E_SHARE_ANNOUNCE=1` since it trusts the payload's claims about
/// who owns what rather than verifying them the way a peer-authenticated
/// transport would.
#[derive(Clone, Debug)]
pub(crate) struct IncomingShareRecord {
    pub(crate) id: String,
    pub(crate) owner_endpoint: String,
    pub(crate) owner_user_id: String,
    pub(crate) recipient_user_id: String,
    pub(crate) collection_id: String,
    pub(crate) collection_title: String,
    pub(crate) collection_description: String,
    pub(crate) collection_type: String,
    pub(crate) permissions: String,
    pub(crate) token: String,
    pub(crate) expiry_utc: String,
    pub(crate) max_bitrate_kbps: Option<u64>,
    pub(crate) max_concurrent_streams: u64,
    pub(crate) items: Vec<serde_json::Value>,
    pub(crate) received_at: u64,
}

impl IncomingShareRecord {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "shareGrantId": self.id,
            "ownerEndpoint": self.owner_endpoint,
            "ownerUserId": self.owner_user_id,
            "recipientUserId": self.recipient_user_id,
            "collectionId": self.collection_id,
            "collectionTitle": self.collection_title,
            "collectionDescription": self.collection_description,
            "collectionType": self.collection_type,
            "permissions": self.permissions,
            "allowDownload": share_grant_allows_download(&self.permissions),
            "allowStream": share_grant_allows_stream(&self.permissions),
            "allowReshare": share_grant_allows_reshare(&self.permissions),
            "token": self.token,
            "expiryUtc": self.expiry_utc,
            "maxBitrateKbps": self.max_bitrate_kbps,
            "maxConcurrentStreams": self.max_concurrent_streams,
            "items": self.items,
            "receivedAt": self.received_at,
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct IncomingShareStore {
    records: Vec<IncomingShareRecord>,
}

impl IncomingShareStore {
    /// Keyed by `shareGrantId` so a retried or repeated announce updates
    /// the existing entry instead of duplicating it.
    pub(crate) fn upsert(&mut self, record: IncomingShareRecord) {
        if let Some(existing) = self
            .records
            .iter_mut()
            .find(|candidate| candidate.id == record.id)
        {
            *existing = record;
            return;
        }
        if self.records.len() >= MAX_INCOMING_SHARES {
            self.records.remove(0);
        }
        self.records.push(record);
    }

    pub(crate) fn list(&self) -> &[IncomingShareRecord] {
        &self.records
    }
}
