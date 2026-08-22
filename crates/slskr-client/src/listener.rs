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
