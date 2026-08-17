//! The slskdN mesh-sync message contract.
//!
//! Mesh sync has two transports in the frozen implementation: a length-
//! prefixed JSON overlay and a `MESH:<TYPE>:<JSON>` private-message fallback.
//! This module owns the common typed JSON payload and the private-message
//! envelope and the frozen Ed25519 message/entry signing contracts. Mesh-
//! database dispatch remains a runtime behaviour above this codec.

use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{
    de::{Error as _, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serde_json::Value;

pub const MESH_SYNC_PRIVATE_PREFIX: &str = "MESH:";
/// Frozen slskdN's safe JSON parser rejects remote payloads larger than 1 MiB.
pub const MAX_MESH_SYNC_PAYLOAD_BYTES: usize = 1024 * 1024;
/// Frozen MeshSyncService permits at most twice its normal batch size on input.
pub const MAX_MESH_SYNC_ENTRIES: usize = 2_000;
pub const MAX_MESH_SYNC_CLIENT_FIELD_UTF16_UNITS: usize = 64;
pub const MESH_SYNC_FLAC_KEY_HEX_LEN: usize = 16;
pub const MESH_SYNC_MAX_CHUNK_LENGTH: i32 = 32_768;
pub const MESH_SYNC_MAX_SIGNATURE_AGE_MS: u64 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MeshMessageType {
    Hello = 1,
    ReqDelta = 2,
    PushDelta = 3,
    ReqKey = 4,
    RespKey = 5,
    Ack = 6,
    ReqChunk = 7,
    RespChunk = 8,
    DhtStore = 9,
}

impl MeshMessageType {
    #[must_use]
    pub const fn private_name(self) -> &'static str {
        match self {
            Self::Hello => "HELLO",
            Self::ReqDelta => "REQDELTA",
            Self::PushDelta => "PUSHDELTA",
            Self::ReqKey => "REQKEY",
            Self::RespKey => "RESPKEY",
            Self::Ack => "ACK",
            Self::ReqChunk => "REQCHUNK",
            Self::RespChunk => "RESPCHUNK",
            Self::DhtStore => "DHTSTORE",
        }
    }

    #[must_use]
    pub const fn overlay_name(self) -> &'static str {
        match self {
            Self::Hello => "mesh_sync_hello",
            Self::ReqDelta => "mesh_req_delta",
            Self::PushDelta => "mesh_push_delta",
            Self::ReqKey => "mesh_req_key",
            Self::RespKey => "mesh_resp_key",
            Self::Ack => "mesh_ack",
            Self::ReqChunk => "mesh_req_chunk",
            Self::RespChunk => "mesh_resp_chunk",
            Self::DhtStore => "mesh_dht_store",
        }
    }

    #[must_use]
    pub const fn signing_name(self) -> &'static str {
        match self {
            Self::Hello => "Hello",
            Self::ReqDelta => "ReqDelta",
            Self::PushDelta => "PushDelta",
            Self::ReqKey => "ReqKey",
            Self::RespKey => "RespKey",
            Self::Ack => "Ack",
            Self::ReqChunk => "ReqChunk",
            Self::RespChunk => "RespChunk",
            Self::DhtStore => "DhtStore",
        }
    }
}

impl TryFrom<i32> for MeshMessageType {
    type Error = MeshSyncError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::ReqDelta),
            3 => Ok(Self::PushDelta),
            4 => Ok(Self::ReqKey),
            5 => Ok(Self::RespKey),
            6 => Ok(Self::Ack),
            7 => Ok(Self::ReqChunk),
            8 => Ok(Self::RespChunk),
            9 => Ok(Self::DhtStore),
            other => Err(MeshSyncError::UnknownType(other)),
        }
    }
}

impl Serialize for MeshMessageType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> Deserialize<'de> for MeshMessageType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;
        Self::try_from(value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshSyncBase {
    #[serde(rename = "proto_version")]
    pub protocol_version: i32,
    #[serde(rename = "public_key")]
    pub public_key: String,
    #[serde(rename = "signature")]
    pub signature: String,
    #[serde(rename = "timestamp_ms")]
    pub timestamp_unix_ms: i64,
}

impl Default for MeshSyncBase {
    fn default() -> Self {
        Self {
            protocol_version: 1,
            public_key: String::new(),
            signature: String::new(),
            timestamp_unix_ms: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshHashEntry {
    #[serde(rename = "seq_id")]
    pub sequence_id: i64,
    #[serde(rename = "flac_key")]
    pub flac_key: String,
    #[serde(rename = "byte_hash")]
    pub byte_hash: String,
    pub size: i64,
    #[serde(rename = "meta_flags")]
    pub metadata_flags: Option<i32>,
    #[serde(rename = "signer_pk")]
    pub signer_public_key: Option<String>,
    #[serde(rename = "sig")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshHelloMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "client_id")]
    pub client_id: String,
    #[serde(rename = "client_version")]
    pub client_version: String,
    #[serde(rename = "latest_seq_id")]
    pub latest_sequence_id: i64,
    #[serde(rename = "hash_count")]
    pub hash_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshReqDeltaMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "since_seq_id")]
    pub since_sequence_id: i64,
    #[serde(rename = "max_entries")]
    pub max_entries: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshPushDeltaMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    pub entries: Vec<MeshHashEntry>,
    #[serde(rename = "latest_seq_id")]
    pub latest_sequence_id: i64,
    #[serde(rename = "has_more")]
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshReqKeyMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "flac_key")]
    pub flac_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRespKeyMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "flac_key")]
    pub flac_key: String,
    pub found: bool,
    pub entry: Option<MeshHashEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshAckMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "merged_count")]
    pub merged_count: i32,
    #[serde(rename = "latest_seq_id")]
    pub latest_sequence_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshReqChunkMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "flac_key")]
    pub flac_key: String,
    pub offset: i64,
    pub length: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRespChunkMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    #[serde(rename = "flac_key")]
    pub flac_key: String,
    pub offset: i64,
    #[serde(rename = "data_base64")]
    pub data_base64: String,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtStoreMessage {
    #[serde(rename = "type")]
    pub message_type: MeshMessageType,
    #[serde(flatten)]
    pub base: MeshSyncBase,
    /// Base64 JSON representation of the frozen `byte[] Key` property.
    pub key: String,
    /// Base64 JSON representation of the frozen `byte[] Value` property.
    pub value: String,
    #[serde(rename = "requester_id")]
    pub requester_id: String,
    #[serde(rename = "ttl_seconds")]
    pub ttl_seconds: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshSyncMessage {
    Hello(MeshHelloMessage),
    ReqDelta(MeshReqDeltaMessage),
    PushDelta(MeshPushDeltaMessage),
    ReqKey(MeshReqKeyMessage),
    RespKey(MeshRespKeyMessage),
    Ack(MeshAckMessage),
    ReqChunk(MeshReqChunkMessage),
    RespChunk(MeshRespChunkMessage),
    DhtStore(DhtStoreMessage),
}

impl MeshSyncMessage {
    #[must_use]
    pub const fn message_type(&self) -> MeshMessageType {
        match self {
            Self::Hello(message) => message.message_type,
            Self::ReqDelta(message) => message.message_type,
            Self::PushDelta(message) => message.message_type,
            Self::ReqKey(message) => message.message_type,
            Self::RespKey(message) => message.message_type,
            Self::Ack(message) => message.message_type,
            Self::ReqChunk(message) => message.message_type,
            Self::RespChunk(message) => message.message_type,
            Self::DhtStore(message) => message.message_type,
        }
    }

    /// Validate the message-specific constraints applied by the frozen
    /// `MeshSyncService` before dispatch. Signature verification and the
    /// hash-database entry checks happen at a higher runtime layer.
    pub fn validate(&self) -> Result<(), MeshSyncError> {
        match self {
            Self::Hello(message) => {
                if message.latest_sequence_id < 0 {
                    return Err(MeshSyncError::InvalidField("latest_seq_id"));
                }
                if message.hash_count < 0 {
                    return Err(MeshSyncError::InvalidField("hash_count"));
                }
                if utf16_len(&message.client_id) > MAX_MESH_SYNC_CLIENT_FIELD_UTF16_UNITS {
                    return Err(MeshSyncError::InvalidField("client_id"));
                }
                if utf16_len(&message.client_version) > MAX_MESH_SYNC_CLIENT_FIELD_UTF16_UNITS {
                    return Err(MeshSyncError::InvalidField("client_version"));
                }
            }
            Self::ReqDelta(message) => {
                if message.since_sequence_id < 0 {
                    return Err(MeshSyncError::InvalidField("since_seq_id"));
                }
                if message.max_entries < 0
                    || usize::try_from(message.max_entries).unwrap_or(usize::MAX)
                        > MAX_MESH_SYNC_ENTRIES
                {
                    return Err(MeshSyncError::InvalidField("max_entries"));
                }
            }
            Self::PushDelta(message) => {
                if message.entries.len() > MAX_MESH_SYNC_ENTRIES {
                    return Err(MeshSyncError::InvalidField("entries"));
                }
                if message.latest_sequence_id < 0 {
                    return Err(MeshSyncError::InvalidField("latest_seq_id"));
                }
            }
            Self::ReqKey(message) => validate_flac_key(&message.flac_key)?,
            Self::ReqChunk(message) => {
                validate_flac_key(&message.flac_key)?;
                if message.offset < 0 {
                    return Err(MeshSyncError::InvalidField("offset"));
                }
                if message.length <= 0 || message.length > MESH_SYNC_MAX_CHUNK_LENGTH {
                    return Err(MeshSyncError::InvalidField("length"));
                }
            }
            Self::RespKey(_) | Self::Ack(_) | Self::RespChunk(_) | Self::DhtStore(_) => {}
        }
        Ok(())
    }

    pub fn encode_json(&self) -> Result<Vec<u8>, MeshSyncError> {
        match self {
            Self::Hello(message) => Ok(serde_json::to_vec(message)?),
            Self::ReqDelta(message) => Ok(serde_json::to_vec(message)?),
            Self::PushDelta(message) => Ok(serde_json::to_vec(message)?),
            Self::ReqKey(message) => Ok(serde_json::to_vec(message)?),
            Self::RespKey(message) => Ok(serde_json::to_vec(message)?),
            Self::Ack(message) => Ok(serde_json::to_vec(message)?),
            Self::ReqChunk(message) => Ok(serde_json::to_vec(message)?),
            Self::RespChunk(message) => Ok(serde_json::to_vec(message)?),
            Self::DhtStore(message) => Ok(serde_json::to_vec(message)?),
        }
    }

    /// Sign the message using the frozen slskdN `type|timestamp|payload` format.
    ///
    /// The frozen signer serializes the concrete message with null properties
    /// omitted, removes `signature` from the top-level object, and applies
    /// literal camel-case filters for `publicKey` and `timestampMs`. Those
    /// filters do not match the model's explicit snake-case wire names, so
    /// `public_key` and `timestamp_ms` remain part of the signed JSON payload.
    pub fn sign_at(
        &mut self,
        signing_key: &SigningKey,
        timestamp_unix_ms: i64,
    ) -> Result<(), MeshSyncError> {
        let public_key = BASE64.encode(signing_key.verifying_key().to_bytes());
        {
            let base = self.base_mut();
            base.timestamp_unix_ms = timestamp_unix_ms;
            base.public_key = public_key;
            base.signature.clear();
        }

        let payload_json = self.signing_payload_json()?;
        let signable = format!(
            "{}|{}|{}",
            self.message_type().signing_name(),
            timestamp_unix_ms,
            String::from_utf8(payload_json).map_err(|_| MeshSyncError::JsonNotUtf8)?
        );
        self.base_mut().signature = BASE64.encode(signing_key.sign(signable.as_bytes()).to_bytes());
        Ok(())
    }

    /// Sign using the current Unix timestamp in milliseconds.
    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<(), MeshSyncError> {
        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MeshSyncError::InvalidTimestamp)?
            .as_millis()
            .try_into()
            .map_err(|_| MeshSyncError::InvalidTimestamp)?;
        self.sign_at(signing_key, timestamp_unix_ms)
    }

    /// Return the canonical JSON payload used by the frozen signer.
    pub fn signing_payload_json(&self) -> Result<Vec<u8>, MeshSyncError> {
        let encoded = self.encode_json()?;
        let mut deserializer = serde_json::Deserializer::from_slice(&encoded);
        let value = OrderedJson::deserialize(&mut deserializer)?;
        deserializer.end()?;
        let mut output = Vec::with_capacity(encoded.len());
        value.write_signing_json(&mut output, true)?;
        Ok(output)
    }

    /// Verify a message against an explicit current Unix timestamp.
    pub fn verify_signature_at(&self, now_unix_ms: i64) -> Result<(), MeshSyncError> {
        let base = self.base();
        if base.public_key.trim().is_empty() {
            return Err(MeshSyncError::MissingCredential("public_key"));
        }
        if base.signature.trim().is_empty() {
            return Err(MeshSyncError::MissingCredential("signature"));
        }
        if base.timestamp_unix_ms == 0 {
            return Err(MeshSyncError::InvalidTimestamp);
        }

        let age_ms = now_unix_ms.abs_diff(base.timestamp_unix_ms);
        if age_ms > MESH_SYNC_MAX_SIGNATURE_AGE_MS {
            return Err(MeshSyncError::StaleTimestamp { age_ms });
        }

        let public_key_bytes = BASE64
            .decode(&base.public_key)
            .map_err(|_| MeshSyncError::InvalidBase64("public_key"))?;
        let public_key: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| MeshSyncError::InvalidPublicKey)?;
        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| MeshSyncError::InvalidPublicKey)?;

        let signature_bytes = BASE64
            .decode(&base.signature)
            .map_err(|_| MeshSyncError::InvalidBase64("signature"))?;
        let signature: [u8; 64] = signature_bytes
            .try_into()
            .map_err(|_| MeshSyncError::InvalidSignature)?;

        let payload_json = self.signing_payload_json()?;
        let signable = format!(
            "{}|{}|{}",
            self.message_type().signing_name(),
            base.timestamp_unix_ms,
            String::from_utf8(payload_json).map_err(|_| MeshSyncError::JsonNotUtf8)?
        );
        verifying_key
            .verify_strict(signable.as_bytes(), &Signature::from_bytes(&signature))
            .map_err(|_| MeshSyncError::InvalidSignature)
    }

    /// Verify using the current Unix timestamp in milliseconds.
    pub fn verify_signature(&self) -> Result<(), MeshSyncError> {
        let now_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MeshSyncError::InvalidTimestamp)?
            .as_millis()
            .try_into()
            .map_err(|_| MeshSyncError::InvalidTimestamp)?;
        self.verify_signature_at(now_unix_ms)
    }

    fn base(&self) -> &MeshSyncBase {
        match self {
            Self::Hello(message) => &message.base,
            Self::ReqDelta(message) => &message.base,
            Self::PushDelta(message) => &message.base,
            Self::ReqKey(message) => &message.base,
            Self::RespKey(message) => &message.base,
            Self::Ack(message) => &message.base,
            Self::ReqChunk(message) => &message.base,
            Self::RespChunk(message) => &message.base,
            Self::DhtStore(message) => &message.base,
        }
    }

    fn base_mut(&mut self) -> &mut MeshSyncBase {
        match self {
            Self::Hello(message) => &mut message.base,
            Self::ReqDelta(message) => &mut message.base,
            Self::PushDelta(message) => &mut message.base,
            Self::ReqKey(message) => &mut message.base,
            Self::RespKey(message) => &mut message.base,
            Self::Ack(message) => &mut message.base,
            Self::ReqChunk(message) => &mut message.base,
            Self::RespChunk(message) => &mut message.base,
            Self::DhtStore(message) => &mut message.base,
        }
    }

    pub fn decode_json(bytes: &[u8]) -> Result<Self, MeshSyncError> {
        if bytes.len() > MAX_MESH_SYNC_PAYLOAD_BYTES {
            return Err(MeshSyncError::PayloadTooLarge {
                actual: bytes.len(),
                max: MAX_MESH_SYNC_PAYLOAD_BYTES,
            });
        }
        let value: Value = serde_json::from_slice(bytes)?;
        let raw_type = value
            .get("type")
            .and_then(Value::as_i64)
            .ok_or(MeshSyncError::MissingType)?;
        let message_type = MeshMessageType::try_from(
            i32::try_from(raw_type).map_err(|_| MeshSyncError::InvalidTypeValue)?,
        )?;
        let decoded = match message_type {
            MeshMessageType::Hello => Self::Hello(serde_json::from_value(value)?),
            MeshMessageType::ReqDelta => Self::ReqDelta(serde_json::from_value(value)?),
            MeshMessageType::PushDelta => Self::PushDelta(serde_json::from_value(value)?),
            MeshMessageType::ReqKey => Self::ReqKey(serde_json::from_value(value)?),
            MeshMessageType::RespKey => Self::RespKey(serde_json::from_value(value)?),
            MeshMessageType::Ack => Self::Ack(serde_json::from_value(value)?),
            MeshMessageType::ReqChunk => Self::ReqChunk(serde_json::from_value(value)?),
            MeshMessageType::RespChunk => Self::RespChunk(serde_json::from_value(value)?),
            MeshMessageType::DhtStore => Self::DhtStore(serde_json::from_value(value)?),
        };
        if decoded.message_type() != message_type {
            return Err(MeshSyncError::TypeMismatch {
                expected: message_type as i32,
                actual: decoded.message_type() as i32,
            });
        }
        Ok(decoded)
    }

    pub fn encode_private_message(&self) -> Result<String, MeshSyncError> {
        let payload =
            String::from_utf8(self.encode_json()?).map_err(|_| MeshSyncError::JsonNotUtf8)?;
        Ok(format!(
            "{MESH_SYNC_PRIVATE_PREFIX}{}:{payload}",
            self.message_type().private_name()
        ))
    }

    pub fn decode_private_message(message: &str) -> Result<Self, MeshSyncError> {
        let body = message
            .strip_prefix(MESH_SYNC_PRIVATE_PREFIX)
            .ok_or(MeshSyncError::InvalidPrivateEnvelope)?;
        let (wire_name, payload) = body
            .split_once(':')
            .ok_or(MeshSyncError::InvalidPrivateEnvelope)?;
        let decoded = Self::decode_json(payload.as_bytes())?;
        if decoded.message_type().private_name() != wire_name {
            return Err(MeshSyncError::PrivateTypeMismatch {
                expected: decoded.message_type().private_name(),
                actual: wire_name.to_owned(),
            });
        }
        Ok(decoded)
    }
}

/// Return the domain-separated canonical bytes used for a mesh hash-entry
/// signature by frozen slskdN.
pub fn mesh_hash_entry_signing_bytes(entry: &MeshHashEntry) -> Vec<u8> {
    const DOMAIN_TAG: &[u8] = b"slskdn/hashdb-entry/v1";
    const MISSING_META_FLAGS: i32 = i32::MIN;

    let mut bytes = Vec::new();
    append_length_prefixed_bytes(&mut bytes, DOMAIN_TAG);
    append_length_prefixed_bytes(&mut bytes, entry.flac_key.as_bytes());
    append_length_prefixed_bytes(&mut bytes, entry.byte_hash.as_bytes());
    bytes.extend_from_slice(&entry.size.to_be_bytes());
    bytes.extend_from_slice(
        &entry
            .metadata_flags
            .unwrap_or(MISSING_META_FLAGS)
            .to_be_bytes(),
    );
    bytes
}

/// Sign a hash entry in place using frozen slskdN's entry-signing contract.
pub fn sign_mesh_hash_entry(
    entry: &mut MeshHashEntry,
    signing_key: &SigningKey,
) -> Result<(), MeshSyncError> {
    entry.signer_public_key = Some(BASE64.encode(signing_key.verifying_key().to_bytes()));
    entry.signature = Some(
        BASE64.encode(
            signing_key
                .sign(&mesh_hash_entry_signing_bytes(entry))
                .to_bytes(),
        ),
    );
    Ok(())
}

/// Verify a signed hash entry and return the embedded Ed25519 public key.
pub fn verify_mesh_hash_entry_signature(entry: &MeshHashEntry) -> Result<[u8; 32], MeshSyncError> {
    let public_key = entry
        .signer_public_key
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or(MeshSyncError::MissingCredential("signer_pk"))?;
    let signature = entry
        .signature
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or(MeshSyncError::MissingCredential("sig"))?;
    let public_key_bytes: [u8; 32] = BASE64
        .decode(public_key)
        .map_err(|_| MeshSyncError::InvalidBase64("signer_pk"))?
        .try_into()
        .map_err(|_| MeshSyncError::InvalidPublicKey)?;
    let signature_bytes: [u8; 64] = BASE64
        .decode(signature)
        .map_err(|_| MeshSyncError::InvalidBase64("sig"))?
        .try_into()
        .map_err(|_| MeshSyncError::InvalidSignature)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key_bytes).map_err(|_| MeshSyncError::InvalidPublicKey)?;
    verifying_key
        .verify_strict(
            &mesh_hash_entry_signing_bytes(entry),
            &Signature::from_bytes(&signature_bytes),
        )
        .map_err(|_| MeshSyncError::InvalidSignature)?;
    Ok(public_key_bytes)
}

fn append_length_prefixed_bytes(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as i32).to_be_bytes());
    output.extend_from_slice(value);
}

#[derive(Debug, thiserror::Error)]
pub enum MeshSyncError {
    #[error("mesh sync JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("mesh sync message type is missing")]
    MissingType,
    #[error("mesh sync message type is not a valid signed 32-bit integer")]
    InvalidTypeValue,
    #[error("mesh sync payload is too large: {actual} bytes (maximum {max})")]
    PayloadTooLarge { actual: usize, max: usize },
    #[error("unknown mesh sync message type {0}")]
    UnknownType(i32),
    #[error("mesh sync message type mismatch: expected {expected}, decoded {actual}")]
    TypeMismatch { expected: i32, actual: i32 },
    #[error("mesh sync JSON payload is not UTF-8")]
    JsonNotUtf8,
    #[error("invalid mesh sync private-message envelope")]
    InvalidPrivateEnvelope,
    #[error("mesh sync private-message type mismatch: expected {expected}, got {actual}")]
    PrivateTypeMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid mesh sync field: {0}")]
    InvalidField(&'static str),
    #[error("mesh sync message is missing {0}")]
    MissingCredential(&'static str),
    #[error("mesh sync timestamp is invalid")]
    InvalidTimestamp,
    #[error("mesh sync timestamp age is too large: {age_ms}ms")]
    StaleTimestamp { age_ms: u64 },
    #[error("mesh sync {0} is not valid base64")]
    InvalidBase64(&'static str),
    #[error("mesh sync public key is invalid")]
    InvalidPublicKey,
    #[error("mesh sync signature is invalid")]
    InvalidSignature,
}

#[derive(Debug)]
enum OrderedJson {
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Self>),
    Object(Vec<(String, Self)>),
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderedJsonVisitor;

        impl<'de> Visitor<'de> for OrderedJsonVisitor {
            type Value = OrderedJson;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrderedJson::Null)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrderedJson::Bool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrderedJson::Number(value.into()))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrderedJson::Number(value.into()))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let number = serde_json::Number::from_f64(value)
                    .ok_or_else(|| E::custom("non-finite JSON number"))?;
                Ok(OrderedJson::Number(number))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrderedJson::String(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(OrderedJson::String(value))
            }

            fn visit_seq<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = Vec::new();
                while let Some(value) = access.next_element()? {
                    values.push(value);
                }
                Ok(OrderedJson::Array(values))
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut fields = Vec::new();
                while let Some(key) = access.next_key::<String>()? {
                    fields.push((key, access.next_value()?));
                }
                Ok(OrderedJson::Object(fields))
            }
        }

        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

impl OrderedJson {
    fn write_signing_json(
        &self,
        output: &mut Vec<u8>,
        top_level: bool,
    ) -> Result<(), MeshSyncError> {
        match self {
            Self::Null => output.extend_from_slice(b"null"),
            Self::Bool(value) => {
                output.extend_from_slice(if *value { b"true" } else { b"false" });
            }
            Self::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
            Self::String(value) => append_csharp_json_string(output, value),
            Self::Array(values) => {
                output.push(b'[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(b',');
                    }
                    value.write_signing_json(output, false)?;
                }
                output.push(b']');
            }
            Self::Object(fields) => {
                output.push(b'{');
                let mut wrote_field = false;
                for (name, value) in fields {
                    let ignored_top_level = top_level
                        && (name.eq_ignore_ascii_case("publicKey")
                            || name.eq_ignore_ascii_case("signature")
                            || name.eq_ignore_ascii_case("timestampMs"));
                    if ignored_top_level || matches!(value, Self::Null) {
                        continue;
                    }
                    if wrote_field {
                        output.push(b',');
                    }
                    append_csharp_json_string(output, name);
                    output.push(b':');
                    value.write_signing_json(output, false)?;
                    wrote_field = true;
                }
                output.push(b'}');
            }
        }
        Ok(())
    }
}

fn append_csharp_json_string(output: &mut Vec<u8>, value: &str) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    output.push(b'"');
    for unit in value.encode_utf16() {
        match unit {
            0x22 => output.extend_from_slice(br#"\""#),
            0x5c => output.extend_from_slice(br#"\\"#),
            0x08 => output.extend_from_slice(br#"\b"#),
            0x09 => output.extend_from_slice(br#"\t"#),
            0x0a => output.extend_from_slice(br#"\n"#),
            0x0c => output.extend_from_slice(br#"\f"#),
            0x0d => output.extend_from_slice(br#"\r"#),
            unit if unit < 0x20 => {
                output.extend_from_slice(br#"\u"#);
                output.push(HEX[((unit >> 12) & 0xf) as usize]);
                output.push(HEX[((unit >> 8) & 0xf) as usize]);
                output.push(HEX[((unit >> 4) & 0xf) as usize]);
                output.push(HEX[(unit & 0xf) as usize]);
            }
            0x3c => output.extend_from_slice(br#"\u003C"#),
            0x3e => output.extend_from_slice(br#"\u003E"#),
            0x26 => output.extend_from_slice(br#"\u0026"#),
            0x27 => output.extend_from_slice(br#"\u0027"#),
            0x20..=0x7e => output.push(unit as u8),
            _ => {
                output.extend_from_slice(br#"\u"#);
                output.push(HEX[((unit >> 12) & 0xf) as usize]);
                output.push(HEX[((unit >> 8) & 0xf) as usize]);
                output.push(HEX[((unit >> 4) & 0xf) as usize]);
                output.push(HEX[(unit & 0xf) as usize]);
            }
        }
    }
    output.push(b'"');
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn validate_flac_key(value: &str) -> Result<(), MeshSyncError> {
    if value.len() != MESH_SYNC_FLAC_KEY_HEX_LEN
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(MeshSyncError::InvalidField("flac_key"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use ed25519_dalek::SigningKey;

    use super::BASE64;
    use super::{
        mesh_hash_entry_signing_bytes, sign_mesh_hash_entry, verify_mesh_hash_entry_signature,
        MeshHashEntry, MeshHelloMessage, MeshMessageType, MeshReqChunkMessage, MeshReqDeltaMessage,
        MeshReqKeyMessage, MeshSyncBase, MeshSyncError, MeshSyncMessage,
        MAX_MESH_SYNC_PAYLOAD_BYTES, MESH_SYNC_MAX_SIGNATURE_AGE_MS,
    };

    fn hello() -> MeshSyncMessage {
        MeshSyncMessage::Hello(MeshHelloMessage {
            message_type: MeshMessageType::Hello,
            base: MeshSyncBase::default(),
            client_id: "peer".to_owned(),
            client_version: "1.0".to_owned(),
            latest_sequence_id: 0,
            hash_count: 0,
        })
    }

    #[test]
    fn mesh_sync_uses_numeric_type_and_frozen_property_names() {
        let encoded = hello().encode_json().expect("encode hello");
        let json = String::from_utf8(encoded).expect("JSON is UTF-8");
        assert!(json.contains("\"type\":1"));
        assert!(json.contains("\"proto_version\":1"));
        assert!(json.contains("\"client_id\":\"peer\""));
        assert!(json.contains("\"latest_seq_id\":0"));
    }

    #[test]
    fn mesh_sync_rejects_unknown_and_mislabeled_private_messages() {
        let unknown = br#"{"type":99}"#;
        assert!(matches!(
            MeshSyncMessage::decode_json(unknown),
            Err(MeshSyncError::UnknownType(99))
        ));

        let valid = hello()
            .encode_private_message()
            .expect("encode private hello");
        let mislabeled = valid.replacen("MESH:HELLO:", "MESH:REQKEY:", 1);
        assert!(matches!(
            MeshSyncMessage::decode_private_message(&mislabeled),
            Err(MeshSyncError::PrivateTypeMismatch { .. })
        ));
    }

    #[test]
    fn mesh_sync_matches_frozen_message_specific_validation() {
        let base = MeshSyncBase::default();
        let valid_key = "0123456789abcdef".to_owned();
        assert!(MeshSyncMessage::Hello(MeshHelloMessage {
            message_type: MeshMessageType::Hello,
            base: base.clone(),
            client_id: "peer".to_owned(),
            client_version: "1.0".to_owned(),
            latest_sequence_id: 0,
            hash_count: 0,
        })
        .validate()
        .is_ok());
        assert!(MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
            message_type: MeshMessageType::ReqDelta,
            base: base.clone(),
            since_sequence_id: 0,
            max_entries: 2_000,
        })
        .validate()
        .is_ok());
        assert!(MeshSyncMessage::ReqKey(MeshReqKeyMessage {
            message_type: MeshMessageType::ReqKey,
            base: base.clone(),
            flac_key: valid_key.clone(),
        })
        .validate()
        .is_ok());
        assert!(MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
            message_type: MeshMessageType::ReqChunk,
            base,
            flac_key: valid_key,
            offset: 0,
            length: 32_768,
        })
        .validate()
        .is_ok());

        let mut invalid_delta = MeshSyncMessage::ReqDelta(MeshReqDeltaMessage {
            message_type: MeshMessageType::ReqDelta,
            base: MeshSyncBase::default(),
            since_sequence_id: -1,
            max_entries: 0,
        });
        assert!(matches!(
            invalid_delta.validate(),
            Err(MeshSyncError::InvalidField("since_seq_id"))
        ));
        if let MeshSyncMessage::ReqDelta(message) = &mut invalid_delta {
            message.since_sequence_id = 0;
            message.max_entries = 2_001;
        }
        assert!(matches!(
            invalid_delta.validate(),
            Err(MeshSyncError::InvalidField("max_entries"))
        ));

        let invalid_key = MeshSyncMessage::ReqKey(MeshReqKeyMessage {
            message_type: MeshMessageType::ReqKey,
            base: MeshSyncBase::default(),
            flac_key: "not-a-flac-key".to_owned(),
        });
        assert!(matches!(
            invalid_key.validate(),
            Err(MeshSyncError::InvalidField("flac_key"))
        ));

        let invalid_chunk = MeshSyncMessage::ReqChunk(MeshReqChunkMessage {
            message_type: MeshMessageType::ReqChunk,
            base: MeshSyncBase::default(),
            flac_key: "0123456789abcdef".to_owned(),
            offset: 0,
            length: 32_769,
        });
        assert!(matches!(
            invalid_chunk.validate(),
            Err(MeshSyncError::InvalidField("length"))
        ));

        assert!(matches!(
            MeshSyncMessage::decode_json(&vec![b' '; MAX_MESH_SYNC_PAYLOAD_BYTES + 1]),
            Err(MeshSyncError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn mesh_sync_signing_matches_frozen_canonical_payload_and_verifies() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let timestamp = 1_700_000_000_000;
        let mut message = hello();
        message
            .sign_at(&signing_key, timestamp)
            .expect("sign mesh hello");

        let public_key = BASE64.encode(signing_key.verifying_key().to_bytes());
        assert_eq!(
            String::from_utf8(message.signing_payload_json().expect("canonical payload"))
                .expect("canonical JSON"),
            format!(
                r#"{{"type":1,"proto_version":1,"public_key":"{public_key}","timestamp_ms":1700000000000,"client_id":"peer","client_version":"1.0","latest_seq_id":0,"hash_count":0}}"#
            )
        );
        message
            .verify_signature_at(timestamp)
            .expect("verify mesh hello");

        if let MeshSyncMessage::Hello(message) = &mut message {
            message.client_id = "tampered".to_owned();
        }
        assert!(matches!(
            message.verify_signature_at(timestamp),
            Err(MeshSyncError::InvalidSignature)
        ));
    }

    #[test]
    fn mesh_sync_signatures_reject_stale_and_missing_credentials() {
        let signing_key = SigningKey::from_bytes(&[8; 32]);
        let mut message = hello();
        message
            .sign_at(&signing_key, 1_700_000_000_000)
            .expect("sign mesh hello");
        assert!(matches!(
            message.verify_signature_at(
                1_700_000_000_000 + MESH_SYNC_MAX_SIGNATURE_AGE_MS as i64 + 1,
            ),
            Err(MeshSyncError::StaleTimestamp { .. })
        ));

        if let MeshSyncMessage::Hello(message) = &mut message {
            message.base.signature.clear();
        }
        assert!(matches!(
            message.verify_signature_at(1_700_000_000_000),
            Err(MeshSyncError::MissingCredential("signature"))
        ));
    }

    #[test]
    fn mesh_hash_entry_signing_matches_frozen_domain_and_ignores_sequence_id() {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let mut entry = MeshHashEntry {
            sequence_id: 42,
            flac_key: "deadbeefcafebabe".to_owned(),
            byte_hash: "a".repeat(64),
            size: 123_456_789,
            metadata_flags: Some(0x1234),
            signer_public_key: None,
            signature: None,
        };
        assert_eq!(mesh_hash_entry_signing_bytes(&entry).len(), 126);
        sign_mesh_hash_entry(&mut entry, &signing_key).expect("sign hash entry");
        verify_mesh_hash_entry_signature(&entry).expect("verify hash entry");

        entry.sequence_id = 9_999;
        verify_mesh_hash_entry_signature(&entry).expect("sequence id is not signed");
        entry.size += 1;
        assert!(matches!(
            verify_mesh_hash_entry_signature(&entry),
            Err(MeshSyncError::InvalidSignature)
        ));
    }
}
