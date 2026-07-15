use std::net::Ipv4Addr;

use proptest::prelude::*;
use slskr_protocol::{DecodeError, EncodeError, ProtocolTextEncoding, Reader, Writer};

proptest! {
    #[test]
    fn u8_round_trips(value in any::<u8>()) {
        let mut writer = Writer::new();
        writer.write_u8(value);

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_u8()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn bool_round_trips(value in any::<bool>()) {
        let mut writer = Writer::new();
        writer.write_bool(value);

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_bool()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn u32_round_trips(value in any::<u32>()) {
        let mut writer = Writer::new();
        writer.write_u32_le(value);

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_u32_le()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn u16_round_trips(value in any::<u16>()) {
        let mut writer = Writer::new();
        writer.write_u16_le(value);

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_u16_le()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn u64_round_trips(value in any::<u64>()) {
        let mut writer = Writer::new();
        writer.write_u64_le(value);

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_u64_le()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn ipv4_round_trips(octets in any::<[u8; 4]>()) {
        let value = Ipv4Addr::from(octets);
        let mut writer = Writer::new();
        writer.write_ipv4(value);

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_ipv4()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn utf8_strings_round_trip(value in ".*") {
        let mut writer = Writer::new();
        writer.write_string(&value)?;

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_string()?, value);
        prop_assert!(reader.finish().is_ok());
    }

    #[test]
    fn length_prefixed_bytes_round_trip(value in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let mut writer = Writer::new();
        writer.write_len_prefixed_bytes("bytes", &value)?;

        let mut reader = Reader::new(writer.as_slice());
        prop_assert_eq!(reader.read_len_prefixed_bytes()?, value);
        prop_assert!(reader.finish().is_ok());
    }
}

#[test]
fn ipv4_uses_soulseek_reversed_wire_order() {
    let mut reader = Reader::new(&[205, 185, 127, 79]);
    assert_eq!(
        reader.read_ipv4().unwrap(),
        Ipv4Addr::new(79, 127, 185, 205)
    );
    assert!(reader.finish().is_ok());

    let mut writer = Writer::new();
    writer.write_ipv4(Ipv4Addr::new(79, 127, 185, 205));
    assert_eq!(writer.as_slice(), &[205, 185, 127, 79]);
}

#[test]
fn strings_fall_back_to_latin1() {
    let input = [1, 0, 0, 0, 0xE9];
    let mut reader = Reader::new(&input);

    assert_eq!(reader.read_string().unwrap(), "é");
    assert!(reader.finish().is_ok());
}

#[test]
fn windows_1251_strings_preserve_detected_encoding_and_wire_bytes() {
    let encoded = [0xCC, 0xF3, 0xE7, 0xFB, 0xEA, 0xE0];
    let mut input = (encoded.len() as u32).to_le_bytes().to_vec();
    input.extend_from_slice(&encoded);
    let mut reader = Reader::new(&input);

    let (value, encoding) = reader.read_string_with_encoding().unwrap();
    assert_eq!(value, "Музыка");
    assert_eq!(encoding, ProtocolTextEncoding::Windows1251);

    let mut writer = Writer::new();
    writer.write_string_with_encoding(&value, encoding).unwrap();
    assert_eq!(writer.as_slice(), input);
}

#[test]
fn latin1_strings_preserve_wire_bytes() {
    let input = [4, 0, 0, 0, b'c', b'a', b'f', 0xE9];
    let mut reader = Reader::new(&input);
    let (value, encoding) = reader.read_string_with_encoding().unwrap();
    assert_eq!(value, "café");
    assert_eq!(encoding, ProtocolTextEncoding::Latin1);

    let mut writer = Writer::new();
    writer.write_string_with_encoding(&value, encoding).unwrap();
    assert_eq!(writer.as_slice(), input);
}

#[test]
fn legacy_encoding_rejects_unrepresentable_text() {
    let mut writer = Writer::new();
    assert!(matches!(
        writer.write_string_with_encoding("snowman ☃", ProtocolTextEncoding::Latin1),
        Err(EncodeError::UnrepresentableString {
            encoding: "ISO-8859-1"
        })
    ));
}

#[test]
fn invalid_bool_is_rejected() {
    let mut reader = Reader::new(&[2]);

    assert_eq!(reader.read_bool(), Err(DecodeError::InvalidBool(2)));
}

#[test]
fn reader_reports_trailing_bytes() {
    let mut reader = Reader::new(&[1, 2]);

    assert_eq!(reader.read_u8().unwrap(), 1);
    assert_eq!(reader.finish(), Err(DecodeError::TrailingBytes(1)));
}
