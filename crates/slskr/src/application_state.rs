//! Owned compatibility snapshots for the application-state projections.
//!
//! HTTP and SignalR both expose this projection. Capture the small values they
//! need while each lock is held, then serialize the owned snapshot after all
//! in-memory locks have been released. This keeps response construction from
//! extending lock lifetimes or coupling the two transports to store guards.

use crate::{
    config::{AppConfig, ControllerProfile},
    vpn, AppState, RuntimeCompatState, SessionSnapshot,
};

struct ApplicationStateSnapshot {
    session: SessionSnapshot,
    pending_reconnect: bool,
    pending_restart: bool,
    vpn: serde_json::Value,
    relay_enabled: bool,
    relay_updated_at: u64,
    relay_agent_enabled: bool,
    bridge_running: bool,
    bridge_config_updates: u64,
    operations: serde_json::Value,
    runtime: serde_json::Value,
    distributed_network: serde_json::Value,
    shares: serde_json::Value,
    room_names: Vec<String>,
    user_names: Vec<String>,
    runtime_credentials_configured: bool,
    connected_endpoint: Option<String>,
    version: serde_json::Value,
}

fn application_vpn_json(
    runtime: &RuntimeCompatState,
    profile: ControllerProfile,
) -> serde_json::Value {
    let mut vpn = serde_json::json!({
        "isReady": runtime.vpn.is_ready,
        "isConnected": runtime.vpn.is_connected,
        "publicIPAddress": runtime.vpn.public_ip_address.map(|value| value.to_string()),
        "location": runtime.vpn.location,
        "forwardedPort": runtime.vpn.forwarded_port,
    });
    if let Some(relay_status) = runtime.vpn.relay.as_ref() {
        vpn["relay"] = relay_status.json();
    }
    if profile == ControllerProfile::Native {
        vpn["portForwards"] = serde_json::Value::Array(
            runtime
                .vpn
                .port_forwards
                .iter()
                .map(vpn::PortForward::json)
                .collect(),
        );
    }
    vpn
}

async fn application_state_snapshot(state: &AppState) -> ApplicationStateSnapshot {
    let profile = state.config.controller_profile;
    let session = state.session.read().await.clone();
    let shares = state.share_lifecycle.read().await.json(profile);
    let room_names = state
        .rooms
        .read()
        .await
        .records
        .iter()
        .map(|room| room.name.clone())
        .collect();
    let user_names = state
        .users
        .read()
        .await
        .records
        .iter()
        .map(|user| user.username.clone())
        .collect();
    let (relay_enabled, relay_updated_at) = {
        let relay = state.relay.read().await;
        (relay.enabled, relay.updated_at)
    };
    let (
        pending_reconnect,
        pending_restart,
        vpn,
        relay_agent_enabled,
        bridge_running,
        bridge_config_updates,
        operations,
        runtime,
    ) = {
        let runtime = state.runtime.read().await;
        (
            runtime.application_reconnect_pending,
            runtime.application_restart_requested,
            application_vpn_json(&runtime, profile),
            runtime.relay_agent_enabled,
            runtime.bridge_running,
            runtime.bridge_config_updates,
            serde_json::json!({
                "profileInvitesCreated": runtime.profile_invites_created,
                "cacheWarmRuns": runtime.cache_warm_runs,
                "backfillRuns": runtime.backfill_runs,
                "songidRuns": runtime.songid_runs,
                "lidarrSyncRuns": runtime.lidarr_sync_runs,
                "lidarrManualImports": runtime.lidarr_manual_imports,
            }),
            runtime.json_value(),
        )
    };
    let distributed_settings = *state.soulseek_distributed_settings.read().await;
    let distributed_network = state.distributed_network.read().await.application_json(
        distributed_settings,
        session.state == "connected",
        profile,
    );
    let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();

    ApplicationStateSnapshot {
        session,
        pending_reconnect,
        pending_restart,
        vpn,
        relay_enabled,
        relay_updated_at,
        relay_agent_enabled,
        bridge_running,
        bridge_config_updates,
        operations,
        runtime,
        distributed_network,
        shares,
        room_names,
        user_names,
        runtime_credentials_configured,
        connected_endpoint: crate::connected_server_address(state),
        version: crate::controller_version_json(state),
    }
}

pub(crate) async fn application_state_json_for_state(state: &AppState) -> String {
    application_state_json(application_state_snapshot(state).await, &state.config)
}

fn application_state_json(snapshot: ApplicationStateSnapshot, config: &AppConfig) -> String {
    let runtime_profile = if config.current_upstream_behavior {
        serde_json::Value::Null
    } else {
        serde_json::json!(config.controller_profile.as_str())
    };
    let version =
        crate::application_state_version_json(snapshot.version, config.controller_profile);
    serde_json::json!({
        "product": "slskR",
        "runtimeProfile": runtime_profile,
        "version": version,
        "pendingReconnect": snapshot.pending_reconnect,
        "pendingRestart": snapshot.pending_restart,
        "server": crate::controller_server_state_json(
            &snapshot.session,
            config,
            snapshot.runtime_credentials_configured,
            snapshot.connected_endpoint.as_deref(),
        ),
        "connectionWatchdog": {
            "enabled": config.reconnect,
            "reconnectDelaySeconds": config.reconnect_delay.as_secs(),
        },
        "vpn": snapshot.vpn,
        "health": {
            "search": {"incoming": {"latency": 0, "queueDepth": 0, "dropRate": 0}},
        },
        "relay": {
            "enabled": snapshot.relay_enabled,
            "agentEnabled": snapshot.relay_agent_enabled,
            "updated_at": snapshot.relay_updated_at,
        },
        "bridge": {
            "enabled": config.integrations.bridge.enabled,
            "running": snapshot.bridge_running,
            "configUpdates": snapshot.bridge_config_updates,
            "host": null,
            "port": null,
            "endpointConfigured": config.integrations.bridge.endpoint_configured(),
        },
        "operations": snapshot.operations,
        "runtime": snapshot.runtime,
        "user": {
            "username": snapshot.session.username,
            "privilegesSeconds": snapshot.session.privileges_seconds,
        },
        "distributedNetwork": snapshot.distributed_network,
        "shares": snapshot.shares,
        "rooms": snapshot.room_names,
        "users": snapshot.user_names,
    })
    .to_string()
}
