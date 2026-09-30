use std::{
    path::Path,
    process::Stdio,
    sync::{Arc, OnceLock},
    time::Duration,
};

use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::Semaphore,
    time,
};

use crate::script_process_group;

use crate::config::{ControllerProfile, ScriptIntegrationSettings};

const MAX_CONCURRENT_SCRIPT_RUNS: usize = 32;
const MAX_SCRIPT_OUTPUT_BYTES: usize = 1024 * 1024;
const SCRIPT_TIMEOUT: Duration = Duration::from_secs(300);

fn format_timeout(duration: Duration) -> String {
    let milliseconds = duration.as_millis();
    if milliseconds.is_multiple_of(1_000) {
        format!("{}s", milliseconds / 1_000)
    } else {
        format!("{milliseconds}ms")
    }
}

static SCRIPT_RUN_PERMITS: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn script_run_permits() -> &'static Arc<Semaphore> {
    SCRIPT_RUN_PERMITS.get_or_init(|| Arc::new(Semaphore::new(MAX_CONCURRENT_SCRIPT_RUNS)))
}

fn command_for(
    script: &ScriptIntegrationSettings,
    target: ControllerProfile,
) -> Result<Command, String> {
    if !script.run.command.is_empty() {
        if target == ControllerProfile::Native
            && !script.run.command.starts_with("-c")
            && script.run.command.chars().any(|ch| {
                matches!(
                    ch,
                    '&' | '|' | ';' | '`' | '$' | '(' | ')' | '<' | '>' | '\n' | '\r'
                )
            })
        {
            return Err("Command contains disallowed shell metacharacters".to_owned());
        }
        #[cfg(windows)]
        let (shell, prefix) = ("cmd.exe".to_owned(), "/c");
        #[cfg(not(windows))]
        // CI containers and service managers commonly omit SHELL.  `/bin/sh`
        // is the POSIX fallback required for command-mode integrations.
        let (shell, prefix) = (
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned()),
            "-c",
        );
        let mut command = Command::new(shell);
        if script.run.command.starts_with(prefix) {
            command.args(
                shell_words::split(&script.run.command)
                    .map_err(|error| format!("invalid command arguments: {error}"))?,
            );
        } else {
            command
                .arg(prefix)
                .arg(script.run.command.trim_matches('"'));
        }
        return Ok(command);
    }

    let mut command = Command::new(&script.run.executable);
    if let Some(arguments) = &script.run.arglist {
        command.args(arguments);
    } else if !script.run.args.is_empty() {
        command.args(
            shell_words::split(&script.run.args)
                .map_err(|error| format!("invalid script arguments: {error}"))?,
        );
    }
    Ok(command)
}

pub(crate) async fn run(
    script: &ScriptIntegrationSettings,
    script_directory: &Path,
    target: ControllerProfile,
    payload: &str,
) -> Result<Vec<String>, String> {
    run_with_timeout(script, script_directory, target, payload, SCRIPT_TIMEOUT).await
}

async fn run_with_timeout(
    script: &ScriptIntegrationSettings,
    script_directory: &Path,
    target: ControllerProfile,
    payload: &str,
    timeout_duration: Duration,
) -> Result<Vec<String>, String> {
    tokio::fs::create_dir_all(script_directory)
        .await
        .map_err(|error| format!("failed to create script directory: {error}"))?;
    let mut command = command_for(script, target)?;
    command
        .current_dir(script_directory)
        .env("SLSKD_SCRIPT_DATA", payload)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    script_process_group::configure(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to run script: {error}"))?;
    // Declared after child: group cancellation happens before child drop.
    let mut process_group = script_process_group::ProcessGroup::new(child.id());
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "script stdout pipe was not created".to_owned())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "script stderr pipe was not created".to_owned())?;
    let output = time::timeout(timeout_duration, async {
        let (stdout, stderr) = tokio::try_join!(
            read_script_output(&mut stdout, "stdout"),
            read_script_output(&mut stderr, "stderr"),
        )?;
        let status = child
            .wait()
            .await
            .map_err(|error| format!("failed to wait for script: {error}"))?;
        // There is no await between reaping and disarming the numeric ID.
        process_group.completed();
        Ok::<_, String>((status, stdout, stderr))
    })
    .await
    .map_err(|_| {
        format!(
            "script timed out after {}",
            format_timeout(timeout_duration)
        )
    })??;
    let (status, stdout, stderr) = output;
    let stderr = String::from_utf8_lossy(&stderr);
    if !stderr.is_empty() {
        return Err(format!(
            "STDERR: {}",
            stderr.lines().collect::<Vec<_>>().join(" ")
        ));
    }
    if !status.success() {
        return Err(format!("script exited unsuccessfully: {status}"));
    }
    Ok(String::from_utf8_lossy(&stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

async fn read_script_output<R>(reader: &mut R, stream: &str) -> Result<Vec<u8>, String>
where
    R: AsyncRead + Unpin,
{
    let mut output = Vec::new();
    reader
        .take((MAX_SCRIPT_OUTPUT_BYTES.saturating_add(1)) as u64)
        .read_to_end(&mut output)
        .await
        .map_err(|error| format!("script {stream} read failed: {error}"))?;
    if output.len() > MAX_SCRIPT_OUTPUT_BYTES {
        return Err(format!(
            "script {stream} exceeded the {MAX_SCRIPT_OUTPUT_BYTES} byte output limit"
        ));
    }
    Ok(output)
}

pub(crate) fn dispatch(
    tasks: &crate::managed_tasks::ManagedTaskRegistry,
    scripts: std::collections::BTreeMap<String, ScriptIntegrationSettings>,
    script_directory: std::path::PathBuf,
    target: ControllerProfile,
    event_name: &str,
    data: &serde_json::Value,
) {
    let mut payload = serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "type": event_name,
        "version": 0,
    });
    if let (Some(payload), Some(data)) = (payload.as_object_mut(), data.as_object()) {
        payload.extend(data.clone());
    }
    let payload = payload.to_string();
    for (name, script) in scripts {
        if !script.on.iter().any(|value| {
            value.eq_ignore_ascii_case("Any") || value.eq_ignore_ascii_case(event_name)
        }) {
            continue;
        }
        let permit = match Arc::clone(script_run_permits()).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                eprintln!(
                    "[Warning] script: Dropping event type {event_name} for script '{name}'; the concurrent script-run limit ({MAX_CONCURRENT_SCRIPT_RUNS}) is full"
                );
                continue;
            }
        };
        let payload = payload.clone();
        let directory = script_directory.clone();
        let event_name = event_name.to_owned();
        tasks.spawn(async move {
            let _permit = permit;
            match run(&script, &directory, target, &payload).await {
                Ok(output) => eprintln!(
                    "[Debug] script: Script '{name}' ran successfully; output: {output:?}"
                ),
                Err(error) => eprintln!(
                    "[Warning] script: Failed to run script '{name}' for event type {event_name}: {error}"
                ),
            }
        });
    }
}

#[cfg(all(test, unix))]
#[path = "scripts_tests.rs"]
mod tests;
