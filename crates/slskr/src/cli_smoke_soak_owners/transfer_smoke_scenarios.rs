use super::*;

/// Transfer resume smoke: download with non-zero offset, verify remaining bytes match.
pub(in crate::cli) async fn transfer_resume_smoke() -> Result<(), String> {
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
pub(in crate::cli) async fn transfer_reject_smoke() -> Result<(), String> {
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
