use std::{
    path::Path,
    process::Stdio,
    sync::{Arc, OnceLock},
    time::Duration,
};

use tokio::{process::Command, sync::Semaphore, time};

use crate::config::{ControllerProfile, ScriptIntegrationSettings};

const MAX_CONCURRENT_SCRIPT_RUNS: usize = 32;

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
    let output = if target == ControllerProfile::Native {
        time::timeout(Duration::from_secs(300), command.output())
            .await
            .map_err(|_| "script timed out after 300s".to_owned())?
    } else {
        command.output().await
    }
    .map_err(|error| format!("failed to run script: {error}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        return Err(format!(
            "STDERR: {}",
            stderr.lines().collect::<Vec<_>>().join(" ")
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub(crate) fn dispatch(
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
        tokio::spawn(async move {
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
mod tests {
    use super::*;
    use crate::config::ScriptRunSettings;

    fn script(run: ScriptRunSettings) -> ScriptIntegrationSettings {
        ScriptIntegrationSettings {
            on: vec!["DownloadFileComplete".to_owned()],
            run,
        }
    }

    #[tokio::test]
    async fn executable_args_and_arglist_modes_receive_event_payload_in_script_directory() {
        let directory =
            std::env::temp_dir().join(format!("slskr-script-modes-{}", uuid::Uuid::new_v4()));
        let payload = r#"{"type":"DownloadFileComplete","version":0}"#;
        let args = script(ScriptRunSettings {
            executable: "/bin/sh".to_owned(),
            args: "-c 'printf %s \"$SLSKD_SCRIPT_DATA\" > args.json'".to_owned(),
            ..Default::default()
        });
        run(&args, &directory, ControllerProfile::Legacy, payload)
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(directory.join("args.json"))
                .await
                .unwrap(),
            payload
        );

        let arglist = script(ScriptRunSettings {
            executable: "/bin/sh".to_owned(),
            arglist: Some(vec![
                "-c".to_owned(),
                "printf %s \"$SLSKD_SCRIPT_DATA\" > arglist.json".to_owned(),
            ]),
            ..Default::default()
        });
        run(&arglist, &directory, ControllerProfile::Native, payload)
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(directory.join("arglist.json"))
                .await
                .unwrap(),
            payload
        );
        tokio::fs::remove_dir_all(directory).await.unwrap();
    }

    #[test]
    fn native_command_safeguard_preserves_the_frozen_target_difference() {
        let command = script(ScriptRunSettings {
            command: "echo $SLSKD_SCRIPT_DATA".to_owned(),
            ..Default::default()
        });
        assert!(command_for(&command, ControllerProfile::Legacy).is_ok());
        assert_eq!(
            command_for(&command, ControllerProfile::Native).unwrap_err(),
            "Command contains disallowed shell metacharacters"
        );
    }

    #[tokio::test]
    async fn any_event_dispatch_runs_only_matching_scripts() {
        let directory =
            std::env::temp_dir().join(format!("slskr-script-dispatch-{}", uuid::Uuid::new_v4()));
        let mut scripts = std::collections::BTreeMap::new();
        scripts.insert(
            "any".to_owned(),
            ScriptIntegrationSettings {
                on: vec!["Any".to_owned()],
                run: ScriptRunSettings {
                    executable: "/bin/sh".to_owned(),
                    arglist: Some(vec![
                        "-c".to_owned(),
                        "printf %s \"$SLSKD_SCRIPT_DATA\" > event.json".to_owned(),
                    ]),
                    ..Default::default()
                },
            },
        );
        dispatch(
            scripts,
            directory.clone(),
            ControllerProfile::Native,
            "DownloadFileComplete",
            &serde_json::json!({"localFilename": "/downloads/file.flac"}),
        );
        for _ in 0..100 {
            if directory.join("event.json").exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let payload = tokio::fs::read_to_string(directory.join("event.json"))
            .await
            .unwrap();
        let payload = serde_json::from_str::<serde_json::Value>(&payload).unwrap();
        assert_eq!(payload["type"], "DownloadFileComplete");
        assert_eq!(payload["version"], 0);
        assert_eq!(payload["localFilename"], "/downloads/file.flac");
        assert!(payload["id"].is_string());
        assert!(payload["timestamp"].is_string());
        tokio::fs::remove_dir_all(directory).await.unwrap();
    }
}
