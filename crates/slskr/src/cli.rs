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
    overlay_control::{send_udp_control, ControlEnvelope},
    peer_connect::{
        send_obfuscated_peer_init, send_obfuscated_peer_init_with_token, send_peer_init,
        send_peer_init_with_token, send_pierce_firewall, IndirectPeerRequest,
    },
    quic_control::{discover_certificate_public_key_pin, send_quic_control, QUIC_CONTROL_ALPN},
    quic_data::{send_quic_data, QUIC_DATA_ALPN},
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
    io::{duplex, AsyncRead, AsyncWriteExt},
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
        Some("overlay-udp-probe") => overlay_udp_probe().await,
        Some("overlay-quic-control-probe") => overlay_quic_control_probe().await,
        Some("quic-data-probe") => quic_data_probe().await,
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
            Some("overlay-udp") => vec!["overlay-udp-probe"],
            Some("overlay-quic-control") => vec!["overlay-quic-control-probe"],
            Some("quic-data") => vec!["quic-data-probe"],
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
  SLSKR_OVERLAY_ENDPOINT=<ip:port> slskr probe overlay-udp
  SLSKR_OVERLAY_ENDPOINT=<ip:port> slskr probe overlay-quic-control
  SLSKR_OVERLAY_ENDPOINT=<ip:port> slskr probe quic-data
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

fn direct_overlay_probe_endpoint() -> Result<SocketAddr, String> {
    required_env_any(&["SLSKR_OVERLAY_ENDPOINT"])?
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid SLSKR_OVERLAY_ENDPOINT: {error}"))
}

fn direct_overlay_probe_envelope() -> Result<ControlEnvelope, String> {
    ControlEnvelope::new_signed(
        "probe",
        b"slskr-direct-transport-probe".to_vec(),
        &SigningKey::from_bytes(&[0x71_u8; 32]),
    )
    .map_err(|error| format!("direct overlay probe envelope failed: {error}"))
}

async fn overlay_udp_probe() -> Result<(), String> {
    let endpoint = direct_overlay_probe_endpoint()?;
    let envelope = direct_overlay_probe_envelope()?;
    let sent = send_udp_control(endpoint, &envelope)
        .await
        .map_err(|error| format!("UDP overlay probe failed: {error}"))?;
    emit_and_result(
        ProbeContext::new("overlay-udp")
            .ok(format!("endpoint={endpoint} bytes={sent} envelope=probe")),
    )
}

async fn overlay_quic_control_probe() -> Result<(), String> {
    let endpoint = direct_overlay_probe_endpoint()?;
    let envelope = direct_overlay_probe_envelope()?;
    let certificate_pin =
        discover_certificate_public_key_pin(endpoint, QUIC_CONTROL_ALPN, "slskdn-overlay")
            .await
            .map_err(|error| format!("QUIC control certificate discovery failed: {error}"))?;
    send_quic_control(endpoint, &envelope, certificate_pin)
        .await
        .map_err(|error| format!("QUIC control probe failed: {error}"))?;
    emit_and_result(ProbeContext::new("overlay-quic-control").ok(format!(
        "endpoint={endpoint} envelope=probe certificate_pin=discovered-for-frozen-target"
    )))
}

async fn quic_data_probe() -> Result<(), String> {
    let endpoint = direct_overlay_probe_endpoint()?;
    let payload = b"slskr-direct-quic-data-probe";
    let certificate_pin =
        discover_certificate_public_key_pin(endpoint, QUIC_DATA_ALPN, "slskdn-overlay-data")
            .await
            .map_err(|error| format!("QUIC data certificate discovery failed: {error}"))?;
    let sent = send_quic_data(endpoint, payload, certificate_pin)
        .await
        .map_err(|error| format!("QUIC data probe failed: {error}"))?;
    emit_and_result(
        ProbeContext::new("quic-data")
            .with_bytes(sent as u64)
            .ok(format!("endpoint={endpoint} bytes={sent} payload=raw")),
    )
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
    let obfuscated_port_override = optional_env("SLSK_OBFUSCATED_PORT_OVERRIDE")
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|error| format!("invalid SLSK_OBFUSCATED_PORT_OVERRIDE: {error}"))
        })
        .transpose()?;
    if let Some(port) = obfuscated_port_override {
        validated_obfuscated_port(ROTATED_OBFUSCATION_TYPE, port)?;
    }

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
                if obfuscated_port_override.is_some()
                    || validated_obfuscated_port(
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
    let obfuscated_port = match obfuscated_port_override {
        Some(port) => port,
        None => validated_obfuscated_port(address.obfuscation_type, address.obfuscated_port)?,
    };

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
            obfuscated_port,
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
                TcpStream::connect((host.as_str(), obfuscated_port)),
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
    let attempts = env_usize("SLSK_DISTRIBUTED_PEER_ADDRESS_ATTEMPTS", 3)?.max(1);
    let retry_delay = env_duration_secs("SLSK_DISTRIBUTED_PROBE_RETRY_SECONDS", 1, true)?;

    let ctx = ProbeContext::new("distributed-peer").with_peer(&peer_username);

    let mut last_error = None;
    for attempt in 1..=attempts {
        let result = async {
            let address = resolve_peer_address(
                &username,
                &password,
                &peer_username,
                &server_address,
                timeout,
            )
            .await?;
            let port = match optional_env("SLSK_DISTRIBUTED_PORT_OVERRIDE") {
                Some(value) => value
                    .parse::<u16>()
                    .map_err(|error| format!("invalid SLSK_DISTRIBUTED_PORT_OVERRIDE: {error}"))?,
                None => peer_regular_port(&address)?,
            };
            let host = optional_env("SLSK_DISTRIBUTED_HOST_OVERRIDE")
                .unwrap_or_else(|| address.ip.to_string());
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
            receive_distributed_ping(&mut distributed, timeout).await
        }
        .await;

        match result {
            Ok((received_branch_level, received_branch_root)) => {
                return emit_and_result(ctx.ok(format!(
                    "distributed peer probe completed; attempt={attempt}; ping=received; branch_metadata={}; host_override={}",
                    if received_branch_level && received_branch_root {
                        "level+root"
                    } else if received_branch_level {
                        "level"
                    } else if received_branch_root {
                        "root"
                    } else {
                        "none"
                    },
                    optional_env("SLSK_DISTRIBUTED_HOST_OVERRIDE").is_some()
                )));
            }
            Err(error) => {
                last_error = Some(error);
                if attempt < attempts && !retry_delay.is_zero() {
                    time::sleep(retry_delay).await;
                }
            }
        }
    }

    Err(format!(
        "distributed peer probe failed after {attempts} attempts: {}",
        last_error.unwrap_or_else(|| "unknown error".to_owned())
    ))
}

async fn receive_distributed_ping<S>(
    distributed: &mut DistributedConnection<S>,
    timeout: Duration,
) -> Result<(bool, bool), String>
where
    S: AsyncRead + Unpin,
{
    let response_deadline = Instant::now() + timeout;
    let mut received_branch_level = false;
    let mut received_branch_root = false;

    loop {
        let remaining = response_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("distributed peer response timed out".to_owned());
        }

        match time::timeout(remaining, distributed.receive()).await {
            Ok(Ok(DistributedMessage::Ping | DistributedMessage::PingResponse { .. })) => {
                return Ok((received_branch_level, received_branch_root));
            }
            Ok(Ok(DistributedMessage::BranchLevel { .. })) => {
                received_branch_level = true;
            }
            Ok(Ok(DistributedMessage::BranchRoot { .. })) => {
                received_branch_root = true;
            }
            Ok(Ok(message)) => {
                return Err(format!(
                    "distributed peer returned unexpected response: {message:?}"
                ));
            }
            Ok(Err(error)) => return Err(format!("distributed peer receive failed: {error}")),
            Err(_) => return Err("distributed peer response timed out".to_owned()),
        }
    }
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
    wait_for_advertised_port_metadata(
        &mut session,
        &username,
        advertised_port,
        env_duration_secs("SLSK_INDIRECT_METADATA_TIMEOUT_SECONDS", 20, false)?,
    )
    .await?;

    let token = env_u32("SLSK_INDIRECT_TOKEN", 0x51ab_2001)?;
    let request =
        IndirectPeerRequest::new(token, peer_username.clone(), ConnectionKind::PeerMessages);
    session
        .send_server_message(request.server_message())
        .await
        .map_err(|error| format!("indirect connect request failed: {error}"))?;
    println!(
        "indirect peer request sent; peer={}; token={}; advertised_port={}",
        redact_username(&peer_username),
        token,
        advertised_port
    );
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
    let mut last_listener_error = None;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(last_listener_error
                .map(|error| format!("indirect probe listener accept timed out: {error}"))
                .unwrap_or_else(|| "indirect probe listener accept timed out".to_owned()));
        }

        tokio::select! {
            accept_result = listener.accept_with_timeout(remaining.min(Duration::from_secs(3))) => {
                match accept_result {
                    Ok(incoming) => return Ok(incoming),
                    Err(error) => {
                        // Public Soulseek listener ports receive unsolicited
                        // peer attempts. One connection that closes before its
                        // init frame must not consume the entire indirect
                        // probe; keep accepting until the token-bearing
                        // PierceFirewall connection arrives.
                        last_listener_error = Some(error.to_string());
                    }
                }
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

#[path = "cli_smoke_soak.rs"]
mod smoke_soak;
use smoke_soak::*;

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
#[path = "cli_tests.rs"]
mod tests;
