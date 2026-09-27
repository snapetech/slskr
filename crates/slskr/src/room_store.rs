use super::{
    json_escape, json_option, json_u32_option, json_usize_option, truncate_utf8_bytes,
    unix_timestamp, unix_timestamp_millis, RecordListFilter, RoomList, RoomListEntry,
};

pub(super) const MAX_ROOM_MESSAGES_PER_ROOM: usize = 1_000;
pub(super) const MAX_TOTAL_ROOM_MESSAGES: usize = 10_000;
pub(super) const MAX_ROOM_RECORDS: usize = 1_024;
pub(super) const MAX_ROOM_MEMBERS_PER_ROOM: usize = 10_000;
pub(super) const MAX_TOTAL_ROOM_MEMBERS: usize = 50_000;
pub(super) const MAX_ROOM_NAME_BYTES: usize = 1024;
pub(super) const MAX_ROOM_USERNAME_BYTES: usize = 1024;
pub(super) const MAX_ROOM_MESSAGE_BODY_BYTES: usize = 4 * 1024;
pub(super) const MAX_ROOM_TICKER_BYTES: usize = 16 * 1024;
pub(super) const MAX_ROOM_ERROR_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomMessageRecord {
    pub(crate) id: u64,
    pub(crate) username: String,
    pub(crate) body: String,
    pub(crate) created_at: u64,
    pub(crate) created_at_ms: u64,
}

impl RoomMessageRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"username\":\"{}\",\"body\":\"{}\",\"created_at\":{},\"created_at_ms\":{}}}",
            self.id,
            json_escape(&self.username),
            json_escape(&self.body),
            self.created_at,
            self.created_at_ms
        )
    }

    pub(crate) fn controller_json(&self, room_name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": self.id.to_string(),
            "timestamp": self.created_at.to_string(),
            "createdAtMs": self.created_at_ms,
            "username": self.username,
            "message": self.body,
            "roomName": room_name,
        })
    }
}

/// Mirrors the oracle's real `UserDataResponse` fields for a room member,
/// sourced from the server's real `JoinedRoom` roster snapshot rather than
/// a placeholder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomRosterEntry {
    pub(crate) username: String,
    pub(crate) status: u32,
    pub(crate) average_speed: u32,
    pub(crate) upload_count: u64,
    pub(crate) file_count: u32,
    pub(crate) directory_count: u32,
    pub(crate) slots_free: u32,
    pub(crate) country_code: String,
}

impl RoomRosterEntry {
    /// `status` matches the wire protocol's numeric UserPresence codes
    /// (0=offline, 1=away, 2=online), which the oracle serializes by enum
    /// name via a global `JsonStringEnumConverter`.
    pub(crate) fn controller_json(&self, local_username: &str) -> serde_json::Value {
        serde_json::json!({
            "username": self.username,
            "status": match self.status {
                1 => "Away",
                2 => "Online",
                _ => "Offline",
            },
            "averageSpeed": self.average_speed,
            "uploadCount": self.upload_count,
            "fileCount": self.file_count,
            "directoryCount": self.directory_count,
            "slotsFree": self.slots_free,
            "countryCode": self.country_code,
            // Matches the oracle's real `Self = self ? self : (bool?)null`:
            // present-and-true only for the caller's own username, absent
            // (never `false`) otherwise.
            "self": if self.username.eq_ignore_ascii_case(local_username) {
                serde_json::Value::Bool(true)
            } else {
                serde_json::Value::Null
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomRecord {
    pub(crate) name: String,
    pub(crate) joined: bool,
    pub(crate) kind: &'static str,
    pub(crate) user_count: Option<u32>,
    pub(crate) operated: bool,
    pub(crate) last_error: Option<String>,
    pub(crate) ticker: Option<String>,
    pub(crate) members: Vec<String>,
    pub(crate) roster: Vec<RoomRosterEntry>,
    pub(crate) messages: Vec<RoomMessageRecord>,
    pub(crate) updated_at: u64,
}

impl RoomRecord {
    pub(crate) fn json(&self) -> String {
        let messages = self
            .messages
            .iter()
            .map(RoomMessageRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"name\":\"{}\",\"joined\":{},\"kind\":\"{}\",\"user_count\":{},\"operated\":{},\"last_error\":{},\"ticker\":{},\"members\":{},\"messages\":[{}],\"message_count\":{},\"updated_at\":{}}}",
            json_escape(&self.name),
            self.joined,
            self.kind,
            json_u32_option(self.user_count),
            self.operated,
            json_option(self.last_error.as_deref()),
            json_option(self.ticker.as_deref()),
            serde_json::to_string(&self.members).unwrap_or_else(|_| "[]".to_owned()),
            messages,
            self.messages.len(),
            self.updated_at
        )
    }

    pub(crate) fn controller_info_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "userCount": self.user_count.unwrap_or(0),
            "isPrivate": self.kind != "public",
            "isOwned": self.operated,
            "isModerated": self.operated,
            "lastError": self.last_error,
            "ticker": self.ticker,
            "memberCount": self.members.len(),
        })
    }

    pub(crate) fn controller_room_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "isPrivate": self.kind != "public",
            "users": self.members,
            "messages": self.messages.iter().map(|message| message.controller_json(&self.name)).collect::<Vec<_>>(),
            "ticker": self.ticker,
            "lastError": self.last_error,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RoomStore {
    pub(crate) records: Vec<RoomRecord>,
    pub(crate) next_message_id: u64,
    pub(crate) updated_at: u64,
    pub(crate) max_records: usize,
    pub(crate) max_members_per_room: usize,
}

impl RoomStore {
    pub(crate) fn new() -> Self {
        Self::with_limits(MAX_ROOM_RECORDS, MAX_ROOM_MEMBERS_PER_ROOM)
    }

    pub(crate) fn with_limits(max_records: usize, max_members_per_room: usize) -> Self {
        Self {
            records: Vec::new(),
            next_message_id: 1,
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
            max_members_per_room: max_members_per_room.max(1),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::RoomRecord>) -> Self {
        let mut store = Self::new();
        store.records = records
            .into_iter()
            .take(store.max_records)
            .map(|record| RoomRecord {
                name: bounded_room_name(&record.name),
                joined: record.subscribed,
                kind: if record.owner.is_some() {
                    "private"
                } else {
                    "local"
                },
                user_count: None,
                operated: false,
                last_error: None,
                ticker: None,
                members: Vec::new(),
                roster: Vec::new(),
                messages: Vec::new(),
                updated_at: u64::try_from(record.last_activity).unwrap_or_default(),
            })
            .collect();
        store.updated_at = store
            .records
            .iter()
            .map(|record| record.updated_at)
            .max()
            .unwrap_or_else(unix_timestamp);
        store
    }

    pub(crate) fn merge_configured(&mut self, rooms: &[String]) {
        for room in rooms {
            let _ = self.join(room.clone());
        }
    }

    pub(crate) fn join(&mut self, name: String) -> Option<RoomRecord> {
        let name = bounded_room_name(&name);
        let now = unix_timestamp();
        if let Some(record) = self.records.iter_mut().find(|record| record.name == name) {
            record.joined = true;
            record.last_error = None;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = RoomRecord {
            name,
            joined: true,
            kind: "local",
            user_count: None,
            operated: false,
            last_error: None,
            ticker: None,
            members: Vec::new(),
            roster: Vec::new(),
            messages: Vec::new(),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn leave(&mut self, name: &str) -> Option<RoomRecord> {
        let name = bounded_room_name(name);
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.name == name)?;
        record.joined = false;
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn fail_join(&mut self, name: &str, reason: String) -> Option<RoomRecord> {
        let name = bounded_room_name(name);
        let reason = truncate_utf8_bytes(reason, MAX_ROOM_ERROR_BYTES);
        let now = unix_timestamp();
        if let Some(record) = self.records.iter_mut().find(|record| record.name == name) {
            record.joined = false;
            record.last_error = Some(reason);
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = RoomRecord {
            name,
            joined: false,
            kind: "local",
            user_count: None,
            operated: false,
            last_error: Some(reason),
            ticker: None,
            members: Vec::new(),
            roster: Vec::new(),
            messages: Vec::new(),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn apply_room_list(&mut self, room_list: &RoomList) {
        for entry in &room_list.public_rooms {
            self.upsert_room_list_entry(entry, "public", false);
        }
        for entry in &room_list.owned_private_rooms {
            self.upsert_room_list_entry(entry, "owned_private", true);
        }
        for entry in &room_list.private_rooms {
            let operated = room_list
                .operated_private_rooms
                .iter()
                .any(|room| room == &entry.name);
            self.upsert_room_list_entry(entry, "private", operated);
        }
        for room in &room_list.operated_private_rooms {
            let room = bounded_room_name(room);
            if self.records.len() < self.max_records
                && !self.records.iter().any(|record| record.name == room)
            {
                let now = unix_timestamp();
                self.records.push(RoomRecord {
                    name: room,
                    joined: false,
                    kind: "operated_private",
                    user_count: None,
                    operated: true,
                    last_error: None,
                    ticker: None,
                    members: Vec::new(),
                    roster: Vec::new(),
                    messages: Vec::new(),
                    updated_at: now,
                });
                self.updated_at = now;
            }
        }
    }

    pub(crate) fn upsert_room_list_entry(
        &mut self,
        entry: &RoomListEntry,
        kind: &'static str,
        operated: bool,
    ) -> Option<RoomRecord> {
        let name = bounded_room_name(&entry.name);
        let now = unix_timestamp();
        if let Some(record) = self.records.iter_mut().find(|record| record.name == name) {
            record.kind = kind;
            record.user_count = Some(entry.user_count);
            record.operated = operated;
            record.last_error = None;
            record.updated_at = now;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.records.len() >= self.max_records {
            return None;
        }
        let record = RoomRecord {
            name,
            joined: false,
            kind,
            user_count: Some(entry.user_count),
            operated,
            last_error: None,
            ticker: None,
            members: Vec::new(),
            roster: Vec::new(),
            messages: Vec::new(),
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn add_message(
        &mut self,
        room: &str,
        username: String,
        body: String,
    ) -> Option<RoomRecord> {
        let room = bounded_room_name(room);
        let record_index = self.records.iter().position(|record| record.name == room)?;
        while self.total_messages() >= MAX_TOTAL_ROOM_MESSAGES {
            self.evict_oldest_message()?;
        }
        let now = unix_timestamp();
        let now_ms = unix_timestamp_millis().max(
            self.records
                .iter()
                .flat_map(|record| record.messages.iter())
                .map(|message| message.created_at_ms)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        let id = self.allocate_message_id();
        let record = &mut self.records[record_index];
        record.messages.push(RoomMessageRecord {
            id,
            username: truncate_utf8_bytes(username, MAX_ROOM_USERNAME_BYTES),
            body: truncate_utf8_bytes(body, MAX_ROOM_MESSAGE_BODY_BYTES),
            created_at: now,
            created_at_ms: now_ms,
        });
        if record.messages.len() > MAX_ROOM_MESSAGES_PER_ROOM {
            let excess = record.messages.len() - MAX_ROOM_MESSAGES_PER_ROOM;
            record.messages.drain(0..excess);
        }
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn allocate_message_id(&mut self) -> u64 {
        let mut candidate = self.next_message_id.max(1);
        for _ in 0..=self.total_messages() {
            if !self
                .records
                .iter()
                .flat_map(|room| room.messages.iter())
                .any(|message| message.id == candidate)
            {
                self.next_message_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded room message history must leave an available u64 id")
    }

    pub(crate) fn activity_json(&self, local_username: &str) -> String {
        let activity = self
            .records
            .iter()
            .filter(|room| room.joined)
            .filter_map(|room| {
                room.messages
                    .iter()
                    .rev()
                    .find(|message| message.username != local_username)
                    .map(|message| {
                        (
                            room.name.clone(),
                            serde_json::Value::from(message.created_at.saturating_mul(1_000)),
                        )
                    })
            })
            .collect::<serde_json::Map<String, serde_json::Value>>();
        serde_json::Value::Object(activity).to_string()
    }

    pub(crate) fn set_ticker(&mut self, room: &str, ticker: String) -> Option<RoomRecord> {
        let room = bounded_room_name(room);
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.name == room)?;
        record.ticker = Some(truncate_utf8_bytes(ticker, MAX_ROOM_TICKER_BYTES));
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    /// Applies the real roster snapshot from the server's `JoinedRoom`
    /// message -- matches the oracle's `IRoomTracker`, which populates the
    /// room's user list from this same message rather than leaving it
    /// empty until someone happens to join/leave afterward.
    pub(crate) fn apply_roster(
        &mut self,
        room: &str,
        roster: Vec<RoomRosterEntry>,
    ) -> Option<RoomRecord> {
        let room = bounded_room_name(room);
        let now = unix_timestamp();
        let cap = self.max_members_per_room;
        let record = self.records.iter_mut().find(|record| record.name == room)?;
        record.roster = roster.into_iter().take(cap).collect();
        record.members = record
            .roster
            .iter()
            .map(|user| user.username.clone())
            .collect();
        record.user_count = Some(u32::try_from(record.roster.len()).unwrap_or(u32::MAX));
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn add_member(
        &mut self,
        room: &str,
        username: String,
    ) -> Result<Option<RoomRecord>, ()> {
        let room = bounded_room_name(room);
        let now = unix_timestamp();
        let username = truncate_utf8_bytes(username.trim().to_owned(), MAX_ROOM_USERNAME_BYTES);
        if username.is_empty() {
            return Ok(None);
        }
        let total_members = self.total_members();
        let record = match self.records.iter_mut().find(|record| record.name == room) {
            Some(record) => record,
            None => return Ok(None),
        };
        if !record
            .members
            .iter()
            .any(|member| member.eq_ignore_ascii_case(&username))
        {
            if record.members.len() >= self.max_members_per_room
                || total_members >= MAX_TOTAL_ROOM_MEMBERS
            {
                return Err(());
            }
            record.members.push(username.clone());
        }
        if !record
            .roster
            .iter()
            .any(|user| user.username.eq_ignore_ascii_case(&username))
            && record.roster.len() < self.max_members_per_room
        {
            record.roster.push(RoomRosterEntry {
                username: username.clone(),
                status: 0,
                average_speed: 0,
                upload_count: 0,
                file_count: 0,
                directory_count: 0,
                slots_free: 0,
                country_code: String::new(),
            });
        }
        record.user_count = Some(u32::try_from(record.members.len()).unwrap_or(u32::MAX));
        record.updated_at = now;
        self.updated_at = now;
        Ok(Some(record.clone()))
    }

    pub(crate) fn remove_member(&mut self, room: &str, username: &str) -> Option<RoomRecord> {
        let room = bounded_room_name(room);
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.name == room)?;
        let member = record
            .members
            .iter()
            .position(|member| member.eq_ignore_ascii_case(username))?;
        record.members.remove(member);
        record
            .roster
            .retain(|user| !user.username.eq_ignore_ascii_case(username));
        record.user_count = Some(u32::try_from(record.members.len()).unwrap_or(u32::MAX));
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn total_messages(&self) -> usize {
        self.records
            .iter()
            .map(|record| record.messages.len())
            .sum()
    }

    pub(crate) fn total_members(&self) -> usize {
        self.records.iter().map(|record| record.members.len()).sum()
    }

    pub(crate) fn evict_oldest_message(&mut self) -> Option<()> {
        let room_index = self
            .records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| {
                record
                    .messages
                    .first()
                    .map(|message| (index, message.created_at))
            })
            .min_by_key(|(index, created_at)| (*created_at, *index))?
            .0;
        self.records[room_index].messages.remove(0);
        Some(())
    }

    pub(crate) fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let filtered_count = self
            .records
            .iter()
            .filter(|record| filter.joined.is_none_or(|joined| record.joined == joined))
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.name.to_ascii_lowercase().contains(q))
            })
            .count();
        format!(
            "{{\"entries\":{},\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            self.json_array(query),
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    pub(crate) fn json_array(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| filter.joined.is_none_or(|joined| record.joined == joined))
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.name.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(RoomRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }

    pub(crate) fn joined_names_json(&self) -> String {
        let records = self
            .records
            .iter()
            .filter(|record| record.joined)
            .map(|record| json_option(Some(record.name.as_str())))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }

    pub(crate) fn controller_available_json(&self) -> String {
        serde_json::Value::Array(
            self.records
                .iter()
                .map(RoomRecord::controller_info_json)
                .collect::<Vec<_>>(),
        )
        .to_string()
    }

    pub(crate) fn summary_json(&self) -> String {
        let joined = self.records.iter().filter(|record| record.joined).count();
        let messages = self
            .records
            .iter()
            .map(|record| record.messages.len())
            .sum::<usize>();
        format!(
            "{{\"total\":{},\"joined\":{},\"messages\":{},\"updated_at\":{}}}",
            self.records.len(),
            joined,
            messages,
            self.updated_at
        )
    }
}

pub(crate) fn bounded_room_name(name: &str) -> String {
    truncate_utf8_bytes(name.to_owned(), MAX_ROOM_NAME_BYTES)
}

// Collection Models
