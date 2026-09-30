//! Own the diagnostic child, its Linux ptrace permission and partial output.

use std::{
    path::PathBuf,
    process::{Child, ExitStatus},
    time::{Duration, Instant},
};

use crate::script_process_group::ProcessGroup;

pub(super) struct CoreDumpProcess {
    pub(super) child: Child,
    process_group: Option<ProcessGroup>,
    output_path: PathBuf,
    ptrace_enabled: bool,
    keep_output: bool,
}

impl CoreDumpProcess {
    // The caller must configure a separate owned process group before spawn.
    pub(super) fn new(child: Child, output_path: PathBuf) -> Self {
        Self {
            process_group: Some(ProcessGroup::new(Some(child.id()))),
            child,
            output_path,
            ptrace_enabled: false,
            keep_output: false,
        }
    }

    pub(super) fn authorize_ptrace(&mut self) -> Result<(), String> {
        rustix::process::set_ptracer(rustix::process::PTracer::ProcessID(
            rustix::process::Pid::from_child(&self.child),
        ))
        .map_err(|error| format!("application dump child authorization failed: {error}"))?;
        self.ptrace_enabled = true;
        Ok(())
    }

    pub(super) fn wait_until(&mut self, deadline: Instant) -> Result<ExitStatus, String> {
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    // No numeric group signal may target a reaped leader.
                    if let Some(group) = self.process_group.as_mut() {
                        group.completed();
                    }
                    return Ok(status);
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(25));
                }
                Ok(None) => return Err("application dump timed out".to_owned()),
                Err(error) => return Err(format!("application dump wait failed: {error}")),
            }
        }
    }

    pub(super) fn keep_output(&mut self) {
        self.keep_output = true;
    }
}

impl Drop for CoreDumpProcess {
    fn drop(&mut self) {
        // Group cleanup runs while the leader is still owned and unreaped.
        self.process_group.take();
        let _ = self.child.kill();
        if let Err(error) = self.child.wait() {
            eprintln!("application dump child cleanup wait failed: {error}");
        }
        if self.ptrace_enabled {
            if let Err(error) = rustix::process::set_ptracer(rustix::process::PTracer::None) {
                eprintln!("application dump child permission cleanup failed: {error}");
            }
        }
        if !self.keep_output {
            let _ = std::fs::remove_file(&self.output_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn spawn(command: &str, arguments: &[&str], output: PathBuf) -> CoreDumpProcess {
        let mut command = Command::new(command);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::script_process_group::configure_blocking(&mut command);
        CoreDumpProcess::new(command.spawn().unwrap(), output)
    }

    #[test]
    fn timeout_and_early_failure_reap_actual_children_and_remove_partial_output() {
        for wait in [false, true] {
            let path = std::env::temp_dir().join(format!("slskr-dump-{}", uuid::Uuid::new_v4()));
            std::fs::write(&path, b"partial").unwrap();
            let mut owner = spawn("/bin/sleep", &["30"], path.clone());
            let pid = owner.child.id();
            if wait {
                assert!(owner
                    .wait_until(Instant::now())
                    .unwrap_err()
                    .contains("timed out"));
            }
            drop(owner);
            assert!(!PathBuf::from(format!("/proc/{pid}")).exists());
            assert!(!path.exists());
        }
    }

    #[test]
    fn completed_child_preserves_only_explicitly_committed_output() {
        for success in [false, true] {
            let path = std::env::temp_dir().join(format!("slskr-dump-{}", uuid::Uuid::new_v4()));
            std::fs::write(&path, b"complete").unwrap();
            let mut owner = spawn(
                "/bin/sh",
                &["-c", if success { "exit 0" } else { "exit 7" }],
                path.clone(),
            );
            let status = owner
                .wait_until(Instant::now() + Duration::from_secs(5))
                .unwrap();
            assert_eq!(status.success(), success);
            if status.success() {
                owner.keep_output();
            }
            drop(owner);
            assert_eq!(path.exists(), success);
            if success {
                assert_eq!(std::fs::read(&path).unwrap(), b"complete");
                std::fs::remove_file(path).unwrap();
            }
        }
    }
}
