//! Reserved cleanup capacity prevents cancellation from abandoning relay state.
use crate::AppState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::mpsc;

const MAX_PENDING_CLEANUPS: usize = 256;

#[derive(Debug, Default)]
pub(crate) struct ConnectionCleanup {
    sender: Mutex<Option<mpsc::Sender<String>>>,
    closed: AtomicBool,
}

impl ConnectionCleanup {
    pub(crate) async fn reserve(
        &self,
        state: &Arc<AppState>,
        connection_id: String,
    ) -> Result<ConnectionLease, String> {
        let sender = {
            let mut sender = self
                .sender
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if self.closed.load(Ordering::Acquire) {
                return Err("relay cleanup admission is closed".to_owned());
            }
            if sender.is_none() {
                let (created, mut receiver) = mpsc::channel::<String>(MAX_PENDING_CLEANUPS);
                let task_state = Arc::downgrade(state);
                if !state.managed_background_tasks.try_spawn(async move {
                    while let Some(id) = receiver.recv().await {
                        let Some(state) = task_state.upgrade() else {
                            break;
                        };
                        state
                            .relay
                            .write()
                            .await
                            .protocol
                            .deregister_connection(&id);
                    }
                }) {
                    return Err("daemon task admission is closed".to_owned());
                }
                *sender = Some(created);
            }
            sender.as_ref().expect("cleanup sender initialized").clone()
        };
        let permit = sender
            .reserve_owned()
            .await
            .map_err(|_| "relay cleanup admission is closed".to_owned())?;
        // Shutdown can begin while capacity is being reserved.
        if self.closed.load(Ordering::Acquire) {
            return Err("relay cleanup admission is closed".to_owned());
        }
        Ok(ConnectionLease {
            connection_id,
            permit: Some(permit),
        })
    }

    pub(crate) fn close(&self) {
        let mut sender = self
            .sender
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.closed.store(true, Ordering::Release);
        sender.take();
    }
}

impl Drop for ConnectionCleanup {
    fn drop(&mut self) {
        self.close();
    }
}

pub(crate) struct ConnectionLease {
    connection_id: String,
    permit: Option<mpsc::OwnedPermit<String>>,
}

impl ConnectionLease {
    pub(crate) fn completed(&mut self) {
        self.permit.take();
    }
}

impl Drop for ConnectionLease {
    fn drop(&mut self) {
        // Synchronous sender removal prevents further messages immediately.
        super::unregister_hub_connection(&self.connection_id);
        if let Some(permit) = self.permit.take() {
            permit.send(std::mem::take(&mut self.connection_id));
        }
    }
}
