use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ClientError;

pub const DEFAULT_MAX_TRANSFER_CHUNK_LEN: usize = 16 * 1024 * 1024;

#[derive(Debug)]
pub struct FileTransferConnection<S> {
    stream: S,
}

impl<S> FileTransferConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> FileTransferConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send_token(&mut self, token: u32) -> Result<(), ClientError> {
        self.stream.write_u32_le(token).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn receive_token(&mut self) -> Result<u32, ClientError> {
        Ok(self.stream.read_u32_le().await?)
    }

    pub async fn send_offset(&mut self, offset: u64) -> Result<(), ClientError> {
        self.stream.write_u64_le(offset).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn receive_offset(&mut self) -> Result<u64, ClientError> {
        Ok(self.stream.read_u64_le().await?)
    }

    pub async fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), ClientError> {
        self.stream.write_all(chunk).await?;
        self.stream.flush().await?;
        Ok(())
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
        let mut chunk = vec![0; length];
        self.stream.read_exact(&mut chunk).await?;
        Ok(chunk)
    }
}
