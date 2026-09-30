use super::*;

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
    pub(super) fn from_layers<E: ConfigEnv>(
        file: &MeshGatewayFileConfig,
        env: &E,
    ) -> Result<Self, String> {
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
