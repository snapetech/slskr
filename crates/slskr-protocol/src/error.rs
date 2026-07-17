#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("unexpected end of input while reading {context}: needed {needed} bytes, remaining {remaining}")]
    UnexpectedEof {
        context: &'static str,
        needed: usize,
        remaining: usize,
    },
    #[error("invalid boolean value {0}")]
    InvalidBool(u8),
    #[error("string length {length} exceeds remaining bytes {remaining}")]
    InvalidStringLength { length: usize, remaining: usize },
    #[error("{field} count {count} exceeds maximum possible {maximum} for remaining bytes")]
    InvalidCount {
        field: &'static str,
        count: usize,
        maximum: usize,
    },
    #[error("frame length {length} is smaller than code width {code_width}")]
    InvalidFrameLength { length: usize, code_width: usize },
    #[error("frame length {length} exceeds remaining bytes {remaining}")]
    IncompleteFrame { length: usize, remaining: usize },
    #[error("{field} length mismatch: expected {expected}, actual {actual}")]
    InvalidVectorLength {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("trailing bytes after decode: {0}")]
    TrailingBytes(usize),
    #[error("invalid compressed payload for {0}")]
    InvalidCompressedPayload(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    #[error("{field} length {length} exceeds u32::MAX")]
    LengthOverflow { field: &'static str, length: usize },
    #[error("{field} count {count} exceeds maximum {maximum}")]
    CountTooLarge {
        field: &'static str,
        count: usize,
        maximum: usize,
    },
    #[error("string contains text that cannot be represented as {encoding}")]
    UnrepresentableString { encoding: &'static str },
}

impl EncodeError {
    pub(crate) fn length_overflow(field: &'static str, length: usize) -> Self {
        Self::LengthOverflow { field, length }
    }
}

pub(crate) fn unexpected_eof(
    context: &'static str,
    needed: usize,
    remaining: usize,
) -> DecodeError {
    DecodeError::UnexpectedEof {
        context,
        needed,
        remaining,
    }
}
