use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DhtFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "port")]
    pub(in crate::config) dht_port: Option<u16>,
    pub(in crate::config) overlay_port: Option<u16>,
    pub(in crate::config) advertised_overlay_port: Option<u16>,
    pub(in crate::config) vpn_port_sync: Option<String>,
    pub(in crate::config) bootstrap_routers: Option<Vec<String>>,
    pub(in crate::config) announce_interval_seconds: Option<u64>,
    pub(in crate::config) discovery_interval_seconds: Option<u64>,
    pub(in crate::config) min_neighbors: Option<usize>,
    pub(in crate::config) bootstrap_timeout_seconds: Option<u64>,
    pub(in crate::config) cold_bootstrap_timeout_seconds: Option<u64>,
    pub(in crate::config) lan_only_bootstrap_timeout_seconds: Option<u64>,
    pub(in crate::config) lan_only: Option<bool>,
    pub(in crate::config) enable_upnp: Option<bool>,
    pub(in crate::config) enable_stun: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshFileConfig {
    pub(in crate::config) trusted_peers: Vec<TrustedMeshPeerInput>,
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "enableOverlay", alias = "EnableOverlay")]
    pub(in crate::config) enable_overlay: Option<bool>,
    #[serde(alias = "enableDht", alias = "EnableDht")]
    pub(in crate::config) enable_dht: Option<bool>,
    #[serde(alias = "enableStun", alias = "EnableStun")]
    pub(in crate::config) enable_stun: Option<bool>,
    pub(in crate::config) enable_soulseek_capability_handshake: Option<bool>,
    pub(in crate::config) enable_soulseek_rendezvous: Option<bool>,
    pub(in crate::config) probe_soulseek_rendezvous_capabilities: Option<bool>,
    pub(in crate::config) dht: MeshDhtFileConfig,
    pub(in crate::config) overlay: MeshPortsFileConfig,
    pub(in crate::config) security: MeshSecurityFileConfig,
    pub(in crate::config) sync_security: MeshSyncSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(in crate::config) struct MeshGatewayFileConfig {
    #[serde(alias = "Enabled")]
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "BindAddress", alias = "bind_address")]
    pub(in crate::config) bind_address: Option<String>,
    #[serde(alias = "Port")]
    pub(in crate::config) port: Option<u16>,
    #[serde(alias = "ApiKey", alias = "api_key")]
    pub(in crate::config) api_key: Option<String>,
    #[serde(alias = "CsrfToken", alias = "csrf_token")]
    pub(in crate::config) csrf_token: Option<String>,
    #[serde(alias = "AllowedServices", alias = "allowed_services")]
    pub(in crate::config) allowed_services: Option<Vec<String>>,
    #[serde(alias = "MaxRequestBodyBytes", alias = "max_request_body_bytes")]
    pub(in crate::config) max_request_body_bytes: Option<usize>,
    #[serde(alias = "RequestTimeoutSeconds", alias = "request_timeout_seconds")]
    pub(in crate::config) request_timeout_seconds: Option<u64>,
    #[serde(alias = "LogBodies", alias = "log_bodies")]
    pub(in crate::config) log_bodies: Option<bool>,
    #[serde(
        alias = "RequireRiskAcknowledgment",
        alias = "require_risk_acknowledgment"
    )]
    pub(in crate::config) require_risk_acknowledgment: Option<bool>,
    #[serde(alias = "IUnderstandTheRisk", alias = "i_understand_the_risk")]
    pub(in crate::config) i_understand_the_risk: Option<bool>,
    #[serde(alias = "AllowedOrigins", alias = "allowed_origins")]
    pub(in crate::config) allowed_origins: Option<Vec<String>>,
    #[serde(alias = "EnableRateLimiting", alias = "enable_rate_limiting")]
    pub(in crate::config) enable_rate_limiting: Option<bool>,
    #[serde(alias = "MaxRequestsPerMinute", alias = "max_requests_per_minute")]
    pub(in crate::config) max_requests_per_minute: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshDhtFileConfig {
    pub(in crate::config) bootstrap_nodes: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshPortsFileConfig {
    pub(in crate::config) udp_port: Option<u16>,
    pub(in crate::config) quic_port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct MeshSecurityFileConfig {
    pub(in crate::config) enforce_remote_payload_limits: Option<bool>,
    pub(in crate::config) max_remote_payload_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshSyncRootFileConfig {
    pub(in crate::config) sync_security: MeshSyncSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshSyncSecurityFileConfig {
    pub(in crate::config) max_invalid_entries_per_window: Option<u32>,
    pub(in crate::config) max_invalid_messages_per_window: Option<u32>,
    pub(in crate::config) rate_limit_window_minutes: Option<u64>,
    pub(in crate::config) quarantine_violation_threshold: Option<u32>,
    pub(in crate::config) quarantine_duration_minutes: Option<u64>,
    pub(in crate::config) proof_of_possession_enabled: Option<bool>,
    #[serde(alias = "requireSignedEntries", alias = "RequireSignedEntries")]
    pub(in crate::config) require_signed_entries: Option<bool>,
    pub(in crate::config) consensus_min_peers: Option<usize>,
    pub(in crate::config) consensus_min_agreements: Option<usize>,
    pub(in crate::config) alert_threshold_signature_failures: Option<u32>,
    pub(in crate::config) alert_threshold_rate_limit_violations: Option<u32>,
    pub(in crate::config) alert_threshold_quarantine_events: Option<u32>,
}

impl MeshSyncSecurityFileConfig {
    pub(super) fn is_configured(&self) -> bool {
        self.max_invalid_entries_per_window.is_some()
            || self.max_invalid_messages_per_window.is_some()
            || self.rate_limit_window_minutes.is_some()
            || self.quarantine_violation_threshold.is_some()
            || self.quarantine_duration_minutes.is_some()
            || self.proof_of_possession_enabled.is_some()
            || self.require_signed_entries.is_some()
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
    pub(in crate::config) enable: Option<bool>,
    pub(in crate::config) listen_port: Option<u16>,
    pub(in crate::config) enable_quic: Option<bool>,
    pub(in crate::config) quic_listen_port: Option<u16>,
    pub(in crate::config) share_quic_with_dht_port: Option<bool>,
    pub(in crate::config) quic_backend_listen_port: Option<u16>,
    pub(in crate::config) trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayDataFileConfig {
    pub(in crate::config) enable: Option<bool>,
    pub(in crate::config) listen_port: Option<u16>,
    pub(in crate::config) share_with_dht_port: Option<bool>,
    pub(in crate::config) backend_listen_port: Option<u16>,
    pub(in crate::config) max_concurrent_streams: Option<usize>,
    pub(in crate::config) relay_authentication_token: Option<String>,
    pub(in crate::config) allowed_relay_destinations: Vec<String>,
    pub(in crate::config) max_concurrent_relays: Option<usize>,
    pub(in crate::config) max_relay_bytes_per_direction: Option<u64>,
    pub(in crate::config) max_relay_duration_seconds: Option<u64>,
    pub(in crate::config) trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) mode: Option<String>,
    pub(in crate::config) controller: RelayControllerFileConfig,
    pub(in crate::config) agents: BTreeMap<String, RelayAgentFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayControllerFileConfig {
    pub(in crate::config) address: Option<String>,
    pub(in crate::config) ignore_certificate_errors: Option<bool>,
    pub(in crate::config) pinned_spki: Option<String>,
    pub(in crate::config) api_key: Option<String>,
    pub(in crate::config) secret: Option<String>,
    pub(in crate::config) downloads: Option<bool>,
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
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) profile: Option<String>,
    pub(in crate::config) network_guard: NetworkGuardFileConfig,
    pub(in crate::config) path_guard: PathGuardFileConfig,
    pub(in crate::config) content_safety: ContentSafetyFileConfig,
    pub(in crate::config) peer_reputation: PeerReputationFileConfig,
    pub(in crate::config) violation_tracker: ViolationTrackerFileConfig,
    pub(in crate::config) adversarial: AdversarialFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkGuardFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) max_connections_per_ip: Option<usize>,
    pub(in crate::config) max_global_connections: Option<usize>,
    pub(in crate::config) max_messages_per_minute: Option<u32>,
    pub(in crate::config) max_message_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PathGuardFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) max_path_length: Option<usize>,
    pub(in crate::config) max_path_depth: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentSafetyFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) verify_magic_bytes: Option<bool>,
    pub(in crate::config) quarantine_suspicious: Option<bool>,
    pub(in crate::config) quarantine_directory: Option<PathBuf>,
    pub(in crate::config) block_executables: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PeerReputationFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) trusted_threshold: Option<u8>,
    pub(in crate::config) untrusted_threshold: Option<u8>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViolationTrackerFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) violations_before_auto_ban: Option<u32>,
    pub(in crate::config) base_ban_duration_minutes: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialFileConfig {
    pub(in crate::config) privacy: AdversarialPrivacyFileConfig,
    pub(in crate::config) anonymity: AdversarialAnonymityFileConfig,
    #[serde(flatten)]
    pub(in crate::config) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialPrivacyFileConfig {
    pub(in crate::config) padding: AdversarialPaddingFileConfig,
    #[serde(flatten)]
    pub(in crate::config) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialPaddingFileConfig {
    pub(in crate::config) max_unpadded_bytes: Option<usize>,
    pub(in crate::config) max_padded_bytes: Option<usize>,
    #[serde(flatten)]
    pub(in crate::config) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialAnonymityFileConfig {
    pub(in crate::config) relay_only: AdversarialRelayOnlyFileConfig,
    #[serde(flatten)]
    pub(in crate::config) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialRelayOnlyFileConfig {
    pub(in crate::config) relay_peer_data_endpoints: Vec<String>,
    pub(in crate::config) relay_authentication_token: Option<String>,
    #[serde(flatten)]
    pub(in crate::config) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(in crate::config) struct AdvancedNetworkingFileOverlay {
    pub(in crate::config) dht: Option<DhtFileConfig>,
    #[serde(rename = "Mesh")]
    pub(in crate::config) mesh_sync: Option<MeshSyncRootFileConfig>,
    pub(in crate::config) mesh: Option<MeshFileConfig>,
    #[serde(
        rename = "SignalSystem",
        alias = "signalSystem",
        alias = "signal_system"
    )]
    pub(in crate::config) signal_system: Option<SignalSystemFileConfig>,
    pub(in crate::config) overlay: Option<OverlayFileConfig>,
    pub(in crate::config) overlay_data: Option<OverlayDataFileConfig>,
    pub(in crate::config) relay: Option<RelayFileConfig>,
    pub(in crate::config) security: Option<SecurityFileConfig>,
    #[serde(rename = "PodCore", alias = "podcore")]
    pub(in crate::config) podcore: Option<PodCoreFileConfig>,
}

impl AdvancedNetworkingSettings {
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
        file: &FileConfig,
        env: &E,
        target: ControllerProfile,
        current_upstream_behavior: bool,
        _state_dir: &Path,
    ) -> Result<Self, String> {
        let native_profile = target == ControllerProfile::Native;
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
        let signal_system_file = yaml_overlay
            .signal_system
            .as_ref()
            .unwrap_or(&file.signal_system);
        let mesh_sync_file = yaml_overlay.mesh_sync.as_ref().unwrap_or(&file.mesh_sync);
        let overlay_file = yaml_overlay.overlay.as_ref().unwrap_or(&file.overlay);
        let overlay_data_file = yaml_overlay
            .overlay_data
            .as_ref()
            .unwrap_or(&file.overlay_data);
        let relay_file = yaml_overlay.relay.as_ref().unwrap_or(&file.relay);
        let security_file = yaml_overlay.security.as_ref().unwrap_or(&file.security);
        let podcore_file = yaml_overlay.podcore.as_ref().unwrap_or(&file.podcore);
        // Current upstream defaults the independent public UDP/QUIC socket
        // family to 50300, which also happens to be the default Soulseek TCP
        // port. A customized Soulseek TCP port does not move those UDP
        // defaults; only the shared TCP overlay advertisement follows the
        // customized listener after listener resolution below.
        let current_public_port = if native_profile && current_upstream_behavior {
            50_300_u16
        } else {
            50_305_u16
        };
        let dht_enabled = env_bool_layer(
            env,
            "SLSKR_DHT_ENABLED",
            dht_file.enabled.unwrap_or(native_profile),
        )?;
        let dht_port = env_parse_layer(
            env,
            "SLSKR_DHT_PORT",
            dht_file.dht_port,
            if native_profile {
                current_public_port
            } else {
                0_u16
            },
        )?;
        if dht_enabled && dht_port == 0 {
            return Err("dht.dht_port must be between 1 and 65535 when DHT is enabled".to_owned());
        }
        // The upstream DHT overlay option remains 50305 until startup mutates
        // it to the Soulseek listener port when shared TCP mode is active.
        let overlay_port = dht_file.overlay_port.unwrap_or(50_305);
        if native_profile && overlay_port == 0 {
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
            enabled: mesh_file.enabled.unwrap_or(native_profile),
            enable_overlay: mesh_file.enable_overlay.unwrap_or(true),
            enable_dht: mesh_file.enable_dht.unwrap_or(true),
            enable_stun: mesh_file.enable_stun.unwrap_or(true),
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

        let signal_system = SignalSystemSettings::from_layers(signal_system_file, env)?;

        // .NET configuration keys are case-insensitive, so the documented
        // `Mesh:SyncSecurity` section and the lower-case YAML `mesh` section
        // are one logical tree. Accept the split spelling as well for older
        // native profile examples that emitted both roots.
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
            require_signed_entries: sync.require_signed_entries.unwrap_or(false),
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
            listen_port: overlay_file.listen_port.unwrap_or(current_public_port),
            enable_quic: overlay_file.enable_quic.unwrap_or(true),
            quic_listen_port: overlay_file.quic_listen_port.unwrap_or(current_public_port),
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
            listen_port: overlay_data_file.listen_port.unwrap_or(
                if native_profile && current_upstream_behavior {
                    current_public_port
                } else {
                    50_401
                },
            ),
            share_with_dht_port: overlay_data_file.share_with_dht_port.unwrap_or(true),
            backend_listen_port: overlay_data_file.backend_listen_port.unwrap_or(55_401),
            max_concurrent_streams: overlay_data_file.max_concurrent_streams.unwrap_or(8),
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
            || (overlay_data.share_with_dht_port && overlay_data.backend_listen_port == 0)
            || overlay_data.max_concurrent_streams == 0
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
        if relay_enabled
            && relay_mode == "agent"
            && (!(controller.address.starts_with("http://")
                || controller.address.starts_with("https://"))
                || !(16..=255).contains(&controller.api_key.len())
                || !(16..=255).contains(&controller.secret.len()))
        {
            return Err("relay agent mode requires a controller URL and 16-255 character API key and secret".to_owned());
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
            signal_system,
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
            gold_star_club_autojoin: env_bool_layer(
                env,
                "SLSKR_POD_GOLD_STAR_CLUB_AUTOJOIN",
                podcore_file
                    .gold_star_club
                    .autojoin
                    .unwrap_or(!current_upstream_behavior),
            )?,
            overlay,
            overlay_data,
            relay,
            security,
        })
    }
}

pub(super) fn validate_certificate_pins(
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
