//! Cancellation ownership for an integration script's Unix process group.

pub(super) fn configure(command: &mut tokio::process::Command) {
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(not(unix))]
    let _ = command;
}

#[cfg(target_os = "linux")]
pub(super) fn configure_blocking(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

pub(super) struct ProcessGroup {
    #[cfg(unix)]
    pid: Option<rustix::process::Pid>,
}

impl ProcessGroup {
    pub(super) fn new(child_pid: Option<u32>) -> Self {
        #[cfg(unix)]
        {
            let pid = child_pid
                .and_then(|pid| i32::try_from(pid).ok())
                .filter(|pid| *pid > 1)
                .and_then(rustix::process::Pid::from_raw);
            Self { pid }
        }
        #[cfg(not(unix))]
        {
            let _ = child_pid;
            Self {}
        }
    }

    /// Disarm immediately after wait reaps the leader. Keeping a numeric
    /// group ID after reaping could target a subsequently reused process ID.
    pub(super) fn completed(&mut self) {
        #[cfg(unix)]
        {
            self.pid = None;
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Some(pid) = self.pid {
            // The leader has its own group and remains unreaped while this
            // guard is armed. Cancellation kills only this owned script tree.
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::ProcessGroup;

    #[test]
    fn rejects_missing_and_non_owned_group_ids() {
        for pid in [None, Some(0), Some(1), Some(u32::MAX)] {
            assert!(ProcessGroup::new(pid).pid.is_none());
        }
    }
}
