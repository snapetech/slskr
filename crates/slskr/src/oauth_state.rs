use super::*;

pub(super) const MAX_OAUTH_STATES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OAuthStateRecord {
    pub(super) provider: String,
    pub(super) redirect_uri: String,
    pub(super) code_verifier: Option<String>,
    pub(super) created_at: u64,
    pub(super) expires_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OAuthStateStore {
    pub(super) records: BTreeMap<String, OAuthStateRecord>,
    max_records: usize,
}

impl Default for OAuthStateStore {
    fn default() -> Self {
        Self::with_max_records(MAX_OAUTH_STATES)
    }
}

#[allow(dead_code)]
impl OAuthStateStore {
    pub(super) fn with_max_records(max_records: usize) -> Self {
        Self {
            records: BTreeMap::new(),
            max_records: max_records.max(1),
        }
    }

    pub(super) fn from_persisted(records: Vec<crate::persistence::OAuthStateRecord>) -> Self {
        let now = super::unix_timestamp();
        let records = records
            .into_iter()
            .filter_map(|record| {
                let created_at = u64::try_from(record.created_at).ok()?;
                let expires_at = u64::try_from(record.expires_at).ok()?;
                (expires_at > now).then_some((
                    record.state,
                    OAuthStateRecord {
                        provider: record.provider,
                        redirect_uri: record.redirect_uri,
                        code_verifier: None,
                        created_at,
                        expires_at,
                    },
                ))
            })
            .take(MAX_OAUTH_STATES)
            .collect();
        Self {
            records,
            max_records: MAX_OAUTH_STATES,
        }
    }

    pub(super) fn issue(
        &mut self,
        provider: &str,
        redirect_uri: &str,
        ttl_seconds: u64,
    ) -> Option<String> {
        let now = super::unix_timestamp();
        self.prune(now);
        if self.records.len() >= self.max_records {
            return None;
        }
        let state = secure_oauth_state()?;
        let code_verifier = secure_spotify_code_verifier()?;
        self.records.insert(
            state.clone(),
            OAuthStateRecord {
                provider: provider.to_owned(),
                redirect_uri: redirect_uri.to_owned(),
                code_verifier: Some(code_verifier),
                created_at: now,
                expires_at: now.saturating_add(ttl_seconds),
            },
        );
        Some(state)
    }

    pub(super) fn consume(&mut self, provider: &str, state: &str) -> Option<OAuthStateRecord> {
        let now = super::unix_timestamp();
        self.prune(now);
        let record = self.records.remove(state)?;
        (record.provider == provider && record.expires_at > now).then_some(record)
    }

    pub(super) fn prune(&mut self, now: u64) {
        self.records.retain(|_, record| record.expires_at > now);
    }
}

pub(super) async fn persist_oauth_state_checked(
    state: &AppState,
    token: &str,
    record: &OAuthStateRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::OAuthStateRecord {
        state: token.to_owned(),
        provider: record.provider.clone(),
        redirect_uri: record.redirect_uri.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
        expires_at: i64::try_from(record.expires_at).unwrap_or(i64::MAX),
    };
    db.upsert_oauth_state(&persisted)
        .await
        .map_err(|error| format!("OAuth state persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_oauth_states_if_unchanged(
    state: &AppState,
    previous: OAuthStateStore,
    mutated: &OAuthStateStore,
) {
    let mut oauth_states = state.oauth_states.write().await;
    if *oauth_states == *mutated {
        *oauth_states = previous;
    }
}

pub(super) async fn consume_oauth_state(
    state: &AppState,
    provider: &str,
    token: &str,
) -> Result<Option<OAuthStateRecord>, String> {
    let _oauth_persistence = state.oauth_persistence_lock.lock().await;
    {
        let mut oauth_states = state.oauth_states.write().await;
        let now = super::unix_timestamp();
        oauth_states.prune(now);
        let Some(record) = oauth_states.records.get(token).cloned() else {
            return Ok(None);
        };
        if record.provider != provider || record.expires_at < now {
            return Ok(None);
        }
    }
    if let Some(db) = state.db.as_ref() {
        db.delete_oauth_state(token)
            .await
            .map_err(|error| format!("OAuth state persistence delete failed: {error}"))?;
    }
    let consumed = state.oauth_states.write().await.records.remove(token);
    drop(_oauth_persistence);
    Ok(consumed)
}

pub(super) fn secure_oauth_state() -> Option<String> {
    secure_oauth_state_with(|bytes| SysRng.try_fill_bytes(bytes).is_ok())
}

pub(super) fn secure_oauth_state_with(fill: impl FnOnce(&mut [u8; 32]) -> bool) -> Option<String> {
    let mut bytes = [0_u8; 32];
    fill(&mut bytes).then(|| format!("slskr-{}", hex::encode(bytes)))
}

fn secure_spotify_code_verifier() -> Option<String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut bytes = [0_u8; 64];
    SysRng.try_fill_bytes(&mut bytes).ok()?;
    Some(
        bytes
            .into_iter()
            .map(|byte| char::from(ALPHABET[usize::from(byte) % ALPHABET.len()]))
            .collect(),
    )
}

pub(super) async fn load_oauth_state_store(
    db: Option<&persistence::DatabaseManager>,
) -> Result<OAuthStateStore, String> {
    let Some(db) = db else {
        return Ok(OAuthStateStore::default());
    };
    let now = i64::try_from(unix_timestamp()).unwrap_or(i64::MAX);
    db.delete_expired_oauth_states(now)
        .await
        .map_err(|error| format!("failed to delete expired persisted OAuth states: {error}"))?;
    let records = db
        .list_oauth_states(now, EVENT_HISTORY_LIMIT as i32, 0)
        .await
        .map_err(|error| format!("failed to load persisted OAuth states: {error}"))?;
    Ok(OAuthStateStore::from_persisted(records))
}
