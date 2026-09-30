use std::future::Future;
use tokio::task::JoinSet;

const MAX_SOAK_TASKS: usize = 64;

/// CLI soak workers are bounded and remain owned until their scope ends.
#[derive(Default)]
pub(super) struct SoakTaskSet {
    tasks: JoinSet<()>,
    closed: bool,
}

impl SoakTaskSet {
    pub(super) fn try_spawn(&mut self, work: impl Future<Output = ()> + Send + 'static) -> bool {
        if self.closed {
            return false;
        }
        while let Some(result) = self.tasks.try_join_next() {
            if let Err(error) = result {
                println!("live soak worker failed: {error}");
            }
        }
        if self.tasks.len() >= MAX_SOAK_TASKS {
            return false;
        }
        self.tasks.spawn(work);
        true
    }

    pub(super) async fn shutdown(&mut self) {
        self.closed = true;
        self.tasks.abort_all();
        while let Some(result) = self.tasks.join_next().await {
            if let Err(error) = result {
                if !error.is_cancelled() {
                    println!("live soak worker failed: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::io::AsyncReadExt;

    struct Lease(Arc<AtomicUsize>);
    impl Drop for Lease {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn stalled_workers_are_bounded_joined_and_close_owned_streams() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut tasks = SoakTaskSet::default();
        let (reader, mut peer) = tokio::io::duplex(16);
        count.fetch_add(1, Ordering::SeqCst);
        let lease = Lease(count.clone());
        assert!(tasks.try_spawn(async move {
            let _lease = lease;
            let _reader = reader;
            std::future::pending::<()>().await;
        }));
        for _ in 1..MAX_SOAK_TASKS {
            count.fetch_add(1, Ordering::SeqCst);
            let lease = Lease(count.clone());
            assert!(tasks.try_spawn(async move {
                let _lease = lease;
                std::future::pending::<()>().await;
            }));
        }
        count.fetch_add(1, Ordering::SeqCst);
        let rejected = Lease(count.clone());
        assert!(!tasks.try_spawn(async move {
            let _lease = rejected;
        }));
        assert_eq!(count.load(Ordering::SeqCst), MAX_SOAK_TASKS);
        tasks.shutdown().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
        assert!(tasks.tasks.is_empty());
        assert!(!tasks.try_spawn(async {}));
    }

    #[tokio::test]
    async fn finished_workers_release_capacity_before_new_admission() {
        let mut tasks = SoakTaskSet::default();
        for _ in 0..MAX_SOAK_TASKS {
            assert!(tasks.try_spawn(async {}));
        }
        tokio::task::yield_now().await;
        assert!(tasks.try_spawn(async {}));
        tasks.shutdown().await;
    }

    #[tokio::test]
    async fn cancelling_owner_closes_child_stream() {
        let (reader, mut peer) = tokio::io::duplex(16);
        let (ready, started) = tokio::sync::oneshot::channel();
        let owner = tokio::spawn(async move {
            let mut tasks = SoakTaskSet::default();
            assert!(tasks.try_spawn(async move {
                let _reader = reader;
                std::future::pending::<()>().await;
            }));
            ready.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        started.await.unwrap();
        owner.abort();
        let _ = owner.await;
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(1), peer.read(&mut [0]))
                .await
                .unwrap()
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn plain_and_obfuscated_soak_deadlines_close_stalled_clients() {
        use super::super::*;
        use tokio::io::AsyncRead;
        for obfuscated in [false, true] {
            let listener = Listener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let owner = tokio::spawn(async move {
                if obfuscated {
                    run_obfuscated_listener(listener, Duration::from_secs(1)).await
                } else {
                    run_listener(listener, Duration::from_secs(1)).await
                }
            });
            let stream = TcpStream::connect(address).await.unwrap();
            let mut client: Box<dyn AsyncRead + Unpin + Send> = if obfuscated {
                Box::new(
                    send_obfuscated_peer_init(
                        stream,
                        "soak-deadline",
                        ConnectionKind::PeerMessages,
                    )
                    .await
                    .unwrap(),
                )
            } else {
                Box::new(
                    send_peer_init(stream, "soak-deadline", ConnectionKind::PeerMessages)
                        .await
                        .unwrap(),
                )
            };
            time::timeout(Duration::from_secs(2), owner)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(
                time::timeout(Duration::from_secs(1), client.read(&mut [0]))
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
            let rebound = Listener::bind(address).await.unwrap();
            drop(rebound);
        }
    }
}
