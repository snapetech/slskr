use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DaemonFlagsSettings {
    pub force_migrations: bool,
    pub legacy_windows_tcp_keepalive: bool,
    pub log_sql: bool,
    pub log_unobserved_exceptions: bool,
    pub optimistic_relay_file_info: bool,
    pub volatile: bool,
}

impl DaemonFlagsSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        force_migrations: Option<bool>,
        legacy_windows_tcp_keepalive: Option<bool>,
        log_sql: Option<bool>,
        log_unobserved_exceptions: Option<bool>,
        optimistic_relay_file_info: Option<bool>,
        volatile: Option<bool>,
        env: &E,
    ) -> Result<Self, String> {
        Ok(Self {
            force_migrations: env_bool_layer(
                env,
                "SLSKD_FORCE_MIGRATIONS",
                force_migrations.unwrap_or(false),
            )?,
            legacy_windows_tcp_keepalive: env_bool_layer(
                env,
                "SLSKD_LEGACY_WINDOWS_TCP_KEEPALIVE",
                legacy_windows_tcp_keepalive.unwrap_or(false),
            )?,
            log_sql: env_bool_layer(env, "SLSKD_LOG_SQL", log_sql.unwrap_or(false))?,
            log_unobserved_exceptions: env_bool_layer(
                env,
                "SLSKD_LOG_UNOBSERVED_EXCEPTIONS",
                log_unobserved_exceptions.unwrap_or(false),
            )?,
            optimistic_relay_file_info: env_bool_layer(
                env,
                "SLSKD_OPTIMISTIC_RELAY_FILE_INFO",
                optimistic_relay_file_info.unwrap_or(false),
            )?,
            volatile: env_bool_layer(env, "SLSKD_VOLATILE", volatile.unwrap_or(false))?,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LoggerSettings {
    pub disk: bool,
    pub loki: Option<String>,
    pub no_color: bool,
}

impl LoggerSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        disk: Option<bool>,
        loki: Option<String>,
        no_color: Option<bool>,
        env: &E,
    ) -> Result<Self, String> {
        let loki = env
            .var("SLSKD_LOKI")
            .or(loki)
            .filter(|value| !value.trim().is_empty());
        if loki
            .as_deref()
            .is_some_and(|value| !(value.starts_with("http://") || value.starts_with("https://")))
        {
            return Err("logger.loki must be an http:// or https:// URL".to_owned());
        }
        Ok(Self {
            disk: env_bool_layer(env, "SLSKD_DISK_LOGGER", disk.unwrap_or(false))?,
            loki,
            no_color: env_bool_layer(env, "SLSKD_NO_COLOR", no_color.unwrap_or(false))?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryTracingSettings {
    pub enabled: bool,
    pub exporter: String,
    pub jaeger_endpoint: Option<String>,
    pub jaeger_port: Option<u16>,
    pub otlp_endpoint: Option<String>,
}

impl TelemetryTracingSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        enabled: Option<bool>,
        exporter: Option<String>,
        jaeger_endpoint: Option<String>,
        jaeger_port: Option<u16>,
        otlp_endpoint: Option<String>,
        env: &E,
    ) -> Result<Self, String> {
        let exporter = env
            .var("SLSKD_TELEMETRY_TRACING_EXPORTER")
            .or(exporter)
            .unwrap_or_else(|| "console".to_owned())
            .to_ascii_lowercase();
        if !matches!(exporter.as_str(), "console" | "jaeger" | "otlp") {
            return Err("telemetry.tracing.exporter must be console, jaeger, or otlp".to_owned());
        }
        Ok(Self {
            enabled: env_bool_layer(env, "SLSKD_TELEMETRY_TRACING", enabled.unwrap_or(false))?,
            exporter,
            jaeger_endpoint: env
                .var("SLSKD_TELEMETRY_JAEGER_ENDPOINT")
                .or(jaeger_endpoint)
                .filter(|value| !value.trim().is_empty()),
            jaeger_port: match env.var("SLSKD_TELEMETRY_JAEGER_PORT") {
                Some(value) => Some(
                    value
                        .parse::<u16>()
                        .map_err(|error| format!("invalid SLSKD_TELEMETRY_JAEGER_PORT: {error}"))?,
                ),
                None => jaeger_port,
            },
            otlp_endpoint: env
                .var("SLSKD_TELEMETRY_OTLP_ENDPOINT")
                .or(otlp_endpoint)
                .filter(|value| !value.trim().is_empty()),
        })
    }
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

impl RetentionSettings {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn from_layers<E: ConfigEnv>(
        search: Option<u64>,
        logs: Option<u64>,
        files_complete: Option<u64>,
        files_incomplete: Option<u64>,
        upload: TransferTypeRetentionFileConfig,
        download: TransferTypeRetentionFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let settings = Self {
            search_minutes: env_parse_option_layer(env, "SLSKR_RETENTION_SEARCH", search)?,
            logs_days: env_parse_layer(env, "SLSKR_RETENTION_LOGS", logs, 180_u64)?,
            files_complete_minutes: env_parse_option_layer(
                env,
                "SLSKR_RETENTION_FILES_COMPLETE",
                files_complete,
            )?,
            files_incomplete_minutes: env_parse_option_layer(
                env,
                "SLSKR_RETENTION_FILES_INCOMPLETE",
                files_incomplete,
            )?,
            upload: TransferRetentionSettings {
                succeeded_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_SUCCEEDED",
                    upload.succeeded,
                )?,
                errored_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_ERRORED",
                    upload.errored,
                )?,
                cancelled_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_CANCELLED",
                    upload.cancelled,
                )?,
                failed_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_UPLOAD_FAILED",
                    upload.failed,
                )?,
            },
            download: TransferRetentionSettings {
                succeeded_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_SUCCEEDED",
                    download.succeeded,
                )?,
                errored_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_ERRORED",
                    download.errored,
                )?,
                cancelled_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_CANCELLED",
                    download.cancelled,
                )?,
                failed_minutes: env_parse_option_layer(
                    env,
                    "SLSKR_RETENTION_DOWNLOAD_FAILED",
                    download.failed,
                )?,
            },
        };
        if settings.logs_days < 1 {
            return Err("retention.logs must be at least 1 day".to_owned());
        }
        for (name, value, minimum) in [
            ("retention.search", settings.search_minutes, 5),
            (
                "retention.files.complete",
                settings.files_complete_minutes,
                30,
            ),
            (
                "retention.files.incomplete",
                settings.files_incomplete_minutes,
                30,
            ),
            (
                "retention.transfers.upload.succeeded",
                settings.upload.succeeded_minutes,
                5,
            ),
            (
                "retention.transfers.upload.errored",
                settings.upload.errored_minutes,
                5,
            ),
            (
                "retention.transfers.upload.cancelled",
                settings.upload.cancelled_minutes,
                5,
            ),
            (
                "retention.transfers.upload.failed",
                settings.upload.failed_minutes,
                5,
            ),
            (
                "retention.transfers.download.succeeded",
                settings.download.succeeded_minutes,
                5,
            ),
            (
                "retention.transfers.download.errored",
                settings.download.errored_minutes,
                5,
            ),
            (
                "retention.transfers.download.cancelled",
                settings.download.cancelled_minutes,
                5,
            ),
            (
                "retention.transfers.download.failed",
                settings.download.failed_minutes,
                5,
            ),
        ] {
            if value.is_some_and(|value| value < minimum) {
                return Err(format!("{name} must be at least {minimum} minutes"));
            }
        }
        Ok(settings)
    }
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

impl SearchRetentionSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        cleanup_interval_seconds: Option<u64>,
        max_age_days: Option<u64>,
        max_count: Option<usize>,
        env: &E,
    ) -> Result<Self, String> {
        let cleanup_interval_seconds = env_parse_layer(
            env,
            "SLSKD_SEARCH_RETENTION_CLEANUP_INTERVAL",
            cleanup_interval_seconds,
            86_400_u64,
        )?;
        if cleanup_interval_seconds < 3_600 {
            return Err(
                "filters.search_retention.cleanup_interval_seconds must be at least 3600"
                    .to_owned(),
            );
        }
        Ok(Self {
            max_age_days: env_parse_layer(
                env,
                "SLSKD_SEARCH_RETENTION_MAX_AGE_DAYS",
                max_age_days,
                30_u64,
            )?,
            max_count: env_parse_layer(
                env,
                "SLSKD_SEARCH_RETENTION_MAX_COUNT",
                max_count,
                1_000_usize,
            )?,
            cleanup_interval: Duration::from_secs(cleanup_interval_seconds),
        })
    }
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

impl CoreWorkflowSettings {
    pub(super) fn from_layers<E: ConfigEnv>(env: &E) -> Result<Self, String> {
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
        Ok(Self {
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
        })
    }
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
    pub auto_acknowledge_private_messages: bool,
    pub auto_acknowledge_privilege_notifications: bool,
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

    pub(super) fn parse(value: &str) -> Result<Self, String> {
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
    pub auto_replace_max_retries: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DownloadFilterSettings {
    pub exclude: Vec<String>,
}

impl DownloadFilterSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: &DownloadFiltersFileConfig,
        env: &E,
        current_upstream_behavior: bool,
    ) -> Result<Self, String> {
        const MAX_EXCLUSIONS: usize = 100;
        const MAX_EXCLUSION_LENGTH: usize = 256;

        let exclude_names = if current_upstream_behavior {
            vec![
                "SLSKR_DOWNLOAD_FILTER_EXCLUDE",
                "DOWNLOAD_FILTER_EXCLUDE",
                "SLSKD_DOWNLOAD_FILTER_EXCLUDE",
            ]
        } else {
            vec!["SLSKD_DOWNLOAD_FILTER_EXCLUDE"]
        };
        let exclude = string_array_any_layer(env, &exclude_names, file.exclude.clone());
        if exclude.len() > MAX_EXCLUSIONS {
            return Err(format!(
                "filters.download.exclude supports at most {MAX_EXCLUSIONS} exclusions"
            ));
        }
        for (index, value) in exclude.iter().enumerate() {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(format!(
                    "filters.download.exclude entry {index} must not be blank"
                ));
            }
            if trimmed.len() > MAX_EXCLUSION_LENGTH {
                return Err(format!(
                    "filters.download.exclude entry {index} exceeds {MAX_EXCLUSION_LENGTH} characters"
                ));
            }
        }
        Ok(Self {
            exclude: exclude
                .into_iter()
                .map(|value| value.trim().to_owned())
                .collect(),
        })
    }
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
    pub blacklisted: BlacklistedGroupSettings,
    pub user_defined: BTreeMap<String, UserDefinedTransferGroupSettings>,
}

/// Matches the oracle's real `Groups.Blacklisted`: a user in this group is
/// classified as "blacklisted" ahead of every other group (including
/// privileged), by `UserService.GetGroup`/`IsBlacklisted`. Only the
/// exact-username `members` list is enforced for now -- the oracle's
/// `patterns` (regex against username) and `cidrs` (IP-range) checks are
/// a separate, deferred follow-up.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlacklistedGroupSettings {
    pub members: Vec<String>,
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

    pub(super) fn parse(value: &str) -> Result<Self, String> {
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
    pub(super) fn defaults(target: ControllerProfile) -> Self {
        Self {
            enabled: target == ControllerProfile::Native,
            api_permit_limit: 200,
            api_window_seconds: 60,
            federation_permit_limit: 30,
            federation_window_seconds: 60,
            mesh_gateway_permit_limit: 60,
            mesh_gateway_window_seconds: 60,
        }
    }
}
