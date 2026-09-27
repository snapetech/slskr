use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileConfig {
    pub(super) headless: Option<bool>,
    pub(super) flags: ControllerFlagsFileConfig,
    pub(super) logger: LoggerFileConfig,
    pub(super) permissions: PermissionsFileConfig,
    pub(super) telemetry: TelemetryFileConfig,
    pub(super) retention: RetentionFileConfig,
    pub(super) realm: RealmFileConfig,
    #[serde(rename = "multiRealm", alias = "multi_realm")]
    pub(super) multi_realm: MultiRealmFileConfig,
    #[serde(
        alias = "socialFederation",
        alias = "SocialFederation",
        alias = "social_federation"
    )]
    pub(super) social_federation: SocialFederationFileConfig,
    #[serde(
        alias = "federationPublishing",
        alias = "FederationPublishing",
        alias = "federation_publishing"
    )]
    pub(super) federation_publishing: FederationPublishingFileConfig,
    pub(super) filters: FiltersFileConfig,
    pub(super) app: AppFileConfig,
    pub(super) blacklist: ManagedBlacklistFileConfig,
    pub(super) feature: FeatureFileConfig,
    pub(super) player: PlayerFileConfig,
    pub(super) solid: SolidFileConfig,
    pub(super) song_id: SongIdFileConfig,
    #[serde(rename = "virtualSoulfind", alias = "virtual_soulfind")]
    pub(super) virtual_soulfind: VirtualSoulfindFileConfig,
    pub(super) metrics: MetricsFileConfig,
    pub(super) network: NetworkFileConfig,
    pub(super) listeners: ListenerFileConfig,
    pub(super) dht: DhtFileConfig,
    pub(super) auto_replace: AutoReplaceFileConfig,
    #[serde(rename = "Mesh")]
    pub(super) mesh_sync: MeshSyncRootFileConfig,
    pub(super) mesh: MeshFileConfig,
    #[serde(rename = "meshGateway", alias = "MeshGateway", alias = "mesh_gateway")]
    pub(super) mesh_gateway: MeshGatewayFileConfig,
    #[serde(
        rename = "SignalSystem",
        alias = "signalSystem",
        alias = "signal_system"
    )]
    pub(super) signal_system: SignalSystemFileConfig,
    pub(super) overlay: OverlayFileConfig,
    pub(super) overlay_data: OverlayDataFileConfig,
    pub(super) relay: RelayFileConfig,
    pub(super) security: SecurityFileConfig,
    pub(super) profile: ProfileFileConfig,
    pub(super) timeouts: TimeoutFileConfig,
    pub(super) shares: ShareFileConfig,
    pub(super) transfers: TransferFileConfig,
    pub(super) groups: GroupsFileConfig,
    pub(super) compatibility: CompatibilityFileConfig,
    pub(super) auth: AuthFileConfig,
    pub(super) web: WebFileConfig,
    pub(super) persistence: PersistenceFileConfig,
    pub(super) podcore: PodCoreFileConfig,
    pub(super) virtual_soulfind_v2: VirtualSoulfindV2FileConfig,
    #[serde(alias = "integration", alias = "Integration")]
    pub(super) integrations: IntegrationsFileConfig,
    pub(super) diagnostics: DiagnosticsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct SocialFederationFileConfig {
    #[serde(alias = "Enabled")]
    pub(super) enabled: Option<bool>,
    #[serde(alias = "Mode")]
    pub(super) mode: Option<String>,
    #[serde(alias = "Domain")]
    pub(super) domain: Option<String>,
    #[serde(alias = "BaseUrl", alias = "base_url")]
    pub(super) base_url: Option<String>,
    #[serde(alias = "ApprovedPeers", alias = "approved_peers")]
    pub(super) approved_peers: Vec<String>,
    #[serde(alias = "OutboxMaxActivities", alias = "outbox_max_activities")]
    pub(super) outbox_max_activities: Option<u32>,
    #[serde(alias = "PageSize", alias = "page_size")]
    pub(super) page_size: Option<u32>,
    #[serde(alias = "VerifySignatures", alias = "verify_signatures")]
    pub(super) verify_signatures: Option<bool>,
    #[serde(alias = "HttpTimeoutSeconds", alias = "http_timeout_seconds")]
    pub(super) http_timeout_seconds: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct FederationPublishingFileConfig {
    #[serde(alias = "Enabled")]
    pub(super) enabled: Option<bool>,
    #[serde(alias = "PublishableDomains", alias = "publishable_domains")]
    pub(super) publishable_domains: Vec<String>,
    #[serde(alias = "DefaultVisibility", alias = "default_visibility")]
    pub(super) default_visibility: Option<String>,
    #[serde(alias = "ApprovedCircles", alias = "approved_circles")]
    pub(super) approved_circles: Vec<String>,
    #[serde(
        alias = "RequireModerationApproval",
        alias = "require_moderation_approval"
    )]
    pub(super) require_moderation_approval: Option<bool>,
    #[serde(alias = "IncludeExternalLinks", alias = "include_external_links")]
    pub(super) include_external_links: Option<bool>,
    #[serde(alias = "MaxMetadataSizeKb", alias = "max_metadata_size_kb")]
    pub(super) max_metadata_size_kb: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RealmSettings {
    pub id: String,
    pub governance_roots: Vec<String>,
    pub bootstrap_nodes: Vec<String>,
    pub gossip_enabled: bool,
    pub replication_enabled: bool,
    pub max_gossip_hops: u32,
    pub gossip_interval_seconds: u64,
    pub federation_allowed: bool,
}

impl RealmSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: RealmFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let id = env
            .var("SLSKR_REALM_ID")
            .or_else(|| env.var("SLSKD_REALM_ID"))
            .or(file.id)
            .unwrap_or_else(|| DEFAULT_REALM_ID.to_owned())
            .trim()
            .to_owned();
        if id.len() < 3
            || id.len() > 64
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || id.starts_with('.')
            || id.ends_with('.')
            || id.contains("..")
        {
            return Err(
                "realm.id must be 3-64 characters of letters, numbers, hyphens, underscores, or periods and may not contain consecutive periods".to_owned(),
            );
        }

        let governance_roots = env
            .var("SLSKR_REALM_GOVERNANCE_ROOTS")
            .or_else(|| env.var("SLSKD_REALM_GOVERNANCE_ROOTS"))
            .map(|roots| {
                roots
                    .split(',')
                    .map(str::trim)
                    .filter(|root| !root.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or(file.governance_roots);
        if governance_roots.is_empty() || governance_roots.iter().any(|root| root.trim().is_empty())
        {
            return Err(
                "realm.governanceRoots must contain at least one non-empty root".to_owned(),
            );
        }
        let bootstrap_nodes = file
            .bootstrap_nodes
            .into_iter()
            .map(|node| node.trim().to_owned())
            .collect::<Vec<_>>();
        if bootstrap_nodes.iter().any(String::is_empty) {
            return Err("realm.bootstrapNodes cannot contain empty entries".to_owned());
        }
        if !(1..=10).contains(&file.policies.max_gossip_hops) {
            return Err("realm.policies.maxGossipHops must be between 1 and 10".to_owned());
        }
        if !(30..=3600).contains(&file.policies.gossip_interval_seconds) {
            return Err(
                "realm.policies.gossipIntervalSeconds must be between 30 and 3600".to_owned(),
            );
        }
        Ok(Self {
            id,
            governance_roots,
            bootstrap_nodes,
            gossip_enabled: file.policies.gossip_enabled,
            replication_enabled: file.policies.replication_enabled,
            max_gossip_hops: file.policies.max_gossip_hops,
            gossip_interval_seconds: file.policies.gossip_interval_seconds,
            federation_allowed: file.policies.federation_allowed,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RealmFileConfig {
    pub(super) id: Option<String>,
    #[serde(alias = "governance_roots")]
    pub(super) governance_roots: Vec<String>,
    #[serde(alias = "bootstrap_nodes")]
    pub(super) bootstrap_nodes: Vec<String>,
    pub(super) policies: RealmPoliciesFileConfig,
}

impl Default for RealmFileConfig {
    fn default() -> Self {
        Self {
            id: Some(DEFAULT_REALM_ID.to_owned()),
            governance_roots: vec![DEFAULT_GOVERNANCE_ROOT.to_owned()],
            bootstrap_nodes: Vec::new(),
            policies: RealmPoliciesFileConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RealmPoliciesFileConfig {
    pub(super) gossip_enabled: bool,
    pub(super) replication_enabled: bool,
    pub(super) max_gossip_hops: u32,
    pub(super) gossip_interval_seconds: u64,
    pub(super) federation_allowed: bool,
}

impl Default for RealmPoliciesFileConfig {
    fn default() -> Self {
        Self {
            gossip_enabled: true,
            replication_enabled: true,
            max_gossip_hops: 3,
            gossip_interval_seconds: 300,
            federation_allowed: true,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct MultiRealmFileConfig {
    pub(super) realms: Vec<RealmFileConfig>,
    pub(super) bridge: MultiRealmBridgeFileConfig,
    pub(super) is_bridging_enabled: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct MultiRealmBridgeFileConfig {
    pub(super) enabled: bool,
    pub(super) allowed_flows: Vec<String>,
    pub(super) disallowed_flows: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiagnosticsFileConfig {
    pub(super) allow_memory_dump: Option<bool>,
    pub(super) allow_remote_dump: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebFileConfig {
    pub(super) socket: Option<PathBuf>,
    pub(super) url_base: Option<String>,
    pub(super) content_path: Option<PathBuf>,
    pub(super) logging: Option<bool>,
    pub(super) https: HttpsFileConfig,
    pub(super) enforce_security: Option<bool>,
    pub(super) allow_remote_no_auth: Option<bool>,
    pub(super) passthrough_allowed_cidrs: Option<String>,
    pub(super) max_request_body_size: Option<i64>,
    pub(super) cors: WebCorsFileConfig,
    pub(super) rate_limiting: WebRateLimitingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpsFileConfig {
    pub(super) disabled: Option<bool>,
    pub(super) port: Option<u16>,
    pub(super) ip_address: Option<String>,
    pub(super) force: Option<bool>,
    pub(super) certificate: HttpsCertificateFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpsCertificateFileConfig {
    pub(super) pfx: Option<PathBuf>,
    pub(super) password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebCorsFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) allow_credentials: Option<bool>,
    pub(super) allowed_origins: Vec<String>,
    pub(super) allowed_headers: Vec<String>,
    pub(super) allowed_methods: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebRateLimitingFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) api_permit_limit: Option<i32>,
    pub(super) api_window_seconds: Option<i32>,
    pub(super) federation_permit_limit: Option<i32>,
    pub(super) federation_window_seconds: Option<i32>,
    pub(super) mesh_gateway_permit_limit: Option<i32>,
    pub(super) mesh_gateway_window_seconds: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControllerFlagsFileConfig {
    pub(super) no_logo: Option<bool>,
    pub(super) no_start: Option<bool>,
    pub(super) no_version_check: Option<bool>,
    pub(super) experimental: Option<bool>,
    pub(super) hash_from_audio_file_enabled: Option<bool>,
    pub(super) case_sensitive_reg_ex: Option<bool>,
    pub(super) no_share_scan: Option<bool>,
    pub(super) force_share_scan: Option<bool>,
    pub(super) force_migrations: Option<bool>,
    pub(super) legacy_windows_tcp_keepalive: Option<bool>,
    pub(super) log_sql: Option<bool>,
    pub(super) log_unobserved_exceptions: Option<bool>,
    pub(super) optimistic_relay_file_info: Option<bool>,
    pub(super) volatile: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggerFileConfig {
    pub(super) disk: Option<bool>,
    pub(super) loki: Option<String>,
    pub(super) no_color: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PermissionsFileConfig {
    pub(super) file: FilePermissionsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilePermissionsFileConfig {
    pub(super) mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryFileConfig {
    pub(super) tracing: TelemetryTracingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryTracingFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) exporter: Option<String>,
    pub(super) jaeger_endpoint: Option<String>,
    pub(super) jaeger_port: Option<u16>,
    pub(super) otlp_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetentionFileConfig {
    pub(super) search: Option<u64>,
    pub(super) logs: Option<u64>,
    pub(super) transfers: TransferRetentionFileConfig,
    pub(super) files: FileRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferRetentionFileConfig {
    pub(super) upload: TransferTypeRetentionFileConfig,
    pub(super) download: TransferTypeRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferTypeRetentionFileConfig {
    pub(super) succeeded: Option<u64>,
    pub(super) errored: Option<u64>,
    pub(super) cancelled: Option<u64>,
    pub(super) failed: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileRetentionFileConfig {
    pub(super) complete: Option<u64>,
    pub(super) incomplete: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FiltersFileConfig {
    pub(super) search: SearchFiltersFileConfig,
    pub(super) search_retention: SearchRetentionFileConfig,
    pub(super) download: DownloadFiltersFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchFiltersFileConfig {
    pub(super) request: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchRetentionFileConfig {
    pub(super) max_age_days: Option<u64>,
    pub(super) max_count: Option<usize>,
    pub(super) cleanup_interval_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadFiltersFileConfig {
    pub(super) exclude: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupsFileConfig {
    pub(super) default: TransferGroupFileConfig,
    pub(super) leechers: LeecherTransferGroupFileConfig,
    pub(super) blacklisted: UserBlacklistFileConfig,
    pub(super) user_defined: BTreeMap<String, UserDefinedTransferGroupFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferGroupFileConfig {
    pub(super) upload: TransferGroupUploadFileConfig,
    pub(super) limits: Option<TransferLimitsFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LeecherTransferGroupFileConfig {
    pub(super) upload: TransferGroupUploadFileConfig,
    pub(super) limits: Option<TransferLimitsFileConfig>,
    pub(super) thresholds: LeecherThresholdFileConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserDefinedTransferGroupFileConfig {
    pub(super) upload: TransferGroupUploadFileConfig,
    pub(super) limits: Option<TransferLimitsFileConfig>,
    pub(super) members: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferGroupUploadFileConfig {
    pub(super) priority: Option<u32>,
    #[serde(alias = "queue_strategy")]
    pub(super) strategy: Option<String>,
    pub(super) slots: Option<u32>,
    pub(super) speed_limit: Option<u32>,
    pub(super) allowed_file_types: Vec<String>,
    pub(super) limits: Option<TransferLimitsFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LeecherThresholdFileConfig {
    pub(super) files: Option<u32>,
    pub(super) directories: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferLimitsFileConfig {
    pub(super) queued: NullableConfig<TransferLimitFileConfig>,
    pub(super) daily: NullableConfig<TransferLimitFileConfig>,
    pub(super) weekly: NullableConfig<TransferLimitFileConfig>,
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

#[derive(Clone, Debug, Default)]
pub(super) enum NullableConfig<T> {
    #[default]
    Missing,
    Null,
    Value(T),
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
    pub(super) files: Option<u32>,
    pub(super) megabytes: Option<u32>,
    pub(super) failures: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserBlacklistFileConfig {
    pub(super) members: Vec<String>,
    pub(super) patterns: Vec<String>,
    pub(super) cidrs: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct FeatureFileConfig {
    pub(super) swagger: Option<bool>,
    #[serde(alias = "CollectionsSharing")]
    pub(super) collections_sharing: Option<bool>,
    #[serde(alias = "Streaming")]
    pub(super) streaming: Option<bool>,
    #[serde(alias = "StreamingRelayFallback")]
    pub(super) streaming_relay_fallback: Option<bool>,
    #[serde(alias = "MeshParallelSearch")]
    pub(super) mesh_parallel_search: Option<bool>,
    #[serde(alias = "MeshPublishAvailability")]
    pub(super) mesh_publish_availability: Option<bool>,
    #[serde(alias = "IdentityFriends")]
    pub(super) identity_friends: Option<bool>,
    #[serde(alias = "Solid")]
    pub(super) solid: Option<bool>,
    #[serde(alias = "ScenePodBridge")]
    pub(super) scene_pod_bridge: Option<bool>,
    #[serde(alias = "SongId")]
    pub(super) song_id: Option<bool>,
    #[serde(alias = "Mesh")]
    pub(super) mesh: Option<bool>,
    #[serde(alias = "Dht")]
    pub(super) dht: Option<bool>,
    #[serde(alias = "Pods")]
    pub(super) pods: Option<bool>,
    #[serde(alias = "SocialFederation")]
    pub(super) social_federation: Option<bool>,
    #[serde(alias = "VirtualSoulfind")]
    pub(super) virtual_soulfind: Option<bool>,
    #[serde(alias = "MultiSourceDownloads")]
    pub(super) multi_source_downloads: Option<bool>,
    #[serde(alias = "ScenePodBridgeOptions")]
    pub(super) scene_pod_bridge_options: ScenePodBridgeFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ScenePodBridgeFileConfig {
    #[serde(alias = "ProxyTransfers")]
    pub(super) proxy_transfers: Option<bool>,
    #[serde(alias = "ExportPodAvailability")]
    pub(super) export_pod_availability: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerFileConfig {
    pub(super) external_visualizer: PlayerExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerExternalVisualizerFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) path: Option<String>,
    pub(super) arguments: Option<Vec<String>>,
    pub(super) working_directory: Option<PathBuf>,
    pub(super) name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SolidFileConfig {
    #[serde(alias = "allowInsecureHttp")]
    pub(super) allow_insecure_http: Option<bool>,
    #[serde(alias = "allowLocalhostForWebId")]
    pub(super) allow_localhost_for_web_id: Option<bool>,
    #[serde(alias = "maxFetchBytes")]
    pub(super) max_fetch_bytes: Option<usize>,
    #[serde(alias = "timeoutSeconds")]
    pub(super) timeout_seconds: Option<u64>,
    #[serde(alias = "allowedHosts")]
    pub(super) allowed_hosts: Option<Vec<String>>,
    #[serde(alias = "clientIdUrl")]
    pub(super) client_id_url: Option<String>,
    #[serde(alias = "redirectPath")]
    pub(super) redirect_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SongIdFileConfig {
    pub(super) max_concurrent_runs: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindFileConfig {
    pub(super) bridge: VirtualSoulfindBridgeFileConfig,
    pub(super) disaster_mode: VirtualSoulfindDisasterModeFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindBridgeFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) port: Option<u16>,
    pub(super) bind_address: Option<String>,
    pub(super) max_clients: Option<usize>,
    pub(super) require_auth: Option<bool>,
    pub(super) password: Option<String>,
    pub(super) max_requests_per_minute: Option<u32>,
    pub(super) max_transfers_per_session: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindDisasterModeFileConfig {
    pub(super) auto: Option<bool>,
    pub(super) force: Option<bool>,
    pub(super) unavailable_threshold_minutes: Option<u64>,
    pub(super) enable_graceful_degradation: Option<bool>,
    pub(super) recovery_check_interval_minutes: Option<u64>,
    pub(super) recovery_healthy_checks_required: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) url: Option<String>,
    pub(super) authentication: MetricsAuthenticationFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsAuthenticationFileConfig {
    pub(super) disabled: Option<bool>,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManagedBlacklistFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) file: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CompatibilityFileConfig {
    pub(super) profile: Option<String>,
    pub(super) remote_configuration: Option<bool>,
    pub(super) debug: Option<bool>,
    pub(super) no_config_watch: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VirtualSoulfindV2FileConfig {
    pub(super) enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DhtFileConfig {
    pub(super) enabled: Option<bool>,
    #[serde(alias = "port")]
    pub(super) dht_port: Option<u16>,
    pub(super) overlay_port: Option<u16>,
    pub(super) advertised_overlay_port: Option<u16>,
    pub(super) vpn_port_sync: Option<String>,
    pub(super) bootstrap_routers: Option<Vec<String>>,
    pub(super) announce_interval_seconds: Option<u64>,
    pub(super) discovery_interval_seconds: Option<u64>,
    pub(super) min_neighbors: Option<usize>,
    pub(super) bootstrap_timeout_seconds: Option<u64>,
    pub(super) cold_bootstrap_timeout_seconds: Option<u64>,
    pub(super) lan_only_bootstrap_timeout_seconds: Option<u64>,
    pub(super) lan_only: Option<bool>,
    pub(super) enable_upnp: Option<bool>,
    pub(super) enable_stun: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshFileConfig {
    pub(super) trusted_peers: Vec<TrustedMeshPeerInput>,
    pub(super) enabled: Option<bool>,
    #[serde(alias = "enableOverlay", alias = "EnableOverlay")]
    pub(super) enable_overlay: Option<bool>,
    #[serde(alias = "enableDht", alias = "EnableDht")]
    pub(super) enable_dht: Option<bool>,
    #[serde(alias = "enableStun", alias = "EnableStun")]
    pub(super) enable_stun: Option<bool>,
    pub(super) enable_soulseek_capability_handshake: Option<bool>,
    pub(super) enable_soulseek_rendezvous: Option<bool>,
    pub(super) probe_soulseek_rendezvous_capabilities: Option<bool>,
    pub(super) dht: MeshDhtFileConfig,
    pub(super) overlay: MeshPortsFileConfig,
    pub(super) security: MeshSecurityFileConfig,
    pub(super) sync_security: MeshSyncSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct MeshGatewayFileConfig {
    #[serde(alias = "Enabled")]
    pub(super) enabled: Option<bool>,
    #[serde(alias = "BindAddress", alias = "bind_address")]
    pub(super) bind_address: Option<String>,
    #[serde(alias = "Port")]
    pub(super) port: Option<u16>,
    #[serde(alias = "ApiKey", alias = "api_key")]
    pub(super) api_key: Option<String>,
    #[serde(alias = "CsrfToken", alias = "csrf_token")]
    pub(super) csrf_token: Option<String>,
    #[serde(alias = "AllowedServices", alias = "allowed_services")]
    pub(super) allowed_services: Option<Vec<String>>,
    #[serde(alias = "MaxRequestBodyBytes", alias = "max_request_body_bytes")]
    pub(super) max_request_body_bytes: Option<usize>,
    #[serde(alias = "RequestTimeoutSeconds", alias = "request_timeout_seconds")]
    pub(super) request_timeout_seconds: Option<u64>,
    #[serde(alias = "LogBodies", alias = "log_bodies")]
    pub(super) log_bodies: Option<bool>,
    #[serde(
        alias = "RequireRiskAcknowledgment",
        alias = "require_risk_acknowledgment"
    )]
    pub(super) require_risk_acknowledgment: Option<bool>,
    #[serde(alias = "IUnderstandTheRisk", alias = "i_understand_the_risk")]
    pub(super) i_understand_the_risk: Option<bool>,
    #[serde(alias = "AllowedOrigins", alias = "allowed_origins")]
    pub(super) allowed_origins: Option<Vec<String>>,
    #[serde(alias = "EnableRateLimiting", alias = "enable_rate_limiting")]
    pub(super) enable_rate_limiting: Option<bool>,
    #[serde(alias = "MaxRequestsPerMinute", alias = "max_requests_per_minute")]
    pub(super) max_requests_per_minute: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshDhtFileConfig {
    pub(super) bootstrap_nodes: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshPortsFileConfig {
    pub(super) udp_port: Option<u16>,
    pub(super) quic_port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct MeshSecurityFileConfig {
    pub(super) enforce_remote_payload_limits: Option<bool>,
    pub(super) max_remote_payload_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshSyncRootFileConfig {
    pub(super) sync_security: MeshSyncSecurityFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MeshSyncSecurityFileConfig {
    pub(super) max_invalid_entries_per_window: Option<u32>,
    pub(super) max_invalid_messages_per_window: Option<u32>,
    pub(super) rate_limit_window_minutes: Option<u64>,
    pub(super) quarantine_violation_threshold: Option<u32>,
    pub(super) quarantine_duration_minutes: Option<u64>,
    pub(super) proof_of_possession_enabled: Option<bool>,
    #[serde(alias = "requireSignedEntries", alias = "RequireSignedEntries")]
    pub(super) require_signed_entries: Option<bool>,
    pub(super) consensus_min_peers: Option<usize>,
    pub(super) consensus_min_agreements: Option<usize>,
    pub(super) alert_threshold_signature_failures: Option<u32>,
    pub(super) alert_threshold_rate_limit_violations: Option<u32>,
    pub(super) alert_threshold_quarantine_events: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SignalSystemFileConfig {
    #[serde(alias = "Enabled")]
    pub(super) enabled: Option<bool>,
    #[serde(alias = "DeduplicationCacheSize")]
    pub(super) deduplication_cache_size: Option<usize>,
    #[serde(
        alias = "DefaultTtl",
        alias = "defaultTTL",
        alias = "default_ttl_seconds"
    )]
    pub(super) default_ttl: Option<SignalDurationFileValue>,
    #[serde(alias = "MeshChannel")]
    pub(super) mesh_channel: SignalChannelFileConfig,
    #[serde(alias = "BtExtensionChannel")]
    pub(super) bt_extension_channel: SignalChannelFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SignalChannelFileConfig {
    #[serde(alias = "Enabled")]
    pub(super) enabled: Option<bool>,
    #[serde(alias = "Priority")]
    pub(super) priority: Option<u8>,
    #[serde(alias = "RequireActiveSession")]
    pub(super) require_active_session: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SignalDurationFileValue {
    Seconds(u64),
    Text(String),
}

impl SignalSystemSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: &SignalSystemFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let enabled = env_bool_any_layer(
            env,
            &["SLSKD_SIGNALSYSTEM_ENABLED", "SLSKR_SIGNAL_SYSTEM_ENABLED"],
            file.enabled.unwrap_or(true),
        )?;
        let deduplication_cache_size = env_parse_any_layer(
            env,
            &[
                "SLSKD_SIGNALSYSTEM_DEDUPLICATIONCACHESIZE",
                "SLSKR_SIGNAL_SYSTEM_DEDUPLICATION_CACHE_SIZE",
            ],
            file.deduplication_cache_size,
            10_000_usize,
        )?;
        if !(100..=1_000_000).contains(&deduplication_cache_size) {
            return Err(
                "SignalSystem.DeduplicationCacheSize must be between 100 and 1000000".to_owned(),
            );
        }

        let default_ttl = match optional_env_any(
            env,
            &[
                "SLSKD_SIGNALSYSTEM_DEFAULTTTL",
                "SLSKR_SIGNAL_SYSTEM_DEFAULT_TTL",
            ],
        ) {
            Some(value) => parse_signal_duration("SignalSystem.DefaultTtl", &value)?,
            None => match file.default_ttl.as_ref() {
                Some(SignalDurationFileValue::Seconds(value)) => {
                    signal_duration_from_seconds("SignalSystem.DefaultTtl", *value)?
                }
                Some(SignalDurationFileValue::Text(value)) => {
                    parse_signal_duration("SignalSystem.DefaultTtl", value)?
                }
                None => Duration::from_secs(5 * 60),
            },
        };

        let channel = |file: &SignalChannelFileConfig,
                       enabled_names: &[&str],
                       priority_names: &[&str],
                       session_names: &[&str],
                       default_priority: u8,
                       default_session: bool|
         -> Result<SignalChannelSettings, String> {
            let enabled = env_bool_any_layer(env, enabled_names, file.enabled.unwrap_or(true))?;
            let priority =
                env_parse_any_layer(env, priority_names, file.priority, default_priority)?;
            let require_active_session = env_bool_any_layer(
                env,
                session_names,
                file.require_active_session.unwrap_or(default_session),
            )?;
            if !(1..=10).contains(&priority) {
                return Err(format!(
                    "SignalSystem channel priority must be between 1 and 10, got {priority}"
                ));
            }
            Ok(SignalChannelSettings {
                enabled,
                priority,
                require_active_session,
            })
        };

        Ok(Self {
            enabled,
            deduplication_cache_size,
            default_ttl,
            mesh_channel: channel(
                &file.mesh_channel,
                &[
                    "SLSKD_SIGNALSYSTEM_MESHCHANNEL_ENABLED",
                    "SLSKR_SIGNAL_SYSTEM_MESH_CHANNEL_ENABLED",
                ],
                &[
                    "SLSKD_SIGNALSYSTEM_MESHCHANNEL_PRIORITY",
                    "SLSKR_SIGNAL_SYSTEM_MESH_CHANNEL_PRIORITY",
                ],
                &[
                    "SLSKD_SIGNALSYSTEM_MESHCHANNEL_REQUIREACTIVESESSION",
                    "SLSKR_SIGNAL_SYSTEM_MESH_CHANNEL_REQUIRE_ACTIVE_SESSION",
                ],
                1,
                false,
            )?,
            bt_extension_channel: channel(
                &file.bt_extension_channel,
                &[
                    "SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_ENABLED",
                    "SLSKR_SIGNAL_SYSTEM_BT_EXTENSION_CHANNEL_ENABLED",
                ],
                &[
                    "SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_PRIORITY",
                    "SLSKR_SIGNAL_SYSTEM_BT_EXTENSION_CHANNEL_PRIORITY",
                ],
                &[
                    "SLSKD_SIGNALSYSTEM_BTEXTENSIONCHANNEL_REQUIREACTIVESESSION",
                    "SLSKR_SIGNAL_SYSTEM_BT_EXTENSION_CHANNEL_REQUIRE_ACTIVE_SESSION",
                ],
                1,
                false,
            )?,
        })
    }
}

fn signal_duration_from_seconds(path: &str, seconds: u64) -> Result<Duration, String> {
    if seconds == 0 {
        return Err(format!("{path} must be greater than zero"));
    }
    Ok(Duration::from_secs(seconds))
}

fn parse_signal_duration(path: &str, value: &str) -> Result<Duration, String> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return signal_duration_from_seconds(path, seconds);
    }
    let (days, clock) = if let Some((days, clock)) = value.split_once('.') {
        (
            days.parse::<u64>()
                .map_err(|_| format!("invalid {path}: invalid day count"))?,
            clock,
        )
    } else {
        (0_u64, value)
    };
    let components = clock.split(':').collect::<Vec<_>>();
    if components.len() != 3 {
        return Err(format!("invalid {path}: expected seconds or HH:MM:SS"));
    }
    let hours = components[0]
        .parse::<u64>()
        .map_err(|_| format!("invalid {path}: invalid hours"))?;
    let minutes = components[1]
        .parse::<u64>()
        .map_err(|_| format!("invalid {path}: invalid minutes"))?;
    let seconds = components[2]
        .parse::<u64>()
        .map_err(|_| format!("invalid {path}: invalid seconds"))?;
    if minutes >= 60 || seconds >= 60 {
        return Err(format!(
            "invalid {path}: minutes and seconds must be below 60"
        ));
    }
    let total = days
        .saturating_mul(24 * 60 * 60)
        .saturating_add(hours.saturating_mul(60 * 60))
        .saturating_add(minutes.saturating_mul(60))
        .saturating_add(seconds);
    signal_duration_from_seconds(path, total)
}

impl MeshSyncSecurityFileConfig {
    fn is_configured(&self) -> bool {
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
    pub(super) enable: Option<bool>,
    pub(super) listen_port: Option<u16>,
    pub(super) enable_quic: Option<bool>,
    pub(super) quic_listen_port: Option<u16>,
    pub(super) share_quic_with_dht_port: Option<bool>,
    pub(super) quic_backend_listen_port: Option<u16>,
    pub(super) trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverlayDataFileConfig {
    pub(super) enable: Option<bool>,
    pub(super) listen_port: Option<u16>,
    pub(super) share_with_dht_port: Option<bool>,
    pub(super) backend_listen_port: Option<u16>,
    pub(super) max_concurrent_streams: Option<usize>,
    pub(super) relay_authentication_token: Option<String>,
    pub(super) allowed_relay_destinations: Vec<String>,
    pub(super) max_concurrent_relays: Option<usize>,
    pub(super) max_relay_bytes_per_direction: Option<u64>,
    pub(super) max_relay_duration_seconds: Option<u64>,
    pub(super) trusted_certificate_pins: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) mode: Option<String>,
    pub(super) controller: RelayControllerFileConfig,
    pub(super) agents: BTreeMap<String, RelayAgentFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RelayControllerFileConfig {
    pub(super) address: Option<String>,
    pub(super) ignore_certificate_errors: Option<bool>,
    pub(super) pinned_spki: Option<String>,
    pub(super) api_key: Option<String>,
    pub(super) secret: Option<String>,
    pub(super) downloads: Option<bool>,
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
    pub(super) enabled: Option<bool>,
    pub(super) profile: Option<String>,
    pub(super) network_guard: NetworkGuardFileConfig,
    pub(super) path_guard: PathGuardFileConfig,
    pub(super) content_safety: ContentSafetyFileConfig,
    pub(super) peer_reputation: PeerReputationFileConfig,
    pub(super) violation_tracker: ViolationTrackerFileConfig,
    pub(super) adversarial: AdversarialFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkGuardFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) max_connections_per_ip: Option<usize>,
    pub(super) max_global_connections: Option<usize>,
    pub(super) max_messages_per_minute: Option<u32>,
    pub(super) max_message_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PathGuardFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) max_path_length: Option<usize>,
    pub(super) max_path_depth: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentSafetyFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) verify_magic_bytes: Option<bool>,
    pub(super) quarantine_suspicious: Option<bool>,
    pub(super) quarantine_directory: Option<PathBuf>,
    pub(super) block_executables: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PeerReputationFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) trusted_threshold: Option<u8>,
    pub(super) untrusted_threshold: Option<u8>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ViolationTrackerFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) violations_before_auto_ban: Option<u32>,
    pub(super) base_ban_duration_minutes: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialFileConfig {
    pub(super) privacy: AdversarialPrivacyFileConfig,
    pub(super) anonymity: AdversarialAnonymityFileConfig,
    #[serde(flatten)]
    pub(super) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialPrivacyFileConfig {
    pub(super) padding: AdversarialPaddingFileConfig,
    #[serde(flatten)]
    pub(super) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialPaddingFileConfig {
    pub(super) max_unpadded_bytes: Option<usize>,
    pub(super) max_padded_bytes: Option<usize>,
    #[serde(flatten)]
    pub(super) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialAnonymityFileConfig {
    pub(super) relay_only: AdversarialRelayOnlyFileConfig,
    #[serde(flatten)]
    pub(super) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AdversarialRelayOnlyFileConfig {
    pub(super) relay_peer_data_endpoints: Vec<String>,
    pub(super) relay_authentication_token: Option<String>,
    #[serde(flatten)]
    pub(super) _native_compatibility: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct AdvancedNetworkingFileOverlay {
    pub(super) dht: Option<DhtFileConfig>,
    #[serde(rename = "Mesh")]
    pub(super) mesh_sync: Option<MeshSyncRootFileConfig>,
    pub(super) mesh: Option<MeshFileConfig>,
    #[serde(
        rename = "SignalSystem",
        alias = "signalSystem",
        alias = "signal_system"
    )]
    pub(super) signal_system: Option<SignalSystemFileConfig>,
    pub(super) overlay: Option<OverlayFileConfig>,
    pub(super) overlay_data: Option<OverlayDataFileConfig>,
    pub(super) relay: Option<RelayFileConfig>,
    pub(super) security: Option<SecurityFileConfig>,
    #[serde(rename = "PodCore", alias = "podcore")]
    pub(super) podcore: Option<PodCoreFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(super) struct MediaAdvancedServiceFileOverlay {
    pub(super) feature: Option<FeatureFileConfig>,
    pub(super) player: Option<PlayerFileConfig>,
    pub(super) solid: Option<SolidFileConfig>,
    pub(super) song_id: Option<SongIdFileConfig>,
    #[serde(rename = "virtualSoulfind", alias = "virtual_soulfind")]
    pub(super) virtual_soulfind: Option<VirtualSoulfindFileConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TrustedMeshPeerInput {
    #[serde(alias = "peerId")]
    pub(super) peer_id: String,
    pub(super) username: String,
    #[serde(alias = "overlayEndpoint")]
    pub(super) overlay_endpoint: String,
    #[serde(alias = "certificateSha256")]
    pub(super) certificate_sha256: String,
    #[serde(default, alias = "rangeEndpoint")]
    pub(super) range_endpoint: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodCoreFileConfig {
    #[serde(alias = "Join")]
    pub(super) join: PodJoinFileConfig,
    #[serde(alias = "Security")]
    pub(super) security: PodSecurityFileConfig,
    #[serde(
        rename = "GoldStarClub",
        alias = "goldStarClub",
        alias = "gold_star_club"
    )]
    pub(super) gold_star_club: GoldStarClubFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodJoinFileConfig {
    #[serde(alias = "SignatureMode")]
    pub(super) signature_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PodSecurityFileConfig {
    #[serde(alias = "SignatureMode")]
    pub(super) signature_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GoldStarClubFileConfig {
    #[serde(alias = "AutoJoin", alias = "autoJoin", alias = "auto_join")]
    pub(super) autojoin: Option<bool>,
}

impl AdvancedNetworkingSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
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

impl MediaAdvancedServiceSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: &FileConfig,
        env: &E,
        target: ControllerProfile,
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
        let enabled_by_default = target == ControllerProfile::Native;
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
            allow_insecure_http: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SOLID_ALLOW_INSECURE_HTTP",
                    "SLSKD_SOLID_ALLOW_INSECURE_HTTP",
                ],
                solid_file.allow_insecure_http.unwrap_or(false),
            )?,
            allow_localhost_for_web_id: env_bool_any_layer(
                env,
                &[
                    "SLSKR_SOLID_ALLOW_LOCALHOST_FOR_WEB_ID",
                    "SLSKD_SOLID_ALLOW_LOCALHOST_FOR_WEB_ID",
                    "SLSKD_SOLID_ALLOWLOCALHOSTFORWEBID",
                ],
                solid_file.allow_localhost_for_web_id.unwrap_or(false),
            )?,
            max_fetch_bytes: solid_file.max_fetch_bytes.unwrap_or(1_000_000),
            timeout: Duration::from_secs(solid_file.timeout_seconds.unwrap_or(10)),
            allowed_hosts,
            client_id_url: optional_env_any(
                env,
                &["SLSKR_SOLID_CLIENT_ID_URL", "SLSKD_SOLID_CLIENT_ID_URL"],
            )
            .or_else(|| solid_file.client_id_url.clone())
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
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
    pub(super) http_bind: Option<String>,
    pub(super) state_dir: Option<PathBuf>,
    pub(super) auto_connect: Option<bool>,
    pub(super) reconnect: Option<bool>,
    pub(super) reconnect_seconds: Option<u64>,
    pub(super) ping_seconds: Option<u64>,
    pub(super) log_level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkFileConfig {
    pub(super) server_address: Option<String>,
    pub(super) listen_port: Option<u32>,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(super) credential_store: Option<String>,
    pub(super) credential_file: Option<PathBuf>,
    pub(super) private_message_auto_response: PrivateMessageAutoResponseFileConfig,
    pub(super) obfuscation: SoulseekObfuscationFileConfig,
    pub(super) connection: SoulseekConnectionFileConfig,
    pub(super) distributed_network: SoulseekDistributedFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekDistributedFileConfig {
    pub(super) disabled: Option<bool>,
    pub(super) disable_children: Option<bool>,
    pub(super) child_limit: Option<usize>,
    pub(super) logging: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionFileConfig {
    pub(super) timeout: SoulseekConnectionTimeoutFileConfig,
    pub(super) buffer: SoulseekConnectionBufferFileConfig,
    pub(super) proxy: SoulseekProxyFileConfig,
    pub(super) auto_acknowledge_private_messages: Option<bool>,
    pub(super) auto_acknowledge_privilege_notifications: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionTimeoutFileConfig {
    pub(super) connect: Option<u64>,
    pub(super) inactivity: Option<u64>,
    pub(super) transfer: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekConnectionBufferFileConfig {
    pub(super) read: Option<usize>,
    pub(super) write: Option<usize>,
    pub(super) transfer: Option<usize>,
    pub(super) write_queue: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekProxyFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) address: Option<String>,
    pub(super) port: Option<u16>,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoulseekObfuscationFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) mode: Option<String>,
    pub(super) advertise_regular_port: Option<bool>,
    pub(super) prefer_outbound: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrivateMessageAutoResponseFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) message: Option<String>,
    pub(super) cooldown_minutes: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListenerFileConfig {
    pub(super) regular_bind: Option<String>,
    pub(super) advertised_port: Option<u32>,
    pub(super) obfuscated_bind: Option<String>,
    pub(super) obfuscated_advertised_port: Option<u32>,
    pub(super) overlay_bind: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileFileConfig {
    pub(super) user_info_description: Option<String>,
    pub(super) user_info_picture: Option<String>,
    pub(super) soulseek_diagnostic_level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimeoutFileConfig {
    pub(super) peer_response_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShareFileConfig {
    pub(super) dirs: Vec<String>,
    pub(super) fixture: Option<String>,
    pub(super) follow_symlinks: Option<bool>,
    pub(super) include_hidden: Option<bool>,
    pub(super) scan_max_files: Option<usize>,
    pub(super) cache_tsv_enabled: Option<bool>,
    pub(super) cache: ShareCacheFileConfig,
    pub(super) probe_media_attributes: Option<bool>,
    pub(super) filters: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShareCacheFileConfig {
    pub(super) storage_mode: Option<String>,
    pub(super) workers: Option<usize>,
    pub(super) retention: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferFileConfig {
    pub(super) history_limit: Option<usize>,
    pub(super) max_active: Option<usize>,
    pub(super) allow_inbound: Option<bool>,
    pub(super) allow_outbound: Option<bool>,
    pub(super) auto_retry: TransferAutoRetryFileConfig,
    pub(super) rescue: TransferRescueFileConfig,
    pub(super) completed_path_template: Option<String>,
    pub(super) upload: TransferUploadFileConfig,
    pub(super) download: TransferDownloadFileConfig,
    pub(super) groups: GroupsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AutoReplaceFileConfig {
    pub(super) interval_seconds: Option<u64>,
    pub(super) size_threshold_percent: Option<f64>,
    pub(super) max_retries: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferUploadFileConfig {
    pub(super) slots: Option<u32>,
    pub(super) speed_limit: Option<u32>,
    pub(super) limits: Option<TransferLimitsFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadFileConfig {
    pub(super) slots: Option<u32>,
    pub(super) speed_limit: Option<u32>,
    pub(super) retry: TransferDownloadRetryFileConfig,
    pub(super) destination: TransferDownloadDestinationFileConfig,
    pub(super) completed_layout: Option<String>,
    pub(super) completed_path_template: Option<String>,
    pub(super) auto_replace_stuck: Option<bool>,
    pub(super) auto_replace_threshold: Option<f64>,
    pub(super) auto_replace_interval: Option<u64>,
    pub(super) auto_retry: TransferAutoRetryFileConfig,
    pub(super) cost_based_scheduling: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadRetryFileConfig {
    pub(super) partial: Option<String>,
    pub(super) incomplete: Option<String>,
    pub(super) attempts: Option<u32>,
    pub(super) delay: Option<u64>,
    pub(super) max_delay: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadDestinationFileConfig {
    pub(super) subdirectory: NullableConfig<String>,
    pub(super) exists: Option<String>,
    pub(super) permissions: TransferDownloadPermissionsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadPermissionsFileConfig {
    pub(super) mode: Option<String>,
}

impl SoulseekConnectionSettings {
    pub(super) fn from_layers<E: ConfigEnv>(
        file: SoulseekConnectionFileConfig,
        env: &E,
        target: ControllerProfile,
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
            if target == ControllerProfile::Legacy {
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
        let auto_acknowledge_private_messages = env_bool_any_layer(
            env,
            &[
                "SLSKR_SLSK_AUTO_ACKNOWLEDGE_PRIVATE_MESSAGES",
                "SLSKD_SLSK_AUTO_ACKNOWLEDGE_PRIVATE_MESSAGES",
                "SLSK_AUTO_ACKNOWLEDGE_PRIVATE_MESSAGES",
            ],
            file.auto_acknowledge_private_messages.unwrap_or(false),
        )?;
        let auto_acknowledge_privilege_notifications = env_bool_any_layer(
            env,
            &[
                "SLSKR_SLSK_AUTO_ACKNOWLEDGE_PRIVILEGE_NOTIFICATIONS",
                "SLSKD_SLSK_AUTO_ACKNOWLEDGE_PRIVILEGE_NOTIFICATIONS",
                "SLSK_AUTO_ACKNOWLEDGE_PRIVILEGE_NOTIFICATIONS",
            ],
            file.auto_acknowledge_privilege_notifications
                .unwrap_or(false),
        )?;
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
            auto_acknowledge_private_messages,
            auto_acknowledge_privilege_notifications,
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
                // Frozen slskd/native profile treat an explicitly null limit window the
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
        target: ControllerProfile,
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
        if target == ControllerProfile::Legacy && !allowed_file_types.is_empty() {
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
    pub(super) fn from_layers<E: ConfigEnv>(
        canonical: GroupsFileConfig,
        compatibility: GroupsFileConfig,
        env: &E,
        target: ControllerProfile,
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
        if target == ControllerProfile::Native {
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
        let mut seen_blacklisted = std::collections::HashSet::new();
        let blacklisted_members = blacklisted_members
            .into_iter()
            .map(|member| member.trim().to_owned())
            .filter(|member| !member.is_empty())
            .filter(|member| seen_blacklisted.insert(member.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        Ok(Self {
            default,
            leechers,
            blacklisted: BlacklistedGroupSettings {
                members: blacklisted_members,
            },
            user_defined,
        })
    }
}

pub(super) fn groups_file_config_is_empty(value: &GroupsFileConfig) -> bool {
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
    pub(super) fn from_layers<E: ConfigEnv>(
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
    pub(super) fn from_layers<E: ConfigEnv>(
        mut file: TransferDownloadFileConfig,
        auto_replace: AutoReplaceFileConfig,
        env: &E,
        target: ControllerProfile,
        current_upstream_behavior: bool,
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
            ControllerProfile::Legacy => file.retry.partial,
            ControllerProfile::Native => file.retry.incomplete,
        }
        .unwrap_or_else(|| "resume".to_owned())
        .to_ascii_lowercase();
        if !matches!(incomplete.as_str(), "resume" | "overwrite") {
            return Err(format!(
                "download retry strategy '{incomplete}' must be resume or overwrite"
            ));
        }
        let default_attempts = if target == ControllerProfile::Legacy {
            3
        } else {
            1
        };
        let attempts = file.retry.attempts.unwrap_or(default_attempts);
        let delay_ms = file.retry.delay.unwrap_or(5_000);
        let max_delay_ms = file.retry.max_delay.unwrap_or(60_000);
        match target {
            ControllerProfile::Legacy => {
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
            ControllerProfile::Native => {
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
        let auto_replace_stuck_names: &[&str] = if current_upstream_behavior {
            &[
                "SLSKR_AUTO_REPLACE_STUCK",
                "AUTO_REPLACE_STUCK",
                "SLSKD_AUTO_REPLACE_STUCK",
            ]
        } else {
            &["SLSKD_AUTO_REPLACE_STUCK"]
        };
        let auto_replace_stuck = env_bool_any_layer(
            env,
            auto_replace_stuck_names,
            file.auto_replace_stuck.unwrap_or(false),
        )?;
        let auto_replace_threshold_default = if current_upstream_behavior { 0.0 } else { 5.0 };
        let auto_replace_threshold_names: &[&str] = if current_upstream_behavior {
            &[
                "SLSKR_AUTO_REPLACE_THRESHOLD",
                "AUTO_REPLACE_THRESHOLD",
                "SLSKD_AUTO_REPLACE_THRESHOLD",
            ]
        } else {
            &["SLSKD_AUTO_REPLACE_THRESHOLD"]
        };
        let auto_replace_threshold_percent = bounded_config_value(
            "AUTO_REPLACE_THRESHOLD",
            env_parse_any_layer(
                env,
                auto_replace_threshold_names,
                current_upstream_behavior
                    .then_some(auto_replace.size_threshold_percent)
                    .flatten()
                    .or(file.auto_replace_threshold),
                auto_replace_threshold_default,
            )?,
            if current_upstream_behavior { 0.0 } else { 0.1 },
            50.0,
        )?;
        let auto_replace_interval_default = if current_upstream_behavior { 300 } else { 60 };
        let auto_replace_interval_names: &[&str] = if current_upstream_behavior {
            &[
                "SLSKR_AUTO_REPLACE_INTERVAL",
                "AUTO_REPLACE_INTERVAL",
                "SLSKD_AUTO_REPLACE_INTERVAL",
            ]
        } else {
            &["SLSKD_AUTO_REPLACE_INTERVAL"]
        };
        let auto_replace_interval_seconds = bounded_config_value(
            "AUTO_REPLACE_INTERVAL",
            env_parse_any_layer(
                env,
                auto_replace_interval_names,
                current_upstream_behavior
                    .then_some(auto_replace.interval_seconds)
                    .flatten()
                    .or(file.auto_replace_interval),
                auto_replace_interval_default,
            )?,
            if current_upstream_behavior { 60 } else { 10 },
            3_600,
        )?;
        let auto_replace_max_retries_names: &[&str] = if current_upstream_behavior {
            &[
                "SLSKR_AUTO_REPLACE_MAX_RETRIES",
                "AUTO_REPLACE_MAX_RETRIES",
                "SLSKD_AUTO_REPLACE_MAX_RETRIES",
            ]
        } else {
            &[]
        };
        let max_retries_configured = current_upstream_behavior
            && (auto_replace.max_retries.is_some()
                || optional_env_any(env, auto_replace_max_retries_names).is_some());
        let auto_replace_max_retries = if current_upstream_behavior || max_retries_configured {
            Some(bounded_config_value(
                "AUTO_REPLACE_MAX_RETRIES",
                env_parse_any_layer(
                    env,
                    auto_replace_max_retries_names,
                    auto_replace.max_retries,
                    3_usize,
                )?,
                0,
                100,
            )?)
        } else {
            None
        };

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
            auto_replace_max_retries,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferAutoRetryFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) retry_delay_seconds: Option<u64>,
    pub(super) check_interval_seconds: Option<u64>,
    pub(super) max_attempts: Option<usize>,
    pub(super) max_files_per_cycle: Option<usize>,
    pub(super) max_files_per_peer_per_cycle: Option<usize>,
    pub(super) peer_cooldown_seconds: Option<u64>,
    pub(super) alternate_sources_enabled: Option<bool>,
    pub(super) max_alternate_source_searches_per_cycle: Option<usize>,
    pub(super) alternate_source_size_tolerance_percent: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferRescueFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) max_queue_time_seconds: Option<u64>,
    pub(super) min_throughput_kbps: Option<u64>,
    pub(super) min_duration_seconds: Option<u64>,
    pub(super) stalled_timeout_seconds: Option<u64>,
    pub(super) check_interval_seconds: Option<u64>,
    pub(super) retry_cooldown_seconds: Option<u64>,
    pub(super) max_files_per_cycle: Option<usize>,
    pub(super) alternate_source_size_tolerance_percent: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthFileConfig {
    pub(super) disabled: Option<bool>,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(super) jwt: AuthJwtFileConfig,
    pub(super) api_token: Option<String>,
    pub(super) read_write_token: Option<String>,
    pub(super) read_only_token: Option<String>,
    pub(super) nowplaying_token: Option<String>,
    pub(super) cookie_auth_enabled: Option<bool>,
    pub(super) rate_limit_anonymous: Option<u32>,
    pub(super) rate_limit_authenticated: Option<u32>,
    pub(super) trusted_proxy_cidrs: Vec<String>,
    pub(super) api_keys: BTreeMap<String, ControllerApiKeyFileConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControllerApiKeyFileConfig {
    pub(super) key: String,
    pub(super) role: String,
    pub(super) cidr: String,
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
    pub(super) key: Option<String>,
    pub(super) ttl: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceFileConfig {
    pub(super) enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationsFileConfig {
    pub(super) spotify: SpotifyFileConfig,
    pub(super) lidarr: LidarrFileConfig,
    pub(super) chromaprint: ChromaprintFileConfig,
    #[serde(rename = "acoustId", alias = "acoustid", alias = "AcoustID")]
    pub(super) acoustid: AcoustIdFileConfig,
    #[serde(rename = "musicBrainz", alias = "musicbrainz")]
    pub(super) musicbrainz: MusicBrainzFileConfig,
    pub(super) youtube: SourceFeedApiKeyFileConfig,
    pub(super) lastfm: SourceFeedApiKeyFileConfig,
    pub(super) ntfy: NtfyFileConfig,
    pub(super) pushover: PushoverFileConfig,
    pub(super) pushbullet: PushbulletFileConfig,
    pub(super) ftp: FtpFileConfig,
    pub(super) vpn: VpnFileConfig,
    pub(super) scripts: BTreeMap<String, ScriptIntegrationSettings>,
    pub(super) webhooks: BTreeMap<String, FrozenWebhookSettings>,
    pub(super) bridge: BridgeFileConfig,
    pub(super) external_visualizer: ExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChromaprintFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) algorithm: Option<u32>,
    #[serde(alias = "ffmpegPath")]
    pub(super) ffmpeg_path: Option<String>,
    #[serde(alias = "sampleRate")]
    pub(super) sample_rate: Option<u32>,
    pub(super) channels: Option<u32>,
    #[serde(alias = "durationSeconds")]
    pub(super) duration_seconds: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcoustIdFileConfig {
    pub(super) enabled: Option<bool>,
    #[serde(rename = "clientId", alias = "client_id")]
    pub(super) client_id: Option<String>,
    #[serde(alias = "baseUrl")]
    pub(super) base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MusicBrainzFileConfig {
    #[serde(alias = "baseUrl")]
    pub(super) base_url: Option<String>,
    #[serde(alias = "userAgent")]
    pub(super) user_agent: Option<String>,
    #[serde(alias = "timeoutSeconds")]
    pub(super) timeout_seconds: Option<f64>,
    #[serde(alias = "retryAttempts")]
    pub(super) retry_attempts: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NtfyFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) url: Option<String>,
    pub(super) access_token: Option<String>,
    pub(super) notification_prefix: Option<String>,
    pub(super) notify_on_private_message: Option<bool>,
    pub(super) notify_on_room_mention: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushoverFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) user_key: Option<String>,
    pub(super) token: Option<String>,
    pub(super) notification_prefix: Option<String>,
    pub(super) notify_on_private_message: Option<bool>,
    pub(super) notify_on_room_mention: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushbulletFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) access_token: Option<String>,
    pub(super) notification_prefix: Option<String>,
    pub(super) notify_on_private_message: Option<bool>,
    pub(super) notify_on_room_mention: Option<bool>,
    pub(super) retry_attempts: Option<u32>,
    pub(super) cooldown_time: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FtpFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) address: Option<String>,
    pub(super) port: Option<u16>,
    pub(super) encryption_mode: Option<String>,
    pub(super) ignore_certificate_errors: Option<bool>,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(super) remote_path: Option<String>,
    pub(super) overwrite_existing: Option<bool>,
    pub(super) connection_timeout: Option<u64>,
    pub(super) retry_attempts: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VpnFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) port_forwarding: Option<bool>,
    pub(super) self_hosted_relay: Option<bool>,
    pub(super) polling_interval: Option<u64>,
    pub(super) gluetun: GluetunFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GluetunFileConfig {
    pub(super) url: Option<String>,
    pub(super) timeout: Option<u64>,
    pub(super) auth: Option<String>,
    pub(super) username: Option<String>,
    pub(super) password: Option<String>,
    pub(super) api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourceFeedApiKeyFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpotifyFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) client_id: Option<String>,
    pub(super) client_secret: Option<String>,
    pub(super) redirect_uri: Option<String>,
    pub(super) timeout_seconds: Option<u64>,
    pub(super) max_items_per_import: Option<u64>,
    pub(super) market: Option<String>,
    pub(super) scopes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LidarrFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) url: Option<String>,
    pub(super) api_key: Option<String>,
    pub(super) timeout_seconds: Option<u64>,
    pub(super) sync_wanted_to_wishlist: Option<bool>,
    pub(super) sync_interval_seconds: Option<u64>,
    pub(super) max_items_per_sync: Option<u64>,
    pub(super) auto_download: Option<bool>,
    pub(super) wishlist_filter: Option<String>,
    pub(super) wishlist_max_results: Option<u64>,
    pub(super) auto_import_completed: Option<bool>,
    pub(super) import_delay_seconds: Option<u64>,
    pub(super) import_retry_max_attempts: Option<u32>,
    pub(super) import_retry_delay_seconds: Option<u64>,
    pub(super) import_path_from: Option<String>,
    pub(super) import_path_to: Option<String>,
    pub(super) import_mode: Option<String>,
    pub(super) import_replace_existing_files: Option<bool>,
    pub(super) skip_already_owned_albums: Option<bool>,
    pub(super) delete_rejected_downloads: Option<bool>,
    pub(super) blacklist_rejected_downloads: Option<bool>,
    pub(super) edition_match_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BridgeFileConfig {
    pub(super) enabled: Option<bool>,
    pub(super) host: Option<String>,
    pub(super) port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExternalVisualizerFileConfig {
    pub(super) command: Option<String>,
    pub(super) launch_enabled: Option<bool>,
}
