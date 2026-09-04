//! Minimal SignalR JSON-hub transport for the relay controller.
//!
//! The relay controller in the oracle is a typed SignalR hub.  slskR's HTTP
//! server owns the raw upgraded socket, so this module implements the small
//! JSON hub protocol directly on top of the existing websocket framing
//! primitives: handshake, challenge/login, token issuance, and agent result
//! callbacks.

use std::{net::IpAddr, net::SocketAddr, sync::Arc};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
    time::{self, Duration},
};
use uuid::Uuid;

use crate::{relay, AppState};

const SIGNALR_RECORD_SEPARATOR: char = '\x1e';
const MAX_SIGNALR_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_SIGNALR_MESSAGES_PER_FRAME: usize = 256;
const MAX_WEBSOCKET_FRAME_BYTES: u64 = 4 * 1024 * 1024;
pub(crate) const WEBSOCKET_READ_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const SIGNALR_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const WEBSOCKET_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const HUB_INBOUND_QUEUE_CAPACITY: usize = 8;

#[derive(Debug)]
pub(crate) enum WebSocketFrame {
    Text(String),
    Ping(Vec<u8>),
    Pong,
    Close(Vec<u8>),
}

/// Serve one already-upgraded `/hub/relay` connection.
pub(crate) async fn serve<R, W>(
    mut reader: R,
    writer: &mut W,
    state: Arc<AppState>,
    remote_addr: Option<SocketAddr>,
) -> Result<(), String>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    let handshake = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await?;
    let WebSocketFrame::Text(handshake) = handshake else {
        return Err("relay SignalR handshake must be a text frame".to_owned());
    };
    let mut initial_messages = signalr_messages(&handshake)?;
    let Some(first) = initial_messages.first() else {
        return Err("relay SignalR handshake is empty".to_owned());
    };
    let handshake_value = serde_json::from_str::<Value>(first)
        .map_err(|_| "relay SignalR handshake is not valid JSON".to_owned())?;
    if !handshake_value.is_object() {
        return Err("relay SignalR handshake must be a JSON object".to_owned());
    }
    write_signalr_json(writer, &json!({})).await?;
    initial_messages.drain(0..1);

    let settings = state.advanced_networking.read().await.relay.clone();
    if !settings.enabled || !matches!(settings.mode.as_str(), "controller" | "debug") {
        return Err("relay is not enabled in controller mode".to_owned());
    }

    let connection_id = format!("relay-{}", Uuid::new_v4().simple());
    let now = crate::unix_timestamp();
    let challenge = state
        .relay
        .write()
        .await
        .protocol
        .issue_challenge(&connection_id, now);
    if let Err(error) = write_signalr_json(
        writer,
        &json!({
            "type": 1,
            "target": "Challenge",
            "arguments": [challenge],
        }),
    )
    .await
    {
        state
            .relay
            .write()
            .await
            .protocol
            .deregister_connection(&connection_id);
        return Err(error);
    }

    // Do not register the live sender until the challenge has reached the
    // client. If the initial write fails, there is no socket loop to perform
    // the normal cleanup path.
    let (outbound_tx, mut outbound_rx) = mpsc::channel(relay::HUB_OUTBOUND_QUEUE_CAPACITY);
    relay::register_hub_connection(connection_id.clone(), outbound_tx);

    let (inbound_tx, mut inbound_rx) = mpsc::channel(HUB_INBOUND_QUEUE_CAPACITY);
    let reader_task = tokio::spawn(async move {
        loop {
            let frame = read_ws_frame_with_timeout(&mut reader, WEBSOCKET_READ_TIMEOUT).await;
            let done = matches!(&frame, Ok(WebSocketFrame::Close(_)) | Err(_));
            if inbound_tx.send(frame).await.is_err() || done {
                break;
            }
        }
    });

    let mut keepalive = time::interval(SIGNALR_KEEPALIVE_INTERVAL);
    keepalive.tick().await;
    let remote_ip = remote_addr.map_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), |addr| {
        addr.ip()
    });
    let serve_result = async {
        for message in initial_messages {
            handle_signalr_message(&message, &state, &connection_id, remote_ip, writer).await?;
        }

        loop {
            tokio::select! {
                inbound = inbound_rx.recv() => match inbound {
                    Some(Ok(WebSocketFrame::Text(text))) => {
                        for message in signalr_messages(&text)? {
                            handle_signalr_message(&message, &state, &connection_id, remote_ip, writer).await?;
                        }
                    }
                    Some(Ok(WebSocketFrame::Ping(payload))) => write_ws_frame(writer, 0x8a, &payload).await?,
                    Some(Ok(WebSocketFrame::Pong)) => {}
                    Some(Ok(WebSocketFrame::Close(payload))) => {
                        write_ws_frame(writer, 0x88, &payload).await?;
                        return Ok(());
                    }
                    Some(Err(error)) => return Err(error),
                    None => return Ok(()),
                },
                outbound = outbound_rx.recv() => match outbound {
                    Some(message) => write_signalr_text(writer, &message).await?,
                    None => return Ok(()),
                },
                _ = keepalive.tick() => {
                    write_signalr_json(writer, &json!({"type": 6})).await?;
                },
            }
        }
    }
    .await;

    reader_task.abort();
    let _ = reader_task.await;
    relay::unregister_hub_connection(&connection_id);
    state
        .relay
        .write()
        .await
        .protocol
        .deregister_connection(&connection_id);
    serve_result
}

async fn handle_signalr_message<W>(
    message: &str,
    state: &Arc<AppState>,
    connection_id: &str,
    remote_ip: IpAddr,
    writer: &mut W,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let value = serde_json::from_str::<Value>(message)
        .map_err(|_| "relay SignalR invocation is not valid JSON".to_owned())?;
    let message_type = value.get("type").and_then(Value::as_u64).unwrap_or(0);
    if message_type == 6 {
        return write_signalr_json(writer, &json!({ "type": 6 })).await;
    }
    if message_type != 1 {
        return Ok(());
    }
    let invocation_id = value
        .get("invocationId")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let target = value
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let arguments = value
        .get("arguments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let result = match target {
        "Login" => {
            let agent_name = arguments
                .first()
                .and_then(Value::as_str)
                .unwrap_or_default();
            let credential = arguments.get(1).and_then(Value::as_str).unwrap_or_default();
            let settings = state.advanced_networking.read().await.relay.clone();
            let scheme = relay::credential_scheme(state.config.controller_profile);
            let authenticated = state.relay.write().await.protocol.authenticate_agent(
                &settings,
                scheme,
                connection_id,
                agent_name,
                credential,
                remote_ip,
                crate::unix_timestamp(),
            );
            if authenticated {
                Ok(Value::Null)
            } else {
                Err("Unauthorized".to_owned())
            }
        }
        "BeginShareUpload" => {
            let token = {
                let mut relay_state = state.relay.write().await;
                let agent = relay_state
                    .protocol
                    .registered_agent_name(connection_id)
                    .map(str::to_owned);
                agent.and_then(|agent| {
                    relay_state
                        .protocol
                        .issue_share_upload_token(&agent, crate::unix_timestamp())
                })
            };
            token.map_or_else(
                || Err("Unauthorized".to_owned()),
                |token| Ok(Value::String(token)),
            )
        }
        "ReturnFileInfo" => {
            let registered = state
                .relay
                .read()
                .await
                .protocol
                .registered_agent_name(connection_id)
                .is_some();
            let Some(id) = arguments
                .first()
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
            else {
                return invocation_error(writer, invocation_id.as_deref(), "Invalid request id")
                    .await;
            };
            let exists = arguments.get(1).and_then(Value::as_bool).unwrap_or(false);
            let length = arguments.get(2).and_then(Value::as_u64).unwrap_or_default();
            if !registered {
                Err("Unauthorized".to_owned())
            } else {
                state.relay.write().await.protocol.complete_file_info(
                    connection_id,
                    id,
                    exists,
                    length,
                );
                Ok(Value::Null)
            }
        }
        "NotifyFileUploadFailed" => {
            let registered = state
                .relay
                .read()
                .await
                .protocol
                .registered_agent_name(connection_id)
                .is_some();
            let Some(id) = arguments
                .first()
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
            else {
                return invocation_error(writer, invocation_id.as_deref(), "Invalid request id")
                    .await;
            };
            let error = arguments
                .get(1)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| "relay agent failed to provide the requested file".to_owned());
            if !registered {
                Err("Unauthorized".to_owned())
            } else {
                state.relay.write().await.protocol.fail_file_stream(
                    connection_id,
                    id,
                    error.clone(),
                );
                state
                    .relay
                    .write()
                    .await
                    .protocol
                    .fail_file_info(connection_id, id, error);
                Ok(Value::Null)
            }
        }
        _ => Err(format!("Unknown hub method: {target}")),
    };

    let Some(invocation_id) = invocation_id else {
        return Ok(());
    };
    write_invocation_completion(writer, &invocation_id, result).await
}

async fn invocation_error<W>(
    writer: &mut W,
    invocation_id: Option<&str>,
    error: &str,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let Some(invocation_id) = invocation_id else {
        return Ok(());
    };
    write_invocation_completion(writer, invocation_id, Err(error.to_owned())).await
}

async fn write_invocation_completion<W>(
    writer: &mut W,
    invocation_id: &str,
    result: Result<Value, String>,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let completion = match result {
        Ok(result) => json!({
            "type": 3,
            "invocationId": invocation_id,
            "result": result,
        }),
        Err(error) => json!({
            "type": 3,
            "invocationId": invocation_id,
            "error": error,
        }),
    };
    write_signalr_json(writer, &completion).await
}

pub(crate) fn signalr_messages(text: &str) -> Result<Vec<String>, String> {
    if text.len() > MAX_SIGNALR_MESSAGE_BYTES {
        return Err("relay SignalR message is too large".to_owned());
    }
    let mut messages = Vec::new();
    for message in text
        .split(SIGNALR_RECORD_SEPARATOR)
        .filter(|message| !message.is_empty())
    {
        if messages.len() >= MAX_SIGNALR_MESSAGES_PER_FRAME {
            return Err("relay SignalR frame contains too many messages".to_owned());
        }
        messages.push(message.to_owned());
    }
    if messages.is_empty() {
        return Err("relay SignalR message is empty".to_owned());
    }
    Ok(messages)
}

pub(crate) async fn write_signalr_json<W>(writer: &mut W, value: &Value) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    write_signalr_text(writer, &value.to_string()).await
}

pub(crate) async fn write_signalr_text<W>(writer: &mut W, text: &str) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let mut payload = text.as_bytes().to_vec();
    payload.push(SIGNALR_RECORD_SEPARATOR as u8);
    write_ws_frame(writer, 0x81, &payload).await
}

pub(crate) async fn write_ws_frame<W>(
    writer: &mut W,
    opcode: u8,
    payload: &[u8],
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() as u64 > MAX_WEBSOCKET_FRAME_BYTES {
        return Err("relay websocket response is too large".to_owned());
    }
    time::timeout(
        WEBSOCKET_WRITE_TIMEOUT,
        write_ws_frame_inner(writer, opcode, payload),
    )
    .await
    .map_err(|_| "relay websocket write deadline exceeded".to_owned())?
}

async fn write_ws_frame_inner<W>(writer: &mut W, opcode: u8, payload: &[u8]) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let mut header = Vec::with_capacity(10);
    header.push(opcode);
    match payload.len() {
        0..=125 => header.push(payload.len() as u8),
        126..=65535 => {
            header.push(126);
            header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            header.push(127);
            header.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    writer
        .write_all(&header)
        .await
        .map_err(|error| error.to_string())?;
    writer
        .write_all(payload)
        .await
        .map_err(|error| error.to_string())?;
    writer.flush().await.map_err(|error| error.to_string())
}

pub(crate) async fn read_ws_frame<R>(reader: &mut R) -> Result<WebSocketFrame, String>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0_u8; 2];
    reader
        .read_exact(&mut header)
        .await
        .map_err(|error| error.to_string())?;
    if header[0] & 0x70 != 0 {
        return Err("relay websocket frame used reserved bits".to_owned());
    }
    let opcode = header[0] & 0x0f;
    let is_control = opcode >= 0x8;
    if !matches!(opcode, 0x1 | 0x8 | 0x9 | 0xa) {
        return Err("relay websocket frame used an unsupported opcode".to_owned());
    }
    if is_control && header[0] & 0x80 == 0 {
        return Err("relay websocket control frame was fragmented".to_owned());
    }
    if !is_control && header[0] & 0x80 == 0 {
        return Err("relay websocket data frame was fragmented".to_owned());
    }
    if header[1] & 0x80 == 0 {
        return Err("relay websocket frame was not masked".to_owned());
    }
    let mut length = u64::from(header[1] & 0x7f);
    if length == 126 {
        let mut bytes = [0_u8; 2];
        reader
            .read_exact(&mut bytes)
            .await
            .map_err(|error| error.to_string())?;
        length = u64::from(u16::from_be_bytes(bytes));
        if length < 126 {
            return Err("relay websocket frame used non-canonical length".to_owned());
        }
    } else if length == 127 {
        let mut bytes = [0_u8; 8];
        reader
            .read_exact(&mut bytes)
            .await
            .map_err(|error| error.to_string())?;
        length = u64::from_be_bytes(bytes);
        if length & (1_u64 << 63) != 0 {
            return Err("relay websocket frame length used reserved high bit".to_owned());
        }
        if length <= u64::from(u16::MAX) {
            return Err("relay websocket frame used non-canonical length".to_owned());
        }
    }
    if length > MAX_WEBSOCKET_FRAME_BYTES || (is_control && length > 125) {
        return Err("relay websocket frame is too large".to_owned());
    }
    let mut mask = [0_u8; 4];
    reader
        .read_exact(&mut mask)
        .await
        .map_err(|error| error.to_string())?;
    let mut payload = vec![0_u8; length as usize];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|error| error.to_string())?;
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    Ok(match opcode {
        0x1 => WebSocketFrame::Text(
            String::from_utf8(payload)
                .map_err(|_| "relay websocket text was not UTF-8".to_owned())?,
        ),
        0x8 => {
            validate_close_payload(&payload)?;
            WebSocketFrame::Close(payload)
        }
        0x9 => WebSocketFrame::Ping(payload),
        0xa => WebSocketFrame::Pong,
        _ => unreachable!(),
    })
}

fn validate_close_payload(payload: &[u8]) -> Result<(), String> {
    if payload.len() == 1 {
        return Err("relay websocket close frame used a one-byte payload".to_owned());
    }
    if payload.len() >= 2 {
        let code = u16::from_be_bytes([payload[0], payload[1]]);
        if !matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999) {
            return Err("relay websocket close frame used an invalid status code".to_owned());
        }
        std::str::from_utf8(&payload[2..])
            .map_err(|_| "relay websocket close reason was not valid UTF-8".to_owned())?;
    }
    Ok(())
}

pub(crate) async fn read_ws_frame_with_timeout<R>(
    reader: &mut R,
    timeout: Duration,
) -> Result<WebSocketFrame, String>
where
    R: AsyncRead + Unpin,
{
    time::timeout(timeout, read_ws_frame(reader))
        .await
        .map_err(|_| "relay websocket read deadline exceeded".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masked_frame(first_byte: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(6 + payload.len());
        frame.push(first_byte);
        frame.push(0x80 | payload.len() as u8);
        frame.extend_from_slice(&[0, 0, 0, 0]);
        frame.extend_from_slice(payload);
        frame
    }

    #[tokio::test]
    async fn websocket_read_deadline_releases_blocked_reader() {
        let (_client, mut reader) = tokio::io::duplex(64);
        let error = time::timeout(
            Duration::from_millis(100),
            read_ws_frame_with_timeout(&mut reader, Duration::from_millis(10)),
        )
        .await
        .expect("read deadline")
        .expect_err("blocked websocket reader must time out");
        assert!(error.contains("read deadline exceeded"), "{error}");
    }

    #[tokio::test]
    async fn websocket_rejects_fragmented_data_frames() {
        let frame = masked_frame(0x01, b"partial");
        let error = read_ws_frame(&mut &frame[..])
            .await
            .expect_err("fragmented data frame");
        assert_eq!(error, "relay websocket data frame was fragmented");
    }

    #[tokio::test]
    async fn websocket_rejects_malformed_close_payloads() {
        for payload in [
            vec![0],
            2000_u16.to_be_bytes().to_vec(),
            [1000_u16.to_be_bytes().as_slice(), &[0xff]].concat(),
        ] {
            let frame = masked_frame(0x88, &payload);
            let error = read_ws_frame(&mut &frame[..])
                .await
                .expect_err("malformed close frame");
            assert!(error.contains("close"), "{error}");
        }
    }

    #[test]
    fn signalr_parser_rejects_message_bursts() {
        let burst = std::iter::repeat_n("{}", MAX_SIGNALR_MESSAGES_PER_FRAME + 1)
            .collect::<Vec<_>>()
            .join(&SIGNALR_RECORD_SEPARATOR.to_string());
        let error = signalr_messages(&burst).expect_err("message burst");
        assert!(error.contains("too many messages"), "{error}");
    }
}
