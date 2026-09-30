use super::*;

pub(super) async fn read_quic_data_command_line(
    receive: &mut QuicDataReceiveStream,
) -> Result<(String, Vec<u8>), QuicDataError> {
    let mut bytes = Vec::with_capacity(256);
    let mut byte = [0_u8; 1];
    while bytes.len() < 256 {
        let read = receive.read_chunk(&mut byte).await?;
        if read == 0 {
            break;
        }
        bytes.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    let line = String::from_utf8_lossy(&bytes).trim_end().to_owned();
    Ok((line, bytes))
}

pub(super) async fn read_quic_data_command_line_with_timeout(
    receive: &mut QuicDataReceiveStream,
) -> Result<(String, Vec<u8>), QuicDataError> {
    timeout(QUIC_DATA_READ_TIMEOUT, read_quic_data_command_line(receive))
        .await
        .map_err(|_| QuicDataError::Timeout("data command read"))?
}

pub(super) async fn write_quic_data_error(
    send: &mut QuicDataSendStream,
    reason: &str,
) -> Result<(), QuicDataError> {
    timeout(DESTINATION_WRITE_TIMEOUT, async {
        send.write_all(format!("ERR {reason}\n").as_bytes()).await?;
        send.finish()
    })
    .await
    .map_err(|_| QuicDataError::Timeout("data error write"))?
}

pub(super) fn relay_authentication_valid(line: &str, configured_token: &str) -> bool {
    if configured_token.is_empty() || !line.starts_with("AUTH ") {
        return false;
    }
    let Ok(presented) = BASE64.decode(line[5..].trim()) else {
        return false;
    };
    let expected = configured_token.as_bytes();
    if presented.len() != expected.len() {
        return false;
    }
    let difference = presented
        .iter()
        .zip(expected)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        });
    difference == 0
}

pub(super) fn allowed_relay_destination(policy: &QuicDataPolicy, host: &str, port: u16) -> bool {
    let requested = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    policy
        .allowed_relay_destinations
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(&requested))
}

pub(super) async fn copy_quic_to_tcp(
    mut receive: QuicDataReceiveStream,
    mut target: tokio::net::tcp::OwnedWriteHalf,
    max_bytes: u64,
) -> Result<(), String> {
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8 * 1024];
    while total < max_bytes {
        let remaining =
            usize::try_from((max_bytes - total).min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = receive
            .read_chunk(&mut buffer[..remaining])
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        target
            .write_all(&buffer[..read])
            .await
            .map_err(|error| error.to_string())?;
        total = total.saturating_add(read as u64);
    }
    Ok(())
}

pub(super) async fn copy_tcp_to_quic(
    mut source: tokio::net::tcp::OwnedReadHalf,
    mut send: QuicDataSendStream,
    max_bytes: u64,
) -> Result<(), String> {
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8 * 1024];
    while total < max_bytes {
        let remaining =
            usize::try_from((max_bytes - total).min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = source
            .read(&mut buffer[..remaining])
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        send.write_all(&buffer[..read])
            .await
            .map_err(|error| error.to_string())?;
        total = total.saturating_add(read as u64);
    }
    send.finish().map_err(|error| error.to_string())
}
