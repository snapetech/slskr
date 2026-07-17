use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{connection::ConnectionKind, ClientError};
use slskr_protocol::{
    decode_rotated, encode_rotated,
    frame::{InitFrame, MessageFrame},
    RawFrame,
};

pub const DEFAULT_MAX_FRAME_LEN: usize = 16 * 1024 * 1024;

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
    let encoded = read_len_prefixed_frame(reader, max_len).await?;
    Ok(MessageFrame::decode(&encoded)?)
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
    let encoded = read_len_prefixed_frame(reader, max_len).await?;
    Ok(InitFrame::decode(&encoded)?)
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
    let mut length_bytes = [first_len_byte, 0, 0, 0];
    reader.read_exact(&mut length_bytes[1..]).await?;
    let length = u32::from_le_bytes(length_bytes) as usize;
    if length > max_len {
        return Err(ClientError::FrameTooLarge {
            length,
            max: max_len,
        });
    }
    let encoded_len = prefixed_frame_len(length, 4, max_len)?;

    let mut encoded = Vec::with_capacity(encoded_len);
    encoded.extend_from_slice(&length_bytes);
    encoded.resize(encoded_len, 0);
    reader.read_exact(&mut encoded[4..]).await?;
    Ok(InitFrame::decode(&encoded)?)
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
    validate_frame_len(frame.payload.len().saturating_add(1), max_len)?;
    writer.write_all(&frame.encode()?).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn read_obfuscated_init_frame<R>(reader: &mut R) -> Result<InitFrame, ClientError>
where
    R: AsyncRead + Unpin,
{
    let encoded = read_obfuscated_len_prefixed_frame(reader, DEFAULT_MAX_FRAME_LEN).await?;
    Ok(InitFrame::decode(&encoded)?)
}

pub async fn write_obfuscated_init_frame<W>(
    writer: &mut W,
    frame: &InitFrame,
) -> Result<(), ClientError>
where
    W: AsyncWrite + Unpin,
{
    write_obfuscated_init_frame_with_key(writer, frame, rand::random()).await
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

async fn read_len_prefixed_frame<R>(reader: &mut R, max_len: usize) -> Result<Vec<u8>, ClientError>
where
    R: AsyncRead + Unpin,
{
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
