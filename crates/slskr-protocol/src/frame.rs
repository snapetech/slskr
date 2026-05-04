use crate::{
    error::{DecodeError, EncodeError},
    primitives::{Reader, Writer},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageFrame {
    pub code: u32,
    pub payload: Vec<u8>,
}

impl MessageFrame {
    #[must_use]
    pub fn new(code: u32, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            code,
            payload: payload.into(),
        }
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(input);
        let length = reader.read_u32_le()? as usize;
        if length < 4 {
            return Err(DecodeError::InvalidFrameLength {
                length,
                code_width: 4,
            });
        }
        if length > reader.remaining() {
            return Err(DecodeError::IncompleteFrame {
                length,
                remaining: reader.remaining(),
            });
        }

        let code = reader.read_u32_le()?;
        let payload = reader.read_bytes(length - 4)?.to_vec();
        reader.finish()?;
        Ok(Self { code, payload })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let length = self
            .payload
            .len()
            .checked_add(4)
            .ok_or_else(|| EncodeError::length_overflow("message frame", self.payload.len()))?;
        let length = u32::try_from(length)
            .map_err(|_| EncodeError::length_overflow("message frame", length))?;

        let mut writer = Writer::with_capacity(4 + length as usize);
        writer.write_u32_le(length);
        writer.write_u32_le(self.code);
        writer.write_bytes(&self.payload);
        Ok(writer.into_inner())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitFrame {
    pub code: u8,
    pub payload: Vec<u8>,
}

impl InitFrame {
    #[must_use]
    pub fn new(code: u8, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            code,
            payload: payload.into(),
        }
    }

    pub fn decode(input: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(input);
        let length = reader.read_u32_le()? as usize;
        if length < 1 {
            return Err(DecodeError::InvalidFrameLength {
                length,
                code_width: 1,
            });
        }
        if length > reader.remaining() {
            return Err(DecodeError::IncompleteFrame {
                length,
                remaining: reader.remaining(),
            });
        }

        let code = reader.read_u8()?;
        let payload = reader.read_bytes(length - 1)?.to_vec();
        reader.finish()?;
        Ok(Self { code, payload })
    }

    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let length = self
            .payload
            .len()
            .checked_add(1)
            .ok_or_else(|| EncodeError::length_overflow("init frame", self.payload.len()))?;
        let length = u32::try_from(length)
            .map_err(|_| EncodeError::length_overflow("init frame", length))?;

        let mut writer = Writer::with_capacity(4 + length as usize);
        writer.write_u32_le(length);
        writer.write_u8(self.code);
        writer.write_bytes(&self.payload);
        Ok(writer.into_inner())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFrame {
    pub payload: Vec<u8>,
}

impl RawFrame {
    #[must_use]
    pub fn new(payload: impl Into<Vec<u8>>) -> Self {
        Self {
            payload: payload.into(),
        }
    }

    #[must_use]
    pub fn decode(input: &[u8]) -> Self {
        Self {
            payload: input.to_vec(),
        }
    }

    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        self.payload.clone()
    }
}
