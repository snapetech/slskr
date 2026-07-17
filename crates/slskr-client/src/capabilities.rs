use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use slskr_protocol::{peer::PeerMessage, DecodeError, Reader};

pub const PEER_CAPABILITY_MESSAGE_CODE: u32 = 0x534C_534B;
pub const PEER_CAPABILITY_ENVELOPE_VERSION: u16 = 1;
pub const MAX_CAPABILITY_ENVELOPE_BYTES: usize = 64 * 1024;
pub const MAX_PEER_CAPABILITY_RECORDS: usize = 1_024;
pub const MAX_CAPABILITY_ENDPOINTS: usize = 256;
pub const PEER_CAPABILITY_RECEIPT_LEASE: Duration = Duration::from_secs(24 * 60 * 60);
const PEER_CAPABILITY_MAGIC: i32 = 0x4E44_534B;
const MAX_CAPABILITY_FEATURES: usize = 256;
const MAX_CAPABILITY_STRING_BYTES: usize = 4_096;
const MAX_CAPABILITY_SIGNATURE_BYTES: usize = 4_096;
pub const FEATURE_CAPABILITIES_V1: &str = "slskdn-capabilities-v1";
pub const FEATURE_MESH_V1: &str = "slskdn-mesh-v1";
pub const FEATURE_SHARED_UDP_V1: &str = "slskdn-shared-udp-v1";
pub const FEATURE_WISHLIST_V1: &str = "slskdn-wishlist-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum PeerCapabilityMessageType {
    Hello = 1,
    Acknowledge = 2,
}

impl TryFrom<i32> for PeerCapabilityMessageType {
    type Error = CapabilityError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
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
    pub nonce: String,
    pub descriptor: PeerCapabilityDescriptor,
}

impl PeerCapabilityEnvelope {
    #[must_use]
    pub fn new(
        message_type: PeerCapabilityMessageType,
        nonce: impl Into<String>,
        descriptor: PeerCapabilityDescriptor,
    ) -> Self {
        Self {
            version: PEER_CAPABILITY_ENVELOPE_VERSION,
            message_type,
            nonce: nonce.into(),
            descriptor,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, CapabilityError> {
        if self.version != PEER_CAPABILITY_ENVELOPE_VERSION {
            return Err(CapabilityError::UnsupportedVersion(i32::from(self.version)));
        }
        if self.nonce.trim().is_empty() {
            return Err(CapabilityError::BlankField("nonce"));
        }
        validate_overlay_port(self.descriptor.overlay_port)?;
        if self.descriptor.features.len() > MAX_CAPABILITY_FEATURES {
            return Err(CapabilityError::FieldTooLong {
                field: "features",
                length: self.descriptor.features.len(),
                max: MAX_CAPABILITY_FEATURES,
            });
        }
        let mut payload = Vec::new();
        write_i32(&mut payload, PEER_CAPABILITY_MAGIC);
        write_i32(&mut payload, i32::from(self.version));
        write_i32(&mut payload, self.message_type as i32);
        write_bounded_string(&mut payload, "nonce", &self.nonce)?;
        write_bounded_string(&mut payload, "peer_id", &self.descriptor.peer_id)?;
        write_i32(
            &mut payload,
            self.descriptor.overlay_port.map_or(-1, i32::from),
        );
        write_i32(
            &mut payload,
            i32::try_from(self.descriptor.max_payload_length).map_err(|_| {
                CapabilityError::FieldTooLong {
                    field: "max_payload_length",
                    length: self.descriptor.max_payload_length as usize,
                    max: i32::MAX as usize,
                }
            })?,
        );
        write_i32(
            &mut payload,
            i32::try_from(self.descriptor.features.len()).map_err(|_| {
                CapabilityError::FieldTooLong {
                    field: "features",
                    length: self.descriptor.features.len(),
                    max: MAX_CAPABILITY_FEATURES,
                }
            })?,
        );
        for feature in &self.descriptor.features {
            write_bounded_string(&mut payload, "feature", feature)?;
        }
        if let Some(signature) = self.descriptor.signature {
            payload.push(1);
            write_bounded_string(&mut payload, "signature_algorithm", "Ed25519")?;
            write_bounded_bytes(&mut payload, "public_key", &self.descriptor.public_key)?;
            write_bounded_bytes(&mut payload, "signature", &signature)?;
        } else {
            payload.push(0);
        }
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
        let magic = read_i32(&mut reader)?;
        if magic != PEER_CAPABILITY_MAGIC {
            return Err(CapabilityError::MagicMismatch(magic));
        }
        let version_i32 = read_i32(&mut reader)?;
        let version = u16::try_from(version_i32)
            .map_err(|_| CapabilityError::UnsupportedVersion(version_i32))?;
        if version != PEER_CAPABILITY_ENVELOPE_VERSION {
            return Err(CapabilityError::UnsupportedVersion(version_i32));
        }
        let message_type = PeerCapabilityMessageType::try_from(read_i32(&mut reader)?)?;
        let nonce = read_bounded_string(&mut reader, "nonce")?;
        if nonce.trim().is_empty() {
            return Err(CapabilityError::BlankField("nonce"));
        }
        let peer_id = read_bounded_string(&mut reader, "peer_id")?;
        let overlay_port = match read_i32(&mut reader)? {
            -1 => None,
            value @ 1..=65_535 => Some(value as u16),
            value => return Err(CapabilityError::InvalidOverlayPort(value)),
        };
        let max_payload_length = read_i32(&mut reader)?;
        if max_payload_length < 0 {
            return Err(CapabilityError::InvalidMaxPayloadLength(max_payload_length));
        }
        let feature_count = read_bounded_count(&mut reader, "features", MAX_CAPABILITY_FEATURES)?;
        let mut features = Vec::with_capacity(feature_count);
        for _ in 0..feature_count {
            features.push(read_bounded_string(&mut reader, "feature")?);
        }
        let has_signature = reader.read_u8()?;
        let (public_key, signature) = match has_signature {
            0 => ([0; 32], None),
            1 => {
                let algorithm = read_bounded_string(&mut reader, "signature_algorithm")?;
                if !algorithm.eq_ignore_ascii_case("Ed25519") {
                    return Err(CapabilityError::InvalidSignatureAlgorithm(algorithm));
                }
                let public_key = read_bounded_array::<32>(&mut reader, "public_key")?;
                let signature = read_bounded_array::<64>(&mut reader, "signature")?;
                (public_key, Some(signature))
            }
            value => return Err(CapabilityError::InvalidSignaturePresence(value)),
        };
        let descriptor = PeerCapabilityDescriptor {
            peer_id,
            username: String::new(),
            features: normalize_values(features, "feature")?,
            endpoints: Vec::new(),
            overlay_port,
            max_payload_length: max_payload_length as u32,
            issued_at_unix: 0,
            expires_at_unix: u64::MAX,
            public_key,
            signature,
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
    remote_username: &str,
    local_descriptor: &PeerCapabilityDescriptor,
    now: SystemTime,
) -> Result<Option<PeerMessage>, CapabilityError> {
    let Some(mut envelope) = decode_peer_capability_message(message)? else {
        return Ok(None);
    };

    envelope.descriptor.username = bounded_non_blank(remote_username.to_owned(), "username")?;
    let received_at = unix_seconds(now)?;
    envelope.descriptor.issued_at_unix = received_at;
    envelope.descriptor.expires_at_unix = received_at
        .checked_add(PEER_CAPABILITY_RECEIPT_LEASE.as_secs())
        .ok_or(CapabilityError::InvalidTime)?;
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
    pub overlay_port: Option<u16>,
    pub max_payload_length: u32,
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
        let username = bounded_non_blank(username.into(), "username")?;
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
            overlay_port: None,
            max_payload_length: MAX_CAPABILITY_ENVELOPE_BYTES as u32,
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

    #[must_use]
    pub const fn with_overlay_port(mut self, overlay_port: Option<u16>) -> Self {
        self.overlay_port = overlay_port;
        self
    }

    pub fn verify(&self, now: SystemTime) -> Result<(), CapabilityError> {
        let now = unix_seconds(now)?;
        if self.expires_at_unix <= self.issued_at_unix {
            return Err(CapabilityError::InvalidValidity);
        }
        if now < self.issued_at_unix {
            return Err(CapabilityError::NotYetValid);
        }
        if now >= self.expires_at_unix {
            return Err(CapabilityError::Expired);
        }
        if !self
            .peer_id
            .eq_ignore_ascii_case(&peer_id_for_public_key(&self.public_key))
        {
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
        validate_overlay_port(self.overlay_port)?;
        if self.features.len() > MAX_CAPABILITY_FEATURES {
            return Err(CapabilityError::FieldTooLong {
                field: "features",
                length: self.features.len(),
                max: MAX_CAPABILITY_FEATURES,
            });
        }
        let mut payload = Vec::new();
        write_bounded_string(&mut payload, "peer_id", &self.peer_id)?;
        write_i32(&mut payload, self.overlay_port.map_or(-1, i32::from));
        write_i32(
            &mut payload,
            i32::try_from(self.max_payload_length)
                .map_err(|_| CapabilityError::InvalidMaxPayloadLength(i32::MAX))?,
        );
        write_i32(
            &mut payload,
            i32::try_from(self.features.len()).map_err(|_| CapabilityError::FieldTooLong {
                field: "features",
                length: self.features.len(),
                max: MAX_CAPABILITY_FEATURES,
            })?,
        );
        for feature in &self.features {
            write_bounded_string(&mut payload, "feature", feature)?;
        }
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
        mut descriptor: PeerCapabilityDescriptor,
        now: SystemTime,
    ) -> Result<Option<PeerCapabilityDescriptor>, CapabilityError> {
        descriptor.username = bounded_non_blank(descriptor.username, "username")?;
        descriptor.verify(now)?;
        let key = capability_username_key(&descriptor.username);
        self.prune_expired(now)?;
        if self
            .records
            .get(&key)
            .is_some_and(|existing| !existing.peer_id.eq_ignore_ascii_case(&descriptor.peer_id))
        {
            return Err(CapabilityError::PeerIdentityChanged);
        }
        if self.records.len() >= MAX_PEER_CAPABILITY_RECORDS && !self.records.contains_key(&key) {
            return Err(CapabilityError::RegistryFull {
                max: MAX_PEER_CAPABILITY_RECORDS,
            });
        }
        Ok(self.records.insert(key, descriptor))
    }

    #[must_use]
    pub fn get(&self, username: &str) -> Option<&PeerCapabilityDescriptor> {
        self.records.get(&capability_username_key(username))
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

fn capability_username_key(username: &str) -> String {
    username.trim().to_ascii_lowercase()
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
    #[error("descriptor is not yet valid")]
    NotYetValid,
    #[error("descriptor is missing a signature")]
    MissingSignature,
    #[error("descriptor public key is invalid")]
    InvalidPublicKey,
    #[error("descriptor signature is invalid")]
    InvalidSignature,
    #[error("descriptor peer id does not match its public key")]
    PeerIdMismatch,
    #[error("peer capability identity changed before the existing record expired")]
    PeerIdentityChanged,
    #[error("signing key does not match descriptor public key")]
    SigningKeyMismatch,
    #[error("capability envelope magic mismatch: {0:#x}")]
    MagicMismatch(i32),
    #[error("unsupported capability envelope version {0}")]
    UnsupportedVersion(i32),
    #[error("invalid capability envelope message type {0}")]
    InvalidMessageType(i32),
    #[error("invalid overlay port {0}")]
    InvalidOverlayPort(i32),
    #[error("invalid maximum payload length {0}")]
    InvalidMaxPayloadLength(i32),
    #[error("unsupported descriptor signature algorithm {0}")]
    InvalidSignatureAlgorithm(String),
    #[error("invalid descriptor signature-presence byte {0}")]
    InvalidSignaturePresence(u8),
    #[error("capability envelope length {length} exceeds maximum {max}")]
    EnvelopeTooLarge { length: usize, max: usize },
    #[error("capability envelope decode error: {0}")]
    Decode(String),
    #[error("peer capability registry is full (maximum {max} records)")]
    RegistryFull { max: usize },
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

fn write_bounded_string(
    payload: &mut Vec<u8>,
    field: &'static str,
    value: &str,
) -> Result<(), CapabilityError> {
    write_bounded_bytes(payload, field, value.as_bytes())
}

fn validate_overlay_port(overlay_port: Option<u16>) -> Result<(), CapabilityError> {
    if overlay_port == Some(0) {
        Err(CapabilityError::InvalidOverlayPort(0))
    } else {
        Ok(())
    }
}

fn write_bounded_bytes(
    payload: &mut Vec<u8>,
    field: &'static str,
    value: &[u8],
) -> Result<(), CapabilityError> {
    let max = if field == "signature" || field == "public_key" {
        MAX_CAPABILITY_SIGNATURE_BYTES
    } else {
        MAX_CAPABILITY_STRING_BYTES
    };
    if value.len() > max {
        return Err(CapabilityError::FieldTooLong {
            field,
            length: value.len(),
            max,
        });
    }
    write_i32(
        payload,
        i32::try_from(value.len()).map_err(|_| CapabilityError::FieldTooLong {
            field,
            length: value.len(),
            max,
        })?,
    );
    payload.extend_from_slice(value);
    Ok(())
}

fn write_i32(payload: &mut Vec<u8>, value: i32) {
    payload.extend_from_slice(&value.to_le_bytes());
}

fn read_i32(reader: &mut Reader<'_>) -> Result<i32, CapabilityError> {
    Ok(i32::from_le_bytes(read_array::<4>(reader, "i32")?))
}

fn read_bounded_count(
    reader: &mut Reader<'_>,
    field: &'static str,
    maximum: usize,
) -> Result<usize, CapabilityError> {
    let count = read_i32(reader)?;
    if count < 0 || usize::try_from(count).map_or(true, |count| count > maximum) {
        return Err(CapabilityError::Decode(format!(
            "invalid {field} count: {count}"
        )));
    }
    Ok(count as usize)
}

fn read_bounded_string(
    reader: &mut Reader<'_>,
    field: &'static str,
) -> Result<String, CapabilityError> {
    let length = read_bounded_count(reader, field, MAX_CAPABILITY_STRING_BYTES)?;
    String::from_utf8(reader.read_bytes(length)?.to_vec())
        .map_err(|error| CapabilityError::Decode(format!("invalid {field} UTF-8: {error}")))
}

fn read_bounded_array<const N: usize>(
    reader: &mut Reader<'_>,
    field: &'static str,
) -> Result<[u8; N], CapabilityError> {
    let length = read_bounded_count(reader, field, MAX_CAPABILITY_SIGNATURE_BYTES)?;
    if length != N {
        return Err(CapabilityError::Decode(format!(
            "invalid {field} length: expected {N}, received {length}"
        )));
    }
    read_array::<N>(reader, field)
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

fn bounded_non_blank(value: String, field: &'static str) -> Result<String, CapabilityError> {
    if value.len() > MAX_CAPABILITY_STRING_BYTES {
        return Err(CapabilityError::FieldTooLong {
            field,
            length: value.len(),
            max: MAX_CAPABILITY_STRING_BYTES,
        });
    }
    non_blank(value, field)
}

fn normalize_values(
    values: Vec<String>,
    field: &'static str,
) -> Result<Vec<String>, CapabilityError> {
    let (collection, maximum) = match field {
        "feature" => ("features", MAX_CAPABILITY_FEATURES),
        "endpoint" => ("endpoints", MAX_CAPABILITY_ENDPOINTS),
        _ => (field, usize::MAX),
    };
    if values.len() > maximum {
        return Err(CapabilityError::FieldTooLong {
            field: collection,
            length: values.len(),
            max: maximum,
        });
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(bounded_non_blank(value, field)?);
    }
    output.sort_by_key(|value| value.to_ascii_lowercase());
    output.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    Ok(output)
}

fn peer_id_for_public_key(public_key: &[u8; 32]) -> String {
    let digest = Sha256::digest(public_key);
    base32_lower(&digest[..20])
}

fn base32_lower(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut output = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut bits = 0_u16;
    let mut bit_count = 0_u8;
    for byte in bytes {
        bits = (bits << 8) | u16::from(*byte);
        bit_count += 8;
        while bit_count >= 5 {
            bit_count -= 5;
            output.push(ALPHABET[((bits >> bit_count) & 31) as usize] as char);
        }
    }
    if bit_count > 0 {
        output.push(ALPHABET[((bits << (5 - bit_count)) & 31) as usize] as char);
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
        assert_eq!(descriptor.peer_id.len(), 32);
        assert!(descriptor
            .peer_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || (b'2'..=b'7').contains(&byte)));
    }

    #[test]
    fn envelope_round_trips_and_rejects_trailing_bytes() {
        let envelope = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Hello,
            "09090909090909090909090909090909",
            descriptor(),
        );
        let mut payload = envelope.encode().unwrap();
        let decoded = PeerCapabilityEnvelope::decode(&payload).unwrap();

        assert_eq!(decoded.version, envelope.version);
        assert_eq!(decoded.message_type, envelope.message_type);
        assert_eq!(decoded.nonce, envelope.nonce);
        assert_eq!(decoded.descriptor.peer_id, envelope.descriptor.peer_id);
        assert_eq!(decoded.descriptor.features, envelope.descriptor.features);
        assert_eq!(
            decoded.descriptor.overlay_port,
            envelope.descriptor.overlay_port
        );
        assert_eq!(
            decoded.descriptor.max_payload_length,
            envelope.descriptor.max_payload_length
        );
        assert_eq!(
            decoded.descriptor.public_key,
            envelope.descriptor.public_key
        );
        assert_eq!(decoded.descriptor.signature, envelope.descriptor.signature);
        decoded.descriptor.verify(now()).unwrap();

        payload.push(1);
        assert!(matches!(
            PeerCapabilityEnvelope::decode(&payload).unwrap_err(),
            CapabilityError::Decode(_)
        ));
    }

    #[test]
    fn envelope_matches_frozen_runtime_unsigned_wire_fixture() {
        let descriptor = PeerCapabilityDescriptor {
            peer_id: "p".to_owned(),
            username: "observation-only".to_owned(),
            features: vec!["mesh".to_owned()],
            endpoints: vec!["not-on-wire".to_owned()],
            overlay_port: Some(50_305),
            max_payload_length: 65_536,
            issued_at_unix: 123,
            expires_at_unix: 456,
            public_key: [0; 32],
            signature: None,
        };
        let envelope =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, "n", descriptor);
        let expected = vec![
            0x4b, 0x53, 0x44, 0x4e, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, b'n', 0x01, 0x00, 0x00, 0x00, b'p', 0x81, 0xc4, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, b'm', b'e', b's', b'h',
            0x00,
        ];

        assert_eq!(envelope.encode().unwrap(), expected);
        let decoded = PeerCapabilityEnvelope::decode(&expected).unwrap();
        assert_eq!(decoded.nonce, "n");
        assert_eq!(decoded.descriptor.peer_id, "p");
        assert_eq!(decoded.descriptor.overlay_port, Some(50_305));
        assert_eq!(decoded.descriptor.max_payload_length, 65_536);
        assert_eq!(decoded.descriptor.features, vec!["mesh"]);
        assert!(decoded.descriptor.signature.is_none());
    }

    #[test]
    fn envelope_rejects_unknown_message_type() {
        let envelope = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Hello,
            "09090909090909090909090909090909",
            descriptor(),
        );
        let mut payload = envelope.encode().unwrap();
        payload[8] = 99;

        assert_eq!(
            PeerCapabilityEnvelope::decode(&payload).unwrap_err(),
            CapabilityError::InvalidMessageType(99)
        );
    }

    #[test]
    fn envelope_decoder_rejects_blank_nonce() {
        let envelope =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, "n", descriptor());
        let mut payload = envelope.encode().unwrap();
        payload[16] = b' ';

        assert_eq!(
            PeerCapabilityEnvelope::decode(&payload).unwrap_err(),
            CapabilityError::BlankField("nonce")
        );
    }

    #[test]
    fn envelope_encoder_rejects_unsupported_version() {
        let mut envelope =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, "nonce", descriptor());
        envelope.version = PEER_CAPABILITY_ENVELOPE_VERSION + 1;

        assert_eq!(
            envelope.encode().unwrap_err(),
            CapabilityError::UnsupportedVersion(i32::from(PEER_CAPABILITY_ENVELOPE_VERSION + 1))
        );
    }

    #[test]
    fn zero_overlay_port_is_rejected_on_encode_sign_and_decode() {
        let invalid = descriptor().with_overlay_port(Some(0));
        assert_eq!(
            invalid.clone().sign(&signing_key()).unwrap_err(),
            CapabilityError::InvalidOverlayPort(0)
        );
        let envelope =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, "nonce", invalid);
        assert_eq!(
            envelope.encode().unwrap_err(),
            CapabilityError::InvalidOverlayPort(0)
        );

        let mut payload = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Hello,
            "nonce",
            descriptor().with_overlay_port(Some(50_305)),
        )
        .encode()
        .unwrap();
        let peer_id_offset = 12 + 4 + "nonce".len();
        let overlay_port_offset = peer_id_offset + 4 + descriptor().peer_id.len();
        payload[overlay_port_offset..overlay_port_offset + 4].copy_from_slice(&0_i32.to_le_bytes());
        assert_eq!(
            PeerCapabilityEnvelope::decode(&payload).unwrap_err(),
            CapabilityError::InvalidOverlayPort(0)
        );
    }

    #[test]
    fn peer_message_exchange_updates_registry_and_acknowledges_hello() {
        let hello = PeerCapabilityEnvelope::new(
            PeerCapabilityMessageType::Hello,
            "03030303030303030303030303030303",
            descriptor(),
        );
        let message = peer_capability_message(&hello).unwrap();
        let decoded = decode_peer_capability_message(&message).unwrap().unwrap();
        assert_eq!(decoded.nonce, hello.nonce);
        assert!(decoded.descriptor.username.is_empty());

        let mut registry = PeerCapabilityRegistry::new();
        let response = handle_peer_capability_message(
            &mut registry,
            &message,
            "Alice",
            &local_descriptor(),
            now(),
        )
        .unwrap()
        .expect("acknowledgement");

        assert!(registry.get("alice").is_some());
        let registered = registry.get("alice").unwrap();
        assert_eq!(registered.issued_at_unix, unix_seconds(now()).unwrap());
        assert_eq!(
            registered.expires_at_unix,
            unix_seconds(now()).unwrap() + PEER_CAPABILITY_RECEIPT_LEASE.as_secs()
        );
        let acknowledgement = decode_peer_capability_message(&response)
            .unwrap()
            .expect("capability ack");
        assert_eq!(
            acknowledgement.message_type,
            PeerCapabilityMessageType::Acknowledge
        );
        assert_eq!(acknowledgement.nonce, "03030303030303030303030303030303");
        assert!(acknowledgement.descriptor.username.is_empty());
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
            "Alice",
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
    fn future_and_inverted_descriptor_validity_fail_closed() {
        let future = now() + Duration::from_secs(60);
        let descriptor = PeerCapabilityDescriptor::unsigned(
            "Alice",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &signing_key(),
            future,
        )
        .unwrap()
        .sign(&signing_key())
        .unwrap();
        assert_eq!(
            descriptor.verify(now()).unwrap_err(),
            CapabilityError::NotYetValid
        );

        let mut inverted = descriptor;
        inverted.expires_at_unix = inverted.issued_at_unix;
        assert_eq!(
            inverted.verify(future).unwrap_err(),
            CapabilityError::InvalidValidity
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
    fn registry_prunes_expired_records_and_rejects_new_peers_at_limit() {
        let mut registry = PeerCapabilityRegistry::new();
        let alice = descriptor();
        registry.records.insert("alice".to_owned(), alice.clone());
        for index in 1..MAX_PEER_CAPABILITY_RECORDS {
            registry
                .records
                .insert(format!("peer-{index}"), alice.clone());
        }

        assert!(registry.update(descriptor(), now()).is_ok());
        let bob = PeerCapabilityDescriptor::unsigned(
            "Bob",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &signing_key(),
            now(),
        )
        .unwrap()
        .sign(&signing_key())
        .unwrap();
        assert_eq!(
            registry.update(bob.clone(), now()).unwrap_err(),
            CapabilityError::RegistryFull {
                max: MAX_PEER_CAPABILITY_RECORDS
            }
        );

        let later = now() + Duration::from_secs(61);
        let refreshed_bob = PeerCapabilityDescriptor::unsigned(
            "Bob",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &signing_key(),
            later,
        )
        .unwrap()
        .sign(&signing_key())
        .unwrap();
        assert!(registry.update(refreshed_bob, later).is_ok());
        assert_eq!(registry.len(), 1);
        assert!(registry.get("bob").is_some());
    }

    #[test]
    fn registry_pins_peer_identity_until_record_expiry() {
        let mut registry = PeerCapabilityRegistry::new();
        registry.update(descriptor(), now()).unwrap();

        let replacement_key = SigningKey::from_bytes(&[9; 32]);
        let replacement = PeerCapabilityDescriptor::unsigned(
            "Alice",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &replacement_key,
            now(),
        )
        .unwrap()
        .sign(&replacement_key)
        .unwrap();
        assert_eq!(
            registry.update(replacement.clone(), now()).unwrap_err(),
            CapabilityError::PeerIdentityChanged
        );
        assert_eq!(registry.get("alice").unwrap().peer_id, descriptor().peer_id);

        let later = now() + Duration::from_secs(61);
        let rotated = PeerCapabilityDescriptor::unsigned(
            "Alice",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &replacement_key,
            later,
        )
        .unwrap()
        .sign(&replacement_key)
        .unwrap();
        assert!(registry.update(rotated.clone(), later).is_ok());
        assert_eq!(registry.get("ALICE").unwrap().peer_id, rotated.peer_id);
    }

    #[test]
    fn registry_canonicalizes_username_before_identity_pinning() {
        let mut registry = PeerCapabilityRegistry::new();
        registry.update(descriptor(), now()).unwrap();

        let replacement_key = SigningKey::from_bytes(&[9; 32]);
        let mut replacement = PeerCapabilityDescriptor::unsigned(
            "Alice",
            vec![FEATURE_CAPABILITIES_V1.to_owned()],
            Vec::new(),
            Duration::from_secs(60),
            &replacement_key,
            now(),
        )
        .unwrap()
        .sign(&replacement_key)
        .unwrap();
        replacement.username = " Alice ".to_owned();

        assert_eq!(
            registry.update(replacement, now()).unwrap_err(),
            CapabilityError::PeerIdentityChanged
        );
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get(" ALICE ").unwrap().username, "Alice");
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

        let oversized = "x".repeat(MAX_CAPABILITY_STRING_BYTES + 1);
        assert_eq!(
            PeerCapabilityDescriptor::unsigned(
                oversized,
                vec![FEATURE_CAPABILITIES_V1.to_owned()],
                Vec::new(),
                Duration::from_secs(60),
                &signing_key(),
                now(),
            )
            .unwrap_err(),
            CapabilityError::FieldTooLong {
                field: "username",
                length: MAX_CAPABILITY_STRING_BYTES + 1,
                max: MAX_CAPABILITY_STRING_BYTES,
            }
        );
    }

    #[test]
    fn capability_exchange_rejects_oversized_remote_username() {
        let hello =
            PeerCapabilityEnvelope::new(PeerCapabilityMessageType::Hello, "nonce", descriptor());
        let message = peer_capability_message(&hello).unwrap();
        let oversized = "x".repeat(MAX_CAPABILITY_STRING_BYTES + 1);
        let mut registry = PeerCapabilityRegistry::new();

        assert_eq!(
            handle_peer_capability_message(
                &mut registry,
                &message,
                &oversized,
                &local_descriptor(),
                now(),
            )
            .unwrap_err(),
            CapabilityError::FieldTooLong {
                field: "username",
                length: MAX_CAPABILITY_STRING_BYTES + 1,
                max: MAX_CAPABILITY_STRING_BYTES,
            }
        );
        assert!(registry.is_empty());
    }

    #[test]
    fn capability_registry_rejects_oversized_username_directly() {
        let mut oversized = descriptor();
        oversized.username = "x".repeat(MAX_CAPABILITY_STRING_BYTES + 1);
        let mut registry = PeerCapabilityRegistry::new();

        assert!(matches!(
            registry.update(oversized, now()),
            Err(CapabilityError::FieldTooLong {
                field: "username",
                length,
                max: MAX_CAPABILITY_STRING_BYTES,
            }) if length == MAX_CAPABILITY_STRING_BYTES + 1
        ));
        assert!(registry.is_empty());
    }

    #[test]
    fn capability_builder_rejects_oversized_and_excessive_advertisements() {
        for (features, endpoints, expected_field, expected_length, expected_max) in [
            (
                vec!["x".repeat(MAX_CAPABILITY_STRING_BYTES + 1)],
                Vec::new(),
                "feature",
                MAX_CAPABILITY_STRING_BYTES + 1,
                MAX_CAPABILITY_STRING_BYTES,
            ),
            (
                Vec::new(),
                vec!["x".repeat(MAX_CAPABILITY_STRING_BYTES + 1)],
                "endpoint",
                MAX_CAPABILITY_STRING_BYTES + 1,
                MAX_CAPABILITY_STRING_BYTES,
            ),
            (
                Vec::new(),
                (0..=MAX_CAPABILITY_ENDPOINTS)
                    .map(|index| format!("tcp:127.0.0.1:{index}"))
                    .collect(),
                "endpoints",
                MAX_CAPABILITY_ENDPOINTS + 1,
                MAX_CAPABILITY_ENDPOINTS,
            ),
        ] {
            assert!(matches!(
                PeerCapabilityDescriptor::unsigned(
                    "alice",
                    features,
                    endpoints,
                    Duration::from_secs(60),
                    &signing_key(),
                    now(),
                ),
                Err(CapabilityError::FieldTooLong { field, length, max })
                    if field == expected_field
                        && length == expected_length
                        && max == expected_max
            ));
        }
    }
}
