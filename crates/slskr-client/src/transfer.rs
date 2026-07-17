use slskr_protocol::peer::{PeerMessage, TransferRequest, TransferResponse};
use slskr_protocol::ProtocolTextEncoding;

use crate::{file_transfer::FileTransferConnection, ClientError};
use tokio::io::{AsyncRead, AsyncWrite};

const UPLOAD_DIRECTION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadTransfer {
    pub username: String,
    pub filename: String,
    pub token: u32,
    pub state: DownloadState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadState {
    New,
    Queued,
    PlaceInQueue(u32),
    Requested { size: Option<u64> },
    Accepted { size: Option<u64> },
    Rejected { reason: String },
    Failed { reason: String },
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadTransfer {
    pub username: String,
    pub filename: String,
    pub token: u32,
    pub size: u64,
    pub state: UploadState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadState {
    New,
    Requested,
    Accepted,
    Rejected { reason: String },
    Failed { reason: String },
    Completed,
}

impl DownloadTransfer {
    #[must_use]
    pub fn new(username: impl Into<String>, filename: impl Into<String>, token: u32) -> Self {
        Self {
            username: username.into(),
            filename: filename.into(),
            token,
            state: DownloadState::New,
        }
    }

    pub fn queue_upload_message(&mut self) -> PeerMessage {
        self.state = DownloadState::Queued;
        PeerMessage::QueueUpload {
            filename: self.filename.clone(),
        }
    }

    pub fn place_in_queue_request_message(&self) -> PeerMessage {
        PeerMessage::PlaceInQueueRequest {
            filename: self.filename.clone(),
        }
    }

    pub fn handle_peer_message(&mut self, message: PeerMessage) -> Result<(), ClientError> {
        if self.state.is_terminal() {
            return Err(ClientError::InvalidTransferState {
                operation: "handle peer message",
                state: self.state.name(),
            });
        }
        match message {
            PeerMessage::PlaceInQueueResponse { filename, place } => {
                self.validate_filename(filename)?;
                self.state = DownloadState::PlaceInQueue(place);
                Ok(())
            }
            PeerMessage::TransferRequest(request) => self.handle_transfer_request(request),
            PeerMessage::TransferResponse(response) => self.handle_transfer_response(response),
            PeerMessage::UploadFailed { filename } => {
                self.validate_filename(filename)?;
                self.state = DownloadState::Failed {
                    reason: "upload failed".to_owned(),
                };
                Ok(())
            }
            PeerMessage::UploadDenied { filename, reason } => {
                self.validate_filename(filename)?;
                self.state = DownloadState::Rejected { reason };
                Ok(())
            }
            message => Err(ClientError::UnexpectedTransferMessage(Box::new(message))),
        }
    }

    pub fn accept_upload_response_message(&mut self, size: u64) -> PeerMessage {
        self.state = DownloadState::Accepted { size: Some(size) };
        PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: self.token,
            size: Some(size),
        })
    }

    pub fn reject_upload_response_message(&mut self, reason: impl Into<String>) -> PeerMessage {
        let reason = reason.into();
        self.state = DownloadState::Rejected {
            reason: reason.clone(),
        };
        PeerMessage::TransferResponse(TransferResponse::Rejected {
            token: self.token,
            reason,
        })
    }

    pub async fn receive_file_from<S>(
        &mut self,
        connection: &mut FileTransferConnection<S>,
        offset: u64,
        remaining: usize,
    ) -> Result<Vec<u8>, ClientError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        if !matches!(self.state, DownloadState::Accepted { .. }) {
            return Err(ClientError::InvalidTransferState {
                operation: "receive file",
                state: self.state.name(),
            });
        }
        if let Some(expected) = self.expected_size() {
            let remaining = u64::try_from(remaining).unwrap_or(u64::MAX);
            let actual =
                offset
                    .checked_add(remaining)
                    .ok_or(ClientError::TransferOffsetOutOfRange {
                        offset,
                        size: expected,
                    })?;
            if actual != expected {
                return Err(ClientError::TransferSizeMismatch { expected, actual });
            }
        }

        let token = connection.receive_token().await?;
        self.validate_token(token)?;
        connection.send_offset(offset).await?;
        let bytes = connection.read_chunk(remaining).await?;
        self.state = DownloadState::Completed;
        Ok(bytes)
    }

    fn handle_transfer_request(&mut self, request: TransferRequest) -> Result<(), ClientError> {
        if request.direction != UPLOAD_DIRECTION {
            return Err(ClientError::TransferDirectionMismatch {
                expected: UPLOAD_DIRECTION,
                received: request.direction,
            });
        }
        self.validate_token(request.token)?;
        self.validate_filename(request.filename)?;
        self.state = DownloadState::Requested { size: request.size };
        Ok(())
    }

    fn handle_transfer_response(&mut self, response: TransferResponse) -> Result<(), ClientError> {
        match response {
            TransferResponse::Allowed { token, size } => {
                self.validate_token(token)?;
                self.state = DownloadState::Accepted { size };
                Ok(())
            }
            TransferResponse::Rejected { token, reason } => {
                self.validate_token(token)?;
                self.state = DownloadState::Rejected { reason };
                Ok(())
            }
        }
    }

    fn validate_token(&self, received: u32) -> Result<(), ClientError> {
        if received == self.token {
            Ok(())
        } else {
            Err(ClientError::TransferTokenMismatch {
                expected: self.token,
                received,
            })
        }
    }

    fn expected_size(&self) -> Option<u64> {
        match &self.state {
            DownloadState::Requested { size } | DownloadState::Accepted { size } => *size,
            _ => None,
        }
    }

    fn validate_filename(&self, received: String) -> Result<(), ClientError> {
        if received == self.filename {
            Ok(())
        } else {
            Err(ClientError::TransferFilenameMismatch {
                expected: self.filename.clone(),
                received,
            })
        }
    }
}

impl DownloadState {
    const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Rejected { .. } | Self::Failed { .. } | Self::Completed
        )
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Queued => "queued",
            Self::PlaceInQueue(_) => "queued",
            Self::Requested { .. } => "requested",
            Self::Accepted { .. } => "accepted",
            Self::Rejected { .. } => "rejected",
            Self::Failed { .. } => "failed",
            Self::Completed => "completed",
        }
    }
}

impl UploadTransfer {
    #[must_use]
    pub fn new(
        username: impl Into<String>,
        filename: impl Into<String>,
        token: u32,
        size: u64,
    ) -> Self {
        Self {
            username: username.into(),
            filename: filename.into(),
            token,
            size,
            state: UploadState::New,
        }
    }

    pub fn transfer_request_message(&mut self) -> PeerMessage {
        self.state = UploadState::Requested;
        PeerMessage::TransferRequest(TransferRequest {
            direction: UPLOAD_DIRECTION,
            token: self.token,
            filename: self.filename.clone(),
            filename_encoding: ProtocolTextEncoding::Utf8,
            size: Some(self.size),
        })
    }

    pub fn handle_peer_message(&mut self, message: PeerMessage) -> Result<(), ClientError> {
        if self.state.is_terminal() {
            return Err(ClientError::InvalidTransferState {
                operation: "handle peer message",
                state: self.state.name(),
            });
        }
        match message {
            PeerMessage::TransferResponse(TransferResponse::Allowed { token, .. }) => {
                self.validate_token(token)?;
                self.state = UploadState::Accepted;
                Ok(())
            }
            PeerMessage::TransferResponse(TransferResponse::Rejected { token, reason }) => {
                self.validate_token(token)?;
                self.state = UploadState::Rejected { reason };
                Ok(())
            }
            PeerMessage::UploadFailed { filename } => {
                self.validate_filename(filename)?;
                self.state = UploadState::Failed {
                    reason: "upload failed".to_owned(),
                };
                Ok(())
            }
            message => Err(ClientError::UnexpectedTransferMessage(Box::new(message))),
        }
    }

    pub async fn send_file_to<S>(
        &mut self,
        connection: &mut FileTransferConnection<S>,
        bytes: &[u8],
    ) -> Result<u64, ClientError>
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        if !matches!(self.state, UploadState::Accepted) {
            return Err(ClientError::InvalidTransferState {
                operation: "send file",
                state: self.state.name(),
            });
        }
        let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if actual != self.size {
            return Err(ClientError::TransferSizeMismatch {
                expected: self.size,
                actual,
            });
        }

        connection.send_token(self.token).await?;
        let offset = connection.receive_offset().await?;
        let start = usize::try_from(offset).map_err(|_| ClientError::TransferOffsetOutOfRange {
            offset,
            size: bytes.len() as u64,
        })?;
        if start > bytes.len() {
            return Err(ClientError::TransferOffsetOutOfRange {
                offset,
                size: bytes.len() as u64,
            });
        }

        connection.write_chunk(&bytes[start..]).await?;
        self.state = UploadState::Completed;
        Ok(offset)
    }

    fn validate_token(&self, received: u32) -> Result<(), ClientError> {
        if received == self.token {
            Ok(())
        } else {
            Err(ClientError::TransferTokenMismatch {
                expected: self.token,
                received,
            })
        }
    }

    fn validate_filename(&self, received: String) -> Result<(), ClientError> {
        if received == self.filename {
            Ok(())
        } else {
            Err(ClientError::TransferFilenameMismatch {
                expected: self.filename.clone(),
                received,
            })
        }
    }
}

impl UploadState {
    const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Rejected { .. } | Self::Failed { .. } | Self::Completed
        )
    }

    const fn name(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Requested => "requested",
            Self::Accepted => "accepted",
            Self::Rejected { .. } => "rejected",
            Self::Failed { .. } => "failed",
            Self::Completed => "completed",
        }
    }
}
