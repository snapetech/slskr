use std::{
    future::Future,
    path::{Path, PathBuf},
    time::Duration,
};

use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use slskr_client::overlay::{
    connect_tls_overlay, MeshHello, MeshServiceCall, FEATURE_MESH_SERVICE,
};
use slskr_client::overlay_control::{send_udp_control, ControlEnvelope};
use tokio::io::AsyncWriteExt;
use tokio::time::timeout;

use crate::config::TrustedMeshPeer;

const CONTENT_CHUNK_BYTES: u64 = 32 * 1024;
const MAX_CONTENT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_CONTENT_ID_BYTES: usize = 512;
const MAX_POD_MESSAGE_BYTES: usize = 16 * 1024;
const CONTENT_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const POD_MESSAGE_CALL_TIMEOUT: Duration = Duration::from_secs(10);

struct StagingFileGuard {
    file: Option<tokio::fs::File>,
    path: PathBuf,
    committed: bool,
}

impl StagingFileGuard {
    fn new(path: &Path, file: tokio::fs::File) -> Self {
        Self {
            file: Some(file),
            path: path.to_owned(),
            committed: false,
        }
    }

    fn file_mut(&mut self) -> Result<&mut tokio::fs::File, String> {
        self.file
            .as_mut()
            .ok_or_else(|| "mesh content staging file is closed".to_owned())
    }

    fn commit(&mut self) {
        self.file.take();
        self.committed = true;
    }
}

impl Drop for StagingFileGuard {
    fn drop(&mut self) {
        self.file.take();
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub async fn fetch_content(
    peer: &TrustedMeshPeer,
    local_username: &str,
    authentication_key: &SigningKey,
    content_id: &str,
    size: u64,
    expected_sha256: &str,
    output: &Path,
) -> Result<(), String> {
    validate_request(local_username, content_id, size, expected_sha256)?;
    fetch_content_inner(
        peer,
        local_username,
        authentication_key,
        content_id,
        size,
        expected_sha256,
        output,
    )
    .await
}

/// Deliver a PodCore message to a configured trusted mesh peer through the
/// same authenticated `pods/PostMessage` service used by the frozen runtime's
/// mesh adapter.  The caller must supply a trusted certificate pin; capability
/// records alone are not sufficient to authenticate an overlay connection.
pub async fn post_pod_message(
    peer: &TrustedMeshPeer,
    local_username: &str,
    authentication_key: &SigningKey,
    pod_id: &str,
    channel_id: &str,
    body: &str,
    signature: &str,
) -> Result<String, String> {
    validate_pod_message(local_username, pod_id, channel_id, body, signature)?;
    let mut hello = MeshHello::new(
        local_username,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        uuid::Uuid::new_v4().simple().to_string(),
    )
    .map_err(|error| format!("pod message hello failed: {error}"))?;
    hello
        .authenticate(authentication_key, &peer.certificate_sha256)
        .map_err(|error| format!("pod message hello authentication failed: {error}"))?;
    let mut client = connect_tls_overlay(peer.overlay_endpoint, peer.certificate_sha256, hello)
        .await
        .map_err(|error| format!("pod message connection failed: {error}"))?;
    if !client.remote_username.eq_ignore_ascii_case(&peer.username) {
        return Err("pod message overlay identity did not match the trusted peer".to_owned());
    }

    let payload = serde_json::to_vec(&serde_json::json!({
        "PodId": pod_id,
        "ChannelId": channel_id,
        "Body": body,
        "Signature": signature,
    }))
    .map_err(|error| format!("pod message request encode failed: {error}"))?;
    let call = MeshServiceCall::new(
        uuid::Uuid::new_v4().to_string(),
        "pods",
        "PostMessage",
        payload,
    )
    .map_err(|error| format!("pod message request failed: {error}"))?;
    let reply = bounded_mesh_operation(
        client.call(&call),
        "pod message call",
        POD_MESSAGE_CALL_TIMEOUT,
    )
    .await?;
    if reply.status_code != 0 {
        return Err(format!(
            "pod message peer rejected delivery with status {}: {}",
            reply.status_code,
            reply.error_message.as_deref().unwrap_or("remote error")
        ));
    }
    let response = serde_json::from_slice::<serde_json::Value>(&reply.payload)
        .map_err(|error| format!("pod message response decode failed: {error}"))?;
    if !response
        .get("Success")
        .or_else(|| response.get("success"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Err("pod message peer reported unsuccessful delivery".to_owned());
    }
    response
        .get("MessageId")
        .or_else(|| response.get("messageId"))
        .and_then(serde_json::Value::as_str)
        .filter(|message_id| !message_id.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "pod message peer omitted the delivered message ID".to_owned())
}

/// Deliver a PodCore message using the frozen slskdN UDP control envelope.
///
/// The target sends the complete `PodMessage` JSON as the envelope payload and
/// signs the envelope with its persistent Ed25519 control key.  No response is
/// expected from the UDP transport; a successful kernel send is the target's
/// `IOverlayClient.SendAsync` success condition.
pub async fn post_pod_message_control(
    peer: &TrustedMeshPeer,
    local_username: &str,
    authentication_key: &SigningKey,
    request: &PodMessageControlRequest<'_>,
) -> Result<String, String> {
    validate_pod_message(
        local_username,
        request.pod_id,
        request.channel_id,
        request.body,
        request.signature,
    )?;
    let message_id = if request.message_id.trim().is_empty() {
        uuid::Uuid::new_v4().simple().to_string()
    } else {
        request.message_id.trim().to_owned()
    };
    if message_id.len() > 2 * 1024 || message_id.chars().any(char::is_control) {
        return Err("pod message ID is invalid".to_owned());
    }
    let timestamp_unix_ms = if request.timestamp_unix_ms <= 0 {
        i64::try_from(crate::utils::unix_timestamp_millis())
            .map_err(|_| "pod message timestamp is out of range".to_owned())?
    } else {
        request.timestamp_unix_ms
    };
    let payload = serde_json::to_vec(&serde_json::json!({
        "MessageId": message_id,
        "PodId": request.pod_id,
        "ChannelId": request.channel_id,
        "SenderPeerId": local_username,
        "Body": request.body,
        "TimestampUnixMs": timestamp_unix_ms,
        "Signature": request.signature,
        "SigVersion": request.sig_version,
    }))
    .map_err(|error| format!("pod message control payload encode failed: {error}"))?;
    let envelope = ControlEnvelope::signed_at(
        "pod_message",
        payload,
        &message_id,
        timestamp_unix_ms,
        authentication_key,
    )
    .map_err(|error| format!("pod message control envelope failed: {error}"))?;
    send_udp_control(peer.overlay_endpoint, &envelope)
        .await
        .map_err(|error| format!("pod message control send failed: {error}"))?;
    Ok(message_id)
}

pub struct PodMessageControlRequest<'a> {
    pub message_id: &'a str,
    pub pod_id: &'a str,
    pub channel_id: &'a str,
    pub body: &'a str,
    pub timestamp_unix_ms: i64,
    pub signature: &'a str,
    pub sig_version: i32,
}

async fn fetch_content_inner(
    peer: &TrustedMeshPeer,
    local_username: &str,
    authentication_key: &SigningKey,
    content_id: &str,
    size: u64,
    expected_sha256: &str,
    output: &Path,
) -> Result<(), String> {
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    let file = options
        .open(output)
        .await
        .map_err(|error| format!("mesh content staging create failed: {error}"))?;
    let mut staging = StagingFileGuard::new(output, file);
    let result = async {
        let mut hello = MeshHello::new(
            local_username,
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            uuid::Uuid::new_v4().simple().to_string(),
        )
        .map_err(|error| format!("mesh content hello failed: {error}"))?;
        hello
            .authenticate(authentication_key, &peer.certificate_sha256)
            .map_err(|error| format!("mesh content hello authentication failed: {error}"))?;
        let mut client = connect_tls_overlay(peer.overlay_endpoint, peer.certificate_sha256, hello)
            .await
            .map_err(|error| format!("mesh content connection failed: {error}"))?;
        if !client.remote_username.eq_ignore_ascii_case(&peer.username) {
            return Err("mesh content overlay identity did not match the trusted peer".to_owned());
        }

        let mut hasher = Sha256::new();
        let mut offset = 0_u64;
        while offset < size {
            let length = (size - offset).min(CONTENT_CHUNK_BYTES);
            let payload = serde_json::to_vec(&serde_json::json!({
                "contentId": content_id,
                "range": {
                    "offset": offset,
                    "length": length,
                }
            }))
            .map_err(|error| format!("mesh content request encode failed: {error}"))?;
            let call = MeshServiceCall::new(
                uuid::Uuid::new_v4().to_string(),
                "MeshContent",
                "GetByContentId",
                payload,
            )
            .map_err(|error| format!("mesh content request failed: {error}"))?;
            let reply = bounded_mesh_operation(
                client.call(&call),
                "mesh content call",
                CONTENT_CALL_TIMEOUT,
            )
            .await?;
            if reply.status_code != 0 {
                return Err(format!(
                    "mesh content peer rejected range with status {}: {}",
                    reply.status_code,
                    reply.error_message.as_deref().unwrap_or("remote error")
                ));
            }
            if reply.payload.len() as u64 != length {
                return Err(format!(
                    "mesh content range length mismatch: expected {length}, received {}",
                    reply.payload.len()
                ));
            }
            staging
                .file_mut()?
                .write_all(&reply.payload)
                .await
                .map_err(|error| format!("mesh content staging write failed: {error}"))?;
            hasher.update(&reply.payload);
            offset += length;
        }
        staging
            .file_mut()?
            .sync_all()
            .await
            .map_err(|error| format!("mesh content staging sync failed: {error}"))?;
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected_sha256) {
            return Err("mesh content SHA-256 verification failed".to_owned());
        }
        Ok(())
    }
    .await;
    if result.is_ok() {
        staging.commit();
    }
    result
}

async fn bounded_mesh_operation<T, E, F>(
    operation: F,
    label: &'static str,
    deadline: Duration,
) -> Result<T, String>
where
    E: std::fmt::Display,
    F: Future<Output = Result<T, E>>,
{
    timeout(deadline, operation)
        .await
        .map_err(|_| format!("{label} timed out"))?
        .map_err(|error| format!("{label} failed: {error}"))
}

fn validate_request(
    local_username: &str,
    content_id: &str,
    size: u64,
    expected_sha256: &str,
) -> Result<(), String> {
    if local_username.trim().is_empty()
        || content_id.trim().is_empty()
        || content_id.len() > MAX_CONTENT_ID_BYTES
        || content_id.chars().any(char::is_control)
        || size == 0
        || size > MAX_CONTENT_BYTES
        || expected_sha256.len() != 64
        || !expected_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("mesh content request is invalid".to_owned());
    }
    Ok(())
}

fn validate_pod_message(
    local_username: &str,
    pod_id: &str,
    channel_id: &str,
    body: &str,
    signature: &str,
) -> Result<(), String> {
    if local_username.trim().is_empty()
        || pod_id.trim().is_empty()
        || channel_id.trim().is_empty()
        || body.trim().is_empty()
        || body.len() > MAX_POD_MESSAGE_BYTES
        || signature.len() > 2 * 1024
        || [local_username, pod_id, channel_id, body, signature]
            .iter()
            .any(|value| value.chars().any(char::is_control))
    {
        return Err("pod message request is invalid".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::{TcpListener, UdpSocket};

    #[tokio::test]
    async fn slskdn_pod_control_route_emits_signed_messagepack_envelope() {
        let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let endpoint = receiver.local_addr().unwrap();
        let peer = TrustedMeshPeer {
            peer_id: "peer".to_owned(),
            username: "remote".to_owned(),
            overlay_endpoint: endpoint,
            certificate_sha256: [1_u8; 32],
            range_endpoint: None,
        };
        let key = SigningKey::from_bytes(&[3_u8; 32]);
        let request = PodMessageControlRequest {
            message_id: "message-1",
            pod_id: "pod-1",
            channel_id: "general",
            body: "hello over control",
            timestamp_unix_ms: 1_725_000_000_123,
            signature: "pod-signature",
            sig_version: 1,
        };

        let returned_id = post_pod_message_control(&peer, "local", &key, &request)
            .await
            .expect("send pod control message");
        assert_eq!(returned_id, "message-1");

        let mut bytes = [0_u8; slskr_client::overlay_control::CONTROL_MAX_DATAGRAM_BYTES];
        let (length, _) = receiver.recv_from(&mut bytes).await.unwrap();
        let envelope = ControlEnvelope::decode(&bytes[..length]).unwrap();
        assert_eq!(envelope.message_type, "pod_message");
        envelope.verify().unwrap();
        let payload = serde_json::from_slice::<serde_json::Value>(&envelope.payload).unwrap();
        assert_eq!(payload["MessageId"], "message-1");
        assert_eq!(payload["SenderPeerId"], "local");
        assert_eq!(payload["Body"], "hello over control");
        assert_eq!(payload["SigVersion"], 1);
    }

    #[tokio::test]
    async fn existing_output_is_never_deleted_when_creation_fails() {
        let root = std::env::temp_dir().join(format!(
            "slskr-mesh-existing-output-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("existing.bin");
        std::fs::write(&output, b"owned by another operation").unwrap();
        let peer = TrustedMeshPeer {
            peer_id: "peer".to_owned(),
            username: "remote".to_owned(),
            overlay_endpoint: "127.0.0.1:9".parse().unwrap(),
            certificate_sha256: [1_u8; 32],
            range_endpoint: None,
        };
        let key = SigningKey::from_bytes(&[2_u8; 32]);

        let error = fetch_content(&peer, "local", &key, "content", 1, &"a".repeat(64), &output)
            .await
            .unwrap_err();
        assert!(error.contains("staging create failed"), "{error}");
        assert_eq!(
            std::fs::read(&output).unwrap(),
            b"owned by another operation"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mesh_staging_file_is_private_before_network_io() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "slskr-mesh-private-output-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("partial.bin");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = listener.local_addr().unwrap();
        let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            accepted_tx.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        let peer = TrustedMeshPeer {
            peer_id: "peer".to_owned(),
            username: "remote".to_owned(),
            overlay_endpoint: endpoint,
            certificate_sha256: [1_u8; 32],
            range_endpoint: None,
        };
        let key = SigningKey::from_bytes(&[2_u8; 32]);
        let output_for_fetch = output.clone();
        let fetch = tokio::spawn(async move {
            fetch_content(
                &peer,
                "local",
                &key,
                "content",
                1,
                &"a".repeat(64),
                &output_for_fetch,
            )
            .await
        });

        accepted_rx.await.unwrap();
        assert_eq!(
            std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o600
        );

        fetch.abort();
        let _ = fetch.await;
        server.abort();
        let _ = server.await;
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn oversized_content_is_rejected_before_staging_file_creation() {
        let root = std::env::temp_dir().join(format!(
            "slskr-mesh-oversized-output-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("oversized.bin");
        let peer = TrustedMeshPeer {
            peer_id: "peer".to_owned(),
            username: "remote".to_owned(),
            overlay_endpoint: "127.0.0.1:9".parse().unwrap(),
            certificate_sha256: [1_u8; 32],
            range_endpoint: None,
        };
        let key = SigningKey::from_bytes(&[2_u8; 32]);

        let error = fetch_content(
            &peer,
            "local",
            &key,
            "content",
            MAX_CONTENT_BYTES + 1,
            &"a".repeat(64),
            &output,
        )
        .await
        .unwrap_err();
        assert_eq!(error, "mesh content request is invalid");
        assert!(!output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn mesh_operations_do_not_wait_forever_for_remote_peers() {
        let error = bounded_mesh_operation(
            std::future::pending::<Result<(), &'static str>>(),
            "mesh content call",
            Duration::from_millis(10),
        )
        .await
        .unwrap_err();

        assert_eq!(error, "mesh content call timed out");
    }

    #[tokio::test]
    async fn cancelled_fetch_removes_owned_staging_file() {
        let root = std::env::temp_dir().join(format!(
            "slskr-mesh-cancelled-output-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let output = root.join("partial.bin");
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = listener.local_addr().unwrap();
        let (accepted_tx, accepted_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            accepted_tx.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        let peer = TrustedMeshPeer {
            peer_id: "peer".to_owned(),
            username: "remote".to_owned(),
            overlay_endpoint: endpoint,
            certificate_sha256: [1_u8; 32],
            range_endpoint: None,
        };
        let key = SigningKey::from_bytes(&[2_u8; 32]);
        let output_for_fetch = output.clone();
        let fetch = tokio::spawn(async move {
            fetch_content(
                &peer,
                "local",
                &key,
                "content",
                1,
                &"a".repeat(64),
                &output_for_fetch,
            )
            .await
        });

        accepted_rx.await.unwrap();
        assert!(output.exists());
        fetch.abort();
        assert!(fetch.await.unwrap_err().is_cancelled());
        assert!(!output.exists());

        server.abort();
        let _ = server.await;
        std::fs::remove_dir_all(root).unwrap();
    }
}
