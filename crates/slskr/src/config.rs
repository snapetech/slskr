use std::{
    collections::BTreeMap,
    env, fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use serde::Deserialize;
use slskr_client::{
    protocol::peer::FileEntry,
    server::LoginCredentials,
    version::{DEFAULT_LISTEN_PORT, DEFAULT_SERVER_ADDRESS},
};

const MAX_CONFIG_FILE_BYTES: u64 = 1024 * 1024;
const MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES: usize = 4 * 1024;
const MAX_COMPLETED_PATH_TEMPLATE_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub config_file: Option<PathBuf>,
    pub http_bind: SocketAddr,
    pub state_dir: PathBuf,
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
    pub listener_bind: Option<String>,
    pub advertised_port: u32,
    pub obfuscated_listener_bind: Option<String>,
    pub obfuscated_advertised_port: Option<u32>,
    pub obfuscation_enabled: bool,
    pub obfuscation_mode: SoulseekObfuscationMode,
    pub obfuscation_prefer_outbound: bool,
    pub peer_host_override: Option<Ipv4Addr>,
    pub test_user_endpoint_overrides: BTreeMap<String, SocketAddr>,
    pub user_info_description: String,
    pub peer_response_timeout: Duration,
    pub share_settings: ShareSettings,
    pub transfer_history_limit: usize,
    pub transfer_max_active: usize,
    pub transfer_allow_inbound: bool,
    pub transfer_allow_outbound: bool,
    pub transfer_auto_retry: TransferAutoRetrySettings,
    pub download_completed_path_template: String,
    pub private_message_auto_response: PrivateMessageAutoResponseSettings,
    pub pod_join_signature_mode: PodSignatureMode,
    pub auth_required: bool,
    pub api_token: Option<String>,
    pub api_cookie_auth_enabled: bool,
    pub api_rate_limit_anonymous: u32,
    pub api_rate_limit_authenticated: u32,
    pub trusted_proxy_cidrs: Vec<TrustedProxyCidr>,
    pub persistence_enabled: bool,
    pub integrations: IntegrationSettings,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        let (config_file, file_config) = load_file_config()?;
        Self::from_layers(config_file, file_config, &ProcessEnv)
    }

    pub fn from_layers<E: ConfigEnv>(
        config_file: Option<PathBuf>,
        file_config: FileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let http_bind = env
            .var("SLSKR_HTTP_BIND")
            .or(file_config.app.http_bind)
            .unwrap_or_else(|| "127.0.0.1:5030".to_owned())
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid SLSKR_HTTP_BIND: {error}"))?;
        let state_dir = env
            .var("SLSKR_STATE_DIR")
            .map(PathBuf::from)
            .or(file_config.app.state_dir)
            .unwrap_or_else(default_state_dir);
        let server_address = env
            .var("SLSK_SERVER")
            .or(file_config.network.server_address)
            .unwrap_or_else(|| DEFAULT_SERVER_ADDRESS.to_owned());
        let listen_port = env_parse_layer(
            env,
            "SLSK_LISTEN_PORT",
            file_config.network.listen_port,
            DEFAULT_LISTEN_PORT,
        )?;
        let username = optional_env_any(env, &["SLSK_USERNAME"]).or(file_config.network.username);
        let password = optional_env_any(env, &["SLSK_PASSWORD"]).or(file_config.network.password);
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
        let auto_connect = env_bool_layer(
            env,
            "SLSKR_AUTO_CONNECT",
            file_config.app.auto_connect.unwrap_or(
                username.is_some() && password.is_some() || credential_store.auto_connect_default(),
            ),
        )?;
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
        let listener_bind = env
            .var("SLSKR_LISTENER_BIND")
            .or(file_config.listeners.regular_bind);
        let advertised_port = env_parse_layer(
            env,
            "SLSKR_ADVERTISED_PORT",
            file_config.listeners.advertised_port,
            listen_port,
        )?;
        let obfuscated_listener_bind = env
            .var("SLSKR_OBFUSCATED_LISTENER_BIND")
            .or(file_config.listeners.obfuscated_bind);
        let obfuscated_advertised_port = env_parse_option_layer(
            env,
            "SLSKR_OBFUSCATED_ADVERTISED_PORT",
            file_config.listeners.obfuscated_advertised_port,
        )?;
        let obfuscation_enabled = env_bool_layer(
            env,
            "SLSK_OBFUSCATION",
            file_config.network.obfuscation.enabled.unwrap_or(true),
        )?;
        let obfuscation_mode = SoulseekObfuscationMode::parse(
            env.var("SLSK_OBFUSCATION_MODE")
                .or(file_config.network.obfuscation.mode)
                .as_deref()
                .unwrap_or("compatibility"),
        )?;
        let obfuscation_prefer_outbound = env_bool_layer(
            env,
            "SLSK_OBFUSCATION_PREFER_OUTBOUND",
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
        let user_info_description = env
            .var("SLSKR_USER_INFO_DESCRIPTION")
            .or(file_config.profile.user_info_description)
            .unwrap_or_else(|| "slskr daemon".to_owned());
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
        let share_settings = ShareSettings::from_layers(file_config.shares, env)?;
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
        let transfer_auto_retry =
            TransferAutoRetrySettings::from_layers(file_config.transfers.auto_retry, env)?;
        let download_completed_path_template = optional_env_any(
            env,
            &[
                "SLSKR_DOWNLOAD_COMPLETED_PATH_TEMPLATE",
                "DOWNLOAD_COMPLETED_PATH_TEMPLATE",
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
        )?;
        let api_token = env.var("SLSKR_API_TOKEN").or(file_config.auth.api_token);
        if let Some(token) = api_token.as_deref() {
            validate_api_token(token)?;
        }
        let auth_disabled = env_bool_layer(
            env,
            "SLSKR_AUTH_DISABLED",
            file_config
                .auth
                .disabled
                .unwrap_or_else(|| http_bind.ip().is_loopback() && api_token.is_none()),
        )?;
        let auth_required = !auth_disabled;
        if auth_required && api_token.is_none() {
            return Err(
                "SLSKR_API_TOKEN or [auth].api_token is required when HTTP auth is enabled"
                    .to_owned(),
            );
        }
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
        let integrations = IntegrationSettings::from_layers(file_config.integrations, env)?;

        Ok(Self {
            config_file,
            http_bind,
            state_dir,
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
            listener_bind,
            advertised_port,
            obfuscated_listener_bind,
            obfuscated_advertised_port,
            obfuscation_enabled,
            obfuscation_mode,
            obfuscation_prefer_outbound,
            peer_host_override,
            test_user_endpoint_overrides,
            user_info_description,
            peer_response_timeout,
            share_settings,
            transfer_history_limit,
            transfer_max_active,
            transfer_allow_inbound,
            transfer_allow_outbound,
            transfer_auto_retry,
            download_completed_path_template,
            private_message_auto_response,
            pod_join_signature_mode,
            auth_required,
            api_token,
            api_cookie_auth_enabled,
            api_rate_limit_anonymous,
            api_rate_limit_authenticated,
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

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"config_file\":{},\"http_bind\":\"{}\",\"state_dir\":\"{}\",\"server_address\":\"{}\",\"listen_port\":{},\"advertised_port\":{},\"listener_bind\":{},\"obfuscated_listener_bind\":{},\"obfuscated_advertised_port\":{},\"obfuscation\":{},\"peer_host_override\":{},\"test_user_endpoint_overrides\":{},\"username\":{},\"credentials_configured\":{},\"credential_store\":\"{}\",\"credential_file\":\"{}\",\"auto_connect\":{},\"reconnect\":{},\"reconnect_seconds\":{},\"ping_seconds\":{},\"log_level\":\"{}\",\"peer_response_timeout_seconds\":{},\"share_roots\":{},\"share_follow_symlinks\":{},\"share_include_hidden\":{},\"share_scan_max_files\":{},\"share_cache_tsv_enabled\":{},\"transfer_history_limit\":{},\"transfer_max_active\":{},\"transfer_allow_inbound\":{},\"transfer_allow_outbound\":{},\"transfer_auto_retry\":{},\"download_completed_path_template_configured\":{},\"private_message_auto_response\":{},\"pod_join_signature_mode\":\"{}\",\"auth_required\":{},\"api_token_configured\":{},\"api_cookie_auth_enabled\":{},\"trusted_proxy_cidrs\":{},\"persistence_enabled\":{},\"integrations\":{}}}",
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
            format_args!(
                "{{\"enabled\":{},\"mode\":\"{}\",\"prefer_outbound\":{},\"effective_prefer_outbound\":{}}}",
                self.obfuscation_enabled,
                self.obfuscation_mode.as_str(),
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
            !self.download_completed_path_template.is_empty(),
            self.private_message_auto_response.sanitized_json(),
            self.pod_join_signature_mode.as_str(),
            self.auth_required,
            self.api_token.is_some(),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoulseekObfuscationMode {
    Compatibility,
    Prefer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PodSignatureMode {
    Off,
    Warn,
    Enforce,
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

#[derive(Clone, Debug, Eq, PartialEq)]
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
    pub alternate_source_size_tolerance_percent: u32,
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
            alternate_source_size_tolerance_percent: bounded_config_value(
                "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
                env_parse_layer(
                    env,
                    "SLSKR_TRANSFER_AUTO_RETRY_ALTERNATE_SOURCE_SIZE_TOLERANCE_PERCENT",
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
    pub spotify: SpotifyIntegrationSettings,
    pub lidarr: LidarrIntegrationSettings,
    pub bridge: BridgeIntegrationSettings,
    pub external_visualizer: ExternalVisualizerSettings,
}

impl IntegrationSettings {
    pub fn from_layers<E: ConfigEnv>(
        file_config: IntegrationsFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        Ok(Self {
            spotify: SpotifyIntegrationSettings::from_layers(file_config.spotify, env)?,
            lidarr: LidarrIntegrationSettings::from_layers(file_config.lidarr, env)?,
            bridge: BridgeIntegrationSettings::from_layers(file_config.bridge, env)?,
            external_visualizer: ExternalVisualizerSettings::from_layers(
                file_config.external_visualizer,
                env,
            )?,
        })
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"spotify\":{},\"lidarr\":{},\"bridge\":{},\"external_visualizer\":{}}}",
            self.spotify.sanitized_json(),
            self.lidarr.sanitized_json(),
            self.bridge.sanitized_json(),
            self.external_visualizer.sanitized_json()
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpotifyIntegrationSettings {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub market: String,
    pub scopes: String,
}

impl SpotifyIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: SpotifyFileConfig, env: &E) -> Result<Self, String> {
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_SPOTIFY_ENABLED",
                file_config.enabled.unwrap_or(false),
            )?,
            client_id: env.var("SLSKR_SPOTIFY_CLIENT_ID").or(file_config.client_id),
            client_secret: env
                .var("SLSKR_SPOTIFY_CLIENT_SECRET")
                .or(file_config.client_secret),
            redirect_uri: env
                .var("SLSKR_SPOTIFY_REDIRECT_URI")
                .or(file_config.redirect_uri),
            market: env
                .var("SLSKR_SPOTIFY_MARKET")
                .or(file_config.market)
                .unwrap_or_else(|| "US".to_owned()),
            scopes: env
                .var("SLSKR_SPOTIFY_SCOPES")
                .or(file_config.scopes)
                .unwrap_or_else(|| {
                    "playlist-read-private playlist-read-collaborative user-library-read".to_owned()
                }),
        })
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
            "{{\"enabled\":{},\"client_id_configured\":{},\"client_secret_configured\":{},\"redirect_uri\":null,\"redirect_uri_configured\":{},\"market\":\"{}\",\"scopes\":\"{}\"}}",
            self.enabled,
            self.client_id.is_some(),
            self.client_secret.is_some(),
            self.redirect_uri.is_some(),
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
}

impl LidarrIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: LidarrFileConfig, env: &E) -> Result<Self, String> {
        let url = env.var("SLSKR_LIDARR_URL").or(file_config.url);
        if let Some(url) = url.as_deref() {
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
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_LIDARR_ENABLED",
                file_config.enabled.unwrap_or(false),
            )?,
            url,
            api_key: env.var("SLSKR_LIDARR_API_KEY").or(file_config.api_key),
            timeout_seconds: env_parse_layer(
                env,
                "SLSKR_LIDARR_TIMEOUT_SECONDS",
                file_config.timeout_seconds,
                20_u64,
            )?,
        })
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
            "{{\"enabled\":{},\"url\":null,\"url_configured\":{},\"api_key_configured\":{},\"timeout_seconds\":{}}}",
            self.enabled,
            self.url.is_some(),
            self.api_key.is_some(),
            self.timeout_seconds
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

#[derive(Clone, Debug, Default)]
pub struct ExternalVisualizerSettings {
    pub command: Option<String>,
    pub launch_enabled: bool,
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
        })
    }

    pub fn configured(&self) -> bool {
        self.command
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn launchable(&self) -> bool {
        self.configured() && self.launch_enabled
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"configured\":{},\"launch_enabled\":{},\"command\":null}}",
            self.configured(),
            self.launch_enabled
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
    ) -> Result<Self, String> {
        let enabled = env_bool_layer(
            env,
            "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE",
            file_config.enabled.unwrap_or(false),
        )?;
        let message = env
            .var("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE")
            .or(file_config.message)
            .unwrap_or_else(|| {
                "Hi, I'm human and testing a slskr client. Shares may be temporarily unavailable while I validate the client."
                    .to_owned()
            });
        if message.trim().is_empty() {
            return Err("private-message auto response must not be blank".to_owned());
        }
        if message.len() > MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES {
            return Err(format!(
                "private-message auto response exceeds {MAX_PRIVATE_MESSAGE_AUTO_RESPONSE_BYTES} bytes"
            ));
        }
        let cooldown_minutes = env_parse_layer(
            env,
            "SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_COOLDOWN_MINUTES",
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
            message: message.trim().to_owned(),
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
    pub roots: Vec<PathBuf>,
    pub follow_symlinks: bool,
    pub include_hidden: bool,
    pub max_files: usize,
    pub cache_tsv_enabled: bool,
}

impl ShareSettings {
    pub fn from_layers<E: ConfigEnv>(
        file_config: ShareFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let fixture = env
            .var("SLSKR_SHARE_FIXTURE")
            .or(file_config.fixture)
            .unwrap_or_default();
        let roots = match env.var("SLSKR_SHARE_DIRS") {
            Some(value) => parse_share_dirs(&value),
            None => file_config.dirs.into_iter().map(PathBuf::from).collect(),
        };
        Ok(Self {
            fixture_entries: parse_share_entries(&fixture)?,
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
            cache_tsv_enabled: env_bool_layer(
                env,
                "SLSKR_SHARE_CACHE_TSV_ENABLED",
                file_config.cache_tsv_enabled.unwrap_or(true),
            )?,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    app: AppFileConfig,
    network: NetworkFileConfig,
    listeners: ListenerFileConfig,
    profile: ProfileFileConfig,
    timeouts: TimeoutFileConfig,
    shares: ShareFileConfig,
    transfers: TransferFileConfig,
    auth: AuthFileConfig,
    persistence: PersistenceFileConfig,
    podcore: PodCoreFileConfig,
    integrations: IntegrationsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodCoreFileConfig {
    join: PodJoinFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodJoinFileConfig {
    signature_mode: Option<String>,
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekObfuscationFileConfig {
    enabled: Option<bool>,
    mode: Option<String>,
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileFileConfig {
    user_info_description: Option<String>,
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferFileConfig {
    history_limit: Option<usize>,
    max_active: Option<usize>,
    allow_inbound: Option<bool>,
    allow_outbound: Option<bool>,
    auto_retry: TransferAutoRetryFileConfig,
    completed_path_template: Option<String>,
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
    alternate_source_size_tolerance_percent: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthFileConfig {
    disabled: Option<bool>,
    api_token: Option<String>,
    cookie_auth_enabled: Option<bool>,
    rate_limit_anonymous: Option<u32>,
    rate_limit_authenticated: Option<u32>,
    trusted_proxy_cidrs: Vec<String>,
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
    bridge: BridgeFileConfig,
    external_visualizer: ExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpotifyFileConfig {
    enabled: Option<bool>,
    client_id: Option<String>,
    client_secret: Option<String>,
    redirect_uri: Option<String>,
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
        || config.auth.api_token.is_some()
        || config.integrations.spotify.client_secret.is_some()
        || config.integrations.lidarr.api_key.is_some()
}

pub trait ConfigEnv {
    fn var(&self, name: &str) -> Option<String>;
}

pub struct ProcessEnv;

impl ConfigEnv for ProcessEnv {
    fn var(&self, name: &str) -> Option<String> {
        env::var(name).ok()
    }
}

pub fn optional_env_any(env: &dyn ConfigEnv, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env.var(name))
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

pub fn parse_share_dirs(value: &str) -> Vec<PathBuf> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
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
        assert!(error.contains("not a regular file"));

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

        for (name, value) in [
            ("SLSK_PRIVATE_MESSAGE_AUTO_RESPONSE_MESSAGE", ""),
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
        assert_eq!(retry.alternate_source_size_tolerance_percent, 5);
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
}
