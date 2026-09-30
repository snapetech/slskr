use super::{
    await_fixture_server_task, incoming_connection_name, normalize_command, peer_probe_messages,
    redact_peer_text, scrub_socket_addr, validated_duration_secs, validated_obfuscated_port,
};
use slskr_client::{
    listener::IncomingConnection, protocol::server::ServerMessage, stream::PeerMessageConnection,
};
use std::ffi::OsString;
use std::{
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};
use tokio::io::duplex;

fn normalize(args: &[&str]) -> Vec<String> {
    normalize_command(args.iter().map(OsString::from)).unwrap()
}

#[test]
fn grouped_commands_map_to_internal_runner_names() {
    assert_eq!(normalize(&["login", "smoke"]), ["login-smoke"]);
    assert_eq!(normalize(&["soak", "live"]), ["live-soak"]);
    assert_eq!(normalize(&["smoke", "local-peer"]), ["local-peer-smoke"]);
    assert_eq!(
        normalize(&["probe", "obfuscated-peer"]),
        ["obfuscated-peer-probe"]
    );
    assert_eq!(
        normalize(&["probe", "overlay-service"]),
        ["overlay-service-probe"]
    );
    assert_eq!(normalize(&["probe", "dht-store"]), ["dht-store-probe"]);
    assert_eq!(
        normalize(&["probe", "wishlist-interval"]),
        ["wishlist-interval-probe"]
    );
    assert_eq!(normalize(&["probe", "user-watch"]), ["user-watch-probe"]);
    assert_eq!(
        normalize(&["smoke", "distributed-tree"]),
        ["distributed-tree-smoke"]
    );
    assert_eq!(normalize(&["smoke", "room-create"]), ["room-create-smoke"]);
    assert_eq!(
        normalize(&["smoke", "server-relogin"]),
        ["server-relogin-smoke"]
    );
    assert_eq!(
        normalize(&["smoke", "server-reconnect"]),
        ["server-reconnect-smoke"]
    );
    assert_eq!(
        normalize(&["smoke", "closed-listener"]),
        ["closed-listener-smoke"]
    );
    assert_eq!(
        normalize(&["smoke", "bad-obfuscation-type"]),
        ["bad-obfuscation-type-smoke"]
    );
    assert_eq!(
        normalize(&["smoke", "malformed-peer-response"]),
        ["malformed-peer-response-smoke"]
    );
}

#[test]
fn internal_runner_names_still_pass_through() {
    assert_eq!(normalize(&["login-smoke"]), ["login-smoke"]);
    assert_eq!(normalize(&["plain-peer-probe"]), ["plain-peer-probe"]);
}

#[test]
fn peer_probe_messages_target_same_user() {
    let messages = peer_probe_messages("peer");
    assert!(matches!(
        &messages[0],
        ServerMessage::WatchUserRequest { username } if username == "peer"
    ));
    assert!(matches!(
        &messages[3],
        ServerMessage::GetPeerAddressRequest { username } if username == "peer"
    ));
}

#[test]
fn scrub_socket_addr_hides_host_address() {
    let address = SocketAddr::from((Ipv4Addr::new(192, 0, 2, 10), 2234));
    assert_eq!(scrub_socket_addr(address), "ipv4:2234");
}

#[test]
fn peer_text_redaction_removes_terminal_controls() {
    let redacted = redact_peer_text("rejected\n\x1b[31mforged");
    assert_eq!(redacted, "len20");
    assert!(!redacted.chars().any(char::is_control));
}

#[test]
fn duration_validation_rejects_zero_and_unrepresentable_timers() {
    let zero =
        validated_duration_secs("TEST_SECONDS", 0, false).expect_err("zero interval should fail");
    assert!(zero.contains("greater than zero"), "{zero}");

    let oversized = validated_duration_secs("TEST_SECONDS", u64::MAX, false)
        .expect_err("unrepresentable interval should fail");
    assert!(oversized.contains("timer range"), "{oversized}");

    assert_eq!(
        validated_duration_secs("TEST_SECONDS", 0, true).unwrap(),
        Duration::ZERO
    );
}

#[test]
fn obfuscated_port_validation_rejects_unsupported_and_missing_endpoints() {
    assert_eq!(validated_obfuscated_port(1, 2235).unwrap(), 2235);
    assert!(validated_obfuscated_port(2, 2235)
        .unwrap_err()
        .contains("unsupported obfuscation type"));
    assert!(validated_obfuscated_port(1, 0)
        .unwrap_err()
        .contains("did not advertise"));
}

#[test]
fn incoming_connection_names_are_stable() {
    let (stream, _) = duplex(8);
    let incoming = IncomingConnection::PeerMessages(PeerMessageConnection::new(stream));
    assert_eq!(incoming_connection_name(&incoming), "peer_messages");
}

#[tokio::test]
async fn fixture_server_task_errors_fail_the_probe() {
    let failed = tokio::spawn(async { Err("fixture send failed".to_owned()) });
    assert_eq!(
        await_fixture_server_task(failed, "reject")
            .await
            .expect_err("fixture error must propagate"),
        "fixture send failed"
    );

    let completed = tokio::spawn(async { Ok(()) });
    await_fixture_server_task(completed, "resume")
        .await
        .expect("successful fixture task");
}
