use super::fixtures::*;
use std::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

struct PendingReader {
    bytes: Vec<u8>,
    ready: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
}

impl AsyncRead for PendingReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.bytes.is_empty() {
            self.ready.store(true, Ordering::SeqCst);
            return Poll::Pending;
        }
        let count = self.bytes.len().min(buffer.remaining());
        buffer.put_slice(&self.bytes[..count]);
        self.bytes.drain(..count);
        Poll::Ready(Ok(()))
    }
}

impl Drop for PendingReader {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

fn signalr_handshake() -> Vec<u8> {
    let payload = b"{\"protocol\":\"json\",\"version\":1}\x1e";
    let mask = [1, 2, 3, 4];
    let mut frame = vec![0x81, 0x80 | u8::try_from(payload.len()).unwrap()];
    frame.extend(mask);
    frame.extend(
        payload
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ mask[i % 4]),
    );
    frame
}

#[tokio::test]
async fn cancelled_event_and_signalr_connections_drop_pending_readers() {
    for signalr in [false, true] {
        let (state, _receiver) = test_state_with_env(MapEnv::default());
        let ready = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let reader = PendingReader {
            bytes: if signalr {
                signalr_handshake()
            } else {
                Vec::new()
            },
            ready: Arc::clone(&ready),
            dropped: Arc::clone(&dropped),
        };
        let task_state = Arc::clone(&state);
        struct OwnedConnection(tokio::task::JoinHandle<Result<(), String>>);
        impl Drop for OwnedConnection {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let mut task = OwnedConnection(tokio::spawn(async move {
            let mut writer = tokio::io::sink();
            if signalr {
                crate::signalr_ws::serve(reader, &mut writer, task_state, "application").await
            } else {
                crate::events_ws::stream_events(
                    reader,
                    &mut writer,
                    &task_state.events,
                    task_state.event_tx.subscribe(),
                )
                .await
            }
        }));
        tokio::time::timeout(Duration::from_secs(2), async {
            while !ready.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("reader entered pending frame read");
        task.0.abort();
        assert!((&mut task.0).await.unwrap_err().is_cancelled());
        assert!(
            dropped.load(Ordering::SeqCst),
            "reader must drop with its connection"
        );
        state.shutdown_managed_tasks().await;
        fs::remove_dir_all(&state.config.state_dir).unwrap();
    }
}

#[tokio::test]
async fn track_processing_rejects_admission_after_shutdown() {
    let (state, _receiver) =
        test_state_with_env(MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"));
    let intent = state
        .virtual_soulfind_v2
        .write()
        .await
        .enqueue_track(
            &serde_json::json!("Music"),
            &uuid::Uuid::new_v4().to_string(),
            &serde_json::json!("Normal"),
            None,
        )
        .unwrap();
    let id = intent["desiredTrackId"].as_str().unwrap();
    state.shutdown_managed_tasks().await;
    let response = crate::route_http_request(
        "POST",
        &format!("/api/virtualsoulfind/v2/intents/tracks/{id}/process"),
        None,
        "",
        &state,
    )
    .await
    .unwrap();
    assert_eq!(response.status, "503 Service Unavailable");
    assert_eq!(
        state.virtual_soulfind_v2.read().await.track(id).unwrap()["status"],
        "Pending"
    );
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}
