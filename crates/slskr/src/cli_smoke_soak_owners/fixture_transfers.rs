use super::*;

pub(in crate::cli) async fn await_fixture_server_task(
    task: tokio::task::JoinHandle<Result<(), String>>,
    probe: &str,
) -> Result<(), String> {
    task.await
        .map_err(|error| format!("{probe} server task failed: {error}"))?
}

pub(super) async fn run_fixture_browse_smoke(
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

pub(super) async fn run_fixture_download_smoke(
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

pub(super) fn build_fixture_shared_file_list_payload(
    filename: &str,
    size: usize,
) -> Result<Vec<u8>, String> {
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

pub(super) fn encode_fixture_file_entry(
    writer: &mut Writer,
    entry: &FileEntry,
) -> Result<(), String> {
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
