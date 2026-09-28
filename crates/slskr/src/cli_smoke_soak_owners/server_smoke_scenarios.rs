use super::*;

pub(in crate::cli) async fn distributed_tree_smoke() -> Result<(), String> {
    let timeout = Duration::from_secs(5);
    let listener = Listener::bind("127.0.0.1:0")
        .await
        .map_err(|error| format!("distributed listener bind failed: {error}"))?;
    let address = listener
        .local_addr()
        .map_err(|error| format!("distributed listener address failed: {error}"))?;
    let server_task = tokio::spawn(async move {
        let (incoming, _) = listener
            .accept()
            .await
            .map_err(|error| format!("distributed listener accept failed: {error}"))?;
        let IncomingConnection::PeerInit {
            kind: ConnectionKind::Distributed,
            stream,
            ..
        } = incoming
        else {
            return Err("distributed listener received the wrong connection kind".to_owned());
        };
        let mut connection = DistributedConnection::new(stream);
        let message = connection
            .receive()
            .await
            .map_err(|error| format!("distributed listener receive failed: {error}"))?;
        if message != DistributedMessage::Ping {
            return Err("distributed listener did not receive Ping".to_owned());
        }
        for response in [
            DistributedMessage::BranchLevel { level: 1 },
            DistributedMessage::BranchRoot {
                username: "distributed-fixture-root".to_owned(),
            },
            DistributedMessage::PingResponse { token: 1 },
        ] {
            connection
                .send(&response)
                .await
                .map_err(|error| format!("distributed listener response failed: {error}"))?;
        }
        Ok(())
    });

    let stream = TcpStream::connect(address)
        .await
        .map_err(|error| format!("distributed fixture connect failed: {error}"))?;
    let stream = send_peer_init(
        stream,
        "slskr-distributed-fixture",
        ConnectionKind::Distributed,
    )
    .await
    .map_err(|error| format!("distributed fixture init failed: {error}"))?;
    let mut connection = DistributedConnection::new(stream);
    connection
        .send(&DistributedMessage::Ping)
        .await
        .map_err(|error| format!("distributed fixture ping failed: {error}"))?;
    let (received_branch_level, received_branch_root) =
        receive_distributed_ping(&mut connection, timeout)
            .await
            .map_err(|error| format!("distributed fixture response failed: {error}"))?;
    if !received_branch_level || !received_branch_root {
        return Err("distributed fixture did not send complete branch metadata".to_owned());
    }
    time::timeout(timeout, server_task)
        .await
        .map_err(|_| "distributed listener task timed out".to_owned())?
        .map_err(|error| format!("distributed listener task failed: {error}"))??;

    let (parent_side, parent_peer) = duplex(2_048);
    let mut tree = DistributedTree::new("local");
    let mut parent_peer = DistributedConnection::new(parent_peer);
    tree.connect_parent(
        ParentInfo {
            username: "parent".to_owned(),
            ip: Ipv4Addr::LOCALHOST,
            port: u32::from(address.port()),
        },
        DistributedConnection::new(parent_side),
    );
    if tree.branch_level() != 1 || tree.branch_root() != "parent" || tree.parent().is_none() {
        return Err("distributed parent adoption did not update branch state".to_owned());
    }
    if !tree
        .send_branch_info_to_parent()
        .await
        .map_err(|error| format!("distributed branch report failed: {error}"))?
    {
        return Err("distributed branch report did not reach the parent".to_owned());
    }
    for expected in [
        DistributedMessage::BranchLevel { level: 1 },
        DistributedMessage::BranchRoot {
            username: "parent".to_owned(),
        },
        DistributedMessage::ChildDepth { depth: 0 },
    ] {
        let received = time::timeout(timeout, parent_peer.receive())
            .await
            .map_err(|_| "distributed parent report timed out".to_owned())?
            .map_err(|error| format!("distributed parent report receive failed: {error}"))?;
        if received != expected {
            return Err("distributed parent received incorrect branch metadata".to_owned());
        }
    }

    let (first_tree, first_peer) = duplex(2_048);
    let (second_tree, second_peer) = duplex(2_048);
    let mut first_peer = DistributedConnection::new(first_peer);
    let mut second_peer = DistributedConnection::new(second_peer);
    tree.add_child("first", DistributedConnection::new(first_tree))
        .map_err(|error| format!("distributed first child failed: {error}"))?;
    tree.add_child("second", DistributedConnection::new(second_tree))
        .map_err(|error| format!("distributed second child failed: {error}"))?;
    let search = DistributedSearch {
        identifier: 49,
        username: "origin".to_owned(),
        token: 0x51ab_5001,
        query: "distributed fixture".to_owned(),
    };
    let forwarded = tree
        .forward_search_to_children(&search, Some("first"))
        .await
        .map_err(|error| format!("distributed search forwarding failed: {error}"))?;
    if forwarded != 1 {
        return Err(format!(
            "distributed search reached {forwarded} children instead of one"
        ));
    }
    let received = time::timeout(timeout, second_peer.receive())
        .await
        .map_err(|_| "distributed child search timed out".to_owned())?
        .map_err(|error| format!("distributed child search receive failed: {error}"))?;
    if received != DistributedMessage::Search(search) {
        return Err("distributed child received incorrect search payload".to_owned());
    }
    if time::timeout(Duration::from_millis(25), first_peer.receive())
        .await
        .is_ok()
    {
        return Err("distributed search was reflected to its source child".to_owned());
    }
    if tree.handle_child_message("second", DistributedMessage::ChildDepth { depth: 3 })
        != DistributedEvent::BranchChanged
        || tree.child_info("second").map(|child| child.depth) != Some(3)
    {
        return Err("distributed child depth was not tracked".to_owned());
    }
    if tree.remove_child("SECOND").is_none() || tree.child_info("second").is_some() {
        return Err("distributed child disconnect was not handled".to_owned());
    }

    emit_and_result(
        ProbeContext::new("distributed-tree").ok(
            "Ping round-trip, parent adoption, search forwarding, and child lifecycle completed",
        ),
    )
}

pub(in crate::cli) async fn room_create_smoke() -> Result<(), String> {
    let (client, server) = duplex(1_024);
    let mut client = ServerConnection::new(client);
    let mut server = ServerConnection::new(server);
    let room = "slskr-room-create-fixture".to_owned();

    client
        .send(&ServerMessage::JoinRoom {
            room: room.clone(),
            private: false,
        })
        .await
        .map_err(|error| format!("room-create request send failed: {error}"))?;
    let request = server
        .receive_with_direction(slskr_client::protocol::server::Direction::ClientToServer)
        .await
        .map_err(|error| format!("room-create request decode failed: {error}"))?;
    if request
        != (ServerMessage::JoinRoom {
            room: room.clone(),
            private: false,
        })
    {
        return Err("room-create request did not preserve public/private intent".to_owned());
    }

    server
        .send(&ServerMessage::JoinedRoom(JoinedRoom {
            room: room.clone(),
            users: Vec::new(),
            owner: None,
            operators: Vec::new(),
        }))
        .await
        .map_err(|error| format!("room-create response send failed: {error}"))?;
    match client
        .receive_with_direction(slskr_client::protocol::server::Direction::ServerToClient)
        .await
        .map_err(|error| format!("room-create response decode failed: {error}"))?
    {
        ServerMessage::JoinedRoom(joined) if joined.room == room => {}
        _ => return Err("room-create response was not decoded as JoinedRoom".to_owned()),
    }

    for rejection in [
        ServerMessage::CantCreateRoom { room: room.clone() },
        ServerMessage::CantJoinRoom { room: room.clone() },
    ] {
        server
            .send(&rejection)
            .await
            .map_err(|error| format!("room rejection send failed: {error}"))?;
        let decoded = client
            .receive_with_direction(slskr_client::protocol::server::Direction::ServerToClient)
            .await
            .map_err(|error| format!("room rejection decode failed: {error}"))?;
        if decoded != rejection {
            return Err("room rejection code changed during round-trip".to_owned());
        }
    }

    emit_and_result(
        ProbeContext::new("room-create")
            .ok("public join, JoinedRoom, CannotCreateRoom, and CannotJoinRoom completed"),
    )
}

pub(in crate::cli) async fn server_relogin_smoke() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let timeout = env_duration_secs("SLSKR_RELOGIN_TIMEOUT_SECONDS", 15, false)?;
    let ctx = ProbeContext::new("server-relogin").with_peer(&username);

    let mut first =
        login_probe_session(&server_address, username.clone(), password.clone()).await?;
    let _second = login_probe_session(&server_address, username, password).await?;
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return emit_and_result(ctx.fail("first session did not receive Relogged"));
        }
        match time::timeout(remaining, first.receive()).await {
            Ok(Ok(ServerMessage::Relogged)) => {
                return emit_and_result(ctx.ok("first session received Relogged"));
            }
            Ok(Ok(ServerMessage::MessageUserResponse(message))) => {
                first
                    .send_server_message(ServerMessage::MessageAcked { id: message.id })
                    .await
                    .map_err(|error| format!("relogin message acknowledgement failed: {error}"))?;
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                return Err(format!("first relogin session receive failed: {error}"));
            }
            Err(_) => return emit_and_result(ctx.fail("first session did not receive Relogged")),
        }
    }
}

pub(in crate::cli) async fn server_reconnect_smoke() -> Result<(), String> {
    let username = required_env_any(&["SLSK_USERNAME"])?;
    let password = required_env_any(&["SLSK_PASSWORD"])?;
    let server_address =
        std::env::var("SLSK_SERVER").unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned());
    let delay = env_duration_secs("SLSKR_RECONNECT_DELAY_SECONDS", 2, true)?;
    let attempts = env_usize("SLSKR_RECONNECT_ATTEMPTS", 3)?;
    let ctx = ProbeContext::new("server-reconnect").with_peer(&username);

    let first = login_probe_session(&server_address, username.clone(), password.clone()).await?;
    drop(first);
    time::sleep(delay).await;

    let mut last_error = String::new();
    for attempt in 1..=attempts {
        match login_probe_session(&server_address, username.clone(), password.clone()).await {
            Ok(mut reconnected) => {
                reconnected
                    .send_ping()
                    .await
                    .map_err(|error| format!("reconnected session ping failed: {error}"))?;
                return emit_and_result(ctx.ok(format!(
                    "session reconnected and pinged on attempt {attempt}"
                )));
            }
            Err(error) => last_error = error,
        }
        time::sleep(delay).await;
    }
    emit_and_result(ctx.fail(format!(
        "session did not reconnect after {attempts} attempts: {last_error}"
    )))
}
