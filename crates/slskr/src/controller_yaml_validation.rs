use super::*;

fn controller_web_authentication_validation_error(
    value: &serde_json::Value,
    target: ControllerProfile,
) -> Option<String> {
    let target_error = |controller_error: String| match target {
        ControllerProfile::Legacy => controller_error,
        ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
    };
    let authentication = match value
        .get("web")
        .and_then(serde_json::Value::as_object)
        .and_then(|web| web.get("authentication"))
    {
        None | Some(serde_json::Value::Null) => return None,
        Some(authentication) => authentication.as_object()?,
    };
    let scalar = |candidate: &serde_json::Value| match candidate {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    };
    let mut invalid_credentials = Vec::new();
    for (key, field) in [("username", "Username"), ("password", "Password")] {
        let Some(candidate) = authentication.get(key) else {
            continue;
        };
        if candidate.is_null() {
            continue;
        }
        let Some(value) = scalar(candidate) else {
            return Some(target_error(
                "No node deserializer was able to deserialize the node into type System.String, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e"
                    .to_owned(),
            ));
        };
        if !(1..=255).contains(&value.encode_utf16().count()) {
            invalid_credentials.push(format!(
                "      The field {field} must be a string with a minimum length of 1 and a maximum length of 255."
            ));
        }
    }
    if !invalid_credentials.is_empty() {
        return Some(target_error(format!(
            "Invalid configuration:\n  Web:\n    Authentication:\n{}",
            invalid_credentials.join("\n")
        )));
    }
    let jwt = match authentication.get("jwt") {
        None | Some(serde_json::Value::Null) => return None,
        Some(jwt) => {
            let Some(jwt) = jwt.as_object() else {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type slskd.Options+WebOptions+WebAuthenticationOptions+JwtOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                        .to_owned(),
                ));
            };
            jwt
        }
    };
    if let Some(key) = jwt.get("key") {
        if !key.is_null() {
            let Some(key) = scalar(key) else {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type System.String, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e"
                        .to_owned(),
                ));
            };
            if !(32..=255).contains(&key.encode_utf16().count()) {
                return Some(target_error(
                    "Invalid configuration:\n  Web:\n    Authentication:\n      Jwt:\n        The field Key must be a string with a minimum length of 32 and a maximum length of 255."
                        .to_owned(),
                ));
            }
        }
    }
    if let Some(ttl) = jwt.get("ttl") {
        let parsed = match ttl {
            serde_json::Value::Null => Some(0_i64),
            serde_json::Value::Number(value) => value.as_i64(),
            serde_json::Value::String(value) => value.parse::<i64>().ok(),
            _ => None,
        };
        if parsed.is_some_and(|ttl| ttl > i32::MAX as i64) {
            return Some(target_error(
                "Exception during deserialization: Arithmetic operation resulted in an overflow."
                    .to_owned(),
            ));
        }
        if !parsed.is_some_and(|ttl| (3_600..=i32::MAX as i64).contains(&ttl)) {
            return Some(target_error(
                "Invalid configuration:\n  Web:\n    Authentication:\n      Jwt:\n        The field Ttl must be between 3600 and 2147483647."
                    .to_owned(),
            ));
        }
    }
    None
}

fn controller_transfer_validation_error(
    value: &serde_json::Value,
    target: ControllerProfile,
) -> Option<String> {
    // Frozen native profile accepts the documented `transfers.download` section but
    // rejects legacy `transfers.groups`; groups validate only at the top level.
    if target == ControllerProfile::Native && value.pointer("/transfers/groups").is_some() {
        return Some("Invalid YAML configuration".to_owned());
    }
    let target_error = |controller_error: String| match target {
        ControllerProfile::Legacy => controller_error,
        ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
    };
    let range_invalid = |candidate: &serde_json::Value| {
        !candidate.is_null()
            && !candidate
                .as_i64()
                .is_some_and(|value| (1..=i32::MAX as i64).contains(&value))
    };
    let field_error = |prefix: &str, field: &str| {
        target_error(format!(
            "{prefix}The field {field} must be between 1 and 2147483647."
        ))
    };

    let transfers = value
        .get("transfers")
        .and_then(serde_json::Value::as_object);
    let global = value.get("global").and_then(serde_json::Value::as_object);
    let upload = transfers
        .and_then(|transfers| transfers.get("upload"))
        .or_else(|| global.and_then(|global| global.get("upload")))
        .and_then(serde_json::Value::as_object);
    if let Some(upload) = upload {
        for (key, field) in [("slots", "Slots"), ("speed_limit", "SpeedLimit")] {
            if upload.get(key).is_some_and(range_invalid) {
                return Some(field_error(
                    "Invalid configuration:\n  Transfers:\n    Upload:\n      ",
                    field,
                ));
            }
        }
        if let Some(error) = controller_transfer_limits_validation_error(
            upload.get("limits"),
            "Invalid configuration:\n  Transfers:\n    Upload:\n      Limits:\n",
            target,
        ) {
            return Some(error);
        }
    }

    let download = transfers
        .and_then(|transfers| transfers.get("download"))
        .or_else(|| global.and_then(|global| global.get("download")))
        .and_then(serde_json::Value::as_object);
    if let Some(download) = download {
        let prefix = if target == ControllerProfile::Legacy {
            "Invalid configuration:\n  Transfers:\n    Download:\n      "
        } else {
            "Invalid configuration:\n  Global:\n    Download:\n      "
        };
        for (key, field) in [("slots", "Slots"), ("speed_limit", "SpeedLimit")] {
            if download.get(key).is_some_and(range_invalid) {
                return Some(field_error(prefix, field));
            }
        }
        if let Some(retry) = download.get("retry") {
            let Some(retry) = retry.as_object() else {
                return Some(target_error("Invalid configuration".to_owned()));
            };
            let (strategy_key, strategy_field) = if target == ControllerProfile::Legacy {
                ("partial", "Partial")
            } else {
                ("incomplete", "Incomplete")
            };
            if let Some(strategy) = retry.get(strategy_key) {
                let valid = strategy.as_str().is_some_and(|strategy| {
                    strategy.eq_ignore_ascii_case("overwrite")
                        || strategy.eq_ignore_ascii_case("resume")
                });
                if !strategy.is_null() && !valid {
                    return Some(target_error(format!(
                        "{prefix}Retry:\n        The {strategy_field} field must be one of: Overwrite, Resume. Case insensitive."
                    )));
                }
            }
            let retry_range = |key: &str, minimum: i64, maximum: i64| {
                retry.get(key).is_some_and(|candidate| {
                    !candidate.is_null()
                        && !candidate
                            .as_i64()
                            .is_some_and(|value| (minimum..=maximum).contains(&value))
                })
            };
            for (key, field, minimum, maximum) in if target == ControllerProfile::Legacy {
                [
                    ("attempts", "Attempts", 1, i32::MAX as i64),
                    ("delay", "Delay", 1_000, i32::MAX as i64),
                    ("max_delay", "MaxDelay", 30_000, i32::MAX as i64),
                ]
            } else {
                [
                    ("attempts", "Attempts", 1, 20),
                    ("delay", "Delay", 0, 3_600_000),
                    ("max_delay", "MaxDelay", 1_000, 86_400_000),
                ]
            } {
                if retry_range(key, minimum, maximum) {
                    return Some(target_error(format!(
                        "{prefix}Retry:\n        The field {field} must be between {minimum} and {maximum}."
                    )));
                }
            }
        }
        if target == ControllerProfile::Legacy {
            if let Some(destination) = download
                .get("destination")
                .and_then(serde_json::Value::as_object)
            {
                if let Some(subdirectory) = destination
                    .get("subdirectory")
                    .and_then(serde_json::Value::as_str)
                {
                    if subdirectory
                        .split(['/', '\\'])
                        .any(|component| matches!(component, "." | ".."))
                    {
                        return Some(format!(
                            "{prefix}Destination:\n        The Subdirectory field contains one or more unsafe path traversal segments ('.' or '..')"
                        ));
                    }
                }
                if let Some(exists) = destination.get("exists") {
                    let valid = exists.as_str().is_some_and(|exists| {
                        exists.eq_ignore_ascii_case("overwrite")
                            || exists.eq_ignore_ascii_case("rename")
                    });
                    if !exists.is_null() && !valid {
                        return Some(format!(
                            "{prefix}Destination:\n        The Exists field must be one of: Overwrite, Rename. Case insensitive."
                        ));
                    }
                }
                if let Some(mode) = destination
                    .get("permissions")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|permissions| permissions.get("mode"))
                    .and_then(serde_json::Value::as_str)
                {
                    let valid = matches!(mode.len(), 3 | 4)
                        && mode.bytes().all(|value| matches!(value, b'0'..=b'7'));
                    if !valid {
                        return Some(format!(
                            "{prefix}Destination:\n        Permissions:\n          Field Mode is invalid. Specify a three- or four-character string consisting of only 0-7 (chmod syntax, [0]000-[7]777, inclusive)"
                        ));
                    }
                }
            }
        }
        if target == ControllerProfile::Native {
            for (key, minimum, maximum) in [
                ("auto_replace_threshold", 0.1_f64, 50.0_f64),
                ("auto_replace_interval", 10.0_f64, 3_600.0_f64),
            ] {
                if let Some(candidate) = download.get(key) {
                    if !candidate
                        .as_f64()
                        .is_some_and(|value| (minimum..=maximum).contains(&value))
                    {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                }
            }
        }
    }

    let groups = transfers
        .and_then(|transfers| transfers.get("groups"))
        .or_else(|| value.get("groups"))
        .and_then(serde_json::Value::as_object);
    let groups = groups?;

    let validate_group = |name: &str,
                          group: &serde_json::Value,
                          display_name: &str|
     -> Option<String> {
        let group = group.as_object()?;
        let upload = group.get("upload").and_then(serde_json::Value::as_object);
        let prefix = format!(
                "Invalid configuration:\n  Transfers:\n    Groups:\n      {display_name}:\n        Upload:\n          "
            );
        if let Some(upload) = upload {
            for (key, field) in [
                ("priority", "Priority"),
                ("slots", "Slots"),
                ("speed_limit", "SpeedLimit"),
            ] {
                if upload.get(key).is_some_and(range_invalid) {
                    return Some(field_error(&prefix, field));
                }
            }
            if let Some(strategy) = upload.get("strategy") {
                let valid = strategy.as_str().is_some_and(|strategy| {
                    strategy.eq_ignore_ascii_case("roundrobin")
                        || strategy.eq_ignore_ascii_case("firstinfirstout")
                });
                if !strategy.is_null() && !valid {
                    return Some(target_error(format!(
                            "{prefix}The Strategy field must be one of: RoundRobin, FirstInFirstOut. Case insensitive."
                        )));
                }
            }
            if let Some(error) = controller_transfer_limits_validation_error(
                upload.get("limits"),
                &format!("{prefix}Limits:\n"),
                target,
            ) {
                return Some(error);
            }
        }
        if let Some(error) = controller_transfer_limits_validation_error(
                group.get("limits"),
                &format!(
                    "Invalid configuration:\n  Transfers:\n    Groups:\n      {display_name}:\n        Limits:\n"
                ),
                target,
            ) {
                return Some(error);
            }
        if name == "leechers" {
            if let Some(thresholds) = group
                .get("thresholds")
                .and_then(serde_json::Value::as_object)
            {
                for (key, field) in [("files", "Files"), ("directories", "Directories")] {
                    if thresholds.get(key).is_some_and(range_invalid) {
                        return Some(field_error(
                                "Invalid configuration:\n  Transfers:\n    Groups:\n      Leechers:\n        Thresholds:\n          ",
                                field,
                            ));
                    }
                }
            }
        }
        None
    };

    for (name, display_name) in [
        ("default", "Default"),
        ("leechers", "Leechers"),
        ("blacklisted", "Blacklisted"),
    ] {
        if let Some(group) = groups.get(name) {
            if let Some(error) = validate_group(name, group, display_name) {
                return Some(error);
            }
        }
    }
    if let Some(user_defined) = groups
        .get("user_defined")
        .and_then(serde_json::Value::as_object)
    {
        for (name, group) in user_defined {
            if let Some(error) = validate_group(name, group, name) {
                return Some(error);
            }
        }
    }
    None
}

fn controller_transfer_limits_validation_error(
    limits: Option<&serde_json::Value>,
    prefix: &str,
    target: ControllerProfile,
) -> Option<String> {
    let limits = match limits {
        None | Some(serde_json::Value::Null) => return None,
        Some(limits) => limits.as_object()?,
    };
    for (window_key, window_name) in [
        ("queued", "Queued"),
        ("daily", "Daily"),
        ("weekly", "Weekly"),
    ] {
        let Some(window) = limits.get(window_key) else {
            continue;
        };
        if window.is_null() {
            if target == ControllerProfile::Native {
                return Some("Invalid YAML configuration".to_owned());
            }
            continue;
        }
        let Some(window) = window.as_object() else {
            return Some(match target {
                ControllerProfile::Legacy => "Invalid configuration".to_owned(),
                ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
            });
        };
        for (key, field) in [
            ("files", "Files"),
            ("megabytes", "Megabytes"),
            ("failures", "Failures"),
        ] {
            if window.get(key).is_some_and(|candidate| {
                !candidate.is_null()
                    && !candidate
                        .as_i64()
                        .is_some_and(|value| (1..=i32::MAX as i64).contains(&value))
            }) {
                let limits_indent = prefix
                    .trim_end_matches('\n')
                    .rsplit('\n')
                    .next()
                    .map(|line| line.len() - line.trim_start().len())
                    .unwrap_or_default();
                let window_indent = " ".repeat(limits_indent + 2);
                let field_indent = " ".repeat(limits_indent + 4);
                let error = format!(
                    "{prefix}{window_indent}{window_name}:\n{field_indent}The field {field} must be between 1 and 2147483647."
                );
                return Some(match target {
                    ControllerProfile::Legacy => error,
                    ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
                });
            }
        }
    }
    None
}

fn controller_soulseek_connection_validation_error(
    value: &serde_json::Value,
    target: ControllerProfile,
) -> Option<String> {
    let connection = value.pointer("/soulseek/connection")?;
    let Some(connection) = connection.as_object() else {
        return Some(match target {
            ControllerProfile::Legacy => "Invalid configuration".to_owned(),
            ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
        });
    };
    let integer = |candidate: &serde_json::Value| {
        candidate
            .as_i64()
            .or_else(|| candidate.as_str().and_then(|value| value.parse().ok()))
    };
    let invalid_range = |candidate: &serde_json::Value, minimum: i64, maximum: i64| {
        !candidate.is_null()
            && !integer(candidate).is_some_and(|value| (minimum..=maximum).contains(&value))
    };
    let finish = |section: &str, errors: Vec<String>| {
        (!errors.is_empty()).then(|| match target {
            ControllerProfile::Legacy => format!(
                "Invalid configuration:\n  Soulseek:\n    Connection:\n      {section}:\n        {}",
                errors.join("\n        ")
            ),
            ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
        })
    };

    if let Some(buffer) = connection.get("buffer") {
        let Some(buffer) = buffer.as_object() else {
            return Some(match target {
                ControllerProfile::Legacy => "Invalid configuration".to_owned(),
                ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
            });
        };
        let mut errors = Vec::new();
        for (key, field, minimum, maximum) in [
            ("read", "Read", 1_024, i32::MAX as i64),
            ("write", "Write", 1_024, i32::MAX as i64),
            ("transfer", "Transfer", 81_920, i32::MAX as i64),
            ("write_queue", "WriteQueue", 5, 5_000),
        ] {
            if buffer
                .get(key)
                .is_some_and(|candidate| invalid_range(candidate, minimum, maximum))
            {
                errors.push(format!(
                    "The field {field} must be between {minimum} and {maximum}."
                ));
            }
        }
        if let Some(error) = finish("Buffer", errors) {
            return Some(error);
        }
    }

    if let Some(timeout) = connection.get("timeout") {
        let Some(timeout) = timeout.as_object() else {
            return Some(match target {
                ControllerProfile::Legacy => "Invalid configuration".to_owned(),
                ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
            });
        };
        let mut errors = Vec::new();
        for (key, field, minimum) in [
            ("connect", "Connect", 1_000),
            ("inactivity", "Inactivity", 1_000),
            ("transfer", "Transfer", 30_000),
        ] {
            if timeout
                .get(key)
                .is_some_and(|candidate| invalid_range(candidate, minimum, i32::MAX as i64))
            {
                errors.push(format!(
                    "The field {field} must be between {minimum} and {}.",
                    i32::MAX
                ));
            }
        }
        if let Some(error) = finish("Timeout", errors) {
            return Some(error);
        }
    }

    if let Some(proxy) = connection.get("proxy") {
        let Some(proxy) = proxy.as_object() else {
            return Some(match target {
                ControllerProfile::Legacy => "Invalid configuration".to_owned(),
                ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
            });
        };
        let enabled = proxy
            .get("enabled")
            .and_then(|candidate| {
                candidate.as_bool().or_else(|| {
                    candidate
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse().ok())
                })
            })
            .unwrap_or(false);
        let mut errors = Vec::new();
        for (key, field) in [
            ("address", "Address"),
            ("username", "Username"),
            ("password", "Password"),
        ] {
            if let Some(candidate) = proxy.get(key).filter(|candidate| !candidate.is_null()) {
                let invalid = candidate.as_str().is_none_or(|value| {
                    let length = value.encode_utf16().count();
                    length > 255 || (target == ControllerProfile::Legacy && length == 0)
                });
                if invalid {
                    errors.push(if target == ControllerProfile::Legacy {
                        format!("The field {field} must be a string with a minimum length of 1 and a maximum length of 255.")
                    } else {
                        format!("The field {field} must be a string with a maximum length of 255.")
                    });
                }
            }
        }
        if proxy
            .get("port")
            .is_some_and(|candidate| invalid_range(candidate, 1, 65_535))
        {
            errors.push("The field Port must be between 1 and 65535.".to_owned());
        }
        let address_missing = proxy
            .get("address")
            .and_then(serde_json::Value::as_str)
            .is_none_or(|address| address.trim().is_empty());
        if enabled && address_missing {
            errors.push("The Enabled field is true, but no Address has been specified.".to_owned());
        }
        if enabled && proxy.get("port").is_none_or(serde_json::Value::is_null) {
            errors.push("The Enabled field is true, but no Port has been specified.".to_owned());
        }
        if let Some(error) = finish("Proxy", errors) {
            return Some(error);
        }
    }
    None
}

fn controller_soulseek_profile_distributed_validation_error(
    value: &serde_json::Value,
    target: ControllerProfile,
) -> Option<String> {
    let invalid = |detail: String| match target {
        ControllerProfile::Legacy => detail,
        ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
    };
    let soulseek = value.get("soulseek")?;
    if soulseek.is_null() {
        return None;
    }
    let soulseek = soulseek.as_object()?;
    if let Some(level) = soulseek.get("diagnostic_level") {
        let level = match level {
            serde_json::Value::Null => "info".to_owned(),
            serde_json::Value::String(level) => level.clone(),
            serde_json::Value::Bool(level) => level.to_string(),
            serde_json::Value::Number(level) => level.to_string(),
            _ => {
                return Some(invalid(
                    "Exception during deserialization: Failed to create an instance of type 'System.String'."
                        .to_owned(),
                ));
            }
        };
        if !matches!(
            level.to_ascii_lowercase().as_str(),
            "none" | "warning" | "info" | "debug" | "trace"
        ) {
            return Some(invalid(
                "Invalid configuration:\n  Soulseek:\n    The DiagnosticLevel field must be one of: None, Warning, Info, Debug, Trace. Case insensitive."
                    .to_owned(),
            ));
        }
    }
    if let Some(picture) = soulseek.get("picture") {
        let picture = match picture {
            serde_json::Value::Null => None,
            serde_json::Value::String(picture) => Some(picture.clone()),
            serde_json::Value::Bool(picture) => Some(picture.to_string()),
            serde_json::Value::Number(picture) => Some(picture.to_string()),
            _ => {
                return Some(invalid(
                    "Exception during deserialization: Failed to create an instance of type 'System.String'."
                        .to_owned(),
                ));
            }
        };
        if let Some(picture) = picture.filter(|picture| !picture.is_empty()) {
            let path = Path::new(&picture);
            if !path.is_file() || fs::File::open(path).is_err() {
                return Some(invalid(format!(
                    "Invalid configuration:\n  Soulseek:\n    The Picture field specifies a non-existent file '{picture}'."
                )));
            }
        }
    }
    let distributed = soulseek.get("distributed_network")?;
    if distributed.is_null() {
        return None;
    }
    let Some(distributed) = distributed.as_object() else {
        return Some(invalid(
            "No node deserializer was able to deserialize the node into type slskd.Options+SoulseekOptions+DistributedNetworkOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                .to_owned(),
        ));
    };
    for key in ["disabled", "disable_children", "logging"] {
        let Some(candidate) = distributed.get(key) else {
            continue;
        };
        let valid = candidate.is_null()
            || candidate.is_boolean()
            || candidate.as_str().is_some_and(|value| {
                matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
            });
        if !valid {
            let rendered = candidate
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| candidate.to_string());
            return Some(invalid(format!(
                "Exception during deserialization: The value \"{rendered}\" is not a valid YAML Boolean"
            )));
        }
    }
    if let Some(limit) = distributed.get("child_limit") {
        let parsed = match limit {
            serde_json::Value::Null => Some(25_i64),
            serde_json::Value::Number(limit) => limit.as_i64(),
            serde_json::Value::String(limit) => limit.parse::<i64>().ok(),
            _ => None,
        };
        let Some(parsed) = parsed else {
            return Some(invalid(
                "Exception during deserialization: Input string was not in a correct format."
                    .to_owned(),
            ));
        };
        if parsed > i64::from(i32::MAX) || parsed < i64::from(i32::MIN) {
            return Some(invalid(
                "Exception during deserialization: Arithmetic operation resulted in an overflow."
                    .to_owned(),
            ));
        }
        if parsed < 1 {
            return Some(invalid(
                "Invalid configuration:\n  Soulseek:\n    DistributedNetwork:\n      The field ChildLimit must be between 1 and 2147483647."
                    .to_owned(),
            ));
        }
    }
    None
}

pub(crate) fn controller_yaml_target_validation_error(
    value: &serde_json::Value,
    target: ControllerProfile,
) -> Option<String> {
    let target_error = |controller_error: String| match target {
        ControllerProfile::Legacy => controller_error,
        ControllerProfile::Native => "Invalid YAML configuration".to_owned(),
    };
    if let Some(error) = controller_web_authentication_validation_error(value, target) {
        return Some(error);
    }
    if let Some(error) = controller_transfer_validation_error(value, target) {
        return Some(error);
    }
    if let Some(error) = controller_soulseek_connection_validation_error(value, target) {
        return Some(error);
    }
    if let Some(error) = controller_soulseek_profile_distributed_validation_error(value, target) {
        return Some(error);
    }
    if target == ControllerProfile::Native {
        if let Some(diagnostics) = value.get("diagnostics") {
            if !diagnostics.is_null() {
                let Some(diagnostics) = diagnostics.as_object() else {
                    return Some("Invalid YAML configuration".to_owned());
                };
                for key in ["allow_memory_dump", "allow_remote_dump"] {
                    if let Some(candidate) = diagnostics.get(key) {
                        let valid = candidate.is_null()
                            || candidate.is_boolean()
                            || candidate.as_str().is_some_and(|value| {
                                matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
                            });
                        if !valid {
                            return Some("Invalid YAML configuration".to_owned());
                        }
                    }
                }
            }
        }
        if let Some(web) = value.get("web") {
            if !web.is_null() {
                let Some(web) = web.as_object() else {
                    return Some("Invalid YAML configuration".to_owned());
                };
                if let Some(candidate) = web.get("max_request_body_size") {
                    let parsed = if candidate.is_null() {
                        Some(10 * 1024 * 1024_i64)
                    } else {
                        candidate.as_i64().or_else(|| {
                            candidate
                                .as_str()
                                .and_then(|value| value.parse::<i64>().ok())
                        })
                    };
                    if !parsed.is_some_and(|value| (1..=i32::MAX as i64).contains(&value)) {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                }
                for key in ["enforce_security", "allow_remote_no_auth"] {
                    if let Some(candidate) = web.get(key) {
                        let valid = candidate.is_null()
                            || candidate.is_boolean()
                            || candidate.as_str().is_some_and(|value| {
                                matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
                            });
                        if !valid {
                            return Some("Invalid YAML configuration".to_owned());
                        }
                    }
                }
                if let Some(authentication) = web.get("authentication") {
                    if !authentication.is_null() {
                        let Some(authentication) = authentication.as_object() else {
                            return Some("Invalid YAML configuration".to_owned());
                        };
                        if let Some(disabled) = authentication.get("disabled") {
                            let valid = disabled.is_null()
                                || disabled.is_boolean()
                                || disabled.as_str().is_some_and(|value| {
                                    matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
                                });
                            if !valid {
                                return Some("Invalid YAML configuration".to_owned());
                            }
                        }
                        if let Some(passthrough) = authentication.get("passthrough") {
                            if !passthrough.is_null() {
                                let Some(passthrough) = passthrough.as_object() else {
                                    return Some("Invalid YAML configuration".to_owned());
                                };
                                if let Some(allowed_cidrs) = passthrough.get("allowed_cidrs") {
                                    if !(allowed_cidrs.is_null()
                                        || allowed_cidrs.is_string()
                                        || allowed_cidrs.is_boolean()
                                        || allowed_cidrs.is_number())
                                    {
                                        return Some("Invalid YAML configuration".to_owned());
                                    }
                                }
                            }
                        }
                    }
                }
                if let Some(rate_limiting) = web.get("rate_limiting") {
                    if !rate_limiting.is_null() {
                        let Some(rate_limiting) = rate_limiting.as_object() else {
                            return Some("Invalid YAML configuration".to_owned());
                        };
                        if let Some(enabled) = rate_limiting.get("enabled") {
                            let valid = enabled.is_null()
                                || enabled.is_boolean()
                                || enabled.as_str().is_some_and(|value| {
                                    matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
                                });
                            if !valid {
                                return Some("Invalid YAML configuration".to_owned());
                            }
                        }
                        for key in [
                            "api_permit_limit",
                            "api_window_seconds",
                            "federation_permit_limit",
                            "federation_window_seconds",
                            "mesh_gateway_permit_limit",
                            "mesh_gateway_window_seconds",
                        ] {
                            if let Some(candidate) = rate_limiting.get(key) {
                                let valid = candidate.is_null()
                                    || candidate.as_i64().is_some_and(|value| {
                                        (i32::MIN as i64..=i32::MAX as i64).contains(&value)
                                    })
                                    || candidate
                                        .as_str()
                                        .is_some_and(|value| value.parse::<i32>().is_ok());
                                if !valid {
                                    return Some("Invalid YAML configuration".to_owned());
                                }
                            }
                        }
                    }
                }
                if let Some(cors) = web.get("cors") {
                    if !cors.is_null() {
                        let Some(cors) = cors.as_object() else {
                            return Some("Invalid YAML configuration".to_owned());
                        };
                        for key in ["enabled", "allow_credentials"] {
                            if let Some(candidate) = cors.get(key) {
                                let valid = candidate.is_null()
                                    || candidate.is_boolean()
                                    || candidate.as_str().is_some_and(|value| {
                                        matches!(
                                            value.to_ascii_lowercase().as_str(),
                                            "true" | "false"
                                        )
                                    });
                                if !valid {
                                    return Some("Invalid YAML configuration".to_owned());
                                }
                            }
                        }
                        for key in ["allowed_origins", "allowed_headers", "allowed_methods"] {
                            if let Some(candidate) = cors.get(key) {
                                let valid = candidate.is_null()
                                    || candidate.as_array().is_some_and(|values| {
                                        values.iter().all(|value| {
                                            value.is_null()
                                                || value.is_string()
                                                || value.is_boolean()
                                                || value.is_number()
                                        })
                                    });
                                if !valid {
                                    return Some("Invalid YAML configuration".to_owned());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(filters) = value.pointer("/filters/search/request") {
        if let Some(filters) = filters.as_array() {
            for filter in filters.iter().filter_map(serde_json::Value::as_str) {
                if crate::dotnet_regex::DotNetRegex::validate(filter).is_err() {
                    return Some(target_error(format!(
                        "Invalid configuration:\n  Filters:\n    Search:\n      Search request filter '{filter}' is not a valid regular expression"
                    )));
                }
            }
        }
    }
    if let Some(instance_name) = value.get("instance_name") {
        let error = match instance_name {
            serde_json::Value::Array(_) => Some(
                "No node deserializer was able to deserialize the node into type System.String, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e"
                    .to_owned(),
            ),
            serde_json::Value::Object(_) => Some(
                "Exception during deserialization: Failed to create an instance of type 'System.String'."
                    .to_owned(),
            ),
            _ => None,
        };
        if let Some(error) = error {
            return Some(target_error(error));
        }
    }
    if let Some(headless) = value.get("headless") {
        match headless {
            serde_json::Value::Null | serde_json::Value::Bool(_) => {}
            serde_json::Value::String(headless)
                if matches!(headless.to_ascii_lowercase().as_str(), "true" | "false") => {}
            serde_json::Value::Array(_) => {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type System.Boolean, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e"
                        .to_owned(),
                ));
            }
            headless => {
                let headless = headless
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| headless.to_string());
                return Some(target_error(format!(
                    "Exception during deserialization: The value \"{headless}\" is not a valid YAML Boolean"
                )));
            }
        }
    }
    if let Some(flags) = value.get("flags") {
        if flags.is_null() {
            if target == ControllerProfile::Native {
                return Some("Invalid YAML configuration".to_owned());
            }
        } else {
            let Some(flags) = flags.as_object() else {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type slskd.Options+FlagsOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                        .to_owned(),
                ));
            };
            for key in [
                "no_logo",
                "no_start",
                "no_version_check",
                "experimental",
                "case_sensitive_reg_ex",
                "no_share_scan",
                "force_share_scan",
                "hash_from_audio_file_enabled",
            ] {
                let Some(candidate) = flags.get(key) else {
                    continue;
                };
                match candidate {
                    serde_json::Value::Null | serde_json::Value::Bool(_) => {}
                    serde_json::Value::String(value)
                        if matches!(value.to_ascii_lowercase().as_str(), "true" | "false") => {}
                    serde_json::Value::Array(_) => {
                        return Some(target_error(
                            "No node deserializer was able to deserialize the node into type System.Boolean, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e"
                                .to_owned(),
                        ));
                    }
                    candidate => {
                        let candidate = candidate
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| candidate.to_string());
                        return Some(target_error(format!(
                            "Exception during deserialization: The value \"{candidate}\" is not a valid YAML Boolean"
                        )));
                    }
                }
            }
        }
    }
    if let Some(blacklist) = value.get("blacklist") {
        if !blacklist.is_null() {
            let Some(blacklist) = blacklist.as_object() else {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type slskd.Options+BlacklistOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                        .to_owned(),
                ));
            };
            let enabled = match blacklist.get("enabled") {
                None | Some(serde_json::Value::Null) => false,
                Some(serde_json::Value::Bool(enabled)) => *enabled,
                Some(serde_json::Value::String(enabled))
                    if matches!(enabled.to_ascii_lowercase().as_str(), "true" | "false") =>
                {
                    enabled.eq_ignore_ascii_case("true")
                }
                Some(enabled) => {
                    let enabled = enabled
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| enabled.to_string());
                    return Some(target_error(format!(
                        "Exception during deserialization: The value \"{enabled}\" is not a valid YAML Boolean"
                    )));
                }
            };
            let file = match blacklist.get("file") {
                None | Some(serde_json::Value::Null) => None,
                Some(serde_json::Value::String(file)) => Some(file.clone()),
                Some(serde_json::Value::Bool(file)) => Some(file.to_string()),
                Some(serde_json::Value::Number(file)) => Some(file.to_string()),
                Some(_) => {
                    return Some(target_error(
                        "Exception during deserialization: Failed to create an instance of type 'System.String'."
                            .to_owned(),
                    ));
                }
            };
            if target == ControllerProfile::Legacy && file.as_deref().is_some_and(str::is_empty) {
                return Some("The value cannot be an empty string. (Parameter 'path')".to_owned());
            }
            if let Some(file) = file.as_deref().filter(|file| !file.is_empty()) {
                let path = Path::new(file);
                if !path.is_file() || fs::File::open(path).is_err() {
                    return Some(target_error(format!(
                        "Invalid configuration:\n  Blacklist:\n    The File field specifies a non-existent file '{file}'."
                    )));
                }
            }
            if enabled {
                let Some(file) = file.as_deref().filter(|file| !file.trim().is_empty()) else {
                    return Some(target_error(
                        "Invalid configuration:\n  Blacklist:\n    The Enabled field is true, but no File has been specified."
                            .to_owned(),
                    ));
                };
                if crate::config::validate_managed_blacklist_file_format(Path::new(file), target)
                    .is_err()
                {
                    return Some(target_error(
                        "Invalid configuration:\n  Blacklist:\n    Failed to detect blacklist format. Only CIDR, P2P and DAT formats are supported"
                            .to_owned(),
                    ));
                }
            }
        }
    }
    if let Some(metrics) = value.get("metrics") {
        if !metrics.is_null() {
            let Some(metrics) = metrics.as_object() else {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type slskd.Options+MetricsOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                        .to_owned(),
                ));
            };
            let parse_boolean = |candidate: Option<&serde_json::Value>, default: bool| {
                match candidate {
                    None | Some(serde_json::Value::Null) => Ok(default),
                    Some(serde_json::Value::Bool(value)) => Ok(*value),
                    Some(serde_json::Value::String(value))
                        if matches!(value.to_ascii_lowercase().as_str(), "true" | "false") =>
                    {
                        Ok(value.eq_ignore_ascii_case("true"))
                    }
                    Some(value) => Err(target_error(format!(
                        "Exception during deserialization: The value \"{}\" is not a valid YAML Boolean",
                        value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string())
                    ))),
                }
            };
            let enabled = match parse_boolean(metrics.get("enabled"), false) {
                Ok(enabled) => enabled,
                Err(error) => return Some(error),
            };
            if metrics
                .get("url")
                .is_some_and(|url| url.is_array() || url.is_object())
            {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type System.String, System.Private.CoreLib, Version=10.0.0.0, Culture=neutral, PublicKeyToken=7cec85d7bea7798e"
                        .to_owned(),
                ));
            }
            let authentication = match metrics.get("authentication") {
                None | Some(serde_json::Value::Null) => None,
                Some(authentication) => {
                    let Some(authentication) = authentication.as_object() else {
                        return Some(target_error(
                            "No node deserializer was able to deserialize the node into type slskd.Options+MetricsOptions+MetricsAuthenticationOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                                .to_owned(),
                        ));
                    };
                    Some(authentication)
                }
            };
            let disabled = match parse_boolean(
                authentication.and_then(|authentication| authentication.get("disabled")),
                false,
            ) {
                Ok(disabled) => disabled,
                Err(error) => return Some(error),
            };
            let scalar_string = |candidate: Option<&serde_json::Value>, default: &str| {
                match candidate {
                    None | Some(serde_json::Value::Null) => Ok(default.to_owned()),
                    Some(serde_json::Value::String(value)) => Ok(value.clone()),
                    Some(serde_json::Value::Bool(value)) => Ok(value.to_string()),
                    Some(serde_json::Value::Number(value)) => Ok(value.to_string()),
                    Some(_) => Err(target_error(
                        "Exception during deserialization: Failed to create an instance of type 'System.String'."
                            .to_owned(),
                    )),
                }
            };
            let default_identity = match target {
                ControllerProfile::Legacy => "slskd",
                ControllerProfile::Native => "slskr",
            };
            let username = match scalar_string(
                authentication.and_then(|authentication| authentication.get("username")),
                default_identity,
            ) {
                Ok(username) => username,
                Err(error) => return Some(error),
            };
            let password = match scalar_string(
                authentication.and_then(|authentication| authentication.get("password")),
                "",
            ) {
                Ok(password) => password,
                Err(error) => return Some(error),
            };
            if target == ControllerProfile::Legacy || (enabled && !disabled) {
                let invalid_username = username.trim().is_empty()
                    || !(1..=255).contains(&username.encode_utf16().count());
                let invalid_password = password.trim().is_empty()
                    || !(1..=255).contains(&password.encode_utf16().count());
                if invalid_username || invalid_password {
                    if target == ControllerProfile::Native {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                    let mut messages = Vec::new();
                    if invalid_username {
                        messages.push("      The field Username must be a string with a minimum length of 1 and a maximum length of 255.");
                    }
                    if invalid_password {
                        messages.push("      The field Password must be a string with a minimum length of 1 and a maximum length of 255.");
                    }
                    return Some(format!(
                        "Invalid configuration:\n  Metrics:\n    Authentication:\n{}",
                        messages.join("\n")
                    ));
                }
            }
        }
    }
    if let Some(feature) = value.get("feature") {
        if !feature.is_null() {
            let Some(feature) = feature.as_object() else {
                return Some(target_error(
                    "No node deserializer was able to deserialize the node into type slskd.Options+FeatureOptions, slskd, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
                        .to_owned(),
                ));
            };
            if let Some(swagger) = feature.get("swagger") {
                match swagger {
                    serde_json::Value::Null | serde_json::Value::Bool(_) => {}
                    serde_json::Value::String(swagger)
                        if matches!(swagger.to_ascii_lowercase().as_str(), "true" | "false") => {}
                    swagger => {
                        let swagger = swagger
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| swagger.to_string());
                        return Some(target_error(format!(
                            "Exception during deserialization: The value \"{swagger}\" is not a valid YAML Boolean"
                        )));
                    }
                }
            }
        }
    }
    if value
        .pointer("/transfers/download/completed_path_template")
        .is_some_and(|template| template.is_array() || template.is_object())
    {
        return Some(target_error(
            "Exception during deserialization: Failed to create an instance of type 'System.String'."
            .to_owned(),
        ));
    }
    if target == ControllerProfile::Native {
        if let Some(auto_retry) = value.pointer("/transfers/download/auto_retry") {
            if !auto_retry.is_null() {
                let Some(auto_retry) = auto_retry.as_object() else {
                    return Some("Invalid YAML configuration".to_owned());
                };
                for key in ["enabled", "alternate_sources_enabled"] {
                    if let Some(candidate) = auto_retry.get(key) {
                        let valid = candidate.is_null()
                            || candidate.is_boolean()
                            || candidate.as_str().is_some_and(|value| {
                                matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
                            });
                        if !valid {
                            return Some("Invalid YAML configuration".to_owned());
                        }
                    }
                }
                for (key, minimum, maximum) in [
                    ("retry_delay_seconds", 10_i64, 86_400_i64),
                    ("check_interval_seconds", 10, 3_600),
                    ("max_attempts", 0, 100),
                    ("max_files_per_cycle", 1, 100),
                    ("max_files_per_peer_per_cycle", 1, 20),
                    ("peer_cooldown_seconds", 60, 86_400),
                    ("max_alternate_source_searches_per_cycle", 0, 10),
                ] {
                    if let Some(candidate) = auto_retry.get(key) {
                        let parsed = if candidate.is_null() {
                            Some(0)
                        } else {
                            candidate.as_i64().or_else(|| {
                                candidate
                                    .as_str()
                                    .and_then(|value| value.parse::<i32>().ok())
                                    .map(i64::from)
                            })
                        };
                        if !parsed.is_some_and(|value| (minimum..=maximum).contains(&value)) {
                            return Some("Invalid YAML configuration".to_owned());
                        }
                    }
                }
                if let Some(candidate) = auto_retry.get("alternate_source_size_tolerance_percent") {
                    let parsed = if candidate.is_null() {
                        Some(0.0)
                    } else {
                        candidate.as_f64().or_else(|| {
                            candidate
                                .as_str()
                                .and_then(|value| value.parse::<f64>().ok())
                        })
                    };
                    // Frozen native profile#65a14a8 uses integer RangeAttribute
                    // operands on this double and rounds the boundary before
                    // validation.  Upstream correction: snapetech/slskdN#271.
                    if !parsed
                        .is_some_and(|value| value.is_finite() && (-0.5..=100.5).contains(&value))
                    {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                }
            }
        }
    }
    if let Some(auto_response) = value.pointer("/soulseek/private_message_auto_response") {
        let Some(auto_response) = auto_response.as_object() else {
            return Some(target_error(
                "Exception during deserialization: Failed to create an instance of type 'slskd.Options+SoulseekOptions+PrivateMessageAutoResponseOptions'."
                    .to_owned(),
            ));
        };
        if let Some(enabled) = auto_response.get("enabled") {
            let valid = enabled.is_boolean()
                || enabled.as_str().is_some_and(|value| {
                    matches!(value.to_ascii_lowercase().as_str(), "true" | "false")
                });
            if !valid {
                return Some(target_error("Invalid configuration".to_owned()));
            }
        }
        if auto_response
            .get("message")
            .is_some_and(|message| message.is_array() || message.is_object())
        {
            return Some(target_error(
                "Exception during deserialization: Failed to create an instance of type 'System.String'."
                    .to_owned(),
            ));
        }
        if let Some(cooldown) = auto_response.get("cooldown_minutes") {
            let cooldown = cooldown.as_i64().or_else(|| {
                cooldown
                    .as_str()
                    .and_then(|cooldown| cooldown.parse::<i64>().ok())
            });
            if !cooldown.is_some_and(|cooldown| (1..=1_440).contains(&cooldown)) {
                return Some(target_error(
                    "Invalid configuration:\n  Soulseek.PrivateMessageAutoResponse:\n    The field CooldownMinutes must be between 1 and 1440."
                        .to_owned(),
                ));
            }
        }
    }
    if target == ControllerProfile::Native {
        if let Some(integrations) = value.get("integrations") {
            let Some(integrations) = integrations.as_object() else {
                return Some("Invalid YAML configuration".to_owned());
            };
            if let Some(spotify) = integrations.get("spotify") {
                let Some(spotify) = spotify.as_object() else {
                    return Some("Invalid YAML configuration".to_owned());
                };
                let enabled = match spotify.get("enabled") {
                    None => false,
                    Some(value) => match value.as_bool() {
                        Some(value) => value,
                        None => return Some("Invalid YAML configuration".to_owned()),
                    },
                };
                for (key, minimum, maximum) in [
                    ("timeout_seconds", 1_u64, 120_u64),
                    ("max_items_per_import", 1, 5_000),
                ] {
                    if let Some(candidate) = spotify.get(key) {
                        if !candidate
                            .as_u64()
                            .is_some_and(|value| (minimum..=maximum).contains(&value))
                        {
                            return Some("Invalid YAML configuration".to_owned());
                        }
                    }
                }
                if enabled
                    && spotify
                        .get("client_id")
                        .and_then(serde_json::Value::as_str)
                        .is_none_or(|value| value.trim().is_empty())
                {
                    return Some("Invalid YAML configuration".to_owned());
                }
                if enabled
                    && spotify
                        .get("market")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("US")
                        .encode_utf16()
                        .count()
                        != 2
                {
                    return Some("Invalid YAML configuration".to_owned());
                }
            }
            if let Some(lidarr) = integrations.get("lidarr") {
                let Some(lidarr) = lidarr.as_object() else {
                    return Some("Invalid YAML configuration".to_owned());
                };
                let enabled = match lidarr.get("enabled") {
                    None => false,
                    Some(value) => match value.as_bool() {
                        Some(value) => value,
                        None => return Some("Invalid YAML configuration".to_owned()),
                    },
                };
                for key in [
                    "sync_wanted_to_wishlist",
                    "auto_download",
                    "auto_import_completed",
                    "import_replace_existing_files",
                    "delete_rejected_downloads",
                    "blacklist_rejected_downloads",
                ] {
                    if lidarr.get(key).is_some_and(|value| !value.is_boolean()) {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                }
                for (key, minimum, maximum) in [
                    ("timeout_seconds", 1_u64, 120_u64),
                    ("sync_interval_seconds", 300, i32::MAX as u64),
                    ("max_items_per_sync", 1, 1_000),
                    ("wishlist_max_results", 10, 1_000),
                ] {
                    if let Some(candidate) = lidarr.get(key) {
                        if !candidate
                            .as_u64()
                            .is_some_and(|value| (minimum..=maximum).contains(&value))
                        {
                            return Some("Invalid YAML configuration".to_owned());
                        }
                    }
                }
                if enabled {
                    let valid_url = lidarr
                        .get("url")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|value| {
                            !value.trim().is_empty() && reqwest::Url::parse(value).is_ok()
                        });
                    let valid_key = lidarr
                        .get("api_key")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|value| !value.trim().is_empty());
                    if !valid_url || !valid_key {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                    let auto_import = lidarr
                        .get("auto_import_completed")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let from = lidarr
                        .get("import_path_from")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let to = lidarr
                        .get("import_path_to")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    if auto_import && from.trim().is_empty() != to.trim().is_empty() {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                    if !matches!(
                        lidarr
                            .get("import_mode")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("move"),
                        "move" | "copy" | "Move" | "Copy"
                    ) {
                        return Some("Invalid YAML configuration".to_owned());
                    }
                }
            }
        }
    }
    if let Some(dht) = value.get("dht") {
        let Some(dht) = dht.as_object() else {
            return Some(target_error(
                "Exception during deserialization: Failed to create an instance of type 'slskd.DhtRendezvous.DhtRendezvousOptions'."
                    .to_owned(),
            ));
        };
        let enabled = match dht.get("enabled") {
            None => true,
            Some(serde_json::Value::Bool(enabled)) => *enabled,
            Some(_) => return Some(target_error("Invalid configuration".to_owned())),
        };
        if let Some(port) = dht.get("dht_port") {
            let valid = port
                .as_u64()
                .is_some_and(|port| port <= 65_535 && (!enabled || port > 0));
            if !valid {
                return Some(target_error(
                    "Invalid configuration:\n  DhtRendezvous:\n    DHT rendezvous requires an explicit UDP port between 1 and 65535. Configure dht.dht_port to a stable forwarded or allow-listed port."
                        .to_owned(),
                ));
            }
        }
    }
    if let Some(debug) = value.get("debug") {
        if !debug.is_boolean() {
            let debug = debug
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| debug.to_string());
            return Some(target_error(format!(
                "Exception during deserialization: The value \"{debug}\" is not a valid YAML Boolean"
            )));
        }
    }
    if let Some(port) = value.pointer("/web/port") {
        let valid_port = port
            .as_u64()
            .is_some_and(|port| (1..=65_535).contains(&port));
        if !valid_port {
            let error = match port {
                serde_json::Value::String(port) => format!(
                    "Exception during deserialization: The input string '{port}' was not in a correct format."
                ),
                serde_json::Value::Bool(port) => format!(
                    "Exception during deserialization: The input string '{port}' was not in a correct format."
                ),
                _ => "Invalid configuration:\n  Web:\n    The field Port must be between 1 and 65535."
                    .to_owned(),
            };
            return Some(target_error(error));
        }
    }
    if let Some(address) = value.pointer("/web/ip_address") {
        if target == ControllerProfile::Native {
            return Some("Invalid YAML configuration".to_owned());
        }
        let valid = match address {
            serde_json::Value::Null => true,
            serde_json::Value::String(address) => {
                !address.is_empty()
                    && address
                        .split(',')
                        .map(str::trim)
                        .all(|address| parse_compat_ip_address(address).is_ok())
            }
            serde_json::Value::Number(address) => address
                .as_u64()
                .is_some_and(|address| address <= u32::MAX.into()),
            _ => false,
        };
        if !valid {
            return Some(
                "Invalid configuration:\n  Web:\n    The IpAddress field specifies an invalid IPv4 or IPv6 IP address."
                    .to_owned(),
            );
        }
    }
    if let Some(address) = value.pointer("/soulseek/listen_ip_address") {
        let valid = match address {
            serde_json::Value::Null => true,
            serde_json::Value::String(address) => address.parse::<IpAddr>().is_ok(),
            serde_json::Value::Number(address) => address
                .as_u64()
                .is_some_and(|address| address <= u32::MAX.into()),
            _ => false,
        };
        if !valid {
            return Some(target_error(
                "Invalid configuration:\n  Soulseek:\n    The ListenIpAddress field specifies an invalid IPv4 or IPv6 IP address."
                    .to_owned(),
            ));
        }
    }
    if let Some(port) = value.pointer("/soulseek/port") {
        let valid_port = port
            .as_u64()
            .is_some_and(|port| (1024..=65_535).contains(&port));
        if !valid_port {
            let deserialization_value = match port {
                serde_json::Value::String(port) => Some(port.clone()),
                serde_json::Value::Bool(port) => Some(port.to_string()),
                serde_json::Value::Number(port) if port.as_i64().is_none() => {
                    Some(port.to_string())
                }
                _ => None,
            };
            let error = deserialization_value.map_or_else(
                || {
                    "Invalid configuration:\n  Soulseek:\n    The field Port must be between 1024 and 65535."
                        .to_owned()
                },
                |port| {
                    format!(
                        "Exception during deserialization: The input string '{port}' was not in a correct format."
                    )
                },
            );
            return Some(target_error(error));
        }
    }
    if let Some(port) = value.pointer("/soulseek/listen_port") {
        let valid_port = port
            .as_u64()
            .is_some_and(|port| (1024..=65_535).contains(&port));
        if !valid_port {
            let deserialization_value = match port {
                serde_json::Value::String(port) => Some(port.clone()),
                serde_json::Value::Bool(port) => Some(port.to_string()),
                serde_json::Value::Number(port) if port.as_i64().is_none() => {
                    Some(port.to_string())
                }
                _ => None,
            };
            let error = deserialization_value.map_or_else(
                || {
                    "Invalid configuration:\n  Soulseek:\n    The field ListenPort must be between 1024 and 65535."
                        .to_owned()
                },
                |port| {
                    format!(
                        "Exception during deserialization: The input string '{port}' was not in a correct format."
                    )
                },
            );
            return Some(target_error(error));
        }
    }
    if target == ControllerProfile::Native {
        let obfuscation = value.pointer("/soulseek/obfuscation");
        if let Some(obfuscation) = obfuscation {
            let Some(obfuscation) = obfuscation.as_object() else {
                return Some("Invalid YAML configuration".to_owned());
            };
            for boolean_key in ["enabled", "advertise_regular_port", "prefer_outbound"] {
                if obfuscation
                    .get(boolean_key)
                    .is_some_and(|candidate| !candidate.is_boolean())
                {
                    return Some("Invalid YAML configuration".to_owned());
                }
            }
            let enabled = obfuscation
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            let mode = match obfuscation.get("mode") {
                None => "compatibility",
                Some(serde_json::Value::String(mode)) => mode.as_str(),
                Some(_) => return Some("Invalid YAML configuration".to_owned()),
            };
            if !matches!(
                mode.to_ascii_lowercase().as_str(),
                "compatibility" | "prefer" | "only"
            ) || (enabled && mode.eq_ignore_ascii_case("only"))
            {
                return Some("Invalid YAML configuration".to_owned());
            }
            let obfuscated_port = match obfuscation.get("listen_port") {
                None => 0,
                Some(port) => match port.as_u64() {
                    Some(port) if port == 0 || (1024..=65_535).contains(&port) => port,
                    _ => return Some("Invalid YAML configuration".to_owned()),
                },
            };
            let regular_port = value
                .pointer("/soulseek/listen_port")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(50_300);
            if enabled && obfuscated_port > 0 && obfuscated_port == regular_port {
                return Some("Invalid YAML configuration".to_owned());
            }
        }
    }
    let no_connect = value
        .pointer("/flags/no_connect")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let loopback_listener = value
        .pointer("/soulseek/listen_ip_address")
        .and_then(serde_json::Value::as_str)
        .and_then(|address| address.parse::<IpAddr>().ok())
        .is_some_and(|address| address.is_loopback());
    (target == ControllerProfile::Native && !no_connect && loopback_listener)
        .then(|| "Invalid YAML configuration".to_owned())
}
