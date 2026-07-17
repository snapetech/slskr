use std::{
    net::Ipv4Addr,
    time::{Duration, Instant},
};

use slskr_client::{
    distributed_tree::{
        BranchInfoReporter, DistributedEvent, DistributedTree, ParentInfo,
        MAX_DISTRIBUTED_USERNAME_BYTES,
    },
    search::MAX_OUTBOUND_SEARCH_FIELD_BYTES,
    server::ServerSession,
    stream::{DistributedConnection, ServerConnection},
    ClientError,
};
use slskr_protocol::{
    distributed::{DistributedMessage, DistributedSearch},
    server::{Direction, PossibleParent, ServerMessage},
};
use tokio::io::duplex;

#[test]
fn choose_parent_ignores_self_and_invalid_ports_then_picks_stable_candidate() {
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let candidates = vec![
        possible_parent("LOCAL", [10, 0, 0, 1], 2234),
        possible_parent("zero", [10, 0, 0, 2], 0),
        possible_parent("overflow", [10, 0, 0, 5], u32::from(u16::MAX) + 1),
        possible_parent("unspecified", [0, 0, 0, 0], 2234),
        possible_parent("multicast", [224, 0, 0, 1], 2234),
        possible_parent("broadcast", [255, 255, 255, 255], 2234),
        possible_parent("zoe", [10, 0, 0, 4], 2234),
        possible_parent("alice", [10, 0, 0, 3], 2234),
    ];

    assert_eq!(
        tree.choose_parent(&candidates),
        Some(ParentInfo {
            username: "alice".to_owned(),
            ip: Ipv4Addr::new(10, 0, 0, 3),
            port: 2234,
        })
    );
}

#[test]
fn choose_parent_ignores_oversized_identity() {
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let oversized = "a".repeat(MAX_DISTRIBUTED_USERNAME_BYTES + 1);
    let candidates = vec![
        possible_parent(&oversized, [10, 0, 0, 1], 2234),
        possible_parent("valid", [10, 0, 0, 2], 2234),
    ];

    assert_eq!(tree.choose_parent(&candidates).unwrap().username, "valid");
}

#[test]
fn choose_parent_ignores_blank_identity() {
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let candidates = vec![
        possible_parent("   ", [10, 0, 0, 1], 2234),
        possible_parent("valid", [10, 0, 0, 2], 2234),
    ];

    assert_eq!(tree.choose_parent(&candidates).unwrap().username, "valid");
}

#[test]
fn choose_parent_ignores_control_characters_in_identity() {
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let candidates = vec![
        possible_parent("forged\r\nparent", [10, 0, 0, 1], 2234),
        possible_parent("valid", [10, 0, 0, 2], 2234),
    ];

    assert_eq!(tree.choose_parent(&candidates).unwrap().username, "valid");
}

#[test]
fn choose_parent_canonicalizes_identities_before_rejecting_self() {
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new(" local ");
    let candidates = vec![
        possible_parent(" LOCAL ", [10, 0, 0, 1], 2234),
        possible_parent(" valid ", [10, 0, 0, 2], 2234),
    ];

    assert_eq!(tree.local_username(), "local");
    assert_eq!(tree.choose_parent(&candidates).unwrap().username, "valid");
}

#[test]
fn parent_state_tracks_connect_disconnect_reset_and_server_reports() {
    let (tree_side, _peer_side) = duplex(512);
    let mut tree = DistributedTree::new("local");

    assert_eq!(
        tree.have_no_parent_message(),
        ServerMessage::HaveNoParent { no_parent: true }
    );
    assert_eq!(
        tree.set_accepting_children(true),
        ServerMessage::AcceptChildren { accept: true }
    );
    assert!(tree.accepting_children());

    tree.connect_parent(
        parent_info("parent", [10, 0, 0, 2], 2234),
        DistributedConnection::new(tree_side),
    );

    assert_eq!(tree.parent().unwrap().username, "parent");
    assert_eq!(tree.branch_level(), 1);
    assert_eq!(tree.branch_root(), "parent");
    assert_eq!(
        tree.have_no_parent_message(),
        ServerMessage::HaveNoParent { no_parent: false }
    );
    assert_eq!(
        tree.branch_server_messages(),
        [
            ServerMessage::BranchLevel { level: 1 },
            ServerMessage::BranchRoot {
                username: "parent".to_owned(),
            },
        ]
    );

    tree.disconnect_parent();
    assert!(tree.parent().is_none());
    assert_eq!(tree.branch_level(), 0);
    assert_eq!(tree.branch_root(), "local");

    let (tree_side, _peer_side) = duplex(512);
    tree.connect_parent(
        parent_info("parent", [10, 0, 0, 2], 2234),
        DistributedConnection::new(tree_side),
    );
    tree.reset();
    assert!(tree.parent().is_none());
    assert_eq!(tree.children_len(), 0);
    assert!(!tree.accepting_children());
}

#[test]
fn connect_parent_rejects_invalid_identity_or_endpoint_without_mutating_branch_state() {
    let mut tree = DistributedTree::new("local");

    for (username, ip, port) in [
        ("   ".to_owned(), [10, 0, 0, 2], 2234),
        ("forged\r\nparent".to_owned(), [10, 0, 0, 2], 2234),
        (
            "x".repeat(MAX_DISTRIBUTED_USERNAME_BYTES + 1),
            [10, 0, 0, 2],
            2234,
        ),
        (" LOCAL ".to_owned(), [10, 0, 0, 2], 2234),
        ("parent".to_owned(), [10, 0, 0, 2], 0),
        ("parent".to_owned(), [10, 0, 0, 2], u32::from(u16::MAX) + 1),
        ("parent".to_owned(), [0, 0, 0, 0], 2234),
        ("parent".to_owned(), [224, 0, 0, 1], 2234),
        ("parent".to_owned(), [255, 255, 255, 255], 2234),
    ] {
        let (tree_side, _peer_side) = duplex(64);
        tree.connect_parent(
            parent_info(&username, ip, port),
            DistributedConnection::new(tree_side),
        );

        assert!(tree.parent().is_none());
        assert_eq!(tree.branch_level(), 0);
        assert_eq!(tree.branch_root(), "local");
    }
}

#[test]
fn parent_messages_update_branch_state_and_surface_searches() {
    let mut tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let search = distributed_search(7, "remote", 99, "rare");

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::BranchLevel { level: 3 }),
        DistributedEvent::BranchChanged
    );
    assert_eq!(tree.branch_level(), 4);

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::BranchRoot {
            username: "root".to_owned(),
        }),
        DistributedEvent::BranchChanged
    );
    assert_eq!(tree.branch_root(), "root");

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::Search(search.clone())),
        DistributedEvent::Search(search)
    );
}

#[test]
fn parent_oversized_branch_root_is_ignored_without_mutating_state() {
    let mut tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let oversized = "x".repeat(MAX_DISTRIBUTED_USERNAME_BYTES + 1);

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::BranchRoot {
            username: oversized,
        }),
        DistributedEvent::Ignored
    );
    assert_eq!(tree.branch_root(), "local");
}

#[test]
fn parent_blank_branch_root_is_ignored_without_mutating_state() {
    let mut tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::BranchRoot {
            username: "  ".to_owned(),
        }),
        DistributedEvent::Ignored
    );
    assert_eq!(tree.branch_root(), "local");
}

#[test]
fn parent_control_character_branch_root_is_ignored_without_mutating_state() {
    let mut tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::BranchRoot {
            username: "forged\nroot".to_owned(),
        }),
        DistributedEvent::Ignored
    );
    assert_eq!(tree.branch_root(), "local");
}

#[test]
fn parent_local_branch_root_is_ignored_without_mutating_state() {
    let mut tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");

    assert_eq!(
        tree.handle_parent_message(DistributedMessage::BranchRoot {
            username: " LOCAL ".to_owned(),
        }),
        DistributedEvent::Ignored
    );
    assert_eq!(tree.branch_root(), "local");
}

#[test]
fn child_depth_updates_local_child_depth() {
    let (child_a, _peer_a) = duplex(512);
    let (child_b, _peer_b) = duplex(512);
    let mut tree = DistributedTree::new("local");
    tree.add_child("alice", DistributedConnection::new(child_a))
        .unwrap();
    tree.add_child("bob", DistributedConnection::new(child_b))
        .unwrap();

    assert_eq!(tree.child_depth(), 1);
    assert_eq!(
        tree.handle_child_message("alice", DistributedMessage::ChildDepth { depth: 4 }),
        DistributedEvent::BranchChanged
    );
    assert_eq!(tree.child_info("alice").unwrap().depth, 4);
    assert_eq!(tree.child_depth(), 5);
    assert_eq!(
        tree.handle_child_message("missing", DistributedMessage::ChildDepth { depth: 9 }),
        DistributedEvent::Ignored
    );
}

#[test]
fn messages_from_unknown_children_are_ignored() {
    let (child, _peer) = duplex(512);
    let mut tree = DistributedTree::new("local");
    tree.add_child("alice", DistributedConnection::new(child))
        .unwrap();
    let search = distributed_search(1, "mallory", 2, "spoofed");

    assert_eq!(
        tree.handle_child_message("missing", DistributedMessage::Ping),
        DistributedEvent::Ignored
    );
    assert_eq!(
        tree.handle_child_message("missing", DistributedMessage::Search(search.clone())),
        DistributedEvent::Ignored
    );
    assert_eq!(
        tree.handle_child_message(" ALICE ", DistributedMessage::Search(search.clone())),
        DistributedEvent::Search(search)
    );
}

#[test]
fn malformed_distributed_searches_are_ignored_at_ingress() {
    let (child, _peer) = duplex(512);
    let mut tree = DistributedTree::new("local");
    tree.add_child("alice", DistributedConnection::new(child))
        .unwrap();

    for search in [
        distributed_search(1, "   ", 2, "query"),
        distributed_search(1, "remote", 2, "   "),
        distributed_search(
            1,
            &"u".repeat(MAX_OUTBOUND_SEARCH_FIELD_BYTES + 1),
            2,
            "query",
        ),
        distributed_search(
            1,
            "remote",
            2,
            &"q".repeat(MAX_OUTBOUND_SEARCH_FIELD_BYTES + 1),
        ),
    ] {
        assert_eq!(
            tree.handle_parent_message(DistributedMessage::Search(search.clone())),
            DistributedEvent::Ignored
        );
        assert_eq!(
            tree.handle_child_message("alice", DistributedMessage::Search(search)),
            DistributedEvent::Ignored
        );
    }
}

#[tokio::test]
async fn malformed_distributed_searches_are_rejected_before_forwarding() {
    let (child, mut peer) = duplex(16 * 1024);
    let mut tree = DistributedTree::new("local");
    tree.add_child("alice", DistributedConnection::new(child))
        .unwrap();
    let search = distributed_search(
        1,
        "remote",
        2,
        &"q".repeat(MAX_OUTBOUND_SEARCH_FIELD_BYTES + 1),
    );

    assert!(matches!(
        tree.forward_search_to_children(&search, None)
            .await
            .unwrap_err(),
        ClientError::SearchFieldTooLong {
            field: "distributed search query",
            length,
            max: MAX_OUTBOUND_SEARCH_FIELD_BYTES,
        } if length == MAX_OUTBOUND_SEARCH_FIELD_BYTES + 1
    ));
    use tokio::io::AsyncReadExt as _;
    let mut byte = [0];
    assert!(
        tokio::time::timeout(Duration::from_millis(10), peer.read(&mut byte))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn branch_info_is_sent_to_parent_as_distributed_messages() {
    let (tree_side, parent_side) = duplex(1024);
    let (child_side, _child_peer) = duplex(512);
    let mut tree = DistributedTree::new("local");
    let mut parent = DistributedConnection::new(parent_side);

    tree.connect_parent(
        parent_info("parent", [10, 0, 0, 2], 2234),
        DistributedConnection::new(tree_side),
    );
    tree.add_child("child", DistributedConnection::new(child_side))
        .unwrap();
    tree.handle_child_message("child", DistributedMessage::ChildDepth { depth: 2 });

    assert!(tree.send_branch_info_to_parent().await.unwrap());
    assert_eq!(
        parent.receive().await.unwrap(),
        DistributedMessage::BranchLevel { level: 1 }
    );
    assert_eq!(
        parent.receive().await.unwrap(),
        DistributedMessage::BranchRoot {
            username: "parent".to_owned(),
        }
    );
    assert_eq!(
        parent.receive().await.unwrap(),
        DistributedMessage::ChildDepth { depth: 3 }
    );
}

#[tokio::test]
async fn failed_parent_write_disconnects_and_resets_branch_state() {
    let (tree_side, parent_side) = duplex(64);
    let mut tree = DistributedTree::new("local");
    tree.connect_parent(
        parent_info("parent", [10, 0, 0, 2], 2234),
        DistributedConnection::new(tree_side),
    );
    drop(parent_side);

    assert!(tree.send_branch_info_to_parent().await.is_err());
    assert!(tree.parent().is_none());
    assert_eq!(tree.branch_level(), 0);
    assert_eq!(tree.branch_root(), "local");
    assert_eq!(
        tree.have_no_parent_message(),
        ServerMessage::HaveNoParent { no_parent: true }
    );
}

#[tokio::test]
async fn distributed_searches_forward_to_children_except_source() {
    let (tree_a, peer_a) = duplex(1024);
    let (tree_b, peer_b) = duplex(1024);
    let mut tree = DistributedTree::new("local");
    let mut peer_a = DistributedConnection::new(peer_a);
    let mut peer_b = DistributedConnection::new(peer_b);
    let search = distributed_search(5, "origin", 44, "album");

    tree.add_child("alice", DistributedConnection::new(tree_a))
        .unwrap();
    tree.add_child("bob", DistributedConnection::new(tree_b))
        .unwrap();

    assert_eq!(
        tree.forward_search_to_children(&search, Some("alice"))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        peer_b.receive().await.unwrap(),
        DistributedMessage::Search(search)
    );

    let timed_out = tokio::time::timeout(Duration::from_millis(25), peer_a.receive()).await;
    assert!(timed_out.is_err());
}

#[test]
fn distributed_tree_rejects_new_children_at_limit_but_allows_replacement() {
    let mut tree = DistributedTree::with_max_children("local", 1);
    let (first, _) = duplex(64);
    assert!(!tree
        .add_child("first", DistributedConnection::new(first))
        .unwrap());

    let (replacement, _) = duplex(64);
    assert!(tree
        .add_child("first", DistributedConnection::new(replacement))
        .unwrap());

    let (second, _) = duplex(64);
    let error = tree
        .add_child("second", DistributedConnection::new(second))
        .unwrap_err();
    assert!(matches!(
        error,
        slskr_client::ClientError::DistributedChildCapacityFull { max: 1 }
    ));
    assert_eq!(tree.children_len(), 1);
    assert!(tree.child_info("first").is_some());
}

#[test]
fn distributed_tree_treats_child_username_casing_as_one_identity() {
    let mut tree = DistributedTree::with_max_children("local", 1);
    let (first, _) = duplex(64);
    tree.add_child("Alice", DistributedConnection::new(first))
        .unwrap();

    let (replacement, _) = duplex(64);
    assert!(tree
        .add_child("alice", DistributedConnection::new(replacement))
        .unwrap());
    assert_eq!(tree.children_len(), 1);
    assert_eq!(tree.child_info("ALICE").unwrap().username, "alice");

    assert_eq!(
        tree.handle_child_message("aLiCe", DistributedMessage::ChildDepth { depth: 3 }),
        DistributedEvent::BranchChanged
    );
    assert_eq!(tree.child_info("alice").unwrap().depth, 3);
    assert_eq!(tree.remove_child("ALICE").unwrap().username, "alice");
    assert_eq!(tree.children_len(), 0);
}

#[test]
fn distributed_tree_treats_surrounding_whitespace_as_one_child_identity() {
    let mut tree = DistributedTree::with_max_children("local", 1);
    let (first, _) = duplex(64);
    tree.add_child(" Alice ", DistributedConnection::new(first))
        .unwrap();

    let (replacement, _) = duplex(64);
    assert!(tree
        .add_child("alice", DistributedConnection::new(replacement))
        .unwrap());
    assert_eq!(tree.children_len(), 1);
    assert_eq!(tree.child_info(" ALICE ").unwrap().username, "alice");
    assert_eq!(tree.remove_child(" alice ").unwrap().username, "alice");
}

#[test]
fn distributed_tree_rejects_oversized_child_username_without_storing_it() {
    let mut tree = DistributedTree::new("local");
    let (child, _) = duplex(64);
    let oversized = "x".repeat(MAX_DISTRIBUTED_USERNAME_BYTES + 1);

    let error = tree
        .add_child(&oversized, DistributedConnection::new(child))
        .unwrap_err();

    assert!(matches!(
        error,
        ClientError::DistributedUsernameTooLong { length, max }
            if length == MAX_DISTRIBUTED_USERNAME_BYTES + 1
                && max == MAX_DISTRIBUTED_USERNAME_BYTES
    ));
    assert_eq!(tree.children_len(), 0);
}

#[test]
fn distributed_tree_rejects_blank_child_username_without_storing_it() {
    let mut tree = DistributedTree::new("local");
    let (child, _) = duplex(64);

    let error = tree
        .add_child("  ", DistributedConnection::new(child))
        .unwrap_err();

    assert!(matches!(error, ClientError::BlankDistributedUsername));
    assert_eq!(tree.children_len(), 0);
}

#[test]
fn distributed_tree_rejects_control_characters_in_child_username() {
    let mut tree = DistributedTree::new("local");
    let (child, _) = duplex(64);

    let error = tree
        .add_child("forged\nchild", DistributedConnection::new(child))
        .unwrap_err();

    assert!(matches!(error, ClientError::InvalidDistributedUsername));
    assert_eq!(tree.children_len(), 0);
}

#[test]
fn distributed_tree_rejects_local_identity_as_child() {
    let mut tree = DistributedTree::new("local");
    let (child, _) = duplex(64);

    assert!(matches!(
        tree.add_child(" LOCAL ", DistributedConnection::new(child)),
        Err(ClientError::DistributedIdentityLoop { username }) if username == "LOCAL"
    ));
    assert_eq!(tree.children_len(), 0);
}

#[test]
fn distributed_tree_rejects_parent_identity_as_child() {
    let mut tree = DistributedTree::new("local");
    let (parent, _) = duplex(64);
    tree.connect_parent(
        parent_info("parent", [10, 0, 0, 1], 2234),
        DistributedConnection::new(parent),
    );
    let (child, _) = duplex(64);

    assert!(matches!(
        tree.add_child("PARENT", DistributedConnection::new(child)),
        Err(ClientError::DistributedIdentityLoop { username }) if username == "PARENT"
    ));
    assert_eq!(tree.children_len(), 0);
}

#[test]
fn distributed_tree_rejects_existing_child_as_parent() {
    let mut tree = DistributedTree::new("local");
    let (child, _) = duplex(64);
    tree.add_child("peer", DistributedConnection::new(child))
        .unwrap();
    let (parent, _) = duplex(64);

    tree.connect_parent(
        parent_info("PEER", [10, 0, 0, 1], 2234),
        DistributedConnection::new(parent),
    );

    assert!(tree.parent().is_none());
    assert_eq!(tree.branch_level(), 0);
    assert_eq!(tree.branch_root(), "local");
    assert!(tree.child_info("peer").is_some());
}

#[tokio::test]
async fn distributed_search_source_exclusion_is_case_insensitive() {
    let (tree_side, peer_side) = duplex(1024);
    let mut tree = DistributedTree::new("local");
    let mut peer = DistributedConnection::new(peer_side);
    let search = distributed_search(5, "origin", 44, "album");
    tree.add_child("Alice", DistributedConnection::new(tree_side))
        .unwrap();

    assert_eq!(
        tree.forward_search_to_children(&search, Some("ALICE"))
            .await
            .unwrap(),
        0
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(25), peer.receive())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn distributed_forwarding_evicts_failed_children_without_blocking_healthy_children() {
    let (failed_tree, failed_peer) = duplex(1024);
    let (healthy_tree, healthy_peer) = duplex(1024);
    let mut tree = DistributedTree::new("local");
    let mut healthy_peer = DistributedConnection::new(healthy_peer);
    let search = distributed_search(5, "origin", 44, "album");
    tree.add_child("failed", DistributedConnection::new(failed_tree))
        .unwrap();
    tree.add_child("healthy", DistributedConnection::new(healthy_tree))
        .unwrap();
    drop(failed_peer);

    assert!(tree
        .forward_search_to_children(&search, None)
        .await
        .is_err());
    assert_eq!(
        healthy_peer.receive().await.unwrap(),
        DistributedMessage::Search(search)
    );
    assert!(tree.child_info("failed").is_none());
    assert!(tree.child_info("healthy").is_some());
    assert_eq!(tree.children_len(), 1);
}

#[tokio::test]
async fn distributed_forwarding_times_out_and_evicts_stalled_child() {
    let (stalled_tree, _non_reading_child) = duplex(1);
    let mut tree = DistributedTree::new("local");
    let search = distributed_search(5, "origin", 44, &"x".repeat(4_096));
    tree.add_child("stalled", DistributedConnection::new(stalled_tree))
        .unwrap();

    assert!(matches!(
        tree.forward_search_to_children_with_timeout(&search, None, Duration::from_millis(10),)
            .await,
        Err(ClientError::TimedOut {
            operation: "distributed search forwarding",
        })
    ));
    assert!(tree.child_info("stalled").is_none());
}

#[tokio::test]
async fn parent_reporting_times_out_and_disconnects_stalled_parent() {
    let (parent_tree, _non_reading_parent) = duplex(1);
    let mut tree = DistributedTree::new("local");
    tree.connect_parent(
        parent_info("parent", [10, 0, 0, 1], 2234),
        DistributedConnection::new(parent_tree),
    );

    assert!(matches!(
        tree.send_branch_info_to_parent_with_timeout(Duration::from_millis(10))
            .await,
        Err(ClientError::TimedOut {
            operation: "distributed parent send",
        })
    ));
    assert!(tree.parent().is_none());
}

#[tokio::test]
async fn branch_info_reporter_schedules_server_updates() {
    let now = Instant::now();
    let mut reporter = BranchInfoReporter::new(Duration::from_secs(5), now).unwrap();
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let (client, server) = duplex(1024);
    let mut session = ServerSession::new(ServerConnection::new(client));
    let mut server = ServerConnection::new(server);

    assert!(reporter
        .due_messages(now + Duration::from_secs(4), &tree)
        .is_none());

    let messages = reporter
        .due_messages(now + Duration::from_secs(5), &tree)
        .unwrap();
    assert_eq!(
        messages,
        [
            ServerMessage::BranchLevel { level: 0 },
            ServerMessage::BranchRoot {
                username: "local".to_owned(),
            },
        ]
    );

    tree.send_branch_info_to_server(&mut session).await.unwrap();
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::BranchLevel { level: 0 }
    );
    assert_eq!(
        server
            .receive_with_direction(Direction::ClientToServer)
            .await
            .unwrap(),
        ServerMessage::BranchRoot {
            username: "local".to_owned(),
        }
    );
}

#[test]
fn branch_info_reporter_rejects_zero_interval() {
    assert!(matches!(
        BranchInfoReporter::new(Duration::ZERO, Instant::now()),
        Err(ClientError::InvalidInterval {
            field: "branch info"
        })
    ));
}

#[test]
fn branch_info_reporter_rejects_interval_that_overflows_deadline() {
    assert!(matches!(
        BranchInfoReporter::new(Duration::MAX, Instant::now()),
        Err(ClientError::InvalidInterval {
            field: "branch info"
        })
    ));
}

#[test]
fn branch_info_reporter_catch_up_does_not_overflow_deadline() {
    let now = Instant::now();
    let interval = Duration::from_nanos(1);
    let mut reporter = BranchInfoReporter::new(interval, now).unwrap();
    let tree: DistributedTree<tokio::io::DuplexStream> = DistributedTree::new("local");
    let far_future = now.checked_add(Duration::MAX).unwrap_or_else(|| {
        let mut candidate = Duration::from_secs(u64::MAX / 2);
        while now.checked_add(candidate).is_none() {
            candidate /= 2;
        }
        now + candidate
    });

    assert!(reporter.due_messages(far_future, &tree).is_some());
    assert!(reporter.next_due() > far_future || reporter.next_due() == far_future);
}

fn possible_parent(username: &str, ip: [u8; 4], port: u32) -> PossibleParent {
    PossibleParent {
        username: username.to_owned(),
        ip: Ipv4Addr::from(ip),
        port,
    }
}

fn parent_info(username: &str, ip: [u8; 4], port: u32) -> ParentInfo {
    ParentInfo {
        username: username.to_owned(),
        ip: Ipv4Addr::from(ip),
        port,
    }
}

fn distributed_search(
    identifier: u32,
    username: &str,
    token: u32,
    query: &str,
) -> DistributedSearch {
    DistributedSearch {
        identifier,
        username: username.to_owned(),
        token,
        query: query.to_owned(),
    }
}
