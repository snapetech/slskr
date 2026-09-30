use super::fixtures::*;

const TEST_NAME: &str = "focused_controller_tests::delayed_filesystem::mounted_sync_delay_preserves_queue_and_crash_recovery";

struct OwnedWriter(std::process::Child);

impl Drop for OwnedWriter {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the isolated delayed FUSE mount from run-rf-delayed-filesystem-proof.py"]
async fn mounted_sync_delay_preserves_queue_and_crash_recovery() {
    if let Some(directory) = std::env::var_os("SLSKR_RF047_DELAYED_CHILD") {
        let directory = std::path::PathBuf::from(directory);
        let (state, _receiver) = test_state_with_env(MapEnv::default().with(
            "SLSKR_STATE_DIR",
            directory.to_str().expect("child directory"),
        ));
        for index in 0..8 {
            {
                let mut transfers = state.transfers.write().await;
                let entry = transfers.create(
                    0,
                    Some("mounted-crash-peer".to_owned()),
                    format!("Remote/Mounted-{index}.flac"),
                    None,
                    Some(100),
                );
                transfers.update_status(entry.id, "in_progress", Some(25), None);
            }
            crate::persist_transfer_durability(&state).await;
        }
        fs::write(directory.join("ready"), b"durable").expect("publish durable writer state");
        use std::io::Read;
        let _ = std::io::stdin().read(&mut [0_u8; 1]);
        return;
    }

    let mount =
        std::path::PathBuf::from(std::env::var_os("SLSKR_RF047_MOUNT").expect("isolated mount"));
    let control =
        std::path::PathBuf::from(std::env::var_os("SLSKR_RF047_CONTROL").expect("fixture control"));
    let directory = mount.join("concurrent");
    let (state, _receiver) = test_state_with_env(MapEnv::default().with(
        "SLSKR_STATE_DIR",
        directory.to_str().expect("state directory"),
    ));
    {
        let mut transfers = state.transfers.write().await;
        transfers.create(
            0,
            Some("mounted-peer".to_owned()),
            "Remote/Mounted.flac".to_owned(),
            None,
            Some(100),
        );
    }
    fs::write(control.join("enabled"), b"500ms").expect("enable filesystem sync delay");
    let persistence_state = Arc::clone(&state);
    let started = std::time::Instant::now();
    let persistence = tokio::spawn(async move {
        crate::persist_transfer_durability(&persistence_state).await;
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !control.join("observed").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("filesystem must observe actual sync request");
    assert!(
        !persistence.is_finished(),
        "filesystem sync must still be delayed"
    );
    let queue_started = std::time::Instant::now();
    let mut transfers = tokio::time::timeout(Duration::from_millis(250), state.transfers.write())
        .await
        .expect("mounted slow sync must leave queue writable");
    transfers.update_status(1, "in_progress", Some(25), None);
    drop(transfers);
    assert!(queue_started.elapsed() < Duration::from_millis(250));
    persistence.await.expect("first durability flush completes");
    assert!(
        started.elapsed() >= Duration::from_millis(450),
        "actual mounted sync delay was exercised"
    );
    crate::persist_transfer_durability(&state).await;
    let restarted = crate::TransferQueue::new(&state.config);
    assert_eq!(restarted.entries.len(), 1);
    assert_eq!(restarted.entries[0].bytes_transferred, 25);
    let events = fs::read_to_string(directory.join("transfer-events.tsv")).expect("ordered events");
    assert_eq!(events.matches("\tqueued\t").count(), 1);
    assert_eq!(events.matches("\t25\tin_progress\t").count(), 1);

    for cycle in 0..3 {
        let directory = mount.join(format!("crash-{cycle}"));
        fs::create_dir_all(&directory).expect("create isolated crash directory");
        let child = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--ignored", "--exact", TEST_NAME])
            .env("SLSKR_RF047_DELAYED_CHILD", &directory)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("start delayed-storage writer");
        let mut writer = OwnedWriter(child);
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        while !directory.join("ready").exists() && std::time::Instant::now() < deadline {
            assert!(
                writer.0.try_wait().expect("writer status").is_none(),
                "writer exited before durable state"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(
            directory.join("ready").exists(),
            "writer readiness deadline"
        );
        writer.0.kill().expect("kill writer without shutdown");
        writer.0.wait().expect("reap killed writer");
        let (restarted, _receiver) = test_state_with_env(MapEnv::default().with(
            "SLSKR_STATE_DIR",
            directory.to_str().expect("restart directory"),
        ));
        let transfers = restarted.transfers.read().await;
        assert_eq!(transfers.entries.len(), 8);
        for entry in &transfers.entries {
            assert_eq!(entry.bytes_transferred, 25);
            assert_eq!(entry.status, "queued");
            assert_eq!(entry.reason.as_deref(), Some("resumed after restart"));
        }
        let events =
            fs::read_to_string(directory.join("transfer-events.tsv")).expect("recovered events");
        assert_eq!(events.matches("\tqueued\t").count(), 8);
        assert_eq!(events.matches("\t25\tin_progress\t").count(), 8);
    }
    println!("RF047 mounted 500ms sync delay: queue writable; three killed writers; 24 transfers and 48 ordered events recovered");
}
