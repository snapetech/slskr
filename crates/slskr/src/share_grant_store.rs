use super::{
    json_escape, truncate_utf8_bytes, unix_timestamp, AppState, MAX_SHARE_ACCESS_TOKENS,
    MAX_SHARE_ACCESS_TOKEN_TTL_SECONDS, MAX_SHARE_GRANTS, MAX_SHARE_GRANT_PERMISSIONS_BYTES,
    MAX_USER_USERNAME_BYTES,
};
use rand::{rngs::SysRng, TryRng};
use std::collections::{BTreeMap, HashSet};

// Share Grant Models
#[derive(Clone, Debug)]
pub(crate) struct ShareGrantRecord {
    pub(crate) id: String,
    pub(crate) collection_id: String,
    pub(crate) username: String,
    pub(crate) shared_at: u64,
    pub(crate) permissions: String,
    pub(crate) max_concurrent_streams: Option<u32>,
}

impl ShareGrantRecord {
    pub(crate) fn json(&self) -> String {
        let mut json = format!(
            "{{\"id\":\"{}\",\"collection_id\":\"{}\",\"collectionId\":\"{}\",\"username\":\"{}\",\"shared_at\":{},\"permissions\":\"{}\",\"allow_download\":{},\"allow_stream\":{},\"allow_reshare\":{}}}",
            json_escape(&self.id),
            json_escape(&self.collection_id),
            json_escape(&self.collection_id),
            json_escape(&self.username),
            self.shared_at,
            json_escape(&self.permissions),
            share_grant_allows_download(&self.permissions),
            share_grant_allows_stream(&self.permissions),
            share_grant_allows_reshare(&self.permissions)
        );
        if let Some(limit) = self.max_concurrent_streams {
            json.pop();
            json.push_str(&format!(",\"maxConcurrentStreams\":{limit}}}"));
        }
        json
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ShareGrantStore {
    pub(crate) records: Vec<ShareGrantRecord>,
    pub(crate) next_id: u64,
    pub(crate) updated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShareAccessTokenRecord {
    pub(crate) grant_id: String,
    pub(crate) expires_at: u64,
}

#[derive(Debug)]
pub(crate) struct ShareAccessTokenStore {
    pub(crate) records: BTreeMap<String, ShareAccessTokenRecord>,
    pub(crate) max_records: usize,
}

impl Default for ShareAccessTokenStore {
    fn default() -> Self {
        Self::with_max_records(MAX_SHARE_ACCESS_TOKENS)
    }
}

impl ShareAccessTokenStore {
    pub(crate) fn with_max_records(max_records: usize) -> Self {
        Self {
            records: BTreeMap::new(),
            max_records: max_records.max(1),
        }
    }

    pub(crate) fn issue(&mut self, grant_id: String, ttl_seconds: u64) -> Option<(String, u64)> {
        let now = unix_timestamp();
        self.prune(now);
        if self.records.len() >= self.max_records {
            return None;
        }
        let token = secure_share_grant_token()?;
        let expires_at =
            now.saturating_add(ttl_seconds.clamp(1, MAX_SHARE_ACCESS_TOKEN_TTL_SECONDS));
        self.records.insert(
            share_access_token_digest(&token),
            ShareAccessTokenRecord {
                grant_id,
                expires_at,
            },
        );
        Some((token, expires_at))
    }

    pub(crate) fn from_persisted(
        records: Vec<crate::persistence::ShareAccessTokenRecord>,
        valid_grant_ids: &HashSet<&str>,
    ) -> Self {
        let now = unix_timestamp();
        let mut store = Self::default();
        for record in records.into_iter().take(store.max_records) {
            let Ok(expires_at) = u64::try_from(record.expires_at) else {
                continue;
            };
            if expires_at <= now
                || !valid_grant_ids.contains(record.grant_id.as_str())
                || record.token_digest.len() != 64
                || !record
                    .token_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                continue;
            }
            store.records.insert(
                record.token_digest.to_ascii_lowercase(),
                ShareAccessTokenRecord {
                    grant_id: record.grant_id,
                    expires_at,
                },
            );
        }
        store
    }

    pub(crate) fn validate(&mut self, token: &str) -> Option<ShareAccessTokenRecord> {
        let now = unix_timestamp();
        self.prune(now);
        self.records.get(&share_access_token_digest(token)).cloned()
    }

    pub(crate) fn revoke_grant(&mut self, grant_id: &str) {
        self.records.retain(|_, record| record.grant_id != grant_id);
    }

    pub(crate) fn remove_if_unchanged(&mut self, digest: &str, expected: &ShareAccessTokenRecord) {
        if self.records.get(digest) == Some(expected) {
            self.records.remove(digest);
        }
    }

    pub(crate) fn prune(&mut self, now: u64) {
        self.records.retain(|_, record| record.expires_at > now);
    }
}

pub(crate) fn share_access_token_digest(token: &str) -> String {
    use sha2::{Digest, Sha256};

    hex::encode(Sha256::digest(token.as_bytes()))
}

impl ShareGrantStore {
    pub(crate) fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            updated_at: unix_timestamp(),
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::ShareGrantRecord>) -> Self {
        let mut next_id = 1;
        let mut updated_at = unix_timestamp();
        for record in &records {
            if let Some(number) = record
                .id
                .strip_prefix("grant-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                next_id = next_id.max(number.saturating_add(1));
            }
        }
        let mut seen_ids = std::collections::HashSet::new();
        let mut seen_grants = std::collections::HashSet::new();
        let records = records
            .into_iter()
            .filter_map(|record| {
                if record.max_concurrent_streams.is_some_and(|limit| {
                    !(1..=i64::from(crate::share_stream_limits::MAX_EXPLICIT_STREAM_LIMIT))
                        .contains(&limit)
                }) {
                    return None;
                }
                let username = normalize_share_grant_username(&record.username)?;
                (seen_ids.insert(record.id.clone())
                    && seen_grants
                        .insert((record.collection_id.clone(), username.to_ascii_lowercase())))
                .then_some((record, username))
            })
            .take(MAX_SHARE_GRANTS)
            .map(|(record, username)| {
                let shared_at = u64::try_from(record.shared_at).unwrap_or(0);
                updated_at = updated_at.max(shared_at);
                ShareGrantRecord {
                    id: record.id,
                    collection_id: record.collection_id,
                    username,
                    shared_at,
                    permissions: bounded_share_grant_permissions(&record.permissions),
                    max_concurrent_streams: record
                        .max_concurrent_streams
                        .and_then(|limit| u32::try_from(limit).ok())
                        .filter(|limit| {
                            (1..=crate::share_stream_limits::MAX_EXPLICIT_STREAM_LIMIT)
                                .contains(limit)
                        }),
                }
            })
            .collect();
        Self {
            records,
            next_id,
            updated_at,
        }
    }

    /// Preserve the legacy store helper used by the full controller matrix;
    /// the active route supplies its explicit permission contract below.
    #[allow(dead_code)]
    pub(crate) fn create_with_contract(
        &mut self,
        id: Option<String>,
        collection_id: String,
        username: String,
    ) -> Option<(ShareGrantRecord, bool)> {
        self.create_with_contract_and_permissions(id, collection_id, username, "read")
    }

    pub(crate) fn create_with_contract_and_permissions(
        &mut self,
        id: Option<String>,
        collection_id: String,
        username: String,
        permissions: &str,
    ) -> Option<(ShareGrantRecord, bool)> {
        let username = normalize_share_grant_username(&username)?;
        if let Some(record) = self.records.iter().find(|record| {
            record.collection_id == collection_id && record.username.eq_ignore_ascii_case(&username)
        }) {
            return Some((record.clone(), false));
        }
        if self.records.len() >= MAX_SHARE_GRANTS {
            return None;
        }
        let id = id.unwrap_or_else(|| format!("grant-{}", self.allocate_id()));
        let now = unix_timestamp();
        let record = ShareGrantRecord {
            id,
            collection_id,
            username,
            shared_at: now,
            permissions: bounded_share_grant_permissions(permissions),
            max_concurrent_streams: None,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some((record, true))
    }

    fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            let id = format!("grant-{candidate}");
            if !self.records.iter().any(|record| record.id == id) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded share-grant store must leave an available u64 id")
    }

    pub(crate) fn get(&self, id: &str) -> Option<ShareGrantRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    pub(crate) fn get_by_collection(&self, collection_id: &str) -> Vec<ShareGrantRecord> {
        self.records
            .iter()
            .filter(|r| r.collection_id == collection_id)
            .cloned()
            .collect()
    }

    pub(crate) fn update(&mut self, id: &str, permissions: String) -> Option<ShareGrantRecord> {
        let record = self.records.iter_mut().find(|r| r.id == id)?;
        record.permissions = bounded_share_grant_permissions(&permissions);
        self.updated_at = unix_timestamp();
        Some(record.clone())
    }

    pub(crate) fn set_stream_limit(
        &mut self,
        id: &str,
        limit: Option<u32>,
    ) -> Option<ShareGrantRecord> {
        let record = self.records.iter_mut().find(|record| record.id == id)?;
        record.max_concurrent_streams = limit;
        self.updated_at = unix_timestamp();
        Some(record.clone())
    }

    pub(crate) fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    pub(crate) fn delete_by_collection(&mut self, collection_id: &str) -> Vec<ShareGrantRecord> {
        let mut removed = Vec::new();
        self.records.retain(|record| {
            if record.collection_id == collection_id {
                removed.push(record.clone());
                false
            } else {
                true
            }
        });
        if !removed.is_empty() {
            self.updated_at = unix_timestamp();
        }
        removed
    }

    #[allow(dead_code)]
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"entries\":{},\"count\":{},\"updated_at\":{}}}",
            self.json_array(),
            self.records.len(),
            self.updated_at
        )
    }

    pub(crate) fn json_array(&self) -> String {
        let records = self
            .records
            .iter()
            .map(ShareGrantRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!("[{}]", records)
    }
}

pub(crate) fn normalize_share_grant_username(username: &str) -> Option<String> {
    let username = truncate_utf8_bytes(username.trim().to_owned(), MAX_USER_USERNAME_BYTES);
    (!username.is_empty()).then_some(username)
}

pub(crate) fn bounded_share_grant_permissions(permissions: &str) -> String {
    let permissions = truncate_utf8_bytes(
        permissions.trim().to_owned(),
        MAX_SHARE_GRANT_PERMISSIONS_BYTES,
    );
    if permissions.is_empty() {
        "read".to_owned()
    } else {
        permissions
    }
}

pub(crate) fn share_grant_allows_download(permissions: &str) -> bool {
    share_grant_allows(permissions, "download")
}

pub(crate) fn share_grant_allows_stream(permissions: &str) -> bool {
    share_grant_allows(permissions, "stream")
}

pub(crate) fn share_grant_allows_reshare(permissions: &str) -> bool {
    share_grant_allows(permissions, "reshare")
}

fn share_grant_allows(permissions: &str, token: &str) -> bool {
    permissions
        .split(|character: char| character == ',' || character.is_ascii_whitespace())
        .any(|permission| permission.eq_ignore_ascii_case(token))
}

pub(super) async fn persist_share_grant(
    state: &AppState,
    record: &ShareGrantRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::ShareGrantRecord {
        id: record.id.clone(),
        collection_id: record.collection_id.clone(),
        username: record.username.clone(),
        shared_at: i64::try_from(record.shared_at).unwrap_or(i64::MAX),
        permissions: record.permissions.clone(),
        max_concurrent_streams: record.max_concurrent_streams.map(i64::from),
    };
    db.upsert_share_grant(&persisted)
        .await
        .map_err(|error| format!("share grant persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_share_grant_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_share_grant(id)
        .await
        .map_err(|error| format!("share grant revocation persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_share_access_token(
    state: &AppState,
    token_digest: &str,
    record: &ShareAccessTokenRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let now = i64::try_from(unix_timestamp()).unwrap_or(i64::MAX);
    db.delete_expired_share_access_tokens(now)
        .await
        .map_err(|error| format!("share access token cleanup failed: {error}"))?;
    db.upsert_share_access_token(&crate::persistence::ShareAccessTokenRecord {
        token_digest: token_digest.to_owned(),
        grant_id: record.grant_id.clone(),
        expires_at: i64::try_from(record.expires_at).unwrap_or(i64::MAX),
    })
    .await
    .map_err(|error| format!("share access token persistence failed: {error}"))?;
    Ok(true)
}

pub(super) fn secure_share_grant_token() -> Option<String> {
    secure_share_grant_token_with(|bytes| SysRng.try_fill_bytes(bytes).is_ok())
}

pub(super) fn secure_share_grant_token_with(
    fill: impl FnOnce(&mut [u8; 32]) -> bool,
) -> Option<String> {
    let mut bytes = [0_u8; 32];
    fill(&mut bytes).then(|| format!("share-{}", hex::encode(bytes)))
}
