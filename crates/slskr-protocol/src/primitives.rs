use std::net::Ipv4Addr;

use encoding_rs::WINDOWS_1251;

use crate::error::{unexpected_eof, DecodeError, EncodeError};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProtocolTextEncoding {
    #[default]
    Utf8,
    Windows1251,
    Latin1,
}

impl ProtocolTextEncoding {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Windows1251 => "windows-1251",
            Self::Latin1 => "ISO-8859-1",
        }
    }
}

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
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u16_le(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.read_exact("u16", 2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u64_le(&mut self) -> Result<u64, DecodeError> {
        let bytes = self.read_exact("u64", 8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_ipv4(&mut self) -> Result<Ipv4Addr, DecodeError> {
        let bytes = self.read_exact("ipv4", 4)?;
        Ok(Ipv4Addr::new(bytes[3], bytes[2], bytes[1], bytes[0]))
    }

    pub fn read_string(&mut self) -> Result<String, DecodeError> {
        self.read_string_with_encoding().map(|(value, _)| value)
    }

    pub fn read_string_with_encoding(
        &mut self,
    ) -> Result<(String, ProtocolTextEncoding), DecodeError> {
        let length = self.read_u32_le()? as usize;
        if length > self.remaining() {
            return Err(DecodeError::InvalidStringLength {
                length,
                remaining: self.remaining(),
            });
        }

        let bytes = self.read_exact("string", length)?;
        match std::str::from_utf8(bytes) {
            Ok(value) => Ok((value.to_owned(), ProtocolTextEncoding::Utf8)),
            Err(_) => {
                let (windows_1251, _, _) = WINDOWS_1251.decode(bytes);
                if looks_like_windows_1251(bytes, &windows_1251) {
                    Ok((windows_1251.into_owned(), ProtocolTextEncoding::Windows1251))
                } else {
                    Ok((
                        bytes.iter().map(|byte| char::from(*byte)).collect(),
                        ProtocolTextEncoding::Latin1,
                    ))
                }
            }
        }
    }

    pub fn read_len_prefixed_bytes(&mut self) -> Result<Vec<u8>, DecodeError> {
        let length = self.read_u32_le()? as usize;
        Ok(self.read_bytes(length)?.to_vec())
    }

    pub fn read_bounded_count(
        &mut self,
        field: &'static str,
        minimum_bytes_per_item: usize,
    ) -> Result<usize, DecodeError> {
        let count = self.read_u32_le()? as usize;
        let maximum = self
            .remaining()
            .checked_div(minimum_bytes_per_item)
            .unwrap_or_else(|| self.remaining());

        if count > maximum {
            return Err(DecodeError::InvalidCount {
                field,
                count,
                maximum,
            });
        }

        Ok(count)
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

    pub fn write_string_with_encoding(
        &mut self,
        value: &str,
        encoding: ProtocolTextEncoding,
    ) -> Result<(), EncodeError> {
        match encoding {
            ProtocolTextEncoding::Utf8 => self.write_string(value),
            ProtocolTextEncoding::Windows1251 => {
                let (encoded, _, had_errors) = WINDOWS_1251.encode(value);
                if had_errors {
                    return Err(EncodeError::UnrepresentableString {
                        encoding: encoding.as_str(),
                    });
                }
                self.write_len_prefixed_bytes("string", &encoded)
            }
            ProtocolTextEncoding::Latin1 => {
                let encoded = value
                    .chars()
                    .map(|character| {
                        u8::try_from(u32::from(character)).map_err(|_| {
                            EncodeError::UnrepresentableString {
                                encoding: encoding.as_str(),
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                self.write_len_prefixed_bytes("string", &encoded)
            }
        }
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

fn looks_like_windows_1251(bytes: &[u8], decoded: &str) -> bool {
    let high_byte_count = bytes.iter().filter(|byte| **byte >= 0x80).count();
    if high_byte_count < 4 {
        return false;
    }
    let cyrillic_count = decoded
        .chars()
        .filter(|character| ('\u{0400}'..='\u{04ff}').contains(character))
        .count();
    cyrillic_count >= 4 && cyrillic_count.saturating_mul(5) >= high_byte_count.saturating_mul(3)
}
