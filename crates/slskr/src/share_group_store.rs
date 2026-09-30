use super::{
    bounded_user_username, json_escape, json_usize_option, truncate_utf8_bytes, unix_timestamp,
    AppState, RecordListFilter, MAX_LIST_DESCRIPTION_BYTES, MAX_LIST_NAME_BYTES, MAX_SHARE_GROUPS,
    MAX_SHARE_GROUP_MEMBERS, MAX_TOTAL_SHARE_GROUP_MEMBERS,
};

#[derive(Clone, Debug)]
pub(crate) struct ShareGroupMember {
    pub(crate) username: String,
    pub(crate) added_at: u64,
}

impl ShareGroupMember {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"username\":\"{}\",\"added_at\":{}}}",
            json_escape(&self.username),
            self.added_at
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ShareGroupRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) members: Vec<ShareGroupMember>,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
}

impl ShareGroupRecord {
    pub(crate) fn json(&self) -> String {
        let members = self
            .members
            .iter()
            .map(ShareGroupMember::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"description\":\"{}\",\"members\":[{}],\"member_count\":{},\"created_at\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.description),
            members,
            self.members.len(),
            self.created_at,
            self.updated_at
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ShareGroupStore {
    pub(crate) records: Vec<ShareGroupRecord>,
    pub(crate) next_id: u64,
    pub(crate) updated_at: u64,
    pub(crate) max_records: usize,
    pub(crate) max_members_per_group: usize,
}

#[allow(dead_code)]
impl ShareGroupStore {
    pub(crate) fn new() -> Self {
        Self::with_limits(MAX_SHARE_GROUPS, MAX_SHARE_GROUP_MEMBERS)
    }

    pub(crate) fn with_limits(max_records: usize, max_members_per_group: usize) -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
            max_members_per_group: max_members_per_group.max(1),
        }
    }

    pub(crate) fn from_persisted(
        groups: Vec<crate::persistence::ShareGroupRecord>,
        members: Vec<crate::persistence::ShareGroupMemberRecord>,
    ) -> Self {
        let mut store = Self::new();
        let mut max_id = 0_u64;
        let mut retained_members = 0_usize;
        let mut seen_ids = std::collections::HashSet::new();
        for record in groups {
            if let Some(value) = record
                .id
                .strip_prefix("sg-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                max_id = max_id.max(value);
            }
            if store.records.len() < store.max_records && seen_ids.insert(record.id.clone()) {
                store.records.push(ShareGroupRecord {
                    id: record.id,
                    name: truncate_utf8_bytes(record.name, MAX_LIST_NAME_BYTES),
                    description: truncate_utf8_bytes(
                        record.description,
                        MAX_LIST_DESCRIPTION_BYTES,
                    ),
                    members: Vec::new(),
                    created_at: u64::try_from(record.created_at).unwrap_or_default(),
                    updated_at: u64::try_from(record.updated_at).unwrap_or_default(),
                });
            }
        }
        for mut member in members {
            member.username = bounded_user_username(&member.username);
            if let Some(group) = store
                .records
                .iter_mut()
                .find(|record| record.id == member.group_id)
            {
                if retained_members < MAX_TOTAL_SHARE_GROUP_MEMBERS
                    && group.members.len() < store.max_members_per_group
                    && !group
                        .members
                        .iter()
                        .any(|existing| existing.username.eq_ignore_ascii_case(&member.username))
                {
                    group.members.push(ShareGroupMember {
                        username: member.username,
                        added_at: u64::try_from(member.added_at).unwrap_or_default(),
                    });
                    retained_members = retained_members.saturating_add(1);
                }
            }
        }
        for group in &mut store.records {
            group.members.sort_by(|left, right| {
                left.username
                    .cmp(&right.username)
                    .then(left.added_at.cmp(&right.added_at))
            });
        }
        store.next_id = max_id.saturating_add(1).max(1);
        store.updated_at = store
            .records
            .iter()
            .map(|record| record.updated_at)
            .max()
            .unwrap_or_else(unix_timestamp);
        store
    }

    pub(crate) fn create(&mut self, name: String, description: String) -> Option<ShareGroupRecord> {
        if self.records.len() >= self.max_records {
            return None;
        }
        let now = unix_timestamp();
        let id = format!("sg-{}", self.allocate_id());
        let record = ShareGroupRecord {
            id,
            name: truncate_utf8_bytes(name, MAX_LIST_NAME_BYTES),
            description: truncate_utf8_bytes(description, MAX_LIST_DESCRIPTION_BYTES),
            members: Vec::new(),
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
            let id = format!("sg-{candidate}");
            if !self.records.iter().any(|record| record.id == id) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded share-group store must leave an available u64 id")
    }

    pub(crate) fn get(&self, id: &str) -> Option<ShareGroupRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    pub(crate) fn update(
        &mut self,
        id: &str,
        name: String,
        description: String,
    ) -> Option<ShareGroupRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.name = truncate_utf8_bytes(name, MAX_LIST_NAME_BYTES);
        record.description = truncate_utf8_bytes(description, MAX_LIST_DESCRIPTION_BYTES);
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

    pub(crate) fn add_member(
        &mut self,
        group_id: &str,
        username: String,
    ) -> Result<Option<(ShareGroupRecord, bool)>, ()> {
        let username = bounded_user_username(&username);
        let total_members = self.total_members();
        let now = unix_timestamp();
        let Some(record) = self.records.iter_mut().find(|r| r.id == group_id) else {
            return Ok(None);
        };
        let added = if record
            .members
            .iter()
            .any(|member| member.username.eq_ignore_ascii_case(&username))
        {
            false
        } else {
            if record.members.len() >= self.max_members_per_group
                || total_members >= MAX_TOTAL_SHARE_GROUP_MEMBERS
            {
                return Err(());
            }
            record.members.push(ShareGroupMember {
                username,
                added_at: now,
            });
            record.updated_at = now;
            self.updated_at = now;
            true
        };
        Ok(Some((record.clone(), added)))
    }

    pub(crate) fn remove_member(
        &mut self,
        group_id: &str,
        username: &str,
    ) -> Option<ShareGroupRecord> {
        let username = bounded_user_username(username);
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == group_id)?;
        if let Some(pos) = record
            .members
            .iter()
            .position(|member| member.username.eq_ignore_ascii_case(&username))
        {
            record.members.remove(pos);
            record.updated_at = now;
            self.updated_at = now;
            Some(record.clone())
        } else {
            None
        }
    }

    pub(crate) fn user_group_json(&self, username: &str) -> String {
        let username = bounded_user_username(username);
        let groups = self
            .records
            .iter()
            .filter_map(|record| {
                let member = record
                    .members
                    .iter()
                    .find(|member| member.username == username)?;
                Some(serde_json::json!({
                    "id": record.id,
                    "name": record.name,
                    "description": record.description,
                    "added_at": member.added_at,
                }))
            })
            .collect::<Vec<_>>();
        let primary_group = groups
            .first()
            .and_then(|group| group.get("name"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("default");
        let primary_group_id = groups.first().and_then(|group| group.get("id")).cloned();
        serde_json::json!({
            "username": username,
            "group": primary_group,
            "group_id": primary_group_id,
            "groups": groups,
            "groupCount": groups.len(),
        })
        .to_string()
    }

    pub(crate) fn total_members(&self) -> usize {
        self.records.iter().map(|record| record.members.len()).sum()
    }

    #[allow(dead_code)]
    pub(crate) fn json(&self, query: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let filtered_count = self
            .records
            .iter()
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
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.name.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(ShareGroupRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

pub(super) async fn persist_share_group(
    state: &AppState,
    record: &ShareGroupRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::ShareGroupRecord {
        id: record.id.clone(),
        name: record.name.clone(),
        description: record.description.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    let members = record
        .members
        .iter()
        .map(|member| crate::persistence::ShareGroupMemberRecord {
            group_id: record.id.clone(),
            username: member.username.clone(),
            added_at: i64::try_from(member.added_at).unwrap_or(i64::MAX),
        })
        .collect::<Vec<_>>();
    db.replace_share_group(&persisted, &members)
        .await
        .map_err(|error| format!("share group persistence failed: {error}"))?;
    Ok(true)
}

pub(super) fn share_group_store_matches(
    current: &ShareGroupStore,
    expected: &ShareGroupStore,
) -> bool {
    current.next_id == expected.next_id
        && current.updated_at == expected.updated_at
        && current.max_records == expected.max_records
        && current.max_members_per_group == expected.max_members_per_group
        && current.records.len() == expected.records.len()
        && current
            .records
            .iter()
            .zip(&expected.records)
            .all(|(current, expected)| {
                current.id == expected.id
                    && current.name == expected.name
                    && current.description == expected.description
                    && current.created_at == expected.created_at
                    && current.updated_at == expected.updated_at
                    && current.members.len() == expected.members.len()
                    && current
                        .members
                        .iter()
                        .zip(&expected.members)
                        .all(|(current, expected)| {
                            current.username == expected.username
                                && current.added_at == expected.added_at
                        })
            })
}

pub(super) async fn persist_share_group_delete(state: &AppState, id: &str) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_share_group(id)
        .await
        .map_err(|error| format!("share group revocation persistence failed: {error}"))?;
    Ok(true)
}
