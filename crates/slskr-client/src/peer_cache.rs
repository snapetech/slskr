use std::{collections::HashMap, sync::Arc};

use slskr_protocol::peer::PeerMessage;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

use crate::{stream::PeerMessageConnection, ClientError};

#[derive(Debug)]
pub struct PeerConnectionCache<S> {
    connections: Arc<Mutex<HashMap<String, PeerMessageConnection<S>>>>,
}

impl<S> Clone for PeerConnectionCache<S> {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
        }
    }
}

impl<S> Default for PeerConnectionCache<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> PeerConnectionCache<S> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn insert(
        &self,
        username: impl Into<String>,
        connection: PeerMessageConnection<S>,
    ) -> Option<PeerMessageConnection<S>> {
        self.connections
            .lock()
            .await
            .insert(username.into(), connection)
    }

    pub async fn remove(&self, username: &str) -> Option<PeerMessageConnection<S>> {
        self.connections.lock().await.remove(username)
    }

    pub async fn contains(&self, username: &str) -> bool {
        self.connections.lock().await.contains_key(username)
    }

    pub async fn len(&self) -> usize {
        self.connections.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

impl<S> PeerConnectionCache<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn send_to(
        &self,
        username: &str,
        message: &PeerMessage,
    ) -> Result<bool, ClientError> {
        let mut connections = self.connections.lock().await;
        let Some(connection) = connections.get_mut(username) else {
            return Ok(false);
        };

        connection.send(message).await?;
        Ok(true)
    }

    pub async fn receive_from(&self, username: &str) -> Result<Option<PeerMessage>, ClientError> {
        let mut connections = self.connections.lock().await;
        let Some(connection) = connections.get_mut(username) else {
            return Ok(None);
        };

        Ok(Some(connection.receive().await?))
    }
}
