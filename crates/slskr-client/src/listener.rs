use slskr_protocol::init::InitMessage;
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
        let (stream, address) = self.inner.accept().await?;
        let incoming = time::timeout(timeout, demux_incoming(stream))
            .await
            .map_err(|_| ClientError::TimedOut {
                operation: "peer initialization handshake",
            })??;
        Ok((incoming, address))
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
        let (stream, address) = self.inner.accept().await?;
        let incoming = time::timeout(timeout, demux_obfuscated_incoming(stream))
            .await
            .map_err(|_| ClientError::TimedOut {
                operation: "obfuscated peer initialization handshake",
            })??;
        Ok((incoming, address))
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
    match InitMessage::decode(read_obfuscated_init_frame(&mut stream).await?)? {
        InitMessage::PeerInit {
            username,
            connection_type,
            token,
        } => {
            let username = normalize_peer_username(&username)?.to_owned();
            let kind = ConnectionKind::try_from_connection_type(&connection_type)?;
            if kind == ConnectionKind::PeerMessages {
                Ok(IncomingConnection::ObfuscatedPeerMessages(
                    ObfuscatedPeerMessageConnection::new(stream),
                ))
            } else {
                Ok(IncomingConnection::PeerInit {
                    username,
                    kind,
                    token,
                    stream,
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
