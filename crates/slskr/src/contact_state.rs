use super::{
    bounded_user_username, json_escape, json_u32_option, truncate_utf8_bytes, unix_seconds_rfc3339,
    unix_timestamp, AppState, RecordListFilter, MAX_CONTACT_RECORDS, MAX_CONTACT_STATUS_BYTES,
};

// Contact Models
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContactRecord {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) online: bool,
    pub(crate) status: String,
    pub(crate) free_upload_slots: Option<u32>,
    pub(crate) queue_length: Option<u32>,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
}

impl ContactRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"username\":\"{}\",\"online\":{},\"status\":\"{}\",\"free_upload_slots\":{},\"queue_length\":{},\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.username),
            self.online,
            json_escape(&self.status),
            json_u32_option(self.free_upload_slots),
            json_u32_option(self.queue_length),
            self.created_at,
            self.updated_at
        )
    }

    pub(crate) fn native_json(&self, peer_id: &str) -> String {
        serde_json::json!({
            "id": self.id,
            "peerId": peer_id,
            "nickname": self.username,
            "verified": true,
            "lastSeen": serde_json::Value::Null,
            "cachedEndpointsJson": serde_json::Value::Null,
            "createdAt": unix_seconds_rfc3339(self.created_at),
        })
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContactStore {
    pub(crate) records: Vec<ContactRecord>,
    pub(crate) next_id: u64,
    pub(crate) updated_at: u64,
    pub(crate) max_records: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ContactUpdateError {
    NotFound,
    DuplicateUsername,
}

impl ContactStore {
    pub(crate) fn new() -> Self {
        Self::with_max_records(MAX_CONTACT_RECORDS)
    }

    pub(crate) fn with_max_records(max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::ContactRecord>) -> Self {
        let mut next_id = 1;
        let mut updated_at = unix_timestamp();
        let mut projected = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        for mut record in records {
            record.username = bounded_user_username(&record.username);
            if let Some(number) = record
                .id
                .strip_prefix("contact-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                next_id = next_id.max(number.saturating_add(1));
            }
            if projected.len() >= MAX_CONTACT_RECORDS
                || !seen_ids.insert(record.id.clone())
                || projected.iter().any(|existing: &ContactRecord| {
                    existing.username.eq_ignore_ascii_case(&record.username)
                })
            {
                continue;
            }
            let created_at = u64::try_from(record.created_at).unwrap_or(0);
            let record_updated_at = u64::try_from(record.updated_at).unwrap_or(created_at);
            updated_at = updated_at.max(record_updated_at);
            projected.push(ContactRecord {
                id: record.id,
                username: record.username,
                online: record.online,
                status: truncate_utf8_bytes(record.status, MAX_CONTACT_STATUS_BYTES),
                free_upload_slots: record
                    .free_upload_slots
                    .and_then(|value| u32::try_from(value).ok()),
                queue_length: record
                    .queue_length
                    .and_then(|value| u32::try_from(value).ok()),
                created_at,
                updated_at: record_updated_at,
            });
        }
        Self {
            records: projected,
            next_id,
            updated_at,
            max_records: MAX_CONTACT_RECORDS,
        }
    }

    pub(crate) fn create(&mut self, username: String) -> Result<(ContactRecord, bool), ()> {
        self.create_with_contract(None, username)
    }

    pub(crate) fn create_with_contract(
        &mut self,
        id: Option<String>,
        username: String,
    ) -> Result<(ContactRecord, bool), ()> {
        let username = bounded_user_username(&username);
        if let Some(record) = self
            .records
            .iter()
            .find(|record| record.username.eq_ignore_ascii_case(&username))
        {
            return Ok((record.clone(), false));
        }
        if self.records.len() >= self.max_records {
            return Err(());
        }
        let now = unix_timestamp();
        let id = id.unwrap_or_else(|| format!("contact-{}", self.allocate_id()));
        let record = ContactRecord {
            id,
            username,
            online: false,
            status: "offline".to_string(),
            free_upload_slots: None,
            queue_length: None,
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Ok((record, true))
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            let id = format!("contact-{candidate}");
            if !self.records.iter().any(|record| record.id == id) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded contact store must leave an available u64 id")
    }

    pub(crate) fn get(&self, id: &str) -> Option<ContactRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    pub(crate) fn update(
        &mut self,
        id: &str,
        username: Option<String>,
        online: Option<bool>,
    ) -> Result<ContactRecord, ContactUpdateError> {
        if !self.records.iter().any(|record| record.id == id) {
            return Err(ContactUpdateError::NotFound);
        }
        let username = username.map(|username| bounded_user_username(&username));
        if username.as_deref().is_some_and(|username| {
            self.records
                .iter()
                .any(|record| record.id != id && record.username.eq_ignore_ascii_case(username))
        }) {
            return Err(ContactUpdateError::DuplicateUsername);
        }
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|r| r.id == id)
            .expect("contact existence checked before mutation");
        if let Some(u) = username {
            record.username = u;
        }
        if let Some(o) = online {
            record.online = o;
            record.status = if o {
                "online".to_string()
            } else {
                "offline".to_string()
            };
        }
        record.updated_at = now;
        self.updated_at = now;
        Ok(record.clone())
    }

    pub(crate) fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    pub(crate) fn json_array(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.username.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(ContactRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }

    pub(crate) fn nearby_json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| record.online)
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.username.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(ContactRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

pub(super) async fn persist_contact_checked(
    state: &AppState,
    record: &ContactRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::ContactRecord {
        id: record.id.clone(),
        username: record.username.clone(),
        online: record.online,
        status: record.status.clone(),
        free_upload_slots: record.free_upload_slots.map(i64::from),
        queue_length: record.queue_length.map(i64::from),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    db.upsert_contact(&persisted)
        .await
        .map_err(|error| format!("contact persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_contact_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_contact(id)
        .await
        .map_err(|error| format!("contact deletion persistence failed: {error}"))?;
    Ok(true)
}
