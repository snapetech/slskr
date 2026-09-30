//! Bounded admission for completed-download FTP work, owned by daemon shutdown.
use std::{future::Future, sync::Arc};
use tokio::sync::Semaphore;

const MAX_ADMITTED_UPLOADS: usize = 64;
const MAX_ACTIVE_UPLOADS: usize = 4;

#[derive(Debug)]
pub(crate) struct FtpUploadQueue {
    admitted: Arc<Semaphore>,
    active: Arc<Semaphore>,
}

impl Default for FtpUploadQueue {
    fn default() -> Self {
        Self {
            admitted: Arc::new(Semaphore::new(MAX_ADMITTED_UPLOADS)),
            active: Arc::new(Semaphore::new(MAX_ACTIVE_UPLOADS)),
        }
    }
}

impl FtpUploadQueue {
    /// Full admission applies backpressure before creating a worker. An
    /// accepted job retains its slot through queueing and upload completion.
    pub(crate) async fn submit(
        &self,
        tasks: &crate::managed_tasks::ManagedTaskRegistry,
        upload: impl Future<Output = ()> + Send + 'static,
    ) -> bool {
        let Ok(admitted) = Arc::clone(&self.admitted).acquire_owned().await else {
            return false;
        };
        let active = Arc::clone(&self.active);
        tasks.try_spawn(async move {
            let _admitted = admitted;
            let Ok(_active) = active.acquire_owned().await else {
                return;
            };
            upload.await;
        })
    }

    pub(crate) fn close(&self) {
        self.admitted.close();
        self.active.close();
    }
}

impl Drop for FtpUploadQueue {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
#[path = "ftp_upload_queue_tests.rs"]
mod tests;
