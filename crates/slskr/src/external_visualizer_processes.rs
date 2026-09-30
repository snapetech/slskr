//! Bounded ownership and reaping of daemon-launched visualizer children.

use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    process::{Child, Command},
    sync::{OwnedSemaphorePermit, Semaphore},
};

#[derive(Debug)]
struct OwnedChild {
    child: Child,
    _permit: OwnedSemaphorePermit,
}

#[derive(Debug)]
pub(super) struct ExternalVisualizerProcesses {
    children: Mutex<Option<Vec<OwnedChild>>>,
}

impl Default for ExternalVisualizerProcesses {
    fn default() -> Self {
        Self {
            children: Mutex::new(Some(Vec::new())),
        }
    }
}

impl ExternalVisualizerProcesses {
    pub(super) fn launch(
        &self,
        command: &mut Command,
        permits: &Arc<Semaphore>,
    ) -> io::Result<u32> {
        let mut children = self
            .children
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(children) = children.as_mut() else {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "external visualizer admission is closed",
            ));
        };
        reap_finished(children);
        let permit = Arc::clone(permits).try_acquire_owned().map_err(|_| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "external visualizer process limit reached",
            )
        })?;
        let child = command.kill_on_drop(true).spawn()?;
        let id = child.id().expect("newly spawned child has a process ID");
        children.push(OwnedChild {
            child,
            _permit: permit,
        });
        Ok(id)
    }

    pub(super) fn reap_finished(&self) {
        let mut children = self
            .children
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(children) = children.as_mut() {
            reap_finished(children);
        }
    }

    /// Close admission under the same lock as launch, then kill and reap children.
    pub(super) async fn shutdown(&self) -> usize {
        let children = self
            .children
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let Some(mut children) = children else {
            return 0;
        };
        for child in &mut children {
            if let Err(error) = child.child.start_kill() {
                tracing::warn!(%error, "external visualizer kill failed");
            }
        }
        let mut reaped = 0;
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            for child in &mut children {
                match child.child.wait().await {
                    Ok(_) => reaped += 1,
                    Err(error) => tracing::warn!(%error, "external visualizer reap failed"),
                }
            }
        })
        .await;
        if result.is_err() {
            tracing::warn!("external visualizer shutdown reap deadline exceeded");
        }
        reaped
    }
}

fn reap_finished(children: &mut Vec<OwnedChild>) {
    children.retain_mut(|child| {
        let process_id = child.child.id();
        match child.child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    tracing::warn!(
                        ?process_id,
                        ?status,
                        "external visualizer exited unsuccessfully"
                    );
                }
                false
            }
            Ok(None) => true,
            Err(error) => {
                tracing::warn!(?process_id, %error, "external visualizer process wait failed");
                true
            }
        }
    });
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sleeper() -> Command {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exec sleep 30"]);
        command
    }

    #[tokio::test]
    async fn caps_children_and_joins_them_before_rejecting_late_launch() {
        let owner = ExternalVisualizerProcesses::default();
        let permits = Arc::new(Semaphore::new(crate::MAX_EXTERNAL_VISUALIZER_PROCESSES));
        for _ in 0..crate::MAX_EXTERNAL_VISUALIZER_PROCESSES {
            owner.launch(&mut sleeper(), &permits).unwrap();
        }
        assert_eq!(
            owner.launch(&mut sleeper(), &permits).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(
            owner.shutdown().await,
            crate::MAX_EXTERNAL_VISUALIZER_PROCESSES
        );
        assert_eq!(owner.shutdown().await, 0);
        assert_eq!(
            owner.launch(&mut sleeper(), &permits).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
    }

    #[tokio::test]
    async fn completed_and_failed_children_release_capacity() {
        let owner = ExternalVisualizerProcesses::default();
        let permits = Arc::new(Semaphore::new(crate::MAX_EXTERNAL_VISUALIZER_PROCESSES));
        for _ in 0..crate::MAX_EXTERNAL_VISUALIZER_PROCESSES + 1 {
            let mut command = Command::new("/slskr-intentionally-missing-visualizer");
            assert!(owner.launch(&mut command, &permits).is_err());
        }
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exit 7"]);
        owner.launch(&mut command, &permits).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                owner.reap_finished();
                if owner.children.lock().unwrap().as_ref().unwrap().is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        owner.launch(&mut sleeper(), &permits).unwrap();
        assert_eq!(owner.shutdown().await, 1);
    }
}
