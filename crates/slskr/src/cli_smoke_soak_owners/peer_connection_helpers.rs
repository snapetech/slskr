use super::*;

pub(in crate::cli) fn peer_regular_port(
    address: &slskr_client::protocol::server::PeerAddress,
) -> Result<u16, String> {
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

pub(in crate::cli) fn validated_obfuscated_port(
    obfuscation_type: u32,
    port: u16,
) -> Result<u16, String> {
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

pub(in crate::cli) async fn connect_plain_peer_messages(
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

pub(in crate::cli) async fn connect_plain_file_transfer(
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

pub(in crate::cli) fn browse_payload_preview(payload: &[u8]) -> String {
    let text = String::from_utf8_lossy(payload);
    sanitize_inline_detail(&text.chars().take(240).collect::<String>())
}

pub(in crate::cli) fn sanitize_inline_detail(value: &str) -> String {
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

pub(in crate::cli) fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
