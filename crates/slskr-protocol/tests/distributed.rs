use std::collections::HashSet;

use slskr_protocol::{
    distributed::{DistributedCode, DistributedMessage, DistributedSearch},
    frame::{InitFrame, MessageFrame},
};

#[test]
fn distributed_codes_map_known_values() {
    assert_eq!(DistributedCode::try_from(0), Ok(DistributedCode::Ping));
    assert_eq!(
        DistributedCode::try_from(93),
        Ok(DistributedCode::EmbeddedMessage)
    );
    assert_eq!(DistributedCode::try_from(8), Err(8));
}

#[test]
fn distributed_code_inventory_is_complete_and_unique() {
    let mut seen = HashSet::new();

    for code in DistributedCode::ALL {
        assert!(
            seen.insert(code.as_u8()),
            "duplicate distributed code {}",
            code.as_u8()
        );
        assert_eq!(DistributedCode::try_from(code.as_u8()), Ok(*code));
    }

    assert_eq!(DistributedCode::ALL.len(), 6);
}

#[test]
fn distributed_core_messages_round_trip() {
    let messages = [
        DistributedMessage::Ping,
        DistributedMessage::Search(DistributedSearch {
            identifier: 49,
            username: "sender".to_owned(),
            token: 77,
            query: "search text".to_owned(),
        }),
        DistributedMessage::BranchLevel { level: 2 },
        DistributedMessage::BranchRoot {
            username: "root".to_owned(),
        },
        DistributedMessage::ChildDepth { depth: 4 },
        DistributedMessage::EmbeddedMessage {
            code: 3,
            payload: vec![1, 2, 3],
        },
    ];

    for message in messages {
        let decoded = DistributedMessage::decode(message.encode().unwrap()).unwrap();
        assert_eq!(decoded, message);
    }
}

#[test]
fn embedded_server_message_encodes_as_code_93_payload() {
    let server_frame = MessageFrame::new(26, [1, 2, 3]);
    let message = DistributedMessage::EmbeddedServerMessage(server_frame.clone());
    let frame = message.encode().unwrap();

    assert_eq!(frame.code, 93);
    assert_eq!(frame.payload, server_frame.encode().unwrap());
    assert_eq!(
        DistributedMessage::decode(frame).unwrap(),
        DistributedMessage::EmbeddedServerMessage(server_frame.clone())
    );
    assert_eq!(
        DistributedMessage::decode_embedded_server(server_frame.clone()),
        DistributedMessage::EmbeddedServerMessage(server_frame)
    );
}

#[test]
fn unknown_distributed_messages_preserve_payload() {
    let message = DistributedMessage::decode(InitFrame::new(9, [1, 2, 3])).unwrap();

    assert_eq!(
        message,
        DistributedMessage::Unknown {
            code: 9,
            payload: vec![1, 2, 3]
        }
    );
}
