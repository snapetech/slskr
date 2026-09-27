use super::*;

pub(super) fn normalize_frozen_transfer_options_shape(
    response: &mut serde_json::Value,
    target: ControllerProfile,
) {
    match target {
        ControllerProfile::Legacy => {
            if let Some(global) = response
                .as_object_mut()
                .expect("options response is an object")
                .remove("global")
            {
                merge_json_objects(&mut response["transfers"], &global);
            }
        }
        ControllerProfile::Native => {
            let groups = response
                .get_mut("global")
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|global| global.remove("groups"));
            if let Some(groups) = groups {
                merge_json_objects(&mut response["groups"], &groups);
            }
            let upload_limits = response
                .pointer_mut("/global/upload")
                .and_then(serde_json::Value::as_object_mut)
                .and_then(|upload| upload.remove("limits"));
            if let Some(limits) = upload_limits {
                merge_json_objects(&mut response["global"]["limits"], &limits);
            }
            if let Some(groups) = response
                .get_mut("groups")
                .and_then(serde_json::Value::as_object_mut)
            {
                let move_limits = |group: &mut serde_json::Value| {
                    let limits = group
                        .get_mut("upload")
                        .and_then(serde_json::Value::as_object_mut)
                        .and_then(|upload| upload.remove("limits"));
                    if let Some(limits) = limits {
                        merge_json_objects(&mut group["limits"], &limits);
                    }
                };
                for (name, group) in groups.iter_mut() {
                    if name == "userDefined" {
                        if let Some(user_defined) = group.as_object_mut() {
                            for user_group in user_defined.values_mut() {
                                move_limits(user_group);
                            }
                        }
                    } else {
                        move_limits(group);
                    }
                }
            }
        }
    }

    fn materialize_null_windows(limits: &mut serde_json::Value) {
        let Some(limits) = limits.as_object_mut() else {
            return;
        };
        for window in ["queued", "daily", "weekly"] {
            if limits.get(window).is_some_and(serde_json::Value::is_null) {
                limits.insert(window.to_owned(), serde_json::json!({}));
            }
        }
    }

    match target {
        ControllerProfile::Legacy => {
            if let Some(limits) = response.pointer_mut("/transfers/upload/limits") {
                materialize_null_windows(limits);
            }
            if let Some(groups) = response
                .pointer_mut("/transfers/groups")
                .and_then(serde_json::Value::as_object_mut)
            {
                for (name, group) in groups.iter_mut() {
                    if name == "userDefined" {
                        if let Some(user_defined) = group.as_object_mut() {
                            for user_group in user_defined.values_mut() {
                                if let Some(limits) = user_group.pointer_mut("/upload/limits") {
                                    materialize_null_windows(limits);
                                }
                            }
                        }
                    } else if let Some(limits) = group.pointer_mut("/upload/limits") {
                        materialize_null_windows(limits);
                    }
                }
            }
        }
        ControllerProfile::Native => {
            if let Some(limits) = response.pointer_mut("/global/limits") {
                materialize_null_windows(limits);
            }
            if let Some(groups) = response
                .get_mut("groups")
                .and_then(serde_json::Value::as_object_mut)
            {
                for (name, group) in groups.iter_mut() {
                    if name == "userDefined" {
                        if let Some(user_defined) = group.as_object_mut() {
                            for user_group in user_defined.values_mut() {
                                if let Some(limits) = user_group.get_mut("limits") {
                                    materialize_null_windows(limits);
                                }
                            }
                        }
                    } else if let Some(limits) = group.get_mut("limits") {
                        materialize_null_windows(limits);
                    }
                }
            }
        }
    }
}

pub(super) fn transfer_limit_options_json(
    limit: &Option<crate::config::TransferLimitSettings>,
) -> serde_json::Value {
    let Some(limit) = limit else {
        return serde_json::Value::Null;
    };
    let mut value = serde_json::Map::new();
    if let Some(files) = limit.files {
        value.insert("files".to_owned(), serde_json::json!(files));
    }
    if let Some(megabytes) = limit.megabytes {
        value.insert("megabytes".to_owned(), serde_json::json!(megabytes));
    }
    if let Some(failures) = limit.failures {
        value.insert("failures".to_owned(), serde_json::json!(failures));
    }
    serde_json::Value::Object(value)
}

pub(super) fn transfer_download_options_json(
    config: &AppConfig,
    native_profile: bool,
) -> serde_json::Value {
    let download = &config.transfer_download;
    let auto_replace_threshold = if download.auto_replace_threshold_percent.fract() == 0.0 {
        serde_json::json!(download.auto_replace_threshold_percent as u64)
    } else {
        serde_json::json!(download.auto_replace_threshold_percent)
    };
    let auto_retry_size_tolerance = if config
        .transfer_auto_retry
        .alternate_source_size_tolerance_percent
        .fract()
        == 0.0
    {
        serde_json::json!(
            config
                .transfer_auto_retry
                .alternate_source_size_tolerance_percent as u64
        )
    } else {
        serde_json::json!(
            config
                .transfer_auto_retry
                .alternate_source_size_tolerance_percent
        )
    };
    let retry = if native_profile {
        serde_json::json!({
            "incomplete": download.retry.incomplete,
            "attempts": download.retry.attempts,
            "delay": download.retry.delay.as_millis(),
            "maxDelay": download.retry.max_delay.as_millis(),
        })
    } else {
        serde_json::json!({
            "partial": download.retry.incomplete,
            "attempts": download.retry.attempts,
            "delay": download.retry.delay.as_millis(),
            "maxDelay": download.retry.max_delay.as_millis(),
        })
    };
    if native_profile {
        serde_json::json!({
            "slots": download.slots,
            "speedLimit": download.speed_limit_kib,
            "retry": retry,
            "completedLayout": download.completed_layout,
            "completedPathTemplate": config.download_completed_path_template,
            "autoReplaceStuck": download.auto_replace_stuck,
            "autoReplaceThreshold": auto_replace_threshold,
            "autoReplaceInterval": download.auto_replace_interval.as_secs(),
            "autoRetry": {
                "enabled": config.transfer_auto_retry.enabled,
                "retryDelaySeconds": config.transfer_auto_retry.retry_delay.as_secs(),
                "checkIntervalSeconds": config.transfer_auto_retry.check_interval.as_secs(),
                "maxAttempts": config.transfer_auto_retry.max_attempts,
                "maxFilesPerCycle": config.transfer_auto_retry.max_files_per_cycle,
                "maxFilesPerPeerPerCycle": config.transfer_auto_retry.max_files_per_peer_per_cycle,
                "peerCooldownSeconds": config.transfer_auto_retry.peer_cooldown.as_secs(),
                "alternateSourcesEnabled": config.transfer_auto_retry.alternate_sources_enabled,
                "maxAlternateSourceSearchesPerCycle": config.transfer_auto_retry.max_alternate_source_searches_per_cycle,
                "alternateSourceSizeTolerancePercent": auto_retry_size_tolerance,
            },
        })
    } else {
        let mut permissions = serde_json::Map::new();
        if let Some(mode) = download.destination.permissions_mode.as_ref() {
            permissions.insert("mode".to_owned(), serde_json::json!(mode));
        }
        serde_json::json!({
            "slots": download.slots,
            "speedLimit": download.speed_limit_kib,
            "retry": retry,
            "destination": {
                "subdirectory": download.destination.subdirectory,
                "exists": download.destination.exists,
                "permissions": permissions,
            },
        })
    }
}

pub(super) fn transfer_limits_options_json(
    limits: &crate::config::TransferLimitsSettings,
) -> serde_json::Value {
    serde_json::json!({
        "queued": transfer_limit_options_json(&limits.queued),
        "daily": transfer_limit_options_json(&limits.daily),
        "weekly": transfer_limit_options_json(&limits.weekly),
    })
}

pub(super) fn transfer_group_options_json(
    upload: &crate::config::TransferGroupUploadSettings,
    native_profile: bool,
) -> serde_json::Value {
    let mut upload_json = serde_json::json!({
        "priority": upload.priority,
        "strategy": upload.strategy.as_frozen_str(),
        "slots": upload.slots,
        "speedLimit": upload.speed_limit_kib,
    });
    let limits = transfer_limits_options_json(&upload.limits);
    if native_profile {
        upload_json["allowedFileTypes"] = serde_json::json!(upload.allowed_file_types);
        serde_json::json!({"upload": upload_json, "limits": limits})
    } else {
        upload_json["limits"] = limits;
        serde_json::json!({"upload": upload_json})
    }
}

pub(super) fn transfer_groups_options_json(
    config: &AppConfig,
    native_profile: bool,
) -> serde_json::Value {
    let groups = &config.transfer_groups;
    let default = transfer_group_options_json(&groups.default.upload, native_profile);
    let mut leechers = transfer_group_options_json(&groups.leechers.upload, native_profile);
    leechers["thresholds"] = serde_json::json!({
        "files": groups.leechers.threshold_files,
        "directories": groups.leechers.threshold_directories,
    });
    let mut user_defined = serde_json::Map::new();
    for (name, group) in &groups.user_defined {
        let mut projected = transfer_group_options_json(&group.upload, native_profile);
        projected["members"] = serde_json::json!(group.members);
        user_defined.insert(name.clone(), projected);
    }
    serde_json::json!({
        "default": default,
        "leechers": leechers,
        "blacklisted": {
            "members": config.managed_blacklist.members,
            "patterns": config.managed_blacklist.patterns,
            "cidrs": config.managed_blacklist.cidr_values,
        },
        "userDefined": user_defined,
    })
}

pub(super) const MAX_CONTROLLER_YAML_BYTES: usize = 1024 * 1024;
pub(super) const MAX_CONTROLLER_YAML_DEPTH: usize = 64;
pub(super) const MAX_CONTROLLER_YAML_NODES: usize = 65_536;

pub(super) fn merge_json_objects(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(target), serde_json::Value::Object(patch)) => {
            for (key, value) in patch {
                merge_json_objects(
                    target.entry(key.clone()).or_insert(serde_json::Value::Null),
                    value,
                );
            }
        }
        (target, patch) => *target = patch.clone(),
    }
}

pub(super) fn native_adversarial_yaml_update(
    current: &str,
    payload: &serde_json::Value,
) -> Result<String, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "Settings cannot be null".to_owned())?;
    let default_yaml = NATIVE_DEFAULT_ADVERSARIAL_YAML.replace("__NATIVE_EMPTY__", "");
    let mut rendered = default_yaml
        .trim_end_matches(['\r', '\n'])
        .strip_suffix("...")
        .unwrap_or(&default_yaml)
        .trim_end_matches(['\r', '\n'])
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    apply_native_adversarial_json_object(
        &mut rendered,
        &["security".to_owned(), "adversarial".to_owned()],
        object,
    );

    let mut current_lines = current
        .trim_end_matches(['\r', '\n'])
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if current_lines
        .last()
        .is_some_and(|line| line.trim() == "...")
    {
        current_lines.pop();
    }
    while current_lines.last().is_some_and(|line| line.is_empty()) {
        current_lines.pop();
    }

    let security_index = current_lines.iter().position(|line| line == "security:");
    let rendered_adversarial = rendered.iter().skip(1).cloned().collect::<Vec<_>>();
    match security_index {
        None => {
            current_lines.extend(rendered);
        }
        Some(security_index) => {
            let security_end = current_lines
                .iter()
                .enumerate()
                .skip(security_index + 1)
                .find(|(_, line)| !line.is_empty() && yaml_indentation(line) == 0)
                .map(|(index, _)| index)
                .unwrap_or(current_lines.len());
            let adversarial_index = current_lines
                .iter()
                .enumerate()
                .take(security_end)
                .skip(security_index + 1)
                .find(|(_, line)| *line == "  adversarial:")
                .map(|(index, _)| index);
            if let Some(adversarial_index) = adversarial_index {
                let adversarial_end = current_lines
                    .iter()
                    .enumerate()
                    .take(security_end)
                    .skip(adversarial_index + 1)
                    .find(|(_, line)| !line.is_empty() && yaml_indentation(line) <= 2)
                    .map(|(index, _)| index)
                    .unwrap_or(security_end);
                current_lines.splice(adversarial_index..adversarial_end, rendered_adversarial);
            } else {
                current_lines.splice(security_end..security_end, rendered_adversarial);
            }
        }
    }
    let mut output = current_lines.join("\n");
    output.push_str("\n...\n");
    Ok(output)
}

pub(super) fn yaml_indentation(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

pub(super) fn apply_native_adversarial_json_object(
    lines: &mut Vec<String>,
    parent_path: &[String],
    object: &serde_json::Map<String, serde_json::Value>,
) {
    for (key, value) in object {
        let mut path = parent_path.to_vec();
        path.push(camel_to_snake_case(key));
        match value {
            serde_json::Value::Object(nested) if nested.is_empty() => {}
            serde_json::Value::Object(nested) => {
                if !replace_native_yaml_value(lines, &path, value) {
                    apply_native_adversarial_json_object(lines, &path, nested);
                }
            }
            _ => {
                replace_native_yaml_value(lines, &path, value);
            }
        }
    }
}

pub(super) fn replace_native_yaml_value(
    lines: &mut Vec<String>,
    target_path: &[String],
    value: &serde_json::Value,
) -> bool {
    let mut stack = Vec::<(usize, String)>::new();
    for index in 0..lines.len() {
        let line = &lines[index];
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('-') || matches!(trimmed, "---" | "...") {
            continue;
        }
        let Some((key, _)) = trimmed.split_once(':') else {
            continue;
        };
        let indentation = yaml_indentation(line);
        while stack
            .last()
            .is_some_and(|(ancestor_indentation, _)| *ancestor_indentation >= indentation)
        {
            stack.pop();
        }
        let mut path = stack.iter().map(|(_, key)| key.clone()).collect::<Vec<_>>();
        path.push(key.to_owned());
        if path == target_path {
            if value.is_object()
                && lines
                    .iter()
                    .skip(index + 1)
                    .find(|candidate| !candidate.trim().is_empty())
                    .is_some_and(|candidate| yaml_indentation(candidate) > indentation)
            {
                return false;
            }
            let end = lines
                .iter()
                .enumerate()
                .skip(index + 1)
                .find(|(_, candidate)| {
                    let candidate = candidate.as_str();
                    !candidate.trim().is_empty()
                        && !candidate.trim_start().starts_with('-')
                        && yaml_indentation(candidate) <= indentation
                })
                .map(|(end, _)| end)
                .unwrap_or(lines.len());
            let replacement = native_yaml_value_lines(key, indentation, target_path, value);
            lines.splice(index..end, replacement);
            return true;
        }
        stack.push((indentation, key.to_owned()));
    }
    false
}

pub(super) fn native_yaml_value_lines(
    key: &str,
    indentation: usize,
    path: &[String],
    value: &serde_json::Value,
) -> Vec<String> {
    let prefix = " ".repeat(indentation);
    match value {
        serde_json::Value::Array(values) if values.is_empty() => {
            vec![format!("{prefix}{key}: []")]
        }
        serde_json::Value::Array(values) => {
            let mut output = vec![format!("{prefix}{key}:")];
            output.extend(values.iter().map(|value| {
                format!(
                    "{prefix}- {}",
                    native_yaml_scalar(path, value).unwrap_or_default()
                )
            }));
            output
        }
        serde_json::Value::Object(object) if object.is_empty() => {
            vec![format!("{prefix}{key}: {{}}")]
        }
        serde_json::Value::Object(object) => {
            let mut output = vec![format!("{prefix}{key}:")];
            for (child_key, child_value) in object {
                let child_key = camel_to_snake_case(child_key);
                let child_path = path
                    .iter()
                    .cloned()
                    .chain(std::iter::once(child_key.clone()))
                    .collect::<Vec<_>>();
                output.extend(native_yaml_value_lines(
                    &child_key,
                    indentation + 2,
                    &child_path,
                    child_value,
                ));
            }
            output
        }
        _ => vec![format!(
            "{prefix}{key}: {}",
            native_yaml_scalar(path, value).unwrap_or_default()
        )],
    }
}

pub(super) fn native_yaml_scalar(path: &[String], value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => Some(String::new()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::String(value) => {
            let enum_value = if path.last().is_some_and(|key| {
                matches!(
                    key.as_str(),
                    "profile" | "mode" | "primary_transport" | "isolation_method"
                )
            }) || path.last().is_some_and(|key| key == "preference_order")
            {
                [
                    "Disabled",
                    "Standard",
                    "Enhanced",
                    "Maximum",
                    "Custom",
                    "Direct",
                    "Tor",
                    "I2P",
                    "RelayOnly",
                    "Auto",
                    "WebSocket",
                    "HttpTunnel",
                    "Obfs4",
                    "Meek",
                    "DirectQuic",
                    "TorOnionQuic",
                    "I2PQuic",
                    "SocksAuth",
                ]
                .into_iter()
                .find(|candidate| candidate.eq_ignore_ascii_case(value))
                .unwrap_or(value)
            } else {
                value
            };
            if enum_value.is_empty() {
                Some("''".to_owned())
            } else if enum_value.starts_with([
                '-', '?', ':', '!', '&', '*', '#', '{', '}', '[', ']', ',', '|', '>', '@', '`',
            ]) || enum_value.ends_with(char::is_whitespace)
                || enum_value.starts_with(char::is_whitespace)
                || enum_value.contains(": ")
                || enum_value.contains(" #")
                || matches!(
                    enum_value.to_ascii_lowercase().as_str(),
                    "true" | "false" | "null" | "~"
                )
                || enum_value.parse::<f64>().is_ok()
            {
                Some(format!("'{}'", enum_value.replace('\'', "''")))
            } else {
                Some(enum_value.to_owned())
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
    }
}

pub(super) fn validate_native_adversarial_settings(
    payload: &serde_json::Value,
) -> Result<(), &'static str> {
    let bucket_sizes = payload
        .pointer("/privacy/padding/bucketSizes")
        .or_else(|| payload.pointer("/privacy/padding/bucket_sizes"));
    if bucket_sizes
        .and_then(serde_json::Value::as_array)
        .is_some_and(|values| values.iter().any(|value| value.as_i64().unwrap_or(0) <= 0))
    {
        return Err("Bucket sizes must be positive");
    }
    Ok(())
}

pub(super) async fn apply_controller_yaml_upload(body: &str, state: &AppState) -> HttpResponse {
    let text = match controller_options_config_body(body) {
        Ok(text) => text,
        Err(error) => return routing::bad_request_response(&error),
    };
    if text.len() > MAX_CONTROLLER_YAML_BYTES {
        return HttpResponse {
            status: "413 Payload Too Large",
            content_type: "application/json",
            body: serde_json::json!("YAML is too large").to_string(),
        };
    }
    let parsed = match parse_controller_yaml(&text) {
        Ok(parsed) => parsed,
        Err(error) => return routing::bad_request_response(&error),
    };
    if let Some(error) =
        controller_yaml_target_validation_error(&parsed, state.config.controller_profile)
    {
        return HttpResponse {
            status: "400 Bad Request",
            content_type: "application/json; charset=utf-8",
            body: serde_json::Value::String(error).to_string(),
        };
    }
    if let Err(error) = write_controller_compatibility_yaml(&state.config, &text) {
        eprintln!("configuration update failed: {error}");
        return routing::internal_server_error_response("Failed to update configuration file");
    }
    apply_watched_controller_configuration(state, Some(&text), &state.controller_cli_environment)
        .await;
    if effective_controller_no_config_watch(state) {
        state.runtime.write().await.set_restart_requested(true);
    }
    record_event(
        state,
        "options.yaml.updated",
        "options/yaml",
        Some(format!("bytes={}", text.len())),
    )
    .await;
    HttpResponse {
        status: "200 OK",
        content_type: "",
        body: String::new(),
    }
}

pub(super) fn controller_options_config_validate_response(
    body: &str,
    target: ControllerProfile,
) -> Result<HttpResponse, String> {
    let text = controller_options_config_body(body)?;
    let parsed = parse_controller_yaml(&text);
    let parse_failed = parsed.is_err();
    let validation = parsed
        .as_ref()
        .err()
        .cloned()
        .or_else(|| controller_yaml_target_validation_error(parsed.as_ref().ok()?, target));
    let (content_type, body) = match validation {
        None => ("", String::new()),
        Some(error) => {
            let error = if target == ControllerProfile::Legacy
                && parse_failed
                && text.trim_start().starts_with("web:")
            {
                "No node deserializer was able to deserialize the node into type slskd.Options+WebOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null".to_owned()
            } else {
                error
            };
            (
                "application/json; charset=utf-8",
                serde_json::Value::String(error).to_string(),
            )
        }
    };
    Ok(HttpResponse {
        status: "200 OK",
        content_type,
        body,
    })
}

pub(super) fn json_object_field_ci<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Option<&'a serde_json::Value> {
    object
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}

pub(super) fn options_model_binding_problem_response() -> HttpResponse {
    HttpResponse {
        status: "400 Bad Request",
        content_type: "application/json; charset=utf-8",
        body: r#"{"title":"One or more validation errors occurred.","status":400,"detail":"The request is invalid.","errors":{}}"#.to_owned(),
    }
}

pub(super) fn normalize_controller_options_overlay(
    body: &str,
    target: ControllerProfile,
) -> Result<serde_json::Value, String> {
    let payload = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|_| "Invalid options overlay".to_owned())?;
    let object = payload
        .as_object()
        .ok_or_else(|| "Options overlay is required".to_owned())?;
    let mut normalized = serde_json::Map::new();

    if let Some(soulseek) = json_object_field_ci(object, "soulseek") {
        let soulseek = soulseek
            .as_object()
            .ok_or_else(|| "Invalid options overlay".to_owned())?;
        let mut normalized_soulseek = serde_json::Map::new();
        if let Some(address) = json_object_field_ci(soulseek, "listenIpAddress") {
            let address = address
                .as_str()
                .filter(|value| value.parse::<IpAddr>().is_ok())
                .ok_or_else(|| "Invalid options overlay".to_owned())?;
            normalized_soulseek.insert(
                "listenIpAddress".to_owned(),
                serde_json::Value::String(address.to_owned()),
            );
        }
        if let Some(port) = json_object_field_ci(soulseek, "listenPort") {
            let port = port
                .as_u64()
                .filter(|port| (1024..=65535).contains(port))
                .ok_or_else(|| "Invalid options overlay".to_owned())?;
            normalized_soulseek.insert("listenPort".to_owned(), serde_json::json!(port));
        }
        if target == ControllerProfile::Native {
            if let Some(auto_response) =
                json_object_field_ci(soulseek, "privateMessageAutoResponse")
            {
                let auto_response = auto_response
                    .as_object()
                    .ok_or_else(|| "Invalid options overlay".to_owned())?;
                if let Some(enabled) = json_object_field_ci(auto_response, "enabled") {
                    let enabled = enabled
                        .as_bool()
                        .ok_or_else(|| "Invalid options overlay".to_owned())?;
                    normalized_soulseek.insert(
                        "privateMessageAutoResponse".to_owned(),
                        serde_json::json!({"enabled": enabled}),
                    );
                }
            }
        }
        if !normalized_soulseek.is_empty() {
            normalized.insert(
                "soulseek".to_owned(),
                serde_json::Value::Object(normalized_soulseek),
            );
        }
    }

    if target == ControllerProfile::Native {
        if let Some(integration) = json_object_field_ci(object, "integration") {
            if !integration.is_object() {
                return Err("Invalid options overlay".to_owned());
            }
            normalized.insert(
                "integration".to_owned(),
                controller_yaml_api_projection(integration.clone()),
            );
        }
    }
    Ok(serde_json::Value::Object(normalized))
}

pub(super) async fn apply_controller_options_overlay(body: &str, state: &AppState) -> HttpResponse {
    let overlay = match normalize_controller_options_overlay(body, state.config.controller_profile)
    {
        Ok(overlay) => overlay,
        Err(error) => return routing::bad_request_response(&error),
    };
    let listen_changed = overlay
        .get("soulseek")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|soulseek| {
            soulseek.contains_key("listenIpAddress") || soulseek.contains_key("listenPort")
        });
    if let Some(enabled) = overlay
        .pointer("/soulseek/privateMessageAutoResponse/enabled")
        .and_then(serde_json::Value::as_bool)
    {
        state
            .private_message_auto_response_settings
            .write()
            .await
            .enabled = enabled;
        if !enabled {
            *state.private_message_auto_responses.write().await =
                PrivateMessageAutoResponseTracker::default();
        }
    }
    state.options_overlay.write().await.apply(overlay.clone());
    if listen_changed
        && state.config.controller_profile == ControllerProfile::Native
        && state.session.read().await.state == "connected"
    {
        state.runtime.write().await.set_reconnect_pending(true);
    }
    record_event(
        state,
        "options.updated",
        "options",
        Some("volatile=true".to_owned()),
    )
    .await;
    HttpResponse {
        status: "200 OK",
        content_type: "application/json; charset=utf-8",
        body: overlay.to_string(),
    }
}

pub(super) fn controller_options_mutation_response(
    body: &str,
    acknowledgements: u64,
    acknowledgement_persistence_enabled: bool,
) -> Result<String, String> {
    let payload = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|error| format!("invalid options JSON: {error}"))?;
    let Some(object) = payload.as_object() else {
        return Err("options payload must be a JSON object".to_owned());
    };
    let accepted_keys = object.keys().cloned().collect::<Vec<_>>();
    Ok(serde_json::json!({
        "status": "accepted",
        "persisted": acknowledgement_persistence_enabled,
        "configPersisted": false,
        "restart_required": false,
        "runtimeMutationEnabled": false,
        "acceptedKeys": accepted_keys,
        "acknowledgements": acknowledgements,
        "note": "runtime option mutation is not enabled",
        "message": if acknowledgement_persistence_enabled {
            "persisted compatibility acknowledgement"
        } else {
            "non-persisted compatibility acknowledgement"
        },
        "compatibility": {
            "surface": "slskd-options",
            "mutationPersistence": if acknowledgement_persistence_enabled {
                "runtime_compat_state"
            } else {
                "none"
            },
        },
    })
    .to_string())
}

pub(super) fn controller_server_state_json(
    session: &SessionSnapshot,
    config: &AppConfig,
    runtime_credentials_configured: bool,
    connected_endpoint: Option<&str>,
) -> serde_json::Value {
    let connected = session.state == "connected";
    let state = match session.state {
        "connected" => "Connected, LoggedIn",
        "connecting" => "Connecting",
        "disconnecting" => "Disconnecting",
        "error" => "Error",
        _ => "Disconnected",
    };
    let config_credentials_configured = config.credentials().is_some();
    let credential_source = if runtime_credentials_configured {
        "runtime"
    } else if config_credentials_configured {
        "config"
    } else {
        "none"
    };
    let connected_endpoint = connected.then_some(connected_endpoint).flatten();
    let connected_address = connected_endpoint.and_then(|endpoint| {
        endpoint
            .rsplit_once(':')
            .map(|(address, _)| address.trim_matches(['[', ']']))
    });
    if config.controller_profile == ControllerProfile::Legacy {
        let mut response = serde_json::json!({
            "state": state,
            "isConnected": connected,
            "isLoggedIn": connected,
            "isTransitioning": session.state == "connecting" || session.state == "disconnecting",
        });
        if connected {
            response["address"] = serde_json::json!(connected_address);
            response["ipEndPoint"] = serde_json::json!(connected_endpoint);
        }
        return response;
    }
    let mut response = serde_json::json!({
        "credentialsConfigured": runtime_credentials_configured || config_credentials_configured,
        "credentialSource": credential_source,
        "credentialStore": config.credential_store.as_str(),
        "credentialStoreModes": credential_store::supported_store_modes(),
        "writableCredentialStoreModes": credential_store::writable_store_modes(),
        "state": state,
        "isConnected": connected,
        "isConnecting": session.state == "connecting",
        "isDisconnecting": session.state == "disconnecting",
        "isLoggedIn": connected,
        "isLoggingIn": session.state == "connecting",
        "isTransitioning": session.state == "connecting" || session.state == "disconnecting",
        "lastError": public_session_error(session.last_error.as_deref()),
        "runtimeCredentialsConfigured": runtime_credentials_configured,
    });
    let object = response
        .as_object_mut()
        .expect("server compatibility response must be an object");
    if connected {
        object.insert("address".to_owned(), serde_json::json!(connected_address));
        object.insert(
            "ipEndPoint".to_owned(),
            serde_json::json!(connected_endpoint),
        );
    } else if config.controller_profile == ControllerProfile::Native {
        object.insert("address".to_owned(), serde_json::json!(""));
        object.insert(
            "ipEndPoint".to_owned(),
            serde_json::json!("255.255.255.255:0"),
        );
    }
    response
}
