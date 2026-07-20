use std::net::Ipv4Addr;

use slskr_client::{
    server::{LoginCredentials, ServerSession, SessionInfo},
    stream::ServerConnection,
    version::{CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION},
    ClientError,
};
use slskr_protocol::server::{Direction, LoginResponse, ObfuscatedPort, ServerMessage, WaitPort};
use tokio::io::duplex;

#[tokio::test]
async fn credentials_build_protocol_login_request() {
    let credentials = LoginCredentials::new("username", "password", 175, 1);
    let request = credentials.clone().into_login_request();

    assert_eq!(credentials.login_hash(), "d51c9a7e9353746a6020f9602d452929");
    assert_eq!(
        credentials.password_hash(),
        "5f4dcc3b5aa765d61d8327deb882cf99"
    );
    assert_eq!(request.username, "username");
    assert_eq!(request.password, "password");
    assert_eq!(request.major_version, 175);
    assert_eq!(request.minor_version, 1);
    assert_eq!(request.hash, "d51c9a7e9353746a6020f9602d452929");
}

#[test]
fn credential_debug_output_redacts_plaintext_password() {
    let credentials = LoginCredentials::new("username", "unique-secret", 175, 1);

    let output = format!("{credentials:?}");

    assert!(output.contains("username"));
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("unique-secret"));
}

#[test]
fn session_debug_output_redacts_password_hash() {
    let session = SessionInfo {
        greeting: "motd".to_owned(),
        ip: Ipv4Addr::LOCALHOST,
        password_hash: "redacted-password-hash-placeholder".to_owned(),
        is_supporter: true,
    };

    let output = format!("{session:?}");

    assert!(output.contains("motd"));
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("redacted-password-hash-placeholder"));
}

#[tokio::test]
async fn default_client_credentials_use_reserved_version_band() {
    let credentials = LoginCredentials::default_client("username", "password");
    let request = credentials.into_login_request();

    assert_eq!(request.major_version, CLIENT_MAJOR_VERSION);
    assert_eq!(request.minor_version, CLIENT_MINOR_VERSION);
    assert_eq!(request.hash, "d51c9a7e9353746a6020f9602d452929");
}

#[tokio::test]
async fn login_sends_request_and_returns_session_info() {
    let (client, server) = duplex(512);
    let mut session = ServerSession::new(ServerConnection::new(client));
    let mut server = ServerConnection::new(server);

    let client_task = tokio::spawn(async move {
        session
            .login(LoginCredentials::new("username", "password", 175, 1))
            .await
            .unwrap()
    });

    let request = server
        .receive_with_direction(Direction::ClientToServer)
        .await
        .unwrap();
    let ServerMessage::LoginRequest(request) = request else {
        panic!("expected login request");
    };
    assert_eq!(request.hash, "d51c9a7e9353746a6020f9602d452929");

    server
        .send(&ServerMessage::LoginResponse(LoginResponse::Success {
            greet: "motd".to_owned(),
            ip: Ipv4Addr::new(127, 0, 0, 1),
            hash: "5f4dcc3b5aa765d61d8327deb882cf99".to_owned(),
            is_supporter: true,
        }))
        .await
        .unwrap();

    assert_eq!(
        client_task.await.unwrap(),
        SessionInfo {
            greeting: "motd".to_owned(),
            ip: Ipv4Addr::new(127, 0, 0, 1),
            password_hash: "5f4dcc3b5aa765d61d8327deb882cf99".to_owned(),
            is_supporter: true,
        }
    );
}

#[tokio::test]
async fn login_with_wait_port_sends_both_frames_before_waiting_for_response() {
    let (client, server) = duplex(512);
    let mut session = ServerSession::new(ServerConnection::new(client));
    let mut server = ServerConnection::new(server);
    let wait_port = WaitPort {
        port: 2234,
        obfuscation: Some(ObfuscatedPort {
            kind: 1,
            port: 2235,
        }),
    };

    let client_task = tokio::spawn(async move {
        session
            .login_with_wait_port(
                LoginCredentials::new("username", "password", 175, 1),
                wait_port,
            )
            .await
            .unwrap()
    });

    assert!(matches!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::LoginRequest(_)
    ));
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::SetWaitPort(WaitPort {
            port: 2234,
            obfuscation: Some(ObfuscatedPort {
                kind: 1,
                port: 2235,
            }),
        })
    );

    server
        .send(&ServerMessage::LoginResponse(LoginResponse::Success {
            greet: "motd".to_owned(),
            ip: Ipv4Addr::LOCALHOST,
            hash: "hash".to_owned(),
            is_supporter: false,
        }))
        .await
        .unwrap();

    assert_eq!(client_task.await.unwrap().greeting, "motd");
}

#[tokio::test]
async fn login_failure_is_reported() {
    let (client, server) = duplex(512);
    let mut session = ServerSession::new(ServerConnection::new(client));
    let mut server = ServerConnection::new(server);

    let client_task = tokio::spawn(async move {
        session
            .login(LoginCredentials::new("bad", "password", 175, 1))
            .await
            .unwrap_err()
    });

    let _ = server
        .receive_with_direction(Direction::ClientToServer)
        .await
        .unwrap();
    server
        .send(&ServerMessage::LoginResponse(LoginResponse::Failure {
            reason: "INVALIDUSERNAME".to_owned(),
            detail: Some("Nick empty.".to_owned()),
        }))
        .await
        .unwrap();

    let error = client_task.await.unwrap();
    assert!(matches!(
        error,
        ClientError::LoginRejected {
            reason,
            detail
        } if reason == "INVALIDUSERNAME" && detail == " (Nick empty.)"
    ));
}

#[tokio::test]
async fn set_wait_port_and_ping_send_server_messages() {
    let (client, server) = duplex(512);
    let mut session = ServerSession::new(ServerConnection::new(client));
    let mut server = ServerConnection::new(server);

    session.set_wait_port(2234).await.unwrap();
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::SetWaitPort(WaitPort {
            port: 2234,
            obfuscation: None,
        })
    );

    session.send_ping().await.unwrap();
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::ServerPing
    );
}

#[tokio::test]
async fn set_wait_port_obfuscated_sends_metadata() {
    let (client, server) = duplex(512);
    let mut session = ServerSession::new(ServerConnection::new(client));
    let mut server = ServerConnection::new(server);

    session
        .set_wait_port_obfuscated(2234, 1, 2235)
        .await
        .unwrap();

    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::SetWaitPort(WaitPort {
            port: 2234,
            obfuscation: Some(ObfuscatedPort {
                kind: 1,
                port: 2235,
            }),
        })
    );
}
