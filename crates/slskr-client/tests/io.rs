use slskr_client::{
    connection::ConnectionKind,
    io::{
        read_connection_kind, read_init_frame, read_message_frame_buffered,
        read_message_frame_with_max, read_obfuscated_init_frame, read_raw_frame,
        read_raw_frame_with_max, write_connection_kind, write_init_frame,
        write_init_frame_with_max, write_message_frame, write_message_frame_with_max,
        write_obfuscated_init_frame_with_key_and_max,
        write_obfuscated_message_frame_with_key_and_max, write_raw_frame, write_raw_frame_with_max,
        MAX_FRAME_LEN,
    },
    ClientError,
};
use slskr_protocol::{
    encode_rotated,
    init::{InitCode, MAX_INIT_FIELD_BYTES, MAX_PEER_INIT_FRAME_LEN},
    InitFrame, MessageFrame, RawFrame,
};
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn connection_kind_round_trips() {
    let (mut client, mut server) = duplex(16);

    write_connection_kind(&mut client, ConnectionKind::PeerMessages)
        .await
        .unwrap();
    assert_eq!(
        read_connection_kind(&mut server).await.unwrap(),
        ConnectionKind::PeerMessages
    );
}

#[tokio::test]
async fn oversized_raw_frame_is_rejected_before_payload_read() {
    let (_client, mut server) = duplex(64);

    let error = read_raw_frame_with_max(&mut server, 1024, 16)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge {
            length: 1024,
            max: 16
        }
    ));
}

#[tokio::test]
async fn message_frame_round_trips() {
    let (mut client, mut server) = duplex(64);
    let frame = MessageFrame::new(26, [1, 2, 3]);

    write_message_frame(&mut client, &frame).await.unwrap();
    assert_eq!(
        read_message_frame_with_max(&mut server, 1024)
            .await
            .unwrap(),
        frame
    );
}

#[tokio::test]
async fn buffered_message_frame_reports_clean_connection_close() {
    let (client, mut server) = duplex(64);
    drop(client);
    let mut buffer = Vec::new();

    let error = read_message_frame_buffered(&mut server, &mut buffer, 1024)
        .await
        .unwrap_err();
    assert!(matches!(error, ClientError::ConnectionClosed));
}

#[tokio::test]
async fn init_frame_round_trips() {
    let (mut client, mut server) = duplex(64);
    let frame = InitFrame::new(1, [1, 2, 3]);

    write_init_frame(&mut client, &frame).await.unwrap();
    assert_eq!(read_init_frame(&mut server).await.unwrap(), frame);
}

#[tokio::test]
async fn oversized_peer_init_frame_is_rejected_before_full_frame_allocation() {
    let (mut client, mut server) = duplex(64);
    client.write_u32_le(MAX_FRAME_LEN as u32).await.unwrap();
    client.write_u8(InitCode::PeerInit.as_u8()).await.unwrap();

    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        read_init_frame(&mut server),
    )
    .await
    .expect("oversized init should be rejected from its bounded header")
    .unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge {
            length: MAX_FRAME_LEN,
            max: MAX_PEER_INIT_FRAME_LEN,
        }
    ));
}

#[tokio::test]
async fn oversized_peer_init_field_is_rejected_before_frame_read() {
    let (mut client, mut server) = duplex(64);
    client
        .write_u32_le(MAX_PEER_INIT_FRAME_LEN as u32)
        .await
        .unwrap();
    client.write_u8(InitCode::PeerInit.as_u8()).await.unwrap();
    client
        .write_u32_le((MAX_INIT_FIELD_BYTES + 1) as u32)
        .await
        .unwrap();

    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        read_init_frame(&mut server),
    )
    .await
    .expect("oversized init field should be rejected from its prefix")
    .unwrap_err();
    assert!(matches!(
        error,
        ClientError::PeerUsernameTooLong {
            length,
            max: MAX_INIT_FIELD_BYTES,
        } if length == MAX_INIT_FIELD_BYTES + 1
    ));
}

#[tokio::test]
async fn oversized_obfuscated_peer_init_is_rejected_before_full_frame_allocation() {
    let (mut client, mut server) = duplex(64);
    let key = 0x1357_9bdf;
    let mut header = (MAX_FRAME_LEN as u32).to_le_bytes().to_vec();
    header.push(InitCode::PeerInit.as_u8());
    client
        .write_all(&encode_rotated(&header, key))
        .await
        .unwrap();

    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        read_obfuscated_init_frame(&mut server),
    )
    .await
    .expect("oversized obfuscated init should be rejected from its bounded header")
    .unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge {
            length: MAX_FRAME_LEN,
            max: MAX_PEER_INIT_FRAME_LEN,
        }
    ));
}

#[tokio::test]
async fn oversized_obfuscated_peer_init_field_is_rejected_before_frame_read() {
    let (mut client, mut server) = duplex(64);
    let key = 0x2468_ace0;
    let mut header = (MAX_PEER_INIT_FRAME_LEN as u32).to_le_bytes().to_vec();
    header.push(InitCode::PeerInit.as_u8());
    header.extend_from_slice(&((MAX_INIT_FIELD_BYTES + 1) as u32).to_le_bytes());
    client
        .write_all(&encode_rotated(&header, key))
        .await
        .unwrap();

    let error = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        read_obfuscated_init_frame(&mut server),
    )
    .await
    .expect("oversized obfuscated init field should be rejected from its prefix")
    .unwrap_err();
    assert!(matches!(
        error,
        ClientError::PeerUsernameTooLong {
            length,
            max: MAX_INIT_FIELD_BYTES,
        } if length == MAX_INIT_FIELD_BYTES + 1
    ));
}

#[tokio::test]
async fn raw_frame_round_trips() {
    let (mut client, mut server) = duplex(64);
    let frame = RawFrame::new([1, 2, 3]);

    write_raw_frame(&mut client, &frame).await.unwrap();
    assert_eq!(read_raw_frame(&mut server, 3).await.unwrap(), frame);
}

#[tokio::test]
async fn oversized_message_frame_is_rejected_before_payload_read() {
    let (mut client, mut server) = duplex(64);
    write_message_frame(&mut client, &MessageFrame::new(1, [1, 2, 3]))
        .await
        .unwrap();

    let error = read_message_frame_with_max(&mut server, 2)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge { length: 7, max: 2 }
    ));
}

#[tokio::test]
async fn caller_frame_limit_cannot_raise_the_safety_ceiling() {
    let (mut client, mut server) = duplex(64);
    client
        .write_u32_le((MAX_FRAME_LEN + 1) as u32)
        .await
        .unwrap();

    let error = read_message_frame_with_max(&mut server, usize::MAX)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ClientError::FrameTooLarge {
            length,
            max
        } if length == MAX_FRAME_LEN + 1 && max == MAX_FRAME_LEN
    ));
}

#[tokio::test]
async fn oversized_outbound_frames_are_rejected_before_write() {
    let (mut writer, mut reader) = duplex(64);

    for error in [
        write_message_frame_with_max(&mut writer, &MessageFrame::new(1, [1, 2, 3]), 6)
            .await
            .unwrap_err(),
        write_init_frame_with_max(&mut writer, &InitFrame::new(1, [1, 2, 3]), 3)
            .await
            .unwrap_err(),
        write_raw_frame_with_max(&mut writer, &RawFrame::new([1, 2, 3]), 2)
            .await
            .unwrap_err(),
        write_obfuscated_message_frame_with_key_and_max(
            &mut writer,
            &MessageFrame::new(1, [1, 2, 3]),
            7,
            6,
        )
        .await
        .unwrap_err(),
        write_obfuscated_init_frame_with_key_and_max(
            &mut writer,
            &InitFrame::new(1, [1, 2, 3]),
            7,
            3,
        )
        .await
        .unwrap_err(),
    ] {
        assert!(matches!(error, ClientError::FrameTooLarge { .. }));
    }

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(25), reader.read_u8())
            .await
            .is_err()
    );
}
