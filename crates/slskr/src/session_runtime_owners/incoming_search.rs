use super::*;

pub(super) struct IncomingSearchQueueLease(pub(super) Arc<AppState>);

impl Drop for IncomingSearchQueueLease {
    fn drop(&mut self) {
        self.0
            .incoming_search_queue_depth
            .fetch_sub(1, Ordering::AcqRel);
    }
}

pub(crate) async fn schedule_incoming_search_response(
    state: Arc<AppState>,
    username: String,
    token: u32,
    query: String,
) {
    let circuit_breaker = state
        .core_workflow_settings
        .read()
        .await
        .incoming_search
        .circuit_breaker;
    let depth = state
        .incoming_search_queue_depth
        .fetch_add(1, Ordering::AcqRel);
    if depth >= circuit_breaker {
        state
            .incoming_search_queue_depth
            .fetch_sub(1, Ordering::AcqRel);
        return;
    }
    let task_state = Arc::clone(&state);
    let queue_lease = IncomingSearchQueueLease(Arc::clone(&state));
    state.spawn_managed_task(async move {
        let state = task_state;
        let gate = Arc::clone(&state.incoming_searches);
        let result = async {
            let _permit = gate
                .acquire_owned()
                .await
                .map_err(|_| "incoming search gate closed".to_owned())?;
            let Some(response) = build_file_search_response(&state, token, &query).await else {
                record_soulseek_diagnostic(
                    &state,
                    logging::LogLevel::Debug,
                    "search",
                    format!(
                        "Incoming public search for {:?} matched no shared files",
                        query
                    ),
                )
                .await;
                return Ok::<(), String>(());
            };
            let address = request_peer_endpoint(&state, &username).await?;
            send_peer_message_oneway(&state, &address, PeerMessage::FileSearchResponse(response))
                .await?;
            record_soulseek_diagnostic(
                &state,
                logging::LogLevel::Debug,
                "search",
                format!(
                    "Sent incoming public search response to {} for {:?}",
                    redact_username(&username),
                    query
                ),
            )
            .await;
            Ok(())
        }
        .await;
        drop(queue_lease);
        if let Err(error) = result {
            record_daemon_log(
                &state,
                logging::LogLevel::Warn,
                "search",
                format!(
                    "incoming search response for {} failed: {error}",
                    redact_username(&username)
                ),
            )
            .await;
        }
    });
}
