use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fmt, fs,
    io::{Read as _, Seek as _, SeekFrom},
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as StdUdpSocket},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex as StdMutex, RwLock as StdRwLock,
    },
    time::{Duration, Instant},
};

use crate::mesh_security::OverlayRateLimiter;
use crate::quic_alpn;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rcgen::generate_simple_self_signed;
use sha2::{Digest, Sha256};
use slskr_client::overlay::{
    CloseTunnelRequest, GetTunnelDataRequest, MeshHello, MeshHelloAck, MeshSearchFileDto,
    MeshSearchRequestMessage, MeshSearchResponseMessage, MeshServiceCall, MeshServiceReply,
    OpenTunnelRequest, OpenTunnelResponse, OverlayFramer, Ping, Pong, TunnelDataRequest,
    TunnelDataResponse, FEATURE_MESH_SEARCH, FEATURE_MESH_SERVICE, MAX_OVERLAY_MESSAGE_BYTES,
    OVERLAY_MAGIC, OVERLAY_VERSION,
};
use slskr_client::overlay_control::ControlEnvelope;
use slskr_client::quic_control::{QuicControlConnection, QuicControlError, QuicControlServer};
use slskr_client::quic_data::{
    QuicDataConnection, QuicDataError, QuicDataInboundStream, QuicDataReceiveStream,
    QuicDataSendStream, QuicDataServer,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{lookup_host, tcp::OwnedWriteHalf, TcpListener, TcpStream, UdpSocket},
    sync::{mpsc, Mutex, RwLock, Semaphore},
    task::{AbortHandle, JoinSet},
    time::timeout,
};
use tokio_rustls::{
    rustls::{
        pki_types::{CertificateDer, PrivatePkcs8KeyDer},
        ServerConfig,
    },
    TlsAcceptor,
};

const MAX_GATEWAY_CONNECTIONS: usize = 128;
const MAX_OVERLAY_METADATA_USERNAME_BYTES: usize = 256;
const MAX_OVERLAY_METADATA_FEATURES: usize = 20;
const MAX_OVERLAY_METADATA_FEATURE_BYTES: usize = 32;
const MAX_OVERLAY_METADATA_THUMBPRINT_BYTES: usize = 128;
const MAX_TUNNELS: usize = 128;
const MAX_TUNNELS_PER_PEER: usize = 10;
const MAX_REPLAY_NONCES: usize = 4_096;
const MAX_REPLAY_NONCES_PER_PEER: usize = 128;
const MAX_POD_ID_BYTES: usize = 512;
const MAX_DESTINATION_HOST_BYTES: usize = 255;
const MAX_SERVICE_NAME_BYTES: usize = 128;
const MAX_REQUEST_NONCE_BYTES: usize = 64;
const MAX_CERTIFICATE_BYTES: u64 = 64 * 1024;
const MAX_PRIVATE_KEY_BYTES: u64 = 16 * 1024;
const REQUEST_FRESHNESS_SECONDS: u64 = 300;
const DESTINATION_RESOLVE_TIMEOUT: Duration = Duration::from_secs(5);
const DESTINATION_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DESTINATION_WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_MESSAGE_READ_TIMEOUT: Duration = Duration::from_secs(30);
const OVERLAY_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(2 * 60);
const OVERLAY_IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const QUIC_DATA_READ_TIMEOUT: Duration = Duration::from_secs(30);
const INBOUND_BUFFER_CHUNKS: usize = 64;
const TUNNEL_CHUNK_BYTES: usize = 8 * 1024;
const MAX_POD_MESSAGE_BODY_BYTES: usize = 4 * 1024;
const QUIC_DATA_MAX_PAYLOAD_BYTES: usize = slskr_client::quic_data::DEFAULT_MAX_PAYLOAD_BYTES;
const QUIC_PROXY_MAX_SESSIONS: usize = 128;
const QUIC_PROXY_PREFIX_SESSION_LIMIT: usize = 8;
const QUIC_PROXY_GLOBAL_ATTEMPT_LIMIT: usize = 64;
const QUIC_PROXY_PREFIX_ATTEMPT_LIMIT: usize = 4;
const QUIC_PROXY_ATTEMPT_WINDOW: Duration = Duration::from_secs(10);
const QUIC_PROXY_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const QUIC_PROXY_PENDING_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_MESH_CONTENT_BYTES: usize = 32 * 1024;
const MAX_CONTENT_ID_BYTES: usize = 512;
const MAX_SHADOW_MBID_BYTES: usize = 100;
const MAX_SHADOW_BATCH: usize = 20;
#[path = "private_gateway_owners/gateway_identity.rs"]
mod gateway_identity;
use self::gateway_identity::load_or_create_certificate;
#[path = "private_gateway_owners/gateway_models.rs"]
mod gateway_models;
pub use self::gateway_models::{
    Gateway, OutboundOverlayGuard, OverlayConnectionMetadata, QuicDataPolicy,
};
use self::gateway_models::{
    GatewayConnectionAdmission, OverlayLiveness, OverlayMetadataGuard, Tunnel,
};
#[path = "private_gateway_owners/gateway_services.rs"]
mod gateway_services;
#[path = "private_gateway_owners/gateway_transport.rs"]
mod gateway_transport;
#[path = "private_gateway_owners/mesh_content_projection.rs"]
mod mesh_content_projection;
use self::mesh_content_projection::{
    mesh_content_range, mesh_search_error_response, mesh_search_file_dto, overlay_timestamp,
    shadow_index_result, valid_shadow_mbid, MeshContentRequest, ShadowBatchRequest,
    ShadowQueryRequest,
};
#[path = "private_gateway_owners/peer_authentication.rs"]
mod peer_authentication;
use self::peer_authentication::{
    authenticate_overlay_peer, gateway_peer_identity, gateway_replay_nonce_key,
    resolve_destination, resolve_public_relay_destination, valid_open_tunnel_request,
};
#[path = "private_gateway_owners/pod_request_models.rs"]
mod pod_request_models;
use self::pod_request_models::{
    PodControlMessage, PodIdRequest, PodMessageRequest, PodMessagesRequest,
};
#[path = "private_gateway_owners/quic_proxy.rs"]
mod quic_proxy;
use self::quic_proxy::{
    forward_dht_responses, is_dht_packet, is_quic_initial_packet, overlay_datagram_limiter_id,
    prune_quic_proxy_sessions, select_quic_proxy_backend, QuicProxyAdmissionGate, QuicProxySession,
};
#[path = "private_gateway_owners/quic_relay.rs"]
mod quic_relay;
use self::quic_relay::{
    allowed_relay_destination, copy_quic_to_tcp, copy_tcp_to_quic,
    read_quic_data_command_line_with_timeout, relay_authentication_valid, write_quic_data_error,
};
#[path = "private_gateway_owners/service_policy.rs"]
mod service_policy;
use self::service_policy::{
    bounded_required, local_service_enabled, overlay_service_enabled, parse_payload, service_reply,
    valid_service_call,
};
#[cfg(test)]
#[path = "private_gateway_owners/tests.rs"]
mod tests;

#[cfg(test)]
use self::gateway_identity::{read_identity_file, write_new_identity, write_secret};

#[cfg(test)]
use self::peer_authentication::{
    valid_destination_ip, valid_public_relay_ip, verify_overlay_peer_authentication,
};
