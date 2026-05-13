use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use slskr_protocol::{peer::PeerMessage, DecodeError, Reader, Writer};

pub const PEER_CAPABILITY_MESSAGE_CODE: u32 = 0x534C_534B;
pub const PEER_CAPABILITY_ENVELOPE_VERSION: u16 = 1;
pub const MAX_CAPABILITY_ENVELOPE_BYTES: usize = 64 * 1024;
pub const FEATURE_CAPABILITIES_V1: &str = "slskdn-capabilities-v1";
pub const FEATURE_MESH_V1: &str = "slskdn-mesh-v1";
pub const FEATURE_SHARED_UDP_V1: &str = "slskdn-shared-udp-v1";
pub const FEATURE_WISHLIST_V1: &str = "slskdn-wishlist-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PeerCapabilityMessageType {
    Hello = 1,
    Acknowledge = 2,
}

impl TryFrom<u8> for PeerCapabilityMessageType {
    type Error = CapabilityError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Acknowledge),
            _ => Err(CapabilityError::InvalidMessageType(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerCapabilityEnvelope {
    pub version: u16,
    pub message_type: PeerCapabilityMessageType,
    pub nonce: [u8; 16],
    pub descriptor: PeerCapabilityDescriptor,
}

impl PeerCapabilityEnvelope {
    #[must_use]
    pub const fn new(
        message_type: PeerCapabilityMessageType,
        nonce: [u8; 16],
        descriptor: PeerCapabilityDescriptor,
    ) -> Self {
        Self {
            version: PEER_CAPABILITY_ENVELOPE_VERSION,
            message_type,
            nonce,
            descriptor,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, CapabilityError> {
        let mut writer = Writer::new();
        writer.write_u16_le(self.version);
        writer.write_u8(self.message_type as u8);
        writer.write_bytes(&self.nonce);
        write_protocol_string(&mut writer, "peer_id", &self.descriptor.peer_id)?;
        write_protocol_string(&mut writer, "username", &self.descriptor.username)?;
        write_protocol_string_list(&mut writer, "features", &self.descriptor.features)?;
        write_protocol_string_list(&mut writer, "endpoints", &self.descriptor.endpoints)?;
        writer.write_u64_le(self.descriptor.issued_at_unix);
        writer.write_u64_le(self.descriptor.expires_at_unix);
        writer.write_bytes(&self.descriptor.public_key);
        writer.write_bytes(
            &self
                .descriptor
                .signature
                .ok_or(CapabilityError::MissingSignature)?,
        );
        let payload = writer.into_inner();
        if payload.len() > MAX_CAPABILITY_ENVELOPE_BYTES {
            return Err(CapabilityError::EnvelopeTooLarge {
                length: payload.len(),
                max: MAX_CAPABILITY_ENVELOPE_BYTES,
            });
        }
        Ok(payload)
    }

    pub fn decode(payload: &[u8]) -> Result<Self, CapabilityError> {
        if payload.len() > MAX_CAPABILITY_ENVELOPE_BYTES {
            return Err(CapabilityError::EnvelopeTooLarge {
                length: payload.len(),
                max: MAX_CAPABILITY_ENVELOPE_BYTES,
            });
        }

        let mut reader = Reader::new(payload);
        let version = reader.read_u16_le()?;
        if version != PEER_CAPABILITY_ENVELOPE_VERSION {
            return Err(CapabilityError::UnsupportedVersion(version));
        }
        let message_type = PeerCapabilityMessageType::try_from(reader.read_u8()?)?;
        let nonce = read_array::<16>(&mut reader, "nonce")?;
        let descriptor = PeerCapabilityDescriptor {
            peer_id: reader.read_string()?,
            username: reader.read_string()?,
            features: read_protocol_string_list(&mut reader, "features")?,
            endpoints: read_protocol_string_list(&mut reader, "endpoints")?,
            issued_at_unix: reader.read_u64_le()?,
            expires_at_unix: reader.read_u64_le()?,
            public_key: read_array::<32>(&mut reader, "public key")?,
            signature: Some(read_array::<64>(&mut reader, "signature")?),
        };
        reader.finish()?;
        Ok(Self {
            version,
            message_type,
            nonce,
            descriptor,
        })
    }
}

pub fn peer_capability_message(
    envelope: &PeerCapabilityEnvelope,
) -> Result<PeerMessage, CapabilityError> {
    Ok(PeerMessage::Unknown {
        code: PEER_CAPABILITY_MESSAGE_CODE,
        payload: envelope.encode()?,
    })
}

pub fn decode_peer_capability_message(
    message: &PeerMessage,
) -> Result<Option<PeerCapabilityEnvelope>, CapabilityError> {
    let PeerMessage::Unknown { code, payload } = message else {
        return Ok(None);
    };
    if *code != PEER_CAPABILITY_MESSAGE_CODE {
        return Ok(None);
    }
    PeerCapabilityEnvelope::decode(payload).map(Some)
}

pub fn handle_peer_capability_message(
    registry: &mut PeerCapabilityRegistry,
    message: &PeerMessage,
    local_descriptor: &PeerCapabilityDescriptor,
    now: SystemTime,
) -> Result<Option<PeerMessage>, CapabilityError> {
    let Some(envelope) = decode_peer_capability_message(message)? else {
        return Ok(None);
    };

    registry.update(envelope.descriptor, now)?;
    if envelope.message_type == PeerCapabilityMessageType::Hello {
        let acknowledgement = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Acknowledge,
            envelope.nonce,
            local_descriptor.clone(),
        );
        return peer_capability_message(&acknowledgement).map(Some);
    }

    Ok(None)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerCapabilityDescriptor {
    pub peer_id: String,
    pub username: String,
    pub features: Vec<String>,
    pub endpoints: Vec<String>,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub public_key: [u8; 32],
    pub signature: Option<[u8; 64]>,
}

impl PeerCapabilityDescriptor {
    pub fn unsigned(
        username: impl Into<String>,
        features: Vec<String>,
        endpoints: Vec<String>,
        validity: Duration,
        signing_key: &SigningKey,
        now: SystemTime,
    ) -> Result<Self, CapabilityError> {
        let username = non_blank(username.into(), "username")?;
        let public_key = signing_key.verifying_key().to_bytes();
        let issued_at_unix = unix_seconds(now)?;
        let expires_at_unix = issued_at_unix
            .checked_add(validity.as_secs())
            .ok_or(CapabilityError::InvalidTime)?;
        if expires_at_unix <= issued_at_unix {
            return Err(CapabilityError::InvalidValidity);
        }

        Ok(Self {
            peer_id: peer_id_for_public_key(&public_key),
            username,
            features: normalize_values(features, "feature")?,
            endpoints: normalize_values(endpoints, "endpoint")?,
            issued_at_unix,
            expires_at_unix,
            public_key,
            signature: None,
        })
    }

    pub fn sign(mut self, signing_key: &SigningKey) -> Result<Self, CapabilityError> {
        if signing_key.verifying_key().to_bytes() != self.public_key {
            return Err(CapabilityError::SigningKeyMismatch);
        }
        let signature = signing_key.sign(&self.canonical_payload()?);
        self.signature = Some(signature.to_bytes());
        Ok(self)
    }

    pub fn verify(&self, now: SystemTime) -> Result<(), CapabilityError> {
        if unix_seconds(now)? >= self.expires_at_unix {
            return Err(CapabilityError::Expired);
        }
        if self.peer_id != peer_id_for_public_key(&self.public_key) {
            return Err(CapabilityError::PeerIdMismatch);
        }
        let signature = self.signature.ok_or(CapabilityError::MissingSignature)?;
        let verifying_key = VerifyingKey::from_bytes(&self.public_key)
            .map_err(|_| CapabilityError::InvalidPublicKey)?;
        let signature = Signature::from_bytes(&signature);
        verifying_key
            .verify(&self.canonical_payload()?, &signature)
            .map_err(|_| CapabilityError::InvalidSignature)
    }

    pub fn canonical_payload(&self) -> Result<Vec<u8>, CapabilityError> {
        let mut payload = Vec::new();
        write_field(&mut payload, "peerId", &self.peer_id)?;
        write_field(&mut payload, "username", &self.username)?;
        write_list(&mut payload, "features", &self.features)?;
        write_list(&mut payload, "endpoints", &self.endpoints)?;
        write_field(&mut payload, "issuedAt", &self.issued_at_unix.to_string())?;
        write_field(&mut payload, "expiresAt", &self.expires_at_unix.to_string())?;
        write_field(&mut payload, "publicKey", &hex_lower(&self.public_key))?;
        Ok(payload)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeerCapabilityRegistry {
    records: HashMap<String, PeerCapabilityDescriptor>,
}

impl PeerCapabilityRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        descriptor: PeerCapabilityDescriptor,
        now: SystemTime,
    ) -> Result<Option<PeerCapabilityDescriptor>, CapabilityError> {
        descriptor.verify(now)?;
        let key = descriptor.username.to_ascii_lowercase();
        Ok(self.records.insert(key, descriptor))
    }

    #[must_use]
    pub fn get(&self, username: &str) -> Option<&PeerCapabilityDescriptor> {
        self.records.get(&username.to_ascii_lowercase())
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn prune_expired(&mut self, now: SystemTime) -> Result<usize, CapabilityError> {
        let now = unix_seconds(now)?;
        let before = self.records.len();
        self.records
            .retain(|_, descriptor| descriptor.expires_at_unix > now);
        Ok(before - self.records.len())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("{0} must not be blank")]
    BlankField(&'static str),
    #[error("descriptor validity must be positive")]
    InvalidValidity,
    #[error("descriptor time is invalid")]
    InvalidTime,
    #[error("descriptor is expired")]
    Expired,
    #[error("descriptor is missing a signature")]
    MissingSignature,
    #[error("descriptor public key is invalid")]
    InvalidPublicKey,
    #[error("descriptor signature is invalid")]
    InvalidSignature,
    #[error("descriptor peer id does not match its public key")]
    PeerIdMismatch,
    #[error("signing key does not match descriptor public key")]
    SigningKeyMismatch,
    #[error("unsupported capability envelope version {0}")]
    UnsupportedVersion(u16),
    #[error("invalid capability envelope message type {0}")]
    InvalidMessageType(u8),
    #[error("capability envelope length {length} exceeds maximum {max}")]
    EnvelopeTooLarge { length: usize, max: usize },
    #[error("capability envelope decode error: {0}")]
    Decode(String),
    #[error("{field} length {length} exceeds maximum {max}")]
    FieldTooLong {
        field: &'static str,
        length: usize,
        max: usize,
    },
}

impl From<DecodeError> for CapabilityError {
    fn from(error: DecodeError) -> Self {
        Self::Decode(error.to_string())
    }
}

fn write_protocol_string(
    writer: &mut Writer,
    field: &'static str,
    value: &str,
) -> Result<(), CapabilityError> {
    writer
        .write_string(value)
        .map_err(|_| CapabilityError::FieldTooLong {
            field,
            length: value.len(),
            max: u32::MAX as usize,
        })
}

fn write_protocol_string_list(
    writer: &mut Writer,
    field: &'static str,
    values: &[String],
) -> Result<(), CapabilityError> {
    let len = u32::try_from(values.len()).map_err(|_| CapabilityError::FieldTooLong {
        field,
        length: values.len(),
        max: u32::MAX as usize,
    })?;
    writer.write_u32_le(len);
    for value in values {
        write_protocol_string(writer, field, value)?;
    }
    Ok(())
}

fn read_protocol_string_list(
    reader: &mut Reader<'_>,
    field: &'static str,
) -> Result<Vec<String>, CapabilityError> {
    let count = reader.read_bounded_count(field, 4)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(reader.read_string()?);
    }
    Ok(values)
}

fn read_array<const N: usize>(
    reader: &mut Reader<'_>,
    context: &'static str,
) -> Result<[u8; N], CapabilityError> {
    let bytes = reader.read_bytes(N)?;
    bytes
        .try_into()
        .map_err(|_| CapabilityError::Decode(format!("invalid {context} length")))
}

fn unix_seconds(time: SystemTime) -> Result<u64, CapabilityError> {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| CapabilityError::InvalidTime)
}

fn non_blank(value: String, field: &'static str) -> Result<String, CapabilityError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(CapabilityError::BlankField(field))
    } else {
        Ok(value)
    }
}

fn normalize_values(
    values: Vec<String>,
    field: &'static str,
) -> Result<Vec<String>, CapabilityError> {
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(non_blank(value, field)?);
    }
    output.sort();
    output.dedup();
    Ok(output)
}

fn write_list(
    payload: &mut Vec<u8>,
    field: &'static str,
    values: &[String],
) -> Result<(), CapabilityError> {
    write_field(payload, field, &values.join(","))?;
    Ok(())
}

fn write_field(
    payload: &mut Vec<u8>,
    field: &'static str,
    value: &str,
) -> Result<(), CapabilityError> {
    const MAX_FIELD_LEN: usize = 16 * 1024;
    if value.len() > MAX_FIELD_LEN {
        return Err(CapabilityError::FieldTooLong {
            field,
            length: value.len(),
            max: MAX_FIELD_LEN,
        });
    }
    payload.extend_from_slice(field.as_bytes());
    payload.push(b'=');
    payload.extend_from_slice(value.as_bytes());
    payload.push(b'\n');
    Ok(())
}

fn peer_id_for_public_key(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    format!("slskdn-{}", hex_lower(&digest[..16]))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn local_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[8; 32])
    }

    fn now() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    fn descriptor() -> PeerCapabilityDescriptor {
        PeerCapabilityDescriptor::unsigned(
            "Alice",
            vec![
                FEATURE_MESH_V1.to_owned(),
                FEATURE_CAPABILITIES_V1.to_owned(),
                FEATURE_MESH_V1.to_owned(),
            ],
            vec!["udp:1.2.3.4:2234".to_owned()],
            Duration::from_secs(60),
            &signing_key(),
            now(),
        )
        .unwrap()
        .sign(&signing_key())
        .unwrap()
    }

    fn local_descriptor() -> PeerCapabilityDescriptor {
        PeerCapabilityDescriptor::unsigned(
            "Local",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            vec!["tcp:127.0.0.1:2234".to_owned()],
            Duration::from_secs(60),
            &local_signing_key(),
            now(),
        )
        .unwrap()
        .sign(&local_signing_key())
        .unwrap()
    }

    #[test]
    fn signed_descriptor_verifies_and_normalizes_values() {
        let descriptor = descriptor();

        descriptor.verify(now()).unwrap();
        assert_eq!(
            descriptor.features,
            vec![FEATURE_CAPABILITIES_V1, FEATURE_MESH_V1]
        );
        assert!(descriptor.peer_id.starts_with("slskdn-"));
    }

    #[test]
    fn envelope_round_trips_and_rejects_trailing_bytes() {
        let envelope =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, [9; 16], descriptor());
        let mut payload = envelope.encode().unwrap();
        let decoded = PeerCapabilityEnvelope::decode(&payload).unwrap();

        assert_eq!(decoded, envelope);
        decoded.descriptor.verify(now()).unwrap();

        payload.push(1);
        assert!(matches!(
            PeerCapabilityEnvelope::decode(&payload).unwrap_err(),
            CapabilityError::Decode(_)
        ));
    }

    #[test]
    fn envelope_rejects_unknown_message_type() {
        let envelope =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, [9; 16], descriptor());
        let mut payload = envelope.encode().unwrap();
        payload[2] = 99;

        assert_eq!(
            PeerCapabilityEnvelope::decode(&payload).unwrap_err(),
            CapabilityError::InvalidMessageType(99)
        );
    }

    #[test]
    fn peer_message_exchange_updates_registry_and_acknowledges_hello() {
        let hello =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, [3; 16], descriptor());
        let message = peer_capability_message(&hello).unwrap();
        let decoded = decode_peer_capability_message(&message).unwrap().unwrap();
        assert_eq!(decoded, hello);

        let mut registry = PeerCapabilityRegistry::new();
        let response =
            handle_peer_capability_message(&mut registry, &message, &local_descriptor(), now())
                .unwrap()
                .expect("acknowledgement");

        assert!(registry.get("alice").is_some());
        let acknowledgement = decode_peer_capability_message(&response)
            .unwrap()
            .expect("capability ack");
        assert_eq!(
            acknowledgement.message_type,
            PeerCapabilityMessageType::Acknowledge
        );
        assert_eq!(acknowledgement.nonce, [3; 16]);
        assert_eq!(acknowledgement.descriptor.username, "Local");
        acknowledgement.descriptor.verify(now()).unwrap();
    }

    #[test]
    fn peer_message_exchange_ignores_unrelated_unknown_messages_and_rejects_bad_payloads() {
        let mut registry = PeerCapabilityRegistry::new();
        let unrelated = PeerMessage::Unknown {
            code: 123,
            payload: vec![1, 2, 3],
        };
        assert!(decode_peer_capability_message(&unrelated)
            .unwrap()
            .is_none());
        assert!(handle_peer_capability_message(
            &mut registry,
            &unrelated,
            &local_descriptor(),
            now()
        )
        .unwrap()
        .is_none());
        assert!(registry.is_empty());

        let bad = PeerMessage::Unknown {
            code: PEER_CAPABILITY_MESSAGE_CODE,
            payload: vec![1, 2, 3],
        };
        assert!(matches!(
            decode_peer_capability_message(&bad).unwrap_err(),
            CapabilityError::UnsupportedVersion(_) | CapabilityError::Decode(_)
        ));
    }

    #[test]
    fn forged_descriptor_fails_closed() {
        let mut descriptor = descriptor();
        descriptor.features.push(FEATURE_WISHLIST_V1.to_owned());

        assert_eq!(
            descriptor.verify(now()).unwrap_err(),
            CapabilityError::InvalidSignature
        );
    }

    #[test]
    fn expired_descriptor_fails_closed() {
        let descriptor = descriptor();
        let expired_at = now() + Duration::from_secs(61);

        assert_eq!(
            descriptor.verify(expired_at).unwrap_err(),
            CapabilityError::Expired
        );
    }

    #[test]
    fn registry_is_case_insensitive_and_prunes_expired_records() {
        let mut registry = PeerCapabilityRegistry::new();
        registry.update(descriptor(), now()).unwrap();

        assert!(registry.get("alice").is_some());
        assert!(registry.get("ALICE").is_some());
        assert_eq!(
            registry
                .prune_expired(now() + Duration::from_secs(61))
                .unwrap(),
            1
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn blank_values_are_rejected_before_signing() {
        let error = PeerCapabilityDescriptor::unsigned(
            " ",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &signing_key(),
            now(),
        )
        .unwrap_err();

        assert_eq!(error, CapabilityError::BlankField("username"));
    }
}
