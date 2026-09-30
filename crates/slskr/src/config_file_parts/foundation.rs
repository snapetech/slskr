use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControllerFlagsFileConfig {
    pub(in crate::config) no_logo: Option<bool>,
    pub(in crate::config) no_start: Option<bool>,
    pub(in crate::config) no_version_check: Option<bool>,
    pub(in crate::config) experimental: Option<bool>,
    pub(in crate::config) hash_from_audio_file_enabled: Option<bool>,
    pub(in crate::config) case_sensitive_reg_ex: Option<bool>,
    pub(in crate::config) no_share_scan: Option<bool>,
    pub(in crate::config) force_share_scan: Option<bool>,
    pub(in crate::config) force_migrations: Option<bool>,
    pub(in crate::config) legacy_windows_tcp_keepalive: Option<bool>,
    pub(in crate::config) log_sql: Option<bool>,
    pub(in crate::config) log_unobserved_exceptions: Option<bool>,
    pub(in crate::config) optimistic_relay_file_info: Option<bool>,
    pub(in crate::config) volatile: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerFileConfig {
    pub(in crate::config) disk: Option<bool>,
    pub(in crate::config) loki: Option<String>,
    pub(in crate::config) no_color: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PermissionsFileConfig {
    pub(in crate::config) file: FilePermissionsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilePermissionsFileConfig {
    pub(in crate::config) mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryFileConfig {
    pub(in crate::config) tracing: TelemetryTracingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryTracingFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) exporter: Option<String>,
    pub(in crate::config) jaeger_endpoint: Option<String>,
    pub(in crate::config) jaeger_port: Option<u16>,
    pub(in crate::config) otlp_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetentionFileConfig {
    pub(in crate::config) search: Option<u64>,
    pub(in crate::config) logs: Option<u64>,
    pub(in crate::config) transfers: TransferRetentionFileConfig,
    pub(in crate::config) files: FileRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileRetentionFileConfig {
    pub(in crate::config) complete: Option<u64>,
    pub(in crate::config) incomplete: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManagedBlacklistFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) file: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CompatibilityFileConfig {
    pub(in crate::config) profile: Option<String>,
    pub(in crate::config) remote_configuration: Option<bool>,
    pub(in crate::config) debug: Option<bool>,
    pub(in crate::config) no_config_watch: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppFileConfig {
    pub(in crate::config) http_bind: Option<String>,
    pub(in crate::config) state_dir: Option<PathBuf>,
    pub(in crate::config) auto_connect: Option<bool>,
    pub(in crate::config) reconnect: Option<bool>,
    pub(in crate::config) reconnect_seconds: Option<u64>,
    pub(in crate::config) ping_seconds: Option<u64>,
    pub(in crate::config) log_level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceFileConfig {
    pub(in crate::config) enabled: Option<bool>,
}
