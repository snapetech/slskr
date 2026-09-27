use std::{collections::VecDeque, fs, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{secure_oauth_state, unix_timestamp};

pub(super) const MAX_SOURCE_FEED_IMPORT_HISTORY: usize = 100;
pub(super) const MAX_SOURCE_FEED_HISTORY_SUGGESTIONS: usize = 25;
pub(super) const MAX_SOURCE_FEED_HISTORY_SKIPPED_ROWS: usize = 25;
pub(super) const MAX_SOURCE_FEED_PREVIEW_LENGTH: usize = 160;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SpotifyConnectionStore {
    pub(super) access_token: String,
    pub(super) refresh_token: String,
    pub(super) scope: String,
    pub(super) expires_at: i64,
    pub(super) display_name: String,
    pub(super) spotify_user_id: String,
}

impl SpotifyConnectionStore {
    pub(super) fn status_json(&self, configured: bool) -> serde_json::Value {
        let connected = !self.refresh_token.trim().is_empty();
        let expires_at = connected
            .then(|| chrono::DateTime::from_timestamp(self.expires_at, 0))
            .flatten()
            .map(|value| value.to_rfc3339());
        let mut value = serde_json::json!({
            "configured": configured,
            "connected": connected,
            "displayName": if connected { self.display_name.as_str() } else { "" },
            "spotifyUserId": if connected { self.spotify_user_id.as_str() } else { "" },
            "scope": if connected { self.scope.as_str() } else { "" },
            "expiresAt": expires_at,
        });
        value
            .as_object_mut()
            .expect("Spotify connection status is an object")
            .retain(|_, candidate| !candidate.is_null());
        value
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SpotifySourceTarget {
    pub(super) kind: String,
    pub(super) id: String,
    pub(super) requires_user_token: bool,
    pub(super) scope_hint: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SpotifySourceRow {
    pub(super) title: String,
    pub(super) artist: String,
    pub(super) album: String,
    pub(super) source: String,
    pub(super) source_id: String,
    pub(super) provider_url: String,
    pub(super) raw_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProtectedSpotifyConnection {
    pub(super) version: u32,
    pub(super) nonce: String,
    pub(super) ciphertext: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct SourceFeedImportHistoryStore {
    #[serde(default)]
    pub(super) history: VecDeque<serde_json::Value>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct LidarrSyncRuntimeState {
    pub(super) is_syncing: bool,
    pub(super) last_sync_at: Option<String>,
    pub(super) next_sync_at: Option<String>,
    pub(super) last_error: Option<String>,
    pub(super) last_result: Option<serde_json::Value>,
}

impl LidarrSyncRuntimeState {
    pub(super) fn new(settings: &crate::config::LidarrIntegrationSettings) -> Self {
        let next_sync_at = settings.enabled.then(|| {
            (chrono::Utc::now()
                + chrono::Duration::seconds(
                    i64::try_from(settings.sync_interval_seconds).unwrap_or(i64::MAX),
                ))
            .to_rfc3339()
        });
        Self {
            next_sync_at,
            ..Self::default()
        }
    }

    pub(super) fn json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({
            "isSyncing": self.is_syncing,
            "lastSyncAt": self.last_sync_at,
            "nextSyncAt": self.next_sync_at,
            "lastError": self.last_error,
            "lastResult": self.last_result,
        });
        value
            .as_object_mut()
            .expect("Lidarr sync state is an object")
            .retain(|_, candidate| !candidate.is_null());
        value
    }
}

impl SourceFeedImportHistoryStore {
    pub(super) fn load(state_dir: &Path) -> Self {
        let path = state_dir.join("source-feed-import-history.json");
        let Ok(json) = fs::read_to_string(path) else {
            return Self::default();
        };
        let mut store = serde_json::from_str::<Self>(&json).unwrap_or_default();
        store.history.truncate(MAX_SOURCE_FEED_IMPORT_HISTORY);
        store
    }

    pub(super) fn record(
        &mut self,
        request: &serde_json::Value,
        source_text: &str,
        result: &serde_json::Value,
    ) {
        let random = secure_oauth_state().unwrap_or_else(|| unix_timestamp().to_string());
        let seed = format!(
            "{random}|{}|{}|{}|{}",
            result["provider"].as_str().unwrap_or("local"),
            result["sourceKind"].as_str().unwrap_or("auto"),
            result["sourceId"].as_str().unwrap_or_default(),
            source_text.trim(),
        );
        let import_id = hex::encode(Sha256::digest(seed.as_bytes()))[..16].to_owned();
        let source_fingerprint = hex::encode(Sha256::digest(source_text.trim().as_bytes()));
        let compact_preview = source_text.split_whitespace().collect::<Vec<_>>().join(" ");
        let source_preview = compact_preview
            .chars()
            .take(MAX_SOURCE_FEED_PREVIEW_LENGTH)
            .collect::<String>();
        let suggestions = result["suggestions"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .take(MAX_SOURCE_FEED_HISTORY_SUGGESTIONS)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let skipped_rows = result["skippedRows"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .take(MAX_SOURCE_FEED_HISTORY_SKIPPED_ROWS)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let entry = serde_json::json!({
            "importId": import_id,
            "importedAt": chrono::Utc::now().to_rfc3339(),
            "provider": result["provider"],
            "sourceKind": result["sourceKind"],
            "sourceId": result["sourceId"],
            "sourceFingerprint": source_fingerprint,
            "sourcePreview": source_preview,
            "limit": request.get("limit").and_then(serde_json::Value::as_i64).unwrap_or(500),
            "includeAlbum": request.get("includeAlbum").and_then(serde_json::Value::as_bool).unwrap_or(false),
            "fetchProviderUrls": request.get("fetchProviderUrls").and_then(serde_json::Value::as_bool).unwrap_or(true),
            "totalRows": result["totalRows"],
            "suggestionCount": result["suggestionCount"],
            "duplicateCount": result["duplicateCount"],
            "skippedCount": result["skippedCount"],
            "networkRequestCount": result["networkRequestCount"],
            "requiresAccessToken": result["requiresAccessToken"],
            "requiredScopeHint": result["requiredScopeHint"],
            "suggestions": suggestions,
            "skippedRows": skipped_rows,
        });
        self.history.push_front(entry);
        self.history.truncate(MAX_SOURCE_FEED_IMPORT_HISTORY);
    }

    pub(super) fn persist(&self, state_dir: &Path) -> Result<(), String> {
        let path = state_dir.join("source-feed-import-history.json");
        let temporary = state_dir.join("source-feed-import-history.json.tmp");
        let json = serde_json::to_vec(self)
            .map_err(|error| format!("source feed history serialization failed: {error}"))?;
        fs::write(&temporary, json)
            .map_err(|error| format!("source feed history write failed: {error}"))?;
        fs::rename(&temporary, &path)
            .map_err(|error| format!("source feed history publish failed: {error}"))
    }

    pub(super) async fn record_and_persist(
        history: &tokio::sync::RwLock<Self>,
        persistence_lock: &tokio::sync::Mutex<()>,
        state_dir: &Path,
        request: &serde_json::Value,
        source_text: &str,
        result: &serde_json::Value,
    ) -> Result<(), String> {
        let _persistence_turn = persistence_lock.lock().await;
        let (previous, mutated) = {
            let mut history = history.write().await;
            let previous = history.clone();
            history.record(request, source_text, result);
            (previous, history.clone())
        };

        if let Err(error) = mutated.persist(state_dir) {
            let mut history = history.write().await;
            if *history == mutated {
                *history = previous;
            }
            return Err(error);
        }

        Ok(())
    }
}

#[cfg(test)]
mod source_feed_history_tests {
    use super::*;
    use std::future::Future;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::task::Poll;

    static NEXT_TEST_STATE_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestStateDir(PathBuf);

    impl TestStateDir {
        fn new() -> Self {
            let id = NEXT_TEST_STATE_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "slskr-source-feed-history-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create source-feed history state directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestStateDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn queued_history_writes_keep_reads_available_and_persist_in_order() {
        let state_dir = TestStateDir::new();
        let history = tokio::sync::RwLock::new(SourceFeedImportHistoryStore::default());
        let persistence_lock = tokio::sync::Mutex::new(());
        let held_turn = persistence_lock.lock().await;

        let first_request = serde_json::json!({"limit": 500});
        let first_result = serde_json::json!({
            "provider": "local",
            "sourceKind": "text",
            "sourceId": "first",
            "totalRows": 1,
            "suggestionCount": 1,
            "duplicateCount": 0,
            "skippedCount": 0,
            "networkRequestCount": 0,
            "requiresAccessToken": false,
            "requiredScopeHint": "",
            "suggestions": [{"title": "First track", "artist": "First artist"}],
            "skippedRows": []
        });
        let second_request = serde_json::json!({"limit": 500});
        let second_result = serde_json::json!({
            "provider": "local",
            "sourceKind": "text",
            "sourceId": "second",
            "totalRows": 1,
            "suggestionCount": 1,
            "duplicateCount": 0,
            "skippedCount": 0,
            "networkRequestCount": 0,
            "requiresAccessToken": false,
            "requiredScopeHint": "",
            "suggestions": [{"title": "Second track", "artist": "Second artist"}],
            "skippedRows": []
        });

        let first = SourceFeedImportHistoryStore::record_and_persist(
            &history,
            &persistence_lock,
            state_dir.path(),
            &first_request,
            "First artist - First track",
            &first_result,
        );
        let second = SourceFeedImportHistoryStore::record_and_persist(
            &history,
            &persistence_lock,
            state_dir.path(),
            &second_request,
            "Second artist - Second track",
            &second_result,
        );
        tokio::pin!(first, second);

        let both_waiting = std::future::poll_fn(|context| {
            let first_waiting = first.as_mut().poll(context).is_pending();
            let second_waiting = second.as_mut().poll(context).is_pending();
            Poll::Ready(first_waiting && second_waiting)
        })
        .await;
        assert!(
            both_waiting,
            "a history writer passed the held persistence turn"
        );

        assert!(history.read().await.history.is_empty());
        assert!(SourceFeedImportHistoryStore::load(state_dir.path())
            .history
            .is_empty());

        drop(held_turn);
        let (first_write, second_write) = tokio::join!(&mut first, &mut second);
        first_write.expect("first source-feed history write");
        second_write.expect("second source-feed history write");

        let memory = history.read().await.clone();
        let persisted = SourceFeedImportHistoryStore::load(state_dir.path());
        assert_eq!(memory, persisted);
        assert_eq!(memory.history.len(), 2);
        assert_eq!(memory.history[0]["sourceId"], "second");
        assert_eq!(memory.history[1]["sourceId"], "first");

        let missing_state_dir = state_dir.path().join("missing");
        let error = SourceFeedImportHistoryStore::record_and_persist(
            &history,
            &persistence_lock,
            &missing_state_dir,
            &second_request,
            "Failed artist - Failed track",
            &second_result,
        )
        .await
        .expect_err("source-feed persistence fails for a missing state directory");
        assert!(error.contains("source feed history write failed"));
        assert_eq!(history.read().await.clone(), memory);
        assert_eq!(SourceFeedImportHistoryStore::load(state_dir.path()), memory);
    }
}
