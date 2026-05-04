use std::{
    env,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use slskr_client::{
    connection::ConnectionKind,
    listener::{IncomingConnection, Listener},
    peer_connect::{
        send_obfuscated_peer_init, send_peer_init, send_pierce_firewall, IndirectPeerRequest,
    },
    server::{LoginCredentials, ServerSession},
    stream::{ObfuscatedPeerMessageConnection, PeerMessageConnection, ServerConnection},
};
use slskr_protocol::{
    peer::{PeerMessage, UserInfo},
    server::{ConnectToPeerResponse, ObfuscatedPort, ServerMessage, WaitPort},
};
use tokio::net::TcpStream;
use tokio::time;

const SOULFIND_IMAGE: &str = "ghcr.io/soulfind-dev/soulfind:latest";
static NONCE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[tokio::test]
async fn soulfind_accepts_login_and_basic_maintenance_messages() {
    let Some(mut soulfind) = SoulfindRunner::start_optional().await else {
        eprintln!("skipping Soulfind contract test; set SOULFIND_PATH or SLSK_SOULFIND_DOCKER=1");
        return;
    };

    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, soulfind.port));
    let connection = ServerConnection::connect(address).await.unwrap();
    let mut session = ServerSession::new(connection);

    let info = session
        .login(LoginCredentials::new(
            unique_username("slskrtest"),
            "password",
            175,
            1,
        ))
        .await
        .unwrap();
    assert!(!info.greeting.is_empty());

    session.set_wait_port(0).await.unwrap();
    session.send_ping().await.unwrap();

    let mut saw_room_list = false;
    for _ in 0..8 {
        let Ok(message) = time::timeout(Duration::from_millis(500), session.receive()).await else {
            break;
        };
        if matches!(message.unwrap(), ServerMessage::RoomList(_)) {
            saw_room_list = true;
            break;
        }
    }

    soulfind.stop();
    assert!(saw_room_list, "Soulfind did not send a typed room list");
}

#[tokio::test]
async fn soulfind_accepts_room_search_and_reconnect_flow() {
    let Some(mut soulfind) = SoulfindRunner::start_optional().await else {
        eprintln!("skipping Soulfind contract test; set SOULFIND_PATH or SLSK_SOULFIND_DOCKER=1");
        return;
    };

    let mut session = login_to_soulfind(soulfind.port, "flow").await;
    session
        .send_server_message(ServerMessage::RoomListRequest)
        .await
        .unwrap();
    session
        .send_server_message(ServerMessage::JoinRoom {
            room: "slskr-contract".to_owned(),
        })
        .await
        .unwrap();
    session
        .send_server_message(ServerMessage::FileSearchRequest(
            slskr_protocol::server::SearchRequest {
                token: 42,
                query: "slskr contract query".to_owned(),
            },
        ))
        .await
        .unwrap();
    session
        .send_server_message(ServerMessage::LeaveRoom {
            room: "slskr-contract".to_owned(),
        })
        .await
        .unwrap();

    assert!(
        observe_for(&mut session, Duration::from_secs(2), |message| {
            matches!(message, ServerMessage::RoomList(_))
        })
        .await
    );

    soulfind = soulfind.restart().await;
    let mut session = login_to_soulfind(soulfind.port, "reconnect").await;
    session.send_ping().await.unwrap();
    soulfind.stop();
}

#[tokio::test]
async fn soulfind_relays_obfuscated_port_metadata() {
    let Some(mut soulfind) = SoulfindRunner::start_optional().await else {
        eprintln!("skipping Soulfind contract test; set SOULFIND_PATH or SLSK_SOULFIND_DOCKER=1");
        return;
    };

    let target_username = unique_username("slskrobftarget");
    let requester_username = unique_username("slskrobfreq");
    let mut target = login_to_soulfind_as(soulfind.port, &target_username).await;
    let mut requester = login_to_soulfind_as(soulfind.port, &requester_username).await;

    target
        .send_server_message(ServerMessage::SetWaitPort(WaitPort {
            port: 22_341,
            obfuscation: Some(ObfuscatedPort {
                kind: 1,
                port: 22_342,
            }),
        }))
        .await
        .unwrap();
    let address = request_peer_address_matching(
        &mut requester,
        &target_username,
        22_341,
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(address.port, 22_341);
    assert_eq!(address.obfuscation_type, 1);
    assert_eq!(address.obfuscated_port, 22_342);

    soulfind.stop();
}

#[tokio::test]
async fn soulfind_routes_local_direct_and_indirect_peer_connections_without_overrides() {
    let Some(mut soulfind) = SoulfindRunner::start_optional().await else {
        eprintln!("skipping Soulfind contract test; set SOULFIND_PATH or SLSK_SOULFIND_DOCKER=1");
        return;
    };

    let requester_username = unique_username("slskrreq");
    let target_username = unique_username("slskrtgt");
    let requester_listener = Listener::bind("0.0.0.0:0").await.unwrap();
    let requester_listen_port = requester_listener.local_addr().unwrap().port();
    let target_listener = Listener::bind("0.0.0.0:0").await.unwrap();
    let target_listen_port = target_listener.local_addr().unwrap().port();
    let mut requester = login_to_soulfind_as(soulfind.port, &requester_username).await;
    let mut target = login_to_soulfind_as(soulfind.port, &target_username).await;

    requester
        .set_wait_port(u32::from(requester_listen_port))
        .await
        .unwrap();
    target
        .set_wait_port(u32::from(target_listen_port))
        .await
        .unwrap();

    let target_address = request_peer_address_matching(
        &mut requester,
        &target_username,
        u32::from(target_listen_port),
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(target_address.port, u32::from(target_listen_port));

    let direct_accept_task = tokio::spawn(async move { target_listener.accept().await });
    let stream = connect_reported_local_peer(target_address.ip, target_address.port as u16)
        .await
        .unwrap();
    let mut outbound = PeerMessageConnection::new(
        send_peer_init(stream, &requester_username, ConnectionKind::PeerMessages)
            .await
            .unwrap(),
    );
    let (incoming, _) = direct_accept_task.await.unwrap().unwrap();
    let IncomingConnection::PeerInit {
        username,
        kind,
        token,
        stream,
    } = incoming
    else {
        panic!("expected direct PeerInit");
    };
    assert_eq!(username, requester_username);
    assert_eq!(kind, ConnectionKind::PeerMessages);
    assert_eq!(token, 0);
    let mut inbound = PeerMessageConnection::new(stream);
    round_trip_user_info(&mut outbound, &mut inbound).await;

    let token = 0x5eed_0001;
    let indirect_request =
        IndirectPeerRequest::new(token, target_username, ConnectionKind::PeerMessages);
    requester
        .send_server_message(indirect_request.server_message())
        .await
        .unwrap();
    let connect_response =
        wait_for_connect_to_peer(&mut target, token, Duration::from_secs(2)).await;
    assert_eq!(connect_response.username, requester_username);
    assert_eq!(connect_response.port, u32::from(requester_listen_port));

    let indirect_accept_task = tokio::spawn(async move { requester_listener.accept().await });
    let stream = connect_reported_local_peer(connect_response.ip, connect_response.port as u16)
        .await
        .unwrap();
    let mut outbound = PeerMessageConnection::new(
        send_pierce_firewall(stream, connect_response.token)
            .await
            .unwrap(),
    );
    let (incoming, _) = indirect_accept_task.await.unwrap().unwrap();
    let stream = indirect_request.complete(incoming).unwrap();
    let mut inbound = PeerMessageConnection::new(stream);
    round_trip_user_info(&mut outbound, &mut inbound).await;

    soulfind.stop();
}

#[tokio::test]
async fn soulfind_routes_local_obfuscated_peer_connections_without_overrides() {
    let Some(mut soulfind) = SoulfindRunner::start_optional().await else {
        eprintln!("skipping Soulfind contract test; set SOULFIND_PATH or SLSK_SOULFIND_DOCKER=1");
        return;
    };

    let requester_username = unique_username("slskrobfreq");
    let target_username = unique_username("slskrobftgt");
    let regular_listener = Listener::bind("0.0.0.0:0").await.unwrap();
    let regular_port = regular_listener.local_addr().unwrap().port();
    let obfuscated_listener = Listener::bind("0.0.0.0:0").await.unwrap();
    let obfuscated_port = obfuscated_listener.local_addr().unwrap().port();
    let mut requester = login_to_soulfind_as(soulfind.port, &requester_username).await;
    let mut target = login_to_soulfind_as(soulfind.port, &target_username).await;

    target
        .set_wait_port_obfuscated(u32::from(regular_port), 1, u32::from(obfuscated_port))
        .await
        .unwrap();

    let target_address = request_peer_address_matching(
        &mut requester,
        &target_username,
        u32::from(regular_port),
        Duration::from_secs(3),
    )
    .await;
    assert_eq!(target_address.obfuscation_type, 1);
    assert_eq!(target_address.obfuscated_port, obfuscated_port);

    let obfuscated_accept_task =
        tokio::spawn(async move { obfuscated_listener.accept_obfuscated().await });
    let stream = connect_reported_local_peer(target_address.ip, target_address.obfuscated_port)
        .await
        .unwrap();
    let mut outbound = ObfuscatedPeerMessageConnection::new(
        send_obfuscated_peer_init(stream, &requester_username, ConnectionKind::PeerMessages)
            .await
            .unwrap(),
    );
    let (incoming, _) = obfuscated_accept_task.await.unwrap().unwrap();
    let IncomingConnection::ObfuscatedPeerMessages(mut inbound) = incoming else {
        panic!("expected obfuscated peer messages");
    };
    round_trip_obfuscated_user_info(&mut outbound, &mut inbound).await;

    drop(regular_listener);
    soulfind.stop();
}

async fn login_to_soulfind(port: u16, label: &str) -> ServerSession<tokio::net::TcpStream> {
    login_to_soulfind_as(port, &unique_username(&format!("slskr{label}"))).await
}

async fn login_to_soulfind_as(port: u16, username: &str) -> ServerSession<tokio::net::TcpStream> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let connection = ServerConnection::connect(address).await.unwrap();
    let mut session = ServerSession::new(connection);
    let info = session
        .login(LoginCredentials::new(username, "password", 175, 1))
        .await
        .unwrap();
    assert!(!info.greeting.is_empty());
    session
}

async fn request_peer_address_matching<S>(
    session: &mut ServerSession<S>,
    username: &str,
    expected_port: u32,
    duration: Duration,
) -> slskr_protocol::server::PeerAddress
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = time::Instant::now() + duration;
    let mut last = None;
    while time::Instant::now() < deadline {
        session
            .send_server_message(ServerMessage::GetPeerAddressRequest {
                username: username.to_owned(),
            })
            .await
            .unwrap();
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        let Ok(Ok(message)) =
            time::timeout(remaining.min(Duration::from_millis(250)), session.receive()).await
        else {
            continue;
        };
        if let ServerMessage::GetPeerAddressResponse(address) = message {
            if address.port == expected_port {
                return address;
            }
            last = Some(address);
        }
    }
    panic!("timed out waiting for peer address port {expected_port}; last={last:?}");
}

async fn connect_reported_local_peer(ip: Ipv4Addr, port: u16) -> std::io::Result<TcpStream> {
    match time::timeout(Duration::from_secs(1), TcpStream::connect((ip, port))).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(error)) if !ip.is_loopback() => {
            let fallback = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).await;
            fallback.map_err(|_| error)
        }
        Ok(Err(error)) => Err(error),
        Err(_) if !ip.is_loopback() => TcpStream::connect((Ipv4Addr::LOCALHOST, port)).await,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "reported peer connect timed out",
        )),
    }
}

async fn wait_for_connect_to_peer<S>(
    session: &mut ServerSession<S>,
    token: u32,
    duration: Duration,
) -> ConnectToPeerResponse
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = time::Instant::now() + duration;
    while time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        let Ok(Ok(message)) = time::timeout(remaining, session.receive()).await else {
            break;
        };
        match message {
            ServerMessage::ConnectToPeerResponse(response) if response.token == token => {
                return response;
            }
            ServerMessage::CantConnectToPeerResponse { token: failed } if failed == token => {
                panic!("Soulfind reported CantConnectToPeer for token {token}");
            }
            _ => {}
        }
    }
    panic!("timed out waiting for ConnectToPeer response");
}

async fn round_trip_user_info<A, B>(
    outbound: &mut PeerMessageConnection<A>,
    inbound: &mut PeerMessageConnection<B>,
) where
    A: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    B: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    outbound.send(&PeerMessage::UserInfoRequest).await.unwrap();
    assert_eq!(
        inbound.receive().await.unwrap(),
        PeerMessage::UserInfoRequest
    );
    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr Soulfind peer contract".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .unwrap();
    assert!(matches!(
        outbound.receive().await.unwrap(),
        PeerMessage::UserInfoResponse(_)
    ));
}

async fn round_trip_obfuscated_user_info<A, B>(
    outbound: &mut ObfuscatedPeerMessageConnection<A>,
    inbound: &mut ObfuscatedPeerMessageConnection<B>,
) where
    A: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    B: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    outbound.send(&PeerMessage::UserInfoRequest).await.unwrap();
    assert_eq!(
        inbound.receive().await.unwrap(),
        PeerMessage::UserInfoRequest
    );
    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr Soulfind obfuscated peer contract".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .unwrap();
    assert!(matches!(
        outbound.receive().await.unwrap(),
        PeerMessage::UserInfoResponse(_)
    ));
}

async fn observe_for<S, F>(
    session: &mut ServerSession<S>,
    duration: Duration,
    mut predicate: F,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    F: FnMut(&ServerMessage) -> bool,
{
    let deadline = time::Instant::now() + duration;
    while time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        let Ok(Ok(message)) = time::timeout(remaining, session.receive()).await else {
            return false;
        };
        if predicate(&message) {
            return true;
        }
    }
    false
}

#[derive(Debug)]
struct SoulfindRunner {
    port: u16,
    process: Option<Child>,
    docker_container: Option<String>,
    data_dir: Option<PathBuf>,
}

impl SoulfindRunner {
    async fn start_optional() -> Option<Self> {
        let port = allocate_ephemeral_port();
        if let Ok(path) = env::var("SOULFIND_PATH") {
            return Some(Self::start_binary(path.into(), port).await);
        }

        if env::var("SLSK_SOULFIND_DOCKER").as_deref() == Ok("1") {
            return Some(Self::start_docker(port).await);
        }

        None
    }

    async fn restart(mut self) -> Self {
        let port = self.port;
        let docker = self.docker_container.is_some();
        let binary = self.process.as_ref().map(|process| process.id());
        self.stop();

        if docker {
            return Self::start_docker(port).await;
        }

        if binary.is_some() {
            let path = env::var("SOULFIND_PATH").expect("SOULFIND_PATH disappeared");
            return Self::start_binary(path.into(), port).await;
        }

        unreachable!("runner has neither Docker nor process state")
    }

    async fn start_binary(path: PathBuf, port: u16) -> Self {
        let data_dir = env::temp_dir().join(format!("slskr-soulfind-{port}-{}", process_nonce()));
        std::fs::create_dir_all(&data_dir).unwrap();

        let process = Command::new(path)
            .args([
                "--port",
                &port.to_string(),
                "--data-dir",
                data_dir.to_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        wait_for_ready().await;
        Self {
            port,
            process: Some(process),
            docker_container: None,
            data_dir: Some(data_dir),
        }
    }

    async fn start_docker(port: u16) -> Self {
        let container = format!("slskr-soulfind-{port}-{}", process_nonce());
        let status = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "--name",
                &container,
                "-p",
                &format!("127.0.0.1:{port}:2242"),
                SOULFIND_IMAGE,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "failed to start Soulfind Docker container"
        );

        wait_for_ready().await;
        Self {
            port,
            process: None,
            docker_container: Some(container),
            data_dir: None,
        }
    }

    fn stop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
        if let Some(container) = self.docker_container.take() {
            let _ = Command::new("docker")
                .args(["rm", "-f", &container])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        if let Some(data_dir) = self.data_dir.take() {
            let _ = std::fs::remove_dir_all(data_dir);
        }
    }
}

impl Drop for SoulfindRunner {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn wait_for_ready() {
    time::sleep(Duration::from_secs(1)).await;
}

fn allocate_ephemeral_port() -> u16 {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.local_addr().unwrap().port()
}

fn process_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn unique_username(prefix: &str) -> String {
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}{:x}{counter:x}", process_nonce() % 0xffff_ffff)
}
