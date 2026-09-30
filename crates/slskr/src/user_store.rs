use super::{
    json_escape, json_option, json_u32_option, truncate_utf8_bytes, unix_timestamp, AppState,
    UserStats, UserStatus, WatchedUser,
};
use std::collections::HashSet;

pub(super) const MAX_USER_USERNAME_BYTES: usize = 1024;
const MAX_USER_RECORDS: usize = 4_096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UserRecord {
    pub(crate) username: String,
    pub(crate) watched: bool,
    pub(crate) status: Option<String>,
    pub(crate) privileged: bool,
    pub(crate) average_speed: Option<u32>,
    pub(crate) upload_count: Option<u32>,
    pub(crate) file_count: Option<u32>,
    pub(crate) directory_count: Option<u32>,
    pub(crate) updated_at: u64,
}

impl UserRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"username\":\"{}\",\"watched\":{},\"status\":{},\"average_speed\":{},\"upload_count\":{},\"file_count\":{},\"directory_count\":{},\"updated_at\":{}}}",
            json_escape(&self.username),
            self.watched,
            json_option(self.status.as_deref()),
            json_u32_option(self.average_speed),
            json_u32_option(self.upload_count),
            json_u32_option(self.file_count),
            json_u32_option(self.directory_count),
            self.updated_at
        )
    }

    pub(crate) fn controller_status_json(&self) -> serde_json::Value {
        let presence = match self.status.as_deref() {
            Some("online") | Some("Online") => "Online",
            Some("away") | Some("Away") => "Away",
            _ => "Offline",
        };
        serde_json::json!({
            "username": self.username,
            "presence": presence,
            "isPrivileged": self.privileged,
        })
    }

    pub(crate) fn controller_info_json(&self) -> serde_json::Value {
        serde_json::json!({
            "description": "",
            "hasFreeUploadSlot": true,
            "hasPicture": false,
            "picture": null,
            "queueLength": 0,
            "uploadSlots": 0,
            "uploadSpeed": self.average_speed.unwrap_or(0),
            "uploadCount": self.upload_count.unwrap_or(0),
            "fileCount": self.file_count.unwrap_or(0),
            "directoryCount": self.directory_count.unwrap_or(0),
        })
    }
}

#[derive(Debug)]
pub(crate) struct UserStore {
    pub(crate) records: Vec<UserRecord>,
    pub(crate) updated_at: u64,
    pub(crate) max_records: usize,
}

impl UserStore {
    pub(crate) fn new() -> Self {
        Self::with_max_records(MAX_USER_RECORDS)
    }

    pub(crate) fn with_max_records(max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::UserProjectionRecord>) -> Self {
        let mut updated_at = unix_timestamp();
        let mut seen_usernames = HashSet::new();
        let records = records
            .into_iter()
            .map(|mut record| {
                record.username = bounded_user_username(&record.username);
                record
            })
            .filter(|record| seen_usernames.insert(record.username.clone()))
            .take(MAX_USER_RECORDS)
            .map(|record| {
                let persisted_updated_at = record.updated_at.max(0) as u64;
                updated_at = updated_at.max(persisted_updated_at);
                UserRecord {
                    username: record.username,
                    watched: record.watched,
                    status: record.status,
                    privileged: false,
                    average_speed: record.average_speed.and_then(|value| value.try_into().ok()),
                    upload_count: record.upload_count.and_then(|value| value.try_into().ok()),
                    file_count: record.file_count.and_then(|value| value.try_into().ok()),
                    directory_count: record
                        .directory_count
                        .and_then(|value| value.try_into().ok()),
                    updated_at: persisted_updated_at,
                }
            })
            .collect();
        Self {
            records,
            updated_at,
            max_records: MAX_USER_RECORDS,
        }
    }

    pub(crate) fn watch(&mut self, username: String) -> Option<UserRecord> {
        let username = bounded_user_username(&username);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.watched = true;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = UserRecord {
            username,
            watched: true,
            status: None,
            privileged: false,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn unwatch(&mut self, username: &str) -> Option<UserRecord> {
        let username = bounded_user_username(username);
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.username == username)?;
        record.watched = false;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn apply_watched_user(&mut self, user: &WatchedUser) -> Option<UserRecord> {
        let username = bounded_user_username(&user.username);
        let now = unix_timestamp();
        let status = user.status.map(|status| status.to_string());
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = status;
            if let Some(stats) = user.stats.as_ref() {
                record.average_speed = Some(stats.average_speed);
                record.upload_count = Some(stats.upload_count);
                record.file_count = Some(stats.file_count);
                record.directory_count = Some(stats.directory_count);
            }
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = UserRecord {
            username,
            watched: true,
            status,
            privileged: false,
            average_speed: user.stats.as_ref().map(|stats| stats.average_speed),
            upload_count: user.stats.as_ref().map(|stats| stats.upload_count),
            file_count: user.stats.as_ref().map(|stats| stats.file_count),
            directory_count: user.stats.as_ref().map(|stats| stats.directory_count),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn apply_status(&mut self, status: &UserStatus) -> Option<UserRecord> {
        let username = bounded_user_username(&status.username);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.status = Some(status.status.to_string());
            record.privileged = status.privileged;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = UserRecord {
            username,
            watched: false,
            status: Some(status.status.to_string()),
            privileged: status.privileged,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn apply_stats(
        &mut self,
        username: String,
        stats: &UserStats,
    ) -> Option<UserRecord> {
        let username = bounded_user_username(&username);
        let now = unix_timestamp();
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.username == username)
        {
            record.average_speed = Some(stats.average_speed);
            record.upload_count = Some(stats.upload_count);
            record.file_count = Some(stats.file_count);
            record.directory_count = Some(stats.directory_count);
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = UserRecord {
            username,
            watched: false,
            status: None,
            privileged: false,
            average_speed: Some(stats.average_speed),
            upload_count: Some(stats.upload_count),
            file_count: Some(stats.file_count),
            directory_count: Some(stats.directory_count),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn json(&self) -> String {
        let records = self
            .records
            .iter()
            .map(UserRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }

    pub(crate) fn summary_json(&self) -> String {
        let watched = self.records.iter().filter(|record| record.watched).count();
        format!(
            "{{\"total\":{},\"watched\":{},\"updated_at\":{}}}",
            self.records.len(),
            watched,
            self.updated_at
        )
    }
}

pub(crate) fn bounded_user_username(username: &str) -> String {
    truncate_utf8_bytes(username.to_owned(), MAX_USER_USERNAME_BYTES)
}

pub(super) async fn persist_user_projection(
    state: &AppState,
    record: &UserRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::UserProjectionRecord {
        username: record.username.clone(),
        watched: record.watched,
        status: record.status.clone(),
        average_speed: record.average_speed.map(i64::from),
        upload_count: record.upload_count.map(i64::from),
        file_count: record.file_count.map(i64::from),
        directory_count: record.directory_count.map(i64::from),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    db.upsert_user_projection(&persisted)
        .await
        .map_err(|error| format!("user projection persistence failed: {error}"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_negative_user_timestamp_does_not_wrap() {
        let store = UserStore::from_persisted(vec![crate::persistence::UserProjectionRecord {
            username: "peer".to_owned(),
            watched: false,
            status: None,
            average_speed: None,
            upload_count: None,
            file_count: None,
            directory_count: None,
            updated_at: -1,
        }]);
        assert_eq!(store.records[0].updated_at, 0);
        assert_ne!(store.updated_at, u64::MAX);
    }
}
