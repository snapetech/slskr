use std::{
    collections::hash_map::DefaultHasher,
    future::Future,
    hash::{Hash, Hasher},
    pin::Pin,
    sync::Arc,
};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

use crate::{
    connection::ConnectionKind,
    file_transfer::FileTransferConnection,
    listener::IncomingConnection,
    peer_cache::PeerConnectionCache,
    peer_connect::IndirectPeerRequest,
    server::ServerSession,
    stream::{DistributedConnection, PeerMessageConnection},
    ClientError,
};

type ConnectFuture<S> =
    Pin<Box<dyn Future<Output = Result<PeerMessageConnection<S>, ClientError>> + Send>>;

pub type PeerConnector<S> = Arc<dyn Fn(String) -> ConnectFuture<S> + Send + Sync>;

const PEER_CONNECT_STRIPES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenGenerator {
    next: u32,
}

impl TokenGenerator {
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self { next: seed }
    }

    #[must_use]
    pub fn next_token(&mut self) -> u32 {
        let token = self.next;
        self.next = self.next.wrapping_add(1);
        token
    }
}

impl Default for TokenGenerator {
    fn default() -> Self {
        Self::new(1)
    }
}

pub struct ConnectionManager<ServerStream, PeerStream> {
    server: Arc<Mutex<ServerSession<ServerStream>>>,
    peer_cache: PeerConnectionCache<PeerStream>,
    connector: PeerConnector<PeerStream>,
    tokens: Arc<Mutex<TokenGenerator>>,
    peer_connects: Arc<[Mutex<()>; PEER_CONNECT_STRIPES]>,
}

impl<ServerStream, PeerStream> ConnectionManager<ServerStream, PeerStream>
where
    ServerStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    PeerStream: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    #[must_use]
    pub fn new(
        server: ServerSession<ServerStream>,
        peer_cache: PeerConnectionCache<PeerStream>,
        connector: PeerConnector<PeerStream>,
    ) -> Self {
        Self {
            server: Arc::new(Mutex::new(server)),
            peer_cache,
            connector,
            tokens: Arc::new(Mutex::new(TokenGenerator::default())),
            peer_connects: Arc::new(std::array::from_fn(|_| Mutex::new(()))),
        }
    }

    #[must_use]
    pub fn with_token_seed(mut self, seed: u32) -> Self {
        self.tokens = Arc::new(Mutex::new(TokenGenerator::new(seed)));
        self
    }

    #[must_use]
    pub fn peer_cache(&self) -> PeerConnectionCache<PeerStream> {
        self.peer_cache.clone()
    }

    pub async fn ensure_peer_messages(&self, username: &str) -> Result<bool, ClientError> {
        if self.peer_cache.contains(username).await {
            return Ok(false);
        }

        let _gate = self.peer_connects[peer_connect_stripe(username)]
            .lock()
            .await;
        if self.peer_cache.contains(username).await {
            return Ok(false);
        }
        let connection = (self.connector)(username.to_owned()).await?;
        self.peer_cache
            .insert(username.to_owned(), connection)
            .await?;
        Ok(true)
    }

    pub async fn request_indirect_peer_messages(
        &self,
        username: &str,
    ) -> Result<IndirectPeerRequest, ClientError> {
        self.request_indirect(username, ConnectionKind::PeerMessages)
            .await
    }

    pub async fn request_indirect_distributed(
        &self,
        username: &str,
    ) -> Result<IndirectPeerRequest, ClientError> {
        self.request_indirect(username, ConnectionKind::Distributed)
            .await
    }

    pub async fn request_indirect_file_transfer(
        &self,
        username: &str,
    ) -> Result<IndirectPeerRequest, ClientError> {
        self.request_indirect(username, ConnectionKind::FileTransfer)
            .await
    }

    pub async fn request_indirect(
        &self,
        username: &str,
        kind: ConnectionKind,
    ) -> Result<IndirectPeerRequest, ClientError> {
        let token = self.tokens.lock().await.next_token();
        let request = IndirectPeerRequest::new(token, username.to_owned(), kind);
        self.server
            .lock()
            .await
            .send_server_message(request.server_message())
            .await?;
        Ok(request)
    }

    pub fn complete_inbound_distributed(
        &self,
        request: &IndirectPeerRequest,
        incoming: IncomingConnection<PeerStream>,
    ) -> Result<DistributedConnection<PeerStream>, ClientError> {
        let stream = request.complete(incoming)?;
        Ok(DistributedConnection::new(stream))
    }

    pub fn complete_inbound_file_transfer(
        &self,
        request: &IndirectPeerRequest,
        incoming: IncomingConnection<PeerStream>,
    ) -> Result<FileTransferConnection<PeerStream>, ClientError> {
        let stream = request.complete(incoming)?;
        Ok(FileTransferConnection::new(stream))
    }

    pub async fn complete_inbound_peer_messages(
        &self,
        request: &IndirectPeerRequest,
        incoming: IncomingConnection<PeerStream>,
    ) -> Result<(), ClientError> {
        let stream = request.complete(incoming)?;
        self.peer_cache
            .insert(request.username.clone(), PeerMessageConnection::new(stream))
            .await?;
        Ok(())
    }
}

fn peer_connect_stripe(username: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    username.to_ascii_lowercase().hash(&mut hasher);
    usize::try_from(hasher.finish() % PEER_CONNECT_STRIPES as u64).unwrap_or(0)
}
