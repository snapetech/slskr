use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const STATE_VERSION: u32 = 1;
const MAX_STATE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_HASH_ENTRIES: usize = 16_384;
const MAX_HASH_MERGE_ENTRIES: usize = 1_000;
const MAX_SHADOW_RECORDINGS: usize = 4_096;
const MAX_SHADOW_MERGE_RECORDS: usize = 256;
const MAX_PEERS_PER_RECORDING: usize = 64;
const MAX_FLAC_KEY_BYTES: usize = 256;
const MAX_RECORDING_ID_BYTES: usize = 128;
const MAX_PEER_ID_BYTES: usize = 256;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct HashDbEntry {
    pub flac_key: String,
    pub byte_hash: String,
    pub size: u64,
    pub full_file_hash: String,
    pub music_brainz_id: String,
    pub file_sha256: String,
    pub first_seen_at: u64,
    pub last_updated_at: u64,
    pub seq_id: u64,
    pub use_count: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShadowIndexRecord {
    #[serde(alias = "mbid")]
    pub recording_id: String,
    pub peer_ids: Vec<String>,
    pub updated_at: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct ContentDiscoveryState {
    version: u32,
    latest_seq: u64,
    hash_entries: Vec<HashDbEntry>,
    shadow_records: Vec<ShadowIndexRecord>,
}

#[derive(Debug)]
pub struct ContentDiscoveryStore {
    state_path: Option<PathBuf>,
    hash_entries: Vec<HashDbEntry>,
    shadow_records: Vec<ShadowIndexRecord>,
    latest_seq: u64,
}

impl ContentDiscoveryStore {
    pub fn in_memory() -> Self {
        Self {
            state_path: None,
            hash_entries: Vec::new(),
            shadow_records: Vec::new(),
            latest_seq: 0,
        }
    }

    pub fn load(state_dir: &Path) -> Result<Self, String> {
        let state_path = state_dir.join("content-discovery-state.json");
        let Some(mut state) = read_state(&state_path)? else {
            return Ok(Self {
                state_path: Some(state_path),
                ..Self::in_memory()
            });
        };
        if state.version != STATE_VERSION {
            return Err(format!(
                "unsupported content discovery state version: {}",
                state.version
            ));
        }
        if state.hash_entries.len() > MAX_HASH_ENTRIES
            || state.shadow_records.len() > MAX_SHADOW_RECORDINGS
        {
            return Err("content discovery state exceeds bounded record capacity".to_owned());
        }
        let now = crate::unix_timestamp();
        let mut normalized_hashes = Vec::with_capacity(state.hash_entries.len());
        for entry in state.hash_entries.drain(..) {
            let persisted_last_updated_at = entry.last_updated_at;
            let mut entry = normalize_hash_entry(entry, now)?;
            if persisted_last_updated_at != 0 {
                entry.last_updated_at = persisted_last_updated_at;
            }
            normalized_hashes.push(entry);
        }
        let mut normalized_shadow = Vec::with_capacity(state.shadow_records.len());
        for record in state.shadow_records.drain(..) {
            let persisted_updated_at = record.updated_at;
            let mut record = normalize_shadow_record(record, now)?;
            if persisted_updated_at != 0 {
                record.updated_at = persisted_updated_at;
            }
            normalized_shadow.push(record);
        }
        dedupe_hash_entries(&mut normalized_hashes)?;
        dedupe_shadow_records(&mut normalized_shadow);
        let latest_seq = state.latest_seq.max(
            normalized_hashes
                .iter()
                .map(|entry| entry.seq_id)
                .max()
                .unwrap_or(0),
        );
        Ok(Self {
            state_path: Some(state_path),
            hash_entries: normalized_hashes,
            shadow_records: normalized_shadow,
            latest_seq,
        })
    }

    pub fn latest_seq(&self) -> u64 {
        self.latest_seq
    }

    pub fn hash_entries(&self) -> &[HashDbEntry] {
        &self.hash_entries
    }

    pub fn shadow_records(&self) -> &[ShadowIndexRecord] {
        &self.shadow_records
    }

    pub fn lookup_hash(&self, flac_key: &str) -> Option<&HashDbEntry> {
        let flac_key = flac_key.trim();
        self.hash_entries
            .iter()
            .find(|entry| entry.flac_key.eq_ignore_ascii_case(flac_key))
    }

    pub fn hashes_by_size(&self, size: u64) -> Vec<&HashDbEntry> {
        self.hash_entries
            .iter()
            .filter(|entry| entry.size == size)
            .collect()
    }

    pub fn verified_file_hash(&self, filename: &str, size: u64) -> Option<String> {
        let entry = self.lookup_hash(&generate_flac_key(filename, size))?;
        if entry.size != size {
            return None;
        }
        let hashes = [&entry.file_sha256, &entry.full_file_hash]
            .into_iter()
            .filter(|hash| !hash.is_empty())
            .collect::<Vec<_>>();
        let hash = hashes.first()?;
        hashes
            .iter()
            .all(|candidate| candidate.eq_ignore_ascii_case(hash))
            .then(|| (*hash).clone())
    }

    pub fn recording_ids_for_hash(&self, expected_hash: &str, size: u64) -> Vec<String> {
        let expected_hash = expected_hash.trim();
        let mut seen = HashSet::new();
        self.hash_entries
            .iter()
            .filter(|entry| entry.size == size && !entry.music_brainz_id.is_empty())
            .filter(|entry| {
                [&entry.byte_hash, &entry.full_file_hash, &entry.file_sha256]
                    .into_iter()
                    .any(|candidate| {
                        !candidate.is_empty() && candidate.eq_ignore_ascii_case(expected_hash)
                    })
            })
            .filter_map(|entry| {
                seen.insert(entry.music_brainz_id.to_ascii_lowercase())
                    .then_some(entry.music_brainz_id.clone())
            })
            .collect()
    }

    pub fn peer_ids_for_recordings(&self, recording_ids: &[String]) -> Vec<String> {
        let recording_ids = recording_ids
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<HashSet<_>>();
        let mut seen = HashSet::new();
        self.shadow_records
            .iter()
            .filter(|record| recording_ids.contains(&record.recording_id.to_ascii_lowercase()))
            .flat_map(|record| record.peer_ids.iter())
            .filter_map(|peer_id| {
                seen.insert(peer_id.to_ascii_lowercase())
                    .then_some(peer_id.clone())
            })
            .collect()
    }

    pub fn merge_hash_entries(&mut self, entries: Vec<HashDbEntry>) -> Result<usize, String> {
        if entries.is_empty() || entries.len() > MAX_HASH_MERGE_ENTRIES {
            return Err(format!(
                "hash merge requires 1 to {MAX_HASH_MERGE_ENTRIES} entries"
            ));
        }
        let previous_entries = self.hash_entries.clone();
        let previous_seq = self.latest_seq;
        let now = crate::unix_timestamp();
        let entries = entries
            .into_iter()
            .map(|entry| normalize_hash_entry(entry, now))
            .collect::<Result<Vec<_>, _>>()?;
        self.latest_seq
            .checked_add(entries.len() as u64)
            .ok_or_else(|| "hash database sequence is exhausted".to_owned())?;
        let mut merged = 0;
        for mut incoming in entries {
            let same_key = self.hash_entries.iter().position(|entry| {
                !incoming.flac_key.is_empty()
                    && entry.size == incoming.size
                    && entry.flac_key.eq_ignore_ascii_case(&incoming.flac_key)
            });
            if same_key.is_some_and(|index| {
                !same_hash_identity(&self.hash_entries[index], &incoming)
                    || hash_metadata_conflicts(&self.hash_entries[index], &incoming)
            }) {
                self.hash_entries = previous_entries;
                self.latest_seq = previous_seq;
                return Err("flacKey and size conflict with existing SHA-256 metadata".to_owned());
            }
            let existing = same_key.or_else(|| {
                self.hash_entries.iter().position(|entry| {
                    same_hash_identity(entry, &incoming)
                        && !hash_metadata_conflicts(entry, &incoming)
                })
            });
            self.latest_seq += 1;
            incoming.seq_id = self.latest_seq;
            if let Some(index) = existing {
                merge_missing_hash_metadata(&mut incoming, &self.hash_entries[index]);
                incoming.first_seen_at = self.hash_entries[index].first_seen_at;
                incoming.use_count = self.hash_entries[index].use_count.saturating_add(1);
                self.hash_entries[index] = incoming;
            } else {
                if self.hash_entries.len() >= MAX_HASH_ENTRIES {
                    self.hash_entries = previous_entries;
                    self.latest_seq = previous_seq;
                    return Err("hash database capacity is full".to_owned());
                }
                incoming.first_seen_at = now;
                incoming.use_count = 1;
                self.hash_entries.push(incoming);
            }
            merged += 1;
        }
        if let Err(error) = self.persist() {
            self.hash_entries = previous_entries;
            self.latest_seq = previous_seq;
            return Err(error);
        }
        Ok(merged)
    }

    pub fn merge_shadow_records(
        &mut self,
        records: Vec<ShadowIndexRecord>,
    ) -> Result<usize, String> {
        if records.is_empty() || records.len() > MAX_SHADOW_MERGE_RECORDS {
            return Err(format!(
                "shadow-index merge requires 1 to {MAX_SHADOW_MERGE_RECORDS} records"
            ));
        }
        let previous = self.shadow_records.clone();
        let now = crate::unix_timestamp();
        let records = records
            .into_iter()
            .map(|record| normalize_shadow_record(record, now))
            .collect::<Result<Vec<_>, _>>()?;
        let mut merged = 0;
        for incoming in records {
            if let Some(existing) = self.shadow_records.iter_mut().find(|record| {
                record
                    .recording_id
                    .eq_ignore_ascii_case(&incoming.recording_id)
            }) {
                *existing = incoming;
            } else {
                if self.shadow_records.len() >= MAX_SHADOW_RECORDINGS {
                    self.shadow_records = previous;
                    return Err("shadow-index capacity is full".to_owned());
                }
                self.shadow_records.push(incoming);
            }
            merged += 1;
        }
        if let Err(error) = self.persist() {
            self.shadow_records = previous;
            return Err(error);
        }
        Ok(merged)
    }

    fn persist(&self) -> Result<(), String> {
        let Some(path) = self.state_path.as_deref() else {
            return Ok(());
        };
        let state = ContentDiscoveryState {
            version: STATE_VERSION,
            latest_seq: self.latest_seq,
            hash_entries: self.hash_entries.clone(),
            shadow_records: self.shadow_records.clone(),
        };
        let body = serde_json::to_vec_pretty(&state)
            .map_err(|error| format!("content discovery state encode failed: {error}"))?;
        if body.len() as u64 > MAX_STATE_BYTES {
            return Err("content discovery state exceeds the file size limit".to_owned());
        }
        crate::write_file_atomic(path, body)
            .map_err(|error| format!("content discovery state write failed: {error}"))
    }
}

pub fn generate_flac_key(filename: &str, size: u64) -> String {
    let normalized = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(filename)
        .to_lowercase();
    let input = format!("{normalized}:{size}");
    hex::encode(&Sha256::digest(input.as_bytes())[..8])
}

fn normalize_hash_entry(mut entry: HashDbEntry, now: u64) -> Result<HashDbEntry, String> {
    entry.flac_key = bounded_non_control(entry.flac_key.trim(), MAX_FLAC_KEY_BYTES, "flacKey")?;
    entry.music_brainz_id = bounded_non_control(
        entry.music_brainz_id.trim(),
        MAX_RECORDING_ID_BYTES,
        "musicBrainzId",
    )?;
    entry.byte_hash = normalize_hash(&entry.byte_hash, "byteHash")?;
    entry.full_file_hash = normalize_hash(&entry.full_file_hash, "fullFileHash")?;
    entry.file_sha256 = normalize_hash(&entry.file_sha256, "fileSha256")?;
    if !entry.full_file_hash.is_empty()
        && !entry.file_sha256.is_empty()
        && entry.full_file_hash != entry.file_sha256
    {
        return Err("fullFileHash conflicts with fileSha256".to_owned());
    }
    if entry.size == 0 {
        return Err("hash entry size must be positive".to_owned());
    }
    if entry.byte_hash.is_empty() && entry.full_file_hash.is_empty() && entry.file_sha256.is_empty()
    {
        return Err("hash entry requires byteHash, fullFileHash, or fileSha256".to_owned());
    }
    if entry.flac_key.is_empty() {
        entry.flac_key = format!(
            "{}-{}",
            entry.size,
            [&entry.file_sha256, &entry.full_file_hash, &entry.byte_hash]
                .into_iter()
                .find(|value| !value.is_empty())
                .expect("a normalized hash exists")
        );
    }
    if entry.first_seen_at == 0 {
        entry.first_seen_at = now;
    }
    entry.last_updated_at = now;
    entry.use_count = entry.use_count.max(1);
    Ok(entry)
}

fn normalize_shadow_record(
    mut record: ShadowIndexRecord,
    now: u64,
) -> Result<ShadowIndexRecord, String> {
    record.recording_id = bounded_non_control(
        record.recording_id.trim(),
        MAX_RECORDING_ID_BYTES,
        "recordingId",
    )?;
    if record.recording_id.is_empty() {
        return Err("shadow-index recordingId is required".to_owned());
    }
    if record.peer_ids.is_empty() || record.peer_ids.len() > MAX_PEERS_PER_RECORDING {
        return Err(format!(
            "shadow-index record requires 1 to {MAX_PEERS_PER_RECORDING} peerIds"
        ));
    }
    let mut seen = HashSet::new();
    let mut peer_ids = Vec::with_capacity(record.peer_ids.len());
    for peer_id in record.peer_ids {
        let peer_id = bounded_non_control(peer_id.trim(), MAX_PEER_ID_BYTES, "peerId")?;
        if peer_id.is_empty() {
            return Err("shadow-index peerId is required".to_owned());
        }
        if seen.insert(peer_id.to_ascii_lowercase()) {
            peer_ids.push(peer_id);
        }
    }
    record.peer_ids = peer_ids;
    record.updated_at = now;
    Ok(record)
}

fn normalize_hash(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{field} must be a 64-character SHA-256 digest"));
    }
    Ok(value.to_ascii_lowercase())
}

fn bounded_non_control(value: &str, max_bytes: usize, field: &str) -> Result<String, String> {
    if value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(format!("{field} is invalid or exceeds {max_bytes} bytes"));
    }
    Ok(value.to_owned())
}

fn same_hash_identity(left: &HashDbEntry, right: &HashDbEntry) -> bool {
    left.size == right.size
        && ((!left.byte_hash.is_empty() && left.byte_hash.eq_ignore_ascii_case(&right.byte_hash))
            || [&left.full_file_hash, &left.file_sha256]
                .into_iter()
                .filter(|hash| !hash.is_empty())
                .any(|left_hash| {
                    [&right.full_file_hash, &right.file_sha256]
                        .into_iter()
                        .any(|right_hash| left_hash.eq_ignore_ascii_case(right_hash))
                }))
}

fn hash_metadata_conflicts(left: &HashDbEntry, right: &HashDbEntry) -> bool {
    if !left.byte_hash.is_empty()
        && !right.byte_hash.is_empty()
        && !left.byte_hash.eq_ignore_ascii_case(&right.byte_hash)
    {
        return true;
    }
    let mut whole_file_hash = None;
    for hash in [
        &left.full_file_hash,
        &left.file_sha256,
        &right.full_file_hash,
        &right.file_sha256,
    ] {
        if hash.is_empty() {
            continue;
        }
        if whole_file_hash.is_some_and(|existing: &String| !existing.eq_ignore_ascii_case(hash)) {
            return true;
        }
        whole_file_hash = Some(hash);
    }
    false
}

fn merge_missing_hash_metadata(incoming: &mut HashDbEntry, existing: &HashDbEntry) {
    if incoming.flac_key.is_empty() {
        incoming.flac_key.clone_from(&existing.flac_key);
    }
    if incoming.byte_hash.is_empty() {
        incoming.byte_hash.clone_from(&existing.byte_hash);
    }
    if incoming.full_file_hash.is_empty() {
        incoming.full_file_hash.clone_from(&existing.full_file_hash);
    }
    if incoming.file_sha256.is_empty() {
        incoming.file_sha256.clone_from(&existing.file_sha256);
    }
    if incoming.music_brainz_id.is_empty() {
        incoming
            .music_brainz_id
            .clone_from(&existing.music_brainz_id);
    }
}

fn dedupe_hash_entries(entries: &mut Vec<HashDbEntry>) -> Result<(), String> {
    let mut deduped: Vec<HashDbEntry> = Vec::with_capacity(entries.len());
    for mut entry in entries.drain(..) {
        let same_key = deduped.iter().position(|current| {
            !entry.flac_key.is_empty()
                && current.size == entry.size
                && current.flac_key.eq_ignore_ascii_case(&entry.flac_key)
        });
        if same_key.is_some_and(|index| {
            !same_hash_identity(&deduped[index], &entry)
                || hash_metadata_conflicts(&deduped[index], &entry)
        }) {
            return Err("flacKey and size conflict with persisted SHA-256 metadata".to_owned());
        }
        if let Some(index) = same_key.or_else(|| {
            deduped.iter().position(|current| {
                same_hash_identity(current, &entry) && !hash_metadata_conflicts(current, &entry)
            })
        }) {
            merge_missing_hash_metadata(&mut entry, &deduped[index]);
            deduped[index] = entry;
        } else {
            deduped.push(entry);
        }
    }
    *entries = deduped;
    Ok(())
}

fn dedupe_shadow_records(records: &mut Vec<ShadowIndexRecord>) {
    let mut deduped: Vec<ShadowIndexRecord> = Vec::with_capacity(records.len());
    for record in records.drain(..) {
        if let Some(index) = deduped.iter().position(|current| {
            current
                .recording_id
                .eq_ignore_ascii_case(&record.recording_id)
        }) {
            deduped[index] = record;
        } else {
            deduped.push(record);
        }
    }
    *records = deduped;
}

fn read_state(path: &Path) -> Result<Option<ContentDiscoveryState>, String> {
    #[cfg(not(unix))]
    {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!("content discovery state metadata failed: {error}"));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("content discovery state path must be a regular file".to_owned());
        }
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("content discovery state open failed: {error}")),
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("content discovery state metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("content discovery state path must be a regular file".to_owned());
    }
    if metadata.len() > MAX_STATE_BYTES {
        return Err("content discovery state file is too large".to_owned());
    }
    let mut body = Vec::new();
    file.take(MAX_STATE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| format!("content discovery state read failed: {error}"))?;
    if body.len() as u64 > MAX_STATE_BYTES {
        return Err("content discovery state file is too large".to_owned());
    }
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|error| format!("content discovery state parse failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn hash_entry(hash: &str, recording_id: &str) -> HashDbEntry {
        HashDbEntry {
            flac_key: "track-key".to_owned(),
            size: 123,
            file_sha256: hash.to_owned(),
            music_brainz_id: recording_id.to_owned(),
            ..HashDbEntry::default()
        }
    }

    #[test]
    fn exact_size_hash_resolution_reaches_deduplicated_shadow_peers() {
        let mut store = ContentDiscoveryStore::in_memory();
        store
            .merge_hash_entries(vec![hash_entry(&HASH.to_ascii_uppercase(), "Recording-1")])
            .expect("merge hash");
        store
            .merge_shadow_records(vec![ShadowIndexRecord {
                recording_id: "recording-1".to_owned(),
                peer_ids: vec![
                    "Peer-A".to_owned(),
                    "peer-a".to_owned(),
                    "Peer-B".to_owned(),
                ],
                ..ShadowIndexRecord::default()
            }])
            .expect("merge shadow record");

        let recording_ids = store.recording_ids_for_hash(HASH, 123);
        assert_eq!(recording_ids, vec!["Recording-1"]);
        assert_eq!(
            store.peer_ids_for_recordings(&recording_ids),
            vec!["Peer-A", "Peer-B"]
        );
        assert!(store.recording_ids_for_hash(HASH, 124).is_empty());
    }

    #[test]
    fn rescue_lookup_rejects_conflicting_whole_file_hashes() {
        assert_eq!(
            generate_flac_key("Remote/Album/Track.FLAC", 123),
            "072a888a552d6ba1"
        );
        let mut store = ContentDiscoveryStore::in_memory();
        let mut entry = hash_entry(HASH, "recording-1");
        entry.flac_key = generate_flac_key("Track.FLAC", 123);
        entry.full_file_hash = HASH.to_owned();
        store.merge_hash_entries(vec![entry]).expect("merge hash");
        assert_eq!(
            store.verified_file_hash("folder\\TRACK.flac", 123),
            Some(HASH.to_owned())
        );
        assert_eq!(store.verified_file_hash("track.flac", 124), None);

        let mut conflicting = hash_entry(HASH, "recording-1");
        conflicting.flac_key = generate_flac_key("Track.FLAC", 123);
        conflicting.full_file_hash =
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
        let error = store
            .merge_hash_entries(vec![conflicting])
            .expect_err("reject conflicting full hashes");
        assert_eq!(error, "fullFileHash conflicts with fileSha256");
        assert_eq!(
            store.verified_file_hash("track.flac", 123),
            Some(HASH.to_owned())
        );
    }

    #[test]
    fn equivalent_whole_file_hash_fields_merge_without_conflict() {
        let mut store = ContentDiscoveryStore::in_memory();
        let existing = hash_entry(HASH, "recording-1");
        store
            .merge_hash_entries(vec![existing])
            .expect("merge file hash");

        let incoming = HashDbEntry {
            flac_key: "track-key".to_owned(),
            size: 123,
            full_file_hash: HASH.to_owned(),
            music_brainz_id: "recording-1".to_owned(),
            ..HashDbEntry::default()
        };
        store
            .merge_hash_entries(vec![incoming])
            .expect("merge equivalent full-file hash field");

        assert_eq!(store.hash_entries().len(), 1);
        assert_eq!(store.hash_entries()[0].file_sha256, HASH);
        assert_eq!(store.hash_entries()[0].full_file_hash, HASH);
        assert_eq!(store.hash_entries()[0].use_count, 2);
    }

    #[test]
    fn invalid_hash_batch_is_rejected_without_partial_mutation() {
        let mut store = ContentDiscoveryStore::in_memory();
        let error = store
            .merge_hash_entries(vec![
                hash_entry(HASH, "recording-1"),
                hash_entry("bad", "x"),
            ])
            .expect_err("invalid batch");
        assert!(error.contains("64-character SHA-256"));
        assert!(store.hash_entries().is_empty());
        assert_eq!(store.latest_seq(), 0);
    }

    #[test]
    fn mismatched_size_cannot_replace_verified_flac_key() {
        let mut store = ContentDiscoveryStore::in_memory();
        let flac_key = generate_flac_key("Track.flac", 123);
        let mut legitimate = hash_entry(HASH, "recording-1");
        legitimate.flac_key.clone_from(&flac_key);
        legitimate.full_file_hash = HASH.to_owned();
        store
            .merge_hash_entries(vec![legitimate])
            .expect("merge legitimate hash");

        let attacker_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let mut mismatched = hash_entry(attacker_hash, "recording-2");
        mismatched.flac_key = flac_key;
        mismatched.size = 999;
        store
            .merge_hash_entries(vec![mismatched])
            .expect("retain distinct mismatched-size hash");

        assert_eq!(store.hash_entries().len(), 2);
        assert_eq!(
            store.verified_file_hash("track.flac", 123),
            Some(HASH.to_owned())
        );
        assert_eq!(store.hashes_by_size(999)[0].file_sha256, attacker_hash);
    }

    #[test]
    fn conflicting_hash_cannot_replace_matching_flac_key_and_size() {
        let mut store = ContentDiscoveryStore::in_memory();
        let legitimate = hash_entry(HASH, "recording-1");
        store
            .merge_hash_entries(vec![legitimate])
            .expect("merge legitimate hash");
        let latest_seq = store.latest_seq();
        let stored_legitimate = store.hash_entries().to_vec();

        let attacker_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let poisoned = hash_entry(attacker_hash, "recording-2");
        let mut unrelated = hash_entry(
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "unrelated",
        );
        unrelated.flac_key = "unrelated-key".to_owned();
        let error = store
            .merge_hash_entries(vec![unrelated, poisoned])
            .expect_err("reject conflicting hash metadata atomically");

        assert_eq!(
            error,
            "flacKey and size conflict with existing SHA-256 metadata"
        );
        assert_eq!(store.hash_entries(), stored_legitimate);
        assert_eq!(store.latest_seq(), latest_seq);
    }

    #[test]
    fn persisted_conflicting_flac_key_metadata_is_rejected() {
        let mut entries = vec![
            hash_entry(HASH, "recording-1"),
            hash_entry(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "recording-2",
            ),
        ];
        let error = dedupe_hash_entries(&mut entries)
            .expect_err("persisted collision must not select the last record");
        assert_eq!(
            error,
            "flacKey and size conflict with persisted SHA-256 metadata"
        );
    }

    #[test]
    fn persisted_complementary_whole_file_hashes_preserve_metadata() {
        let mut file_hash = hash_entry(HASH, "recording-1");
        let mut full_hash = HashDbEntry {
            flac_key: file_hash.flac_key.clone(),
            size: file_hash.size,
            full_file_hash: HASH.to_owned(),
            ..HashDbEntry::default()
        };
        file_hash.last_updated_at = 1;
        full_hash.last_updated_at = 2;
        let mut entries = vec![file_hash, full_hash];

        dedupe_hash_entries(&mut entries).expect("dedupe complementary hashes");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_sha256, HASH);
        assert_eq!(entries[0].full_file_hash, HASH);
        assert_eq!(entries[0].last_updated_at, 2);
    }

    #[test]
    fn exhausted_hash_sequence_rejects_the_entire_batch() {
        let mut store = ContentDiscoveryStore::in_memory();
        store.latest_seq = u64::MAX - 1;
        let original = hash_entry(HASH, "existing");
        store.hash_entries.push(original.clone());

        let error = store
            .merge_hash_entries(vec![
                hash_entry(
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "recording-1",
                ),
                hash_entry(
                    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                    "recording-2",
                ),
            ])
            .expect_err("sequence overflow must reject the batch");

        assert_eq!(error, "hash database sequence is exhausted");
        assert_eq!(store.latest_seq(), u64::MAX - 1);
        assert_eq!(store.hash_entries(), &[original]);
    }

    #[test]
    fn new_hash_metadata_is_server_owned() {
        let mut store = ContentDiscoveryStore::in_memory();
        let mut incoming = hash_entry(HASH, "recording-1");
        incoming.first_seen_at = u64::MAX;
        incoming.last_updated_at = u64::MAX;
        incoming.use_count = u32::MAX;

        store
            .merge_hash_entries(vec![incoming])
            .expect("merge hash");

        let stored = &store.hash_entries()[0];
        assert_ne!(stored.first_seen_at, u64::MAX);
        assert_ne!(stored.last_updated_at, u64::MAX);
        assert_eq!(stored.use_count, 1);
    }

    #[test]
    fn durable_store_rehydrates_hash_and_shadow_relations() {
        let root = std::env::temp_dir().join(format!(
            "slskr-content-discovery-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("create state directory");
        let mut store = ContentDiscoveryStore::load(&root).expect("new store");
        store
            .merge_hash_entries(vec![hash_entry(HASH, "recording-1")])
            .expect("persist hash");
        store
            .merge_shadow_records(vec![ShadowIndexRecord {
                recording_id: "recording-1".to_owned(),
                peer_ids: vec!["peer-a".to_owned()],
                ..ShadowIndexRecord::default()
            }])
            .expect("persist shadow record");
        store.hash_entries[0].first_seen_at = 101;
        store.hash_entries[0].last_updated_at = 202;
        store.shadow_records[0].updated_at = 303;
        store.persist().expect("persist distinct timestamps");
        drop(store);

        let loaded = ContentDiscoveryStore::load(&root).expect("reload store");
        assert_eq!(loaded.hashes_by_size(123).len(), 1);
        assert_eq!(loaded.shadow_records().len(), 1);
        assert_eq!(loaded.hash_entries()[0].first_seen_at, 101);
        assert_eq!(loaded.hash_entries()[0].last_updated_at, 202);
        assert_eq!(loaded.shadow_records()[0].updated_at, 303);
        fs::remove_dir_all(root).expect("remove state directory");
    }
}
