//! Conservative QUIC Initial ALPN inspection for the shared mesh UDP socket.
//!
//! QUIC Initial packets use keys derived from the visible destination
//! connection ID and a public version salt.  That lets the shared listener
//! distinguish the control and data backends before either backend has a
//! connection object.  Any ambiguity returns `None`; the caller then uses its
//! safe control-plane fallback.

use ring::{
    aead::{self, quic::HeaderProtectionKey},
    hkdf,
};

const QUIC_V1_INITIAL_SALT: [u8; 20] = [
    0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9d, 0xa6, 0xa4, 0xc8, 0x0c, 0xad,
    0xcc, 0xbb, 0x7f, 0x0a,
];
const QUIC_V2_INITIAL_SALT: [u8; 20] = [
    0x0d, 0xed, 0xe3, 0xde, 0xf7, 0x00, 0xa6, 0xdb, 0x81, 0x93, 0x81, 0xbe, 0x6e, 0x26, 0x9d, 0xcb,
    0xf9, 0xbd, 0x2e, 0x09,
];
const SAMPLE_LENGTH: usize = 16;
const AEAD_TAG_LENGTH: usize = 16;
const MAX_CRYPTO_FRAME_LENGTH: usize = 4096;
const FRAME_TYPE_PADDING: u64 = 0;
const FRAME_TYPE_PING: u64 = 1;
const FRAME_TYPE_CRYPTO: u64 = 6;
const TLS_CLIENT_HELLO: u8 = 1;
const TLS_EXTENSION_ALPN: u16 = 16;

struct OutputLength(usize);

impl hkdf::KeyType for OutputLength {
    fn len(&self) -> usize {
        self.0
    }
}

/// Extract the first ALPN offered by a complete QUIC Initial ClientHello.
pub(crate) fn first_alpn(datagram: &[u8]) -> Option<String> {
    first_alpn_inner(datagram).ok().flatten()
}

fn first_alpn_inner(datagram: &[u8]) -> Result<Option<String>, ()> {
    if datagram.len() < 1_200 || datagram.first().is_none_or(|byte| byte & 0xc0 != 0xc0) {
        return Err(());
    }

    let version = u32::from_be_bytes(datagram[1..5].try_into().map_err(|_| ())?);
    let packet_type = (datagram[0] & 0x30) >> 4;
    let salt = match (version, packet_type) {
        (1, 0) => &QUIC_V1_INITIAL_SALT,
        (0x6b33_43cf, 1) => &QUIC_V2_INITIAL_SALT,
        _ => return Err(()),
    };

    let mut offset = 5;
    let dcid_length = *datagram.get(offset).ok_or(())? as usize;
    offset += 1;
    if dcid_length > 20 || offset.checked_add(dcid_length).ok_or(())? > datagram.len() {
        return Err(());
    }
    let dcid = &datagram[offset..offset + dcid_length];
    offset += dcid_length;

    let scid_length = *datagram.get(offset).ok_or(())? as usize;
    offset += 1;
    offset = offset.checked_add(scid_length).ok_or(())?;
    if offset > datagram.len() {
        return Err(());
    }

    let token_length = read_varint(datagram, &mut offset).ok_or(())?;
    let token_length = usize::try_from(token_length).map_err(|_| ())?;
    offset = offset.checked_add(token_length).ok_or(())?;
    if offset > datagram.len() {
        return Err(());
    }

    let packet_length =
        usize::try_from(read_varint(datagram, &mut offset).ok_or(())?).map_err(|_| ())?;
    let packet_number_offset = offset;
    let packet_end = packet_number_offset.checked_add(packet_length).ok_or(())?;
    if packet_end > datagram.len() || packet_length < 4 + AEAD_TAG_LENGTH {
        return Err(());
    }

    let sample_offset = packet_number_offset.checked_add(4).ok_or(())?;
    let sample_end = sample_offset.checked_add(SAMPLE_LENGTH).ok_or(())?;
    if sample_end > datagram.len() {
        return Err(());
    }

    let initial_secret = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(dcid);
    let client_initial_secret = expand_bytes(&initial_secret, "client in", 32)?;
    let client_initial_prk = hkdf::Prk::new_less_safe(hkdf::HKDF_SHA256, &client_initial_secret);
    let header_protection_key = expand_quic_key(&client_initial_prk, "quic hp")?;
    let packet_key = expand_bytes(&client_initial_prk, "quic key", 16)?;
    let packet_iv = expand_bytes(&client_initial_prk, "quic iv", 12)?;

    let mask = header_protection_key
        .new_mask(&datagram[sample_offset..sample_end])
        .map_err(|_| ())?;
    let first_byte = datagram[0] ^ (mask[0] & 0x0f);
    let packet_number_length = usize::from((first_byte & 0x03) + 1);
    if packet_number_offset
        .checked_add(packet_number_length)
        .ok_or(())?
        > packet_end
        || packet_number_length + AEAD_TAG_LENGTH > packet_length
    {
        return Err(());
    }

    let mut packet_number_bytes = [0_u8; 4];
    for (index, byte) in packet_number_bytes
        .iter_mut()
        .take(packet_number_length)
        .enumerate()
    {
        *byte = datagram[packet_number_offset + index] ^ mask[index + 1];
    }
    let packet_number = packet_number_bytes[..packet_number_length]
        .iter()
        .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte));

    let mut associated_data = datagram[..packet_number_offset + packet_number_length].to_vec();
    associated_data[0] = first_byte;
    associated_data[packet_number_offset..]
        .copy_from_slice(&packet_number_bytes[..packet_number_length]);

    let payload_start = packet_number_offset + packet_number_length;
    let payload_length = packet_length - packet_number_length;
    if payload_length <= AEAD_TAG_LENGTH || payload_start + payload_length > datagram.len() {
        return Err(());
    }

    let mut encrypted_payload = datagram[payload_start..payload_start + payload_length].to_vec();
    let mut nonce = [0_u8; 12];
    nonce.copy_from_slice(&packet_iv);
    let packet_number_bytes = packet_number.to_be_bytes();
    for (index, byte) in packet_number_bytes.iter().enumerate() {
        nonce[4 + index] ^= byte;
    }
    let key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_128_GCM, &packet_key).map_err(|_| ())?,
    );
    let plaintext = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::from(associated_data),
            &mut encrypted_payload,
        )
        .map_err(|_| ())?;
    extract_alpn_from_initial_plaintext(plaintext)
}

fn expand_quic_key(secret: &hkdf::Prk, label: &'static str) -> Result<HeaderProtectionKey, ()> {
    let label = quic_label(label, 16)?;
    secret
        .expand(&[label.as_slice()], ring_quic_aes_128())
        .map(HeaderProtectionKey::from)
        .map_err(|_| ())
}

fn ring_quic_aes_128() -> &'static ring::aead::quic::Algorithm {
    &ring::aead::quic::AES_128
}

fn expand_bytes(secret: &hkdf::Prk, label: &'static str, length: usize) -> Result<Vec<u8>, ()> {
    let label = quic_label(label, length)?;
    let mut output = vec![0_u8; length];
    secret
        .expand(&[label.as_slice()], OutputLength(length))
        .map_err(|_| ())?
        .fill(&mut output)
        .map_err(|_| ())?;
    Ok(output)
}

fn quic_label(label: &'static str, length: usize) -> Result<Vec<u8>, ()> {
    let full_label = format!("tls13 {label}");
    let label_length = u8::try_from(full_label.len()).map_err(|_| ())?;
    let length = u16::try_from(length).map_err(|_| ())?;
    let mut info = Vec::with_capacity(2 + 1 + full_label.len() + 1);
    info.extend_from_slice(&length.to_be_bytes());
    info.push(label_length);
    info.extend_from_slice(full_label.as_bytes());
    info.push(0);
    Ok(info)
}

fn read_varint(buffer: &[u8], offset: &mut usize) -> Option<u64> {
    let first = *buffer.get(*offset)?;
    let length = 1_usize << usize::from(first >> 6);
    let end = offset.checked_add(length)?;
    if end > buffer.len() {
        return None;
    }
    let mut value = u64::from(first & 0x3f);
    for byte in &buffer[*offset + 1..end] {
        value = (value << 8) | u64::from(*byte);
    }
    *offset = end;
    Some(value)
}

fn extract_alpn_from_initial_plaintext(plaintext: &[u8]) -> Result<Option<String>, ()> {
    let mut offset = 0;
    while offset < plaintext.len() {
        let frame_type = read_varint(plaintext, &mut offset).ok_or(())?;
        match frame_type {
            FRAME_TYPE_PADDING | FRAME_TYPE_PING => {}
            FRAME_TYPE_CRYPTO => {
                let crypto_offset = read_varint(plaintext, &mut offset).ok_or(())?;
                let crypto_length = usize::try_from(read_varint(plaintext, &mut offset).ok_or(())?)
                    .map_err(|_| ())?;
                if crypto_offset != 0
                    || crypto_length > MAX_CRYPTO_FRAME_LENGTH
                    || offset + crypto_length > plaintext.len()
                {
                    return Err(());
                }
                return extract_alpn_from_client_hello(&plaintext[offset..offset + crypto_length]);
            }
            _ => return Err(()),
        }
    }
    Err(())
}

fn extract_alpn_from_client_hello(message: &[u8]) -> Result<Option<String>, ()> {
    if message.len() < 4 || message[0] != TLS_CLIENT_HELLO {
        return Err(());
    }
    let declared_length =
        (usize::from(message[1]) << 16) | (usize::from(message[2]) << 8) | usize::from(message[3]);
    let body_length = declared_length.min(message.len() - 4);
    let body = &message[4..4 + body_length];
    let mut offset = 0;
    if offset + 34 > body.len() {
        return Err(());
    }
    offset += 34;
    let session_id_length = usize::from(*body.get(offset).ok_or(())?);
    offset += 1;
    offset += session_id_length;
    if offset + 2 > body.len() {
        return Err(());
    }
    let cipher_suites_length = usize::from(u16::from_be_bytes(
        body[offset..offset + 2].try_into().map_err(|_| ())?,
    ));
    offset += 2 + cipher_suites_length;
    let compression_length = usize::from(*body.get(offset).ok_or(())?);
    offset += 1 + compression_length;
    if offset + 2 > body.len() {
        return Err(());
    }
    let extensions_length = usize::from(u16::from_be_bytes(
        body[offset..offset + 2].try_into().map_err(|_| ())?,
    ));
    offset += 2;
    let extensions_end = offset.checked_add(extensions_length).ok_or(())?;
    if extensions_end > body.len() {
        return Err(());
    }

    while offset + 4 <= extensions_end {
        let extension_type =
            u16::from_be_bytes(body[offset..offset + 2].try_into().map_err(|_| ())?);
        let extension_length = usize::from(u16::from_be_bytes(
            body[offset + 2..offset + 4].try_into().map_err(|_| ())?,
        ));
        offset += 4;
        if offset + extension_length > extensions_end {
            return Err(());
        }
        if extension_type == TLS_EXTENSION_ALPN {
            return extract_alpn_extension(&body[offset..offset + extension_length]);
        }
        offset += extension_length;
    }
    Err(())
}

fn extract_alpn_extension(extension: &[u8]) -> Result<Option<String>, ()> {
    if extension.len() < 3 {
        return Err(());
    }
    let protocols_length = usize::from(u16::from_be_bytes(
        extension[..2].try_into().map_err(|_| ())?,
    ));
    if protocols_length < 1 || 2 + protocols_length > extension.len() {
        return Err(());
    }
    let name_length = usize::from(extension[2]);
    if 3 + name_length > extension.len() {
        return Err(());
    }
    let name = std::str::from_utf8(&extension[3..3 + name_length]).map_err(|_| ())?;
    Ok(Some(name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_initial_and_truncated_packets() {
        assert_eq!(first_alpn(&[]), None);
        assert_eq!(first_alpn(&[0xc0; 1_200]), None);
        assert_eq!(first_alpn(&[0x40; 1_200]), None);
    }

    #[test]
    fn extracts_first_alpn_from_client_hello() {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x03, 0x03]);
        body.extend_from_slice(&[0; 32]);
        body.push(0); // session id
        body.extend_from_slice(&[0, 2, 0x13, 0x01]);
        body.extend_from_slice(&[1, 0]);
        let alpn = b"\x00\x14\x13slskdn-overlay-data";
        let extension_length = 4 + alpn.len();
        body.extend_from_slice(&(extension_length as u16).to_be_bytes());
        body.extend_from_slice(&TLS_EXTENSION_ALPN.to_be_bytes());
        body.extend_from_slice(&(alpn.len() as u16).to_be_bytes());
        body.extend_from_slice(alpn);
        let mut message = vec![TLS_CLIENT_HELLO, 0, 0, body.len() as u8];
        message.extend_from_slice(&body);
        assert_eq!(
            extract_alpn_from_client_hello(&message).unwrap(),
            Some("slskdn-overlay-data".to_owned())
        );
    }
}
