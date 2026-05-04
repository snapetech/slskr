use crate::{
    error::{DecodeError, EncodeError},
    frame::{InitFrame, MessageFrame},
    primitives::{Reader, Writer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DistributedCode {
    Ping = 0,
    Search = 3,
    BranchLevel = 4,
    BranchRoot = 5,
    ChildDepth = 7,
    EmbeddedMessage = 93,
}

impl DistributedCode {
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for DistributedCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let code = match value {
            0 => Self::Ping,
            3 => Self::Search,
            4 => Self::BranchLevel,
            5 => Self::BranchRoot,
            7 => Self::ChildDepth,
            93 => Self::EmbeddedMessage,
            _ => return Err(value),
        };
        Ok(code)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedSearch {
    pub identifier: u32,
    pub username: String,
    pub token: u32,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributedMessage {
    Ping,
    Search(DistributedSearch),
    BranchLevel { level: u32 },
    BranchRoot { username: String },
    ChildDepth { depth: u32 },
    EmbeddedMessage { code: u8, payload: Vec<u8> },
    EmbeddedServerMessage(MessageFrame),
    Unknown { code: u8, payload: Vec<u8> },
}

impl DistributedMessage {
    pub fn decode(frame: InitFrame) -> Result<Self, DecodeError> {
        let Ok(code) = DistributedCode::try_from(frame.code) else {
            return Ok(Self::Unknown {
                code: frame.code,
                payload: frame.payload,
            });
        };

        let mut reader = Reader::new(&frame.payload);
        let message = match code {
            DistributedCode::Ping => Self::Ping,
            DistributedCode::Search => Self::Search(DistributedSearch {
                identifier: reader.read_u32_le()?,
                username: reader.read_string()?,
                token: reader.read_u32_le()?,
                query: reader.read_string()?,
            }),
            DistributedCode::BranchLevel => Self::BranchLevel {
                level: reader.read_u32_le()?,
            },
            DistributedCode::BranchRoot => Self::BranchRoot {
                username: reader.read_string()?,
            },
            DistributedCode::ChildDepth => Self::ChildDepth {
                depth: reader.read_u32_le()?,
            },
            DistributedCode::EmbeddedMessage => Self::EmbeddedMessage {
                code: reader.read_u8()?,
                payload: reader.read_len_prefixed_bytes()?,
            },
        };
        reader.finish()?;
        Ok(message)
    }

    pub fn decode_embedded_server(frame: MessageFrame) -> Self {
        Self::EmbeddedServerMessage(frame)
    }

    pub fn encode(&self) -> Result<InitFrame, EncodeError> {
        let mut writer = Writer::new();
        let code = match self {
            Self::Ping => DistributedCode::Ping.as_u8(),
            Self::Search(value) => {
                writer.write_u32_le(value.identifier);
                writer.write_string(&value.username)?;
                writer.write_u32_le(value.token);
                writer.write_string(&value.query)?;
                DistributedCode::Search.as_u8()
            }
            Self::BranchLevel { level } => {
                writer.write_u32_le(*level);
                DistributedCode::BranchLevel.as_u8()
            }
            Self::BranchRoot { username } => {
                writer.write_string(username)?;
                DistributedCode::BranchRoot.as_u8()
            }
            Self::ChildDepth { depth } => {
                writer.write_u32_le(*depth);
                DistributedCode::ChildDepth.as_u8()
            }
            Self::EmbeddedMessage { code, payload } => {
                writer.write_u8(*code);
                writer.write_len_prefixed_bytes("distributed embedded payload", payload)?;
                DistributedCode::EmbeddedMessage.as_u8()
            }
            Self::EmbeddedServerMessage(frame) => {
                return Ok(InitFrame::new(
                    DistributedCode::EmbeddedMessage.as_u8(),
                    frame.encode()?,
                ));
            }
            Self::Unknown { code, payload } => return Ok(InitFrame::new(*code, payload.clone())),
        };

        Ok(InitFrame::new(code, writer.into_inner()))
    }
}
