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

#[tokio::test]
async fn compatibility_script_timeout_bounds_hung_commands() {
    let directory =
        std::env::temp_dir().join(format!("slskr-script-timeout-{}", uuid::Uuid::new_v4()));
    let script = script(ScriptRunSettings {
        executable: "/bin/sh".to_owned(),
        arglist: Some(vec!["-c".to_owned(), "sleep 1".to_owned()]),
        ..Default::default()
    });

    let error = run_with_timeout(
        &script,
        &directory,
        ControllerProfile::Legacy,
        "{}",
        Duration::from_millis(10),
    )
    .await
    .unwrap_err();

    assert_eq!(error, "script timed out after 10ms");
    tokio::fs::remove_dir_all(directory).await.unwrap();
}

#[tokio::test]
async fn script_nonzero_exit_without_stderr_is_reported() {
    let directory =
        std::env::temp_dir().join(format!("slskr-script-exit-{}", uuid::Uuid::new_v4()));
    let script = script(ScriptRunSettings {
        executable: "/bin/sh".to_owned(),
        arglist: Some(vec!["-c".to_owned(), "exit 7".to_owned()]),
        ..Default::default()
    });

    let error = run(&script, &directory, ControllerProfile::Native, "{}")
        .await
        .expect_err("non-zero script exit");

    assert!(error.contains("script exited unsuccessfully"), "{error}");
    tokio::fs::remove_dir_all(directory).await.unwrap();
}

#[tokio::test]
async fn script_output_reader_rejects_oversized_output() {
    let data = vec![b'x'; MAX_SCRIPT_OUTPUT_BYTES + 1];
    let mut reader = &data[..];
    let error = read_script_output(&mut reader, "stdout")
        .await
        .expect_err("oversized script output");

    assert!(error.contains("output limit"), "{error}");
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
    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    dispatch(
        &tasks,
        scripts,
        directory.clone(),
        ControllerProfile::Native,
        "DownloadFileComplete",
        &serde_json::json!({"localFilename": "/downloads/file.flac"}),
    );
    let event_path = directory.join("event.json");
    let mut payload = None;
    for _ in 0..100 {
        if let Ok(contents) = tokio::fs::read_to_string(&event_path).await {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
                payload = Some(value);
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    tasks.shutdown().await;
    let payload = payload.expect("script event payload was not written as valid JSON");
    assert_eq!(payload["type"], "DownloadFileComplete");
    assert_eq!(payload["version"], 0);
    assert_eq!(payload["localFilename"], "/downloads/file.flac");
    assert!(payload["id"].is_string());
    assert!(payload["timestamp"].is_string());
    tokio::fs::remove_dir_all(directory).await.unwrap();
}
#[tokio::test]
async fn script_dispatch_rejects_work_after_registry_shutdown() {
    let tasks = crate::managed_tasks::ManagedTaskRegistry::default();
    tasks.shutdown().await;
    let directory =
        std::env::temp_dir().join(format!("slskr-script-stopped-{}", uuid::Uuid::new_v4()));
    let mut integration = script(ScriptRunSettings {
        executable: "/bin/sh".to_owned(),
        arglist: Some(vec!["-c".to_owned(), "touch should-not-run".to_owned()]),
        ..Default::default()
    });
    integration.on = vec!["Any".to_owned()];
    dispatch(
        &tasks,
        std::collections::BTreeMap::from([("stopped".to_owned(), integration)]),
        directory.clone(),
        ControllerProfile::Native,
        "DownloadFileComplete",
        &serde_json::json!({}),
    );
    tokio::task::yield_now().await;
    assert!(
        !directory.exists(),
        "closed script registry must not create a process or directory"
    );
}

#[cfg(target_os = "linux")]
fn descendant_running(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .map(|(_, fields)| fields.trim_start().starts_with('Z'))
                .map(|zombie| !zombie)
        })
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
async fn descendant_stopped(pid: u32) {
    tokio::time::timeout(Duration::from_secs(2), async {
        while descendant_running(pid) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled script must leave no live descendant");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn script_timeout_and_cancellation_stop_real_shell_descendants() {
    for cancel in [false, true] {
        let directory =
            std::env::temp_dir().join(format!("slskr-script-child-{}", uuid::Uuid::new_v4()));
        let integration = script(ScriptRunSettings {
            executable: "/bin/sh".to_owned(),
            arglist: Some(vec![
                "-c".to_owned(),
                "sleep 30 & echo $! > descendant.pid; wait".to_owned(),
            ]),
            ..Default::default()
        });
        let task_directory = directory.clone();
        struct OwnedRun(tokio::task::JoinHandle<Result<Vec<String>, String>>);
        impl Drop for OwnedRun {
            fn drop(&mut self) {
                self.0.abort();
            }
        }
        let mut task = OwnedRun(tokio::spawn(async move {
            run_with_timeout(
                &integration,
                &task_directory,
                ControllerProfile::Native,
                "{}",
                if cancel {
                    Duration::from_secs(30)
                } else {
                    Duration::from_secs(1)
                },
            )
            .await
        }));
        let pid = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if let Ok(pid) = tokio::fs::read_to_string(directory.join("descendant.pid")).await {
                    if let Ok(pid) = pid.trim().parse::<u32>() {
                        break pid;
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("script descendant readiness");
        // Keep a stable kernel reference for cleanup even if an assertion
        // fails. A numeric PID could otherwise be reused before drop.
        struct OwnedDescendant(Option<rustix::fd::OwnedFd>);
        impl Drop for OwnedDescendant {
            fn drop(&mut self) {
                if let Some(fd) = &self.0 {
                    let _ = rustix::process::pidfd_send_signal(fd, rustix::process::Signal::KILL);
                }
            }
        }
        let mut descendant = OwnedDescendant(Some(
            rustix::process::pidfd_open(
                rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap(),
                rustix::process::PidfdFlags::empty(),
            )
            .expect("open owned descendant pidfd"),
        ));
        assert!(
            descendant_running(pid),
            "fixture descendant must actually run before cancellation"
        );
        if cancel {
            task.0.abort();
            assert!((&mut task.0).await.unwrap_err().is_cancelled());
        } else {
            assert!((&mut task.0)
                .await
                .unwrap()
                .unwrap_err()
                .contains("timed out"));
        }
        descendant_stopped(pid).await;
        descendant.0 = None;
        tokio::fs::remove_dir_all(directory).await.unwrap();
    }
}
