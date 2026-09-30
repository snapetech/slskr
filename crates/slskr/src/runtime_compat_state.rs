use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RuntimeCompatState {
    pub(super) application_restart_requested: bool,
    pub(super) application_reconnect_pending: bool,
    pub(super) vpn_reconnect_requested: bool,
    pub(super) vpn: vpn::Status,
    pub(super) gc_runs: u64,
    pub(super) autoreplace_enabled: bool,
    pub(super) accelerated_downloads_enabled: bool,
    pub(super) relay_agent_enabled: bool,
    pub(super) bridge_running: bool,
    pub(super) bridge_config_updates: u64,
    pub(super) bridge_started_at: Option<u64>,
    pub(super) bridge_active_clients: BTreeMap<String, serde_json::Value>,
    pub(super) bridge_total_connections: u64,
    pub(super) bridge_total_searches: u64,
    pub(super) bridge_total_downloads: u64,
    pub(super) bridge_total_room_joins: u64,
    pub(super) bridge_total_bytes_proxied: u64,
    pub(super) options_updates: u64,
    pub(super) options_yaml_uploads: u64,
    pub(super) options_yaml_validations: u64,
    pub(super) profile_invites_created: u64,
    pub(super) cache_warm_runs: u64,
    pub(super) backfill_runs: u64,
    pub(super) songid_runs: u64,
    pub(super) songid_run_records: Vec<serde_json::Value>,
    pub(super) lidarr_sync_runs: u64,
    pub(super) lidarr_manual_imports: u64,
    pub(super) updated_at: u64,
}

impl RuntimeCompatState {
    pub(super) fn new() -> Self {
        Self {
            application_restart_requested: false,
            application_reconnect_pending: false,
            vpn_reconnect_requested: false,
            vpn: vpn::Status::default(),
            gc_runs: 0,
            autoreplace_enabled: false,
            accelerated_downloads_enabled: false,
            relay_agent_enabled: false,
            bridge_running: false,
            bridge_config_updates: 0,
            bridge_started_at: None,
            bridge_active_clients: BTreeMap::new(),
            bridge_total_connections: 0,
            bridge_total_searches: 0,
            bridge_total_downloads: 0,
            bridge_total_room_joins: 0,
            bridge_total_bytes_proxied: 0,
            options_updates: 0,
            options_yaml_uploads: 0,
            options_yaml_validations: 0,
            profile_invites_created: 0,
            cache_warm_runs: 0,
            backfill_runs: 0,
            songid_runs: 0,
            songid_run_records: Vec::new(),
            lidarr_sync_runs: 0,
            lidarr_manual_imports: 0,
            updated_at: unix_timestamp(),
        }
    }

    pub(super) fn try_from_persisted(
        record: &crate::persistence::RuntimeCompatRecord,
    ) -> Result<Self, String> {
        let songid_run_records =
            serde_json::from_str::<Vec<serde_json::Value>>(&record.songid_run_records_json)
                .map_err(|error| {
                    format!("runtime compatibility SongID state is invalid: {error}")
                })?;
        if songid_run_records.len() > MAX_SONGID_RUNS {
            return Err(format!(
                "runtime compatibility SongID state exceeds the {}-run limit",
                MAX_SONGID_RUNS
            ));
        }
        Ok(Self {
            // These flags describe work owned by the current process. A
            // restart clears a pending restart request, and a bridge listener
            // must bind successfully before it is reported as running.
            application_restart_requested: false,
            application_reconnect_pending: false,
            vpn_reconnect_requested: false,
            vpn: vpn::Status::default(),
            gc_runs: u64::try_from(record.gc_runs).unwrap_or_default(),
            autoreplace_enabled: record.autoreplace_enabled,
            accelerated_downloads_enabled: false,
            relay_agent_enabled: record.relay_agent_enabled,
            bridge_running: false,
            bridge_config_updates: u64::try_from(record.bridge_config_updates).unwrap_or_default(),
            bridge_started_at: None,
            bridge_active_clients: BTreeMap::new(),
            bridge_total_connections: 0,
            bridge_total_searches: 0,
            bridge_total_downloads: 0,
            bridge_total_room_joins: 0,
            bridge_total_bytes_proxied: 0,
            options_updates: u64::try_from(record.options_updates).unwrap_or_default(),
            options_yaml_uploads: u64::try_from(record.options_yaml_uploads).unwrap_or_default(),
            options_yaml_validations: u64::try_from(record.options_yaml_validations)
                .unwrap_or_default(),
            profile_invites_created: u64::try_from(record.profile_invites_created)
                .unwrap_or_default(),
            cache_warm_runs: u64::try_from(record.cache_warm_runs).unwrap_or_default(),
            backfill_runs: u64::try_from(record.backfill_runs).unwrap_or_default(),
            songid_runs: u64::try_from(record.songid_runs).unwrap_or_default(),
            songid_run_records,
            lidarr_sync_runs: u64::try_from(record.lidarr_sync_runs).unwrap_or_default(),
            lidarr_manual_imports: u64::try_from(record.lidarr_manual_imports).unwrap_or_default(),
            updated_at: u64::try_from(record.updated_at).unwrap_or_else(|_| unix_timestamp()),
        })
    }

    #[allow(dead_code)]
    pub(super) fn from_persisted(record: &crate::persistence::RuntimeCompatRecord) -> Self {
        Self::try_from_persisted(record).unwrap_or_else(|_| Self::new())
    }

    pub(super) fn persistence_record(
        &self,
        relay: &RelayState,
    ) -> crate::persistence::RuntimeCompatRecord {
        crate::persistence::RuntimeCompatRecord {
            id: "runtime".to_owned(),
            // Restart requests and listener status are process-local latches.
            // Never let an unrelated durable compatibility write turn them
            // into work that is rehydrated after a process restart.
            application_restart_requested: false,
            gc_runs: i64::try_from(self.gc_runs).unwrap_or(i64::MAX),
            autoreplace_enabled: self.autoreplace_enabled,
            relay_enabled: relay.enabled,
            relay_agent_enabled: self.relay_agent_enabled,
            bridge_running: false,
            bridge_config_updates: i64::try_from(self.bridge_config_updates).unwrap_or(i64::MAX),
            options_updates: i64::try_from(self.options_updates).unwrap_or(i64::MAX),
            options_yaml_uploads: i64::try_from(self.options_yaml_uploads).unwrap_or(i64::MAX),
            options_yaml_validations: i64::try_from(self.options_yaml_validations)
                .unwrap_or(i64::MAX),
            profile_invites_created: i64::try_from(self.profile_invites_created)
                .unwrap_or(i64::MAX),
            cache_warm_runs: i64::try_from(self.cache_warm_runs).unwrap_or(i64::MAX),
            backfill_runs: i64::try_from(self.backfill_runs).unwrap_or(i64::MAX),
            songid_runs: i64::try_from(self.songid_runs).unwrap_or(i64::MAX),
            songid_run_records_json: serde_json::to_string(&self.songid_run_records)
                .unwrap_or_else(|_| "[]".to_owned()),
            lidarr_sync_runs: i64::try_from(self.lidarr_sync_runs).unwrap_or(i64::MAX),
            lidarr_manual_imports: i64::try_from(self.lidarr_manual_imports).unwrap_or(i64::MAX),
            updated_at: i64::try_from(self.updated_at.max(relay.updated_at)).unwrap_or(i64::MAX),
        }
    }

    pub(super) fn set_restart_requested(&mut self, requested: bool) -> serde_json::Value {
        self.application_restart_requested = requested;
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "accepted": true,
            "persisted": false,
            "pendingRestart": self.application_restart_requested,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn set_reconnect_pending(&mut self, pending: bool) {
        self.application_reconnect_pending = pending;
        self.updated_at = unix_timestamp();
    }

    pub(super) fn record_gc(&mut self) -> serde_json::Value {
        self.gc_runs = self.gc_runs.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "collected": true,
            "gcRuns": self.gc_runs,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn set_autoreplace(&mut self, enabled: bool) -> serde_json::Value {
        self.autoreplace_enabled = enabled;
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "enabled": self.autoreplace_enabled,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn set_relay_agent(&mut self, enabled: bool) -> serde_json::Value {
        self.relay_agent_enabled = enabled;
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "enabled": self.relay_agent_enabled,
            "relayAgentEnabled": self.relay_agent_enabled,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn set_bridge_running(
        &mut self,
        running: bool,
        configured: bool,
    ) -> serde_json::Value {
        self.bridge_running = running && configured;
        if !self.bridge_running {
            self.bridge_started_at = None;
            self.bridge_active_clients.clear();
        }
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "status": if configured {
                if self.bridge_running { "running" } else { "stopped" }
            } else {
                "disabled"
            },
            "started": self.bridge_running,
            "stopped": !self.bridge_running,
            "configured": configured,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn record_bridge_config_update(
        &mut self,
        configured: bool,
        accepted_keys: Vec<String>,
    ) -> serde_json::Value {
        self.bridge_config_updates = self.bridge_config_updates.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "enabled": configured,
            "configured": configured,
            "persisted": true,
            "restart_required": true,
            "acceptedKeys": accepted_keys,
            "configUpdates": self.bridge_config_updates,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn record_options_update(&mut self) -> u64 {
        self.options_updates = self.options_updates.saturating_add(1);
        self.updated_at = unix_timestamp();
        self.options_updates
    }

    pub(super) fn record_profile_invite(&mut self) -> serde_json::Value {
        self.profile_invites_created = self.profile_invites_created.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "count": self.profile_invites_created,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn record_cache_warm(&mut self, warmed: usize) -> serde_json::Value {
        self.cache_warm_runs = self.cache_warm_runs.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "status": if warmed == 0 { "empty" } else { "warmed" },
            "warmed": warmed,
            "runs": self.cache_warm_runs,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn record_backfill(&mut self, queued: usize) -> serde_json::Value {
        self.backfill_runs = self.backfill_runs.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "status": if queued == 0 { "idle" } else { "queued" },
            "queued": queued,
            "runs": self.backfill_runs,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn record_songid_run(
        &mut self,
        matches: Vec<serde_json::Value>,
        library_items: usize,
        shared_files: usize,
    ) -> Option<serde_json::Value> {
        self.songid_runs = self.allocate_songid_run_id();
        self.updated_at = unix_timestamp();
        let match_count = matches.len();
        let analysis_fields = serde_json::json!({
            "scorecard": {},
            "assessment": {"verdict": "unclassified", "confidence": 0, "summary": ""},
            "metadata": {"title": "", "artist": "", "album": "", "extra": {}},
            "provenance": {"signalCount": 0, "signals": [], "toolAvailable": false, "manifestHint": false, "verified": false},
            "perturbations": [],
            "stems": [],
            "corpusMatches": [],
            "clips": [],
            "transcripts": [],
            "ocr": [],
            "comments": [],
            "chapters": [],
            "segments": [],
            "mixGroups": [],
            "identityAssessment": {"verdict": "unclassified", "confidence": 0, "summary": ""},
            "syntheticAssessment": {"verdict": "insufficient_evidence", "confidence": "low", "syntheticScore": 0, "confidenceScore": 0, "knownFamilyScore": 0, "familyLabel": "none", "qualityClass": "clean_excerpt", "perturbationStability": 0, "topEvidenceFor": [], "topEvidenceAgainst": [], "notes": [], "summary": ""},
        });
        let mut record = serde_json::json!({
            "id": format!("songid-{}", self.songid_runs),
            "source": "",
            "sourceType": "text_query",
            // Matches the oracle's real terminal status vocabulary
            // exactly ("queued" | "running" | "completed" | "failed" --
            // never "matched"): whether matches were found is reflected
            // in the matches/matchCount fields, not a separate status
            // value the oracle never uses.
            "status": "completed",
            "query": "",
            "createdAt": chrono::Utc::now().to_rfc3339(),
            "summary": if matches.is_empty() { "SongID analysis completed." } else { "SongID matches found." },
            "currentStage": "completed",
            "percentComplete": 1.0,
            "artifactDirectory": "",
            "evidence": [],
            "tracks": [],
            "albums": [],
            "artists": [],
            "plans": [],
            "options": [],
            "libraryItems": library_items,
            "sharedFiles": shared_files,
            "matches": matches,
            "matchCount": match_count,
            "runs": self.songid_runs,
            "persisted": true,
            "updated_at": self.updated_at,
        });
        if let (Some(record), Some(fields)) = (record.as_object_mut(), analysis_fields.as_object())
        {
            record.extend(fields.clone());
        }
        if self.songid_run_records.len() == MAX_SONGID_RUNS {
            self.songid_run_records.remove(0);
        }
        self.songid_run_records.push(record.clone());
        Some(record)
    }

    pub(super) fn queue_songid_run(
        &mut self,
        source: &str,
        source_type: &str,
        library_items: usize,
        shared_files: usize,
    ) -> Option<serde_json::Value> {
        let record = self.record_songid_run(Vec::new(), library_items, shared_files)?;
        let id = record.get("id")?.as_str()?.to_owned();
        self.update_songid_run(&id, |run| {
            run["source"] = serde_json::json!(source);
            run["sourceType"] = serde_json::json!(source_type);
            run["status"] = serde_json::json!("queued");
            run["query"] = serde_json::json!("");
            run["summary"] = serde_json::json!("Queued for SongID analysis.");
            run["currentStage"] = serde_json::json!("queued");
            run["percentComplete"] = serde_json::json!(0.05);
        })
    }

    pub(super) fn update_songid_run(
        &mut self,
        id: &str,
        update: impl FnOnce(&mut serde_json::Value),
    ) -> Option<serde_json::Value> {
        let now = unix_timestamp();
        let updated = {
            let run = self.songid_run_records.iter_mut().find(|run| {
                run.get("id")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|run_id| run_id == id)
            })?;
            update(run);
            if let Some(object) = run.as_object_mut() {
                object.insert("updated_at".to_owned(), serde_json::json!(now));
            }
            run.clone()
        };
        self.updated_at = now;
        Some(updated)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn complete_songid_run(
        &mut self,
        id: &str,
        source: &str,
        source_type: &str,
        query: &str,
        matches: Vec<serde_json::Value>,
        library_items: usize,
        shared_files: usize,
        summary: &str,
        evidence: Vec<String>,
        metadata: serde_json::Value,
        full_source_fingerprint: Option<serde_json::Value>,
        acoustid_finding: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let match_count = matches.len();
        self.update_songid_run(id, |run| {
            run["source"] = serde_json::json!(source);
            run["sourceType"] = serde_json::json!(source_type);
            run["query"] = serde_json::json!(query);
            run["status"] = serde_json::json!("completed");
            run["summary"] = serde_json::json!(summary);
            run["currentStage"] = serde_json::json!("completed");
            run["percentComplete"] = serde_json::json!(1.0);
            run["libraryItems"] = serde_json::json!(library_items);
            run["sharedFiles"] = serde_json::json!(shared_files);
            run["matches"] = serde_json::Value::Array(matches);
            run["matchCount"] = serde_json::json!(match_count);
            run["evidence"] = serde_json::Value::Array(
                evidence
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            );
            run["metadata"] = metadata;
            if let Some(fingerprint) = full_source_fingerprint {
                run["fullSourceFingerprint"] = fingerprint;
            }
            if let Some(finding) = acoustid_finding {
                run["clips"] = serde_json::json!([{
                    "clipId": "full-source",
                    "acoustId": finding,
                }]);
                run["scorecard"] = serde_json::json!({
                    "acoustIdHitCount": 1,
                    "rawAcoustIdHitCount": 1,
                });
            }
        })
    }

    pub(super) fn fail_songid_run(&mut self, id: &str) -> Option<serde_json::Value> {
        self.fail_songid_run_with_reason(id, "Analysis could not be queued.")
    }

    pub(super) fn fail_songid_run_with_reason(
        &mut self,
        id: &str,
        reason: &str,
    ) -> Option<serde_json::Value> {
        self.update_songid_run(id, |run| {
            run["status"] = serde_json::json!("failed");
            run["summary"] = serde_json::json!("SongID analysis failed.");
            run["currentStage"] = serde_json::json!("failed");
            if let Some(evidence) = run
                .get_mut("evidence")
                .and_then(|value| value.as_array_mut())
            {
                evidence.push(serde_json::json!(reason));
            }
        })
    }

    pub(super) fn requeue_songid_run(&mut self, id: &str) -> Option<serde_json::Value> {
        self.update_songid_run(id, |run| {
            run["status"] = serde_json::json!("queued");
            run["summary"] = serde_json::json!("Queued for SongID analysis.");
            run["currentStage"] = serde_json::json!("queued");
            run["percentComplete"] = serde_json::json!(0.05);
        })
    }

    pub(super) fn allocate_songid_run_id(&self) -> u64 {
        let mut candidate = self.songid_runs.wrapping_add(1).max(1);
        for _ in 0..=self.songid_run_records.len() {
            let id = format!("songid-{candidate}");
            if !self.songid_run_records.iter().any(|record| {
                record.get("id").and_then(serde_json::Value::as_str) == Some(id.as_str())
            }) {
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded SongID run history must leave an available u64 id")
    }

    pub(super) fn songid_run(&self, id: &str) -> Option<serde_json::Value> {
        self.songid_run_records
            .iter()
            .find(|run| run.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .cloned()
    }

    pub(super) fn record_lidarr_sync(
        &mut self,
        missing_count: usize,
        configured: bool,
    ) -> serde_json::Value {
        self.lidarr_sync_runs = self.lidarr_sync_runs.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "synced": true,
            "queued": missing_count > 0,
            "status": if configured { "configured" } else { "local" },
            "missingCount": missing_count,
            "runs": self.lidarr_sync_runs,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn record_lidarr_manual_import(
        &mut self,
        imported: usize,
        configured: bool,
        directory: String,
        items: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        self.lidarr_manual_imports = self.lidarr_manual_imports.saturating_add(1);
        self.updated_at = unix_timestamp();
        serde_json::json!({
            "imported": imported,
            "status": if configured { "configured" } else { "local" },
            "directory": directory,
            "items": items,
            "runs": self.lidarr_manual_imports,
            "persisted": true,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "pendingRestart": self.application_restart_requested,
            "pendingReconnect": self.application_reconnect_pending,
            "gcRuns": self.gc_runs,
            "autoreplaceEnabled": self.autoreplace_enabled,
            "relayAgentEnabled": self.relay_agent_enabled,
            "bridgeRunning": self.bridge_running,
            "bridgeConfigUpdates": self.bridge_config_updates,
            "optionsUpdates": self.options_updates,
            "optionsYamlUploads": self.options_yaml_uploads,
            "optionsYamlValidations": self.options_yaml_validations,
            "profileInvitesCreated": self.profile_invites_created,
            "cacheWarmRuns": self.cache_warm_runs,
            "backfillRuns": self.backfill_runs,
            "songidRuns": self.songid_runs,
            "lidarrSyncRuns": self.lidarr_sync_runs,
            "lidarrManualImports": self.lidarr_manual_imports,
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn rollback_changes_if_unchanged(&mut self, previous: &Self, mutated: &Self) {
        macro_rules! rollback_field {
            ($field:ident) => {
                restore_changed_value_if_unchanged(
                    &mut self.$field,
                    &previous.$field,
                    &mutated.$field,
                );
            };
        }

        rollback_field!(application_restart_requested);
        rollback_field!(application_reconnect_pending);
        rollback_field!(vpn_reconnect_requested);
        rollback_field!(vpn);
        rollback_field!(gc_runs);
        rollback_field!(autoreplace_enabled);
        rollback_field!(accelerated_downloads_enabled);
        rollback_field!(relay_agent_enabled);
        rollback_field!(bridge_running);
        rollback_field!(bridge_config_updates);
        rollback_field!(bridge_started_at);
        rollback_field!(bridge_active_clients);
        rollback_field!(bridge_total_connections);
        rollback_field!(bridge_total_searches);
        rollback_field!(bridge_total_downloads);
        rollback_field!(bridge_total_room_joins);
        rollback_field!(bridge_total_bytes_proxied);
        rollback_field!(options_updates);
        rollback_field!(options_yaml_uploads);
        rollback_field!(options_yaml_validations);
        rollback_field!(profile_invites_created);
        rollback_field!(cache_warm_runs);
        rollback_field!(backfill_runs);
        rollback_field!(songid_runs);
        rollback_field!(songid_run_records);
        rollback_field!(lidarr_sync_runs);
        rollback_field!(lidarr_manual_imports);
        rollback_field!(updated_at);
    }
}

pub(super) async fn mutate_runtime_compat_state<T>(
    state: &AppState,
    mutate: impl FnOnce(&mut RuntimeCompatState, &mut RelayState) -> T,
) -> Result<T, String> {
    let _runtime_persistence = state.runtime_persistence_lock.lock().await;
    let (result, previous_runtime, mutated_runtime, previous_relay, mutated_relay, record) = {
        let mut runtime = state.runtime.write().await;
        let mut relay = state.relay.write().await;
        let previous_runtime = runtime.clone();
        let previous_relay = relay.clone();
        let result = mutate(&mut runtime, &mut relay);
        let mutated_runtime = runtime.clone();
        let mutated_relay = relay.clone();
        let record = runtime.persistence_record(&relay);
        (
            result,
            previous_runtime,
            mutated_runtime,
            previous_relay,
            mutated_relay,
            record,
        )
    };
    if let Some(db) = state.db.as_ref() {
        let persistence_error = db
            .upsert_runtime_compat_state(&record)
            .await
            .err()
            .map(|error| error.to_string());
        if let Some(error) = persistence_error {
            let mut runtime = state.runtime.write().await;
            let mut relay = state.relay.write().await;
            runtime.rollback_changes_if_unchanged(&previous_runtime, &mutated_runtime);
            relay.rollback_changes_if_unchanged(&previous_relay, &mutated_relay);
            return Err(format!("runtime compatibility persistence failed: {error}"));
        }
    }
    Ok(result)
}

/// Serialize process-local compatibility latches with durable runtime writes
/// without adding them to the SQLite mirror. These latches are reset by a
/// process restart and must not be rehydrated as pending work.
pub(super) async fn mutate_runtime_compat_state_in_memory<T>(
    state: &AppState,
    mutate: impl FnOnce(&mut RuntimeCompatState) -> T,
) -> T {
    let _runtime_persistence = state.runtime_persistence_lock.lock().await;
    let mut runtime = state.runtime.write().await;
    mutate(&mut runtime)
}
