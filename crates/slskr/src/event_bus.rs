//! Compatibility event publication shared by the HTTP and SignalR surfaces.
//!
//! Domain code owns state transitions; this module owns the small, observable
//! translation from those transitions to the compatibility event stream. Keep
//! the target-visible event topic, ordering, and payload rules here so a route
//! refactor cannot accidentally create a second event contract.

use serde_json::Value;

use crate::{config, signalr_ws, AppState, EventRecord, SearchRecord, TransferEntry};

pub(crate) fn transfer_hub_activity_json(entry: &TransferEntry) -> Value {
    let size = entry.size.unwrap_or(0);
    let percent_complete = if size == 0 {
        0.0
    } else {
        ((entry.bytes_transferred as f64 / size as f64) * 100.0).min(100.0)
    };
    let state = crate::controller_transfer_state(&entry.status);
    let previous_state = entry
        .previous_status
        .as_deref()
        .map(crate::controller_transfer_state)
        .unwrap_or(state);
    serde_json::json!({
        "timestamp": crate::unix_seconds_rfc3339(crate::unix_timestamp()),
        "id": entry.id.to_string(),
        "requestId": entry.request_id,
        "direction": if entry.direction == 0 { "Download" } else { "Upload" },
        "username": entry.peer_username.as_deref().unwrap_or_default(),
        "filename": entry.filename,
        "previousState": previous_state,
        "state": state,
        "size": i64::try_from(size).unwrap_or(i64::MAX),
        "bytesTransferred": i64::try_from(entry.bytes_transferred).unwrap_or(i64::MAX),
        "averageSpeed": entry.average_speed_at(crate::unix_timestamp()),
        "percentComplete": percent_complete,
        "placeInQueue": Value::Null,
    })
}

pub(crate) fn transfer_hub_removed_json(entry: &TransferEntry) -> Value {
    serde_json::json!({
        "id": entry.id.to_string(),
        "requestId": entry.request_id,
        "direction": if entry.direction == 0 { "Download" } else { "Upload" },
        "username": entry.peer_username.as_deref().unwrap_or_default(),
        "filename": entry.filename,
    })
}

pub(crate) fn publish_transfer_hub_event(state: &AppState, kind: &str, entry: &TransferEntry) {
    let detail = match kind {
        "removed" => transfer_hub_removed_json(entry),
        _ => transfer_hub_activity_json(entry),
    };
    publish_signalr_hub_event(
        state,
        &format!("transfer.hub.{kind}"),
        entry.id.to_string(),
        detail,
    );
}

pub(crate) fn publish_search_hub_event(state: &AppState, kind: &str, search: &SearchRecord) {
    let Some(detail) = signalr_ws::search_hub_payload(
        search,
        state.config.controller_profile == config::ControllerProfile::Native,
    ) else {
        return;
    };
    publish_signalr_hub_event(
        state,
        &format!("search.hub.{kind}"),
        search.id.clone(),
        detail,
    );
}

pub(crate) fn publish_songid_hub_event(state: &AppState, kind: &str, run: &Value) {
    if !run.is_object() {
        return;
    }
    let resource = run
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    publish_signalr_hub_event(state, &format!("songid.hub.{kind}"), resource, run.clone());
}

pub(crate) fn publish_signalr_hub_event(
    state: &AppState,
    kind: &str,
    resource: String,
    detail: Value,
) {
    if state.event_tx.receiver_count() == 0 {
        return;
    }
    let record = bounded_hub_event_record(kind, resource, detail);
    let _ = state.event_tx.send(record);
}

fn bounded_hub_event_record(kind: &str, resource: String, detail: Value) -> EventRecord {
    EventRecord {
        id: 0,
        kind: crate::truncate_utf8_bytes(kind.to_owned(), crate::MAX_EVENT_KIND_BYTES),
        resource: crate::truncate_utf8_bytes(resource, crate::MAX_EVENT_RESOURCE_BYTES),
        detail: crate::bounded_event_detail(Some(detail.to_string())),
        created_at: crate::unix_timestamp(),
    }
}

#[cfg(test)]
mod tests {
    use super::bounded_hub_event_record;

    #[test]
    fn live_hub_events_use_the_same_bounds_as_persisted_events() {
        let detail = serde_json::json!({
            "payload": "x".repeat(crate::MAX_EVENT_DETAIL_BYTES + 1)
        });
        let detail_length = detail.to_string().len();
        let record = bounded_hub_event_record(
            &"k".repeat(crate::MAX_EVENT_KIND_BYTES + 1),
            "r".repeat(crate::MAX_EVENT_RESOURCE_BYTES + 1),
            detail,
        );

        assert_eq!(record.kind.len(), crate::MAX_EVENT_KIND_BYTES);
        assert_eq!(record.resource.len(), crate::MAX_EVENT_RESOURCE_BYTES);
        assert_eq!(
            record.detail,
            Some(format!(
                "<omitted oversized event detail: {detail_length} bytes>"
            ))
        );
    }
}
