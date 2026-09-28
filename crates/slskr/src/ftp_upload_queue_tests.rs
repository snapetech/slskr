use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::time::{self, Duration};

struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn accepted_queued_uploads_run_when_active_slots_are_released() {
    let queue = FtpUploadQueue::default();
    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    let release = Arc::new(Semaphore::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    for _ in 0..MAX_ADMITTED_UPLOADS {
        let release = Arc::clone(&release);
        let completed = Arc::clone(&completed);
        assert!(
            queue
                .submit(&tasks, async move {
                    let permit = release.acquire().await.unwrap();
                    permit.forget();
                    completed.fetch_add(1, Ordering::SeqCst);
                })
                .await
        );
    }
    assert_eq!(queue.admitted.available_permits(), 0);
    release.add_permits(MAX_ADMITTED_UPLOADS);
    time::timeout(Duration::from_secs(2), async {
        while completed.load(Ordering::SeqCst) != MAX_ADMITTED_UPLOADS
            || queue.admitted.available_permits() != MAX_ADMITTED_UPLOADS
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("all accepted FTP jobs completed");
    assert_eq!(queue.active.available_permits(), MAX_ACTIVE_UPLOADS);
    queue.close();
    tasks.shutdown().await;
}

#[tokio::test]
async fn upload_admission_and_active_work_are_bounded_and_shutdown_wakes_waiters() {
    let queue = FtpUploadQueue::default();
    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    let running = Arc::new(AtomicUsize::new(0));
    for _ in 0..MAX_ADMITTED_UPLOADS {
        let count = Arc::clone(&running);
        assert!(
            queue
                .submit(&tasks, async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    let _active = Active(count);
                    std::future::pending::<()>().await;
                })
                .await
        );
    }
    time::timeout(Duration::from_secs(2), async {
        while running.load(Ordering::SeqCst) != MAX_ACTIVE_UPLOADS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("active upload workers started");
    assert_eq!(queue.admitted.available_permits(), 0);
    assert_eq!(queue.active.available_permits(), 0);

    let extra_started = Arc::new(AtomicBool::new(false));
    let extra = Arc::clone(&extra_started);
    let waiting = queue.submit(&tasks, async move {
        extra.store(true, Ordering::SeqCst);
    });
    tokio::pin!(waiting);
    assert!(time::timeout(Duration::from_millis(20), &mut waiting)
        .await
        .is_err());
    assert!(!extra_started.load(Ordering::SeqCst));
    assert_eq!(running.load(Ordering::SeqCst), MAX_ACTIVE_UPLOADS);

    queue.close();
    assert!(!time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap());
    tasks.shutdown().await;
    assert_eq!(running.load(Ordering::SeqCst), 0);
    assert_eq!(queue.admitted.available_permits(), MAX_ADMITTED_UPLOADS);
    assert_eq!(queue.active.available_permits(), MAX_ACTIVE_UPLOADS);
    assert!(!queue.submit(&tasks, async {}).await);
}

#[tokio::test]
async fn completed_upload_releases_both_slots_and_closed_registry_rejects_work() {
    let queue = FtpUploadQueue::default();
    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    let completed = Arc::new(AtomicBool::new(false));
    let done = Arc::clone(&completed);
    assert!(
        queue
            .submit(&tasks, async move {
                done.store(true, Ordering::SeqCst);
            })
            .await
    );
    time::timeout(Duration::from_secs(2), async {
        while !completed.load(Ordering::SeqCst)
            || queue.admitted.available_permits() != MAX_ADMITTED_UPLOADS
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(queue.active.available_permits(), MAX_ACTIVE_UPLOADS);
    tasks.shutdown().await;
    let rejected_started = Arc::new(AtomicBool::new(false));
    let rejected = Arc::clone(&rejected_started);
    assert!(
        !queue
            .submit(&tasks, async move {
                rejected.store(true, Ordering::SeqCst);
            })
            .await
    );
    assert!(!rejected_started.load(Ordering::SeqCst));
    assert_eq!(queue.admitted.available_permits(), MAX_ADMITTED_UPLOADS);
}
