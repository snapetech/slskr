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

pub fn valid_sec_websocket_key(sec_websocket_key: &str) -> bool {
    let Ok(decoded) = STANDARD.decode(sec_websocket_key.as_bytes()) else {
        return false;
    };
    decoded.len() == WEBSOCKET_KEY_DECODED_LEN
}

pub async fn write_upgrade_response<W>(
    writer: &mut W,
    sec_websocket_key: &str,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let accept_key = derive_accept_key(sec_websocket_key.as_bytes());
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept_key}\r\n\
         \r\n"
    );
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|error| error.to_string())?;
    writer.flush().await.map_err(|error| error.to_string())
}

pub async fn stream_events<R, W>(
    mut reader: R,
    writer: &mut W,
    events: &RwLock<EventStore>,
    mut receiver: broadcast::Receiver<EventRecord>,
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

    let (frame_tx, mut frame_rx) = mpsc::unbounded_channel();
    let reader_task = tokio::spawn(async move {
        loop {
            let frame = read_client_frame(&mut reader).await;
            let done = matches!(frame, Ok(ClientFrame::Close) | Err(_));
            if frame_tx.send(frame).is_err() || done {
                break;
            }
        }
    });

    let mut heartbeat = time::interval(HEARTBEAT_INTERVAL);
    let result = async {
        loop {
            tokio::select! {
                frame = frame_rx.recv() => match frame {
                    Some(Ok(ClientFrame::Close)) | None => return Ok(()),
                    Some(Ok(ClientFrame::Ping(payload))) => write_frame(writer, 0x8a, &payload).await?,
                    Some(Ok(ClientFrame::Pong | ClientFrame::Other)) => {}
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
            _ = heartbeat.tick() => write_ping_frame(writer).await?,
            }
        }
    }
    .await;

    reader_task.abort();
    result
}

enum ClientFrame {
    Close,
    Ping(Vec<u8>),
    Pong,
    Other,
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
    let opcode = header[0] & 0x0f;
    let is_control = opcode >= 0x8;
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
    } else if len == 127 {
        let mut extended = [0_u8; 8];
        reader
            .read_exact(&mut extended)
            .await
            .map_err(|error| error.to_string())?;
        len = u64::from_be_bytes(extended);
    }
    if len > 64 * 1024 {
        return Err("client websocket frame is too large".to_owned());
    }
    if is_control && len > 125 {
        return Err("client websocket control frame is too large".to_owned());
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
        _ => Ok(ClientFrame::Other),
    }
}

fn event_frame_json(record: &EventRecord) -> String {
    let data = record.json();
    format!(
        "{{\"topic\":\"{}\",\"id\":{},\"type\":\"{}\",\"data\":{},\"timestamp\":{}}}",
        topic_for_kind(record.kind),
        record.id,
        record.kind,
        data,
        record.created_at,
    )
}

fn topic_for_kind(kind: &str) -> &'static str {
    match kind.split('.').next().unwrap_or(kind) {
        "search" => "searches",
        "transfer" => "transfers",
        "message" => "messages",
        "room" => "rooms",
        "user" => "users",
        "session" => "application",
        _ => "application",
    }
}

async fn write_ping_frame<W>(writer: &mut W) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    writer
        .write_all(&[0x89, 0x00])
        .await
        .map_err(|error| error.to_string())?;
    writer.flush().await.map_err(|error| error.to_string())
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
            write_upgrade_response(&mut writer, key).await.unwrap();
            let _ = ready_tx.send(());
            let _ = stream_events(reader, &mut writer, &server_events, server_event_rx).await;
        });

        let (mut socket, _) = connect_async(format!("ws://{address}/api/events/ws"))
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
}
