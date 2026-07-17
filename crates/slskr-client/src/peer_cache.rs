use std::{collections::HashMap, sync::Arc};

use slskr_protocol::peer::PeerMessage;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;

use crate::{stream::PeerMessageConnection, ClientError};

pub const DEFAULT_MAX_PEER_CONNECTIONS: usize = 1_024;
pub const MAX_PEER_USERNAME_BYTES: usize = 4_096;

type SharedPeerConnection<S> = Arc<Mutex<Option<PeerMessageConnection<S>>>>;

#[derive(Debug)]
pub struct PeerConnectionCache<S> {
    connections: Arc<Mutex<HashMap<String, SharedPeerConnection<S>>>>,
    max_connections: usize,
}

impl<S> Clone for PeerConnectionCache<S> {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            max_connections: self.max_connections,
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
        Self::with_max_connections(DEFAULT_MAX_PEER_CONNECTIONS)
    }

    #[must_use]
    pub fn with_max_connections(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            max_connections: max_connections.max(1),
        }
    }

    pub async fn insert(
        &self,
        username: impl Into<String>,
        connection: PeerMessageConnection<S>,
    ) -> Result<Option<PeerMessageConnection<S>>, ClientError> {
        let username = username.into();
        let username = normalize_peer_username(&username)?;
        let username = username_key(username);
        let mut connections = self.connections.lock().await;
        if connections.len() >= self.max_connections && !connections.contains_key(&username) {
            return Err(ClientError::PeerConnectionCacheFull {
                max: self.max_connections,
            });
        }
        let replaced = connections.insert(username, Arc::new(Mutex::new(Some(connection))));
        drop(connections);
        match replaced {
            Some(replaced) => Ok(replaced.lock().await.take()),
            None => Ok(None),
        }
    }

    pub async fn remove(&self, username: &str) -> Option<PeerMessageConnection<S>> {
        let removed = self
            .connections
            .lock()
            .await
            .remove(&username_key(username));
        match removed {
            Some(removed) => removed.lock().await.take(),
            None => None,
        }
    }

    pub async fn contains(&self, username: &str) -> bool {
        self.connections
            .lock()
            .await
            .contains_key(&username_key(username))
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
        let key = username_key(username);
        let Some(connection) = self.connections.lock().await.get(&key).cloned() else {
            return Ok(false);
        };

        let mut connection_guard = connection.lock().await;
        let Some(active) = connection_guard.as_mut() else {
            return Ok(false);
        };
        if let Err(error) = active.send(message).await {
            *connection_guard = None;
            drop(connection_guard);
            self.remove_if_current(&key, &connection).await;
            return Err(error);
        }
        Ok(true)
    }

    pub async fn receive_from(&self, username: &str) -> Result<Option<PeerMessage>, ClientError> {
        let key = username_key(username);
        let Some(connection) = self.connections.lock().await.get(&key).cloned() else {
            return Ok(None);
        };

        let mut connection_guard = connection.lock().await;
        let Some(active) = connection_guard.as_mut() else {
            return Ok(None);
        };
        match active.receive().await {
            Ok(message) => Ok(Some(message)),
            Err(error) => {
                *connection_guard = None;
                drop(connection_guard);
                self.remove_if_current(&key, &connection).await;
                Err(error)
            }
        }
    }

    async fn remove_if_current(&self, key: &str, connection: &SharedPeerConnection<S>) {
        let mut connections = self.connections.lock().await;
        if connections
            .get(key)
            .is_some_and(|current| Arc::ptr_eq(current, connection))
        {
            connections.remove(key);
        }
    }
}

fn username_key(username: &str) -> String {
    username.trim().to_ascii_lowercase()
}

pub(crate) fn normalize_peer_username(username: &str) -> Result<&str, ClientError> {
    let username = username.trim();
    if username.is_empty() {
        Err(ClientError::BlankPeerUsername)
    } else if username.len() > MAX_PEER_USERNAME_BYTES {
        Err(ClientError::PeerUsernameTooLong {
            length: username.len(),
            max: MAX_PEER_USERNAME_BYTES,
        })
    } else {
        Ok(username)
    }
}
