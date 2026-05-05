use std::{env, fs, net::SocketAddr, path::PathBuf, time::Duration};

use serde::Deserialize;
use slskr_client::{
    protocol::peer::FileEntry,
    server::LoginCredentials,
    version::{DEFAULT_LISTEN_PORT, DEFAULT_SERVER_ADDRESS},
};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub config_file: Option<PathBuf>,
    pub http_bind: SocketAddr,
    pub state_dir: PathBuf,
    pub server_address: String,
    pub listen_port: u32,
    pub username: Option<String>,
    pub password: Option<String>,
    pub auto_connect: bool,
    pub reconnect: bool,
    pub reconnect_delay: Duration,
    pub ping_interval: Duration,
    pub listener_bind: Option<String>,
    pub advertised_port: u32,
    pub obfuscated_listener_bind: Option<String>,
    pub obfuscated_advertised_port: Option<u32>,
    pub user_info_description: String,
    pub peer_response_timeout: Duration,
    pub share_settings: ShareSettings,
    pub transfer_history_limit: usize,
    pub transfer_max_active: usize,
    pub transfer_allow_inbound: bool,
    pub transfer_allow_outbound: bool,
    pub auth_required: bool,
    pub api_token: Option<String>,
    pub api_rate_limit_anonymous: u32,
    pub api_rate_limit_authenticated: u32,
    pub persistence_enabled: bool,
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
        let auto_connect = env_bool_layer(
            env,
            "SLSKR_AUTO_CONNECT",
            file_config
                .app
                .auto_connect
                .unwrap_or(username.is_some() && password.is_some()),
        )?;
        let reconnect = env_bool_layer(
            env,
            "SLSKR_RECONNECT",
            file_config.app.reconnect.unwrap_or(auto_connect),
        )?;
        let reconnect_delay = Duration::from_secs(env_parse_layer(
            env,
            "SLSKR_RECONNECT_SECONDS",
            file_config.app.reconnect_seconds,
            30,
        )?);
        let ping_interval = Duration::from_secs(env_parse_layer(
            env,
            "SLSKR_PING_SECONDS",
            file_config.app.ping_seconds,
            300,
        )?);
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
        let user_info_description = env
            .var("SLSKR_USER_INFO_DESCRIPTION")
            .or(file_config.profile.user_info_description)
            .unwrap_or_else(|| "slskr daemon".to_owned());
        let peer_response_timeout = Duration::from_secs(env_parse_layer(
            env,
            "SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS",
            file_config.timeouts.peer_response_seconds,
            5,
        )?);
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
        let api_token = env.var("SLSKR_API_TOKEN").or(file_config.auth.api_token);
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
        let persistence_enabled = env_bool_layer(
            env,
            "SLSKR_PERSISTENCE_ENABLED",
            file_config.persistence.enabled.unwrap_or(false),
        )?;

        Ok(Self {
            config_file,
            http_bind,
            state_dir,
            server_address,
            listen_port,
            username,
            password,
            auto_connect,
            reconnect,
            reconnect_delay,
            ping_interval,
            listener_bind,
            advertised_port,
            obfuscated_listener_bind,
            obfuscated_advertised_port,
            user_info_description,
            peer_response_timeout,
            share_settings,
            transfer_history_limit,
            transfer_max_active,
            transfer_allow_inbound,
            transfer_allow_outbound,
            auth_required,
            api_token,
            api_rate_limit_anonymous,
            api_rate_limit_authenticated,
            persistence_enabled,
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
            "{{\"config_file\":{},\"http_bind\":\"{}\",\"state_dir\":\"{}\",\"server_address\":\"{}\",\"listen_port\":{},\"advertised_port\":{},\"listener_bind\":{},\"obfuscated_listener_bind\":{},\"obfuscated_advertised_port\":{},\"username\":{},\"credentials_configured\":{},\"auto_connect\":{},\"reconnect\":{},\"reconnect_seconds\":{},\"ping_seconds\":{},\"peer_response_timeout_seconds\":{},\"share_roots\":{},\"share_follow_symlinks\":{},\"share_include_hidden\":{},\"share_scan_max_files\":{},\"transfer_history_limit\":{},\"transfer_max_active\":{},\"transfer_allow_inbound\":{},\"transfer_allow_outbound\":{},\"auth_required\":{},\"api_token_configured\":{},\"persistence_enabled\":{}}}",
            json_option(
                self.config_file
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .as_deref()
            ),
            json_escape(&self.http_bind.to_string()),
            json_escape(&self.state_dir.display().to_string()),
            json_escape(&self.server_address),
            self.listen_port,
            self.advertised_port,
            json_option(self.listener_bind.as_deref()),
            json_option(self.obfuscated_listener_bind.as_deref()),
            json_u32_option(self.obfuscated_advertised_port),
            json_option(self.username.as_deref().map(redact_username).as_deref()),
            self.username.is_some() && self.password.is_some(),
            self.auto_connect,
            self.reconnect,
            self.reconnect_delay.as_secs(),
            self.ping_interval.as_secs(),
            self.peer_response_timeout.as_secs(),
            self.share_settings.roots.len(),
            self.share_settings.follow_symlinks,
            self.share_settings.include_hidden,
            self.share_settings.max_files,
            self.transfer_history_limit,
            self.transfer_max_active,
            self.transfer_allow_inbound,
            self.transfer_allow_outbound,
            self.auth_required,
            self.api_token.is_some(),
            self.persistence_enabled
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkFileConfig {
    server_address: Option<String>,
    listen_port: Option<u32>,
    username: Option<String>,
    password: Option<String>,
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
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferFileConfig {
    history_limit: Option<usize>,
    max_active: Option<usize>,
    allow_inbound: Option<bool>,
    allow_outbound: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthFileConfig {
    disabled: Option<bool>,
    api_token: Option<String>,
    rate_limit_anonymous: Option<u32>,
    rate_limit_authenticated: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceFileConfig {
    enabled: Option<bool>,
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
    let body = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read config file {}: {error}", path.display()))?;
    let config = toml::from_str::<FileConfig>(&body)
        .map_err(|error| format!("failed to parse config file {}: {error}", path.display()))?;
    Ok((Some(path), config))
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
        size,
        extension: extension_for(filename.trim()),
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
