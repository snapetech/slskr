use super::*;

pub(in crate::cli) async fn local_peer_smoke() -> Result<(), String> {
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
            config.credentials.a_username.clone(),
            config.credentials.a_password,
        ))
        .await
        .map_err(|error| format!("account A login failed: {error}"))?;
    b_session
        .login(LoginCredentials::default_client(
            config.credentials.b_username.clone(),
            config.credentials.b_password,
        ))
        .await
        .map_err(|error| format!("account B login failed: {error}"))?;

    a_session
        .set_wait_port(u32::from(indirect_address.port()))
        .await
        .map_err(|error| format!("account A wait-port update failed: {error}"))?;

    run_direct_peer_message_smoke(&config.credentials.a_username).await?;
    run_obfuscated_peer_message_smoke(&config.credentials.a_username).await?;
    run_direct_file_transfer_smoke(&config.credentials.a_username).await?;
    run_indirect_peer_message_smoke(
        &mut a_session,
        &mut b_session,
        indirect_listener,
        &config.credentials.a_username,
        &config.credentials.b_username,
        config.indirect_host_override.as_deref(),
        config.indirect_timeout,
    )
    .await?;

    b_session
        .send_server_message(ServerMessage::GetPeerAddressRequest {
            username: config.credentials.a_username,
        })
        .await
        .map_err(|error| format!("peer-address request failed: {error}"))?;

    println!("local peer smoke completed");
    Ok(())
}

pub(in crate::cli) async fn fixture_peer_smoke() -> Result<(), String> {
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

pub(in crate::cli) async fn closed_listener_smoke() -> Result<(), String> {
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

pub(in crate::cli) async fn bad_obfuscation_type_smoke() -> Result<(), String> {
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

pub(in crate::cli) async fn malformed_peer_response_smoke() -> Result<(), String> {
    let timeout = Duration::from_secs(5);
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("malformed-peer listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("malformed-peer listener address failed: {error}"))?;
    let server_task = ProbeTask::spawn(async move {
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
