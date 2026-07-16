use slskr_client::{
    connection::ConnectionKind,
    io::{
        read_connection_kind, read_init_frame, read_message_frame_with_max, read_raw_frame,
        read_raw_frame_with_max, write_connection_kind, write_init_frame,
        write_init_frame_with_max, write_message_frame, write_message_frame_with_max,
        write_obfuscated_init_frame_with_key_and_max,
        write_obfuscated_message_frame_with_key_and_max, write_raw_frame, write_raw_frame_with_max,
    },
    ClientError,
};
use slskr_protocol::{InitFrame, MessageFrame, RawFrame};
use tokio::io::{duplex, AsyncReadExt};

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
async fn init_frame_round_trips() {
    let (mut client, mut server) = duplex(64);
    let frame = InitFrame::new(1, [1, 2, 3]);

    write_init_frame(&mut client, &frame).await.unwrap();
    assert_eq!(read_init_frame(&mut server).await.unwrap(), frame);
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
