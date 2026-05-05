use sha2::{Digest, Sha256};
use slskr_client::protocol::{
    distributed::DistributedMessage,
    init::InitMessage,
    peer::{FileEntry, PeerMessage, TransferRequest, TransferResponse, UserInfo},
    server::{ConnectToPeerResponse, SearchRequest, ServerMessage},
    Writer, ROTATED_OBFUSCATION_TYPE,
};
use slskr_client::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    io::read_init_frame_with_first_len_byte,
    listener::{IncomingConnection, Listener},
    peer_connect::{
        send_obfuscated_peer_init, send_obfuscated_peer_init_with_token, send_peer_init,
        send_peer_init_with_token, send_pierce_firewall, IndirectPeerRequest,
    },
    server::{LoginCredentials, ServerSession},
    share_payload::{compress_zlib_payload, decompress_peer_share_payload},
    stream::{
        DistributedConnection, ObfuscatedPeerMessageConnection, PeerMessageConnection,
        ServerConnection,
    },
    version::{
        CLIENT_MAJOR_VERSION, CLIENT_MINOR_VERSION, CLIENT_NAME, DEFAULT_LISTEN_PORT,
        DEFAULT_SERVER_ADDRESS,
    },
};
use std::{ffi::OsString, fs, net::SocketAddr, path::PathBuf, time::Duration};
use tokio::net::TcpStream;
use tokio::time::{self, Instant};

pub async fn run_from_env() -> Result<(), String> {
    run_from_args(std::env::args_os().skip(1)).await
}

pub async fn run_from_args<I>(args: I) -> Result<(), String>
where
    I: IntoIterator<Item = OsString>,
{
    let args = normalize_command(args)?;
    let mut args = args.iter().map(String::as_str);
    match args.next() {
        Some("obfuscated-peer-probe") => obfuscated_peer_probe().await,
        Some("indirect-peer-probe") => indirect_peer_probe().await,
        Some("plain-peer-probe") => plain_peer_probe().await,
        Some("browse-peer-probe") => browse_peer_probe().await,
        Some("download-peer-probe") => download_peer_probe().await,
        Some("private-message-probe") => private_message_probe().await,
        Some("room-message-probe") => room_message_probe().await,
        Some("distributed-peer-probe") => distributed_peer_probe().await,
        Some("file-transfer-peer-probe") => file_transfer_peer_probe().await,
        Some("metadata-relogin-probe") => metadata_relogin_probe().await,
        Some("negative-indirect-probe") => negative_indirect_probe().await,
        Some("peer-address-probe") => peer_address_probe().await,
        Some("fixture-peer-smoke") => fixture_peer_smoke().await,
        Some("local-peer-smoke") => local_peer_smoke().await,
        Some("live-soak") => live_soak().await,
        Some("login-smoke") => login_smoke().await,
        Some("version") => {
            println!("{CLIENT_NAME} {CLIENT_MAJOR_VERSION}.{CLIENT_MINOR_VERSION}");
            Ok(())
        }
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(command) => Err(format!("unknown command: {command}\n\n{}", usage())),
    }
}

fn normalize_command<I>(args: I) -> Result<Vec<String>, String>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args
        .into_iter()
        .map(|arg| {
            arg.into_string()
                .map_err(|_| "arguments must be valid UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let Some(first) = args.first().map(String::as_str) else {
        return Ok(args);
    };

    let normalized = match first {
        "login" if args.get(1).map(String::as_str) == Some("smoke") => vec!["login-smoke"],
        "soak" if args.get(1).map(String::as_str) == Some("live") => vec!["live-soak"],
        "smoke" if args.get(1).map(String::as_str) == Some("local-peer") => {
            vec!["local-peer-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("fixture-peer") => {
            vec!["fixture-peer-smoke"]
        }
        "probe" => match args.get(1).map(String::as_str) {
            Some("peer-address") => vec!["peer-address-probe"],
            Some("plain-peer") => vec!["plain-peer-probe"],
            Some("browse-peer") => vec!["browse-peer-probe"],
            Some("download-peer") => vec!["download-peer-probe"],
            Some("private-message") => vec!["private-message-probe"],
            Some("room-message") => vec!["room-message-probe"],
            Some("obfuscated-peer") => vec!["obfuscated-peer-probe"],
            Some("indirect-peer") => vec!["indirect-peer-probe"],
            Some("distributed-peer") => vec!["distributed-peer-probe"],
            Some("file-transfer-peer") => vec!["file-transfer-peer-probe"],
            Some("metadata-relogin") => vec!["metadata-relogin-probe"],
            Some("negative-indirect") => vec!["negative-indirect-probe"],
            _ => return Err(format!("unknown probe command\n\n{}", usage())),
        },
        _ => return Ok(args),
    };

    Ok(normalized
        .into_iter()
        .map(str::to_owned)
        .chain(args.into_iter().skip(2))
        .collect())
}

fn print_usage() {
    eprintln!("{}", usage());
}

fn usage() -> &'static str {
    "usage:
  slskr version
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr login smoke
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr soak live
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe peer-address
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe plain-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe browse-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> SLSK_DOWNLOAD_FILENAME='Share\\File.txt' slskr probe download-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_MESSAGE_USERNAME=<user2> SLSK_MESSAGE_PASSWORD=<pass2> slskr probe private-message
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr probe room-message
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_OBFUSCATED_PEER_USERNAME=<peer> slskr probe obfuscated-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe indirect-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe distributed-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe file-transfer-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe metadata-relogin
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe negative-indirect
  SLSKR_A_USERNAME=<user> SLSKR_A_PASSWORD=<pass> SLSKR_B_USERNAME=<user> SLSKR_B_PASSWORD=<pass> slskr smoke local-peer
  slskr smoke fixture-peer

Legacy slskr-cli command names are still accepted during migration."
}

async fn peer_address_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_PEER_USERNAME", "SLSK_OBFUSCATED_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_PEER_ADDRESS_PROBE_TIMEOUT_SECONDS", 10)?);
    let attempts = env_usize("SLSK_PEER_ADDRESS_PROBE_ATTEMPTS", 5)?;

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username, password))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;

    for attempt in 1..=attempts {
        session
            .send_server_message(ServerMessage::GetPeerAddressRequest {
                username: peer_username.clone(),
            })
            .await
            .map_err(|error| format!("peer-address request failed: {error}"))?;
        let address = wait_for_peer_address_response(&mut session, timeout).await?;
        println!(
            "peer address attempt={attempt}{} port={} obfuscation_type={} obfuscated_port={}",
            peer_address_ip_detail(&address)?,
            address.port,
            address.obfuscation_type,
            address.obfuscated_port
        );
        if attempt < attempts {
            time::sleep(Duration::from_secs(2)).await;
        }
    }

    Ok(())
}

async fn login_smoke() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let listen_port = std::env::var("SLSK_LISTEN_PORT")
        .ok()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|error| format!("invalid SLSK_LISTEN_PORT: {error}"))
        })
        .transpose()?
        .unwrap_or(DEFAULT_LISTEN_PORT);

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    let info = session
        .login(LoginCredentials::default_client(username.clone(), password))
        .await
        .map_err(|error| format!("login failed for {username}: {error}"))?;
    session
        .set_wait_port(listen_port)
        .await
        .map_err(|error| format!("set wait port failed: {error}"))?;
    session
        .send_ping()
        .await
        .map_err(|error| format!("ping failed: {error}"))?;

    println!("logged in; supporter={}", info.is_supporter);
    Ok(())
}

async fn obfuscated_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_OBFUSCATED_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_OBFUSCATED_PROBE_TIMEOUT_SECONDS", 15)?);

    let attempts = env_usize("SLSK_OBFUSCATED_PEER_ADDRESS_ATTEMPTS", 5)?;
    let mut last_error = None;
    let mut address = None;
    for _ in 0..attempts {
        match resolve_peer_address(
            &username,
            &password,
            &peer_username,
            &server_address,
            timeout,
        )
        .await
        {
            Ok(candidate) if candidate.obfuscation_type == 1 && candidate.obfuscated_port != 0 => {
                address = Some(candidate);
                break;
            }
            Ok(candidate) => {
                last_error = Some(format!(
                    "peer did not advertise rotated obfuscation: type={} obfuscated_port={}",
                    candidate.obfuscation_type, candidate.obfuscated_port
                ));
            }
            Err(error) => last_error = Some(error),
        }
        time::sleep(Duration::from_secs(1)).await;
    }
    let address = address.ok_or_else(|| {
        last_error.unwrap_or_else(|| "peer did not advertise rotated obfuscation".to_owned())
    })?;
    if address.obfuscation_type != 1 || address.obfuscated_port == 0 {
        return Err(format!(
            "peer did not advertise rotated obfuscation: type={} obfuscated_port={}",
            address.obfuscation_type, address.obfuscated_port
        ));
    }

    let host =
        optional_env("SLSK_OBFUSCATED_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let stream = time::timeout(
        timeout,
        TcpStream::connect((host.as_str(), address.obfuscated_port)),
    )
    .await
    .map_err(|_| "obfuscated peer connect timed out".to_owned())?
    .map_err(|error| format!("obfuscated peer connect failed: {error}"))?;
    let init_token = env_u32("SLSK_OBFUSCATED_PEER_INIT_TOKEN", 0)?;
    let stream = send_obfuscated_peer_init_with_token(
        stream,
        &username,
        ConnectionKind::PeerMessages,
        init_token,
    )
    .await
    .map_err(|error| format!("obfuscated peer init failed: {error}"))?;
    let init_settle_millis = env_u64("SLSK_OBFUSCATED_INIT_SETTLE_MILLIS", 100)?;
    if init_settle_millis > 0 {
        time::sleep(Duration::from_millis(init_settle_millis)).await;
    }
    if env_bool("SLSK_OBFUSCATED_DIAGNOSTIC", false)? {
        return obfuscated_peer_diagnostic(
            &username,
            &peer_username,
            &host,
            address.obfuscated_port,
            init_token,
            timeout,
            init_settle_millis,
        )
        .await;
    }
    let primary = obfuscated_user_info_attempt(stream, timeout, true).await;
    let used_plain_response_fallback = match primary {
        Ok(()) => false,
        Err(primary_error) if env_bool("SLSK_OBFUSCATED_ALLOW_PLAIN_RESPONSE", true)? => {
            let stream = time::timeout(
                timeout,
                TcpStream::connect((host.as_str(), address.obfuscated_port)),
            )
            .await
            .map_err(|_| "obfuscated peer fallback connect timed out".to_owned())?
            .map_err(|error| format!("obfuscated peer fallback connect failed: {error}"))?;
            let stream = send_obfuscated_peer_init_with_token(
                stream,
                &username,
                ConnectionKind::PeerMessages,
                init_token,
            )
            .await
            .map_err(|error| format!("obfuscated peer fallback init failed after primary failure ({primary_error}): {error}"))?;
            if init_settle_millis > 0 {
                time::sleep(Duration::from_millis(init_settle_millis)).await;
            }
            obfuscated_user_info_attempt(stream, timeout, false)
                .await
                .map_err(|fallback_error| {
                    format!(
                        "obfuscated user-info failed; primary={primary_error}; plain-response fallback={fallback_error}"
                    )
                })?;
            true
        }
        Err(error) => return Err(error),
    };

    if used_plain_response_fallback {
        println!(
            "obfuscated peer probe completed with plain-response fallback; peer={}; host_override={}",
            redact_username(&peer_username),
            optional_env("SLSK_OBFUSCATED_HOST_OVERRIDE").is_some()
        );
    } else {
        println!(
            "obfuscated peer probe completed; peer={}; host_override={}",
            redact_username(&peer_username),
            optional_env("SLSK_OBFUSCATED_HOST_OVERRIDE").is_some()
        );
    }
    Ok(())
}

async fn obfuscated_user_info_attempt(
    stream: TcpStream,
    timeout: Duration,
    receive_obfuscated: bool,
) -> Result<(), String> {
    let mut peer = ObfuscatedPeerMessageConnection::new(stream);
    peer.send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("obfuscated user-info request failed: {error}"))?;
    let stream = peer.into_inner();
    let response = if receive_obfuscated {
        let mut peer = ObfuscatedPeerMessageConnection::new(stream);
        time::timeout(timeout, peer.receive())
            .await
            .map_err(|_| "obfuscated user-info response timed out".to_owned())?
            .map_err(|error| format!("obfuscated user-info response failed: {error}"))?
    } else {
        let mut peer = PeerMessageConnection::new(stream);
        time::timeout(timeout, peer.receive())
            .await
            .map_err(|_| "plain user-info response on obfuscated connection timed out".to_owned())?
            .map_err(|error| {
                format!("plain user-info response on obfuscated connection failed: {error}")
            })?
    };
    user_info_response_result(response)
}

async fn obfuscated_peer_diagnostic(
    username: &str,
    peer_username: &str,
    host: &str,
    port: u16,
    init_token: u32,
    timeout: Duration,
    init_settle_millis: u64,
) -> Result<(), String> {
    let variants = [
        (true, true, "obfuscated-request/obfuscated-response"),
        (true, false, "obfuscated-request/plain-response"),
        (false, true, "plain-request/obfuscated-response"),
        (false, false, "plain-request/plain-response"),
    ];
    let mut details = Vec::new();

    for (send_obfuscated, receive_obfuscated, label) in variants {
        let result = obfuscated_peer_diagnostic_attempt(
            username,
            host,
            port,
            init_token,
            timeout,
            init_settle_millis,
            send_obfuscated,
            receive_obfuscated,
        )
        .await;
        match result {
            Ok(()) => {
                println!(
                    "obfuscated peer diagnostic completed; peer={}; winning_variant={label}; host_override={}",
                    redact_username(peer_username),
                    optional_env("SLSK_OBFUSCATED_HOST_OVERRIDE").is_some()
                );
                return Ok(());
            }
            Err(error) => details.push(format!("{label}: {error}")),
        }
    }

    Err(format!(
        "obfuscated peer diagnostic failed; peer={}; variants=[{}]",
        redact_username(peer_username),
        details.join(" | ")
    ))
}

#[allow(clippy::too_many_arguments)]
async fn obfuscated_peer_diagnostic_attempt(
    username: &str,
    host: &str,
    port: u16,
    init_token: u32,
    timeout: Duration,
    init_settle_millis: u64,
    send_obfuscated: bool,
    receive_obfuscated: bool,
) -> Result<(), String> {
    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
        .await
        .map_err(|_| "connect timed out".to_owned())?
        .map_err(|error| format!("connect failed: {error}"))?;
    let stream = send_obfuscated_peer_init_with_token(
        stream,
        username,
        ConnectionKind::PeerMessages,
        init_token,
    )
    .await
    .map_err(|error| format!("init failed: {error}"))?;
    if init_settle_millis > 0 {
        time::sleep(Duration::from_millis(init_settle_millis)).await;
    }

    if send_obfuscated && receive_obfuscated {
        let mut peer = ObfuscatedPeerMessageConnection::new(stream);
        peer.send(&PeerMessage::UserInfoRequest)
            .await
            .map_err(|error| format!("request send failed: {error}"))?;
        let response = time::timeout(timeout, peer.receive())
            .await
            .map_err(|_| "response timed out".to_owned())?
            .map_err(|error| format!("response failed: {error}"))?;
        return user_info_response_result(response);
    }

    if send_obfuscated {
        let mut peer = ObfuscatedPeerMessageConnection::new(stream);
        peer.send(&PeerMessage::UserInfoRequest)
            .await
            .map_err(|error| format!("request send failed: {error}"))?;
        let stream = peer.into_inner();
        let mut plain = PeerMessageConnection::new(stream);
        let response = time::timeout(timeout, plain.receive())
            .await
            .map_err(|_| "response timed out".to_owned())?
            .map_err(|error| format!("response failed: {error}"))?;
        return user_info_response_result(response);
    }

    let mut plain = PeerMessageConnection::new(stream);
    plain
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("request send failed: {error}"))?;
    let stream = plain.into_inner();
    if receive_obfuscated {
        let mut peer = ObfuscatedPeerMessageConnection::new(stream);
        let response = time::timeout(timeout, peer.receive())
            .await
            .map_err(|_| "response timed out".to_owned())?
            .map_err(|error| format!("response failed: {error}"))?;
        user_info_response_result(response)
    } else {
        let mut plain = PeerMessageConnection::new(stream);
        let response = time::timeout(timeout, plain.receive())
            .await
            .map_err(|_| "response timed out".to_owned())?
            .map_err(|error| format!("response failed: {error}"))?;
        user_info_response_result(response)
    }
}

fn user_info_response_result(response: PeerMessage) -> Result<(), String> {
    if matches!(response, PeerMessage::UserInfoResponse(_)) {
        Ok(())
    } else {
        Err(format!("unexpected response: {response:?}"))
    }
}

async fn plain_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_PLAIN_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_PLAIN_PROBE_TIMEOUT_SECONDS", 15)?);

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username.clone(), password))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;

    session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: peer_username.clone(),
        })
        .await
        .map_err(|error| format!("peer-address request failed: {error}"))?;
    let address = wait_for_peer_address_response(&mut session, timeout).await?;
    if address.port == 0 {
        return Err("peer did not advertise a plain listener port".to_owned());
    }
    let port = u16::try_from(address.port).map_err(|_| {
        format!(
            "peer advertised invalid plain listener port: {}",
            address.port
        )
    })?;

    let host = optional_env("SLSK_PLAIN_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "plain peer connect timed out".to_owned())?
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    let init_token = env_u32("SLSK_PLAIN_PEER_INIT_TOKEN", 0)?;
    let stream =
        send_peer_init_with_token(stream, &username, ConnectionKind::PeerMessages, init_token)
            .await
            .map_err(|error| format!("plain peer init failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);

    peer.send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("plain user-info request failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "plain user-info response timed out".to_owned())?
        .map_err(|error| format!("plain user-info response failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected plain peer response: {response:?}"));
    }

    println!(
        "plain peer probe completed; peer={}; host_override={}",
        redact_username(&peer_username),
        optional_env("SLSK_PLAIN_HOST_OVERRIDE").is_some()
    );
    Ok(())
}

async fn browse_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_BROWSE_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let expected = optional_env("SLSK_BROWSE_EXPECTED");
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_BROWSE_PROBE_TIMEOUT_SECONDS", 20)?);

    let address = resolve_peer_address(
        &username,
        &password,
        &peer_username,
        &server_address,
        timeout,
    )
    .await?;
    let port = peer_regular_port(&address)?;
    let host = optional_env("SLSK_BROWSE_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let mut peer = connect_plain_peer_messages(&username, &host, port, timeout).await?;
    peer.send(&PeerMessage::GetShareFileList)
        .await
        .map_err(|error| format!("browse request failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "browse response timed out".to_owned())?
        .map_err(|error| format!("browse response failed: {error}"))?;
    let payload = decompress_peer_share_payload(&response)
        .ok_or_else(|| format!("unexpected browse response: {response:?}"))?
        .map_err(|error| format!("browse payload decompress failed: {error}"))?;
    let preview = browse_payload_preview(&payload);
    if let Some(expected) = expected.as_deref() {
        let text = String::from_utf8_lossy(&payload);
        if !text.contains(expected) {
            return Err(format!(
                "browse payload missing expected fixture; expected={expected}; preview={preview}"
            ));
        }
    }

    println!(
        "browse peer probe completed; peer={}; bytes={}; preview={}",
        redact_username(&peer_username),
        payload.len(),
        preview
    );
    Ok(())
}

async fn download_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_DOWNLOAD_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let filename = required_env_any(&["SLSK_DOWNLOAD_FILENAME"])?;
    let expected = optional_env("SLSK_DOWNLOAD_EXPECTED");
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_DOWNLOAD_PROBE_TIMEOUT_SECONDS", 30)?);
    let token = env_u32("SLSK_DOWNLOAD_TOKEN", 0x51ab_4001)?;

    if optional_env("SLSK_DOWNLOAD_LISTENER_BIND").is_some() {
        return queued_download_peer_probe(
            username,
            password,
            peer_username,
            filename,
            expected,
            server_address,
            timeout,
            token,
        )
        .await;
    }

    let address = resolve_peer_address(
        &username,
        &password,
        &peer_username,
        &server_address,
        timeout,
    )
    .await?;
    let port = peer_regular_port(&address)?;
    let host =
        optional_env("SLSK_DOWNLOAD_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let size = negotiate_download_size(&username, &host, port, timeout, token, &filename).await?;
    let remaining = usize::try_from(size)
        .map_err(|_| format!("download size too large for probe buffer: {size}"))?;
    let mut file = connect_plain_file_transfer(&username, &host, port, timeout).await?;
    let got_token = time::timeout(timeout, file.receive_token())
        .await
        .map_err(|_| "download file token timed out".to_owned())?
        .map_err(|error| format!("download file token failed: {error}"))?;
    if got_token != token {
        return Err(format!(
            "download file token mismatch: expected {token}, received {got_token}"
        ));
    }
    file.send_offset(0)
        .await
        .map_err(|error| format!("download file offset send failed: {error}"))?;
    let bytes = time::timeout(timeout, file.read_chunk(remaining))
        .await
        .map_err(|_| "download file payload timed out".to_owned())?
        .map_err(|error| format!("download file payload failed: {error}"))?;
    if let Some(expected) = expected.as_deref() {
        let text = String::from_utf8_lossy(&bytes);
        if !text.contains(expected) {
            return Err(format!(
                "download payload mismatch; expected={expected}; payload={}",
                sanitize_inline_detail(&text)
            ));
        }
    }
    let sha256 = hex_lower(&Sha256::digest(&bytes));
    if let Some(expected_sha256) = optional_env("SLSK_DOWNLOAD_SHA256") {
        if !sha256.eq_ignore_ascii_case(&expected_sha256) {
            return Err(format!(
                "download sha256 mismatch; expected={expected_sha256}; actual={sha256}"
            ));
        }
    }
    println!(
        "download peer probe completed; peer={}; filename={}; bytes={}; sha256={}",
        redact_username(&peer_username),
        filename,
        bytes.len(),
        sha256
    );
    Ok(())
}

async fn negotiate_download_size(
    username: &str,
    host: &str,
    port: u16,
    timeout: Duration,
    token: u32,
    filename: &str,
) -> Result<u64, String> {
    let attempts = env_usize("SLSK_DOWNLOAD_QUEUE_ATTEMPTS", 6)?;
    let delay = Duration::from_secs(env_u64("SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS", 3)?);
    let mut last_rejection = None;

    for attempt in 1..=attempts {
        let mut peer = connect_plain_peer_messages(username, host, port, timeout).await?;
        if attempt > 1 || env_bool("SLSK_DOWNLOAD_SEND_QUEUE_UPLOAD", true)? {
            peer.send(&PeerMessage::QueueUpload {
                filename: filename.to_owned(),
            })
            .await
            .map_err(|error| format!("download queue-upload send failed: {error}"))?;
            peer.send(&PeerMessage::PlaceInQueueRequest {
                filename: filename.to_owned(),
            })
            .await
            .map_err(|error| format!("download place-in-queue send failed: {error}"))?;
            let _ = time::timeout(Duration::from_millis(750), peer.receive()).await;
        }

        peer.send(&PeerMessage::TransferRequest(TransferRequest {
            direction: 0,
            token,
            filename: filename.to_owned(),
            size: None,
        }))
        .await
        .map_err(|error| format!("download transfer request failed: {error}"))?;
        let response = time::timeout(timeout, peer.receive())
            .await
            .map_err(|_| "download transfer response timed out".to_owned())?
            .map_err(|error| format!("download transfer response failed: {error}"))?;
        match response {
            PeerMessage::TransferResponse(TransferResponse::Allowed { token: got, size }) => {
                if got != token {
                    return Err(format!(
                        "download transfer response token mismatch: expected {token}, received {got}"
                    ));
                }
                return size
                    .ok_or_else(|| "download transfer response did not include size".to_owned());
            }
            PeerMessage::TransferResponse(TransferResponse::Rejected { token: got, reason }) => {
                if got != token {
                    return Err(format!(
                        "download transfer rejection token mismatch: expected {token}, received {got}; reason={reason}"
                    ));
                }
                let queued = reason.eq_ignore_ascii_case("queued")
                    || reason.to_ascii_lowercase().contains("queue");
                last_rejection = Some(reason.clone());
                if !queued || attempt == attempts {
                    return Err(format!(
                        "download transfer rejected; token={got}; reason={reason}; filename={filename}; attempt={attempt}/{attempts}"
                    ));
                }
                time::sleep(delay).await;
            }
            PeerMessage::PlaceInQueueResponse { place, .. } => {
                last_rejection = Some(format!("queued at place {place}"));
                if attempt == attempts {
                    return Err(format!(
                        "download remained queued; filename={filename}; place={place}; attempts={attempts}"
                    ));
                }
                time::sleep(delay).await;
            }
            other => {
                return Err(format!(
                    "unexpected download negotiation response: {other:?}"
                ))
            }
        }
    }

    Err(format!(
        "download did not become available; filename={filename}; last={}",
        last_rejection.unwrap_or_else(|| "none".to_owned())
    ))
}

#[allow(clippy::too_many_arguments)]
async fn queued_download_peer_probe(
    username: String,
    password: String,
    peer_username: String,
    filename: String,
    expected: Option<String>,
    server_address: String,
    timeout: Duration,
    token: u32,
) -> Result<(), String> {
    let listener_bind = required_env_any(&["SLSK_DOWNLOAD_LISTENER_BIND"])?;
    let listener = Listener::bind(listener_bind.as_str())
        .await
        .map_err(|error| format!("download listener bind failed: {error}"))?;
    let local_address = listener
        .local_addr()
        .map_err(|error| format!("download listener address failed: {error}"))?;
    let advertised_port = env_u16("SLSK_DOWNLOAD_ADVERTISED_PORT", local_address.port())?;

    let mut session = login_probe_session(&server_address, username.clone(), password).await?;
    session
        .set_wait_port(u32::from(advertised_port))
        .await
        .map_err(|error| format!("download wait-port update failed: {error}"))?;
    session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: peer_username.clone(),
        })
        .await
        .map_err(|error| format!("download peer-address request failed: {error}"))?;
    let address = wait_for_peer_address_response(&mut session, timeout).await?;
    let port = peer_regular_port(&address)?;
    let host =
        optional_env("SLSK_DOWNLOAD_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());

    let mut peer = connect_plain_peer_messages(&username, &host, port, timeout).await?;
    peer.send(&PeerMessage::QueueUpload {
        filename: filename.clone(),
    })
    .await
    .map_err(|error| format!("queued download queue-upload send failed: {error}"))?;
    peer.send(&PeerMessage::PlaceInQueueRequest {
        filename: filename.clone(),
    })
    .await
    .map_err(|error| format!("queued download place-in-queue send failed: {error}"))?;
    peer.send(&PeerMessage::TransferRequest(TransferRequest {
        direction: 0,
        token,
        filename: filename.clone(),
        size: None,
    }))
    .await
    .map_err(|error| format!("queued download transfer request failed: {error}"))?;

    let (remote_token, size, mut peer) = wait_for_queued_transfer_request(
        &mut session,
        &listener,
        peer,
        &peer_username,
        &filename,
        token,
        timeout,
    )
    .await?;

    peer.send(&PeerMessage::TransferResponse(TransferResponse::Allowed {
        token: remote_token,
        size: Some(size),
    }))
    .await
    .map_err(|error| format!("queued download transfer response send failed: {error}"))?;

    let (mut file, token_already_received) = wait_for_queued_file_transfer(
        &mut session,
        &listener,
        &peer_username,
        &host,
        port,
        &username,
        remote_token,
        timeout,
    )
    .await?;
    if !token_already_received {
        let got_token = time::timeout(timeout, file.receive_token())
            .await
            .map_err(|_| "queued download file token timed out".to_owned())?
            .map_err(|error| format!("queued download file token failed: {error}"))?;
        if got_token != remote_token {
            return Err(format!(
                "queued download file token mismatch: expected {remote_token}, received {got_token}"
            ));
        }
    }
    file.send_offset(0)
        .await
        .map_err(|error| format!("queued download file offset send failed: {error}"))?;
    println!("queued download offset sent; token={remote_token}; size={size}");
    let remaining = usize::try_from(size)
        .map_err(|_| format!("queued download size too large for probe buffer: {size}"))?;
    let bytes = time::timeout(timeout, file.read_chunk(remaining))
        .await
        .map_err(|_| "queued download file payload timed out".to_owned())?
        .map_err(|error| format!("queued download file payload failed: {error}"))?;
    if let Some(expected) = expected.as_deref() {
        let text = String::from_utf8_lossy(&bytes);
        if !text.contains(expected) {
            return Err(format!(
                "queued download payload mismatch; expected={expected}; payload={}",
                sanitize_inline_detail(&text)
            ));
        }
    }
    let sha256 = hex_lower(&Sha256::digest(&bytes));
    if let Some(expected_sha256) = optional_env("SLSK_DOWNLOAD_SHA256") {
        if !sha256.eq_ignore_ascii_case(&expected_sha256) {
            return Err(format!(
                "queued download sha256 mismatch; expected={expected_sha256}; actual={sha256}"
            ));
        }
    }
    println!(
        "queued download peer probe completed; peer={}; filename={}; bytes={}; sha256={}; advertised_port={advertised_port}",
        redact_username(&peer_username),
        filename,
        bytes.len(),
        sha256
    );
    Ok(())
}

async fn wait_for_queued_transfer_request(
    session: &mut ServerSession<TcpStream>,
    listener: &Listener,
    mut peer: PeerMessageConnection<TcpStream>,
    peer_username: &str,
    filename: &str,
    request_token: u32,
    timeout: Duration,
) -> Result<(u32, u64, PeerMessageConnection<TcpStream>), String> {
    let deadline = Instant::now() + timeout;
    let mut queued_seen = false;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        tokio::select! {
            peer_result = peer.receive() => {
                match peer_result.map_err(|error| format!("queued download peer receive failed: {error}"))? {
                    PeerMessage::TransferRequest(TransferRequest { direction: 1, token, filename: got_filename, size })
                        if got_filename == filename =>
                    {
                        let size = size.ok_or_else(|| "queued transfer request did not include size".to_owned())?;
                        return Ok((token, size, peer));
                    }
                    PeerMessage::TransferResponse(TransferResponse::Allowed { token: got, size }) if got == request_token => {
                        let size = size.ok_or_else(|| "download transfer response did not include size".to_owned())?;
                        return Ok((got, size, peer));
                    }
                    PeerMessage::TransferResponse(TransferResponse::Rejected { token: got, reason }) if got == request_token => {
                        if reason.eq_ignore_ascii_case("queued") || reason.to_ascii_lowercase().contains("queue") {
                            queued_seen = true;
                        } else {
                            return Err(format!("queued download rejected; token={got}; reason={reason}; filename={filename}"));
                        }
                    }
                    PeerMessage::PlaceInQueueResponse { filename: got_filename, place } if got_filename == filename => {
                        queued_seen = true;
                        println!("queued download place={place}; filename={filename}");
                    }
                    other => {
                        println!("queued download ignored peer message: {other:?}");
                    }
                }
            }
            accept_result = listener.accept() => {
                let (incoming, _) = accept_result.map_err(|error| format!("queued download listener accept failed: {error}"))?;
                let mut inbound = incoming_peer_messages(incoming, peer_username, "queued download")?;
                match inbound.receive().await.map_err(|error| format!("queued download inbound receive failed: {error}"))? {
                    PeerMessage::TransferRequest(TransferRequest { direction: 1, token, filename: got_filename, size })
                        if got_filename == filename =>
                    {
                        let size = size.ok_or_else(|| "queued inbound transfer request did not include size".to_owned())?;
                        return Ok((token, size, inbound));
                    }
                    other => return Err(format!("queued download unexpected inbound message: {other:?}")),
                }
            }
            receive_result = session.receive() => {
                handle_download_server_event(session, receive_result, None).await?;
            }
            _ = time::sleep(remaining) => break,
        }
    }

    Err(format!(
        "timed out waiting for queued transfer request; filename={filename}; queued_seen={queued_seen}"
    ))
}

#[allow(clippy::too_many_arguments)]
async fn wait_for_queued_file_transfer(
    session: &mut ServerSession<TcpStream>,
    listener: &Listener,
    peer_username: &str,
    host: &str,
    port: u16,
    username: &str,
    remote_token: u32,
    timeout: Duration,
) -> Result<(FileTransferConnection<TcpStream>, bool), String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        tokio::select! {
            accept_result = listener.accept_raw() => {
                let (stream, _) = accept_result.map_err(|error| format!("queued download file accept failed: {error}"))?;
                return classify_queued_file_stream(stream, peer_username, remote_token).await;
            }
            receive_result = session.receive() => {
                let expected = if env_bool("SLSK_DOWNLOAD_ALLOW_INDIRECT_FILE", false)? {
                    Some(remote_token)
                } else {
                    None
                };
                if let Some(connection) = handle_download_server_event(session, receive_result, expected).await? {
                    return Ok((connection, true));
                }
            }
            _ = time::sleep(remaining) => break,
        }
    }

    let mut second_chance = connect_plain_file_transfer(username, host, port, timeout).await?;
    second_chance
        .send_token(remote_token)
        .await
        .map_err(|error| format!("queued download second-chance token send failed: {error}"))?;
    Ok((second_chance, true))
}

async fn classify_queued_file_stream(
    mut stream: TcpStream,
    expected_username: &str,
    expected_token: u32,
) -> Result<(FileTransferConnection<TcpStream>, bool), String> {
    use tokio::io::AsyncReadExt;

    let first = stream
        .read_u8()
        .await
        .map_err(|error| format!("queued download file first byte failed: {error}"))?;
    if let Ok(ConnectionKind::FileTransfer) = ConnectionKind::try_from(first) {
        println!("queued download file stream classified as F-prefixed");
        return Ok((FileTransferConnection::new(stream), false));
    }
    if ConnectionKind::try_from(first).is_err() && first == expected_token.to_le_bytes()[0] {
        let mut token_bytes = [0_u8; 4];
        token_bytes[0] = first;
        stream
            .read_exact(&mut token_bytes[1..])
            .await
            .map_err(|error| format!("queued download token-first read failed: {error}"))?;
        let got = u32::from_le_bytes(token_bytes);
        if got == expected_token {
            println!("queued download file stream classified as token-first");
            return Ok((FileTransferConnection::new(stream), true));
        }
    }

    let frame = read_init_frame_with_first_len_byte(&mut stream, first)
        .await
        .map_err(|error| format!("queued download file init read failed: {error}"))?;
    match InitMessage::decode(frame)
        .map_err(|error| format!("queued download file init decode failed: {error}"))?
    {
        InitMessage::PeerInit {
            username,
            connection_type,
            token,
        } => {
            let kind = ConnectionKind::try_from_connection_type(&connection_type)
                .map_err(|error| format!("queued download file init kind failed: {error}"))?;
            if username != expected_username {
                return Err(format!(
                    "queued download file username mismatch: expected={}, received={}",
                    redact_username(expected_username),
                    redact_username(&username)
                ));
            }
            if kind != ConnectionKind::FileTransfer {
                return Err(format!(
                    "queued download file expected F init, got {kind:?}"
                ));
            }
            if token != 0 && token != expected_token {
                return Err(format!(
                    "queued download file init token mismatch: expected {expected_token}, received {token}"
                ));
            }
            println!("queued download file stream classified as peer-init");
            Ok((FileTransferConnection::new(stream), false))
        }
        other => Err(format!("queued download file unexpected init: {other:?}")),
    }
}

async fn handle_download_server_event(
    session: &mut ServerSession<TcpStream>,
    receive_result: Result<ServerMessage, slskr_client::ClientError>,
    expected_transfer_token: Option<u32>,
) -> Result<Option<FileTransferConnection<TcpStream>>, String> {
    match receive_result {
        Ok(ServerMessage::MessageUserResponse(private_message)) => {
            session
                .send_server_message(ServerMessage::MessageAcked {
                    id: private_message.id,
                })
                .await
                .map_err(|error| format!("queued download message ack failed: {error}"))?;
            Ok(None)
        }
        Ok(ServerMessage::ConnectToPeerResponse(response))
            if expected_transfer_token.is_some()
                && response.connection_type == ConnectionKind::FileTransfer.as_str() =>
        {
            let token = expected_transfer_token.expect("checked above");
            let port = u16::try_from(response.port).map_err(|_| {
                format!(
                    "queued download indirect response advertised invalid port: {}",
                    response.port
                )
            })?;
            let host = optional_env("SLSK_DOWNLOAD_INDIRECT_HOST_OVERRIDE")
                .unwrap_or_else(|| response.ip.to_string());
            let stream = time::timeout(
                Duration::from_secs(env_u64("SLSK_DOWNLOAD_INDIRECT_TIMEOUT_SECONDS", 20)?),
                TcpStream::connect((host.as_str(), port)),
            )
            .await
            .map_err(|_| "queued download indirect connect timed out".to_owned())?
            .map_err(|error| format!("queued download indirect connect failed: {error}"))?;
            let stream = send_pierce_firewall(stream, response.token)
                .await
                .map_err(|error| format!("queued download indirect pierce failed: {error}"))?;
            let mut file = FileTransferConnection::new(stream);
            file.send_token(token)
                .await
                .map_err(|error| format!("queued download indirect token send failed: {error}"))?;
            Ok(Some(file))
        }
        Ok(ServerMessage::CantConnectToPeerRequest { token, username }) => {
            println!(
                "queued download observed cant-connect request token={token}; peer={}",
                redact_username(&username)
            );
            Ok(None)
        }
        Ok(ServerMessage::CantConnectToPeerResponse { token }) => {
            println!("queued download observed cant-connect response token={token}");
            Ok(None)
        }
        Ok(ServerMessage::Relogged) => Err("account was logged in elsewhere".to_owned()),
        Ok(_) => Ok(None),
        Err(error) => Err(format!("queued download server receive failed: {error}")),
    }
}

fn incoming_peer_messages(
    incoming: IncomingConnection<TcpStream>,
    expected_username: &str,
    label: &str,
) -> Result<PeerMessageConnection<TcpStream>, String> {
    match incoming {
        IncomingConnection::PeerInit {
            username,
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } if username == expected_username => Ok(PeerMessageConnection::new(stream)),
        IncomingConnection::PeerMessages(connection) => Ok(connection),
        other => Err(format!(
            "{label} expected peer-message inbound, got {}",
            incoming_connection_name(&other)
        )),
    }
}

async fn private_message_probe() -> Result<(), String> {
    let sender_username = required_env_any(&["SLSK_USERNAME"])?;
    let sender_password = required_env_any(&["SLSK_PASSWORD"])?;
    let receiver_username = required_env_any(&["SLSK_MESSAGE_USERNAME", "SLSK_PEER_USERNAME"])?;
    let receiver_password = required_env_any(&["SLSK_MESSAGE_PASSWORD", "SLSK_PEER_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_MESSAGE_PROBE_TIMEOUT_SECONDS", 20)?);
    let message = optional_env("SLSK_MESSAGE_BODY").unwrap_or_else(|| {
        format!(
            "slskr-private-message-probe-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        )
    });

    let mut sender =
        login_probe_session(&server_address, sender_username.clone(), sender_password).await?;
    let mut receiver = login_probe_session(
        &server_address,
        receiver_username.clone(),
        receiver_password,
    )
    .await?;
    sender
        .send_server_message(ServerMessage::MessageUserRequest {
            username: receiver_username.clone(),
            message: message.clone(),
        })
        .await
        .map_err(|error| format!("private message send failed: {error}"))?;
    let id = wait_for_private_message(
        &mut receiver,
        &sender_username,
        &message,
        timeout,
        "private message",
    )
    .await?;
    receiver
        .send_server_message(ServerMessage::MessageAcked { id })
        .await
        .map_err(|error| format!("private message ack failed: {error}"))?;

    println!(
        "private message probe completed; sender={}; receiver={}; id={}",
        redact_username(&sender_username),
        redact_username(&receiver_username),
        id
    );
    Ok(())
}

async fn room_message_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_ROOM_PROBE_TIMEOUT_SECONDS", 20)?);
    let room = optional_env("SLSK_ROOM_NAME").unwrap_or_else(|| "slskr-live-interop".to_owned());
    let message = optional_env("SLSK_ROOM_MESSAGE").unwrap_or_else(|| {
        format!(
            "slskr-room-message-probe-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        )
    });

    let mut session = login_probe_session(&server_address, username.clone(), password).await?;
    session
        .send_server_message(ServerMessage::JoinRoom { room: room.clone() })
        .await
        .map_err(|error| format!("room join failed: {error}"))?;
    session
        .send_server_message(ServerMessage::SayChatroomRequest {
            room: room.clone(),
            message: message.clone(),
        })
        .await
        .map_err(|error| format!("room message send failed: {error}"))?;
    wait_for_room_message(&mut session, &room, &username, &message, timeout).await?;
    session
        .send_server_message(ServerMessage::LeaveRoom { room: room.clone() })
        .await
        .map_err(|error| format!("room leave failed: {error}"))?;

    println!(
        "room message probe completed; room={}; user={}",
        sanitize_inline_detail(&room),
        redact_username(&username)
    );
    Ok(())
}

async fn distributed_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username =
        required_env_any(&["SLSK_DISTRIBUTED_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_DISTRIBUTED_PROBE_TIMEOUT_SECONDS", 15)?);

    let address = resolve_peer_address(
        &username,
        &password,
        &peer_username,
        &server_address,
        timeout,
    )
    .await?;
    let port = peer_regular_port(&address)?;
    let host =
        optional_env("SLSK_DISTRIBUTED_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "distributed peer connect timed out".to_owned())?
        .map_err(|error| format!("distributed peer connect failed: {error}"))?;
    let stream = send_peer_init(stream, &username, ConnectionKind::Distributed)
        .await
        .map_err(|error| format!("distributed peer init failed: {error}"))?;
    let mut distributed = DistributedConnection::new(stream);

    distributed
        .send(&DistributedMessage::Ping)
        .await
        .map_err(|error| format!("distributed ping send failed: {error}"))?;

    println!(
        "distributed peer probe completed; peer={}; host_override={}",
        redact_username(&peer_username),
        optional_env("SLSK_DISTRIBUTED_HOST_OVERRIDE").is_some()
    );
    Ok(())
}

async fn file_transfer_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_FILE_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_FILE_PROBE_TIMEOUT_SECONDS", 15)?);

    let address = resolve_peer_address(
        &username,
        &password,
        &peer_username,
        &server_address,
        timeout,
    )
    .await?;
    let port = peer_regular_port(&address)?;
    let host = optional_env("SLSK_FILE_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "file-transfer peer connect timed out".to_owned())?
        .map_err(|error| format!("file-transfer peer connect failed: {error}"))?;
    let stream = send_peer_init(stream, &username, ConnectionKind::FileTransfer)
        .await
        .map_err(|error| format!("file-transfer peer init failed: {error}"))?;
    let mut transfer = FileTransferConnection::new(stream);

    let token = env_u32("SLSK_FILE_PROBE_TOKEN", 0x51ab_3001)?;
    transfer
        .send_token(token)
        .await
        .map_err(|error| format!("file-transfer token send failed: {error}"))?;
    let echoed = time::timeout(timeout, transfer.receive_token())
        .await
        .map_err(|_| "file-transfer token echo timed out".to_owned())?
        .map_err(|error| format!("file-transfer token echo failed: {error}"))?;
    if echoed != token {
        return Err(format!(
            "file-transfer token mismatch: expected {token}, received {echoed}"
        ));
    }

    println!(
        "file-transfer peer probe completed; peer={}; host_override={}",
        redact_username(&peer_username),
        optional_env("SLSK_FILE_HOST_OVERRIDE").is_some()
    );
    Ok(())
}

async fn metadata_relogin_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_PEER_USERNAME", "SLSK_OBFUSCATED_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_METADATA_RELOGIN_TIMEOUT_SECONDS", 15)?);
    let delay = Duration::from_secs(env_u64("SLSK_METADATA_RELOGIN_DELAY_SECONDS", 5)?);

    let before = resolve_peer_address(
        &username,
        &password,
        &peer_username,
        &server_address,
        timeout,
    )
    .await?;
    time::sleep(delay).await;
    let after = resolve_peer_address(
        &username,
        &password,
        &peer_username,
        &server_address,
        timeout,
    )
    .await?;

    println!(
        "metadata relogin probe completed; before_port={} before_obfuscation_type={} before_obfuscated_port={} after_port={} after_obfuscation_type={} after_obfuscated_port={}",
        before.port,
        before.obfuscation_type,
        before.obfuscated_port,
        after.port,
        after.obfuscation_type,
        after.obfuscated_port
    );
    Ok(())
}

async fn negative_indirect_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_INDIRECT_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_NEGATIVE_INDIRECT_TIMEOUT_SECONDS", 20)?);

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username, password))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;
    session
        .set_wait_port(0)
        .await
        .map_err(|error| format!("negative indirect wait-port update failed: {error}"))?;

    let token = env_u32("SLSK_NEGATIVE_INDIRECT_TOKEN", 0x51ab_4001)?;
    let request = IndirectPeerRequest::new(token, peer_username, ConnectionKind::PeerMessages);
    session
        .send_server_message(request.server_message())
        .await
        .map_err(|error| format!("negative indirect connect request failed: {error}"))?;

    wait_for_cant_connect_response(&mut session, token, timeout).await?;
    println!("negative indirect probe completed; cant-connect received");
    Ok(())
}

async fn indirect_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_INDIRECT_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = Duration::from_secs(env_u64("SLSK_INDIRECT_PROBE_TIMEOUT_SECONDS", 20)?);
    let listener_bind =
        std::env::var("SLSK_INDIRECT_LISTENER_BIND").unwrap_or_else(|_| "0.0.0.0:0".to_owned());

    let listener = Listener::bind(listener_bind.as_str())
        .await
        .map_err(|error| format!("indirect probe listener bind failed: {error}"))?;
    let local_address = listener
        .local_addr()
        .map_err(|error| format!("indirect probe listener address failed: {error}"))?;
    let advertised_port = env_u16("SLSK_INDIRECT_ADVERTISED_PORT", local_address.port())?;

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username.clone(), password))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;
    session
        .set_wait_port(u32::from(advertised_port))
        .await
        .map_err(|error| format!("indirect probe wait-port update failed: {error}"))?;

    let token = env_u32("SLSK_INDIRECT_TOKEN", 0x51ab_2001)?;
    let request =
        IndirectPeerRequest::new(token, peer_username.clone(), ConnectionKind::PeerMessages);
    session
        .send_server_message(request.server_message())
        .await
        .map_err(|error| format!("indirect connect request failed: {error}"))?;
    if env_bool("SLSK_INDIRECT_SEND_PEER_ADDRESS", false)? {
        session
            .send_server_message(ServerMessage::GetPeerAddressRequest {
                username: peer_username.clone(),
            })
            .await
            .map_err(|error| format!("indirect peer-address request failed: {error}"))?;
    }

    let (incoming, address) =
        wait_for_indirect_probe_inbound(&mut session, &listener, token, timeout).await?;
    let name = incoming_connection_name(&incoming);
    let stream = request
        .complete(incoming)
        .map_err(|error| format!("indirect probe completion failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);
    respond_to_user_info_request(&mut peer, "slskr indirect probe").await?;

    println!(
        "indirect peer probe completed; peer={}; inbound={}; from={}",
        redact_username(&peer_username),
        name,
        scrub_socket_addr(address)
    );
    Ok(())
}

async fn wait_for_indirect_probe_inbound(
    session: &mut ServerSession<TcpStream>,
    listener: &Listener,
    token: u32,
    timeout: Duration,
) -> Result<(IncomingConnection<TcpStream>, SocketAddr), String> {
    let deadline = Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("indirect probe listener accept timed out".to_owned());
        }

        tokio::select! {
            accept_result = listener.accept() => {
                return accept_result.map_err(|error| format!("indirect probe accept failed: {error}"));
            }
            receive_result = session.receive() => {
                match receive_result {
                    Ok(ServerMessage::CantConnectToPeerRequest { token: failed, .. }) if failed == token => {
                        return Err("server reported indirect connect request failure".to_owned());
                    }
                    Ok(ServerMessage::CantConnectToPeerResponse { token: failed }) if failed == token => {
                        return Err("server reported indirect connect response failure".to_owned());
                    }
                    Ok(ServerMessage::ConnectToPeerResponse(response)) if response.token == token => {
                        return Err(format!(
                            "requester received unexpected connect-to-peer response for {}:{}",
                            response.ip, response.port
                        ));
                    }
                    Ok(ServerMessage::MessageUserResponse(private_message)) => {
                        session
                            .send_server_message(ServerMessage::MessageAcked {
                                id: private_message.id,
                            })
                            .await
                            .map_err(|error| format!("indirect probe message ack failed: {error}"))?;
                    }
                    Ok(ServerMessage::Relogged) => {
                        return Err("account was logged in elsewhere".to_owned());
                    }
                    Ok(_) => {}
                    Err(error) => return Err(format!("indirect probe server receive failed: {error}")),
                }
            }
            () = time::sleep(remaining) => {
                return Err("indirect probe listener accept timed out".to_owned());
            }
        }
    }
}

async fn local_peer_smoke() -> Result<(), String> {
    let config = PeerSmokeConfig::from_env()?;
    let indirect_listener = Listener::bind(config.indirect_listener_bind.as_str())
        .await
        .map_err(|error| format!("indirect listener bind failed: {error}"))?;
    let indirect_address = indirect_listener
        .local_addr()
        .map_err(|error| format!("indirect listener address failed: {error}"))?;

    let a_connection = ServerConnection::connect(config.server_address.as_str())
        .await
        .map_err(|error| format!("account A connect failed: {error}"))?;
    let b_connection = ServerConnection::connect(config.server_address.as_str())
        .await
        .map_err(|error| format!("account B connect failed: {error}"))?;
    let mut a_session = ServerSession::new(a_connection);
    let mut b_session = ServerSession::new(b_connection);

    a_session
        .login(LoginCredentials::default_client(
            config.a_username.clone(),
            config.a_password,
        ))
        .await
        .map_err(|error| format!("account A login failed: {error}"))?;
    b_session
        .login(LoginCredentials::default_client(
            config.b_username.clone(),
            config.b_password,
        ))
        .await
        .map_err(|error| format!("account B login failed: {error}"))?;

    a_session
        .set_wait_port(u32::from(indirect_address.port()))
        .await
        .map_err(|error| format!("account A wait-port update failed: {error}"))?;

    run_direct_peer_message_smoke(&config.a_username).await?;
    run_obfuscated_peer_message_smoke(&config.a_username).await?;
    run_direct_file_transfer_smoke(&config.a_username).await?;
    run_indirect_peer_message_smoke(
        &mut a_session,
        &mut b_session,
        indirect_listener,
        &config.a_username,
        &config.b_username,
        config.indirect_host_override.as_deref(),
        config.indirect_timeout,
    )
    .await?;

    b_session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: config.a_username,
        })
        .await
        .map_err(|error| format!("peer-address request failed: {error}"))?;

    println!("local peer smoke completed");
    Ok(())
}

async fn fixture_peer_smoke() -> Result<(), String> {
    let fixture_path = std::env::var("SLSKR_FIXTURE_PEER_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("target/open-commons-fixtures/commons-click-track.ogg"));
    let bytes = fs::read(&fixture_path).map_err(|error| {
        format!(
            "fixture read failed for {}: {error}",
            fixture_path.display()
        )
    })?;
    let expected_sha256 = optional_env("SLSKR_FIXTURE_PEER_SHA256").unwrap_or_else(|| {
        "e5e09f8ef9617a355e71e2d0b00f2554201aa124a9a821c4a7f76f0441a369a0".to_owned()
    });
    let actual_sha256 = hex_lower(&Sha256::digest(&bytes));
    if !actual_sha256.eq_ignore_ascii_case(&expected_sha256) {
        return Err(format!(
            "fixture sha256 mismatch; expected={expected_sha256}; actual={actual_sha256}"
        ));
    }

    let local_username = optional_env("SLSKR_FIXTURE_PEER_USERNAME")
        .unwrap_or_else(|| "slskr-fixture-peer".to_owned());
    let virtual_filename = optional_env("SLSKR_FIXTURE_PEER_VIRTUAL_FILENAME")
        .unwrap_or_else(|| "open-commons\\commons-click-track.ogg".to_owned());
    let timeout = Duration::from_secs(env_u64("SLSKR_FIXTURE_PEER_TIMEOUT_SECONDS", 10)?);

    run_fixture_browse_smoke(&local_username, &virtual_filename, bytes.len(), timeout).await?;
    run_fixture_download_smoke(&local_username, &virtual_filename, bytes.clone(), timeout).await?;

    println!(
        "fixture peer smoke completed; file={}; bytes={}; sha256={actual_sha256}",
        fixture_path.display(),
        bytes.len()
    );
    Ok(())
}

async fn run_fixture_browse_smoke(
    local_username: &str,
    virtual_filename: &str,
    size: usize,
    timeout: Duration,
) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("fixture browse listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("fixture browse listener address failed: {error}"))?;
    let payload = build_fixture_shared_file_list_payload(virtual_filename, size)?;
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|error| format!("fixture browse accept failed: {error}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } = incoming
        else {
            return Err("fixture browse expected peer-message init".to_owned());
        };
        let mut peer = PeerMessageConnection::new(stream);
        let request = peer
            .receive()
            .await
            .map_err(|error| format!("fixture browse request receive failed: {error}"))?;
        if request != PeerMessage::GetShareFileList {
            return Err(format!("fixture browse unexpected request: {request:?}"));
        }
        peer.send(&PeerMessage::SharedFileListResponse(payload))
            .await
            .map_err(|error| format!("fixture browse response send failed: {error}"))
    });

    let mut peer =
        connect_plain_peer_messages(local_username, "127.0.0.1", address.port(), timeout).await?;
    peer.send(&PeerMessage::GetShareFileList)
        .await
        .map_err(|error| format!("fixture browse request send failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "fixture browse response timed out".to_owned())?
        .map_err(|error| format!("fixture browse response receive failed: {error}"))?;
    let decompressed = decompress_peer_share_payload(&response)
        .ok_or_else(|| format!("fixture browse unexpected response: {response:?}"))?
        .map_err(|error| format!("fixture browse decompress failed: {error}"))?;
    let preview = browse_payload_preview(&decompressed);
    if !String::from_utf8_lossy(&decompressed).contains(virtual_filename) {
        return Err(format!(
            "fixture browse payload missing filename={virtual_filename}; preview={preview}"
        ));
    }
    server_task
        .await
        .map_err(|error| format!("fixture browse server task failed: {error}"))??;
    Ok(())
}

async fn run_fixture_download_smoke(
    local_username: &str,
    virtual_filename: &str,
    bytes: Vec<u8>,
    timeout: Duration,
) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("fixture download listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("fixture download listener address failed: {error}"))?;
    let expected_size = u64::try_from(bytes.len())
        .map_err(|_| "fixture download bytes exceed u64 size".to_owned())?;
    let expected_bytes = bytes.clone();
    let server_filename = virtual_filename.to_owned();
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|error| format!("fixture download negotiation accept failed: {error}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } = incoming
        else {
            return Err("fixture download expected peer-message init".to_owned());
        };
        let mut peer = PeerMessageConnection::new(stream);
        let request = peer
            .receive()
            .await
            .map_err(|error| format!("fixture download request receive failed: {error}"))?;
        let token = match request {
            PeerMessage::TransferRequest(TransferRequest {
                direction,
                token,
                filename,
                ..
            }) if direction == 0 && filename == server_filename => token,
            other => return Err(format!("fixture download unexpected request: {other:?}")),
        };
        peer.send(&PeerMessage::TransferResponse(TransferResponse::Allowed {
            token,
            size: Some(expected_size),
        }))
        .await
        .map_err(|error| format!("fixture download response send failed: {error}"))?;

        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|error| format!("fixture download file accept failed: {error}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            stream,
            ..
        } = incoming
        else {
            return Err("fixture download expected file-transfer init".to_owned());
        };
        let mut file = FileTransferConnection::new(stream);
        file.send_token(token)
            .await
            .map_err(|error| format!("fixture download token send failed: {error}"))?;
        let offset = file
            .receive_offset()
            .await
            .map_err(|error| format!("fixture download offset receive failed: {error}"))?;
        if offset != 0 {
            return Err(format!("fixture download unexpected offset: {offset}"));
        }
        file.write_chunk(&bytes)
            .await
            .map_err(|error| format!("fixture download payload send failed: {error}"))
    });

    let mut peer =
        connect_plain_peer_messages(local_username, "127.0.0.1", address.port(), timeout).await?;
    let token = 0x51ab_7001;
    peer.send(&PeerMessage::TransferRequest(TransferRequest {
        direction: 0,
        token,
        filename: virtual_filename.to_owned(),
        size: None,
    }))
    .await
    .map_err(|error| format!("fixture download request send failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "fixture download response timed out".to_owned())?
        .map_err(|error| format!("fixture download response receive failed: {error}"))?;
    match response {
        PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: got,
            size: Some(size),
        }) if got == token && size == expected_size => {}
        other => return Err(format!("fixture download unexpected response: {other:?}")),
    }

    let mut file =
        connect_plain_file_transfer(local_username, "127.0.0.1", address.port(), timeout).await?;
    let got_token = time::timeout(timeout, file.receive_token())
        .await
        .map_err(|_| "fixture download file token timed out".to_owned())?
        .map_err(|error| format!("fixture download file token receive failed: {error}"))?;
    if got_token != token {
        return Err(format!(
            "fixture download token mismatch: expected {token}, received {got_token}"
        ));
    }
    file.send_offset(0)
        .await
        .map_err(|error| format!("fixture download offset send failed: {error}"))?;
    let downloaded = time::timeout(timeout, file.read_chunk(expected_bytes.len()))
        .await
        .map_err(|_| "fixture download payload timed out".to_owned())?
        .map_err(|error| format!("fixture download payload read failed: {error}"))?;
    if downloaded != expected_bytes {
        return Err("fixture download payload bytes differ".to_owned());
    }
    server_task
        .await
        .map_err(|error| format!("fixture download server task failed: {error}"))??;
    Ok(())
}

fn build_fixture_shared_file_list_payload(filename: &str, size: usize) -> Result<Vec<u8>, String> {
    let size = u64::try_from(size).map_err(|_| "fixture size exceeds u64".to_owned())?;
    let folder = filename
        .rsplit_once('\\')
        .map(|(folder, _)| folder)
        .unwrap_or("");
    let extension = filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    let mut writer = Writer::new();
    writer.write_u32_le(1);
    writer
        .write_string(folder)
        .map_err(|error| format!("fixture share folder encode failed: {error}"))?;
    writer.write_u32_le(1);
    let entry = FileEntry {
        code: 1,
        filename: filename.to_owned(),
        size,
        extension,
        attributes: Vec::new(),
    };
    encode_fixture_file_entry(&mut writer, &entry)?;
    compress_zlib_payload(&writer.into_inner())
        .map_err(|error| format!("fixture share payload compression failed: {error}"))
}

fn encode_fixture_file_entry(writer: &mut Writer, entry: &FileEntry) -> Result<(), String> {
    writer.write_u8(entry.code);
    writer
        .write_string(&entry.filename)
        .map_err(|error| format!("fixture filename encode failed: {error}"))?;
    writer.write_u64_le(entry.size);
    writer.write_u64_le(entry.size);
    writer
        .write_string(&entry.extension)
        .map_err(|error| format!("fixture extension encode failed: {error}"))?;
    let attribute_count = u32::try_from(entry.attributes.len())
        .map_err(|_| "too many fixture file attributes".to_owned())?;
    writer.write_u32_le(attribute_count);
    for attribute in &entry.attributes {
        writer.write_u32_le(attribute.code);
        writer.write_u32_le(attribute.value);
    }
    Ok(())
}

async fn live_soak() -> Result<(), String> {
    let config = LiveSoakConfig::from_env()?;
    let listener = Listener::bind(config.listener_bind.as_str())
        .await
        .map_err(|error| format!("listener bind failed: {error}"))?;
    let listener_address = listener
        .local_addr()
        .map_err(|error| format!("listener address failed: {error}"))?;
    let obfuscated_listener = if let Some(bind) = &config.obfuscated_listener_bind {
        let listener = Listener::bind(bind.as_str())
            .await
            .map_err(|error| format!("obfuscated listener bind failed: {error}"))?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("obfuscated listener address failed: {error}"))?;
        Some((listener, address))
    } else {
        None
    };

    let connection = ServerConnection::connect(config.server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    let info = session
        .login(LoginCredentials::default_client(
            config.username.clone(),
            config.password.clone(),
        ))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;

    let advertised_port = if std::env::var("SLSK_SOAK_ADVERTISED_PORT").is_err()
        && config.listener_bind.rsplit_once(':').map(|(_, port)| port) == Some("0")
    {
        listener_address.port()
    } else {
        config.advertised_port
    };

    if let Some((_, obfuscated_address)) = &obfuscated_listener {
        let obfuscated_advertised_port = config
            .obfuscated_advertised_port
            .unwrap_or_else(|| obfuscated_address.port());
        session
            .set_wait_port_obfuscated(
                u32::from(advertised_port),
                ROTATED_OBFUSCATION_TYPE,
                u32::from(obfuscated_advertised_port),
            )
            .await
            .map_err(|error| format!("set obfuscated wait port failed: {error}"))?;
    } else {
        session
            .set_wait_port(u32::from(advertised_port))
            .await
            .map_err(|error| format!("set wait port failed: {error}"))?;
    }
    session
        .send_server_message(ServerMessage::SetStatus { status: 2 })
        .await
        .map_err(|error| format!("set status failed: {error}"))?;
    session
        .send_server_message(ServerMessage::SharedFoldersFiles {
            folders: config.shared_folders,
            files: config.shared_files,
        })
        .await
        .map_err(|error| format!("share count update failed: {error}"))?;
    session
        .send_server_message(ServerMessage::CheckPrivilegesRequest)
        .await
        .map_err(|error| format!("check privileges failed: {error}"))?;
    session
        .send_ping()
        .await
        .map_err(|error| format!("initial ping failed: {error}"))?;

    if let Some(peer) = &config.peer_username {
        for message in peer_probe_messages(peer) {
            session
                .send_server_message(message)
                .await
                .map_err(|error| format!("peer probe failed: {error}"))?;
        }
    }

    if let Some(query) = &config.search_query {
        session
            .send_server_message(ServerMessage::FileSearchRequest(SearchRequest {
                token: config.search_token,
                query: query.clone(),
            }))
            .await
            .map_err(|error| format!("search dispatch failed: {error}"))?;
    }

    println!(
        "live soak started; supporter={}; listener={}; advertised_port={}; obfuscated_port={}; duration_seconds={}; search_enabled={}",
        info.is_supporter,
        listener_address,
        advertised_port,
        obfuscated_listener
            .as_ref()
            .map(|(_, address)| {
                config
                    .obfuscated_advertised_port
                    .unwrap_or_else(|| address.port())
                    .to_string()
            })
            .unwrap_or_else(|| "disabled".to_owned()),
        config.duration.as_secs(),
        config.search_query.is_some()
    );

    let listener_duration = config.duration;
    let listener_task =
        tokio::spawn(async move { run_listener(listener, listener_duration).await });
    let obfuscated_listener_task = obfuscated_listener.map(|(listener, _)| {
        let duration = config.duration;
        tokio::spawn(async move { run_obfuscated_listener(listener, duration).await })
    });
    let server_result = run_server_soak(&mut session, &config).await;
    let listener_result = listener_task
        .await
        .map_err(|error| format!("listener task failed: {error}"))?;
    let obfuscated_listener_result = if let Some(task) = obfuscated_listener_task {
        Some(
            task.await
                .map_err(|error| format!("obfuscated listener task failed: {error}"))?,
        )
    } else {
        None
    };

    server_result?;
    listener_result?;
    if let Some(result) = obfuscated_listener_result {
        result?;
    }
    println!("live soak completed");
    Ok(())
}

#[derive(Debug, Clone)]
struct PeerSmokeConfig {
    a_username: String,
    a_password: String,
    b_username: String,
    b_password: String,
    server_address: String,
    indirect_listener_bind: String,
    indirect_host_override: Option<String>,
    indirect_timeout: Duration,
}

impl PeerSmokeConfig {
    fn from_env() -> Result<Self, String> {
        Ok(Self {
            a_username: required_env_any(&["SLSKR_A_USERNAME", "SLSK_A_USERNAME"])?,
            a_password: required_env_any(&["SLSKR_A_PASSWORD", "SLSK_A_PASSWORD"])?,
            b_username: required_env_any(&["SLSKR_B_USERNAME", "SLSK_B_USERNAME"])?,
            b_password: required_env_any(&["SLSKR_B_PASSWORD", "SLSK_B_PASSWORD"])?,
            server_address: std::env::var("SLSK_SERVER")
                .unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned()),
            indirect_listener_bind: std::env::var("SLSKR_INDIRECT_LISTENER_BIND")
                .unwrap_or_else(|_| "0.0.0.0:0".to_owned()),
            indirect_host_override: optional_env("SLSKR_INDIRECT_HOST_OVERRIDE"),
            indirect_timeout: Duration::from_secs(env_u64("SLSKR_INDIRECT_TIMEOUT_SECONDS", 10)?),
        })
    }
}

async fn run_direct_peer_message_smoke(local_username: &str) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("peer listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("peer listener address failed: {error}"))?;
    let accept_task = tokio::spawn(async move { listener.accept().await });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("peer direct connect failed: {error}"))?;
    let stream = send_peer_init(stream, local_username, ConnectionKind::PeerMessages)
        .await
        .map_err(|error| format!("peer init send failed: {error}"))?;
    let mut outbound = PeerMessageConnection::new(stream);

    let (incoming, _) = accept_task
        .await
        .map_err(|error| format!("peer accept task failed: {error}"))?
        .map_err(|error| format!("peer accept failed: {error}"))?;
    let mut inbound = match incoming {
        IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } => PeerMessageConnection::new(stream),
        other => {
            return Err(format!(
                "unexpected peer message inbound: {}",
                incoming_connection_name(&other)
            ))
        }
    };

    outbound
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("peer user-info request send failed: {error}"))?;
    let request = inbound
        .receive()
        .await
        .map_err(|error| format!("peer user-info request receive failed: {error}"))?;
    if request != PeerMessage::UserInfoRequest {
        return Err(format!("unexpected peer message: {request:?}"));
    }

    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr local peer smoke".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .map_err(|error| format!("peer user-info response send failed: {error}"))?;
    let response = outbound
        .receive()
        .await
        .map_err(|error| format!("peer user-info response receive failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected peer response: {response:?}"));
    }

    println!("direct peer-message smoke completed");
    Ok(())
}

async fn run_obfuscated_peer_message_smoke(local_username: &str) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("obfuscated peer listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("obfuscated peer listener address failed: {error}"))?;
    let accept_task = tokio::spawn(async move { listener.accept_obfuscated().await });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("obfuscated peer direct connect failed: {error}"))?;
    let stream = send_obfuscated_peer_init(stream, local_username, ConnectionKind::PeerMessages)
        .await
        .map_err(|error| format!("obfuscated peer init send failed: {error}"))?;
    let mut outbound = ObfuscatedPeerMessageConnection::new(stream);

    let (incoming, _) = accept_task
        .await
        .map_err(|error| format!("obfuscated peer accept task failed: {error}"))?
        .map_err(|error| format!("obfuscated peer accept failed: {error}"))?;
    let mut inbound = match incoming {
        IncomingConnection::ObfuscatedPeerMessages(connection) => connection,
        other => {
            return Err(format!(
                "unexpected obfuscated peer inbound: {}",
                incoming_connection_name(&other)
            ))
        }
    };

    outbound
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("obfuscated user-info request send failed: {error}"))?;
    let request = inbound
        .receive()
        .await
        .map_err(|error| format!("obfuscated user-info request receive failed: {error}"))?;
    if request != PeerMessage::UserInfoRequest {
        return Err(format!("unexpected obfuscated peer message: {request:?}"));
    }

    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr obfuscated peer smoke".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .map_err(|error| format!("obfuscated user-info response send failed: {error}"))?;
    let response = outbound
        .receive()
        .await
        .map_err(|error| format!("obfuscated user-info response receive failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected obfuscated peer response: {response:?}"));
    }

    println!("obfuscated peer-message smoke completed");
    Ok(())
}

async fn run_direct_file_transfer_smoke(local_username: &str) -> Result<(), String> {
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("file listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("file listener address failed: {error}"))?;
    let accept_task = tokio::spawn(async move { listener.accept().await });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("file direct connect failed: {error}"))?;
    let stream = send_peer_init(stream, local_username, ConnectionKind::FileTransfer)
        .await
        .map_err(|error| format!("file peer init send failed: {error}"))?;
    let mut outbound = FileTransferConnection::new(stream);

    let (incoming, _) = accept_task
        .await
        .map_err(|error| format!("file accept task failed: {error}"))?
        .map_err(|error| format!("file accept failed: {error}"))?;
    let mut inbound = match incoming {
        IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            stream,
            ..
        } => FileTransferConnection::new(stream),
        other => {
            return Err(format!(
                "unexpected file inbound: {}",
                incoming_connection_name(&other)
            ))
        }
    };

    outbound
        .send_token(0x51ab_0001)
        .await
        .map_err(|error| format!("file token send failed: {error}"))?;
    outbound
        .send_offset(2)
        .await
        .map_err(|error| format!("file offset send failed: {error}"))?;
    outbound
        .write_chunk(b"slskr")
        .await
        .map_err(|error| format!("file chunk send failed: {error}"))?;

    let token = inbound
        .receive_token()
        .await
        .map_err(|error| format!("file token receive failed: {error}"))?;
    let offset = inbound
        .receive_offset()
        .await
        .map_err(|error| format!("file offset receive failed: {error}"))?;
    let chunk = inbound
        .read_chunk(5)
        .await
        .map_err(|error| format!("file chunk receive failed: {error}"))?;
    if token != 0x51ab_0001 || offset != 2 || chunk != b"slskr" {
        return Err("file transfer smoke payload mismatch".to_owned());
    }

    println!("direct file-transfer smoke completed");
    Ok(())
}

async fn run_indirect_peer_message_smoke(
    requester_session: &mut ServerSession<TcpStream>,
    target_session: &mut ServerSession<TcpStream>,
    listener: Listener,
    requester_username: &str,
    target_username: &str,
    host_override: Option<&str>,
    timeout: Duration,
) -> Result<(), String> {
    let token = 0x51ab_1001;
    let request = IndirectPeerRequest::new(
        token,
        target_username.to_owned(),
        ConnectionKind::PeerMessages,
    );
    requester_session
        .send_server_message(request.server_message())
        .await
        .map_err(|error| format!("indirect connect request failed: {error}"))?;
    requester_session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: target_username.to_owned(),
        })
        .await
        .map_err(|error| format!("indirect peer-address request failed: {error}"))?;

    let response = wait_for_connect_to_peer_response(target_session, token, timeout).await?;
    if response.username != requester_username {
        return Err("indirect connect response requester mismatch".to_owned());
    }
    if ConnectionKind::try_from_connection_type(&response.connection_type)
        .map_err(|error| format!("indirect response connection type failed: {error}"))?
        != ConnectionKind::PeerMessages
    {
        return Err("indirect connect response kind mismatch".to_owned());
    }

    let connect_host = host_override
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| response.ip.to_string());
    let connect_address = format!("{connect_host}:{}", response.port);
    let accept_task = tokio::spawn(async move { listener.accept().await });
    let stream = time::timeout(timeout, TcpStream::connect(connect_address.as_str()))
        .await
        .map_err(|_| "indirect peer connect timed out".to_owned())?
        .map_err(|error| format!("indirect peer connect failed: {error}"))?;
    let stream = send_pierce_firewall(stream, response.token)
        .await
        .map_err(|error| format!("indirect pierce-firewall send failed: {error}"))?;
    let mut outbound = PeerMessageConnection::new(stream);

    let (incoming, _) = time::timeout(timeout, accept_task)
        .await
        .map_err(|_| "indirect listener accept timed out".to_owned())?
        .map_err(|error| format!("indirect accept task failed: {error}"))?
        .map_err(|error| format!("indirect accept failed: {error}"))?;
    let stream = request
        .complete(incoming)
        .map_err(|error| format!("indirect completion failed: {error}"))?;
    let mut inbound = PeerMessageConnection::new(stream);

    outbound
        .send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("indirect user-info request send failed: {error}"))?;
    let peer_message = inbound
        .receive()
        .await
        .map_err(|error| format!("indirect user-info request receive failed: {error}"))?;
    if peer_message != PeerMessage::UserInfoRequest {
        return Err(format!(
            "unexpected indirect peer message: {peer_message:?}"
        ));
    }
    inbound
        .send(&PeerMessage::UserInfoResponse(UserInfo {
            description: "slskr indirect peer smoke".to_owned(),
            picture: None,
            total_uploads: 0,
            queue_size: 0,
            slots_free: true,
            upload_permissions: None,
        }))
        .await
        .map_err(|error| format!("indirect user-info response send failed: {error}"))?;
    let response = outbound
        .receive()
        .await
        .map_err(|error| format!("indirect user-info response receive failed: {error}"))?;
    if !matches!(response, PeerMessage::UserInfoResponse(_)) {
        return Err(format!("unexpected indirect peer response: {response:?}"));
    }

    println!(
        "indirect peer-message smoke completed; host_override={}; requester={}",
        host_override.is_some(),
        redact_username(requester_username)
    );
    Ok(())
}

async fn wait_for_connect_to_peer_response(
    session: &mut ServerSession<TcpStream>,
    token: u32,
    timeout: Duration,
) -> Result<ConnectToPeerResponse, String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::ConnectToPeerResponse(response))) if response.token == token => {
                return Ok(response);
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("indirect message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::CantConnectToPeerResponse { token: failed }))
                if failed == token =>
            {
                return Err("server reported indirect connect failure".to_owned());
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("indirect server receive failed: {error}")),
            Err(_) => break,
        }
    }

    Err("timed out waiting for indirect connect response".to_owned())
}

async fn wait_for_cant_connect_response(
    session: &mut ServerSession<TcpStream>,
    token: u32,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::CantConnectToPeerResponse { token: failed }))
                if failed == token =>
            {
                return Ok(());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("negative indirect message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                return Err(format!("negative indirect server receive failed: {error}"))
            }
            Err(_) => break,
        }
    }

    Err("timed out waiting for cant-connect response".to_owned())
}

async fn resolve_peer_address(
    username: &str,
    password: &str,
    peer_username: &str,
    server_address: &str,
    timeout: Duration,
) -> Result<slskr_client::protocol::server::PeerAddress, String> {
    let connection = ServerConnection::connect(server_address)
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(
            username.to_owned(),
            password.to_owned(),
        ))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;
    session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: peer_username.to_owned(),
        })
        .await
        .map_err(|error| format!("peer-address request failed: {error}"))?;
    wait_for_peer_address_response(&mut session, timeout).await
}

async fn login_probe_session(
    server_address: &str,
    username: String,
    password: String,
) -> Result<ServerSession<TcpStream>, String> {
    let connection = ServerConnection::connect(server_address)
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username, password))
        .await
        .map_err(|error| format!("login failed for configured user: {error}"))?;
    Ok(session)
}

async fn wait_for_private_message(
    session: &mut ServerSession<TcpStream>,
    sender: &str,
    body: &str,
    timeout: Duration,
    label: &str,
) -> Result<u32, String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::MessageUserResponse(private_message)))
                if private_message.username == sender && private_message.message == body =>
            {
                return Ok(private_message.id);
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("{label} unrelated message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("{label} receive failed: {error}")),
            Err(_) => break,
        }
    }
    Err(format!("{label} timed out"))
}

async fn wait_for_room_message(
    session: &mut ServerSession<TcpStream>,
    room: &str,
    username: &str,
    body: &str,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::SayChatroomResponse {
                room: got_room,
                username: got_username,
                message,
            }))
            | Ok(Ok(ServerMessage::GlobalRoomMessage {
                room: got_room,
                username: got_username,
                message,
            })) if got_room == room && got_username == username && message == body => {
                return Ok(());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("room probe unrelated message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("room probe receive failed: {error}")),
            Err(_) => break,
        }
    }
    Err("room message timed out".to_owned())
}

fn peer_regular_port(address: &slskr_client::protocol::server::PeerAddress) -> Result<u16, String> {
    if address.port == 0 {
        return Err("peer did not advertise a plain listener port".to_owned());
    }

    u16::try_from(address.port).map_err(|_| {
        format!(
            "peer advertised invalid plain listener port: {}",
            address.port
        )
    })
}

async fn connect_plain_peer_messages(
    username: &str,
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<PeerMessageConnection<TcpStream>, String> {
    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
        .await
        .map_err(|_| "plain peer connect timed out".to_owned())?
        .map_err(|error| format!("plain peer connect failed: {error}"))?;
    let stream = send_peer_init(stream, username, ConnectionKind::PeerMessages)
        .await
        .map_err(|error| format!("plain peer init failed: {error}"))?;
    Ok(PeerMessageConnection::new(stream))
}

async fn connect_plain_file_transfer(
    username: &str,
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<FileTransferConnection<TcpStream>, String> {
    let stream = time::timeout(timeout, TcpStream::connect((host, port)))
        .await
        .map_err(|_| "plain file-transfer connect timed out".to_owned())?
        .map_err(|error| format!("plain file-transfer connect failed: {error}"))?;
    let stream = send_peer_init(stream, username, ConnectionKind::FileTransfer)
        .await
        .map_err(|error| format!("plain file-transfer init failed: {error}"))?;
    Ok(FileTransferConnection::new(stream))
}

fn browse_payload_preview(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    sanitize_inline_detail(&text.chars().take(240).collect::<String>())
}

fn sanitize_inline_detail(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_graphic() || ch == ' ' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

async fn wait_for_peer_address_response(
    session: &mut ServerSession<TcpStream>,
    timeout: Duration,
) -> Result<slskr_client::protocol::server::PeerAddress, String> {
    let deadline = Instant::now() + timeout;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            session.receive(),
        )
        .await
        {
            Ok(Ok(ServerMessage::GetPeerAddressResponse(address))) => return Ok(address),
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("probe message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("probe server receive failed: {error}")),
            Err(_) => break,
        }
    }

    Err("timed out waiting for peer-address response".to_owned())
}

fn required_env_any(names: &[&str]) -> Result<String, String> {
    for name in names {
        if let Ok(value) = std::env::var(name) {
            return Ok(value);
        }
    }

    Err(format!("one of {} is required", names.join(", ")))
}

#[derive(Debug, Clone)]
struct LiveSoakConfig {
    username: String,
    password: String,
    server_address: String,
    listener_bind: String,
    advertised_port: u16,
    obfuscated_listener_bind: Option<String>,
    obfuscated_advertised_port: Option<u16>,
    duration: Duration,
    max_events: usize,
    ping_interval: Duration,
    peer_username: Option<String>,
    search_query: Option<String>,
    search_token: u32,
    shared_folders: u32,
    shared_files: u32,
}

impl LiveSoakConfig {
    fn from_env() -> Result<Self, String> {
        let listen_port = env_u16("SLSK_LISTEN_PORT", DEFAULT_LISTEN_PORT as u16)?;
        let advertised_port = env_u16("SLSK_SOAK_ADVERTISED_PORT", listen_port)?;
        Ok(Self {
            username: required_env_any(&["SLSK_USERNAME"])?,
            password: required_env_any(&["SLSK_PASSWORD"])?,
            server_address: std::env::var("SLSK_SERVER")
                .unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned()),
            listener_bind: std::env::var("SLSK_SOAK_LISTENER_BIND")
                .unwrap_or_else(|_| format!("0.0.0.0:{listen_port}")),
            advertised_port,
            obfuscated_listener_bind: optional_env("SLSK_SOAK_OBFUSCATED_LISTENER_BIND"),
            obfuscated_advertised_port: optional_env("SLSK_SOAK_OBFUSCATED_ADVERTISED_PORT")
                .map(|value| {
                    value.parse::<u16>().map_err(|error| {
                        format!("invalid SLSK_SOAK_OBFUSCATED_ADVERTISED_PORT: {error}")
                    })
                })
                .transpose()?,
            duration: Duration::from_secs(env_u64("SLSK_SOAK_SECONDS", 60)?),
            max_events: env_usize("SLSK_SOAK_MAX_EVENTS", 40)?,
            ping_interval: Duration::from_secs(env_u64("SLSK_SOAK_PING_SECONDS", 30)?),
            peer_username: optional_env("SLSK_SOAK_PEER_USERNAME"),
            search_query: optional_env("SLSK_SOAK_SEARCH_QUERY"),
            search_token: env_u32("SLSK_SOAK_SEARCH_TOKEN", 1_000_001)?,
            shared_folders: env_u32("SLSK_SOAK_SHARED_FOLDERS", 0)?,
            shared_files: env_u32("SLSK_SOAK_SHARED_FILES", 0)?,
        })
    }
}

async fn run_server_soak<S>(
    session: &mut ServerSession<S>,
    config: &LiveSoakConfig,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = Instant::now() + config.duration;
    let mut next_ping = Instant::now() + config.ping_interval;
    let mut events = 0usize;

    while Instant::now() < deadline && events < config.max_events {
        let now = Instant::now();
        let next_wait = next_ping.min(deadline).saturating_duration_since(now);

        match time::timeout(next_wait, session.receive()).await {
            Ok(Ok(message)) => {
                events += 1;
                handle_server_message(session, message).await?;
            }
            Ok(Err(error)) => return Err(format!("server receive failed: {error}")),
            Err(_) => {
                if Instant::now() >= next_ping {
                    session
                        .send_ping()
                        .await
                        .map_err(|error| format!("periodic ping failed: {error}"))?;
                    next_ping = Instant::now() + config.ping_interval;
                    println!("server ping sent");
                }
            }
        }
    }

    println!("server soak observed {events} event(s)");
    Ok(())
}

async fn handle_server_message<S>(
    session: &mut ServerSession<S>,
    message: ServerMessage,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match message {
        ServerMessage::MessageUserResponse(private_message) => {
            let id = private_message.id;
            session
                .send_server_message(ServerMessage::MessageAcked { id })
                .await
                .map_err(|error| format!("message ack failed: {error}"))?;
            println!("server event: private_message acked id={id}");
        }
        ServerMessage::CheckPrivilegesResponse { seconds } => {
            println!("server event: privileges seconds={seconds}");
        }
        ServerMessage::WatchUserResponse(user) => {
            println!("server event: watched user exists={}", user.exists);
        }
        ServerMessage::GetUserStatusResponse(status) => {
            println!(
                "server event: user status status={} privileged={}",
                status.status, status.privileged
            );
        }
        ServerMessage::GetUserStats { stats, .. } => {
            println!(
                "server event: user stats files={} dirs={}",
                stats.file_count, stats.directory_count
            );
        }
        ServerMessage::GetPeerAddressResponse(address) => {
            println!(
                "server event: peer address port={} obfuscation_type={} obfuscated_port={}",
                address.port, address.obfuscation_type, address.obfuscated_port
            );
        }
        ServerMessage::ConnectToPeerResponse(response) => {
            handle_live_soak_connect_to_peer_response(response).await?;
        }
        ServerMessage::RoomList(rooms) => {
            println!(
                "server event: room list public={} owned_private={} private={} operated_private={}",
                rooms.public_rooms.len(),
                rooms.owned_private_rooms.len(),
                rooms.private_rooms.len(),
                rooms.operated_private_rooms.len()
            );
        }
        ServerMessage::ExcludedSearchPhrases(phrases) => {
            println!("server event: excluded phrases count={}", phrases.len());
        }
        ServerMessage::FileSearchIncoming { .. } => {
            println!("server event: incoming search");
        }
        ServerMessage::PossibleParents(parents) => {
            println!("server event: possible parents count={}", parents.len());
        }
        ServerMessage::WishlistInterval { seconds } => {
            println!("server event: wishlist interval seconds={seconds}");
        }
        ServerMessage::ParentMinSpeed { speed } => {
            println!("server event: parent min speed={speed}");
        }
        ServerMessage::ParentSpeedRatio { ratio } => {
            println!("server event: parent speed ratio={ratio}");
        }
        ServerMessage::ResetDistributed => {
            println!("server event: reset distributed");
        }
        ServerMessage::Relogged => {
            return Err("account was logged in elsewhere".to_owned());
        }
        ServerMessage::Unknown { code, payload } => {
            println!(
                "server event: unknown code={code} payload_len={}",
                payload.len()
            );
        }
        other => {
            println!("server event: {}", server_message_name(&other));
        }
    }

    Ok(())
}

async fn handle_live_soak_connect_to_peer_response(
    response: ConnectToPeerResponse,
) -> Result<(), String> {
    let kind = ConnectionKind::try_from_connection_type(&response.connection_type)
        .map_err(|error| format!("connect-to-peer response kind failed: {error}"))?;

    let timeout = Duration::from_secs(env_u64("SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS", 20)?);
    let host =
        optional_env("SLSK_SOAK_INDIRECT_HOST_OVERRIDE").unwrap_or_else(|| response.ip.to_string());
    let port = u16::try_from(response.port).map_err(|_| {
        format!(
            "connect-to-peer response advertised invalid port: {}",
            response.port
        )
    })?;
    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "live soak indirect connect timed out".to_owned())?
        .map_err(|error| format!("live soak indirect connect failed: {error}"))?;
    let stream = send_pierce_firewall(stream, response.token)
        .await
        .map_err(|error| format!("live soak pierce-firewall send failed: {error}"))?;

    if kind == ConnectionKind::PeerMessages {
        let mut peer = PeerMessageConnection::new(stream);
        peer.send(&PeerMessage::UserInfoRequest)
            .await
            .map_err(|error| format!("live soak indirect user-info request failed: {error}"))?;
        let peer_response = match time::timeout(timeout, peer.receive()).await {
            Ok(Ok(message)) => message,
            Ok(Err(error)) => {
                println!("live soak indirect peer-message closed before response: {error}");
                return Ok(());
            }
            Err(_) => {
                println!("live soak indirect peer-message response timed out");
                return Ok(());
            }
        };
        match peer_response {
            PeerMessage::UserInfoResponse(_) => {}
            PeerMessage::UserInfoRequest => {
                peer.send(&PeerMessage::UserInfoResponse(UserInfo {
                    description: "slskr live soak indirect".to_owned(),
                    picture: None,
                    total_uploads: 0,
                    queue_size: 0,
                    slots_free: true,
                    upload_permissions: None,
                }))
                .await
                .map_err(|error| {
                    format!("live soak indirect user-info response send failed: {error}")
                })?;
            }
            other => {
                return Err(format!("unexpected live soak indirect response: {other:?}"));
            }
        }
    } else if kind == ConnectionKind::Distributed {
        let mut distributed = DistributedConnection::new(stream);
        distributed
            .send(&DistributedMessage::Ping)
            .await
            .map_err(|error| format!("live soak indirect distributed ping failed: {error}"))?;
    } else if kind == ConnectionKind::FileTransfer {
        let mut transfer = FileTransferConnection::new(stream);
        let token = time::timeout(timeout, transfer.receive_token())
            .await
            .map_err(|_| "live soak indirect file token timed out".to_owned())?
            .map_err(|error| format!("live soak indirect file token failed: {error}"))?;
        transfer
            .send_token(token)
            .await
            .map_err(|error| format!("live soak indirect file token echo failed: {error}"))?;
    }

    println!(
        "server event: connect_to_peer answered requester={} kind={} token={} host_override={}",
        redact_username(&response.username),
        response.connection_type,
        response.token,
        optional_env("SLSK_SOAK_INDIRECT_HOST_OVERRIDE").is_some()
    );
    Ok(())
}

async fn run_listener(listener: Listener, duration: Duration) -> Result<(), String> {
    let deadline = Instant::now() + duration;
    let mut accepted = 0usize;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            listener.accept(),
        )
        .await
        {
            Ok(Ok((incoming, address))) => {
                accepted += 1;
                let name = incoming_connection_name(&incoming);
                let response_result = handle_plain_soak_incoming(incoming).await;
                println!(
                    "listener event: {} from {}",
                    name,
                    scrub_socket_addr(address)
                );
                response_result?;
            }
            Ok(Err(error)) => return Err(format!("listener accept failed: {error}")),
            Err(_) => break,
        }
    }

    println!("listener observed {accepted} inbound connection(s)");
    Ok(())
}

async fn run_obfuscated_listener(listener: Listener, duration: Duration) -> Result<(), String> {
    let deadline = Instant::now() + duration;
    let mut accepted = 0usize;

    while Instant::now() < deadline {
        match time::timeout(
            deadline.saturating_duration_since(Instant::now()),
            listener.accept_obfuscated(),
        )
        .await
        {
            Ok(Ok((incoming, address))) => {
                accepted += 1;
                let name = incoming_connection_name(&incoming);
                let response_result = handle_obfuscated_soak_incoming(incoming).await;
                println!(
                    "obfuscated listener event: {} from {}",
                    name,
                    scrub_socket_addr(address)
                );
                response_result?;
            }
            Ok(Err(error)) => return Err(format!("obfuscated listener accept failed: {error}")),
            Err(_) => break,
        }
    }

    println!("obfuscated listener observed {accepted} inbound connection(s)");
    Ok(())
}

async fn handle_plain_soak_incoming(incoming: IncomingConnection<TcpStream>) -> Result<(), String> {
    match incoming {
        IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } => {
            let mut peer = PeerMessageConnection::new(stream);
            respond_to_user_info_request(&mut peer, "slskr live soak").await?;
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::Distributed,
            stream,
            ..
        } => {
            let mut distributed = DistributedConnection::new(stream);
            let message = time::timeout(Duration::from_secs(5), distributed.receive())
                .await
                .map_err(|_| "distributed receive timed out".to_owned())?
                .map_err(|error| format!("distributed receive failed: {error}"))?;
            if message == DistributedMessage::Ping {
                distributed
                    .send(&DistributedMessage::Ping)
                    .await
                    .map_err(|error| format!("distributed ping response failed: {error}"))?;
            }
        }
        IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            stream,
            ..
        } => {
            let mut transfer = FileTransferConnection::new(stream);
            let token = time::timeout(Duration::from_secs(5), transfer.receive_token())
                .await
                .map_err(|_| "file-transfer token receive timed out".to_owned())?
                .map_err(|error| format!("file-transfer token receive failed: {error}"))?;
            transfer
                .send_token(token)
                .await
                .map_err(|error| format!("file-transfer token echo failed: {error}"))?;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_obfuscated_soak_incoming(
    incoming: IncomingConnection<TcpStream>,
) -> Result<(), String> {
    if let IncomingConnection::ObfuscatedPeerMessages(mut peer) = incoming {
        respond_to_user_info_request(&mut peer, "slskr obfuscated live soak").await?;
    }

    Ok(())
}

async fn respond_to_user_info_request<C>(peer: &mut C, description: &str) -> Result<(), String>
where
    C: PeerUserInfoResponder,
{
    match time::timeout(Duration::from_secs(5), peer.receive_user_info_request()).await {
        Ok(Ok(true)) => {
            peer.send_user_info_response(UserInfo {
                description: description.to_owned(),
                picture: None,
                total_uploads: 0,
                queue_size: 0,
                slots_free: true,
                upload_permissions: None,
            })
            .await
        }
        Ok(Ok(false)) => Ok(()),
        Ok(Err(error)) => Err(error),
        Err(_) => Ok(()),
    }
}

trait PeerUserInfoResponder {
    async fn receive_user_info_request(&mut self) -> Result<bool, String>;
    async fn send_user_info_response(&mut self, info: UserInfo) -> Result<(), String>;
}

impl<S> PeerUserInfoResponder for PeerMessageConnection<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn receive_user_info_request(&mut self) -> Result<bool, String> {
        Ok(self
            .receive()
            .await
            .map_err(|error| format!("peer receive failed: {error}"))?
            == PeerMessage::UserInfoRequest)
    }

    async fn send_user_info_response(&mut self, info: UserInfo) -> Result<(), String> {
        self.send(&PeerMessage::UserInfoResponse(info))
            .await
            .map_err(|error| format!("peer response send failed: {error}"))
    }
}

impl<S> PeerUserInfoResponder for ObfuscatedPeerMessageConnection<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn receive_user_info_request(&mut self) -> Result<bool, String> {
        Ok(self
            .receive()
            .await
            .map_err(|error| format!("obfuscated peer receive failed: {error}"))?
            == PeerMessage::UserInfoRequest)
    }

    async fn send_user_info_response(&mut self, info: UserInfo) -> Result<(), String> {
        self.send(&PeerMessage::UserInfoResponse(info))
            .await
            .map_err(|error| format!("obfuscated peer response send failed: {error}"))
    }
}

fn peer_probe_messages(peer: &str) -> [ServerMessage; 4] {
    [
        ServerMessage::WatchUserRequest {
            username: peer.to_owned(),
        },
        ServerMessage::GetUserStatusRequest {
            username: peer.to_owned(),
        },
        ServerMessage::GetUserStatsRequest {
            username: peer.to_owned(),
        },
        ServerMessage::GetPeerAddressRequest {
            username: peer.to_owned(),
        },
    ]
}

fn incoming_connection_name<S>(incoming: &IncomingConnection<S>) -> &'static str {
    match incoming {
        IncomingConnection::PeerMessages(_) => "peer_messages",
        IncomingConnection::ObfuscatedPeerMessages(_) => "obfuscated_peer_messages",
        IncomingConnection::FileTransfer(_) => "file_transfer",
        IncomingConnection::Distributed(_) => "distributed",
        IncomingConnection::PeerInit { .. } => "peer_init",
        IncomingConnection::PierceFirewall { .. } => "pierce_firewall",
        IncomingConnection::UnknownInit { .. } => "unknown_init",
    }
}

fn server_message_name(message: &ServerMessage) -> &'static str {
    match message {
        ServerMessage::LoginRequest(_) => "login_request",
        ServerMessage::LoginResponse(_) => "login_response",
        ServerMessage::SetWaitPort(_) => "set_wait_port",
        ServerMessage::GetPeerAddressRequest { .. } => "get_peer_address_request",
        ServerMessage::GetPeerAddressResponse(_) => "get_peer_address_response",
        ServerMessage::WatchUserRequest { .. } => "watch_user_request",
        ServerMessage::WatchUserResponse(_) => "watch_user_response",
        ServerMessage::UnwatchUser { .. } => "unwatch_user",
        ServerMessage::GetUserStatusRequest { .. } => "get_user_status_request",
        ServerMessage::GetUserStatusResponse(_) => "get_user_status_response",
        ServerMessage::IgnoreUser { .. } => "ignore_user",
        ServerMessage::UnignoreUser { .. } => "unignore_user",
        ServerMessage::SayChatroomRequest { .. } => "say_chatroom_request",
        ServerMessage::SayChatroomResponse { .. } => "say_chatroom_response",
        ServerMessage::ConnectToPeerRequest(_) => "connect_to_peer_request",
        ServerMessage::ConnectToPeerResponse(_) => "connect_to_peer_response",
        ServerMessage::MessageUserRequest { .. } => "message_user_request",
        ServerMessage::MessageUserResponse(_) => "message_user_response",
        ServerMessage::MessageAcked { .. } => "message_acked",
        ServerMessage::FileSearchRequest(_) => "file_search_request",
        ServerMessage::FileSearchIncoming { .. } => "file_search_incoming",
        ServerMessage::JoinRoom { .. } => "join_room",
        ServerMessage::LeaveRoom { .. } => "leave_room",
        ServerMessage::SetStatus { .. } => "set_status",
        ServerMessage::ServerPing => "server_ping",
        ServerMessage::SharedFoldersFiles { .. } => "shared_folders_files",
        ServerMessage::GetUserStatsRequest { .. } => "get_user_stats_request",
        ServerMessage::GetUserStats { .. } => "get_user_stats",
        ServerMessage::Relogged => "relogged",
        ServerMessage::UserSearch(_) => "user_search",
        ServerMessage::RoomListRequest => "room_list_request",
        ServerMessage::RoomList(_) => "room_list",
        ServerMessage::PrivilegedUsers(_) => "privileged_users",
        ServerMessage::HaveNoParent { .. } => "have_no_parent",
        ServerMessage::ParentMinSpeed { .. } => "parent_min_speed",
        ServerMessage::ParentSpeedRatio { .. } => "parent_speed_ratio",
        ServerMessage::CheckPrivilegesRequest => "check_privileges_request",
        ServerMessage::CheckPrivilegesResponse { .. } => "check_privileges_response",
        ServerMessage::AcceptChildren { .. } => "accept_children",
        ServerMessage::PossibleParents(_) => "possible_parents",
        ServerMessage::WishlistSearch(_) => "wishlist_search",
        ServerMessage::WishlistInterval { .. } => "wishlist_interval",
        ServerMessage::RoomSearch(_) => "room_search",
        ServerMessage::SendUploadSpeed { .. } => "send_upload_speed",
        ServerMessage::BranchLevel { .. } => "branch_level",
        ServerMessage::BranchRoot { .. } => "branch_root",
        ServerMessage::ResetDistributed => "reset_distributed",
        ServerMessage::MessageUsers { .. } => "message_users",
        ServerMessage::JoinGlobalRoom => "join_global_room",
        ServerMessage::LeaveGlobalRoom => "leave_global_room",
        ServerMessage::GlobalRoomMessage { .. } => "global_room_message",
        ServerMessage::ExcludedSearchPhrases(_) => "excluded_search_phrases",
        ServerMessage::CantConnectToPeerRequest { .. } => "cant_connect_to_peer_request",
        ServerMessage::CantConnectToPeerResponse { .. } => "cant_connect_to_peer_response",
        ServerMessage::CantCreateRoom { .. } => "cant_create_room",
        ServerMessage::Unknown { .. } => "unknown",
    }
}

fn scrub_socket_addr(address: SocketAddr) -> String {
    format!(
        "{}:{}",
        if address.is_ipv4() { "ipv4" } else { "ipv6" },
        address.port()
    )
}

fn redact_username(username: &str) -> String {
    if username.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", username.chars().count())
    }
}

fn peer_address_ip_detail(
    address: &slskr_client::protocol::server::PeerAddress,
) -> Result<String, String> {
    if env_bool("SLSK_PEER_ADDRESS_SHOW_IP", false)? {
        Ok(format!(" ip={}", address.ip))
    } else {
        Ok(String::new())
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_u16(name: &str, default: u16) -> Result<u16, String> {
    env_parse(name, default)
}

fn env_u32(name: &str, default: u32) -> Result<u32, String> {
    env_parse(name, default)
}

fn env_u64(name: &str, default: u64) -> Result<u64, String> {
    env_parse(name, default)
}

fn env_usize(name: &str, default: usize) -> Result<usize, String> {
    env_parse(name, default)
}

fn env_bool(name: &str, default: bool) -> Result<bool, String> {
    let Ok(value) = std::env::var(name) else {
        return Ok(default);
    };

    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid {name}: expected boolean")),
    }
}

fn env_parse<T>(name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(value) => value
            .parse::<T>()
            .map_err(|error| format!("invalid {name}: {error}")),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        incoming_connection_name, normalize_command, peer_probe_messages, scrub_socket_addr,
    };
    use slskr_client::{
        listener::IncomingConnection, protocol::server::ServerMessage,
        stream::PeerMessageConnection,
    };
    use std::ffi::OsString;
    use std::net::{Ipv4Addr, SocketAddr};
    use tokio::io::duplex;

    fn normalize(args: &[&str]) -> Vec<String> {
        normalize_command(args.iter().map(OsString::from)).unwrap()
    }

    #[test]
    fn grouped_commands_map_to_legacy_runner_names() {
        assert_eq!(normalize(&["login", "smoke"]), ["login-smoke"]);
        assert_eq!(normalize(&["soak", "live"]), ["live-soak"]);
        assert_eq!(normalize(&["smoke", "local-peer"]), ["local-peer-smoke"]);
        assert_eq!(
            normalize(&["probe", "obfuscated-peer"]),
            ["obfuscated-peer-probe"]
        );
    }

    #[test]
    fn legacy_runner_names_still_pass_through() {
        assert_eq!(normalize(&["login-smoke"]), ["login-smoke"]);
        assert_eq!(normalize(&["plain-peer-probe"]), ["plain-peer-probe"]);
    }

    #[test]
    fn peer_probe_messages_target_same_user() {
        let messages = peer_probe_messages("peer");
        assert!(matches!(
            &messages[0],
            ServerMessage::WatchUserRequest { username } if username == "peer"
        ));
        assert!(matches!(
            &messages[3],
            ServerMessage::GetPeerAddressRequest { username } if username == "peer"
        ));
    }

    #[test]
    fn scrub_socket_addr_hides_host_address() {
        let address = SocketAddr::from((Ipv4Addr::new(192, 0, 2, 10), 2234));
        assert_eq!(scrub_socket_addr(address), "ipv4:2234");
    }

    #[test]
    fn incoming_connection_names_are_stable() {
        let (stream, _) = duplex(8);
        let incoming = IncomingConnection::PeerMessages(PeerMessageConnection::new(stream));
        assert_eq!(incoming_connection_name(&incoming), "peer_messages");
    }
}
