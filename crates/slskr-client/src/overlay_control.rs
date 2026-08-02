//! The slskdN UDP overlay control envelope.
//!
//! slskdN uses MessagePack's keyed-object array representation for its
//! signed control plane.  This module keeps that small wire contract
//! dependency-free so the client can interoperate without replacing the
//! existing JSON/TLS mesh-service protocol.

use std::{
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use tokio::net::UdpSocket;

pub const CONTROL_MAX_DATAGRAM_BYTES: usize = 8 * 1024;
pub const CONTROL_MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
pub const CONTROL_TIMESTAMP_SKEW: Duration = Duration::from_secs(120);
const MAX_STRING_BYTES: usize = 64 * 1024;
const ENVELOPE_FIELDS: usize = 6;

/// The `[Type, Payload, PublicKey, Signature, TimestampUnixMs, MessageId]`
/// MessagePack array emitted by `slskd.Mesh.Overlay.ControlEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlEnvelope {
    pub message_type: String,
    pub payload: Vec<u8>,
    pub public_key: String,
    pub signature: String,
    pub timestamp_unix_ms: i64,
    pub message_id: String,
}

impl ControlEnvelope {
    pub fn new_signed(
        message_type: impl Into<String>,
        payload: Vec<u8>,
        signing_key: &SigningKey,
    ) -> Result<Self, ControlEnvelopeError> {
        let timestamp_unix_ms = unix_timestamp_millis()?;
        let message_id = hex_message_id();
        Self::signed_at(
            message_type,
            payload,
            message_id,
            timestamp_unix_ms,
            signing_key,
        )
    }

    pub fn signed_at(
        message_type: impl Into<String>,
        payload: Vec<u8>,
        message_id: impl Into<String>,
        timestamp_unix_ms: i64,
        signing_key: &SigningKey,
    ) -> Result<Self, ControlEnvelopeError> {
        let mut envelope = Self {
            message_type: message_type.into(),
            payload,
            public_key: String::new(),
            signature: String::new(),
            timestamp_unix_ms,
            message_id: message_id.into(),
        };
        envelope.validate_unsigned()?;
        envelope.public_key = BASE64.encode(signing_key.verifying_key().to_bytes());
        envelope.signature = BASE64.encode(signing_key.sign(&envelope.signable_data()).to_bytes());
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn encode(&self) -> Result<Vec<u8>, ControlEnvelopeError> {
        self.validate()?;
        let mut encoded = Vec::with_capacity(self.payload.len() + 256);
        encoded.push(0x96); // fixarray(6), the default MessagePack class shape.
        write_string(&mut encoded, &self.message_type)?;
        write_bytes(&mut encoded, &self.payload)?;
        write_string(&mut encoded, &self.public_key)?;
        write_string(&mut encoded, &self.signature)?;
        encoded.push(0xd3); // int64; the C# property is a System.Int64.
        encoded.extend_from_slice(&self.timestamp_unix_ms.to_be_bytes());
        write_string(&mut encoded, &self.message_id)?;
        if encoded.len() > CONTROL_MAX_DATAGRAM_BYTES {
            return Err(ControlEnvelopeError::DatagramTooLarge {
                actual: encoded.len(),
                max: CONTROL_MAX_DATAGRAM_BYTES,
            });
        }
        Ok(encoded)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ControlEnvelopeError> {
        if bytes.len() > CONTROL_MAX_DATAGRAM_BYTES {
            return Err(ControlEnvelopeError::DatagramTooLarge {
                actual: bytes.len(),
                max: CONTROL_MAX_DATAGRAM_BYTES,
            });
        }
        let mut reader = MessagePackReader::new(bytes);
        let fields = reader.read_array_len()?;
        if fields != ENVELOPE_FIELDS {
            return Err(ControlEnvelopeError::InvalidShape {
                expected: ENVELOPE_FIELDS,
                actual: fields,
            });
        }
        let message_type = reader.read_string("type")?;
        let payload = reader.read_bytes("payload")?;
        let public_key = reader.read_string("public_key")?;
        let signature = reader.read_string("signature")?;
        let timestamp_unix_ms = reader.read_i64("timestamp_unix_ms")?;
        let message_id = reader.read_string("message_id")?;
        reader.finish()?;
        let envelope = Self {
            message_type,
            payload,
            public_key,
            signature,
            timestamp_unix_ms,
            message_id,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    pub fn verify(&self) -> Result<(), ControlEnvelopeError> {
        self.validate()?;
        let public_key = decode_public_key(&self.public_key)?;
        let signature = decode_signature(&self.signature)?;
        let verifying_key = VerifyingKey::from_bytes(&public_key)
            .map_err(|_| ControlEnvelopeError::InvalidField("public_key"))?;
        verifying_key
            .verify(&self.signable_data(), &Signature::from_bytes(&signature))
            .map_err(|_| ControlEnvelopeError::InvalidSignature)
    }

    pub fn timestamp_is_current(&self, now_unix_ms: i64) -> bool {
        now_unix_ms.abs_diff(self.timestamp_unix_ms) <= CONTROL_TIMESTAMP_SKEW.as_millis() as u64
    }

    fn validate_unsigned(&self) -> Result<(), ControlEnvelopeError> {
        validate_string("type", &self.message_type, 256)?;
        validate_string("message_id", &self.message_id, 256)?;
        if self.payload.len() > CONTROL_MAX_PAYLOAD_BYTES {
            return Err(ControlEnvelopeError::PayloadTooLarge {
                actual: self.payload.len(),
                max: CONTROL_MAX_PAYLOAD_BYTES,
            });
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ControlEnvelopeError> {
        self.validate_unsigned()?;
        validate_string("public_key", &self.public_key, MAX_STRING_BYTES)?;
        validate_string("signature", &self.signature, MAX_STRING_BYTES)?;
        if self.public_key.is_empty() || self.signature.is_empty() {
            return Err(ControlEnvelopeError::InvalidField("signature credentials"));
        }
        let _ = decode_public_key(&self.public_key)?;
        let _ = decode_signature(&self.signature)?;
        Ok(())
    }

    fn signable_data(&self) -> Vec<u8> {
        let payload_hash = Sha256::digest(&self.payload);
        format!(
            "{}|{}|{}|{}",
            self.message_type,
            self.message_id,
            self.timestamp_unix_ms,
            BASE64.encode(payload_hash),
        )
        .into_bytes()
    }

    /// Legacy format accepted by slskdN during its compatibility window.
    #[must_use]
    pub fn legacy_signable_data(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}",
            self.message_type,
            self.timestamp_unix_ms,
            BASE64.encode(&self.payload),
        )
        .into_bytes()
    }
}

/// Send one signed control envelope over the slskdN UDP control plane.
pub async fn send_udp_control(
    endpoint: SocketAddr,
    envelope: &ControlEnvelope,
) -> Result<usize, ControlEnvelopeError> {
    let payload = envelope.encode()?;
    let bind = match endpoint {
        SocketAddr::V4(_) => "0.0.0.0:0",
        SocketAddr::V6(_) => "[::]:0",
    };
    let socket = UdpSocket::bind(bind).await?;
    Ok(socket.send_to(&payload, endpoint).await?)
}

fn unix_timestamp_millis() -> Result<i64, ControlEnvelopeError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ControlEnvelopeError::InvalidTime)?
        .as_millis();
    i64::try_from(millis).map_err(|_| ControlEnvelopeError::InvalidTime)
}

fn hex_message_id() -> String {
    let bytes = rand::random::<[u8; 16]>();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(b"0123456789abcdef"[(byte >> 4) as usize]));
        output.push(char::from(b"0123456789abcdef"[(byte & 0x0f) as usize]));
    }
    output
}

fn validate_string(
    field: &'static str,
    value: &str,
    max_bytes: usize,
) -> Result<(), ControlEnvelopeError> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        return Err(ControlEnvelopeError::InvalidField(field));
    }
    Ok(())
}

fn decode_public_key(value: &str) -> Result<[u8; 32], ControlEnvelopeError> {
    let bytes = BASE64
        .decode(value)
        .map_err(|_| ControlEnvelopeError::InvalidField("public_key"))?;
    bytes
        .try_into()
        .map_err(|_| ControlEnvelopeError::InvalidField("public_key"))
}

fn decode_signature(value: &str) -> Result<[u8; 64], ControlEnvelopeError> {
    let bytes = BASE64
        .decode(value)
        .map_err(|_| ControlEnvelopeError::InvalidField("signature"))?;
    bytes
        .try_into()
        .map_err(|_| ControlEnvelopeError::InvalidField("signature"))
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), ControlEnvelopeError> {
    let bytes = value.as_bytes();
    if bytes.len() > u32::MAX as usize || bytes.len() > MAX_STRING_BYTES {
        return Err(ControlEnvelopeError::InvalidField("string"));
    }
    match bytes.len() {
        0..=31 => output.push(0xa0 | bytes.len() as u8),
        32..=255 => {
            output.push(0xd9);
            output.push(bytes.len() as u8);
        }
        256..=65_535 => {
            output.push(0xda);
            output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        }
        _ => {
            output.push(0xdb);
            output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        }
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn write_bytes(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ControlEnvelopeError> {
    if bytes.len() > CONTROL_MAX_PAYLOAD_BYTES {
        return Err(ControlEnvelopeError::PayloadTooLarge {
            actual: bytes.len(),
            max: CONTROL_MAX_PAYLOAD_BYTES,
        });
    }
    match bytes.len() {
        0..=255 => {
            output.push(0xc4);
            output.push(bytes.len() as u8);
        }
        256..=65_535 => {
            output.push(0xc5);
            output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        }
        _ => {
            output.push(0xc6);
            output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        }
    }
    output.extend_from_slice(bytes);
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ControlEnvelopeError {
    #[error("control envelope I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("control envelope field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("control envelope payload is too large: {actual} > {max}")]
    PayloadTooLarge { actual: usize, max: usize },
    #[error("control envelope datagram is too large: {actual} > {max}")]
    DatagramTooLarge { actual: usize, max: usize },
    #[error("control envelope has {actual} fields; expected {expected}")]
    InvalidShape { expected: usize, actual: usize },
    #[error("control envelope MessagePack is truncated while reading {0}")]
    Truncated(&'static str),
    #[error("control envelope MessagePack has an invalid {field} marker")]
    InvalidMarker { field: &'static str },
    #[error("control envelope MessagePack contains invalid UTF-8 in {0}")]
    InvalidUtf8(&'static str),
    #[error("control envelope MessagePack has trailing bytes")]
    TrailingBytes,
    #[error("control envelope signature is invalid")]
    InvalidSignature,
    #[error("control envelope timestamp is invalid")]
    InvalidTime,
}

struct MessagePackReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> MessagePackReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read_array_len(&mut self) -> Result<usize, ControlEnvelopeError> {
        let marker = self.read_byte("array")?;
        match marker {
            0x90..=0x9f => Ok((marker & 0x0f) as usize),
            0xdc => Ok(usize::from(self.read_u16("array")?)),
            0xdd => self.read_u32("array").and_then(|value| {
                usize::try_from(value).map_err(|_| ControlEnvelopeError::InvalidField("array"))
            }),
            _ => Err(ControlEnvelopeError::InvalidMarker { field: "array" }),
        }
    }

    fn read_string(&mut self, field: &'static str) -> Result<String, ControlEnvelopeError> {
        let length = self.read_string_len(field)?;
        let bytes = self.read_slice(length, field)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ControlEnvelopeError::InvalidUtf8(field))
    }

    fn read_bytes(&mut self, field: &'static str) -> Result<Vec<u8>, ControlEnvelopeError> {
        let marker = self.read_byte(field)?;
        let length = match marker {
            0xc4 => usize::from(self.read_byte(field)?),
            0xc5 => usize::from(self.read_u16(field)?),
            0xc6 => usize::try_from(self.read_u32(field)?)
                .map_err(|_| ControlEnvelopeError::InvalidField(field))?,
            _ => return Err(ControlEnvelopeError::InvalidMarker { field }),
        };
        if length > CONTROL_MAX_PAYLOAD_BYTES {
            return Err(ControlEnvelopeError::PayloadTooLarge {
                actual: length,
                max: CONTROL_MAX_PAYLOAD_BYTES,
            });
        }
        Ok(self.read_slice(length, field)?.to_vec())
    }

    fn read_i64(&mut self, field: &'static str) -> Result<i64, ControlEnvelopeError> {
        let marker = self.read_byte(field)?;
        match marker {
            0xd3 => {
                let bytes = self.read_slice(8, field)?;
                Ok(i64::from_be_bytes(
                    bytes.try_into().expect("eight-byte slice"),
                ))
            }
            0xd2 => Ok(i64::from(i32::from_be_bytes(
                self.read_slice(4, field)?
                    .try_into()
                    .expect("four-byte slice"),
            ))),
            0xd1 => Ok(i64::from(i16::from_be_bytes(
                self.read_slice(2, field)?
                    .try_into()
                    .expect("two-byte slice"),
            ))),
            0xd0 => Ok(i64::from(i8::from_be_bytes(
                self.read_slice(1, field)?
                    .try_into()
                    .expect("one-byte slice"),
            ))),
            0xcc => Ok(i64::from(self.read_byte(field)?)),
            0xcd => Ok(i64::from(self.read_u16(field)?)),
            0xce => Ok(i64::from(self.read_u32(field)?)),
            0xcf => {
                let bytes = self.read_slice(8, field)?;
                u64::from_be_bytes(bytes.try_into().expect("eight-byte slice"))
                    .try_into()
                    .map_err(|_| ControlEnvelopeError::InvalidField(field))
            }
            value @ 0x00..=0x7f => Ok(i64::from(value)),
            value @ 0xe0..=0xff => Ok(i64::from(i8::from_be_bytes([value]))),
            _ => Err(ControlEnvelopeError::InvalidMarker { field }),
        }
    }

    fn finish(&self) -> Result<(), ControlEnvelopeError> {
        if self.position == self.bytes.len() {
            Ok(())
        } else {
            Err(ControlEnvelopeError::TrailingBytes)
        }
    }

    fn read_string_len(&mut self, field: &'static str) -> Result<usize, ControlEnvelopeError> {
        let marker = self.read_byte(field)?;
        let length = match marker {
            0xa0..=0xbf => usize::from(marker & 0x1f),
            0xd9 => usize::from(self.read_byte(field)?),
            0xda => usize::from(self.read_u16(field)?),
            0xdb => usize::try_from(self.read_u32(field)?)
                .map_err(|_| ControlEnvelopeError::InvalidField(field))?,
            _ => return Err(ControlEnvelopeError::InvalidMarker { field }),
        };
        if length > MAX_STRING_BYTES {
            return Err(ControlEnvelopeError::InvalidField(field));
        }
        Ok(length)
    }

    fn read_byte(&mut self, field: &'static str) -> Result<u8, ControlEnvelopeError> {
        let byte = self
            .bytes
            .get(self.position)
            .copied()
            .ok_or(ControlEnvelopeError::Truncated(field))?;
        self.position += 1;
        Ok(byte)
    }

    fn read_u16(&mut self, field: &'static str) -> Result<u16, ControlEnvelopeError> {
        let bytes = self.read_slice(2, field)?;
        Ok(u16::from_be_bytes(
            bytes.try_into().expect("two-byte slice"),
        ))
    }

    fn read_u32(&mut self, field: &'static str) -> Result<u32, ControlEnvelopeError> {
        let bytes = self.read_slice(4, field)?;
        Ok(u32::from_be_bytes(
            bytes.try_into().expect("four-byte slice"),
        ))
    }

    fn read_slice(
        &mut self,
        length: usize,
        field: &'static str,
    ) -> Result<&'a [u8], ControlEnvelopeError> {
        let end = self
            .position
            .checked_add(length)
            .ok_or(ControlEnvelopeError::Truncated(field))?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or(ControlEnvelopeError::Truncated(field))?;
        self.position = end;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_envelope_matches_target_shape_and_round_trips() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let envelope = ControlEnvelope::signed_at(
            "pod_message",
            br#"{"MessageId":"message-1"}"#.to_vec(),
            "message-1",
            1_725_000_000_123,
            &key,
        )
        .expect("signed envelope");
        let encoded = envelope.encode().expect("encode envelope");
        assert_eq!(encoded[0], 0x96);
        let decoded = ControlEnvelope::decode(&encoded).expect("decode envelope");
        assert_eq!(decoded, envelope);
        decoded.verify().expect("verify envelope");
    }

    #[test]
    fn decoder_rejects_wrong_shape_and_trailing_bytes() {
        assert!(matches!(
            ControlEnvelope::decode(&[0x95]),
            Err(ControlEnvelopeError::InvalidShape { .. })
        ));
        let key = SigningKey::from_bytes(&[8; 32]);
        let envelope = ControlEnvelope::signed_at("ping", Vec::new(), "message-2", 1, &key)
            .expect("signed envelope");
        let mut encoded = envelope.encode().expect("encode envelope");
        encoded.push(0);
        assert!(matches!(
            ControlEnvelope::decode(&encoded),
            Err(ControlEnvelopeError::TrailingBytes)
        ));
    }

    #[tokio::test]
    async fn udp_sender_emits_decodable_datagram() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind receiver");
        let endpoint = socket.local_addr().expect("receiver address");
        let key = SigningKey::from_bytes(&[9; 32]);
        let envelope =
            ControlEnvelope::signed_at("pod_message", b"payload".to_vec(), "message-3", 2, &key)
                .expect("signed envelope");
        let sent = send_udp_control(endpoint, &envelope)
            .await
            .expect("send envelope");
        assert_eq!(sent, envelope.encode().unwrap().len());
        let mut received = [0_u8; CONTROL_MAX_DATAGRAM_BYTES];
        let (length, remote) = socket
            .recv_from(&mut received)
            .await
            .expect("receive envelope");
        assert!(!remote.ip().is_unspecified());
        let decoded = ControlEnvelope::decode(&received[..length]).expect("decode datagram");
        assert_eq!(decoded, envelope);
    }
}
