use std::{collections::HashMap, sync::Arc};

use slskr_protocol::peer::PeerMessage;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::Mutex;
use tokio::time::{self, Duration};

use crate::{stream::PeerMessageConnection, ClientError};

pub const DEFAULT_MAX_PEER_CONNECTIONS: usize = 1_024;
pub const MAX_PEER_USERNAME_BYTES: usize = 4_096;
pub const DEFAULT_PEER_IO_TIMEOUT: Duration = Duration::from_secs(30);

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
        let key = lookup_username_key(username)?;
        let removed = self.connections.lock().await.remove(&key);
        match removed {
            Some(removed) => removed.lock().await.take(),
            None => None,
        }
    }

    pub async fn contains(&self, username: &str) -> bool {
        let Some(key) = lookup_username_key(username) else {
            return false;
        };
        let Some(connection) = self.connections.lock().await.get(&key).cloned() else {
            return false;
        };
        let is_active = connection.lock().await.is_some();
        if !is_active {
            let mut connections = self.connections.lock().await;
            if connections
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &connection))
            {
                connections.remove(&key);
            }
        }
        is_active
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
        self.send_to_with_timeout(username, message, DEFAULT_PEER_IO_TIMEOUT)
            .await
    }

    pub async fn send_to_with_timeout(
        &self,
        username: &str,
        message: &PeerMessage,
        timeout: Duration,
    ) -> Result<bool, ClientError> {
        let Some(key) = lookup_username_key(username) else {
            return Ok(false);
        };
        let Some(connection) = self.connections.lock().await.get(&key).cloned() else {
            return Ok(false);
        };

        let mut connection_guard = connection.lock().await;
        let Some(active) = connection_guard.as_mut() else {
            return Ok(false);
        };
        match time::timeout(timeout, active.send(message)).await {
            Ok(Ok(())) => Ok(true),
            Ok(Err(error)) => {
                *connection_guard = None;
                drop(connection_guard);
                self.remove_if_current(&key, &connection).await;
                Err(error)
            }
            Err(_) => {
                *connection_guard = None;
                drop(connection_guard);
                self.remove_if_current(&key, &connection).await;
                Err(ClientError::TimedOut {
                    operation: "cached peer send",
                })
            }
        }
    }

    pub async fn receive_from(&self, username: &str) -> Result<Option<PeerMessage>, ClientError> {
        self.receive_from_with_timeout(username, DEFAULT_PEER_IO_TIMEOUT)
            .await
    }

    pub async fn receive_from_with_timeout(
        &self,
        username: &str,
        timeout: Duration,
    ) -> Result<Option<PeerMessage>, ClientError> {
        let Some(key) = lookup_username_key(username) else {
            return Ok(None);
        };
        let Some(connection) = self.connections.lock().await.get(&key).cloned() else {
            return Ok(None);
        };

        let mut connection_guard = connection.lock().await;
        let Some(active) = connection_guard.as_mut() else {
            return Ok(None);
        };
        match time::timeout(timeout, active.receive()).await {
            Err(_) => {
                *connection_guard = None;
                drop(connection_guard);
                self.remove_if_current(&key, &connection).await;
                Err(ClientError::TimedOut {
                    operation: "cached peer receive",
                })
            }
            Ok(Ok(message)) => Ok(Some(message)),
            Ok(Err(error)) => {
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

fn lookup_username_key(username: &str) -> Option<String> {
    normalize_peer_username(username).ok().map(username_key)
}

pub(crate) fn normalize_peer_username(username: &str) -> Result<&str, ClientError> {
    let username = username.trim();
    if username.is_empty() {
        Err(ClientError::BlankPeerUsername)
    } else if username.chars().any(char::is_control) {
        Err(ClientError::InvalidPeerUsername)
    } else if username.len() > MAX_PEER_USERNAME_BYTES {
        Err(ClientError::PeerUsernameTooLong {
            length: username.len(),
            max: MAX_PEER_USERNAME_BYTES,
        })
    } else {
        Ok(username)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::PeerMessageConnection;
    use tokio::io::duplex;

    #[tokio::test]
    async fn contains_rejects_stale_connection_tombstones() {
        let cache = PeerConnectionCache::with_max_connections(1);
        let (stream, _) = duplex(64);
        cache
            .insert("peer", PeerMessageConnection::new(stream))
            .await
            .unwrap();

        let connection = cache.connections.lock().await.get("peer").cloned().unwrap();
        *connection.lock().await = None;

        assert!(!cache.contains("peer").await);
        assert!(cache.is_empty().await);

        let (replacement, _) = duplex(64);
        cache
            .insert("other", PeerMessageConnection::new(replacement))
            .await
            .unwrap();
        assert!(cache.contains("other").await);
    }
}
