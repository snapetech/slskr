//! Controller full mesh fixtures ownership.

use super::*;

pub(super) async fn spawn_mesh_range_source(
    content: Arc<Vec<u8>>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mesh range source");
    let address = listener.local_addr().expect("mesh range source address");
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let content = Arc::clone(&content);
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0_u8; 1_024];
                loop {
                    let count = stream.read(&mut buffer).await.expect("read range request");
                    if count == 0 {
                        return;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).expect("range request UTF-8");
                let range = request
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("range"))
                    .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                    .expect("range header");
                let (start, end) = range.split_once('-').expect("range bounds");
                let start = start.parse::<usize>().expect("range start");
                let end = end.parse::<usize>().expect("range end");
                let body = &content[start..=end];
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {start}-{end}/{}\r\nConnection: close\r\n\r\n",
                            body.len(),
                            content.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .expect("write range headers");
                stream.write_all(body).await.expect("write range body");
            });
        }
    });
    (address, task)
}

pub(super) fn build_stun_success_response(transaction_id: [u8; 12], mapped: SocketAddr) -> Vec<u8> {
    let SocketAddr::V4(mapped) = mapped else {
        panic!("STUN test fixture requires an IPv4 mapped address");
    };
    let xor_port = mapped.port() ^ ((crate::mesh_dht_runtime::STUN_MAGIC_COOKIE >> 16) as u16);
    let xor_address = u32::from(*mapped.ip()) ^ crate::mesh_dht_runtime::STUN_MAGIC_COOKIE;
    let mut attribute = Vec::with_capacity(8);
    attribute.push(0x00);
    attribute.push(0x01);
    attribute.extend_from_slice(&xor_port.to_be_bytes());
    attribute.extend_from_slice(&xor_address.to_be_bytes());

    let mut response = Vec::with_capacity(32);
    response.extend_from_slice(&0x0101_u16.to_be_bytes());
    response.extend_from_slice(&(attribute.len() as u16 + 4).to_be_bytes());
    response.extend_from_slice(&crate::mesh_dht_runtime::STUN_MAGIC_COOKIE.to_be_bytes());
    response.extend_from_slice(&transaction_id);
    response.extend_from_slice(&0x0020_u16.to_be_bytes());
    response.extend_from_slice(&(attribute.len() as u16).to_be_bytes());
    response.extend_from_slice(&attribute);
    response
}

pub(super) async fn serve_one_stun_response(socket: &tokio::net::UdpSocket, mapped: SocketAddr) {
    let mut buf = [0_u8; 128];
    let (count, peer) = socket.recv_from(&mut buf).await.expect("recv STUN request");
    assert!(count >= 20, "STUN request too short");
    let mut transaction_id = [0_u8; 12];
    transaction_id.copy_from_slice(&buf[8..20]);
    let response = build_stun_success_response(transaction_id, mapped);
    socket
        .send_to(&response, peer)
        .await
        .expect("send STUN response");
}

pub(super) async fn send_mesh_sync_fixture_message(
    state: &std::sync::Arc<crate::AppState>,
    session: &mut slskr_client::server::ServerSession<tokio::net::TcpStream>,
    fixture: &mut slskr_client::stream::ServerConnection<tokio::net::TcpStream>,
    remote_key: &ed25519_dalek::SigningKey,
    timestamp: i64,
    mut message: slskr_client::mesh_sync::MeshSyncMessage,
) -> String {
    message
        .sign_at(remote_key, timestamp)
        .expect("sign mesh-sync request");
    let body = message
        .encode_private_message()
        .expect("encode mesh-sync request");
    crate::session_runtime::project_server_message(
        state,
        session,
        &slskr_client::protocol::server::ServerMessage::MessageUserResponse(
            slskr_client::protocol::server::PrivateMessage {
                id: 1,
                timestamp: 1,
                username: "mesh-peer".to_owned(),
                message: body,
                is_new: true,
                was_replayed: false,
            },
        ),
    )
    .await;
    match fixture
        .receive_with_direction(slskr_client::protocol::server::Direction::ClientToServer)
        .await
        .expect("receive mesh-sync response")
    {
        slskr_client::protocol::server::ServerMessage::MessageUserRequest { username, message } => {
            assert_eq!(username, "mesh-peer");
            message
        }
        other => panic!("unexpected mesh-sync response: {other:?}"),
    }
}

pub(super) async fn send_mesh_sync_fixture_without_response(
    state: &std::sync::Arc<crate::AppState>,
    session: &mut slskr_client::server::ServerSession<tokio::net::TcpStream>,
    fixture: &mut slskr_client::stream::ServerConnection<tokio::net::TcpStream>,
    remote_key: &ed25519_dalek::SigningKey,
    timestamp: i64,
    mut message: slskr_client::mesh_sync::MeshSyncMessage,
) {
    message
        .sign_at(remote_key, timestamp)
        .expect("sign mesh-sync response fixture");
    let body = message
        .encode_private_message()
        .expect("encode mesh-sync response fixture");
    crate::session_runtime::project_server_message(
        state,
        session,
        &slskr_client::protocol::server::ServerMessage::MessageUserResponse(
            slskr_client::protocol::server::PrivateMessage {
                id: 2,
                timestamp: 2,
                username: "mesh-peer".to_owned(),
                message: body,
                is_new: true,
                was_replayed: false,
            },
        ),
    )
    .await;
    match tokio::time::timeout(
        std::time::Duration::from_millis(100),
        fixture.receive_with_direction(slskr_client::protocol::server::Direction::ClientToServer),
    )
    .await
    {
        Err(_) => {}
        Ok(Ok(response)) => panic!("unexpected mesh-sync response: {response:?}"),
        Ok(Err(error)) => panic!("mesh-sync response channel failed: {error}"),
    }
}
