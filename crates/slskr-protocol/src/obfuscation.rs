use crate::{error::unexpected_eof, DecodeError};

pub const ROTATED_OBFUSCATION_TYPE: u32 = 1;

pub fn encode_rotated(input: &[u8], key: u32) -> Vec<u8> {
    let mut output = Vec::with_capacity(4 + input.len());
    output.extend_from_slice(&key.to_le_bytes());
    output.extend_from_slice(input);
    apply_rotated_keystream(&mut output[4..], key);
    output
}

pub fn decode_rotated(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
    if input.len() < 4 {
        return Err(unexpected_eof("obfuscation key", 4, input.len()));
    }

    let key = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    let mut output = input[4..].to_vec();
    apply_rotated_keystream(&mut output, key);
    Ok(output)
}

fn apply_rotated_keystream(buffer: &mut [u8], initial_key: u32) {
    let mut key = initial_key;
    for chunk in buffer.chunks_mut(4) {
        key = key.rotate_left(1);
        let key_bytes = key.to_le_bytes();
        for (byte, key_byte) in chunk.iter_mut().zip(key_bytes) {
            *byte ^= key_byte;
        }
    }
}
