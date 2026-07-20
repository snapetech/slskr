use std::{
    collections::BTreeMap,
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::Duration,
};

use rand::{rngs::SysRng, TryRng};
use serde::{Deserialize, Deserializer, Serialize};
use slskr_client::{protocol::peer::FileEntry, server::LoginCredentials};

const MAX_CONFIG_FILE_BYTES: u64 = 1024 * 1024;
const MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES: usize = 4 * 1024;
const MAX_COMPLETED_PATH_TEMPLATE_BYTES: usize = 4 * 1024;
const MAX_TRUSTED_MESH_PEERS: usize = 256;
const MAX_MESH_IDENTITY_BYTES: usize = 256;
const MAX_MESH_RANGE_ENDPOINT_BYTES: usize = 4 * 1024;
const CONTROLLER_DEFAULT_SERVER_ADDRESS: &str = "vps.slsknet.org:2271";
const CONTROLLER_DEFAULT_LISTEN_PORT: u32 = 50_300;

fn random_controller_jwt_key() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| format!("failed to generate controller JWT signing key: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub config_file: Option<PathBuf>,
    pub http_bind: SocketAddr,
    pub http_binds: Vec<SocketAddr>,
    pub controller_http_address: Option<String>,
    pub state_dir: PathBuf,
    pub instance_name: String,
    pub downloads_dir: PathBuf,
    pub incomplete_dir: PathBuf,
    pub server_address: String,
    pub listen_port: u32,
    pub username: Option<String>,
    pub password: Option<String>,
    pub credential_store: CredentialStoreMode,
    pub credential_file: PathBuf,
    pub auto_connect: bool,
    pub reconnect: bool,
    pub reconnect_delay: Duration,
    pub ping_interval: Duration,
    pub log_level: String,
    pub daemon_flags: DaemonFlagsSettings,
    pub logger: LoggerSettings,
    pub permissions_file_mode: Option<String>,
    pub telemetry_tracing: TelemetryTracingSettings,
    pub retention: RetentionSettings,
    pub search_retention: SearchRetentionSettings,
    pub core_workflow: CoreWorkflowSettings,
    pub advanced_networking: AdvancedNetworkingSettings,
    pub media_services: MediaAdvancedServiceSettings,
    pub controller_web: ControllerWebSettings,
    pub controller_api_keys: BTreeMap<String, ControllerApiKeySettings>,
    pub listener_bind: Option<String>,
    pub advertised_port: u32,
    pub obfuscated_listener_bind: Option<String>,
    pub obfuscated_advertised_port: Option<u32>,
    pub overlay_bind: Option<SocketAddr>,
    pub dht_enabled: bool,
    pub dht_port: u16,
    pub trusted_mesh_peers: Vec<TrustedMeshPeer>,
    pub obfuscation_enabled: bool,
    pub obfuscation_mode: SoulseekObfuscationMode,
    pub obfuscation_listen_port: u32,
    pub obfuscation_advertise_regular_port: bool,
    pub obfuscation_prefer_outbound: bool,
    pub peer_host_override: Option<Ipv4Addr>,
    pub test_user_endpoint_overrides: BTreeMap<String, SocketAddr>,
    pub user_info_description: String,
    pub user_info_picture: Option<PathBuf>,
    pub soulseek_diagnostic_level: SoulseekDiagnosticLevel,
    pub soulseek_distributed: SoulseekDistributedSettings,
    pub peer_response_timeout: Duration,
    pub soulseek_connection: SoulseekConnectionSettings,
    pub share_settings: ShareSettings,
    pub transfer_history_limit: usize,
    pub transfer_max_active: usize,
    pub transfer_allow_inbound: bool,
    pub transfer_allow_outbound: bool,
    pub transfer_upload: TransferUploadSettings,
    pub transfer_download: TransferDownloadSettings,
    pub transfer_groups: TransferGroupsSettings,
    pub transfer_auto_retry: TransferAutoRetrySettings,
    pub transfer_rescue: TransferRescueSettings,
    pub managed_blacklist: ManagedBlacklistSettings,
    pub download_completed_path_template: String,
    pub private_message_auto_response: PrivateMessageAutoResponseSettings,
    pub pod_join_signature_mode: PodSignatureMode,
    pub virtual_soulfind_v2_enabled: bool,
    pub controller_compatibility_target: ControllerCompatibilityTarget,
    pub controller_headless: bool,
    pub remote_configuration: bool,
    pub remote_file_management: bool,
    pub controller_debug: bool,
    pub controller_no_config_watch: bool,
    pub controller_no_logo: bool,
    pub controller_no_start: bool,
    pub controller_no_version_check: bool,
    pub controller_experimental: bool,
    pub controller_hash_from_audio_file_enabled: bool,
    pub controller_case_sensitive_regex: bool,
    pub controller_search_request_filters: Vec<String>,
    pub controller_no_share_scan: bool,
    pub controller_force_share_scan: bool,
    pub controller_swagger: bool,
    pub controller_metrics_enabled: bool,
    pub controller_metrics_url: String,
    pub controller_metrics_auth_disabled: bool,
    pub controller_metrics_username: String,
    pub controller_metrics_password: String,
    pub controller_web_auth_username: String,
    pub controller_web_auth_password: String,
    pub controller_web_jwt_key: String,
    pub controller_web_jwt_key_configured: bool,
    pub controller_web_jwt_ttl_millis: u64,
    pub auth_required: bool,
    pub api_token: Option<String>,
    pub api_read_write_token: Option<String>,
    pub api_read_only_token: Option<String>,
    pub api_nowplaying_token: Option<String>,
    pub api_cookie_auth_enabled: bool,
    pub api_rate_limit_anonymous: u32,
    pub api_rate_limit_authenticated: u32,
    pub controller_web_enforce_security: bool,
    pub controller_web_allow_remote_no_auth: bool,
    pub controller_web_passthrough_allowed_cidrs: Option<String>,
    pub controller_web_passthrough_cidrs: Vec<TrustedProxyCidr>,
    pub controller_web_max_request_body_size: usize,
    pub controller_web_cors: ControllerWebCorsSettings,
    pub controller_web_rate_limiting: ControllerWebRateLimitingSettings,
    pub controller_diagnostics_allow_memory_dump: bool,
    pub controller_diagnostics_allow_remote_dump: bool,
    pub trusted_proxy_cidrs: Vec<TrustedProxyCidr>,
    pub persistence_enabled: bool,
    pub integrations: IntegrationSettings,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DaemonFlagsSettings {
    pub force_migrations: bool,
    pub legacy_windows_tcp_keepalive: bool,
    pub log_sql: bool,
    pub log_unobserved_exceptions: bool,
    pub optimistic_relay_file_info: bool,
    pub volatile: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoggerSettings {
    pub disk: bool,
    pub loki: Option<String>,
    pub no_color: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryTracingSettings {
    pub enabled: bool,
    pub exporter: String,
    pub jaeger_endpoint: Option<String>,
    pub jaeger_port: Option<u16>,
    pub otlp_endpoint: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetentionSettings {
    pub search_minutes: Option<u64>,
    pub logs_days: u64,
    pub files_complete_minutes: Option<u64>,
    pub files_incomplete_minutes: Option<u64>,
    pub upload: TransferRetentionSettings,
    pub download: TransferRetentionSettings,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransferRetentionSettings {
    pub succeeded_minutes: Option<u64>,
    pub errored_minutes: Option<u64>,
    pub cancelled_minutes: Option<u64>,
    pub failed_minutes: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchRetentionSettings {
    pub max_age_days: u64,
    pub max_count: usize,
    pub cleanup_interval: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreWorkflowSettings {
    pub rooms: Vec<String>,
    pub liked_interests: Vec<String>,
    pub hated_interests: Vec<String>,
    pub destinations: Vec<DestinationSettings>,
    pub wishlist: WishlistSettings,
    pub incoming_search: IncomingSearchSettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct DestinationSettings {
    pub name: String,
    pub path: PathBuf,
    pub default: bool,
}

impl Default for DestinationSettings {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::new(),
            default: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WishlistSettings {
    pub enabled: bool,
    pub interval: Duration,
    pub auto_download: bool,
    pub max_results: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IncomingSearchSettings {
    pub concurrency: usize,
    pub circuit_breaker: usize,
    pub response_file_limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerWebSettings {
    pub socket: Option<PathBuf>,
    pub url_base: String,
    pub content_path: PathBuf,
    pub content_path_display: String,
    pub logging: bool,
    pub https: ControllerHttpsSettings,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerHttpsSettings {
    pub disabled: bool,
    pub binds: Vec<SocketAddr>,
    pub configured_ip_address: Option<String>,
    pub force: bool,
    pub certificate_pfx: Option<PathBuf>,
    pub certificate_password: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControllerApiKeySettings {
    pub key: String,
    pub role: String,
    pub cidr: String,
    pub cidrs: Vec<TrustedProxyCidr>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControllerWebCorsSettings {
    pub enabled: bool,
    pub allow_credentials: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allowed_methods: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControllerWebRateLimitingSettings {
    pub enabled: bool,
    pub api_permit_limit: i32,
    pub api_window_seconds: i32,
    pub federation_permit_limit: i32,
    pub federation_window_seconds: i32,
    pub mesh_gateway_permit_limit: i32,
    pub mesh_gateway_window_seconds: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoulseekConnectionSettings {
    pub buffer_read: usize,
    pub buffer_write: usize,
    pub buffer_transfer: usize,
    pub buffer_write_queue: usize,
    pub timeout_connect: Duration,
    pub timeout_inactivity: Duration,
    pub timeout_transfer: Duration,
    pub proxy: SoulseekProxySettings,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoulseekProxySettings {
    pub enabled: bool,
    pub address: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoulseekDiagnosticLevel {
    None,
    Warning,
    Info,
    Debug,
    Trace,
}

impl SoulseekDiagnosticLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "warning" => Ok(Self::Warning),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(
                "Soulseek diagnostic level must be one of None, Warning, Info, Debug, Trace"
                    .to_owned(),
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SoulseekDistributedSettings {
    pub disabled: bool,
    pub disable_children: bool,
    pub child_limit: usize,
    pub logging: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferUploadSettings {
    pub slots: u32,
    pub speed_limit_kib: u32,
    pub limits: TransferLimitsSettings,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferDownloadSettings {
    pub slots: u32,
    pub speed_limit_kib: u32,
    pub retry: TransferDownloadRetrySettings,
    pub destination: TransferDownloadDestinationSettings,
    pub completed_layout: String,
    pub auto_replace_stuck: bool,
    pub auto_replace_threshold_percent: f64,
    pub auto_replace_interval: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferDownloadRetrySettings {
    pub incomplete: String,
    pub attempts: u32,
    pub delay: Duration,
    pub max_delay: Duration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferDownloadDestinationSettings {
    pub subdirectory: Option<String>,
    pub exists: String,
    pub permissions_mode: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferGroupsSettings {
    pub default: TransferGroupSettings,
    pub leechers: LeecherTransferGroupSettings,
    pub user_defined: BTreeMap<String, UserDefinedTransferGroupSettings>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferGroupSettings {
    pub upload: TransferGroupUploadSettings,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeecherTransferGroupSettings {
    pub upload: TransferGroupUploadSettings,
    pub threshold_files: u32,
    pub threshold_directories: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserDefinedTransferGroupSettings {
    pub upload: TransferGroupUploadSettings,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferGroupUploadSettings {
    pub priority: u32,
    pub strategy: TransferQueueStrategy,
    pub slots: u32,
    pub speed_limit_kib: u32,
    pub allowed_file_types: Vec<String>,
    pub limits: TransferLimitsSettings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferQueueStrategy {
    RoundRobin,
    FirstInFirstOut,
}

impl TransferQueueStrategy {
    pub fn as_frozen_str(self) -> &'static str {
        match self {
            Self::RoundRobin => "roundrobin",
            Self::FirstInFirstOut => "firstinfirstout",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "roundrobin" => Ok(Self::RoundRobin),
            "firstinfirstout" => Ok(Self::FirstInFirstOut),
            _ => Err(format!("Queue strategy '{value}' is invalid")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferLimitsSettings {
    pub queued: Option<TransferLimitSettings>,
    pub daily: Option<TransferLimitSettings>,
    pub weekly: Option<TransferLimitSettings>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TransferLimitSettings {
    pub files: Option<u32>,
    pub megabytes: Option<u32>,
    pub failures: Option<u32>,
}

impl ControllerWebRateLimitingSettings {
    fn defaults(target: ControllerCompatibilityTarget) -> Self {
        Self {
            enabled: target == ControllerCompatibilityTarget::Slskdn,
            api_permit_limit: 200,
            api_window_seconds: 60,
            federation_permit_limit: 30,
            federation_window_seconds: 60,
            mesh_gateway_permit_limit: 60,
            mesh_gateway_window_seconds: 60,
        }
    }
}

impl AppConfig {
    pub fn from_layers<E: ConfigEnv>(
        config_file: Option<PathBuf>,
        file_config: FileConfig,
        base_env: &E,
    ) -> Result<Self, String> {
        let state_dir = optional_env_any(base_env, &["SLSKR_STATE_DIR", "SLSKD_APP_DIR"])
            .map(PathBuf::from)
            .or_else(|| file_config.app.state_dir.clone())
            .unwrap_or_else(default_state_dir);
        let controller_compatibility_target = ControllerCompatibilityTarget::parse(
            base_env
                .var("SLSKR_CONTROLLER_COMPATIBILITY_TARGET")
                .or_else(|| file_config.compatibility.controller_target.clone())
                .as_deref()
                .unwrap_or("slskdn"),
        )?;
        let controller_yaml = controller_yaml_environment(
            &state_dir.join("slskd.yml"),
            controller_compatibility_target,
        )?;
        let layered_env = ControllerYamlEnv {
            base: base_env,
            yaml: controller_yaml,
        };
        let env = &layered_env;
        let advanced_networking = AdvancedNetworkingSettings::from_layers(
            &file_config,
            env,
            controller_compatibility_target,
            &state_dir,
        )?;
        let media_services = MediaAdvancedServiceSettings::from_layers(
            &file_config,
            env,
            controller_compatibility_target,
        )?;
        let controller_headless =
            env_bool_layer(env, "SLSKD_HEADLESS", file_config.headless.unwrap_or(false))?;
        let controller_swagger = env_bool_layer(
            env,
            "SLSKD_SWAGGER",
            file_config.feature.swagger.unwrap_or(
                controller_compatibility_target == ControllerCompatibilityTarget::Slskdn,
            ),
        )?;
        let controller_metrics_enabled = env_bool_layer(
            env,
            "SLSKD_METRICS",
            file_config.metrics.enabled.unwrap_or(false),
        )?;
        let controller_metrics_url = env
            .var("SLSKD_METRICS_URL")
            .or(file_config.metrics.url)
            .unwrap_or_else(|| "/metrics".to_owned());
        let controller_metrics_auth_disabled = env_bool_layer(
            env,
            "SLSKD_METRICS_NO_AUTH",
            file_config.metrics.authentication.disabled.unwrap_or(false),
        )?;
        let controller_metrics_username = env
            .var("SLSKD_METRICS_USERNAME")
            .or(file_config.metrics.authentication.username)
            .unwrap_or_else(|| "slskd".to_owned());
        let controller_metrics_password = env
            .var("SLSKD_METRICS_PASSWORD")
            .or(file_config.metrics.authentication.password)
            .unwrap_or_else(|| {
                if controller_compatibility_target == ControllerCompatibilityTarget::Slskd {
                    "slskd".to_owned()
                } else {
                    String::new()
                }
            });
        let controller_web_auth_username = env
            .var("SLSKD_USERNAME")
            .or(file_config.auth.username)
            .unwrap_or_else(|| "slskd".to_owned());
        let controller_web_auth_password = env
            .var("SLSKD_PASSWORD")
            .or(file_config.auth.password)
            .unwrap_or_else(|| "slskd".to_owned());
        for (field, value) in [
            ("username", controller_web_auth_username.as_str()),
            ("password", controller_web_auth_password.as_str()),
        ] {
            let length = value.encode_utf16().count();
            if !(1..=255).contains(&length) {
                return Err(format!(
                    "web authentication {field} must contain between 1 and 255 characters"
                ));
            }
        }
        let controller_web_jwt_key = env.var("SLSKD_JWT_KEY").or(file_config.auth.jwt.key);
        let controller_web_jwt_key_configured = controller_web_jwt_key.is_some();
        let controller_web_jwt_key = controller_web_jwt_key
            .map(Ok)
            .unwrap_or_else(random_controller_jwt_key)?;
        if !(32..=255).contains(&controller_web_jwt_key.encode_utf16().count()) {
            return Err(
                "web authentication JWT key must contain between 32 and 255 characters".to_owned(),
            );
        }
        let controller_web_jwt_ttl_millis = env_parse_layer(
            env,
            "SLSKD_JWT_TTL",
            file_config.auth.jwt.ttl,
            if controller_compatibility_target == ControllerCompatibilityTarget::Slskd {
                604_800_000_u64
            } else {
                3_600_000_u64
            },
        )?;
        if controller_web_jwt_ttl_millis < 3_600 {
            return Err("web authentication JWT TTL must be at least 3600 milliseconds".to_owned());
        }
        let metrics_auth_requires_credentials = controller_compatibility_target
            == ControllerCompatibilityTarget::Slskd
            || (controller_metrics_enabled && !controller_metrics_auth_disabled);
        if metrics_auth_requires_credentials {
            for (field, value) in [
                ("username", controller_metrics_username.as_str()),
                ("password", controller_metrics_password.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(format!(
                        "metrics authentication {field} must be configured when metrics auth is enabled"
                    ));
                }
                let length = value.encode_utf16().count();
                if !(1..=255).contains(&length) {
                    return Err(format!(
                        "metrics authentication {field} must contain between 1 and 255 characters"
                    ));
                }
            }
        }
        let instance_name = optional_env_any(env, &["SLSKR_INSTANCE_NAME", "SLSKD_INSTANCE_NAME"])
            .unwrap_or_else(|| "default".to_owned());
        let configured_downloads_dir =
            optional_env_any(env, &["SLSKR_DOWNLOADS_DIR", "SLSKD_DOWNLOADS_DIR"]);
        let downloads_dir = configured_downloads_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| state_dir.join("downloads"));
        validate_controller_storage_directory(
            "Directories.Downloads",
            &downloads_dir,
            controller_compatibility_target,
            configured_downloads_dir.is_some(),
        )?;
        let configured_incomplete_dir =
            optional_env_any(env, &["SLSKR_INCOMPLETE_DIR", "SLSKD_INCOMPLETE_DIR"]);
        let incomplete_dir = configured_incomplete_dir
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| state_dir.join("incomplete"));
        validate_controller_storage_directory(
            "Directories.Incomplete",
            &incomplete_dir,
            controller_compatibility_target,
            configured_incomplete_dir.is_some(),
        )?;
        let configured_native_http_bind = file_config.app.http_bind.as_deref();
        let base_http_bind = configured_native_http_bind
            .unwrap_or("127.0.0.1:5030")
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid configured HTTP bind: {error}"))?;
        let http_port =
            env_parse_any_layer(env, &["SLSKD_HTTP_PORT"], None, base_http_bind.port())?;
        let (http_binds, controller_http_address) = if let Some(value) = env.var("SLSKR_HTTP_BIND")
        {
            let address = value
                .parse::<SocketAddr>()
                .map_err(|error| format!("invalid SLSKR_HTTP_BIND: {error}"))?;
            (vec![address], Some(address.ip().to_string()))
        } else {
            match controller_compatibility_target {
                ControllerCompatibilityTarget::Slskd => {
                    let configured = env.var("SLSKD_HTTP_IP_ADDRESS");
                    let ips = match configured.as_deref() {
                        Some(value) if !value.trim().is_empty() => value
                            .split(',')
                            .map(str::trim)
                            .map(|value| {
                                parse_compat_ip_address(value).map_err(|error| {
                                    format!("invalid SLSKD_HTTP_IP_ADDRESS: {error}")
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        Some(_) => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
                        None if configured_native_http_bind.is_some() => vec![base_http_bind.ip()],
                        None => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
                    };
                    (
                        ips.into_iter()
                            .map(|ip| SocketAddr::new(ip, http_port))
                            .collect(),
                        configured,
                    )
                }
                ControllerCompatibilityTarget::Slskdn => {
                    let configured = env.var("SLSKD_HTTP_ADDRESS");
                    let raw = configured
                        .clone()
                        .unwrap_or_else(|| base_http_bind.ip().to_string());
                    let ip = if raw == "*" {
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
                    } else {
                        raw.parse::<IpAddr>()
                            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
                    };
                    (vec![SocketAddr::new(ip, http_port)], Some(raw))
                }
            }
        };
        let http_bind = *http_binds
            .first()
            .ok_or_else(|| "HTTP bind list must not be empty".to_owned())?;
        let controller_socket = env
            .var("SLSKD_HTTP_SOCKET")
            .map(PathBuf::from)
            .or(file_config.web.socket)
            .filter(|path| !path.as_os_str().is_empty());
        if controller_socket
            .as_deref()
            .is_some_and(|path| !path.is_absolute())
        {
            return Err("web.socket must be an absolute path".to_owned());
        }
        let mut controller_url_base = env
            .var("SLSKD_URL_BASE")
            .or(file_config.web.url_base)
            .unwrap_or_else(|| "/".to_owned());
        if !controller_url_base.starts_with('/')
            || controller_url_base.contains(['?', '#'])
            || controller_url_base
                .split('/')
                .any(|segment| segment == "..")
        {
            return Err("web.url_base must be an absolute non-traversing URL path".to_owned());
        }
        if controller_url_base.len() > 1 {
            controller_url_base = controller_url_base.trim_end_matches('/').to_owned();
        }
        let configured_content_path = env
            .var("SLSKD_CONTENT_PATH")
            .map(PathBuf::from)
            .or(file_config.web.content_path);
        let controller_content_path_raw = configured_content_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("wwwroot"));
        if controller_content_path_raw.as_os_str().is_empty()
            || controller_content_path_raw
                .to_string_lossy()
                .encode_utf16()
                .count()
                > 255
        {
            return Err("web.content_path must contain between 1 and 255 characters".to_owned());
        }
        if configured_content_path.is_some() && controller_content_path_raw.is_absolute() {
            return Err(
                "web.content_path must be relative to the application directory".to_owned(),
            );
        }
        let controller_content_path = if controller_content_path_raw.is_absolute() {
            controller_content_path_raw.clone()
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."))
                .join(&controller_content_path_raw)
        };
        if configured_content_path.is_some() && !controller_content_path.is_dir() {
            return Err(format!(
                "web.content_path directory does not exist: {}",
                controller_content_path.display()
            ));
        }
        let https_disabled = env_bool_layer(
            env,
            "SLSKD_NO_HTTPS",
            file_config.web.https.disabled.unwrap_or(false),
        )?;
        let https_port = env_parse_layer(
            env,
            "SLSKD_HTTPS_PORT",
            file_config.web.https.port,
            5031_u16,
        )?;
        let https_configured_ip_address = env
            .var("SLSKD_HTTPS_IP_ADDRESS")
            .or(file_config.web.https.ip_address);
        let https_ips = match controller_compatibility_target {
            ControllerCompatibilityTarget::Slskd => match https_configured_ip_address.as_deref() {
                Some(value) if !value.trim().is_empty() => value
                    .split(',')
                    .map(str::trim)
                    .map(parse_compat_ip_address)
                    .collect::<Result<Vec<_>, _>>()?,
                _ => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
            },
            ControllerCompatibilityTarget::Slskdn => vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)],
        };
        let https_certificate_pfx = env
            .var("SLSKD_HTTPS_CERT_PFX")
            .map(PathBuf::from)
            .or(file_config.web.https.certificate.pfx)
            .filter(|path| !path.as_os_str().is_empty());
        if https_certificate_pfx
            .as_deref()
            .is_some_and(|path| !path.is_file())
        {
            return Err("web.https.certificate.pfx must identify a readable file".to_owned());
        }
        let controller_web = ControllerWebSettings {
            socket: controller_socket,
            url_base: controller_url_base,
            content_path: controller_content_path,
            content_path_display: controller_content_path_raw.display().to_string(),
            logging: env_bool_layer(
                env,
                "SLSKD_HTTP_LOGGING",
                file_config.web.logging.unwrap_or(false),
            )?,
            https: ControllerHttpsSettings {
                disabled: https_disabled,
                binds: https_ips
                    .into_iter()
                    .map(|ip| SocketAddr::new(ip, https_port))
                    .collect(),
                configured_ip_address: https_configured_ip_address,
                force: env_bool_layer(
                    env,
                    "SLSKD_HTTPS_FORCE",
                    file_config.web.https.force.unwrap_or(false),
                )?,
                certificate_pfx: https_certificate_pfx,
                certificate_password: env
                    .var("SLSKD_HTTPS_CERT_PASSWORD")
                    .or(file_config.web.https.certificate.password)
                    .unwrap_or_default(),
            },
        };
        let controller_api_key_files = match env.var("SLSKD_API_KEYS_JSON") {
            Some(value) => {
                serde_json::from_str::<BTreeMap<String, ControllerApiKeyFileConfig>>(&value)
                    .map_err(|error| format!("invalid web.authentication.api_keys: {error}"))?
            }
            None => file_config.auth.api_keys,
        };
        let mut controller_api_keys = BTreeMap::new();
        for (name, configured) in controller_api_key_files {
            let key_length = configured.key.encode_utf16().count();
            if !(16..=255).contains(&key_length) {
                return Err(format!(
                    "web.authentication.api_keys.{name}.key must contain between 16 and 255 characters"
                ));
            }
            let role = configured.role.to_ascii_lowercase();
            if !matches!(role.as_str(), "readonly" | "readwrite" | "administrator") {
                return Err(format!(
                    "web.authentication.api_keys.{name}.role must be readonly, readwrite, or administrator"
                ));
            }
            let default_cidr =
                if controller_compatibility_target == ControllerCompatibilityTarget::Slskdn {
                    "127.0.0.1/32,::1/128"
                } else {
                    "0.0.0.0/0,::/0"
                };
            let cidrs = configured
                .cidr
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .chain(
                    configured
                        .cidr
                        .trim()
                        .is_empty()
                        .then_some(default_cidr)
                        .into_iter()
                        .flat_map(|value| value.split(',')),
                )
                .map(TrustedProxyCidr::parse)
                .collect::<Result<Vec<_>, _>>()?;
            controller_api_keys.insert(
                name,
                ControllerApiKeySettings {
                    key: configured.key,
                    role,
                    cidr: if configured.cidr.trim().is_empty() {
                        default_cidr.to_owned()
                    } else {
                        configured.cidr
                    },
                    cidrs,
                },
            );
        }
        let configured_server_address = env
            .var("SLSK_SERVER")
            .or(file_config.network.server_address)
            .unwrap_or_else(|| CONTROLLER_DEFAULT_SERVER_ADDRESS.to_owned());
        let (configured_server_host, configured_server_port) =
            split_server_address(&configured_server_address)?;
        let server_host = env
            .var("SLSKD_SLSK_ADDRESS")
            .unwrap_or(configured_server_host);
        let server_port =
            env_parse_any_layer(env, &["SLSKD_SLSK_PORT"], None, configured_server_port)?;
        let server_address = format_host_port(&server_host, server_port);
        let listen_port = env_parse_any_layer(
            env,
            &["SLSK_LISTEN_PORT", "SLSKD_SLSK_LISTEN_PORT"],
            file_config.network.listen_port,
            CONTROLLER_DEFAULT_LISTEN_PORT,
        )?;
        if !(1024..=65_535).contains(&listen_port) {
            return Err("Soulseek.ListenPort must be between 1024 and 65535".to_owned());
        }
        let username = optional_env_any(env, &["SLSK_USERNAME", "SLSKD_SLSK_USERNAME"])
            .or(file_config.network.username);
        let password = optional_env_any(env, &["SLSK_PASSWORD", "SLSKD_SLSK_PASSWORD"])
            .or(file_config.network.password);
        let credential_store = CredentialStoreMode::parse(
            env.var("SLSKR_CREDENTIAL_STORE")
                .or(file_config.network.credential_store)
                .unwrap_or_else(|| "os".to_owned())
                .as_str(),
        )?;
        let credential_file = env
            .var("SLSKR_CREDENTIAL_FILE")
            .map(PathBuf::from)
            .or(file_config.network.credential_file)
            .unwrap_or_else(|| state_dir.join("soulseek-credentials.json"));
        let auto_connect_default = file_config.app.auto_connect.unwrap_or(
            username.is_some() && password.is_some() || credential_store.auto_connect_default(),
        );
        let auto_connect = if env.var("SLSKR_AUTO_CONNECT").is_some() {
            env_bool_layer(env, "SLSKR_AUTO_CONNECT", auto_connect_default)?
        } else if env.var("SLSKD_NO_CONNECT").is_some() {
            !env_bool_layer(env, "SLSKD_NO_CONNECT", false)?
        } else {
            auto_connect_default
        };
        let reconnect = env_bool_layer(
            env,
            "SLSKR_RECONNECT",
            file_config.app.reconnect.unwrap_or(auto_connect),
        )?;
        let reconnect_delay = validated_runtime_interval(
            "SLSKR_RECONNECT_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_RECONNECT_SECONDS",
                file_config.app.reconnect_seconds,
                30,
            )?,
        )?;
        let ping_interval = validated_runtime_interval(
            "SLSKR_PING_SECONDS",
            env_parse_layer(env, "SLSKR_PING_SECONDS", file_config.app.ping_seconds, 300)?,
        )?;
        let log_level = env
            .var("SLSKR_LOG_LEVEL")
            .or(file_config.app.log_level)
            .or_else(|| env.var("RUST_LOG"))
            .unwrap_or_else(|| "info".to_owned());
        let daemon_flags = DaemonFlagsSettings {
            force_migrations: env_bool_layer(
                env,
                "SLSKD_FORCE_MIGRATIONS",
                file_config.flags.force_migrations.unwrap_or(false),
            )?,
            legacy_windows_tcp_keepalive: env_bool_layer(
                env,
                "SLSKD_LEGACY_WINDOWS_TCP_KEEPALIVE",
                file_config
                    .flags
                    .legacy_windows_tcp_keepalive
                    .unwrap_or(false),
            )?,
            log_sql: env_bool_layer(
                env,
                "SLSKD_LOG_SQL",
                file_config.flags.log_sql.unwrap_or(false),
            )?,
            log_unobserved_exceptions: env_bool_layer(
                env,
                "SLSKD_LOG_UNOBSERVED_EXCEPTIONS",
                file_config.flags.log_unobserved_exceptions.unwrap_or(false),
            )?,
            optimistic_relay_file_info: env_bool_layer(
                env,
                "SLSKD_OPTIMISTIC_RELAY_FILE_INFO",
                file_config
                    .flags
                    .optimistic_relay_file_info
                    .unwrap_or(false),
            )?,
            volatile: env_bool_layer(
                env,
                "SLSKD_VOLATILE",
                file_config.flags.volatile.unwrap_or(false),
            )?,
        };
        let logger_loki = env
            .var("SLSKD_LOKI")
            .or(file_config.logger.loki)
            .filter(|value| !value.trim().is_empty());
        if logger_loki
            .as_deref()
            .is_some_and(|value| !(value.starts_with("http://") || value.starts_with("https://")))
        {
            return Err("logger.loki must be an http:// or https:// URL".to_owned());
        }
        let logger = LoggerSettings {
            disk: env_bool_layer(
                env,
                "SLSKD_DISK_LOGGER",
                file_config.logger.disk.unwrap_or(false),
            )?,
            loki: logger_loki,
            no_color: env_bool_layer(
                env,
                "SLSKD_NO_COLOR",
                file_config.logger.no_color.unwrap_or(false),
            )?,
        };
        let permissions_file_mode = env
            .var("SLSKD_FILE_PERMISSION_MODE")
            .or(file_config.permissions.file.mode)
            .filter(|value| !value.is_empty());
        if permissions_file_mode.as_deref().is_some_and(|mode| {
            !(3..=4).contains(&mode.len())
                || !mode.bytes().all(|byte| (b'0'..=b'7').contains(&byte))
        }) {
            return Err(
                "permissions.file.mode must be a three- or four-character chmod value".to_owned(),
            );
        }
        if controller_compatibility_target == ControllerCompatibilityTarget::Slskd
            && permissions_file_mode.is_some()
        {
            return Err("The 'permissions' keys have been moved under a new 'destination' key under transfers -> download, and the behavior has changed.  See https://github.com/slskd/slskd/pull/1756 for details".to_owned());
        }
        let telemetry_exporter = env
            .var("SLSKD_TELEMETRY_TRACING_EXPORTER")
            .or(file_config.telemetry.tracing.exporter)
            .unwrap_or_else(|| "console".to_owned())
            .to_ascii_lowercase();
        if !matches!(telemetry_exporter.as_str(), "console" | "jaeger" | "otlp") {
            return Err("telemetry.tracing.exporter must be console, jaeger, or otlp".to_owned());
        }
        let telemetry_tracing =
            TelemetryTracingSettings {
                enabled: env_bool_layer(
                    env,
                    "SLSKD_TELEMETRY_TRACING",
                    file_config.telemetry.tracing.enabled.unwrap_or(false),
                )?,
                exporter: telemetry_exporter,
                jaeger_endpoint: env
                    .var("SLSKD_TELEMETRY_JAEGER_ENDPOINT")
                    .or(file_config.telemetry.tracing.jaeger_endpoint)
                    .filter(|value| !value.trim().is_empty()),
                jaeger_port: match env.var("SLSKD_TELEMETRY_JAEGER_PORT") {
                    Some(value) => Some(value.parse::<u16>().map_err(|error| {
                        format!("invalid SLSKD_TELEMETRY_JAEGER_PORT: {error}")
                    })?),
                    None => file_config.telemetry.tracing.jaeger_port,
                },
                otlp_endpoint: env
                    .var("SLSKD_TELEMETRY_OTLP_ENDPOINT")
                    .or(file_config.telemetry.tracing.otlp_endpoint)
                    .filter(|value| !value.trim().is_empty()),
            };
        let retention = RetentionSettings {
            search_minutes: env_parse_option_layer(
                env,
                "SLSKR_RETENTION_SEARCH",
                file_config.retention.search,
            )?,
            logs_days: env_parse_layer(
                env,
                "SLSKR_RETENTION_LOGS",
                file_config.retention.logs,
                180_u64,
            )?,
            files_complete_minutes: env_parse_option_layer(
                env,
                "SLSKR_RETENTION_FILES_COMPLETE",
                file_config.retention.files.complete,
            )?,
            files_incomplete_minutes: env_parse_option_layer(
                env,
                "SLSKR_RETENTION_FILES_INCOMPLETE",
                file_config.retention.files.incomplete,
            )?,
            upload: TransferRetentionSettings {
                succeeded_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_SUCCEEDED",
                    file_config.retention.transfers.upload.succeeded,
                )?,
                errored_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_ERRORED",
                    file_config.retention.transfers.upload.errored,
                )?,
                cancelled_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_CANCELLED",
                    file_config.retention.transfers.upload.cancelled,
                )?,
                failed_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_FAILED",
                    file_config.retention.transfers.upload.failed,
                )?,
            },
            download: TransferRetentionSettings {
                succeeded_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_SUCCEEDED",
                    file_config.retention.transfers.download.succeeded,
                )?,
                errored_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_ERRORED",
                    file_config.retention.transfers.download.errored,
                )?,
                cancelled_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_CANCELLED",
                    file_config.retention.transfers.download.cancelled,
                )?,
                failed_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_FAILED",
                    file_config.retention.transfers.download.failed,
                )?,
            },
        };
        if retention.logs_days < 1 {
            return Err("retention.logs must be at least 1 day".to_owned());
        }
        for (name, value, minimum) in [
            ("retention.search", retention.search_minutes, 5),
            (
                "retention.files.complete",
                retention.files_complete_minutes,
                30,
            ),
            (
                "retention.files.incomplete",
                retention.files_incomplete_minutes,
                30,
            ),
            (
                "retention.transfers.upload.succeeded",
                retention.upload.succeeded_minutes,
                5,
            ),
            (
                "retention.transfers.upload.errored",
                retention.upload.errored_minutes,
                5,
            ),
            (
                "retention.transfers.upload.cancelled",
                retention.upload.cancelled_minutes,
                5,
            ),
            (
                "retention.transfers.upload.failed",
                retention.upload.failed_minutes,
                5,
            ),
            (
                "retention.transfers.download.succeeded",
                retention.download.succeeded_minutes,
                5,
            ),
            (
                "retention.transfers.download.errored",
                retention.download.errored_minutes,
                5,
            ),
            (
                "retention.transfers.download.cancelled",
                retention.download.cancelled_minutes,
                5,
            ),
            (
                "retention.transfers.download.failed",
                retention.download.failed_minutes,
                5,
            ),
        ] {
            if value.is_some_and(|value| value < minimum) {
                return Err(format!("{name} must be at least {minimum} minutes"));
            }
        }
        let search_retention_cleanup_seconds = env_parse_layer(
            env,
            "SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL",
            file_config
                .filters
                .search_retention
                .cleanup_interval_seconds,
            86_400_u64,
        )?;
        if search_retention_cleanup_seconds < 3_600 {
            return Err(
                "filters.search_retention.cleanup_interval_seconds must be at least 3600"
                    .to_owned(),
            );
        }
        let search_retention = SearchRetentionSettings {
            max_age_days: env_parse_layer(
                env,
                "SLSKD_SEARCH_RETENTION_MAX_AGE_DAYS",
                file_config.filters.search_retention.max_age_days,
                30_u64,
            )?,
            max_count: env_parse_layer(
                env,
                "SLSKD_SEARCH_RETENTION_MAX_COUNT",
                file_config.filters.search_retention.max_count,
                1_000_usize,
            )?,
            cleanup_interval: Duration::from_secs(search_retention_cleanup_seconds),
        };
        let rooms = normalized_controller_values(controller_string_array_layer(
            env,
            "SLSKD_ROOMS",
            Vec::new(),
        ));
        let liked_interests = normalized_controller_values(controller_string_array_layer(
            env,
            "SLSKD_SLSK_LIKED_INTERESTS",
            Vec::new(),
        ));
        let hated_interests = normalized_controller_values(controller_string_array_layer(
            env,
            "SLSKD_SLSK_HATED_INTERESTS",
            Vec::new(),
        ));
        for (name, values) in [
            ("rooms", &rooms),
            ("soulseek.liked_interests", &liked_interests),
            ("soulseek.hated_interests", &hated_interests),
        ] {
            if values.len() > 1_000 {
                return Err(format!("{name} may contain at most 1000 entries"));
            }
            if values.iter().any(|value| value.len() > 1_024) {
                return Err(format!("{name} entries may not exceed 1024 bytes"));
            }
        }
        let destinations = match env.var("SLSKD_DESTINATIONS_JSON") {
            Some(json) => serde_json::from_str::<Vec<DestinationSettings>>(&json)
                .map_err(|error| format!("invalid destinations.folders configuration: {error}"))?,
            None => Vec::new(),
        };
        if destinations.len() > 256 {
            return Err("destinations.folders may contain at most 256 entries".to_owned());
        }
        let mut destination_paths = std::collections::BTreeSet::new();
        let mut default_destinations = 0_usize;
        for destination in &destinations {
            if destination.path.as_os_str().is_empty() || !destination.path.is_absolute() {
                return Err("destinations.folders.path must be absolute".to_owned());
            }
            if destination
                .path
                .components()
                .any(|component| component == std::path::Component::ParentDir)
            {
                return Err(
                    "destinations.folders.path may not contain traversal segments".to_owned(),
                );
            }
            if !destination_paths.insert(destination.path.clone()) {
                return Err("destinations.folders paths must be unique".to_owned());
            }
            default_destinations += usize::from(destination.default);
        }
        if default_destinations > 1 {
            return Err("destinations.folders may contain only one default".to_owned());
        }
        let wishlist_interval_seconds =
            env_parse_layer(env, "SLSKD_WISHLIST_INTERVAL", None, 3_600_u64)?;
        if wishlist_interval_seconds < 300 {
            return Err("wishlist.interval_seconds must be at least 300".to_owned());
        }
        let wishlist_max_results =
            env_parse_layer(env, "SLSKD_WISHLIST_MAX_RESULTS", None, 100_usize)?;
        if !(10..=1_000).contains(&wishlist_max_results) {
            return Err("wishlist.max_results must be between 10 and 1000".to_owned());
        }
        let incoming_search = IncomingSearchSettings {
            concurrency: env_parse_layer(
                env,
                "SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY",
                None,
                10_usize,
            )?,
            circuit_breaker: env_parse_layer(
                env,
                "SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER",
                None,
                500_usize,
            )?,
            response_file_limit: env_parse_layer(
                env,
                "SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT",
                None,
                500_usize,
            )?,
        };
        if !(1..=100).contains(&incoming_search.concurrency) {
            return Err(
                "throttling.search.incoming.concurrency must be between 1 and 100".to_owned(),
            );
        }
        if !(100..=10_000).contains(&incoming_search.circuit_breaker) {
            return Err(
                "throttling.search.incoming.circuit_breaker must be between 100 and 10000"
                    .to_owned(),
            );
        }
        if !(100..=5_000).contains(&incoming_search.response_file_limit) {
            return Err(
                "throttling.search.incoming.response_file_limit must be between 100 and 5000"
                    .to_owned(),
            );
        }
        let core_workflow = CoreWorkflowSettings {
            rooms,
            liked_interests,
            hated_interests,
            destinations,
            wishlist: WishlistSettings {
                enabled: env_bool_layer(env, "SLSKD_WISHLIST_ENABLED", true)?,
                interval: Duration::from_secs(wishlist_interval_seconds),
                auto_download: env_bool_layer(env, "SLSKD_WISHLIST_AUTO_DOWNLOAD", false)?,
                max_results: wishlist_max_results,
            },
            incoming_search,
        };
        let listener_bind = optional_env_any(
            env,
            &["SLSKR_LISTENER_BIND", "SLSKD_SLSK_LISTEN_IP_ADDRESS"],
        )
        .map(|value| {
            if env.var("SLSKR_LISTENER_BIND").is_none() && value.parse::<IpAddr>().is_ok() {
                format_host_port(&value, u16::try_from(listen_port).unwrap_or(u16::MAX))
            } else {
                value
            }
        })
        .or(file_config.listeners.regular_bind)
        .or_else(|| {
            Some(
                SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                    u16::try_from(listen_port).unwrap_or(u16::MAX),
                )
                .to_string(),
            )
        });
        if env.var("SLSKR_LISTENER_BIND").is_none() {
            if let Some(address) = env.var("SLSKD_SLSK_LISTEN_IP_ADDRESS") {
                address.parse::<IpAddr>().map_err(|_| {
                    "Soulseek.ListenIpAddress specifies an invalid IPv4 or IPv6 IP address"
                        .to_owned()
                })?;
            }
        }
        if controller_compatibility_target == ControllerCompatibilityTarget::Slskdn
            && auto_connect
            && listener_bind.as_deref().is_some_and(|value| {
                value
                    .parse::<SocketAddr>()
                    .map(|address| address.ip().is_loopback())
                    .or_else(|_| value.parse::<IpAddr>().map(|address| address.is_loopback()))
                    .unwrap_or(false)
            })
        {
            return Err(
                "Soulseek.ListenIpAddress must not be a loopback address when the client is connecting. Use 0.0.0.0 or a reachable LAN/VPN interface instead."
                    .to_owned(),
            );
        }
        let advertised_port = env_parse_layer(
            env,
            "SLSKR_ADVERTISED_PORT",
            file_config.listeners.advertised_port,
            listen_port,
        )?;
        let upstream_obfuscated_port =
            env_parse_any_option(env, &["SLSKD_SLSK_OBFUSCATION_LISTEN_PORT"])?;
        let obfuscated_listener_bind = env
            .var("SLSKR_OBFUSCATED_LISTENER_BIND")
            .or_else(|| {
                upstream_obfuscated_port
                    .filter(|port| *port != 0)
                    .map(|port| {
                        let host = listener_bind
                            .as_deref()
                            .and_then(|value| value.parse::<SocketAddr>().ok())
                            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |bind| bind.ip());
                        SocketAddr::new(host, port).to_string()
                    })
            })
            .or(file_config.listeners.obfuscated_bind)
            .or_else(|| {
                (controller_compatibility_target == ControllerCompatibilityTarget::Slskdn
                    && listen_port < 65_535)
                    .then(|| {
                        let host = listener_bind
                            .as_deref()
                            .and_then(|value| value.parse::<SocketAddr>().ok())
                            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |bind| bind.ip());
                        SocketAddr::new(host, (listen_port + 1) as u16).to_string()
                    })
            });
        let obfuscated_advertised_port = if env.var("SLSKR_OBFUSCATED_ADVERTISED_PORT").is_some() {
            env_parse_option_layer(
                env,
                "SLSKR_OBFUSCATED_ADVERTISED_PORT",
                file_config.listeners.obfuscated_advertised_port,
            )?
        } else {
            upstream_obfuscated_port
                .filter(|port| *port != 0)
                .map(u32::from)
                .or(file_config.listeners.obfuscated_advertised_port)
                .or_else(|| {
                    (controller_compatibility_target == ControllerCompatibilityTarget::Slskdn
                        && listen_port < 65_535)
                        .then_some(listen_port + 1)
                })
        };
        let overlay_bind = env
            .var("SLSKR_OVERLAY_BIND")
            .or(file_config.listeners.overlay_bind)
            .map(|value| {
                let address = value
                    .parse::<SocketAddr>()
                    .map_err(|error| format!("invalid SLSKR_OVERLAY_BIND: {error}"))?;
                if address.port() == 0 {
                    return Err("SLSKR_OVERLAY_BIND port must be non-zero".to_owned());
                }
                Ok(address)
            })
            .transpose()?
            .or_else(|| {
                (controller_compatibility_target == ControllerCompatibilityTarget::Slskdn
                    && advanced_networking.overlay.enable)
                    .then_some(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        advanced_networking.dht.overlay_port,
                    ))
            });
        let dht_enabled = advanced_networking.dht.enabled;
        let dht_port = advanced_networking.dht.dht_port;
        let trusted_mesh_peers = trusted_mesh_peers_from_layers(
            env.var("SLSKR_TRUSTED_MESH_PEERS"),
            file_config.mesh.trusted_peers,
        )?;
        let obfuscation_enabled = env_bool_any_layer(
            env,
            &["SLSK_OBFUSCATION", "SLSKD_SLSK_OBFUSCATION"],
            file_config.network.obfuscation.enabled.unwrap_or(true),
        )?;
        let obfuscation_mode = SoulseekObfuscationMode::parse(
            optional_env_any(
                env,
                &["SLSK_OBFUSCATION_MODE", "SLSKD_SLSK_OBFUSCATION_MODE"],
            )
            .or(file_config.network.obfuscation.mode)
            .as_deref()
            .unwrap_or("compatibility"),
        )?;
        let obfuscation_listen_port = u32::from(upstream_obfuscated_port.unwrap_or_default());
        let obfuscation_advertise_regular_port = env_bool_any_layer(
            env,
            &[
                "SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
                "SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
            ],
            file_config
                .network
                .obfuscation
                .advertise_regular_port
                .unwrap_or(true),
        )?;
        let obfuscation_prefer_outbound = env_bool_any_layer(
            env,
            &[
                "SLSK_OBFUSCATION_PREFER_OUTBOUND",
                "SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND",
            ],
            file_config
                .network
                .obfuscation
                .prefer_outbound
                .unwrap_or(true),
        )?;
        let peer_host_override = env
            .var("SLSKR_PEER_HOST_OVERRIDE")
            .map(|value| {
                value
                    .parse::<Ipv4Addr>()
                    .map_err(|error| format!("invalid SLSKR_PEER_HOST_OVERRIDE: {error}"))
            })
            .transpose()?;
        let test_user_endpoint_overrides =
            parse_user_endpoint_overrides(env.var("SLSKR_TEST_USER_ENDPOINT_OVERRIDES"))?;
        let user_info_description = optional_env_any(
            env,
            &["SLSKR_USER_INFO_DESCRIPTION", "SLSKD_SLSK_DESCRIPTION"],
        )
        .or(file_config.profile.user_info_description)
        .unwrap_or_else(|| match controller_compatibility_target {
            ControllerCompatibilityTarget::Slskd => {
                "A slskd user. https://github.com/slskd/slskd".to_owned()
            }
            ControllerCompatibilityTarget::Slskdn => {
                "A slskdN user. Unofficial fork of slskd: https://github.com/snapetech/slskdn"
                    .to_owned()
            }
        });
        let user_info_picture = optional_env_any(
            env,
            &[
                "SLSKR_USER_INFO_PICTURE",
                "SLSKD_SLSK_PICTURE",
                "SLSK_PICTURE",
            ],
        )
        .or(file_config.profile.user_info_picture)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
        if let Some(path) = user_info_picture.as_deref() {
            let metadata = fs::metadata(path).map_err(|error| {
                format!(
                    "Soulseek picture '{}' is not readable: {error}",
                    path.display()
                )
            })?;
            if !metadata.is_file() {
                return Err(format!(
                    "Soulseek picture '{}' is not a regular file",
                    path.display()
                ));
            }
            fs::File::open(path).map_err(|error| {
                format!(
                    "Soulseek picture '{}' is not readable: {error}",
                    path.display()
                )
            })?;
        }
        let soulseek_diagnostic_level = SoulseekDiagnosticLevel::parse(
            optional_env_any(
                env,
                &[
                    "SLSKR_SLSK_DIAG_LEVEL",
                    "SLSKD_SLSK_DIAG_LEVEL",
                    "SLSK_DIAG_LEVEL",
                ],
            )
            .or(file_config.profile.soulseek_diagnostic_level)
            .as_deref()
            .unwrap_or("info"),
        )?;
        let soulseek_distributed = SoulseekDistributedSettings {
            disabled: env_bool_any_layer(
                env,
                &["SLSKR_SLSK_NO_DNET", "SLSKD_SLSK_NO_DNET", "SLSK_NO_DNET"],
                file_config
                    .network
                    .distributed_network
                    .disabled
                    .unwrap_or(false),
            )?,
            disable_children: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SLSK_DNET_NO_CHILDREN",
                    "SLSKD_SLSK_DNET_NO_CHILDREN",
                    "SLSK_DNET_NO_CHILDREN",
                ],
                file_config
                    .network
                    .distributed_network
                    .disable_children
                    .unwrap_or(false),
            )?,
            child_limit: bounded_config_value(
                "SLSK_DNET_CHILDREN",
                env_parse_any_layer(
                    env,
                    &[
                        "SLSKR_SLSK_DNET_CHILDREN",
                        "SLSKD_SLSK_DNET_CHILDREN",
                        "SLSK_DNET_CHILDREN",
                    ],
                    file_config.network.distributed_network.child_limit,
                    25_usize,
                )?,
                1,
                i32::MAX as usize,
            )?,
            logging: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SLSK_DNET_LOGGING",
                    "SLSKD_SLSK_DNET_LOGGING",
                    "SLSK_DNET_LOGGING",
                ],
                file_config
                    .network
                    .distributed_network
                    .logging
                    .unwrap_or(false),
            )?,
        };
        let peer_response_timeout_seconds = env_parse_layer(
            env,
            "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
            file_config.timeouts.peer_response_seconds,
            5_u64,
        )?;
        let peer_response_timeout = validated_runtime_interval(
            "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
            peer_response_timeout_seconds,
        )?;
        let soulseek_connection = SoulseekConnectionSettings::from_layers(
            file_config.network.connection,
            env,
            controller_compatibility_target,
        )?;
        let controller_case_sensitive_regex = env_bool_layer(
            env,
            "SLSKD_CASE_SENSITIVE_REGEX",
            file_config.flags.case_sensitive_reg_ex.unwrap_or(false),
        )?;
        let controller_search_request_filters = controller_string_array_layer(
            env,
            "SLSKD_SEARCH_REQUEST_FILTER",
            file_config.filters.search.request.clone(),
        );
        for filter in &controller_search_request_filters {
            crate::dotnet_regex::DotNetRegex::validate(filter).map_err(|_| {
                format!("Search request filter '{filter}' is not a valid regular expression")
            })?;
        }
        let canonical_groups = file_config.transfers.groups.clone();
        let compatibility_groups = file_config.groups.clone();
        let user_blacklist_file_config = match env.var("SLSKR_FROZEN_TRANSFER_GROUPS_JSON") {
            Some(json) => {
                serde_json::from_str::<GroupsFileConfig>(&json)
                    .map_err(|error| format!("invalid transfer groups configuration: {error}"))?
                    .blacklisted
            }
            None if groups_file_config_is_empty(&canonical_groups) => {
                compatibility_groups.blacklisted.clone()
            }
            None => canonical_groups.blacklisted.clone(),
        };
        let share_settings =
            ShareSettings::from_layers(file_config.shares, env, controller_compatibility_target)?;
        let transfer_history_limit = env_parse_layer(
            env,
            "SLSKR_TRANSFER_HISTORY_LIMIT",
            file_config.transfers.history_limit,
            500_usize,
        )?;
        let transfer_max_active = env_parse_layer(
            env,
            "SLSKR_TRANSFER_MAX_ACTIVE",
            file_config.transfers.max_active,
            3_usize,
        )?;
        let transfer_allow_inbound = env_bool_layer(
            env,
            "SLSKR_TRANSFER_ALLOW_INBOUND",
            file_config.transfers.allow_inbound.unwrap_or(true),
        )?;
        let transfer_allow_outbound = env_bool_layer(
            env,
            "SLSKR_TRANSFER_ALLOW_OUTBOUND",
            file_config.transfers.allow_outbound.unwrap_or(true),
        )?;
        let transfer_upload =
            TransferUploadSettings::from_layers(file_config.transfers.upload, env)?;
        let transfer_download = TransferDownloadSettings::from_layers(
            file_config.transfers.download,
            env,
            controller_compatibility_target,
        )?;
        let transfer_groups = TransferGroupsSettings::from_layers(
            canonical_groups,
            compatibility_groups,
            env,
            controller_compatibility_target,
        )?;
        let transfer_auto_retry =
            TransferAutoRetrySettings::from_layers(file_config.transfers.auto_retry, env)?;
        let transfer_rescue =
            TransferRescueSettings::from_layers(file_config.transfers.rescue, env)?;
        let managed_blacklist = ManagedBlacklistSettings::from_layers(
            file_config.blacklist,
            &user_blacklist_file_config,
            env,
            controller_compatibility_target,
        )?;
        let download_completed_path_template = optional_env_any(
            env,
            &[
                "SLSKR_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
                "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
            ],
        )
        .or(file_config.transfers.completed_path_template)
        .unwrap_or_default();
        if download_completed_path_template.len() > MAX_COMPLETED_PATH_TEMPLATE_BYTES {
            return Err(format!(
                "download completed path template exceeds {MAX_COMPLETED_PATH_TEMPLATE_BYTES} bytes"
            ));
        }
        if download_completed_path_template.contains('\0') {
            return Err("download completed path template contains a NUL byte".to_owned());
        }
        let private_message_auto_response = PrivateMessageAutoResponseSettings::from_layers(
            file_config.network.private_message_auto_response,
            env,
            if controller_compatibility_target == ControllerCompatibilityTarget::Slskdn {
                "Hi, I'm human and testing a slskdN client. Shares may be temporarily unavailable while I validate the client."
            } else {
                "Hi, I'm human and testing a slskd client. Shares may be temporarily unavailable while I validate the client."
            },
        )?;
        let remote_configuration = env_bool_any_layer(
            env,
            &["SLSKR_REMOTE_CONFIGURATION", "SLSKD_REMOTE_CONFIGURATION"],
            file_config
                .compatibility
                .remote_configuration
                .unwrap_or(false),
        )?;
        let remote_file_management = env_bool_any_layer(
            env,
            &[
                "SLSKR_REMOTE_FILE_MANAGEMENT",
                "SLSKD_REMOTE_FILE_MANAGEMENT",
            ],
            false,
        )?;
        let controller_debug = env_bool_any_layer(
            env,
            &["SLSKR_DEBUG", "SLSKD_DEBUG"],
            file_config.compatibility.debug.unwrap_or(false),
        )?;
        let controller_no_config_watch = env_bool_any_layer(
            env,
            &["SLSKR_NO_CONFIG_WATCH", "SLSKD_NO_CONFIG_WATCH"],
            file_config.compatibility.no_config_watch.unwrap_or(false),
        )?;
        let controller_no_logo = env_bool_layer(
            env,
            "SLSKD_NO_LOGO",
            file_config.flags.no_logo.unwrap_or(false),
        )?;
        let controller_no_start = env_bool_layer(
            env,
            "SLSKD_NO_START",
            file_config.flags.no_start.unwrap_or(false),
        )?;
        let controller_no_version_check = env_bool_layer(
            env,
            "SLSKD_NO_VERSION_CHECK",
            file_config.flags.no_version_check.unwrap_or(false),
        )?;
        let controller_experimental = env_bool_layer(
            env,
            "SLSKD_EXPERIMENTAL",
            file_config.flags.experimental.unwrap_or(false),
        )?;
        let controller_hash_from_audio_file_enabled = env_bool_layer(
            env,
            "SLSKR_CONTROLLER_YAML_HASH_FROM_AUDIO_FILE_ENABLED",
            file_config
                .flags
                .hash_from_audio_file_enabled
                .unwrap_or(false),
        )?;
        let controller_no_share_scan = env_bool_layer(
            env,
            "SLSKD_NO_SHARE_SCAN",
            file_config.flags.no_share_scan.unwrap_or(false),
        )?;
        let controller_force_share_scan = env_bool_layer(
            env,
            "SLSKD_FORCE_SHARE_SCAN",
            file_config.flags.force_share_scan.unwrap_or(false),
        )?;
        let api_token = env.var("SLSKR_API_TOKEN").or(file_config.auth.api_token);
        let api_read_write_token = env
            .var("SLSKR_API_READ_WRITE_TOKEN")
            .or(file_config.auth.read_write_token);
        let api_read_only_token = env
            .var("SLSKR_API_READ_ONLY_TOKEN")
            .or(file_config.auth.read_only_token);
        let api_nowplaying_token = env
            .var("SLSKR_API_NOWPLAYING_TOKEN")
            .or(file_config.auth.nowplaying_token);
        let configured_tokens = [
            api_token.as_deref(),
            api_read_write_token.as_deref(),
            api_read_only_token.as_deref(),
            api_nowplaying_token.as_deref(),
        ];
        for token in configured_tokens.into_iter().flatten() {
            validate_api_token(token)?;
        }
        let token_count = configured_tokens.into_iter().flatten().count();
        let unique_token_count = configured_tokens
            .into_iter()
            .flatten()
            .collect::<std::collections::HashSet<_>>()
            .len();
        if token_count != unique_token_count {
            return Err("API tokens for different roles must be distinct".to_owned());
        }
        let auth_disabled = env_bool_any_layer(
            env,
            &["SLSKR_AUTH_DISABLED", "SLSKD_NO_AUTH"],
            file_config.auth.disabled.unwrap_or(false),
        )?;
        let auth_required = !auth_disabled;
        let api_cookie_auth_enabled = env_bool_layer(
            env,
            "SLSKR_API_COOKIE_AUTH_ENABLED",
            file_config.auth.cookie_auth_enabled.unwrap_or(false),
        )?;
        let api_rate_limit_anonymous = env_parse_layer(
            env,
            "SLSKR_API_RATE_LIMIT_ANONYMOUS",
            file_config.auth.rate_limit_anonymous,
            1000_u32,
        )?;
        let api_rate_limit_authenticated = env_parse_layer(
            env,
            "SLSKR_API_RATE_LIMIT_AUTHENTICATED",
            file_config.auth.rate_limit_authenticated,
            5000_u32,
        )?;
        let web_max_request_body_size_default =
            if controller_compatibility_target == ControllerCompatibilityTarget::Slskdn {
                10 * 1024 * 1024
            } else {
                crate::http_server::BODY_SIZE_LIMIT as i64
            };
        let controller_web_max_request_body_size = env_parse_layer(
            env,
            "SLSKD_WEB_MAX_REQUEST_BODY_SIZE",
            file_config.web.max_request_body_size,
            web_max_request_body_size_default,
        )?;
        if !(1..=i32::MAX as i64).contains(&controller_web_max_request_body_size) {
            return Err("web.max_request_body_size must be between 1 and 2147483647".to_owned());
        }
        let controller_web_max_request_body_size = controller_web_max_request_body_size as usize;
        let controller_web_enforce_security = env_bool_layer(
            env,
            "SLSKD_ENFORCE_SECURITY",
            file_config.web.enforce_security.unwrap_or(false),
        )?;
        let controller_web_allow_remote_no_auth = env_bool_layer(
            env,
            "SLSKD_ALLOW_REMOTE_NO_AUTH",
            file_config.web.allow_remote_no_auth.unwrap_or(false),
        )?;
        let controller_web_passthrough_allowed_cidrs = env
            .var("SLSKD_PASSTHROUGH_ALLOWED_CIDRS")
            .or(file_config.web.passthrough_allowed_cidrs);
        let controller_web_passthrough_cidrs =
            controller_passthrough_cidrs(controller_web_passthrough_allowed_cidrs.as_deref());
        let controller_diagnostics_allow_memory_dump = env_bool_layer(
            env,
            "SLSKD_ALLOW_MEMORY_DUMP",
            file_config.diagnostics.allow_memory_dump.unwrap_or(false),
        )?;
        let controller_diagnostics_allow_remote_dump = env_bool_layer(
            env,
            "SLSKD_ALLOW_REMOTE_DUMP",
            file_config.diagnostics.allow_remote_dump.unwrap_or(false),
        )?;
        let controller_web_cors = ControllerWebCorsSettings {
            enabled: env_bool_layer(
                env,
                "SLSKD_WEB_CORS_ENABLED",
                file_config.web.cors.enabled.unwrap_or(false),
            )?,
            allow_credentials: env_bool_layer(
                env,
                "SLSKD_WEB_CORS_ALLOW_CREDENTIALS",
                file_config.web.cors.allow_credentials.unwrap_or(false),
            )?,
            allowed_origins: controller_string_array_layer(
                env,
                "SLSKD_WEB_CORS_ALLOWED_ORIGINS",
                file_config.web.cors.allowed_origins,
            ),
            allowed_headers: controller_string_array_layer(
                env,
                "SLSKD_WEB_CORS_ALLOWED_HEADERS",
                file_config.web.cors.allowed_headers,
            ),
            allowed_methods: controller_string_array_layer(
                env,
                "SLSKD_WEB_CORS_ALLOWED_METHODS",
                file_config.web.cors.allowed_methods,
            ),
        };
        let web_rate_limiting_defaults =
            ControllerWebRateLimitingSettings::defaults(controller_compatibility_target);
        let controller_web_rate_limiting = ControllerWebRateLimitingSettings {
            enabled: env_bool_layer(
                env,
                "SLSKD_WEB_RATE_LIMITING",
                file_config
                    .web
                    .rate_limiting
                    .enabled
                    .unwrap_or(web_rate_limiting_defaults.enabled),
            )?,
            api_permit_limit: env_parse_layer(
                env,
                "SLSKD_WEB_API_PERMIT_LIMIT",
                file_config.web.rate_limiting.api_permit_limit,
                web_rate_limiting_defaults.api_permit_limit,
            )?,
            api_window_seconds: env_parse_layer(
                env,
                "SLSKD_WEB_API_WINDOW_SECONDS",
                file_config.web.rate_limiting.api_window_seconds,
                web_rate_limiting_defaults.api_window_seconds,
            )?,
            federation_permit_limit: env_parse_layer(
                env,
                "SLSKD_WEB_FEDERATION_PERMIT_LIMIT",
                file_config.web.rate_limiting.federation_permit_limit,
                web_rate_limiting_defaults.federation_permit_limit,
            )?,
            federation_window_seconds: env_parse_layer(
                env,
                "SLSKD_WEB_FEDERATION_WINDOW_SECONDS",
                file_config.web.rate_limiting.federation_window_seconds,
                web_rate_limiting_defaults.federation_window_seconds,
            )?,
            mesh_gateway_permit_limit: env_parse_layer(
                env,
                "SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT",
                file_config.web.rate_limiting.mesh_gateway_permit_limit,
                web_rate_limiting_defaults.mesh_gateway_permit_limit,
            )?,
            mesh_gateway_window_seconds: env_parse_layer(
                env,
                "SLSKD_WEB_MESH_GATEWAY_WINDOW_SECONDS",
                file_config.web.rate_limiting.mesh_gateway_window_seconds,
                web_rate_limiting_defaults.mesh_gateway_window_seconds,
            )?,
        };
        let trusted_proxy_cidrs = trusted_proxy_cidrs_from_layers(
            env.var("SLSKR_TRUSTED_PROXY_CIDRS"),
            file_config.auth.trusted_proxy_cidrs,
        )?;
        let persistence_enabled = env_bool_layer(
            env,
            "SLSKR_PERSISTENCE_ENABLED",
            file_config.persistence.enabled.unwrap_or(false),
        )?;
        let pod_join_signature_mode = PodSignatureMode::parse(
            env.var("SLSKR_POD_JOIN_SIGNATURE_MODE")
                .or(file_config.podcore.join.signature_mode)
                .unwrap_or_else(|| "off".to_owned())
                .as_str(),
        )?;
        let virtual_soulfind_v2_enabled = env_bool_layer(
            env,
            "SLSKR_VIRTUAL_SOULFIND_V2_ENABLED",
            file_config.virtual_soulfind_v2.enabled.unwrap_or(true),
        )?;
        let mut integrations = IntegrationSettings::from_layers(file_config.integrations, env)?;
        integrations.external_visualizer = media_services.external_visualizer.clone();

        Ok(Self {
            config_file,
            http_bind,
            http_binds,
            controller_http_address,
            state_dir,
            instance_name,
            downloads_dir,
            incomplete_dir,
            server_address,
            listen_port,
            username,
            password,
            credential_store,
            credential_file,
            auto_connect,
            reconnect,
            reconnect_delay,
            ping_interval,
            log_level,
            daemon_flags,
            logger,
            permissions_file_mode,
            telemetry_tracing,
            retention,
            search_retention,
            core_workflow,
            advanced_networking,
            media_services,
            controller_web,
            controller_api_keys,
            listener_bind,
            advertised_port,
            obfuscated_listener_bind,
            obfuscated_advertised_port,
            overlay_bind,
            dht_enabled,
            dht_port,
            trusted_mesh_peers,
            obfuscation_enabled,
            obfuscation_mode,
            obfuscation_listen_port,
            obfuscation_advertise_regular_port,
            obfuscation_prefer_outbound,
            peer_host_override,
            test_user_endpoint_overrides,
            user_info_description,
            user_info_picture,
            soulseek_diagnostic_level,
            soulseek_distributed,
            peer_response_timeout,
            soulseek_connection,
            share_settings,
            transfer_history_limit,
            transfer_max_active,
            transfer_allow_inbound,
            transfer_allow_outbound,
            transfer_upload,
            transfer_download,
            transfer_groups,
            transfer_auto_retry,
            transfer_rescue,
            managed_blacklist,
            download_completed_path_template,
            private_message_auto_response,
            pod_join_signature_mode,
            virtual_soulfind_v2_enabled,
            controller_compatibility_target,
            controller_headless,
            remote_configuration,
            remote_file_management,
            controller_debug,
            controller_no_config_watch,
            controller_no_logo,
            controller_no_start,
            controller_no_version_check,
            controller_experimental,
            controller_hash_from_audio_file_enabled,
            controller_case_sensitive_regex,
            controller_search_request_filters,
            controller_no_share_scan,
            controller_force_share_scan,
            controller_swagger,
            controller_metrics_enabled,
            controller_metrics_url,
            controller_metrics_auth_disabled,
            controller_metrics_username,
            controller_metrics_password,
            controller_web_auth_username,
            controller_web_auth_password,
            controller_web_jwt_key,
            controller_web_jwt_key_configured,
            controller_web_jwt_ttl_millis,
            auth_required,
            api_token,
            api_read_write_token,
            api_read_only_token,
            api_nowplaying_token,
            api_cookie_auth_enabled,
            api_rate_limit_anonymous,
            api_rate_limit_authenticated,
            controller_web_enforce_security,
            controller_web_allow_remote_no_auth,
            controller_web_passthrough_allowed_cidrs,
            controller_web_passthrough_cidrs,
            controller_web_max_request_body_size,
            controller_web_cors,
            controller_web_rate_limiting,
            controller_diagnostics_allow_memory_dump,
            controller_diagnostics_allow_remote_dump,
            trusted_proxy_cidrs,
            persistence_enabled,
            integrations,
        })
    }

    pub fn credentials(&self) -> Option<LoginCredentials> {
        Some(LoginCredentials::default_client(
            self.username.clone()?,
            self.password.clone()?,
        ))
    }

    pub fn controller_passthrough_allows(&self, remote: Option<SocketAddr>) -> bool {
        let Some(remote) = remote else {
            return false;
        };
        let ip = match remote.ip() {
            IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4),
            ip => ip,
        };
        ip.is_loopback()
            || (self.controller_web_allow_remote_no_auth
                && self
                    .controller_web_passthrough_cidrs
                    .iter()
                    .any(|cidr| cidr.contains(ip)))
    }

    pub fn validate_controller_startup_hardening(&self) -> Result<(), String> {
        if self.controller_compatibility_target != ControllerCompatibilityTarget::Slskdn {
            return Ok(());
        }

        let check = |condition: bool, rule: &str, message: &str| -> Result<(), String> {
            if !condition {
                return Ok(());
            }
            if self.controller_web_enforce_security {
                Err(format!("[{rule}] {message}"))
            } else {
                eprintln!("warning: [{rule}] {message}");
                Ok(())
            }
        };

        check(
            !self.auth_required
                && self.http_binds.iter().any(|address| !address.ip().is_loopback())
                && !self.controller_web_allow_remote_no_auth,
            "AuthDisabledNonLoopback",
            "Authentication is disabled and the application binds to a non-loopback address. Set Web.AllowRemoteNoAuth=true to allow, or bind to loopback only.",
        )?;
        check(
            !self.auth_required
                && self.controller_web_allow_remote_no_auth
                && self
                    .controller_web_passthrough_allowed_cidrs
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty()),
            "RemoteNoAuthWithoutCidrs",
            "Web.AllowRemoteNoAuth is enabled without Web.Authentication.Passthrough.AllowedCidrs. Remote no-auth access must be constrained to explicit CIDRs.",
        )?;
        check(
            self.controller_web_cors.enabled
                && self.controller_web_cors.allow_credentials
                && (self.controller_web_cors.allowed_origins.is_empty()
                    || self
                        .controller_web_cors
                        .allowed_origins
                        .iter()
                        .any(|origin| origin.eq_ignore_ascii_case("*"))),
            "CorsCredentialsWithWildcard",
            "CORS is configured with AllowCredentials and wildcard/any origin, which is unsafe. Use an explicit AllowedOrigins list and no wildcard.",
        )?;
        check(
            self.controller_diagnostics_allow_memory_dump && !self.auth_required,
            "MemoryDumpWithAuthDisabled",
            "Diagnostics.AllowMemoryDump is true while authentication is disabled. Enable authentication or set AllowMemoryDump=false.",
        )?;
        check(
            self.controller_metrics_enabled
                && !self.controller_metrics_auth_disabled
                && self.controller_metrics_password.trim().is_empty(),
            "WeakMetricsPassword",
            "Web.Authentication.Metrics.Password is empty. The Prometheus metrics endpoint will be protected with no password. Set a strong password via web.authentication.metrics.password or disable the metrics endpoint.",
        )?;
        if self.controller_hash_from_audio_file_enabled {
            return Err(
                "[HashFromAudioFileEnabled] Flags.HashFromAudioFileEnabled is true but audio hash from file requires unavailable PCM extraction support. Set it to false; this option is not supported in this build."
                    .to_owned(),
            );
        }
        Ok(())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"config_file\":{},\"http_bind\":\"{}\",\"state_dir\":\"{}\",\"server_address\":\"{}\",\"listen_port\":{},\"advertised_port\":{},\"listener_bind\":{},\"obfuscated_listener_bind\":{},\"obfuscated_advertised_port\":{},\"overlay_bind\":{},\"dht_enabled\":{},\"dht_port\":{},\"trusted_mesh_peers\":{},\"obfuscation\":{},\"peer_host_override\":{},\"test_user_endpoint_overrides\":{},\"username\":{},\"credentials_configured\":{},\"credential_store\":\"{}\",\"credential_file\":\"{}\",\"auto_connect\":{},\"reconnect\":{},\"reconnect_seconds\":{},\"ping_seconds\":{},\"log_level\":\"{}\",\"peer_response_timeout_seconds\":{},\"share_roots\":{},\"share_follow_symlinks\":{},\"share_include_hidden\":{},\"share_scan_max_files\":{},\"share_cache_tsv_enabled\":{},\"transfer_history_limit\":{},\"transfer_max_active\":{},\"transfer_allow_inbound\":{},\"transfer_allow_outbound\":{},\"transfer_auto_retry\":{},\"transfer_rescue\":{},\"download_completed_path_template_configured\":{},\"private_message_auto_response\":{},\"pod_join_signature_mode\":\"{}\",\"virtual_soulfind_v2_enabled\":{},\"controller_compatibility_target\":\"{}\",\"remote_configuration\":{},\"auth_required\":{},\"api_token_configured\":{},\"api_read_write_token_configured\":{},\"api_read_only_token_configured\":{},\"api_nowplaying_token_configured\":{},\"api_cookie_auth_enabled\":{},\"trusted_proxy_cidrs\":{},\"persistence_enabled\":{},\"integrations\":{}}}",
            json_option(
                self.config_file
                    .as_ref()
                    .map(|_| "config://file".to_owned())
                    .as_deref()
            ),
            json_escape(&self.http_bind.to_string()),
            "state://configured",
            json_escape(&self.server_address),
            self.listen_port,
            self.advertised_port,
            json_option(self.listener_bind.as_deref()),
            json_option(self.obfuscated_listener_bind.as_deref()),
            json_u32_option(self.obfuscated_advertised_port),
            json_option(self.overlay_bind.map(|bind| bind.to_string()).as_deref()),
            self.dht_enabled,
            self.dht_port,
            self.trusted_mesh_peers.len(),
            format_args!(
                "{{\"enabled\":{},\"mode\":\"{}\",\"listen_port\":{},\"advertise_regular_port\":{},\"prefer_outbound\":{},\"effective_prefer_outbound\":{}}}",
                self.obfuscation_enabled,
                self.obfuscation_mode.as_str(),
                self.obfuscation_listen_port,
                self.obfuscation_advertise_regular_port,
                self.obfuscation_prefer_outbound,
                self.prefer_obfuscated_outbound(),
            ),
            json_option(self.peer_host_override.map(|ip| ip.to_string()).as_deref()),
            self.test_user_endpoint_overrides.len(),
            json_option(self.username.as_deref().map(redact_username).as_deref()),
            self.username.is_some() && self.password.is_some(),
            self.credential_store.as_str(),
            "credential://configured",
            self.auto_connect,
            self.reconnect,
            self.reconnect_delay.as_secs(),
            self.ping_interval.as_secs(),
            json_escape(&self.log_level),
            self.peer_response_timeout.as_secs(),
            self.share_settings.roots.len(),
            self.share_settings.follow_symlinks,
            self.share_settings.include_hidden,
            self.share_settings.max_files,
            self.share_settings.cache_tsv_enabled,
            self.transfer_history_limit,
            self.transfer_max_active,
            self.transfer_allow_inbound,
            self.transfer_allow_outbound,
            self.transfer_auto_retry.sanitized_json(),
            self.transfer_rescue.sanitized_json(),
            !self.download_completed_path_template.is_empty(),
            self.private_message_auto_response.sanitized_json(),
            self.pod_join_signature_mode.as_str(),
            self.virtual_soulfind_v2_enabled,
            self.controller_compatibility_target.as_str(),
            self.remote_configuration,
            self.auth_required,
            self.api_token.is_some(),
            self.api_read_write_token.is_some(),
            self.api_read_only_token.is_some(),
            self.api_nowplaying_token.is_some(),
            self.api_cookie_auth_enabled,
            self.trusted_proxy_cidrs.len(),
            self.persistence_enabled,
            self.integrations.sanitized_json()
        )
    }

    pub fn prefer_obfuscated_outbound(&self) -> bool {
        self.obfuscation_enabled
            && self.obfuscation_mode == SoulseekObfuscationMode::Prefer
            && self.obfuscation_prefer_outbound
    }
}

pub fn parse_compat_ip_address(value: &str) -> Result<IpAddr, String> {
    let value = value.trim();
    let unbracketed = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(value);
    if let Ok(address) = unbracketed.parse::<IpAddr>() {
        return Ok(address);
    }
    if unbracketed.is_empty() || unbracketed.contains(':') {
        return Err("invalid IPv4 or IPv6 address".to_owned());
    }
    let parts = unbracketed
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err("invalid IPv4 or IPv6 address".to_owned());
            }
            part.parse::<u32>()
                .map_err(|_| "invalid IPv4 or IPv6 address".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let value = match parts.as_slice() {
        [value] => *value,
        [a, b] if *a <= u8::MAX.into() && *b <= 0x00ff_ffff => (a << 24) | b,
        [a, b, c] if *a <= u8::MAX.into() && *b <= u8::MAX.into() && *c <= u16::MAX.into() => {
            (a << 24) | (b << 16) | c
        }
        [a, b, c, d] if [a, b, c, d].into_iter().all(|part| *part <= u8::MAX.into()) => {
            (a << 24) | (b << 16) | (c << 8) | d
        }
        _ => return Err("invalid IPv4 or IPv6 address".to_owned()),
    };
    Ok(IpAddr::V4(Ipv4Addr::from(value)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrustedMeshPeer {
    pub peer_id: String,
    pub username: String,
    pub overlay_endpoint: SocketAddr,
    pub certificate_sha256: [u8; 32],
    pub range_endpoint: Option<String>,
}

impl TrustedMeshPeer {
    pub fn matches(&self, identity: &str) -> bool {
        self.peer_id.eq_ignore_ascii_case(identity) || self.username.eq_ignore_ascii_case(identity)
    }

    pub fn range_url(
        &self,
        expected_hash: &str,
        size: u64,
        recording_id: Option<&str>,
    ) -> Option<String> {
        if expected_hash.len() != 64 || !expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        let endpoint = self.range_endpoint.as_deref()?;
        Some(
            endpoint
                .replace("{sha256}", expected_hash)
                .replace("{size}", &size.to_string())
                .replace(
                    "{recordingId}",
                    &crate::url_encode(recording_id.unwrap_or_default()),
                ),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoulseekObfuscationMode {
    Compatibility,
    Prefer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerCompatibilityTarget {
    Slskd,
    Slskdn,
}

impl ControllerCompatibilityTarget {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "slskd" => Ok(Self::Slskd),
            "slskdn" => Ok(Self::Slskdn),
            _ => Err("SLSKR_CONTROLLER_COMPATIBILITY_TARGET must be slskd or slskdn".to_owned()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Slskd => "slskd",
            Self::Slskdn => "slskdn",
        }
    }
}

fn validate_controller_storage_directory(
    field: &str,
    path: &std::path::Path,
    target: ControllerCompatibilityTarget,
    explicitly_configured: bool,
) -> Result<(), String> {
    if !explicitly_configured {
        return Ok(());
    }
    if path.as_os_str().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if target == ControllerCompatibilityTarget::Slskd && !path.is_absolute() {
        return Err(format!(
            "{field} must be an absolute path for the slskd compatibility target"
        ));
    }
    let metadata =
        fs::metadata(path).map_err(|_| format!("{field} specifies a non-existent directory"))?;
    if !metadata.is_dir() {
        return Err(format!("{field} must specify a directory"));
    }

    let probe = path.join(format!(
        ".slskr-write-probe-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|_| format!("{field} must specify a writable directory"))?;
    drop(file);
    fs::remove_file(&probe).map_err(|_| format!("{field} writeability probe cleanup failed"))?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum PodSignatureMode {
    Off,
    Warn,
    Enforce,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdvancedNetworkingSettings {
    pub dht: DhtSettings,
    pub mesh: MeshRuntimeSettings,
    pub mesh_sync_security: MeshSyncSecuritySettings,
    pub pod_join_signature_mode: PodSignatureMode,
    pub pod_security_signature_mode: PodSignatureMode,
    pub overlay: OverlaySettings,
    pub overlay_data: OverlayDataSettings,
    pub relay: RelaySettings,
    pub security: SecuritySettings,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DhtSettings {
    pub enabled: bool,
    pub dht_port: u16,
    pub overlay_port: u16,
    pub advertised_overlay_port: u16,
    pub vpn_port_sync: String,
    pub bootstrap_routers: Vec<String>,
    pub announce_interval: Duration,
    pub discovery_interval: Duration,
    pub min_neighbors: usize,
    pub bootstrap_timeout: Duration,
    pub cold_bootstrap_timeout: Duration,
    pub lan_only_bootstrap_timeout: Duration,
    pub lan_only: bool,
    pub enable_upnp: bool,
    pub enable_stun: bool,
}

impl DhtSettings {
    pub fn effective_overlay_port(&self) -> u16 {
        if self.advertised_overlay_port == 0 {
            self.overlay_port
        } else {
            self.advertised_overlay_port
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MeshRuntimeSettings {
    pub enabled: bool,
    pub enable_soulseek_capability_handshake: bool,
    pub enable_soulseek_rendezvous: bool,
    pub probe_soulseek_rendezvous_capabilities: bool,
    pub dht_bootstrap_nodes: usize,
    pub udp_port: u16,
    pub quic_port: u16,
    pub enforce_remote_payload_limits: bool,
    pub max_remote_payload_size: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MeshSyncSecuritySettings {
    pub max_invalid_entries_per_window: u32,
    pub max_invalid_messages_per_window: u32,
    pub rate_limit_window: Duration,
    pub quarantine_violation_threshold: u32,
    pub quarantine_duration: Duration,
    pub proof_of_possession_enabled: bool,
    pub consensus_min_peers: usize,
    pub consensus_min_agreements: usize,
    pub alert_threshold_signature_failures: u32,
    pub alert_threshold_rate_limit_violations: u32,
    pub alert_threshold_quarantine_events: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OverlaySettings {
    pub enable: bool,
    pub listen_port: u16,
    pub enable_quic: bool,
    pub quic_listen_port: u16,
    pub share_quic_with_dht_port: bool,
    pub quic_backend_listen_port: u16,
    pub trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OverlayDataSettings {
    pub enable: bool,
    pub listen_port: u16,
    pub relay_authentication_token: String,
    pub allowed_relay_destinations: Vec<String>,
    pub max_concurrent_relays: usize,
    pub max_relay_bytes_per_direction: u64,
    pub max_relay_duration: Duration,
    pub trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RelaySettings {
    pub enabled: bool,
    pub mode: String,
    pub controller: RelayControllerSettings,
    pub agents: BTreeMap<String, RelayAgentSettings>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RelayControllerSettings {
    pub address: String,
    pub ignore_certificate_errors: bool,
    pub pinned_spki: String,
    pub api_key: String,
    pub secret: String,
    pub downloads: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RelayAgentSettings {
    pub instance_name: String,
    pub secret: String,
    pub cidr: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SecuritySettings {
    pub enabled: bool,
    pub profile: String,
    pub network_guard: NetworkGuardSettings,
    pub path_guard: PathGuardSettings,
    pub content_safety: ContentSafetySettings,
    pub peer_reputation: PeerReputationSettings,
    pub violation_tracker: ViolationTrackerSettings,
    pub adversarial: AdversarialSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NetworkGuardSettings {
    pub enabled: bool,
    pub max_connections_per_ip: usize,
    pub max_global_connections: usize,
    pub max_messages_per_minute: u32,
    pub max_message_size: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PathGuardSettings {
    pub enabled: bool,
    pub max_path_length: usize,
    pub max_path_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ContentSafetySettings {
    pub enabled: bool,
    pub verify_magic_bytes: bool,
    pub quarantine_suspicious: bool,
    pub quarantine_directory: PathBuf,
    pub block_executables: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PeerReputationSettings {
    pub enabled: bool,
    pub trusted_threshold: u8,
    pub untrusted_threshold: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ViolationTrackerSettings {
    pub enabled: bool,
    pub violations_before_auto_ban: u32,
    pub base_ban_duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdversarialSettings {
    pub max_unpadded_bytes: usize,
    pub max_padded_bytes: usize,
    pub relay_peer_data_endpoints: Vec<String>,
    pub relay_authentication_token: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaAdvancedServiceSettings {
    pub features: FeatureGateSettings,
    pub external_visualizer: ExternalVisualizerSettings,
    pub solid: SolidSettings,
    pub song_id_max_concurrent_runs: usize,
    pub virtual_soulfind: VirtualSoulfindSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FeatureGateSettings {
    pub collections_sharing: bool,
    pub streaming: bool,
    pub streaming_relay_fallback: bool,
    pub mesh_parallel_search: bool,
    pub mesh_publish_availability: bool,
    pub identity_friends: bool,
    pub solid: bool,
    pub scene_pod_bridge: bool,
    pub scene_pod_bridge_proxy_transfers: bool,
    pub scene_pod_bridge_export_pod_availability: bool,
    pub song_id: bool,
    pub mesh: bool,
    pub dht: bool,
    pub pods: bool,
    pub social_federation: bool,
    pub virtual_soulfind: bool,
    pub multi_source_downloads: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SolidSettings {
    pub allow_insecure_http: bool,
    pub max_fetch_bytes: usize,
    pub timeout: Duration,
    pub allowed_hosts: Vec<String>,
    pub redirect_path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VirtualSoulfindSettings {
    pub bridge: VirtualSoulfindBridgeSettings,
    pub disaster_mode: VirtualSoulfindDisasterModeSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VirtualSoulfindBridgeSettings {
    pub enabled: bool,
    pub port: u16,
    pub bind_address: IpAddr,
    pub max_clients: usize,
    pub require_auth: bool,
    pub password: String,
    pub max_requests_per_minute: u32,
    pub max_transfers_per_session: u32,
}

impl VirtualSoulfindBridgeSettings {
    pub fn endpoint_configured(&self) -> bool {
        self.port != 0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VirtualSoulfindDisasterModeSettings {
    pub auto: bool,
    pub force: bool,
    pub unavailable_threshold: Duration,
    pub enable_graceful_degradation: bool,
    pub recovery_check_interval: Duration,
    pub recovery_healthy_checks_required: u32,
}

impl PodSignatureMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "warn" => Ok(Self::Warn),
            "enforce" => Ok(Self::Enforce),
            _ => Err("SLSKR_POD_JOIN_SIGNATURE_MODE must be off, warn, or enforce".to_owned()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Warn => "warn",
            Self::Enforce => "enforce",
        }
    }
}

impl SoulseekObfuscationMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "compatibility" => Ok(Self::Compatibility),
            "prefer" => Ok(Self::Prefer),
            "only" => Err("Soulseek obfuscation only mode is not supported because regular fallback is required for legacy compatibility".to_owned()),
            _ => Err("SLSK_OBFUSCATION_MODE must be compatibility or prefer".to_owned()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatibility => "compatibility",
            Self::Prefer => "prefer",
        }
    }
}

fn validate_api_token(token: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("HTTP API token must not be empty or whitespace-only".to_owned());
    }
    if token != token.trim() {
        return Err("HTTP API token must not have surrounding whitespace".to_owned());
    }
    if token.chars().any(char::is_control) {
        return Err("HTTP API token must not contain control characters".to_owned());
    }
    if token.len() > crate::http_server::MAX_API_TOKEN_BYTES {
        return Err(format!(
            "HTTP API token exceeds the maximum representable header length of {} bytes",
            crate::http_server::MAX_API_TOKEN_BYTES
        ));
    }
    Ok(())
}

fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
    if seconds == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    let duration = Duration::from_secs(seconds);
    if std::time::Instant::now().checked_add(duration).is_none() {
        return Err(format!("{name} exceeds the runtime timer range"));
    }
    Ok(duration)
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferAutoRetrySettings {
    pub enabled: bool,
    pub retry_delay: Duration,
    pub check_interval: Duration,
    pub max_attempts: usize,
    pub max_files_per_cycle: usize,
    pub max_files_per_peer_per_cycle: usize,
    pub peer_cooldown: Duration,
    pub alternate_sources_enabled: bool,
    pub max_alternate_source_searches_per_cycle: usize,
    pub alternate_source_size_tolerance_percent: f64,
}

impl TransferAutoRetrySettings {
    fn from_layers<E: ConfigEnv>(
        file: TransferAutoRetryFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let retry_delay_seconds = bounded_config_value(
            "SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS",
                file.retry_delay_seconds,
                1800_u64,
            )?,
            10,
            86_400,
        )?;
        let check_interval_seconds = bounded_config_value(
            "SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS",
                file.check_interval_seconds,
                300_u64,
            )?,
            10,
            3_600,
        )?;
        let peer_cooldown_seconds = bounded_config_value(
            "SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS",
                file.peer_cooldown_seconds,
                900_u64,
            )?,
            60,
            86_400,
        )?;
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_ENABLED",
                file.enabled.unwrap_or(true),
            )?,
            retry_delay: Duration::from_secs(retry_delay_seconds),
            check_interval: Duration::from_secs(check_interval_seconds),
            max_attempts: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS",
                    file.max_attempts,
                    5_usize,
                )?,
                0,
                100,
            )?,
            max_files_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE",
                    file.max_files_per_cycle,
                    10_usize,
                )?,
                1,
                100,
            )?,
            max_files_per_peer_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                    file.max_files_per_peer_per_cycle,
                    1_usize,
                )?,
                1,
                20,
            )?,
            peer_cooldown: Duration::from_secs(peer_cooldown_seconds),
            alternate_sources_enabled: env_bool_layer(
                env,
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCES_ENABLED",
                file.alternate_sources_enabled.unwrap_or(true),
            )?,
            max_alternate_source_searches_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
                    file.max_alternate_source_searches_per_cycle,
                    1_usize,
                )?,
                0,
                10,
            )?,
            alternate_source_size_tolerance_percent: {
                let value = env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                    file.alternate_source_size_tolerance_percent,
                    5.0_f64,
                )?;
                // The frozen slskdN snapshot applies an integer-operand
                // RangeAttribute to this double.  Its conversion rounds
                // fractional boundary values, accepting -0.5 through 100.5.
                // Upstream correction: snapetech/slskdN#271.
                if !value.is_finite() || !(-0.5..=100.5).contains(&value) {
                    return Err(
                        "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT must be between 0 and 100"
                            .to_owned(),
                    );
                }
                value
            },
        })
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"retry_delay_seconds\":{},\"check_interval_seconds\":{},\"max_attempts\":{},\"max_files_per_cycle\":{},\"max_files_per_peer_per_cycle\":{},\"peer_cooldown_seconds\":{},\"alternate_sources_enabled\":{},\"max_alternate_source_searches_per_cycle\":{},\"alternate_source_size_tolerance_percent\":{}}}",
            self.enabled,
            self.retry_delay.as_secs(),
            self.check_interval.as_secs(),
            self.max_attempts,
            self.max_files_per_cycle,
            self.max_files_per_peer_per_cycle,
            self.peer_cooldown.as_secs(),
            self.alternate_sources_enabled,
            self.max_alternate_source_searches_per_cycle,
            self.alternate_source_size_tolerance_percent,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferRescueSettings {
    pub enabled: bool,
    pub max_queue_time: Duration,
    pub min_throughput_bytes_per_second: u64,
    pub min_duration: Duration,
    pub stalled_timeout: Duration,
    pub check_interval: Duration,
    pub retry_cooldown: Duration,
    pub max_files_per_cycle: usize,
    pub alternate_source_size_tolerance_percent: u32,
}

impl TransferRescueSettings {
    fn from_layers<E: ConfigEnv>(file: TransferRescueFileConfig, env: &E) -> Result<Self, String> {
        let max_queue_time_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS",
                file.max_queue_time_seconds,
                1_800_u64,
            )?,
            60,
            86_400,
        )?;
        let min_throughput_kbps = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS",
                file.min_throughput_kbps,
                10_u64,
            )?,
            1,
            10_000,
        )?;
        let min_duration_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS",
                file.min_duration_seconds,
                300_u64,
            )?,
            60,
            3_600,
        )?;
        let stalled_timeout_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS",
                file.stalled_timeout_seconds,
                120_u64,
            )?,
            30,
            600,
        )?;
        let check_interval_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_CHECK_INTERVAL_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_CHECK_INTERVAL_SECONDS",
                file.check_interval_seconds,
                45_u64,
            )?,
            15,
            300,
        )?;
        let retry_cooldown_seconds = bounded_config_value(
            "SLSKR_TRANSFER_RESCUE_RETRY_COOLDOWN_SECONDS",
            env_parse_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_RETRY_COOLDOWN_SECONDS",
                file.retry_cooldown_seconds,
                1_800_u64,
            )?,
            60,
            86_400,
        )?;
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_TRANSFER_RESCUE_ENABLED",
                file.enabled.unwrap_or(true),
            )?,
            max_queue_time: Duration::from_secs(max_queue_time_seconds),
            min_throughput_bytes_per_second: min_throughput_kbps.saturating_mul(1_024),
            min_duration: Duration::from_secs(min_duration_seconds),
            stalled_timeout: Duration::from_secs(stalled_timeout_seconds),
            check_interval: Duration::from_secs(check_interval_seconds),
            retry_cooldown: Duration::from_secs(retry_cooldown_seconds),
            max_files_per_cycle: bounded_config_value(
                "SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE",
                    file.max_files_per_cycle,
                    2_usize,
                )?,
                1,
                20,
            )?,
            alternate_source_size_tolerance_percent: bounded_config_value(
                "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                    file.alternate_source_size_tolerance_percent,
                    5_u32,
                )?,
                0,
                100,
            )?,
        })
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"max_queue_time_seconds\":{},\"min_throughput_kbps\":{},\"min_duration_seconds\":{},\"stalled_timeout_seconds\":{},\"check_interval_seconds\":{},\"retry_cooldown_seconds\":{},\"max_files_per_cycle\":{},\"alternate_source_size_tolerance_percent\":{}}}",
            self.enabled,
            self.max_queue_time.as_secs(),
            self.min_throughput_bytes_per_second / 1_024,
            self.min_duration.as_secs(),
            self.stalled_timeout.as_secs(),
            self.check_interval.as_secs(),
            self.retry_cooldown.as_secs(),
            self.max_files_per_cycle,
            self.alternate_source_size_tolerance_percent,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedBlacklistRange {
    first: u32,
    last: u32,
}

impl ManagedBlacklistRange {
    #[must_use]
    pub fn contains(self, address: Ipv4Addr) -> bool {
        let address = u32::from(address);
        (self.first..=self.last).contains(&address)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedBlacklistSettings {
    pub enabled: bool,
    pub file: Option<PathBuf>,
    pub ranges: Vec<ManagedBlacklistRange>,
    pub members: Vec<String>,
    pub patterns: Vec<String>,
    pub cidrs: Vec<TrustedProxyCidr>,
    pub cidr_values: Vec<String>,
}

impl ManagedBlacklistSettings {
    fn from_layers<E: ConfigEnv>(
        file: ManagedBlacklistFileConfig,
        users: &UserBlacklistFileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        let enabled = env_bool_layer(env, "SLSKD_BLACKLIST", file.enabled.unwrap_or(false))?;
        let path = env
            .var("SLSKD_BLACKLIST_FILE")
            .map(PathBuf::from)
            .or(file.file);
        let ranges = if enabled {
            let path = path.as_deref().ok_or_else(|| {
                "Blacklist.Enabled is true, but no Blacklist.File has been specified".to_owned()
            })?;
            load_managed_blacklist_file(path, target)?
        } else {
            Vec::new()
        };
        let members =
            controller_string_array_layer(env, "SLSKD_BLACKLISTED_MEMBERS", users.members.clone());
        let patterns = controller_string_array_layer(
            env,
            "SLSKD_BLACKLISTED_PATTERNS",
            users.patterns.clone(),
        );
        for pattern in &patterns {
            crate::dotnet_regex::DotNetRegex::validate(pattern).map_err(|_| match target {
                ControllerCompatibilityTarget::Slskd => {
                    format!("Pattern '{pattern}' is not a valid regular expression")
                }
                ControllerCompatibilityTarget::Slskdn => {
                    format!("Blacklist pattern {pattern} is invalid")
                }
            })?;
        }
        let cidr_values =
            controller_string_array_layer(env, "SLSKD_BLACKLISTED_CIDRS", users.cidrs.clone());
        let cidrs = cidr_values
            .iter()
            .map(|cidr| {
                if cidr.to_ascii_lowercase().starts_with("::ffff") {
                    return Err(format!("CIDR {cidr} is invalid"));
                }
                let normalized = if cidr.contains('/') {
                    cidr.clone()
                } else if cidr.contains(':') {
                    format!("{cidr}/128")
                } else {
                    format!("{cidr}/32")
                };
                TrustedProxyCidr::parse(&normalized).map_err(|error| match target {
                    ControllerCompatibilityTarget::Slskd => {
                        format!("CIDR {cidr} is invalid: {error}")
                    }
                    ControllerCompatibilityTarget::Slskdn => format!("CIDR {cidr} is invalid"),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            enabled,
            file: path,
            ranges,
            members,
            patterns,
            cidrs,
            cidr_values,
        })
    }

    #[must_use]
    pub fn contains(&self, address: IpAddr) -> bool {
        if self.cidrs.iter().any(|cidr| cidr.contains(address)) {
            return true;
        }
        if !self.enabled {
            return false;
        }
        let address = match address {
            IpAddr::V4(address) => address,
            IpAddr::V6(address) => match address.to_ipv4_mapped() {
                Some(address) => address,
                None => return false,
            },
        };
        self.ranges.iter().any(|range| range.contains(address))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManagedBlacklistFormat {
    Cidr,
    P2p,
    Dat,
}

pub fn validate_managed_blacklist_file_format(
    path: &std::path::Path,
    target: ControllerCompatibilityTarget,
) -> Result<(), String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("failed to read blacklist file {}: {error}", path.display()))?;
    detect_managed_blacklist_format(&body, target).map(|_| ())
}

fn load_managed_blacklist_file(
    path: &std::path::Path,
    target: ControllerCompatibilityTarget,
) -> Result<Vec<ManagedBlacklistRange>, String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("failed to read blacklist file {}: {error}", path.display()))?;
    let format = detect_managed_blacklist_format(&body, target)?;
    let mut ranges = Vec::new();
    for (index, line) in body.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let raw_range = managed_blacklist_line_range(line, format, target).map_err(|error| {
            format!(
                "failed to parse managed blacklist line {} {line:?}: {error}",
                index + 1
            )
        })?;
        ranges.push(parse_ipv4_range(&raw_range).map_err(|error| {
            format!(
                "failed to parse managed blacklist line {} {line:?}: {error}",
                index + 1
            )
        })?);
    }
    ranges.sort_unstable_by_key(|range| (range.first, range.last));
    ranges.dedup();
    Ok(ranges)
}

fn detect_managed_blacklist_format(
    body: &str,
    target: ControllerCompatibilityTarget,
) -> Result<ManagedBlacklistFormat, String> {
    for line in body.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        if parse_ipv4_range(line).is_ok() {
            return Ok(ManagedBlacklistFormat::Cidr);
        }
        if managed_blacklist_line_range(line, ManagedBlacklistFormat::P2p, target)
            .and_then(|range| parse_ipv4_range(&range))
            .is_ok()
        {
            return Ok(ManagedBlacklistFormat::P2p);
        }
        if managed_blacklist_line_range(line, ManagedBlacklistFormat::Dat, target)
            .and_then(|range| parse_ipv4_range(&range))
            .is_ok()
        {
            return Ok(ManagedBlacklistFormat::Dat);
        }
        break;
    }
    Err(
        "Failed to detect blacklist format. Only CIDR, P2P and DAT formats are supported"
            .to_owned(),
    )
}

fn managed_blacklist_line_range(
    line: &str,
    format: ManagedBlacklistFormat,
    target: ControllerCompatibilityTarget,
) -> Result<String, String> {
    match format {
        ManagedBlacklistFormat::Cidr => Ok(line.to_owned()),
        ManagedBlacklistFormat::P2p => {
            let range = match target {
                ControllerCompatibilityTarget::Slskd => line.split(':').nth(1),
                ControllerCompatibilityTarget::Slskdn => {
                    line.rsplit_once(':').map(|(_, range)| range)
                }
            }
            .map(str::trim)
            .filter(|range| !range.is_empty())
            .ok_or_else(|| "invalid P2P blacklist line".to_owned())?;
            Ok(range.to_owned())
        }
        ManagedBlacklistFormat::Dat => {
            let range = line
                .split(',')
                .next()
                .map(|range| range.replace(' ', ""))
                .filter(|range| !range.is_empty())
                .ok_or_else(|| "invalid DAT blacklist line".to_owned())?;
            range
                .split('-')
                .map(trim_ipv4_leading_zeroes)
                .collect::<Result<Vec<_>, _>>()
                .map(|addresses| addresses.join("-"))
        }
    }
}

fn trim_ipv4_leading_zeroes(address: &str) -> Result<String, String> {
    let octets = address
        .split('.')
        .map(|octet| {
            octet
                .parse::<u8>()
                .map(|octet| octet.to_string())
                .map_err(|_| "invalid IPv4 octet".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if octets.len() != 4 {
        return Err("invalid IPv4 address".to_owned());
    }
    Ok(octets.join("."))
}

fn parse_ipv4_range(value: &str) -> Result<ManagedBlacklistRange, String> {
    let value = value.trim();
    if let Some((address, prefix)) = value.split_once('/') {
        let address = address
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| "invalid IPv4 address".to_owned())?;
        let prefix = prefix
            .trim()
            .parse::<u8>()
            .map_err(|_| "invalid IPv4 prefix".to_owned())?;
        if prefix > 32 {
            return Err("invalid IPv4 prefix".to_owned());
        }
        let mask = if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        };
        let first = u32::from(address) & mask;
        return Ok(ManagedBlacklistRange {
            first,
            last: first | !mask,
        });
    }
    let (first, last) = value
        .split_once('-')
        .map_or((value, value), |(first, last)| (first.trim(), last.trim()));
    let first = u32::from(
        first
            .parse::<Ipv4Addr>()
            .map_err(|_| "invalid IPv4 range start".to_owned())?,
    );
    let last = u32::from(
        last.parse::<Ipv4Addr>()
            .map_err(|_| "invalid IPv4 range end".to_owned())?,
    );
    if first > last {
        return Err("IPv4 range start exceeds end".to_owned());
    }
    Ok(ManagedBlacklistRange { first, last })
}

fn bounded_config_value<T>(name: &str, value: T, min: T, max: T) -> Result<T, String>
where
    T: Copy + PartialOrd + std::fmt::Display,
{
    if value < min || value > max {
        return Err(format!("{name} must be between {min} and {max}"));
    }
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialStoreMode {
    Memory,
    Os,
    Systemd,
    File,
}

impl CredentialStoreMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "memory" | "runtime" | "none" => Ok(Self::Memory),
            "os" | "keyring" | "keychain" | "credential-manager" => Ok(Self::Os),
            "systemd" | "systemd-credentials" | "systemd-creds" => Ok(Self::Systemd),
            "file" | "local-file" => Ok(Self::File),
            other => Err(format!(
                "invalid SLSKR_CREDENTIAL_STORE {other:?}; expected memory, os, systemd, or file"
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Os => "os",
            Self::Systemd => "systemd",
            Self::File => "file",
        }
    }

    pub fn auto_connect_default(&self) -> bool {
        !matches!(self, Self::Memory)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedProxyCidr {
    network: IpAddr,
    prefix: u8,
}

impl TrustedProxyCidr {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (addr, prefix) = value
            .split_once('/')
            .ok_or_else(|| format!("trusted proxy CIDR {value:?} must include a prefix length"))?;
        let network = addr.parse::<IpAddr>().map_err(|error| {
            format!("trusted proxy CIDR {value:?} has invalid address: {error}")
        })?;
        let prefix = prefix
            .parse::<u8>()
            .map_err(|error| format!("trusted proxy CIDR {value:?} has invalid prefix: {error}"))?;
        let max_prefix = match network {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max_prefix {
            return Err(format!(
                "trusted proxy CIDR {value:?} prefix exceeds {max_prefix}"
            ));
        }
        Ok(Self { network, prefix })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(network), IpAddr::V4(ip)) => {
                let network = u32::from(network);
                let ip = u32::from(ip);
                self.prefix == 0 || network >> (32 - self.prefix) == ip >> (32 - self.prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(ip)) => {
                let network = u128::from_be_bytes(network.octets());
                let ip = u128::from_be_bytes(ip.octets());
                self.prefix == 0 || network >> (128 - self.prefix) == ip >> (128 - self.prefix)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct IntegrationSettings {
    pub spotify: Box<SpotifyIntegrationSettings>,
    pub lidarr: Box<LidarrIntegrationSettings>,
    pub youtube: SourceFeedApiKeySettings,
    pub lastfm: SourceFeedApiKeySettings,
    pub ntfy: NtfyIntegrationSettings,
    pub pushover: PushoverIntegrationSettings,
    pub pushbullet: PushbulletIntegrationSettings,
    pub ftp: FtpIntegrationSettings,
    pub vpn: VpnIntegrationSettings,
    pub scripts: BTreeMap<String, ScriptIntegrationSettings>,
    pub frozen_webhooks: BTreeMap<String, FrozenWebhookSettings>,
    pub bridge: BridgeIntegrationSettings,
    pub external_visualizer: ExternalVisualizerSettings,
}

impl IntegrationSettings {
    pub fn from_layers<E: ConfigEnv>(
        file_config: IntegrationsFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        Ok(Self {
            spotify: Box::new(SpotifyIntegrationSettings::from_layers(
                file_config.spotify,
                env,
            )?),
            lidarr: Box::new(LidarrIntegrationSettings::from_layers(
                file_config.lidarr,
                env,
            )?),
            youtube: SourceFeedApiKeySettings::from_layers(
                file_config.youtube,
                env,
                "SLSKD_YOUTUBE",
                "SLSKD_YOUTUBE_API_KEY",
                "YouTube source feed imports are enabled but integrations.youtube.api_key is empty.",
            )?,
            lastfm: SourceFeedApiKeySettings::from_layers(
                file_config.lastfm,
                env,
                "SLSKD_LASTFM",
                "SLSKD_LASTFM_API_KEY",
                "Last.fm source feed imports are enabled but integrations.lastfm.api_key is empty.",
            )?,
            ntfy: NtfyIntegrationSettings::from_layers(file_config.ntfy, env)?,
            pushover: PushoverIntegrationSettings::from_layers(file_config.pushover, env)?,
            pushbullet: PushbulletIntegrationSettings::from_layers(file_config.pushbullet, env)?,
            ftp: FtpIntegrationSettings::from_layers(file_config.ftp, env)?,
            vpn: VpnIntegrationSettings::from_layers(file_config.vpn, env)?,
            scripts: ScriptIntegrationSettings::from_layers(file_config.scripts, env)?,
            frozen_webhooks: FrozenWebhookSettings::from_layers(file_config.webhooks, env)?,
            bridge: BridgeIntegrationSettings::from_layers(file_config.bridge, env)?,
            external_visualizer: ExternalVisualizerSettings::from_layers(
                file_config.external_visualizer,
                env,
            )?,
        })
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"spotify\":{},\"lidarr\":{},\"youtube\":{},\"lastfm\":{},\"ntfy\":{},\"pushover\":{},\"pushbullet\":{},\"ftp\":{},\"vpn\":{},\"script_count\":{},\"webhook_count\":{},\"bridge\":{},\"external_visualizer\":{}}}",
            self.spotify.sanitized_json(),
            self.lidarr.sanitized_json(),
            self.youtube.sanitized_json(),
            self.lastfm.sanitized_json(),
            self.ntfy.sanitized_json(),
            self.pushover.sanitized_json(),
            self.pushbullet.sanitized_json(),
            self.ftp.sanitized_json(),
            self.vpn.sanitized_json(),
            self.scripts.len(),
            self.frozen_webhooks.len(),
            self.bridge.sanitized_json(),
            self.external_visualizer.sanitized_json()
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookSettings {
    pub on: Vec<String>,
    pub call: FrozenWebhookCallSettings,
    pub timeout: i32,
    pub retry: FrozenWebhookRetrySettings,
}
impl Default for FrozenWebhookSettings {
    fn default() -> Self {
        Self {
            on: Vec::new(),
            call: FrozenWebhookCallSettings::default(),
            timeout: 5_000,
            retry: FrozenWebhookRetrySettings::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookCallSettings {
    pub url: String,
    pub headers: Vec<FrozenWebhookHeaderSettings>,
    pub ignore_certificate_errors: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookHeaderSettings {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookRetrySettings {
    pub attempts: i32,
}
impl Default for FrozenWebhookRetrySettings {
    fn default() -> Self {
        Self { attempts: 1 }
    }
}

impl FrozenWebhookSettings {
    fn from_layers<E: ConfigEnv>(
        file: BTreeMap<String, Self>,
        env: &E,
    ) -> Result<BTreeMap<String, Self>, String> {
        let hooks = match env.var("SLSKR_FROZEN_WEBHOOKS_JSON") {
            Some(value) => serde_json::from_str::<BTreeMap<String, Self>>(&value)
                .map_err(|_| "invalid integrations.webhooks configuration".to_owned())?,
            None => file,
        };
        for (name, hook) in &hooks {
            if hook.call.url.trim().is_empty() {
                return Err(format!("Webhook {name} Url must not be empty"));
            }
            if !hook.call.url.starts_with("http://") && !hook.call.url.starts_with("https://") {
                return Err("The Url field must contain a fully qualified URL, including protocol (e.g. http:// or https://)".to_owned());
            }
            if hook.timeout < 500 {
                return Err("The field Timeout must be between 500 and 2147483647.".to_owned());
            }
            if hook.retry.attempts < 1 {
                return Err("The field Attempts must be between 1 and 2147483647.".to_owned());
            }
            if hook
                .call
                .headers
                .iter()
                .any(|header| header.name.trim().is_empty())
            {
                return Err("Webhook header Name must not be empty".to_owned());
            }
        }
        Ok(hooks)
    }
}

#[derive(Clone, Debug)]
pub struct NtfyIntegrationSettings {
    pub enabled: bool,
    pub url: String,
    pub access_token: String,
    pub notification_prefix: String,
    pub notify_on_private_message: bool,
    pub notify_on_room_mention: bool,
}

impl NtfyIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: NtfyFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_NTFY", file.enabled.unwrap_or(false))?,
            url: env.var("SLSKD_NTFY_URL").or(file.url).unwrap_or_default(),
            access_token: env
                .var("SLSKD_NTFY_TOKEN")
                .or(file.access_token)
                .unwrap_or_default(),
            notification_prefix: env
                .var("SLSKD_NTFY_NOTIFICATION_PREFIX")
                .or(file.notification_prefix)
                .unwrap_or_else(|| "slskdN".to_owned()),
            notify_on_private_message: env_bool_layer(
                env,
                "SLSKD_NTFY_NOTIFY_ON_PRIVATE_MESSAGE",
                file.notify_on_private_message.unwrap_or(true),
            )?,
            notify_on_room_mention: env_bool_layer(
                env,
                "SLSKD_NTFY_NOTIFY_ON_ROOM_MENTION",
                file.notify_on_room_mention.unwrap_or(true),
            )?,
        };
        if settings.enabled && settings.url.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no Url has been specified for Ntfy.".to_owned(),
            );
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!("{{\"enabled\":{},\"url\":{},\"access_token_configured\":{},\"notification_prefix\":{},\"notify_on_private_message\":{},\"notify_on_room_mention\":{}}}", self.enabled, json_escape(&self.url), !self.access_token.is_empty(), json_escape(&self.notification_prefix), self.notify_on_private_message, self.notify_on_room_mention)
    }
}

impl Default for NtfyIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            access_token: String::new(),
            notification_prefix: "slskdN".to_owned(),
            notify_on_private_message: true,
            notify_on_room_mention: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushoverIntegrationSettings {
    pub enabled: bool,
    pub user_key: String,
    pub token: String,
    pub notification_prefix: String,
    pub notify_on_private_message: bool,
    pub notify_on_room_mention: bool,
}

impl PushoverIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: PushoverFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_PUSHOVER", file.enabled.unwrap_or(false))?,
            user_key: env
                .var("SLSKD_PUSHOVER_USER_KEY")
                .or(file.user_key)
                .unwrap_or_default(),
            token: env
                .var("SLSKD_PUSHOVER_TOKEN")
                .or(file.token)
                .unwrap_or_default(),
            notification_prefix: env
                .var("SLSKD_PUSHOVER_NOTIFICATION_PREFIX")
                .or(file.notification_prefix)
                .unwrap_or_else(|| "slskdN".to_owned()),
            notify_on_private_message: env_bool_layer(
                env,
                "SLSKD_PUSHOVER_NOTIFY_ON_PRIVATE_MESSAGE",
                file.notify_on_private_message.unwrap_or(true),
            )?,
            notify_on_room_mention: env_bool_layer(
                env,
                "SLSKD_PUSHOVER_NOTIFY_ON_ROOM_MENTION",
                file.notify_on_room_mention.unwrap_or(true),
            )?,
        };
        if settings.enabled && settings.user_key.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no UserKey has been specified for Pushover."
                    .to_owned(),
            );
        }
        if settings.enabled && settings.token.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no Token has been specified for Pushover."
                    .to_owned(),
            );
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!("{{\"enabled\":{},\"user_key_configured\":{},\"token_configured\":{},\"notification_prefix\":{},\"notify_on_private_message\":{},\"notify_on_room_mention\":{}}}", self.enabled, !self.user_key.is_empty(), !self.token.is_empty(), json_escape(&self.notification_prefix), self.notify_on_private_message, self.notify_on_room_mention)
    }
}

impl Default for PushoverIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            user_key: String::new(),
            token: String::new(),
            notification_prefix: "slskdN".to_owned(),
            notify_on_private_message: true,
            notify_on_room_mention: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushbulletIntegrationSettings {
    pub enabled: bool,
    pub access_token: String,
    pub notification_prefix: String,
    pub notify_on_private_message: bool,
    pub notify_on_room_mention: bool,
    pub retry_attempts: u32,
    pub cooldown_time: i32,
}

impl PushbulletIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: PushbulletFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_PUSHBULLET", file.enabled.unwrap_or(false))?,
            access_token: env
                .var("SLSKD_PUSHBULLET_ACCESS_TOKEN")
                .or(file.access_token)
                .unwrap_or_default(),
            notification_prefix: env
                .var("SLSKD_PUSHBULLET_NOTIFICATION_PREFIX")
                .or(file.notification_prefix)
                .unwrap_or_else(|| "From slskdN:".to_owned()),
            notify_on_private_message: env_bool_layer(
                env,
                "SLSKD_PUSHBULLET_NOTIFY_ON_PRIVATE_MESSAGE",
                file.notify_on_private_message.unwrap_or(true),
            )?,
            notify_on_room_mention: env_bool_layer(
                env,
                "SLSKD_PUSHBULLET_NOTIFY_ON_ROOM_MENTION",
                file.notify_on_room_mention.unwrap_or(true),
            )?,
            retry_attempts: env_parse_layer(
                env,
                "SLSKD_PUSHBULLET_RETRY_ATTEMPTS",
                file.retry_attempts,
                3,
            )?,
            cooldown_time: env_parse_layer(
                env,
                "SLSKD_PUSHBULLET_COOLDOWN_TIME",
                file.cooldown_time,
                900_000,
            )?,
        };
        if settings.retry_attempts > 5 {
            return Err("The field RetryAttempts must be between 0 and 5.".to_owned());
        }
        if settings.enabled && settings.access_token.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no AccessToken has been specified.".to_owned(),
            );
        }
        Ok(settings)
    }
    fn sanitized_json(&self) -> String {
        format!("{{\"enabled\":{},\"access_token_configured\":{},\"notification_prefix\":{},\"notify_on_private_message\":{},\"notify_on_room_mention\":{},\"retry_attempts\":{},\"cooldown_time\":{}}}", self.enabled, !self.access_token.is_empty(), json_escape(&self.notification_prefix), self.notify_on_private_message, self.notify_on_room_mention, self.retry_attempts, self.cooldown_time)
    }
}

impl Default for PushbulletIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            access_token: String::new(),
            notification_prefix: "From slskdN:".to_owned(),
            notify_on_private_message: true,
            notify_on_room_mention: true,
            retry_attempts: 3,
            cooldown_time: 900_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FtpIntegrationSettings {
    pub enabled: bool,
    pub address: String,
    pub port: u16,
    pub encryption_mode: String,
    pub ignore_certificate_errors: bool,
    pub username: String,
    pub password: String,
    pub remote_path: String,
    pub overwrite_existing: bool,
    pub connection_timeout: u64,
    pub retry_attempts: u32,
}

impl FtpIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: FtpFileConfig, env: &E) -> Result<Self, String> {
        let port = env_parse_layer(env, "SLSKD_FTP_PORT", file.port, 21_u16)?;
        if port == 0 {
            return Err("The field Port must be between 1 and 65535.".to_owned());
        }
        let encryption_mode = env
            .var("SLSKD_FTP_ENCRYPTION_MODE")
            .or(file.encryption_mode)
            .unwrap_or_else(|| "auto".to_owned());
        if !matches!(
            encryption_mode.to_ascii_lowercase().as_str(),
            "none" | "implicit" | "explicit" | "auto"
        ) {
            return Err("The field EncryptionMode is invalid.".to_owned());
        }
        let retry_attempts =
            env_parse_layer(env, "SLSKD_FTP_RETRY_ATTEMPTS", file.retry_attempts, 3_u32)?;
        if retry_attempts > 5 {
            return Err("The field RetryAttempts must be between 0 and 5.".to_owned());
        }
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_FTP", file.enabled.unwrap_or(false))?,
            address: env
                .var("SLSKD_FTP_ADDRESS")
                .or(file.address)
                .unwrap_or_default(),
            port,
            encryption_mode,
            ignore_certificate_errors: env_bool_layer(
                env,
                "SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS",
                file.ignore_certificate_errors.unwrap_or(false),
            )?,
            username: env
                .var("SLSKD_FTP_USERNAME")
                .or(file.username)
                .unwrap_or_default(),
            password: env
                .var("SLSKD_FTP_PASSWORD")
                .or(file.password)
                .unwrap_or_default(),
            remote_path: env
                .var("SLSKD_FTP_REMOTE_PATH")
                .or(file.remote_path)
                .unwrap_or_else(|| "/".to_owned()),
            overwrite_existing: env_bool_layer(
                env,
                "SLSKD_FTP_OVERWRITE_EXISTING",
                file.overwrite_existing.unwrap_or(true),
            )?,
            connection_timeout: env_parse_layer(
                env,
                "SLSKD_FTP_CONNECTION_TIMEOUT",
                file.connection_timeout,
                5_000_u64,
            )?,
            retry_attempts,
        };
        if settings.connection_timeout > i32::MAX as u64 {
            return Err("The field ConnectionTimeout must be between 0 and 2147483647.".to_owned());
        }
        if settings.enabled && settings.address.trim().is_empty() {
            return Err("The Enabled field is true, but no Address has been specified.".to_owned());
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"address\":{},\"port\":{},\"encryption_mode\":{},\"ignore_certificate_errors\":{},\"username\":{},\"password_configured\":{},\"remote_path\":{},\"overwrite_existing\":{},\"connection_timeout\":{},\"retry_attempts\":{}}}",
            self.enabled,
            json_escape(&self.address),
            self.port,
            json_escape(&self.encryption_mode),
            self.ignore_certificate_errors,
            json_escape(&self.username),
            !self.password.is_empty(),
            json_escape(&self.remote_path),
            self.overwrite_existing,
            self.connection_timeout,
            self.retry_attempts,
        )
    }
}

impl Default for FtpIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            address: String::new(),
            port: 21,
            encryption_mode: "auto".to_owned(),
            ignore_certificate_errors: false,
            username: String::new(),
            password: String::new(),
            remote_path: "/".to_owned(),
            overwrite_existing: true,
            connection_timeout: 5_000,
            retry_attempts: 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VpnIntegrationSettings {
    pub enabled: bool,
    pub port_forwarding: bool,
    pub polling_interval: u64,
    pub gluetun: GluetunIntegrationSettings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GluetunIntegrationSettings {
    pub url: String,
    pub timeout: u64,
    pub auth: String,
    pub username: String,
    pub password: String,
    pub api_key: String,
}

impl VpnIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: VpnFileConfig, env: &E) -> Result<Self, String> {
        let polling_interval = env_parse_layer(
            env,
            "SLSKD_VPN_POLLING_INTERVAL",
            file.polling_interval,
            2_500_u64,
        )?;
        if !(500..=i32::MAX as u64).contains(&polling_interval) {
            return Err("The field PollingInterval must be between 500 and 2147483647.".to_owned());
        }
        let timeout = env_parse_layer(
            env,
            "SLSKD_VPN_GLUETUN_TIMEOUT",
            file.gluetun.timeout,
            1_000_u64,
        )?;
        if !(500..=10_000).contains(&timeout) {
            return Err("The field Timeout must be between 500 and 10000.".to_owned());
        }
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_VPN", file.enabled.unwrap_or(false))?,
            port_forwarding: env_bool_layer(
                env,
                "SLSKD_VPN_PORT_FORWARDING",
                file.port_forwarding.unwrap_or(false),
            )?,
            polling_interval,
            gluetun: GluetunIntegrationSettings {
                url: env
                    .var("SLSKD_VPN_GLUETUN_URL")
                    .or(file.gluetun.url)
                    .unwrap_or_default(),
                timeout,
                auth: file.gluetun.auth.unwrap_or_default(),
                username: env
                    .var("SLSKD_VPN_GLUETUN_USERNAME")
                    .or(file.gluetun.username)
                    .unwrap_or_default(),
                password: env
                    .var("SLSKD_VPN_GLUETUN_PASSWORD")
                    .or(file.gluetun.password)
                    .unwrap_or_default(),
                api_key: env
                    .var("SLSKD_VPN_GLUETUN_API_KEY")
                    .or(file.gluetun.api_key)
                    .unwrap_or_default(),
            },
        };
        if settings.enabled {
            if settings.gluetun.url.trim().is_empty() {
                return Err("VPN is enabled but no client is configured".to_owned());
            }
            let url = reqwest::Url::parse(&settings.gluetun.url).map_err(|_| {
                "The gluetun URL must be absolute, e.g. 'http://127.0.0.1:8000'".to_owned()
            })?;
            if url.cannot_be_a_base() {
                return Err(
                    "The gluetun URL must be absolute, e.g. 'http://127.0.0.1:8000'".to_owned(),
                );
            }
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"port_forwarding\":{},\"polling_interval\":{},\"gluetun\":{{\"url\":{},\"timeout\":{},\"auth\":{},\"username\":{},\"password_configured\":{},\"api_key_configured\":{}}}}}",
            self.enabled,
            self.port_forwarding,
            self.polling_interval,
            json_escape(&self.gluetun.url),
            self.gluetun.timeout,
            json_escape(&self.gluetun.auth),
            json_escape(&self.gluetun.username),
            !self.gluetun.password.is_empty(),
            !self.gluetun.api_key.is_empty(),
        )
    }
}

impl Default for VpnIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port_forwarding: false,
            polling_interval: 2_500,
            gluetun: GluetunIntegrationSettings {
                url: String::new(),
                timeout: 1_000,
                auth: String::new(),
                username: String::new(),
                password: String::new(),
                api_key: String::new(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ScriptIntegrationSettings {
    pub on: Vec<String>,
    pub run: ScriptRunSettings,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ScriptRunSettings {
    pub command: String,
    pub executable: String,
    pub args: String,
    #[serde(alias = "args_list")]
    pub arglist: Option<Vec<String>>,
}

impl ScriptIntegrationSettings {
    fn from_layers<E: ConfigEnv>(
        file: BTreeMap<String, Self>,
        env: &E,
    ) -> Result<BTreeMap<String, Self>, String> {
        let scripts = match env.var("SLSKR_FROZEN_SCRIPTS_JSON") {
            Some(value) => serde_json::from_str::<BTreeMap<String, Self>>(&value)
                .map_err(|_| "invalid integrations.scripts configuration".to_owned())?,
            None => file,
        };
        const EVENTS: &[&str] = &[
            "None",
            "Any",
            "DownloadFileComplete",
            "DownloadDirectoryComplete",
            "UploadFileComplete",
            "DownloadFileFailed",
            "PrivateMessageReceived",
            "RoomMessageReceived",
            "SearchResponsesReceived",
            "PeerSearchedUs",
            "PeerDownloadedFromUs",
            "SoulseekClientConnected",
            "SoulseekClientDisconnected",
            "Noop",
        ];
        for script in scripts.values() {
            if script
                .on
                .iter()
                .any(|event| !EVENTS.iter().any(|known| known.eq_ignore_ascii_case(event)))
            {
                return Err("The field On contains an invalid event type.".to_owned());
            }
            let command_set = !script.run.command.trim().is_empty();
            let executable_set = !script.run.executable.trim().is_empty();
            if command_set == executable_set {
                return Err("One and only one of the fields Command or Executable may be specified for a single script. If you intend to use the system shell, omit 'executable'. If you intend to use an executable other than the system shell, omit 'command' and specify either 'args' or 'args_list'.".to_owned());
            }
            if !script.run.args.trim().is_empty() && script.run.arglist.is_some() {
                return Err("Only one of the fields Args or Arglist may be specified for a single script. Specify 'args' if you intend to construct a single quoted string yourself, and specify 'args_list' if you'd like slskd to handle quoting for you.".to_owned());
            }
        }
        Ok(scripts)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceFeedApiKeySettings {
    pub enabled: bool,
    pub api_key: Option<String>,
}

impl SourceFeedApiKeySettings {
    fn from_layers<E: ConfigEnv>(
        file_config: SourceFeedApiKeyFileConfig,
        env: &E,
        enabled_name: &str,
        api_key_name: &str,
        missing_key_error: &str,
    ) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, enabled_name, file_config.enabled.unwrap_or(false))?,
            api_key: optional_env_any(env, &[api_key_name]).or(file_config.api_key),
        };
        if settings.enabled
            && settings
                .api_key
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(missing_key_error.to_owned());
        }
        Ok(settings)
    }

    pub fn configured(&self) -> bool {
        self.enabled
            && self
                .api_key
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"api_key_configured\":{}}}",
            self.enabled,
            self.api_key.is_some()
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpotifyIntegrationSettings {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub timeout_seconds: u64,
    pub max_items_per_import: u64,
    pub market: String,
    pub scopes: String,
}

impl SpotifyIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: SpotifyFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_any_layer(
                env,
                &["SLSKR_SPOTIFY_ENABLED", "SLSKD_SPOTIFY"],
                file_config.enabled.unwrap_or(false),
            )?,
            client_id: optional_env_any(
                env,
                &["SLSKR_SPOTIFY_CLIENT_ID", "SLSKD_SPOTIFY_CLIENT_ID"],
            )
            .or(file_config.client_id),
            client_secret: optional_env_any(
                env,
                &["SLSKR_SPOTIFY_CLIENT_SECRET", "SLSKD_SPOTIFY_CLIENT_SECRET"],
            )
            .or(file_config.client_secret),
            redirect_uri: optional_env_any(
                env,
                &["SLSKR_SPOTIFY_REDIRECT_URI", "SLSKD_SPOTIFY_REDIRECT_URI"],
            )
            .or(file_config.redirect_uri),
            timeout_seconds: env_parse_any_layer(
                env,
                &["SLSKR_SPOTIFY_TIMEOUT_SECONDS", "SLSKD_SPOTIFY_TIMEOUT"],
                file_config.timeout_seconds,
                20_u64,
            )?,
            max_items_per_import: env_parse_any_layer(
                env,
                &[
                    "SLSKR_SPOTIFY_MAX_ITEMS_PER_IMPORT",
                    "SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT",
                ],
                file_config.max_items_per_import,
                500_u64,
            )?,
            market: optional_env_any(env, &["SLSKR_SPOTIFY_MARKET", "SLSKD_SPOTIFY_MARKET"])
                .or(file_config.market)
                .unwrap_or_else(|| "US".to_owned()),
            scopes: env
                .var("SLSKR_SPOTIFY_SCOPES")
                .or(file_config.scopes)
                .unwrap_or_else(|| {
                    "user-library-read user-follow-read playlist-read-private playlist-read-collaborative"
                        .to_owned()
                }),
        };
        if !(1..=120).contains(&settings.timeout_seconds) {
            return Err("Spotify timeout must be between 1 and 120 seconds".to_owned());
        }
        if !(1..=5_000).contains(&settings.max_items_per_import) {
            return Err("Spotify maximum items per import must be between 1 and 5000".to_owned());
        }
        if settings.enabled
            && settings
                .client_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(
                "Spotify is enabled but integrations.spotify.client_id is empty.".to_owned(),
            );
        }
        if settings.enabled && settings.market.encode_utf16().count() != 2 {
            return Err("Spotify market must be a two-letter market code.".to_owned());
        }
        Ok(settings)
    }

    pub fn configured(&self) -> bool {
        self.enabled
            && self
                .client_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"client_id_configured\":{},\"client_secret_configured\":{},\"redirect_uri\":null,\"redirect_uri_configured\":{},\"timeout_seconds\":{},\"max_items_per_import\":{},\"market\":\"{}\",\"scopes\":\"{}\"}}",
            self.enabled,
            self.client_id.is_some(),
            self.client_secret.is_some(),
            self.redirect_uri.is_some(),
            self.timeout_seconds,
            self.max_items_per_import,
            json_escape(&self.market),
            json_escape(&self.scopes)
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct LidarrIntegrationSettings {
    pub enabled: bool,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub sync_wanted_to_wishlist: bool,
    pub sync_interval_seconds: u64,
    pub max_items_per_sync: u64,
    pub auto_download: bool,
    pub wishlist_filter: String,
    pub wishlist_max_results: u64,
    pub auto_import_completed: bool,
    pub import_path_from: String,
    pub import_path_to: String,
    pub import_mode: String,
    pub import_replace_existing_files: bool,
}

impl LidarrIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: LidarrFileConfig, env: &E) -> Result<Self, String> {
        let url =
            optional_env_any(env, &["SLSKR_LIDARR_URL", "SLSKD_LIDARR_URL"]).or(file_config.url);
        if let Some(url) = url.as_deref().filter(|value| !value.trim().is_empty()) {
            let parsed = reqwest::Url::parse(url)
                .map_err(|error| format!("Lidarr URL is invalid: {error}"))?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("Lidarr URL scheme must be http or https".to_owned());
            }
            if parsed.host_str().is_none() {
                return Err("Lidarr URL must include a host".to_owned());
            }
            if !parsed.username().is_empty() || parsed.password().is_some() {
                return Err("Lidarr URL must not contain embedded credentials".to_owned());
            }
            if parsed.query().is_some() || parsed.fragment().is_some() {
                return Err("Lidarr URL must not contain a query or fragment".to_owned());
            }
        }
        let settings = Self {
            enabled: env_bool_any_layer(
                env,
                &["SLSKR_LIDARR_ENABLED", "SLSKD_LIDARR"],
                file_config.enabled.unwrap_or(false),
            )?,
            url,
            api_key: optional_env_any(env, &["SLSKR_LIDARR_API_KEY", "SLSKD_LIDARR_API_KEY"])
                .or(file_config.api_key),
            timeout_seconds: env_parse_any_layer(
                env,
                &["SLSKR_LIDARR_TIMEOUT_SECONDS", "SLSKD_LIDARR_TIMEOUT"],
                file_config.timeout_seconds,
                20_u64,
            )?,
            sync_wanted_to_wishlist: env_bool_any_layer(
                env,
                &["SLSKR_LIDARR_SYNC_WANTED", "SLSKD_LIDARR_SYNC_WANTED"],
                file_config.sync_wanted_to_wishlist.unwrap_or(false),
            )?,
            sync_interval_seconds: env_parse_any_layer(
                env,
                &["SLSKR_LIDARR_SYNC_INTERVAL", "SLSKD_LIDARR_SYNC_INTERVAL"],
                file_config.sync_interval_seconds,
                3_600_u64,
            )?,
            max_items_per_sync: env_parse_any_layer(
                env,
                &["SLSKR_LIDARR_SYNC_MAX_ITEMS", "SLSKD_LIDARR_SYNC_MAX_ITEMS"],
                file_config.max_items_per_sync,
                100_u64,
            )?,
            auto_download: env_bool_any_layer(
                env,
                &["SLSKR_LIDARR_AUTO_DOWNLOAD", "SLSKD_LIDARR_AUTO_DOWNLOAD"],
                file_config.auto_download.unwrap_or(false),
            )?,
            wishlist_filter: optional_env_any(
                env,
                &[
                    "SLSKR_LIDARR_WISHLIST_FILTER",
                    "SLSKD_LIDARR_WISHLIST_FILTER",
                ],
            )
            .or(file_config.wishlist_filter)
            .unwrap_or_default(),
            wishlist_max_results: env_parse_any_layer(
                env,
                &[
                    "SLSKR_LIDARR_WISHLIST_MAX_RESULTS",
                    "SLSKD_LIDARR_WISHLIST_MAX_RESULTS",
                ],
                file_config.wishlist_max_results,
                100_u64,
            )?,
            auto_import_completed: env_bool_any_layer(
                env,
                &[
                    "SLSKR_LIDARR_AUTO_IMPORT_COMPLETED",
                    "SLSKD_LIDARR_AUTO_IMPORT_COMPLETED",
                ],
                file_config.auto_import_completed.unwrap_or(false),
            )?,
            import_path_from: optional_env_any(
                env,
                &[
                    "SLSKR_LIDARR_IMPORT_PATH_FROM",
                    "SLSKD_LIDARR_IMPORT_PATH_FROM",
                ],
            )
            .or(file_config.import_path_from)
            .unwrap_or_default(),
            import_path_to: optional_env_any(
                env,
                &["SLSKR_LIDARR_IMPORT_PATH_TO", "SLSKD_LIDARR_IMPORT_PATH_TO"],
            )
            .or(file_config.import_path_to)
            .unwrap_or_default(),
            import_mode: optional_env_any(
                env,
                &["SLSKR_LIDARR_IMPORT_MODE", "SLSKD_LIDARR_IMPORT_MODE"],
            )
            .or(file_config.import_mode)
            .unwrap_or_else(|| "move".to_owned()),
            import_replace_existing_files: env_bool_any_layer(
                env,
                &[
                    "SLSKR_LIDARR_IMPORT_REPLACE_EXISTING",
                    "SLSKD_LIDARR_IMPORT_REPLACE_EXISTING",
                ],
                file_config.import_replace_existing_files.unwrap_or(false),
            )?,
        };
        if !(1..=120).contains(&settings.timeout_seconds) {
            return Err("Lidarr timeout must be between 1 and 120 seconds".to_owned());
        }
        if settings.sync_interval_seconds < 300 {
            return Err("Lidarr sync interval must be at least 300 seconds".to_owned());
        }
        if !(1..=1_000).contains(&settings.max_items_per_sync) {
            return Err("Lidarr maximum items per sync must be between 1 and 1000".to_owned());
        }
        if !(10..=1_000).contains(&settings.wishlist_max_results) {
            return Err("Lidarr wishlist maximum results must be between 10 and 1000".to_owned());
        }
        if settings.enabled {
            if settings
                .url
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(
                    "Lidarr is enabled but integrations.lidarr.url is not an absolute URL."
                        .to_owned(),
                );
            }
            if settings
                .api_key
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(
                    "Lidarr is enabled but integrations.lidarr.api_key is empty.".to_owned(),
                );
            }
            let from_configured = !settings.import_path_from.trim().is_empty();
            let to_configured = !settings.import_path_to.trim().is_empty();
            if settings.auto_import_completed && from_configured != to_configured {
                return Err(
                    "Lidarr import path mapping requires both import_path_from and import_path_to."
                        .to_owned(),
                );
            }
            if !matches!(
                settings.import_mode.as_str(),
                "move" | "copy" | "Move" | "Copy"
            ) {
                return Err("Lidarr import_mode must be 'move' or 'copy'.".to_owned());
            }
        }
        Ok(settings)
    }

    pub fn configured(&self) -> bool {
        self.enabled
            && self
                .url
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && self
                .api_key
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"url\":null,\"url_configured\":{},\"api_key_configured\":{},\"timeout_seconds\":{},\"sync_wanted_to_wishlist\":{},\"sync_interval_seconds\":{},\"max_items_per_sync\":{},\"auto_download\":{},\"wishlist_filter\":\"{}\",\"wishlist_max_results\":{},\"auto_import_completed\":{},\"import_path_from\":\"{}\",\"import_path_to\":\"{}\",\"import_mode\":\"{}\",\"import_replace_existing_files\":{}}}",
            self.enabled,
            self.url.is_some(),
            self.api_key.is_some(),
            self.timeout_seconds,
            self.sync_wanted_to_wishlist,
            self.sync_interval_seconds,
            self.max_items_per_sync,
            self.auto_download,
            json_escape(&self.wishlist_filter),
            self.wishlist_max_results,
            self.auto_import_completed,
            json_escape(&self.import_path_from),
            json_escape(&self.import_path_to),
            json_escape(&self.import_mode),
            self.import_replace_existing_files,
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct BridgeIntegrationSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

impl BridgeIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: BridgeFileConfig, env: &E) -> Result<Self, String> {
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_BRIDGE_ENABLED",
                file_config.enabled.unwrap_or(false),
            )?,
            host: env
                .var("SLSKR_BRIDGE_HOST")
                .or(file_config.host)
                .unwrap_or_else(|| "localhost".to_owned()),
            port: env_parse_layer(env, "SLSKR_BRIDGE_PORT", file_config.port, 3000_u16)?,
        })
    }

    pub fn endpoint_configured(&self) -> bool {
        !self.host.trim().is_empty() && self.port != 0
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"host\":null,\"port\":null,\"endpoint_configured\":{}}}",
            self.enabled,
            self.endpoint_configured()
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExternalVisualizerSettings {
    pub command: Option<String>,
    pub launch_enabled: bool,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub name: String,
}

impl ExternalVisualizerSettings {
    fn from_layers<E: ConfigEnv>(
        file_config: ExternalVisualizerFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        Ok(Self {
            command: env
                .var("SLSKR_EXTERNAL_VISUALIZER_COMMAND")
                .or(file_config.command),
            launch_enabled: env_bool_layer(
                env,
                "SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED",
                file_config.launch_enabled.unwrap_or(false),
            )?,
            arguments: Vec::new(),
            working_directory: None,
            name: "MilkDrop3".to_owned(),
        })
    }

    pub fn configured(&self) -> bool {
        self.command
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"configured\":{},\"launch_enabled\":{},\"command\":null,\"argument_count\":{},\"working_directory_configured\":{},\"name\":{}}}",
            self.configured(),
            self.launch_enabled,
            self.arguments.len(),
            self.working_directory.is_some(),
            serde_json::to_string(&self.name).unwrap_or_else(|_| "\"MilkDrop3\"".to_owned()),
        )
    }
}

#[derive(Clone, Debug)]
pub struct PrivateMessageAutoResponseSettings {
    pub enabled: bool,
    pub message: String,
    pub cooldown_minutes: u64,
}

impl PrivateMessageAutoResponseSettings {
    fn from_layers<E: ConfigEnv>(
        file_config: PrivateMessageAutoResponseFileConfig,
        env: &E,
        default_message: &str,
    ) -> Result<Self, String> {
        let enabled = env_bool_any_layer(
            env,
            &[
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
                "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
            ],
            file_config.enabled.unwrap_or(false),
        )?;
        let message = optional_env_any(
            env,
            &[
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
                "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
            ],
        )
        .or(file_config.message)
        .unwrap_or_else(|| default_message.to_owned());
        if message.len() > MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES {
            return Err(format!(
                "private-message auto response exceeds {MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES} bytes"
            ));
        }
        let cooldown_minutes = env_parse_any_layer(
            env,
            &[
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
                "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
            ],
            file_config.cooldown_minutes,
            360_u64,
        )?;
        if !(1..=1_440).contains(&cooldown_minutes) {
            return Err(
                "private-message auto-response cooldown must be between 1 and 1440 minutes"
                    .to_owned(),
            );
        }
        Ok(Self {
            enabled,
            message,
            cooldown_minutes,
        })
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"message_configured\":true,\"cooldown_minutes\":{}}}",
            self.enabled, self.cooldown_minutes
        )
    }
}

#[derive(Clone, Debug)]
pub struct ShareSettings {
    pub fixture_entries: Vec<FileEntry>,
    pub directories: Vec<ShareDirectory>,
    pub roots: Vec<PathBuf>,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub max_files: usize,
    pub cache_tsv_enabled: bool,
    pub cache_storage_mode: String,
    pub cache_workers: usize,
    pub cache_retention: Option<Duration>,
    pub probe_media_attributes: bool,
    pub filters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareDirectory {
    pub raw: String,
    pub alias: String,
    pub local_path: PathBuf,
    pub is_excluded: bool,
}

impl ShareDirectory {
    fn parse(value: &str) -> Result<Self, String> {
        let raw = value.trim().trim_end_matches(['/', '\\']).to_owned();
        let (is_excluded, share) = raw
            .strip_prefix(['!', '-'])
            .map_or((false, raw.as_str()), |share| (true, share));
        let (alias, local_path) = if let Some(rest) = share.strip_prefix('[') {
            let Some(close) = rest.find(']') else {
                return Err(format!(
                    "Share '{raw}' contains a relative path; only absolute paths are supported."
                ));
            };
            (rest[..close].to_owned(), PathBuf::from(&rest[close + 1..]))
        } else {
            let path = PathBuf::from(share);
            let alias = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            (alias, path)
        };
        if alias.trim().is_empty() {
            return Err(format!(
                "Share '{raw}' is invalid; alias may not be null, empty or consist of only whitespace"
            ));
        }
        if alias.contains(['/', '\\']) {
            return Err(format!(
                "Share '{raw}' is invalid; aliases may not contain path separators '/' or '\\'"
            ));
        }
        if local_path.as_os_str().is_empty() {
            return Err(format!("Share {raw} does not specify a path"));
        }
        if !local_path.is_absolute() {
            return Err(format!(
                "Share {raw} contains a relative path; only absolute paths are supported."
            ));
        }
        if local_path
            .components()
            .any(|component| component == std::path::Component::ParentDir)
        {
            return Err(format!(
                "Share {raw} contains an unsafe path traversal segment."
            ));
        }
        Ok(Self {
            raw,
            alias,
            local_path,
            is_excluded,
        })
    }
}

impl ShareSettings {
    pub fn from_layers<E: ConfigEnv>(
        file_config: ShareFileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        let fixture = env
            .var("SLSKR_SHARE_FIXTURE")
            .or(file_config.fixture)
            .unwrap_or_default();
        let raw_directories = match optional_env_any(env, &["SLSKR_SHARE_DIRS", "SLSKD_SHARED_DIR"])
        {
            Some(value) => value,
            None => file_config.dirs.join(";"),
        };
        let directories = parse_share_directories(&raw_directories)?;
        let filters = controller_string_array_layer(env, "SLSKD_SHARE_FILTER", file_config.filters);
        for filter in &filters {
            crate::dotnet_regex::DotNetRegex::validate(filter).map_err(|_| {
                format!("Share filter '{filter}' is not a valid regular expression")
            })?;
        }
        let mut aliases = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        for directory in &directories {
            if !aliases.insert(directory.alias.clone()) {
                return Err(format!(
                    "Share alias '{}' collides with another configured share",
                    directory.alias
                ));
            }
            if !paths.insert(directory.local_path.clone()) {
                return Err(format!(
                    "Share path '{}' is configured more than once",
                    directory.local_path.display()
                ));
            }
        }
        let roots = directories
            .iter()
            .filter(|directory| !directory.is_excluded)
            .map(|directory| directory.local_path.clone())
            .collect();
        let storage_mode = env
            .var("SLSKD_SHARE_CACHE_STORAGE_MODE")
            .or(file_config.cache.storage_mode)
            .unwrap_or_else(|| "memory".to_owned())
            .to_ascii_lowercase();
        if !matches!(storage_mode.as_str(), "memory" | "disk") {
            return Err("shares.cache.storage_mode must be memory or disk".to_owned());
        }
        let processor_count = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1);
        let default_workers = if target == ControllerCompatibilityTarget::Slskdn {
            if processor_count <= 2 {
                1
            } else {
                (processor_count / 2).clamp(2, 4)
            }
        } else {
            processor_count
        };
        let cache_workers = env_parse_layer(
            env,
            "SLSKD_SHARE_CACHE_WORKERS",
            file_config.cache.workers,
            default_workers,
        )?;
        if !(1..=128).contains(&cache_workers) {
            return Err("shares.cache.workers must be between 1 and 128".to_owned());
        }
        let cache_retention_minutes = env_parse_option_layer(
            env,
            "SLSKD_SHARE_CACHE_RETENTION",
            file_config.cache.retention,
        )?;
        if cache_retention_minutes.is_some_and(|minutes| minutes < 60) {
            return Err("shares.cache.retention must be at least 60 minutes".to_owned());
        }
        let legacy_cache_enabled = env_bool_layer(
            env,
            "SLSKR_SHARE_CACHE_TSV_ENABLED",
            file_config.cache_tsv_enabled.unwrap_or(true),
        )?;
        Ok(Self {
            fixture_entries: parse_share_entries(&fixture)?,
            directories,
            roots,
            follow_symlinks: env_bool_layer(
                env,
                "SLSKR_SHARE_FOLLOW_SYMLINKS",
                file_config.follow_symlinks.unwrap_or(false),
            )?,
            include_hidden: env_bool_layer(
                env,
                "SLSKR_SHARE_INCLUDE_HIDDEN",
                file_config.include_hidden.unwrap_or(false),
            )?,
            max_files: env_parse_layer(
                env,
                "SLSKR_SHARE_SCAN_MAX_FILES",
                file_config.scan_max_files,
                50_000_usize,
            )?,
            cache_tsv_enabled: legacy_cache_enabled || storage_mode == "disk",
            cache_storage_mode: storage_mode,
            cache_workers,
            cache_retention: cache_retention_minutes
                .map(|minutes| Duration::from_secs(minutes.saturating_mul(60))),
            probe_media_attributes: env_bool_layer(
                env,
                "SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES",
                file_config.probe_media_attributes.unwrap_or(true),
            )?,
            filters,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    headless: Option<bool>,
    flags: ControllerFlagsFileConfig,
    logger: LoggerFileConfig,
    permissions: PermissionsFileConfig,
    telemetry: TelemetryFileConfig,
    retention: RetentionFileConfig,
    filters: FiltersFileConfig,
    app: AppFileConfig,
    blacklist: ManagedBlacklistFileConfig,
    feature: FeatureFileConfig,
    player: PlayerFileConfig,
    solid: SolidFileConfig,
    song_id: SongIdFileConfig,
    #[serde(rename = "virtualSoulfind", alias = "virtual_soulfind")]
    virtual_soulfind: VirtualSoulfindFileConfig,
    metrics: MetricsFileConfig,
    network: NetworkFileConfig,
    listeners: ListenerFileConfig,
    dht: DhtFileConfig,
    #[serde(rename = "Mesh")]
    mesh_sync: MeshSyncRootFileConfig,
    mesh: MeshFileConfig,
    overlay: OverlayFileConfig,
    overlay_data: OverlayDataFileConfig,
    relay: RelayFileConfig,
    security: SecurityFileConfig,
    profile: ProfileFileConfig,
    timeouts: TimeoutFileConfig,
    shares: ShareFileConfig,
    transfers: TransferFileConfig,
    groups: GroupsFileConfig,
    compatibility: CompatibilityFileConfig,
    auth: AuthFileConfig,
    web: WebFileConfig,
    persistence: PersistenceFileConfig,
    podcore: PodCoreFileConfig,
    virtual_soulfind_v2: VirtualSoulfindV2FileConfig,
    integrations: IntegrationsFileConfig,
    diagnostics: DiagnosticsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiagnosticsFileConfig {
    allow_memory_dump: Option<bool>,
    allow_remote_dump: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebFileConfig {
    socket: Option<PathBuf>,
    url_base: Option<String>,
    content_path: Option<PathBuf>,
    logging: Option<bool>,
    https: HttpsFileConfig,
    enforce_security: Option<bool>,
    allow_remote_no_auth: Option<bool>,
    passthrough_allowed_cidrs: Option<String>,
    max_request_body_size: Option<i64>,
    cors: WebCorsFileConfig,
    rate_limiting: WebRateLimitingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpsFileConfig {
    disabled: Option<bool>,
    port: Option<u16>,
    ip_address: Option<String>,
    force: Option<bool>,
    certificate: HttpsCertificateFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpsCertificateFileConfig {
    pfx: Option<PathBuf>,
    password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebCorsFileConfig {
    enabled: Option<bool>,
    allow_credentials: Option<bool>,
    allowed_origins: Vec<String>,
    allowed_headers: Vec<String>,
    allowed_methods: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebRateLimitingFileConfig {
    enabled: Option<bool>,
    api_permit_limit: Option<i32>,
    api_window_seconds: Option<i32>,
    federation_permit_limit: Option<i32>,
    federation_window_seconds: Option<i32>,
    mesh_gateway_permit_limit: Option<i32>,
    mesh_gateway_window_seconds: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControllerFlagsFileConfig {
    no_logo: Option<bool>,
    no_start: Option<bool>,
    no_version_check: Option<bool>,
    experimental: Option<bool>,
    hash_from_audio_file_enabled: Option<bool>,
    case_sensitive_reg_ex: Option<bool>,
    no_share_scan: Option<bool>,
    force_share_scan: Option<bool>,
    force_migrations: Option<bool>,
    legacy_windows_tcp_keepalive: Option<bool>,
    log_sql: Option<bool>,
    log_unobserved_exceptions: Option<bool>,
    optimistic_relay_file_info: Option<bool>,
    volatile: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerFileConfig {
    disk: Option<bool>,
    loki: Option<String>,
    no_color: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PermissionsFileConfig {
    file: FilePermissionsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilePermissionsFileConfig {
    mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryFileConfig {
    tracing: TelemetryTracingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryTracingFileConfig {
    enabled: Option<bool>,
    exporter: Option<String>,
    jaeger_endpoint: Option<String>,
    jaeger_port: Option<u16>,
    otlp_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetentionFileConfig {
    search: Option<u64>,
    logs: Option<u64>,
    transfers: TransferRetentionFileConfig,
    files: FileRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferRetentionFileConfig {
    upload: TransferTypeRetentionFileConfig,
    download: TransferTypeRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferTypeRetentionFileConfig {
    succeeded: Option<u64>,
    errored: Option<u64>,
    cancelled: Option<u64>,
    failed: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileRetentionFileConfig {
    complete: Option<u64>,
    incomplete: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FiltersFileConfig {
    search: SearchFiltersFileConfig,
    search_retention: SearchRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchFiltersFileConfig {
    request: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchRetentionFileConfig {
    max_age_days: Option<u64>,
    max_count: Option<usize>,
    cleanup_interval_seconds: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupsFileConfig {
    default: TransferGroupFileConfig,
    leechers: LeecherTransferGroupFileConfig,
    blacklisted: UserBlacklistFileConfig,
    user_defined: BTreeMap<String, UserDefinedTransferGroupFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferGroupFileConfig {
    upload: TransferGroupUploadFileConfig,
    limits: Option<TransferLimitsFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LeecherTransferGroupFileConfig {
    upload: TransferGroupUploadFileConfig,
    limits: Option<TransferLimitsFileConfig>,
    thresholds: LeecherThresholdFileConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserDefinedTransferGroupFileConfig {
    upload: TransferGroupUploadFileConfig,
    limits: Option<TransferLimitsFileConfig>,
    members: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferGroupUploadFileConfig {
    priority: Option<u32>,
    #[serde(alias = "queue_strategy")]
    strategy: Option<String>,
    slots: Option<u32>,
    speed_limit: Option<u32>,
    allowed_file_types: Vec<String>,
    limits: Option<TransferLimitsFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LeecherThresholdFileConfig {
    files: Option<u32>,
    directories: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferLimitsFileConfig {
    queued: NullableConfig<TransferLimitFileConfig>,
    daily: NullableConfig<TransferLimitFileConfig>,
    weekly: NullableConfig<TransferLimitFileConfig>,
}

impl Default for TransferLimitsFileConfig {
    fn default() -> Self {
        Self {
            queued: NullableConfig::Missing,
            daily: NullableConfig::Missing,
            weekly: NullableConfig::Missing,
        }
    }
}

#[derive(Clone, Debug)]
enum NullableConfig<T> {
    Missing,
    Null,
    Value(T),
}

impl<T> Default for NullableConfig<T> {
    fn default() -> Self {
        Self::Missing
    }
}

impl<'de, T> Deserialize<'de> for NullableConfig<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferLimitFileConfig {
    files: Option<u32>,
    megabytes: Option<u32>,
    failures: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserBlacklistFileConfig {
    members: Vec<String>,
    patterns: Vec<String>,
    cidrs: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct FeatureFileConfig {
    swagger: Option<bool>,
    #[serde(alias = "CollectionsSharing")]
    collections_sharing: Option<bool>,
    #[serde(alias = "Streaming")]
    streaming: Option<bool>,
    #[serde(alias = "StreamingRelayFallback")]
    streaming_relay_fallback: Option<bool>,
    #[serde(alias = "MeshParallelSearch")]
    mesh_parallel_search: Option<bool>,
    #[serde(alias = "MeshPublishAvailability")]
    mesh_publish_availability: Option<bool>,
    #[serde(alias = "IdentityFriends")]
    identity_friends: Option<bool>,
    #[serde(alias = "Solid")]
    solid: Option<bool>,
    #[serde(alias = "ScenePodBridge")]
    scene_pod_bridge: Option<bool>,
    #[serde(alias = "SongId")]
    song_id: Option<bool>,
    #[serde(alias = "Mesh")]
    mesh: Option<bool>,
    #[serde(alias = "Dht")]
    dht: Option<bool>,
    #[serde(alias = "Pods")]
    pods: Option<bool>,
    #[serde(alias = "SocialFederation")]
    social_federation: Option<bool>,
    #[serde(alias = "VirtualSoulfind")]
    virtual_soulfind: Option<bool>,
    #[serde(alias = "MultiSourceDownloads")]
    multi_source_downloads: Option<bool>,
    #[serde(alias = "ScenePodBridgeOptions")]
    scene_pod_bridge_options: ScenePodBridgeFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ScenePodBridgeFileConfig {
    #[serde(alias = "ProxyTransfers")]
    proxy_transfers: Option<bool>,
    #[serde(alias = "ExportPodAvailability")]
    export_pod_availability: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerFileConfig {
    external_visualizer: PlayerExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerExternalVisualizerFileConfig {
    enabled: Option<bool>,
    path: Option<String>,
    arguments: Option<Vec<String>>,
    working_directory: Option<PathBuf>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SolidFileConfig {
    #[serde(alias = "allowInsecureHttp")]
    allow_insecure_http: Option<bool>,
    #[serde(alias = "maxFetchBytes")]
    max_fetch_bytes: Option<usize>,
    #[serde(alias = "timeoutSeconds")]
    timeout_seconds: Option<u64>,
    #[serde(alias = "allowedHosts")]
    allowed_hosts: Option<Vec<String>>,
    #[serde(alias = "redirectPath")]
    redirect_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SongIdFileConfig {
    max_concurrent_runs: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindFileConfig {
    bridge: VirtualSoulfindBridgeFileConfig,
    disaster_mode: VirtualSoulfindDisasterModeFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindBridgeFileConfig {
    enabled: Option<bool>,
    port: Option<u16>,
    bind_address: Option<String>,
    max_clients: Option<usize>,
    require_auth: Option<bool>,
    password: Option<String>,
    max_requests_per_minute: Option<u32>,
    max_transfers_per_session: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindDisasterModeFileConfig {
    auto: Option<bool>,
    force: Option<bool>,
    unavailable_threshold_minutes: Option<u64>,
    enable_graceful_degradation: Option<bool>,
    recovery_check_interval_minutes: Option<u64>,
    recovery_healthy_checks_required: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsFileConfig {
    enabled: Option<bool>,
    url: Option<String>,
    authentication: MetricsAuthenticationFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsAuthenticationFileConfig {
    disabled: Option<bool>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManagedBlacklistFileConfig {
    enabled: Option<bool>,
    file: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CompatibilityFileConfig {
    controller_target: Option<String>,
    remote_configuration: Option<bool>,
    debug: Option<bool>,
    no_config_watch: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VirtualSoulfindV2FileConfig {
    enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DhtFileConfig {
    enabled: Option<bool>,
    #[serde(alias = "port")]
    dht_port: Option<u16>,
    overlay_port: Option<u16>,
    advertised_overlay_port: Option<u16>,
    vpn_port_sync: Option<String>,
    bootstrap_routers: Option<Vec<String>>,
    announce_interval_seconds: Option<u64>,
    discovery_interval_seconds: Option<u64>,
    min_neighbors: Option<usize>,
    bootstrap_timeout_seconds: Option<u64>,
    cold_bootstrap_timeout_seconds: Option<u64>,
    lan_only_bootstrap_timeout_seconds: Option<u64>,
    lan_only: Option<bool>,
    enable_upnp: Option<bool>,
    enable_stun: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshFileConfig {
    trusted_peers: Vec<TrustedMeshPeerInput>,
    enabled: Option<bool>,
    enable_soulseek_capability_handshake: Option<bool>,
    enable_soulseek_rendezvous: Option<bool>,
    probe_soulseek_rendezvous_capabilities: Option<bool>,
    dht: MeshDhtFileConfig,
    overlay: MeshPortsFileConfig,
    security: MeshSecurityFileConfig,
    sync_security: MeshSyncSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshDhtFileConfig {
    bootstrap_nodes: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshPortsFileConfig {
    udp_port: Option<u16>,
    quic_port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct MeshSecurityFileConfig {
    enforce_remote_payload_limits: Option<bool>,
    max_remote_payload_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshSyncRootFileConfig {
    sync_security: MeshSyncSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshSyncSecurityFileConfig {
    max_invalid_entries_per_window: Option<u32>,
    max_invalid_messages_per_window: Option<u32>,
    rate_limit_window_minutes: Option<u64>,
    quarantine_violation_threshold: Option<u32>,
    quarantine_duration_minutes: Option<u64>,
    proof_of_possession_enabled: Option<bool>,
    consensus_min_peers: Option<usize>,
    consensus_min_agreements: Option<usize>,
    alert_threshold_signature_failures: Option<u32>,
    alert_threshold_rate_limit_violations: Option<u32>,
    alert_threshold_quarantine_events: Option<u32>,
}

impl MeshSyncSecurityFileConfig {
    fn is_configured(&self) -> bool {
        self.max_invalid_entries_per_window.is_some()
            || self.max_invalid_messages_per_window.is_some()
            || self.rate_limit_window_minutes.is_some()
            || self.quarantine_violation_threshold.is_some()
            || self.quarantine_duration_minutes.is_some()
            || self.proof_of_possession_enabled.is_some()
            || self.consensus_min_peers.is_some()
            || self.consensus_min_agreements.is_some()
            || self.alert_threshold_signature_failures.is_some()
            || self.alert_threshold_rate_limit_violations.is_some()
            || self.alert_threshold_quarantine_events.is_some()
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayFileConfig {
    enable: Option<bool>,
    listen_port: Option<u16>,
    enable_quic: Option<bool>,
    quic_listen_port: Option<u16>,
    share_quic_with_dht_port: Option<bool>,
    quic_backend_listen_port: Option<u16>,
    trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayDataFileConfig {
    enable: Option<bool>,
    listen_port: Option<u16>,
    relay_authentication_token: Option<String>,
    allowed_relay_destinations: Vec<String>,
    max_concurrent_relays: Option<usize>,
    max_relay_bytes_per_direction: Option<u64>,
    max_relay_duration_seconds: Option<u64>,
    trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayFileConfig {
    enabled: Option<bool>,
    mode: Option<String>,
    controller: RelayControllerFileConfig,
    agents: BTreeMap<String, RelayAgentFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayControllerFileConfig {
    address: Option<String>,
    ignore_certificate_errors: Option<bool>,
    pinned_spki: Option<String>,
    api_key: Option<String>,
    secret: Option<String>,
    downloads: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayAgentFileConfig {
    pub instance_name: String,
    pub secret: String,
    pub cidr: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SecurityFileConfig {
    enabled: Option<bool>,
    profile: Option<String>,
    network_guard: NetworkGuardFileConfig,
    path_guard: PathGuardFileConfig,
    content_safety: ContentSafetyFileConfig,
    peer_reputation: PeerReputationFileConfig,
    violation_tracker: ViolationTrackerFileConfig,
    adversarial: AdversarialFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkGuardFileConfig {
    enabled: Option<bool>,
    max_connections_per_ip: Option<usize>,
    max_global_connections: Option<usize>,
    max_messages_per_minute: Option<u32>,
    max_message_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PathGuardFileConfig {
    enabled: Option<bool>,
    max_path_length: Option<usize>,
    max_path_depth: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentSafetyFileConfig {
    enabled: Option<bool>,
    verify_magic_bytes: Option<bool>,
    quarantine_suspicious: Option<bool>,
    quarantine_directory: Option<PathBuf>,
    block_executables: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PeerReputationFileConfig {
    enabled: Option<bool>,
    trusted_threshold: Option<u8>,
    untrusted_threshold: Option<u8>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViolationTrackerFileConfig {
    enabled: Option<bool>,
    violations_before_auto_ban: Option<u32>,
    base_ban_duration_minutes: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdversarialFileConfig {
    privacy: AdversarialPrivacyFileConfig,
    anonymity: AdversarialAnonymityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdversarialPrivacyFileConfig {
    padding: AdversarialPaddingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdversarialPaddingFileConfig {
    max_unpadded_bytes: Option<usize>,
    max_padded_bytes: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdversarialAnonymityFileConfig {
    relay_only: AdversarialRelayOnlyFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdversarialRelayOnlyFileConfig {
    relay_peer_data_endpoints: Vec<String>,
    relay_authentication_token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AdvancedNetworkingFileOverlay {
    dht: Option<DhtFileConfig>,
    #[serde(rename = "Mesh")]
    mesh_sync: Option<MeshSyncRootFileConfig>,
    mesh: Option<MeshFileConfig>,
    overlay: Option<OverlayFileConfig>,
    overlay_data: Option<OverlayDataFileConfig>,
    relay: Option<RelayFileConfig>,
    security: Option<SecurityFileConfig>,
    #[serde(rename = "PodCore", alias = "podcore")]
    podcore: Option<PodCoreFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct MediaAdvancedServiceFileOverlay {
    feature: Option<FeatureFileConfig>,
    player: Option<PlayerFileConfig>,
    solid: Option<SolidFileConfig>,
    song_id: Option<SongIdFileConfig>,
    #[serde(rename = "virtualSoulfind", alias = "virtual_soulfind")]
    virtual_soulfind: Option<VirtualSoulfindFileConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedMeshPeerInput {
    #[serde(alias = "peerId")]
    peer_id: String,
    username: String,
    #[serde(alias = "overlayEndpoint")]
    overlay_endpoint: String,
    #[serde(alias = "certificateSha256")]
    certificate_sha256: String,
    #[serde(default, alias = "rangeEndpoint")]
    range_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodCoreFileConfig {
    #[serde(alias = "Join")]
    join: PodJoinFileConfig,
    #[serde(alias = "Security")]
    security: PodSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodJoinFileConfig {
    #[serde(alias = "SignatureMode")]
    signature_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodSecurityFileConfig {
    #[serde(alias = "SignatureMode")]
    signature_mode: Option<String>,
}

impl AdvancedNetworkingSettings {
    fn from_layers<E: ConfigEnv>(
        file: &FileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
        _state_dir: &Path,
    ) -> Result<Self, String> {
        let slskdn = target == ControllerCompatibilityTarget::Slskdn;
        let yaml_overlay = env
            .var("SLSKR_ADVANCED_NETWORKING_JSON")
            .map(|value| {
                serde_json::from_str::<AdvancedNetworkingFileOverlay>(&value)
                    .map_err(|error| format!("invalid advanced networking YAML: {error}"))
            })
            .transpose()?
            .unwrap_or_default();
        let dht_file = yaml_overlay.dht.as_ref().unwrap_or(&file.dht);
        let mesh_file = yaml_overlay.mesh.as_ref().unwrap_or(&file.mesh);
        let mesh_sync_file = yaml_overlay.mesh_sync.as_ref().unwrap_or(&file.mesh_sync);
        let overlay_file = yaml_overlay.overlay.as_ref().unwrap_or(&file.overlay);
        let overlay_data_file = yaml_overlay
            .overlay_data
            .as_ref()
            .unwrap_or(&file.overlay_data);
        let relay_file = yaml_overlay.relay.as_ref().unwrap_or(&file.relay);
        let security_file = yaml_overlay.security.as_ref().unwrap_or(&file.security);
        let podcore_file = yaml_overlay.podcore.as_ref().unwrap_or(&file.podcore);
        let dht_enabled =
            env_bool_layer(env, "SLSKR_DHT_ENABLED", dht_file.enabled.unwrap_or(slskdn))?;
        let dht_port = env_parse_layer(
            env,
            "SLSKR_DHT_PORT",
            dht_file.dht_port,
            if slskdn { 50_305_u16 } else { 0_u16 },
        )?;
        if dht_enabled && dht_port == 0 {
            return Err("dht.dht_port must be between 1 and 65535 when DHT is enabled".to_owned());
        }
        let overlay_port = dht_file.overlay_port.unwrap_or(50_305);
        if slskdn && overlay_port == 0 {
            return Err("dht.overlay_port must be between 1 and 65535".to_owned());
        }
        let vpn_port_sync = dht_file
            .vpn_port_sync
            .clone()
            .unwrap_or_else(|| "disabled".to_owned())
            .trim()
            .to_ascii_lowercase();
        if !matches!(
            vpn_port_sync.as_str(),
            "disabled" | "primary" | "target_port"
        ) {
            return Err("dht.vpn_port_sync must be disabled, primary, or target_port".to_owned());
        }
        let positive_seconds = |path: &str, value: u64| -> Result<Duration, String> {
            if value == 0 {
                Err(format!("{path} must be greater than zero"))
            } else {
                Ok(Duration::from_secs(value))
            }
        };
        let dht = DhtSettings {
            enabled: dht_enabled,
            dht_port,
            overlay_port,
            advertised_overlay_port: dht_file.advertised_overlay_port.unwrap_or(0),
            vpn_port_sync,
            bootstrap_routers: {
                let mut routers = vec![
                    "router.bittorrent.com".to_owned(),
                    "router.utorrent.com".to_owned(),
                    "dht.transmissionbt.com".to_owned(),
                ];
                routers.extend(dht_file.bootstrap_routers.clone().unwrap_or_default());
                routers
            },
            announce_interval: positive_seconds(
                "dht.announce_interval_seconds",
                dht_file.announce_interval_seconds.unwrap_or(900),
            )?,
            discovery_interval: positive_seconds(
                "dht.discovery_interval_seconds",
                dht_file.discovery_interval_seconds.unwrap_or(600),
            )?,
            min_neighbors: dht_file.min_neighbors.unwrap_or(3),
            bootstrap_timeout: positive_seconds(
                "dht.bootstrap_timeout_seconds",
                dht_file.bootstrap_timeout_seconds.unwrap_or(120),
            )?,
            cold_bootstrap_timeout: positive_seconds(
                "dht.cold_bootstrap_timeout_seconds",
                dht_file.cold_bootstrap_timeout_seconds.unwrap_or(180),
            )?,
            lan_only_bootstrap_timeout: positive_seconds(
                "dht.lan_only_bootstrap_timeout_seconds",
                dht_file.lan_only_bootstrap_timeout_seconds.unwrap_or(30),
            )?,
            lan_only: dht_file.lan_only.unwrap_or(false),
            enable_upnp: dht_file.enable_upnp.unwrap_or(false),
            enable_stun: dht_file.enable_stun.unwrap_or(true),
        };
        if dht.min_neighbors == 0 {
            return Err("dht.min_neighbors must be greater than zero".to_owned());
        }
        if dht.bootstrap_routers.len() > 64
            || dht
                .bootstrap_routers
                .iter()
                .any(|router| router.trim().is_empty() || router.len() > 253)
        {
            return Err("dht.bootstrap_routers must contain at most 64 valid hostnames".to_owned());
        }

        let mesh = MeshRuntimeSettings {
            enabled: mesh_file.enabled.unwrap_or(slskdn),
            enable_soulseek_capability_handshake: mesh_file
                .enable_soulseek_capability_handshake
                .unwrap_or(true),
            enable_soulseek_rendezvous: mesh_file.enable_soulseek_rendezvous.unwrap_or(false),
            probe_soulseek_rendezvous_capabilities: mesh_file
                .probe_soulseek_rendezvous_capabilities
                .unwrap_or(true),
            dht_bootstrap_nodes: mesh_file.dht.bootstrap_nodes.unwrap_or(60),
            udp_port: mesh_file.overlay.udp_port.unwrap_or(50_301),
            quic_port: mesh_file.overlay.quic_port.unwrap_or(50_302),
            enforce_remote_payload_limits: mesh_file
                .security
                .enforce_remote_payload_limits
                .unwrap_or(true),
            max_remote_payload_size: mesh_file
                .security
                .max_remote_payload_size
                .unwrap_or(1024 * 1024),
        };
        if mesh.dht_bootstrap_nodes == 0 || mesh.dht_bootstrap_nodes > 10_000 {
            return Err("mesh.dht.bootstrap_nodes must be between 1 and 10000".to_owned());
        }
        if mesh.udp_port == 0 || mesh.quic_port == 0 {
            return Err("mesh overlay ports must be between 1 and 65535".to_owned());
        }
        if !(1024..=16 * 1024 * 1024).contains(&mesh.max_remote_payload_size) {
            return Err(
                "mesh.security.maxRemotePayloadSize must be between 1024 and 16777216".to_owned(),
            );
        }

        // .NET configuration keys are case-insensitive, so the documented
        // `Mesh:SyncSecurity` section and the lower-case YAML `mesh` section
        // are one logical tree. Accept the split spelling as well for older
        // slskdN examples that emitted both roots.
        let sync = if mesh_file.sync_security.is_configured() {
            &mesh_file.sync_security
        } else {
            &mesh_sync_file.sync_security
        };
        let mesh_sync_security = MeshSyncSecuritySettings {
            max_invalid_entries_per_window: sync.max_invalid_entries_per_window.unwrap_or(50),
            max_invalid_messages_per_window: sync.max_invalid_messages_per_window.unwrap_or(10),
            rate_limit_window: positive_seconds(
                "Mesh.sync_security.rate_limit_window_minutes",
                sync.rate_limit_window_minutes
                    .unwrap_or(5)
                    .saturating_mul(60),
            )?,
            quarantine_violation_threshold: sync.quarantine_violation_threshold.unwrap_or(3),
            quarantine_duration: positive_seconds(
                "Mesh.sync_security.quarantine_duration_minutes",
                sync.quarantine_duration_minutes
                    .unwrap_or(30)
                    .saturating_mul(60),
            )?,
            proof_of_possession_enabled: sync.proof_of_possession_enabled.unwrap_or(false),
            consensus_min_peers: sync.consensus_min_peers.unwrap_or(5),
            consensus_min_agreements: sync.consensus_min_agreements.unwrap_or(3),
            alert_threshold_signature_failures: sync
                .alert_threshold_signature_failures
                .unwrap_or(50),
            alert_threshold_rate_limit_violations: sync
                .alert_threshold_rate_limit_violations
                .unwrap_or(20),
            alert_threshold_quarantine_events: sync.alert_threshold_quarantine_events.unwrap_or(10),
        };
        if mesh_sync_security.max_invalid_entries_per_window == 0
            || mesh_sync_security.max_invalid_messages_per_window == 0
            || mesh_sync_security.quarantine_violation_threshold == 0
            || mesh_sync_security.consensus_min_peers == 0
            || mesh_sync_security.consensus_min_agreements == 0
            || mesh_sync_security.consensus_min_agreements > mesh_sync_security.consensus_min_peers
        {
            return Err("Mesh.sync_security limits and consensus values are invalid".to_owned());
        }

        validate_certificate_pins(
            "overlay.trusted_certificate_pins",
            &overlay_file.trusted_certificate_pins,
        )?;
        let overlay = OverlaySettings {
            enable: overlay_file.enable.unwrap_or(true),
            listen_port: overlay_file.listen_port.unwrap_or(50_305),
            enable_quic: overlay_file.enable_quic.unwrap_or(true),
            quic_listen_port: overlay_file.quic_listen_port.unwrap_or(50_305),
            share_quic_with_dht_port: overlay_file.share_quic_with_dht_port.unwrap_or(true),
            quic_backend_listen_port: overlay_file.quic_backend_listen_port.unwrap_or(55_305),
            trusted_certificate_pins: overlay_file.trusted_certificate_pins.clone(),
        };
        if overlay.enable
            && (overlay.listen_port == 0
                || (overlay.enable_quic
                    && (overlay.quic_listen_port == 0 || overlay.quic_backend_listen_port == 0)))
        {
            return Err("overlay listener ports must be between 1 and 65535".to_owned());
        }

        validate_certificate_pins(
            "overlay_data.trusted_certificate_pins",
            &overlay_data_file.trusted_certificate_pins,
        )?;
        let overlay_data = OverlayDataSettings {
            enable: overlay_data_file.enable.unwrap_or(false),
            listen_port: overlay_data_file.listen_port.unwrap_or(50_401),
            relay_authentication_token: overlay_data_file
                .relay_authentication_token
                .clone()
                .unwrap_or_default(),
            allowed_relay_destinations: overlay_data_file.allowed_relay_destinations.clone(),
            max_concurrent_relays: overlay_data_file.max_concurrent_relays.unwrap_or(4),
            max_relay_bytes_per_direction: overlay_data_file
                .max_relay_bytes_per_direction
                .unwrap_or(64 * 1024 * 1024),
            max_relay_duration: positive_seconds(
                "overlay_data.max_relay_duration_seconds",
                overlay_data_file.max_relay_duration_seconds.unwrap_or(300),
            )?,
            trusted_certificate_pins: overlay_data_file.trusted_certificate_pins.clone(),
        };
        if overlay_data.listen_port == 0
            || overlay_data.max_concurrent_relays == 0
            || overlay_data.max_relay_bytes_per_direction == 0
            || overlay_data.allowed_relay_destinations.len() > 256
            || overlay_data
                .allowed_relay_destinations
                .iter()
                .any(|endpoint| endpoint.parse::<SocketAddr>().is_err())
        {
            return Err("overlay_data relay limits or destinations are invalid".to_owned());
        }

        let relay_mode = env
            .var("SLSKD_RELAY_MODE")
            .or_else(|| env.var("RELAY_MODE"))
            .or_else(|| relay_file.mode.clone())
            .unwrap_or_else(|| "controller".to_owned())
            .trim()
            .to_ascii_lowercase();
        if !matches!(relay_mode.as_str(), "controller" | "agent" | "debug") {
            return Err("relay.mode must be controller, agent, or debug".to_owned());
        }
        let controller = RelayControllerSettings {
            address: env
                .var("SLSKD_CONTROLLER_ADDRESS")
                .or_else(|| env.var("CONTROLLER_ADDRESS"))
                .or_else(|| relay_file.controller.address.clone())
                .unwrap_or_default(),
            ignore_certificate_errors: env_bool_any_layer(
                env,
                &[
                    "SLSKD_CONTROLLER_IGNORE_CERTIFICATE_ERRORS",
                    "CONTROLLER_IGNORE_CERTIFICATE_ERRORS",
                ],
                relay_file
                    .controller
                    .ignore_certificate_errors
                    .unwrap_or(false),
            )?,
            pinned_spki: env
                .var("SLSKD_CONTROLLER_PINNED_SPKI")
                .or_else(|| env.var("CONTROLLER_PINNED_SPKI"))
                .or_else(|| relay_file.controller.pinned_spki.clone())
                .unwrap_or_default(),
            api_key: env
                .var("SLSKD_CONTROLLER_API_KEY")
                .or_else(|| env.var("CONTROLLER_API_KEY"))
                .or_else(|| relay_file.controller.api_key.clone())
                .unwrap_or_default(),
            secret: env
                .var("SLSKD_CONTROLLER_SECRET")
                .or_else(|| env.var("CONTROLLER_SECRET"))
                .or_else(|| relay_file.controller.secret.clone())
                .unwrap_or_default(),
            downloads: env_bool_any_layer(
                env,
                &["SLSKD_CONTROLLER_DOWNLOADS", "CONTROLLER_DOWNLOADS"],
                relay_file.controller.downloads.unwrap_or(false),
            )?,
        };
        let relay_enabled = env_bool_any_layer(
            env,
            &["SLSKD_RELAY", "RELAY"],
            relay_file.enabled.unwrap_or(false),
        )?;
        if relay_enabled && relay_mode == "agent" {
            if !(controller.address.starts_with("http://")
                || controller.address.starts_with("https://"))
                || !(16..=255).contains(&controller.api_key.len())
                || !(16..=255).contains(&controller.secret.len())
            {
                return Err("relay agent mode requires a controller URL and 16-255 character API key and secret".to_owned());
            }
        }
        let mut agents = BTreeMap::new();
        for (name, agent) in &relay_file.agents {
            let cidr = if agent.cidr.trim().is_empty() {
                "0.0.0.0/0,::/0".to_owned()
            } else {
                agent.cidr.clone()
            };
            if agent.instance_name.trim().is_empty()
                || !(16..=255).contains(&agent.secret.len())
                || cidr
                    .split(',')
                    .any(|value| TrustedProxyCidr::parse(value.trim()).is_err())
            {
                return Err(format!("relay.agents.{name} is invalid"));
            }
            agents.insert(
                name.clone(),
                RelayAgentSettings {
                    instance_name: agent.instance_name.clone(),
                    secret: agent.secret.clone(),
                    cidr,
                },
            );
        }
        let relay = RelaySettings {
            enabled: relay_enabled,
            mode: relay_mode,
            controller,
            agents,
        };

        let network = &security_file.network_guard;
        let path = &security_file.path_guard;
        let content = &security_file.content_safety;
        let reputation = &security_file.peer_reputation;
        let violations = &security_file.violation_tracker;
        let profile = security_file
            .profile
            .clone()
            .unwrap_or_else(|| "Standard".to_owned());
        if !matches!(
            profile.to_ascii_lowercase().as_str(),
            "minimal" | "standard" | "maximum" | "custom"
        ) {
            return Err(
                "security.profile must be Minimal, Standard, Maximum, or Custom".to_owned(),
            );
        }
        let network_guard = NetworkGuardSettings {
            enabled: network.enabled.unwrap_or(true),
            max_connections_per_ip: network.max_connections_per_ip.unwrap_or(100),
            max_global_connections: network.max_global_connections.unwrap_or(100),
            max_messages_per_minute: network.max_messages_per_minute.unwrap_or(60),
            max_message_size: network.max_message_size.unwrap_or(65_536),
        };
        if !(1..=1000).contains(&network_guard.max_connections_per_ip)
            || !(1..=10_000).contains(&network_guard.max_global_connections)
            || !(1..=1000).contains(&network_guard.max_messages_per_minute)
            || !(1024..=10 * 1024 * 1024).contains(&network_guard.max_message_size)
        {
            return Err("security.network_guard limits are invalid".to_owned());
        }
        let path_guard = PathGuardSettings {
            enabled: path.enabled.unwrap_or(true),
            max_path_length: path.max_path_length.unwrap_or(512),
            max_path_depth: path.max_path_depth.unwrap_or(20),
        };
        if !(1..=4096).contains(&path_guard.max_path_length)
            || !(1..=100).contains(&path_guard.max_path_depth)
        {
            return Err("security.path_guard limits are invalid".to_owned());
        }
        let peer_reputation = PeerReputationSettings {
            enabled: reputation.enabled.unwrap_or(true),
            trusted_threshold: reputation.trusted_threshold.unwrap_or(70),
            untrusted_threshold: reputation.untrusted_threshold.unwrap_or(20),
        };
        if peer_reputation.trusted_threshold > 100
            || peer_reputation.untrusted_threshold > 100
            || peer_reputation.untrusted_threshold > peer_reputation.trusted_threshold
        {
            return Err("security.peer_reputation thresholds are invalid".to_owned());
        }
        let padding = &security_file.adversarial.privacy.padding;
        let anonymity = &security_file.adversarial.anonymity.relay_only;
        let max_unpadded_bytes = padding.max_unpadded_bytes.unwrap_or(0);
        let max_padded_bytes = padding.max_padded_bytes.unwrap_or(0);
        if max_padded_bytes != 0 && max_unpadded_bytes != 0 && max_padded_bytes < max_unpadded_bytes
        {
            return Err("security.adversarial privacy padding limits are invalid".to_owned());
        }
        if anonymity.relay_peer_data_endpoints.len() > 64
            || anonymity
                .relay_peer_data_endpoints
                .iter()
                .any(|endpoint| endpoint.parse::<SocketAddr>().is_err())
        {
            return Err("security.adversarial relay endpoints are invalid".to_owned());
        }
        let security = SecuritySettings {
            enabled: security_file.enabled.unwrap_or(true),
            profile,
            network_guard,
            path_guard,
            content_safety: ContentSafetySettings {
                enabled: content.enabled.unwrap_or(true),
                verify_magic_bytes: content.verify_magic_bytes.unwrap_or(true),
                quarantine_suspicious: content.quarantine_suspicious.unwrap_or(true),
                quarantine_directory: content.quarantine_directory.clone().unwrap_or_default(),
                block_executables: content.block_executables.unwrap_or(true),
            },
            peer_reputation,
            violation_tracker: ViolationTrackerSettings {
                enabled: violations.enabled.unwrap_or(true),
                violations_before_auto_ban: violations.violations_before_auto_ban.unwrap_or(5),
                base_ban_duration: positive_seconds(
                    "security.violation_tracker.base_ban_duration_minutes",
                    violations
                        .base_ban_duration_minutes
                        .unwrap_or(60)
                        .saturating_mul(60),
                )?,
            },
            adversarial: AdversarialSettings {
                max_unpadded_bytes,
                max_padded_bytes,
                relay_peer_data_endpoints: anonymity.relay_peer_data_endpoints.clone(),
                relay_authentication_token: anonymity
                    .relay_authentication_token
                    .clone()
                    .unwrap_or_default(),
            },
        };
        if security.violation_tracker.violations_before_auto_ban == 0
            || security.violation_tracker.violations_before_auto_ban > 100
        {
            return Err(
                "security.violation_tracker.violations_before_auto_ban must be between 1 and 100"
                    .to_owned(),
            );
        }

        Ok(Self {
            dht,
            mesh,
            mesh_sync_security,
            pod_join_signature_mode: PodSignatureMode::parse(
                env.var("SLSKR_POD_JOIN_SIGNATURE_MODE")
                    .or_else(|| podcore_file.join.signature_mode.clone())
                    .as_deref()
                    .unwrap_or("off"),
            )?,
            pod_security_signature_mode: PodSignatureMode::parse(
                podcore_file
                    .security
                    .signature_mode
                    .as_deref()
                    .unwrap_or("off"),
            )?,
            overlay,
            overlay_data,
            relay,
            security,
        })
    }
}

impl MediaAdvancedServiceSettings {
    fn from_layers<E: ConfigEnv>(
        file: &FileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        let yaml_overlay = env
            .var("SLSKR_ADVANCED_NETWORKING_JSON")
            .map(|value| {
                serde_json::from_str::<MediaAdvancedServiceFileOverlay>(&value)
                    .map_err(|error| format!("invalid media/advanced-service YAML: {error}"))
            })
            .transpose()?
            .unwrap_or_default();
        let feature = yaml_overlay.feature.as_ref().unwrap_or(&file.feature);
        let player = yaml_overlay.player.as_ref().unwrap_or(&file.player);
        let solid_file = yaml_overlay.solid.as_ref().unwrap_or(&file.solid);
        let song_id = yaml_overlay.song_id.as_ref().unwrap_or(&file.song_id);
        let virtual_soulfind = yaml_overlay
            .virtual_soulfind
            .as_ref()
            .unwrap_or(&file.virtual_soulfind);
        let enabled_by_default = target == ControllerCompatibilityTarget::Slskdn;
        let features = FeatureGateSettings {
            collections_sharing: feature.collections_sharing.unwrap_or(enabled_by_default),
            streaming: feature.streaming.unwrap_or(enabled_by_default),
            streaming_relay_fallback: feature
                .streaming_relay_fallback
                .unwrap_or(enabled_by_default),
            mesh_parallel_search: feature.mesh_parallel_search.unwrap_or(enabled_by_default),
            mesh_publish_availability: feature
                .mesh_publish_availability
                .unwrap_or(enabled_by_default),
            identity_friends: feature.identity_friends.unwrap_or(enabled_by_default),
            solid: feature.solid.unwrap_or(enabled_by_default),
            scene_pod_bridge: feature.scene_pod_bridge.unwrap_or(false),
            scene_pod_bridge_proxy_transfers: feature
                .scene_pod_bridge_options
                .proxy_transfers
                .unwrap_or(false),
            scene_pod_bridge_export_pod_availability: feature
                .scene_pod_bridge_options
                .export_pod_availability
                .unwrap_or(false),
            song_id: feature.song_id.unwrap_or(enabled_by_default),
            mesh: feature.mesh.unwrap_or(enabled_by_default),
            dht: feature.dht.unwrap_or(enabled_by_default),
            pods: feature.pods.unwrap_or(enabled_by_default),
            social_federation: feature.social_federation.unwrap_or(enabled_by_default),
            virtual_soulfind: feature.virtual_soulfind.unwrap_or(enabled_by_default),
            multi_source_downloads: feature.multi_source_downloads.unwrap_or(enabled_by_default),
        };

        let visualizer_file = &player.external_visualizer;
        let legacy_visualizer = &file.integrations.external_visualizer;
        let external_visualizer = ExternalVisualizerSettings {
            command: env
                .var("SLSKR_EXTERNAL_VISUALIZER_COMMAND")
                .or_else(|| visualizer_file.path.clone())
                .or_else(|| legacy_visualizer.command.clone()),
            launch_enabled: env_bool_layer(
                env,
                "SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED",
                visualizer_file
                    .enabled
                    .or(legacy_visualizer.launch_enabled)
                    .unwrap_or(false),
            )?,
            arguments: visualizer_file.arguments.clone().unwrap_or_default(),
            working_directory: visualizer_file.working_directory.clone(),
            name: visualizer_file
                .name
                .clone()
                .unwrap_or_else(|| "MilkDrop3".to_owned()),
        };
        let allowed_hosts = solid_file
            .allowed_hosts
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|host| host.trim().trim_end_matches('.').to_ascii_lowercase())
            .collect::<Vec<_>>();
        if allowed_hosts.len() > 256
            || allowed_hosts.iter().any(|host| {
                host.is_empty()
                    || host.len() > 253
                    || host.contains(['/', '\\', '@'])
                    || reqwest::Url::parse(&format!("https://{host}/"))
                        .ok()
                        .and_then(|url| url.host_str().map(str::to_owned))
                        .is_none()
            })
        {
            return Err("solid.allowedHosts contains an invalid hostname".to_owned());
        }
        let solid = SolidSettings {
            allow_insecure_http: solid_file.allow_insecure_http.unwrap_or(false),
            max_fetch_bytes: solid_file.max_fetch_bytes.unwrap_or(1_000_000),
            timeout: Duration::from_secs(solid_file.timeout_seconds.unwrap_or(10)),
            allowed_hosts,
            redirect_path: solid_file
                .redirect_path
                .clone()
                .unwrap_or_else(|| "/solid/callback".to_owned()),
        };
        if !(1..=100 * 1024 * 1024).contains(&solid.max_fetch_bytes)
            || solid.timeout.is_zero()
            || solid.timeout > Duration::from_secs(300)
            || !solid.redirect_path.starts_with('/')
            || solid.redirect_path.starts_with("//")
        {
            return Err("solid fetch limits or redirectPath are invalid".to_owned());
        }

        let song_id_max_concurrent_runs = env_parse_layer(
            env,
            "SLSKD_SONGID_MAX_CONCURRENT_RUNS",
            song_id.max_concurrent_runs,
            2_usize,
        )?;
        if !(1..=1024).contains(&song_id_max_concurrent_runs) {
            return Err("song_id.max_concurrent_runs must be between 1 and 1024".to_owned());
        }

        let bridge_file = &virtual_soulfind.bridge;
        let bridge = VirtualSoulfindBridgeSettings {
            enabled: bridge_file.enabled.unwrap_or(false),
            port: bridge_file.port.unwrap_or(2242),
            bind_address: bridge_file
                .bind_address
                .as_deref()
                .unwrap_or("127.0.0.1")
                .parse::<IpAddr>()
                .map_err(|_| "virtualSoulfind.bridge.bindAddress is invalid".to_owned())?,
            max_clients: bridge_file.max_clients.unwrap_or(10),
            require_auth: bridge_file.require_auth.unwrap_or(true),
            password: bridge_file.password.clone().unwrap_or_default(),
            max_requests_per_minute: bridge_file.max_requests_per_minute.unwrap_or(60),
            max_transfers_per_session: bridge_file.max_transfers_per_session.unwrap_or(10),
        };
        if bridge.enabled && bridge.port == 0
            || !(1..=10_000).contains(&bridge.max_clients)
            || bridge.max_requests_per_minute == 0
            || bridge.max_transfers_per_session == 0
            || (bridge.enabled && bridge.require_auth && bridge.password.is_empty())
            || bridge.password.len() > 1024
        {
            return Err("virtualSoulfind.bridge settings are invalid".to_owned());
        }
        let disaster_file = &virtual_soulfind.disaster_mode;
        let disaster_mode = VirtualSoulfindDisasterModeSettings {
            auto: disaster_file.auto.unwrap_or(false),
            force: disaster_file.force.unwrap_or(false),
            unavailable_threshold: Duration::from_secs(
                disaster_file
                    .unavailable_threshold_minutes
                    .unwrap_or(10)
                    .saturating_mul(60),
            ),
            enable_graceful_degradation: disaster_file.enable_graceful_degradation.unwrap_or(true),
            recovery_check_interval: Duration::from_secs(
                disaster_file
                    .recovery_check_interval_minutes
                    .unwrap_or(5)
                    .saturating_mul(60),
            ),
            recovery_healthy_checks_required: disaster_file
                .recovery_healthy_checks_required
                .unwrap_or(3),
        };
        if disaster_mode.unavailable_threshold.is_zero()
            || disaster_mode.recovery_check_interval.is_zero()
            || disaster_mode.recovery_healthy_checks_required == 0
        {
            return Err("virtualSoulfind.disasterMode settings are invalid".to_owned());
        }

        Ok(Self {
            features,
            external_visualizer,
            solid,
            song_id_max_concurrent_runs,
            virtual_soulfind: VirtualSoulfindSettings {
                bridge,
                disaster_mode,
            },
        })
    }
}

fn validate_certificate_pins(
    path: &str,
    pins: &BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    if pins.len() > 256 {
        return Err(format!("{path} must contain at most 256 endpoints"));
    }
    for (endpoint, values) in pins {
        if endpoint.parse::<SocketAddr>().is_err()
            || values.is_empty()
            || values.len() > 16
            || values
                .iter()
                .any(|pin| pin.trim().is_empty() || pin.len() > 128)
        {
            return Err(format!("{path} contains an invalid endpoint or pin"));
        }
    }
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppFileConfig {
    http_bind: Option<String>,
    state_dir: Option<PathBuf>,
    auto_connect: Option<bool>,
    reconnect: Option<bool>,
    reconnect_seconds: Option<u64>,
    ping_seconds: Option<u64>,
    log_level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkFileConfig {
    server_address: Option<String>,
    listen_port: Option<u32>,
    username: Option<String>,
    password: Option<String>,
    credential_store: Option<String>,
    credential_file: Option<PathBuf>,
    private_message_auto_response: PrivateMessageAutoResponseFileConfig,
    obfuscation: SoulseekObfuscationFileConfig,
    connection: SoulseekConnectionFileConfig,
    distributed_network: SoulseekDistributedFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekDistributedFileConfig {
    disabled: Option<bool>,
    disable_children: Option<bool>,
    child_limit: Option<usize>,
    logging: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionFileConfig {
    timeout: SoulseekConnectionTimeoutFileConfig,
    buffer: SoulseekConnectionBufferFileConfig,
    proxy: SoulseekProxyFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionTimeoutFileConfig {
    connect: Option<u64>,
    inactivity: Option<u64>,
    transfer: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionBufferFileConfig {
    read: Option<usize>,
    write: Option<usize>,
    transfer: Option<usize>,
    write_queue: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekProxyFileConfig {
    enabled: Option<bool>,
    address: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekObfuscationFileConfig {
    enabled: Option<bool>,
    mode: Option<String>,
    advertise_regular_port: Option<bool>,
    prefer_outbound: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrivateMessageAutoResponseFileConfig {
    enabled: Option<bool>,
    message: Option<String>,
    cooldown_minutes: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListenerFileConfig {
    regular_bind: Option<String>,
    advertised_port: Option<u32>,
    obfuscated_bind: Option<String>,
    obfuscated_advertised_port: Option<u32>,
    overlay_bind: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileFileConfig {
    user_info_description: Option<String>,
    user_info_picture: Option<String>,
    soulseek_diagnostic_level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeoutFileConfig {
    peer_response_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShareFileConfig {
    dirs: Vec<String>,
    fixture: Option<String>,
    follow_symlinks: Option<bool>,
    include_hidden: Option<bool>,
    scan_max_files: Option<usize>,
    cache_tsv_enabled: Option<bool>,
    cache: ShareCacheFileConfig,
    probe_media_attributes: Option<bool>,
    filters: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShareCacheFileConfig {
    storage_mode: Option<String>,
    workers: Option<usize>,
    retention: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferFileConfig {
    history_limit: Option<usize>,
    max_active: Option<usize>,
    allow_inbound: Option<bool>,
    allow_outbound: Option<bool>,
    auto_retry: TransferAutoRetryFileConfig,
    rescue: TransferRescueFileConfig,
    completed_path_template: Option<String>,
    upload: TransferUploadFileConfig,
    download: TransferDownloadFileConfig,
    groups: GroupsFileConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferUploadFileConfig {
    slots: Option<u32>,
    speed_limit: Option<u32>,
    limits: Option<TransferLimitsFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadFileConfig {
    slots: Option<u32>,
    speed_limit: Option<u32>,
    retry: TransferDownloadRetryFileConfig,
    destination: TransferDownloadDestinationFileConfig,
    completed_layout: Option<String>,
    completed_path_template: Option<String>,
    auto_replace_stuck: Option<bool>,
    auto_replace_threshold: Option<f64>,
    auto_replace_interval: Option<u64>,
    auto_retry: TransferAutoRetryFileConfig,
    cost_based_scheduling: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadRetryFileConfig {
    partial: Option<String>,
    incomplete: Option<String>,
    attempts: Option<u32>,
    delay: Option<u64>,
    max_delay: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadDestinationFileConfig {
    subdirectory: NullableConfig<String>,
    exists: Option<String>,
    permissions: TransferDownloadPermissionsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadPermissionsFileConfig {
    mode: Option<String>,
}

impl SoulseekConnectionSettings {
    fn from_layers<E: ConfigEnv>(
        file: SoulseekConnectionFileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        let parse_bounded = |name: &str,
                             aliases: &[&str],
                             file_value: Option<usize>,
                             default: usize,
                             minimum: usize,
                             maximum: usize|
         -> Result<usize, String> {
            bounded_config_value(
                name,
                env_parse_any_layer(env, aliases, file_value, default)?,
                minimum,
                maximum,
            )
        };
        let buffer_read = parse_bounded(
            "SLSK_READ_BUFFER",
            &[
                "SLSKR_SLSK_READ_BUFFER",
                "SLSKD_SLSK_READ_BUFFER",
                "SLSK_READ_BUFFER",
            ],
            file.buffer.read,
            16_384,
            1_024,
            i32::MAX as usize,
        )?;
        let buffer_write = parse_bounded(
            "SLSK_WRITE_BUFFER",
            &[
                "SLSKR_SLSK_WRITE_BUFFER",
                "SLSKD_SLSK_WRITE_BUFFER",
                "SLSK_WRITE_BUFFER",
            ],
            file.buffer.write,
            16_384,
            1_024,
            i32::MAX as usize,
        )?;
        let buffer_transfer = parse_bounded(
            "SLSK_TRANSFER_BUFFER",
            &[
                "SLSKR_SLSK_TRANSFER_BUFFER",
                "SLSKD_SLSK_TRANSFER_BUFFER",
                "SLSK_TRANSFER_BUFFER",
            ],
            file.buffer.transfer,
            262_144,
            81_920,
            i32::MAX as usize,
        )?;
        let buffer_write_queue = parse_bounded(
            "SLSK_WRITE_QUEUE",
            &[
                "SLSKR_SLSK_WRITE_QUEUE",
                "SLSKD_SLSK_WRITE_QUEUE",
                "SLSK_WRITE_QUEUE",
            ],
            file.buffer.write_queue,
            50,
            5,
            5_000,
        )?;
        let parse_timeout = |name: &str,
                             aliases: &[&str],
                             file_value: Option<u64>,
                             default: u64,
                             minimum: u64|
         -> Result<Duration, String> {
            bounded_config_value(
                name,
                env_parse_any_layer(env, aliases, file_value, default)?,
                minimum,
                i32::MAX as u64,
            )
            .map(Duration::from_millis)
        };
        let timeout_connect = parse_timeout(
            "SLSK_CONNECTION_TIMEOUT",
            &[
                "SLSKR_SLSK_CONNECTION_TIMEOUT",
                "SLSKD_SLSK_CONNECTION_TIMEOUT",
                "SLSK_CONNECTION_TIMEOUT",
            ],
            file.timeout.connect,
            10_000,
            1_000,
        )?;
        let timeout_inactivity = parse_timeout(
            "SLSK_INACTIVITY_TIMEOUT",
            &[
                "SLSKR_SLSK_INACTIVITY_TIMEOUT",
                "SLSKD_SLSK_INACTIVITY_TIMEOUT",
                "SLSK_INACTIVITY_TIMEOUT",
            ],
            file.timeout.inactivity,
            if target == ControllerCompatibilityTarget::Slskd {
                15_000
            } else {
                60_000
            },
            1_000,
        )?;
        let timeout_transfer = parse_timeout(
            "SLSK_TRANSFER_TIMEOUT",
            &[
                "SLSKR_SLSK_TRANSFER_TIMEOUT",
                "SLSKD_SLSK_TRANSFER_TIMEOUT",
                "SLSK_TRANSFER_TIMEOUT",
            ],
            file.timeout.transfer,
            60_000,
            30_000,
        )?;
        let proxy_enabled = env_bool_any_layer(
            env,
            &[
                "SLSKR_SLSK_PROXY_ENABLED",
                "SLSKD_SLSK_PROXY_ENABLED",
                "SLSK_PROXY_ENABLED",
            ],
            file.proxy.enabled.unwrap_or(false),
        )?;
        let layered_string = |aliases: &[&str], file_value: Option<String>| {
            optional_env_any(env, aliases)
                .or(file_value)
                .unwrap_or_default()
        };
        let proxy_address = layered_string(
            &[
                "SLSKR_SLSK_PROXY_ADDRESS",
                "SLSKD_SLSK_PROXY_ADDRESS",
                "SLSK_PROXY_ADDRESS",
            ],
            file.proxy.address,
        );
        let proxy_username = layered_string(
            &[
                "SLSKR_SLSK_PROXY_USERNAME",
                "SLSKD_SLSK_PROXY_USERNAME",
                "SLSK_PROXY_USERNAME",
            ],
            file.proxy.username,
        );
        let proxy_password = layered_string(
            &[
                "SLSKR_SLSK_PROXY_PASSWORD",
                "SLSKD_SLSK_PROXY_PASSWORD",
                "SLSK_PROXY_PASSWORD",
            ],
            file.proxy.password,
        );
        let proxy_port = optional_env_any(
            env,
            &[
                "SLSKR_SLSK_PROXY_PORT",
                "SLSKD_SLSK_PROXY_PORT",
                "SLSK_PROXY_PORT",
            ],
        )
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|error| format!("invalid SLSK_PROXY_PORT: {error}"))
        })
        .transpose()?
        .or(file.proxy.port);
        for (field, value) in [
            ("Address", proxy_address.as_str()),
            ("Username", proxy_username.as_str()),
            ("Password", proxy_password.as_str()),
        ] {
            if value.encode_utf16().count() > 255 {
                return Err(format!("Soulseek proxy {field} exceeds 255 characters"));
            }
        }
        if proxy_enabled && proxy_address.trim().is_empty() {
            return Err("Soulseek proxy is enabled but no address is configured".to_owned());
        }
        if proxy_enabled && proxy_port.is_none() {
            return Err("Soulseek proxy is enabled but no port is configured".to_owned());
        }
        Ok(Self {
            buffer_read,
            buffer_write,
            buffer_transfer,
            buffer_write_queue,
            timeout_connect,
            timeout_inactivity,
            timeout_transfer,
            proxy: SoulseekProxySettings {
                enabled: proxy_enabled,
                address: proxy_address,
                port: proxy_port,
                username: proxy_username,
                password: proxy_password,
            },
        })
    }
}

impl TransferLimitSettings {
    fn from_file(value: TransferLimitFileConfig, path: &str) -> Result<Self, String> {
        for (name, candidate) in [
            ("files", value.files),
            ("megabytes", value.megabytes),
            ("failures", value.failures),
        ] {
            if candidate == Some(0) {
                return Err(format!("{path}.{name} must be greater than or equal to 1"));
            }
        }
        Ok(Self {
            files: value.files,
            megabytes: value.megabytes,
            failures: value.failures,
        })
    }
}

impl TransferLimitsSettings {
    fn from_file(value: Option<TransferLimitsFileConfig>, path: &str) -> Result<Self, String> {
        let value = value.unwrap_or_default();
        fn window(
            value: NullableConfig<TransferLimitFileConfig>,
            path: &str,
        ) -> Result<Option<TransferLimitSettings>, String> {
            match value {
                NullableConfig::Missing => Ok(Some(TransferLimitSettings::default())),
                // Frozen slskd/slskdN treat an explicitly null limit window the
                // same as an omitted window and materialize the default object.
                NullableConfig::Null => Ok(Some(TransferLimitSettings::default())),
                NullableConfig::Value(value) => {
                    TransferLimitSettings::from_file(value, path).map(Some)
                }
            }
        }
        Ok(Self {
            queued: window(value.queued, &format!("{path}.queued"))?,
            daily: window(value.daily, &format!("{path}.daily"))?,
            weekly: window(value.weekly, &format!("{path}.weekly"))?,
        })
    }
}

impl TransferGroupUploadSettings {
    fn from_file(
        value: TransferGroupUploadFileConfig,
        compatibility_limits: Option<TransferLimitsFileConfig>,
        path: &str,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        let priority = value.priority.unwrap_or(1);
        let slots = value.slots.unwrap_or(i32::MAX as u32);
        let speed_limit_kib = value.speed_limit.unwrap_or(i32::MAX as u32);
        for (name, candidate) in [
            ("priority", priority),
            ("slots", slots),
            ("speed_limit", speed_limit_kib),
        ] {
            if candidate == 0 || candidate > i32::MAX as u32 {
                return Err(format!("{path}.{name} must be between 1 and {}", i32::MAX));
            }
        }
        let allowed_file_types = value
            .allowed_file_types
            .into_iter()
            .map(|entry| entry.trim().to_owned())
            .collect::<Vec<_>>();
        if target == ControllerCompatibilityTarget::Slskd && !allowed_file_types.is_empty() {
            return Err(format!(
                "{path}.allowed_file_types is not supported by slskd"
            ));
        }
        Ok(Self {
            priority,
            strategy: TransferQueueStrategy::parse(
                value.strategy.as_deref().unwrap_or("roundrobin"),
            )?,
            slots,
            speed_limit_kib,
            allowed_file_types,
            limits: TransferLimitsSettings::from_file(
                value.limits.or(compatibility_limits),
                &format!("{path}.limits"),
            )?,
        })
    }
}

impl TransferGroupsSettings {
    fn from_layers<E: ConfigEnv>(
        canonical: GroupsFileConfig,
        compatibility: GroupsFileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        let groups = match env.var("SLSKR_FROZEN_TRANSFER_GROUPS_JSON") {
            Some(json) => serde_json::from_str::<GroupsFileConfig>(&json)
                .map_err(|error| format!("invalid transfer groups configuration: {error}"))?,
            None if groups_file_config_is_empty(&canonical) => compatibility,
            None => canonical,
        };
        let default = TransferGroupSettings {
            upload: TransferGroupUploadSettings::from_file(
                groups.default.upload,
                groups.default.limits,
                "transfers.groups.default.upload",
                target,
            )?,
        };
        let leechers = LeecherTransferGroupSettings {
            upload: TransferGroupUploadSettings::from_file(
                groups.leechers.upload,
                groups.leechers.limits,
                "transfers.groups.leechers.upload",
                target,
            )?,
            threshold_files: groups.leechers.thresholds.files.unwrap_or(1),
            threshold_directories: groups.leechers.thresholds.directories.unwrap_or(1),
        };
        if leechers.threshold_files == 0 || leechers.threshold_directories == 0 {
            return Err(
                "transfers.groups.leechers.thresholds values must be greater than or equal to 1"
                    .to_owned(),
            );
        }
        let blacklisted_members = groups.blacklisted.members.clone();
        let mut user_defined = BTreeMap::new();
        for (name, group) in groups.user_defined {
            if ["privileged", "default", "leechers"]
                .iter()
                .any(|built_in| name.eq_ignore_ascii_case(built_in))
            {
                return Err(format!(
                    "User defined group '{name}' collides with a built in group.  Choose a different name."
                ));
            }
            let members = group
                .members
                .into_iter()
                .map(|member| member.trim().to_owned())
                .collect::<Vec<_>>();
            user_defined.insert(
                name.clone(),
                UserDefinedTransferGroupSettings {
                    upload: TransferGroupUploadSettings::from_file(
                        group.upload,
                        group.limits,
                        &format!("transfers.groups.user_defined.{name}.upload"),
                        target,
                    )?,
                    members,
                },
            );
        }
        if target == ControllerCompatibilityTarget::Slskdn {
            let mut memberships = BTreeMap::<String, String>::new();
            for member in &blacklisted_members {
                let member = member.trim();
                if !member.is_empty() {
                    memberships.insert(member.to_ascii_lowercase(), "blacklisted".to_owned());
                }
            }
            for (group_name, group) in &user_defined {
                for member in &group.members {
                    if member.is_empty() {
                        continue;
                    }
                    let key = member.to_ascii_lowercase();
                    if memberships.insert(key, group_name.clone()).is_some() {
                        return Err(format!(
                            "One or more users are defined in multiple groups: {member}. Each user can only belong to one explicit group."
                        ));
                    }
                }
            }
        }
        Ok(Self {
            default,
            leechers,
            user_defined,
        })
    }
}

fn groups_file_config_is_empty(value: &GroupsFileConfig) -> bool {
    value.default.upload.priority.is_none()
        && value.default.upload.strategy.is_none()
        && value.default.upload.slots.is_none()
        && value.default.upload.speed_limit.is_none()
        && value.default.upload.allowed_file_types.is_empty()
        && value.default.upload.limits.is_none()
        && value.default.limits.is_none()
        && value.leechers.upload.priority.is_none()
        && value.leechers.upload.strategy.is_none()
        && value.leechers.upload.slots.is_none()
        && value.leechers.upload.speed_limit.is_none()
        && value.leechers.upload.allowed_file_types.is_empty()
        && value.leechers.upload.limits.is_none()
        && value.leechers.limits.is_none()
        && value.leechers.thresholds.files.is_none()
        && value.leechers.thresholds.directories.is_none()
        && value.blacklisted.members.is_empty()
        && value.blacklisted.patterns.is_empty()
        && value.blacklisted.cidrs.is_empty()
        && value.user_defined.is_empty()
}

impl TransferUploadSettings {
    fn from_layers<E: ConfigEnv>(
        mut file: TransferUploadFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        if let Some(json) = env.var("SLSKR_FROZEN_TRANSFER_UPLOAD_JSON") {
            file = serde_json::from_str::<TransferUploadFileConfig>(&json)
                .map_err(|error| format!("invalid transfer upload configuration: {error}"))?;
        }
        let slots = env_parse_layer(env, "SLSKD_UPLOAD_SLOTS", file.slots, 10_u32)?;
        let speed_limit_kib = env_parse_layer(
            env,
            "SLSKD_UPLOAD_SPEED_LIMIT",
            file.speed_limit,
            i32::MAX as u32,
        )?;
        if slots == 0 || slots > i32::MAX as u32 {
            return Err(format!("upload slots must be between 1 and {}", i32::MAX));
        }
        if speed_limit_kib == 0 || speed_limit_kib > i32::MAX as u32 {
            return Err(format!(
                "upload speed limit must be between 1 and {}",
                i32::MAX
            ));
        }
        Ok(Self {
            slots,
            speed_limit_kib,
            limits: TransferLimitsSettings::from_file(file.limits, "transfers.upload.limits")?,
        })
    }
}

impl TransferDownloadSettings {
    fn from_layers<E: ConfigEnv>(
        mut file: TransferDownloadFileConfig,
        env: &E,
        target: ControllerCompatibilityTarget,
    ) -> Result<Self, String> {
        if let Some(json) = env.var("SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON") {
            file = serde_json::from_str::<TransferDownloadFileConfig>(&json)
                .map_err(|error| format!("invalid transfer download configuration: {error}"))?;
        }
        let slots = env_parse_layer(env, "SLSKD_DOWNLOAD_SLOTS", file.slots, i32::MAX as u32)?;
        let speed_limit_kib = env_parse_layer(
            env,
            "SLSKD_DOWNLOAD_SPEED_LIMIT",
            file.speed_limit,
            i32::MAX as u32,
        )?;
        for (name, value) in [("slots", slots), ("speed limit", speed_limit_kib)] {
            if value == 0 || value > i32::MAX as u32 {
                return Err(format!(
                    "download {name} must be between 1 and {}",
                    i32::MAX
                ));
            }
        }

        let incomplete = match target {
            ControllerCompatibilityTarget::Slskd => file.retry.partial,
            ControllerCompatibilityTarget::Slskdn => file.retry.incomplete,
        }
        .unwrap_or_else(|| "resume".to_owned())
        .to_ascii_lowercase();
        if !matches!(incomplete.as_str(), "resume" | "overwrite") {
            return Err(format!(
                "download retry strategy '{incomplete}' must be resume or overwrite"
            ));
        }
        let default_attempts = if target == ControllerCompatibilityTarget::Slskd {
            3
        } else {
            1
        };
        let attempts = file.retry.attempts.unwrap_or(default_attempts);
        let delay_ms = file.retry.delay.unwrap_or(5_000);
        let max_delay_ms = file.retry.max_delay.unwrap_or(60_000);
        match target {
            ControllerCompatibilityTarget::Slskd => {
                if attempts == 0 {
                    return Err(
                        "download retry attempts must be greater than or equal to 1".to_owned()
                    );
                }
                if delay_ms < 1_000 {
                    return Err(
                        "download retry delay must be greater than or equal to 1000".to_owned()
                    );
                }
                if max_delay_ms < 30_000 {
                    return Err(
                        "download retry max delay must be greater than or equal to 30000"
                            .to_owned(),
                    );
                }
            }
            ControllerCompatibilityTarget::Slskdn => {
                if !(1..=20).contains(&attempts) {
                    return Err("download retry attempts must be between 1 and 20".to_owned());
                }
                if delay_ms > 3_600_000 {
                    return Err("download retry delay must be between 0 and 3600000".to_owned());
                }
                if !(1_000..=86_400_000).contains(&max_delay_ms) {
                    return Err(
                        "download retry max delay must be between 1000 and 86400000".to_owned()
                    );
                }
            }
        }

        let subdirectory = match file.destination.subdirectory {
            NullableConfig::Missing => Some("${SOURCE_DIRECTORY}".to_owned()),
            NullableConfig::Null => None,
            NullableConfig::Value(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err("download destination subdirectory must not be empty".to_owned());
                }
                let path = Path::new(trimmed);
                if path.is_absolute()
                    || path
                        .components()
                        .any(|component| component == std::path::Component::ParentDir)
                {
                    return Err(
                        "download destination subdirectory must be a non-traversing relative path"
                            .to_owned(),
                    );
                }
                Some(value)
            }
        };
        let exists = file
            .destination
            .exists
            .unwrap_or_else(|| "rename".to_owned())
            .to_ascii_lowercase();
        if !matches!(exists.as_str(), "rename" | "overwrite") {
            return Err(format!(
                "download destination exists strategy '{exists}' must be rename or overwrite"
            ));
        }
        let permissions_mode = file.destination.permissions.mode;
        if let Some(mode) = permissions_mode.as_deref() {
            let valid = matches!(mode.len(), 3 | 4)
                && mode.bytes().all(|value| matches!(value, b'0'..=b'7'));
            if !valid {
                return Err("download destination permissions mode must be a three- or four-character chmod value".to_owned());
            }
        }

        let completed_layout = env
            .var("SLSKD_DOWNLOAD_COMPLETED_LAYOUT")
            .or(file.completed_layout)
            .unwrap_or_else(|| "remote_folder".to_owned())
            .to_ascii_lowercase();
        let auto_replace_stuck = env_bool_layer(
            env,
            "SLSKD_AUTO_REPLACE_STUCK",
            file.auto_replace_stuck.unwrap_or(false),
        )?;
        let auto_replace_threshold_percent = bounded_config_value(
            "SLSKD_AUTO_REPLACE_THRESHOLD",
            env_parse_layer(
                env,
                "SLSKD_AUTO_REPLACE_THRESHOLD",
                file.auto_replace_threshold,
                5.0_f64,
            )?,
            0.1,
            50.0,
        )?;
        let auto_replace_interval_seconds = bounded_config_value(
            "SLSKD_AUTO_REPLACE_INTERVAL",
            env_parse_layer(
                env,
                "SLSKD_AUTO_REPLACE_INTERVAL",
                file.auto_replace_interval,
                60_u64,
            )?,
            10,
            3_600,
        )?;

        Ok(Self {
            slots,
            speed_limit_kib,
            retry: TransferDownloadRetrySettings {
                incomplete,
                attempts,
                delay: Duration::from_millis(delay_ms),
                max_delay: Duration::from_millis(max_delay_ms),
            },
            destination: TransferDownloadDestinationSettings {
                subdirectory,
                exists,
                permissions_mode,
            },
            completed_layout,
            auto_replace_stuck,
            auto_replace_threshold_percent,
            auto_replace_interval: Duration::from_secs(auto_replace_interval_seconds),
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferAutoRetryFileConfig {
    enabled: Option<bool>,
    retry_delay_seconds: Option<u64>,
    check_interval_seconds: Option<u64>,
    max_attempts: Option<usize>,
    max_files_per_cycle: Option<usize>,
    max_files_per_peer_per_cycle: Option<usize>,
    peer_cooldown_seconds: Option<u64>,
    alternate_sources_enabled: Option<bool>,
    max_alternate_source_searches_per_cycle: Option<usize>,
    alternate_source_size_tolerance_percent: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferRescueFileConfig {
    enabled: Option<bool>,
    max_queue_time_seconds: Option<u64>,
    min_throughput_kbps: Option<u64>,
    min_duration_seconds: Option<u64>,
    stalled_timeout_seconds: Option<u64>,
    check_interval_seconds: Option<u64>,
    retry_cooldown_seconds: Option<u64>,
    max_files_per_cycle: Option<usize>,
    alternate_source_size_tolerance_percent: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthFileConfig {
    disabled: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    jwt: AuthJwtFileConfig,
    api_token: Option<String>,
    read_write_token: Option<String>,
    read_only_token: Option<String>,
    nowplaying_token: Option<String>,
    cookie_auth_enabled: Option<bool>,
    rate_limit_anonymous: Option<u32>,
    rate_limit_authenticated: Option<u32>,
    trusted_proxy_cidrs: Vec<String>,
    api_keys: BTreeMap<String, ControllerApiKeyFileConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControllerApiKeyFileConfig {
    key: String,
    role: String,
    cidr: String,
}

impl Default for ControllerApiKeyFileConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            role: "readonly".to_owned(),
            cidr: String::new(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthJwtFileConfig {
    key: Option<String>,
    ttl: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceFileConfig {
    enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationsFileConfig {
    spotify: SpotifyFileConfig,
    lidarr: LidarrFileConfig,
    youtube: SourceFeedApiKeyFileConfig,
    lastfm: SourceFeedApiKeyFileConfig,
    ntfy: NtfyFileConfig,
    pushover: PushoverFileConfig,
    pushbullet: PushbulletFileConfig,
    ftp: FtpFileConfig,
    vpn: VpnFileConfig,
    scripts: BTreeMap<String, ScriptIntegrationSettings>,
    webhooks: BTreeMap<String, FrozenWebhookSettings>,
    bridge: BridgeFileConfig,
    external_visualizer: ExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NtfyFileConfig {
    enabled: Option<bool>,
    url: Option<String>,
    access_token: Option<String>,
    notification_prefix: Option<String>,
    notify_on_private_message: Option<bool>,
    notify_on_room_mention: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushoverFileConfig {
    enabled: Option<bool>,
    user_key: Option<String>,
    token: Option<String>,
    notification_prefix: Option<String>,
    notify_on_private_message: Option<bool>,
    notify_on_room_mention: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushbulletFileConfig {
    enabled: Option<bool>,
    access_token: Option<String>,
    notification_prefix: Option<String>,
    notify_on_private_message: Option<bool>,
    notify_on_room_mention: Option<bool>,
    retry_attempts: Option<u32>,
    cooldown_time: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FtpFileConfig {
    enabled: Option<bool>,
    address: Option<String>,
    port: Option<u16>,
    encryption_mode: Option<String>,
    ignore_certificate_errors: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    remote_path: Option<String>,
    overwrite_existing: Option<bool>,
    connection_timeout: Option<u64>,
    retry_attempts: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VpnFileConfig {
    enabled: Option<bool>,
    port_forwarding: Option<bool>,
    polling_interval: Option<u64>,
    gluetun: GluetunFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GluetunFileConfig {
    url: Option<String>,
    timeout: Option<u64>,
    auth: Option<String>,
    username: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourceFeedApiKeyFileConfig {
    enabled: Option<bool>,
    api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpotifyFileConfig {
    enabled: Option<bool>,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
    timeout_seconds: Option<u64>,
    max_items_per_import: Option<u64>,
    market: Option<String>,
    scopes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LidarrFileConfig {
    enabled: Option<bool>,
    url: Option<String>,
    api_key: Option<String>,
    timeout_seconds: Option<u64>,
    sync_wanted_to_wishlist: Option<bool>,
    sync_interval_seconds: Option<u64>,
    max_items_per_sync: Option<u64>,
    auto_download: Option<bool>,
    wishlist_filter: Option<String>,
    wishlist_max_results: Option<u64>,
    auto_import_completed: Option<bool>,
    import_path_from: Option<String>,
    import_path_to: Option<String>,
    import_mode: Option<String>,
    import_replace_existing_files: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BridgeFileConfig {
    enabled: Option<bool>,
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExternalVisualizerFileConfig {
    command: Option<String>,
    launch_enabled: Option<bool>,
}

pub fn default_state_dir() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".local/state"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("slskr")
}

fn default_config_file() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("slskr/config.toml")
}

pub fn load_file_config() -> Result<(Option<PathBuf>, FileConfig), String> {
    let explicit_path = env::var_os("SLSKR_CONFIG").map(PathBuf::from);
    let path = explicit_path.clone().or_else(|| {
        let default = default_config_file();
        default.exists().then_some(default)
    });

    let Some(path) = path else {
        return Ok((None, FileConfig::default()));
    };
    let config = read_file_config(&path)?;
    Ok((Some(path), config))
}

fn read_file_config(path: &std::path::Path) -> Result<FileConfig, String> {
    use std::io::Read;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("failed to read config file {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("failed to inspect config file {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "config path {} is not a regular file",
            path.display()
        ));
    }
    if metadata.len() > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "config file {} is too large: {} bytes, max is {MAX_CONFIG_FILE_BYTES}",
            path.display(),
            metadata.len()
        ));
    }
    let mut body = String::new();
    file.take(MAX_CONFIG_FILE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("failed to read config file {}: {error}", path.display()))?;
    if body.len() as u64 > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "config file {} is too large: more than {MAX_CONFIG_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let config = toml::from_str::<FileConfig>(&body)
        .map_err(|error| format!("failed to parse config file {}: {error}", path.display()))?;
    warn_insecure_config_permissions(path, &metadata, &config);
    Ok(config)
}

#[cfg(unix)]
fn warn_insecure_config_permissions(
    path: &std::path::Path,
    metadata: &fs::Metadata,
    config: &FileConfig,
) {
    use std::os::unix::fs::PermissionsExt;

    if !config_contains_sensitive_values(config) {
        return;
    }

    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        eprintln!(
            "warning: config file {} contains secrets and is readable by group/other users; recommended mode is 0600",
            path.display()
        );
    }
}

#[cfg(not(unix))]
fn warn_insecure_config_permissions(
    _path: &std::path::Path,
    _metadata: &fs::Metadata,
    _config: &FileConfig,
) {
}

fn config_contains_sensitive_values(config: &FileConfig) -> bool {
    config.network.username.is_some()
        || config.network.password.is_some()
        || config.metrics.authentication.password.is_some()
        || config.auth.password.is_some()
        || config.auth.jwt.key.is_some()
        || config.auth.api_token.is_some()
        || config.auth.read_write_token.is_some()
        || config.auth.read_only_token.is_some()
        || config.auth.nowplaying_token.is_some()
        || config.integrations.spotify.client_secret.is_some()
        || config.integrations.lidarr.api_key.is_some()
        || config.integrations.youtube.api_key.is_some()
        || config.integrations.lastfm.api_key.is_some()
        || config.integrations.ntfy.access_token.is_some()
        || config.integrations.pushover.user_key.is_some()
        || config.integrations.pushover.token.is_some()
        || config.integrations.pushbullet.access_token.is_some()
        || config.integrations.ftp.password.is_some()
        || config.integrations.vpn.gluetun.password.is_some()
        || config.integrations.vpn.gluetun.api_key.is_some()
}

pub trait ConfigEnv {
    fn var(&self, name: &str) -> Option<String>;

    fn command_line_var(&self, _name: &str) -> Option<String> {
        None
    }
}

pub struct ProcessEnv;

impl ConfigEnv for ProcessEnv {
    fn var(&self, name: &str) -> Option<String> {
        env::var(name).ok()
    }
}

struct ControllerYamlEnv<'a, E> {
    base: &'a E,
    yaml: BTreeMap<String, String>,
}

impl<E: ConfigEnv> ConfigEnv for ControllerYamlEnv<'_, E> {
    fn var(&self, name: &str) -> Option<String> {
        self.base
            .command_line_var(name)
            .or_else(|| self.yaml.get(name).cloned())
            .or_else(|| self.base.var(name))
    }

    fn command_line_var(&self, name: &str) -> Option<String> {
        self.base.command_line_var(name)
    }
}

const CONTROLLER_YAML_CORE_MAPPINGS: &[(&str, &str)] = &[
    ("headless", "SLSKD_HEADLESS"),
    ("blacklist.enabled", "SLSKD_BLACKLIST"),
    ("blacklist.file", "SLSKD_BLACKLIST_FILE"),
    ("feature.swagger", "SLSKD_SWAGGER"),
    ("flags.force_migrations", "SLSKD_FORCE_MIGRATIONS"),
    (
        "flags.legacy_windows_tcp_keepalive",
        "SLSKD_LEGACY_WINDOWS_TCP_KEEPALIVE",
    ),
    ("flags.log_sql", "SLSKD_LOG_SQL"),
    (
        "flags.log_unobserved_exceptions",
        "SLSKD_LOG_UNOBSERVED_EXCEPTIONS",
    ),
    (
        "flags.optimistic_relay_file_info",
        "SLSKD_OPTIMISTIC_RELAY_FILE_INFO",
    ),
    ("flags.volatile", "SLSKD_VOLATILE"),
    ("rooms", "SLSKD_ROOMS"),
    ("logger.disk", "SLSKD_DISK_LOGGER"),
    ("logger.loki", "SLSKD_LOKI"),
    ("logger.no_color", "SLSKD_NO_COLOR"),
    (
        "shares.cache.storage_mode",
        "SLSKD_SHARE_CACHE_STORAGE_MODE",
    ),
    ("shares.cache.workers", "SLSKD_SHARE_CACHE_WORKERS"),
    ("shares.cache.retention", "SLSKD_SHARE_CACHE_RETENTION"),
    (
        "shares.probe_media_attributes",
        "SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES",
    ),
    ("soulseek.liked_interests", "SLSKD_SLSK_LIKED_INTERESTS"),
    ("soulseek.hated_interests", "SLSKD_SLSK_HATED_INTERESTS"),
    ("wishlist.enabled", "SLSKD_WISHLIST_ENABLED"),
    ("wishlist.interval_seconds", "SLSKD_WISHLIST_INTERVAL"),
    ("wishlist.auto_download", "SLSKD_WISHLIST_AUTO_DOWNLOAD"),
    ("wishlist.max_results", "SLSKD_WISHLIST_MAX_RESULTS"),
    (
        "throttling.search.incoming.concurrency",
        "SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY",
    ),
    (
        "throttling.search.incoming.circuit_breaker",
        "SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER",
    ),
    (
        "throttling.search.incoming.response_file_limit",
        "SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT",
    ),
    ("permissions.file.mode", "SLSKD_FILE_PERMISSION_MODE"),
    ("telemetry.tracing.enabled", "SLSKD_TELEMETRY_TRACING"),
    (
        "telemetry.tracing.exporter",
        "SLSKD_TELEMETRY_TRACING_EXPORTER",
    ),
    (
        "telemetry.tracing.jaeger_endpoint",
        "SLSKD_TELEMETRY_JAEGER_ENDPOINT",
    ),
    (
        "telemetry.tracing.jaeger_port",
        "SLSKD_TELEMETRY_JAEGER_PORT",
    ),
    (
        "telemetry.tracing.otlp_endpoint",
        "SLSKD_TELEMETRY_OTLP_ENDPOINT",
    ),
    ("retention.search", "SLSKR_RETENTION_SEARCH"),
    ("retention.logs", "SLSKR_RETENTION_LOGS"),
    ("retention.files.complete", "SLSKR_RETENTION_FILES_COMPLETE"),
    (
        "retention.files.incomplete",
        "SLSKR_RETENTION_FILES_INCOMPLETE",
    ),
    (
        "retention.transfers.upload.succeeded",
        "SLSKR_RETENTION_UPLOAD_SUCCEEDED",
    ),
    (
        "retention.transfers.upload.errored",
        "SLSKR_RETENTION_UPLOAD_ERRORED",
    ),
    (
        "retention.transfers.upload.cancelled",
        "SLSKR_RETENTION_UPLOAD_CANCELLED",
    ),
    (
        "retention.transfers.upload.failed",
        "SLSKR_RETENTION_UPLOAD_FAILED",
    ),
    (
        "retention.transfers.download.succeeded",
        "SLSKR_RETENTION_DOWNLOAD_SUCCEEDED",
    ),
    (
        "retention.transfers.download.errored",
        "SLSKR_RETENTION_DOWNLOAD_ERRORED",
    ),
    (
        "retention.transfers.download.cancelled",
        "SLSKR_RETENTION_DOWNLOAD_CANCELLED",
    ),
    (
        "retention.transfers.download.failed",
        "SLSKR_RETENTION_DOWNLOAD_FAILED",
    ),
    (
        "filters.search_retention.max_age_days",
        "SLSKD_SEARCH_RETENTION_MAX_AGE_DAYS",
    ),
    (
        "filters.search_retention.max_count",
        "SLSKD_SEARCH_RETENTION_MAX_COUNT",
    ),
    (
        "filters.search_retention.cleanup_interval_seconds",
        "SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL",
    ),
    ("metrics.enabled", "SLSKD_METRICS"),
    ("metrics.url", "SLSKD_METRICS_URL"),
    ("metrics.authentication.disabled", "SLSKD_METRICS_NO_AUTH"),
    ("metrics.authentication.username", "SLSKD_METRICS_USERNAME"),
    ("metrics.authentication.password", "SLSKD_METRICS_PASSWORD"),
    (
        "web.max_request_body_size",
        "SLSKD_WEB_MAX_REQUEST_BODY_SIZE",
    ),
    ("web.socket", "SLSKD_HTTP_SOCKET"),
    ("web.url_base", "SLSKD_URL_BASE"),
    ("web.content_path", "SLSKD_CONTENT_PATH"),
    ("web.logging", "SLSKD_HTTP_LOGGING"),
    ("web.https.disabled", "SLSKD_NO_HTTPS"),
    ("web.https.port", "SLSKD_HTTPS_PORT"),
    ("web.https.ip_address", "SLSKD_HTTPS_IP_ADDRESS"),
    ("web.https.force", "SLSKD_HTTPS_FORCE"),
    ("web.https.certificate.pfx", "SLSKD_HTTPS_CERT_PFX"),
    (
        "web.https.certificate.password",
        "SLSKD_HTTPS_CERT_PASSWORD",
    ),
    ("web.enforce_security", "SLSKD_ENFORCE_SECURITY"),
    ("web.allow_remote_no_auth", "SLSKD_ALLOW_REMOTE_NO_AUTH"),
    ("web.authentication.disabled", "SLSKR_AUTH_DISABLED"),
    ("web.authentication.username", "SLSKD_USERNAME"),
    ("web.authentication.password", "SLSKD_PASSWORD"),
    ("web.authentication.jwt.key", "SLSKD_JWT_KEY"),
    ("web.authentication.jwt.ttl", "SLSKD_JWT_TTL"),
    (
        "flags.hash_from_audio_file_enabled",
        "SLSKR_CONTROLLER_YAML_HASH_FROM_AUDIO_FILE_ENABLED",
    ),
    ("diagnostics.allow_memory_dump", "SLSKD_ALLOW_MEMORY_DUMP"),
    ("diagnostics.allow_remote_dump", "SLSKD_ALLOW_REMOTE_DUMP"),
    (
        "web.authentication.passthrough.allowed_cidrs",
        "SLSKD_PASSTHROUGH_ALLOWED_CIDRS",
    ),
    ("web.cors.enabled", "SLSKD_WEB_CORS_ENABLED"),
    (
        "web.cors.allow_credentials",
        "SLSKD_WEB_CORS_ALLOW_CREDENTIALS",
    ),
    ("web.cors.allowed_origins", "SLSKD_WEB_CORS_ALLOWED_ORIGINS"),
    ("web.cors.allowed_headers", "SLSKD_WEB_CORS_ALLOWED_HEADERS"),
    ("web.cors.allowed_methods", "SLSKD_WEB_CORS_ALLOWED_METHODS"),
    ("web.rate_limiting.enabled", "SLSKD_WEB_RATE_LIMITING"),
    (
        "web.rate_limiting.api_permit_limit",
        "SLSKD_WEB_API_PERMIT_LIMIT",
    ),
    (
        "web.rate_limiting.api_window_seconds",
        "SLSKD_WEB_API_WINDOW_SECONDS",
    ),
    (
        "web.rate_limiting.federation_permit_limit",
        "SLSKD_WEB_FEDERATION_PERMIT_LIMIT",
    ),
    (
        "web.rate_limiting.federation_window_seconds",
        "SLSKD_WEB_FEDERATION_WINDOW_SECONDS",
    ),
    (
        "web.rate_limiting.mesh_gateway_permit_limit",
        "SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT",
    ),
    (
        "web.rate_limiting.mesh_gateway_window_seconds",
        "SLSKD_WEB_MESH_GATEWAY_WINDOW_SECONDS",
    ),
    ("instance_name", "SLSKD_INSTANCE_NAME"),
    ("directories.downloads", "SLSKD_DOWNLOADS_DIR"),
    ("directories.incomplete", "SLSKD_INCOMPLETE_DIR"),
    ("dht.dht_port", "SLSKR_DHT_PORT"),
    ("dht.enabled", "SLSKR_DHT_ENABLED"),
    ("debug", "SLSKD_DEBUG"),
    ("flags.no_config_watch", "SLSKD_NO_CONFIG_WATCH"),
    ("flags.no_connect", "SLSKD_NO_CONNECT"),
    ("flags.no_logo", "SLSKD_NO_LOGO"),
    ("flags.no_start", "SLSKD_NO_START"),
    ("flags.no_version_check", "SLSKD_NO_VERSION_CHECK"),
    ("flags.experimental", "SLSKD_EXPERIMENTAL"),
    ("flags.case_sensitive_reg_ex", "SLSKD_CASE_SENSITIVE_REGEX"),
    ("flags.no_share_scan", "SLSKD_NO_SHARE_SCAN"),
    ("flags.force_share_scan", "SLSKD_FORCE_SHARE_SCAN"),
    ("remote_configuration", "SLSKD_REMOTE_CONFIGURATION"),
    ("remote_file_management", "SLSKD_REMOTE_FILE_MANAGEMENT"),
    ("shares.directories", "SLSKD_SHARED_DIR"),
    ("shares.filters", "SLSKD_SHARE_FILTER"),
    ("filters.search.request", "SLSKD_SEARCH_REQUEST_FILTER"),
    (
        "transfers.groups.blacklisted.members",
        "SLSKD_BLACKLISTED_MEMBERS",
    ),
    (
        "transfers.groups.blacklisted.patterns",
        "SLSKD_BLACKLISTED_PATTERNS",
    ),
    (
        "transfers.groups.blacklisted.cidrs",
        "SLSKD_BLACKLISTED_CIDRS",
    ),
    ("groups.blacklisted.members", "SLSKD_BLACKLISTED_MEMBERS"),
    ("groups.blacklisted.patterns", "SLSKD_BLACKLISTED_PATTERNS"),
    ("groups.blacklisted.cidrs", "SLSKD_BLACKLISTED_CIDRS"),
    ("soulseek.address", "SLSKD_SLSK_ADDRESS"),
    ("soulseek.port", "SLSKD_SLSK_PORT"),
    ("soulseek.username", "SLSKD_SLSK_USERNAME"),
    ("soulseek.password", "SLSKD_SLSK_PASSWORD"),
    ("soulseek.description", "SLSKD_SLSK_DESCRIPTION"),
    ("soulseek.picture", "SLSKD_SLSK_PICTURE"),
    ("soulseek.diagnostic_level", "SLSKD_SLSK_DIAG_LEVEL"),
    (
        "soulseek.distributed_network.disabled",
        "SLSKD_SLSK_NO_DNET",
    ),
    (
        "soulseek.distributed_network.disable_children",
        "SLSKD_SLSK_DNET_NO_CHILDREN",
    ),
    (
        "soulseek.distributed_network.child_limit",
        "SLSKD_SLSK_DNET_CHILDREN",
    ),
    (
        "soulseek.distributed_network.logging",
        "SLSKD_SLSK_DNET_LOGGING",
    ),
    ("soulseek.listen_ip_address", "SLSKD_SLSK_LISTEN_IP_ADDRESS"),
    ("soulseek.listen_port", "SLSKD_SLSK_LISTEN_PORT"),
    ("soulseek.connection.buffer.read", "SLSKD_SLSK_READ_BUFFER"),
    (
        "soulseek.connection.buffer.write",
        "SLSKD_SLSK_WRITE_BUFFER",
    ),
    (
        "soulseek.connection.buffer.transfer",
        "SLSKD_SLSK_TRANSFER_BUFFER",
    ),
    (
        "soulseek.connection.buffer.write_queue",
        "SLSKD_SLSK_WRITE_QUEUE",
    ),
    (
        "soulseek.connection.timeout.connect",
        "SLSKD_SLSK_CONNECTION_TIMEOUT",
    ),
    (
        "soulseek.connection.timeout.inactivity",
        "SLSKD_SLSK_INACTIVITY_TIMEOUT",
    ),
    (
        "soulseek.connection.timeout.transfer",
        "SLSKD_SLSK_TRANSFER_TIMEOUT",
    ),
    (
        "soulseek.connection.proxy.enabled",
        "SLSKD_SLSK_PROXY_ENABLED",
    ),
    (
        "soulseek.connection.proxy.address",
        "SLSKD_SLSK_PROXY_ADDRESS",
    ),
    ("soulseek.connection.proxy.port", "SLSKD_SLSK_PROXY_PORT"),
    (
        "soulseek.connection.proxy.username",
        "SLSKD_SLSK_PROXY_USERNAME",
    ),
    (
        "soulseek.connection.proxy.password",
        "SLSKD_SLSK_PROXY_PASSWORD",
    ),
    ("soulseek.obfuscation.enabled", "SLSKD_SLSK_OBFUSCATION"),
    ("soulseek.obfuscation.mode", "SLSKD_SLSK_OBFUSCATION_MODE"),
    (
        "soulseek.obfuscation.listen_port",
        "SLSKD_SLSK_OBFUSCATION_LISTEN_PORT",
    ),
    (
        "soulseek.obfuscation.advertise_regular_port",
        "SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
    ),
    (
        "soulseek.obfuscation.prefer_outbound",
        "SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND",
    ),
    (
        "soulseek.private_message_auto_response.cooldown_minutes",
        "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
    ),
    (
        "soulseek.private_message_auto_response.enabled",
        "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
    ),
    (
        "soulseek.private_message_auto_response.message",
        "SLSKD_SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
    ),
    ("integrations.lidarr.api_key", "SLSKD_LIDARR_API_KEY"),
    ("integrations.lidarr.enabled", "SLSKD_LIDARR"),
    (
        "integrations.lidarr.timeout_seconds",
        "SLSKD_LIDARR_TIMEOUT",
    ),
    ("integrations.lidarr.url", "SLSKD_LIDARR_URL"),
    (
        "integrations.lidarr.sync_wanted_to_wishlist",
        "SLSKD_LIDARR_SYNC_WANTED",
    ),
    (
        "integrations.lidarr.sync_interval_seconds",
        "SLSKD_LIDARR_SYNC_INTERVAL",
    ),
    (
        "integrations.lidarr.max_items_per_sync",
        "SLSKD_LIDARR_SYNC_MAX_ITEMS",
    ),
    (
        "integrations.lidarr.auto_download",
        "SLSKD_LIDARR_AUTO_DOWNLOAD",
    ),
    (
        "integrations.lidarr.wishlist_filter",
        "SLSKD_LIDARR_WISHLIST_FILTER",
    ),
    (
        "integrations.lidarr.wishlist_max_results",
        "SLSKD_LIDARR_WISHLIST_MAX_RESULTS",
    ),
    (
        "integrations.lidarr.auto_import_completed",
        "SLSKD_LIDARR_AUTO_IMPORT_COMPLETED",
    ),
    (
        "integrations.lidarr.import_path_from",
        "SLSKD_LIDARR_IMPORT_PATH_FROM",
    ),
    (
        "integrations.lidarr.import_path_to",
        "SLSKD_LIDARR_IMPORT_PATH_TO",
    ),
    (
        "integrations.lidarr.import_mode",
        "SLSKD_LIDARR_IMPORT_MODE",
    ),
    (
        "integrations.lidarr.import_replace_existing_files",
        "SLSKD_LIDARR_IMPORT_REPLACE_EXISTING",
    ),
    ("integrations.spotify.client_id", "SLSKD_SPOTIFY_CLIENT_ID"),
    (
        "integrations.spotify.client_secret",
        "SLSKD_SPOTIFY_CLIENT_SECRET",
    ),
    ("integrations.spotify.enabled", "SLSKD_SPOTIFY"),
    ("integrations.spotify.market", "SLSKD_SPOTIFY_MARKET"),
    (
        "integrations.spotify.redirect_uri",
        "SLSKD_SPOTIFY_REDIRECT_URI",
    ),
    (
        "integrations.spotify.timeout_seconds",
        "SLSKD_SPOTIFY_TIMEOUT",
    ),
    (
        "integrations.spotify.max_items_per_import",
        "SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT",
    ),
    ("integrations.youtube.enabled", "SLSKD_YOUTUBE"),
    ("integrations.youtube.api_key", "SLSKD_YOUTUBE_API_KEY"),
    ("integrations.lastfm.enabled", "SLSKD_LASTFM"),
    ("integrations.lastfm.api_key", "SLSKD_LASTFM_API_KEY"),
    ("integrations.ntfy.enabled", "SLSKD_NTFY"),
    ("integrations.ntfy.url", "SLSKD_NTFY_URL"),
    ("integrations.ntfy.access_token", "SLSKD_NTFY_TOKEN"),
    (
        "integrations.ntfy.notification_prefix",
        "SLSKD_NTFY_NOTIFICATION_PREFIX",
    ),
    (
        "integrations.ntfy.notify_on_private_message",
        "SLSKD_NTFY_NOTIFY_ON_PRIVATE_MESSAGE",
    ),
    (
        "integrations.ntfy.notify_on_room_mention",
        "SLSKD_NTFY_NOTIFY_ON_ROOM_MENTION",
    ),
    ("integrations.pushover.enabled", "SLSKD_PUSHOVER"),
    ("integrations.pushover.user_key", "SLSKD_PUSHOVER_USER_KEY"),
    ("integrations.pushover.token", "SLSKD_PUSHOVER_TOKEN"),
    (
        "integrations.pushover.notification_prefix",
        "SLSKD_PUSHOVER_NOTIFICATION_PREFIX",
    ),
    (
        "integrations.pushover.notify_on_private_message",
        "SLSKD_PUSHOVER_NOTIFY_ON_PRIVATE_MESSAGE",
    ),
    (
        "integrations.pushover.notify_on_room_mention",
        "SLSKD_PUSHOVER_NOTIFY_ON_ROOM_MENTION",
    ),
    ("integrations.pushbullet.enabled", "SLSKD_PUSHBULLET"),
    (
        "integrations.pushbullet.access_token",
        "SLSKD_PUSHBULLET_ACCESS_TOKEN",
    ),
    (
        "integrations.pushbullet.notification_prefix",
        "SLSKD_PUSHBULLET_NOTIFICATION_PREFIX",
    ),
    (
        "integrations.pushbullet.notify_on_private_message",
        "SLSKD_PUSHBULLET_NOTIFY_ON_PRIVATE_MESSAGE",
    ),
    (
        "integrations.pushbullet.notify_on_room_mention",
        "SLSKD_PUSHBULLET_NOTIFY_ON_ROOM_MENTION",
    ),
    (
        "integrations.pushbullet.retry_attempts",
        "SLSKD_PUSHBULLET_RETRY_ATTEMPTS",
    ),
    (
        "integrations.pushbullet.cooldown_time",
        "SLSKD_PUSHBULLET_COOLDOWN_TIME",
    ),
    ("integrations.ftp.enabled", "SLSKD_FTP"),
    ("integrations.ftp.address", "SLSKD_FTP_ADDRESS"),
    ("integrations.ftp.port", "SLSKD_FTP_PORT"),
    (
        "integrations.ftp.encryption_mode",
        "SLSKD_FTP_ENCRYPTION_MODE",
    ),
    (
        "integrations.ftp.ignore_certificate_errors",
        "SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS",
    ),
    ("integrations.ftp.username", "SLSKD_FTP_USERNAME"),
    ("integrations.ftp.password", "SLSKD_FTP_PASSWORD"),
    ("integrations.ftp.remote_path", "SLSKD_FTP_REMOTE_PATH"),
    (
        "integrations.ftp.overwrite_existing",
        "SLSKD_FTP_OVERWRITE_EXISTING",
    ),
    (
        "integrations.ftp.connection_timeout",
        "SLSKD_FTP_CONNECTION_TIMEOUT",
    ),
    (
        "integrations.ftp.retry_attempts",
        "SLSKD_FTP_RETRY_ATTEMPTS",
    ),
    ("integrations.vpn.enabled", "SLSKD_VPN"),
    (
        "integrations.vpn.port_forwarding",
        "SLSKD_VPN_PORT_FORWARDING",
    ),
    (
        "integrations.vpn.polling_interval",
        "SLSKD_VPN_POLLING_INTERVAL",
    ),
    ("integrations.vpn.gluetun.url", "SLSKD_VPN_GLUETUN_URL"),
    (
        "integrations.vpn.gluetun.timeout",
        "SLSKD_VPN_GLUETUN_TIMEOUT",
    ),
    (
        "integrations.vpn.gluetun.username",
        "SLSKD_VPN_GLUETUN_USERNAME",
    ),
    (
        "integrations.vpn.gluetun.password",
        "SLSKD_VPN_GLUETUN_PASSWORD",
    ),
    (
        "integrations.vpn.gluetun.api_key",
        "SLSKD_VPN_GLUETUN_API_KEY",
    ),
    ("transfers.download.slots", "SLSKD_DOWNLOAD_SLOTS"),
    (
        "transfers.download.speed_limit",
        "SLSKD_DOWNLOAD_SPEED_LIMIT",
    ),
    (
        "transfers.download.completed_layout",
        "SLSKD_DOWNLOAD_COMPLETED_LAYOUT",
    ),
    (
        "transfers.download.auto_replace_stuck",
        "SLSKD_AUTO_REPLACE_STUCK",
    ),
    (
        "transfers.download.auto_replace_threshold",
        "SLSKD_AUTO_REPLACE_THRESHOLD",
    ),
    (
        "transfers.download.auto_replace_interval",
        "SLSKD_AUTO_REPLACE_INTERVAL",
    ),
    (
        "transfers.download.completed_path_template",
        "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
    ),
    (
        "transfers.download.auto_retry.alternate_source_size_tolerance_percent",
        "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
    ),
    (
        "transfers.download.auto_retry.alternate_sources_enabled",
        "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCES_ENABLED",
    ),
    (
        "transfers.download.auto_retry.check_interval_seconds",
        "SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS",
    ),
    (
        "transfers.download.auto_retry.enabled",
        "SLSKR_TRANSFER_AUTO_RETRY_ENABLED",
    ),
    (
        "transfers.download.auto_retry.max_alternate_source_searches_per_cycle",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
    ),
    (
        "transfers.download.auto_retry.max_attempts",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS",
    ),
    (
        "transfers.download.auto_retry.max_files_per_cycle",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE",
    ),
    (
        "transfers.download.auto_retry.max_files_per_peer_per_cycle",
        "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
    ),
    (
        "transfers.download.auto_retry.peer_cooldown_seconds",
        "SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS",
    ),
    (
        "transfers.download.auto_retry.retry_delay_seconds",
        "SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS",
    ),
    ("web.ip_address", "SLSKD_HTTP_IP_ADDRESS"),
    ("web.address", "SLSKD_HTTP_ADDRESS"),
    ("web.port", "SLSKD_HTTP_PORT"),
];

fn controller_yaml_environment(
    path: &std::path::Path,
    target: ControllerCompatibilityTarget,
) -> Result<BTreeMap<String, String>, String> {
    use std::io::Read;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(BTreeMap::new());
        }
        Err(error) => return Err(format!("failed to inspect controller YAML: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("controller YAML must be a regular file".to_owned());
    }
    if metadata.len() > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "controller YAML is too large: {} bytes, max is {MAX_CONFIG_FILE_BYTES}",
            metadata.len()
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("failed to read controller YAML: {error}"))?;
    let mut body = String::new();
    file.by_ref()
        .take(MAX_CONFIG_FILE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("failed to read controller YAML: {error}"))?;
    if body.len() as u64 > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "controller YAML is too large: max is {MAX_CONFIG_FILE_BYTES} bytes"
        ));
    }
    let root = serde_yaml::from_str::<serde_yaml::Value>(&body)
        .map_err(|_| "invalid controller YAML".to_owned())?;
    let mut nodes = 0;
    validate_controller_yaml_shape(&root, 0, &mut nodes)?;
    if !matches!(
        root,
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Null
    ) {
        return Err("controller YAML root must be a mapping".to_owned());
    }

    let mut values = BTreeMap::new();
    if target == ControllerCompatibilityTarget::Slskdn {
        values.insert(
            "SLSKR_ADVANCED_NETWORKING_JSON".to_owned(),
            serde_json::to_string(&root)
                .map_err(|_| "invalid advanced networking controller YAML".to_owned())?,
        );
    }
    for (yaml_path, environment_name) in CONTROLLER_YAML_CORE_MAPPINGS {
        if matches!(
            *environment_name,
            "SLSKD_BLACKLISTED_MEMBERS" | "SLSKD_BLACKLISTED_PATTERNS" | "SLSKD_BLACKLISTED_CIDRS"
        ) {
            let target_path_matches = match target {
                ControllerCompatibilityTarget::Slskd => {
                    yaml_path.starts_with("transfers.groups.blacklisted.")
                }
                ControllerCompatibilityTarget::Slskdn => true,
            };
            if !target_path_matches {
                continue;
            }
        }
        let Some(value) = controller_yaml_value(&root, yaml_path) else {
            continue;
        };
        let value = match value {
            serde_yaml::Value::Null => continue,
            serde_yaml::Value::Bool(value) => value.to_string(),
            serde_yaml::Value::Number(value) => value.to_string(),
            serde_yaml::Value::String(value)
                if *environment_name == "SLSKD_INSTANCE_NAME" && value.is_empty() =>
            {
                continue;
            }
            serde_yaml::Value::String(value) => value.clone(),
            serde_yaml::Value::Sequence(values)
                if matches!(
                    *environment_name,
                    "SLSKD_SHARED_DIR"
                        | "SLSKD_SHARE_FILTER"
                        | "SLSKD_SEARCH_REQUEST_FILTER"
                        | "SLSKD_BLACKLISTED_MEMBERS"
                        | "SLSKD_BLACKLISTED_PATTERNS"
                        | "SLSKD_BLACKLISTED_CIDRS"
                        | "SLSKD_WEB_CORS_ALLOWED_ORIGINS"
                        | "SLSKD_WEB_CORS_ALLOWED_HEADERS"
                        | "SLSKD_WEB_CORS_ALLOWED_METHODS"
                        | "SLSKD_ROOMS"
                        | "SLSKD_SLSK_LIKED_INTERESTS"
                        | "SLSKD_SLSK_HATED_INTERESTS"
                ) =>
            {
                values
                    .iter()
                    .map(|value| {
                        if let Some(value) = value.as_str() {
                            return Ok(value.to_owned());
                        }
                        if matches!(
                            *environment_name,
                            "SLSKD_WEB_CORS_ALLOWED_ORIGINS"
                                | "SLSKD_WEB_CORS_ALLOWED_HEADERS"
                                | "SLSKD_WEB_CORS_ALLOWED_METHODS"
                        ) {
                            match value {
                                serde_yaml::Value::Bool(value) => return Ok(value.to_string()),
                                serde_yaml::Value::Number(value) => return Ok(value.to_string()),
                                serde_yaml::Value::Null => return Ok(String::new()),
                                _ => {}
                            }
                        }
                        Err(format!(
                            "invalid controller YAML value for {yaml_path}: expected string array"
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .join(";")
            }
            _ => {
                return Err(format!(
                    "invalid controller YAML value for {yaml_path}: expected scalar"
                ));
            }
        };
        values.insert((*environment_name).to_owned(), value);
    }
    if let Some(webhooks) = controller_yaml_value(&root, "integrations.webhooks") {
        let json = serde_json::to_string(webhooks)
            .map_err(|_| "invalid integrations.webhooks configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_WEBHOOKS_JSON".to_owned(), json);
    }
    if let Some(api_keys) = controller_yaml_value(&root, "web.authentication.api_keys") {
        let json = serde_json::to_string(api_keys)
            .map_err(|_| "invalid web.authentication.api_keys configuration".to_owned())?;
        values.insert("SLSKD_API_KEYS_JSON".to_owned(), json);
    }
    if let Some(destinations) = controller_yaml_value(&root, "destinations.folders") {
        let json = serde_json::to_string(destinations)
            .map_err(|_| "invalid destinations.folders configuration".to_owned())?;
        values.insert("SLSKD_DESTINATIONS_JSON".to_owned(), json);
    }
    if let Some(scripts) = controller_yaml_value(&root, "integrations.scripts") {
        let json = serde_json::to_string(scripts)
            .map_err(|_| "invalid integrations.scripts configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_SCRIPTS_JSON".to_owned(), json);
    }
    let groups = controller_yaml_value(&root, "transfers.groups").or_else(|| {
        (target == ControllerCompatibilityTarget::Slskdn)
            .then(|| controller_yaml_value(&root, "groups"))
            .flatten()
    });
    if let Some(groups) = groups {
        let json = serde_json::to_string(groups)
            .map_err(|_| "invalid transfers.groups configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_TRANSFER_GROUPS_JSON".to_owned(), json);
    }
    if let Some(upload) = controller_yaml_value(&root, "transfers.upload") {
        let json = serde_json::to_string(upload)
            .map_err(|_| "invalid transfers.upload configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_TRANSFER_UPLOAD_JSON".to_owned(), json);
    }
    if let Some(download) = controller_yaml_value(&root, "transfers.download") {
        let json = serde_json::to_string(download)
            .map_err(|_| "invalid transfers.download configuration".to_owned())?;
        values.insert("SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON".to_owned(), json);
    }
    Ok(values)
}

fn controller_yaml_value<'a>(
    root: &'a serde_yaml::Value,
    path: &str,
) -> Option<&'a serde_yaml::Value> {
    let mut current = root;
    for key in path.split('.') {
        let mapping = current.as_mapping()?;
        current = mapping.get(serde_yaml::Value::String(key.to_owned()))?;
    }
    Some(current)
}

fn validate_controller_yaml_shape(
    value: &serde_yaml::Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), String> {
    if depth > 64 {
        return Err("controller YAML exceeds maximum nesting depth".to_owned());
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > 65_536 {
        return Err("controller YAML contains too many values".to_owned());
    }
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, child) in mapping {
                if !matches!(key, serde_yaml::Value::String(_)) {
                    return Err("controller YAML keys must be strings".to_owned());
                }
                validate_controller_yaml_shape(child, depth + 1, nodes)?;
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for child in sequence {
                validate_controller_yaml_shape(child, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn optional_env_any(env: &dyn ConfigEnv, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env.var(name))
}

fn controller_string_array_layer<E: ConfigEnv>(
    env: &E,
    name: &str,
    file_value: Vec<String>,
) -> Vec<String> {
    env.var(name).map_or(file_value, |value| {
        if value.is_empty() {
            Vec::new()
        } else {
            value.split(';').map(str::to_owned).collect()
        }
    })
}

fn normalized_controller_values(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.to_ascii_lowercase()))
        .collect()
}

fn trusted_proxy_cidrs_from_layers(
    env_value: Option<String>,
    file_value: Vec<String>,
) -> Result<Vec<TrustedProxyCidr>, String> {
    let values = match env_value {
        Some(value) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        None => file_value,
    };
    values
        .into_iter()
        .map(|value| TrustedProxyCidr::parse(&value))
        .collect()
}

fn controller_passthrough_cidrs(value: Option<&str>) -> Vec<TrustedProxyCidr> {
    value
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter_map(|value| {
            let normalized = if value.contains('/') {
                value.to_owned()
            } else if value.contains(':') {
                format!("{value}/128")
            } else {
                format!("{value}/32")
            };
            TrustedProxyCidr::parse(&normalized).ok()
        })
        .collect()
}

fn trusted_mesh_peers_from_layers(
    env_value: Option<String>,
    file_value: Vec<TrustedMeshPeerInput>,
) -> Result<Vec<TrustedMeshPeer>, String> {
    let values = match env_value {
        Some(value) => serde_json::from_str::<Vec<TrustedMeshPeerInput>>(&value)
            .map_err(|error| format!("invalid SLSKR_TRUSTED_MESH_PEERS JSON: {error}"))?,
        None => file_value,
    };
    if values.len() > MAX_TRUSTED_MESH_PEERS {
        return Err(format!(
            "trusted mesh peer count exceeds {MAX_TRUSTED_MESH_PEERS}"
        ));
    }

    let mut peers = Vec::with_capacity(values.len());
    for value in values {
        let peer_id = bounded_mesh_identity(&value.peer_id, "peer_id")?;
        let username = bounded_mesh_identity(&value.username, "username")?;
        if peers.iter().any(|peer: &TrustedMeshPeer| {
            peer.peer_id.eq_ignore_ascii_case(&peer_id)
                || peer.username.eq_ignore_ascii_case(&username)
                || peer.peer_id.eq_ignore_ascii_case(&username)
                || peer.username.eq_ignore_ascii_case(&peer_id)
        }) {
            return Err(format!(
                "trusted mesh peer identity {peer_id:?}/{username:?} is duplicated"
            ));
        }
        let overlay_endpoint = value
            .overlay_endpoint
            .trim()
            .parse::<SocketAddr>()
            .map_err(|error| format!("trusted mesh overlay endpoint is invalid: {error}"))?;
        if overlay_endpoint.port() == 0 {
            return Err("trusted mesh overlay endpoint port must be non-zero".to_owned());
        }
        if overlay_endpoint.ip().is_unspecified() || overlay_endpoint.ip().is_multicast() {
            return Err(
                "trusted mesh overlay endpoint must be a unicast destination address".to_owned(),
            );
        }
        let certificate_sha256 = decode_mesh_certificate_pin(&value.certificate_sha256)?;
        let range_endpoint = value
            .range_endpoint
            .as_deref()
            .map(validate_mesh_range_endpoint)
            .transpose()?;
        peers.push(TrustedMeshPeer {
            peer_id,
            username,
            overlay_endpoint,
            certificate_sha256,
            range_endpoint,
        });
    }
    Ok(peers)
}

fn bounded_mesh_identity(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("trusted mesh peer {field} is required"));
    }
    if value.len() > MAX_MESH_IDENTITY_BYTES {
        return Err(format!(
            "trusted mesh peer {field} exceeds {MAX_MESH_IDENTITY_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(format!(
            "trusted mesh peer {field} contains a control character"
        ));
    }
    Ok(value.to_owned())
}

fn decode_mesh_certificate_pin(value: &str) -> Result<[u8; 32], String> {
    let value = value.trim();
    if value.len() != 64 {
        return Err(
            "trusted mesh certificate_sha256 must contain exactly 64 hex digits".to_owned(),
        );
    }
    let bytes = hex::decode(value)
        .map_err(|_| "trusted mesh certificate_sha256 must be hexadecimal".to_owned())?;
    let pin: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "trusted mesh certificate_sha256 must contain 32 bytes".to_owned())?;
    if pin.iter().all(|byte| *byte == 0) {
        return Err("trusted mesh certificate_sha256 must not be all zeroes".to_owned());
    }
    Ok(pin)
}

fn validate_mesh_range_endpoint(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("trusted mesh range_endpoint must not be blank".to_owned());
    }
    if value.len() > MAX_MESH_RANGE_ENDPOINT_BYTES {
        return Err(format!(
            "trusted mesh range_endpoint exceeds {MAX_MESH_RANGE_ENDPOINT_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err("trusted mesh range_endpoint contains a control character".to_owned());
    }
    let scheme_end = value
        .find("://")
        .ok_or_else(|| "trusted mesh range_endpoint is missing an authority".to_owned())?;
    let path_start = value[scheme_end + 3..]
        .find('/')
        .map_or(value.len(), |offset| scheme_end + 3 + offset);
    if value[..path_start].contains(['{', '}']) {
        return Err(
            "trusted mesh range_endpoint placeholders are allowed only in the path".to_owned(),
        );
    }
    let parseable = value
        .replace("{sha256}", &"0".repeat(64))
        .replace("{size}", "1")
        .replace("{recordingId}", "recording-id");
    if parseable.contains(['{', '}']) {
        return Err("trusted mesh range_endpoint contains an unknown placeholder".to_owned());
    }
    let url = reqwest::Url::parse(&parseable)
        .map_err(|error| format!("trusted mesh range_endpoint is invalid: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("trusted mesh range_endpoint must use http or https".to_owned());
    }
    if url.host_str().is_none() {
        return Err("trusted mesh range_endpoint must include a host".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("trusted mesh range_endpoint must not contain embedded credentials".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("trusted mesh range_endpoint must not contain a query or fragment".to_owned());
    }
    Ok(value.to_owned())
}

fn parse_user_endpoint_overrides(
    value: Option<String>,
) -> Result<BTreeMap<String, SocketAddr>, String> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let mut overrides = BTreeMap::new();
    for entry in value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (username, endpoint) = entry.split_once('=').ok_or_else(|| {
            format!(
                "invalid SLSKR_TEST_USER_ENDPOINT_OVERRIDES entry {entry:?}; expected user=host:port"
            )
        })?;
        let username = username.trim();
        if username.is_empty() {
            return Err("SLSKR_TEST_USER_ENDPOINT_OVERRIDES contains an empty username".to_owned());
        }
        let endpoint = endpoint
            .trim()
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid endpoint override for {username}: {error}"))?;
        overrides.insert(username.to_owned(), endpoint);
    }
    Ok(overrides)
}

fn env_parse_layer<E, T>(
    env: &E,
    name: &str,
    file_value: Option<T>,
    default: T,
) -> Result<T, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env.var(name) {
        Some(value) => value
            .parse::<T>()
            .map_err(|error| format!("invalid {name}: {error}")),
        None => Ok(file_value.unwrap_or(default)),
    }
}

fn env_parse_any_layer<E, T>(
    env: &E,
    names: &[&str],
    file_value: Option<T>,
    default: T,
) -> Result<T, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some((name, value)) = names
        .iter()
        .find_map(|name| env.var(name).map(|value| (*name, value)))
    else {
        return Ok(file_value.unwrap_or(default));
    };
    value
        .parse::<T>()
        .map_err(|error| format!("invalid {name}: {error}"))
}

fn env_parse_any_option<E, T>(env: &E, names: &[&str]) -> Result<Option<T>, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some((name, value)) = names
        .iter()
        .find_map(|name| env.var(name).map(|value| (*name, value)))
    else {
        return Ok(None);
    };
    value
        .parse::<T>()
        .map(Some)
        .map_err(|error| format!("invalid {name}: {error}"))
}

fn split_server_address(value: &str) -> Result<(String, u16), String> {
    let value = value.trim();
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, suffix) = rest
            .split_once(']')
            .ok_or_else(|| "invalid Soulseek server address: missing closing bracket".to_owned())?;
        let port = suffix.strip_prefix(':').ok_or_else(|| {
            "invalid Soulseek server address: missing port after bracket".to_owned()
        })?;
        (host, port)
    } else {
        value
            .rsplit_once(':')
            .ok_or_else(|| "invalid Soulseek server address: expected host:port".to_owned())?
    };
    if host.trim().is_empty() {
        return Err("invalid Soulseek server address: host is empty".to_owned());
    }
    let port = port
        .parse::<u16>()
        .map_err(|error| format!("invalid Soulseek server port: {error}"))?;
    if port == 0 {
        return Err("invalid Soulseek server port: must be between 1 and 65535".to_owned());
    }
    Ok((host.trim().to_owned(), port))
}

fn format_host_port(host: &str, port: u16) -> String {
    let host = host.trim().trim_matches(['[', ']']);
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn env_parse_option_layer<E, T>(
    env: &E,
    name: &str,
    file_value: Option<T>,
) -> Result<Option<T>, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env.var(name) {
        Some(value) => value
            .parse::<T>()
            .map(Some)
            .map_err(|error| format!("invalid {name}: {error}")),
        None => Ok(file_value),
    }
}

fn env_bool_layer<E: ConfigEnv>(env: &E, name: &str, default: bool) -> Result<bool, String> {
    match env.var(name) {
        Some(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(format!("invalid {name}: expected boolean")),
        },
        None => Ok(default),
    }
}

fn env_bool_any_layer<E: ConfigEnv>(
    env: &E,
    names: &[&str],
    default: bool,
) -> Result<bool, String> {
    let Some((name, value)) = names
        .iter()
        .find_map(|name| env.var(name).map(|value| (*name, value)))
    else {
        return Ok(default);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid {name}: expected boolean")),
    }
}

pub fn redact_username(username: &str) -> String {
    if username.len() <= 2 {
        return "**".to_owned();
    }
    let first = username.chars().next().unwrap_or('*');
    let last = username.chars().last().unwrap_or('*');
    format!("{first}***{last}")
}

pub fn json_option(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_owned())
}

pub fn json_bool_option(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            ch if ch <= '\u{1f}' => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

pub fn json_u32_option(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub fn json_u64_option(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub fn json_usize_option(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub fn parse_share_entries(value: &str) -> Result<Vec<FileEntry>, String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(parse_share_entry)
        .collect()
}

pub fn parse_share_entry(value: &str) -> Result<FileEntry, String> {
    let (filename, size) = value
        .rsplit_once('=')
        .ok_or_else(|| "SLSKR_SHARE_FIXTURE entries must be path=size".to_owned())?;
    let size = size
        .parse::<u64>()
        .map_err(|error| format!("invalid SLSKR_SHARE_FIXTURE size: {error}"))?;
    Ok(FileEntry {
        code: 1,
        filename: filename.trim().replace('\\', "/"),
        filename_encoding: slskr_client::protocol::ProtocolTextEncoding::Utf8,
        size,
        extension: extension_for(filename.trim()),
        extension_encoding: slskr_client::protocol::ProtocolTextEncoding::Utf8,
        attributes: Vec::new(),
    })
}

pub fn parse_share_directories(value: &str) -> Result<Vec<ShareDirectory>, String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ShareDirectory::parse)
        .collect()
}

fn extension_for(filename: &str) -> String {
    filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Clone, Default)]
    struct MapEnv {
        values: BTreeMap<String, String>,
    }

    impl MapEnv {
        fn with(mut self, name: &str, value: &str) -> Self {
            self.values.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl super::ConfigEnv for MapEnv {
        fn var(&self, name: &str) -> Option<String> {
            self.values.get(name).cloned()
        }
    }

    #[test]
    fn controller_web_max_request_body_size_matches_slskdn_layers_and_bounds() {
        let root = std::env::temp_dir().join(format!(
            "slskr-web-body-limit-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "web:\n  max_request_body_size: 7340032\n",
        )
        .unwrap();

        let yaml = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKD_WEB_MAX_REQUEST_BODY_SIZE", "6291456"),
        )
        .unwrap();
        assert_eq!(yaml.controller_web_max_request_body_size, 7 * 1024 * 1024);
        std::fs::remove_dir_all(root).unwrap();

        let slskdn =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .unwrap();
        assert_eq!(
            slskdn.controller_web_max_request_body_size,
            10 * 1024 * 1024
        );
        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_AUTH_DISABLED", "true"),
        )
        .unwrap();
        assert_eq!(
            slskd.controller_web_max_request_body_size,
            crate::http_server::BODY_SIZE_LIMIT
        );

        for invalid in ["0", "-1", "2147483648"] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSKD_WEB_MAX_REQUEST_BODY_SIZE", invalid),
            )
            .expect_err("out-of-range request body limit must fail startup");
            assert!(error.contains("web.max_request_body_size"), "{error}");
        }
    }

    #[test]
    fn controller_web_cors_reads_frozen_yaml_and_defaults() {
        let root = std::env::temp_dir().join(format!(
            "slskr-web-cors-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "web:\n  cors:\n    enabled: true\n    allow_credentials: true\n    allowed_origins: [https://one.example, https://two.example]\n    allowed_headers: [X-One, X-Two]\n    allowed_methods: [GET, POST]\n",
        )
        .unwrap();

        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKD_WEB_CORS_ENABLED", "false")
                .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://ignored.example"),
        )
        .unwrap();
        assert_eq!(
            config.controller_web_cors,
            super::ControllerWebCorsSettings {
                enabled: true,
                allow_credentials: true,
                allowed_origins: vec![
                    "https://one.example".to_owned(),
                    "https://two.example".to_owned(),
                ],
                allowed_headers: vec!["X-One".to_owned(), "X-Two".to_owned()],
                allowed_methods: vec!["GET".to_owned(), "POST".to_owned()],
            }
        );
        std::fs::remove_dir_all(root).unwrap();

        let defaults =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .unwrap();
        assert_eq!(
            defaults.controller_web_cors,
            super::ControllerWebCorsSettings::default()
        );

        let unsafe_cors = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_ENFORCE_SECURITY", "true")
                .with("SLSKD_WEB_CORS_ENABLED", "true")
                .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
                .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "*"),
        )
        .unwrap();
        let error = unsafe_cors
            .validate_controller_startup_hardening()
            .expect_err("enforced credentialed wildcard CORS must fail startup");
        assert!(error.contains("CorsCredentialsWithWildcard"), "{error}");

        let enforced_explicit = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_ENFORCE_SECURITY", "true")
                .with("SLSKD_WEB_CORS_ENABLED", "true")
                .with("SLSKD_WEB_CORS_ALLOW_CREDENTIALS", "true")
                .with("SLSKD_WEB_CORS_ALLOWED_ORIGINS", "https://allowed.example"),
        )
        .expect("explicit credentialed CORS is valid under enforcement");
        assert!(enforced_explicit.controller_web_enforce_security);
    }

    #[test]
    fn controller_no_auth_passthrough_reads_yaml_and_enforces_remote_cidrs() {
        let root = std::env::temp_dir().join(format!(
            "slskr-web-passthrough-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "web:\n  allow_remote_no_auth: true\n  authentication:\n    disabled: true\n    passthrough:\n      allowed_cidrs: 192.0.2.0/24,invalid\n",
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "false"),
        )
        .unwrap();
        assert!(!config.auth_required);
        assert!(config.controller_web_allow_remote_no_auth);
        assert_eq!(
            config.controller_web_passthrough_allowed_cidrs.as_deref(),
            Some("192.0.2.0/24,invalid")
        );
        assert!(config.controller_passthrough_allows(Some("127.0.0.1:1".parse().unwrap())));
        assert!(config.controller_passthrough_allows(Some("192.0.2.44:1".parse().unwrap())));
        assert!(!config.controller_passthrough_allows(Some("198.51.100.1:1".parse().unwrap())));
        assert!(!config.controller_passthrough_allows(None));
        std::fs::remove_dir_all(root).unwrap();

        let nonloopback_config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_HTTP_BIND", "0.0.0.0:5030")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_ENFORCE_SECURITY", "true"),
        )
        .unwrap();
        let nonloopback = nonloopback_config
            .validate_controller_startup_hardening()
            .expect_err("enforced non-loopback no-auth bind must fail");
        assert!(
            nonloopback.contains("AuthDisabledNonLoopback"),
            "{nonloopback}"
        );

        let missing_cidrs_config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_ENFORCE_SECURITY", "true")
                .with("SLSKD_ALLOW_REMOTE_NO_AUTH", "true"),
        )
        .unwrap();
        let missing_cidrs = missing_cidrs_config
            .validate_controller_startup_hardening()
            .expect_err("enforced remote no-auth without CIDRs must fail");
        assert!(
            missing_cidrs.contains("RemoteNoAuthWithoutCidrs"),
            "{missing_cidrs}"
        );
    }

    #[test]
    fn controller_diagnostics_dump_reads_yaml_and_enforces_no_auth_hardening() {
        let root = std::env::temp_dir().join(format!(
            "slskr-diagnostics-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "diagnostics:\n  allow_memory_dump: true\n  allow_remote_dump: true\n",
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .unwrap();
        assert!(config.controller_diagnostics_allow_memory_dump);
        assert!(config.controller_diagnostics_allow_remote_dump);
        let environment_override = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_ALLOW_MEMORY_DUMP", "false")
                .with("SLSKD_ALLOW_REMOTE_DUMP", "false"),
        )
        .unwrap();
        assert!(environment_override.controller_diagnostics_allow_memory_dump);
        assert!(environment_override.controller_diagnostics_allow_remote_dump);
        std::fs::remove_file(root.join("slskd.yml")).unwrap();
        let environment_only = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_ALLOW_MEMORY_DUMP", "true")
                .with("SLSKD_ALLOW_REMOTE_DUMP", "true"),
        )
        .unwrap();
        assert!(environment_only.controller_diagnostics_allow_memory_dump);
        assert!(environment_only.controller_diagnostics_allow_remote_dump);
        std::fs::remove_dir_all(root).unwrap();

        let unsafe_dump = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_ENFORCE_SECURITY", "true")
                .with("SLSKD_ALLOW_MEMORY_DUMP", "true"),
        )
        .unwrap();
        let error = unsafe_dump
            .validate_controller_startup_hardening()
            .expect_err("enforced memory dump with disabled authentication must fail");
        assert!(error.contains("MemoryDumpWithAuthDisabled"), "{error}");
    }

    #[test]
    fn controller_remaining_hardening_rules_match_frozen_startup_policy() {
        let weak_metrics = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_ENFORCE_SECURITY", "true")
                .with("SLSKD_METRICS", "true")
                .with("SLSKD_METRICS_USERNAME", "slskd")
                .with("SLSKD_METRICS_PASSWORD", " "),
        )
        .expect_err("whitespace metrics password must fail options validation");
        assert!(
            weak_metrics.contains("metrics authentication password must be configured"),
            "{weak_metrics}"
        );

        let root = std::env::temp_dir().join(format!(
            "slskr-hash-from-audio-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "flags:\n  hash_from_audio_file_enabled: true\n",
        )
        .unwrap();
        let hash_from_audio = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .unwrap();
        assert!(hash_from_audio.controller_hash_from_audio_file_enabled);
        let error = hash_from_audio
            .validate_controller_startup_hardening()
            .expect_err("unsupported audio hash flag must always fail startup");
        assert!(error.contains("HashFromAudioFileEnabled"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controller_web_rate_limiting_reads_frozen_yaml_and_profile_defaults() {
        let root = std::env::temp_dir().join(format!(
            "slskr-web-rate-limit-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "web:\n  rate_limiting:\n    enabled: false\n    api_permit_limit: 201\n    api_window_seconds: 0\n    federation_permit_limit: 31\n    federation_window_seconds: 61\n    mesh_gateway_permit_limit: 62\n    mesh_gateway_window_seconds: 63\n",
        )
        .unwrap();

        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKD_WEB_API_PERMIT_LIMIT", "202"),
        )
        .unwrap();
        assert_eq!(
            config.controller_web_rate_limiting,
            super::ControllerWebRateLimitingSettings {
                enabled: false,
                api_permit_limit: 201,
                api_window_seconds: 0,
                federation_permit_limit: 31,
                federation_window_seconds: 61,
                mesh_gateway_permit_limit: 62,
                mesh_gateway_window_seconds: 63,
            }
        );
        std::fs::remove_dir_all(root).unwrap();

        let slskdn =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .unwrap();
        assert_eq!(slskdn.controller_web_rate_limiting.api_permit_limit, 200);
        assert!(slskdn.controller_web_rate_limiting.enabled);
        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_AUTH_DISABLED", "true"),
        )
        .unwrap();
        assert!(!slskd.controller_web_rate_limiting.enabled);

        let disabled_negative = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_WEB_RATE_LIMITING", "false")
                .with("SLSKD_WEB_API_PERMIT_LIMIT", "-1")
                .with("SLSKD_WEB_API_WINDOW_SECONDS", "-2"),
        )
        .unwrap();
        assert_eq!(
            disabled_negative
                .controller_web_rate_limiting
                .api_permit_limit,
            -1
        );
        assert_eq!(
            disabled_negative
                .controller_web_rate_limiting
                .api_window_seconds,
            -2
        );

        let enabled_zero = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_WEB_API_PERMIT_LIMIT", "0"),
        )
        .expect("frozen slskdN accepts zero permit limits until first policy use");
        assert_eq!(
            enabled_zero.controller_web_rate_limiting.api_permit_limit,
            0
        );
    }

    #[test]
    fn controller_surfaces_accept_dotnet_backtracking_regex_syntax() {
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_SHARE_FILTER", r"(?<=/)private(?=\.flac$)")
                .with("SLSKD_SEARCH_REQUEST_FILTER", r"^(secret)\1$")
                .with("SLSKD_BLACKLISTED_PATTERNS", r"^(?<stem>blocked)\k<stem>$"),
        )
        .expect("valid .NET lookaround and backreference syntax");

        assert_eq!(
            config.share_settings.filters,
            vec![r"(?<=/)private(?=\.flac$)"]
        );
        assert_eq!(
            config.controller_search_request_filters,
            vec![r"^(secret)\1$"]
        );
        assert_eq!(
            config.managed_blacklist.patterns,
            vec![r"^(?<stem>blocked)\k<stem>$"]
        );
    }

    #[test]
    fn frozen_targets_read_blacklisted_groups_from_their_distinct_yaml_paths() {
        for (target, expected) in [
            ("slskd", "^transfers-path$"),
            ("slskdn", "^top-level-path$"),
        ] {
            let root = std::env::temp_dir().join(format!(
                "slskr-blacklist-yaml-path-{target}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join("slskd.yml"),
                "transfers:\n  groups:\n    blacklisted:\n      patterns: ['^transfers-path$']\ngroups:\n  blacklisted:\n    patterns: ['^top-level-path$']\n",
            )
            .unwrap();

            let config = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default()
                    .with("SLSKD_APP_DIR", root.to_str().unwrap())
                    .with("SLSKR_AUTH_DISABLED", "true")
                    .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target),
            )
            .unwrap();
            assert_eq!(config.managed_blacklist.patterns, vec![expected]);
            std::fs::remove_dir_all(root).unwrap();
        }

        let root = std::env::temp_dir().join(format!(
            "slskr-blacklist-yaml-path-slskdn-transfers-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "transfers:\n  groups:\n    blacklisted:\n      patterns: ['^documented-slskdn-path$']\n",
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .unwrap();
        assert_eq!(
            config.managed_blacklist.patterns,
            vec!["^documented-slskdn-path$"]
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn frozen_transfer_download_settings_match_both_target_profiles() {
        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with(
                    "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                    r#"{"slots":4,"speed_limit":777,"retry":{"partial":"overwrite","attempts":4,"delay":1200,"max_delay":31000},"destination":{"subdirectory":"Music/${SOURCE_USERNAME}","exists":"overwrite","permissions":{"mode":"0750"}}}"#,
                ),
        )
        .unwrap();
        assert_eq!(slskd.transfer_download.slots, 4);
        assert_eq!(slskd.transfer_download.speed_limit_kib, 777);
        assert_eq!(slskd.transfer_download.retry.incomplete, "overwrite");
        assert_eq!(slskd.transfer_download.retry.attempts, 4);
        assert_eq!(slskd.transfer_download.retry.delay.as_millis(), 1200);
        assert_eq!(
            slskd.transfer_download.destination.subdirectory.as_deref(),
            Some("Music/${SOURCE_USERNAME}")
        );
        assert_eq!(
            slskd
                .transfer_download
                .destination
                .permissions_mode
                .as_deref(),
            Some("0750")
        );

        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with(
                    "SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON",
                    r#"{"slots":5,"speed_limit":888,"retry":{"incomplete":"overwrite","attempts":5,"delay":1300,"max_delay":32000},"completed_layout":"uploader_folder","auto_replace_stuck":true,"auto_replace_threshold":7.5,"auto_replace_interval":90}"#,
                )
                .with("SLSKD_DOWNLOAD_SLOTS", "6")
                .with("SLSKD_AUTO_REPLACE_INTERVAL", "91"),
        )
        .unwrap();
        assert_eq!(slskdn.transfer_download.slots, 6);
        assert_eq!(slskdn.transfer_download.speed_limit_kib, 888);
        assert_eq!(slskdn.transfer_download.retry.incomplete, "overwrite");
        assert_eq!(slskdn.transfer_download.completed_layout, "uploader_folder");
        assert!(slskdn.transfer_download.auto_replace_stuck);
        assert_eq!(slskdn.transfer_download.auto_replace_threshold_percent, 7.5);
        assert_eq!(slskdn.transfer_download.auto_replace_interval.as_secs(), 91);

        for (target, json, expected) in [
            ("slskd", r#"{"retry":{"attempts":0}}"#, "attempts"),
            (
                "slskd",
                r#"{"destination":{"permissions":{"mode":"999"}}}"#,
                "permissions",
            ),
            (
                "slskdn",
                r#"{"auto_replace_threshold":0.0}"#,
                "AUTO_REPLACE_THRESHOLD",
            ),
        ] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default()
                    .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
                    .with("SLSKR_FROZEN_TRANSFER_DOWNLOAD_JSON", json),
            )
            .expect_err("invalid frozen transfer download setting must fail");
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn frozen_transfer_groups_and_upload_settings_load_validate_and_preserve_null_windows() {
        let root = std::env::temp_dir().join(format!(
            "slskr-transfer-groups-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "transfers:\n  upload:\n    slots: 20\n    speed_limit: 1200\n    limits:\n      queued:\n        files: 50\n        megabytes: 500\n      daily: null\n      weekly:\n        failures: 8\n  groups:\n    default:\n      upload:\n        priority: 20\n        strategy: firstinfirstout\n        slots: 9\n    leechers:\n      thresholds:\n        files: 4\n        directories: 2\n      upload:\n        priority: 90\n        slots: 1\n        speed_limit: 100\n    user_defined:\n      friends:\n        upload:\n          priority: 5\n          slots: 7\n          limits:\n            queued:\n              files: 100\n        members: [alice, bob]\n",
        )
        .unwrap();

        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_APP_DIR", root.to_str().unwrap()),
        )
        .unwrap();
        assert_eq!(config.transfer_upload.slots, 20);
        assert_eq!(config.transfer_upload.speed_limit_kib, 1200);
        assert_eq!(
            config.transfer_upload.limits.queued.as_ref().unwrap().files,
            Some(50)
        );
        assert_eq!(
            config.transfer_upload.limits.daily,
            Some(super::TransferLimitSettings::default())
        );
        assert_eq!(
            config
                .transfer_upload
                .limits
                .weekly
                .as_ref()
                .unwrap()
                .failures,
            Some(8)
        );
        assert_eq!(config.transfer_groups.default.upload.priority, 20);
        assert_eq!(
            config.transfer_groups.default.upload.strategy,
            super::TransferQueueStrategy::FirstInFirstOut
        );
        assert_eq!(config.transfer_groups.leechers.threshold_files, 4);
        assert_eq!(config.transfer_groups.leechers.threshold_directories, 2);
        assert_eq!(config.transfer_groups.leechers.upload.speed_limit_kib, 100);
        assert_eq!(
            config.transfer_groups.user_defined["friends"].members,
            vec!["alice", "bob"]
        );
        assert_eq!(
            config.transfer_groups.user_defined["friends"]
                .upload
                .limits
                .queued
                .as_ref()
                .unwrap()
                .files,
            Some(100)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn frozen_transfer_group_validation_is_target_specific() {
        let duplicate = r#"{
            "user_defined": {
                "first": {"members": ["alice"]},
                "second": {"members": ["alice"]}
            }
        }"#;
        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", duplicate),
        )
        .expect_err("slskdN rejects duplicate explicit group membership");
        assert!(slskdn.contains("multiple groups"), "{slskdn}");

        let blacklist_duplicate = r#"{
            "blacklisted": {"members": ["alice"]},
            "user_defined": {"first": {"members": ["ALICE"]}}
        }"#;
        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", blacklist_duplicate),
        )
        .expect_err("slskdN rejects blacklisted/user-defined duplicate membership");
        assert!(slskdn.contains("multiple groups"), "{slskdn}");

        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", duplicate),
        )
        .expect("slskd resolves duplicate memberships by group priority");
        assert_eq!(slskd.transfer_groups.user_defined.len(), 2);

        for json in [
            r#"{"default":{"upload":{"priority":0}}}"#,
            r#"{"default":{"upload":{"slots":0}}}"#,
            r#"{"leechers":{"thresholds":{"files":0}}}"#,
            r#"{"default":{"upload":{"strategy":"invalid"}}}"#,
            r#"{"default":{"upload":{"limits":{"queued":{"files":0}}}}}"#,
        ] {
            assert!(
                super::AppConfig::from_layers(
                    None,
                    super::FileConfig::default(),
                    &MapEnv::default().with("SLSKR_FROZEN_TRANSFER_GROUPS_JSON", json),
                )
                .is_err(),
                "accepted invalid groups JSON: {json}"
            );
        }
    }

    #[test]
    fn frozen_slskd_startup_aliases_drive_core_runtime_configuration() {
        let env = MapEnv::default()
            .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
            .with("SLSKD_APP_DIR", "/tmp/slskd-compatible-state")
            .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.2")
            .with("SLSKD_HTTP_PORT", "55030")
            .with("SLSKD_SLSK_ADDRESS", "soulseek.example")
            .with("SLSKD_SLSK_PORT", "2271")
            .with("SLSKD_SLSK_USERNAME", "upstream-user")
            .with("SLSKD_SLSK_PASSWORD", "upstream-password")
            .with("SLSKD_SLSK_LISTEN_IP_ADDRESS", "0.0.0.0")
            .with("SLSKD_SLSK_LISTEN_PORT", "55031")
            .with("SLSKD_SLSK_DESCRIPTION", "upstream description")
            .with("SLSKD_SLSK_OBFUSCATION", "true")
            .with("SLSKD_SLSK_OBFUSCATION_MODE", "prefer")
            .with("SLSKD_SLSK_OBFUSCATION_LISTEN_PORT", "55032")
            .with("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "true")
            .with("SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND", "false")
            .with(
                "SLSKD_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
                "{uploader}/{remote_folder}",
            )
            .with("SLSKD_NO_CONNECT", "true")
            .with("SLSKD_REMOTE_CONFIGURATION", "true")
            .with("SLSKD_DEBUG", "true")
            .with("SLSKD_NO_CONFIG_WATCH", "true");
        let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect("frozen slskd aliases");

        assert_eq!(
            config.state_dir,
            std::path::PathBuf::from("/tmp/slskd-compatible-state")
        );
        assert_eq!(config.http_bind, "127.0.0.2:55030".parse().unwrap());
        assert_eq!(config.server_address, "soulseek.example:2271");
        assert_eq!(config.username.as_deref(), Some("upstream-user"));
        assert_eq!(config.password.as_deref(), Some("upstream-password"));
        assert_eq!(config.listen_port, 55031);
        assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:55031"));
        assert_eq!(
            config.obfuscated_listener_bind.as_deref(),
            Some("0.0.0.0:55032")
        );
        assert_eq!(config.obfuscated_advertised_port, Some(55032));
        assert_eq!(config.obfuscation_listen_port, 55032);
        assert!(config.obfuscation_advertise_regular_port);
        assert_eq!(
            config.obfuscation_mode,
            super::SoulseekObfuscationMode::Prefer
        );
        assert!(!config.obfuscation_prefer_outbound);
        assert_eq!(
            config.download_completed_path_template,
            "{uploader}/{remote_folder}"
        );
        assert_eq!(config.user_info_description, "upstream description");
        assert!(!config.auto_connect);
        assert!(config.remote_configuration);
        assert!(config.controller_debug);
        assert!(config.controller_no_config_watch);
    }

    #[test]
    fn frozen_listener_defaults_bind_the_projected_unspecified_address_and_port() {
        for target in ["slskd", "slskdn"] {
            let config = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default()
                    .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
                    .with("SLSKD_NO_CONNECT", "true")
                    .with("SLSKR_AUTH_DISABLED", "true"),
            )
            .expect("frozen listener defaults");

            assert_eq!(config.listen_port, 50300);
            assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:50300"));
        }
    }

    #[test]
    fn native_startup_names_take_precedence_over_frozen_slskd_aliases() {
        let env = MapEnv::default()
            .with("SLSKR_HTTP_BIND", "127.0.0.1:51000")
            .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.2")
            .with("SLSKD_HTTP_PORT", "52000")
            .with("SLSK_LISTEN_PORT", "51001")
            .with("SLSKD_SLSK_LISTEN_PORT", "52001")
            .with("SLSK_USERNAME", "native-user")
            .with("SLSKD_SLSK_USERNAME", "upstream-user")
            .with("SLSKR_AUTO_CONNECT", "true")
            .with("SLSKD_NO_CONNECT", "true")
            .with("SLSKR_REMOTE_CONFIGURATION", "false")
            .with("SLSKD_REMOTE_CONFIGURATION", "true")
            .with("SLSKR_DEBUG", "false")
            .with("SLSKD_DEBUG", "true")
            .with("SLSKR_NO_CONFIG_WATCH", "false")
            .with("SLSKD_NO_CONFIG_WATCH", "true");
        let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect("native precedence");

        assert_eq!(config.http_bind, "127.0.0.1:51000".parse().unwrap());
        assert_eq!(config.listen_port, 51001);
        assert_eq!(config.username.as_deref(), Some("native-user"));
        assert!(config.auto_connect);
        assert!(!config.remote_configuration);
        assert!(!config.controller_debug);
        assert!(!config.controller_no_config_watch);
    }

    #[test]
    fn invalid_frozen_slskd_alias_reports_the_exact_name() {
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_HTTP_PORT", "not-a-port"),
        )
        .expect_err("invalid frozen alias must fail");
        assert!(error.contains("SLSKD_HTTP_PORT"), "{error}");
    }

    #[test]
    fn frozen_web_bind_profiles_preserve_multi_address_and_target_specific_names() {
        let slskd_default = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("frozen slskd default web bind");
        assert_eq!(slskd_default.controller_http_address, None);
        assert_eq!(slskd_default.http_binds, vec!["[::]:5030".parse().unwrap()]);

        let slskd_multi = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.1, ::1")
                .with("SLSKD_HTTP_PORT", "55440"),
        )
        .expect("frozen slskd comma-separated web binds");
        assert_eq!(
            slskd_multi.controller_http_address.as_deref(),
            Some("127.0.0.1, ::1")
        );
        assert_eq!(
            slskd_multi.http_binds,
            vec![
                "127.0.0.1:55440".parse().unwrap(),
                "[::1]:55440".parse().unwrap()
            ]
        );

        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.2")
                .with("SLSKD_HTTP_ADDRESS", "*")
                .with("SLSKD_HTTP_PORT", "55441"),
        )
        .expect("frozen slskdN address web bind");
        assert_eq!(slskdn.controller_http_address.as_deref(), Some("*"));
        assert_eq!(slskdn.http_binds, vec!["0.0.0.0:55441".parse().unwrap()]);

        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_HTTP_IP_ADDRESS", "127.0.0.1,not-an-ip"),
        )
        .expect_err("invalid frozen slskd web IP list must fail");
        assert!(error.contains("SLSKD_HTTP_IP_ADDRESS"), "{error}");
    }

    #[test]
    fn frozen_directory_yaml_environment_and_target_validation_drive_storage_roots() {
        let root = std::env::temp_dir().join(format!(
            "slskr-controller-directories-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let yaml_downloads = root.join("yaml-downloads");
        let yaml_incomplete = root.join("yaml-incomplete");
        let yaml_share_a = root.join("yaml-share-a");
        let yaml_share_b = root.join("yaml-share-b");
        let env_downloads = root.join("env-downloads");
        std::fs::create_dir_all(&yaml_downloads).unwrap();
        std::fs::create_dir_all(&yaml_incomplete).unwrap();
        std::fs::create_dir_all(&yaml_share_a).unwrap();
        std::fs::create_dir_all(&yaml_share_b).unwrap();
        std::fs::create_dir_all(&env_downloads).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            format!(
                "directories:\n  downloads: '{}'\n  incomplete: '{}'\nshares:\n  directories:\n    - '{}'\n    - '{}'\n",
                yaml_downloads.display(),
                yaml_incomplete.display(),
                yaml_share_a.display(),
                yaml_share_b.display()
            ),
        )
        .unwrap();

        let yaml = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("directory YAML provider");
        assert_eq!(yaml.downloads_dir, yaml_downloads);
        assert_eq!(yaml.incomplete_dir, yaml_incomplete);
        assert_eq!(
            yaml.share_settings.roots,
            vec![yaml_share_a.clone(), yaml_share_b.clone()]
        );

        let environment = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKD_DOWNLOADS_DIR", env_downloads.to_str().unwrap()),
        )
        .expect("directory YAML precedence over frozen environment alias");
        assert_eq!(environment.downloads_dir, yaml_downloads);
        assert_eq!(environment.incomplete_dir, yaml_incomplete);

        let missing = root.join("missing");
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKR_DOWNLOADS_DIR", missing.to_str().unwrap()),
        )
        .expect_err("missing configured directory must fail");
        assert!(error.contains("non-existent directory"), "{error}");

        let current = std::env::current_dir().unwrap();
        let relative_root = current.join(format!(
            ".slskr-relative-directory-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&relative_root).unwrap();
        let relative = relative_root
            .strip_prefix(&current)
            .unwrap()
            .to_str()
            .unwrap();
        let slskd_error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKR_DOWNLOADS_DIR", relative),
        )
        .expect_err("slskd rejects relative download directories");
        assert!(slskd_error.contains("absolute path"), "{slskd_error}");
        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKR_DOWNLOADS_DIR", relative),
        )
        .expect("slskdN accepts an existing relative download directory");
        assert_eq!(slskdn.downloads_dir, std::path::PathBuf::from(relative));

        std::fs::remove_dir_all(relative_root).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn slskdn_rejects_loopback_listener_only_when_connecting() {
        let root = std::env::temp_dir().join(format!(
            "slskr-no-connect-validation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let base = MapEnv::default()
            .with("SLSKR_STATE_DIR", root.to_str().unwrap())
            .with("SLSKR_AUTH_DISABLED", "true")
            .with("SLSKD_SLSK_USERNAME", "fixture-user")
            .with("SLSKD_SLSK_PASSWORD", "fixture-password")
            .with("SLSKD_SLSK_LISTEN_IP_ADDRESS", "127.0.0.1")
            .with("SLSKD_SLSK_LISTEN_PORT", "55091");

        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &base
                .clone()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKD_NO_CONNECT", "false"),
        )
        .expect("frozen slskd permits a loopback listener while connecting");
        assert!(slskd.auto_connect);

        let disconnected = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &base
                .clone()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_NO_CONNECT", "true"),
        )
        .expect("frozen slskdN permits a loopback listener when no-connect is set");
        assert!(!disconnected.auto_connect);

        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &base
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_NO_CONNECT", "false"),
        )
        .expect_err("frozen slskdN rejects a loopback listener while connecting");
        assert_eq!(
            error,
            "Soulseek.ListenIpAddress must not be a loopback address when the client is connecting. Use 0.0.0.0 or a reachable LAN/VPN interface instead."
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controller_yaml_is_a_real_startup_provider_for_core_settings() {
        let root = std::env::temp_dir().join(format!(
            "slskr-controller-yaml-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "instance_name: yaml-instance\ndebug: true\nremote_configuration: true\nremote_file_management: true\nflags:\n  no_connect: true\n  no_config_watch: true\ndht:\n  enabled: false\n  dht_port: 55200\nsoulseek:\n  address: yaml.example\n  port: 2271\n  username: yaml-user\n  password: yaml-password\n  description: yaml description\n  listen_ip_address: 0.0.0.0\n  listen_port: 55100\n  obfuscation:\n    enabled: true\n    mode: prefer\n    listen_port: 55101\n    prefer_outbound: false\n  private_message_auto_response:\n    enabled: true\n    message: yaml auto response\n    cooldown_minutes: 15\nintegrations:\n  spotify:\n    enabled: true\n    client_id: yaml-client\n    client_secret: yaml-client-secret\n    redirect_uri: https://localhost/callback\n    market: CA\n  lidarr:\n    enabled: true\n    url: https://lidarr.example\n    api_key: yaml-lidarr-key\n    timeout_seconds: 30\ntransfers:\n  download:\n    completed_path_template: '{uploader}/{remote_folder}'\n    auto_retry:\n      enabled: false\n      retry_delay_seconds: 1200\n      check_interval_seconds: 120\n      max_attempts: 7\n      max_files_per_cycle: 8\n      max_files_per_peer_per_cycle: 2\n      peer_cooldown_seconds: 600\n      alternate_sources_enabled: false\n      max_alternate_source_searches_per_cycle: 2\n      alternate_source_size_tolerance_percent: 7\nweb:\n  ip_address: 127.0.0.3\n  port: 55102\n",
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("controller YAML startup provider");

        assert!(config.controller_debug);
        assert_eq!(config.instance_name, "yaml-instance");
        assert!(config.remote_configuration);
        assert!(config.remote_file_management);
        assert!(config.controller_no_config_watch);
        assert!(!config.auto_connect);
        assert_eq!(config.server_address, "yaml.example:2271");
        assert_eq!(config.username.as_deref(), Some("yaml-user"));
        assert_eq!(config.password.as_deref(), Some("yaml-password"));
        assert_eq!(config.user_info_description, "yaml description");
        assert_eq!(config.listen_port, 55100);
        assert_eq!(config.listener_bind.as_deref(), Some("0.0.0.0:55100"));
        assert_eq!(
            config.obfuscated_listener_bind.as_deref(),
            Some("0.0.0.0:55101")
        );
        assert_eq!(config.http_bind, "127.0.0.3:55102".parse().unwrap());
        assert!(!config.dht_enabled);
        assert_eq!(config.dht_port, 55200);
        assert_eq!(
            config.obfuscation_mode,
            super::SoulseekObfuscationMode::Prefer
        );
        assert_eq!(config.obfuscation_listen_port, 55101);
        assert!(config.obfuscation_advertise_regular_port);
        assert!(!config.obfuscation_prefer_outbound);
        assert_eq!(
            config.download_completed_path_template,
            "{uploader}/{remote_folder}"
        );
        assert!(config.private_message_auto_response.enabled);
        assert_eq!(
            config.private_message_auto_response.message,
            "yaml auto response"
        );
        assert_eq!(config.private_message_auto_response.cooldown_minutes, 15);
        assert!(config.integrations.spotify.enabled);
        assert_eq!(
            config.integrations.spotify.client_id.as_deref(),
            Some("yaml-client")
        );
        assert_eq!(config.integrations.spotify.market, "CA");
        assert!(config.integrations.lidarr.enabled);
        assert_eq!(
            config.integrations.lidarr.url.as_deref(),
            Some("https://lidarr.example")
        );
        assert_eq!(config.integrations.lidarr.timeout_seconds, 30);
        assert!(!config.transfer_auto_retry.enabled);
        assert_eq!(config.transfer_auto_retry.retry_delay.as_secs(), 1200);
        assert_eq!(config.transfer_auto_retry.check_interval.as_secs(), 120);
        assert_eq!(config.transfer_auto_retry.max_attempts, 7);
        assert_eq!(config.transfer_auto_retry.max_files_per_cycle, 8);
        assert_eq!(config.transfer_auto_retry.max_files_per_peer_per_cycle, 2);
        assert_eq!(config.transfer_auto_retry.peer_cooldown.as_secs(), 600);
        assert!(!config.transfer_auto_retry.alternate_sources_enabled);
        assert_eq!(
            config
                .transfer_auto_retry
                .max_alternate_source_searches_per_cycle,
            2
        );
        assert_eq!(
            config
                .transfer_auto_retry
                .alternate_source_size_tolerance_percent,
            7.0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn controller_yaml_instance_name_matches_frozen_unvalidated_string_binding() {
        let root = std::env::temp_dir().join(format!(
            "slskr-controller-instance-name-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let long_name = "a".repeat(300);
        let cases = [
            ("instance_name: null\n", "default".to_owned()),
            ("instance_name: \"\"\n", "default".to_owned()),
            ("instance_name: 123\n", "123".to_owned()),
            ("instance_name: true\n", "true".to_owned()),
            (
                "instance_name: \"line one\\nline two\"\n",
                "line one\nline two".to_owned(),
            ),
        ];

        for (yaml, expected) in cases {
            std::fs::write(root.join("slskd.yml"), yaml).unwrap();
            let config = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
            )
            .expect("frozen instance-name scalar binding");
            assert_eq!(config.instance_name, expected);
        }

        std::fs::write(
            root.join("slskd.yml"),
            format!("instance_name: \"{long_name}\"\n"),
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
        )
        .expect("frozen long instance name");
        assert_eq!(config.instance_name, long_name);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn frozen_share_directories_preserve_aliases_exclusions_and_raw_values() {
        let root = std::env::temp_dir().join(format!(
            "slskr-controller-share-aliases-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let excluded = root.join("excluded");
        std::fs::create_dir_all(&excluded).unwrap();
        let value = format!("[Library]{};!{}", root.display(), excluded.display());
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_SHARED_DIR", &value),
        )
        .expect("aliased and excluded frozen shares");

        assert_eq!(config.share_settings.directories.len(), 2);
        assert_eq!(config.share_settings.directories[0].alias, "Library");
        assert!(!config.share_settings.directories[0].is_excluded);
        assert_eq!(config.share_settings.directories[1].alias, "excluded");
        assert!(config.share_settings.directories[1].is_excluded);
        assert_eq!(config.share_settings.roots, vec![root.clone()]);
        assert_eq!(
            config.share_settings.directories[0].raw,
            format!("[Library]{}", root.display())
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn controller_yaml_precedes_frozen_environment_aliases_while_native_names_win() {
        let root = std::env::temp_dir().join(format!(
            "slskr-controller-yaml-precedence-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "debug: true\nremote_configuration: true\nweb:\n  port: 55102\nsoulseek:\n  listen_port: 55100\n",
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_STATE_DIR", root.to_str().unwrap())
                .with("SLSKD_DEBUG", "false")
                .with("SLSKR_REMOTE_CONFIGURATION", "false")
                .with("SLSKD_HTTP_PORT", "55202")
                .with("SLSK_LISTEN_PORT", "55200"),
        )
        .expect("controller precedence");

        assert!(config.controller_debug);
        assert!(!config.remote_configuration);
        assert_eq!(config.http_bind.port(), 55102);
        assert_eq!(config.listen_port, 55200);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn controller_yaml_startup_provider_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "slskr-controller-yaml-link-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let outside = root.with_extension("outside.yml");
        std::fs::write(&outside, "debug: true\n").unwrap();
        symlink(&outside, root.join("slskd.yml")).unwrap();
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_STATE_DIR", root.to_str().unwrap()),
        )
        .expect_err("controller YAML symlink must fail");
        assert_eq!(error, "controller YAML must be a regular file");
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_file(outside);
    }

    #[test]
    fn config_file_reader_rejects_oversized_files() {
        let path = std::env::temp_dir().join(format!(
            "slskr-config-large-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(super::MAX_CONFIG_FILE_BYTES + 1).unwrap();

        let error =
            super::read_file_config(&path).expect_err("oversized config should be rejected");
        assert!(error.contains("config file"));
        assert!(error.contains("too large"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn config_file_reader_rejects_non_regular_paths() {
        let path = std::env::temp_dir().join(format!(
            "slskr-config-dir-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir(&path).unwrap();

        let error = super::read_file_config(&path).expect_err("directory should be rejected");
        assert!(error.contains("config"));
        assert!(error.contains(&path.display().to_string()));

        let _ = std::fs::remove_dir(path);
    }

    #[cfg(unix)]
    #[test]
    fn config_file_reader_rejects_symlinks() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "slskr-config-symlink-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let target = root.join("target.toml");
        let linked = root.join("config.toml");
        std::fs::write(&target, "[auth]\napi_token = \"outside-secret\"\n").unwrap();
        symlink(&target, &linked).unwrap();

        let error = super::read_file_config(&linked).expect_err("symlink should be rejected");
        assert!(error.contains("failed to read config file"));
        assert!(!error.contains("outside-secret"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn config_file_reader_parses_small_files() {
        let path = std::env::temp_dir().join(format!(
            "slskr-config-small-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, "[app]\nhttp_bind = \"127.0.0.1:5555\"\n").unwrap();

        let config = super::read_file_config(&path).expect("small config parsed");
        assert_eq!(config.app.http_bind.as_deref(), Some("127.0.0.1:5555"));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn config_sensitive_value_detection_covers_secrets() {
        let empty = super::FileConfig::default();
        assert!(!super::config_contains_sensitive_values(&empty));

        let with_api_token = super::FileConfig {
            auth: super::AuthFileConfig {
                api_token: Some("token".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(super::config_contains_sensitive_values(&with_api_token));

        let with_integration_secret = super::FileConfig {
            integrations: super::IntegrationsFileConfig {
                spotify: super::SpotifyFileConfig {
                    client_secret: Some("secret".to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(super::config_contains_sensitive_values(
            &with_integration_secret
        ));
    }

    #[test]
    fn lidarr_url_rejects_embedded_credentials_before_projection() {
        let env = MapEnv::default()
            .with("SLSKR_LIDARR_URL", "https://operator:secret@example.com")
            .with("SLSKR_LIDARR_API_KEY", "api-key");
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("credential-bearing Lidarr URL should be rejected");
        assert_eq!(error, "Lidarr URL must not contain embedded credentials");
        assert!(!error.contains("operator"));
        assert!(!error.contains("secret"));

        for url in [
            "ftp://example.com/lidarr",
            "https://example.com/lidarr?api_key=secret",
            "https://example.com/lidarr#ignored-api-path",
        ] {
            let env = MapEnv::default().with("SLSKR_LIDARR_URL", url);
            let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
                .expect_err("non-base Lidarr URL should be rejected");
            assert!(!error.contains("api_key=secret"), "{error}");
        }
    }

    #[test]
    fn trusted_proxy_cidrs_parse_from_env_and_file() {
        let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/32,::1/128");
        let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect("trusted proxy env config");
        assert_eq!(config.trusted_proxy_cidrs.len(), 2);
        assert!(config.trusted_proxy_cidrs[0].contains("127.0.0.1".parse().unwrap()));
        assert!(config.trusted_proxy_cidrs[1].contains("::1".parse().unwrap()));

        let file_config = super::FileConfig {
            auth: super::AuthFileConfig {
                trusted_proxy_cidrs: vec!["10.0.0.0/8".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        };
        let config = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
            .expect("trusted proxy file config");
        assert!(config.trusted_proxy_cidrs[0].contains("10.1.2.3".parse().unwrap()));
    }

    #[test]
    fn api_token_rejects_blank_env_and_file_values() {
        for token in ["", " \t\r\n"] {
            let env = MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_API_TOKEN", token);
            let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
                .expect_err("blank environment API token must fail");
            assert!(error.contains("must not be empty"));

            let file_config = super::FileConfig {
                auth: super::AuthFileConfig {
                    disabled: Some(false),
                    api_token: Some(token.to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            };
            let error = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
                .expect_err("blank file API token must fail");
            assert!(error.contains("must not be empty"));
        }
    }

    #[test]
    fn role_api_tokens_are_distinct_and_never_serialized() {
        let env = MapEnv::default()
            .with("SLSKR_API_TOKEN", "admin-token")
            .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
            .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
            .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token");
        let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect("distinct role tokens");
        let sanitized = config.sanitized_json();
        for token in [
            "admin-token",
            "write-token",
            "read-token",
            "nowplaying-token",
        ] {
            assert!(!sanitized.contains(token));
        }
        assert!(sanitized.contains("\"api_read_write_token_configured\":true"));
        assert!(sanitized.contains("\"api_read_only_token_configured\":true"));
        assert!(sanitized.contains("\"api_nowplaying_token_configured\":true"));

        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_API_TOKEN", "duplicate-token")
                .with("SLSKR_API_READ_ONLY_TOKEN", "duplicate-token"),
        )
        .expect_err("role tokens must not alias");
        assert_eq!(error, "API tokens for different roles must be distinct");
    }

    #[test]
    fn controller_compatibility_target_is_explicit_bounded_and_projected() {
        let default =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .expect("default controller compatibility target");
        assert_eq!(
            default.controller_compatibility_target,
            super::ControllerCompatibilityTarget::Slskdn
        );

        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("slskd controller compatibility target");
        assert_eq!(
            slskd.controller_compatibility_target,
            super::ControllerCompatibilityTarget::Slskd
        );
        assert!(slskd
            .sanitized_json()
            .contains("\"controller_compatibility_target\":\"slskd\""));

        let file = super::FileConfig {
            compatibility: super::CompatibilityFileConfig {
                controller_target: Some("slskd".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        let from_file = super::AppConfig::from_layers(
            None,
            file,
            &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("file controller compatibility target");
        assert_eq!(
            from_file.controller_compatibility_target,
            super::ControllerCompatibilityTarget::Slskd
        );

        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "auto"),
        )
        .expect_err("ambiguous compatibility target must fail");
        assert!(error.contains("must be slskd or slskdn"));
    }

    #[test]
    fn api_token_rejects_unrepresentable_env_and_file_values() {
        for token in [
            " leading",
            "trailing ",
            "token\tvalue",
            "token\nvalue",
            "token\u{7f}value",
        ] {
            let env = MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "false")
                .with("SLSKR_API_TOKEN", token);
            let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
                .expect_err("unrepresentable environment API token must fail");
            assert!(
                error.contains("whitespace") || error.contains("control"),
                "{error}"
            );

            let file_config = super::FileConfig {
                auth: super::AuthFileConfig {
                    disabled: Some(false),
                    api_token: Some(token.to_owned()),
                    ..Default::default()
                },
                ..Default::default()
            };
            let error = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
                .expect_err("unrepresentable file API token must fail");
            assert!(
                error.contains("whitespace") || error.contains("control"),
                "{error}"
            );
        }
    }

    #[test]
    fn api_token_length_matches_http_header_capacity() {
        let maximum = "x".repeat(crate::http_server::MAX_API_TOKEN_BYTES);
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_API_TOKEN", &maximum),
        )
        .expect("maximum representable token");
        assert_eq!(config.api_token.as_deref(), Some(maximum.as_str()));

        let oversized = format!("{maximum}x");
        let env = MapEnv::default().with("SLSKR_API_TOKEN", &oversized);
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("oversized environment token must fail");
        assert!(error.contains("maximum representable"), "{error}");

        let file_config = super::FileConfig {
            auth: super::AuthFileConfig {
                api_token: Some(oversized),
                ..Default::default()
            },
            ..Default::default()
        };
        let error = super::AppConfig::from_layers(None, file_config, &MapEnv::default())
            .expect_err("oversized file token must fail");
        assert!(error.contains("maximum representable"), "{error}");
    }

    #[test]
    fn trusted_proxy_cidrs_reject_invalid_prefixes() {
        let env = MapEnv::default().with("SLSKR_TRUSTED_PROXY_CIDRS", "127.0.0.1/33");
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("invalid trusted proxy prefix should fail");
        assert!(error.contains("prefix exceeds"));
    }

    #[test]
    fn peer_response_timeout_rejects_zero() {
        let env = MapEnv::default().with("SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS", "0");
        let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect_err("zero peer timeout should fail");
        assert!(error.contains("must be greater than zero"), "{error}");
    }

    #[test]
    fn soulseek_connection_defaults_bounds_and_target_difference_are_exact() {
        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("slskdN connection defaults");
        let connection = &slskdn.soulseek_connection;
        assert_eq!(connection.buffer_read, 16_384);
        assert_eq!(connection.buffer_write, 16_384);
        assert_eq!(connection.buffer_transfer, 262_144);
        assert_eq!(connection.buffer_write_queue, 50);
        assert_eq!(connection.timeout_connect, Duration::from_millis(10_000));
        assert_eq!(connection.timeout_inactivity, Duration::from_millis(60_000));
        assert_eq!(connection.timeout_transfer, Duration::from_millis(60_000));
        assert!(!connection.proxy.enabled);
        assert_eq!(connection.proxy.port, None);

        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("slskd connection defaults");
        assert_eq!(
            slskd.soulseek_connection.timeout_inactivity,
            Duration::from_millis(15_000)
        );

        for (name, value) in [
            ("SLSKD_SLSK_READ_BUFFER", "1023"),
            ("SLSKD_SLSK_WRITE_BUFFER", "1023"),
            ("SLSKD_SLSK_TRANSFER_BUFFER", "81919"),
            ("SLSKD_SLSK_WRITE_QUEUE", "4"),
            ("SLSKD_SLSK_CONNECTION_TIMEOUT", "999"),
            ("SLSKD_SLSK_INACTIVITY_TIMEOUT", "999"),
            ("SLSKD_SLSK_TRANSFER_TIMEOUT", "29999"),
        ] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default()
                    .with("SLSKR_AUTH_DISABLED", "true")
                    .with(name, value),
            )
            .expect_err("out-of-range Soulseek connection value");
            assert!(error.contains("must be between"), "{name}: {error}");
        }

        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_SLSK_PROXY_ENABLED", "true"),
        )
        .expect_err("enabled proxy needs endpoint");
        assert!(error.contains("no address"), "{error}");
    }

    #[test]
    fn private_message_auto_response_is_opt_in_bounded_and_redacted() {
        let env = MapEnv::default()
            .with("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE", "true")
            .with(
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE",
                "human response",
            )
            .with("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES", "15");
        let config = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
            .expect("auto-response config");
        assert!(config.private_message_auto_response.enabled);
        assert_eq!(
            config.private_message_auto_response.message,
            "human response"
        );
        assert_eq!(config.private_message_auto_response.cooldown_minutes, 15);
        let sanitized = config.sanitized_json();
        assert!(sanitized.contains("private_message_auto_response"));
        assert!(!sanitized.contains("human response"));

        let blank = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE", ""),
        )
        .expect("blank frozen slskdN auto-response message");
        assert!(blank.private_message_auto_response.message.is_empty());

        for (name, value) in [
            ("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES", "0"),
            (
                "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
                "1441",
            ),
        ] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(name, value),
            )
            .expect_err("invalid auto-response config");
            assert!(error.contains("auto response") || error.contains("auto-response"));
        }
    }

    #[test]
    fn runtime_intervals_reject_zero_and_unrepresentable_values() {
        for name in [
            "SLSKR_RECONNECT_SECONDS",
            "SLSKR_PING_SECONDS",
            "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
        ] {
            for value in ["0", &u64::MAX.to_string()] {
                let env = MapEnv::default().with(name, value);
                let error = super::AppConfig::from_layers(None, super::FileConfig::default(), &env)
                    .expect_err("invalid runtime interval must fail at startup");
                assert!(
                    error.contains("greater than zero") || error.contains("timer range"),
                    "{name}={value}: {error}"
                );
            }
        }
    }

    #[test]
    fn transfer_auto_retry_defaults_match_the_frozen_client_policy() {
        let config =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .expect("default auto-retry config");
        let retry = &config.transfer_auto_retry;
        assert!(retry.enabled);
        assert_eq!(retry.retry_delay.as_secs(), 1800);
        assert_eq!(retry.check_interval.as_secs(), 300);
        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.max_files_per_cycle, 10);
        assert_eq!(retry.max_files_per_peer_per_cycle, 1);
        assert_eq!(retry.peer_cooldown.as_secs(), 900);
        assert!(retry.alternate_sources_enabled);
        assert_eq!(retry.max_alternate_source_searches_per_cycle, 1);
        assert_eq!(retry.alternate_source_size_tolerance_percent, 5.0);
        assert!(config.sanitized_json().contains("\"transfer_auto_retry\""));
    }

    #[test]
    fn transfer_auto_retry_bounds_are_enforced_at_startup() {
        for (name, value) in [
            ("SLSKR_TRANSFER_AUTO_RETRY_DELAY_SECONDS", "9"),
            ("SLSKR_TRANSFER_AUTO_RETRY_CHECK_INTERVAL_SECONDS", "3601"),
            ("SLSKR_TRANSFER_AUTO_RETRY_MAX_ATTEMPTS", "101"),
            ("SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_CYCLE", "0"),
            (
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_FILES_PER_PEER_PER_CYCLE",
                "21",
            ),
            ("SLSKR_TRANSFER_AUTO_RETRY_PEER_COOLDOWN_SECONDS", "59"),
            (
                "SLSKR_TRANSFER_AUTO_RETRY_MAX_ALTERNATE_SOURCE_SEARCHES_PER_CYCLE",
                "11",
            ),
            (
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                "101",
            ),
        ] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(name, value),
            )
            .expect_err("out-of-range auto-retry config must fail");
            assert!(error.contains("must be between"), "{name}={value}: {error}");
        }
    }

    #[test]
    fn transfer_auto_retry_preserves_fractional_tolerance_and_frozen_boundary_rounding() {
        for (value, expected) in [("5.5", 5.5), ("-0.5", -0.5), ("100.5", 100.5)] {
            let config = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(
                    "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                    value,
                ),
            )
            .expect("frozen slskdN tolerance must bind");
            assert_eq!(
                config
                    .transfer_auto_retry
                    .alternate_source_size_tolerance_percent,
                expected
            );
        }
        for value in ["-0.5001", "100.5001"] {
            assert!(super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(
                    "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                    value,
                ),
            )
            .is_err());
        }
    }

    #[test]
    fn managed_blacklist_parses_cidr_p2p_and_dat_ranges() {
        let root = std::env::temp_dir().join(format!(
            "slskr-managed-blacklist-formats-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let cases = [
            ("cidr.txt", "127.0.0.0/24\n"),
            ("p2p.txt", "loopback:127.0.0.1-127.0.0.2\n"),
            (
                "dat.txt",
                "127.000.000.001 - 127.000.000.002 , 000 , local\n",
            ),
        ];
        for (name, body) in cases {
            let path = root.join(name);
            std::fs::write(&path, body).unwrap();
            let config = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default()
                    .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                    .with("SLSKD_BLACKLIST", "true")
                    .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
            )
            .expect("managed blacklist format");

            assert!(config
                .managed_blacklist
                .contains("127.0.0.1".parse().unwrap()));
            assert!(config
                .managed_blacklist
                .contains("::ffff:127.0.0.2".parse().unwrap()));
            assert!(!config
                .managed_blacklist
                .contains("127.0.1.1".parse().unwrap()));
        }

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn managed_blacklist_preserves_target_specific_p2p_colon_handling() {
        let root = std::env::temp_dir().join(format!(
            "slskr-managed-blacklist-p2p-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("colon.p2p");
        std::fs::write(&path, "category:label:127.0.0.1-127.0.0.1\n").unwrap();

        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_BLACKLIST", "true")
                .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
        )
        .expect("slskdN uses the final P2P colon");
        assert!(slskdn
            .managed_blacklist
            .contains("127.0.0.1".parse().unwrap()));

        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd")
                .with("SLSKD_BLACKLIST", "true")
                .with("SLSKD_BLACKLIST_FILE", path.to_str().unwrap()),
        );
        assert!(slskd.is_err(), "slskd uses the first P2P colon");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controller_swagger_defaults_split_by_target_and_honors_environment() {
        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("slskd swagger default");
        assert!(!slskd.controller_swagger);

        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .expect("slskdN swagger default");
        assert!(slskdn.controller_swagger);

        let disabled = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_SWAGGER", "false"),
        )
        .expect("slskdN swagger environment override");
        assert!(!disabled.controller_swagger);
    }

    #[test]
    fn controller_metrics_defaults_split_by_target_and_enforce_credentials() {
        let slskd = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("slskd metrics defaults");
        assert!(!slskd.controller_metrics_enabled);
        assert_eq!(slskd.controller_metrics_url, "/metrics");
        assert_eq!(slskd.controller_metrics_username, "slskd");
        assert_eq!(slskd.controller_metrics_password, "slskd");

        let slskdn = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .expect("slskdN disabled metrics defaults");
        assert!(!slskdn.controller_metrics_enabled);
        assert!(slskdn.controller_metrics_password.is_empty());

        let missing_password = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_METRICS", "true"),
        );
        assert!(missing_password.is_err());

        let configured = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_METRICS", "true")
                .with("SLSKD_METRICS_URL", "prometheus")
                .with("SLSKD_METRICS_USERNAME", "metrics-user")
                .with("SLSKD_METRICS_PASSWORD", "metrics-pass"),
        )
        .expect("slskdN configured metrics auth");
        assert!(configured.controller_metrics_enabled);
        assert_eq!(configured.controller_metrics_url, "prometheus");
        assert_eq!(configured.controller_metrics_username, "metrics-user");
        assert_eq!(configured.controller_metrics_password, "metrics-pass");
    }

    #[test]
    fn controller_headless_defaults_false_and_honors_environment() {
        let default = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskd"),
        )
        .expect("headless default");
        assert!(!default.controller_headless);

        let headless = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn")
                .with("SLSKD_HEADLESS", "true"),
        )
        .expect("headless environment override");
        assert!(headless.controller_headless);
    }

    #[test]
    fn controller_startup_flags_default_false_and_honor_environment() {
        let default = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_AUTH_DISABLED", "true"),
        )
        .expect("startup flag defaults");
        assert!(!default.controller_no_logo);
        assert!(!default.controller_no_start);
        assert!(!default.controller_no_version_check);
        assert!(!default.controller_experimental);
        assert!(!default.controller_case_sensitive_regex);
        assert!(default.controller_search_request_filters.is_empty());
        assert!(default.share_settings.filters.is_empty());
        assert!(!default.controller_no_share_scan);
        assert!(!default.controller_force_share_scan);

        let configured = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKR_AUTH_DISABLED", "true")
                .with("SLSKD_NO_LOGO", "true")
                .with("SLSKD_NO_START", "true")
                .with("SLSKD_NO_VERSION_CHECK", "true")
                .with("SLSKD_EXPERIMENTAL", "true")
                .with("SLSKD_CASE_SENSITIVE_REGEX", "true")
                .with("SLSKD_SEARCH_REQUEST_FILTER", "first;second")
                .with("SLSKD_SHARE_FILTER", "secret;private")
                .with("SLSKD_NO_SHARE_SCAN", "true")
                .with("SLSKD_FORCE_SHARE_SCAN", "true"),
        )
        .expect("startup environment flags");
        assert!(configured.controller_no_logo);
        assert!(configured.controller_no_start);
        assert!(configured.controller_no_version_check);
        assert!(configured.controller_experimental);
        assert!(configured.controller_case_sensitive_regex);
        assert_eq!(
            configured.controller_search_request_filters,
            ["first", "second"]
        );
        assert_eq!(configured.share_settings.filters, ["secret", "private"]);
        assert!(configured.controller_no_share_scan);
        assert!(configured.controller_force_share_scan);
    }

    #[test]
    fn transfer_rescue_defaults_match_the_frozen_client_policy() {
        let config =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .expect("default rescue config");
        let rescue = &config.transfer_rescue;
        assert!(rescue.enabled);
        assert_eq!(rescue.max_queue_time.as_secs(), 1_800);
        assert_eq!(rescue.min_throughput_bytes_per_second, 10 * 1_024);
        assert_eq!(rescue.min_duration.as_secs(), 300);
        assert_eq!(rescue.stalled_timeout.as_secs(), 120);
        assert_eq!(rescue.check_interval.as_secs(), 45);
        assert_eq!(rescue.retry_cooldown.as_secs(), 1_800);
        assert_eq!(rescue.max_files_per_cycle, 2);
        assert_eq!(rescue.alternate_source_size_tolerance_percent, 5);
        assert!(config.sanitized_json().contains("\"transfer_rescue\""));
    }

    #[test]
    fn transfer_rescue_bounds_are_enforced_at_startup() {
        for (name, value) in [
            ("SLSKR_TRANSFER_RESCUE_MAX_QUEUE_TIME_SECONDS", "59"),
            ("SLSKR_TRANSFER_RESCUE_MIN_THROUGHPUT_KBPS", "0"),
            ("SLSKR_TRANSFER_RESCUE_MIN_DURATION_SECONDS", "59"),
            ("SLSKR_TRANSFER_RESCUE_STALLED_TIMEOUT_SECONDS", "29"),
            ("SLSKR_TRANSFER_RESCUE_CHECK_INTERVAL_SECONDS", "14"),
            ("SLSKR_TRANSFER_RESCUE_RETRY_COOLDOWN_SECONDS", "59"),
            ("SLSKR_TRANSFER_RESCUE_MAX_FILES_PER_CYCLE", "0"),
            (
                "SLSKR_TRANSFER_RESCUE_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                "101",
            ),
        ] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(name, value),
            )
            .expect_err("out-of-range rescue config must fail");
            assert!(error.contains("must be between"), "{name}={value}: {error}");
        }
    }

    #[test]
    fn soulseek_obfuscation_defaults_to_regular_first_compatibility() {
        let compatibility =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .expect("default obfuscation config");
        assert!(compatibility.obfuscation_enabled);
        assert_eq!(
            compatibility.obfuscation_mode,
            super::SoulseekObfuscationMode::Compatibility
        );
        assert!(compatibility.obfuscation_prefer_outbound);
        assert!(compatibility.obfuscation_advertise_regular_port);
        assert!(!compatibility.prefer_obfuscated_outbound());

        let prefer = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_OBFUSCATION_MODE", "prefer"),
        )
        .expect("prefer obfuscation config");
        assert!(prefer.prefer_obfuscated_outbound());

        for value in ["only", "unknown"] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSK_OBFUSCATION_MODE", value),
            )
            .expect_err("unsupported obfuscation mode must fail");
            assert!(
                error.to_ascii_lowercase().contains("obfuscation"),
                "{error}"
            );
        }

        let missing_regular_advertisement = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT", "false"),
        )
        .expect("frozen slskdN options validation accepts the incompatible combination");
        assert!(missing_regular_advertisement.obfuscation_enabled);
        assert!(!missing_regular_advertisement.obfuscation_advertise_regular_port);
    }

    #[test]
    fn pod_join_signature_modes_are_validated() {
        let default =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .expect("default pod signature config");
        assert_eq!(
            default.pod_join_signature_mode,
            super::PodSignatureMode::Off
        );

        for (value, expected) in [
            ("warn", super::PodSignatureMode::Warn),
            ("enforce", super::PodSignatureMode::Enforce),
        ] {
            let config = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", value),
            )
            .expect("supported pod signature mode");
            assert_eq!(config.pod_join_signature_mode, expected);
        }

        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_POD_JOIN_SIGNATURE_MODE", "accept-anything"),
        )
        .expect_err("invalid pod signature mode must fail");
        assert!(error.contains("off, warn, or enforce"), "{error}");
    }

    #[test]
    fn virtual_soulfind_v2_defaults_enabled_and_honors_file_and_env_layers() {
        let default =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .expect("default VirtualSoulfind v2 config");
        assert!(default.virtual_soulfind_v2_enabled);

        let file_disabled = super::AppConfig::from_layers(
            None,
            super::FileConfig {
                virtual_soulfind_v2: super::VirtualSoulfindV2FileConfig {
                    enabled: Some(false),
                },
                ..super::FileConfig::default()
            },
            &MapEnv::default(),
        )
        .expect("file-disabled VirtualSoulfind v2 config");
        assert!(!file_disabled.virtual_soulfind_v2_enabled);

        let env_enabled = super::AppConfig::from_layers(
            None,
            super::FileConfig {
                virtual_soulfind_v2: super::VirtualSoulfindV2FileConfig {
                    enabled: Some(false),
                },
                ..super::FileConfig::default()
            },
            &MapEnv::default().with("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED", "true"),
        )
        .expect("environment-enabled VirtualSoulfind v2 config");
        assert!(env_enabled.virtual_soulfind_v2_enabled);
        assert!(env_enabled
            .sanitized_json()
            .contains("\"virtual_soulfind_v2_enabled\":true"));
    }

    #[test]
    fn trusted_mesh_peers_are_bounded_pinned_and_redacted() {
        let value = serde_json::json!([{
            "peerId": "peer-a",
            "username": "mesh-user",
            "overlayEndpoint": "127.0.0.1:50305",
            "certificateSha256": "11".repeat(32),
            "rangeEndpoint": "https://mesh.example/content/{sha256}?ignored"
        }]);
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
        )
        .expect_err("query-bearing endpoint must fail");
        assert!(error.contains("query or fragment"), "{error}");

        let value = serde_json::json!([{
            "peerId": "peer-a",
            "username": "mesh-user",
            "overlayEndpoint": "127.0.0.1:50305",
            "certificateSha256": "11".repeat(32),
            "rangeEndpoint": "https://{recordingId}.example/content/{sha256}"
        }]);
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
        )
        .expect_err("authority placeholder must fail");
        assert!(error.contains("only in the path"), "{error}");

        let value = serde_json::json!([{
            "peerId": "peer-a",
            "username": "mesh-user",
            "overlayEndpoint": "127.0.0.1:50305",
            "certificateSha256": "11".repeat(32),
            "rangeEndpoint": "https://mesh.example/content/{sha256}/{size}/{recordingId}"
        }]);
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
        )
        .expect("trusted mesh config");
        let peer = &config.trusted_mesh_peers[0];
        assert!(peer.matches("PEER-A"));
        assert!(peer.matches("MESH-USER"));
        assert_eq!(peer.certificate_sha256, [0x11; 32]);
        assert_eq!(
            peer.range_url(&"a".repeat(64), 42, Some("recording-1")),
            Some(format!(
                "https://mesh.example/content/{}/42/recording-1",
                "a".repeat(64)
            ))
        );
        assert_eq!(
            peer.range_url(&"a".repeat(64), 42, Some("recording/../?next=1")),
            Some(format!(
                "https://mesh.example/content/{}/42/recording%2F..%2F%3Fnext%3D1",
                "a".repeat(64)
            ))
        );
        assert!(peer.range_url("not-a-sha256", 42, None).is_none());
        let sanitized = config.sanitized_json();
        assert!(sanitized.contains("\"trusted_mesh_peers\":1"));
        assert!(!sanitized.contains("mesh.example"));
        assert!(!sanitized.contains("mesh-user"));
        assert!(!sanitized.contains(&"11".repeat(32)));
    }

    #[test]
    fn trusted_mesh_peer_config_rejects_ambiguous_or_unpinned_identities() {
        for value in [
            serde_json::json!([{
                "peerId": "peer-a",
                "username": "mesh-user",
                "overlayEndpoint": "127.0.0.1:0",
                "certificateSha256": "11".repeat(32)
            }]),
            serde_json::json!([{
                "peerId": "peer-a",
                "username": "mesh-user",
                "overlayEndpoint": "127.0.0.1:50305",
                "certificateSha256": "00".repeat(32)
            }]),
            serde_json::json!([
                {
                    "peerId": "peer-a",
                    "username": "mesh-user",
                    "overlayEndpoint": "127.0.0.1:50305",
                    "certificateSha256": "11".repeat(32)
                },
                {
                    "peerId": "MESH-USER",
                    "username": "other-user",
                    "overlayEndpoint": "127.0.0.1:50306",
                    "certificateSha256": "22".repeat(32)
                }
            ]),
        ] {
            super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSKR_TRUSTED_MESH_PEERS", &value.to_string()),
            )
            .expect_err("invalid trusted mesh peer must fail");
        }
    }

    #[test]
    fn soulseek_profile_and_distributed_defaults_match_frozen_targets() {
        let config =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &MapEnv::default())
                .unwrap();

        assert_eq!(config.user_info_picture, None);
        assert_eq!(
            config.soulseek_diagnostic_level,
            super::SoulseekDiagnosticLevel::Info
        );
        assert_eq!(
            config.soulseek_distributed,
            super::SoulseekDistributedSettings {
                disabled: false,
                disable_children: false,
                child_limit: 25,
                logging: false,
            }
        );
    }

    #[test]
    fn soulseek_profile_and_distributed_layers_apply_in_one_contract() {
        let root = std::env::temp_dir().join(format!(
            "slskr-soulseek-profile-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let picture = root.join("picture.bin");
        std::fs::write(&picture, [0_u8, 1, 2, 255]).unwrap();
        let file: super::FileConfig = serde_yaml::from_str(&format!(
            "profile:\n  user_info_picture: {}\n  soulseek_diagnostic_level: warning\nnetwork:\n  distributed_network:\n    disabled: true\n    disable_children: true\n    child_limit: 7\n    logging: true\n",
            picture.display()
        ))
        .unwrap();
        let file_config = super::AppConfig::from_layers(None, file, &MapEnv::default()).unwrap();
        assert_eq!(
            file_config.user_info_picture.as_deref(),
            Some(picture.as_path())
        );
        assert_eq!(
            file_config.soulseek_diagnostic_level,
            super::SoulseekDiagnosticLevel::Warning
        );
        assert_eq!(
            file_config.soulseek_distributed,
            super::SoulseekDistributedSettings {
                disabled: true,
                disable_children: true,
                child_limit: 7,
                logging: true,
            }
        );

        let environment = MapEnv::default()
            .with("SLSK_PICTURE", picture.to_str().unwrap())
            .with("SLSK_DIAG_LEVEL", "debug")
            .with("SLSK_NO_DNET", "false")
            .with("SLSK_DNET_NO_CHILDREN", "false")
            .with("SLSK_DNET_CHILDREN", "31")
            .with("SLSK_DNET_LOGGING", "false");
        let environment_config =
            super::AppConfig::from_layers(None, super::FileConfig::default(), &environment)
                .unwrap();
        assert_eq!(
            environment_config.soulseek_diagnostic_level,
            super::SoulseekDiagnosticLevel::Debug
        );
        assert_eq!(
            environment_config.soulseek_distributed,
            super::SoulseekDistributedSettings {
                disabled: false,
                disable_children: false,
                child_limit: 31,
                logging: false,
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn soulseek_profile_and_distributed_validation_is_bounded() {
        for level in ["", "INFOO"] {
            let error = super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSK_DIAG_LEVEL", level),
            )
            .expect_err("invalid diagnostic level must fail");
            assert!(error.contains("diagnostic"), "{error}");
        }
        let trace = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_DIAG_LEVEL", "trace"),
        )
        .expect("the frozen runtime enum accepts trace");
        assert_eq!(
            trace.soulseek_diagnostic_level,
            super::SoulseekDiagnosticLevel::Trace
        );
        for limit in ["0".to_owned(), (i64::from(i32::MAX) + 1).to_string()] {
            super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with("SLSK_DNET_CHILDREN", &limit),
            )
            .expect_err("invalid distributed child limit must fail");
        }

        let missing =
            std::env::temp_dir().join(format!("slskr-missing-picture-{}", std::process::id()));
        super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_PICTURE", missing.to_str().unwrap()),
        )
        .expect_err("missing picture must fail");
        super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSK_PICTURE", std::env::temp_dir().to_str().unwrap()),
        )
        .expect_err("picture directory must fail");
    }

    #[test]
    fn daemon_foundation_contracts_load_as_one_runtime_policy() {
        let root = std::env::temp_dir().join(format!(
            "slskr-daemon-foundation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let content_name = format!("slskr-foundation-wwwroot-{}", std::process::id());
        let content = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(&content_name);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&content).unwrap();
        let socket = root.join("slskr.sock");
        std::fs::write(
            root.join("slskd.yml"),
            format!(
                r#"flags:
  force_migrations: true
  legacy_windows_tcp_keepalive: true
  log_sql: true
  log_unobserved_exceptions: true
  optimistic_relay_file_info: true
  volatile: true
logger:
  disk: true
  loki: https://loki.example
  no_color: true
permissions:
  file:
    mode: "0640"
telemetry:
  tracing:
    enabled: true
    exporter: jaeger
    jaeger_endpoint: collector.example
    jaeger_port: 4318
    otlp_endpoint: https://otlp.example
retention:
  search: 10
  logs: 9
  files:
    complete: 30
    incomplete: 31
  transfers:
    upload: {{succeeded: 5, errored: 6, cancelled: 7, failed: 8}}
    download: {{succeeded: 9, errored: 10, cancelled: 11, failed: 12}}
filters:
  search_retention:
    max_age_days: 4
    max_count: 77
    cleanup_interval_seconds: 3600
web:
  socket: "{}"
  url_base: /slsk
  content_path: "{}"
  logging: true
  https:
    disabled: true
    port: 5443
    force: true
  authentication:
    api_keys:
      operator:
        key: 0123456789abcdef
        role: readwrite
        cidr: 127.0.0.1/32
"#,
                socket.display(),
                content_name,
            ),
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default().with("SLSKD_APP_DIR", root.to_str().unwrap()),
        )
        .unwrap();
        assert!(config.daemon_flags.force_migrations);
        assert!(config.daemon_flags.legacy_windows_tcp_keepalive);
        assert!(config.daemon_flags.log_sql);
        assert!(config.daemon_flags.log_unobserved_exceptions);
        assert!(config.daemon_flags.optimistic_relay_file_info);
        assert!(config.daemon_flags.volatile);
        assert_eq!(config.logger.loki.as_deref(), Some("https://loki.example"));
        assert!(config.logger.disk && config.logger.no_color);
        assert_eq!(config.permissions_file_mode.as_deref(), Some("0640"));
        assert!(config.telemetry_tracing.enabled);
        assert_eq!(config.telemetry_tracing.exporter, "jaeger");
        assert_eq!(config.telemetry_tracing.jaeger_port, Some(4318));
        assert_eq!(config.retention.search_minutes, Some(10));
        assert_eq!(config.retention.download.failed_minutes, Some(12));
        assert_eq!(config.search_retention.max_count, 77);
        assert_eq!(config.search_retention.cleanup_interval.as_secs(), 3600);
        assert_eq!(
            config.controller_web.socket.as_deref(),
            Some(socket.as_path())
        );
        assert_eq!(config.controller_web.url_base, "/slsk");
        assert_eq!(config.controller_web.content_path, content);
        assert_eq!(config.controller_web.content_path_display, content_name);
        assert!(config.controller_web.logging);
        assert!(config.controller_web.https.disabled);
        assert!(config.controller_web.https.force);
        assert_eq!(config.controller_web.https.binds[0].port(), 5443);
        let key = &config.controller_api_keys["operator"];
        assert_eq!(key.role, "readwrite");
        assert!(key.cidrs[0].contains("127.0.0.1".parse().unwrap()));
        std::fs::remove_dir_all(&content).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn daemon_foundation_validation_rejects_unsafe_values() {
        for (name, value) in [
            ("SLSKD_FILE_PERMISSION_MODE", "888"),
            ("SLSKR_RETENTION_LOGS", "0"),
            ("SLSKR_RETENTION_SEARCH", "4"),
            ("SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL", "3599"),
            ("SLSKD_TELEMETRY_TRACING_EXPORTER", "zipkin"),
        ] {
            super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(name, value),
            )
            .expect_err("invalid daemon foundation setting must fail startup");
        }
    }

    #[test]
    fn core_workflow_contracts_load_as_one_runtime_policy() {
        let root = std::env::temp_dir().join(format!(
            "slskr-core-workflow-config-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            r#"rooms: [Ambient, Jazz, ambient]
soulseek:
  liked_interests: [Aphex Twin, Ambient]
  hated_interests: [spam]
destinations:
  folders:
    - name: Music
      path: /downloads/music
      default: true
shares:
  cache:
    storage_mode: disk
    workers: 3
    retention: 120
  probe_media_attributes: false
wishlist:
  enabled: false
  interval_seconds: 600
  auto_download: true
  max_results: 250
throttling:
  search:
    incoming:
      concurrency: 4
      circuit_breaker: 600
      response_file_limit: 700
"#,
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .unwrap();
        assert_eq!(config.core_workflow.rooms, ["Ambient", "Jazz"]);
        assert_eq!(
            config.core_workflow.liked_interests,
            ["Aphex Twin", "Ambient"]
        );
        assert_eq!(config.core_workflow.hated_interests, ["spam"]);
        assert_eq!(config.core_workflow.destinations.len(), 1);
        assert!(config.core_workflow.destinations[0].default);
        assert_eq!(config.core_workflow.wishlist.interval.as_secs(), 600);
        assert!(config.core_workflow.wishlist.auto_download);
        assert_eq!(config.core_workflow.wishlist.max_results, 250);
        assert_eq!(config.core_workflow.incoming_search.concurrency, 4);
        assert_eq!(config.core_workflow.incoming_search.circuit_breaker, 600);
        assert_eq!(
            config.core_workflow.incoming_search.response_file_limit,
            700
        );
        assert_eq!(config.share_settings.cache_storage_mode, "disk");
        assert_eq!(config.share_settings.cache_workers, 3);
        assert_eq!(
            config.share_settings.cache_retention.unwrap().as_secs(),
            7_200
        );
        assert!(!config.share_settings.probe_media_attributes);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn core_workflow_validation_rejects_out_of_range_values() {
        for (name, value) in [
            ("SLSKD_SHARE_CACHE_WORKERS", "0"),
            ("SLSKD_SHARE_CACHE_RETENTION", "59"),
            ("SLSKD_WISHLIST_INTERVAL", "299"),
            ("SLSKD_WISHLIST_MAX_RESULTS", "9"),
            ("SLSKD_THROTTLING_SEARCH_INCOMING_CONCURRENCY", "0"),
            ("SLSKD_THROTTLING_SEARCH_INCOMING_CIRCUIT_BREAKER", "99"),
            ("SLSKD_THROTTLING_SEARCH_INCOMING_RESPONSE_FILE_LIMIT", "99"),
        ] {
            super::AppConfig::from_layers(
                None,
                super::FileConfig::default(),
                &MapEnv::default().with(name, value),
            )
            .expect_err("invalid core workflow setting must fail startup");
        }
    }

    #[test]
    fn advanced_networking_security_contracts_load_as_one_runtime_policy() {
        let root = std::env::temp_dir().join(format!(
            "slskr-advanced-networking-config-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            r#"dht:
  enabled: true
  dht_port: 51001
  overlay_port: 51002
  advertised_overlay_port: 51003
  vpn_port_sync: target_port
  bootstrap_routers: [router.example:6881]
  announce_interval_seconds: 120
  discovery_interval_seconds: 90
  min_neighbors: 7
  bootstrap_timeout_seconds: 20
  cold_bootstrap_timeout_seconds: 30
  lan_only_bootstrap_timeout_seconds: 10
  lan_only: true
  enable_upnp: true
  enable_stun: false
mesh:
  enabled: true
  enable_soulseek_capability_handshake: false
  enable_soulseek_rendezvous: true
  probe_soulseek_rendezvous_capabilities: false
  dht: { bootstrap_nodes: 17 }
  overlay: { udp_port: 51004, quic_port: 51005 }
  security: { enforceRemotePayloadLimits: true, maxRemotePayloadSize: 262144 }
  sync_security:
    max_invalid_entries_per_window: 8
    max_invalid_messages_per_window: 4
    rate_limit_window_minutes: 2
    quarantine_violation_threshold: 2
    quarantine_duration_minutes: 11
    proof_of_possession_enabled: true
    consensus_min_peers: 4
    consensus_min_agreements: 2
    alert_threshold_signature_failures: 9
    alert_threshold_rate_limit_violations: 8
    alert_threshold_quarantine_events: 7
PodCore:
  Join: { SignatureMode: warn }
  Security: { SignatureMode: enforce }
overlay:
  enable: true
  listen_port: 51006
  enable_quic: true
  quic_listen_port: 51007
  share_quic_with_dht_port: false
  quic_backend_listen_port: 51008
  trusted_certificate_pins: { "127.0.0.1:51007": [pin-value] }
overlay_data:
  enable: true
  listen_port: 51009
  relay_authentication_token: overlay-token
  allowed_relay_destinations: ["8.8.8.8:443"]
  max_concurrent_relays: 3
  max_relay_bytes_per_direction: 123456
  max_relay_duration_seconds: 45
  trusted_certificate_pins: { "127.0.0.1:51009": [data-pin] }
relay:
  enabled: true
  mode: controller
  controller:
    address: https://controller.example
    ignore_certificate_errors: true
    api_key: 1234567890abcdef
    secret: abcdef1234567890
    downloads: true
  agents:
    edge:
      instance_name: edge-one
      secret: 0123456789abcdef
      cidr: 127.0.0.1/32
security:
  enabled: true
  profile: Custom
  network_guard:
    enabled: true
    max_connections_per_ip: 12
    max_global_connections: 345
    max_messages_per_minute: 67
    max_message_size: 8192
  path_guard: { enabled: true, max_path_length: 333, max_path_depth: 13 }
  content_safety:
    enabled: true
    verify_magic_bytes: false
    quarantine_suspicious: false
    quarantine_directory: /tmp/quarantine
    block_executables: false
  peer_reputation: { enabled: true, trusted_threshold: 80, untrusted_threshold: 10 }
  violation_tracker: { enabled: true, violations_before_auto_ban: 3, base_ban_duration_minutes: 15 }
  adversarial:
    privacy: { padding: { max_unpadded_bytes: 1024, max_padded_bytes: 2048 } }
    anonymity:
      relay_only:
        relay_peer_data_endpoints: ["8.8.4.4:443"]
        relay_authentication_token: anonymity-token
"#,
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .unwrap();
        let advanced = &config.advanced_networking;
        assert_eq!(advanced.dht.dht_port, 51_001);
        assert_eq!(advanced.dht.effective_overlay_port(), 51_003);
        assert_eq!(advanced.dht.vpn_port_sync, "target_port");
        assert!(advanced.dht.lan_only);
        assert_eq!(advanced.mesh.dht_bootstrap_nodes, 17);
        assert!(!advanced.mesh.enable_soulseek_capability_handshake);
        assert_eq!(advanced.mesh.max_remote_payload_size, 262_144);
        assert_eq!(advanced.mesh_sync_security.consensus_min_agreements, 2);
        assert_eq!(
            advanced.pod_join_signature_mode,
            super::PodSignatureMode::Warn
        );
        assert_eq!(
            advanced.pod_security_signature_mode,
            super::PodSignatureMode::Enforce
        );
        assert_eq!(advanced.overlay.quic_backend_listen_port, 51_008);
        assert_eq!(advanced.overlay_data.max_concurrent_relays, 3);
        assert!(advanced.relay.enabled);
        assert_eq!(advanced.relay.agents["edge"].instance_name, "edge-one");
        assert_eq!(advanced.security.network_guard.max_global_connections, 345);
        assert_eq!(advanced.security.path_guard.max_path_depth, 13);
        assert_eq!(advanced.security.peer_reputation.trusted_threshold, 80);
        assert_eq!(advanced.security.adversarial.max_padded_bytes, 2048);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn advanced_networking_validation_rejects_inconsistent_security_limits() {
        let root = std::env::temp_dir().join(format!(
            "slskr-advanced-networking-invalid-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "Mesh:\n  sync_security:\n    consensus_min_peers: 2\n    consensus_min_agreements: 3\n",
        )
        .unwrap();
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .expect_err("inconsistent consensus must fail");
        assert!(error.contains("sync_security"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn media_advanced_service_contracts_load_as_one_runtime_policy() {
        let root = std::env::temp_dir().join(format!(
            "slskr-media-advanced-service-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            r#"feature:
  collectionsSharing: false
  streaming: false
  streamingRelayFallback: false
  meshParallelSearch: false
  meshPublishAvailability: false
  identityFriends: false
  solid: false
  scenePodBridge: true
  scenePodBridgeOptions:
    proxyTransfers: true
    exportPodAvailability: true
  songId: false
  mesh: false
  dht: false
  pods: false
  socialFederation: false
  virtualSoulfind: false
  multiSourceDownloads: false
player:
  external_visualizer:
    enabled: true
    path: /bin/echo
    arguments: [visualizer, --fixture]
    working_directory: /tmp
    name: Fixture Visualizer
solid:
  allowInsecureHttp: true
  maxFetchBytes: 7654321
  timeoutSeconds: 23
  allowedHosts: [Pod.Example., identity.example]
  redirectPath: /fixture/callback
song_id:
  max_concurrent_runs: 7
virtualSoulfind:
  bridge:
    enabled: true
    port: 4322
    bindAddress: 127.0.0.2
    maxClients: 17
    requireAuth: true
    password: fixture-secret
    maxRequestsPerMinute: 71
    maxTransfersPerSession: 19
  disasterMode:
    auto: true
    force: true
    unavailableThresholdMinutes: 13
    enableGracefulDegradation: false
    recoveryCheckIntervalMinutes: 11
    recoveryHealthyChecksRequired: 5
"#,
        )
        .unwrap();
        let config = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .unwrap();
        let media = &config.media_services;
        let feature = &media.features;
        assert!(!feature.collections_sharing);
        assert!(!feature.streaming);
        assert!(!feature.streaming_relay_fallback);
        assert!(!feature.mesh_parallel_search);
        assert!(!feature.mesh_publish_availability);
        assert!(!feature.identity_friends);
        assert!(!feature.solid);
        assert!(feature.scene_pod_bridge);
        assert!(feature.scene_pod_bridge_proxy_transfers);
        assert!(feature.scene_pod_bridge_export_pod_availability);
        assert!(!feature.song_id);
        assert!(!feature.mesh);
        assert!(!feature.dht);
        assert!(!feature.pods);
        assert!(!feature.social_federation);
        assert!(!feature.virtual_soulfind);
        assert!(!feature.multi_source_downloads);

        let visualizer = &media.external_visualizer;
        assert!(visualizer.launch_enabled);
        assert_eq!(visualizer.command.as_deref(), Some("/bin/echo"));
        assert_eq!(visualizer.arguments, ["visualizer", "--fixture"]);
        assert_eq!(
            visualizer.working_directory.as_deref(),
            Some(std::path::Path::new("/tmp"))
        );
        assert_eq!(visualizer.name, "Fixture Visualizer");

        assert!(media.solid.allow_insecure_http);
        assert_eq!(media.solid.max_fetch_bytes, 7_654_321);
        assert_eq!(media.solid.timeout.as_secs(), 23);
        assert_eq!(
            media.solid.allowed_hosts,
            ["pod.example", "identity.example"]
        );
        assert_eq!(media.solid.redirect_path, "/fixture/callback");
        assert_eq!(media.song_id_max_concurrent_runs, 7);

        let bridge = &media.virtual_soulfind.bridge;
        assert!(bridge.enabled);
        assert_eq!(bridge.port, 4322);
        assert_eq!(
            bridge.bind_address,
            "127.0.0.2".parse::<std::net::IpAddr>().unwrap()
        );
        assert_eq!(bridge.max_clients, 17);
        assert!(bridge.require_auth);
        assert_eq!(bridge.password, "fixture-secret");
        assert_eq!(bridge.max_requests_per_minute, 71);
        assert_eq!(bridge.max_transfers_per_session, 19);

        let disaster = &media.virtual_soulfind.disaster_mode;
        assert!(disaster.auto);
        assert!(disaster.force);
        assert_eq!(disaster.unavailable_threshold.as_secs(), 13 * 60);
        assert!(!disaster.enable_graceful_degradation);
        assert_eq!(disaster.recovery_check_interval.as_secs(), 11 * 60);
        assert_eq!(disaster.recovery_healthy_checks_required, 5);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn media_advanced_service_validation_rejects_unsafe_values() {
        let root = std::env::temp_dir().join(format!(
            "slskr-media-advanced-service-invalid-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("slskd.yml"),
            "virtualSoulfind:\n  bridge:\n    enabled: true\n    requireAuth: true\n    password: ''\n",
        )
        .unwrap();
        let error = super::AppConfig::from_layers(
            None,
            super::FileConfig::default(),
            &MapEnv::default()
                .with("SLSKD_APP_DIR", root.to_str().unwrap())
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", "slskdn"),
        )
        .expect_err("an enabled authenticated bridge requires a password");
        assert!(error.contains("virtualSoulfind.bridge"), "{error}");
        std::fs::remove_dir_all(root).unwrap();
    }
}
