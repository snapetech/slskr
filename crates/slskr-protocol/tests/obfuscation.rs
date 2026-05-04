use proptest::prelude::*;
use slskr_protocol::{decode_rotated, encode_rotated, DecodeError};

#[test]
fn rotated_obfuscation_matches_public_vector() {
    let plain = hex_bytes("0800000079000000e8030000");
    let obfuscated = hex_bytes("1494ee4a2028dd952850ba2b4aa37457");

    assert_eq!(encode_rotated(&plain, 0x4aee_9414), obfuscated);
    assert_eq!(decode_rotated(&obfuscated).unwrap(), plain);
}

#[test]
fn rotated_obfuscation_handles_partial_final_blocks() {
    let plain = [1, 2, 3, 4, 5, 6, 7];
    let obfuscated = encode_rotated(&plain, 0x1020_3040);

    assert_eq!(decode_rotated(&obfuscated).unwrap(), plain);
}

#[test]
fn rotated_decode_rejects_missing_key() {
    assert!(matches!(
        decode_rotated(&[1, 2, 3]),
        Err(DecodeError::UnexpectedEof {
            context: "obfuscation key",
            needed: 4,
            remaining: 3
        })
    ));
}

proptest! {
    #[test]
    fn rotated_obfuscation_round_trips(input in proptest::collection::vec(any::<u8>(), 0..4096), key in any::<u32>()) {
        let encoded = encode_rotated(&input, key);
        prop_assert_eq!(decode_rotated(&encoded)?, input);
    }
}

fn hex_bytes(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let value = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(value, 16).unwrap()
        })
        .collect()
}
