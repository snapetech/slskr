use slskr_protocol::{distributed::DistributedMessage, peer::PeerMessage, server::ServerMessage};

pub fn trace_server_message(direction: &str, message: &ServerMessage) {
    tracing::debug!(target: "soulseek.client.server", direction, ?message, "server message");
}

pub fn trace_peer_message(username: &str, direction: &str, message: &PeerMessage) {
    tracing::debug!(
        target: "soulseek.client.peer",
        username,
        direction,
        ?message,
        "peer message"
    );
}

pub fn trace_distributed_message(username: &str, direction: &str, message: &DistributedMessage) {
    tracing::debug!(
        target: "soulseek.client.distributed",
        username,
        direction,
        ?message,
        "distributed message"
    );
}
