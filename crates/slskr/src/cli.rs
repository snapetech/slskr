use crate::probe_output::{emit_and_result, ProbeContext};
use crate::{config::TrustedMeshPeer, mesh_dht};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use slskr_client::protocol::{
    distributed::{DistributedMessage, DistributedSearch},
    init::InitMessage,
    peer::{FileEntry, PeerMessage, TransferRequest, TransferResponse, UserInfo},
    server::{ConnectToPeerResponse, JoinedRoom, SearchRequest, ServerMessage, WaitPort},
    ProtocolTextEncoding, Writer, ROTATED_OBFUSCATION_TYPE,
};
use slskr_client::{
    connection::ConnectionKind,
    distributed_tree::{DistributedEvent, DistributedTree, ParentInfo},
    file_transfer::FileTransferConnection,
    io::read_init_frame_with_first_len_byte,
    listener::{IncomingConnection, Listener},
    overlay::{connect_tls_overlay, MeshHello, MeshServiceCall, FEATURE_MESH_SERVICE},
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
use std::{
    ffi::OsString,
    fs,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::{self, Instant};
use tokio::{
    io::{duplex, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

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
        Some("direct-user-info-probe") => direct_user_info_probe().await,
        Some("browse-peer-probe") => browse_peer_probe().await,
        Some("search-peer-probe") => search_peer_probe().await,
        Some("download-peer-probe") => download_peer_probe().await,
        Some("private-message-probe") => private_message_probe().await,
        Some("room-message-probe") => room_message_probe().await,
        Some("user-watch-probe") => user_watch_probe().await,
        Some("wishlist-interval-probe") => wishlist_interval_probe().await,
        Some("distributed-peer-probe") => distributed_peer_probe().await,
        Some("file-transfer-peer-probe") => file_transfer_peer_probe().await,
        Some("metadata-relogin-probe") => metadata_relogin_probe().await,
        Some("negative-indirect-probe") => negative_indirect_probe().await,
        Some("peer-address-probe") => peer_address_probe().await,
        Some("overlay-service-probe") => overlay_service_probe().await,
        Some("dht-store-probe") => dht_store_probe().await,
        Some("fixture-peer-smoke") => fixture_peer_smoke().await,
        Some("distributed-tree-smoke") => distributed_tree_smoke().await,
        Some("room-create-smoke") => room_create_smoke().await,
        Some("server-relogin-smoke") => server_relogin_smoke().await,
        Some("server-reconnect-smoke") => server_reconnect_smoke().await,
        Some("closed-listener-smoke") => closed_listener_smoke().await,
        Some("bad-obfuscation-type-smoke") => bad_obfuscation_type_smoke().await,
        Some("malformed-peer-response-smoke") => malformed_peer_response_smoke().await,
        Some("transfer-resume-smoke") => transfer_resume_smoke().await,
        Some("transfer-reject-smoke") => transfer_reject_smoke().await,
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
        "smoke" if args.get(1).map(String::as_str) == Some("distributed-tree") => {
            vec!["distributed-tree-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("room-create") => {
            vec!["room-create-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("server-relogin") => {
            vec!["server-relogin-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("server-reconnect") => {
            vec!["server-reconnect-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("closed-listener") => {
            vec!["closed-listener-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("bad-obfuscation-type") => {
            vec!["bad-obfuscation-type-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("malformed-peer-response") => {
            vec!["malformed-peer-response-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("transfer-resume") => {
            vec!["transfer-resume-smoke"]
        }
        "smoke" if args.get(1).map(String::as_str) == Some("transfer-reject") => {
            vec!["transfer-reject-smoke"]
        }
        "probe" => match args.get(1).map(String::as_str) {
            Some("peer-address") => vec!["peer-address-probe"],
            Some("overlay-service") => vec!["overlay-service-probe"],
            Some("dht-store") => vec!["dht-store-probe"],
            Some("plain-peer") => vec!["plain-peer-probe"],
            Some("browse-peer") => vec!["browse-peer-probe"],
            Some("search-peer") => vec!["search-peer-probe"],
            Some("download-peer") => vec!["download-peer-probe"],
            Some("private-message") => vec!["private-message-probe"],
            Some("room-message") => vec!["room-message-probe"],
            Some("user-watch") => vec!["user-watch-probe"],
            Some("wishlist-interval") => vec!["wishlist-interval-probe"],
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
  SLSKR_OVERLAY_ENDPOINT=<ip:port> SLSKR_OVERLAY_CERTIFICATE_SHA256=<hex> SLSK_USERNAME=<user> SLSK_PEER_USERNAME=<peer> slskr probe overlay-service
  SLSKR_OVERLAY_ENDPOINT=<ip:port> SLSKR_OVERLAY_CERTIFICATE_SHA256=<hex> SLSK_USERNAME=<user> SLSK_PEER_USERNAME=<peer> slskr probe dht-store
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe plain-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe browse-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> SLSK_SEARCH_QUERY=<query> slskr probe search-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> SLSK_DOWNLOAD_FILENAME='Share\\File.txt' slskr probe download-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_MESSAGE_USERNAME=<user2> SLSK_MESSAGE_PASSWORD=<pass2> slskr probe private-message
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr probe room-message
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe user-watch
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr probe wishlist-interval
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_OBFUSCATED_PEER_USERNAME=<peer> slskr probe obfuscated-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe indirect-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe distributed-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe file-transfer-peer
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe metadata-relogin
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> slskr probe negative-indirect
  SLSKR_A_USERNAME=<user> SLSKR_A_PASSWORD=<pass> SLSKR_B_USERNAME=<user> SLSKR_B_PASSWORD=<pass> slskr smoke local-peer
  slskr smoke fixture-peer
  slskr smoke distributed-tree
  slskr smoke room-create
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr smoke server-relogin
  SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> slskr smoke server-reconnect
  slskr smoke closed-listener
  slskr smoke bad-obfuscation-type
  slskr smoke malformed-peer-response"
}

async fn overlay_service_probe() -> Result<(), String> {
    let endpoint = required_env_any(&["SLSKR_OVERLAY_ENDPOINT"])?
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid SLSKR_OVERLAY_ENDPOINT: {error}"))?;
    let certificate_hex = required_env_any(&["SLSKR_OVERLAY_CERTIFICATE_SHA256"])?;
    let certificate_bytes = hex::decode(&certificate_hex)
        .map_err(|_| "SLSKR_OVERLAY_CERTIFICATE_SHA256 must be 64 hexadecimal digits".to_owned())?;
    let certificate_sha256: [u8; 32] = certificate_bytes
        .try_into()
        .map_err(|_| "SLSKR_OVERLAY_CERTIFICATE_SHA256 must be 64 hexadecimal digits".to_owned())?;
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let peer_username = required_env_any(&["SLSK_PEER_USERNAME"])?;
    let service_name = std::env::var("SLSKR_OVERLAY_SERVICE").unwrap_or_else(|_| "dht".to_owned());
    let method = std::env::var("SLSKR_OVERLAY_METHOD").unwrap_or_else(|_| "Ping".to_owned());
    let payload = std::env::var("SLSKR_OVERLAY_PAYLOAD")
        .unwrap_or_else(|_| r#"{"RequesterId":"AAAAAAAAAAAAAAAAAAAAAAAAAAA="}"#.to_owned())
        .into_bytes();
    let expected = std::env::var("SLSKR_OVERLAY_EXPECTED")
        .ok()
        .filter(|value| !value.is_empty());
    let expected_sha256 = std::env::var("SLSKR_OVERLAY_EXPECTED_SHA256")
        .ok()
        .filter(|value| !value.is_empty());
    let ctx = ProbeContext::new("overlay-service").with_peer(&peer_username);

    let hello = MeshHello::new(
        username,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        uuid::Uuid::new_v4().simple().to_string(),
    )
    .map_err(|error| format!("overlay hello failed: {error}"))?;
    let mut client = connect_tls_overlay(endpoint, certificate_sha256, hello)
        .await
        .map_err(|error| format!("overlay connect failed: {error}"))?;
    if !client.remote_username.eq_ignore_ascii_case(&peer_username) {
        return emit_and_result(ctx.fail("overlay acknowledgement username mismatch"));
    }
    let call = MeshServiceCall::new(
        uuid::Uuid::new_v4().to_string(),
        service_name.clone(),
        method.clone(),
        payload,
    )
    .map_err(|error| format!("overlay service call failed: {error}"))?;
    let reply = client
        .call(&call)
        .await
        .map_err(|error| format!("overlay service call failed: {error}"))?;
    if reply.status_code != 0 {
        return emit_and_result(ctx.fail(format!(
            "overlay service rejected call with status {}: {}",
            reply.status_code,
            reply.error_message.as_deref().unwrap_or("remote error")
        )));
    }
    let response_sha256 = hex::encode(Sha256::digest(&reply.payload));
    if expected_sha256
        .as_deref()
        .is_some_and(|expected| !response_sha256.eq_ignore_ascii_case(expected.trim()))
    {
        return emit_and_result(ctx.fail(format!(
            "overlay service response SHA-256 mismatch: expected {}; received {response_sha256}",
            expected_sha256.as_deref().unwrap_or_default().trim()
        )));
    }
    if let Some(expected) = expected.as_deref() {
        let response = String::from_utf8(reply.payload.clone())
            .map_err(|_| "overlay service response was not UTF-8".to_owned())?;
        if !response.contains(expected) {
            return emit_and_result(
                ctx.fail("overlay service response did not contain expected text"),
            );
        }
        println!("{response}");
    } else if expected_sha256.is_some() {
        println!(
            "response_bytes={} response_sha256={response_sha256}",
            reply.payload.len()
        );
    } else {
        let response = String::from_utf8(reply.payload)
            .map_err(|_| "overlay service response was not UTF-8".to_owned())?;
        println!("{response}");
    }
    emit_and_result(ctx.ok(format!("{service_name}.{method} succeeded")))
}

async fn dht_store_probe() -> Result<(), String> {
    let endpoint = required_env_any(&["SLSKR_OVERLAY_ENDPOINT"])?
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid SLSKR_OVERLAY_ENDPOINT: {error}"))?;
    let certificate_hex = required_env_any(&["SLSKR_OVERLAY_CERTIFICATE_SHA256"])?;
    let certificate_sha256: [u8; 32] = hex::decode(&certificate_hex)
        .map_err(|_| "SLSKR_OVERLAY_CERTIFICATE_SHA256 must be hexadecimal".to_owned())?
        .try_into()
        .map_err(|_| "SLSKR_OVERLAY_CERTIFICATE_SHA256 must be 64 hexadecimal digits".to_owned())?;
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let peer_username = required_env_any(&["SLSK_PEER_USERNAME"])?;
    let peer = TrustedMeshPeer {
        peer_id: peer_username.clone(),
        username: peer_username.clone(),
        overlay_endpoint: endpoint,
        certificate_sha256,
        range_endpoint: None,
    };
    let signing_key = SigningKey::from_bytes(&[0x2a; 32]);
    let ctx = ProbeContext::new("dht-store").with_peer(&peer_username);
    if let Err(error) = mesh_dht::probe_store(&peer, &username, &signing_key).await {
        return emit_and_result(ctx.fail(error));
    }
    emit_and_result(ctx.ok("authenticated signed DHT Store accepted"))
}

async fn peer_address_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_PEER_USERNAME", "SLSK_OBFUSCATED_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_PEER_ADDRESS_PROBE_TIMEOUT_SECONDS", 10, false)?;
    let attempts = env_usize("SLSK_PEER_ADDRESS_PROBE_ATTEMPTS", 5)?;

    let ctx = ProbeContext::new("peer-address").with_peer(&peer_username);

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    session
        .login(LoginCredentials::default_client(username, password))
        .await
        .map_err(|error| {
            let msg = error.to_string();
            let _ = emit_and_result(ctx.fail(msg.clone()));
            format!("login failed for configured user: {msg}")
        })?;

    for attempt in 1..=attempts {
        session
            .send_server_message(ServerMessage::GetPeerAddressRequest {
                username: peer_username.clone(),
            })
            .await
            .map_err(|error| format!("peer-address request failed: {error}"))?;
        let address = wait_for_peer_address_response(&mut session, timeout).await?;
        let detail = format!(
            "peer address attempt={attempt}{} port={} obfuscation_type={} obfuscated_port={}",
            peer_address_ip_detail(&address)?,
            address.port,
            address.obfuscation_type,
            address.obfuscated_port
        );
        println!("{detail}");
        if attempt < attempts {
            time::sleep(Duration::from_secs(2)).await;
        }
    }

    emit_and_result(ctx.ok("peer address resolved"))
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

    let ctx = ProbeContext::new("login-smoke").with_peer(&username);

    let connection = ServerConnection::connect(server_address.as_str())
        .await
        .map_err(|error| format!("connect failed: {error}"))?;
    let mut session = ServerSession::new(connection);
    let info = session
        .login(LoginCredentials::default_client(username.clone(), password))
        .await
        .map_err(|error| {
            let msg = error.to_string();
            let _ = emit_and_result(ctx.fail(msg.clone()));
            format!("login failed for {username}: {msg}")
        })?;
    session
        .set_wait_port(listen_port)
        .await
        .map_err(|error| format!("set wait port failed: {error}"))?;
    session
        .send_ping()
        .await
        .map_err(|error| format!("ping failed: {error}"))?;

    let detail = format!("logged in; supporter={}", info.is_supporter);
    println!("{detail}");
    emit_and_result(ctx.ok(&detail))
}

async fn obfuscated_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_OBFUSCATED_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_OBFUSCATED_PROBE_TIMEOUT_SECONDS", 15, false)?;

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
            Ok(candidate)
                if validated_obfuscated_port(
                    candidate.obfuscation_type,
                    candidate.obfuscated_port,
                )
                .is_ok() =>
            {
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
    let obfuscated_port =
        validated_obfuscated_port(address.obfuscation_type, address.obfuscated_port)?;

    let host =
        optional_env("SLSK_OBFUSCATED_HOST_OVERRIDE").unwrap_or_else(|| address.ip.to_string());
    let stream = time::timeout(
        timeout,
        TcpStream::connect((host.as_str(), obfuscated_port)),
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
    let timeout = env_duration_secs("SLSK_PLAIN_PROBE_TIMEOUT_SECONDS", 15, false)?;

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

async fn direct_user_info_probe() -> Result<(), String> {
    let host = required_env_any(&["SLSK_DIRECT_PEER_HOST"])?;
    let port = required_env_any(&["SLSK_DIRECT_PEER_PORT"])?
        .parse::<u16>()
        .map_err(|error| format!("invalid SLSK_DIRECT_PEER_PORT: {error}"))?;
    let username = optional_env("SLSK_DIRECT_PEER_USERNAME")
        .unwrap_or_else(|| "slskr-description-probe".to_owned());
    let token = env_u32("SLSK_DIRECT_PEER_INIT_TOKEN", 0)?;
    let timeout = env_duration_secs("SLSK_DIRECT_PEER_TIMEOUT_SECONDS", 5, false)?;
    let stream = time::timeout(timeout, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| "direct peer connect timed out".to_owned())?
        .map_err(|error| format!("direct peer connect failed: {error}"))?;
    let stream = send_peer_init_with_token(stream, &username, ConnectionKind::PeerMessages, token)
        .await
        .map_err(|error| format!("direct peer init failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);
    peer.send(&PeerMessage::UserInfoRequest)
        .await
        .map_err(|error| format!("direct user-info request failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "direct user-info response timed out".to_owned())?
        .map_err(|error| format!("direct user-info response failed: {error}"))?;
    match response {
        PeerMessage::UserInfoResponse(info) => {
            if optional_env("SLSK_DIRECT_USER_INFO_INCLUDE_PICTURE")
                .as_deref()
                .is_some_and(|value| matches!(value, "1" | "true" | "TRUE"))
            {
                println!(
                    "{}",
                    serde_json::json!({
                        "description": info.description,
                        "pictureHex": info.picture.as_deref().map(hex::encode),
                    })
                );
            } else {
                println!("{}", serde_json::Value::String(info.description));
            }
            Ok(())
        }
        response => Err(format!("unexpected direct peer response: {response:?}")),
    }
}

async fn browse_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_BROWSE_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let expected = optional_env("SLSK_BROWSE_EXPECTED");
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_BROWSE_PROBE_TIMEOUT_SECONDS", 20, false)?;

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

async fn search_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let peer_username = required_env_any(&["SLSK_SEARCH_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let query = required_env_any(&["SLSK_SEARCH_QUERY"])?;
    let expected = optional_env("SLSK_SEARCH_EXPECTED").unwrap_or_else(|| query.clone());
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_SEARCH_PROBE_TIMEOUT_SECONDS", 20, false)?;
    let token = env_u32("SLSK_SEARCH_TOKEN", 0x51ab_5001)?;
    let attempts = env_u32("SLSK_SEARCH_PROBE_ATTEMPTS", 1)?.max(1);

    let host_override = optional_env("SLSK_SEARCH_HOST_OVERRIDE");
    let port_override = optional_env("SLSK_SEARCH_PORT_OVERRIDE");
    let force_login = env_bool("SLSK_SEARCH_FORCE_LOGIN", false)?;
    let (host, port) = match (host_override, port_override, force_login) {
        (Some(host), Some(port), false) => {
            let port = port
                .parse::<u16>()
                .map_err(|error| format!("invalid SLSK_SEARCH_PORT_OVERRIDE: {error}"))?;
            (host, port)
        }
        (host_override, port_override, _) => {
            let password = required_env_any(&["SLSK_PASSWORD"])?;
            let address = resolve_peer_address(
                &username,
                &password,
                &peer_username,
                &server_address,
                timeout,
            )
            .await?;
            let port = match port_override {
                Some(value) => value
                    .parse::<u16>()
                    .map_err(|error| format!("invalid SLSK_SEARCH_PORT_OVERRIDE: {error}"))?,
                None => peer_regular_port(&address)?,
            };
            let host = host_override.unwrap_or_else(|| address.ip.to_string());
            (host, port)
        }
    };
    let mut last_error = None;
    let mut response = None;
    for attempt in 1..=attempts {
        match search_peer_once(&username, &host, port, timeout, token, &query).await {
            Ok(value) => {
                response = Some(value);
                break;
            }
            Err(error) => {
                last_error = Some(error);
                if attempt < attempts {
                    time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
    let response = response.ok_or_else(|| {
        last_error.unwrap_or_else(|| "search response failed without an error".to_owned())
    })?;
    let found = response
        .results
        .iter()
        .chain(response.private_results.iter())
        .any(|entry| entry.filename.contains(&expected));
    if !found {
        return Err(format!(
            "search response missing expected fixture; expected={expected}; results={:?}; private_results={:?}",
            response.results, response.private_results
        ));
    }

    println!(
        "search peer probe completed; peer={}; results={}; private_results={}",
        redact_username(&peer_username),
        response.results.len(),
        response.private_results.len()
    );
    Ok(())
}

async fn search_peer_once(
    username: &str,
    host: &str,
    port: u16,
    timeout: Duration,
    token: u32,
    query: &str,
) -> Result<slskr_client::protocol::peer::FileSearchResponse, String> {
    let mut peer = connect_plain_peer_messages(username, host, port, timeout).await?;
    peer.send(&PeerMessage::FileSearchRequest {
        token,
        query: query.to_owned(),
    })
    .await
    .map_err(|error| format!("search request failed: {error}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "search response timed out".to_owned())?
        .map_err(|error| format!("search response failed: {error}"))?;
    let PeerMessage::FileSearchResponse(response) = response else {
        return Err(format!("unexpected search response: {response:?}"));
    };
    Ok(response)
}

async fn download_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_DOWNLOAD_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let filename = required_env_any(&["SLSK_DOWNLOAD_FILENAME"])?;
    let expected = optional_env("SLSK_DOWNLOAD_EXPECTED");
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_DOWNLOAD_PROBE_TIMEOUT_SECONDS", 30, false)?;
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
    let delay = env_duration_secs("SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS", 3, true)?;
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
            filename_encoding: ProtocolTextEncoding::Utf8,
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
                        "download transfer rejection token mismatch: expected {token}, received {got}; reason={}",
                        redact_peer_text(&reason)
                    ));
                }
                let queued = reason.eq_ignore_ascii_case("queued")
                    || reason.to_ascii_lowercase().contains("queue");
                last_rejection = Some(redact_peer_text(&reason));
                if !queued || attempt == attempts {
                    return Err(format!(
                        "download transfer rejected; token={got}; reason={}; filename={filename}; attempt={attempt}/{attempts}",
                        redact_peer_text(&reason)
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
                    "unexpected download negotiation response: {}",
                    peer_message_name(&other)
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
        filename_encoding: ProtocolTextEncoding::Utf8,
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
                    PeerMessage::TransferRequest(TransferRequest { direction: 1, token, filename: got_filename, size, .. })
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
                            return Err(format!(
                                "queued download rejected; token={got}; reason={}; filename={filename}",
                                redact_peer_text(&reason)
                            ));
                        }
                    }
                    PeerMessage::PlaceInQueueResponse { filename: got_filename, place } if got_filename == filename => {
                        queued_seen = true;
                        println!("queued download place={place}; filename={filename}");
                    }
                    other => {
                        println!(
                            "queued download ignored peer message: {}",
                            peer_message_name(&other)
                        );
                    }
                }
            }
            accept_result = listener.accept() => {
                let (incoming, _) = accept_result.map_err(|error| format!("queued download listener accept failed: {error}"))?;
                let mut inbound = incoming_peer_messages(incoming, peer_username, "queued download")?;
                match inbound.receive().await.map_err(|error| format!("queued download inbound receive failed: {error}"))? {
                    PeerMessage::TransferRequest(TransferRequest { direction: 1, token, filename: got_filename, size, .. })
                        if got_filename == filename =>
                    {
                        let size = size.ok_or_else(|| "queued inbound transfer request did not include size".to_owned())?;
                        return Ok((token, size, inbound));
                    }
                    other => return Err(format!(
                        "queued download unexpected inbound message: {}",
                        peer_message_name(&other)
                    )),
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
        _ => Err("queued download file received unexpected init message".to_owned()),
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
                env_duration_secs("SLSK_DOWNLOAD_INDIRECT_TIMEOUT_SECONDS", 20, false)?,
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
    let timeout = env_duration_secs("SLSK_MESSAGE_PROBE_TIMEOUT_SECONDS", 20, false)?;
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
    let timeout = env_duration_secs("SLSK_ROOM_PROBE_TIMEOUT_SECONDS", 20, false)?;
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
        .send_server_message(ServerMessage::JoinRoom {
            room: room.clone(),
            private: false,
        })
        .await
        .map_err(|error| format!("room join failed: {error}"))?;
    wait_for_room_join(&mut session, &room, timeout).await?;
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

async fn user_watch_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let watched_username = required_env_any(&["SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_USER_WATCH_PROBE_TIMEOUT_SECONDS", 20, false)?;
    let ctx = ProbeContext::new("user-watch").with_peer(&watched_username);
    let mut session = login_probe_session(&server_address, username, password).await?;

    session
        .send_server_message(ServerMessage::WatchUserRequest {
            username: watched_username.clone(),
        })
        .await
        .map_err(|error| format!("watch-user request failed: {error}"))?;
    session
        .send_server_message(ServerMessage::GetUserStatsRequest {
            username: watched_username.clone(),
        })
        .await
        .map_err(|error| format!("user-stats request failed: {error}"))?;

    let deadline = Instant::now() + timeout;
    let mut watched = false;
    let mut stats = None;
    while !(watched && stats.is_some()) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return emit_and_result(ctx.fail("watch-user and user-stats responses timed out"));
        }
        match time::timeout(remaining, session.receive()).await {
            Ok(Ok(ServerMessage::WatchUserResponse(user)))
                if user.username.eq_ignore_ascii_case(&watched_username) =>
            {
                if !user.exists {
                    return emit_and_result(ctx.fail("watched user does not exist"));
                }
                watched = true;
            }
            Ok(Ok(ServerMessage::GetUserStats {
                username: response_username,
                stats: response_stats,
            })) if response_username.eq_ignore_ascii_case(&watched_username) => {
                stats = Some(response_stats);
            }
            Ok(Ok(ServerMessage::MessageUserResponse(message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked { id: message.id })
                    .await
                    .map_err(|error| {
                        format!("user-watch message acknowledgement failed: {error}")
                    })?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("user-watch receive failed: {error}")),
            Err(_) => {
                return emit_and_result(ctx.fail("watch-user and user-stats responses timed out"));
            }
        }
    }

    let stats = stats.expect("loop exits only after user stats are received");
    emit_and_result(ctx.ok(format!(
        "WatchUser exists; files={}; directories={}",
        stats.file_count, stats.directory_count
    )))
}

async fn wishlist_interval_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_WISHLIST_PROBE_TIMEOUT_SECONDS", 30, false)?;
    let query = optional_env("SLSK_WISHLIST_QUERY").unwrap_or_else(|| {
        format!(
            "slskr-wishlist-live-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        )
    });
    let token = env_u32("SLSK_WISHLIST_TOKEN", 0x51ab_4001)?;
    if token == 0 {
        return Err("SLSK_WISHLIST_TOKEN must be nonzero".to_owned());
    }

    let mut session = login_probe_session(&server_address, username.clone(), password).await?;
    let deadline = Instant::now() + timeout;
    let interval_seconds = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("wishlist interval probe timed out".to_owned());
        }
        match time::timeout(remaining, session.receive()).await {
            Ok(Ok(ServerMessage::WishlistInterval { seconds })) if seconds > 0 => break seconds,
            Ok(Ok(ServerMessage::WishlistInterval { .. })) => {
                return Err("server advertised a zero wishlist interval".to_owned());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("wishlist probe message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                return Err(format!("wishlist interval receive failed: {error}"));
            }
            Err(_) => return Err("wishlist interval probe timed out".to_owned()),
        }
    };

    session
        .send_server_message(ServerMessage::WishlistSearch(SearchRequest {
            token,
            query: query.clone(),
        }))
        .await
        .map_err(|error| format!("wishlist search send failed: {error}"))?;

    match time::timeout(Duration::from_secs(2), session.receive()).await {
        Ok(Ok(ServerMessage::Relogged)) => {
            return Err("account was logged in elsewhere".to_owned());
        }
        Ok(Err(error)) => return Err(format!("wishlist search was rejected: {error}")),
        Ok(Ok(_)) | Err(_) => {}
    }

    let ctx = ProbeContext::new("wishlist-interval");
    emit_and_result(ctx.ok(format!(
        "server interval={interval_seconds}s; WishlistSearch token={token} sent; query_bytes={}",
        query.len()
    )))
}

async fn distributed_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username =
        required_env_any(&["SLSK_DISTRIBUTED_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_DISTRIBUTED_PROBE_TIMEOUT_SECONDS", 15, false)?;

    let ctx = ProbeContext::new("distributed-peer").with_peer(&peer_username);

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

    emit_and_result(ctx.ok(format!(
        "distributed peer probe completed; host_override={}",
        optional_env("SLSK_DISTRIBUTED_HOST_OVERRIDE").is_some()
    )))
}

async fn file_transfer_peer_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_FILE_PEER_USERNAME", "SLSK_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_FILE_PROBE_TIMEOUT_SECONDS", 15, false)?;

    let ctx = ProbeContext::new("file-transfer-peer").with_peer(&peer_username);

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
        return emit_and_result(ctx.fail(format!(
            "file-transfer token mismatch: expected {token}, received {echoed}"
        )));
    }

    emit_and_result(ctx.ok(format!(
        "file-transfer peer probe completed; host_override={}",
        optional_env("SLSK_FILE_HOST_OVERRIDE").is_some()
    )))
}

async fn metadata_relogin_probe() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let peer_username = required_env_any(&["SLSK_PEER_USERNAME", "SLSK_OBFUSCATED_PEER_USERNAME"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSK_METADATA_RELOGIN_TIMEOUT_SECONDS", 15, false)?;
    let delay = env_duration_secs("SLSK_METADATA_RELOGIN_DELAY_SECONDS", 5, true)?;

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
    let timeout = env_duration_secs("SLSK_NEGATIVE_INDIRECT_TIMEOUT_SECONDS", 20, false)?;

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
    let timeout = env_duration_secs("SLSK_INDIRECT_PROBE_TIMEOUT_SECONDS", 20, false)?;
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
    let timeout = env_duration_secs("SLSKR_FIXTURE_PEER_TIMEOUT_SECONDS", 10, false)?;

    run_fixture_browse_smoke(&local_username, &virtual_filename, bytes.len(), timeout).await?;
    run_fixture_download_smoke(&local_username, &virtual_filename, bytes.clone(), timeout).await?;

    println!(
        "fixture peer smoke completed; file={}; bytes={}; sha256={actual_sha256}",
        fixture_path.display(),
        bytes.len()
    );
    Ok(())
}

async fn distributed_tree_smoke() -> Result<(), String> {
    let timeout = Duration::from_secs(5);
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("distributed listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("distributed listener address failed: {error}"))?;
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|error| format!("distributed listener accept failed: {error}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::Distributed,
            stream,
            ..
        } = incoming
        else {
            return Err("distributed listener received the wrong connection kind".to_owned());
        };
        let mut connection = DistributedConnection::new(stream);
        let message = connection
            .receive()
            .await
            .map_err(|error| format!("distributed listener receive failed: {error}"))?;
        if message != DistributedMessage::Ping {
            return Err("distributed listener did not receive Ping".to_owned());
        }
        connection
            .send(&DistributedMessage::Ping)
            .await
            .map_err(|error| format!("distributed listener response failed: {error}"))
    });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("distributed fixture connect failed: {error}"))?;
    let stream = send_peer_init(
        stream,
        "slskr-distributed-fixture",
        ConnectionKind::Distributed,
    )
    .await
    .map_err(|error| format!("distributed fixture init failed: {error}"))?;
    let mut connection = DistributedConnection::new(stream);
    connection
        .send(&DistributedMessage::Ping)
        .await
        .map_err(|error| format!("distributed fixture ping failed: {error}"))?;
    let response = time::timeout(timeout, connection.receive())
        .await
        .map_err(|_| "distributed fixture response timed out".to_owned())?
        .map_err(|error| format!("distributed fixture response failed: {error}"))?;
    if response != DistributedMessage::Ping {
        return Err("distributed fixture returned a non-Ping response".to_owned());
    }
    time::timeout(timeout, server_task)
        .await
        .map_err(|_| "distributed listener task timed out".to_owned())?
        .map_err(|error| format!("distributed listener task failed: {error}"))??;

    let (parent_side, parent_peer) = duplex(2_048);
    let mut tree = DistributedTree::new("local");
    let mut parent_peer = DistributedConnection::new(parent_peer);
    tree.connect_parent(
        ParentInfo {
            username: "parent".to_owned(),
            ip: Ipv4Addr::LOCALHOST,
            port: u32::from(address.port()),
        },
        DistributedConnection::new(parent_side),
    );
    if tree.branch_level() != 1 || tree.branch_root() != "parent" || tree.parent().is_none() {
        return Err("distributed parent adoption did not update branch state".to_owned());
    }
    if !tree
        .send_branch_info_to_parent()
        .await
        .map_err(|error| format!("distributed branch report failed: {error}"))?
    {
        return Err("distributed branch report did not reach the parent".to_owned());
    }
    for expected in [
        DistributedMessage::BranchLevel { level: 1 },
        DistributedMessage::BranchRoot {
            username: "parent".to_owned(),
        },
        DistributedMessage::ChildDepth { depth: 0 },
    ] {
        let received = time::timeout(timeout, parent_peer.receive())
            .await
            .map_err(|_| "distributed parent report timed out".to_owned())?
            .map_err(|error| format!("distributed parent report receive failed: {error}"))?;
        if received != expected {
            return Err("distributed parent received incorrect branch metadata".to_owned());
        }
    }

    let (first_tree, first_peer) = duplex(2_048);
    let (second_tree, second_peer) = duplex(2_048);
    let mut first_peer = DistributedConnection::new(first_peer);
    let mut second_peer = DistributedConnection::new(second_peer);
    tree.add_child("first", DistributedConnection::new(first_tree))
        .map_err(|error| format!("distributed first child failed: {error}"))?;
    tree.add_child("second", DistributedConnection::new(second_tree))
        .map_err(|error| format!("distributed second child failed: {error}"))?;
    let search = DistributedSearch {
        identifier: 49,
        username: "origin".to_owned(),
        token: 0x51ab_5001,
        query: "distributed fixture".to_owned(),
    };
    let forwarded = tree
        .forward_search_to_children(&search, Some("first"))
        .await
        .map_err(|error| format!("distributed search forwarding failed: {error}"))?;
    if forwarded != 1 {
        return Err(format!(
            "distributed search reached {forwarded} children instead of one"
        ));
    }
    let received = time::timeout(timeout, second_peer.receive())
        .await
        .map_err(|_| "distributed child search timed out".to_owned())?
        .map_err(|error| format!("distributed child search receive failed: {error}"))?;
    if received != DistributedMessage::Search(search) {
        return Err("distributed child received incorrect search payload".to_owned());
    }
    if time::timeout(Duration::from_millis(25), first_peer.receive())
        .await
        .is_ok()
    {
        return Err("distributed search was reflected to its source child".to_owned());
    }
    if tree.handle_child_message("second", DistributedMessage::ChildDepth { depth: 3 })
        != DistributedEvent::BranchChanged
        || tree.child_info("second").map(|child| child.depth) != Some(3)
    {
        return Err("distributed child depth was not tracked".to_owned());
    }
    if tree.remove_child("SECOND").is_none() || tree.child_info("second").is_some() {
        return Err("distributed child disconnect was not handled".to_owned());
    }

    emit_and_result(
        ProbeContext::new("distributed-tree").ok(
            "Ping round-trip, parent adoption, search forwarding, and child lifecycle completed",
        ),
    )
}

async fn room_create_smoke() -> Result<(), String> {
    let (client, server) = duplex(1_024);
    let mut client = ServerConnection::new(client);
    let mut server = ServerConnection::new(server);
    let room = "slskr-room-create-fixture".to_owned();

    client
        .send(&ServerMessage::JoinRoom {
            room: room.clone(),
            private: false,
        })
        .await
        .map_err(|error| format!("room-create request send failed: {error}"))?;
    let request = server
        .receive_with_direction(slskr_client::protocol::server::Direction::ClientToServer)
        .await
        .map_err(|error| format!("room-create request decode failed: {error}"))?;
    if request
        != (ServerMessage::JoinRoom {
            room: room.clone(),
            private: false,
        })
    {
        return Err("room-create request did not preserve public/private intent".to_owned());
    }

    server
        .send(&ServerMessage::JoinedRoom(JoinedRoom {
            room: room.clone(),
            users: Vec::new(),
            owner: None,
            operators: Vec::new(),
        }))
        .await
        .map_err(|error| format!("room-create response send failed: {error}"))?;
    match client
        .receive_with_direction(slskr_client::protocol::server::Direction::ServerToClient)
        .await
        .map_err(|error| format!("room-create response decode failed: {error}"))?
    {
        ServerMessage::JoinedRoom(joined) if joined.room == room => {}
        _ => return Err("room-create response was not decoded as JoinedRoom".to_owned()),
    }

    for rejection in [
        ServerMessage::CantCreateRoom { room: room.clone() },
        ServerMessage::CantJoinRoom { room: room.clone() },
    ] {
        server
            .send(&rejection)
            .await
            .map_err(|error| format!("room rejection send failed: {error}"))?;
        let decoded = client
            .receive_with_direction(slskr_client::protocol::server::Direction::ServerToClient)
            .await
            .map_err(|error| format!("room rejection decode failed: {error}"))?;
        if decoded != rejection {
            return Err("room rejection code changed during round-trip".to_owned());
        }
    }

    emit_and_result(
        ProbeContext::new("room-create")
            .ok("public join, JoinedRoom, CannotCreateRoom, and CannotJoinRoom completed"),
    )
}

async fn server_relogin_smoke() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSKR_RELOGIN_TIMEOUT_SECONDS", 15, false)?;
    let ctx = ProbeContext::new("server-relogin").with_peer(&username);

    let mut first =
        login_probe_session(&server_address, username.clone(), password.clone()).await?;
    let _second = login_probe_session(&server_address, username, password).await?;
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return emit_and_result(ctx.fail("first session did not receive Relogged"));
        }
        match time::timeout(remaining, first.receive()).await {
            Ok(Ok(ServerMessage::Relogged)) => {
                return emit_and_result(ctx.ok("first session received Relogged"));
            }
            Ok(Ok(ServerMessage::MessageUserResponse(message))) => {
                first
                    .send_server_message(ServerMessage::MessageAcked { id: message.id })
                    .await
                    .map_err(|error| format!("relogin message acknowledgement failed: {error}"))?;
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                return Err(format!("first relogin session receive failed: {error}"));
            }
            Err(_) => return emit_and_result(ctx.fail("first session did not receive Relogged")),
        }
    }
}

async fn server_reconnect_smoke() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let delay = env_duration_secs("SLSKR_RECONNECT_DELAY_SECONDS", 2, true)?;
    let attempts = env_usize("SLSKR_RECONNECT_ATTEMPTS", 3)?;
    let ctx = ProbeContext::new("server-reconnect").with_peer(&username);

    let first = login_probe_session(&server_address, username.clone(), password.clone()).await?;
    drop(first);
    time::sleep(delay).await;

    let mut last_error = String::new();
    for attempt in 1..=attempts {
        match login_probe_session(&server_address, username.clone(), password.clone()).await {
            Ok(mut reconnected) => {
                reconnected
                    .send_ping()
                    .await
                    .map_err(|error| format!("reconnected session ping failed: {error}"))?;
                return emit_and_result(ctx.ok(format!(
                    "session reconnected and pinged on attempt {attempt}"
                )));
            }
            Err(error) => last_error = error,
        }
        time::sleep(delay).await;
    }
    emit_and_result(ctx.fail(format!(
        "session did not reconnect after {attempts} attempts: {last_error}"
    )))
}

async fn closed_listener_smoke() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("closed-listener fixture bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("closed-listener fixture address failed: {error}"))?;
    drop(listener);

    match time::timeout(Duration::from_secs(2), TcpStream::connect(address)).await {
        Ok(Err(_)) => emit_and_result(
            ProbeContext::new("closed-listener").ok("connection refusal returned without panic"),
        ),
        Ok(Ok(_)) => Err("connection to the closed listener unexpectedly succeeded".to_owned()),
        Err(_) => Err("connection to the closed listener did not fail promptly".to_owned()),
    }
}

async fn bad_obfuscation_type_smoke() -> Result<(), String> {
    match validated_obfuscated_port(ROTATED_OBFUSCATION_TYPE.saturating_add(1), 2235) {
        Err(error) if error.contains("unsupported obfuscation type") => {
            emit_and_result(ProbeContext::new("bad-obfuscation-type").ok(error))
        }
        Err(error) => Err(format!(
            "bad obfuscation type returned the wrong error: {error}"
        )),
        Ok(_) => Err("unsupported obfuscation type was accepted".to_owned()),
    }
}

async fn malformed_peer_response_smoke() -> Result<(), String> {
    let timeout = Duration::from_secs(5);
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("malformed-peer listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("malformed-peer listener address failed: {error}"))?;
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|error| format!("malformed-peer accept failed: {error}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            mut stream,
            ..
        } = incoming
        else {
            return Err("malformed-peer fixture received the wrong connection kind".to_owned());
        };
        stream
            .write_all(&[8, 0, 0, 0, 1, 2])
            .await
            .map_err(|error| format!("malformed-peer fixture write failed: {error}"))?;
        stream
            .shutdown()
            .await
            .map_err(|error| format!("malformed-peer fixture shutdown failed: {error}"))
    });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("malformed-peer connect failed: {error}"))?;
    let stream = send_peer_init(
        stream,
        "slskr-malformed-fixture",
        ConnectionKind::PeerMessages,
    )
    .await
    .map_err(|error| format!("malformed-peer init failed: {error}"))?;
    let mut peer = PeerMessageConnection::new(stream);
    let result = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "malformed peer response did not terminate promptly".to_owned())?;
    if result.is_ok() {
        return Err("malformed peer response was accepted".to_owned());
    }
    time::timeout(timeout, server_task)
        .await
        .map_err(|_| "malformed-peer fixture task timed out".to_owned())?
        .map_err(|error| format!("malformed-peer fixture task failed: {error}"))??;
    emit_and_result(
        ProbeContext::new("malformed-peer-response")
            .ok("truncated peer frame was rejected without panic"),
    )
}

/// Transfer resume smoke: download with non-zero offset, verify remaining bytes match.
async fn transfer_resume_smoke() -> Result<(), String> {
    let ctx = ProbeContext::new("transfer-resume");
    let full_payload = b"slskr transfer-resume probe payload data!";
    let resume_offset: u64 = 5;
    let remaining = &full_payload[resume_offset as usize..];
    let filename = "slskr\\probe\\resume_test.txt";
    let full_size = full_payload.len();
    let local_username = optional_env("SLSKR_FIXTURE_PEER_USERNAME")
        .unwrap_or_else(|| "slskr-transfer-resume".to_owned());
    let timeout = env_duration_secs("SLSKR_FIXTURE_PEER_TIMEOUT_SECONDS", 10, false)?;

    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("resume listener bind failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("resume listener addr failed: {e}"))?;

    let server_payload = full_payload.to_vec();
    let server_filename = filename.to_owned();
    let offset_val = resume_offset;
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|e| format!("resume accept failed: {e}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } = incoming
        else {
            return Err("resume expected peer-messages init".to_owned());
        };
        let mut peer = PeerMessageConnection::new(stream);
        let req = peer
            .receive()
            .await
            .map_err(|e| format!("resume request receive failed: {e}"))?;
        let token = match req {
            PeerMessage::TransferRequest(TransferRequest {
                direction: 0,
                token,
                filename,
                ..
            }) if filename == server_filename => token,
            other => {
                return Err(format!(
                    "resume unexpected request: {}",
                    peer_message_name(&other)
                ))
            }
        };
        peer.send(&PeerMessage::TransferResponse(TransferResponse::Allowed {
            token,
            size: Some(server_payload.len() as u64),
        }))
        .await
        .map_err(|e| format!("resume response send failed: {e}"))?;

        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|e| format!("resume file accept failed: {e}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::FileTransfer,
            stream,
            ..
        } = incoming
        else {
            return Err("resume expected file-transfer init".to_owned());
        };
        let mut file = FileTransferConnection::new(stream);
        file.send_token(token)
            .await
            .map_err(|e| format!("resume token send failed: {e}"))?;
        let got_offset = file
            .receive_offset()
            .await
            .map_err(|e| format!("resume offset receive failed: {e}"))?;
        if got_offset != offset_val {
            return Err(format!(
                "resume offset mismatch: expected {offset_val}, got {got_offset}"
            ));
        }
        file.write_chunk(&server_payload[offset_val as usize..])
            .await
            .map_err(|e| format!("resume payload send failed: {e}"))
    });

    let mut peer =
        connect_plain_peer_messages(&local_username, "127.0.0.1", addr.port(), timeout).await?;
    let token = 0xB4_0001;
    peer.send(&PeerMessage::TransferRequest(TransferRequest {
        direction: 0,
        token,
        filename: filename.to_owned(),
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: None,
    }))
    .await
    .map_err(|e| format!("resume request send failed: {e}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "resume response timed out".to_owned())?
        .map_err(|e| format!("resume response receive failed: {e}"))?;
    match response {
        PeerMessage::TransferResponse(TransferResponse::Allowed {
            token: got,
            size: Some(size),
        }) if got == token && size as usize == full_size => {}
        other => {
            return Err(format!(
                "resume unexpected response: {}",
                peer_message_name(&other)
            ))
        }
    }

    let mut file =
        connect_plain_file_transfer(&local_username, "127.0.0.1", addr.port(), timeout).await?;
    let got_token = time::timeout(timeout, file.receive_token())
        .await
        .map_err(|_| "resume file token timed out".to_owned())?
        .map_err(|e| format!("resume file token receive failed: {e}"))?;
    if got_token != token {
        return Err(format!(
            "resume token mismatch: expected {token}, got {got_token}"
        ));
    }
    file.send_offset(resume_offset)
        .await
        .map_err(|e| format!("resume offset send failed: {e}"))?;
    let downloaded = time::timeout(timeout, file.read_chunk(remaining.len()))
        .await
        .map_err(|_| "resume payload timed out".to_owned())?
        .map_err(|e| format!("resume payload read failed: {e}"))?;
    if downloaded != remaining {
        return Err(format!(
            "resume payload mismatch: got {} bytes, expected {}",
            downloaded.len(),
            remaining.len()
        ));
    }
    await_fixture_server_task(server_task, "resume").await?;

    emit_and_result(
        ctx.with_bytes(remaining.len() as u64)
            .ok(format!("resume from offset {resume_offset} verified")),
    )
}

/// Transfer reject smoke: server rejects transfer request, verify graceful handling.
async fn transfer_reject_smoke() -> Result<(), String> {
    let ctx = ProbeContext::new("transfer-reject");
    let filename = "slskr\\probe\\reject_test.txt";
    let local_username = optional_env("SLSKR_FIXTURE_PEER_USERNAME")
        .unwrap_or_else(|| "slskr-transfer-reject".to_owned());
    let timeout = env_duration_secs("SLSKR_FIXTURE_PEER_TIMEOUT_SECONDS", 10, false)?;

    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("reject listener bind failed: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| format!("reject listener addr failed: {e}"))?;

    let server_filename = filename.to_owned();
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|e| format!("reject accept failed: {e}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::PeerMessages,
            stream,
            ..
        } = incoming
        else {
            return Err("reject expected peer-messages init".to_owned());
        };
        let mut peer = PeerMessageConnection::new(stream);
        let req = peer
            .receive()
            .await
            .map_err(|e| format!("reject request receive failed: {e}"))?;
        let token = match req {
            PeerMessage::TransferRequest(TransferRequest {
                direction: 0,
                token,
                filename,
                ..
            }) if filename == server_filename => token,
            other => {
                return Err(format!(
                    "reject unexpected request: {}",
                    peer_message_name(&other)
                ))
            }
        };
        peer.send(&PeerMessage::TransferResponse(TransferResponse::Rejected {
            token,
            reason: "certification reject probe".to_owned(),
        }))
        .await
        .map_err(|e| format!("reject response send failed: {e}"))
    });

    let mut peer =
        connect_plain_peer_messages(&local_username, "127.0.0.1", addr.port(), timeout).await?;
    let token = 0xB5_0001;
    peer.send(&PeerMessage::TransferRequest(TransferRequest {
        direction: 0,
        token,
        filename: filename.to_owned(),
        filename_encoding: ProtocolTextEncoding::Utf8,
        size: None,
    }))
    .await
    .map_err(|e| format!("reject request send failed: {e}"))?;
    let response = time::timeout(timeout, peer.receive())
        .await
        .map_err(|_| "reject response timed out".to_owned())?
        .map_err(|e| format!("reject response receive failed: {e}"))?;

    let result = match response {
        PeerMessage::TransferResponse(TransferResponse::Rejected { token: got, .. }) => {
            if got == token {
                ctx.ok("transfer rejected gracefully")
            } else {
                ctx.fail(format!(
                    "rejection token mismatch: expected {token}, got {got}"
                ))
            }
        }
        PeerMessage::TransferResponse(TransferResponse::Allowed { .. }) => {
            ctx.fail("expected rejection but got allowed")
        }
        other => ctx.fail(format!(
            "unexpected response: {}",
            peer_message_name(&other)
        )),
    };

    await_fixture_server_task(server_task, "reject").await?;
    emit_and_result(result)
}

async fn await_fixture_server_task(
    task: tokio::task::JoinHandle<Result<(), String>>,
    probe: &str,
) -> Result<(), String> {
    task.await
        .map_err(|error| format!("{probe} server task failed: {error}"))?
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
            other => {
                return Err(format!(
                    "fixture download unexpected request: {}",
                    peer_message_name(&other)
                ))
            }
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
        filename_encoding: ProtocolTextEncoding::Utf8,
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
        other => {
            return Err(format!(
                "fixture download unexpected response: {}",
                peer_message_name(&other)
            ))
        }
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
        filename_encoding: ProtocolTextEncoding::Utf8,
        size,
        extension,
        extension_encoding: ProtocolTextEncoding::Utf8,
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

    if config.active_probes {
        session
            .send_server_message(ServerMessage::RoomListRequest)
            .await
            .map_err(|error| format!("room list refresh failed: {error}"))?;
        println!("active probe: room list requested");
        if let Some(query) = &config.search_query {
            dispatch_live_soak_search(&mut session, query, config.search_token).await?;
        }
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
    let server_progress = Arc::new(AtomicU64::new(unix_seconds()));
    let watchdog_task = tokio::spawn(run_live_soak_server_watchdog(
        server_progress.clone(),
        config.duration,
        config.watchdog_interval,
        config.watchdog_stale_seconds,
    ));
    let server_result = run_server_soak(&mut session, &config, server_progress).await;
    watchdog_task.abort();
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
            indirect_timeout: env_duration_secs("SLSKR_INDIRECT_TIMEOUT_SECONDS", 10, false)?,
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
    if let Some(wait_port) = optional_env("SLSK_WAIT_PORT")
        .or_else(|| optional_env("SLSK_SEARCH_WAIT_PORT"))
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|error| format!("invalid SLSK_WAIT_PORT/SLSK_SEARCH_WAIT_PORT: {error}"))
        })
        .transpose()?
    {
        session
            .send_server_message(ServerMessage::SetWaitPort(WaitPort {
                port: wait_port,
                obfuscation: None,
            }))
            .await
            .map_err(|error| format!("set wait port failed: {error}"))?;
    }
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

async fn wait_for_room_join(
    session: &mut ServerSession<TcpStream>,
    room: &str,
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
            Ok(Ok(ServerMessage::JoinedRoom(joined))) if joined.room == room => return Ok(()),
            Ok(Ok(ServerMessage::CantCreateRoom { room: rejected })) if rejected == room => {
                return Err("server rejected room creation".to_owned());
            }
            Ok(Ok(ServerMessage::CantJoinRoom { room: rejected })) if rejected == room => {
                return Err("server rejected room join".to_owned());
            }
            Ok(Ok(ServerMessage::MessageUserResponse(private_message))) => {
                session
                    .send_server_message(ServerMessage::MessageAcked {
                        id: private_message.id,
                    })
                    .await
                    .map_err(|error| format!("room join message ack failed: {error}"))?;
            }
            Ok(Ok(ServerMessage::Relogged)) => {
                return Err("account was logged in elsewhere".to_owned());
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Err(format!("room join receive failed: {error}")),
            Err(_) => break,
        }
    }
    Err("room join timed out".to_owned())
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

fn validated_obfuscated_port(obfuscation_type: u32, port: u16) -> Result<u16, String> {
    if obfuscation_type != ROTATED_OBFUSCATION_TYPE {
        return Err(format!(
            "peer advertised unsupported obfuscation type {obfuscation_type}"
        ));
    }
    if port == 0 {
        return Err("peer did not advertise an obfuscated listener port".to_owned());
    }
    Ok(port)
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
    search_interval: Duration,
    active_probes: bool,
    peer_username: Option<String>,
    search_query: Option<String>,
    search_token: u32,
    shared_folders: u32,
    shared_files: u32,
    watchdog_interval: Duration,
    watchdog_stale_seconds: u64,
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
            duration: env_duration_secs("SLSK_SOAK_SECONDS", 60, false)?,
            max_events: env_usize("SLSK_SOAK_MAX_EVENTS", 40)?,
            ping_interval: env_duration_secs("SLSK_SOAK_PING_SECONDS", 30, false)?,
            search_interval: env_duration_secs("SLSK_SOAK_SEARCH_INTERVAL_SECONDS", 900, false)?,
            active_probes: env_bool("SLSK_SOAK_ACTIVE_PROBES", true)?,
            peer_username: optional_env("SLSK_SOAK_PEER_USERNAME"),
            search_query: optional_env("SLSK_SOAK_SEARCH_QUERY").or_else(|| {
                env_bool("SLSK_SOAK_DEFAULT_SEARCH", true)
                    .ok()
                    .filter(|enabled| *enabled)
                    .map(|_| "commons".to_owned())
            }),
            search_token: env_u32("SLSK_SOAK_SEARCH_TOKEN", 1_000_001)?,
            shared_folders: env_u32("SLSK_SOAK_SHARED_FOLDERS", 0)?,
            shared_files: env_u32("SLSK_SOAK_SHARED_FILES", 0)?,
            watchdog_interval: env_duration_secs("SLSK_SOAK_WATCHDOG_SECONDS", 120, false)?,
            watchdog_stale_seconds: env_u64("SLSK_SOAK_WATCHDOG_STALE_SECONDS", 240)?,
        })
    }
}

async fn run_server_soak<S>(
    session: &mut ServerSession<S>,
    config: &LiveSoakConfig,
    progress: Arc<AtomicU64>,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let deadline = Instant::now() + config.duration;
    let mut next_ping = Instant::now() + config.ping_interval;
    let mut next_search = Instant::now() + config.search_interval;
    let send_timeout = env_duration_secs("SLSK_SOAK_SERVER_SEND_TIMEOUT_SECONDS", 20, false)?;
    let mut search_token = config.search_token;
    let mut events = 0usize;

    while Instant::now() < deadline && events < config.max_events {
        let now = Instant::now();
        if now >= next_ping {
            time::timeout(send_timeout, session.send_ping())
                .await
                .map_err(|_| "periodic ping send timed out".to_owned())?
                .map_err(|error| format!("periodic ping failed: {error}"))?;
            next_ping = Instant::now() + config.ping_interval;
            progress.store(unix_seconds(), Ordering::Relaxed);
            println!("server ping sent");
        }
        if config.active_probes && now >= next_search {
            if let Some(query) = &config.search_query {
                search_token = search_token.wrapping_add(1).max(1);
                time::timeout(
                    send_timeout,
                    dispatch_live_soak_search(session, query, search_token),
                )
                .await
                .map_err(|_| "search dispatch timed out".to_owned())??;
                progress.store(unix_seconds(), Ordering::Relaxed);
            }
            next_search = Instant::now() + config.search_interval;
        }

        let next_action = if config.active_probes && config.search_query.is_some() {
            next_ping.min(next_search)
        } else {
            next_ping
        };
        let next_wait = next_action
            .min(deadline)
            .saturating_duration_since(Instant::now());

        match time::timeout(next_wait, session.receive()).await {
            Ok(Ok(message)) => {
                events += 1;
                progress.store(unix_seconds(), Ordering::Relaxed);
                handle_server_message(session, message).await?;
            }
            Ok(Err(error)) => return Err(format!("server receive failed: {error}")),
            Err(_) => {}
        }
    }

    println!("server soak observed {events} event(s)");
    Ok(())
}

async fn run_live_soak_server_watchdog(
    progress: Arc<AtomicU64>,
    duration: Duration,
    interval: Duration,
    stale_seconds: u64,
) {
    let deadline = Instant::now() + duration;
    let mut last_reported = 0_u64;

    while Instant::now() < deadline {
        time::sleep(interval).await;
        let now = unix_seconds();
        let last = progress.load(Ordering::Relaxed);
        let idle = now.saturating_sub(last);
        if idle >= stale_seconds && last != last_reported {
            println!(
                "live soak server watchdog stale idle_seconds={} stale_threshold_seconds={}",
                idle, stale_seconds
            );
            last_reported = last;
        }
    }
}

async fn dispatch_live_soak_search<S>(
    session: &mut ServerSession<S>,
    query: &str,
    token: u32,
) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    session
        .send_server_message(ServerMessage::FileSearchRequest(SearchRequest {
            token,
            query: query.to_owned(),
        }))
        .await
        .map_err(|error| format!("search dispatch failed: {error}"))?;
    println!(
        "active probe: file search dispatched token={token} query={}",
        redact_query(query)
    );
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
        ServerMessage::JoinedRoom(room) => {
            println!(
                "server event: joined room users={} private={}",
                room.users.len(),
                room.owner.is_some()
            );
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
            println!(
                "server event: connect_to_peer received requester={} kind={} token={} host_override={}",
                redact_username(&response.username),
                response.connection_type,
                response.token,
                optional_env("SLSK_SOAK_INDIRECT_HOST_OVERRIDE").is_some()
            );
            let timeout_seconds = env_u64("SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS", 20)?
                .checked_add(5)
                .ok_or_else(|| "SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS is too large".to_owned())?;
            let timeout = validated_duration_secs(
                "SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS",
                timeout_seconds,
                false,
            )?;
            match time::timeout(timeout, handle_live_soak_connect_to_peer_response(response)).await
            {
                Ok(result) => result?,
                Err(_) => {
                    println!("server event: connect_to_peer probe timed out");
                }
            }
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

    let timeout = env_duration_secs("SLSK_SOAK_INDIRECT_TIMEOUT_SECONDS", 20, false)?;
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
        let deadline = Instant::now() + timeout;
        for _ in 0..16 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                log_live_soak_indirect_close("response timed out");
                return Ok(());
            }
            let peer_response = match time::timeout(remaining, peer.receive()).await {
                Ok(Ok(message)) => message,
                Ok(Err(error)) => {
                    log_live_soak_indirect_close(peer_close_reason(&error.to_string()));
                    return Ok(());
                }
                Err(_) => {
                    log_live_soak_indirect_close("response timed out");
                    return Ok(());
                }
            };
            match peer_response {
                PeerMessage::UserInfoResponse(_) => break,
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
                    break;
                }
                other => println!(
                    "live soak indirect interleaved peer message: {}",
                    peer_message_name(&other)
                ),
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
                if let Err(error) = response_result {
                    println!(
                        "listener isolated failed peer connection: {}",
                        peer_close_reason(&error)
                    );
                }
            }
            Ok(Err(error)) => println!(
                "listener rejected invalid peer initialization: {}",
                peer_close_reason(&error.to_string())
            ),
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
                if let Err(error) = response_result {
                    println!(
                        "obfuscated listener isolated failed peer connection: {}",
                        peer_close_reason(&error)
                    );
                }
            }
            Ok(Err(error)) => println!(
                "obfuscated listener rejected invalid peer initialization: {}",
                peer_close_reason(&error.to_string())
            ),
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
            match time::timeout(Duration::from_secs(5), peer.receive()).await {
                Ok(Ok(PeerMessage::UserInfoRequest)) => {
                    peer.send(&PeerMessage::UserInfoResponse(UserInfo {
                        description: "slskr live soak".to_owned(),
                        picture: None,
                        total_uploads: 0,
                        queue_size: 0,
                        slots_free: true,
                        upload_permissions: None,
                    }))
                    .await
                    .map_err(|error| format!("peer response send failed: {error}"))?;
                    println!("listener proof: peer user-info request answered");
                }
                Ok(Ok(PeerMessage::FileSearchResponse(response))) => {
                    println!(
                        "listener proof: search response username={} token={} results={} private_results={} slots_free={} queue_length={} speed={}",
                        redact_username(&response.username),
                        response.token,
                        response.results.len(),
                        response.private_results.len(),
                        response.slot_free,
                        response.queue_length,
                        response.average_speed
                    );
                }
                Ok(Ok(PeerMessage::TransferRequest(request))) => {
                    println!(
                        "listener proof: transfer request direction={} token={} filename={} size={}",
                        request.direction,
                        request.token,
                        redact_path(&request.filename),
                        request
                            .size
                            .map(|size| size.to_string())
                            .unwrap_or_else(|| "unknown".to_owned())
                    );
                }
                Ok(Ok(other)) => {
                    println!("listener proof: peer message {}", peer_message_name(&other));
                }
                Ok(Err(error)) => return Err(format!("peer receive failed: {error}")),
                Err(_) => println!("listener proof: peer connection accepted without message"),
            }
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
        match time::timeout(Duration::from_secs(5), peer.receive()).await {
            Ok(Ok(PeerMessage::UserInfoRequest)) => {
                peer.send(&PeerMessage::UserInfoResponse(UserInfo {
                    description: "slskr obfuscated live soak".to_owned(),
                    picture: None,
                    total_uploads: 0,
                    queue_size: 0,
                    slots_free: true,
                    upload_permissions: None,
                }))
                .await
                .map_err(|error| format!("obfuscated peer response send failed: {error}"))?;
                println!("obfuscated listener proof: peer user-info request answered");
            }
            Ok(Ok(PeerMessage::FileSearchResponse(response))) => {
                println!(
                    "obfuscated listener proof: search response username={} token={} results={} private_results={} slots_free={} queue_length={} speed={}",
                    redact_username(&response.username),
                    response.token,
                    response.results.len(),
                    response.private_results.len(),
                    response.slot_free,
                    response.queue_length,
                    response.average_speed
                );
            }
            Ok(Ok(other)) => {
                println!(
                    "obfuscated listener proof: peer message {}",
                    peer_message_name(&other)
                );
            }
            Ok(Err(error)) => return Err(format!("obfuscated peer receive failed: {error}")),
            Err(_) => {
                println!("obfuscated listener proof: peer connection accepted without message")
            }
        }
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
        ServerMessage::JoinedRoom(_) => "joined_room",
        ServerMessage::LeaveRoom { .. } => "leave_room",
        ServerMessage::SetStatus { .. } => "set_status",
        ServerMessage::ServerPing => "server_ping",
        ServerMessage::SharedFoldersFiles { .. } => "shared_folders_files",
        ServerMessage::GetUserStatsRequest { .. } => "get_user_stats_request",
        ServerMessage::GetUserStats { .. } => "get_user_stats",
        ServerMessage::Relogged => "relogged",
        ServerMessage::UserSearch(_) => "user_search",
        ServerMessage::AddThingILike { .. } => "add_thing_i_like",
        ServerMessage::RemoveThingILike { .. } => "remove_thing_i_like",
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
        ServerMessage::AddThingIHate { .. } => "add_thing_i_hate",
        ServerMessage::RemoveThingIHate { .. } => "remove_thing_i_hate",
        ServerMessage::CantConnectToPeerRequest { .. } => "cant_connect_to_peer_request",
        ServerMessage::CantConnectToPeerResponse { .. } => "cant_connect_to_peer_response",
        ServerMessage::CantCreateRoom { .. } => "cant_create_room",
        ServerMessage::CantJoinRoom { .. } => "cant_join_room",
        ServerMessage::Unknown { .. } => "unknown",
    }
}

fn peer_message_name(message: &PeerMessage) -> &'static str {
    match message {
        PeerMessage::PrivateMessage(_) => "private_message",
        PeerMessage::GetShareFileList => "get_share_file_list",
        PeerMessage::SharedFileListResponse(_) => "shared_file_list_response",
        PeerMessage::FileSearchRequest { .. } => "file_search_request",
        PeerMessage::FileSearchResponse(_) => "file_search_response",
        PeerMessage::RoomInvitation(_) => "room_invitation",
        PeerMessage::CancelledQueuedTransfer(_) => "cancelled_queued_transfer",
        PeerMessage::UserInfoRequest => "user_info_request",
        PeerMessage::UserInfoResponse(_) => "user_info_response",
        PeerMessage::SendConnectToken(_) => "send_connect_token",
        PeerMessage::MoveDownloadToTop(_) => "move_download_to_top",
        PeerMessage::FolderContentsRequest(_) => "folder_contents_request",
        PeerMessage::FolderContentsResponse(_) => "folder_contents_response",
        PeerMessage::TransferRequest(_) => "transfer_request",
        PeerMessage::TransferResponse(_) => "transfer_response",
        PeerMessage::PlaceholdUpload { .. } => "placehold_upload",
        PeerMessage::QueueUpload { .. } => "queue_upload",
        PeerMessage::PlaceInQueueResponse { .. } => "place_in_queue_response",
        PeerMessage::UploadFailed { .. } => "upload_failed",
        PeerMessage::ExactFileSearchRequest(_) => "exact_file_search_request",
        PeerMessage::QueuedDownloads(_) => "queued_downloads",
        PeerMessage::IndirectFileSearchRequest(_) => "indirect_file_search_request",
        PeerMessage::UploadDenied { .. } => "upload_denied",
        PeerMessage::PlaceInQueueRequest { .. } => "place_in_queue_request",
        PeerMessage::UploadQueueNotification => "upload_queue_notification",
        PeerMessage::Unknown { .. } => "unknown",
    }
}

fn redact_query(query: &str) -> String {
    if query.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", query.chars().count())
    }
}

fn redact_path(path: &str) -> String {
    if path.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", path.chars().count())
    }
}

fn redact_peer_text(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", value.chars().count())
    }
}

fn peer_close_reason(error: &str) -> &'static str {
    if error.contains("Connection reset by peer") {
        "connection reset by peer"
    } else if error.contains("unexpected end of file") || error.contains("unexpected end of input")
    {
        "unexpected eof"
    } else {
        "closed"
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Default)]
struct LiveSoakIndirectCloseLog {
    total: u64,
    connection_reset: u64,
    unexpected_eof: u64,
    timed_out: u64,
    other: u64,
}

fn log_live_soak_indirect_close(reason: &'static str) {
    const VERBOSE_LIMIT: u64 = 3;
    const SUMMARY_INTERVAL: u64 = 25;

    static CLOSE_LOG: OnceLock<Mutex<LiveSoakIndirectCloseLog>> = OnceLock::new();
    let mut log = CLOSE_LOG
        .get_or_init(|| Mutex::new(LiveSoakIndirectCloseLog::default()))
        .lock()
        .expect("live soak indirect close log poisoned");

    log.total += 1;
    match reason {
        "connection reset by peer" => log.connection_reset += 1,
        "unexpected eof" => log.unexpected_eof += 1,
        "response timed out" => log.timed_out += 1,
        _ => log.other += 1,
    }

    if log.total <= VERBOSE_LIMIT {
        println!("live soak indirect peer-message closed before response: {reason}");
    } else if log.total.is_multiple_of(SUMMARY_INTERVAL) {
        println!(
            "live soak indirect peer-message close summary total={} connection_reset={} unexpected_eof={} timed_out={} other={}",
            log.total, log.connection_reset, log.unexpected_eof, log.timed_out, log.other
        );
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

fn env_duration_secs(name: &str, default: u64, allow_zero: bool) -> Result<Duration, String> {
    validated_duration_secs(name, env_u64(name, default)?, allow_zero)
}

fn validated_duration_secs(name: &str, seconds: u64, allow_zero: bool) -> Result<Duration, String> {
    if !allow_zero && seconds == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    let duration = Duration::from_secs(seconds);
    if Instant::now().checked_add(duration).is_none() {
        return Err(format!("{name} exceeds the runtime timer range"));
    }
    Ok(duration)
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
        await_fixture_server_task, incoming_connection_name, normalize_command,
        peer_probe_messages, redact_peer_text, scrub_socket_addr, validated_duration_secs,
        validated_obfuscated_port,
    };
    use slskr_client::{
        listener::IncomingConnection, protocol::server::ServerMessage,
        stream::PeerMessageConnection,
    };
    use std::ffi::OsString;
    use std::{
        net::{Ipv4Addr, SocketAddr},
        time::Duration,
    };
    use tokio::io::duplex;

    fn normalize(args: &[&str]) -> Vec<String> {
        normalize_command(args.iter().map(OsString::from)).unwrap()
    }

    #[test]
    fn grouped_commands_map_to_internal_runner_names() {
        assert_eq!(normalize(&["login", "smoke"]), ["login-smoke"]);
        assert_eq!(normalize(&["soak", "live"]), ["live-soak"]);
        assert_eq!(normalize(&["smoke", "local-peer"]), ["local-peer-smoke"]);
        assert_eq!(
            normalize(&["probe", "obfuscated-peer"]),
            ["obfuscated-peer-probe"]
        );
        assert_eq!(
            normalize(&["probe", "overlay-service"]),
            ["overlay-service-probe"]
        );
        assert_eq!(normalize(&["probe", "dht-store"]), ["dht-store-probe"]);
        assert_eq!(
            normalize(&["probe", "wishlist-interval"]),
            ["wishlist-interval-probe"]
        );
        assert_eq!(normalize(&["probe", "user-watch"]), ["user-watch-probe"]);
        assert_eq!(
            normalize(&["smoke", "distributed-tree"]),
            ["distributed-tree-smoke"]
        );
        assert_eq!(normalize(&["smoke", "room-create"]), ["room-create-smoke"]);
        assert_eq!(
            normalize(&["smoke", "server-relogin"]),
            ["server-relogin-smoke"]
        );
        assert_eq!(
            normalize(&["smoke", "server-reconnect"]),
            ["server-reconnect-smoke"]
        );
        assert_eq!(
            normalize(&["smoke", "closed-listener"]),
            ["closed-listener-smoke"]
        );
        assert_eq!(
            normalize(&["smoke", "bad-obfuscation-type"]),
            ["bad-obfuscation-type-smoke"]
        );
        assert_eq!(
            normalize(&["smoke", "malformed-peer-response"]),
            ["malformed-peer-response-smoke"]
        );
    }

    #[test]
    fn internal_runner_names_still_pass_through() {
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
    fn peer_text_redaction_removes_terminal_controls() {
        let redacted = redact_peer_text("rejected\n\x1b[31mforged");
        assert_eq!(redacted, "len20");
        assert!(!redacted.chars().any(char::is_control));
    }

    #[test]
    fn duration_validation_rejects_zero_and_unrepresentable_timers() {
        let zero = validated_duration_secs("TEST_SECONDS", 0, false)
            .expect_err("zero interval should fail");
        assert!(zero.contains("greater than zero"), "{zero}");

        let oversized = validated_duration_secs("TEST_SECONDS", u64::MAX, false)
            .expect_err("unrepresentable interval should fail");
        assert!(oversized.contains("timer range"), "{oversized}");

        assert_eq!(
            validated_duration_secs("TEST_SECONDS", 0, true).unwrap(),
            Duration::ZERO
        );
    }

    #[test]
    fn obfuscated_port_validation_rejects_unsupported_and_missing_endpoints() {
        assert_eq!(validated_obfuscated_port(1, 2235).unwrap(), 2235);
        assert!(validated_obfuscated_port(2, 2235)
            .unwrap_err()
            .contains("unsupported obfuscation type"));
        assert!(validated_obfuscated_port(1, 0)
            .unwrap_err()
            .contains("did not advertise"));
    }

    #[test]
    fn incoming_connection_names_are_stable() {
        let (stream, _) = duplex(8);
        let incoming = IncomingConnection::PeerMessages(PeerMessageConnection::new(stream));
        assert_eq!(incoming_connection_name(&incoming), "peer_messages");
    }

    #[tokio::test]
    async fn fixture_server_task_errors_fail_the_probe() {
        let failed = tokio::spawn(async { Err("fixture send failed".to_owned()) });
        assert_eq!(
            await_fixture_server_task(failed, "reject")
                .await
                .expect_err("fixture error must propagate"),
            "fixture send failed"
        );

        let completed = tokio::spawn(async { Ok(()) });
        await_fixture_server_task(completed, "resume")
            .await
            .expect("successful fixture task");
    }
}
