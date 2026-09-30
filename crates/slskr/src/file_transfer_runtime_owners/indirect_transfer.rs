use super::*;

pub(crate) async fn fail_indirect_transfer(state: &AppState, token: u32, reason: String) {
    let transfer = {
        let transfers = state.transfers.read().await;
        transfers
            .entries
            .iter()
            .find(|entry| entry.token == token && entry.status == "indirect_pending")
            .cloned()
    };
    if let Some(transfer) = transfer {
        let failed = {
            let mut transfers = state.transfers.write().await;
            transfers.update_status(transfer.id, "failed", None, Some(reason))
        };
        if let Some(failed) = failed {
            persist_transfer_projection(state, &failed).await;
        }
    }
}

pub(crate) async fn fail_indirect_browse(state: &AppState, token: u32, reason: String) {
    let _browse_persistence = state.browse_persistence_lock.lock().await;
    let failed = {
        let mut browse = state.browse.write().await;
        browse.fail_indirect(token, reason.clone())
    };
    let persistence_failure = if let Some(record) = failed.as_ref() {
        persist_browse_record_checked(state, record)
            .await
            .err()
            .map(|error| (record.clone(), error))
    } else {
        None
    };
    drop(_browse_persistence);

    if let Some(record) = failed {
        if let Some((failed_record, error)) = persistence_failure {
            crate::browse_runtime::report_browse_projection_failure(state, &failed_record, error)
                .await;
        }
        record_event(
            state,
            "browse.failed",
            record.username.clone(),
            Some(reason),
        )
        .await;
    }
}

pub(super) async fn execute_indirect_file_transfer(
    state: &AppState,
    response: &ConnectToPeerResponse,
    transfer: &TransferEntry,
) -> Result<(u64, u64), String> {
    if let Some(reason) = cancel_download_if_blocked_by_policy(state, transfer).await {
        return Err(reason);
    }
    let mut connection = connect_indirect_file_transfer(state, response).await?;
    if transfer.direction == 1 {
        upload_file_transfer_with_connection(state, transfer, &mut connection, true).await
    } else {
        download_file_transfer_with_connection(state, transfer, &mut connection).await
    }
}
