use std::collections::VecDeque;

use slskr_protocol::{decode_rotated, encode_rotated};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ClientError;

pub const DEFAULT_MAX_TRANSFER_CHUNK_LEN: usize = 16 * 1024 * 1024;
pub const MAX_OBFUSCATED_TRANSFER_FRAME_LEN: usize = 8 * 1024 * 1024;
const OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN: usize = 4;

#[derive(Debug)]
pub struct FileTransferConnection<S> {
    stream: S,
    obfuscated: bool,
    decoded: VecDeque<u8>,
}

impl<S> FileTransferConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self {
            stream,
            obfuscated: false,
            decoded: VecDeque::new(),
        }
    }

    #[must_use]
    pub const fn new_obfuscated(stream: S) -> Self {
        Self {
            stream,
            obfuscated: true,
            decoded: VecDeque::new(),
        }
    }

    #[must_use]
    pub const fn is_obfuscated(&self) -> bool {
        self.obfuscated
    }

    #[must_use]
    pub const fn max_write_chunk_len(&self) -> usize {
        if self.obfuscated {
            MAX_OBFUSCATED_TRANSFER_FRAME_LEN - OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN
        } else {
            DEFAULT_MAX_TRANSFER_CHUNK_LEN
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }

    pub fn into_parts(self) -> (S, bool) {
        (self.stream, self.obfuscated)
    }
}

impl<S> FileTransferConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send_token(&mut self, token: u32) -> Result<(), ClientError> {
        if self.obfuscated {
            self.write_obfuscated_payload(&token.to_le_bytes()).await
        } else {
            self.stream.write_u32_le(token).await?;
            self.stream.flush().await?;
            Ok(())
        }
    }

    pub async fn receive_token(&mut self) -> Result<u32, ClientError> {
        if self.obfuscated {
            let bytes = self
                .read_obfuscated_payload(4, DEFAULT_MAX_TRANSFER_CHUNK_LEN)
                .await?;
            Ok(u32::from_le_bytes(
                bytes.try_into().expect("fixed token length"),
            ))
        } else {
            Ok(self.stream.read_u32_le().await?)
        }
    }

    pub async fn send_offset(&mut self, offset: u64) -> Result<(), ClientError> {
        if self.obfuscated {
            self.write_obfuscated_payload(&offset.to_le_bytes()).await
        } else {
            self.stream.write_u64_le(offset).await?;
            self.stream.flush().await?;
            Ok(())
        }
    }

    pub async fn receive_offset(&mut self) -> Result<u64, ClientError> {
        if self.obfuscated {
            let bytes = self
                .read_obfuscated_payload(8, DEFAULT_MAX_TRANSFER_CHUNK_LEN)
                .await?;
            Ok(u64::from_le_bytes(
                bytes.try_into().expect("fixed offset length"),
            ))
        } else {
            Ok(self.stream.read_u64_le().await?)
        }
    }

    pub async fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ClientError> {
        let max_len = self.max_write_chunk_len();
        if chunk.len() > max_len {
            return Err(ClientError::FrameTooLarge {
                length: chunk.len(),
                max: max_len,
            });
        }
        if self.obfuscated {
            self.write_obfuscated_payload(chunk).await
        } else {
            self.stream.write_all(chunk).await?;
            self.stream.flush().await?;
            Ok(())
        }
    }

    pub async fn read_chunk(&mut self, length: usize) -> Result<Vec<u8>, ClientError> {
        self.read_chunk_with_max(length, DEFAULT_MAX_TRANSFER_CHUNK_LEN)
            .await
    }

    pub async fn read_chunk_with_max(
        &mut self,
        length: usize,
        max_len: usize,
    ) -> Result<Vec<u8>, ClientError> {
        if length > max_len {
            return Err(ClientError::FrameTooLarge {
                length,
                max: max_len,
            });
        }
        if self.obfuscated {
            return self.read_obfuscated_payload(length, max_len).await;
        }
        let mut chunk = vec![0; length];
        self.stream.read_exact(&mut chunk).await?;
        Ok(chunk)
    }

    async fn write_obfuscated_payload(&mut self, payload: &[u8]) -> Result<(), ClientError> {
        if payload.is_empty() {
            return Err(ClientError::FrameTooLarge {
                length: 0,
                max: MAX_OBFUSCATED_TRANSFER_FRAME_LEN - OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN,
            });
        }
        let max_payload = MAX_OBFUSCATED_TRANSFER_FRAME_LEN - OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN;
        if payload.len() > max_payload {
            return Err(ClientError::FrameTooLarge {
                length: payload.len(),
                max: max_payload,
            });
        }

        let mut frame = Vec::with_capacity(OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + payload.len());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);
        self.stream
            .write_all(&encode_rotated(&frame, rand::random()))
            .await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn read_obfuscated_payload(
        &mut self,
        length: usize,
        max_len: usize,
    ) -> Result<Vec<u8>, ClientError> {
        if length > max_len {
            return Err(ClientError::FrameTooLarge {
                length,
                max: max_len,
            });
        }
        while self.decoded.len() < length {
            self.read_next_obfuscated_frame().await?;
        }

        let mut payload = Vec::with_capacity(length);
        for _ in 0..length {
            payload.push(self.decoded.pop_front().expect("decoded length checked"));
        }
        Ok(payload)
    }

    async fn read_next_obfuscated_frame(&mut self) -> Result<(), ClientError> {
        let mut first_block = [0; 8];
        self.stream.read_exact(&mut first_block).await?;
        let decoded_first_block = decode_rotated(&first_block)?;
        let length = u32::from_le_bytes(
            decoded_first_block[..OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN]
                .try_into()
                .expect("fixed frame prefix length"),
        ) as usize;
        let max_payload = MAX_OBFUSCATED_TRANSFER_FRAME_LEN - OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN;
        if length == 0 || length > max_payload {
            return Err(ClientError::FrameTooLarge {
                length,
                max: max_payload,
            });
        }

        let mut encoded = Vec::with_capacity(first_block.len() + length);
        encoded.extend_from_slice(&first_block);
        encoded.resize(first_block.len() + length, 0);
        self.stream
            .read_exact(&mut encoded[first_block.len()..])
            .await?;
        let decoded = decode_rotated(&encoded)?;
        if decoded.len() != OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN + length {
            return Err(ClientError::FrameTooLarge {
                length: decoded.len(),
                max: MAX_OBFUSCATED_TRANSFER_FRAME_LEN,
            });
        }
        self.decoded
            .extend(&decoded[OBFUSCATED_TRANSFER_FRAME_PREFIX_LEN..]);
        Ok(())
    }
}
