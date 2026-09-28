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

#[path = "config_integrations.rs"]
mod integrations;
pub use integrations::*;

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

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

#[path = "config_parts/environment_layers.rs"]
mod environment_layers;
pub use environment_layers::optional_env_any;
use environment_layers::{
    bounded_config_value, controller_string_array_layer, env_bool_any_layer, env_bool_layer,
    env_parse_any_layer, env_parse_any_option, env_parse_layer, env_parse_option_layer,
    normalized_controller_values, profile_env_names, resolve_misc_controller_flags,
    string_array_any_layer, validated_runtime_interval, MiscControllerFlags,
};
#[path = "config_parts/federation_membership.rs"]
mod federation_membership;
pub use federation_membership::{
    FederationPublishingSettings, PodSignatureMode, SocialFederationSettings,
};
#[path = "config_parts/file_loading.rs"]
mod file_loading;
use file_loading::validate_controller_storage_directory;
#[cfg(test)]
use file_loading::{config_contains_sensitive_values, read_file_config};
pub use file_loading::{default_state_dir, load_file_config, CredentialStoreMode};
#[path = "config_parts/media_services.rs"]
mod media_services;
pub use media_services::{
    FeatureGateSettings, MediaAdvancedServiceSettings, SolidSettings,
    VirtualSoulfindBridgeSettings, VirtualSoulfindDisasterModeSettings, VirtualSoulfindSettings,
};
#[path = "config_parts/network_security.rs"]
mod network_security;
pub use network_security::{
    AdvancedNetworkingSettings, AdversarialSettings, ContentSafetySettings, DhtSettings,
    MeshGatewaySettings, MeshRuntimeSettings, MeshSyncSecuritySettings, NetworkGuardSettings,
    OverlayDataSettings, OverlaySettings, PathGuardSettings, PeerReputationSettings,
    RelayAgentSettings, RelayControllerSettings, RelaySettings, SecuritySettings,
    SignalChannelSettings, SignalSystemSettings, ViolationTrackerSettings,
};
#[path = "config_parts/peer_transport.rs"]
mod peer_transport;
pub use peer_transport::{
    parse_compat_ip_address, PrivateMessageAutoResponseSettings, SoulseekObfuscationMode,
    TrustedMeshPeer,
};
use peer_transport::{
    resolve_listener_and_obfuscation, resolve_peer_profile, resolve_soulseek_identity,
    ListenerAndObfuscationResolution, PeerProfileSettings, SoulseekIdentity,
};
#[path = "config_parts/projection.rs"]
mod projection;
pub use projection::{
    json_bool_option, json_escape, json_option, json_u32_option, json_u64_option,
    json_usize_option, redact_username,
};
#[path = "config_parts/startup.rs"]
mod startup;
#[path = "config_parts/transfer_policy.rs"]
mod transfer_policy;
pub use transfer_policy::{
    validate_managed_blacklist_file_format, ManagedBlacklistSettings, ShareDirectory,
    ShareSettings, TransferAutoRetrySettings, TransferRescueSettings,
};
// Preserve established configuration paths for these public helpers.
#[allow(unused_imports)]
pub use transfer_policy::{
    parse_share_directories, parse_share_entries, parse_share_entry, ManagedBlacklistRange,
};
#[path = "config_parts/web_security.rs"]
mod web_security;
pub use web_security::TrustedProxyCidr;
use web_security::{
    resolve_api_and_web_hardening, resolve_auth_disabled, resolve_controller_api_keys,
    resolve_controller_web, resolve_controller_web_auth, trusted_proxy_cidrs_from_layers,
    ApiAndWebHardeningSettings, ControllerWebAuthSettings, ControllerWebResolution,
};
