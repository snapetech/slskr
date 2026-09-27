use super::*;

pub(super) fn transfer_remote_directory(filename: &str) -> &str {
    filename
        .rfind(['/', '\\'])
        .map_or("", |separator| &filename[..separator])
}

pub(super) async fn maybe_import_lidarr_completed_download(
    state: &AppState,
    transfer: &TransferEntry,
) {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return;
    }
    let Some(local_directory) = transfer
        .local_path
        .as_deref()
        .and_then(|path| Path::new(path).parent())
        .map(|path| path.to_string_lossy().to_string())
    else {
        return;
    };
    let remote_directory = transfer_remote_directory(&transfer.filename);
    let has_pending_file = state
        .transfers
        .read()
        .await
        .entries
        .iter()
        .any(|candidate| {
            candidate.id != transfer.id
                && candidate.direction == 0
                && candidate.peer_username == transfer.peer_username
                && transfer_remote_directory(&candidate.filename) == remote_directory
                && !is_terminal_transfer_status(&candidate.status)
        });
    if has_pending_file {
        return;
    }
    let lidarr = state.integration_settings.read().await.lidarr.clone();
    if !lidarr.enabled || !lidarr.auto_import_completed {
        return;
    }
    match run_lidarr_automatic_import_with_history(state, &lidarr, &local_directory).await {
        Ok(result) => {
            if result
                .get("rejectedCandidateCount")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|count| count > 0)
            {
                if let Err(error) =
                    apply_lidarr_rejection_policy(state, transfer, &local_directory, &result).await
                {
                    record_daemon_log(
                        state,
                        logging::LogLevel::Warn,
                        "lidarr",
                        format!(
                            "Lidarr rejected-download policy failed for {local_directory}: {error}"
                        ),
                    )
                    .await;
                }
            }
        }
        Err(error) => {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "lidarr",
                format!("Lidarr auto-import failed for {local_directory}: {error}"),
            )
            .await;
        }
    }
}

/// Issue the controller-side relay download tokens when a local download
/// completes.  The SignalR notification transport is still a separate
/// integration boundary, but token issuance must happen at completion so a
/// connected agent can retry the authenticated HTTP fetch without the old
/// token-passthrough shortcut.
pub(super) async fn issue_relay_download_tokens(state: &AppState, transfer: &TransferEntry) {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return;
    }
    let Some(local_path) = transfer.local_path.as_deref() else {
        return;
    };
    let root = effective_downloads_dir(state);
    let Ok(local_path) = ensure_scoped_download_path(&root, local_path) else {
        return;
    };
    let Ok(filename) = local_path.strip_prefix(&root) else {
        return;
    };
    let filename = filename.to_string_lossy().replace('\\', "/");
    let settings = state.advanced_networking.read().await.relay.clone();
    if !settings.enabled || !matches!(settings.mode.as_str(), "controller" | "debug") {
        return;
    }
    let mut relay = state.relay.write().await;
    let issued = relay
        .protocol
        .issue_download_tokens(&filename, unix_timestamp());
    for (agent_name, token) in issued {
        let sent = relay::send_hub_invocation(
            &relay.protocol,
            &agent_name,
            "NotifyFileDownloadCompleted",
            vec![
                serde_json::Value::String(filename.clone()),
                serde_json::Value::String(token.clone()),
            ],
        );
        if !sent {
            relay.protocol.cancel_download(&token);
            ::tracing::warn!(
                agent = %agent_name,
                "relay download completion notification could not be queued"
            );
        }
    }
}

pub(super) async fn apply_lidarr_rejection_policy(
    state: &AppState,
    transfer: &TransferEntry,
    local_directory: &str,
    result: &serde_json::Value,
) -> Result<(), String> {
    let options = state.integration_settings.read().await.lidarr.clone();
    let rejected_filenames = result
        .get("rejectedFilenames")
        .and_then(serde_json::Value::as_array)
        .map(|filenames| {
            filenames
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if options.blacklist_rejected_downloads {
        let Some(requested_item_id) = transfer.wishlist_item_id.as_deref() else {
            return delete_lidarr_rejected_files_if_enabled(
                &options,
                local_directory,
                &rejected_filenames,
            );
        };
        let Some(username) = transfer
            .peer_username
            .as_deref()
            .filter(|username| !username.trim().is_empty())
        else {
            return delete_lidarr_rejected_files_if_enabled(
                &options,
                local_directory,
                &rejected_filenames,
            );
        };
        let remote_directory = transfer_remote_directory(&transfer.filename);
        if !remote_directory.trim().is_empty() {
            let native_contract = state.config.controller_profile == ControllerProfile::Native;
            let changed_searches_to_publish = {
                let _wishlist_search_persistence =
                    state.wishlist_search_persistence_lock.lock().await;
                let _search_persistence = state.search_persistence_lock.lock().await;
                let ignored = {
                    let mut wishlist = state.wishlist.write().await;
                    if let Some(item_id) =
                        wishlist.resolve_item_id(requested_item_id, native_contract)
                    {
                        let previous = wishlist.clone();
                        let ignored_result = match wishlist.ignore_result(
                            &item_id,
                            username,
                            remote_directory,
                            native_contract,
                        ) {
                            Ok((rule, created)) => Some((rule, created)),
                            Err("not_found") => None,
                            Err("capacity") => {
                                return Err("wishlist ignored-result capacity is full".to_owned());
                            }
                            Err(_) => {
                                return Err("invalid wishlist ignored-result inputs".to_owned());
                            }
                        };
                        let mutated = wishlist.clone();
                        ignored_result.map(|(rule, created)| (previous, mutated, rule, created))
                    } else {
                        None
                    }
                };
                if let Some((previous_wishlist, mutated_wishlist, rule, created)) = ignored {
                    if created {
                        let (previous_searches, mutated_searches, changed_searches) = {
                            let mut searches = state.searches.write().await;
                            let previous = searches.clone();
                            let changed = searches.suppress_ignored_result(&rule);
                            let mutated = searches.clone();
                            (previous, mutated, changed)
                        };
                        if let Err(error) = persist_wishlist_ignored_result_and_searches_checked(
                            state,
                            &rule,
                            &changed_searches,
                        )
                        .await
                        {
                            rollback_wishlist_if_unchanged(
                                state,
                                previous_wishlist,
                                &mutated_wishlist,
                            )
                            .await;
                            let mut searches = state.searches.write().await;
                            if *searches == mutated_searches {
                                *searches = previous_searches;
                            }
                            return Err(error);
                        }
                        Some(changed_searches)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(changed_searches) = changed_searches_to_publish {
                for search in &changed_searches {
                    publish_search_hub_event(state, "update", search);
                }
            }
        }
    }

    delete_lidarr_rejected_files_if_enabled(&options, local_directory, &rejected_filenames)
}

pub(super) fn delete_lidarr_rejected_files_if_enabled(
    options: &config::LidarrIntegrationSettings,
    local_directory: &str,
    filenames: &[String],
) -> Result<(), String> {
    if options.delete_rejected_downloads {
        let _ = delete_lidarr_rejected_files(local_directory, filenames)?;
    }
    Ok(())
}

pub(super) async fn maybe_upload_ftp_completed_download(
    state: &AppState,
    transfer: &TransferEntry,
) {
    if transfer.direction != 0 || !is_successful_transfer_status(&transfer.status) {
        return;
    }
    let Some(local_path) = transfer.local_path.as_deref() else {
        return;
    };
    let root = effective_downloads_dir(state);
    let Ok(local_path) = ensure_scoped_download_path(&root, local_path) else {
        return;
    };
    let options = state.integration_settings.read().await.ftp.clone();
    if !options.enabled {
        return;
    }
    let target = state.config.controller_profile;
    tokio::spawn(async move {
        if let Err(error) = ftp::upload_completed_file(&options, target, &local_path).await {
            eprintln!("[FTP] Completed-download upload failed: {error}");
        }
    });
}
