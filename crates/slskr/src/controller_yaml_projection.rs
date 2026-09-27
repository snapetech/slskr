use super::*;

fn snake_to_camel_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut uppercase = false;
    for character in value.chars() {
        if character == '_' || character == '-' {
            uppercase = true;
        } else if uppercase {
            output.extend(character.to_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }
    output
}

pub(crate) fn camel_to_snake_case(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in characters.iter().copied().enumerate() {
        if character == '-' {
            output.push('_');
            continue;
        }
        if character.is_ascii_uppercase() {
            let previous_is_lower_or_digit = index > 0
                && (characters[index - 1].is_ascii_lowercase()
                    || characters[index - 1].is_ascii_digit());
            let next_is_lower = characters
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_lowercase());
            if index > 0 && (previous_is_lower_or_digit || next_is_lower) && !output.ends_with('_')
            {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }
    output
}

pub(crate) fn controller_yaml_api_projection(value: serde_json::Value) -> serde_json::Value {
    fn project(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => serde_json::Value::Object(
                object
                    .into_iter()
                    .map(|(key, value)| {
                        let projected_key = snake_to_camel_case(&key);
                        let lower = projected_key.to_ascii_lowercase();
                        let value = if matches!(
                            lower.as_str(),
                            "password"
                                | "secret"
                                | "token"
                                | "apikey"
                                | "accesskey"
                                | "accesstoken"
                                | "clientid"
                                | "clientsecret"
                                | "userkey"
                        ) && value.is_string()
                        {
                            serde_json::Value::String("*****".to_owned())
                        } else {
                            project(value)
                        };
                        (projected_key, value)
                    })
                    .collect(),
            ),
            serde_json::Value::Array(array) => {
                serde_json::Value::Array(array.into_iter().map(project).collect())
            }
            value => value,
        }
    }
    let mut projected = project(value);
    if let Some(object) = projected.as_object_mut() {
        if object
            .get("headless")
            .is_some_and(serde_json::Value::is_null)
        {
            object.remove("headless");
        } else if let Some(headless) = object.get_mut("headless") {
            if let Some(value) = headless
                .as_str()
                .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
            {
                *headless = serde_json::Value::Bool(value);
            }
        }
        if let Some(flags) = object
            .get_mut("flags")
            .and_then(serde_json::Value::as_object_mut)
        {
            for key in ["noShareScan", "forceShareScan", "hashFromAudioFileEnabled"] {
                if flags.get(key).is_some_and(serde_json::Value::is_null) {
                    flags.remove(key);
                } else if let Some(candidate) = flags.get_mut(key) {
                    if let Some(value) = candidate
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                    {
                        *candidate = serde_json::Value::Bool(value);
                    }
                }
            }
        }
        if let Some(transfers) = object.remove("transfers") {
            if let Some(global) = object.get_mut("global") {
                merge_json_objects(global, &transfers);
            } else {
                object.insert("global".to_owned(), transfers);
            }
        }
        if let Some(dht) = object.remove("dht") {
            if let Some(rendezvous) = object.get_mut("dhtRendezvous") {
                merge_json_objects(rendezvous, &dht);
            } else {
                object.insert("dhtRendezvous".to_owned(), dht);
            }
        }
        if let Some(integrations) = object.remove("integrations") {
            if let Some(integration) = object.get_mut("integration") {
                merge_json_objects(integration, &integrations);
            } else {
                object.insert("integration".to_owned(), integrations);
            }
        }
        if object
            .get("blacklist")
            .is_some_and(serde_json::Value::is_null)
        {
            object.remove("blacklist");
        } else if let Some(blacklist) = object
            .get_mut("blacklist")
            .and_then(serde_json::Value::as_object_mut)
        {
            if blacklist
                .get("enabled")
                .is_some_and(serde_json::Value::is_null)
            {
                blacklist.remove("enabled");
            } else if let Some(enabled) = blacklist.get_mut("enabled") {
                if let Some(value) = enabled
                    .as_str()
                    .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                {
                    *enabled = serde_json::Value::Bool(value);
                }
            }
            if blacklist
                .get("file")
                .is_some_and(serde_json::Value::is_null)
            {
                blacklist.remove("file");
            } else if let Some(file) = blacklist.get_mut("file") {
                match file {
                    serde_json::Value::Bool(value) => {
                        *file = serde_json::Value::String(value.to_string());
                    }
                    serde_json::Value::Number(value) => {
                        *file = serde_json::Value::String(value.to_string());
                    }
                    _ => {}
                }
            }
        }
        if object
            .get("feature")
            .is_some_and(serde_json::Value::is_null)
        {
            object.remove("feature");
        } else if let Some(feature) = object
            .get_mut("feature")
            .and_then(serde_json::Value::as_object_mut)
        {
            if feature
                .get("swagger")
                .is_some_and(serde_json::Value::is_null)
            {
                feature.remove("swagger");
            } else if let Some(swagger) = feature.get_mut("swagger") {
                if let Some(value) = swagger
                    .as_str()
                    .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                {
                    *swagger = serde_json::Value::Bool(value);
                }
            }
        }
        if object
            .get("metrics")
            .is_some_and(serde_json::Value::is_null)
        {
            object.remove("metrics");
        } else if let Some(metrics) = object
            .get_mut("metrics")
            .and_then(serde_json::Value::as_object_mut)
        {
            if metrics
                .get("enabled")
                .is_some_and(serde_json::Value::is_null)
            {
                metrics.remove("enabled");
            } else if let Some(enabled) = metrics.get_mut("enabled") {
                if let Some(value) = enabled
                    .as_str()
                    .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                {
                    *enabled = serde_json::Value::Bool(value);
                }
            }
            if metrics.get("url").is_some_and(serde_json::Value::is_null) {
                metrics.remove("url");
            } else if let Some(url) = metrics.get_mut("url") {
                if matches!(
                    url,
                    serde_json::Value::Bool(_) | serde_json::Value::Number(_)
                ) {
                    *url = serde_json::Value::String(url.to_string());
                }
            }
            if metrics
                .get("authentication")
                .is_some_and(serde_json::Value::is_null)
            {
                metrics.remove("authentication");
            } else if let Some(authentication) = metrics
                .get_mut("authentication")
                .and_then(serde_json::Value::as_object_mut)
            {
                if authentication
                    .get("disabled")
                    .is_some_and(serde_json::Value::is_null)
                {
                    authentication.remove("disabled");
                } else if let Some(disabled) = authentication.get_mut("disabled") {
                    if let Some(value) = disabled
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                    {
                        *disabled = serde_json::Value::Bool(value);
                    }
                }
                for key in ["username", "password"] {
                    if authentication
                        .get(key)
                        .is_some_and(serde_json::Value::is_null)
                    {
                        authentication.remove(key);
                    } else if let Some(value) = authentication.get_mut(key) {
                        if key == "password" {
                            *value = serde_json::Value::String("*****".to_owned());
                        } else if matches!(
                            value,
                            serde_json::Value::Bool(_) | serde_json::Value::Number(_)
                        ) {
                            *value = serde_json::Value::String(value.to_string());
                        }
                    }
                }
            }
        }
        if object
            .get("diagnostics")
            .is_some_and(serde_json::Value::is_null)
        {
            object.remove("diagnostics");
        } else if let Some(diagnostics) = object
            .get_mut("diagnostics")
            .and_then(serde_json::Value::as_object_mut)
        {
            for key in ["allowMemoryDump", "allowRemoteDump"] {
                if diagnostics.get(key).is_some_and(serde_json::Value::is_null) {
                    diagnostics.remove(key);
                } else if let Some(candidate) = diagnostics.get_mut(key) {
                    if let Some(value) = candidate
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                    {
                        *candidate = serde_json::Value::Bool(value);
                    }
                }
            }
        }
        if let Some(web) = object
            .get_mut("web")
            .and_then(serde_json::Value::as_object_mut)
        {
            if web
                .get("maxRequestBodySize")
                .is_some_and(serde_json::Value::is_null)
            {
                web.remove("maxRequestBodySize");
            } else if let Some(limit) = web.get_mut("maxRequestBodySize") {
                if let Some(value) = limit.as_str().and_then(|value| value.parse::<i64>().ok()) {
                    *limit = serde_json::json!(value);
                }
            }
            for key in ["enforceSecurity", "allowRemoteNoAuth"] {
                if web.get(key).is_some_and(serde_json::Value::is_null) {
                    web.remove(key);
                } else if let Some(candidate) = web.get_mut(key) {
                    if let Some(value) = candidate
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                    {
                        *candidate = serde_json::Value::Bool(value);
                    }
                }
            }
            if web
                .get("authentication")
                .is_some_and(serde_json::Value::is_null)
            {
                web.remove("authentication");
            } else if let Some(authentication) = web
                .get_mut("authentication")
                .and_then(serde_json::Value::as_object_mut)
            {
                if authentication
                    .get("disabled")
                    .is_some_and(serde_json::Value::is_null)
                {
                    authentication.remove("disabled");
                } else if let Some(disabled) = authentication.get_mut("disabled") {
                    if let Some(value) = disabled
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                    {
                        *disabled = serde_json::Value::Bool(value);
                    }
                }
                for key in ["username", "password"] {
                    if authentication
                        .get(key)
                        .is_some_and(serde_json::Value::is_null)
                    {
                        authentication.remove(key);
                    } else if let Some(value) = authentication.get_mut(key) {
                        if key == "password" {
                            *value = serde_json::Value::String("*****".to_owned());
                        } else if matches!(
                            value,
                            serde_json::Value::Bool(_) | serde_json::Value::Number(_)
                        ) {
                            *value = serde_json::Value::String(value.to_string());
                        }
                    }
                }
                if authentication
                    .get("jwt")
                    .is_some_and(serde_json::Value::is_null)
                {
                    authentication.remove("jwt");
                } else if let Some(jwt) = authentication
                    .get_mut("jwt")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    if jwt.get("key").is_some_and(serde_json::Value::is_null) {
                        jwt.remove("key");
                    } else if let Some(key) = jwt.get_mut("key") {
                        *key = serde_json::Value::String("*****".to_owned());
                    }
                    if jwt.get("ttl").is_some_and(serde_json::Value::is_null) {
                        jwt.remove("ttl");
                    } else if let Some(ttl) = jwt.get_mut("ttl") {
                        if let Some(value) =
                            ttl.as_str().and_then(|value| value.parse::<i64>().ok())
                        {
                            *ttl = serde_json::json!(value);
                        }
                    }
                }
                if authentication
                    .get("passthrough")
                    .is_some_and(serde_json::Value::is_null)
                {
                    authentication.remove("passthrough");
                } else if let Some(passthrough) = authentication
                    .get_mut("passthrough")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    if passthrough
                        .get("allowedCidrs")
                        .is_some_and(serde_json::Value::is_null)
                    {
                        passthrough.remove("allowedCidrs");
                    } else if let Some(allowed_cidrs) = passthrough.get_mut("allowedCidrs") {
                        if matches!(
                            allowed_cidrs,
                            serde_json::Value::Bool(_) | serde_json::Value::Number(_)
                        ) {
                            *allowed_cidrs = serde_json::Value::String(allowed_cidrs.to_string());
                        }
                    }
                }
            }
            if web.get("cors").is_some_and(serde_json::Value::is_null) {
                web.remove("cors");
            } else if let Some(cors) = web
                .get_mut("cors")
                .and_then(serde_json::Value::as_object_mut)
            {
                for key in ["enabled", "allowCredentials"] {
                    if cors.get(key).is_some_and(serde_json::Value::is_null) {
                        cors.remove(key);
                    } else if let Some(candidate) = cors.get_mut(key) {
                        if let Some(value) = candidate
                            .as_str()
                            .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                        {
                            *candidate = serde_json::Value::Bool(value);
                        }
                    }
                }
                for key in ["allowedOrigins", "allowedHeaders", "allowedMethods"] {
                    if cors.get(key).is_some_and(serde_json::Value::is_null) {
                        cors.remove(key);
                    } else if let Some(values) =
                        cors.get_mut(key).and_then(serde_json::Value::as_array_mut)
                    {
                        for value in values {
                            if matches!(
                                value,
                                serde_json::Value::Bool(_) | serde_json::Value::Number(_)
                            ) {
                                *value = serde_json::Value::String(value.to_string());
                            }
                        }
                    }
                }
            }
            if let Some(rate_limiting) = web
                .get_mut("rateLimiting")
                .and_then(serde_json::Value::as_object_mut)
            {
                if rate_limiting
                    .get("enabled")
                    .is_some_and(serde_json::Value::is_null)
                {
                    rate_limiting.remove("enabled");
                } else if let Some(enabled) = rate_limiting.get_mut("enabled") {
                    if let Some(value) = enabled
                        .as_str()
                        .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                    {
                        *enabled = serde_json::json!(value);
                    }
                }
                for key in [
                    "apiPermitLimit",
                    "apiWindowSeconds",
                    "federationPermitLimit",
                    "federationWindowSeconds",
                    "meshGatewayPermitLimit",
                    "meshGatewayWindowSeconds",
                ] {
                    if rate_limiting
                        .get(key)
                        .is_some_and(serde_json::Value::is_null)
                    {
                        rate_limiting.remove(key);
                    } else if let Some(candidate) = rate_limiting.get_mut(key) {
                        if let Some(value) = candidate
                            .as_str()
                            .and_then(|value| value.parse::<i32>().ok())
                        {
                            *candidate = serde_json::json!(value);
                        }
                    }
                }
            }
        }
        if let Some(global) = object
            .get_mut("global")
            .and_then(serde_json::Value::as_object_mut)
        {
            if global
                .get("download")
                .is_some_and(serde_json::Value::is_null)
            {
                global.remove("download");
            } else if let Some(download) = global
                .get_mut("download")
                .and_then(serde_json::Value::as_object_mut)
            {
                if download
                    .get("autoRetry")
                    .is_some_and(serde_json::Value::is_null)
                {
                    download.remove("autoRetry");
                } else if let Some(auto_retry) = download
                    .get_mut("autoRetry")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    for (key, default) in [
                        ("enabled", serde_json::json!(true)),
                        ("retryDelaySeconds", serde_json::json!(1800)),
                        ("checkIntervalSeconds", serde_json::json!(300)),
                        ("maxAttempts", serde_json::json!(5)),
                        ("maxFilesPerCycle", serde_json::json!(10)),
                        ("maxFilesPerPeerPerCycle", serde_json::json!(1)),
                        ("peerCooldownSeconds", serde_json::json!(900)),
                        ("alternateSourcesEnabled", serde_json::json!(true)),
                        ("maxAlternateSourceSearchesPerCycle", serde_json::json!(1)),
                        ("alternateSourceSizeTolerancePercent", serde_json::json!(5)),
                    ] {
                        if auto_retry.get(key).is_some_and(serde_json::Value::is_null) {
                            auto_retry.insert(key.to_owned(), default);
                        }
                    }
                    for key in ["enabled", "alternateSourcesEnabled"] {
                        if let Some(value) = auto_retry.get_mut(key) {
                            if let Some(parsed) = value
                                .as_str()
                                .and_then(|value| value.to_ascii_lowercase().parse::<bool>().ok())
                            {
                                *value = serde_json::Value::Bool(parsed);
                            }
                        }
                    }
                    for key in [
                        "retryDelaySeconds",
                        "checkIntervalSeconds",
                        "maxAttempts",
                        "maxFilesPerCycle",
                        "maxFilesPerPeerPerCycle",
                        "peerCooldownSeconds",
                        "maxAlternateSourceSearchesPerCycle",
                    ] {
                        if let Some(value) = auto_retry.get_mut(key) {
                            if let Some(parsed) =
                                value.as_str().and_then(|value| value.parse::<i64>().ok())
                            {
                                *value = serde_json::json!(parsed);
                            }
                        }
                    }
                    if let Some(value) = auto_retry.get_mut("alternateSourceSizeTolerancePercent") {
                        let parsed = value
                            .as_f64()
                            .or_else(|| value.as_str().and_then(|value| value.parse::<f64>().ok()));
                        if let Some(parsed) = parsed.filter(|value| value.is_finite()) {
                            if parsed.fract() == 0.0
                                && parsed >= i64::MIN as f64
                                && parsed <= i64::MAX as f64
                            {
                                *value = serde_json::json!(parsed as i64);
                            } else if let Some(number) = serde_json::Number::from_f64(parsed) {
                                *value = serde_json::Value::Number(number);
                            }
                        }
                    }
                }
            }
        }
        let instance_name = object.get("instanceName").cloned();
        match instance_name {
            Some(serde_json::Value::Null) => {
                object.remove("instanceName");
            }
            Some(serde_json::Value::String(ref value)) if value.is_empty() => {
                object.remove("instanceName");
            }
            Some(serde_json::Value::Bool(value)) => {
                object.insert(
                    "instanceName".to_owned(),
                    serde_json::Value::String(value.to_string()),
                );
            }
            Some(serde_json::Value::Number(value)) => {
                object.insert(
                    "instanceName".to_owned(),
                    serde_json::Value::String(value.to_string()),
                );
            }
            _ => {}
        }
        if let Some(auto_response) = object
            .get_mut("soulseek")
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|soulseek| soulseek.get_mut("privateMessageAutoResponse"))
            .and_then(serde_json::Value::as_object_mut)
        {
            if auto_response
                .get("message")
                .is_some_and(|message| message.is_null())
            {
                auto_response.insert(
                    "message".to_owned(),
                    serde_json::Value::String(
                        "Hi, I'm human and testing a slskr client. Shares may be temporarily unavailable while I validate the client."
                            .to_owned(),
                    ),
                );
            } else if let Some(message) = auto_response.get_mut("message") {
                match message {
                    serde_json::Value::Bool(value) => {
                        *message = serde_json::Value::String(value.to_string());
                    }
                    serde_json::Value::Number(value) => {
                        *message = serde_json::Value::String(value.to_string());
                    }
                    _ => {}
                }
            }
            if let Some(enabled) = auto_response.get_mut("enabled") {
                if let Some(value) = enabled
                    .as_str()
                    .and_then(|value| value.parse::<bool>().ok())
                {
                    *enabled = serde_json::Value::Bool(value);
                }
            }
            if let Some(cooldown) = auto_response.get_mut("cooldownMinutes") {
                if let Some(value) = cooldown
                    .as_str()
                    .and_then(|value| value.parse::<i64>().ok())
                {
                    *cooldown = serde_json::json!(value);
                }
            }
        }
    }
    projected
}
