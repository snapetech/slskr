use super::*;

pub(super) const MAX_SECURITY_BANS: usize = 4_096;
const SECURITY_REPUTATION_PROTOCOL_VIOLATION_PENALTY: i32 = 15;

pub(super) async fn persist_security_ban(
    state: &AppState,
    record: &SecurityBanRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::SecurityBanRecord {
        kind: record.kind.clone(),
        value: record.value.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        reason: record.reason.clone(),
        expires_at: i64::try_from(record.expires_at).unwrap_or(i64::MAX),
        is_permanent: record.is_permanent,
    };
    db.upsert_security_ban(&persisted)
        .await
        .map_err(|error| format!("security ban persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_security_ban_if_unchanged(
    state: &AppState,
    previous_bans: Vec<SecurityBanRecord>,
    previous_updated_at: u64,
    mutated_bans: Vec<SecurityBanRecord>,
    mutated_updated_at: u64,
) {
    let mut security = state.security.write().await;
    if security_ban_state_matches(&security, &mutated_bans, mutated_updated_at) {
        security.bans = previous_bans;
        security.updated_at = previous_updated_at;
    }
}

fn security_ban_state_matches(
    current: &SecurityState,
    expected_bans: &[SecurityBanRecord],
    expected_updated_at: u64,
) -> bool {
    current.updated_at == expected_updated_at
        && current.bans.len() == expected_bans.len()
        && current
            .bans
            .iter()
            .zip(expected_bans)
            .all(|(current, expected)| {
                current.kind == expected.kind
                    && current.value == expected.value
                    && current.created_at == expected.created_at
                    && current.reason == expected.reason
                    && current.expires_at == expected.expires_at
                    && current.is_permanent == expected.is_permanent
            })
}

pub(super) async fn persist_security_unban(
    state: &AppState,
    kind: &str,
    value: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let Some(target) = super::normalize_security_ban_value(kind, value) else {
        return Err("invalid security unban value".to_owned());
    };
    let records = db
        .list_security_bans()
        .await
        .map_err(|error| format!("security unban persistence failed: {error}"))?;
    for record in records {
        if record.kind != kind {
            continue;
        }
        let Some(candidate) = super::normalize_security_ban_value(kind, &record.value) else {
            continue;
        };
        let matches = if kind == "username" {
            candidate.eq_ignore_ascii_case(&target)
        } else {
            candidate == target
        };
        if matches {
            db.delete_security_ban(kind, &record.value)
                .await
                .map_err(|error| format!("security unban persistence failed: {error}"))?;
        }
    }
    Ok(true)
}

/// Durable, expiry-bounded JWT revocation storage, matching the oracle's
/// `JwtRevocationStore`: revocations must survive a restart, or a stable
/// configured JWT key (see `SLSKR_JWT_KEY`) would let a previously-revoked
/// token be honored again for the remainder of its lifetime.
#[derive(Debug, Default)]
pub(super) struct RevokedJwtStore {
    records: BTreeMap<String, u64>,
    state_path: Option<PathBuf>,
}

impl RevokedJwtStore {
    pub(super) const STATE_FILE_NAME: &'static str = "jwt-revocations.json";
    const MAX_STATE_BYTES: u64 = 8 * 1024 * 1024;
    const MAX_RECORDS: usize = 100_000;
    const MAX_JTI_BYTES: usize = 512;

    pub(super) fn load(state_dir: &Path) -> Result<Self, String> {
        let state_path = state_dir.join(Self::STATE_FILE_NAME);
        let records = match fs::symlink_metadata(&state_path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err("JWT revocation state path must be a regular file".to_owned());
                }
                if metadata.len() > Self::MAX_STATE_BYTES {
                    return Err("JWT revocation state file is too large".to_owned());
                }
                let bytes = fs::read(&state_path)
                    .map_err(|error| format!("JWT revocation state read failed: {error}"))?;
                serde_json::from_slice::<BTreeMap<String, u64>>(&bytes)
                    .map_err(|error| format!("JWT revocation state parse failed: {error}"))?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => {
                return Err(format!("JWT revocation state metadata failed: {error}"));
            }
        };
        if records.len() > Self::MAX_RECORDS
            || records.keys().any(|jti| {
                jti.is_empty()
                    || jti.len() > Self::MAX_JTI_BYTES
                    || jti.chars().any(char::is_control)
            })
        {
            return Err("JWT revocation state exceeds its record or identifier limits".to_owned());
        }
        let mut store = Self {
            records,
            state_path: Some(state_path),
        };
        store.prune(unix_timestamp());
        Ok(store)
    }

    pub(super) fn revoke(&mut self, jti: String, expires_at: u64, now: u64) -> Result<(), String> {
        self.records.retain(|_, expiry| *expiry > now);
        if expires_at > now {
            self.records.insert(jti, expires_at);
        }
        self.persist()
    }

    pub(super) fn contains(&mut self, jti: &str, now: u64) -> bool {
        self.prune(now);
        self.records.contains_key(jti)
    }

    fn prune(&mut self, now: u64) {
        let had_expired = self.records.values().any(|expiry| *expiry <= now);
        self.records.retain(|_, expiry| *expiry > now);
        if had_expired {
            if let Err(error) = self.persist() {
                // Keeping an expired revocation on disk is fail-closed; report
                // the failure so operators can repair the state directory.
                eprintln!("[Error] JWT revocation pruning persistence failed: {error}");
            }
        }
    }

    fn persist(&self) -> Result<(), String> {
        let Some(path) = self.state_path.as_deref() else {
            return Ok(());
        };
        let body = serde_json::to_vec(&self.records)
            .map_err(|error| format!("JWT revocation state serialization failed: {error}"))?;
        write_file_atomic(path, body)
            .map_err(|error| format!("JWT revocation state write failed: {error}"))
    }
}

#[derive(Clone, Copy, Debug)]
struct LoginAttempt {
    failures: u8,
    last_failure: u64,
    lockout_until: u64,
}

#[derive(Debug, Default)]
pub(super) struct LoginAttemptStore {
    records: BTreeMap<String, LoginAttempt>,
}

impl LoginAttemptStore {
    const MAX_RECORDS: usize = 10_000;
    const MAX_FAILURES: u8 = 5;
    const WINDOW_SECONDS: u64 = 10 * 60;
    const LOCKOUT_SECONDS: u64 = 5 * 60;

    pub(super) fn is_locked(&mut self, keys: &[String], now: u64) -> bool {
        self.prune(now);
        keys.iter().any(|key| {
            self.records
                .get(key)
                .is_some_and(|attempt| attempt.lockout_until > now)
        })
    }

    pub(super) fn record_failure(&mut self, keys: &[String], now: u64) {
        self.prune(now);
        for key in keys {
            if !self.records.contains_key(key) && self.records.len() >= Self::MAX_RECORDS {
                if let Some(oldest) = self
                    .records
                    .iter()
                    .min_by_key(|(_, attempt)| attempt.last_failure)
                    .map(|(key, _)| key.clone())
                {
                    self.records.remove(&oldest);
                }
            }
            let previous = self.records.get(key).copied();
            let failures = previous
                .filter(|attempt| now.saturating_sub(attempt.last_failure) <= Self::WINDOW_SECONDS)
                .map(|attempt| attempt.failures.saturating_add(1))
                .unwrap_or(1);
            self.records.insert(
                key.clone(),
                LoginAttempt {
                    failures,
                    last_failure: now,
                    lockout_until: if failures >= Self::MAX_FAILURES {
                        now.saturating_add(Self::LOCKOUT_SECONDS)
                    } else {
                        0
                    },
                },
            );
        }
    }

    pub(super) fn clear(&mut self, keys: &[String]) {
        for key in keys {
            self.records.remove(key);
        }
    }

    fn prune(&mut self, now: u64) {
        self.records.retain(|_, attempt| {
            attempt.lockout_until > now
                || now.saturating_sub(attempt.last_failure) <= Self::WINDOW_SECONDS
        });
    }
}

#[derive(Clone, Debug)]
pub(super) struct SecurityBanRecord {
    pub(super) kind: String,
    pub(super) value: String,
    pub(super) created_at: u64,
    pub(super) reason: String,
    pub(super) expires_at: u64,
    pub(super) is_permanent: bool,
}

#[derive(Clone, Debug)]
pub(super) struct SecurityReputationProfile {
    pub(super) username: String,
    pub(super) first_seen: u64,
    pub(super) last_seen: u64,
    pub(super) successful_transfers: u64,
    pub(super) failed_transfers: u64,
    pub(super) aborted_transfers: u64,
    pub(super) total_bytes_transferred: u64,
    pub(super) malformed_messages: u64,
    pub(super) protocol_violations: u64,
    pub(super) content_mismatches: u64,
    pub(super) slots_available_count: u64,
}

impl SecurityReputationProfile {
    pub(super) fn new(username: &str, now: u64) -> Self {
        Self {
            username: username.to_owned(),
            first_seen: now,
            last_seen: now,
            successful_transfers: 0,
            failed_transfers: 0,
            aborted_transfers: 0,
            total_bytes_transferred: 0,
            malformed_messages: 0,
            protocol_violations: 0,
            content_mismatches: 0,
            slots_available_count: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct SecurityState {
    pub(super) bans: Vec<SecurityBanRecord>,
    pub(super) violations: BTreeMap<String, u32>,
    pub(super) reputation: BTreeMap<String, i32>,
    pub(super) reputation_profiles: BTreeMap<String, SecurityReputationProfile>,
    pub(super) event_sink: crate::mesh_security::SecurityEventSink,
    pub(super) updated_at: u64,
}

impl SecurityState {
    pub(super) fn new() -> Self {
        Self {
            bans: Vec::new(),
            violations: BTreeMap::new(),
            reputation: BTreeMap::new(),
            reputation_profiles: BTreeMap::new(),
            event_sink: crate::mesh_security::SecurityEventSink::new(),
            updated_at: unix_timestamp(),
        }
    }

    pub(super) fn from_persisted(records: Vec<crate::persistence::SecurityBanRecord>) -> Self {
        let mut updated_at = unix_timestamp();
        let mut seen = std::collections::HashSet::new();
        let bans = records
            .into_iter()
            .filter_map(|record| {
                let value = normalize_security_ban_value(&record.kind, &record.value)?;
                let deduplication_value = if record.kind == "username" {
                    value.to_ascii_lowercase()
                } else {
                    value.clone()
                };
                seen.insert((record.kind.clone(), deduplication_value))
                    .then_some((record, value))
            })
            .take(MAX_SECURITY_BANS)
            .map(|(record, value)| {
                let created_at = u64::try_from(record.created_at).unwrap_or(0);
                updated_at = updated_at.max(created_at);
                SecurityBanRecord {
                    kind: record.kind,
                    value,
                    created_at,
                    reason: record.reason,
                    expires_at: u64::try_from(record.expires_at).unwrap_or(created_at),
                    is_permanent: record.is_permanent,
                }
            })
            .collect();
        Self {
            bans,
            violations: BTreeMap::new(),
            reputation: BTreeMap::new(),
            reputation_profiles: BTreeMap::new(),
            event_sink: crate::mesh_security::SecurityEventSink::new(),
            updated_at,
        }
    }

    pub(super) fn ensure_reputation_profile(&mut self, key: &str, username: &str) {
        let now = unix_timestamp();
        self.reputation_profiles
            .entry(key.to_owned())
            .or_insert_with(|| SecurityReputationProfile::new(username, now));
    }

    /// Convenience default over `ban_with_options`, exercised directly by
    /// several unit tests below; every real HTTP call site now passes
    /// real reason/duration/permanent via `security_ban_options`.
    #[allow(dead_code)]
    pub(super) fn ban(&mut self, kind: &str, value: String) -> Option<SecurityBanRecord> {
        self.ban_with_options(kind, value, "Manual ban".to_owned(), 3_600, false)
    }

    pub(super) fn ban_with_options(
        &mut self,
        kind: &str,
        value: String,
        reason: String,
        duration_seconds: u64,
        is_permanent: bool,
    ) -> Option<SecurityBanRecord> {
        let value = normalize_security_ban_value(kind, &value)?;
        let now = unix_timestamp();
        let expires_at = now.saturating_add(duration_seconds);
        if let Some(record) = self.bans.iter_mut().find(|record| {
            record.kind == kind
                && if kind == "username" {
                    record.value.eq_ignore_ascii_case(&value)
                } else {
                    record.value == value
                }
        }) {
            record.created_at = now;
            record.reason = reason;
            record.expires_at = expires_at;
            record.is_permanent = is_permanent;
            self.updated_at = now;
            return Some(record.clone());
        }
        if self.bans.len() >= MAX_SECURITY_BANS {
            return None;
        }
        let record = SecurityBanRecord {
            kind: kind.to_owned(),
            value,
            created_at: now,
            reason,
            expires_at,
            is_permanent,
        };
        self.bans.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(super) fn unban(&mut self, kind: &str, value: &str) -> bool {
        let Some(value) = normalize_security_ban_value(kind, value) else {
            return false;
        };
        let before = self.bans.len();
        self.bans.retain(|record| {
            let value_matches = if kind == "username" {
                record.value.eq_ignore_ascii_case(&value)
            } else {
                record.value == value
            };
            !(record.kind == kind && value_matches)
        });
        let removed = before != self.bans.len();
        if removed {
            self.updated_at = unix_timestamp();
        }
        removed
    }

    pub(super) fn record_peer_violation(
        &mut self,
        username: &str,
        settings: &crate::config::SecuritySettings,
    ) -> bool {
        let username = bounded_user_username(username.trim());
        if username.is_empty() || !settings.enabled {
            return false;
        }
        let key = username.to_ascii_lowercase();
        self.ensure_reputation_profile(&key, &username);
        if let Some(profile) = self.reputation_profiles.get_mut(&key) {
            profile.last_seen = unix_timestamp();
            profile.protocol_violations = profile.protocol_violations.saturating_add(1);
        }
        let violation_count = self
            .violations
            .get(&key)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.violations.insert(key.clone(), violation_count);
        self.event_sink
            .report(crate::mesh_security::SecuritySinkEvent::new(
                "Violation",
                crate::mesh_security::SecuritySinkSeverity::Medium,
                "peer security violation recorded",
                None,
                Some(username.clone()),
                Some("violation-tracker".to_owned()),
            ));
        let score = self.reputation.entry(key.clone()).or_insert(50);
        *score = score
            .saturating_sub(SECURITY_REPUTATION_PROTOCOL_VIOLATION_PENALTY)
            .max(0);
        self.updated_at = unix_timestamp();
        if settings.violation_tracker.enabled
            && self.violations.get(&key).copied().unwrap_or(0)
                >= settings.violation_tracker.violations_before_auto_ban
        {
            self.violations.insert(key, 0);
            let banned = self
                .ban_with_options(
                    "username",
                    username.clone(),
                    "Automatic security-policy ban".to_owned(),
                    settings.violation_tracker.base_ban_duration.as_secs(),
                    false,
                )
                .is_some();
            if banned {
                self.event_sink
                    .report(crate::mesh_security::SecuritySinkEvent::new(
                        "Ban",
                        crate::mesh_security::SecuritySinkSeverity::High,
                        "peer automatically banned after security violations",
                        None,
                        Some(username),
                        Some("violation-tracker".to_owned()),
                    ));
            }
            return banned;
        }
        false
    }

    pub(super) fn active_bans(&self) -> usize {
        self.bans.len()
    }

    pub(super) fn is_blocked(&self, kind: &str, value: &str) -> bool {
        let now = unix_timestamp();
        self.bans.iter().any(|record| {
            record.kind == kind
                && record.expires_at > now
                && if kind == "username" {
                    record.value.eq_ignore_ascii_case(value)
                } else {
                    record.value == value
                }
        })
    }

    pub(super) fn json_value(&self) -> serde_json::Value {
        let bans = self
            .bans
            .iter()
            .map(|record| {
                serde_json::json!({
                    "kind": record.kind,
                    "type": record.kind,
                    "value": record.value,
                    "created_at": record.created_at,
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "bans": bans,
            "count": self.bans.len(),
            "updated_at": self.updated_at,
        })
    }

    pub(super) fn native_bans_json(&self) -> String {
        let now = unix_timestamp();
        serde_json::Value::Array(
            self.bans
                .iter()
                .filter(|record| record.expires_at > now)
                .map(|record| {
                    let remaining = record.expires_at.saturating_sub(now);
                    let days = remaining / 86_400;
                    let hours = (remaining % 86_400) / 3_600;
                    let minutes = (remaining % 3_600) / 60;
                    let seconds = remaining % 60;
                    serde_json::json!({
                        "key": if record.kind == "ip" {
                            format!("IP:{}", record.value)
                        } else {
                            format!("User:{}", record.value.to_ascii_lowercase())
                        },
                        "reason": record.reason,
                        "bannedAt": unix_seconds_rfc3339(record.created_at),
                        "expiresAt": unix_seconds_rfc3339(record.expires_at),
                        "isPermanent": record.is_permanent,
                        "timeRemaining": format!("{days}.{hours:02}:{minutes:02}:{seconds:02}"),
                    })
                })
                .collect(),
        )
        .to_string()
    }
}
