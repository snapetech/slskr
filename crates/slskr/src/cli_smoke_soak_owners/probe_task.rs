//! A probe child must not outlive cancellation of its calling probe.
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::task::{JoinError, JoinHandle};

pub(in crate::cli) struct ProbeTask<T>(JoinHandle<T>);

impl<T: Send + 'static> ProbeTask<T> {
    pub(super) fn spawn(future: impl Future<Output = T> + Send + 'static) -> Self {
        Self(tokio::spawn(future))
    }
}

impl<T> From<JoinHandle<T>> for ProbeTask<T> {
    fn from(task: JoinHandle<T>) -> Self {
        Self(task)
    }
}

impl<T> Future for ProbeTask<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}

impl<T> Drop for ProbeTask<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        net::TcpListener,
        sync::oneshot,
        time::{self, Duration},
    };

    struct ListenerClosed {
        listener: Option<TcpListener>,
        sender: Option<oneshot::Sender<()>>,
    }

    impl Drop for ListenerClosed {
        fn drop(&mut self) {
            drop(self.listener.take());
            if let Some(sender) = self.sender.take() {
                let _ = sender.send(());
            }
        }
    }

    async fn stalled_child() -> (ProbeTask<()>, oneshot::Receiver<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let (ready_tx, ready_rx) = oneshot::channel();
        let (closed_tx, closed_rx) = oneshot::channel();
        let task = ProbeTask::spawn(async move {
            let closed = ListenerClosed {
                listener: Some(listener),
                sender: Some(closed_tx),
            };
            let _ = ready_tx.send(());
            if let Some(listener) = &closed.listener {
                let _ = listener.accept().await;
            }
        });
        time::timeout(Duration::from_secs(2), ready_rx)
            .await
            .unwrap()
            .unwrap();
        (task, closed_rx)
    }

    #[tokio::test]
    async fn drop_and_deadline_cancel_owned_listener_tasks() {
        for timeout in [false, true] {
            let (task, closed) = stalled_child().await;
            if timeout {
                assert!(time::timeout(Duration::from_millis(20), task)
                    .await
                    .is_err());
            } else {
                drop(task);
            }
            time::timeout(Duration::from_secs(2), closed)
                .await
                .unwrap()
                .unwrap();
        }
    }

    #[tokio::test]
    async fn owned_task_preserves_results_and_panics() {
        assert_eq!(ProbeTask::spawn(async { 42 }).await.unwrap(), 42);
        assert!(ProbeTask::spawn(async { panic!("fixture failure") })
            .await
            .unwrap_err()
            .is_panic());
    }

    #[tokio::test]
    async fn parent_cancellation_aborts_its_pending_probe_child() {
        let (task, closed) = stalled_child().await;
        let (ready_tx, ready_rx) = oneshot::channel();
        let mut parent = ProbeTask::spawn(async move {
            let _ = ready_tx.send(());
            task.await.unwrap();
        });
        time::timeout(Duration::from_secs(2), ready_rx)
            .await
            .unwrap()
            .unwrap();
        parent.0.abort();
        assert!((&mut parent).await.unwrap_err().is_cancelled());
        time::timeout(Duration::from_secs(2), closed)
            .await
            .unwrap()
            .unwrap();
    }
}
