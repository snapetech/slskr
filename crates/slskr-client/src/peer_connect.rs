use std::future::Future;

use slskr_protocol::init::InitMessage;
use slskr_protocol::server::{ConnectToPeerRequest, ServerMessage};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
    time::{self, Duration},
};

use crate::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    listener::IncomingConnection,
    peer_cache::normalize_peer_username,
    stream::{
        DistributedConnection, InitConnection, ObfuscatedInitConnection, PeerMessageConnection,
        DEFAULT_CONNECT_TIMEOUT,
    },
    ClientError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndirectPeerRequest {
    pub token: u32,
    pub username: String,
    pub kind: ConnectionKind,
}

impl IndirectPeerRequest {
    #[must_use]
    pub fn new(token: u32, username: impl Into<String>, kind: ConnectionKind) -> Self {
        Self {
            token,
            username: username.into(),
            kind,
        }
    }

    #[must_use]
    pub fn server_message(&self) -> ServerMessage {
        ServerMessage::ConnectToPeerRequest(ConnectToPeerRequest {
            token: self.token,
            username: self.username.clone(),
            connection_type: self.kind.as_str().to_owned(),
        })
    }

    pub fn complete<S>(&self, incoming: IncomingConnection<S>) -> Result<S, ClientError> {
        match incoming {
            IncomingConnection::PeerInit {
                username,
                kind,
                token,
                stream,
            } => {
                self.validate_token(token)?;
                self.validate_username(username)?;
                self.validate_kind(kind)?;
                Ok(stream)
            }
            IncomingConnection::PierceFirewall { token, stream } => {
                self.validate_token(token)?;
                Ok(stream)
            }
            IncomingConnection::PeerMessages(_)
            | IncomingConnection::ObfuscatedPeerMessages(_)
            | IncomingConnection::Distributed(_)
            | IncomingConnection::FileTransfer(_) => Err(ClientError::IndirectInitRequired),
            IncomingConnection::UnknownInit { code, payload, .. } => {
                Err(ClientError::UnexpectedInitMessage {
                    code,
                    payload_len: payload.len(),
                    payload,
                })
            }
        }
    }

    fn validate_token(&self, received: u32) -> Result<(), ClientError> {
        if received == self.token {
            Ok(())
        } else {
            Err(ClientError::IndirectTokenMismatch {
                expected: self.token,
                received,
            })
        }
    }

    fn validate_username(&self, received: String) -> Result<(), ClientError> {
        if received.eq_ignore_ascii_case(&self.username) {
            Ok(())
        } else {
            Err(ClientError::IndirectUsernameMismatch {
                expected: self.username.clone(),
                received,
            })
        }
    }

    fn validate_kind(&self, received: ConnectionKind) -> Result<(), ClientError> {
        if received == self.kind {
            Ok(())
        } else {
            Err(ClientError::IndirectKindMismatch {
                expected: self.kind,
                received,
            })
        }
    }
}

pub async fn send_peer_init<S>(
    stream: S,
    username: impl Into<String>,
    kind: ConnectionKind,
) -> Result<S, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    send_peer_init_with_token(stream, username, kind, 0).await
}

pub async fn send_peer_init_with_token<S>(
    stream: S,
    username: impl Into<String>,
    kind: ConnectionKind,
    token: u32,
) -> Result<S, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let username = username.into();
    let username = normalize_peer_username(&username)?.to_owned();
    let mut connection = InitConnection::new(stream);
    connection
        .send(&InitMessage::PeerInit {
            username,
            connection_type: kind.as_str().to_owned(),
            token,
        })
        .await?;
    Ok(connection.into_inner())
}

pub async fn send_obfuscated_peer_init<S>(
    stream: S,
    username: impl Into<String>,
    kind: ConnectionKind,
) -> Result<S, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    send_obfuscated_peer_init_with_token(stream, username, kind, 0).await
}

pub async fn send_obfuscated_peer_init_with_token<S>(
    stream: S,
    username: impl Into<String>,
    kind: ConnectionKind,
    token: u32,
) -> Result<S, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let username = username.into();
    let username = normalize_peer_username(&username)?.to_owned();
    let mut connection = ObfuscatedInitConnection::new(stream);
    connection
        .send(&InitMessage::PeerInit {
            username,
            connection_type: kind.as_str().to_owned(),
            token,
        })
        .await?;
    Ok(connection.into_inner())
}

pub async fn send_pierce_firewall<S>(stream: S, token: u32) -> Result<S, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut connection = InitConnection::new(stream);
    connection
        .send(&InitMessage::PierceFirewall { token })
        .await?;
    Ok(connection.into_inner())
}

pub async fn send_obfuscated_pierce_firewall<S>(stream: S, token: u32) -> Result<S, ClientError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut connection = ObfuscatedInitConnection::new(stream);
    connection
        .send(&InitMessage::PierceFirewall { token })
        .await?;
    Ok(connection.into_inner())
}

pub async fn connect_peer_messages<A>(
    address: A,
    username: impl Into<String>,
) -> Result<PeerMessageConnection<TcpStream>, ClientError>
where
    A: ToSocketAddrs,
{
    connect_peer_messages_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
}

pub async fn connect_peer_messages_with_timeout<A>(
    address: A,
    username: impl Into<String>,
    timeout: Duration,
) -> Result<PeerMessageConnection<TcpStream>, ClientError>
where
    A: ToSocketAddrs,
{
    let username = username.into();
    let username = normalize_peer_username(&username)?.to_owned();
    let stream = peer_connect_deadline(timeout, "peer-message connect", async {
        let stream = TcpStream::connect(address).await?;
        send_peer_init(stream, username, ConnectionKind::PeerMessages).await
    })
    .await?;
    Ok(PeerMessageConnection::new(stream))
}

pub async fn connect_distributed<A>(
    address: A,
    username: impl Into<String>,
) -> Result<DistributedConnection<TcpStream>, ClientError>
where
    A: ToSocketAddrs,
{
    connect_distributed_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
}

pub async fn connect_distributed_with_timeout<A>(
    address: A,
    username: impl Into<String>,
    timeout: Duration,
) -> Result<DistributedConnection<TcpStream>, ClientError>
where
    A: ToSocketAddrs,
{
    let username = username.into();
    let username = normalize_peer_username(&username)?.to_owned();
    let stream = peer_connect_deadline(timeout, "distributed connect", async {
        let stream = TcpStream::connect(address).await?;
        send_peer_init(stream, username, ConnectionKind::Distributed).await
    })
    .await?;
    Ok(DistributedConnection::new(stream))
}

pub async fn connect_file_transfer<A>(
    address: A,
    username: impl Into<String>,
) -> Result<FileTransferConnection<TcpStream>, ClientError>
where
    A: ToSocketAddrs,
{
    connect_file_transfer_with_timeout(address, username, DEFAULT_CONNECT_TIMEOUT).await
}

pub async fn connect_file_transfer_with_timeout<A>(
    address: A,
    username: impl Into<String>,
    timeout: Duration,
) -> Result<FileTransferConnection<TcpStream>, ClientError>
where
    A: ToSocketAddrs,
{
    let username = username.into();
    let username = normalize_peer_username(&username)?.to_owned();
    let stream = peer_connect_deadline(timeout, "file-transfer connect", async {
        let stream = TcpStream::connect(address).await?;
        send_peer_init(stream, username, ConnectionKind::FileTransfer).await
    })
    .await?;
    Ok(FileTransferConnection::new(stream))
}

async fn peer_connect_deadline<T, F>(
    timeout: Duration,
    operation: &'static str,
    future: F,
) -> Result<T, ClientError>
where
    F: Future<Output = Result<T, ClientError>>,
{
    time::timeout(timeout, future)
        .await
        .map_err(|_| ClientError::TimedOut { operation })?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peer_cache::MAX_PEER_USERNAME_BYTES;
    use tokio::io::duplex;

    #[tokio::test]
    async fn peer_connect_deadline_covers_initialization_write() {
        let (client, _non_reading_peer) = duplex(1);
        let username = "x".repeat(MAX_PEER_USERNAME_BYTES);

        assert!(matches!(
            peer_connect_deadline(
                Duration::from_millis(10),
                "peer-message connect",
                send_peer_init(client, username, ConnectionKind::PeerMessages),
            )
            .await,
            Err(ClientError::TimedOut {
                operation: "peer-message connect",
            })
        ));
    }
}
