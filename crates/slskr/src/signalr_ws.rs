//! SignalR JSON-hub compatibility for the frozen legacy and native web clients.
//!
//! The controller already exposes a separate raw event feed for slskR's native
//! UI.  The frozen clients use ASP.NET SignalR instead, so their hub protocol
//! needs to remain available at the original `/hub/*` routes as well.

use std::{collections::HashSet, sync::Arc};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{broadcast, mpsc},
};
use uuid::Uuid;

use crate::{relay_ws, routing, AppState, EventRecord};

const HUB_NAMES: [&str; 7] = [
    "application",
    "logs",
    "search",
    "metrics",
    "songid",
    "listening-party",
    "transfers",
];
const MAX_LISTENING_PARTY_GROUPS_PER_CONNECTION: usize = 64;
const MAX_LISTENING_PARTY_GROUP_COMPONENT_BYTES: usize = 1024;

/// Return the supported hub name for a normalized websocket path.
pub(crate) fn hub_name(path: &str) -> Option<&'static str> {
    let name = path.strip_prefix("/hub/")?;
    HUB_NAMES
        .iter()
        .copied()
        .find(|candidate| *candidate == name)
}

/// Return a hub only when the selected compatibility profile exposes it.
pub(crate) fn hub_name_for_target(
    path: &str,
    target: crate::config::ControllerProfile,
) -> Option<&'static str> {
    let hub = hub_name(path)?;
    let supported = match target {
        crate::config::ControllerProfile::Legacy => {
            matches!(hub, "application" | "logs" | "search" | "metrics")
        }
        crate::config::ControllerProfile::Native => {
            matches!(
                hub,
                "application" | "logs" | "search" | "songid" | "listening-party" | "transfers"
            )
        }
    };
    supported.then_some(hub)
}

/// Return the supported hub name for SignalR's POST negotiation route.
pub(crate) fn negotiate_hub_name(path: &str) -> Option<&'static str> {
    let name = path.strip_prefix("/hub/")?.strip_suffix("/negotiate")?;
    HUB_NAMES
        .iter()
        .copied()
        .find(|candidate| *candidate == name)
}

/// Return a negotiation route only when the selected profile exposes it.
pub(crate) fn negotiate_hub_name_for_target(
    path: &str,
    target: crate::config::ControllerProfile,
) -> Option<&'static str> {
    let hub = negotiate_hub_name(path)?;
    let supported = match target {
        crate::config::ControllerProfile::Legacy => {
            matches!(hub, "application" | "logs" | "search" | "metrics")
        }
        crate::config::ControllerProfile::Native => {
            matches!(
                hub,
                "application" | "logs" | "search" | "songid" | "listening-party" | "transfers"
            )
        }
    };
    supported.then_some(hub)
}

/// Map a hub to the equivalent protected controller surface.
pub(crate) fn auth_path(hub: &str) -> &'static str {
    match hub {
        "application" => "/api/application",
        "logs" => "/api/logs",
        "search" => "/api/searches",
        "metrics" => "/api/metrics",
        "songid" => "/api/songid",
        "listening-party" => "/api/listening-party",
        "transfers" => "/api/transfers",
        _ => "/api/application",
    }
}

/// Build the response consumed by the JavaScript SignalR client before it
/// upgrades the connection to WebSockets.
pub(crate) fn negotiate_response() -> routing::HttpResponse {
    let connection_id = format!("slskr-{}", Uuid::new_v4().simple());
    routing::HttpResponse {
        status: "200 OK",
        content_type: "application/json; charset=utf-8",
        body: json!({
            "negotiateVersion": 1,
            "connectionId": connection_id,
            "connectionToken": connection_id,
            "availableTransports": [
                {
                    "transport": "WebSockets",
                    "transferFormats": ["Text"],
                },
                {
                    "transport": "ServerSentEvents",
                    "transferFormats": ["Text"],
                },
                {
                    "transport": "LongPolling",
                    "transferFormats": ["Text"],
                },
            ],
        })
        .to_string(),
    }
}

/// Serve one already-upgraded target-compatible hub connection.
pub(crate) async fn serve<R, W>(
    mut reader: R,
    writer: &mut W,
    state: Arc<AppState>,
    hub: &'static str,
) -> Result<(), String>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin,
{
    let handshake = relay_ws::read_ws_frame(&mut reader).await?;
    let relay_ws::WebSocketFrame::Text(handshake) = handshake else {
        return Err("SignalR handshake must be a text frame".to_owned());
    };
    let mut initial_messages = relay_ws::signalr_messages(&handshake)?;
    let Some(first) = initial_messages.first() else {
        return Err("SignalR handshake is empty".to_owned());
    };
    let handshake_value = serde_json::from_str::<Value>(first)
        .map_err(|_| "SignalR handshake is not valid JSON".to_owned())?;
    if handshake_value.get("protocol").and_then(Value::as_str) != Some("json")
        || handshake_value.get("version").and_then(Value::as_u64) != Some(1)
    {
        return Err("only SignalR JSON protocol version 1 is supported".to_owned());
    }
    relay_ws::write_signalr_json(writer, &json!({})).await?;
    initial_messages.drain(0..1);

    let mut receiver = state.event_tx.subscribe();
    // SignalR groups are connection-local for this compatibility surface. A
    // bounded set prevents a client from turning repeated JoinParty calls into
    // unbounded per-connection memory growth.
    let mut listening_party_groups = HashSet::new();
    for (target, argument) in initial_hub_messages(&state, hub).await {
        write_invocation(writer, &target, argument).await?;
    }

    let (inbound_tx, mut inbound_rx) = mpsc::unbounded_channel();
    let reader_task = tokio::spawn(async move {
        loop {
            let frame = relay_ws::read_ws_frame(&mut reader).await;
            let done = matches!(&frame, Ok(relay_ws::WebSocketFrame::Close(_)) | Err(_));
            if inbound_tx.send(frame).is_err() || done {
                break;
            }
        }
    });

    let result = async {
        for message in initial_messages {
            handle_client_message(writer, hub, &mut listening_party_groups, &message).await?;
        }

        loop {
            tokio::select! {
                inbound = inbound_rx.recv() => match inbound {
                    Some(Ok(relay_ws::WebSocketFrame::Text(text))) => {
                        for message in relay_ws::signalr_messages(&text)? {
                            handle_client_message(
                                writer,
                                hub,
                                &mut listening_party_groups,
                                &message,
                            )
                            .await?;
                        }
                    }
                    Some(Ok(relay_ws::WebSocketFrame::Ping(payload))) => {
                        relay_ws::write_ws_frame(writer, 0x8a, &payload).await?;
                    }
                    Some(Ok(relay_ws::WebSocketFrame::Pong)) => {}
                    Some(Ok(relay_ws::WebSocketFrame::Close(payload))) => {
                        relay_ws::write_ws_frame(writer, 0x88, &payload).await?;
                        return Ok(());
                    }
                    Some(Err(error)) => return Err(error),
                    None => return Ok(()),
                },
                received = receiver.recv() => match received {
                    Ok(record) => {
                        for (target, argument) in event_hub_messages(
                            &state,
                            hub,
                            &listening_party_groups,
                            &record,
                        )
                        .await
                        {
                            write_invocation(writer, &target, argument).await?;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                },
            }
        }
    }
    .await;

    reader_task.abort();
    let _ = reader_task.await;
    result
}

async fn handle_client_message<W>(
    writer: &mut W,
    hub: &str,
    listening_party_groups: &mut HashSet<String>,
    message: &str,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    let value = serde_json::from_str::<Value>(message)
        .map_err(|_| "SignalR invocation is not valid JSON".to_owned())?;
    match value
        .get("type")
        .and_then(Value::as_u64)
        .unwrap_or_default()
    {
        6 => relay_ws::write_signalr_json(writer, &json!({"type": 6})).await,
        1 => {
            let Some(invocation_id) = value.get("invocationId").and_then(Value::as_str) else {
                return Ok(());
            };
            if hub == "listening-party" {
                let target = value
                    .get("target")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let arguments = value
                    .get("arguments")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if let Err(error) =
                    update_listening_party_membership(listening_party_groups, target, arguments)
                {
                    return write_invocation_error(writer, invocation_id, error).await;
                }
            }
            relay_ws::write_signalr_json(
                writer,
                &json!({
                    "type": 3,
                    "invocationId": invocation_id,
                    "result": Value::Null,
                }),
            )
            .await
        }
        7 => Ok(()),
        _ => Ok(()),
    }
}

fn update_listening_party_membership(
    groups: &mut HashSet<String>,
    target: &str,
    arguments: &[Value],
) -> Result<(), &'static str> {
    let is_join = target == "JoinParty";
    let is_leave = target == "LeaveParty";
    if !is_join && !is_leave {
        return Ok(());
    }
    let group = arguments
        .first()
        .and_then(Value::as_str)
        .and_then(|pod_id| {
            arguments
                .get(1)
                .and_then(Value::as_str)
                .and_then(|channel_id| listening_party_group_key(pod_id, channel_id))
        })
        .ok_or("podId and channelId are required")?;
    if is_join {
        if groups.len() >= MAX_LISTENING_PARTY_GROUPS_PER_CONNECTION && !groups.contains(&group) {
            return Err("listening-party group limit reached");
        }
        groups.insert(group);
    } else {
        groups.remove(&group);
    }
    Ok(())
}

fn listening_party_group_key(pod_id: &str, channel_id: &str) -> Option<String> {
    let pod_id = pod_id.trim();
    let channel_id = channel_id.trim();
    if pod_id.is_empty()
        || channel_id.is_empty()
        || pod_id.len() > MAX_LISTENING_PARTY_GROUP_COMPONENT_BYTES
        || channel_id.len() > MAX_LISTENING_PARTY_GROUP_COMPONENT_BYTES
    {
        return None;
    }
    Some(format!("party:{pod_id}:{channel_id}"))
}

fn listening_party_state(record: &EventRecord) -> Option<Value> {
    record
        .detail
        .as_deref()
        .and_then(|detail| serde_json::from_str::<Value>(detail).ok())
        .filter(Value::is_object)
}

fn listening_party_event_group(record: &EventRecord) -> Option<String> {
    let state = listening_party_state(record)?;
    let pod_id = state
        .get("podId")
        .or_else(|| state.get("PodId"))
        .and_then(Value::as_str);
    let channel_id = state
        .get("channelId")
        .or_else(|| state.get("ChannelId"))
        .and_then(Value::as_str);
    listening_party_group_key(pod_id?, channel_id?)
}

async fn write_invocation_error<W>(
    writer: &mut W,
    invocation_id: &str,
    error: &str,
) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    relay_ws::write_signalr_json(
        writer,
        &json!({
            "type": 3,
            "invocationId": invocation_id,
            "error": error,
        }),
    )
    .await
}

async fn write_invocation<W>(writer: &mut W, target: &str, argument: Value) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
    relay_ws::write_signalr_json(
        writer,
        &json!({
            "type": 1,
            "target": target,
            "arguments": [argument],
        }),
    )
    .await
}

async fn initial_hub_messages(state: &AppState, hub: &str) -> Vec<(String, Value)> {
    match hub {
        "application" => {
            let application_state = application_state_json(state).await;
            let options = {
                let overlay = state.options_overlay.read().await;
                serde_json::from_str::<Value>(&crate::controller_options_json(
                    &state.config,
                    &overlay,
                    true,
                ))
                .unwrap_or_else(|_| json!({}))
            };
            vec![
                (
                    "STATE".to_owned(),
                    serde_json::from_str(&application_state).unwrap_or_else(|_| json!({})),
                ),
                ("OPTIONS".to_owned(), options),
            ]
        }
        "logs" => {
            let events = state.events.read().await;
            let logs = events
                .records
                .iter()
                .rev()
                .filter(|event| event.kind == "log.created")
                .map(EventRecord::data_json)
                .collect::<Vec<_>>();
            vec![("BUFFER".to_owned(), Value::Array(logs))]
        }
        "search" => {
            let searches = state.searches.read().await;
            let records = searches
                .records
                .iter()
                .filter_map(|record| serde_json::from_str::<Value>(&record.json()).ok())
                .collect::<Vec<_>>();
            vec![("LIST".to_owned(), Value::Array(records))]
        }
        "metrics" => {
            let transfers = state.transfers.read().await;
            vec![("Update".to_owned(), transfer_metrics(&transfers))]
        }
        "songid" => vec![("LIST".to_owned(), Value::Array(Vec::new()))],
        _ => Vec::new(),
    }
}

async fn event_hub_messages(
    state: &AppState,
    hub: &str,
    listening_party_groups: &HashSet<String>,
    record: &EventRecord,
) -> Vec<(String, Value)> {
    match hub {
        "application" if matches!(record.topic(), "application" | "settings") => {
            let application_state = application_state_json(state).await;
            let state_value =
                serde_json::from_str(&application_state).unwrap_or_else(|_| json!({}));
            if record.kind.starts_with("options.") {
                let options = {
                    let overlay = state.options_overlay.read().await;
                    serde_json::from_str::<Value>(&crate::controller_options_json(
                        &state.config,
                        &overlay,
                        true,
                    ))
                    .unwrap_or_else(|_| json!({}))
                };
                vec![("OPTIONS".to_owned(), options)]
            } else {
                vec![("STATE".to_owned(), state_value)]
            }
        }
        "logs" if record.kind == "log.created" => {
            vec![("LOG".to_owned(), record.data_json())]
        }
        "search" if record.topic() == "searches" => {
            let searches = state.searches.read().await;
            let search = searches
                .records
                .iter()
                .find(|search| {
                    search.id == record.resource || search.token.to_string() == record.resource
                })
                .and_then(|search| serde_json::from_str::<Value>(&search.json()).ok());
            match (record.kind.as_str(), search) {
                ("search.started" | "search.created", Some(search)) => {
                    vec![("CREATE".to_owned(), search)]
                }
                (_, Some(search)) => vec![("UPDATE".to_owned(), search)],
                _ => vec![("DELETE".to_owned(), json!({"id": record.resource}))],
            }
        }
        "transfers" if record.kind == "transfer.hub.activity" => transfer_hub_payload(record)
            .map(|payload| vec![("ACTIVITY".to_owned(), payload)])
            .unwrap_or_default(),
        "transfers" if record.kind == "transfer.hub.progress" => transfer_hub_payload(record)
            .map(|payload| vec![("PROGRESS".to_owned(), payload)])
            .unwrap_or_default(),
        "transfers" if record.kind == "transfer.hub.removed" => transfer_hub_payload(record)
            .map(|payload| vec![("REMOVED".to_owned(), payload)])
            .unwrap_or_default(),
        "transfers" if record.topic() == "transfers" => {
            let transfers = state.transfers.read().await;
            vec![("activity".to_owned(), transfer_metrics(&transfers))]
        }
        "metrics" if record.topic() == "transfers" => {
            let transfers = state.transfers.read().await;
            vec![("Update".to_owned(), transfer_metrics(&transfers))]
        }
        "songid" if record.topic() == "media" => {
            vec![("UPDATE".to_owned(), record.data_json())]
        }
        "listening-party" if record.topic() == "rooms" => {
            let Some(group) = listening_party_event_group(record) else {
                return Vec::new();
            };
            if !listening_party_groups.contains(&group) {
                return Vec::new();
            }
            let Some(state) = listening_party_state(record) else {
                return Vec::new();
            };
            vec![("partyState".to_owned(), state)]
        }
        _ => Vec::new(),
    }
}

fn transfer_hub_payload(record: &EventRecord) -> Option<Value> {
    record
        .detail
        .as_deref()
        .and_then(|detail| serde_json::from_str::<Value>(detail).ok())
        .filter(Value::is_object)
}

async fn application_state_json(state: &AppState) -> String {
    let session = state.session.read().await;
    let share_lifecycle = state.share_lifecycle.read().await;
    let rooms = state.rooms.read().await;
    let users = state.users.read().await;
    let relay = state.relay.read().await;
    let runtime = state.runtime.read().await;
    let distributed_network = state.distributed_network.read().await;
    let distributed_settings = *state.soulseek_distributed_settings.read().await;
    let runtime_credentials_configured = state.runtime_credentials.read().await.is_some();
    let connected_endpoint = crate::connected_server_address(state);
    crate::application_state_json(
        &session,
        &share_lifecycle,
        &rooms,
        &users,
        &relay,
        &runtime,
        &distributed_network,
        distributed_settings,
        &state.config,
        runtime_credentials_configured,
        connected_endpoint.as_deref(),
        crate::controller_version_json(state),
    )
}

fn transfer_metrics(transfers: &crate::TransferQueue) -> Value {
    fn direction_metrics(transfers: &crate::TransferQueue, direction: u32) -> Value {
        let entries = transfers
            .entries
            .iter()
            .filter(|entry| entry.direction == direction)
            .collect::<Vec<_>>();
        let in_progress = entries
            .iter()
            .copied()
            .filter(|entry| crate::is_active_transfer_status(&entry.status))
            .collect::<Vec<_>>();
        let queued = entries
            .iter()
            .copied()
            .filter(|entry| entry.status == "queued")
            .collect::<Vec<_>>();
        let completed = entries
            .iter()
            .copied()
            .filter(|entry| {
                matches!(
                    entry.status.as_str(),
                    "succeeded" | "failed" | "cancelled" | "rejected"
                )
            })
            .collect::<Vec<_>>();
        let users = |items: &[&crate::TransferEntry]| {
            items
                .iter()
                .filter_map(|entry| entry.peer_username.as_deref())
                .collect::<std::collections::HashSet<_>>()
                .len()
        };
        json!({
            "InProgress": {
                "Files": in_progress.len(),
                "Users": users(&in_progress),
                "AverageSpeed": 0,
                "TotalSpeed": 0,
            },
            "Queued": {
                "Files": queued.len(),
                "Users": users(&queued),
                "Bytes": queued.iter().filter_map(|entry| entry.size).sum::<u64>(),
            },
            "Completed": {
                "Succeeded": completed.iter().filter(|entry| entry.status == "succeeded").count(),
                "Failed": completed.iter().filter(|entry| matches!(entry.status.as_str(), "failed" | "rejected")).count(),
                "Bytes": completed.iter().map(|entry| entry.bytes_transferred).sum::<u64>(),
            },
        })
    }

    json!({
        "Downloads": direction_metrics(transfers, 0),
        "Uploads": direction_metrics(transfers, 1),
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        hub_name, hub_name_for_target, listening_party_event_group, listening_party_group_key,
        negotiate_hub_name, negotiate_hub_name_for_target, negotiate_response,
        update_listening_party_membership, EventRecord, MAX_LISTENING_PARTY_GROUPS_PER_CONNECTION,
    };

    #[test]
    fn target_hub_routes_are_explicitly_bounded() {
        assert_eq!(hub_name("/hub/application"), Some("application"));
        assert_eq!(hub_name("/hub/listening-party"), Some("listening-party"));
        assert_eq!(hub_name("/hub/unknown"), None);
        assert_eq!(negotiate_hub_name("/hub/search/negotiate"), Some("search"));
        assert_eq!(negotiate_hub_name("/hub/search"), None);
    }

    #[test]
    fn hub_routes_match_the_frozen_profile_matrices() {
        use crate::config::ControllerProfile::{Legacy, Native};

        assert_eq!(hub_name_for_target("/hub/metrics", Legacy), Some("metrics"));
        assert_eq!(hub_name_for_target("/hub/transfers", Legacy), None);
        assert_eq!(hub_name_for_target("/hub/songid", Native), Some("songid"));
        assert_eq!(hub_name_for_target("/hub/metrics", Native), None);
        assert_eq!(
            negotiate_hub_name_for_target("/hub/listening-party/negotiate", Native),
            Some("listening-party")
        );
        assert_eq!(
            negotiate_hub_name_for_target("/hub/listening-party/negotiate", Legacy),
            None
        );
    }

    #[test]
    fn negotiation_advertises_json_websocket_transport() {
        let response = negotiate_response();
        let body = serde_json::from_str::<serde_json::Value>(&response.body).expect("json");
        assert_eq!(response.status, "200 OK");
        assert_eq!(body["negotiateVersion"], 1);
        assert_eq!(body["availableTransports"][0]["transport"], "WebSockets");
        assert_eq!(body["availableTransports"][0]["transferFormats"][0], "Text");
    }

    #[test]
    fn listening_party_membership_matches_target_group_names() {
        let mut groups = HashSet::new();
        update_listening_party_membership(
            &mut groups,
            "JoinParty",
            &[
                serde_json::json!(" pod-1 "),
                serde_json::json!(" channel-1 "),
            ],
        )
        .expect("join");
        assert!(groups.contains("party:pod-1:channel-1"));

        update_listening_party_membership(
            &mut groups,
            "LeaveParty",
            &[serde_json::json!("pod-1"), serde_json::json!("channel-1")],
        )
        .expect("leave");
        assert!(groups.is_empty());
        assert_eq!(
            listening_party_group_key("pod", "channel"),
            Some("party:pod:channel".to_owned())
        );
    }

    #[test]
    fn listening_party_events_are_only_routed_to_matching_groups() {
        let record = EventRecord {
            id: 1,
            kind: "listening_party.updated".to_owned(),
            resource: "listening-party/pod-1/channel-1".to_owned(),
            detail: Some(
                serde_json::json!({"podId": "pod-1", "channelId": "channel-1", "action": "play"})
                    .to_string(),
            ),
            created_at: 1,
        };
        assert_eq!(
            listening_party_event_group(&record).as_deref(),
            Some("party:pod-1:channel-1")
        );
        let mut groups = HashSet::new();
        groups.insert("party:other:channel".to_owned());
        assert!(!groups.contains("party:pod-1:channel-1"));
    }

    #[test]
    fn listening_party_group_limit_is_bounded() {
        let mut groups = HashSet::new();
        for index in 0..MAX_LISTENING_PARTY_GROUPS_PER_CONNECTION {
            update_listening_party_membership(
                &mut groups,
                "JoinParty",
                &[
                    serde_json::json!(format!("pod-{index}")),
                    serde_json::json!("channel"),
                ],
            )
            .expect("within bound");
        }
        let result = update_listening_party_membership(
            &mut groups,
            "JoinParty",
            &[serde_json::json!("overflow"), serde_json::json!("channel")],
        );
        assert_eq!(result, Err("listening-party group limit reached"));
    }
}
