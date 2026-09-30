//! Grant-scoped metadata and range reads over the authenticated mesh session.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::Semaphore;

use crate::{
    collection_store::CollectionItem, find_shared_entry_for_content, find_shared_local_file,
    open_shared_local_file, share_grant_allows_download, AppState,
};

const MAX_BACKFILL_ITEMS: usize = 64;
const MAX_BACKFILL_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_BACKFILL_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_BACKFILL_RANGE_BYTES: u64 = 46_000;
const MAX_BACKFILL_GRANT_ID_BYTES: usize = 256;
const MAX_BACKFILL_TOKEN_BYTES: usize = 512;
static RECIPIENT_BACKFILL_LIMIT: Semaphore = Semaphore::const_new(2);

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ShareBackfillRequest {
    grant_id: String,
    token: String,
    #[serde(default)]
    content_id: Option<String>,
    #[serde(default)]
    offset: Option<u64>,
    #[serde(default)]
    length: Option<u64>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShareBackfillItem {
    pub(crate) content_id: String,
    pub(crate) filename: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

pub(crate) async fn handle_mesh_call(
    state: &AppState,
    remote_username: &str,
    method: &str,
    payload: &[u8],
) -> Result<Vec<u8>, (i32, String)> {
    if state.config.controller_profile != crate::ControllerProfile::Native
        || !state.config.current_upstream_behavior
    {
        return Err((3, "Unknown method".to_owned()));
    }
    let request: ShareBackfillRequest = serde_json::from_slice(payload)
        .map_err(|_| (4, "Invalid share backfill request".to_owned()))?;
    validate_request_fields(&request)?;
    match method {
        "GetShareBackfillManifest" if request.content_id.is_none() => {
            let (_, collection_id, permissions) =
                authorize_grant(state, remote_username, &request).await?;
            if !share_grant_allows_download(&permissions) {
                return Err((2, "Share unavailable".to_owned()));
            }
            let items = {
                let collections = state.collections.read().await;
                collections
                    .get(&collection_id)
                    .map(|collection| collection.items.clone())
                    .ok_or_else(|| (2, "Share unavailable".to_owned()))?
            };
            if items.len() > MAX_BACKFILL_ITEMS {
                return Err((4, "Share exceeds the backfill item limit".to_owned()));
            }
            let mut total_bytes = 0_u64;
            let mut manifest = Vec::with_capacity(items.len());
            for item in items {
                let (resolved, _) = resolve_item(state, &item).await?;
                total_bytes = total_bytes
                    .checked_add(resolved.size)
                    .filter(|total| *total <= MAX_BACKFILL_TOTAL_BYTES)
                    .ok_or_else(|| (4, "Share exceeds the backfill byte limit".to_owned()))?;
                manifest.push(resolved);
            }
            authorize_grant(state, remote_username, &request).await?;
            serde_json::to_vec(&manifest)
                .map_err(|_| (1, "Share backfill manifest failed".to_owned()))
        }
        "GetShareBackfillRange" => {
            let content_id = request
                .content_id
                .as_deref()
                .ok_or_else(|| (4, "ContentId is required".to_owned()))?;
            let offset = request
                .offset
                .ok_or_else(|| (4, "Range offset is required".to_owned()))?;
            let length = request
                .length
                .ok_or_else(|| (4, "Range length is required".to_owned()))?;
            if length == 0 || length > MAX_BACKFILL_RANGE_BYTES {
                return Err((4, "Range length is outside the supported limit".to_owned()));
            }
            let (_, collection_id, permissions) =
                authorize_grant(state, remote_username, &request).await?;
            if !share_grant_allows_download(&permissions) {
                return Err((2, "Share unavailable".to_owned()));
            }
            let item = {
                let collections = state.collections.read().await;
                collections
                    .get(&collection_id)
                    .and_then(|collection| {
                        collection
                            .items
                            .iter()
                            .find(|item| item.content_id == content_id)
                            .cloned()
                    })
                    .ok_or_else(|| (2, "Share unavailable".to_owned()))?
            };
            let (resolved, shared_filename) = resolve_item(state, &item).await?;
            if offset
                .checked_add(length)
                .is_none_or(|end| end > resolved.size)
            {
                return Err((4, "Range exceeds the shared file".to_owned()));
            }
            let shared = find_shared_local_file(state, &shared_filename)
                .await
                .ok_or_else(|| (2, "Share unavailable".to_owned()))?;
            let mut file = open_shared_local_file(state, &shared.local_path)
                .await
                .map_err(|_| (2, "Share unavailable".to_owned()))?;
            let metadata = file
                .metadata()
                .map_err(|_| (2, "Share unavailable".to_owned()))?;
            if metadata.len() != resolved.size || !metadata.is_file() {
                return Err((2, "Share unavailable".to_owned()));
            }
            let length =
                usize::try_from(length).map_err(|_| (4, "Range length is invalid".to_owned()))?;
            let bytes = tokio::task::spawn_blocking(move || {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))?;
                let mut bytes = vec![0_u8; length];
                file.read_exact(&mut bytes)?;
                Ok::<_, std::io::Error>(bytes)
            })
            .await
            .map_err(|_| (10, "Share read task failed".to_owned()))?
            .map_err(|_| (10, "Share read failed".to_owned()))?;
            authorize_grant(state, remote_username, &request).await?;
            Ok(bytes)
        }
        _ => Err((3, "Unknown method".to_owned())),
    }
}

pub(crate) async fn backfill_incoming_share(
    state: &AppState,
    grant_id: &str,
    local_username: &str,
) -> Result<Vec<crate::mesh_services::ShareBackfillReceipt>, String> {
    if state.config.controller_profile != crate::ControllerProfile::Native
        || !state.config.current_upstream_behavior
    {
        return Err(
            "Recipient share backfill is unavailable in this controller profile".to_owned(),
        );
    }
    let _permit = RECIPIENT_BACKFILL_LIMIT
        .try_acquire()
        .map_err(|_| "Another incoming share backfill is already running".to_owned())?;
    let incoming = state
        .incoming_shares
        .read()
        .await
        .list()
        .iter()
        .find(|record| record.id == grant_id)
        .cloned()
        .ok_or_else(|| "Incoming share was not found".to_owned())?;
    if !incoming
        .recipient_user_id
        .eq_ignore_ascii_case(local_username)
        || !share_grant_allows_download(&incoming.permissions)
    {
        return Err("Incoming share is not available for this recipient".to_owned());
    }
    if incoming.owner_user_id.trim().is_empty()
        || incoming.owner_user_id.len() > 512
        || incoming.owner_user_id.chars().any(char::is_control)
        || incoming.id.is_empty()
        || incoming.id.len() > MAX_BACKFILL_GRANT_ID_BYTES
        || incoming.token.trim().is_empty()
        || incoming.token.len() > MAX_BACKFILL_TOKEN_BYTES
        || incoming.token.chars().any(char::is_control)
    {
        return Err("Incoming share authorization is invalid".to_owned());
    }
    let peer = crate::trusted_mesh_peer_for(state, &incoming.owner_user_id)
        .filter(|peer| peer.matches(&incoming.owner_user_id))
        .ok_or_else(|| {
            "Incoming share owner is not a configured certificate-pinned mesh peer".to_owned()
        })?;
    crate::mesh_services::backfill_share(
        state.private_gateway.as_ref(),
        &peer,
        local_username,
        &state.capability_signing_key,
        &incoming.id,
        &incoming.token,
        &crate::effective_downloads_dir(state),
    )
    .await
}

fn validate_request_fields(request: &ShareBackfillRequest) -> Result<(), (i32, String)> {
    if request.grant_id.trim().is_empty()
        || request.grant_id.len() > MAX_BACKFILL_GRANT_ID_BYTES
        || request.grant_id.chars().any(char::is_control)
        || request.token.trim().is_empty()
        || request.token.len() > MAX_BACKFILL_TOKEN_BYTES
        || request.token.chars().any(char::is_control)
        || request.content_id.as_ref().is_some_and(|value| {
            value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control)
        })
    {
        return Err((4, "Invalid share backfill request".to_owned()));
    }
    Ok(())
}

async fn authorize_grant(
    state: &AppState,
    remote_username: &str,
    request: &ShareBackfillRequest,
) -> Result<(String, String, String), (i32, String)> {
    let token_record = {
        let mut access_tokens = state.share_access_tokens.write().await;
        access_tokens.validate(&request.token)
    };
    let token = token_record
        .filter(|record| record.grant_id == request.grant_id)
        .ok_or_else(|| (2, "Share unavailable".to_owned()))?;
    let grant = state
        .share_grants
        .read()
        .await
        .get(&request.grant_id)
        .filter(|grant| grant.id == token.grant_id)
        .ok_or_else(|| (2, "Share unavailable".to_owned()))?;
    let direct_recipient = grant.username.eq_ignore_ascii_case(remote_username);
    let group_recipient = state
        .sharegroups
        .read()
        .await
        .get(&grant.username)
        .is_some_and(|group| {
            group
                .members
                .iter()
                .any(|member| member.username.eq_ignore_ascii_case(remote_username))
        });
    if !direct_recipient && !group_recipient {
        return Err((2, "Share unavailable".to_owned()));
    }
    Ok((grant.id, grant.collection_id, grant.permissions))
}

async fn resolve_item(
    state: &AppState,
    item: &CollectionItem,
) -> Result<(ShareBackfillItem, String), (i32, String)> {
    let filename = item_filename(item);
    let entry = find_shared_entry_for_content(state, Some(&filename), Some(&item.content_id))
        .await
        .ok_or_else(|| (2, "Share item is no longer available".to_owned()))?;
    if entry.size == 0 || entry.size > MAX_BACKFILL_FILE_BYTES {
        return Err((4, "Share item exceeds the backfill file limit".to_owned()));
    }
    let shared = find_shared_local_file(state, &entry.filename)
        .await
        .ok_or_else(|| (2, "Share item is no longer available".to_owned()))?;
    let metadata = tokio::fs::metadata(&shared.local_path)
        .await
        .map_err(|_| (2, "Share item is no longer available".to_owned()))?;
    if !metadata.is_file() || metadata.len() != entry.size {
        return Err((2, "Share item is no longer available".to_owned()));
    }
    let sha256 = crate::sha256_local_file_cached(&shared.local_path)
        .await
        .ok_or_else(|| (10, "Share item hash failed".to_owned()))?;
    Ok((
        ShareBackfillItem {
            content_id: item.content_id.clone(),
            filename: filename
                .rsplit(['/', '\\'])
                .next()
                .filter(|name| !name.is_empty())
                .unwrap_or("shared-file")
                .to_owned(),
            size: entry.size,
            sha256,
        },
        entry.filename,
    ))
}

fn item_filename(item: &CollectionItem) -> String {
    if item.file_name.trim().is_empty() {
        item.title.trim().to_owned()
    } else {
        item.file_name.trim().to_owned()
    }
}

pub(crate) fn safe_backfill_filename(item: &ShareBackfillItem) -> String {
    let digest = item.sha256.to_ascii_lowercase();
    let extension = PathBuf::from(&item.filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            !extension.is_empty()
                && extension.len() <= 16
                && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();
    format!("sha256_{digest}{extension}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(grant_id: &str, token: &str, content_id: Option<&str>) -> ShareBackfillRequest {
        ShareBackfillRequest {
            grant_id: grant_id.to_owned(),
            token: token.to_owned(),
            content_id: content_id.map(str::to_owned),
            offset: None,
            length: None,
        }
    }

    #[test]
    fn rejects_control_characters_in_grant_token_and_content_id() {
        assert!(validate_request_fields(&request("grant-1", "valid-token", None)).is_ok());
        for (grant_id, token, content_id) in [
            ("grant\t1", "valid-token", None),
            ("grant\u{0085}1", "valid-token", None),
            ("grant-1", "token\u{0007}", None),
            ("grant-1", "token\u{007f}", None),
            ("grant-1", "valid-token", Some("content\u{0085}id")),
        ] {
            assert!(
                validate_request_fields(&request(grant_id, token, content_id)).is_err(),
                "accepted a control character in grant_id={grant_id:?} token={token:?} content_id={content_id:?}"
            );
        }
    }
}
