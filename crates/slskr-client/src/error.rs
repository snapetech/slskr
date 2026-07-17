use std::io;

use slskr_protocol::{server::ServerMessage, DecodeError, EncodeError};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),
    #[error("encode error: {0}")]
    Encode(#[from] EncodeError),
    #[error("unknown connection kind byte {0}")]
    UnknownConnectionKind(u8),
    #[error("unknown connection type {0}")]
    UnknownConnectionType(String),
    #[error("indirect connection token mismatch: expected {expected}, received {received}")]
    IndirectTokenMismatch { expected: u32, received: u32 },
    #[error("indirect connection username mismatch: expected {expected}, received {received}")]
    IndirectUsernameMismatch { expected: String, received: String },
    #[error("indirect connection kind mismatch: expected {expected:?}, received {received:?}")]
    IndirectKindMismatch {
        expected: crate::connection::ConnectionKind,
        received: crate::connection::ConnectionKind,
    },
    #[error("indirect connection requires a token-bearing initialization message")]
    IndirectInitRequired,
    #[error("transfer token mismatch: expected {expected}, received {received}")]
    TransferTokenMismatch { expected: u32, received: u32 },
    #[error("invalid transfer direction: expected {expected}, received {received}")]
    TransferDirectionMismatch { expected: u32, received: u32 },
    #[error("transfer filename mismatch: expected {expected}, received {received}")]
    TransferFilenameMismatch { expected: String, received: String },
    #[error("transfer offset {offset} exceeds file size {size}")]
    TransferOffsetOutOfRange { offset: u64, size: u64 },
    #[error("transfer payload size {actual} does not match advertised file size {expected}")]
    TransferSizeMismatch { expected: u64, actual: u64 },
    #[error("transfer rejection reason is invalid or exceeds maximum {max} bytes")]
    InvalidTransferReason { max: usize },
    #[error("cannot {operation} while transfer is in {state} state")]
    InvalidTransferState {
        operation: &'static str,
        state: &'static str,
    },
    #[error("unexpected transfer message: {0:?}")]
    UnexpectedTransferMessage(Box<slskr_protocol::peer::PeerMessage>),
    #[error("unexpected search message: {0:?}")]
    UnexpectedSearchMessage(Box<slskr_protocol::peer::PeerMessage>),
    #[error("search {field} length {length} exceeds maximum {max}")]
    SearchFieldTooLong {
        field: &'static str,
        length: usize,
        max: usize,
    },
    #[error("search {field} must not be blank")]
    BlankSearchField { field: &'static str },
    #[error("search {field} contains control characters")]
    InvalidSearchField { field: &'static str },
    #[error("private message recipient list must not be empty")]
    EmptyMessageRecipients,
    #[error("private message recipient must not be blank")]
    BlankMessageRecipient,
    #[error("private message recipient count {count} exceeds maximum {max}")]
    TooManyMessageRecipients { count: usize, max: usize },
    #[error("private message recipient candidate count exceeds maximum {max}")]
    TooManyMessageRecipientCandidates { max: usize },
    #[error("private message {field} length {length} exceeds maximum {max}")]
    PrivateMessageFieldTooLong {
        field: &'static str,
        length: usize,
        max: usize,
    },
    #[error("social {field} must not be blank")]
    BlankSocialField { field: &'static str },
    #[error("social {field} contains control characters")]
    InvalidSocialField { field: &'static str },
    #[error("social {field} length {length} exceeds maximum {max}")]
    SocialFieldTooLong {
        field: &'static str,
        length: usize,
        max: usize,
    },
    #[error("{field} interval must be positive")]
    InvalidInterval { field: &'static str },
    #[error("capability exchange failed: {0}")]
    CapabilityExchange(String),
    #[error("login rejected: {reason}{detail}")]
    LoginRejected { reason: String, detail: String },
    #[error("unexpected server message: {0:?}")]
    UnexpectedServerMessage(Box<ServerMessage>),
    #[error("unexpected init message code {code} with {payload_len} payload bytes")]
    UnexpectedInitMessage {
        code: u8,
        payload: Vec<u8>,
        payload_len: usize,
    },
    #[error("frame length {length} exceeds configured maximum {max}")]
    FrameTooLarge { length: usize, max: usize },
    #[error("decompressed payload length exceeds configured maximum {max}")]
    PayloadTooLarge { max: usize },
    #[error("compressed payload has {remaining} trailing bytes")]
    TrailingCompressedData { remaining: usize },
    #[error("{operation} timed out")]
    TimedOut { operation: &'static str },
    #[error("peer connection cache is full (maximum {max} connections)")]
    PeerConnectionCacheFull { max: usize },
    #[error("peer username must not be blank")]
    BlankPeerUsername,
    #[error("peer username contains control characters")]
    InvalidPeerUsername,
    #[error("peer username length {length} exceeds maximum {max}")]
    PeerUsernameTooLong { length: usize, max: usize },
    #[error("distributed child capacity is full (maximum {max} connections)")]
    DistributedChildCapacityFull { max: usize },
    #[error("distributed username must not be blank")]
    BlankDistributedUsername,
    #[error("distributed username contains control characters")]
    InvalidDistributedUsername,
    #[error("distributed username length {length} exceeds maximum {max}")]
    DistributedUsernameTooLong { length: usize, max: usize },
    #[error("distributed connection to {username} would create an identity loop")]
    DistributedIdentityLoop { username: String },
}

impl ClientError {
    pub(crate) fn unexpected_server_message(message: ServerMessage) -> Self {
        Self::UnexpectedServerMessage(Box::new(message))
    }
}
