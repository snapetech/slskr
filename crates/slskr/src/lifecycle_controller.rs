use super::{cancel_active_share_scan, send_session_command, AppState, SessionCommand};
use std::time::Duration;
use tokio::time;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LifecycleCommand {
    Shutdown,
    Restart,
}

pub(super) const GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) fn schedule_lifecycle_command(state: &AppState, command: LifecycleCommand) {
    let Some(sender) = state.lifecycle_commands.clone() else {
        return;
    };
    state.managed_background_tasks.try_spawn(async move {
        // Let the HTTP response flush before the accept loop tears down the runtime.
        time::sleep(Duration::from_millis(100)).await;
        let _ = sender.send(command).await;
    });
}

/// Disconnects from the Soulseek server (best-effort) before scheduling
/// process shutdown, matching the oracle's `StopAsync` teardown
/// (`Client.Disconnect("Shutting down", ...)`) rather than exiting with the
/// session left connected.
pub(super) async fn initiate_graceful_shutdown(state: &AppState) {
    cancel_active_share_scan(state);
    // A full session command queue must not prevent the process from honoring
    // SIGTERM/SIGINT. The lifecycle command still closes the listeners after
    // the bounded best-effort disconnect window.
    let _ = time::timeout(
        GRACEFUL_SHUTDOWN_DISCONNECT_TIMEOUT,
        send_session_command(state, SessionCommand::Disconnect),
    )
    .await;
    schedule_lifecycle_command(state, LifecycleCommand::Shutdown);
}
