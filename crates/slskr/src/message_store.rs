use super::{
    json_escape, json_usize_option, routing, truncate_utf8_bytes, unix_timestamp,
    unix_timestamp_millis, AppState, ControllerProfile, HttpResponse, RecordListFilter,
};
use std::collections::BTreeMap;

pub(super) const MAX_MESSAGE_RECORDS: usize = 500;
pub(super) const MAX_MESSAGE_USERNAME_BYTES: usize = 1024;
pub(super) const MAX_MESSAGE_BODY_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MessageRecord {
    pub(crate) id: u64,
    pub(crate) username: String,
    pub(crate) direction: &'static str,
    pub(crate) body: String,
    pub(crate) acknowledged: bool,
    pub(crate) created_at: u64,
    pub(crate) created_at_ms: u64,
    pub(crate) updated_at: u64,
    pub(crate) source_id: Option<u32>,
    pub(crate) source_timestamp: Option<u32>,
    pub(crate) was_replayed: bool,
}

impl MessageRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"username\":\"{}\",\"direction\":\"{}\",\"body\":\"{}\",\"acknowledged\":{},\"created_at\":{},\"updated_at\":{}}}",
            self.id,
            json_escape(&self.username),
            self.direction,
            json_escape(&self.body),
            self.acknowledged,
            self.created_at,
            self.updated_at
        )
    }

    pub(crate) fn controller_json(&self) -> serde_json::Value {
        serde_json::json!({
            "timestamp": self.created_at.to_string(),
            "createdAtMs": self.created_at_ms,
            "id": self.id,
            "username": self.username,
            "direction": if self.direction == "inbound" { "In" } else { "Out" },
            "message": self.body,
            "isAcknowledged": self.acknowledged,
            "wasReplayed": self.was_replayed,
            "sourceId": self.source_id,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MessageStore {
    pub(crate) records: Vec<MessageRecord>,
    pub(crate) next_id: u64,
    updated_at: u64,
    max_records: usize,
}

impl MessageStore {
    pub(crate) fn new() -> Self {
        Self::with_max_records(MAX_MESSAGE_RECORDS)
    }

    pub(crate) fn with_max_records(max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::MessageRecord>) -> Self {
        let mut store = Self::new();
        store.records = records
            .into_iter()
            .filter_map(|record| {
                let id = record.id.parse::<u64>().ok()?;
                Some(MessageRecord {
                    id,
                    username: truncate_utf8_bytes(record.username, MAX_MESSAGE_USERNAME_BYTES),
                    direction: match record.direction.as_str() {
                        "inbound" | "incoming" | "In" => "inbound",
                        _ => "outbound",
                    },
                    body: truncate_utf8_bytes(record.content, MAX_MESSAGE_BODY_BYTES),
                    acknowledged: record.read,
                    created_at: u64::try_from(record.created_at).unwrap_or_default(),
                    created_at_ms: u64::try_from(record.created_at)
                        .unwrap_or_default()
                        .saturating_mul(1_000),
                    updated_at: u64::try_from(record.created_at).unwrap_or_default(),
                    source_id: record.source_id.and_then(|value| u32::try_from(value).ok()),
                    source_timestamp: record
                        .source_timestamp
                        .and_then(|value| u32::try_from(value).ok()),
                    was_replayed: record.was_replayed,
                })
            })
            .collect();
        store.records.sort_by_key(|record| record.id);
        store.records.dedup_by_key(|record| record.id);
        let mut previous_created_at_ms = 0_u64;
        for record in &mut store.records {
            record.created_at_ms = record
                .created_at_ms
                .max(previous_created_at_ms.saturating_add(1));
            previous_created_at_ms = record.created_at_ms;
        }
        if store.records.len() > store.max_records {
            let excess = store.records.len() - store.max_records;
            store.records.drain(0..excess);
        }
        store.next_id = store
            .records
            .iter()
            .map(|record| record.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        store.updated_at = store
            .records
            .iter()
            .map(|record| record.updated_at)
            .max()
            .unwrap_or_else(unix_timestamp);
        store
    }

    pub(crate) fn add(
        &mut self,
        username: String,
        direction: &'static str,
        body: String,
    ) -> MessageRecord {
        let now = unix_timestamp();
        let now_ms = unix_timestamp_millis().max(
            self.records
                .iter()
                .map(|record| record.created_at_ms)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        let id = self.allocate_id();
        let record = MessageRecord {
            id,
            username: truncate_utf8_bytes(username, MAX_MESSAGE_USERNAME_BYTES),
            direction,
            body: truncate_utf8_bytes(body, MAX_MESSAGE_BODY_BYTES),
            acknowledged: false,
            created_at: now,
            created_at_ms: now_ms,
            updated_at: now,
            source_id: None,
            source_timestamp: None,
            was_replayed: false,
        };
        self.records.push(record.clone());
        if self.records.len() > self.max_records {
            let excess = self.records.len() - self.max_records;
            self.records.drain(0..excess);
        }
        self.updated_at = now;
        record
    }

    pub(crate) fn remove_older_than(&mut self, cutoff: i64) -> usize {
        let before = self.records.len();
        self.records
            .retain(|record| i64::try_from(record.created_at).unwrap_or(i64::MAX) >= cutoff);
        let removed = before.saturating_sub(self.records.len());
        if removed > 0 {
            self.updated_at = unix_timestamp();
        }
        removed
    }

    fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            if !self.records.iter().any(|record| record.id == candidate) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded message history must leave an available u64 id")
    }

    pub(crate) fn ack(&mut self, id: u64) -> Option<MessageRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.id == id)?;
        record.acknowledged = true;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn json(&self, query: Option<&str>) -> String {
        self.json_filtered(None, query)
    }

    pub(crate) fn json_for_user(&self, username: &str, query: Option<&str>) -> String {
        self.json_filtered(Some(username), query)
    }

    fn json_filtered(&self, username: Option<&str>, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| username.is_none_or(|username| record.username == username))
            .filter(|record| {
                filter
                    .username
                    .as_deref()
                    .is_none_or(|username| record.username == username)
            })
            .filter(|record| {
                filter
                    .direction
                    .as_deref()
                    .is_none_or(|direction| record.direction == direction)
            })
            .filter(|record| {
                filter.q.as_deref().is_none_or(|q| {
                    record.username.to_ascii_lowercase().contains(q)
                        || record.body.to_ascii_lowercase().contains(q)
                })
            })
            .collect::<Vec<_>>();
        let filtered_count = records.len();
        let records = records
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(MessageRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    pub(crate) fn summary_json(&self) -> String {
        let inbound = self
            .records
            .iter()
            .filter(|record| record.direction == "inbound")
            .count();
        let outbound = self
            .records
            .iter()
            .filter(|record| record.direction == "outbound")
            .count();
        let acknowledged = self
            .records
            .iter()
            .filter(|record| record.acknowledged)
            .count();
        format!(
            "{{\"total\":{},\"inbound\":{},\"outbound\":{},\"acknowledged\":{},\"unacknowledged\":{},\"updated_at\":{}}}",
            self.records.len(),
            inbound,
            outbound,
            acknowledged,
            self.records.len().saturating_sub(acknowledged),
            self.updated_at
        )
    }

    pub(crate) fn controller_conversations_json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let mut grouped: BTreeMap<String, Vec<&MessageRecord>> = BTreeMap::new();
        for record in &self.records {
            if filter.q.as_deref().is_some_and(|q| {
                !record.username.to_ascii_lowercase().contains(q)
                    && !record.body.to_ascii_lowercase().contains(q)
            }) {
                continue;
            }
            grouped
                .entry(record.username.clone())
                .or_default()
                .push(record);
        }
        let conversations = grouped
            .into_iter()
            .map(|(username, messages)| controller_conversation_json(username, messages, true))
            .collect::<Vec<_>>();
        serde_json::Value::Array(conversations).to_string()
    }

    pub(crate) fn controller_conversation_json(
        &self,
        username: &str,
        include_messages: bool,
        since: Option<u64>,
    ) -> String {
        let messages = self
            .records
            .iter()
            .filter(|record| record.username == username)
            .collect::<Vec<_>>();
        let mut conversation =
            controller_conversation_json(username.to_owned(), messages.clone(), false);
        if include_messages {
            conversation["messages"] = serde_json::Value::Array(
                messages
                    .into_iter()
                    .filter(|record| since.is_none_or(|since| record.created_at_ms > since))
                    .map(MessageRecord::controller_json)
                    .collect(),
            );
        }
        conversation.to_string()
    }

    pub(crate) fn controller_messages_json(
        &self,
        username: &str,
        unacknowledged_only: bool,
    ) -> String {
        let messages = self
            .records
            .iter()
            .filter(|record| record.username == username)
            .filter(|record| !unacknowledged_only || !record.acknowledged)
            .map(MessageRecord::controller_json)
            .collect::<Vec<_>>();
        serde_json::Value::Array(messages).to_string()
    }

    pub(crate) fn ack_all_for_user(&mut self, username: &str) -> usize {
        let now = unix_timestamp();
        let mut updated = 0;
        for record in self
            .records
            .iter_mut()
            .filter(|record| record.username == username && !record.acknowledged)
        {
            record.acknowledged = true;
            record.updated_at = now;
            updated += 1;
        }
        if updated > 0 {
            self.updated_at = now;
        }
        updated
    }

    pub(crate) fn has_unacknowledged_messages(&self) -> bool {
        self.records.iter().any(|record| !record.acknowledged)
    }
}

fn controller_conversation_json(
    username: String,
    messages: Vec<&MessageRecord>,
    include_messages: bool,
) -> serde_json::Value {
    let unacknowledged = messages
        .iter()
        .filter(|message| !message.acknowledged)
        .count();
    let messages_json = include_messages.then(|| {
        messages
            .into_iter()
            .map(MessageRecord::controller_json)
            .collect::<Vec<_>>()
    });
    let mut value = serde_json::json!({
        "username": username,
        "isActive": true,
        "unAcknowledgedMessageCount": unacknowledged,
        "hasUnAcknowledgedMessages": unacknowledged > 0,
    });
    if let Some(messages) = messages_json {
        value["messages"] = serde_json::Value::Array(messages);
    }
    value
}

pub(super) fn persisted_message_record(
    record: &MessageRecord,
) -> crate::persistence::MessageRecord {
    crate::persistence::MessageRecord {
        id: record.id.to_string(),
        username: record.username.clone(),
        content: record.body.clone(),
        direction: record.direction.to_owned(),
        read: record.acknowledged,
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        source_id: record.source_id.map(i64::from),
        source_timestamp: record.source_timestamp.map(i64::from),
        was_replayed: record.was_replayed,
    }
}

pub(super) async fn controller_conversation_read_failure_response(
    state: &AppState,
    path: &str,
) -> Option<HttpResponse> {
    if !matches!(
        state.config.controller_profile,
        ControllerProfile::Legacy | ControllerProfile::Native
    ) || !path.starts_with("/api/v0/")
    {
        return None;
    }
    let db = state.db.as_ref()?;
    if db.list_messages(1, 0).await.is_err() {
        return Some(routing::internal_server_error_response(
            "conversation storage unavailable",
        ));
    }
    None
}

pub(super) async fn persist_message_record_checked(
    state: &AppState,
    record: &MessageRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.insert_message(&persisted_message_record(record))
        .await
        .map_err(|error| {
            let message = format!("message persistence failed: {error}");
            eprintln!("[Error] {message}");
            message
        })?;
    Ok(true)
}

pub(super) async fn persist_message_records_checked(
    state: &AppState,
    records: &[MessageRecord],
) -> Result<bool, String> {
    if records.is_empty() {
        return Ok(state.db.is_some());
    }
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = records
        .iter()
        .map(persisted_message_record)
        .collect::<Vec<_>>();
    db.insert_messages(&persisted)
        .await
        .map_err(|error| format!("message persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_message_ack_checked(state: &AppState, id: u64) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.mark_message_read(&id.to_string())
        .await
        .map_err(|error| format!("message acknowledgement persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_message_acks_checked(
    state: &AppState,
    ids: &[u64],
) -> Result<bool, String> {
    if ids.is_empty() {
        return Ok(state.db.is_some());
    }
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let ids = ids.iter().map(u64::to_string).collect::<Vec<_>>();
    db.mark_messages_read(&ids)
        .await
        .map_err(|error| format!("message acknowledgement persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_messages_if_unchanged(
    state: &AppState,
    previous: MessageStore,
    mutated: &MessageStore,
) {
    let mut messages = state.messages.write().await;
    if *messages == *mutated {
        *messages = previous;
    }
}

pub(super) async fn persist_conversation_delete_checked(
    state: &AppState,
    username: &str,
) -> Result<Option<u64>, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(None);
    };
    db.delete_messages_from_user(username)
        .await
        .map(Some)
        .map_err(|error| format!("conversation deletion persistence failed: {error}"))
}
