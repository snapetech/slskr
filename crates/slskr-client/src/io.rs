use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{connection::ConnectionKind, ClientError};
use slskr_protocol::{
    decode_rotated, encode_rotated,
    frame::{InitFrame, MessageFrame},
    init::{InitCode, MAX_INIT_FIELD_BYTES, MAX_PEER_INIT_FRAME_LEN},
    RawFrame,
};

pub const DEFAULT_MAX_FRAME_LEN: usize = 16 * 1024 * 1024;
/// Absolute upper bound for one protocol frame, regardless of caller input.
pub const MAX_FRAME_LEN: usize = DEFAULT_MAX_FRAME_LEN;
const MAX_PIERCE_FIREWALL_FRAME_LEN: usize = 1 + 4;

fn bounded_frame_len(value: usize) -> usize {
    value.min(MAX_FRAME_LEN)
}

pub async fn read_connection_kind<R>(reader: &mut R) -> Result<ConnectionKind, ClientError>
where
    R: AsyncRead + Unpin,
{
    let byte = reader.read_u8().await?;
    ConnectionKind::try_from(byte)
}

pub async fn write_connection_kind<W>(
    writer: &mut W,
    kind: ConnectionKind,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    writer.write_u8(kind.as_byte()).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_message_frame<R>(reader: &mut R) -> Result<MessageFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    read_message_frame_with_max(reader, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn read_message_frame_with_max<R>(
    reader: &mut R,
    max_len: usize,
) -> Result<MessageFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    let encoded = read_len_prefixed_frame(reader, max_len).await?;
    Ok(MessageFrame::decode(&encoded)?)
}

/// Reads a message frame while retaining bytes received before a cancelled
/// read. Tokio's `read_exact` is not cancellation-safe: cancelling it after
/// it has consumed only part of a frame loses those bytes and desynchronizes
/// the next frame boundary. Long-lived connections use this variant because
/// their receive future is periodically cancelled by the session loop.
pub async fn read_message_frame_buffered<R>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
    max_len: usize,
) -> Result<MessageFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    loop {
        if buffer.len() >= 4 {
            let length = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
            if length > max_len {
                return Err(ClientError::FrameTooLarge {
                    length,
                    max: max_len,
                });
            }

            let encoded_len = prefixed_frame_len(length, 4, max_len)?;
            if buffer.len() >= encoded_len {
                let encoded = buffer.drain(..encoded_len).collect::<Vec<_>>();
                return Ok(MessageFrame::decode(&encoded)?);
            }
        }

        let mut chunk = [0_u8; 8192];
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Err(ClientError::ConnectionClosed);
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
}

pub async fn write_message_frame<W>(writer: &mut W, frame: &MessageFrame) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_message_frame_with_max(writer, frame, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn write_message_frame_with_max<W>(
    writer: &mut W,
    frame: &MessageFrame,
    max_len: usize,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    validate_frame_len(frame.payload.len().saturating_add(4), max_len)?;
    writer.write_all(&frame.encode()?).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_obfuscated_message_frame<R>(reader: &mut R) -> Result<MessageFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let encoded = read_obfuscated_len_prefixed_frame(reader, DEFAULT_MAX_FRAME_LEN).await?;
    Ok(MessageFrame::decode(&encoded)?)
}

pub async fn write_obfuscated_message_frame<W>(
    writer: &mut W,
    frame: &MessageFrame,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_obfuscated_message_frame_with_key(writer, frame, rand::random()).await
}

pub async fn write_obfuscated_message_frame_with_key<W>(
    writer: &mut W,
    frame: &MessageFrame,
    key: u32,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_obfuscated_message_frame_with_key_and_max(writer, frame, key, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn write_obfuscated_message_frame_with_key_and_max<W>(
    writer: &mut W,
    frame: &MessageFrame,
    key: u32,
    max_len: usize,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    validate_frame_len(frame.payload.len().saturating_add(4), max_len)?;
    writer
        .write_all(&encode_rotated(&frame.encode()?, key))
        .await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_init_frame<R>(reader: &mut R) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    read_init_frame_with_max(reader, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn read_init_frame_with_max<R>(
    reader: &mut R,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    let mut length_bytes = [0_u8; 4];
    reader.read_exact(&mut length_bytes).await?;
    read_init_frame_after_length(reader, length_bytes, max_len).await
}

pub async fn read_init_frame_with_first_len_byte<R>(
    reader: &mut R,
    first_len_byte: u8,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    read_init_frame_with_first_len_byte_and_max(reader, first_len_byte, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn read_init_frame_with_first_len_byte_and_max<R>(
    reader: &mut R,
    first_len_byte: u8,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    let mut length_bytes = [first_len_byte, 0, 0, 0];
    reader.read_exact(&mut length_bytes[1..]).await?;
    read_init_frame_after_length(reader, length_bytes, max_len).await
}

pub async fn write_init_frame<W>(writer: &mut W, frame: &InitFrame) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_init_frame_with_max(writer, frame, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn write_init_frame_with_max<W>(
    writer: &mut W,
    frame: &InitFrame,
    max_len: usize,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    validate_frame_len(frame.payload.len().saturating_add(1), max_len)?;
    writer.write_all(&frame.encode()?).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_obfuscated_init_frame<R>(reader: &mut R) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    read_obfuscated_init_frame_with_max(reader, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn write_obfuscated_init_frame<W>(
    writer: &mut W,
    frame: &InitFrame,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    let key = loop {
        let key: u32 = rand::random();
        if key as usize > DEFAULT_MAX_FRAME_LEN {
            break key;
        }
    };
    write_obfuscated_init_frame_with_key(writer, frame, key).await
}

pub async fn write_obfuscated_init_frame_with_key<W>(
    writer: &mut W,
    frame: &InitFrame,
    key: u32,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_obfuscated_init_frame_with_key_and_max(writer, frame, key, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn write_obfuscated_init_frame_with_key_and_max<W>(
    writer: &mut W,
    frame: &InitFrame,
    key: u32,
    max_len: usize,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    validate_frame_len(frame.payload.len().saturating_add(1), max_len)?;
    writer
        .write_all(&encode_rotated(&frame.encode()?, key))
        .await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_raw_frame<R>(reader: &mut R, length: usize) -> Result<RawFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    read_raw_frame_with_max(reader, length, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn read_raw_frame_with_max<R>(
    reader: &mut R,
    length: usize,
    max_len: usize,
) -> Result<RawFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    if length > max_len {
        return Err(ClientError::FrameTooLarge {
            length,
            max: max_len,
        });
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).await?;
    Ok(RawFrame::new(payload))
}

pub async fn write_raw_frame<W>(writer: &mut W, frame: &RawFrame) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_raw_frame_with_max(writer, frame, DEFAULT_MAX_FRAME_LEN).await
}

pub async fn write_raw_frame_with_max<W>(
    writer: &mut W,
    frame: &RawFrame,
    max_len: usize,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    validate_frame_len(frame.payload.len(), max_len)?;
    writer.write_all(&frame.encode()).await?;
    writer.flush().await?;
    Ok(())
}

fn validate_frame_len(length: usize, max: usize) -> Result<(), ClientError> {
    if length > max {
        Err(ClientError::FrameTooLarge { length, max })
    } else {
        Ok(())
    }
}

fn prefixed_frame_len(
    length: usize,
    prefix_len: usize,
    configured_max: usize,
) -> Result<usize, ClientError> {
    length
        .checked_add(prefix_len)
        .ok_or(ClientError::FrameTooLarge {
            length,
            max: configured_max.min(usize::MAX - prefix_len),
        })
}

async fn read_init_frame_after_length<R>(
    reader: &mut R,
    length_bytes: [u8; 4],
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let length = u32::from_le_bytes(length_bytes) as usize;
    if length > max_len {
        return Err(ClientError::FrameTooLarge {
            length,
            max: max_len,
        });
    }
    if length == 0 {
        return Ok(InitFrame::decode(&length_bytes)?);
    }

    let code = reader.read_u8().await?;
    let mut prefix = Vec::with_capacity(5);
    prefix.extend_from_slice(&length_bytes);
    prefix.push(code);

    match InitCode::try_from(code) {
        Ok(InitCode::PeerInit) => read_peer_init_frame(reader, prefix, length, max_len).await,
        Ok(InitCode::PierceFirewall) if length > MAX_PIERCE_FIREWALL_FRAME_LEN => {
            Err(ClientError::FrameTooLarge {
                length,
                max: MAX_PIERCE_FIREWALL_FRAME_LEN,
            })
        }
        _ => read_init_frame_from_prefix(reader, prefix, length, max_len).await,
    }
}

async fn read_peer_init_frame<R>(
    reader: &mut R,
    mut prefix: Vec<u8>,
    length: usize,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    // The maximum is derived from both bounded string fields and the fixed
    // code/prefix/token bytes. Reject it before allocating a frame-sized Vec.
    if length > MAX_PEER_INIT_FRAME_LEN {
        return Err(ClientError::FrameTooLarge {
            length,
            max: MAX_PEER_INIT_FRAME_LEN,
        });
    }
    if length < 1 + 4 {
        return read_init_frame_from_prefix(reader, prefix, length, max_len).await;
    }

    let mut field_length = [0_u8; 4];
    reader.read_exact(&mut field_length).await?;
    prefix.extend_from_slice(&field_length);
    let username_length = u32::from_le_bytes(field_length) as usize;
    if username_length > MAX_INIT_FIELD_BYTES {
        return Err(ClientError::PeerUsernameTooLong {
            length: username_length,
            max: MAX_INIT_FIELD_BYTES,
        });
    }

    // If the username or the next length prefix is truncated, retain the
    // existing decoder error after reading only this already bounded frame.
    let connection_length_offset = 1 + 4 + username_length + 4;
    if length < connection_length_offset {
        return read_init_frame_from_prefix(reader, prefix, length, max_len).await;
    }

    let username_start = prefix.len();
    prefix.resize(username_start + username_length, 0);
    reader.read_exact(&mut prefix[username_start..]).await?;

    let connection_length_start = prefix.len();
    prefix.resize(connection_length_start + 4, 0);
    reader
        .read_exact(&mut prefix[connection_length_start..])
        .await?;
    let connection_length = u32::from_le_bytes(
        prefix[connection_length_start..connection_length_start + 4]
            .try_into()
            .expect("connection type length is four bytes"),
    ) as usize;
    if connection_length > MAX_INIT_FIELD_BYTES {
        return Err(ClientError::FrameTooLarge {
            length: connection_length,
            max: MAX_INIT_FIELD_BYTES,
        });
    }

    let expected_length = connection_length_offset
        .checked_add(connection_length)
        .and_then(|length| length.checked_add(4))
        .expect("bounded init field lengths fit in usize");
    if length < expected_length {
        return read_init_frame_from_prefix(reader, prefix, length, max_len).await;
    }

    read_init_frame_from_prefix(reader, prefix, length, max_len).await
}

async fn read_init_frame_from_prefix<R>(
    reader: &mut R,
    mut prefix: Vec<u8>,
    length: usize,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let encoded_len = prefixed_frame_len(length, 4, max_len)?;
    let start = prefix.len();
    prefix.resize(encoded_len, 0);
    reader.read_exact(&mut prefix[start..]).await?;
    Ok(InitFrame::decode(&prefix)?)
}

async fn read_obfuscated_init_frame_with_max<R>(
    reader: &mut R,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    let mut first_block = [0_u8; 8];
    reader.read_exact(&mut first_block).await?;
    let decoded_first_block = decode_rotated(&first_block)?;
    let length = u32::from_le_bytes(
        decoded_first_block[..4]
            .try_into()
            .expect("obfuscated init length is four bytes"),
    ) as usize;
    if length > max_len {
        return Err(ClientError::FrameTooLarge {
            length,
            max: max_len,
        });
    }
    if length == 0 {
        return read_obfuscated_init_frame_from_prefix(
            reader,
            first_block.to_vec(),
            length,
            max_len,
        )
        .await;
    }

    let mut prefix = first_block.to_vec();
    prefix.resize(9, 0);
    reader.read_exact(&mut prefix[8..]).await?;
    let decoded_prefix = decode_rotated(&prefix)?;
    let code = decoded_prefix[4];

    match InitCode::try_from(code) {
        Ok(InitCode::PeerInit) => {
            read_obfuscated_peer_init_frame(reader, prefix, length, max_len).await
        }
        Ok(InitCode::PierceFirewall) if length > MAX_PIERCE_FIREWALL_FRAME_LEN => {
            Err(ClientError::FrameTooLarge {
                length,
                max: MAX_PIERCE_FIREWALL_FRAME_LEN,
            })
        }
        _ => read_obfuscated_init_frame_from_prefix(reader, prefix, length, max_len).await,
    }
}

async fn read_obfuscated_peer_init_frame<R>(
    reader: &mut R,
    mut prefix: Vec<u8>,
    length: usize,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    if length > MAX_PEER_INIT_FRAME_LEN {
        return Err(ClientError::FrameTooLarge {
            length,
            max: MAX_PEER_INIT_FRAME_LEN,
        });
    }
    if length < 1 + 4 {
        return read_obfuscated_init_frame_from_prefix(reader, prefix, length, max_len).await;
    }

    prefix.resize(13, 0);
    reader.read_exact(&mut prefix[9..]).await?;
    let decoded_prefix = decode_rotated(&prefix)?;
    let username_length = u32::from_le_bytes(
        decoded_prefix[5..9]
            .try_into()
            .expect("username length is four bytes"),
    ) as usize;
    if username_length > MAX_INIT_FIELD_BYTES {
        return Err(ClientError::PeerUsernameTooLong {
            length: username_length,
            max: MAX_INIT_FIELD_BYTES,
        });
    }

    let connection_length_offset = 1 + 4 + username_length + 4;
    if length < connection_length_offset {
        return read_obfuscated_init_frame_from_prefix(reader, prefix, length, max_len).await;
    }

    let username_start = prefix.len();
    prefix.resize(username_start + username_length, 0);
    reader.read_exact(&mut prefix[username_start..]).await?;

    let connection_length_start = prefix.len();
    prefix.resize(connection_length_start + 4, 0);
    reader
        .read_exact(&mut prefix[connection_length_start..])
        .await?;
    let decoded_prefix = decode_rotated(&prefix)?;
    let connection_length = u32::from_le_bytes(
        decoded_prefix[9 + username_length..13 + username_length]
            .try_into()
            .expect("connection type length is four bytes"),
    ) as usize;
    if connection_length > MAX_INIT_FIELD_BYTES {
        return Err(ClientError::FrameTooLarge {
            length: connection_length,
            max: MAX_INIT_FIELD_BYTES,
        });
    }

    let expected_length = connection_length_offset
        .checked_add(connection_length)
        .and_then(|length| length.checked_add(4))
        .expect("bounded init field lengths fit in usize");
    if length < expected_length {
        return read_obfuscated_init_frame_from_prefix(reader, prefix, length, max_len).await;
    }

    read_obfuscated_init_frame_from_prefix(reader, prefix, length, max_len).await
}

async fn read_obfuscated_init_frame_from_prefix<R>(
    reader: &mut R,
    mut prefix: Vec<u8>,
    length: usize,
    max_len: usize,
) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let encoded_len = prefixed_frame_len(length, 8, max_len)?;
    let start = prefix.len();
    prefix.resize(encoded_len, 0);
    reader.read_exact(&mut prefix[start..]).await?;
    let decoded = decode_rotated(&prefix)?;
    Ok(InitFrame::decode(&decoded)?)
}

async fn read_len_prefixed_frame<R>(reader: &mut R, max_len: usize) -> Result<Vec<u8>, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    let length = reader.read_u32_le().await? as usize;
    if length > max_len {
        return Err(ClientError::FrameTooLarge {
            length,
            max: max_len,
        });
    }

    let encoded_len = prefixed_frame_len(length, 4, max_len)?;
    let mut encoded = Vec::with_capacity(encoded_len);
    encoded.extend_from_slice(&(length as u32).to_le_bytes());
    encoded.resize(encoded_len, 0);
    reader.read_exact(&mut encoded[4..]).await?;
    Ok(encoded)
}

async fn read_obfuscated_len_prefixed_frame<R>(
    reader: &mut R,
    max_len: usize,
) -> Result<Vec<u8>, ClientError>
where
    R: AsyncRead + Unpin,
{
    let max_len = bounded_frame_len(max_len);
    let mut first_block = [0; 8];
    reader.read_exact(&mut first_block).await?;
    let decoded_first_block = decode_rotated(&first_block)?;
    let length = u32::from_le_bytes([
        decoded_first_block[0],
        decoded_first_block[1],
        decoded_first_block[2],
        decoded_first_block[3],
    ]) as usize;
    if length > max_len {
        return Err(ClientError::FrameTooLarge {
            length,
            max: max_len,
        });
    }

    let encoded_len = prefixed_frame_len(length, 8, max_len)?;
    let mut obfuscated = Vec::with_capacity(encoded_len);
    obfuscated.extend_from_slice(&first_block);
    obfuscated.resize(encoded_len, 0);
    reader.read_exact(&mut obfuscated[8..]).await?;
    Ok(decode_rotated(&obfuscated)?)
}

#[cfg(test)]
mod tests {
    use super::prefixed_frame_len;
    use crate::ClientError;

    #[test]
    fn framed_allocation_length_rejects_prefix_overflow() {
        assert!(matches!(
            prefixed_frame_len(usize::MAX, 4, usize::MAX),
            Err(ClientError::FrameTooLarge { length, max })
                if length == usize::MAX && max == usize::MAX - 4
        ));
        assert_eq!(prefixed_frame_len(16, 8, usize::MAX).unwrap(), 24);
    }
}
