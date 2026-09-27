use super::*;

async fn send_ntfy_notification(
    options: &config::NtfyIntegrationSettings,
    title: &str,
    body: &str,
) -> Result<(), String> {
    let resolved = validate_integration_base_url(&options.url)?;
    send_ntfy_notification_to(options, title, body, &options.url, &resolved).await
}

pub(super) async fn send_ntfy_notification_to(
    options: &config::NtfyIntegrationSettings,
    title: &str,
    body: &str,
    endpoint: &str,
    resolved: &ResolvedIntegrationTarget,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve_to_addrs(&resolved.host, &resolved.addrs)
        .build()
        .map_err(|error| format!("failed to build Ntfy client: {error}"))?;
    let mut request = client
        .post(endpoint)
        .header("Title", format!("{}: {title}", options.notification_prefix))
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(body.to_owned());
    if !options.access_token.trim().is_empty() {
        request = request.bearer_auth(&options.access_token);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Ntfy request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Ntfy returned HTTP {}", response.status()));
    }
    Ok(())
}

pub(super) async fn send_pushover_notification(
    options: &config::PushoverIntegrationSettings,
    title: &str,
    body: &str,
    endpoint: &str,
) -> Result<(), String> {
    let prefixed_title = format!("{}: {title}", options.notification_prefix);
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build Pushover client: {error}"))?
        .post(endpoint)
        .form(&[
            ("token", options.token.as_str()),
            ("user", options.user_key.as_str()),
            ("title", prefixed_title.as_str()),
            ("message", body),
        ])
        .send()
        .await
        .map_err(|error| format!("Pushover request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Pushover returned HTTP {}", response.status()));
    }
    Ok(())
}

pub(super) async fn send_pushbullet_notification(
    options: &config::PushbulletIntegrationSettings,
    title: &str,
    body: &str,
    endpoint: &str,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("failed to build Pushbullet client: {error}"))?;
    let payload = serde_json::json!({"title": format!("{} {title}", options.notification_prefix), "body": body, "type": "note"});
    let attempts = options.retry_attempts.max(1);
    let mut last_error = String::new();
    for _ in 0..attempts {
        match client
            .post(endpoint)
            .header("Access-Token", &options.access_token)
            .header("User-Agent", format!("slskR v{APP_VERSION}"))
            .json(&payload)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => last_error = format!("Pushbullet returned HTTP {}", response.status()),
            Err(error) => last_error = format!("Pushbullet request failed: {error}"),
        }
    }
    Err(last_error)
}

pub(super) async fn send_private_message_notifications(
    state: &AppState,
    username: &str,
    message: &str,
) {
    let key = format!("notification:pm:{username}");
    if !state
        .private_message_auto_responses
        .write()
        .await
        .should_respond(&key, unix_timestamp(), 30)
    {
        return;
    }
    let integrations = state.integration_settings.read().await.clone();
    let title = format!("Private Message from {username}");
    if integrations.pushbullet.enabled && integrations.pushbullet.notify_on_private_message {
        if integrations.pushbullet.cooldown_time < 0 {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "pushbullet",
                "Pushbullet cooldown is negative".to_owned(),
            )
            .await;
            return;
        }
        let cooldown = u64::try_from(integrations.pushbullet.cooldown_time)
            .unwrap_or_default()
            .saturating_add(999)
            / 1_000;
        let push_key = format!("pushbullet:pm:{username}");
        if state
            .private_message_auto_responses
            .write()
            .await
            .should_respond(&push_key, unix_timestamp(), cooldown)
        {
            if let Err(error) = send_pushbullet_notification(
                &integrations.pushbullet,
                &title,
                message,
                "https://api.pushbullet.com/v2/pushes",
            )
            .await
            {
                record_daemon_log(state, logging::LogLevel::Warn, "pushbullet", error).await;
            }
        }
    }
    if integrations.ntfy.enabled && integrations.ntfy.notify_on_private_message {
        if let Err(error) = send_ntfy_notification(&integrations.ntfy, &title, message).await {
            record_daemon_log(state, logging::LogLevel::Error, "ntfy", error).await;
        }
    }
    if integrations.pushover.enabled && integrations.pushover.notify_on_private_message {
        if let Err(error) = send_pushover_notification(
            &integrations.pushover,
            &title,
            message,
            "https://api.pushover.net/1/messages.json",
        )
        .await
        {
            record_daemon_log(state, logging::LogLevel::Error, "pushover", error).await;
        }
    }
}

pub(super) async fn send_room_mention_notifications(
    state: &AppState,
    room: &str,
    username: &str,
    message: &str,
) {
    let key = format!("notification:room:{room}:{username}");
    if !state
        .private_message_auto_responses
        .write()
        .await
        .should_respond(&key, unix_timestamp(), 30)
    {
        return;
    }
    let integrations = state.integration_settings.read().await.clone();
    let title = format!("Room Mention by {username} in {room}");
    if integrations.pushbullet.enabled && integrations.pushbullet.notify_on_room_mention {
        if integrations.pushbullet.cooldown_time < 0 {
            record_daemon_log(
                state,
                logging::LogLevel::Warn,
                "pushbullet",
                "Pushbullet cooldown is negative".to_owned(),
            )
            .await;
            return;
        }
        let cooldown = u64::try_from(integrations.pushbullet.cooldown_time)
            .unwrap_or_default()
            .saturating_add(999)
            / 1_000;
        let push_key = format!("pushbullet:room:{room}");
        if state
            .private_message_auto_responses
            .write()
            .await
            .should_respond(&push_key, unix_timestamp(), cooldown)
        {
            if let Err(error) = send_pushbullet_notification(
                &integrations.pushbullet,
                &title,
                message,
                "https://api.pushbullet.com/v2/pushes",
            )
            .await
            {
                record_daemon_log(state, logging::LogLevel::Warn, "pushbullet", error).await;
            }
        }
    }
    if integrations.ntfy.enabled && integrations.ntfy.notify_on_room_mention {
        if let Err(error) = send_ntfy_notification(&integrations.ntfy, &title, message).await {
            record_daemon_log(state, logging::LogLevel::Error, "ntfy", error).await;
        }
    }
    if integrations.pushover.enabled && integrations.pushover.notify_on_room_mention {
        if let Err(error) = send_pushover_notification(
            &integrations.pushover,
            &title,
            message,
            "https://api.pushover.net/1/messages.json",
        )
        .await
        {
            record_daemon_log(state, logging::LogLevel::Error, "pushover", error).await;
        }
    }
}
