use slskr_protocol::{frame::InitFrame, init::InitCode, InitMessage};

#[test]
fn init_codes_map_known_values() {
    assert_eq!(InitCode::try_from(0), Ok(InitCode::PierceFirewall));
    assert_eq!(InitCode::try_from(1), Ok(InitCode::PeerInit));
    assert_eq!(InitCode::try_from(2), Err(2));
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
