use super::*;

const MAX_CONTROLLER_FEATURE_RECORDS: usize = 4_096;
const MAX_CONTROLLER_FEATURE_STATE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct ControllerFeatureStateFile {
    pub(super) version: u32,
    pub(super) records: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug)]
pub(super) struct ControllerFeatureState {
    pub(super) records: BTreeMap<String, serde_json::Value>,
    pub(super) state_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct ControllerFeatureStore {
    state: Arc<RwLock<ControllerFeatureState>>,
    mutation_turn: Arc<AsyncMutex<()>>,
}

impl ControllerFeatureStore {
    pub(super) fn new(state: ControllerFeatureState) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
            mutation_turn: Arc::new(AsyncMutex::new(())),
        }
    }

    pub(super) async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, ControllerFeatureState> {
        self.state.read().await
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    #[allow(
        dead_code,
        reason = "only controller test feature groups need direct fixture writes"
    )]
    pub(super) async fn write_for_test(
        &self,
    ) -> tokio::sync::RwLockWriteGuard<'_, ControllerFeatureState> {
        self.state.write().await
    }

    pub(super) async fn mutate<T, F>(&self, mutation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut ControllerFeatureState) -> Result<T, String> + Send + 'static,
    {
        let mutation_turn = Arc::clone(&self.mutation_turn).lock_owned().await;
        let state = Arc::clone(&self.state);
        let snapshot = Arc::clone(&state).read_owned().await;
        tokio::task::spawn_blocking(move || {
            let mut candidate = (*snapshot).clone();
            drop(snapshot);

            // Mutations persist their candidate before publishing it. Keeping
            // the ordered turn in this non-cancellable worker means an aborted
            // request cannot let a later snapshot overtake an in-flight write.
            let result = mutation(&mut candidate);
            *state.blocking_write() = candidate;
            drop(mutation_turn);
            result
        })
        .await
        .map_err(|error| format!("controller feature mutation worker failed: {error}"))?
    }

    pub(super) async fn upsert(&self, key: String, value: serde_json::Value) -> Result<(), String> {
        self.mutate(move |state| state.upsert(key, value)).await
    }

    pub(super) async fn remove(&self, key: &str) -> Result<Option<serde_json::Value>, String> {
        let key = key.to_owned();
        self.mutate(move |state| state.remove(&key)).await
    }

    pub(super) async fn remove_keys(&self, keys: &[String]) -> Result<usize, String> {
        let keys = keys.to_vec();
        self.mutate(move |state| state.remove_keys(&keys)).await
    }

    pub(super) async fn remove_prefix(&self, prefix: &str) -> Result<usize, String> {
        let prefix = prefix.to_owned();
        self.mutate(move |state| state.remove_prefix(&prefix)).await
    }

    pub(super) async fn record_warm_cache_accesses(
        &self,
        content_ids: &[String],
    ) -> Result<(), String> {
        let content_ids = content_ids.to_vec();
        self.mutate(move |state| state.record_warm_cache_accesses(&content_ids))
            .await
    }

    pub(super) async fn import_soulseek_interests(
        &self,
        username: &str,
        liked: &[String],
        hated: &[String],
    ) -> Result<(), String> {
        let username = username.to_owned();
        let liked = liked.to_vec();
        let hated = hated.to_vec();
        self.mutate(move |state| state.import_soulseek_interests(&username, &liked, &hated))
            .await
    }
}

impl ControllerFeatureState {
    pub(super) fn load(state_dir: &Path) -> Result<Self, String> {
        let state_path = state_dir.join("controller-feature-state.json");
        let records = if state_path.exists() {
            let metadata = fs::metadata(&state_path)
                .map_err(|error| format!("controller feature state metadata failed: {error}"))?;
            if metadata.len() > MAX_CONTROLLER_FEATURE_STATE_BYTES {
                return Err("controller feature state exceeds the 8 MiB limit".to_owned());
            }
            let bytes = fs::read(&state_path)
                .map_err(|error| format!("controller feature state read failed: {error}"))?;
            let file = serde_json::from_slice::<ControllerFeatureStateFile>(&bytes)
                .map_err(|error| format!("controller feature state parse failed: {error}"))?;
            if file.version != 1 || file.records.len() > MAX_CONTROLLER_FEATURE_RECORDS {
                return Err("controller feature state is unsupported or over capacity".to_owned());
            }
            file.records
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            records,
            state_path,
        })
    }

    pub(super) fn validate_storage(&self) -> Result<(), String> {
        let metadata = match fs::symlink_metadata(&self.state_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(format!("controller feature state metadata failed: {error}"));
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("controller feature state path must be a regular file".to_owned());
        }
        Ok(())
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    pub(super) fn in_memory() -> Self {
        let unique = uuid::Uuid::new_v4().simple().to_string();
        Self {
            records: BTreeMap::new(),
            state_path: std::env::temp_dir()
                .join(format!("slskr-controller-features-{unique}.json")),
        }
    }

    pub(super) fn upsert(&mut self, key: String, value: serde_json::Value) -> Result<(), String> {
        if self.records.len() >= MAX_CONTROLLER_FEATURE_RECORDS && !self.records.contains_key(&key)
        {
            return Err("controller feature record capacity is full".to_owned());
        }
        let previous = self.records.insert(key.clone(), value);
        if let Err(error) = self.persist() {
            match previous {
                Some(previous) => {
                    self.records.insert(key, previous);
                }
                None => {
                    self.records.remove(&key);
                }
            }
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn remove(&mut self, key: &str) -> Result<Option<serde_json::Value>, String> {
        let removed = self.records.remove(key);
        if removed.is_some() {
            if let Err(error) = self.persist() {
                self.records
                    .insert(key.to_owned(), removed.clone().unwrap_or_default());
                return Err(error);
            }
        }
        Ok(removed)
    }

    pub(super) fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.records.get(key)
    }

    pub(super) fn values_with_prefix(&self, prefix: &str) -> Vec<serde_json::Value> {
        self.entries_with_prefix(prefix)
            .into_iter()
            .map(|(_, value)| value)
            .collect()
    }

    pub(super) fn entries_with_prefix(&self, prefix: &str) -> Vec<(String, serde_json::Value)> {
        self.records
            .range(prefix.to_owned()..)
            .take_while(|(key, _)| key.starts_with(prefix))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub(super) fn remove_keys(&mut self, keys: &[String]) -> Result<usize, String> {
        let previous = self.records.clone();
        let removed = keys
            .iter()
            .filter(|key| self.records.remove(key.as_str()).is_some())
            .count();
        if removed > 0 {
            if let Err(error) = self.persist() {
                self.records = previous;
                return Err(error);
            }
        }
        Ok(removed)
    }

    pub(super) fn remove_prefix(&mut self, prefix: &str) -> Result<usize, String> {
        let previous = self.records.clone();
        let before = self.records.len();
        self.records.retain(|key, _| !key.starts_with(prefix));
        let removed = before.saturating_sub(self.records.len());
        if removed > 0 {
            if let Err(error) = self.persist() {
                self.records = previous;
                return Err(error);
            }
        }
        Ok(removed)
    }

    pub(super) fn record_warm_cache_accesses(
        &mut self,
        content_ids: &[String],
    ) -> Result<(), String> {
        let previous = self.records.clone();
        let now = unix_timestamp();
        for content_id in content_ids {
            let key = format!("warm-cache/popularity/{content_id}");
            let hits = self
                .records
                .get(&key)
                .and_then(|value| value.get("hits"))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .saturating_add(1);
            if self.records.len() >= MAX_CONTROLLER_FEATURE_RECORDS
                && !self.records.contains_key(&key)
            {
                self.records = previous;
                return Err("warm cache popularity capacity is full".to_owned());
            }
            self.records.insert(
                key,
                serde_json::json!({
                    "contentId": content_id,
                    "hits": hits,
                    "lastUpdated": now,
                }),
            );
        }
        if let Err(error) = self.persist() {
            self.records = previous;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn import_soulseek_interests(
        &mut self,
        username: &str,
        liked: &[String],
        hated: &[String],
    ) -> Result<(), String> {
        let issuer = format!("soulseek:{username}");
        let previous = self.records.clone();
        self.records.retain(|key, value| {
            !(key.starts_with("opinion/")
                && value.get("issuer").and_then(serde_json::Value::as_str) == Some(issuer.as_str())
                && value.get("source").and_then(serde_json::Value::as_str)
                    == Some("soulseek-interest"))
        });
        let now = unix_timestamp().saturating_mul(1000);
        for (kind, values) in [("Like", liked), ("Hate", hated)] {
            for interest in values.iter().take(MAX_INTERESTS_PER_KIND) {
                let interest = interest.trim();
                if interest.is_empty()
                    || interest.len() > MAX_INTEREST_NAME_BYTES
                    || interest.chars().any(char::is_control)
                {
                    continue;
                }
                let (subject_type, subject_id) = interest
                    .split_once(':')
                    .map(|(prefix, value)| {
                        let subject_type = match prefix.to_ascii_lowercase().as_str() {
                            "artist" => "Artist",
                            "album" => "Album",
                            "track" => "Track",
                            "user" => "User",
                            "file" => "File",
                            _ => "Track",
                        };
                        (subject_type, value.trim())
                    })
                    .unwrap_or(("Track", interest));
                if subject_id.is_empty() {
                    continue;
                }
                let id = uuid::Uuid::new_v4().to_string();
                if self.records.len() >= MAX_CONTROLLER_FEATURE_RECORDS {
                    self.records = previous;
                    return Err("opinion record capacity is full".to_owned());
                }
                self.records.insert(
                    format!("opinion/{id}"),
                    serde_json::json!({
                        "id": id,
                        "issuer": issuer,
                        "subjectType": subject_type,
                        "subjectId": subject_id,
                        "kind": kind,
                        "strength": 0.25,
                        "confidence": 0.25,
                        "scope": "soulseek-public",
                        "source": "soulseek-interest",
                        "reason": "Soulseek user interest",
                        "createdUnixMs": now,
                        "updatedUnixMs": now,
                    }),
                );
            }
        }
        if let Err(error) = self.persist() {
            self.records = previous;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn persist(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec(&ControllerFeatureStateFile {
            version: 1,
            records: self.records.clone(),
        })
        .map_err(|error| format!("controller feature state serialization failed: {error}"))?;
        if bytes.len() as u64 > MAX_CONTROLLER_FEATURE_STATE_BYTES {
            return Err("controller feature state exceeds the 8 MiB limit".to_owned());
        }
        write_file_atomic(&self.state_path, &bytes)
            .map_err(|error| format!("controller feature state write failed: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{ControllerFeatureState, ControllerFeatureStore};
    use std::time::Duration;
    use tokio::sync::oneshot;

    #[tokio::test(flavor = "current_thread")]
    async fn controller_feature_mutation_keeps_reads_open_and_finishes_cancelled_writes_in_order() {
        let initial = ControllerFeatureState::in_memory();
        let state_path = initial.state_path.clone();
        let store = ControllerFeatureStore::new(initial);
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let first_store = store.clone();
        let first = tokio::spawn(async move {
            first_store
                .mutate(move |state| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    state.upsert("ordered".to_owned(), serde_json::json!(1))
                })
                .await
        });

        entered_rx.await.unwrap();
        let read = tokio::time::timeout(Duration::from_secs(2), store.read())
            .await
            .expect("reads should stay available while a mutation is doing blocking work");
        assert!(read.get("ordered").is_none());
        drop(read);

        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());

        let (second_started_tx, second_started_rx) = oneshot::channel();
        let second_store = store.clone();
        let second = tokio::spawn(async move {
            let _ = second_started_tx.send(());
            second_store
                .upsert("ordered".to_owned(), serde_json::json!(2))
                .await
        });
        second_started_rx.await.unwrap();
        tokio::task::yield_now().await;
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), second)
            .await
            .expect("the later writer should complete after the cancelled writer")
            .unwrap()
            .unwrap();

        let state = store.read().await;
        assert_eq!(state.get("ordered"), Some(&serde_json::json!(2)));
        drop(state);

        let persisted = serde_json::from_slice::<super::ControllerFeatureStateFile>(
            &std::fs::read(&state_path).unwrap(),
        )
        .unwrap();
        assert_eq!(
            persisted.records.get("ordered"),
            Some(&serde_json::json!(2))
        );
        std::fs::remove_file(state_path).unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn controller_feature_mutation_publishes_successful_prefix_after_later_write_failure() {
        let initial = ControllerFeatureState::in_memory();
        let state_path = initial.state_path.clone();
        let backup_path = state_path.with_extension("previous");
        let store = ControllerFeatureStore::new(initial);
        let mutation_path = state_path.clone();
        let mutation_backup = backup_path.clone();
        let result = store
            .mutate(move |state| {
                state.upsert("committed".to_owned(), serde_json::json!(1))?;
                std::fs::rename(&mutation_path, &mutation_backup)
                    .map_err(|error| error.to_string())?;
                std::fs::create_dir(&mutation_path).map_err(|error| error.to_string())?;
                state.upsert("rolled-back".to_owned(), serde_json::json!(2))
            })
            .await;
        assert!(result.is_err());

        let state = store.read().await;
        assert_eq!(state.get("committed"), Some(&serde_json::json!(1)));
        assert!(state.get("rolled-back").is_none());
        drop(state);

        std::fs::remove_dir(&state_path).unwrap();
        std::fs::rename(&backup_path, &state_path).unwrap();
        let persisted = serde_json::from_slice::<super::ControllerFeatureStateFile>(
            &std::fs::read(&state_path).unwrap(),
        )
        .unwrap();
        assert_eq!(
            persisted.records.get("committed"),
            Some(&serde_json::json!(1))
        );
        assert!(!persisted.records.contains_key("rolled-back"));
        std::fs::remove_file(state_path).unwrap();
    }
}
