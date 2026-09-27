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

use crate::realm_subject_index::{DEFAULT_GOVERNANCE_ROOT, DEFAULT_REALM_ID};

#[path = "config_file.rs"]
mod file_config;
pub use file_config::FileConfig;
use file_config::*;

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
    pub mesh_gateway: MeshGatewaySettings,
    pub media_services: MediaAdvancedServiceSettings,
    pub social_federation: SocialFederationSettings,
    pub federation_publishing: FederationPublishingSettings,
    pub realm: RealmSettings,
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
    pub distributed_parent_override: Option<SocketAddr>,
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
    pub acquisition_planning_enabled: bool,
    pub controller_profile: ControllerProfile,
    /// Native launches follow current upstream behavior by default. An
    /// explicitly selected profile retains its frozen behavior
    /// unless `SLSKR_PARITY_PROFILE=current` overrides it.
    pub current_upstream_behavior: bool,
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
    pub download_filter: DownloadFilterSettings,
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

#[path = "config_settings.rs"]
mod settings;
pub use settings::*;

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
        let profile_is_explicit = base_env.var("SLSKR_CONTROLLER_PROFILE").is_some()
            || file_config.compatibility.profile.is_some();
        let controller_profile = ControllerProfile::parse(
            base_env
                .var("SLSKR_CONTROLLER_PROFILE")
                .or_else(|| file_config.compatibility.profile.clone())
                .as_deref()
                .unwrap_or("native"),
        )?;
        let current_upstream_behavior = match base_env
            .var("SLSKR_PARITY_PROFILE")
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some("current") => true,
            Some("frozen") => false,
            Some(_) => return Err("SLSKR_PARITY_PROFILE must be current or frozen".to_owned()),
            None => !profile_is_explicit,
        };
        let controller_yaml =
            controller_yaml_environment(&state_dir.join("slskd.yml"), controller_profile)?;
        let layered_env = ControllerYamlEnv {
            base: base_env,
            yaml: controller_yaml,
        };
        let env = &layered_env;
        let auth_disabled = resolve_auth_disabled(env, file_config.auth.disabled.unwrap_or(false))?;
        let mut advanced_networking = AdvancedNetworkingSettings::from_layers(
            &file_config,
            env,
            controller_profile,
            current_upstream_behavior,
            &state_dir,
        )?;
        let mesh_gateway = MeshGatewaySettings::from_layers(&file_config.mesh_gateway, env)?;
        let media_services =
            MediaAdvancedServiceSettings::from_layers(&file_config, env, controller_profile)?;
        let social_federation =
            SocialFederationSettings::from_layers(file_config.social_federation, env)?;
        let federation_publishing =
            FederationPublishingSettings::from_layers(file_config.federation_publishing, env)?;
        let realm = RealmSettings::from_layers(file_config.realm, env)?;
        let controller_headless =
            env_bool_layer(env, "SLSKD_HEADLESS", file_config.headless.unwrap_or(false))?;
        let controller_swagger = env_bool_layer(
            env,
            "SLSKD_SWAGGER",
            file_config
                .feature
                .swagger
                .unwrap_or(controller_profile == ControllerProfile::Native),
        )?;
        let ControllerWebAuthSettings {
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
        } = resolve_controller_web_auth(
            env,
            controller_profile,
            !auth_disabled,
            file_config.metrics.enabled,
            file_config.metrics.url,
            file_config.metrics.authentication.disabled,
            file_config.metrics.authentication.username,
            file_config.metrics.authentication.password,
            file_config.auth.username,
            file_config.auth.password,
            file_config.auth.jwt.key,
            file_config.auth.jwt.ttl,
        )?;
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
            controller_profile,
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
            controller_profile,
            configured_incomplete_dir.is_some(),
        )?;
        let ControllerWebResolution {
            http_bind,
            http_binds,
            controller_http_address,
            controller_web,
        } = resolve_controller_web(
            env,
            controller_profile,
            file_config.app.http_bind,
            file_config.web.socket,
            file_config.web.url_base,
            file_config.web.content_path,
            file_config.web.logging,
            file_config.web.https,
        )?;
        let controller_api_keys =
            resolve_controller_api_keys(env, controller_profile, file_config.auth.api_keys)?;
        let SoulseekIdentity {
            server_address,
            listen_port,
            username,
            password,
            credential_store,
            credential_file,
            auto_connect,
        } = resolve_soulseek_identity(
            env,
            &state_dir,
            file_config.network.server_address,
            file_config.network.listen_port,
            file_config.network.username,
            file_config.network.password,
            file_config.network.credential_store,
            file_config.network.credential_file,
            file_config.app.auto_connect,
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
        let daemon_flags = DaemonFlagsSettings::from_layers(
            file_config.flags.force_migrations,
            file_config.flags.legacy_windows_tcp_keepalive,
            file_config.flags.log_sql,
            file_config.flags.log_unobserved_exceptions,
            file_config.flags.optimistic_relay_file_info,
            file_config.flags.volatile,
            env,
        )?;
        let logger = LoggerSettings::from_layers(
            file_config.logger.disk,
            file_config.logger.loki,
            file_config.logger.no_color,
            env,
        )?;
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
        if controller_profile == ControllerProfile::Legacy && permissions_file_mode.is_some() {
            return Err("The 'permissions' keys have been moved under a new 'destination' key under transfers -> download, and the behavior has changed.  See https://github.com/slskd/slskd/pull/1756 for details".to_owned());
        }
        let telemetry_tracing = TelemetryTracingSettings::from_layers(
            file_config.telemetry.tracing.enabled,
            file_config.telemetry.tracing.exporter,
            file_config.telemetry.tracing.jaeger_endpoint,
            file_config.telemetry.tracing.jaeger_port,
            file_config.telemetry.tracing.otlp_endpoint,
            env,
        )?;
        let retention = RetentionSettings::from_layers(
            file_config.retention.search,
            file_config.retention.logs,
            file_config.retention.files.complete,
            file_config.retention.files.incomplete,
            file_config.retention.transfers.upload,
            file_config.retention.transfers.download,
            env,
        )?;
        let search_retention = SearchRetentionSettings::from_layers(
            file_config
                .filters
                .search_retention
                .cleanup_interval_seconds,
            file_config.filters.search_retention.max_age_days,
            file_config.filters.search_retention.max_count,
            env,
        )?;
        let core_workflow = CoreWorkflowSettings::from_layers(env)?;
        let ListenerAndObfuscationResolution {
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
        } = resolve_listener_and_obfuscation(
            env,
            controller_profile,
            current_upstream_behavior,
            &advanced_networking,
            listen_port,
            auto_connect,
            file_config.listeners.regular_bind,
            file_config.listeners.advertised_port,
            file_config.listeners.obfuscated_bind,
            file_config.listeners.obfuscated_advertised_port,
            file_config.listeners.overlay_bind,
            file_config.mesh.trusted_peers,
            file_config.network.obfuscation.enabled,
            file_config.network.obfuscation.mode,
            file_config.network.obfuscation.advertise_regular_port,
            file_config.network.obfuscation.prefer_outbound,
        )?;
        let current_shared_mesh_tcp = controller_profile == ControllerProfile::Native
            && current_upstream_behavior
            && dht_enabled
            && advanced_networking.mesh.enabled
            && advanced_networking.mesh.enable_dht
            && advanced_networking.mesh.enable_overlay
            && listener_bind
                .as_deref()
                .and_then(|value| value.parse::<SocketAddr>().ok())
                .is_some_and(|bind| overlay_bind == Some(bind));
        if current_shared_mesh_tcp {
            if let Some(bind) = listener_bind
                .as_deref()
                .and_then(|value| value.parse::<SocketAddr>().ok())
            {
                // The current upstream startup path mutates the DHT overlay
                // option to the Soulseek listen port before any consumer
                // reads it. Do the same so saved legacy overlay_port values
                // cannot advertise a port that the shared listener does not
                // own.
                advanced_networking.dht.overlay_port = bind.port();
            }
        }
        let PeerProfileSettings {
            peer_host_override,
            distributed_parent_override,
            test_user_endpoint_overrides,
            user_info_description,
            user_info_picture,
            soulseek_diagnostic_level,
        } = resolve_peer_profile(
            env,
            file_config.profile.user_info_description,
            file_config.profile.user_info_picture,
            file_config.profile.soulseek_diagnostic_level,
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
            controller_profile,
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
        let download_filter = DownloadFilterSettings::from_layers(
            &file_config.filters.download,
            env,
            current_upstream_behavior,
        )?;
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
            ShareSettings::from_layers(file_config.shares, env, controller_profile)?;
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
            file_config.auto_replace,
            env,
            controller_profile,
            current_upstream_behavior,
        )?;
        let transfer_groups = TransferGroupsSettings::from_layers(
            canonical_groups,
            compatibility_groups,
            env,
            controller_profile,
        )?;
        let transfer_auto_retry =
            TransferAutoRetrySettings::from_layers(file_config.transfers.auto_retry, env)?;
        let transfer_rescue =
            TransferRescueSettings::from_layers(file_config.transfers.rescue, env)?;
        let managed_blacklist = ManagedBlacklistSettings::from_layers(
            file_config.blacklist,
            &user_blacklist_file_config,
            env,
            controller_profile,
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
            "Hi, I'm human and testing an slskR client. Shares may be temporarily unavailable while I validate the client.",
        )?;
        let MiscControllerFlags {
            remote_configuration,
            remote_file_management,
            controller_debug,
            controller_no_config_watch,
            controller_no_logo,
            controller_no_start,
            controller_no_version_check,
            controller_experimental,
            controller_hash_from_audio_file_enabled,
            controller_no_share_scan,
            controller_force_share_scan,
        } = resolve_misc_controller_flags(
            env,
            file_config.compatibility.remote_configuration,
            file_config.compatibility.debug,
            file_config.compatibility.no_config_watch,
            file_config.flags.no_logo,
            file_config.flags.no_start,
            file_config.flags.no_version_check,
            file_config.flags.experimental,
            file_config.flags.hash_from_audio_file_enabled,
            file_config.flags.no_share_scan,
            file_config.flags.force_share_scan,
        )?;
        let ApiAndWebHardeningSettings {
            api_token,
            api_read_write_token,
            api_read_only_token,
            api_nowplaying_token,
            auth_required,
            api_cookie_auth_enabled,
            api_rate_limit_anonymous,
            api_rate_limit_authenticated,
            controller_web_max_request_body_size,
            controller_web_enforce_security,
            controller_web_allow_remote_no_auth,
            controller_web_passthrough_allowed_cidrs,
            controller_web_passthrough_cidrs,
            controller_diagnostics_allow_memory_dump,
            controller_diagnostics_allow_remote_dump,
            controller_web_cors,
            controller_web_rate_limiting,
        } = resolve_api_and_web_hardening(
            env,
            controller_profile,
            file_config.auth.api_token,
            file_config.auth.read_write_token,
            file_config.auth.read_only_token,
            file_config.auth.nowplaying_token,
            file_config.auth.disabled,
            file_config.auth.cookie_auth_enabled,
            file_config.auth.rate_limit_anonymous,
            file_config.auth.rate_limit_authenticated,
            file_config.web.max_request_body_size,
            file_config.web.enforce_security,
            file_config.web.allow_remote_no_auth,
            file_config.web.passthrough_allowed_cidrs,
            file_config.diagnostics.allow_memory_dump,
            file_config.diagnostics.allow_remote_dump,
            file_config.web.cors,
            file_config.web.rate_limiting,
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
        let virtual_soulfind_v2_explicit = file_config.virtual_soulfind_v2.enabled.is_some()
            || env.var("SLSKR_VIRTUAL_SOULFIND_V2_ENABLED").is_some();
        let requested_virtual_soulfind_v2_enabled = env_bool_layer(
            env,
            "SLSKR_VIRTUAL_SOULFIND_V2_ENABLED",
            file_config.virtual_soulfind_v2.enabled.unwrap_or(false),
        )?;
        // The frozen native controller registers the v2 routes but keeps their
        // API option disabled by default. Its source-provider controller reads a
        // separate options type whose acquisition-planning option defaults to
        // enabled. Preserve both target defaults while allowing an explicit
        // file or environment value to control both contracts.
        let virtual_soulfind_v2_enabled = requested_virtual_soulfind_v2_enabled;
        let acquisition_planning_enabled = if virtual_soulfind_v2_explicit {
            requested_virtual_soulfind_v2_enabled
        } else {
            true
        };
        let mut integrations = IntegrationSettings::from_layers(
            file_config.integrations,
            env,
            current_upstream_behavior,
        )?;
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
            mesh_gateway,
            media_services,
            social_federation,
            federation_publishing,
            realm,
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
            distributed_parent_override,
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
            acquisition_planning_enabled,
            controller_profile,
            current_upstream_behavior,
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
            download_filter,
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

    /// Current upstream-style native deployments put Soulseek peer traffic and
    /// the TLS mesh overlay on one public TCP listener. Keep an explicitly
    /// configured legacy overlay bind as a supported dedicated-listener escape
    /// hatch, while making the current/default projection share the endpoint.
    pub fn shared_mesh_tcp(&self) -> bool {
        if self.controller_profile != ControllerProfile::Native
            || !self.current_upstream_behavior
            || !self.dht_enabled
            || !self.advanced_networking.mesh.enabled
            || !self.advanced_networking.mesh.enable_dht
            || !self.advanced_networking.mesh.enable_overlay
        {
            return false;
        }
        let Some(listener_bind) = self
            .listener_bind
            .as_deref()
            .and_then(|value| value.parse::<SocketAddr>().ok())
        else {
            return false;
        };
        self.overlay_bind == Some(listener_bind)
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
        if self.controller_profile != ControllerProfile::Native {
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
            "{{\"config_file\":{},\"http_bind\":\"{}\",\"state_dir\":\"{}\",\"server_address\":\"{}\",\"listen_port\":{},\"advertised_port\":{},\"listener_bind\":{},\"obfuscated_listener_bind\":{},\"obfuscated_advertised_port\":{},\"overlay_bind\":{},\"shared_mesh_tcp\":{},\"dht_enabled\":{},\"dht_port\":{},\"trusted_mesh_peers\":{},\"obfuscation\":{},\"peer_host_override\":{},\"test_user_endpoint_overrides\":{},\"username\":{},\"credentials_configured\":{},\"credential_store\":\"{}\",\"credential_file\":\"{}\",\"auto_connect\":{},\"reconnect\":{},\"reconnect_seconds\":{},\"ping_seconds\":{},\"log_level\":\"{}\",\"peer_response_timeout_seconds\":{},\"share_roots\":{},\"share_follow_symlinks\":{},\"share_include_hidden\":{},\"share_scan_max_files\":{},\"share_cache_tsv_enabled\":{},\"transfer_history_limit\":{},\"transfer_max_active\":{},\"transfer_allow_inbound\":{},\"transfer_allow_outbound\":{},\"transfer_auto_retry\":{},\"transfer_rescue\":{},\"download_completed_path_template_configured\":{},\"private_message_auto_response\":{},\"pod_join_signature_mode\":\"{}\",\"virtual_soulfind_v2_enabled\":{},\"controller_profile\":\"{}\",\"parity_profile\":\"{}\",\"remote_configuration\":{},\"auth_required\":{},\"api_token_configured\":{},\"api_read_write_token_configured\":{},\"api_read_only_token_configured\":{},\"api_nowplaying_token_configured\":{},\"api_cookie_auth_enabled\":{},\"trusted_proxy_cidrs\":{},\"persistence_enabled\":{},\"integrations\":{}}}",
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
            self.shared_mesh_tcp(),
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
            self.controller_profile.as_str(),
            if self.current_upstream_behavior { "current" } else { "frozen" },
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
pub enum ControllerProfile {
    Legacy,
    Native,
}

impl ControllerProfile {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "legacy" => Ok(Self::Legacy),
            "native" => Ok(Self::Native),
            // The historical differential module still labels its two
            // external reference fixtures by their upstream names. Those
            // aliases are compiled only into proof/test builds and are not
            // accepted by the slskr production binary.
            #[cfg(any(test, feature = "bounded-differential"))]
            "slskd" => Ok(Self::Legacy),
            #[cfg(any(test, feature = "bounded-differential"))]
            "slskdn" => Ok(Self::Native),
            _ => Err("SLSKR_CONTROLLER_PROFILE must be legacy or native".to_owned()),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Native => "native",
        }
    }
}

fn validate_controller_storage_directory(
    field: &str,
    path: &std::path::Path,
    target: ControllerProfile,
    explicitly_configured: bool,
) -> Result<(), String> {
    if !explicitly_configured {
        return Ok(());
    }
    if path.as_os_str().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if target == ControllerProfile::Legacy && !path.is_absolute() {
        return Err(format!(
            "{field} must be an absolute path for the legacy compatibility profile"
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
    pub signal_system: SignalSystemSettings,
    pub mesh_sync_security: MeshSyncSecuritySettings,
    pub pod_join_signature_mode: PodSignatureMode,
    pub pod_security_signature_mode: PodSignatureMode,
    pub gold_star_club_autojoin: bool,
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
    pub enable_overlay: bool,
    pub enable_dht: bool,
    pub enable_stun: bool,
    pub enable_soulseek_capability_handshake: bool,
    pub enable_soulseek_rendezvous: bool,
    pub probe_soulseek_rendezvous_capabilities: bool,
    pub dht_bootstrap_nodes: usize,
    pub udp_port: u16,
    pub quic_port: u16,
    pub enforce_remote_payload_limits: bool,
    pub max_remote_payload_size: usize,
}

impl MeshRuntimeSettings {
    /// Match native profile's MeshSecurityOptions effective cap.  Disabling the
    /// strict remote limit relaxes it by at most 10x, but never beyond 10 MiB;
    /// it does not remove the bound entirely.
    pub fn effective_max_remote_payload_size(&self) -> usize {
        if self.enforce_remote_payload_limits {
            self.max_remote_payload_size
        } else {
            self.max_remote_payload_size
                .saturating_mul(10)
                .min(10 * 1024 * 1024)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshGatewaySettings {
    pub enabled: bool,
    pub bind_address: String,
    pub port: u16,
    pub api_key: Option<String>,
    pub csrf_token: Option<String>,
    pub allowed_services: Vec<String>,
    pub max_request_body_bytes: usize,
    pub request_timeout_seconds: u64,
    pub log_bodies: bool,
    pub require_risk_acknowledgment: bool,
    pub i_understand_the_risk: bool,
    pub allowed_origins: Vec<String>,
    pub enable_rate_limiting: bool,
    pub max_requests_per_minute: u32,
}

impl MeshGatewaySettings {
    fn from_layers<E: ConfigEnv>(file: &MeshGatewayFileConfig, env: &E) -> Result<Self, String> {
        let enabled = env_bool_any_layer(
            env,
            &["SLSKR_MESH_GATEWAY_ENABLED", "SLSKD_MESH_GATEWAY_ENABLED"],
            file.enabled.unwrap_or(false),
        )?;
        let bind_address = optional_env_any(
            env,
            &[
                "SLSKR_MESH_GATEWAY_BIND_ADDRESS",
                "SLSKD_MESH_GATEWAY_BIND_ADDRESS",
            ],
        )
        .or_else(|| file.bind_address.clone())
        .unwrap_or_else(|| "127.0.0.1".to_owned());
        if bind_address.trim().is_empty() {
            return Err("MeshGateway.BindAddress cannot be empty".to_owned());
        }
        let port = env_parse_any_layer(
            env,
            &["SLSKR_MESH_GATEWAY_PORT", "SLSKD_MESH_GATEWAY_PORT"],
            file.port,
            0_u16,
        )?;
        let allowed_services = normalized_controller_values(string_array_any_layer(
            env,
            &[
                "SLSKR_MESH_GATEWAY_ALLOWED_SERVICES",
                "SLSKD_MESH_GATEWAY_ALLOWED_SERVICES",
            ],
            file.allowed_services.clone().unwrap_or_default(),
        ));
        let max_request_body_bytes = env_parse_any_layer(
            env,
            &[
                "SLSKR_MESH_GATEWAY_MAX_REQUEST_BODY_BYTES",
                "SLSKD_MESH_GATEWAY_MAX_REQUEST_BODY_BYTES",
            ],
            file.max_request_body_bytes,
            1_048_576_usize,
        )?;
        if !(1..=16 * 1024 * 1024).contains(&max_request_body_bytes) {
            return Err(
                "MeshGateway.MaxRequestBodyBytes must be between 1 and 16777216".to_owned(),
            );
        }
        let request_timeout_seconds = env_parse_any_layer(
            env,
            &[
                "SLSKR_MESH_GATEWAY_REQUEST_TIMEOUT_SECONDS",
                "SLSKD_MESH_GATEWAY_REQUEST_TIMEOUT_SECONDS",
            ],
            file.request_timeout_seconds,
            30_u64,
        )?;
        if !(1..=3_600).contains(&request_timeout_seconds) {
            return Err("MeshGateway.RequestTimeoutSeconds must be between 1 and 3600".to_owned());
        }
        let api_key = optional_env_any(
            env,
            &["SLSKR_MESH_GATEWAY_API_KEY", "SLSKD_MESH_GATEWAY_API_KEY"],
        )
        .or_else(|| file.api_key.clone());
        let csrf_token = optional_env_any(
            env,
            &[
                "SLSKR_MESH_GATEWAY_CSRF_TOKEN",
                "SLSKD_MESH_GATEWAY_CSRF_TOKEN",
            ],
        )
        .or_else(|| file.csrf_token.clone());
        if enabled && allowed_services.is_empty() {
            return Err(
                "MeshGateway.AllowedServices cannot be empty when the gateway is enabled"
                    .to_owned(),
            );
        }
        let is_localhost = matches!(bind_address.trim(), "127.0.0.1" | "localhost" | "::1");
        if enabled
            && !is_localhost
            && api_key
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(
                "MeshGateway.ApiKey is required when binding to a non-localhost address".to_owned(),
            );
        }
        let require_risk_acknowledgment = env_bool_any_layer(
            env,
            &[
                "SLSKR_MESH_GATEWAY_REQUIRE_RISK_ACKNOWLEDGMENT",
                "SLSKD_MESH_GATEWAY_REQUIRE_RISK_ACKNOWLEDGMENT",
            ],
            file.require_risk_acknowledgment.unwrap_or(true),
        )?;
        let i_understand_the_risk = env_bool_any_layer(
            env,
            &[
                "SLSKR_MESH_GATEWAY_I_UNDERSTAND_THE_RISK",
                "SLSKD_MESH_GATEWAY_I_UNDERSTAND_THE_RISK",
            ],
            file.i_understand_the_risk.unwrap_or(false),
        )?;
        if enabled && !is_localhost && require_risk_acknowledgment && !i_understand_the_risk {
            return Err(
                "MeshGateway.IUnderstandTheRisk must be true for non-localhost binding".to_owned(),
            );
        }
        let max_requests_per_minute = env_parse_any_layer(
            env,
            &[
                "SLSKR_MESH_GATEWAY_MAX_REQUESTS_PER_MINUTE",
                "SLSKD_MESH_GATEWAY_MAX_REQUESTS_PER_MINUTE",
            ],
            file.max_requests_per_minute,
            60_u32,
        )?;
        if max_requests_per_minute == 0 {
            return Err("MeshGateway.MaxRequestsPerMinute must be greater than zero".to_owned());
        }
        Ok(Self {
            enabled,
            bind_address,
            port,
            api_key,
            csrf_token,
            allowed_services,
            max_request_body_bytes,
            request_timeout_seconds,
            log_bodies: env_bool_any_layer(
                env,
                &[
                    "SLSKR_MESH_GATEWAY_LOG_BODIES",
                    "SLSKD_MESH_GATEWAY_LOG_BODIES",
                ],
                file.log_bodies.unwrap_or(false),
            )?,
            require_risk_acknowledgment,
            i_understand_the_risk,
            allowed_origins: normalized_controller_values(string_array_any_layer(
                env,
                &[
                    "SLSKR_MESH_GATEWAY_ALLOWED_ORIGINS",
                    "SLSKD_MESH_GATEWAY_ALLOWED_ORIGINS",
                ],
                file.allowed_origins.clone().unwrap_or_default(),
            )),
            enable_rate_limiting: env_bool_any_layer(
                env,
                &[
                    "SLSKR_MESH_GATEWAY_ENABLE_RATE_LIMITING",
                    "SLSKD_MESH_GATEWAY_ENABLE_RATE_LIMITING",
                ],
                file.enable_rate_limiting.unwrap_or(true),
            )?,
            max_requests_per_minute,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SignalSystemSettings {
    pub enabled: bool,
    pub deduplication_cache_size: usize,
    pub default_ttl: Duration,
    pub mesh_channel: SignalChannelSettings,
    pub bt_extension_channel: SignalChannelSettings,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SignalChannelSettings {
    pub enabled: bool,
    pub priority: u8,
    pub require_active_session: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MeshSyncSecuritySettings {
    pub max_invalid_entries_per_window: u32,
    pub max_invalid_messages_per_window: u32,
    pub rate_limit_window: Duration,
    pub quarantine_violation_threshold: u32,
    pub quarantine_duration: Duration,
    pub proof_of_possession_enabled: bool,
    pub require_signed_entries: bool,
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
    pub share_with_dht_port: bool,
    pub backend_listen_port: u16,
    pub max_concurrent_streams: usize,
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

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
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
pub struct SocialFederationSettings {
    pub enabled: bool,
    pub mode: String,
    pub domain: Option<String>,
    pub base_url: Option<String>,
    pub approved_peers: Vec<String>,
    pub outbox_max_activities: u32,
    pub page_size: u32,
    pub verify_signatures: bool,
    pub http_timeout_seconds: u32,
}

impl SocialFederationSettings {
    fn from_layers<E: ConfigEnv>(
        file: SocialFederationFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let page_size = env_parse_any_layer(
            env,
            &["FEDERATION_PAGE_SIZE", "SLSKR_FEDERATION_PAGE_SIZE"],
            file.page_size,
            20_u32,
        )?;
        if !(10..=100).contains(&page_size) {
            return Err("federation.page_size must be between 10 and 100".to_owned());
        }
        let outbox_max_activities = env_parse_any_layer(
            env,
            &[
                "FEDERATION_OUTBOX_MAX_ACTIVITIES",
                "SLSKR_FEDERATION_OUTBOX_MAX_ACTIVITIES",
            ],
            file.outbox_max_activities,
            100_u32,
        )?;
        if !(10..=1_000).contains(&outbox_max_activities) {
            return Err("federation.outbox_max_activities must be between 10 and 1000".to_owned());
        }
        let http_timeout_seconds = env_parse_any_layer(
            env,
            &[
                "FEDERATION_HTTP_TIMEOUT_SECONDS",
                "SLSKR_FEDERATION_HTTP_TIMEOUT_SECONDS",
            ],
            file.http_timeout_seconds,
            30_u32,
        )?;
        if !(5..=120).contains(&http_timeout_seconds) {
            return Err("federation.http_timeout_seconds must be between 5 and 120".to_owned());
        }
        let mode = env
            .var("FEDERATION_MODE")
            .or_else(|| env.var("SLSKR_FEDERATION_MODE"))
            .or(file.mode)
            .unwrap_or_else(|| "Hermit".to_owned());
        let mode = mode.trim().to_owned();
        let domain = env
            .var("FEDERATION_DOMAIN")
            .or_else(|| env.var("SLSKR_FEDERATION_DOMAIN"))
            .or(file.domain)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let base_url = env
            .var("FEDERATION_BASE_URL")
            .or_else(|| env.var("SLSKR_FEDERATION_BASE_URL"))
            .or(file.base_url)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        Ok(Self {
            enabled: env_bool_any_layer(
                env,
                &["FEDERATION_ENABLED", "SLSKR_FEDERATION_ENABLED"],
                file.enabled.unwrap_or(false),
            )?,
            mode,
            domain,
            base_url,
            approved_peers: string_array_any_layer(
                env,
                &[
                    "FEDERATION_APPROVED_PEERS",
                    "SLSKR_FEDERATION_APPROVED_PEERS",
                ],
                file.approved_peers,
            ),
            outbox_max_activities,
            page_size,
            verify_signatures: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_VERIFY_SIGNATURES",
                    "SLSKR_FEDERATION_VERIFY_SIGNATURES",
                ],
                file.verify_signatures.unwrap_or(true),
            )?,
            http_timeout_seconds,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FederationPublishingSettings {
    pub enabled: bool,
    pub publishable_domains: Vec<String>,
    pub default_visibility: String,
    pub approved_circles: Vec<String>,
    pub require_moderation_approval: bool,
    pub include_external_links: bool,
    pub max_metadata_size_kb: u32,
}

impl FederationPublishingSettings {
    fn from_layers<E: ConfigEnv>(
        file: FederationPublishingFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let max_metadata_size_kb = env_parse_any_layer(
            env,
            &[
                "FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB",
                "SLSKR_FEDERATION_PUBLISHING_MAX_METADATA_SIZE_KB",
            ],
            file.max_metadata_size_kb,
            10_u32,
        )?;
        if !(1..=100).contains(&max_metadata_size_kb) {
            return Err(
                "federation_publishing.max_metadata_size_kb must be between 1 and 100".to_owned(),
            );
        }
        let default_visibility = env
            .var("FEDERATION_PUBLISHING_DEFAULT_VISIBILITY")
            .or_else(|| env.var("SLSKR_FEDERATION_PUBLISHING_DEFAULT_VISIBILITY"))
            .or(file.default_visibility)
            .unwrap_or_else(|| "public".to_owned())
            .trim()
            .to_owned();
        Ok(Self {
            enabled: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_ENABLED",
                    "SLSKR_FEDERATION_PUBLISHING_ENABLED",
                ],
                file.enabled.unwrap_or(false),
            )?,
            publishable_domains: string_array_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS",
                    "SLSKR_FEDERATION_PUBLISHING_PUBLISHABLE_DOMAINS",
                ],
                if file.publishable_domains.is_empty() {
                    vec!["music".to_owned()]
                } else {
                    file.publishable_domains
                },
            ),
            default_visibility,
            approved_circles: string_array_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_APPROVED_CIRCLES",
                    "SLSKR_FEDERATION_PUBLISHING_APPROVED_CIRCLES",
                ],
                file.approved_circles,
            ),
            require_moderation_approval: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_REQUIRE_MODERATION",
                    "SLSKR_FEDERATION_PUBLISHING_REQUIRE_MODERATION",
                ],
                file.require_moderation_approval.unwrap_or(true),
            )?,
            include_external_links: env_bool_any_layer(
                env,
                &[
                    "FEDERATION_PUBLISHING_INCLUDE_EXTERNAL_LINKS",
                    "SLSKR_FEDERATION_PUBLISHING_INCLUDE_EXTERNAL_LINKS",
                ],
                file.include_external_links.unwrap_or(true),
            )?,
            max_metadata_size_kb,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SolidSettings {
    pub allow_insecure_http: bool,
    pub allow_localhost_for_web_id: bool,
    pub max_fetch_bytes: usize,
    pub timeout: Duration,
    pub allowed_hosts: Vec<String>,
    pub client_id_url: Option<String>,
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
                // The frozen native profile snapshot applies an integer-operand
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
        target: ControllerProfile,
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
                ControllerProfile::Legacy => {
                    format!("Pattern '{pattern}' is not a valid regular expression")
                }
                ControllerProfile::Native => {
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
                    ControllerProfile::Legacy => {
                        format!("CIDR {cidr} is invalid: {error}")
                    }
                    ControllerProfile::Native => format!("CIDR {cidr} is invalid"),
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
    target: ControllerProfile,
) -> Result<(), String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("failed to read blacklist file {}: {error}", path.display()))?;
    detect_managed_blacklist_format(&body, target).map(|_| ())
}

fn load_managed_blacklist_file(
    path: &std::path::Path,
    target: ControllerProfile,
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
    target: ControllerProfile,
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
    target: ControllerProfile,
) -> Result<String, String> {
    match format {
        ManagedBlacklistFormat::Cidr => Ok(line.to_owned()),
        ManagedBlacklistFormat::P2p => {
            let range = match target {
                ControllerProfile::Legacy => line.split(':').nth(1),
                ControllerProfile::Native => line.rsplit_once(':').map(|(_, range)| range),
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

#[path = "config_integrations.rs"]
mod integrations;
pub use integrations::*;

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
    pub fn from_path(path: &str, alias: Option<&str>) -> Result<Self, String> {
        let path = path.trim();
        if path.contains('\0') {
            return Err("share path contains an invalid NUL character".to_owned());
        }
        let raw = match alias.map(str::trim).filter(|alias| !alias.is_empty()) {
            Some(alias) => {
                if alias.contains('\0') {
                    return Err("share alias contains an invalid NUL character".to_owned());
                }
                format!("[{alias}]{path}")
            }
            None => path.to_owned(),
        };
        Self::parse(&raw)
    }

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
        target: ControllerProfile,
    ) -> Result<Self, String> {
        let fixture = env
            .var("SLSKR_SHARE_FIXTURE")
            .or(file_config.fixture)
            .unwrap_or_default();
        let raw_directories = match optional_env_any(
            env,
            &["SLSKR_SHARE_DIRS", "SLSKR_SHARED_DIR", "SLSKD_SHARED_DIR"],
        ) {
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
        let default_workers = if target == ControllerProfile::Native {
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
            probe_media_attributes: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SHARES_PROBE_MEDIA_ATTRIBUTES",
                    "SHARES_PROBE_MEDIA_ATTRIBUTES",
                    "SLSKD_SHARES_PROBE_MEDIA_ATTRIBUTES",
                ],
                file_config.probe_media_attributes.unwrap_or(true),
            )?,
            filters,
        })
    }
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
        || config.integrations.acoustid.client_id.is_some()
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

#[path = "config_yaml.rs"]
mod yaml;
use yaml::*;

pub fn optional_env_any(env: &dyn ConfigEnv, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env.var(name))
}

fn profile_env_names(
    current_upstream_behavior: bool,
    native_name: &'static str,
    canonical_name: &'static str,
    compatibility_name: &'static str,
) -> Vec<&'static str> {
    if current_upstream_behavior {
        vec![native_name, canonical_name, compatibility_name]
    } else {
        vec![native_name, compatibility_name]
    }
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

fn string_array_any_layer<E: ConfigEnv>(
    env: &E,
    names: &[&str],
    file_value: Vec<String>,
) -> Vec<String> {
    names
        .iter()
        .find_map(|name| env.var(name))
        .map_or(file_value, |value| {
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

struct SoulseekIdentity {
    server_address: String,
    listen_port: u32,
    username: Option<String>,
    password: Option<String>,
    credential_store: CredentialStoreMode,
    credential_file: PathBuf,
    auto_connect: bool,
}

struct ControllerWebAuthSettings {
    controller_metrics_enabled: bool,
    controller_metrics_url: String,
    controller_metrics_auth_disabled: bool,
    controller_metrics_username: String,
    controller_metrics_password: String,
    controller_web_auth_username: String,
    controller_web_auth_password: String,
    controller_web_jwt_key: String,
    controller_web_jwt_key_configured: bool,
    controller_web_jwt_ttl_millis: u64,
}

#[allow(clippy::too_many_arguments)]
fn resolve_controller_web_auth<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    auth_required: bool,
    metrics_enabled: Option<bool>,
    metrics_url: Option<String>,
    metrics_auth_disabled: Option<bool>,
    metrics_username: Option<String>,
    metrics_password: Option<String>,
    web_auth_username: Option<String>,
    web_auth_password: Option<String>,
    web_jwt_key: Option<String>,
    web_jwt_ttl: Option<u64>,
) -> Result<ControllerWebAuthSettings, String> {
    let controller_metrics_enabled =
        env_bool_layer(env, "SLSKD_METRICS", metrics_enabled.unwrap_or(false))?;
    let controller_metrics_url = env
        .var("SLSKD_METRICS_URL")
        .or(metrics_url)
        .unwrap_or_else(|| "/metrics".to_owned());
    let controller_metrics_auth_disabled = env_bool_layer(
        env,
        "SLSKD_METRICS_NO_AUTH",
        metrics_auth_disabled.unwrap_or(false),
    )?;
    let default_identity = match controller_profile {
        ControllerProfile::Legacy => "slskd",
        ControllerProfile::Native => "slskr",
    };
    let controller_metrics_username = env
        .var("SLSKD_METRICS_USERNAME")
        .or(metrics_username)
        .unwrap_or_else(|| default_identity.to_owned());
    let controller_metrics_password = env
        .var("SLSKD_METRICS_PASSWORD")
        .or(metrics_password)
        .unwrap_or_default();
    let controller_web_auth_username = env
        .var("SLSKD_USERNAME")
        .or(web_auth_username)
        .unwrap_or_else(|| default_identity.to_owned());
    let configured_web_auth_password = env.var("SLSKD_PASSWORD").or(web_auth_password);
    let controller_web_auth_password = configured_web_auth_password.clone().unwrap_or_else(|| {
        if auth_required {
            // Preserve the profile's generated-config default when no
            // password layer overrides it. No-auth mode keeps the credential
            // empty because it is not used.
            default_identity.to_owned()
        } else {
            String::new()
        }
    });
    let mut web_auth_fields = vec![("username", controller_web_auth_username.as_str())];
    if auth_required && configured_web_auth_password.is_some() {
        web_auth_fields.push(("password", controller_web_auth_password.as_str()));
    }
    for (field, value) in web_auth_fields {
        let length = value.encode_utf16().count();
        if !(1..=255).contains(&length) {
            return Err(format!(
                "web authentication {field} must contain between 1 and 255 characters"
            ));
        }
    }
    let controller_web_jwt_key = env.var("SLSKD_JWT_KEY").or(web_jwt_key);
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
        web_jwt_ttl,
        if controller_profile == ControllerProfile::Legacy {
            604_800_000_u64
        } else {
            3_600_000_u64
        },
    )?;
    if controller_web_jwt_ttl_millis < 3_600 {
        return Err("web authentication JWT TTL must be at least 3600 milliseconds".to_owned());
    }
    let metrics_auth_requires_credentials =
        controller_metrics_enabled && !controller_metrics_auth_disabled;
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
    Ok(ControllerWebAuthSettings {
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
    })
}

struct MiscControllerFlags {
    remote_configuration: bool,
    remote_file_management: bool,
    controller_debug: bool,
    controller_no_config_watch: bool,
    controller_no_logo: bool,
    controller_no_start: bool,
    controller_no_version_check: bool,
    controller_experimental: bool,
    controller_hash_from_audio_file_enabled: bool,
    controller_no_share_scan: bool,
    controller_force_share_scan: bool,
}

#[allow(clippy::too_many_arguments)]
fn resolve_misc_controller_flags<E: ConfigEnv>(
    env: &E,
    compatibility_remote_configuration: Option<bool>,
    compatibility_debug: Option<bool>,
    compatibility_no_config_watch: Option<bool>,
    flags_no_logo: Option<bool>,
    flags_no_start: Option<bool>,
    flags_no_version_check: Option<bool>,
    flags_experimental: Option<bool>,
    flags_hash_from_audio_file_enabled: Option<bool>,
    flags_no_share_scan: Option<bool>,
    flags_force_share_scan: Option<bool>,
) -> Result<MiscControllerFlags, String> {
    Ok(MiscControllerFlags {
        remote_configuration: env_bool_any_layer(
            env,
            &["SLSKR_REMOTE_CONFIGURATION", "SLSKD_REMOTE_CONFIGURATION"],
            compatibility_remote_configuration.unwrap_or(false),
        )?,
        remote_file_management: env_bool_any_layer(
            env,
            &[
                "SLSKR_REMOTE_FILE_MANAGEMENT",
                "SLSKD_REMOTE_FILE_MANAGEMENT",
            ],
            false,
        )?,
        controller_debug: env_bool_any_layer(
            env,
            &["SLSKR_DEBUG", "SLSKD_DEBUG"],
            compatibility_debug.unwrap_or(false),
        )?,
        controller_no_config_watch: env_bool_any_layer(
            env,
            &["SLSKR_NO_CONFIG_WATCH", "SLSKD_NO_CONFIG_WATCH"],
            compatibility_no_config_watch.unwrap_or(false),
        )?,
        controller_no_logo: env_bool_layer(env, "SLSKD_NO_LOGO", flags_no_logo.unwrap_or(false))?,
        controller_no_start: env_bool_layer(
            env,
            "SLSKD_NO_START",
            flags_no_start.unwrap_or(false),
        )?,
        controller_no_version_check: env_bool_layer(
            env,
            "SLSKD_NO_VERSION_CHECK",
            flags_no_version_check.unwrap_or(false),
        )?,
        controller_experimental: env_bool_layer(
            env,
            "SLSKD_EXPERIMENTAL",
            flags_experimental.unwrap_or(false),
        )?,
        controller_hash_from_audio_file_enabled: env_bool_layer(
            env,
            "SLSKR_CONTROLLER_YAML_HASH_FROM_AUDIO_FILE_ENABLED",
            flags_hash_from_audio_file_enabled.unwrap_or(false),
        )?,
        controller_no_share_scan: env_bool_layer(
            env,
            "SLSKD_NO_SHARE_SCAN",
            flags_no_share_scan.unwrap_or(false),
        )?,
        controller_force_share_scan: env_bool_layer(
            env,
            "SLSKD_FORCE_SHARE_SCAN",
            flags_force_share_scan.unwrap_or(false),
        )?,
    })
}

fn resolve_controller_api_keys<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    auth_api_keys: BTreeMap<String, ControllerApiKeyFileConfig>,
) -> Result<BTreeMap<String, ControllerApiKeySettings>, String> {
    let controller_api_key_files = match env.var("SLSKD_API_KEYS_JSON") {
        Some(value) => serde_json::from_str::<BTreeMap<String, ControllerApiKeyFileConfig>>(&value)
            .map_err(|error| format!("invalid web.authentication.api_keys: {error}"))?,
        None => auth_api_keys,
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
        let default_cidr = if controller_profile == ControllerProfile::Native {
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
    Ok(controller_api_keys)
}

struct PeerProfileSettings {
    peer_host_override: Option<Ipv4Addr>,
    distributed_parent_override: Option<SocketAddr>,
    test_user_endpoint_overrides: BTreeMap<String, SocketAddr>,
    user_info_description: String,
    user_info_picture: Option<PathBuf>,
    soulseek_diagnostic_level: SoulseekDiagnosticLevel,
}

fn resolve_peer_profile<E: ConfigEnv>(
    env: &E,
    profile_user_info_description: Option<String>,
    profile_user_info_picture: Option<String>,
    profile_soulseek_diagnostic_level: Option<String>,
) -> Result<PeerProfileSettings, String> {
    let peer_host_override = env
        .var("SLSKR_PEER_HOST_OVERRIDE")
        .map(|value| {
            value
                .parse::<Ipv4Addr>()
                .map_err(|error| format!("invalid SLSKR_PEER_HOST_OVERRIDE: {error}"))
        })
        .transpose()?;
    let distributed_parent_override = env
        .var("SLSKR_DISTRIBUTED_PARENT_OVERRIDE")
        .map(|value| {
            let address = value
                .parse::<SocketAddr>()
                .map_err(|error| format!("invalid SLSKR_DISTRIBUTED_PARENT_OVERRIDE: {error}"))?;
            if address.port() == 0 {
                return Err("SLSKR_DISTRIBUTED_PARENT_OVERRIDE port must be non-zero".to_owned());
            }
            Ok(address)
        })
        .transpose()?;
    let test_user_endpoint_overrides =
        parse_user_endpoint_overrides(env.var("SLSKR_TEST_USER_ENDPOINT_OVERRIDES"))?;
    let user_info_description = optional_env_any(
        env,
        &["SLSKR_USER_INFO_DESCRIPTION", "SLSKD_SLSK_DESCRIPTION"],
    )
    .or(profile_user_info_description)
    .unwrap_or_else(|| "A slskR user. https://github.com/snapetech/slskr".to_owned());
    let user_info_picture = optional_env_any(
        env,
        &[
            "SLSKR_USER_INFO_PICTURE",
            "SLSKD_SLSK_PICTURE",
            "SLSK_PICTURE",
        ],
    )
    .or(profile_user_info_picture)
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
        .or(profile_soulseek_diagnostic_level)
        .as_deref()
        .unwrap_or("info"),
    )?;
    Ok(PeerProfileSettings {
        peer_host_override,
        distributed_parent_override,
        test_user_endpoint_overrides,
        user_info_description,
        user_info_picture,
        soulseek_diagnostic_level,
    })
}

struct ListenerAndObfuscationResolution {
    listener_bind: Option<String>,
    advertised_port: u32,
    obfuscated_listener_bind: Option<String>,
    obfuscated_advertised_port: Option<u32>,
    overlay_bind: Option<SocketAddr>,
    dht_enabled: bool,
    dht_port: u16,
    trusted_mesh_peers: Vec<TrustedMeshPeer>,
    obfuscation_enabled: bool,
    obfuscation_mode: SoulseekObfuscationMode,
    obfuscation_listen_port: u32,
    obfuscation_advertise_regular_port: bool,
    obfuscation_prefer_outbound: bool,
}

#[allow(clippy::too_many_arguments)]
fn resolve_listener_and_obfuscation<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    current_upstream_behavior: bool,
    advanced_networking: &AdvancedNetworkingSettings,
    listen_port: u32,
    auto_connect: bool,
    listeners_regular_bind: Option<String>,
    listeners_advertised_port: Option<u32>,
    listeners_obfuscated_bind: Option<String>,
    listeners_obfuscated_advertised_port: Option<u32>,
    listeners_overlay_bind: Option<String>,
    mesh_trusted_peers: Vec<TrustedMeshPeerInput>,
    obfuscation_enabled_file: Option<bool>,
    obfuscation_mode_file: Option<String>,
    obfuscation_advertise_regular_port_file: Option<bool>,
    obfuscation_prefer_outbound_file: Option<bool>,
) -> Result<ListenerAndObfuscationResolution, String> {
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
    .or(listeners_regular_bind)
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
                "Soulseek.ListenIpAddress specifies an invalid IPv4 or IPv6 IP address".to_owned()
            })?;
        }
    }
    if controller_profile == ControllerProfile::Native
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
        listeners_advertised_port,
        listen_port,
    )?;
    // The legacy profile has no Soulseek type-1 obfuscation option or listener.
    // The fields are accepted by the shared configuration model because they
    // are part of the native profile, but they must not silently turn the
    // legacy profile into a different network endpoint. Ignore those native
    // profile-only layers for the legacy profile and keep the runtime projection
    // disabled below.
    let supports_soulseek_obfuscation = controller_profile == ControllerProfile::Native;
    let upstream_obfuscated_port = if supports_soulseek_obfuscation {
        env_parse_any_option(env, &["SLSKD_SLSK_OBFUSCATION_LISTEN_PORT"])?
    } else {
        None
    };
    let obfuscated_listener_bind = if supports_soulseek_obfuscation {
        // Current upstream uses a zero obfuscation port as the shared-listener
        // sentinel. Frozen slskdN compatibility retains the historical
        // adjacent dedicated listener when no explicit obfuscation bind was
        // configured.
        env.var("SLSKR_OBFUSCATED_LISTENER_BIND")
            .or(listeners_obfuscated_bind)
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
            .or_else(|| {
                (!current_upstream_behavior && listen_port < 65_535).then(|| {
                    let host = listener_bind
                        .as_deref()
                        .and_then(|value| value.parse::<SocketAddr>().ok())
                        .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |bind| bind.ip());
                    SocketAddr::new(host, (listen_port + 1) as u16).to_string()
                })
            })
    } else {
        None
    };
    let obfuscated_advertised_port = if supports_soulseek_obfuscation {
        if env.var("SLSKR_OBFUSCATED_ADVERTISED_PORT").is_some() {
            env_parse_option_layer(
                env,
                "SLSKR_OBFUSCATED_ADVERTISED_PORT",
                listeners_obfuscated_advertised_port,
            )?
        } else {
            upstream_obfuscated_port
                .filter(|port| *port != 0)
                .map(u32::from)
                .or(listeners_obfuscated_advertised_port)
                .or_else(|| {
                    obfuscated_listener_bind
                        .as_deref()
                        .and_then(|value| value.parse::<SocketAddr>().ok())
                        .map(|address| u32::from(address.port()))
                })
                .or_else(|| {
                    if current_upstream_behavior {
                        Some(listen_port)
                    } else {
                        (listen_port < 65_535).then_some(listen_port + 1)
                    }
                })
        }
    } else {
        None
    };
    let explicit_overlay_bind = env.var("SLSKR_OVERLAY_BIND").or(listeners_overlay_bind);
    let overlay_bind = explicit_overlay_bind
        .clone()
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
            if controller_profile == ControllerProfile::Native
                && current_upstream_behavior
                && advanced_networking.dht.enabled
                && advanced_networking.mesh.enabled
                && advanced_networking.mesh.enable_dht
                && advanced_networking.mesh.enable_overlay
            {
                // Current upstream owns the mesh TCP handshake on the
                // Soulseek listen socket. Keep the legacy explicit
                // `listeners.overlay_bind` escape hatch above, but make the
                // stock/native projection use one public TCP endpoint.
                listener_bind
                    .as_deref()
                    .and_then(|value| value.parse::<SocketAddr>().ok())
            } else {
                (controller_profile == ControllerProfile::Native
                    && advanced_networking.overlay.enable)
                    .then_some(SocketAddr::new(
                        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                        advanced_networking.dht.overlay_port,
                    ))
            }
        });
    let dht_enabled = advanced_networking.dht.enabled;
    let dht_port = advanced_networking.dht.dht_port;
    let trusted_mesh_peers =
        trusted_mesh_peers_from_layers(env.var("SLSKR_TRUSTED_MESH_PEERS"), mesh_trusted_peers)?;
    let (
        obfuscation_enabled,
        obfuscation_mode,
        obfuscation_listen_port,
        obfuscation_advertise_regular_port,
        obfuscation_prefer_outbound,
    ) = if supports_soulseek_obfuscation {
        let enabled = env_bool_any_layer(
            env,
            &["SLSK_OBFUSCATION", "SLSKD_SLSK_OBFUSCATION"],
            obfuscation_enabled_file.unwrap_or(true),
        )?;
        let mode = SoulseekObfuscationMode::parse(
            optional_env_any(
                env,
                &["SLSK_OBFUSCATION_MODE", "SLSKD_SLSK_OBFUSCATION_MODE"],
            )
            .or(obfuscation_mode_file)
            .as_deref()
            .unwrap_or("compatibility"),
        )?;
        let advertise_regular_port = env_bool_any_layer(
            env,
            &[
                "SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
                "SLSKD_SLSK_OBFUSCATION_ADVERTISE_REGULAR_PORT",
            ],
            obfuscation_advertise_regular_port_file.unwrap_or(true),
        )?;
        let prefer_outbound = env_bool_any_layer(
            env,
            &[
                "SLSK_OBFUSCATION_PREFER_OUTBOUND",
                "SLSKD_SLSK_OBFUSCATION_PREFER_OUTBOUND",
            ],
            obfuscation_prefer_outbound_file.unwrap_or(true),
        )?;
        if enabled && !advertise_regular_port && current_upstream_behavior {
            return Err(
                "The regular peer port must be advertised when peer obfuscation is enabled"
                    .to_owned(),
            );
        }
        (
            enabled,
            mode,
            u32::from(upstream_obfuscated_port.unwrap_or_default()),
            advertise_regular_port,
            prefer_outbound,
        )
    } else {
        (
            false,
            SoulseekObfuscationMode::Compatibility,
            0,
            true,
            false,
        )
    };
    Ok(ListenerAndObfuscationResolution {
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
    })
}

struct ControllerWebResolution {
    http_bind: SocketAddr,
    http_binds: Vec<SocketAddr>,
    controller_http_address: Option<String>,
    controller_web: ControllerWebSettings,
}

#[allow(clippy::too_many_arguments)]
fn resolve_controller_web<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    app_http_bind: Option<String>,
    web_socket: Option<PathBuf>,
    web_url_base: Option<String>,
    web_content_path: Option<PathBuf>,
    web_logging: Option<bool>,
    web_https: HttpsFileConfig,
) -> Result<ControllerWebResolution, String> {
    let configured_native_http_bind = app_http_bind.as_deref();
    let base_http_bind = configured_native_http_bind
        .unwrap_or("127.0.0.1:5030")
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid configured HTTP bind: {error}"))?;
    let http_port = env_parse_any_layer(env, &["SLSKD_HTTP_PORT"], None, base_http_bind.port())?;
    let (http_binds, controller_http_address) = if let Some(value) = env.var("SLSKR_HTTP_BIND") {
        let address = value
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid SLSKR_HTTP_BIND: {error}"))?;
        (vec![address], Some(address.ip().to_string()))
    } else {
        match controller_profile {
            ControllerProfile::Legacy => {
                let configured = env.var("SLSKD_HTTP_IP_ADDRESS");
                let ips = match configured.as_deref() {
                    Some(value) if !value.trim().is_empty() => value
                        .split(',')
                        .map(str::trim)
                        .map(|value| {
                            parse_compat_ip_address(value)
                                .map_err(|error| format!("invalid SLSKD_HTTP_IP_ADDRESS: {error}"))
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
            ControllerProfile::Native => {
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
        .or(web_socket)
        .filter(|path| !path.as_os_str().is_empty());
    if controller_socket
        .as_deref()
        .is_some_and(|path| !path.is_absolute())
    {
        return Err("web.socket must be an absolute path".to_owned());
    }
    let mut controller_url_base = env
        .var("SLSKD_URL_BASE")
        .or(web_url_base)
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
        .or(web_content_path);
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
        return Err("web.content_path must be relative to the application directory".to_owned());
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
    let https_disabled =
        env_bool_layer(env, "SLSKD_NO_HTTPS", web_https.disabled.unwrap_or(false))?;
    let https_port = env_parse_layer(env, "SLSKD_HTTPS_PORT", web_https.port, 5031_u16)?;
    let https_configured_ip_address = env.var("SLSKD_HTTPS_IP_ADDRESS").or(web_https.ip_address);
    let https_ips = match controller_profile {
        ControllerProfile::Legacy => match https_configured_ip_address.as_deref() {
            Some(value) if !value.trim().is_empty() => value
                .split(',')
                .map(str::trim)
                .map(parse_compat_ip_address)
                .collect::<Result<Vec<_>, _>>()?,
            _ => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
        },
        ControllerProfile::Native => vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)],
    };
    let https_certificate_pfx = env
        .var("SLSKD_HTTPS_CERT_PFX")
        .map(PathBuf::from)
        .or(web_https.certificate.pfx)
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
        logging: env_bool_layer(env, "SLSKD_HTTP_LOGGING", web_logging.unwrap_or(false))?,
        https: ControllerHttpsSettings {
            disabled: https_disabled,
            binds: https_ips
                .into_iter()
                .map(|ip| SocketAddr::new(ip, https_port))
                .collect(),
            configured_ip_address: https_configured_ip_address,
            force: env_bool_layer(env, "SLSKD_HTTPS_FORCE", web_https.force.unwrap_or(false))?,
            certificate_pfx: https_certificate_pfx,
            certificate_password: env
                .var("SLSKD_HTTPS_CERT_PASSWORD")
                .or(web_https.certificate.password)
                .unwrap_or_default(),
        },
    };
    Ok(ControllerWebResolution {
        http_bind,
        http_binds,
        controller_http_address,
        controller_web,
    })
}

struct ApiAndWebHardeningSettings {
    api_token: Option<String>,
    api_read_write_token: Option<String>,
    api_read_only_token: Option<String>,
    api_nowplaying_token: Option<String>,
    auth_required: bool,
    api_cookie_auth_enabled: bool,
    api_rate_limit_anonymous: u32,
    api_rate_limit_authenticated: u32,
    controller_web_max_request_body_size: usize,
    controller_web_enforce_security: bool,
    controller_web_allow_remote_no_auth: bool,
    controller_web_passthrough_allowed_cidrs: Option<String>,
    controller_web_passthrough_cidrs: Vec<TrustedProxyCidr>,
    controller_diagnostics_allow_memory_dump: bool,
    controller_diagnostics_allow_remote_dump: bool,
    controller_web_cors: ControllerWebCorsSettings,
    controller_web_rate_limiting: ControllerWebRateLimitingSettings,
}

#[allow(clippy::too_many_arguments)]
fn resolve_api_and_web_hardening<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    auth_api_token: Option<String>,
    auth_read_write_token: Option<String>,
    auth_read_only_token: Option<String>,
    auth_nowplaying_token: Option<String>,
    auth_disabled: Option<bool>,
    auth_cookie_auth_enabled: Option<bool>,
    auth_rate_limit_anonymous: Option<u32>,
    auth_rate_limit_authenticated: Option<u32>,
    web_max_request_body_size: Option<i64>,
    web_enforce_security: Option<bool>,
    web_allow_remote_no_auth: Option<bool>,
    web_passthrough_allowed_cidrs: Option<String>,
    diagnostics_allow_memory_dump: Option<bool>,
    diagnostics_allow_remote_dump: Option<bool>,
    web_cors: WebCorsFileConfig,
    web_rate_limiting: WebRateLimitingFileConfig,
) -> Result<ApiAndWebHardeningSettings, String> {
    let api_token = env.var("SLSKR_API_TOKEN").or(auth_api_token);
    let api_read_write_token = env
        .var("SLSKR_API_READ_WRITE_TOKEN")
        .or(auth_read_write_token);
    let api_read_only_token = env
        .var("SLSKR_API_READ_ONLY_TOKEN")
        .or(auth_read_only_token);
    let api_nowplaying_token = env
        .var("SLSKR_API_NOWPLAYING_TOKEN")
        .or(auth_nowplaying_token);
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
    let auth_disabled = resolve_auth_disabled(env, auth_disabled.unwrap_or(false))?;
    let auth_required = !auth_disabled;
    let api_cookie_auth_enabled = env_bool_layer(
        env,
        "SLSKR_API_COOKIE_AUTH_ENABLED",
        auth_cookie_auth_enabled.unwrap_or(false),
    )?;
    let api_rate_limit_anonymous = env_parse_layer(
        env,
        "SLSKR_API_RATE_LIMIT_ANONYMOUS",
        auth_rate_limit_anonymous,
        1000_u32,
    )?;
    let api_rate_limit_authenticated = env_parse_layer(
        env,
        "SLSKR_API_RATE_LIMIT_AUTHENTICATED",
        auth_rate_limit_authenticated,
        5000_u32,
    )?;
    let web_max_request_body_size_default = if controller_profile == ControllerProfile::Native {
        10 * 1024 * 1024
    } else {
        crate::http_server::BODY_SIZE_LIMIT as i64
    };
    let controller_web_max_request_body_size = env_parse_layer(
        env,
        "SLSKD_WEB_MAX_REQUEST_BODY_SIZE",
        web_max_request_body_size,
        web_max_request_body_size_default,
    )?;
    if !(1..=i32::MAX as i64).contains(&controller_web_max_request_body_size) {
        return Err("web.max_request_body_size must be between 1 and 2147483647".to_owned());
    }
    let controller_web_max_request_body_size = controller_web_max_request_body_size as usize;
    let controller_web_enforce_security = env_bool_layer(
        env,
        "SLSKD_ENFORCE_SECURITY",
        web_enforce_security.unwrap_or(false),
    )?;
    let controller_web_allow_remote_no_auth = env_bool_layer(
        env,
        "SLSKD_ALLOW_REMOTE_NO_AUTH",
        web_allow_remote_no_auth.unwrap_or(false),
    )?;
    let controller_web_passthrough_allowed_cidrs = env
        .var("SLSKD_PASSTHROUGH_ALLOWED_CIDRS")
        .or(web_passthrough_allowed_cidrs);
    let controller_web_passthrough_cidrs =
        controller_passthrough_cidrs(controller_web_passthrough_allowed_cidrs.as_deref());
    let controller_diagnostics_allow_memory_dump = env_bool_layer(
        env,
        "SLSKD_ALLOW_MEMORY_DUMP",
        diagnostics_allow_memory_dump.unwrap_or(false),
    )?;
    let controller_diagnostics_allow_remote_dump = env_bool_layer(
        env,
        "SLSKD_ALLOW_REMOTE_DUMP",
        diagnostics_allow_remote_dump.unwrap_or(false),
    )?;
    let controller_web_cors = ControllerWebCorsSettings {
        enabled: env_bool_layer(
            env,
            "SLSKD_WEB_CORS_ENABLED",
            web_cors.enabled.unwrap_or(false),
        )?,
        allow_credentials: env_bool_layer(
            env,
            "SLSKD_WEB_CORS_ALLOW_CREDENTIALS",
            web_cors.allow_credentials.unwrap_or(false),
        )?,
        allowed_origins: controller_string_array_layer(
            env,
            "SLSKD_WEB_CORS_ALLOWED_ORIGINS",
            web_cors.allowed_origins,
        ),
        allowed_headers: controller_string_array_layer(
            env,
            "SLSKD_WEB_CORS_ALLOWED_HEADERS",
            web_cors.allowed_headers,
        ),
        allowed_methods: controller_string_array_layer(
            env,
            "SLSKD_WEB_CORS_ALLOWED_METHODS",
            web_cors.allowed_methods,
        ),
    };
    let web_rate_limiting_defaults =
        ControllerWebRateLimitingSettings::defaults(controller_profile);
    let controller_web_rate_limiting = ControllerWebRateLimitingSettings {
        enabled: env_bool_layer(
            env,
            "SLSKD_WEB_RATE_LIMITING",
            web_rate_limiting
                .enabled
                .unwrap_or(web_rate_limiting_defaults.enabled),
        )?,
        api_permit_limit: env_parse_layer(
            env,
            "SLSKD_WEB_API_PERMIT_LIMIT",
            web_rate_limiting.api_permit_limit,
            web_rate_limiting_defaults.api_permit_limit,
        )?,
        api_window_seconds: env_parse_layer(
            env,
            "SLSKD_WEB_API_WINDOW_SECONDS",
            web_rate_limiting.api_window_seconds,
            web_rate_limiting_defaults.api_window_seconds,
        )?,
        federation_permit_limit: env_parse_layer(
            env,
            "SLSKD_WEB_FEDERATION_PERMIT_LIMIT",
            web_rate_limiting.federation_permit_limit,
            web_rate_limiting_defaults.federation_permit_limit,
        )?,
        federation_window_seconds: env_parse_layer(
            env,
            "SLSKD_WEB_FEDERATION_WINDOW_SECONDS",
            web_rate_limiting.federation_window_seconds,
            web_rate_limiting_defaults.federation_window_seconds,
        )?,
        mesh_gateway_permit_limit: env_parse_layer(
            env,
            "SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT",
            web_rate_limiting.mesh_gateway_permit_limit,
            web_rate_limiting_defaults.mesh_gateway_permit_limit,
        )?,
        mesh_gateway_window_seconds: env_parse_layer(
            env,
            "SLSKD_WEB_MESH_GATEWAY_WINDOW_SECONDS",
            web_rate_limiting.mesh_gateway_window_seconds,
            web_rate_limiting_defaults.mesh_gateway_window_seconds,
        )?,
    };
    Ok(ApiAndWebHardeningSettings {
        api_token,
        api_read_write_token,
        api_read_only_token,
        api_nowplaying_token,
        auth_required,
        api_cookie_auth_enabled,
        api_rate_limit_anonymous,
        api_rate_limit_authenticated,
        controller_web_max_request_body_size,
        controller_web_enforce_security,
        controller_web_allow_remote_no_auth,
        controller_web_passthrough_allowed_cidrs,
        controller_web_passthrough_cidrs,
        controller_diagnostics_allow_memory_dump,
        controller_diagnostics_allow_remote_dump,
        controller_web_cors,
        controller_web_rate_limiting,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_soulseek_identity<E: ConfigEnv>(
    env: &E,
    state_dir: &Path,
    network_server_address: Option<String>,
    network_listen_port: Option<u32>,
    network_username: Option<String>,
    network_password: Option<String>,
    network_credential_store: Option<String>,
    network_credential_file: Option<PathBuf>,
    app_auto_connect: Option<bool>,
) -> Result<SoulseekIdentity, String> {
    let configured_server_address = env
        .var("SLSK_SERVER")
        .or(network_server_address)
        .unwrap_or_else(|| CONTROLLER_DEFAULT_SERVER_ADDRESS.to_owned());
    let (configured_server_host, configured_server_port) =
        split_server_address(&configured_server_address)?;
    let server_host = env
        .var("SLSKD_SLSK_ADDRESS")
        .unwrap_or(configured_server_host);
    let server_port = env_parse_any_layer(env, &["SLSKD_SLSK_PORT"], None, configured_server_port)?;
    let server_address = format_host_port(&server_host, server_port);
    let listen_port = env_parse_any_layer(
        env,
        &["SLSK_LISTEN_PORT", "SLSKD_SLSK_LISTEN_PORT"],
        network_listen_port,
        CONTROLLER_DEFAULT_LISTEN_PORT,
    )?;
    if !(1024..=65_535).contains(&listen_port) {
        return Err("Soulseek.ListenPort must be between 1024 and 65535".to_owned());
    }
    let username =
        optional_env_any(env, &["SLSK_USERNAME", "SLSKD_SLSK_USERNAME"]).or(network_username);
    let password =
        optional_env_any(env, &["SLSK_PASSWORD", "SLSKD_SLSK_PASSWORD"]).or(network_password);
    let credential_store = CredentialStoreMode::parse(
        env.var("SLSKR_CREDENTIAL_STORE")
            .or(network_credential_store)
            .unwrap_or_else(|| "os".to_owned())
            .as_str(),
    )?;
    let credential_file = env
        .var("SLSKR_CREDENTIAL_FILE")
        .map(PathBuf::from)
        .or(network_credential_file)
        .unwrap_or_else(|| state_dir.join("soulseek-credentials.json"));
    let auto_connect_default = app_auto_connect.unwrap_or(
        username.is_some() && password.is_some() || credential_store.auto_connect_default(),
    );
    let auto_connect = if env.var("SLSKR_AUTO_CONNECT").is_some() {
        env_bool_layer(env, "SLSKR_AUTO_CONNECT", auto_connect_default)?
    } else if env.var("SLSKD_NO_CONNECT").is_some() {
        !env_bool_layer(env, "SLSKD_NO_CONNECT", false)?
    } else {
        auto_connect_default
    };
    Ok(SoulseekIdentity {
        server_address,
        listen_port,
        username,
        password,
        credential_store,
        credential_file,
        auto_connect,
    })
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

fn resolve_auth_disabled<E: ConfigEnv>(env: &E, configured: bool) -> Result<bool, String> {
    env_bool_any_layer(env, &["SLSKR_AUTH_DISABLED", "SLSKD_NO_AUTH"], configured)
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
#[path = "config_tests.rs"]
mod tests;
