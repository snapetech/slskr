use super::*;

pub(super) const HASHDB_FLAC_INVENTORY_PREFIX: &str = "hashdb/flac-inventory/";

pub(super) fn hashdb_flac_inventory_key(peer_id: &str, path: &str, size: u64) -> String {
    let file_id = hex::encode(Sha256::digest(
        format!("{peer_id}|{path}|{size}").as_bytes(),
    ));
    format!("{HASHDB_FLAC_INVENTORY_PREFIX}{file_id}")
}

pub(super) fn hashdb_flac_inventory_record(
    peer_id: &str,
    path: &str,
    size: u64,
) -> serde_json::Value {
    let file_id = hex::encode(Sha256::digest(
        format!("{peer_id}|{path}|{size}").as_bytes(),
    ));
    serde_json::json!({
        "fileId": file_id,
        "peerId": peer_id,
        "path": path,
        "size": size,
        "discoveredAt": unix_timestamp(),
        "hashStatus": "none",
        "hashStatusStr": "none",
        "hashSource": "backfill_sniff",
        "hashSourceStr": "backfill_sniff",
    })
}

pub(super) fn hashdb_inventory_records(
    features: &ControllerFeatureState,
    limit: usize,
    candidates_only: bool,
) -> Vec<serde_json::Value> {
    features
        .values_with_prefix(HASHDB_FLAC_INVENTORY_PREFIX)
        .into_iter()
        .filter(|record| {
            !candidates_only
                || record
                    .get("hashStatusStr")
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|status| {
                        status.eq_ignore_ascii_case("none") || status.eq_ignore_ascii_case("failed")
                    })
        })
        .take(limit)
        .collect()
}

pub(super) fn hashdb_inventory_peer_ids(features: &ControllerFeatureState) -> HashSet<String> {
    features
        .values_with_prefix(HASHDB_FLAC_INVENTORY_PREFIX)
        .into_iter()
        .filter_map(|record| {
            record
                .get("peerId")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|peer_id| !peer_id.is_empty())
                .map(str::to_owned)
        })
        .collect()
}

pub(super) fn persisted_hash_db_record(
    entry: &content_discovery::HashDbEntry,
) -> persistence::HashDbRecord {
    persistence::HashDbRecord {
        flac_key: entry.flac_key.clone(),
        byte_hash: entry.byte_hash.clone(),
        size: i64::try_from(entry.size).unwrap_or(i64::MAX),
        first_seen_at: i64::try_from(entry.first_seen_at).unwrap_or(i64::MAX),
        last_updated_at: i64::try_from(entry.last_updated_at).unwrap_or(i64::MAX),
        seq_id: i64::try_from(entry.seq_id).unwrap_or(i64::MAX),
        use_count: i64::from(entry.use_count),
        full_file_hash: entry.full_file_hash.clone(),
        musicbrainz_id: entry.music_brainz_id.clone(),
        file_sha256: entry.file_sha256.clone(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HashDbVerificationRequest {
    filename: Option<String>,
    size: Option<i64>,
    byte_hash: Option<String>,
    #[allow(dead_code)]
    sample_rate: Option<i32>,
    #[allow(dead_code)]
    channels: Option<i32>,
    #[allow(dead_code)]
    bit_depth: Option<i32>,
}

pub(super) fn hashdb_verification_entry_from_body(
    body: &str,
) -> Result<content_discovery::HashDbEntry, String> {
    let request = serde_json::from_str::<HashDbVerificationRequest>(body)
        .map_err(|_| "filename, size, and byteHash are required".to_owned())?;
    let filename = request
        .filename
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "filename, size, and byteHash are required".to_owned())?;
    let size = request
        .size
        .filter(|value| *value > 0)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| "filename, size, and byteHash are required".to_owned())?;
    let byte_hash = request
        .byte_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "filename, size, and byteHash are required".to_owned())?;
    Ok(content_discovery::HashDbEntry {
        flac_key: content_discovery::generate_flac_key(filename, size),
        byte_hash: byte_hash.to_owned(),
        size,
        ..Default::default()
    })
}

pub(super) fn hash_db_entry_from_persistence(
    record: persistence::HashDbRecord,
) -> Result<content_discovery::HashDbEntry, String> {
    Ok(content_discovery::HashDbEntry {
        flac_key: record.flac_key,
        byte_hash: record.byte_hash,
        size: u64::try_from(record.size).map_err(|_| "HashDb size is invalid".to_owned())?,
        full_file_hash: record.full_file_hash,
        music_brainz_id: record.musicbrainz_id,
        file_sha256: record.file_sha256,
        first_seen_at: u64::try_from(record.first_seen_at)
            .map_err(|_| "HashDb firstSeenAt is invalid".to_owned())?,
        last_updated_at: u64::try_from(record.last_updated_at)
            .map_err(|_| "HashDb lastUpdatedAt is invalid".to_owned())?,
        seq_id: u64::try_from(record.seq_id).map_err(|_| "HashDb seqId is invalid".to_owned())?,
        use_count: u32::try_from(record.use_count)
            .map_err(|_| "HashDb useCount is invalid".to_owned())?,
    })
}

static HASH_DB_PERSISTENCE_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
    std::sync::OnceLock::new();

fn hash_db_persistence_lock() -> &'static tokio::sync::Mutex<()> {
    HASH_DB_PERSISTENCE_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

type HashDbPersistenceTurn = tokio::sync::MutexGuard<'static, ()>;

pub(super) async fn hash_db_persistence_turn() -> HashDbPersistenceTurn {
    hash_db_persistence_lock().lock().await
}

pub(super) async fn persist_hash_db_snapshot(
    state: &AppState,
    entries: &[content_discovery::HashDbEntry],
    latest_seq: u64,
    _persistence_turn: &HashDbPersistenceTurn,
) -> Result<(), String> {
    persist_hash_db_snapshot_unlocked(state, entries, latest_seq).await
}

pub(super) async fn persist_current_hash_db_snapshot(
    state: &AppState,
    persistence_turn: &HashDbPersistenceTurn,
) -> Result<(), String> {
    let (entries, latest_seq) = {
        let discovery = state.content_discovery.read().await;
        (discovery.hash_entries().to_vec(), discovery.latest_seq())
    };
    persist_hash_db_snapshot(state, &entries, latest_seq, persistence_turn).await
}

async fn persist_hash_db_snapshot_unlocked(
    state: &AppState,
    entries: &[content_discovery::HashDbEntry],
    latest_seq: u64,
) -> Result<(), String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(());
    };
    let records = entries
        .iter()
        .map(persisted_hash_db_record)
        .collect::<Vec<_>>();
    db.replace_hash_db_snapshot(&records, i64::try_from(latest_seq).unwrap_or(i64::MAX))
        .await
        .map_err(|error| format!("hash database storage unavailable: {error}"))
}

pub(super) async fn rollback_hash_db_entries_if_unchanged(
    state: &AppState,
    previous_entries: Vec<content_discovery::HashDbEntry>,
    previous_latest_seq: u64,
    mutated_entries: &[content_discovery::HashDbEntry],
    mutated_latest_seq: u64,
) {
    let rollback_error = {
        let mut discovery = state.content_discovery.write().await;
        if discovery.latest_seq() != mutated_latest_seq
            || discovery.hash_entries() != mutated_entries
        {
            return;
        }
        discovery
            .restore_hash_entries(previous_entries, previous_latest_seq)
            .err()
    };
    if let Some(error) = rollback_error {
        record_daemon_log(
            state,
            logging::LogLevel::Error,
            "hashdb",
            format!("failed to roll back an unpersisted HashDb merge: {error}"),
        )
        .await;
    }
}

pub(super) async fn read_hash_db_state_json(
    state: &AppState,
    key: &str,
) -> Result<Option<serde_json::Value>, String> {
    if let Some(db) = state.db.as_ref() {
        let record = db
            .get_hash_db_state(key)
            .await
            .map_err(|error| format!("hash database state unavailable: {error}"))?;
        let Some(record) = record else {
            return Ok(None);
        };
        let Some(value) = record.value else {
            return Ok(None);
        };
        return serde_json::from_str(&value)
            .map(Some)
            .map_err(|error| format!("HashDbState value is invalid: {error}"));
    }
    Ok(state.controller_features.read().await.get(key).cloned())
}

pub(super) async fn write_hash_db_state_json(
    state: &AppState,
    key: &str,
    value: Option<serde_json::Value>,
    _persistence_turn: &HashDbPersistenceTurn,
) -> Result<(), String> {
    if let Some(db) = state.db.as_ref() {
        let value = value
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty());
        db.upsert_hash_db_state(&persistence::HashDbStateRecord {
            key: key.to_owned(),
            value,
        })
        .await
        .map_err(|error| format!("hash database state unavailable: {error}"))?;
        return Ok(());
    }
    if let Some(value) = value {
        state
            .controller_features
            .upsert(key.to_owned(), value)
            .await
    } else {
        state.controller_features.remove(key).await.map(|_| ())
    }
}
