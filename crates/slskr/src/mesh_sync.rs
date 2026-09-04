//! Runtime handling for the frozen native profile `MESH:<TYPE>:<JSON>` private-message
//! protocol.

use std::{
    fs::File,
    io::{Read as _, Seek as _, SeekFrom},
    path::PathBuf,
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use slskr_client::{
    mesh_sync::{
        sign_mesh_hash_entry, MeshAckMessage, MeshHashEntry, MeshHelloMessage, MeshMessageType,
        MeshPushDeltaMessage, MeshReqChunkMessage, MeshReqDeltaMessage, MeshReqKeyMessage,
        MeshRespChunkMessage, MeshRespKeyMessage, MeshSyncBase, MeshSyncMessage,
    },
    protocol::server::ServerMessage,
    server::ServerSession,
};
use tokio::io::{AsyncRead, AsyncWrite};

const MESH_SYNC_PREFIX: &str = "MESH:";
const MAX_ENTRIES_PER_SYNC: usize = 1_000;
const MAX_MESH_FILE_SIZE: u64 = 10_000_000_000;

/// Handle one inbound private message. Return `true` when the message belongs
/// to mesh-sync, including malformed or intentionally dropped messages, so the
/// ordinary private-message persistence/auto-response path does not treat a
/// protocol frame as user chat.
pub(crate) async fn handle_private_message<S>(
    state: &super::AppState,
    session: &mut ServerSession<S>,
    from_user: &str,
    body: &str,
) -> bool
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Some(normalized_body) = normalize_mesh_body(body) else {
        return false;
    };
    let username = from_user.trim();
    if username.is_empty()
        || username.encode_utf16().count() > 64
        || username.chars().any(char::is_control)
    {
        reject_message(state, username).await;
        return true;
    }

    let message = match MeshSyncMessage::decode_private_message(&normalized_body) {
        Ok(message) => message,
        Err(error) => {
            tracing::debug!(%error, %username, "invalid mesh-sync private message");
            reject_message(state, username).await;
            return true;
        }
    };
    if let Some(response) = handle_signed_message(state, username, message).await {
        send_message(session, username, response).await;
    }
    true
}

/// Validate and dispatch one decoded mesh message, returning the signed
/// response produced by the frozen `MeshSyncService` dispatcher. `None` means
/// either rejection or one of the valid response/DHT message types that the
/// frozen inbound dispatcher consumes without replying.
pub(crate) async fn handle_signed_message(
    state: &super::AppState,
    from_user: &str,
    message: MeshSyncMessage,
) -> Option<MeshSyncMessage> {
    let username = from_user.trim();
    if username.is_empty()
        || username.encode_utf16().count() > 64
        || username.chars().any(char::is_control)
    {
        reject_message(state, username).await;
        return None;
    }
    if message.verify_signature().is_err() || message.validate().is_err() {
        reject_message(state, username).await;
        return None;
    }
    {
        let mut mesh = state.mesh.write().await;
        if !mesh.enabled || mesh.sync_is_quarantined(username, super::unix_timestamp()) {
            mesh.sync_rejected_messages = mesh.sync_rejected_messages.saturating_add(1);
            return None;
        }
    }

    let mut response = dispatch_message(state, username, message).await?;
    response.sign(&state.capability_signing_key).ok()?;
    Some(response)
}

async fn dispatch_message(
    state: &super::AppState,
    username: &str,
    message: MeshSyncMessage,
) -> Option<MeshSyncMessage> {
    match message {
        MeshSyncMessage::Hello(_request) => Some(MeshSyncMessage::Hello(build_hello(state).await)),
        MeshSyncMessage::ReqDelta(request) => Some(MeshSyncMessage::PushDelta(
            build_delta(state, request).await,
        )),
        MeshSyncMessage::PushDelta(request) => Some(MeshSyncMessage::Ack(
            merge_delta(state, username, request).await,
        )),
        MeshSyncMessage::ReqKey(request) => {
            Some(MeshSyncMessage::RespKey(lookup_key(state, request).await))
        }
        MeshSyncMessage::ReqChunk(request) => {
            Some(MeshSyncMessage::RespChunk(read_chunk(state, request).await))
        }
        // The frozen service routes these through its pending-request router or
        // a separate DHT service. A valid frame is consumed but receives no
        // response from this inbound dispatcher.
        MeshSyncMessage::RespKey(_)
        | MeshSyncMessage::Ack(_)
        | MeshSyncMessage::RespChunk(_)
        | MeshSyncMessage::DhtStore(_) => None,
    }
}

fn normalize_mesh_body(body: &str) -> Option<String> {
    let prefix = body.get(..MESH_SYNC_PREFIX.len())?;
    if !prefix.eq_ignore_ascii_case(MESH_SYNC_PREFIX) {
        return None;
    }
    if prefix == MESH_SYNC_PREFIX {
        Some(body.to_owned())
    } else {
        Some(format!(
            "{MESH_SYNC_PREFIX}{}",
            &body[MESH_SYNC_PREFIX.len()..]
        ))
    }
}

async fn reject_message(state: &super::AppState, username: &str) {
    let settings = state
        .advanced_networking
        .read()
        .await
        .mesh_sync_security
        .clone();
    let mut mesh = state.mesh.write().await;
    mesh.sync_rejected_messages = mesh.sync_rejected_messages.saturating_add(1);
    let _ = mesh.record_invalid_sync_entries(username, 1, &settings, super::unix_timestamp());
}

async fn send_message<S>(session: &mut ServerSession<S>, username: &str, message: MeshSyncMessage)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let body = match message.encode_private_message() {
        Ok(body) => body,
        Err(error) => {
            tracing::warn!(%username, %error, "mesh-sync response could not be encoded");
            return;
        }
    };
    if let Err(error) = session
        .send_server_message(ServerMessage::MessageUserRequest {
            username: username.to_owned(),
            message: body,
        })
        .await
    {
        tracing::warn!(%username, %error, "mesh-sync response could not be sent");
    }
}

async fn build_hello(state: &super::AppState) -> MeshHelloMessage {
    let (client_id, configured_version) = (
        state.config.username.clone(),
        Some(super::APP_VERSION.to_owned()),
    );
    let client_id = client_id
        .filter(|value| !value.trim().is_empty())
        .or_else(|| configured_version.clone())
        .unwrap_or_else(|| "slskr".to_owned());
    let discovery = state.content_discovery.read().await;
    MeshHelloMessage {
        message_type: MeshMessageType::Hello,
        base: MeshSyncBase::default(),
        client_id,
        client_version: configured_version.unwrap_or_else(|| "slskr".to_owned()),
        latest_sequence_id: i64::try_from(discovery.latest_seq()).unwrap_or(i64::MAX),
        hash_count: i32::try_from(discovery.hash_entries().len()).unwrap_or(i32::MAX),
    }
}

async fn build_delta(
    state: &super::AppState,
    request: MeshReqDeltaMessage,
) -> MeshPushDeltaMessage {
    let max_entries = usize::try_from(request.max_entries)
        .unwrap_or_default()
        .min(MAX_ENTRIES_PER_SYNC);
    let discovery = state.content_discovery.read().await;
    let (entries, has_more) = discovery.hash_entries_since_seq(
        u64::try_from(request.since_sequence_id).unwrap_or_default(),
        max_entries,
    );
    let signing_key = state.capability_signing_key.clone();
    let entries = entries
        .iter()
        .filter_map(|entry| mesh_hash_entry(entry, Some(&signing_key)))
        .collect();
    MeshPushDeltaMessage {
        message_type: MeshMessageType::PushDelta,
        base: MeshSyncBase::default(),
        entries,
        latest_sequence_id: i64::try_from(discovery.latest_seq()).unwrap_or(i64::MAX),
        has_more,
    }
}

async fn merge_delta(
    state: &super::AppState,
    username: &str,
    request: MeshPushDeltaMessage,
) -> MeshAckMessage {
    let received = request.entries.len();
    let require_signed_entries = state
        .advanced_networking
        .read()
        .await
        .mesh_sync_security
        .require_signed_entries;
    let mut incoming = Vec::with_capacity(received);
    let mut skipped = 0_u64;
    for entry in request.entries {
        let has_signature = entry.signer_public_key.is_some() && entry.signature.is_some();
        let has_any_signature_field =
            entry.signer_public_key.is_some() || entry.signature.is_some();
        if (require_signed_entries && !has_signature)
            || (has_any_signature_field
                && (!has_signature
                    || slskr_client::mesh_sync::verify_mesh_hash_entry_signature(&entry).is_err()))
        {
            skipped = skipped.saturating_add(1);
            continue;
        }
        let Some(entry) = hash_db_entry(entry) else {
            skipped = skipped.saturating_add(1);
            continue;
        };
        incoming.push(entry);
    }

    let incoming_count = incoming.len();
    let merge_result = if incoming.is_empty() {
        Ok((0, 0))
    } else {
        state
            .content_discovery
            .write()
            .await
            .merge_hash_entries_skipping_invalid(incoming)
    };
    let (merged, skipped_by_store) = match merge_result {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(%username, %error, incoming_count, "mesh-sync delta merge failed");
            (0, incoming_count)
        }
    };
    skipped = skipped.saturating_add(skipped_by_store as u64);
    let latest = state.content_discovery.read().await.latest_seq();
    {
        let mut mesh = state.mesh.write().await;
        mesh.sync_merge_total = mesh.sync_merge_total.saturating_add(1);
        mesh.sync_entries_received = mesh.sync_entries_received.saturating_add(received as u64);
        mesh.sync_entries_merged = mesh.sync_entries_merged.saturating_add(merged as u64);
        mesh.sync_skipped_entries = mesh.sync_skipped_entries.saturating_add(skipped);
        if skipped == 0 {
            mesh.sync_merge_successful = mesh.sync_merge_successful.saturating_add(1);
        } else {
            mesh.sync_merge_failed = mesh.sync_merge_failed.saturating_add(1);
        }
    }
    if skipped > 0 {
        tracing::debug!(%username, received, skipped, "mesh-sync delta contained skipped entries");
    }
    MeshAckMessage {
        message_type: MeshMessageType::Ack,
        base: MeshSyncBase::default(),
        merged_count: i32::try_from(merged).unwrap_or(i32::MAX),
        latest_sequence_id: i64::try_from(latest).unwrap_or(i64::MAX),
    }
}

async fn lookup_key(state: &super::AppState, request: MeshReqKeyMessage) -> MeshRespKeyMessage {
    let discovery = state.content_discovery.read().await;
    let entry = discovery
        .lookup_hash(&request.flac_key)
        .and_then(|entry| mesh_hash_entry(entry, None));
    MeshRespKeyMessage {
        message_type: MeshMessageType::RespKey,
        base: MeshSyncBase::default(),
        flac_key: request.flac_key,
        found: entry.is_some(),
        entry,
    }
}

async fn read_chunk(state: &super::AppState, request: MeshReqChunkMessage) -> MeshRespChunkMessage {
    let path = {
        let shares = state.shares.read().await;
        shares
            .entries
            .iter()
            .find(|entry| {
                entry.size <= MAX_MESH_FILE_SIZE
                    && super::content_discovery::generate_flac_key(&entry.filename, entry.size)
                        .eq_ignore_ascii_case(&request.flac_key)
            })
            .and_then(|entry| shares.local_paths.get(&entry.filename).cloned())
    };
    let Some(path) = path else {
        return failed_chunk(&request);
    };
    let offset = request.offset;
    let length = request.length;
    let result = tokio::task::spawn_blocking(move || read_file_chunk(path, offset, length)).await;
    match result {
        Ok(Ok(data)) => MeshRespChunkMessage {
            message_type: MeshMessageType::RespChunk,
            base: MeshSyncBase::default(),
            flac_key: request.flac_key,
            offset,
            data_base64: BASE64.encode(data),
            success: true,
        },
        _ => failed_chunk(&request),
    }
}

fn failed_chunk(request: &MeshReqChunkMessage) -> MeshRespChunkMessage {
    MeshRespChunkMessage {
        message_type: MeshMessageType::RespChunk,
        base: MeshSyncBase::default(),
        flac_key: request.flac_key.clone(),
        offset: request.offset,
        data_base64: String::new(),
        success: false,
    }
}

fn read_file_chunk(path: PathBuf, offset: i64, length: i32) -> Result<Vec<u8>, String> {
    let offset = u64::try_from(offset).map_err(|_| "negative chunk offset".to_owned())?;
    let length = usize::try_from(length).map_err(|_| "invalid chunk length".to_owned())?;
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let size = file.metadata().map_err(|error| error.to_string())?.len();
    if offset >= size {
        return Err("chunk offset is outside the file".to_owned());
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| error.to_string())?;
    let to_read = length.min(usize::try_from(size - offset).unwrap_or(usize::MAX));
    let mut data = vec![0_u8; to_read];
    file.read_exact(&mut data)
        .map_err(|error| error.to_string())?;
    Ok(data)
}

fn mesh_hash_entry(
    entry: &super::content_discovery::HashDbEntry,
    signing_key: Option<&ed25519_dalek::SigningKey>,
) -> Option<MeshHashEntry> {
    let size = i64::try_from(entry.size).ok()?;
    let sequence_id = i64::try_from(entry.seq_id).ok()?;
    let byte_hash = if !entry.byte_hash.is_empty() {
        entry.byte_hash.clone()
    } else if !entry.full_file_hash.is_empty() {
        entry.full_file_hash.clone()
    } else {
        entry.file_sha256.clone()
    };
    if entry.flac_key.is_empty() || byte_hash.is_empty() || size <= 0 {
        return None;
    }
    let mut wire = MeshHashEntry {
        sequence_id,
        flac_key: entry.flac_key.clone(),
        byte_hash,
        size,
        metadata_flags: None,
        signer_public_key: None,
        signature: None,
    };
    if let Some(signing_key) = signing_key {
        sign_mesh_hash_entry(&mut wire, signing_key).ok()?;
    }
    Some(wire)
}

fn hash_db_entry(entry: MeshHashEntry) -> Option<super::content_discovery::HashDbEntry> {
    if entry.flac_key.is_empty()
        || entry.byte_hash.len() != 64
        || !entry.byte_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        || entry.size <= 0
    {
        return None;
    }
    Some(super::content_discovery::HashDbEntry {
        flac_key: entry.flac_key,
        byte_hash: entry.byte_hash,
        size: u64::try_from(entry.size).ok()?,
        ..super::content_discovery::HashDbEntry::default()
    })
}

#[allow(dead_code)]
fn _mesh_sync_message_types_are_exhaustive(message: &MeshSyncMessage) {
    let _ = match message {
        MeshSyncMessage::Hello(_) => MeshMessageType::Hello,
        MeshSyncMessage::ReqDelta(_) => MeshMessageType::ReqDelta,
        MeshSyncMessage::PushDelta(_) => MeshMessageType::PushDelta,
        MeshSyncMessage::ReqKey(_) => MeshMessageType::ReqKey,
        MeshSyncMessage::RespKey(_) => MeshMessageType::RespKey,
        MeshSyncMessage::Ack(_) => MeshMessageType::Ack,
        MeshSyncMessage::ReqChunk(_) => MeshMessageType::ReqChunk,
        MeshSyncMessage::RespChunk(_) => MeshMessageType::RespChunk,
        MeshSyncMessage::DhtStore(_) => MeshMessageType::DhtStore,
    };
}
