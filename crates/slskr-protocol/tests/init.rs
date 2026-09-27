use std::collections::HashSet;

use slskr_protocol::{
    error::DecodeError,
    frame::InitFrame,
    init::{InitCode, MAX_INIT_FIELD_BYTES},
    InitMessage,
};

#[test]
fn init_codes_map_known_values() {
    assert_eq!(InitCode::try_from(0), Ok(InitCode::PierceFirewall));
    assert_eq!(InitCode::try_from(1), Ok(InitCode::PeerInit));
    assert_eq!(InitCode::try_from(2), Err(2));
}

#[test]
fn init_code_inventory_is_complete_and_unique() {
    let mut seen = HashSet::new();

    for code in InitCode::ALL {
        assert!(
            seen.insert(code.as_u8()),
            "duplicate init code {}",
            code.as_u8()
        );
        assert_eq!(InitCode::try_from(code.as_u8()), Ok(*code));
    }

    assert_eq!(InitCode::ALL.len(), 2);
}

#[test]
fn pierce_firewall_round_trips() {
    let message = InitMessage::PierceFirewall { token: 123 };

    let decoded = InitMessage::decode(message.encode().unwrap()).unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn peer_init_round_trips() {
    let message = InitMessage::PeerInit {
        username: "local".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };

    let encoded = message.encode().unwrap().encode().unwrap();
    let decoded = InitMessage::decode(InitFrame::decode(&encoded).unwrap()).unwrap();
    assert_eq!(decoded, message);
}

#[test]
fn unknown_init_messages_preserve_payload() {
    let message = InitMessage::decode(InitFrame::new(9, [1, 2, 3])).unwrap();

    assert_eq!(
        message,
        InitMessage::Unknown {
            code: 9,
            payload: vec![1, 2, 3]
        }
    );
}

#[test]
fn peer_init_rejects_oversized_username_before_string_ownership() {
    let length = MAX_INIT_FIELD_BYTES + 1;
    let mut payload = (length as u32).to_le_bytes().to_vec();
    payload.extend(std::iter::repeat_n(b'x', length));

    assert!(matches!(
        InitMessage::decode(InitFrame::new(InitCode::PeerInit.as_u8(), payload)),
        Err(DecodeError::InvalidStringLength {
            length: actual,
            remaining: MAX_INIT_FIELD_BYTES,
        }) if actual == length
    ));
}

#[test]
fn peer_init_rejects_oversized_connection_type_before_string_ownership() {
    let mut payload = Vec::new();
    payload.extend_from_slice(&0_u32.to_le_bytes());
    let length = MAX_INIT_FIELD_BYTES + 1;
    payload.extend_from_slice(&(length as u32).to_le_bytes());
    payload.extend(std::iter::repeat_n(b'x', length));

    assert!(matches!(
        InitMessage::decode(InitFrame::new(InitCode::PeerInit.as_u8(), payload)),
        Err(DecodeError::InvalidStringLength {
            length: actual,
            remaining: MAX_INIT_FIELD_BYTES,
        }) if actual == length
    ));
}
