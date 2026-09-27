use super::{
    config, controller_yaml_api_projection, merge_json_objects, parse_controller_yaml,
    read_controller_compatibility_yaml, AppConfig,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub(crate) struct ControllerOptionsOverlayState {
    pub(crate) yaml_effective: serde_json::Value,
    pub(crate) watched_yaml_effective: Option<serde_json::Value>,
    pub(crate) watched_download_exclusions: Option<Vec<String>>,
    pub(crate) watched_obfuscation: Option<ObfuscationReloadState>,
    pub(crate) watched_restart_fingerprint: Option<String>,
    pub(crate) effective: serde_json::Value,
    pub(crate) current: Option<serde_json::Value>,
    pub(crate) watched_share_directories: Option<Vec<String>>,
    pub(crate) watched_instance_name: Option<String>,
    pub(crate) watched_controller_swagger: Option<bool>,
    pub(crate) watched_dht: Option<config::DhtSettings>,
    pub(crate) command_line_environment: BTreeMap<String, String>,
}

impl ControllerOptionsOverlayState {
    pub(crate) fn load(config: &AppConfig) -> Result<Self, String> {
        let yaml_effective = match read_controller_compatibility_yaml(config)? {
            Some(text) => controller_yaml_api_projection(parse_controller_yaml(&text)?),
            None => serde_json::Value::Null,
        };
        Ok(Self {
            yaml_effective,
            watched_download_exclusions: Some(config.download_filter.exclude.clone()),
            watched_obfuscation: Some(ObfuscationReloadState::from_config(config)),
            watched_restart_fingerprint: Some(restart_reload_fingerprint(config)),
            ..Self::default()
        })
    }

    pub(crate) fn apply(&mut self, overlay: serde_json::Value) {
        merge_json_objects(&mut self.effective, &overlay);
        self.current = Some(overlay);
    }

    pub(crate) fn apply_yaml(&mut self, value: serde_json::Value) {
        self.watched_yaml_effective = Some(value);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ObfuscationReloadState {
    pub(crate) enabled: bool,
    pub(crate) mode: config::SoulseekObfuscationMode,
    pub(crate) listen_port: u32,
    pub(crate) advertise_regular_port: bool,
    pub(crate) prefer_outbound: bool,
}

impl ObfuscationReloadState {
    pub(crate) fn from_config(config: &AppConfig) -> Self {
        Self {
            enabled: config.obfuscation_enabled,
            mode: config.obfuscation_mode,
            listen_port: config.obfuscation_listen_port,
            advertise_regular_port: config.obfuscation_advertise_regular_port,
            prefer_outbound: config.obfuscation_prefer_outbound,
        }
    }
}

pub(crate) fn restart_reload_fingerprint(config: &AppConfig) -> String {
    let values = vec![
        format!("{:?}", &config.downloads_dir),
        format!("{:?}", &config.incomplete_dir),
        format!("{:?}", &config.instance_name),
        format!("{:?}", &config.http_binds),
        format!("{:?}", &config.controller_http_address),
        format!("{:?}", config.auto_connect),
        format!("{:?}", config.controller_debug),
        format!("{:?}", config.controller_headless),
        format!("{:?}", config.controller_no_config_watch),
        format!("{:?}", config.controller_no_logo),
        format!("{:?}", config.controller_no_start),
        format!("{:?}", config.controller_no_version_check),
        format!("{:?}", config.controller_experimental),
        format!("{:?}", config.controller_hash_from_audio_file_enabled),
        format!("{:?}", config.controller_case_sensitive_regex),
        format!("{:?}", config.controller_no_share_scan),
        format!("{:?}", config.controller_force_share_scan),
        format!("{:?}", config.managed_blacklist.enabled),
        format!("{:?}", config.controller_swagger),
        format!("{:?}", config.controller_metrics_enabled),
        format!("{:?}", &config.controller_metrics_url),
        format!("{:?}", config.controller_metrics_auth_disabled),
        format!("{:?}", &config.controller_metrics_username),
        format!("{:?}", &config.controller_metrics_password),
        format!("{:?}", config.auth_required),
        format!("{:?}", config.controller_web_jwt_ttl_millis),
        format!(
            "{:?}",
            config
                .controller_web_jwt_key_configured
                .then_some(&config.controller_web_jwt_key)
        ),
        format!("{:?}", config.controller_web_enforce_security),
        format!("{:?}", config.controller_web_allow_remote_no_auth),
        format!("{:?}", config.controller_web_max_request_body_size),
        format!("{:?}", config.controller_web_rate_limiting),
        format!("{:?}", config.controller_diagnostics_allow_memory_dump),
        format!("{:?}", config.controller_diagnostics_allow_remote_dump),
        format!("{:?}", config.soulseek_diagnostic_level),
        format!("{:?}", config.integrations.vpn.enabled),
        format!("{:?}", config.integrations.vpn.polling_interval),
        format!("{:?}", config.shared_mesh_tcp()),
        format!("{:?}", config.transfer_upload.slots),
        format!("{:?}", config.transfer_download.slots),
        format!("{:?}", config.core_workflow.incoming_search.concurrency),
        format!("{:?}", config.share_settings.cache_storage_mode),
        format!("{:?}", config.share_settings.cache_workers),
        format!("{:?}", &config.advanced_networking.mesh),
        format!("{:?}", &config.advanced_networking.overlay),
        format!("{:?}", &config.advanced_networking.overlay_data),
        format!("{:?}", &config.advanced_networking.relay),
        format!("{:?}", config.media_services.external_visualizer),
        format!("{:?}", config.media_services.song_id_max_concurrent_runs),
        format!("{:?}", config.media_services.virtual_soulfind.bridge),
    ];
    values.join("\u{1f}")
}
