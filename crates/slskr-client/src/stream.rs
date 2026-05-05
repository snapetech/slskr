use slskr_protocol::{
    distributed::DistributedMessage,
    init::InitMessage,
    peer::PeerMessage,
    server::{Direction, ServerMessage},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
    time::{self, Duration},
};

use crate::{
    io::{
        read_init_frame, read_message_frame, read_obfuscated_init_frame,
        read_obfuscated_message_frame, write_init_frame, write_message_frame,
        write_obfuscated_init_frame, write_obfuscated_message_frame,
    },
    ClientError,
};

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct ServerConnection<S> {
    stream: S,
}

impl ServerConnection<TcpStream> {
    pub async fn connect<A>(address: A) -> Result<Self, ClientError>
    where
        A: ToSocketAddrs,
    {
        Self::connect_with_timeout(address, DEFAULT_CONNECT_TIMEOUT).await
    }

    pub async fn connect_with_timeout<A>(address: A, timeout: Duration) -> Result<Self, ClientError>
    where
        A: ToSocketAddrs,
    {
        let stream = time::timeout(timeout, TcpStream::connect(address))
            .await
            .map_err(|_| ClientError::TimedOut {
                operation: "server connect",
            })??;
        Ok(Self::new(stream))
    }

    pub async fn readable(&self) -> Result<(), ClientError> {
        Ok(self.stream.readable().await?)
    }
}

impl<S> ServerConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> ServerConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send(&mut self, message: &ServerMessage) -> Result<(), ClientError> {
        write_message_frame(&mut self.stream, &message.encode()?).await
    }

    pub async fn receive(&mut self) -> Result<ServerMessage, ClientError> {
        self.receive_with_direction(Direction::ServerToClient).await
    }

    pub async fn receive_with_direction(
        &mut self,
        direction: Direction,
    ) -> Result<ServerMessage, ClientError> {
        let frame = read_message_frame(&mut self.stream).await?;
        Ok(ServerMessage::decode(frame, direction)?)
    }
}

#[derive(Debug)]
pub struct PeerMessageConnection<S> {
    stream: S,
}

impl<S> PeerMessageConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> PeerMessageConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send(&mut self, message: &PeerMessage) -> Result<(), ClientError> {
        write_message_frame(&mut self.stream, &message.encode()?).await
    }

    pub async fn receive(&mut self) -> Result<PeerMessage, ClientError> {
        let frame = read_message_frame(&mut self.stream).await?;
        Ok(PeerMessage::decode(frame)?)
    }
}

#[derive(Debug)]
pub struct ObfuscatedPeerMessageConnection<S> {
    stream: S,
}

impl<S> ObfuscatedPeerMessageConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> ObfuscatedPeerMessageConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send(&mut self, message: &PeerMessage) -> Result<(), ClientError> {
        write_obfuscated_message_frame(&mut self.stream, &message.encode()?).await
    }

    pub async fn receive(&mut self) -> Result<PeerMessage, ClientError> {
        let frame = read_obfuscated_message_frame(&mut self.stream).await?;
        Ok(PeerMessage::decode(frame)?)
    }
}

#[derive(Debug)]
pub struct DistributedConnection<S> {
    stream: S,
}

impl<S> DistributedConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> DistributedConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send(&mut self, message: &DistributedMessage) -> Result<(), ClientError> {
        write_init_frame(&mut self.stream, &message.encode()?).await
    }

    pub async fn receive(&mut self) -> Result<DistributedMessage, ClientError> {
        let frame = read_init_frame(&mut self.stream).await?;
        Ok(DistributedMessage::decode(frame)?)
    }
}

#[derive(Debug)]
pub struct InitConnection<S> {
    stream: S,
}

impl<S> InitConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> InitConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send(&mut self, message: &InitMessage) -> Result<(), ClientError> {
        write_init_frame(&mut self.stream, &message.encode()?).await
    }

    pub async fn receive(&mut self) -> Result<InitMessage, ClientError> {
        let frame = read_init_frame(&mut self.stream).await?;
        Ok(InitMessage::decode(frame)?)
    }
}

#[derive(Debug)]
pub struct ObfuscatedInitConnection<S> {
    stream: S,
}

impl<S> ObfuscatedInitConnection<S> {
    #[must_use]
    pub const fn new(stream: S) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S> ObfuscatedInitConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send(&mut self, message: &InitMessage) -> Result<(), ClientError> {
        write_obfuscated_init_frame(&mut self.stream, &message.encode()?).await
    }

    pub async fn receive(&mut self) -> Result<InitMessage, ClientError> {
        let frame = read_obfuscated_init_frame(&mut self.stream).await?;
        Ok(InitMessage::decode(frame)?)
    }
}
