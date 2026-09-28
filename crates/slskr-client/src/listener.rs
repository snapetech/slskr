use slskr_protocol::{
    frame::InitFrame,
    init::{InitCode, InitMessage, MAX_PEER_INIT_FRAME_LEN},
};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    time::{self, Duration},
};

use crate::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    io::{read_init_frame_with_first_len_byte, read_obfuscated_init_frame},
    peer_cache::normalize_peer_username,
    stream::{DistributedConnection, ObfuscatedPeerMessageConnection, PeerMessageConnection},
    ClientError,
};

pub const DEFAULT_INIT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

const PIERCE_FIREWALL_FRAME_LEN: usize = 1 + std::mem::size_of::<u32>();

#[derive(Debug)]
pub enum IncomingConnection<S> {
    PeerMessages(PeerMessageConnection<S>),
    ObfuscatedPeerMessages(ObfuscatedPeerMessageConnection<S>),
    FileTransfer(FileTransferConnection<S>),
    Distributed(DistributedConnection<S>),
    PeerInit {
        username: String,
        kind: ConnectionKind,
        token: u32,
        stream: S,
        obfuscated: bool,
    },
    PierceFirewall {
        token: u32,
        stream: S,
    },
    UnknownInit {
        code: u8,
        payload: Vec<u8>,
        stream: S,
    },
}

/// A connection accepted by the current upstream-style shared TCP listener.
///
/// Mesh overlay connections are returned before TLS consumes the stream so the
/// gateway can perform its normal handshake. Soulseek connections are already
/// classified by the existing plain/type-1 demux and retain the exact stream
/// semantics used by the dedicated listener.
#[derive(Debug)]
pub enum SharedIncomingConnection<S> {
    Soulseek(IncomingConnection<S>),
    MeshOverlay(S),
}

#[derive(Debug)]
pub struct Listener {
    inner: TcpListener,
}

impl Listener {
    pub async fn bind<A>(address: A) -> Result<Self, ClientError>
    where
        A: ToSocketAddrs,
    {
        let inner = TcpListener::bind(address).await?;
        Ok(Self { inner })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ClientError> {
        Ok(self.inner.local_addr()?)
    }

    pub async fn accept(&self) -> Result<(IncomingConnection<TcpStream>, SocketAddr), ClientError> {
        self.accept_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
            .await
    }

    pub async fn accept_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<(IncomingConnection<TcpStream>, SocketAddr), ClientError> {
        time::timeout(timeout, async {
            let (stream, address) = self.inner.accept().await?;
            let incoming = demux_incoming(stream).await?;
            Ok((incoming, address))
        })
        .await
        .map_err(|_| ClientError::TimedOut {
            operation: "peer initialization handshake",
        })?
    }

    pub async fn accept_raw(&self) -> Result<(TcpStream, SocketAddr), ClientError> {
        Ok(self.inner.accept().await?)
    }

    pub async fn accept_obfuscated(
        &self,
    ) -> Result<(IncomingConnection<TcpStream>, SocketAddr), ClientError> {
        self.accept_obfuscated_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
            .await
    }

    pub async fn accept_obfuscated_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<(IncomingConnection<TcpStream>, SocketAddr), ClientError> {
        time::timeout(timeout, async {
            let (stream, address) = self.inner.accept().await?;
            let incoming = demux_obfuscated_incoming(stream).await?;
            Ok((incoming, address))
        })
        .await
        .map_err(|_| ClientError::TimedOut {
            operation: "obfuscated peer initialization handshake",
        })?
    }

    /// Accept plain or type-1 framed Soulseek initialization from a shared
    /// endpoint. Legacy one-byte connection-kind traffic remains on the
    /// dedicated [`Self::accept`] contract rather than this shared endpoint.
    pub async fn accept_shared(
        &self,
    ) -> Result<(IncomingConnection<TcpStream>, SocketAddr), ClientError> {
        self.accept_shared_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
            .await
    }

    pub async fn accept_shared_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<(IncomingConnection<TcpStream>, SocketAddr), ClientError> {
        time::timeout(timeout, async {
            let (stream, address) = self.inner.accept().await?;
            let incoming = demux_shared_incoming(stream).await?;
            Ok((incoming, address))
        })
        .await
        .map_err(|_| ClientError::TimedOut {
            operation: "shared plain/obfuscated peer initialization handshake",
        })?
    }

    /// Accept a connection from a TCP port shared by Soulseek and the mesh
    /// overlay. TLS ClientHello bytes are only peeked, never consumed, before
    /// the stream is handed to the gateway.
    pub async fn accept_shared_mesh(
        &self,
    ) -> Result<(SharedIncomingConnection<TcpStream>, SocketAddr), ClientError> {
        self.accept_shared_mesh_with_timeout(DEFAULT_INIT_HANDSHAKE_TIMEOUT)
            .await
    }

    pub async fn accept_shared_mesh_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<(SharedIncomingConnection<TcpStream>, SocketAddr), ClientError> {
        time::timeout(timeout, async {
            let (stream, address) = self.inner.accept().await?;
            let incoming = demux_shared_mesh_incoming(stream).await?;
            Ok((incoming, address))
        })
        .await
        .map_err(|_| ClientError::TimedOut {
            operation: "shared Soulseek/mesh TCP initialization handshake",
        })?
    }

    #[must_use]
    pub fn into_inner(self) -> TcpListener {
        self.inner
    }
}

pub async fn demux_obfuscated_incoming<S>(
    mut stream: S,
) -> Result<IncomingConnection<S>, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    match decode_obfuscated_init_message(read_obfuscated_init_frame(&mut stream).await?)? {
        InitMessage::PeerInit {
            username,
            connection_type,
            token,
        } => {
            let username = normalize_peer_username(&username)?.to_owned();
            let kind = ConnectionKind::try_from_connection_type(&connection_type)?;
            if kind == ConnectionKind::PeerMessages {
                Ok(IncomingConnection::ObfuscatedPeerMessages(
                    ObfuscatedPeerMessageConnection::with_peer_username(stream, Some(username)),
                ))
            } else {
                Ok(IncomingConnection::PeerInit {
                    username,
                    kind,
                    token,
                    stream,
                    obfuscated: true,
                })
            }
        }
        InitMessage::PierceFirewall { token } => {
            Ok(IncomingConnection::PierceFirewall { token, stream })
        }
        InitMessage::Unknown { code, payload } => Ok(IncomingConnection::UnknownInit {
            code,
            payload,
            stream,
        }),
    }
}

/// Demultiplex a shared Soulseek endpoint carrying framed initialization.
///
/// The shared endpoint has an explicit framed contract: plain `InitFrame` and
/// type-1 obfuscated `InitFrame` traffic are accepted here, while the legacy
/// one-byte `P`/`F`/`D` connection-kind contract remains on
/// the dedicated [`demux_incoming`]/[`Listener::accept`] path. Keeping the raw
/// contract separate is necessary because those bytes are also valid first
/// bytes of a frame length or an obfuscation key; no byte-only parser can
/// distinguish those streams without guessing. A prefix advertising two known
/// init forms is rejected before either interpretation is returned.
pub async fn demux_shared_incoming<S>(mut stream: S) -> Result<IncomingConnection<S>, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buffered = Vec::with_capacity(8);
    read_shared_bytes(&mut stream, &mut buffered, 4).await?;

    read_shared_bytes(&mut stream, &mut buffered, 8).await?;
    let candidates = shared_frame_candidates(&buffered)?;
    if candidates.len() == 2 {
        read_shared_bytes(&mut stream, &mut buffered, 9).await?;
        if candidates
            .iter()
            .all(|candidate| shared_candidate_has_known_header(&buffered, *candidate))
        {
            return Err(ClientError::AmbiguousInitFrame);
        }
    }
    let mut unknown = None;
    let mut last_error = None;

    // Prefer the shortest valid candidate after rejecting dual-known headers.
    for candidate in candidates {
        if let Err(error) =
            validate_shared_candidate_header(&mut stream, &mut buffered, candidate).await
        {
            last_error = Some(error);
            continue;
        }
        read_shared_bytes(&mut stream, &mut buffered, candidate.encoded_len).await?;
        let encoded = &buffered[..candidate.encoded_len];
        match decode_shared_candidate(encoded, candidate.obfuscated) {
            Ok(message @ (InitMessage::PeerInit { .. } | InitMessage::PierceFirewall { .. })) => {
                if let Err(error) = validate_shared_init_message(&message) {
                    last_error = Some(error);
                    continue;
                }
                return incoming_from_init_message(message, stream, candidate.obfuscated);
            }
            Ok(message @ InitMessage::Unknown { .. }) => {
                unknown.get_or_insert((candidate, message));
            }
            Err(error) => last_error = Some(error),
        }
    }

    let Some((candidate, message)) = unknown else {
        return Err(last_error.unwrap_or(ClientError::ConnectionClosed));
    };
    // An alternate interpretation may have consumed bytes beyond this frame.
    // Returning the shorter unknown frame would silently truncate its stream.
    if buffered.len() != candidate.encoded_len {
        return Err(ClientError::AmbiguousInitFrame);
    }
    incoming_from_init_message(message, stream, candidate.obfuscated)
}

fn shared_candidate_has_known_header(encoded: &[u8], candidate: SharedFrameCandidate) -> bool {
    let (code, prefix_len) = if candidate.obfuscated {
        let Ok(decoded) = slskr_protocol::decode_rotated(&encoded[..9]) else {
            return false;
        };
        (decoded[4], 8)
    } else {
        (encoded[4], 4)
    };
    let length = candidate.encoded_len - prefix_len;
    match InitCode::try_from(code) {
        Ok(InitCode::PierceFirewall) => length == PIERCE_FIREWALL_FRAME_LEN,
        Ok(InitCode::PeerInit) => (13..=MAX_PEER_INIT_FRAME_LEN).contains(&length),
        Err(_) => false,
    }
}

async fn validate_shared_candidate_header<S>(
    stream: &mut S,
    buffered: &mut Vec<u8>,
    candidate: SharedFrameCandidate,
) -> Result<(), ClientError>
where
    S: AsyncRead + Unpin,
{
    let (prefix_len, code) = if candidate.obfuscated {
        // The first eight obfuscated bytes contain the key and decoded length;
        // nine bytes are enough to expose the decoded init code without
        // allocating the advertised frame body.
        read_shared_bytes(stream, buffered, 9).await?;
        let decoded = slskr_protocol::decode_rotated(&buffered[..9])?;
        (8, decoded[4])
    } else {
        (4, buffered[4])
    };
    let length = candidate
        .encoded_len
        .checked_sub(prefix_len)
        .expect("shared candidate includes its frame prefix");
    let max = match InitCode::try_from(code) {
        Ok(InitCode::PierceFirewall) => PIERCE_FIREWALL_FRAME_LEN,
        Ok(InitCode::PeerInit) => MAX_PEER_INIT_FRAME_LEN,
        // Unknown extension init frames share the known initialization bound.
        // Large peer payloads belong after a successful initialization.
        Err(_) => MAX_PEER_INIT_FRAME_LEN,
    };
    if length > max {
        return Err(ClientError::FrameTooLarge { length, max });
    }
    Ok(())
}

fn validate_shared_init_message(message: &InitMessage) -> Result<(), ClientError> {
    if let InitMessage::PeerInit {
        username,
        connection_type,
        ..
    } = message
    {
        normalize_peer_username(username)?;
        ConnectionKind::try_from_connection_type(connection_type)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct SharedFrameCandidate {
    encoded_len: usize,
    obfuscated: bool,
}

fn shared_frame_candidates(first_block: &[u8]) -> Result<Vec<SharedFrameCandidate>, ClientError> {
    let plain_length =
        u32::from_le_bytes(first_block[..4].try_into().expect("four-byte prefix")) as usize;
    let obfuscated_prefix = slskr_protocol::decode_rotated(first_block)?;
    let obfuscated_length =
        u32::from_le_bytes(obfuscated_prefix[..4].try_into().expect("four-byte prefix")) as usize;

    let mut candidates = Vec::with_capacity(2);
    if (1..=crate::io::DEFAULT_MAX_FRAME_LEN).contains(&plain_length) {
        candidates.push(SharedFrameCandidate {
            encoded_len: 4 + plain_length,
            obfuscated: false,
        });
    }
    if (1..=crate::io::DEFAULT_MAX_FRAME_LEN).contains(&obfuscated_length) {
        candidates.push(SharedFrameCandidate {
            encoded_len: 8 + obfuscated_length,
            obfuscated: true,
        });
    }
    candidates.sort_by_key(|candidate| candidate.encoded_len);
    if candidates.is_empty() {
        return Err(ClientError::FrameTooLarge {
            length: plain_length.max(obfuscated_length),
            max: crate::io::DEFAULT_MAX_FRAME_LEN,
        });
    }
    Ok(candidates)
}

fn decode_shared_candidate(encoded: &[u8], obfuscated: bool) -> Result<InitMessage, ClientError> {
    let decoded = if obfuscated {
        slskr_protocol::decode_rotated(encoded)?
    } else {
        encoded.to_vec()
    };
    let frame = InitFrame::decode(&decoded)?;
    if obfuscated {
        decode_obfuscated_init_message(frame)
    } else {
        Ok(InitMessage::decode(frame)?)
    }
}

async fn read_shared_bytes<S>(
    stream: &mut S,
    buffered: &mut Vec<u8>,
    length: usize,
) -> Result<(), ClientError>
where
    S: AsyncRead + Unpin,
{
    if buffered.len() < length {
        let start = buffered.len();
        buffered.resize(length, 0);
        stream.read_exact(&mut buffered[start..]).await?;
    }
    Ok(())
}

const SHARED_MESH_CLASSIFICATION_ATTEMPTS: usize = 5;
const SHARED_MESH_CLASSIFICATION_RETRY_DELAY: Duration = Duration::from_millis(50);
const TLS_HANDSHAKE_CONTENT_TYPE: u8 = 0x16;
const TLS_MAJOR_VERSION: u8 = 0x03;
const TLS_CLIENT_HELLO_TYPE: u8 = 0x01;
const TLS_CLIENT_HELLO_PREFIX_LEN: usize = 6;
const MAX_TLS_RECORD_LEN: usize = 16 * 1024 + 2048;

/// Classify a stream for the current upstream shared TCP endpoint.
///
/// A mesh overlay connection starts with a bounded TLS ClientHello record.
/// A two-byte `0x16, 0x03` prefix also occurs in valid Soulseek frame lengths,
/// so classification waits for the full TLS record and handshake prefix.
/// The peek does not consume bytes from either protocol.
pub async fn demux_shared_mesh_incoming(
    stream: TcpStream,
) -> Result<SharedIncomingConnection<TcpStream>, ClientError> {
    let mut prefix = [0_u8; TLS_CLIENT_HELLO_PREFIX_LEN];
    for attempt in 0..SHARED_MESH_CLASSIFICATION_ATTEMPTS {
        let peeked = stream.peek(&mut prefix).await?;
        if peeked == 0 {
            break;
        }
        if prefix[0] != TLS_HANDSHAKE_CONTENT_TYPE
            || (peeked >= 2 && prefix[1] != TLS_MAJOR_VERSION)
            || (peeked >= 3 && !(1..=4).contains(&prefix[2]))
        {
            break;
        }

        if peeked >= prefix.len() {
            let record_len = u16::from_be_bytes([prefix[3], prefix[4]]) as usize;
            if (4..=MAX_TLS_RECORD_LEN).contains(&record_len) && prefix[5] == TLS_CLIENT_HELLO_TYPE
            {
                return Ok(SharedIncomingConnection::MeshOverlay(stream));
            }
            break;
        }
        if attempt + 1 < SHARED_MESH_CLASSIFICATION_ATTEMPTS {
            time::sleep(SHARED_MESH_CLASSIFICATION_RETRY_DELAY).await;
        } else {
            return Err(ClientError::TimedOut {
                operation: "shared Soulseek/mesh TCP classification",
            });
        }
    }

    Ok(SharedIncomingConnection::Soulseek(
        demux_shared_incoming(stream).await?,
    ))
}

fn incoming_from_init_message<S>(
    message: InitMessage,
    stream: S,
    obfuscated: bool,
) -> Result<IncomingConnection<S>, ClientError> {
    match message {
        InitMessage::PeerInit {
            username,
            connection_type,
            token,
        } => {
            let username = normalize_peer_username(&username)?.to_owned();
            let kind = ConnectionKind::try_from_connection_type(&connection_type)?;
            if obfuscated && kind == ConnectionKind::PeerMessages {
                Ok(IncomingConnection::ObfuscatedPeerMessages(
                    ObfuscatedPeerMessageConnection::with_peer_username(stream, Some(username)),
                ))
            } else {
                Ok(IncomingConnection::PeerInit {
                    username,
                    kind,
                    token,
                    stream,
                    obfuscated,
                })
            }
        }
        InitMessage::PierceFirewall { token } => {
            Ok(IncomingConnection::PierceFirewall { token, stream })
        }
        InitMessage::Unknown { code, payload } => Ok(IncomingConnection::UnknownInit {
            code,
            payload,
            stream,
        }),
    }
}

const MAX_NESTED_OBFUSCATED_INIT_FRAME_LEN: usize = 1024;

fn decode_obfuscated_init_message(frame: InitFrame) -> Result<InitMessage, ClientError> {
    match InitMessage::decode(frame)? {
        known @ (InitMessage::PeerInit { .. } | InitMessage::PierceFirewall { .. }) => Ok(known),
        InitMessage::Unknown { code, payload } => {
            // slskdN's obfuscated transfer writer wraps the already framed
            // PeerInit in one additional type-1 transfer frame.  The target
            // listener expects the unwrapped form, while its transfer reader
            // expects subsequent tokens and data to remain transfer-framed.
            // Reconstruct only small candidate init frames; never duplicate a
            // large arbitrary payload while probing this compatibility path.
            let nested_len = payload.len().saturating_add(1);
            if nested_len > MAX_NESTED_OBFUSCATED_INIT_FRAME_LEN {
                return Ok(InitMessage::Unknown { code, payload });
            }

            let mut nested = Vec::with_capacity(nested_len);
            nested.push(code);
            nested.extend_from_slice(&payload);
            let Ok(nested_frame) = InitFrame::decode(&nested) else {
                return Ok(InitMessage::Unknown { code, payload });
            };
            match InitMessage::decode(nested_frame) {
                Ok(known @ (InitMessage::PeerInit { .. } | InitMessage::PierceFirewall { .. })) => {
                    Ok(known)
                }
                Ok(_) | Err(_) => Ok(InitMessage::Unknown { code, payload }),
            }
        }
    }
}

pub async fn demux_incoming<S>(mut stream: S) -> Result<IncomingConnection<S>, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let first = stream.read_u8().await?;
    match ConnectionKind::try_from(first) {
        Ok(ConnectionKind::PeerMessages) => {
            return Ok(IncomingConnection::PeerMessages(
                PeerMessageConnection::new(stream),
            ));
        }
        Ok(ConnectionKind::FileTransfer) => {
            return Ok(IncomingConnection::FileTransfer(
                FileTransferConnection::new(stream),
            ));
        }
        Ok(ConnectionKind::Distributed) => {
            return Ok(IncomingConnection::Distributed(DistributedConnection::new(
                stream,
            )));
        }
        Err(ClientError::UnknownConnectionKind(_)) => {}
        Err(error) => return Err(error),
    }

    let frame = read_init_frame_with_first_len_byte(&mut stream, first).await?;
    match InitMessage::decode(frame)? {
        InitMessage::PeerInit {
            username,
            connection_type,
            token,
        } => {
            let username = normalize_peer_username(&username)?.to_owned();
            Ok(IncomingConnection::PeerInit {
                username,
                kind: ConnectionKind::try_from_connection_type(&connection_type)?,
                token,
                stream,
                obfuscated: false,
            })
        }
        InitMessage::PierceFirewall { token } => {
            Ok(IncomingConnection::PierceFirewall { token, stream })
        }
        InitMessage::Unknown { code, payload } => Ok(IncomingConnection::UnknownInit {
            code,
            payload,
            stream,
        }),
    }
}
