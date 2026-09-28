use super::fixtures::*;

#[tokio::test]
async fn daemon_shutdown_wakes_full_ftp_admission_waiters() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    for _ in 0..64 {
        assert!(
            state
                .ftp_uploads
                .submit(
                    &state.managed_background_tasks,
                    std::future::pending::<()>()
                )
                .await
        );
    }
    let waiting = state
        .ftp_uploads
        .submit(&state.managed_background_tasks, async {});
    tokio::pin!(waiting);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    state.shutdown_managed_tasks().await;
    assert!(!tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap());
    assert!(
        !state
            .ftp_uploads
            .submit(&state.managed_background_tasks, async {})
            .await
    );
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}
