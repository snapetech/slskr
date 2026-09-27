use super::{
    collection_owner_forbids, collection_reorder_exceeds_wire_limits, json_escape,
    json_usize_option, truncate_utf8_bytes, unix_seconds_rfc3339, unix_timestamp, AppState,
    RecordListFilter, MAX_COLLECTIONS, MAX_COLLECTION_ITEMS, MAX_LIST_ARTIST_BYTES,
    MAX_LIST_CONTENT_ID_BYTES, MAX_LIST_DESCRIPTION_BYTES, MAX_LIST_KIND_BYTES,
    MAX_LIST_NAME_BYTES, MAX_LIST_TITLE_BYTES, MAX_TOTAL_COLLECTION_ITEMS,
};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CollectionItem {
    pub(crate) id: String,
    pub(crate) content_id: String,
    pub(crate) artist: String,
    pub(crate) title: String,
    pub(crate) kind: String,
    pub(crate) file_name: String,
    pub(crate) album: String,
    pub(crate) content_hash: String,
    pub(crate) added_at: u64,
}

impl CollectionItem {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"contentId\":\"{}\",\"content_id\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"fileName\":\"{}\",\"mediaKind\":\"{}\",\"kind\":\"{}\",\"addedAt\":{},\"added_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.content_id),
            json_escape(&self.content_id),
            json_escape(&self.artist),
            json_escape(&self.title),
            json_escape(if self.file_name.is_empty() { &self.title } else { &self.file_name }),
            json_escape(&self.kind),
            json_escape(&self.kind),
            self.added_at,
            self.added_at
        )
    }

    pub(crate) fn native_json(&self, collection_id: &str, ordinal: usize) -> String {
        serde_json::json!({
            "id": self.id,
            "collectionId": collection_id,
            "ordinal": ordinal,
            "contentId": self.content_id,
            "mediaKind": (!self.kind.is_empty()).then_some(self.kind.as_str()),
            "fileName": (!self.file_name.is_empty()).then_some(self.file_name.as_str()),
            "title": (!self.title.is_empty()).then_some(self.title.as_str()),
            "artist": (!self.artist.is_empty()).then_some(self.artist.as_str()),
            "album": (!self.album.is_empty()).then_some(self.album.as_str()),
            "contentHash": (!self.content_hash.is_empty()).then_some(self.content_hash.as_str()),
        })
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CollectionRecord {
    pub(crate) id: String,
    pub(crate) owner_user_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) collection_type: String,
    pub(crate) items: Vec<CollectionItem>,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
}

impl CollectionRecord {
    pub(crate) fn json(&self) -> String {
        let items = self
            .items
            .iter()
            .map(CollectionItem::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"id\":\"{}\",\"title\":\"{}\",\"name\":\"{}\",\"type\":\"ShareList\",\"description\":\"{}\",\"items\":[{}],\"itemCount\":{},\"item_count\":{},\"createdAt\":{},\"created_at\":{},\"updatedAt\":{},\"updated_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.name),
            json_escape(&self.name),
            json_escape(&self.description),
            items,
            self.items.len(),
            self.items.len(),
            self.created_at,
            self.created_at,
            self.updated_at,
            self.updated_at
        )
    }

    pub(crate) fn native_json(&self) -> String {
        serde_json::json!({
            "id": self.id,
            "ownerUserId": self.owner_user_id,
            "title": self.name,
            "description": (!self.description.is_empty()).then_some(self.description.as_str()),
            "type": self.collection_type,
            "createdAt": unix_seconds_rfc3339(self.created_at),
            "updatedAt": unix_seconds_rfc3339(self.updated_at),
        })
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CollectionStore {
    pub(crate) records: Vec<CollectionRecord>,
    pub(crate) next_id: u64,
    pub(crate) next_item_id: u64,
    pub(crate) updated_at: u64,
    pub(crate) max_records: usize,
    pub(crate) max_items_per_collection: usize,
}

#[allow(dead_code)]
impl CollectionStore {
    pub(crate) fn new() -> Self {
        Self::with_limits(MAX_COLLECTIONS, MAX_COLLECTION_ITEMS)
    }

    pub(crate) fn with_limits(max_records: usize, max_items_per_collection: usize) -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            next_item_id: 1,
            updated_at: unix_timestamp(),
            max_records: max_records.max(1),
            max_items_per_collection: max_items_per_collection.max(1),
        }
    }

    pub(crate) fn from_persisted(
        collections: Vec<crate::persistence::CollectionRecord>,
        items: Vec<crate::persistence::CollectionItemRecord>,
    ) -> Self {
        let mut next_id = 1;
        let mut next_item_id = 1;
        let mut updated_at = unix_timestamp();
        let mut records = Vec::new();
        let mut retained_items = 0_usize;
        let mut seen_collection_ids = HashSet::new();
        for record in collections {
            if let Some(number) = record
                .id
                .strip_prefix("col-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                next_id = next_id.max(number.saturating_add(1));
            }
            if records.len() < MAX_COLLECTIONS && seen_collection_ids.insert(record.id.clone()) {
                let created_at = u64::try_from(record.created_at).unwrap_or(0);
                let record_updated_at = u64::try_from(record.updated_at).unwrap_or(created_at);
                updated_at = updated_at.max(record_updated_at);
                records.push(CollectionRecord {
                    id: record.id,
                    owner_user_id: record.owner_user_id,
                    name: truncate_utf8_bytes(record.name, MAX_LIST_NAME_BYTES),
                    description: truncate_utf8_bytes(
                        record.description,
                        MAX_LIST_DESCRIPTION_BYTES,
                    ),
                    collection_type: record.collection_type,
                    items: Vec::new(),
                    created_at,
                    updated_at: record_updated_at,
                });
            }
        }
        let mut items = items;
        items.sort_by_key(|item| (item.collection_id.clone(), item.position, item.added_at));
        let mut seen_item_ids = HashSet::new();
        for item in items {
            if let Some(number) = item
                .id
                .strip_prefix("item-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                next_item_id = next_item_id.max(number.saturating_add(1));
            }
            if !seen_item_ids.insert(item.id.clone()) {
                continue;
            }
            let Some(collection) = records
                .iter_mut()
                .find(|record| record.id == item.collection_id)
            else {
                continue;
            };
            if collection.items.len() >= MAX_COLLECTION_ITEMS
                || retained_items >= MAX_TOTAL_COLLECTION_ITEMS
            {
                continue;
            }
            collection
                .items
                .push(bounded_collection_item(CollectionItem {
                    id: item.id,
                    content_id: item.content_id,
                    artist: item.artist,
                    title: item.title,
                    kind: item.kind,
                    file_name: item.file_name,
                    album: item.album,
                    content_hash: item.content_hash,
                    added_at: u64::try_from(item.added_at).unwrap_or(0),
                }));
            retained_items = retained_items.saturating_add(1);
        }
        Self {
            records,
            next_id,
            next_item_id,
            updated_at,
            max_records: MAX_COLLECTIONS,
            max_items_per_collection: MAX_COLLECTION_ITEMS,
        }
    }

    pub(crate) fn create(
        &mut self,
        owner_user_id: String,
        name: String,
        description: String,
    ) -> Option<CollectionRecord> {
        let id = format!("col-{}", self.allocate_id());
        self.create_with_contract(id, owner_user_id, name, description, "ShareList".to_owned())
    }

    pub(crate) fn create_with_contract(
        &mut self,
        id: String,
        owner_user_id: String,
        name: String,
        description: String,
        collection_type: String,
    ) -> Option<CollectionRecord> {
        if self.records.len() >= self.max_records {
            return None;
        }
        let now = unix_timestamp();
        let record = CollectionRecord {
            id,
            owner_user_id: truncate_utf8_bytes(owner_user_id, MAX_LIST_NAME_BYTES),
            name: truncate_utf8_bytes(name, MAX_LIST_NAME_BYTES),
            description: truncate_utf8_bytes(description, MAX_LIST_DESCRIPTION_BYTES),
            collection_type,
            items: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn get(&self, id: &str) -> Option<CollectionRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    pub(crate) fn update(
        &mut self,
        id: &str,
        name: String,
        description: String,
    ) -> Option<CollectionRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.name = truncate_utf8_bytes(name, MAX_LIST_NAME_BYTES);
        record.description = truncate_utf8_bytes(description, MAX_LIST_DESCRIPTION_BYTES);
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    pub(crate) fn update_contract(
        &mut self,
        id: &str,
        name: Option<String>,
        description: Option<String>,
        collection_type: Option<String>,
    ) -> Option<CollectionRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|record| record.id == id)?;
        if let Some(name) = name {
            record.name = truncate_utf8_bytes(name, MAX_LIST_NAME_BYTES);
        }
        if let Some(description) = description {
            record.description = truncate_utf8_bytes(description, MAX_LIST_DESCRIPTION_BYTES);
        }
        if let Some(collection_type) = collection_type {
            record.collection_type = collection_type;
        }
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

    #[cfg(any(test, feature = "bounded-differential"))]
    pub(crate) fn add_item(
        &mut self,
        collection_id: &str,
        content_id: String,
        artist: String,
        title: String,
        kind: String,
    ) -> Result<Option<CollectionItem>, ()> {
        self.add_item_with_contract(
            collection_id,
            None,
            content_id,
            artist,
            title,
            kind,
            String::new(),
            String::new(),
            String::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_item_with_contract(
        &mut self,
        collection_id: &str,
        id: Option<String>,
        content_id: String,
        artist: String,
        title: String,
        kind: String,
        file_name: String,
        album: String,
        content_hash: String,
    ) -> Result<Option<CollectionItem>, ()> {
        let now = unix_timestamp();
        let Some(index) = self.records.iter().position(|r| r.id == collection_id) else {
            return Ok(None);
        };
        if self.records[index].items.len() >= self.max_items_per_collection
            || self.total_items() >= MAX_TOTAL_COLLECTION_ITEMS
        {
            return Err(());
        }
        let id = id.unwrap_or_else(|| format!("item-{}", self.allocate_item_id()));
        let item = bounded_collection_item(CollectionItem {
            id,
            content_id,
            artist,
            title,
            kind,
            file_name,
            album,
            content_hash,
            added_at: now,
        });
        let record = &mut self.records[index];
        record.items.push(item.clone());
        record.updated_at = now;
        self.updated_at = now;
        Ok(Some(item))
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            let id = format!("col-{candidate}");
            if !self.records.iter().any(|record| record.id == id) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded collection store must leave an available u64 id")
    }

    pub(crate) fn allocate_item_id(&mut self) -> u64 {
        let retained_items = self
            .records
            .iter()
            .map(|record| record.items.len())
            .sum::<usize>();
        let mut candidate = self.next_item_id.max(1);
        for _ in 0..=retained_items {
            let id = format!("item-{candidate}");
            if !self
                .records
                .iter()
                .flat_map(|record| &record.items)
                .any(|item| item.id == id)
            {
                self.next_item_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded collection items must leave an available u64 id")
    }

    pub(crate) fn update_item(
        &mut self,
        item_id: &str,
        artist: Option<String>,
        title: Option<String>,
        kind: Option<String>,
    ) -> Option<CollectionItem> {
        let now = unix_timestamp();
        for record in &mut self.records {
            if let Some(item) = record.items.iter_mut().find(|item| item.id == item_id) {
                if let Some(artist) = artist {
                    item.artist = truncate_utf8_bytes(artist, MAX_LIST_ARTIST_BYTES);
                }
                if let Some(title) = title {
                    item.title = truncate_utf8_bytes(title, MAX_LIST_TITLE_BYTES);
                }
                if let Some(kind) = kind {
                    item.kind = truncate_utf8_bytes(kind, MAX_LIST_KIND_BYTES);
                }
                record.updated_at = now;
                self.updated_at = now;
                return Some(item.clone());
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_item_contract(
        &mut self,
        item_id: &str,
        content_id: Option<String>,
        artist: Option<String>,
        title: Option<String>,
        kind: Option<String>,
        file_name: Option<String>,
        album: Option<String>,
        content_hash: Option<String>,
    ) -> Option<CollectionItem> {
        let now = unix_timestamp();
        for record in &mut self.records {
            if let Some(item) = record.items.iter_mut().find(|item| item.id == item_id) {
                if let Some(content_id) = content_id {
                    if content_id.trim().is_empty() {
                        return None;
                    }
                    item.content_id = truncate_utf8_bytes(content_id, MAX_LIST_CONTENT_ID_BYTES);
                }
                if let Some(artist) = artist {
                    item.artist = truncate_utf8_bytes(artist, MAX_LIST_ARTIST_BYTES);
                }
                if let Some(title) = title {
                    item.title = truncate_utf8_bytes(title, MAX_LIST_TITLE_BYTES);
                }
                if let Some(kind) = kind {
                    item.kind = truncate_utf8_bytes(kind, MAX_LIST_KIND_BYTES);
                }
                if let Some(file_name) = file_name {
                    item.file_name = truncate_utf8_bytes(file_name, MAX_LIST_TITLE_BYTES);
                }
                if let Some(album) = album {
                    item.album = truncate_utf8_bytes(album, MAX_LIST_TITLE_BYTES);
                }
                if let Some(content_hash) = content_hash {
                    item.content_hash =
                        truncate_utf8_bytes(content_hash, MAX_LIST_CONTENT_ID_BYTES);
                }
                record.updated_at = now;
                self.updated_at = now;
                return Some(item.clone());
            }
        }
        None
    }

    pub(crate) fn remove_item(&mut self, item_id: &str) -> Option<CollectionItem> {
        let now = unix_timestamp();
        for record in &mut self.records {
            if let Some(pos) = record.items.iter().position(|item| item.id == item_id) {
                let item = record.items.remove(pos);
                record.updated_at = now;
                self.updated_at = now;
                return Some(item);
            }
        }
        None
    }

    pub(crate) fn collection_id_for_item(&self, item_id: &str) -> Option<String> {
        self.records
            .iter()
            .find(|record| record.items.iter().any(|item| item.id == item_id))
            .map(|record| record.id.clone())
    }

    pub(crate) fn total_items(&self) -> usize {
        self.records.iter().map(|record| record.items.len()).sum()
    }

    pub(crate) fn reorder_items(
        &mut self,
        collection_id: &str,
        body: &str,
    ) -> Option<CollectionRecord> {
        if collection_reorder_exceeds_wire_limits(body) {
            return None;
        }
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == collection_id)?;
        let payload = serde_json::from_str::<serde_json::Value>(body).ok();
        let ids = payload
            .as_ref()
            .and_then(|value| {
                value
                    .get("item_ids")
                    .or_else(|| value.get("itemIds"))
                    .or_else(|| value.get("items"))
            })
            .and_then(serde_json::Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !ids.is_empty() {
            let mut ordered = Vec::new();
            for id in &ids {
                if let Some(pos) = record.items.iter().position(|item| item.id == *id) {
                    ordered.push(record.items.remove(pos));
                }
            }
            ordered.append(&mut record.items);
            record.items = ordered;
        }
        record.updated_at = now;
        self.updated_at = now;
        Some(record.clone())
    }

    #[allow(dead_code)]
    fn json(&self, query: Option<&str>) -> String {
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
            self.json_array(query, None),
            self.records.len(),
            filtered_count,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    /// `caller_id` matches the oracle's real `GetCollectionsByOwnerAsync`
    /// scoping: when a real per-caller identity is resolvable, only that
    /// caller's own collections are visible. `None` (no resolvable
    /// identity, e.g. the common single-operator `auth_required=false`
    /// deployment) preserves the unrestricted behavior every collection
    /// visible to every caller.
    pub(crate) fn json_array(&self, query: Option<&str>, caller_id: Option<&str>) -> String {
        let filter = RecordListFilter::from_query(query);
        let records = self
            .records
            .iter()
            .filter(|record| !collection_owner_forbids(caller_id, &record.owner_user_id))
            .filter(|record| {
                filter
                    .q
                    .as_deref()
                    .is_none_or(|q| record.name.to_ascii_lowercase().contains(q))
            })
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(CollectionRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

pub(super) async fn persist_collection_checked(
    state: &AppState,
    record: &CollectionRecord,
) -> Result<bool, String> {
    persist_collection_snapshot(state, record, false).await
}

pub(super) async fn persist_collection_created(
    state: &AppState,
    record: &CollectionRecord,
) -> Result<bool, String> {
    persist_collection_snapshot(state, record, true).await
}

async fn persist_collection_snapshot(
    state: &AppState,
    record: &CollectionRecord,
    allow_create: bool,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::CollectionRecord {
        id: record.id.clone(),
        owner_user_id: record.owner_user_id.clone(),
        name: record.name.clone(),
        description: record.description.clone(),
        collection_type: record.collection_type.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        updated_at: i64::try_from(record.updated_at).unwrap_or(i64::MAX),
    };
    let items = record
        .items
        .iter()
        .enumerate()
        .map(
            |(position, item)| crate::persistence::CollectionItemRecord {
                id: item.id.clone(),
                collection_id: record.id.clone(),
                content_id: item.content_id.clone(),
                artist: item.artist.clone(),
                title: item.title.clone(),
                kind: item.kind.clone(),
                file_name: item.file_name.clone(),
                album: item.album.clone(),
                content_hash: item.content_hash.clone(),
                added_at: i64::try_from(item.added_at).unwrap_or(i64::MAX),
                position: i64::try_from(position).unwrap_or(i64::MAX),
            },
        )
        .collect::<Vec<_>>();
    let result = if allow_create {
        db.replace_collection(&persisted, &items).await
    } else {
        db.replace_existing_collection(&persisted, &items).await
    };
    result.map_err(|error| format!("collection persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_collection_delete(state: &AppState, id: &str) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_collection(id)
        .await
        .map_err(|error| format!("collection deletion persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_collections_if_unchanged(
    state: &AppState,
    previous: CollectionStore,
    mutated: &CollectionStore,
) {
    let mut collections = state.collections.write().await;
    if *collections == *mutated {
        *collections = previous;
    }
}

fn bounded_collection_item(mut item: CollectionItem) -> CollectionItem {
    item.content_id = truncate_utf8_bytes(item.content_id, MAX_LIST_CONTENT_ID_BYTES);
    item.artist = truncate_utf8_bytes(item.artist, MAX_LIST_ARTIST_BYTES);
    item.title = truncate_utf8_bytes(item.title, MAX_LIST_TITLE_BYTES);
    item.kind = truncate_utf8_bytes(item.kind, MAX_LIST_KIND_BYTES);
    item.file_name = truncate_utf8_bytes(item.file_name, MAX_LIST_TITLE_BYTES);
    item.album = truncate_utf8_bytes(item.album, MAX_LIST_TITLE_BYTES);
    item.content_hash = truncate_utf8_bytes(item.content_hash, MAX_LIST_CONTENT_ID_BYTES);
    item
}
