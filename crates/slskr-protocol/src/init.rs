use crate::{
    error::{DecodeError, EncodeError},
    frame::InitFrame,
    primitives::{Reader, Writer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InitCode {
    PierceFirewall = 0,
    PeerInit = 1,
}

impl InitCode {
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for InitCode {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::PierceFirewall),
            1 => Ok(Self::PeerInit),
            _ => Err(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitMessage {
    PierceFirewall {
        token: u32,
    },
    PeerInit {
        username: String,
        connection_type: String,
        token: u32,
    },
    Unknown {
        code: u8,
        payload: Vec<u8>,
    },
}

impl InitMessage {
    pub fn decode(frame: InitFrame) -> Result<Self, DecodeError> {
        let Ok(code) = InitCode::try_from(frame.code) else {
            return Ok(Self::Unknown {
                code: frame.code,
                payload: frame.payload,
            });
        };

        let mut reader = Reader::new(&frame.payload);
        let message = match code {
            InitCode::PierceFirewall => Self::PierceFirewall {
                token: reader.read_u32_le()?,
            },
            InitCode::PeerInit => Self::PeerInit {
                username: reader.read_string()?,
                connection_type: reader.read_string()?,
                token: reader.read_u32_le()?,
            },
        };
        reader.finish()?;
        Ok(message)
    }

    pub fn encode(&self) -> Result<InitFrame, EncodeError> {
        let mut writer = Writer::new();
        let code = match self {
            Self::PierceFirewall { token } => {
                writer.write_u32_le(*token);
                InitCode::PierceFirewall.as_u8()
            }
            Self::PeerInit {
                username,
                connection_type,
                token,
            } => {
                writer.write_string(username)?;
                writer.write_string(connection_type)?;
                writer.write_u32_le(*token);
                InitCode::PeerInit.as_u8()
            }
            Self::Unknown { code, payload } => return Ok(InitFrame::new(*code, payload.clone())),
        };

        Ok(InitFrame::new(code, writer.into_inner()))
    }
}
