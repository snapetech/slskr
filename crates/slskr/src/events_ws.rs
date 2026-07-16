//! WebSocket event feed for `/api/events/ws`.

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::{broadcast, mpsc, RwLock},
    time::{self, Duration},
};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;

use crate::{EventRecord, EventStore};
use base64::{engine::general_purpose::STANDARD, Engine};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const WEBSOCKET_KEY_DECODED_LEN: usize = 16;
const CLIENT_FRAME_CHANNEL_CAPACITY: usize = 16;
const WEBSOCKET_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

pub fn valid_sec_websocket_key(sec_websocket_key: &str) -> bool {
    let Ok(decoded) = STANDARD.decode(sec_websocket_key.as_bytes()) else {
        return false;
    };
    decoded.len() == WEBSOCKET_KEY_DECODED_LEN
}

pub async fn write_upgrade_response<W>(
    writer: &mut W,
    sec_websocket_key: &str,
    sec_websocket_protocol: Option<&str>,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let accept_key = derive_accept_key(sec_websocket_key.as_bytes());
    let protocol_header = sec_websocket_protocol
        .map(|protocol| format!("Sec-WebSocket-Protocol: {protocol}\r\n"))
        .unwrap_or_default();
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept_key}\r\n\
         {protocol_header}\
         \r\n"
    );
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    writer.flush().await.map_err(|error| error.to_string())
}

pub async fn stream_events<R, W>(
    reader: R,
    writer: &mut W,
    events: &RwLock<EventStore>,
    receiver: broadcast::Receiver<EventRecord>,
) -> Result<(), String>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    stream_events_with_heartbeat(reader, writer, events, receiver, HEARTBEAT_INTERVAL).await
}

async fn stream_events_with_heartbeat<R, W>(
    mut reader: R,
    writer: &mut W,
    events: &RwLock<EventStore>,
    mut receiver: broadcast::Receiver<EventRecord>,
    heartbeat_interval: Duration,
) -> Result<(), String>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    let mut last_sent_id = 0;

    let replay = {
        let events = events.read().await;
        events.records.clone()
    };
    for record in replay {
        write_text_frame(writer, &event_frame_json(&record)).await?;
        last_sent_id = record.id;
    }

    let (frame_tx, mut frame_rx) = mpsc::channel(CLIENT_FRAME_CHANNEL_CAPACITY);
    let reader_task = tokio::spawn(async move {
        loop {
            let frame = read_client_frame(&mut reader).await;
            let done = matches!(frame, Ok(ClientFrame::Close) | Err(_));
            if frame_tx.send(frame).await.is_err() || done {
                break;
            }
        }
    });

    let mut heartbeat = time::interval(heartbeat_interval);
    let mut awaiting_pong = false;
    let result = async {
        loop {
            tokio::select! {
                frame = frame_rx.recv() => match frame {
                    Some(Ok(ClientFrame::Close)) | None => return Ok(()),
                    Some(Ok(ClientFrame::Ping(payload))) => write_frame(writer, 0x8a, &payload).await?,
                    Some(Ok(ClientFrame::Pong)) => awaiting_pong = false,
                    Some(Err(error)) => return Err(error),
                },
            received = receiver.recv() => match received {
                Ok(record) => {
                if record.id > last_sent_id {
                    write_text_frame(writer, &event_frame_json(&record)).await?;
                    last_sent_id = record.id;
                }
            }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                let events = events.read().await;
                let missed = events
                    .records
                    .iter()
                    .filter(|record| record.id > last_sent_id)
                    .cloned()
                    .collect::<Vec<_>>();
                drop(events);
                for record in missed {
                    write_text_frame(writer, &event_frame_json(&record)).await?;
                    last_sent_id = record.id;
                }
            }
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
            _ = heartbeat.tick() => {
                if awaiting_pong {
                    return Err("websocket heartbeat pong deadline exceeded".to_owned());
                }
                write_ping_frame(writer).await?;
                awaiting_pong = true;
            },
            }
        }
    }
    .await;

    reader_task.abort();
    result
}

#[derive(Debug)]
enum ClientFrame {
    Close,
    Ping(Vec<u8>),
    Pong,
}

async fn read_client_frame<R>(reader: &mut R) -> Result<ClientFrame, String>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0_u8; 2];
    reader
        .read_exact(&mut header)
        .await
        .map_err(|error| error.to_string())?;
    let fin = header[0] & 0x80 != 0;
    let reserved_bits = header[0] & 0x70;
    let opcode = header[0] & 0x0f;
    let is_control = opcode >= 0x8;
    if reserved_bits != 0 {
        return Err("client websocket frame used reserved bits".to_owned());
    }
    if !matches!(opcode, 0x0 | 0x1 | 0x2 | 0x8 | 0x9 | 0xa) {
        return Err("client websocket frame used reserved opcode".to_owned());
    }
    if is_control && !fin {
        return Err("client websocket control frame was fragmented".to_owned());
    }
    let masked = header[1] & 0x80 != 0;
    if !masked {
        return Err("client websocket frame was not masked".to_owned());
    }
    let mut len = u64::from(header[1] & 0x7f);
    if len == 126 {
        let mut extended = [0_u8; 2];
        reader
            .read_exact(&mut extended)
            .await
            .map_err(|error| error.to_string())?;
        len = u64::from(u16::from_be_bytes(extended));
        if len < 126 {
            return Err("client websocket frame used non-canonical length".to_owned());
        }
    } else if len == 127 {
        let mut extended = [0_u8; 8];
        reader
            .read_exact(&mut extended)
            .await
            .map_err(|error| error.to_string())?;
        len = u64::from_be_bytes(extended);
        if len & (1_u64 << 63) != 0 {
            return Err("client websocket frame length used reserved high bit".to_owned());
        }
        if len <= u64::from(u16::MAX) {
            return Err("client websocket frame used non-canonical length".to_owned());
        }
    }
    if len > 64 * 1024 {
        return Err("client websocket frame is too large".to_owned());
    }
    if is_control && len > 125 {
        return Err("client websocket control frame is too large".to_owned());
    }
    if !is_control {
        return Err("event websocket does not accept client data frames".to_owned());
    }
    let mut mask = [0_u8; 4];
    reader
        .read_exact(&mut mask)
        .await
        .map_err(|error| error.to_string())?;
    let mut payload = vec![0_u8; len as usize];
    if len > 0 {
        reader
            .read_exact(&mut payload)
            .await
            .map_err(|error| error.to_string())?;
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    match opcode {
        0x8 => Ok(ClientFrame::Close),
        0x9 => Ok(ClientFrame::Ping(payload)),
        0xa => Ok(ClientFrame::Pong),
        _ => unreachable!("validated websocket control opcode"),
    }
}

fn event_frame_json(record: &EventRecord) -> String {
    serde_json::json!({
        "topic": record.topic(),
        "id": record.id,
        "type": &record.kind,
        "data": record.data_json(),
        "timestamp": record.created_at,
    })
    .to_string()
}

async fn write_ping_frame<W>(writer: &mut W) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    write_frame(writer, 0x89, &[]).await
}

async fn write_text_frame<W>(writer: &mut W, payload: &str) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    write_frame(writer, 0x81, payload.as_bytes()).await
}

async fn write_frame<W>(writer: &mut W, opcode: u8, payload: &[u8]) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    write_frame_with_timeout(writer, opcode, payload, WEBSOCKET_WRITE_TIMEOUT).await
}

async fn write_frame_with_timeout<W>(
    writer: &mut W,
    opcode: u8,
    payload: &[u8],
    timeout: Duration,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    time::timeout(timeout, write_frame_inner(writer, opcode, payload))
        .await
        .map_err(|_| "websocket write deadline exceeded".to_owned())?
}

async fn write_frame_inner<W>(writer: &mut W, opcode: u8, payload: &[u8]) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let mut header = Vec::with_capacity(10);
    header.push(opcode);
    match payload.len() {
        0..=125 => header.push(payload.len() as u8),
        126..=65_535 => {
            header.push(126);
            header.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        len => {
            header.push(127);
            header.extend_from_slice(&(len as u64).to_be_bytes());
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::StreamExt;
    use tokio::{
        io::{BufReader, BufWriter},
        net::TcpListener,
        sync::{broadcast, oneshot, RwLock},
        time::{self, Duration},
    };
    use tokio_tungstenite::connect_async;

    use super::*;
    use crate::{http_server, EventStore};

    #[test]
    fn websocket_key_must_be_base64_nonce() {
        assert!(valid_sec_websocket_key("dGhlIHNhbXBsZSBub25jZQ=="));
        assert!(!valid_sec_websocket_key("not-a-websocket-key"));
        assert!(!valid_sec_websocket_key("c2hvcnQ="));
    }

    #[test]
    fn websocket_event_frame_escapes_untrusted_record_fields() {
        let record = EventRecord {
            id: 7,
            kind: "custom\",\"injected\":true,\"ignored\":\"".to_owned(),
            resource: "resource\"\\\n".to_owned(),
            detail: Some("detail\"\\\n".to_owned()),
            created_at: 11,
        };

        let frame = event_frame_json(&record);
        let parsed: serde_json::Value = serde_json::from_str(&frame).expect("valid event JSON");
        assert_eq!(parsed["type"], record.kind);
        assert_eq!(parsed["data"]["resource"], record.resource);
        assert_eq!(parsed["data"]["detail"], record.detail.unwrap());
        assert!(parsed.get("injected").is_none());
    }

    #[tokio::test]
    async fn upgrade_response_echoes_accepted_subprotocol() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let mut writer = Vec::new();
        write_upgrade_response(&mut writer, key, Some("slskr.api-token.route%2Dtoken"))
            .await
            .unwrap();
        let response = String::from_utf8(writer).unwrap();
        assert!(response.contains("HTTP/1.1 101 Switching Protocols\r\n"));
        assert!(response.contains("Sec-WebSocket-Protocol: slskr.api-token.route%2Dtoken\r\n"));
    }

    #[tokio::test]
    async fn websocket_client_receives_broadcast_event() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let events = Arc::new(RwLock::new(EventStore::new(10)));
        let (event_tx, _) = broadcast::channel(10);
        let (ready_tx, ready_rx) = oneshot::channel();

        let server_events = Arc::clone(&events);
        let server_event_rx = event_tx.subscribe();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read_half, write_half) = stream.into_split();
            let mut reader = BufReader::new(read_half);
            let mut writer = BufWriter::new(write_half);
            let (request, _) = http_server::read_http_request(&mut reader)
                .await
                .unwrap()
                .unwrap();
            let key = request.headers.sec_websocket_key.as_deref().unwrap();
            write_upgrade_response(&mut writer, key, None)
                .await
                .unwrap();
            let _ = ready_tx.send(());
            let _ = stream_events(reader, &mut writer, &server_events, server_event_rx).await;
        });

        // Local-only test listener: the production event feed should be served behind TLS as wss://.
        let (mut socket, _) = connect_async(format!("ws://{address}/api/events/ws")) // nosemgrep: javascript.lang.security.detect-insecure-websocket.detect-insecure-websocket
            .await
            .unwrap();
        ready_rx.await.unwrap();

        let record = {
            let mut events = events.write().await;
            events.record("search.started", "42", Some("query=ambient".to_string()))
        };
        event_tx.send(record).unwrap();

        let message = time::timeout(Duration::from_secs(2), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let text = message.to_text().unwrap();
        assert!(text.contains(r#""topic":"searches""#), "{text}");
        assert!(text.contains(r#""type":"search.started""#), "{text}");
        assert!(text.contains(r#""resource":"42""#), "{text}");
    }

    fn masked_client_frame(first_byte: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(6 + payload.len());
        frame.push(first_byte);
        frame.push(0x80 | payload.len() as u8);
        frame.extend_from_slice(&[0, 0, 0, 0]);
        frame.extend_from_slice(payload);
        frame
    }

    #[tokio::test]
    async fn websocket_client_frame_rejects_reserved_bits() {
        let frame = masked_client_frame(0xc1, b"hello");
        let mut reader = &frame[..];
        let error = read_client_frame(&mut reader).await.unwrap_err();
        assert_eq!(error, "client websocket frame used reserved bits");
    }

    #[tokio::test]
    async fn websocket_client_frame_rejects_reserved_opcode() {
        let frame = masked_client_frame(0x83, b"hello");
        let mut reader = &frame[..];
        let error = read_client_frame(&mut reader).await.unwrap_err();
        assert_eq!(error, "client websocket frame used reserved opcode");
    }

    #[tokio::test]
    async fn event_websocket_rejects_client_data_before_payload_read() {
        for opcode in [0x80, 0x81, 0x82] {
            let frame = masked_client_frame(opcode, b"hello");
            let mut reader = &frame[..];
            let error = read_client_frame(&mut reader).await.unwrap_err();
            assert_eq!(error, "event websocket does not accept client data frames");
            assert_eq!(reader.len(), 9, "mask and payload should remain unread");
        }
    }

    #[tokio::test]
    async fn websocket_client_frame_rejects_non_canonical_lengths() {
        let mut short_as_u16 = vec![0x81, 0xfe, 0, 125];
        short_as_u16.extend_from_slice(&[0, 0, 0, 0]);
        let error = read_client_frame(&mut &short_as_u16[..]).await.unwrap_err();
        assert_eq!(error, "client websocket frame used non-canonical length");

        let mut u16_as_u64 = vec![0x81, 0xff];
        u16_as_u64.extend_from_slice(&u64::from(u16::MAX).to_be_bytes());
        u16_as_u64.extend_from_slice(&[0, 0, 0, 0]);
        let error = read_client_frame(&mut &u16_as_u64[..]).await.unwrap_err();
        assert_eq!(error, "client websocket frame used non-canonical length");
    }

    #[tokio::test]
    async fn websocket_client_frame_rejects_reserved_length_high_bit() {
        let mut frame = vec![0x81, 0xff];
        frame.extend_from_slice(&(1_u64 << 63).to_be_bytes());
        frame.extend_from_slice(&[0, 0, 0, 0]);
        let error = read_client_frame(&mut &frame[..]).await.unwrap_err();
        assert_eq!(
            error,
            "client websocket frame length used reserved high bit"
        );
    }

    #[tokio::test]
    async fn websocket_missing_pong_releases_connection() {
        let (reader, _idle_client) = tokio::io::duplex(64);
        let events = RwLock::new(EventStore::new(10));
        let (_event_tx, receiver) = broadcast::channel(1);
        let mut writer = Vec::new();

        let error = time::timeout(
            Duration::from_millis(100),
            stream_events_with_heartbeat(
                reader,
                &mut writer,
                &events,
                receiver,
                Duration::from_millis(10),
            ),
        )
        .await
        .expect("heartbeat deadline")
        .expect_err("missing pong must close websocket");

        assert!(error.contains("pong deadline"), "{error}");
        assert_eq!(writer, vec![0x89, 0]);
    }

    #[tokio::test]
    async fn websocket_write_deadline_releases_blocked_writer() {
        let (mut writer, _unread_peer) = tokio::io::duplex(64);
        let payload = vec![b'x'; 1024 * 1024];
        let error =
            write_frame_with_timeout(&mut writer, 0x82, &payload, Duration::from_millis(50))
                .await
                .expect_err("blocked websocket writer must time out");
        assert!(error.contains("deadline exceeded"), "{error}");
    }
}
