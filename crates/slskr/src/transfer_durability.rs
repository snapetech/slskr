use super::persistence;
use super::transfer_state_io::{append_transfer_event, write_transfer_state};
use super::{
    publish_transfer_hub_event, unix_timestamp_millis, update_session, AppState, TransferEntry,
    TransferQueue,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
};
use tokio::sync::Mutex as AsyncMutex;

pub(super) fn persisted_transfer_record(entry: &TransferEntry) -> persistence::TransferRecord {
    let completed_at = if matches!(
        entry.status.as_str(),
        "succeeded" | "completed" | "cancelled" | "failed" | "rejected" | "errored"
    ) {
        Some(database_i64(entry.updated_at))
    } else {
        None
    };
    persistence::TransferRecord {
        id: entry.id.to_string(),
        direction: if entry.direction == 0 {
            "download".to_owned()
        } else {
            "upload".to_owned()
        },
        filename: entry.filename.clone(),
        peer_username: entry.peer_username.clone().unwrap_or_default(),
        filesize: database_i64(entry.size.unwrap_or(0)),
        progress: database_i64(entry.bytes_transferred),
        status: entry.status.clone(),
        started_at: database_i64(entry.started_at.unwrap_or(entry.requested_at)),
        completed_at,
        request_id: entry.request_id.clone(),
        wishlist_item_id: entry.wishlist_item_id.clone(),
        request_name: entry.request_name.clone(),
        destination_directory: entry.destination_directory.clone(),
        local_path: entry.local_path.clone(),
        batch_id: entry.batch_id.clone(),
        reason: entry.reason.clone(),
        bit_rate: entry.bit_rate.map(i64::from),
        sample_rate: entry.sample_rate.map(i64::from),
        bit_depth: entry.bit_depth.map(i64::from),
        length_seconds: entry.length_seconds.map(i64::from),
        artist: entry.artist.clone(),
        album: entry.album.clone(),
        title: entry.title.clone(),
        track_number: entry.track_number.map(i64::from),
        year: entry.year.map(i64::from),
        attempts: i64::from(entry.attempts.max(1)),
        auto_replace_attempts: i64::from(entry.auto_replace_attempts),
        next_attempt_at: entry.next_attempt_at.map(database_i64),
        updated_at_ms: database_i64(entry.updated_at_ms),
    }
}

pub(super) fn database_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(super) fn next_transfer_updated_at_ms(previous: u64) -> u64 {
    unix_timestamp_millis().max(previous.saturating_add(1))
}

pub(super) fn persisted_transfer_event_record(
    entry: &TransferEntry,
) -> persistence::TransferEventRecord {
    persistence::TransferEventRecord {
        id: 0,
        transfer_id: entry.id.to_string(),
        direction: if entry.direction == 0 {
            "download".to_owned()
        } else {
            "upload".to_owned()
        },
        token: i64::from(entry.token),
        filename: entry.filename.clone(),
        peer_username: entry.peer_username.clone(),
        filesize: i64::try_from(entry.size.unwrap_or(0)).unwrap_or(i64::MAX),
        progress: i64::try_from(entry.bytes_transferred).unwrap_or(i64::MAX),
        status: entry.status.clone(),
        reason: entry.reason.clone(),
        created_at: i64::try_from(entry.updated_at).unwrap_or(i64::MAX),
        updated_at_ms: database_i64(entry.updated_at_ms),
    }
}

pub(super) async fn record_transfer_rejection(
    state: &AppState,
    direction: u32,
    token: u32,
    filename: String,
    size: Option<u64>,
    reason: String,
) {
    let mut transfers = state.transfers.write().await;
    transfers.record_rejected_request(direction, token, filename, size, reason);
    drop(transfers);
    persist_transfer_durability(state).await;
}

pub(super) async fn persist_transfer_durability(state: &AppState) {
    let snapshot = state.transfers.read().await.durability_snapshot();
    let Some(snapshot) = snapshot else {
        return;
    };
    let (events_attempted, events_error, state_error) = {
        let mut coordinator = snapshot.coordinator.state.lock().await;
        if snapshot.generation <= coordinator.last_attempted_generation {
            return;
        }
        coordinator.last_attempted_generation = snapshot.generation;

        let mut events_error = None;
        for entry in &snapshot.events {
            if let Err(error) = append_transfer_event(&snapshot.events_path, entry) {
                events_error.get_or_insert(error);
            }
        }
        let state_error = write_transfer_state(&snapshot.state_path, &snapshot.entries).err();
        (!snapshot.events.is_empty(), events_error, state_error)
    };

    let latest_generation = if let Ok(mut states) = TRANSFER_DURABILITY_STATES.lock() {
        let Some(durability) = states.get_mut(&snapshot.state_path) else {
            return;
        };
        durability
            .pending_events
            .retain(|(generation, _)| *generation > snapshot.generation);
        if durability
            .state_generation
            .is_some_and(|generation| generation <= snapshot.generation)
        {
            durability.state_generation = None;
        }
        durability.generation
    } else {
        return;
    };

    let mut transfers = state.transfers.write().await;
    if snapshot.generation >= latest_generation {
        if events_attempted {
            transfers.events_error = events_error;
        }
        transfers.state_error = state_error;
    }
}

pub(super) async fn persist_transfer_record(
    state: &AppState,
    entry: &TransferEntry,
) -> Result<(), String> {
    persist_transfer_record_with_hub_event(state, entry, "activity").await
}

pub(super) async fn persist_transfer_progress_record(
    state: &AppState,
    entry: &TransferEntry,
) -> Result<(), String> {
    persist_transfer_record_with_hub_event(state, entry, "progress").await
}

async fn persist_transfer_record_with_hub_event(
    state: &AppState,
    entry: &TransferEntry,
    hub_event_kind: &str,
) -> Result<(), String> {
    persist_transfer_durability(state).await;
    if let Some(db) = state.db.as_ref() {
        let records = [(
            persisted_transfer_record(entry),
            persisted_transfer_event_record(entry),
        )];
        db.insert_transfer_records_with_events(&records)
            .await
            .map_err(|error| format!("failed to persist transfer and event: {error}"))?;
    }
    publish_transfer_hub_event(state, hub_event_kind, entry);
    Ok(())
}

pub(super) async fn persist_transfer_projection(state: &AppState, entry: &TransferEntry) {
    if let Err(error) = persist_transfer_record(state, entry).await {
        update_session(state, |snapshot| {
            snapshot.last_error =
                Some(format!("transfer {} persistence failed: {error}", entry.id));
        })
        .await;
    }
}

pub(super) async fn persist_transfer_records(
    state: &AppState,
    entries: &[TransferEntry],
) -> Result<(), String> {
    persist_transfer_durability(state).await;
    if let Some(db) = state.db.as_ref() {
        let records = entries
            .iter()
            .map(|entry| {
                (
                    persisted_transfer_record(entry),
                    persisted_transfer_event_record(entry),
                )
            })
            .collect::<Vec<_>>();
        db.insert_transfer_records_with_events(&records)
            .await
            .map_err(|error| format!("failed to persist transfers and events: {error}"))?;
    }
    for entry in entries {
        publish_transfer_hub_event(state, "activity", entry);
    }
    Ok(())
}

pub(super) async fn delete_persisted_transfers(
    state: &AppState,
    entries: &[TransferEntry],
) -> Result<(), String> {
    persist_transfer_durability(state).await;
    if let Some(db) = state.db.as_ref() {
        let now = unix_timestamp_millis();
        let records = entries
            .iter()
            .map(|entry| {
                (
                    entry.id.to_string(),
                    database_i64(now.max(entry.updated_at_ms.saturating_add(1))),
                )
            })
            .collect::<Vec<_>>();
        db.delete_transfer_records(&records)
            .await
            .map_err(|error| format!("failed to delete persisted transfers: {error}"))?;
    }
    for entry in entries {
        publish_transfer_hub_event(state, "removed", entry);
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct TransferPersistenceCoordinator {
    pub(super) state: AsyncMutex<TransferPersistenceCoordinatorState>,
}

#[derive(Debug, Default)]
pub(super) struct TransferPersistenceCoordinatorState {
    last_attempted_generation: u64,
}

#[derive(Debug)]
pub(super) struct TransferDurabilitySnapshot {
    generation: u64,
    entries: Vec<TransferEntry>,
    events: Vec<TransferEntry>,
    events_path: PathBuf,
    state_path: PathBuf,
    pub(super) coordinator: Arc<TransferPersistenceCoordinator>,
}

#[derive(Debug, Default)]
struct TransferDurabilityState {
    pending_events: Vec<(u64, TransferEntry)>,
    generation: u64,
    state_generation: Option<u64>,
    coordinator: Arc<TransferPersistenceCoordinator>,
}

static TRANSFER_DURABILITY_STATES: LazyLock<Mutex<HashMap<PathBuf, TransferDurabilityState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(super) fn reset_transfer_durability_state(path: &Path) {
    if let Ok(mut states) = TRANSFER_DURABILITY_STATES.lock() {
        states.remove(path);
    }
}

pub(super) async fn rollback_transfer_mutation_if_unchanged(
    state: &AppState,
    previous: TransferMutationSnapshot,
    mutated: TransferMutationSnapshot,
) -> bool {
    let entries_changed = {
        let mut transfers = state.transfers.write().await;
        let entries_changed = transfers.rollback_mutation_if_unchanged(&previous, &mutated);
        if entries_changed {
            transfers.persist_state();
        }
        entries_changed
    };
    if entries_changed {
        persist_transfer_durability(state).await;
    }
    entries_changed
}

pub(super) async fn remove_transfer_entries_if_unchanged(
    state: &AppState,
    expected: &[TransferEntry],
) -> Vec<TransferEntry> {
    let removed = {
        let mut transfers = state.transfers.write().await;
        let ids = expected
            .iter()
            .filter(|expected| {
                transfers
                    .entries
                    .iter()
                    .any(|current| current.id == expected.id && current == *expected)
            })
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        transfers.remove_entries(&ids)
    };
    if !removed.is_empty() {
        persist_transfer_durability(state).await;
    }
    removed
}

#[derive(Debug)]
pub(super) struct TransferMutationSnapshot {
    entries: Vec<TransferEntry>,
    progress_persisted_at: BTreeMap<u64, u64>,
    next_id: u64,
    next_token: u32,
}

#[allow(dead_code)]
impl TransferQueue {
    pub(super) fn mutation_snapshot(&self) -> TransferMutationSnapshot {
        TransferMutationSnapshot {
            entries: self.entries.clone(),
            progress_persisted_at: self.progress_persisted_at.clone(),
            next_id: self.next_id,
            next_token: self.next_token,
        }
    }

    fn rollback_mutation_if_unchanged(
        &mut self,
        previous: &TransferMutationSnapshot,
        mutated: &TransferMutationSnapshot,
    ) -> bool {
        let previous_by_id = previous
            .entries
            .iter()
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();
        let mutated_by_id = mutated
            .entries
            .iter()
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();
        let mut restored_ids = HashSet::new();
        let mut entries_changed = false;

        for (previous_index, previous_entry) in previous.entries.iter().enumerate() {
            let id = previous_entry.id;
            let mutated_entry = mutated_by_id.get(&id).copied();
            if mutated_entry == Some(previous_entry) {
                continue;
            }
            let current_index = self.entries.iter().position(|entry| entry.id == id);
            match (mutated_entry, current_index) {
                (Some(mutated_entry), Some(current_index))
                    if self.entries[current_index] == *mutated_entry =>
                {
                    self.entries[current_index] = previous_entry.clone();
                    restored_ids.insert(id);
                    entries_changed = true;
                }
                (None, None) => {
                    self.entries.insert(
                        previous_index.min(self.entries.len()),
                        previous_entry.clone(),
                    );
                    restored_ids.insert(id);
                    entries_changed = true;
                }
                _ => {}
            }
        }

        let mut all_created_entries_were_rolled_back = true;
        for mutated_entry in &mutated.entries {
            if previous_by_id.contains_key(&mutated_entry.id) {
                continue;
            }
            match self
                .entries
                .iter()
                .position(|entry| entry.id == mutated_entry.id)
            {
                Some(index) if self.entries[index] == *mutated_entry => {
                    self.entries.remove(index);
                    restored_ids.insert(mutated_entry.id);
                    entries_changed = true;
                }
                _ => all_created_entries_were_rolled_back = false,
            }
        }

        for id in restored_ids {
            let previous_progress = previous.progress_persisted_at.get(&id).copied();
            let mutated_progress = mutated.progress_persisted_at.get(&id).copied();
            if previous_progress == mutated_progress
                || self.progress_persisted_at.get(&id).copied() != mutated_progress
            {
                continue;
            }
            if let Some(progress) = previous_progress {
                self.progress_persisted_at.insert(id, progress);
            } else {
                self.progress_persisted_at.remove(&id);
            }
        }

        if all_created_entries_were_rolled_back {
            if self.next_id == mutated.next_id {
                self.next_id = previous.next_id;
            }
            if self.next_token == mutated.next_token {
                self.next_token = previous.next_token;
            }
        }

        entries_changed
    }

    pub(super) fn mark_durable_mutation(&mut self, event: Option<TransferEntry>) {
        if let Ok(mut states) = TRANSFER_DURABILITY_STATES.lock() {
            let state = states.entry(self.state_path.clone()).or_default();
            state.generation = state.generation.saturating_add(1).max(1);
            let generation = state.generation;
            state.state_generation = Some(generation);
            if let Some(event) = event {
                state.pending_events.push((generation, event));
            }
        }
        // Standalone synchronous queue users have no AppState task that can
        // flush a snapshot after the mutation; retain their immediate file
        // visibility without affecting async lock-held mutation paths.
        if tokio::runtime::Handle::try_current().is_err() {
            self.persist_durability_without_runtime();
        }
    }

    pub(super) fn mark_state_dirty(&mut self) {
        self.mark_durable_mutation(None);
    }

    pub(super) fn persist_state(&mut self) {
        self.mark_state_dirty();
    }

    pub(super) fn persist_event(&mut self, entry: &TransferEntry) {
        self.mark_durable_mutation(Some(entry.clone()));
    }

    pub(super) fn durability_snapshot(&self) -> Option<TransferDurabilitySnapshot> {
        let states = TRANSFER_DURABILITY_STATES.lock().ok()?;
        let state = states.get(&self.state_path)?;
        let generation = state.state_generation?;
        Some(TransferDurabilitySnapshot {
            generation,
            entries: self.entries.clone(),
            events: state
                .pending_events
                .iter()
                .map(|(_, entry)| entry.clone())
                .collect(),
            events_path: self.events_path.clone(),
            state_path: self.state_path.clone(),
            coordinator: Arc::clone(&state.coordinator),
        })
    }

    fn persist_durability_without_runtime(&mut self) {
        let Some(snapshot) = self.durability_snapshot() else {
            return;
        };
        let mut events_error = None;
        for entry in &snapshot.events {
            if let Err(error) = append_transfer_event(&snapshot.events_path, entry) {
                events_error.get_or_insert(error);
            }
        }
        self.state_error = write_transfer_state(&snapshot.state_path, &snapshot.entries).err();
        if !snapshot.events.is_empty() {
            self.events_error = events_error;
        }
        if let Ok(mut states) = TRANSFER_DURABILITY_STATES.lock() {
            if let Some(state) = states.get_mut(&snapshot.state_path) {
                state
                    .pending_events
                    .retain(|(generation, _)| *generation > snapshot.generation);
                if state
                    .state_generation
                    .is_some_and(|generation| generation <= snapshot.generation)
                {
                    state.state_generation = None;
                }
            }
        }
    }

    pub(super) fn should_persist_progress(&mut self, entry: &TransferEntry) -> bool {
        const PROGRESS_PERSIST_INTERVAL_MS: u64 = 1_000;
        let should_persist = self
            .progress_persisted_at
            .get(&entry.id)
            .is_none_or(|persisted_at| {
                entry.updated_at_ms.saturating_sub(*persisted_at) >= PROGRESS_PERSIST_INTERVAL_MS
            });
        if should_persist {
            self.progress_persisted_at
                .insert(entry.id, entry.updated_at_ms);
        }
        should_persist
    }
}
