//! In-memory event history and its compatibility projections.
//!
//! Keeping the bounded event store separate from the controller source makes
//! retention and wire projections auditable without moving domain state into
//! the transport modules.

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EventRecord {
    pub(crate) id: u64,
    pub(crate) kind: String,
    pub(crate) resource: String,
    pub(crate) detail: Option<String>,
    pub(crate) created_at: u64,
}

impl EventRecord {
    pub(crate) fn topic(&self) -> &'static str {
        topic_for_event_kind(&self.kind)
    }

    pub(crate) fn data_json(&self) -> serde_json::Value {
        if self.kind == "log.created" {
            let detail = self
                .detail
                .as_deref()
                .and_then(|detail| serde_json::from_str::<serde_json::Value>(detail).ok())
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "level": "Information",
                        "message": self.detail.as_deref().unwrap_or(&self.resource),
                        "category": self.resource,
                    })
                });
            let category = detail
                .get("category")
                .and_then(|value| value.as_str())
                .unwrap_or(&self.resource);
            let context = detail
                .get("context")
                .and_then(|value| value.as_str())
                .unwrap_or(category);
            return serde_json::json!({
                "id": self.id,
                "kind": self.kind,
                "topic": self.topic(),
                "resource": &self.resource,
                "detail": &self.detail,
                "created_at": self.created_at,
                "timestamp": self.created_at,
                "level": detail.get("level").and_then(|value| value.as_str()).unwrap_or("Information"),
                "message": detail.get("message").and_then(|value| value.as_str()).unwrap_or(""),
                "category": category,
                "context": context,
                "request_id": detail.get("request_id").and_then(|value| value.as_str()),
                "method": detail.get("method").and_then(|value| value.as_str()),
                "path": detail.get("path").and_then(|value| value.as_str()),
                "status": detail.get("status").and_then(|value| value.as_u64()),
                "duration_ms": detail.get("duration_ms").and_then(|value| value.as_u64()),
                "remote_addr": detail.get("remote_addr").and_then(|value| value.as_str()),
            });
        }
        serde_json::json!({
            "id": self.id,
            "kind": self.kind,
            "topic": self.topic(),
            "resource": &self.resource,
            "detail": &self.detail,
            "created_at": self.created_at,
        })
    }

    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":{},\"kind\":\"{}\",\"topic\":\"{}\",\"resource\":\"{}\",\"detail\":{},\"created_at\":{}}}",
            self.id,
            crate::json_escape(&self.kind),
            self.topic(),
            crate::json_escape(&self.resource),
            crate::json_option(self.detail.as_deref()),
            self.created_at
        )
    }

    pub(crate) fn controller_json(&self) -> serde_json::Value {
        let data = self.data_json();
        serde_json::json!({
            "id": self.id.to_string(),
            "timestamp": self.created_at.to_string(),
            "topic": self.topic(),
            "type": self.kind,
            "resource": &self.resource,
            "detail": &self.detail,
            "data": data.to_string(),
            "payload": data,
        })
    }
}

fn topic_for_event_kind(kind: &str) -> &'static str {
    match kind.split('.').next().unwrap_or(kind) {
        "application" | "session" => "application",
        "listener" | "portforwarding" => "listeners",
        "share" => "shares",
        "search" | "wishlist" => "searches",
        "browse" => "browse",
        "transfer" | "upload" | "download" => "transfers",
        "message" | "conversation" => "messages",
        "room" | "listening_party" => "rooms",
        "user" | "contact" | "note" => "users",
        "collection" | "playlist" => "collections",
        "sharegroup" | "grant" => "sharegroups",
        "library" | "catalog" => "library",
        "destination" => "destinations",
        "player" | "playback" | "now_playing" | "external_visualizer" => "player",
        "relay" => "relay",
        "bridge" => "bridge",
        "mesh" => "mesh",
        "webhook" => "webhooks",
        "log" => "logs",
        "security" | "ban" => "security",
        "federation" => "federation",
        "solid" => "solid",
        "lidarr" | "musicbrainz" => "integrations",
        "songid" | "pod" | "stream" => "media",
        "cache" | "backfill" | "telemetry" | "metrics" => "system",
        "config" | "options" => "settings",
        _ => "events",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EventStore {
    pub(crate) records: Vec<EventRecord>,
    pub(crate) next_id: u64,
    pub(crate) history_limit: usize,
    ids: std::collections::HashSet<u64>,
}

impl EventStore {
    pub(crate) fn new(history_limit: usize) -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            history_limit,
            ids: std::collections::HashSet::new(),
        }
    }

    pub(crate) fn from_persisted(
        records: Vec<crate::persistence::EventRecord>,
        history_limit: usize,
    ) -> Self {
        let mut records = records
            .into_iter()
            .filter_map(|record| {
                Some(EventRecord {
                    id: u64::try_from(record.id).ok()?,
                    kind: crate::truncate_utf8_bytes(record.kind, crate::MAX_EVENT_KIND_BYTES),
                    resource: crate::truncate_utf8_bytes(
                        record.resource,
                        crate::MAX_EVENT_RESOURCE_BYTES,
                    ),
                    detail: crate::bounded_event_detail(record.detail),
                    created_at: u64::try_from(record.created_at).ok()?,
                })
            })
            .collect::<Vec<_>>();
        let mut seen_ids = std::collections::HashSet::new();
        records.reverse();
        records.retain(|record| seen_ids.insert(record.id));
        records.reverse();
        if records.len() > history_limit {
            let extra = records.len() - history_limit;
            records.drain(0..extra);
        }
        let ids = records.iter().map(|record| record.id).collect();
        let next_id = records
            .iter()
            .map(|record| record.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        Self {
            records,
            next_id,
            history_limit,
            ids,
        }
    }

    pub(crate) fn record(
        &mut self,
        kind: impl Into<String>,
        resource: impl Into<String>,
        detail: Option<String>,
    ) -> EventRecord {
        let id = self.allocate_id();
        let record = EventRecord {
            id,
            kind: crate::truncate_utf8_bytes(kind.into(), crate::MAX_EVENT_KIND_BYTES),
            resource: crate::truncate_utf8_bytes(resource.into(), crate::MAX_EVENT_RESOURCE_BYTES),
            detail: crate::bounded_event_detail(detail),
            created_at: crate::unix_timestamp(),
        };
        self.records.push(record.clone());
        self.ids.insert(record.id);
        if self.records.len() > self.history_limit {
            let extra = self.records.len() - self.history_limit;
            for removed in self.records.drain(0..extra) {
                self.ids.remove(&removed.id);
            }
        }
        record
    }

    fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            if !self.ids.contains(&candidate) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded event history must leave an available u64 id")
    }

    #[allow(dead_code)]
    pub(crate) fn json(&self, query: Option<&str>) -> String {
        let filter = crate::RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .kind
                    .as_deref()
                    .is_none_or(|kind| record.kind == kind)
            })
            .filter(|record| {
                filter
                    .topic
                    .as_deref()
                    .is_none_or(|topic| record.topic() == topic)
            })
            .filter(|record| {
                filter.q.as_deref().is_none_or(|q| {
                    record.kind.to_ascii_lowercase().contains(q)
                        || record.topic().contains(q)
                        || record.resource.to_ascii_lowercase().contains(q)
                        || record
                            .detail
                            .as_deref()
                            .is_some_and(|detail| detail.to_ascii_lowercase().contains(q))
                })
            })
            .collect::<Vec<_>>();
        let filtered_count = records.len();
        let entries = records
            .into_iter()
            .rev()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(EventRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"entries\":[{}],\"count\":{},\"filtered_count\":{},\"offset\":{},\"limit\":{},\"history_limit\":{},\"next_id\":{}}}",
            entries,
            self.records.len(),
            filtered_count,
            filter.offset,
            crate::json_usize_option(filter.limit),
            self.history_limit,
            self.next_id
        )
    }

    pub(crate) fn controller_json(&self, query: Option<&str>) -> String {
        let filter = crate::RecordListFilter::from_query(query);
        let entries = self
            .records
            .iter()
            .filter(|record| {
                filter
                    .kind
                    .as_deref()
                    .is_none_or(|kind| record.kind == kind)
            })
            .filter(|record| {
                filter
                    .topic
                    .as_deref()
                    .is_none_or(|topic| record.topic() == topic)
            })
            .filter(|record| {
                filter.q.as_deref().is_none_or(|q| {
                    record.kind.to_ascii_lowercase().contains(q)
                        || record.topic().contains(q)
                        || record.resource.to_ascii_lowercase().contains(q)
                        || record
                            .detail
                            .as_deref()
                            .is_some_and(|detail| detail.to_ascii_lowercase().contains(q))
                })
            })
            .rev()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(EventRecord::controller_json)
            .collect::<Vec<_>>();
        serde_json::Value::Array(entries).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::EventStore;

    #[test]
    fn json_escapes_untrusted_event_kinds() {
        let kind = r#"custom\",\"injected\":true,\"ignored\":\""#;
        let mut store = EventStore::new(1);
        store.record(kind, "resource", None);

        let parsed: serde_json::Value =
            serde_json::from_str(&store.json(None)).expect("event records must be valid JSON");
        assert_eq!(parsed["entries"][0]["kind"], kind);
        assert!(parsed["entries"][0].get("injected").is_none());
    }
}
