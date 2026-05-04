use slskr_client::{
    connection::ConnectionKind,
    io::{
        read_connection_kind, read_init_frame, read_message_frame_with_max, read_raw_frame,
        write_connection_kind, write_init_frame, write_message_frame, write_raw_frame,
    },
    ClientError,
};
use slskr_protocol::{InitFrame, MessageFrame, RawFrame};
use tokio::io::duplex;

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
