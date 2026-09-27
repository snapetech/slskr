use super::*;

pub(super) const BACKFILL_MAX_CANDIDATES: usize = 100;
pub(super) const BACKFILL_DEFAULT_CANDIDATES: usize = 10;
pub(super) const BACKFILL_MAX_HEADER_BYTES: usize = 65_536;
pub(super) const BACKFILL_HASH_BYTES: usize = 32_768;
pub(super) const BACKFILL_MAX_PER_PEER_PER_DAY: u32 = 10;
pub(super) const BACKFILL_RUN_INTERVAL_SECONDS: u64 = 600;
pub(super) const BACKFILL_MIN_IDLE_SECONDS: u64 = 300;
const BACKFILL_MAX_PEER_COUNTERS: usize = 4_096;
const BACKFILL_STATE_MAX_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug)]
pub(super) struct BackfillCandidate {
    pub(super) peer_id: String,
    pub(super) path: String,
    pub(super) size: u64,
    pub(super) discovered_at: u64,
    pub(super) peer_backfills_today: u32,
    pub(super) is_peer_online: bool,
    pub(super) is_peer_native: bool,
}

impl BackfillCandidate {
    pub(super) fn file_id(&self) -> String {
        content_discovery::generate_flac_key(&self.path, self.size)
    }

    pub(super) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "fileId": self.file_id(),
            "peerId": self.peer_id,
            "path": self.path,
            "size": self.size,
            "discoveredAt": unix_seconds_rfc3339(self.discovered_at),
            "peerBackfillsToday": self.peer_backfills_today,
            "isPeerOnline": self.is_peer_online,
            "isPeerSlskdn": self.is_peer_native,
        })
    }
}

#[derive(Debug)]
pub(super) struct BackfillState {
    pub(super) state_path: Option<PathBuf>,
    pub(super) enabled: bool,
    pub(super) is_idle: bool,
    pub(super) idle_since: Option<u64>,
    pub(super) total_attempts: u64,
    pub(super) successful: u64,
    pub(super) failed: u64,
    pub(super) rate_limited: u64,
    pub(super) active: usize,
    pub(super) hashes_discovered: u64,
    pub(super) last_cycle_time: Option<u64>,
    pub(super) next_cycle_time: Option<u64>,
    pub(super) peer_daily_counts: BTreeMap<String, (u64, u32)>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersistedBackfillState {
    version: u32,
    peer_daily_counts: BTreeMap<String, (u64, u32)>,
}

#[derive(Debug)]
pub(super) struct PendingBackfillTransfer {
    pub(super) expected_size: u64,
    pub(super) response: oneshot::Sender<Result<Vec<u8>, String>>,
}

impl Default for BackfillState {
    fn default() -> Self {
        Self {
            state_path: None,
            enabled: true,
            is_idle: false,
            idle_since: None,
            total_attempts: 0,
            successful: 0,
            failed: 0,
            rate_limited: 0,
            active: 0,
            hashes_discovered: 0,
            last_cycle_time: None,
            next_cycle_time: None,
            peer_daily_counts: BTreeMap::new(),
        }
    }
}

impl BackfillState {
    pub(super) fn load(state_dir: &Path) -> Result<Self, String> {
        use std::io::Read as _;

        let path = state_dir.join("backfill-state.json");
        let mut state = Self {
            state_path: Some(path.clone()),
            ..Self::default()
        };
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(state),
            Err(error) => return Err(format!("backfill state metadata failed: {error}")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("backfill state path must be a regular file".to_owned());
        }
        if metadata.len() > BACKFILL_STATE_MAX_BYTES {
            return Err("backfill state file is too large".to_owned());
        }
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let mut file = options
            .open(&path)
            .map_err(|error| format!("backfill state read failed: {error}"))?;
        let opened = file
            .metadata()
            .map_err(|error| format!("backfill state metadata failed: {error}"))?;
        if !opened.is_file() || opened.len() > BACKFILL_STATE_MAX_BYTES {
            return Err("backfill state path or size is invalid".to_owned());
        }
        let mut body = Vec::new();
        std::io::Read::take(&mut file, BACKFILL_STATE_MAX_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|error| format!("backfill state read failed: {error}"))?;
        if body.len() as u64 > BACKFILL_STATE_MAX_BYTES {
            return Err("backfill state file is too large".to_owned());
        }
        let persisted = serde_json::from_slice::<PersistedBackfillState>(&body)
            .map_err(|error| format!("backfill state parse failed: {error}"))?;
        if persisted.version != 1 || persisted.peer_daily_counts.len() > BACKFILL_MAX_PEER_COUNTERS
        {
            return Err("backfill state version or capacity is invalid".to_owned());
        }
        let day = unix_timestamp() / 86_400;
        state.peer_daily_counts = persisted
            .peer_daily_counts
            .into_iter()
            .filter(|(username, (record_day, _))| {
                !username.is_empty()
                    && username.len() <= MAX_SEARCH_RESULT_USERNAME_BYTES
                    && *record_day == day
            })
            .collect();
        Ok(state)
    }

    pub(super) fn persist(&self) -> Result<(), String> {
        let Some(path) = self.state_path.as_deref() else {
            return Ok(());
        };
        let body = serde_json::to_vec(&PersistedBackfillState {
            version: 1,
            peer_daily_counts: self.peer_daily_counts.clone(),
        })
        .map_err(|error| format!("backfill state serialization failed: {error}"))?;
        write_file_atomic(path, body)
            .map_err(|error| format!("backfill state write failed: {error}"))
    }

    pub(super) fn peer_count_today(&mut self, username: &str, now: u64) -> u32 {
        let day = now / 86_400;
        self.peer_daily_counts
            .retain(|_, (record_day, _)| *record_day == day);
        let key = username.to_ascii_lowercase();
        if self.peer_daily_counts.len() >= BACKFILL_MAX_PEER_COUNTERS
            && !self.peer_daily_counts.contains_key(&key)
        {
            return BACKFILL_MAX_PER_PEER_PER_DAY;
        }
        let entry = self.peer_daily_counts.entry(key).or_insert((day, 0));
        if entry.0 != day {
            *entry = (day, 0);
        }
        entry.1
    }

    pub(super) fn record_peer_success(&mut self, username: &str, now: u64) {
        let count = self.peer_count_today(username, now);
        if let Some(entry) = self
            .peer_daily_counts
            .get_mut(&username.to_ascii_lowercase())
        {
            entry.1 = count.saturating_add(1);
        }
    }

    pub(super) fn config_json(&self) -> serde_json::Value {
        serde_json::json!({
            "maxGlobalConnections": 2,
            "maxPerPeerPerDay": BACKFILL_MAX_PER_PEER_PER_DAY,
            "maxHeaderBytes": BACKFILL_MAX_HEADER_BYTES,
            "minIdleTimeSeconds": BACKFILL_MIN_IDLE_SECONDS,
            "runIntervalSeconds": BACKFILL_RUN_INTERVAL_SECONDS,
            "transferTimeoutSeconds": 30,
            "enabled": self.enabled,
        })
    }

    pub(super) fn stats_json(&self, now: u64) -> serde_json::Value {
        serde_json::json!({
            "totalAttempts": self.total_attempts,
            "successful": self.successful,
            "failed": self.failed,
            "rateLimited": self.rate_limited,
            "active": self.active,
            "hashesDiscovered": self.hashes_discovered,
            "lastCycleTime": self.last_cycle_time.map(unix_seconds_rfc3339),
            "nextCycleTime": self.next_cycle_time.map(unix_seconds_rfc3339),
            "isIdle": self.is_idle,
            "idleDuration": self
                .idle_since
                .map(|since| format_timespan_hms(i64::try_from(now.saturating_sub(since)).unwrap_or(i64::MAX))),
        })
    }
}
