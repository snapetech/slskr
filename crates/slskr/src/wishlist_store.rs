use super::{
    file_attribute_value, truncate_utf8_bytes, unix_seconds_rfc3339, unix_timestamp,
    virtual_basename, FileEntry, SearchRecord, SearchResultEntry, MAX_LIST_ARTIST_BYTES,
    MAX_LIST_KIND_BYTES, MAX_LIST_TITLE_BYTES, MAX_SEARCH_RESULT_FILENAME_BYTES,
    MAX_SEARCH_RESULT_USERNAME_BYTES, MAX_SEARCH_TARGET_NAME_BYTES, MAX_WISHLIST_DOWNLOADS,
    MAX_WISHLIST_FILTER_BYTES, MAX_WISHLIST_IGNORED_RESULTS, MAX_WISHLIST_IGNORED_RESULTS_PER_ITEM,
    MAX_WISHLIST_ITEMS, MAX_WISHLIST_RESULTS,
};
#[cfg(test)]
use super::{
    is_legacy_wishlist_item_id, normalize_api_path, versioned_wishlist_invalid_id_response,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

// Wishlist Models
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistItem {
    pub(crate) id: String,
    pub(crate) artist: String,
    pub(crate) title: String,
    pub(crate) kind: String,
    pub(crate) filter: String,
    pub(crate) enabled: bool,
    pub(crate) auto_download: bool,
    pub(crate) max_results: usize,
    pub(crate) max_downloads: Option<u64>,
    pub(crate) last_viewed_at: Option<u64>,
    pub(crate) last_searched_at: Option<u64>,
    pub(crate) last_match_count: usize,
    pub(crate) last_visible_hit_count: usize,
    pub(crate) last_hidden_locked_hit_count: usize,
    pub(crate) last_filtered_out_hit_count: usize,
    pub(crate) last_ignored_result_hit_count: usize,
    pub(crate) last_response_count: usize,
    pub(crate) total_search_count: u64,
    pub(crate) total_download_count: u64,
    pub(crate) last_search_id: Option<String>,
    pub(crate) lidarr_album_id: Option<i64>,
    pub(crate) lidarr_track_id: Option<i64>,
    pub(crate) lidarr_track_count: Option<i64>,
    pub(crate) lidarr_duration_seconds: Option<i64>,
    pub(crate) lidarr_release_disambiguation: Option<String>,
    pub(crate) added_at: u64,
}

/// The native controller exposes wishlist identifiers as GUIDs.  Older
/// slskR databases contain the legacy `wish-N` identifiers, so project those
/// identifiers into a deterministic GUID without changing the storage key.
/// Keeping the storage key stable preserves searches, ignored-result rules,
/// and transfer history for installations that switch controller profiles.
pub(crate) fn native_wishlist_item_id(id: &str) -> String {
    if uuid::Uuid::parse_str(id).is_ok() {
        return id.to_owned();
    }
    let mut bytes: [u8; 16] = Sha256::digest(format!("slskr:native-wishlist:{id}").as_bytes())
        [..16]
        .try_into()
        .expect("SHA-256 prefix is sixteen bytes");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

#[cfg(test)]
mod native_wishlist_tests {
    use super::*;

    #[test]
    fn legacy_wishlist_ids_have_a_stable_native_projection_and_reverse_lookup() {
        let mut wishlist = WishlistStore::new();
        let item = wishlist
            .add_item(
                "Legacy Artist".to_owned(),
                "Legacy Album".to_owned(),
                "Audio".to_owned(),
            )
            .expect("legacy wishlist item");
        let (ignored, created) = wishlist
            .ignore_result(&item.id, "legacy-peer", "Remote/Album", false)
            .expect("legacy ignored result");
        assert!(created);

        let native_id = native_wishlist_item_id(&item.id);
        assert_eq!(native_id, native_wishlist_item_id(&item.id));
        assert_ne!(native_id, item.id);
        assert!(uuid::Uuid::parse_str(&native_id).is_ok());
        assert_eq!(
            wishlist.resolve_item_id(&native_id, true),
            Some(item.id.clone())
        );
        assert_eq!(wishlist.resolve_item_id(&native_id, false), None);
        assert_eq!(normalize_api_path("/api/v0/dht/peers"), "/api/dht/peers");
        assert!(is_legacy_wishlist_item_id(&item.id));
        assert!(versioned_wishlist_invalid_id_response(
            "GET",
            &format!("/api/v0/wishlist/{}/searches", item.id),
        )
        .is_none());
        assert!(versioned_wishlist_invalid_id_response(
            "GET",
            "/api/v0/wishlist/not-a-guid/searches",
        )
        .is_some());

        let native_item = serde_json::from_str::<serde_json::Value>(&item.native_json())
            .expect("native wishlist item JSON");
        assert_eq!(native_item["id"], native_id);
        let native_ignored = ignored.native_json();
        assert_eq!(native_ignored["wishlistItemId"], native_id);
    }
}

impl WishlistItem {
    pub(crate) fn search_text(&self) -> String {
        match (self.artist.trim().is_empty(), self.title.trim().is_empty()) {
            (true, true) => String::new(),
            (true, false) => self.title.clone(),
            (false, true) => self.artist.clone(),
            (false, false) => format!("{} {}", self.artist, self.title),
        }
    }

    pub(crate) fn json(&self) -> String {
        let search_text = self.search_text();
        serde_json::json!({
            "id": self.id,
            "artist": self.artist,
            "title": self.title,
            "kind": self.kind,
            "added_at": self.added_at,
            "createdAt": self.added_at,
            "searchText": search_text,
            "filter": self.filter,
            "enabled": self.enabled,
            "autoDownload": self.auto_download,
            "maxResults": self.max_results,
            "maxDownloads": self.max_downloads,
            "lastViewedAt": self.last_viewed_at,
            "lastSearchedAt": self.last_searched_at,
            "lastMatchCount": self.last_match_count,
            "lastVisibleHitCount": self.last_visible_hit_count,
            "lastHiddenLockedHitCount": self.last_hidden_locked_hit_count,
            "lastFilteredOutHitCount": self.last_filtered_out_hit_count,
            "lastIgnoredResultHitCount": self.last_ignored_result_hit_count,
            "lastResponseCount": self.last_response_count,
            "totalSearchCount": self.total_search_count,
            "totalDownloadCount": self.total_download_count,
            "lastSearchId": self.last_search_id,
            "lidarrAlbumId": self.lidarr_album_id,
            "lidarrTrackId": self.lidarr_track_id,
            "lidarrTrackCount": self.lidarr_track_count,
            "lidarrDurationSeconds": self.lidarr_duration_seconds,
            "lidarrReleaseDisambiguation": self.lidarr_release_disambiguation,
        })
        .to_string()
    }

    pub(crate) fn native_json(&self) -> String {
        serde_json::json!({
            "id": native_wishlist_item_id(&self.id),
            "searchText": self.search_text(),
            "filter": self.filter,
            "enabled": self.enabled,
            "autoDownload": self.auto_download,
            "maxResults": self.max_results,
            "createdAt": unix_seconds_rfc3339(self.added_at),
            "lastSearchedAt": self.last_searched_at.map(unix_seconds_rfc3339),
            "lastMatchCount": self.last_match_count,
            "lastVisibleHitCount": self.last_visible_hit_count,
            "lastHiddenLockedHitCount": self.last_hidden_locked_hit_count,
            "lastFilteredOutHitCount": self.last_filtered_out_hit_count,
            "lastIgnoredResultHitCount": self.last_ignored_result_hit_count,
            "lastResponseCount": self.last_response_count,
            "totalSearchCount": self.total_search_count,
            "totalDownloadCount": self.total_download_count,
            "maxDownloads": self.max_downloads,
            "lastSearchId": self.last_search_id,
            "lastViewedAt": self.last_viewed_at.map(unix_seconds_rfc3339),
            "lidarrAlbumId": self.lidarr_album_id,
            "lidarrTrackId": self.lidarr_track_id,
            "lidarrTrackCount": self.lidarr_track_count,
            "lidarrDurationSeconds": self.lidarr_duration_seconds,
            "lidarrReleaseDisambiguation": self.lidarr_release_disambiguation,
        })
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistRecord {
    pub(crate) id: String,
    pub(crate) items: Vec<WishlistItem>,
    pub(crate) updated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistIgnoredResult {
    pub(crate) id: String,
    pub(crate) wishlist_item_id: String,
    pub(crate) username: String,
    pub(crate) directory: String,
    pub(crate) created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistResultFilter {
    pub(crate) clauses: Vec<WishlistFilterClause>,
    pub(crate) exclude: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistFilterClause {
    pub(crate) include: Vec<String>,
    pub(crate) minimum_bitrate: Option<u32>,
}

impl WishlistResultFilter {
    pub(crate) fn parse(filter: &str) -> Self {
        let mut terms = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        for character in filter.chars() {
            match character {
                '"' => quoted = !quoted,
                character if character.is_whitespace() && !quoted => {
                    if !current.is_empty() {
                        terms.push(std::mem::take(&mut current));
                    }
                }
                character => current.push(character),
            }
        }
        if !current.is_empty() {
            terms.push(current);
        }

        let mut clauses = Vec::new();
        let mut include = Vec::new();
        let mut exclude = Vec::new();
        let mut minimum_bitrate = None;
        for raw_term in terms {
            if raw_term.eq_ignore_ascii_case("OR") {
                if !include.is_empty() || minimum_bitrate.is_some() {
                    clauses.push(WishlistFilterClause {
                        include: std::mem::take(&mut include),
                        minimum_bitrate: minimum_bitrate.take(),
                    });
                }
                continue;
            }
            let (target, term) = if let Some(term) = raw_term.strip_prefix('-') {
                (&mut exclude, term)
            } else {
                let normalized_term = raw_term.to_ascii_lowercase();
                if let Some(value) = normalized_term
                    .strip_prefix("minbr:")
                    .or_else(|| normalized_term.strip_prefix("minbitrate:"))
                    .and_then(|value| value.parse::<u32>().ok())
                    .filter(|value| *value > 0)
                {
                    minimum_bitrate = Some(minimum_bitrate.unwrap_or(0).max(value));
                    continue;
                }
                (&mut include, raw_term.as_str())
            };
            let term = term
                .trim()
                .trim_matches('"')
                .trim_start_matches('.')
                .to_ascii_lowercase();
            if !term.is_empty() && !target.contains(&term) {
                target.push(term);
            }
        }
        if !include.is_empty() || minimum_bitrate.is_some() {
            clauses.push(WishlistFilterClause {
                include,
                minimum_bitrate,
            });
        }
        if clauses.is_empty() {
            clauses.push(WishlistFilterClause {
                include: Vec::new(),
                minimum_bitrate: None,
            });
        }
        Self { clauses, exclude }
    }

    pub(crate) fn matches_entry(&self, entry: &SearchResultEntry) -> bool {
        self.matches_with_bitrate(&entry.filename, entry.bit_rate)
    }

    pub(crate) fn matches_file_entry(&self, entry: &FileEntry) -> bool {
        self.matches_with_bitrate(&entry.filename, file_attribute_value(entry, 0))
    }

    #[allow(dead_code)]
    pub(crate) fn matches(&self, filename: &str) -> bool {
        self.matches_with_bitrate(filename, None)
    }

    fn matches_with_bitrate(&self, filename: &str, bit_rate: Option<u32>) -> bool {
        let filename = filename.replace('\\', "/").to_ascii_lowercase();
        let extension = virtual_basename(&filename)
            .rsplit_once('.')
            .map(|(_, extension)| extension)
            .unwrap_or_default();
        if self.exclude.iter().any(|term| filename.contains(term)) {
            return false;
        }
        self.clauses.iter().any(|clause| {
            let filename_match = clause.include.is_empty()
                || clause
                    .include
                    .iter()
                    .any(|term| extension == term || filename.contains(term));
            filename_match
                && clause
                    .minimum_bitrate
                    .is_none_or(|minimum| bit_rate.is_some_and(|value| value >= minimum))
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistResultPolicy {
    pub(crate) filter: WishlistResultFilter,
    pub(crate) max_results: usize,
}

impl WishlistIgnoredResult {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "wishlistItemId": self.wishlist_item_id,
            "username": self.username,
            "directory": self.directory,
            "createdAt": self.created_at,
        })
    }

    pub(crate) fn native_json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "wishlistItemId": native_wishlist_item_id(&self.wishlist_item_id),
            "username": self.username,
            "directory": self.directory,
            "createdAt": unix_seconds_rfc3339(self.created_at),
        })
    }

    pub(crate) fn matches(&self, username: &str, filename: &str) -> bool {
        self.username.eq_ignore_ascii_case(username)
            && normalize_wishlist_directory(&self.directory)
                .eq_ignore_ascii_case(&wishlist_parent_directory(filename))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WishlistStore {
    pub(crate) records: Vec<WishlistRecord>,
    pub(crate) ignored_results: Vec<WishlistIgnoredResult>,
    pub(crate) next_item_id: u64,
    pub(crate) updated_at: u64,
    pub(crate) max_items: usize,
}

impl WishlistStore {
    pub(crate) fn new() -> Self {
        Self::with_max_items(MAX_WISHLIST_ITEMS)
    }

    pub(crate) fn with_max_items(max_items: usize) -> Self {
        Self {
            records: Vec::new(),
            ignored_results: Vec::new(),
            next_item_id: 1,
            updated_at: unix_timestamp(),
            max_items: max_items.max(1),
        }
    }

    pub(crate) fn from_persisted_with_ignored(
        records: Vec<crate::persistence::WishlistItemRecord>,
        ignored_results: Vec<crate::persistence::WishlistIgnoredResultRecord>,
    ) -> Self {
        let mut store = Self::new();
        let mut items = Vec::new();
        let mut seen_ids = HashSet::new();
        for record in records {
            if let Some(number) = record
                .id
                .strip_prefix("wish-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                store.next_item_id = store.next_item_id.max(number.saturating_add(1));
            }
            if items.len() >= store.max_items || !seen_ids.insert(record.id.clone()) {
                continue;
            }
            items.push(WishlistItem {
                id: record.id,
                artist: truncate_utf8_bytes(record.artist, MAX_LIST_ARTIST_BYTES),
                title: truncate_utf8_bytes(record.title, MAX_LIST_TITLE_BYTES),
                kind: truncate_utf8_bytes(record.kind, MAX_LIST_KIND_BYTES),
                filter: truncate_utf8_bytes(record.filter, MAX_WISHLIST_FILTER_BYTES),
                enabled: record.enabled,
                auto_download: record.auto_download,
                max_results: usize::try_from(record.max_results)
                    .unwrap_or(100)
                    .clamp(1, MAX_WISHLIST_RESULTS),
                max_downloads: record
                    .max_downloads
                    .and_then(|value| u64::try_from(value).ok())
                    .filter(|value| *value > 0)
                    .map(|value| value.min(MAX_WISHLIST_DOWNLOADS)),
                last_viewed_at: record
                    .last_viewed_at
                    .and_then(|value| u64::try_from(value).ok()),
                last_searched_at: record
                    .last_searched_at
                    .and_then(|value| u64::try_from(value).ok()),
                last_match_count: usize::try_from(record.last_match_count).unwrap_or(0),
                last_visible_hit_count: usize::try_from(record.last_visible_hit_count).unwrap_or(0),
                last_hidden_locked_hit_count: usize::try_from(record.last_hidden_locked_hit_count)
                    .unwrap_or(0),
                last_filtered_out_hit_count: usize::try_from(record.last_filtered_out_hit_count)
                    .unwrap_or(0),
                last_ignored_result_hit_count: usize::try_from(
                    record.last_ignored_result_hit_count,
                )
                .unwrap_or(0),
                last_response_count: usize::try_from(record.last_response_count).unwrap_or(0),
                total_search_count: u64::try_from(record.total_search_count).unwrap_or(0),
                total_download_count: u64::try_from(record.total_download_count).unwrap_or(0),
                last_search_id: record
                    .last_search_id
                    .map(|id| truncate_utf8_bytes(id, MAX_SEARCH_TARGET_NAME_BYTES)),
                lidarr_album_id: record.lidarr_album_id,
                lidarr_track_id: record.lidarr_track_id,
                lidarr_track_count: record.lidarr_track_count,
                lidarr_duration_seconds: record.lidarr_duration_seconds,
                lidarr_release_disambiguation: record
                    .lidarr_release_disambiguation
                    .map(|value| truncate_utf8_bytes(value, MAX_LIST_TITLE_BYTES)),
                added_at: u64::try_from(record.added_at).unwrap_or(0),
            });
        }
        items.sort_by_key(|item| item.added_at);
        let updated_at = items
            .iter()
            .map(|item| item.added_at)
            .max()
            .unwrap_or_else(unix_timestamp);
        store.records.push(WishlistRecord {
            id: "default".to_owned(),
            items,
            updated_at,
        });
        let known_items = store
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .map(|item| item.id.as_str())
            .collect::<HashSet<_>>();
        let mut seen = HashSet::new();
        store.ignored_results = ignored_results
            .into_iter()
            .filter(|rule| known_items.contains(rule.wishlist_item_id.as_str()))
            .filter_map(|rule| {
                let username = truncate_utf8_bytes(
                    rule.username.trim().to_owned(),
                    MAX_SEARCH_RESULT_USERNAME_BYTES,
                );
                let directory = normalize_wishlist_directory(&rule.directory);
                let key = format!(
                    "{}\0{}\0{}",
                    rule.wishlist_item_id.to_lowercase(),
                    username.to_lowercase(),
                    directory.to_lowercase()
                );
                (!username.is_empty() && !directory.is_empty() && seen.insert(key)).then_some(
                    WishlistIgnoredResult {
                        id: rule.id,
                        wishlist_item_id: rule.wishlist_item_id,
                        username,
                        directory,
                        created_at: u64::try_from(rule.created_at).unwrap_or(0),
                    },
                )
            })
            .take(MAX_WISHLIST_IGNORED_RESULTS)
            .collect();
        store.updated_at = updated_at;
        store
    }

    pub(crate) fn get_or_create(&mut self) -> WishlistRecord {
        let now = unix_timestamp();
        if let Some(record) = self.records.iter_mut().find(|r| r.id == "default") {
            record.clone()
        } else {
            let record = WishlistRecord {
                id: "default".to_string(),
                items: Vec::new(),
                updated_at: now,
            };
            self.records.push(record.clone());
            self.updated_at = now;
            record
        }
    }

    pub(crate) fn add_item(
        &mut self,
        artist: String,
        title: String,
        kind: String,
    ) -> Result<WishlistItem, ()> {
        self.add_item_with_settings(artist, title, kind, String::new(), true, false, 100, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_item_with_settings(
        &mut self,
        artist: String,
        title: String,
        kind: String,
        filter: String,
        enabled: bool,
        auto_download: bool,
        max_results: usize,
        max_downloads: Option<u64>,
    ) -> Result<WishlistItem, ()> {
        self.add_item_with_contract(
            None,
            artist,
            title,
            kind,
            filter,
            enabled,
            auto_download,
            max_results,
            max_downloads,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_item_with_contract(
        &mut self,
        id: Option<String>,
        artist: String,
        title: String,
        kind: String,
        filter: String,
        enabled: bool,
        auto_download: bool,
        max_results: usize,
        max_downloads: Option<u64>,
    ) -> Result<WishlistItem, ()> {
        let now = unix_timestamp();
        self.get_or_create();
        let index = self
            .records
            .iter()
            .position(|record| record.id == "default")
            .ok_or(())?;
        if self.records[index].items.len() >= self.max_items {
            return Err(());
        }
        let id = id.unwrap_or_else(|| format!("wish-{}", self.allocate_item_id()));
        let item = WishlistItem {
            id,
            artist: truncate_utf8_bytes(artist, MAX_LIST_ARTIST_BYTES),
            title: truncate_utf8_bytes(title, MAX_LIST_TITLE_BYTES),
            kind: truncate_utf8_bytes(kind, MAX_LIST_KIND_BYTES),
            filter: truncate_utf8_bytes(filter.trim().to_owned(), MAX_WISHLIST_FILTER_BYTES),
            enabled,
            auto_download,
            max_results: max_results.clamp(1, MAX_WISHLIST_RESULTS),
            max_downloads: max_downloads.map(|value| value.min(MAX_WISHLIST_DOWNLOADS)),
            last_viewed_at: None,
            last_searched_at: None,
            last_match_count: 0,
            last_visible_hit_count: 0,
            last_hidden_locked_hit_count: 0,
            last_filtered_out_hit_count: 0,
            last_ignored_result_hit_count: 0,
            last_response_count: 0,
            total_search_count: 0,
            total_download_count: 0,
            last_search_id: None,
            lidarr_album_id: None,
            lidarr_track_id: None,
            lidarr_track_count: None,
            lidarr_duration_seconds: None,
            lidarr_release_disambiguation: None,
            added_at: now,
        };
        let record = &mut self.records[index];
        record.items.push(item.clone());
        record.updated_at = now;
        self.updated_at = now;
        Ok(item)
    }

    pub(crate) fn allocate_item_id(&mut self) -> u64 {
        let retained_items = self
            .records
            .iter()
            .map(|record| record.items.len())
            .sum::<usize>();
        let mut candidate = self.next_item_id.max(1);
        for _ in 0..=retained_items {
            let id = format!("wish-{candidate}");
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
        unreachable!("bounded wishlist store must leave an available u64 id")
    }

    pub(crate) fn remaining_capacity(&mut self) -> usize {
        self.get_or_create();
        self.records
            .iter()
            .find(|record| record.id == "default")
            .map(|record| self.max_items.saturating_sub(record.items.len()))
            .unwrap_or(0)
    }

    pub(crate) fn can_add_items(&mut self, count: usize) -> bool {
        count <= self.remaining_capacity()
    }

    pub(crate) fn remove_item(&mut self, item_id: &str) -> Option<WishlistRecord> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == "default")?;
        if let Some(pos) = record.items.iter().position(|i| i.id == item_id) {
            record.items.remove(pos);
            self.ignored_results
                .retain(|rule| rule.wishlist_item_id != item_id);
            record.updated_at = now;
            self.updated_at = now;
            Some(record.clone())
        } else {
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_item(
        &mut self,
        item_id: &str,
        artist: Option<String>,
        title: Option<String>,
        kind: Option<String>,
        filter: Option<String>,
        enabled: Option<bool>,
        auto_download: Option<bool>,
        max_results: Option<usize>,
        max_downloads: Option<Option<u64>>,
    ) -> Option<WishlistItem> {
        let now = unix_timestamp();
        let record = self.records.iter_mut().find(|r| r.id == "default")?;
        let item = record.items.iter_mut().find(|item| item.id == item_id)?;
        if let Some(artist) = artist {
            item.artist = truncate_utf8_bytes(artist, MAX_LIST_ARTIST_BYTES);
        }
        if let Some(title) = title {
            item.title = truncate_utf8_bytes(title, MAX_LIST_TITLE_BYTES);
        }
        if let Some(kind) = kind {
            item.kind = truncate_utf8_bytes(kind, MAX_LIST_KIND_BYTES);
        }
        if let Some(filter) = filter {
            item.filter = truncate_utf8_bytes(filter.trim().to_owned(), MAX_WISHLIST_FILTER_BYTES);
        }
        if let Some(enabled) = enabled {
            item.enabled = enabled;
        }
        if let Some(auto_download) = auto_download {
            item.auto_download = auto_download;
        }
        if let Some(max_results) = max_results {
            item.max_results = max_results.clamp(1, MAX_WISHLIST_RESULTS);
        }
        if let Some(max_downloads) = max_downloads {
            item.max_downloads = max_downloads.map(|value| value.min(MAX_WISHLIST_DOWNLOADS));
        }
        record.updated_at = now;
        self.updated_at = now;
        Some(item.clone())
    }

    /// Update one filter across a set of wishlist items as a single in-memory
    /// mutation.  The existence check intentionally happens before any item
    /// is changed so the API can preserve the current target's all-or-nothing
    /// behavior when one requested id is stale.
    pub(crate) fn update_filters(
        &mut self,
        item_ids: &[String],
        filter: String,
    ) -> Result<Vec<WishlistItem>, &'static str> {
        let mut distinct_ids = Vec::new();
        for item_id in item_ids {
            let item_id = item_id.trim();
            if item_id.is_empty() {
                return Err("At least one wishlist item ID is required");
            }
            if !distinct_ids.iter().any(|known| known == item_id) {
                distinct_ids.push(item_id.to_owned());
            }
        }
        if distinct_ids.is_empty() {
            return Err("At least one wishlist item ID is required");
        }

        let known_ids = self
            .records
            .iter()
            .flat_map(|record| record.items.iter())
            .map(|item| item.id.as_str())
            .collect::<HashSet<_>>();
        if distinct_ids
            .iter()
            .any(|item_id| !known_ids.contains(item_id.as_str()))
        {
            return Err("wishlist item not found");
        }

        let now = unix_timestamp();
        let filter = truncate_utf8_bytes(filter.trim().to_owned(), MAX_WISHLIST_FILTER_BYTES);
        let mut updated = Vec::with_capacity(distinct_ids.len());
        for record in &mut self.records {
            for item in &mut record.items {
                if distinct_ids.iter().any(|item_id| item_id == &item.id) {
                    item.filter = filter.clone();
                    updated.push(item.clone());
                }
            }
            if record
                .items
                .iter()
                .any(|item| distinct_ids.iter().any(|item_id| item_id == &item.id))
            {
                record.updated_at = now;
            }
        }
        self.updated_at = now;
        Ok(updated)
    }

    pub(crate) fn update_lidarr_metadata(
        &mut self,
        item_id: &str,
        album_id: Option<i64>,
        track_id: Option<i64>,
        track_count: Option<i64>,
        duration_seconds: Option<i64>,
        release_disambiguation: Option<String>,
    ) -> Option<WishlistItem> {
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == "default")?;
        let item = record.items.iter_mut().find(|item| item.id == item_id)?;
        item.lidarr_album_id = album_id;
        item.lidarr_track_id = track_id;
        item.lidarr_track_count = track_count;
        item.lidarr_duration_seconds = duration_seconds;
        item.lidarr_release_disambiguation = release_disambiguation
            .map(|value| truncate_utf8_bytes(value.trim().to_owned(), MAX_LIST_TITLE_BYTES))
            .filter(|value| !value.is_empty());
        record.updated_at = now;
        self.updated_at = now;
        Some(item.clone())
    }

    pub(crate) fn disable_lidarr_items_for_album(
        &mut self,
        album_id: i64,
        keep_track_ids: &HashSet<i64>,
    ) -> Vec<WishlistItem> {
        let now = unix_timestamp();
        let mut changed = Vec::new();
        for record in &mut self.records {
            for item in &mut record.items {
                if item.lidarr_album_id != Some(album_id) || !item.enabled {
                    continue;
                }
                let keep = item
                    .lidarr_track_id
                    .is_some_and(|track_id| keep_track_ids.contains(&track_id));
                if !keep {
                    item.enabled = false;
                    changed.push(item.clone());
                }
            }
            if !changed.is_empty() {
                record.updated_at = now;
            }
        }
        if !changed.is_empty() {
            self.updated_at = now;
        }
        changed
    }

    pub(crate) fn json_array(&mut self) -> String {
        let record = self.get_or_create();
        let items = record
            .items
            .iter()
            .map(|item| {
                let mut value = serde_json::from_str::<serde_json::Value>(&item.json())
                    .unwrap_or_else(|_| serde_json::json!({ "id": item.id }));
                let rules = self
                    .ignored_results
                    .iter()
                    .filter(|rule| rule.wishlist_item_id == item.id)
                    .map(WishlistIgnoredResult::json)
                    .collect::<Vec<_>>();
                value["ignoredResultCount"] = serde_json::json!(rules.len());
                value["ignoredResults"] = serde_json::Value::Array(rules);
                value.to_string()
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", items)
    }

    pub(crate) fn search_terms(&self) -> Vec<String> {
        self.records
            .iter()
            .flat_map(|record| record.items.iter())
            .filter(|item| item.enabled)
            .map(|item| {
                if item.title.is_empty() {
                    item.artist.clone()
                } else if item.artist.is_empty() {
                    item.title.clone()
                } else {
                    format!("{} {}", item.artist, item.title)
                }
            })
            .filter(|term| !term.trim().is_empty())
            .collect()
    }

    pub(crate) fn get_item(&self, item_id: &str) -> Option<WishlistItem> {
        self.records
            .iter()
            .flat_map(|record| record.items.iter())
            .find(|item| item.id == item_id)
            .cloned()
    }

    pub(crate) fn resolve_item_id(&self, item_id: &str, native: bool) -> Option<String> {
        if self.get_item(item_id).is_some() {
            return Some(item_id.to_owned());
        }
        native
            .then(|| {
                self.records
                    .iter()
                    .flat_map(|record| record.items.iter())
                    .find(|item| native_wishlist_item_id(&item.id) == item_id)
                    .map(|item| item.id.clone())
            })
            .flatten()
    }

    pub(crate) fn result_policy_for(&self, item_id: &str) -> Option<WishlistResultPolicy> {
        let item = self.get_item(item_id)?;
        Some(WishlistResultPolicy {
            filter: WishlistResultFilter::parse(&item.filter),
            max_results: item.max_results,
        })
    }

    pub(crate) fn mark_viewed(&mut self, item_id: &str) -> Option<WishlistItem> {
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == "default")?;
        let item = record.items.iter_mut().find(|item| item.id == item_id)?;
        item.last_viewed_at = Some(now);
        record.updated_at = now;
        self.updated_at = now;
        Some(item.clone())
    }

    pub(crate) fn mark_all_viewed(&mut self) -> Vec<WishlistItem> {
        let now = unix_timestamp();
        let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.id == "default")
        else {
            return Vec::new();
        };
        for item in &mut record.items {
            item.last_viewed_at = Some(now);
        }
        record.updated_at = now;
        self.updated_at = now;
        record.items.clone()
    }

    pub(crate) fn record_completed_search(
        &mut self,
        search: &SearchRecord,
    ) -> Option<WishlistItem> {
        let item_id = search.wishlist_item_id()?;
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == "default")?;
        let item = record.items.iter_mut().find(|item| item.id == item_id)?;
        item.last_searched_at = Some(now);
        item.last_match_count = search.results.len();
        item.last_visible_hit_count = search.results.len();
        item.last_hidden_locked_hit_count = search.hidden_locked_count;
        item.last_filtered_out_hit_count = search.filtered_out_count;
        item.last_ignored_result_hit_count = search.ignored_result_count;
        item.last_response_count = search.raw_response_count;
        item.total_search_count = item.total_search_count.saturating_add(1);
        item.last_search_id = Some(search.id.clone());
        record.updated_at = now;
        self.updated_at = now;
        Some(item.clone())
    }

    pub(crate) fn record_auto_downloads(
        &mut self,
        item_id: &str,
        enqueued_count: usize,
    ) -> Option<WishlistItem> {
        let now = unix_timestamp();
        let record = self
            .records
            .iter_mut()
            .find(|record| record.id == "default")?;
        let item = record.items.iter_mut().find(|item| item.id == item_id)?;
        item.total_download_count = item
            .total_download_count
            .saturating_add(u64::try_from(enqueued_count).unwrap_or(u64::MAX));
        if enqueued_count > 0
            && item
                .max_downloads
                .is_none_or(|limit| item.total_download_count >= limit)
        {
            item.enabled = false;
        }
        record.updated_at = now;
        self.updated_at = now;
        Some(item.clone())
    }

    pub(crate) fn list_ignored_results(&self, item_id: &str) -> Option<Vec<WishlistIgnoredResult>> {
        self.get_item(item_id)?;
        let mut rules = self
            .ignored_results
            .iter()
            .filter(|rule| rule.wishlist_item_id == item_id)
            .cloned()
            .collect::<Vec<_>>();
        rules.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Some(rules)
    }

    pub(crate) fn ignore_result(
        &mut self,
        item_id: &str,
        username: &str,
        directory: &str,
        native_contract: bool,
    ) -> Result<(WishlistIgnoredResult, bool), &'static str> {
        if self.get_item(item_id).is_none() {
            return Err("not_found");
        }
        let username =
            truncate_utf8_bytes(username.trim().to_owned(), MAX_SEARCH_RESULT_USERNAME_BYTES);
        let normalized_directory = normalize_wishlist_directory(directory);
        if username.is_empty() || normalized_directory.is_empty() {
            return Err("invalid");
        }
        // The versioned native profile contract normalizes separators and trailing
        // slashes but preserves a leading slash. The legacy route trims both
        // sides, so keep the two externally visible profiles distinct.
        let directory = if native_contract {
            truncate_utf8_bytes(
                directory
                    .replace('\\', "/")
                    .trim()
                    .trim_end_matches('/')
                    .to_owned(),
                MAX_SEARCH_RESULT_FILENAME_BYTES,
            )
        } else {
            normalized_directory.clone()
        };
        if let Some(existing) = self.ignored_results.iter().find(|rule| {
            rule.wishlist_item_id == item_id
                && rule.username.eq_ignore_ascii_case(&username)
                && normalize_wishlist_directory(&rule.directory)
                    .eq_ignore_ascii_case(&normalized_directory)
        }) {
            return Ok((existing.clone(), false));
        }
        if self.ignored_results.len() >= MAX_WISHLIST_IGNORED_RESULTS
            || self
                .ignored_results
                .iter()
                .filter(|rule| rule.wishlist_item_id == item_id)
                .count()
                >= MAX_WISHLIST_IGNORED_RESULTS_PER_ITEM
        {
            return Err("capacity");
        }
        let rule = WishlistIgnoredResult {
            id: uuid::Uuid::new_v4().to_string(),
            wishlist_item_id: item_id.to_owned(),
            username,
            directory,
            created_at: unix_timestamp(),
        };
        self.ignored_results.push(rule.clone());
        self.updated_at = rule.created_at;
        Ok((rule, true))
    }

    pub(crate) fn delete_ignored_result(&mut self, item_id: &str, rule_id: &str) -> bool {
        let before = self.ignored_results.len();
        self.ignored_results
            .retain(|rule| !(rule.wishlist_item_id == item_id && rule.id == rule_id));
        let deleted = self.ignored_results.len() != before;
        if deleted {
            self.updated_at = unix_timestamp();
        }
        deleted
    }

    pub(crate) fn ignored_results_for(&self, item_id: &str) -> Vec<WishlistIgnoredResult> {
        self.ignored_results
            .iter()
            .filter(|rule| rule.wishlist_item_id == item_id)
            .cloned()
            .collect()
    }

    pub(crate) fn item_id_for_search_text(&self, query: &str) -> Option<String> {
        self.records
            .iter()
            .flat_map(|record| record.items.iter())
            .find(|item| item.search_text() == query)
            .map(|item| item.id.clone())
    }
}

pub(super) fn normalize_wishlist_directory(directory: &str) -> String {
    truncate_utf8_bytes(
        directory
            .replace('\\', "/")
            .trim()
            .trim_matches('/')
            .to_owned(),
        MAX_SEARCH_RESULT_FILENAME_BYTES,
    )
}

pub(super) fn wishlist_parent_directory(filename: &str) -> String {
    let normalized = filename.replace('\\', "/");
    normalized
        .rsplit_once('/')
        .map(|(directory, _)| normalize_wishlist_directory(directory))
        .unwrap_or_default()
}

pub(super) type WishlistQualityKey = (u8, u64, u8, u32, u32, u8, u64);

#[derive(Clone)]
pub(super) struct WishlistAutoDownloadPlan {
    pub(super) username: String,
    pub(super) directory: String,
    pub(super) files: Vec<SearchResultEntry>,
    pub(super) coverage: usize,
    pub(super) weakest_quality: WishlistQualityKey,
    pub(super) representative_quality: WishlistQualityKey,
    pub(super) edition_mismatch: bool,
}

pub(super) fn wishlist_group_track_identity(filename: &str) -> String {
    let leaf = virtual_basename(filename);
    let stem = leaf
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(leaf)
        .trim_end();
    let stem = stem
        .strip_suffix(')')
        .and_then(|value| value.rsplit_once('('))
        .filter(|(_, suffix)| {
            suffix
                .trim()
                .chars()
                .all(|character| character.is_ascii_digit())
        })
        .map(|(value, _)| value.trim_end())
        .unwrap_or(stem);
    stem.to_ascii_lowercase()
}

fn wishlist_is_audio_format(term: &str) -> bool {
    matches!(
        term.trim_start_matches('.'),
        "flac"
            | "alac"
            | "wav"
            | "ape"
            | "aiff"
            | "aif"
            | "mp3"
            | "aac"
            | "m4a"
            | "ogg"
            | "oga"
            | "opus"
    )
}

fn wishlist_quality_key_for_format(entry: &SearchResultEntry, format: &str) -> WishlistQualityKey {
    let format = format.trim_start_matches('.');
    let bitrate = u64::from(entry.bit_rate.unwrap_or_default());
    let has_bitrate = entry.bit_rate.is_some_and(|value| value > 0);
    if matches!(format, "flac" | "alac" | "wav" | "ape" | "aiff" | "aif") {
        let codec_preference = match format {
            "flac" => 6,
            "alac" => 5,
            "wav" => 4,
            "ape" => 3,
            _ => 2,
        };
        return (
            2,
            0,
            codec_preference,
            entry.sample_rate.unwrap_or_default(),
            entry.bit_depth.unwrap_or_default(),
            u8::from(has_bitrate),
            entry.size,
        );
    }

    let efficiency_milli = match format {
        "opus" => 2_000,
        "aac" | "m4a" => 1_250,
        "ogg" | "oga" => 1_100,
        _ => 1_000,
    };
    let codec_preference = match format {
        "opus" => 4,
        "aac" | "m4a" => 3,
        "ogg" | "oga" => 2,
        "mp3" => 1,
        _ => 0,
    };
    (
        u8::from(has_bitrate),
        bitrate.saturating_mul(efficiency_milli),
        codec_preference,
        entry.sample_rate.unwrap_or_default(),
        entry.bit_depth.unwrap_or_default(),
        u8::from(has_bitrate),
        entry.size,
    )
}

pub(super) fn wishlist_quality_key(
    entry: &SearchResultEntry,
    filter: &WishlistResultFilter,
) -> WishlistQualityKey {
    let filename = entry.filename.replace('\\', "/").to_ascii_lowercase();
    let extension = virtual_basename(&filename)
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or_default();
    let mut formats = Vec::new();

    for clause in &filter.clauses {
        if clause
            .minimum_bitrate
            .is_some_and(|minimum| entry.bit_rate.is_none_or(|value| value < minimum))
        {
            continue;
        }
        let filename_matches = clause.include.is_empty()
            || clause
                .include
                .iter()
                .any(|term| extension == term || filename.contains(term));
        if !filename_matches {
            continue;
        }
        let format_hints = clause
            .include
            .iter()
            .filter(|term| {
                wishlist_is_audio_format(term) && extension == term.trim_start_matches('.')
            })
            .map(|term| term.trim_start_matches('.').to_owned())
            .collect::<Vec<_>>();
        if format_hints.is_empty() {
            formats.push(extension.to_owned());
        } else {
            formats.extend(format_hints);
        }
    }
    if formats.is_empty() {
        formats.push(extension.to_owned());
    }
    formats
        .iter()
        .map(|format| wishlist_quality_key_for_format(entry, format))
        .max()
        .unwrap_or_default()
}

pub(super) fn wishlist_track_identity(filename: &str) -> String {
    let leaf = virtual_basename(filename);
    let stem = leaf.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(leaf);
    let mut tokens = stem
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens
        .first()
        .is_some_and(|token| token.chars().all(|character| character.is_ascii_digit()))
    {
        tokens.remove(0);
    } else if tokens.first().is_some_and(|token| token == "disc")
        && tokens
            .get(1)
            .is_some_and(|token| token.chars().all(|character| character.is_ascii_digit()))
    {
        tokens.drain(..2);
    }
    tokens.join(" ")
}

const WISHLIST_EDITION_MISMATCH_MARKERS: &[&str] = &[
    "session",
    "sessions",
    "live",
    "acoustic",
    "unplugged",
    "remix",
    "remixes",
    "karaoke",
    "instrumental",
    "instrumentals",
    "demo",
    "demos",
    "rehearsal",
    "interview",
    "radio edit",
    "bootleg",
    "commentary",
];

pub(super) fn wishlist_edition_mismatch(
    item: &WishlistItem,
    directory: &str,
    files: &[SearchResultEntry],
) -> bool {
    if item.lidarr_track_id.is_none() {
        if let Some(expected_count) = item.lidarr_track_count {
            if (i64::try_from(files.len()).unwrap_or(i64::MAX) - expected_count).abs() > 1 {
                return true;
            }
        }
    }

    if let Some(expected_duration) = item
        .lidarr_duration_seconds
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
    {
        let total_duration = if item.lidarr_track_id.is_some() {
            files.first().and_then(|file| file.length_seconds)
        } else if files.iter().all(|file| file.length_seconds.is_some()) {
            Some(
                files
                    .iter()
                    .filter_map(|file| file.length_seconds)
                    .sum::<u32>(),
            )
        } else {
            None
        };
        if let Some(total_duration) = total_duration {
            let tolerance = 20_u32.max(expected_duration.saturating_mul(5) / 100);
            if total_duration.abs_diff(expected_duration) > tolerance {
                return true;
            }
        }
    }

    let normalized_directory = directory.replace('\\', "/");
    let folder_name = normalized_directory
        .rsplit('/')
        .next()
        .unwrap_or(directory)
        .to_ascii_lowercase();
    let expected_context = format!(
        "{} {} {}",
        item.artist,
        item.title,
        item.lidarr_release_disambiguation
            .as_deref()
            .unwrap_or_default()
    )
    .to_ascii_lowercase();
    WISHLIST_EDITION_MISMATCH_MARKERS
        .iter()
        .any(|marker| folder_name.contains(marker) && !expected_context.contains(marker))
}
