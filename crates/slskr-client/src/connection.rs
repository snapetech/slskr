use crate::ClientError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionKind {
    PeerMessages,
    FileTransfer,
    Distributed,
}

impl ConnectionKind {
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        match self {
            Self::PeerMessages => b'P',
            Self::FileTransfer => b'F',
            Self::Distributed => b'D',
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PeerMessages => "P",
            Self::FileTransfer => "F",
            Self::Distributed => "D",
        }
    }

    pub fn try_from_connection_type(value: &str) -> Result<Self, ClientError> {
        match value.as_bytes() {
            [byte] => Self::try_from(*byte),
            _ => Err(ClientError::UnknownConnectionType(value.to_owned())),
        }
    }
}

impl TryFrom<u8> for ConnectionKind {
    type Error = ClientError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let kind = match value {
            b'P' => Self::PeerMessages,
            b'F' => Self::FileTransfer,
            b'D' => Self::Distributed,
            _ => return Err(ClientError::UnknownConnectionKind(value)),
        };
        Ok(kind)
    }
}
