use std::net::Ipv4Addr;

use encoding_rs::WINDOWS_1252;

use crate::error::{unexpected_eof, DecodeError, EncodeError};

#[derive(Debug, Clone, Copy)]
pub struct Reader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub const fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.input.len() - self.offset
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn finish(self) -> Result<(), DecodeError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes(self.remaining()))
        }
    }

    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.read_exact("u8", 1)?[0])
    }

    pub fn read_bool(&mut self) -> Result<bool, DecodeError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(DecodeError::InvalidBool(value)),
        }
    }

    pub fn read_u32_le(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.read_exact("u32", 4)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().expect("slice length is fixed"),
        ))
    }

    pub fn read_u16_le(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.read_exact("u16", 2)?;
        Ok(u16::from_le_bytes(
            bytes.try_into().expect("slice length is fixed"),
        ))
    }

    pub fn read_u64_le(&mut self) -> Result<u64, DecodeError> {
        let bytes = self.read_exact("u64", 8)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().expect("slice length is fixed"),
        ))
    }

    pub fn read_ipv4(&mut self) -> Result<Ipv4Addr, DecodeError> {
        let bytes = self.read_exact("ipv4", 4)?;
        Ok(Ipv4Addr::new(bytes[3], bytes[2], bytes[1], bytes[0]))
    }

    pub fn read_string(&mut self) -> Result<String, DecodeError> {
        let length = self.read_u32_le()? as usize;
        if length > self.remaining() {
            return Err(DecodeError::InvalidStringLength {
                length,
                remaining: self.remaining(),
            });
        }

        let bytes = self.read_exact("string", length)?;
        match std::str::from_utf8(bytes) {
            Ok(value) => Ok(value.to_owned()),
            Err(_) => {
                let (decoded, _, _) = WINDOWS_1252.decode(bytes);
                Ok(decoded.into_owned())
            }
        }
    }

    pub fn read_len_prefixed_bytes(&mut self) -> Result<Vec<u8>, DecodeError> {
        let length = self.read_u32_le()? as usize;
        Ok(self.read_bytes(length)?.to_vec())
    }

    pub fn read_bytes(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
        self.read_exact("bytes", length)
    }

    fn read_exact(
        &mut self,
        context: &'static str,
        length: usize,
    ) -> Result<&'a [u8], DecodeError> {
        if length > self.remaining() {
            return Err(unexpected_eof(context, length, self.remaining()));
        }

        let start = self.offset;
        self.offset += length;
        Ok(&self.input[start..self.offset])
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Writer {
    output: Vec<u8>,
}

impl Writer {
    #[must_use]
    pub const fn new() -> Self {
        Self { output: Vec::new() }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: Vec::with_capacity(capacity),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.output.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.output.is_empty()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.output
    }

    #[must_use]
    pub fn into_inner(self) -> Vec<u8> {
        self.output
    }

    pub fn write_u8(&mut self, value: u8) {
        self.output.push(value);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    pub fn write_u32_le(&mut self, value: u32) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u16_le(&mut self, value: u16) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u64_le(&mut self, value: u64) {
        self.output.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_ipv4(&mut self, value: Ipv4Addr) {
        let mut octets = value.octets();
        octets.reverse();
        self.output.extend_from_slice(&octets);
    }

    pub fn write_string(&mut self, value: &str) -> Result<(), EncodeError> {
        self.write_len_prefixed_bytes("string", value.as_bytes())
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.output.extend_from_slice(value);
    }

    pub fn write_len_prefixed_bytes(
        &mut self,
        field: &'static str,
        value: &[u8],
    ) -> Result<(), EncodeError> {
        let length = u32::try_from(value.len())
            .map_err(|_| EncodeError::length_overflow(field, value.len()))?;
        self.write_u32_le(length);
        self.write_bytes(value);
        Ok(())
    }
}
