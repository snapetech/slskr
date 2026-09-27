use super::*;

pub(super) async fn record_event(
    state: &AppState,
    kind: impl Into<String>,
    resource: impl Into<String>,
    detail: Option<String>,
) {
    let kind = kind.into();
    let detail = if kind == "browse.failed" && detail.is_some() {
        Some("browse failed".to_owned())
    } else {
        detail
    };
    let _event_persistence = state.event_persistence_lock.lock().await;
    let mut events = state.events.write().await;
    let record = events.record(kind, resource, detail);
    drop(events);
    let persistence_failure = persist_event_record_checked(state, &record).await.err();
    drop(_event_persistence);
    if let Some(error) = persistence_failure {
        eprintln!("[Error] event persistence failed: {error}");
        let session_connected = state.session.read().await.state == "connected";
        if !session_connected {
            update_session(state, |snapshot| {
                snapshot.last_error = Some(error);
            })
            .await;
        }
    }
    let _ = state.event_tx.send(record);
}

pub(super) async fn record_daemon_log(
    state: &AppState,
    level: logging::LogLevel,
    category: &str,
    message: impl Into<String>,
) {
    let message = message.into();
    let configured_level = *state.log_level.read().await;
    let rendered = format!(
        "[{}] {}: {}",
        logging::LogConfig::level_name(level),
        category,
        message
    );
    if configured_level <= level {
        eprintln!("{rendered}");
    }
    if state.config.logger.disk {
        let log_dir = state.config.state_dir.join("logs");
        match tokio::fs::create_dir_all(&log_dir).await {
            Ok(()) => {
                match tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(
                        log_dir.join(format!("slskr-{}.log", chrono::Utc::now().format("%Y%m%d"))),
                    )
                    .await
                {
                    Ok(mut file) => {
                        let line = format!("{} {rendered}\n", logging::format_timestamp());
                        if let Err(error) = file.write_all(line.as_bytes()).await {
                            eprintln!("[Error] daemon disk log write failed: {error}");
                        }
                    }
                    Err(error) => {
                        eprintln!("[Error] daemon disk log open failed: {error}");
                    }
                }
            }
            Err(error) => {
                eprintln!("[Error] daemon log directory creation failed: {error}");
            }
        }
    }
    if let Some(endpoint) = state.config.logger.loki.as_deref() {
        let endpoint = if endpoint.ends_with("/loki/api/v1/push") {
            endpoint.to_owned()
        } else {
            format!("{}/loki/api/v1/push", endpoint.trim_end_matches('/'))
        };
        let timestamp_ns = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or_default()
            .to_string();
        let payload = serde_json::json!({
            "streams": [{
                "stream": {"application": "slskr", "category": category},
                "values": [[timestamp_ns, rendered]],
            }],
        });
        let loki_result = time::timeout(Duration::from_secs(2), async {
            reqwest::Client::builder()
                // A redirect must not move log records away from the
                // configured Loki destination.
                .redirect(reqwest::redirect::Policy::none())
                .build()?
                .post(endpoint)
                .json(&payload)
                .send()
                .await
        })
        .await;
        match loki_result {
            Ok(Ok(response)) if response.status().is_success() => {}
            Ok(Ok(response)) => {
                eprintln!(
                    "[Error] Loki log delivery failed with status {}",
                    response.status()
                );
            }
            Ok(Err(error)) => {
                eprintln!("[Error] Loki log delivery request failed: {error}");
            }
            Err(_) => eprintln!("[Error] Loki log delivery timed out"),
        }
    }
    let detail = serde_json::json!({
        "level": logging::LogConfig::level_name(level),
        "category": category,
        "message": message,
    })
    .to_string();
    record_event(state, "log.created", category, Some(detail)).await;
}

pub(super) fn soulseek_diagnostic_level_allows(
    minimum: crate::config::SoulseekDiagnosticLevel,
    level: logging::LogLevel,
) -> bool {
    match minimum {
        crate::config::SoulseekDiagnosticLevel::None => false,
        crate::config::SoulseekDiagnosticLevel::Warning => level >= logging::LogLevel::Warn,
        crate::config::SoulseekDiagnosticLevel::Info => level >= logging::LogLevel::Info,
        crate::config::SoulseekDiagnosticLevel::Debug => level >= logging::LogLevel::Debug,
        crate::config::SoulseekDiagnosticLevel::Trace => true,
    }
}

pub(super) async fn record_soulseek_diagnostic(
    state: &AppState,
    level: logging::LogLevel,
    category: &str,
    message: impl Into<String>,
) {
    if !soulseek_diagnostic_level_allows(state.config.soulseek_diagnostic_level, level) {
        return;
    }
    if category == "distributed" && !state.soulseek_distributed_settings.read().await.logging {
        return;
    }
    record_daemon_log(state, level, category, message).await;
}

pub(super) async fn record_http_log(
    state: &AppState,
    request_id: &str,
    transaction: &logging::HttpTransactionLog,
) {
    if !state.config.controller_web.logging {
        return;
    }
    let level = logging::response_level(transaction.response.status_code);
    let configured_level = *state.log_level.read().await;
    let log_config = logging::LogConfig {
        level: configured_level,
        log_requests: state.config.controller_web.logging,
        log_responses: state.config.controller_web.logging,
        log_errors_only: false,
        no_color: state.config.logger.no_color,
    };
    if !log_config.should_log(level) {
        return;
    }
    let detail = serde_json::json!({
        "level": logging::LogConfig::level_name(level),
        "category": "http",
        "message": logging::transaction_summary(transaction),
        "request_id": request_id,
        "method": transaction.request.method,
        "path": logging::redacted_path(&transaction.request.path),
        "status": transaction.response.status_code,
        "duration_ms": transaction.response.duration_ms,
        "remote_addr": transaction.request.remote_addr,
    })
    .to_string();
    record_event(state, "log.created", "http", Some(detail)).await;
}

pub(super) async fn persist_event_record_checked(
    state: &AppState,
    record: &EventRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = crate::persistence::EventRecord {
        id: i64::try_from(record.id).unwrap_or(i64::MAX),
        kind: record.kind.clone(),
        resource: record.resource.clone(),
        detail: record.detail.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
    };
    db.insert_event_and_prune(&persisted, EVENT_HISTORY_LIMIT as i32)
        .await
        .map_err(|error| format!("event persistence failed: {error}"))?;
    Ok(true)
}
