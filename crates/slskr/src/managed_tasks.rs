use std::{future::Future, sync::Arc};

use tokio::{sync::Semaphore, task::JoinSet, time};

use std::sync::Mutex;

pub(super) const MAX_HTTP_CONNECTION_TASKS: usize = 256;

/// Owns long-lived supervisor tasks and bounded HTTPS/Unix HTTP handlers so
/// lifecycle shutdown can abort and join them before the runtime exits.
#[derive(Debug)]
pub(super) struct ManagedTaskRegistry {
    tasks: Mutex<Option<JoinSet<()>>>,
}

impl Default for ManagedTaskRegistry {
    fn default() -> Self {
        Self {
            tasks: Mutex::new(Some(JoinSet::new())),
        }
    }
}

impl ManagedTaskRegistry {
    pub(super) fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.try_spawn(future);
    }

    pub(super) fn try_spawn<F>(&self, future: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(tasks) = tasks.as_mut() {
            while tasks.try_join_next().is_some() {}
            tasks.spawn(future);
            return true;
        }
        false
    }

    pub(super) fn spawn_bounded_http<F>(&self, connections: &Arc<Semaphore>, future: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(tasks) = tasks.as_mut() else {
            return false;
        };
        while tasks.try_join_next().is_some() {}
        spawn_bounded_http_connection_task(tasks, connections, future)
    }

    pub(super) async fn shutdown(&self) {
        let tasks = self
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let Some(mut tasks) = tasks else {
            return;
        };
        let _ = time::timeout(crate::MANAGED_BACKGROUND_SHUTDOWN_TIMEOUT, tasks.shutdown()).await;
    }
}

pub(super) fn spawn_bounded_http_connection_task<F>(
    tasks: &mut JoinSet<()>,
    connections: &Arc<Semaphore>,
    future: F,
) -> bool
where
    F: Future<Output = ()> + Send + 'static,
{
    let Ok(permit) = Arc::clone(connections).try_acquire_owned() else {
        return false;
    };
    tasks.spawn(async move {
        let _permit = permit;
        future.await;
    });
    true
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    struct DropMarker(Arc<AtomicBool>);

    impl Drop for DropMarker {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }

    #[tokio::test]
    async fn shutdown_aborts_and_joins_registered_tasks() {
        let registry = ManagedTaskRegistry::default();
        let started = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let task_started = Arc::clone(&started);
        let marker = DropMarker(Arc::clone(&dropped));

        registry.spawn(async move {
            task_started.store(true, Ordering::Release);
            let _marker = marker;
            std::future::pending::<()>().await;
        });

        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while !started.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("managed task should start");

        registry.shutdown().await;

        assert!(dropped.load(Ordering::Acquire));
        registry.shutdown().await;
    }
}
