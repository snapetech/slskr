use super::{
    json_escape, truncate_utf8_bytes, unix_timestamp, AppState, MAX_INTERESTS_PER_KIND,
    MAX_INTEREST_NAME_BYTES,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InterestRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) created_at: u64,
}

impl InterestRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"kind\":\"{}\",\"created_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.kind),
            self.created_at
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InterestStore {
    pub(crate) liked: Vec<InterestRecord>,
    pub(crate) hated: Vec<InterestRecord>,
    pub(crate) next_id: u64,
    pub(crate) updated_at: u64,
}

impl InterestStore {
    pub(crate) fn new() -> Self {
        Self {
            liked: Vec::new(),
            hated: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::InterestRecord>) -> Self {
        let mut store = Self::new();
        store.liked.clear();
        store.hated.clear();
        let mut seen_ids = std::collections::HashSet::new();
        let mut seen_liked = std::collections::HashSet::new();
        let mut seen_hated = std::collections::HashSet::new();
        for mut record in records {
            record.name = truncate_utf8_bytes(record.name, MAX_INTEREST_NAME_BYTES);
            if !seen_ids.insert(record.id.clone()) {
                continue;
            }
            if let Some(number) = record
                .id
                .split_once('-')
                .and_then(|(_, value)| value.parse::<u64>().ok())
            {
                store.next_id = store.next_id.max(number.saturating_add(1));
            }
            let created_at = u64::try_from(record.created_at).unwrap_or(0);
            store.updated_at = store.updated_at.max(created_at);
            let record = InterestRecord {
                id: record.id,
                name: record.name,
                kind: record.kind,
                created_at,
            };
            let normalized_name = record.name.to_ascii_lowercase();
            if record.kind == "hated" {
                if store.hated.len() < MAX_INTERESTS_PER_KIND && seen_hated.insert(normalized_name)
                {
                    store.hated.push(record);
                }
            } else {
                if store.liked.len() < MAX_INTERESTS_PER_KIND && seen_liked.insert(normalized_name)
                {
                    store.liked.push(record);
                }
            }
        }
        store
    }

    pub(crate) fn merge_configured(&mut self, liked: &[String], hated: &[String]) {
        for interest in liked {
            let _ = self.add_liked(interest.clone());
        }
        for interest in hated {
            let _ = self.add_hated(interest.clone());
        }
    }

    pub(crate) fn add_liked(&mut self, name: String) -> Option<(InterestRecord, bool)> {
        let name = truncate_utf8_bytes(name, MAX_INTEREST_NAME_BYTES);
        if let Some(record) = self
            .liked
            .iter()
            .find(|record| record.name.eq_ignore_ascii_case(&name))
        {
            return Some((record.clone(), false));
        }
        if self.liked.len() >= MAX_INTERESTS_PER_KIND {
            return None;
        }
        let id = format!("liked-{}", self.allocate_id());
        let now = unix_timestamp();
        let record = InterestRecord {
            id,
            name,
            kind: "liked".to_string(),
            created_at: now,
        };
        self.liked.push(record.clone());
        self.updated_at = now;
        Some((record, true))
    }

    pub(crate) fn add_hated(&mut self, name: String) -> Option<(InterestRecord, bool)> {
        let name = truncate_utf8_bytes(name, MAX_INTEREST_NAME_BYTES);
        if let Some(record) = self
            .hated
            .iter()
            .find(|record| record.name.eq_ignore_ascii_case(&name))
        {
            return Some((record.clone(), false));
        }
        if self.hated.len() >= MAX_INTERESTS_PER_KIND {
            return None;
        }
        let id = format!("hated-{}", self.allocate_id());
        let now = unix_timestamp();
        let record = InterestRecord {
            id,
            name,
            kind: "hated".to_string(),
            created_at: now,
        };
        self.hated.push(record.clone());
        self.updated_at = now;
        Some((record, true))
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=(self.liked.len() + self.hated.len()) {
            let suffix = format!("-{candidate}");
            if !self
                .liked
                .iter()
                .chain(&self.hated)
                .any(|record| record.id.ends_with(&suffix))
            {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded interest store must leave an available u64 id")
    }

    pub(crate) fn remove_liked(&mut self, id: &str) -> bool {
        if let Some(pos) = self.liked.iter().position(|r| r.id == id) {
            self.liked.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    pub(crate) fn remove_hated(&mut self, id: &str) -> bool {
        if let Some(pos) = self.hated.iter().position(|r| r.id == id) {
            self.hated.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    pub(crate) fn json_liked(&self) -> String {
        let records = self
            .liked
            .iter()
            .map(InterestRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"kind\":\"liked\",\"updated_at\":{}}}",
            records,
            self.liked.len(),
            self.updated_at
        )
    }

    pub(crate) fn json_hated(&self) -> String {
        let records = self
            .hated
            .iter()
            .map(InterestRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"kind\":\"hated\",\"updated_at\":{}}}",
            records,
            self.hated.len(),
            self.updated_at
        )
    }

    pub(crate) fn recommendations_json(&self, field: &str) -> String {
        let recommendations = self
            .liked
            .iter()
            .enumerate()
            .map(|(index, interest)| {
                format!(
                    "{{\"id\":\"rec-{}\",\"interest\":\"{}\",\"query\":\"{}\",\"score\":{},\"source\":\"liked-interest\"}}",
                    index + 1,
                    json_escape(&interest.name),
                    json_escape(&interest.name),
                    100_u64.saturating_sub(index as u64)
                )
            })
            .collect::<Vec<_>>();
        format!(
            "{{\"{}\":[{}],\"count\":{},\"updated_at\":{}}}",
            field,
            recommendations.join(","),
            recommendations.len(),
            self.updated_at
        )
    }

    pub(crate) fn versioned_recommendations_json(&self) -> String {
        let recommendations = self
            .liked
            .iter()
            .enumerate()
            .map(|(index, interest)| {
                serde_json::json!({
                    "item": interest.name,
                    "score": 100_u64.saturating_sub(index as u64),
                })
            })
            .collect::<Vec<_>>();
        let unrecommendations = self
            .hated
            .iter()
            .enumerate()
            .map(|(index, interest)| {
                serde_json::json!({
                    "item": interest.name,
                    "score": 100_u64.saturating_sub(index as u64),
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "recommendations": recommendations,
            "unrecommendations": unrecommendations,
        })
        .to_string()
    }

    pub(crate) fn item_recommendations_json(&self, item_id: &str) -> String {
        let recommendations = self
            .liked
            .iter()
            .enumerate()
            .map(|(index, interest)| {
                format!(
                    "{{\"id\":\"{}-rec-{}\",\"item_id\":\"{}\",\"interest\":\"{}\",\"score\":{},\"source\":\"liked-interest\"}}",
                    json_escape(item_id),
                    index + 1,
                    json_escape(item_id),
                    json_escape(&interest.name),
                    100_u64.saturating_sub(index as u64)
                )
            })
            .collect::<Vec<_>>();
        format!(
            "{{\"item_id\":\"{}\",\"recommendations\":[{}],\"count\":{},\"updated_at\":{}}}",
            json_escape(item_id),
            recommendations.join(","),
            recommendations.len(),
            self.updated_at
        )
    }

    pub(crate) fn versioned_item_recommendations_json(&self, item_id: &str) -> String {
        let recommendations = self
            .liked
            .iter()
            .enumerate()
            .map(|(index, interest)| {
                serde_json::json!({
                    "item": interest.name,
                    "score": 100_u64.saturating_sub(index as u64),
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "item": item_id,
            "recommendations": recommendations,
        })
        .to_string()
    }
}

pub(super) async fn persist_interest_checked(
    state: &AppState,
    record: &InterestRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::InterestRecord {
        id: record.id.clone(),
        name: record.name.clone(),
        kind: record.kind.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
    };
    db.upsert_interest(&persisted)
        .await
        .map_err(|error| format!("interest persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_interest_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_interest(id)
        .await
        .map_err(|error| format!("interest deletion persistence failed: {error}"))?;
    Ok(true)
}
