use super::*;

#[derive(Clone, Debug)]
pub(super) struct PreviewStreamTicket {
    pub(super) family: String,
    pub(super) source: String,
    pub(super) content_id: String,
    pub(super) filename: String,
    pub(super) peer_username: Option<String>,
    pub(super) size: u64,
    pub(super) content_type: String,
    pub(super) source_url: Option<String>,
    pub(super) source_authorization: Option<String>,
    pub(super) overlay_peer_identity: Option<String>,
    pub(super) expected_hash: Option<String>,
    pub(super) created_at: u64,
    pub(super) expires_at: u64,
}

#[derive(Debug)]
pub(super) struct PreviewStreamTicketStore {
    pub(super) records: BTreeMap<String, PreviewStreamTicket>,
    max_records: usize,
}

impl Default for PreviewStreamTicketStore {
    fn default() -> Self {
        Self::with_max_records(MAX_PREVIEW_STREAM_TICKETS)
    }
}

impl PreviewStreamTicketStore {
    pub(super) fn with_max_records(max_records: usize) -> Self {
        Self {
            records: BTreeMap::new(),
            max_records: max_records.max(1),
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "ticket fields are explicit at the single construction boundary"
    )]
    pub(super) fn issue(
        &mut self,
        family: &str,
        source: &str,
        content_id: String,
        filename: String,
        peer_username: Option<String>,
        size: u64,
        content_type: String,
        ttl_seconds: u64,
    ) -> Option<(String, PreviewStreamTicket)> {
        let now = unix_timestamp();
        self.prune(now);
        if self.records.len() >= self.max_records {
            return None;
        }
        let token = secure_oauth_state()?;
        let record = PreviewStreamTicket {
            family: family.to_owned(),
            source: source.to_owned(),
            content_id,
            filename,
            peer_username,
            size,
            content_type,
            source_url: None,
            source_authorization: None,
            overlay_peer_identity: None,
            expected_hash: None,
            created_at: now,
            expires_at: now.saturating_add(ttl_seconds),
        };
        self.records.insert(token.clone(), record.clone());
        Some((token, record))
    }

    pub(super) fn get(&mut self, token: &str) -> Option<PreviewStreamTicket> {
        let now = unix_timestamp();
        self.prune(now);
        self.records.get(token).cloned()
    }

    pub(super) fn find_active_token(
        &mut self,
        family: &str,
        source: &str,
        content_id: &str,
    ) -> Option<String> {
        let now = unix_timestamp();
        self.prune(now);
        self.records
            .iter()
            .find(|(_, record)| {
                record.family == family
                    && record.source == source
                    && record.content_id == content_id
            })
            .map(|(token, _)| token.clone())
    }

    pub(super) fn configure_remote_mesh(
        &mut self,
        token: &str,
        source_url: String,
        source_authorization: Option<String>,
        expected_hash: String,
    ) -> bool {
        let Some(record) = self.records.get_mut(token) else {
            return false;
        };
        record.source_url = Some(source_url);
        record.source_authorization = source_authorization;
        record.expected_hash = Some(expected_hash);
        true
    }

    pub(super) fn configure_remote_overlay(
        &mut self,
        token: &str,
        peer_identity: String,
        expected_hash: String,
    ) -> bool {
        let Some(record) = self.records.get_mut(token) else {
            return false;
        };
        record.overlay_peer_identity = Some(peer_identity);
        record.expected_hash = Some(expected_hash);
        true
    }

    pub(super) fn revoke_source(&mut self, source: &str) {
        self.records.retain(|_, record| record.source != source);
    }

    fn prune(&mut self, now: u64) {
        self.records.retain(|_, record| record.expires_at > now);
    }
}

pub(super) async fn open_preview_stream_ticket(
    state: &AppState,
    family: &str,
    token: &str,
) -> Option<String> {
    let mut tickets = state.stream_tickets.write().await;
    let ticket = tickets.get(token);
    drop(tickets);
    let ticket = ticket?;
    if ticket.family != family {
        return None;
    }
    Some(
        serde_json::json!({
            "ticket": token,
            "status": "available",
            "source": ticket.source,
            "content_id": ticket.content_id,
            "filename": ticket.filename,
            "peer_username": ticket.peer_username,
            "size": ticket.size,
            "contentType": ticket.content_type,
            "created_at": ticket.created_at,
            "expires_at": ticket.expires_at,
            "cacheControl": "no-store",
            "acceptRanges": "none",
        })
        .to_string(),
    )
}
