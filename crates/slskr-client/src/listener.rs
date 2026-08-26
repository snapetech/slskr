use slskr_protocol::{frame::InitFrame, init::InitMessage};
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

pub async fn demux_shared_incoming<S>(mut stream: S) -> Result<IncomingConnection<S>, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // Raw direct connections begin with a one-byte connection kind. Init
    // frames begin with a four-byte length prefix, while type-1 obfuscation
    // begins with a four-byte key. Preserve the raw form before consuming the
    // prefix used to distinguish the two framed forms.
    let first = stream.read_u8().await?;
    if let Ok(kind) = ConnectionKind::try_from(first) {
        return Ok(match kind {
            ConnectionKind::PeerMessages => {
                IncomingConnection::PeerMessages(PeerMessageConnection::new(stream))
            }
            ConnectionKind::FileTransfer => {
                IncomingConnection::FileTransfer(FileTransferConnection::new(stream))
            }
            ConnectionKind::Distributed => {
                IncomingConnection::Distributed(DistributedConnection::new(stream))
            }
        });
    }

    let mut prefix = [0_u8; 4];
    prefix[0] = first;
    stream.read_exact(&mut prefix[1..]).await?;
    let candidate_length = u32::from_le_bytes(prefix) as usize;
    if (1..=crate::io::DEFAULT_MAX_FRAME_LEN).contains(&candidate_length) {
        let mut encoded = Vec::with_capacity(4 + candidate_length);
        encoded.extend_from_slice(&prefix);
        encoded.resize(4 + candidate_length, 0);
        stream.read_exact(&mut encoded[4..]).await?;
        let frame = InitFrame::decode(&encoded)?;
        return incoming_from_init_message(InitMessage::decode(frame)?, stream, false);
    }

    // The first four bytes are the obfuscation key. The next four bytes are
    // the rotated init-frame length prefix. Decode the header before
    // allocating or reading the remainder so malformed peers cannot request
    // an unbounded buffer.
    let mut first_block = [0_u8; 8];
    first_block[..4].copy_from_slice(&prefix);
    stream.read_exact(&mut first_block[4..]).await?;
    let decoded_first_block = slskr_protocol::decode_rotated(&first_block)?;
    let length = u32::from_le_bytes([
        decoded_first_block[0],
        decoded_first_block[1],
        decoded_first_block[2],
        decoded_first_block[3],
    ]) as usize;
    if !(1..=crate::io::DEFAULT_MAX_FRAME_LEN).contains(&length) {
        return Err(ClientError::FrameTooLarge {
            length,
            max: crate::io::DEFAULT_MAX_FRAME_LEN,
        });
    }
    let mut obfuscated = Vec::with_capacity(8 + length);
    obfuscated.extend_from_slice(&first_block);
    obfuscated.resize(8 + length, 0);
    stream.read_exact(&mut obfuscated[8..]).await?;
    let decoded = slskr_protocol::decode_rotated(&obfuscated)?;
    let frame = InitFrame::decode(&decoded)?;
    let message = decode_obfuscated_init_message(frame)?;
    incoming_from_init_message(message, stream, true)
}

const SHARED_MESH_CLASSIFICATION_ATTEMPTS: usize = 5;
const SHARED_MESH_CLASSIFICATION_RETRY_DELAY: Duration = Duration::from_millis(50);
const TLS_HANDSHAKE_CONTENT_TYPE: u8 = 0x16;
const TLS_MAJOR_VERSION: u8 = 0x03;

/// Classify a stream for the current upstream shared TCP endpoint.
///
/// A mesh overlay connection starts with a TLS record header (`0x16, 0x03`).
/// Soulseek plain and obfuscated traffic starts with different bytes and is
/// passed to the existing bounded Soulseek demux. The peek is intentionally
/// conservative: a partial TLS header is rejected after the bounded retry
/// window, while non-TLS traffic is never guessed as mesh traffic.
pub async fn demux_shared_mesh_incoming(
    stream: TcpStream,
) -> Result<SharedIncomingConnection<TcpStream>, ClientError> {
    let mut prefix = [0_u8; 2];
    for attempt in 0..SHARED_MESH_CLASSIFICATION_ATTEMPTS {
        let peeked = stream.peek(&mut prefix).await?;
        if peeked >= prefix.len() {
            if prefix[0] == TLS_HANDSHAKE_CONTENT_TYPE && prefix[1] == TLS_MAJOR_VERSION {
                return Ok(SharedIncomingConnection::MeshOverlay(stream));
            }
            break;
        }
        if peeked == 0 {
            break;
        }
        // Soulseek's tagged connection kinds are one byte long. Only a
        // leading TLS content byte is ambiguous and needs a bounded wait for
        // the version byte.
        if prefix[0] != TLS_HANDSHAKE_CONTENT_TYPE {
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
            ))
        }
        Ok(ConnectionKind::FileTransfer) => {
            return Ok(IncomingConnection::FileTransfer(
                FileTransferConnection::new(stream),
            ))
        }
        Ok(ConnectionKind::Distributed) => {
            return Ok(IncomingConnection::Distributed(DistributedConnection::new(
                stream,
            )))
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
