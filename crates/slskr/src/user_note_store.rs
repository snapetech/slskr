use super::user_store::bounded_user_username;
use super::{json_escape, truncate_utf8_bytes, unix_seconds_rfc3339, unix_timestamp, AppState};

pub(crate) const MAX_USER_NOTES: usize = 4_096;
pub(crate) const MAX_USER_NOTE_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UserNoteRecord {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) note: String,
    pub(crate) color: String,
    pub(crate) icon: String,
    pub(crate) is_high_priority: bool,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
}

impl UserNoteRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"username\":\"{}\",\"note\":\"{}\",\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.username),
            json_escape(&self.note),
            self.created_at,
            self.updated_at
        )
    }

    pub(crate) fn native_json(&self) -> String {
        serde_json::json!({
            "username": self.username,
            "note": self.note,
            "color": self.color,
            "icon": self.icon,
            "isHighPriority": self.is_high_priority,
            "createdAt": unix_seconds_rfc3339(self.created_at),
            "updatedAt": unix_seconds_rfc3339(self.updated_at),
        })
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UserNoteStore {
    pub(crate) records: Vec<UserNoteRecord>,
    pub(crate) next_id: u64,
    pub(crate) updated_at: u64,
}

impl UserNoteStore {
    pub(crate) fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::UserNoteRecord>) -> Self {
        let mut next_id = 1;
        let mut updated_at = unix_timestamp();
        for record in &records {
            if let Some(number) = record
                .id
                .strip_prefix("note-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                next_id = next_id.max(number.saturating_add(1));
            }
        }
        let mut seen_ids = std::collections::HashSet::new();
        let records = records
            .into_iter()
            .filter(|record| seen_ids.insert(record.id.clone()))
            .take(MAX_USER_NOTES)
            .map(|record| {
                let created_at = u64::try_from(record.created_at).unwrap_or(0);
                let record_updated_at = u64::try_from(record.updated_at).unwrap_or(created_at);
                updated_at = updated_at.max(record_updated_at);
                UserNoteRecord {
                    id: record.id,
                    username: bounded_user_username(&record.username),
                    note: truncate_utf8_bytes(record.note, MAX_USER_NOTE_BYTES),
                    color: truncate_utf8_bytes(record.color, 128),
                    icon: truncate_utf8_bytes(record.icon, 128),
                    is_high_priority: record.is_high_priority,
                    created_at,
                    updated_at: record_updated_at,
                }
            })
            .collect();
        Self {
            records,
            next_id,
            updated_at,
        }
    }

    pub(crate) fn create(&mut self, username: String, note: String) -> Option<UserNoteRecord> {
        if self.records.len() >= MAX_USER_NOTES {
            return None;
        }
        let id = format!("note-{}", self.allocate_id());
        let now = unix_timestamp();
        let record = UserNoteRecord {
            id,
            username: bounded_user_username(&username),
            note: truncate_utf8_bytes(note, MAX_USER_NOTE_BYTES),
            color: String::new(),
            icon: String::new(),
            is_high_priority: false,
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            let id = format!("note-{candidate}");
            if !self.records.iter().any(|record| record.id == id) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded user-note store must leave an available u64 id")
    }

    pub(crate) fn get(&self, id: &str) -> Option<UserNoteRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    pub(crate) fn get_by_username(&self, username: &str) -> Option<UserNoteRecord> {
        self.records
            .iter()
            .find(|record| record.username.eq_ignore_ascii_case(username))
            .cloned()
    }

    pub(crate) fn set_versioned(
        &mut self,
        username: String,
        note: String,
        color: String,
        icon: String,
        is_high_priority: bool,
    ) -> Option<UserNoteRecord> {
        let username = bounded_user_username(&username);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username.eq_ignore_ascii_case(&username))
        {
            record.note = truncate_utf8_bytes(note, MAX_USER_NOTE_BYTES);
            record.color = truncate_utf8_bytes(color, 128);
            record.icon = truncate_utf8_bytes(icon, 128);
            record.is_high_priority = is_high_priority;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= MAX_USER_NOTES {
            return None;
        }
        let record = UserNoteRecord {
            id: format!("note-{}", self.allocate_id()),
            username,
            note: truncate_utf8_bytes(note, MAX_USER_NOTE_BYTES),
            color: truncate_utf8_bytes(color, 128),
            icon: truncate_utf8_bytes(icon, 128),
            is_high_priority,
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn update(&mut self, id: &str, note: String) -> Option<UserNoteRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.note = truncate_utf8_bytes(note, MAX_USER_NOTE_BYTES);
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
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

    pub(crate) fn delete_by_username(&mut self, username: &str) -> Option<UserNoteRecord> {
        let position = self
            .records
            .iter()
            .position(|record| record.username.eq_ignore_ascii_case(username))?;
        let removed = self.records.remove(position);
        self.updated_at = unix_timestamp();
        Some(removed)
    }

    pub(crate) fn json(&self, _query: Option<&str>) -> String {
        let records = self
            .records
            .iter()
            .map(UserNoteRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }
}

pub(super) async fn persist_user_note_checked(
    state: &AppState,
    record: &UserNoteRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::UserNoteRecord {
        id: record.id.clone(),
        username: record.username.clone(),
        note: record.note.clone(),
        color: record.color.clone(),
        icon: record.icon.clone(),
        is_high_priority: record.is_high_priority,
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    db.upsert_user_note(&persisted)
        .await
        .map_err(|error| format!("user note persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_user_note_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_user_note(id)
        .await
        .map_err(|error| format!("user note deletion persistence failed: {error}"))?;
    Ok(true)
}
