use super::fixtures::*;
use crate::relay::{self, CredentialScheme};

async fn register_agent(state: &Arc<crate::AppState>, id: &str) {
    let mut settings = state.advanced_networking.read().await.relay.clone();
    settings.agents.insert(
        "owned-agent".to_owned(),
        crate::config::RelayAgentSettings {
            instance_name: "owned-agent".to_owned(),
            secret: "fixture-secret".to_owned(),
            cidr: "127.0.0.0/8".to_owned(),
        },
    );
    let mut runtime = state.relay.write().await;
    let now = crate::unix_timestamp();
    let challenge = runtime.protocol.issue_challenge(id, now);
    let credential = relay::credential_for_with_scheme(
        CredentialScheme::NativeHmacBase64,
        "fixture-secret",
        "owned-agent",
        &challenge,
    );
    assert!(runtime.protocol.authenticate_agent(
        &settings,
        CredentialScheme::NativeHmacBase64,
        id,
        "owned-agent",
        &credential,
        "127.0.0.1".parse().unwrap(),
        now
    ));
}

#[tokio::test]
async fn connection_drop_deregisters_agent_after_contended_protocol_lock_releases() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let id = format!("cleanup-{}", uuid::Uuid::new_v4());
    let lease = state
        .relay_cleanup
        .reserve(&state, id.clone())
        .await
        .unwrap();
    register_agent(&state, &id).await;
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    relay::register_hub_connection(id.clone(), sender);
    let runtime = state.relay.write().await;
    assert!(relay::send_hub_invocation(
        &runtime.protocol,
        "owned-agent",
        "before",
        Vec::new()
    ));
    drop(lease);
    assert!(!relay::send_hub_invocation(
        &runtime.protocol,
        "owned-agent",
        "after",
        Vec::new()
    ));
    assert_eq!(
        runtime.protocol.connection_for_agent("owned-agent"),
        Some(id.as_str())
    );
    drop(runtime);
    tokio::time::timeout(Duration::from_secs(2), async {
        while state
            .relay
            .read()
            .await
            .protocol
            .connection_for_agent("owned-agent")
            .is_some()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("reserved cleanup removes cancelled agent");
    state.shutdown_managed_tasks().await;
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn daemon_shutdown_clears_active_relay_registration_and_rejects_new_leases() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let id = format!("shutdown-{}", uuid::Uuid::new_v4());
    let lease = state
        .relay_cleanup
        .reserve(&state, id.clone())
        .await
        .unwrap();
    register_agent(&state, &id).await;
    let (sender, _receiver) = tokio::sync::mpsc::channel(1);
    relay::register_hub_connection(id, sender);
    state.shutdown_managed_tasks().await;
    assert!(state
        .relay
        .read()
        .await
        .protocol
        .connection_for_agent("owned-agent")
        .is_none());
    assert!(state
        .relay_cleanup
        .reserve(&state, "late".to_owned())
        .await
        .is_err());
    drop(lease);
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}

#[tokio::test]
async fn full_cleanup_reservations_apply_backpressure_and_shutdown_wakes_waiters() {
    let (state, _receiver) = test_state_with_env(MapEnv::default());
    let mut leases = Vec::new();
    for index in 0..256 {
        leases.push(
            state
                .relay_cleanup
                .reserve(&state, format!("capacity-{index}"))
                .await
                .unwrap(),
        );
    }
    let waiting = state.relay_cleanup.reserve(&state, "waiting".to_owned());
    tokio::pin!(waiting);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut waiting)
            .await
            .is_err()
    );
    state.shutdown_managed_tasks().await;
    assert!(tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .is_err());
    drop(leases);
    fs::remove_dir_all(&state.config.state_dir).unwrap();
}
