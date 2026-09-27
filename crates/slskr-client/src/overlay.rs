use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::time::timeout;
use tokio_rustls::{
    client::TlsStream,
    rustls::{
        self,
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        crypto::WebPkiSupportedAlgorithms,
        pki_types::{CertificateDer, ServerName, UnixTime},
        server::ParsedCertificate,
        ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    },
    TlsConnector,
};
use uuid::Uuid;

pub const OVERLAY_MAGIC: &str = "SLSKDNM1";
pub const OVERLAY_VERSION: i32 = 1;
pub const MAX_OVERLAY_MESSAGE_BYTES: usize = 64 * 1024;
pub const FEATURE_MESH_SERVICE: &str = "mesh_service";
pub const FEATURE_MESH_SEARCH: &str = "mesh_search";
const MAX_HANDSHAKE_FEATURES: usize = 20;
const MAX_FEATURE_BYTES: usize = 32;
const MAX_USERNAME_BYTES: usize = 64;
const MAX_NONCE_BYTES: usize = 64;
const MAX_SERVICE_FIELD_BYTES: usize = 128;
const MAX_SERVICE_ERROR_BYTES: usize = 1_024;
const MAX_DISCONNECT_REASON_BYTES: usize = 256;
const MAX_SEARCH_TEXT_BYTES: usize = 1_024;
const MAX_POD_ID_BYTES: usize = 512;
const MAX_DESTINATION_HOST_BYTES: usize = 255;
const MAX_UNMATCHED_SERVICE_FRAMES: usize = 32;
const MAX_SERVICE_CONTROL_FRAMES: usize = 256;
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const PROTOCOL_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const SERVICE_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const CONTROL_TIMESTAMP_SKEW_MILLIS: u64 = 24 * 60 * 60 * 1_000;

mod client;
mod frame;
mod messages;

pub use client::*;
pub use frame::OverlayFramer;
pub use messages::*;
