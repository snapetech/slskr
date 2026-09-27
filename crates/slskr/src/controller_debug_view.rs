use super::*;

pub(super) fn controller_options_debug_view(
    state: &AppState,
    overlay: &ControllerOptionsOverlayState,
) -> String {
    let target = state.config.controller_profile;
    let mut options = serde_json::from_str::<serde_json::Value>(&controller_options_json(
        &state.config,
        overlay,
        true,
    ))
    .expect("controller options projection must remain valid JSON");

    if let Ok(port) = std::env::var("SLSKD_HTTPS_PORT") {
        if let Ok(port) = port.parse::<u16>() {
            options["web"]["https"]["port"] = serde_json::json!(port);
        }
    }
    if target == ControllerProfile::Native {
        for (parent, key) in [
            ("dhtRendezvous", "vpnPortSyncMode"),
            ("security.events", "minLogSeverity"),
            ("security", "profile"),
            ("virtualSoulfindV2", "defaultMode"),
        ] {
            let mut node = &mut options;
            for segment in parent.split('.') {
                node = &mut node[segment];
            }
            if let Some(object) = node.as_object_mut() {
                object.remove(key);
            }
        }
    }

    let root = options
        .as_object_mut()
        .expect("controller options fixture must be an object");
    root.insert(
        "appDirectory".to_owned(),
        serde_json::json!(state.config.state_dir.display().to_string()),
    );
    root.insert(
        "configurationFile".to_owned(),
        serde_json::json!(controller_compatibility_config_path(&state.config)
            .display()
            .to_string()),
    );
    root.insert("generateCertificate".to_owned(), serde_json::json!(false));
    root.insert("generateSecret".to_owned(), serde_json::json!(false));
    root.insert(
        "showEnvironmentVariables".to_owned(),
        serde_json::json!(false),
    );
    root.insert("showHelp".to_owned(), serde_json::json!(false));
    root.insert("showVersion".to_owned(), serde_json::json!(false));

    let dictionary = |type_name: &str| serde_json::Value::String(type_name.to_owned());
    root[if target == ControllerProfile::Native {
        "integration"
    } else {
        "integrations"
    }]["scripts"] = dictionary(if target == ControllerProfile::Native {
        "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+IntegrationOptions+ScriptOptions]"
    } else {
        "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+IntegrationsOptions+ScriptOptions]"
    });
    root[if target == ControllerProfile::Native {
        "integration"
    } else {
        "integrations"
    }]["webhooks"] = dictionary(if target == ControllerProfile::Native {
        "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+IntegrationOptions+WebhookOptions]"
    } else {
        "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+IntegrationsOptions+WebhookOptions]"
    });
    root["relay"]["agents"] = dictionary(
        "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+RelayOptions+RelayAgentConfigurationOptions]",
    );
    if target == ControllerProfile::Native {
        root["groups"]["userDefined"] = dictionary(
            "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+GroupsOptions+UserDefinedOptions]",
        );
    } else {
        root["transfers"]["groups"]["userDefined"] = dictionary(
            "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+TransfersOptions+GroupsOptions+UserDefinedOptions]",
        );
    }
    root["web"]["authentication"]["apiKeys"] = dictionary(
        "System.Collections.Generic.Dictionary`2[System.String,slskd.Options+WebOptions+WebAuthenticationOptions+ApiKeyOptions]",
    );

    let mut output = String::from("slskd:\n");
    render_controller_debug_object(root, 2, &mut Vec::new(), &mut output, state, overlay);
    let urls = if target == ControllerProfile::Native {
        format!("http://{}", state.config.http_bind)
    } else {
        String::new()
    };
    output.push_str(&format!(
        "urls={urls} (VolatileOverlayConfigurationProvider`1)\n"
    ));
    output
}

fn render_controller_debug_object(
    object: &serde_json::Map<String, serde_json::Value>,
    indentation: usize,
    path: &mut Vec<String>,
    output: &mut String,
    state: &AppState,
    overlay: &ControllerOptionsOverlayState,
) {
    let mut entries = object.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(key, _)| key.to_ascii_lowercase());
    for (key, value) in entries {
        let debug_key = key.to_ascii_lowercase();
        path.push(debug_key.clone());
        match value {
            serde_json::Value::Object(nested) if nested.is_empty() => {}
            serde_json::Value::Object(nested) => {
                if !controller_debug_object_has_leaves(nested) {
                    path.pop();
                    continue;
                }
                output.push_str(&format!("{}{debug_key}:\n", " ".repeat(indentation)));
                render_controller_debug_object(
                    nested,
                    indentation + 2,
                    path,
                    output,
                    state,
                    overlay,
                );
            }
            value => {
                let provider = controller_debug_provider(path, state, overlay);
                let raw = controller_debug_value(path, value, provider, state);
                if state.config.controller_profile == ControllerProfile::Native
                    && native_debug_key_is_sensitive(&debug_key)
                {
                    output.push_str(&format!(
                        "{}{debug_key}={}*****\n",
                        " ".repeat(indentation),
                        if raw.is_empty() { " " } else { "" }
                    ));
                } else {
                    output.push_str(&format!(
                        "{}{debug_key}={raw} ({provider})\n",
                        " ".repeat(indentation)
                    ));
                }
            }
        }
        path.pop();
    }
}

fn controller_debug_object_has_leaves(object: &serde_json::Map<String, serde_json::Value>) -> bool {
    object.values().any(|value| match value {
        serde_json::Value::Object(nested) => controller_debug_object_has_leaves(nested),
        _ => true,
    })
}

fn controller_debug_provider<'a>(
    path: &[String],
    state: &AppState,
    overlay: &'a ControllerOptionsOverlayState,
) -> &'a str {
    if json_has_debug_path(&overlay.effective, path) {
        return "VolatileOverlayConfigurationProvider`1";
    }
    let yaml = overlay
        .watched_yaml_effective
        .as_ref()
        .unwrap_or(&overlay.yaml_effective);
    let environment_name = match path.join(".").as_str() {
        "appdirectory" => Some("SLSKD_APP_DIR"),
        "debug" => Some("SLSKD_DEBUG"),
        "headless" => Some("SLSKD_HEADLESS"),
        "blacklist.enabled" => Some("SLSKD_BLACKLIST"),
        "blacklist.file" => Some("SLSKD_BLACKLIST_FILE"),
        "feature.swagger" => Some("SLSKD_SWAGGER"),
        "metrics.enabled" => Some("SLSKD_METRICS"),
        "metrics.url" => Some("SLSKD_METRICS_URL"),
        "metrics.authentication.disabled" => Some("SLSKD_METRICS_NO_AUTH"),
        "metrics.authentication.username" => Some("SLSKD_METRICS_USERNAME"),
        "metrics.authentication.password" => Some("SLSKD_METRICS_PASSWORD"),
        "web.authentication.username" => Some("SLSKD_USERNAME"),
        "web.authentication.password" => Some("SLSKD_PASSWORD"),
        "web.authentication.jwt.key" => Some("SLSKD_JWT_KEY"),
        "web.authentication.jwt.ttl" => Some("SLSKD_JWT_TTL"),
        "diagnostics.allowmemorydump" => Some("SLSKD_ALLOW_MEMORY_DUMP"),
        "diagnostics.allowremotedump" => Some("SLSKD_ALLOW_REMOTE_DUMP"),
        "remoteconfiguration" => Some("SLSKD_REMOTE_CONFIGURATION"),
        "remotefilemanagement" => Some("SLSKD_REMOTE_FILE_MANAGEMENT"),
        "instancename" => Some("SLSKD_INSTANCE_NAME"),
        "flags.noconfigwatch" => Some("SLSKD_NO_CONFIG_WATCH"),
        "flags.noconnect" => Some("SLSKD_NO_CONNECT"),
        "flags.nosharescan" => Some("SLSKD_NO_SHARE_SCAN"),
        "flags.forcesharescan" => Some("SLSKD_FORCE_SHARE_SCAN"),
        "directories.downloads" => Some("SLSKD_DOWNLOADS_DIR"),
        "directories.incomplete" => Some("SLSKD_INCOMPLETE_DIR"),
        "web.ipaddress" => Some("SLSKD_HTTP_IP_ADDRESS"),
        "web.address" => Some("SLSKD_HTTP_ADDRESS"),
        "web.port" => Some("SLSKD_HTTP_PORT"),
        "web.https.port" => Some("SLSKD_HTTPS_PORT"),
        "soulseek.address" => Some("SLSKD_SLSK_ADDRESS"),
        "soulseek.port" => Some("SLSKD_SLSK_PORT"),
        "soulseek.username" => Some("SLSKD_SLSK_USERNAME"),
        "soulseek.password" => Some("SLSKD_SLSK_PASSWORD"),
        "soulseek.listenipaddress" => Some("SLSKD_SLSK_LISTEN_IP_ADDRESS"),
        "soulseek.listenport" => Some("SLSKD_SLSK_LISTEN_PORT"),
        "soulseek.description" => Some("SLSKD_SLSK_DESCRIPTION"),
        "global.download.completedpathtemplate" => Some("SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE"),
        _ => None,
    };
    if environment_name.is_some_and(|name| state.controller_cli_environment.contains_key(name)) {
        return "CommandLineConfigurationProvider";
    }
    if json_has_debug_path(yaml, path) {
        return "YamlConfigurationProvider for 'slskd.yml' (Optional)";
    }
    if path.join(".") == "web.authentication.disabled"
        && (std::env::var_os("SLSKD_NO_AUTH").is_some()
            || std::env::var_os("SLSKR_AUTH_DISABLED").is_some())
    {
        return "EnvironmentVariableConfigurationProvider";
    }
    if environment_name.is_some_and(|name| std::env::var_os(name).is_some()) {
        return "EnvironmentVariableConfigurationProvider";
    }
    "DefaultValueConfigurationProvider"
}

fn json_has_debug_path(value: &serde_json::Value, path: &[String]) -> bool {
    let mut value = value;
    for segment in path {
        let Some(object) = value.as_object() else {
            return false;
        };
        let Some(next) = object
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(segment))
            .map(|(_, value)| value)
        else {
            return false;
        };
        value = next;
    }
    true
}

fn controller_debug_value(
    path: &[String],
    value: &serde_json::Value,
    provider: &str,
    state: &AppState,
) -> String {
    let path = path.join(".");
    if state.config.controller_profile == ControllerProfile::Legacy {
        match path.as_str() {
            "soulseek.password" => {
                return controller_debug_password(state, provider).unwrap_or_else(|| {
                    if value.as_str() == Some("*****") {
                        String::new()
                    } else {
                        controller_debug_scalar(value, provider)
                    }
                });
            }
            "web.authentication.password" | "metrics.authentication.password" => {
                let configured = if path == "metrics.authentication.password" {
                    state.config.controller_metrics_password.clone()
                } else {
                    state
                        .controller_web_auth_password
                        .read()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .clone()
                };
                return if configured.is_empty() && provider == "DefaultValueConfigurationProvider" {
                    // The frozen slskd controller exposes its built-in app-name
                    // defaults in the debug view. Keep the runtime credentials
                    // empty unless the user configured them explicitly, but
                    // preserve that observable compatibility projection.
                    "slskd".to_owned()
                } else {
                    configured
                };
            }
            "web.authentication.jwt.key" => {
                return state
                    .controller_web_jwt_key_current
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
            }
            _ => {}
        }
    } else {
        match path.as_str() {
            "soulseek.password" => {
                return controller_debug_password(state, provider).unwrap_or_else(|| {
                    if value.as_str() == Some("*****") {
                        String::new()
                    } else {
                        controller_debug_scalar(value, provider)
                    }
                });
            }
            "destinations.folders" => {
                return "System.Collections.Generic.List`1[slskd.Options+DestinationOption]"
                    .to_owned();
            }
            "realm.governanceroots" => {
                return "[]".to_owned();
            }
            "realm.isvalid" => return "False".to_owned(),
            "multirealm.realms" => {
                return r#"[{"Id":"default-realm-v1","DisplayName":null,"Description":null,"GovernanceRoots":["default-governance-root"],"BootstrapNodes":[],"Policies":{"GossipEnabled":true,"ReplicationEnabled":true,"MaxGossipHops":3,"GossipIntervalSeconds":300,"FederationAllowed":true},"IsValid":true}]"#.to_owned();
            }
            "multirealm.realmids" => {
                return "System.Collections.Generic.HashSet`1[System.String]".to_owned();
            }
            "virtualsoulfind.capture.audioextensions"
            | "security.pathguard.blockedextensions"
            | "security.paranoidmode.blockedipranges"
            | "security.honeypot.honeypotfiles" => {
                return "System.Collections.Generic.List`1[System.String]".to_owned();
            }
            "integration.spotify.clientid" => return String::new(),
            "web.authentication.password" => {
                return "*****".to_owned();
            }
            "web.authentication.jwt.key" => {
                return "*****".to_owned();
            }
            _ => {}
        }
        if value.as_str() == Some("*****") {
            return String::new();
        }
    }
    controller_debug_scalar(value, provider)
}

fn controller_debug_password(state: &AppState, provider: &str) -> Option<String> {
    match provider {
        "CommandLineConfigurationProvider" => state
            .controller_cli_environment
            .get("SLSKD_SLSK_PASSWORD")
            .cloned(),
        "YamlConfigurationProvider for 'slskd.yml' (Optional)" => {
            read_controller_compatibility_yaml(&state.config)
                .ok()
                .flatten()
                .and_then(|text| parse_controller_yaml(&text).ok())
                .and_then(|value| {
                    value
                        .pointer("/soulseek/password")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned)
                })
        }
        "EnvironmentVariableConfigurationProvider" => std::env::var("SLSKD_SLSK_PASSWORD").ok(),
        _ => state.config.password.clone(),
    }
}

fn controller_debug_scalar(value: &serde_json::Value, provider: &str) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(value) => {
            if provider == "DefaultValueConfigurationProvider" {
                if *value { "True" } else { "False" }.to_owned()
            } else {
                value.to_string()
            }
        }
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(_) => serde_json::to_string(value).unwrap_or_default(),
        serde_json::Value::Object(_) => String::new(),
    }
}

fn native_debug_key_is_sensitive(key: &str) -> bool {
    [
        "password", "passwd", "pwd", "secret", "token", "apikey", "api_key", "key",
    ]
    .into_iter()
    .any(|suffix| key.ends_with(suffix))
}
