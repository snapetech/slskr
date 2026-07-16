use std::path::{Path, PathBuf};

use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use slskr_client::overlay::{
    connect_tls_overlay, MeshHello, MeshServiceCall, FEATURE_MESH_SERVICE,
};
use tokio::io::AsyncWriteExt;

use crate::config::TrustedMeshPeer;

const CONTENT_CHUNK_BYTES: u64 = 32 * 1024;
const MAX_CONTENT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_CONTENT_ID_BYTES: usize = 512;

struct StagingFileGuard {
    path: PathBuf,
    committed: bool,
}

impl StagingFileGuard {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            committed: false,
        }
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for StagingFileGuard {
    fn drop(&mut self) {
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
    let mut file = options
        .open(output)
        .await
        .map_err(|error| format!("mesh content staging create failed: {error}"))?;
    let mut staging = StagingFileGuard::new(output);
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
            let reply = client
                .call(&call)
                .await
                .map_err(|error| format!("mesh content call failed: {error}"))?;
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
            file.write_all(&reply.payload)
                .await
                .map_err(|error| format!("mesh content staging write failed: {error}"))?;
            hasher.update(&reply.payload);
            offset += length;
        }
        file.sync_all()
            .await
            .map_err(|error| format!("mesh content staging sync failed: {error}"))?;
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected_sha256) {
            return Err("mesh content SHA-256 verification failed".to_owned());
        }
        Ok(())
    }
    .await;
    drop(file);
    if result.is_ok() {
        staging.commit();
    }
    result
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

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
