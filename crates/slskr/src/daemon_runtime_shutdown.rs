use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

const BLOCKING_WORK_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn shutdown(runtime: Runtime) {
    let elapsed = shutdown_with_timeout(runtime, BLOCKING_WORK_SHUTDOWN_TIMEOUT);
    if elapsed >= BLOCKING_WORK_SHUTDOWN_TIMEOUT {
        eprintln!(
            "daemon runtime shutdown reached its five-second blocking-work deadline; remaining blocking work may continue until the operating system call returns"
        );
    }
}

fn shutdown_with_timeout(runtime: Runtime, timeout: Duration) -> Duration {
    let started = Instant::now();
    runtime.shutdown_timeout(timeout);
    started.elapsed()
}

#[cfg(test)]
mod tests {
    use super::shutdown_with_timeout;
    use std::sync::mpsc::{self, TryRecvError};
    use std::time::Duration;

    #[test]
    fn running_blocking_work_cannot_hold_runtime_shutdown_past_its_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("build test runtime");
        let (started_sender, started_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let (finished_sender, finished_receiver) = mpsc::channel();

        runtime.spawn_blocking(move || {
            started_sender.send(()).expect("report worker start");
            release_receiver.recv().expect("wait for test release");
            finished_sender.send(()).expect("report worker completion");
        });
        started_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("blocking worker starts");

        let timeout = Duration::from_millis(25);
        let elapsed = shutdown_with_timeout(runtime, timeout);
        assert!(
            elapsed >= timeout,
            "shutdown returned before its deadline: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "shutdown exceeded its cap: {elapsed:?}"
        );
        assert_eq!(finished_receiver.try_recv(), Err(TryRecvError::Empty));

        release_sender.send(()).expect("release blocking worker");
        finished_receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("blocking worker completes after release");
    }
}
