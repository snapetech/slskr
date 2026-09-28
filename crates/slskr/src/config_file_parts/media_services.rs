use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct FeatureFileConfig {
    pub(in crate::config) swagger: Option<bool>,
    #[serde(alias = "CollectionsSharing")]
    pub(in crate::config) collections_sharing: Option<bool>,
    #[serde(alias = "Streaming")]
    pub(in crate::config) streaming: Option<bool>,
    #[serde(alias = "StreamingRelayFallback")]
    pub(in crate::config) streaming_relay_fallback: Option<bool>,
    #[serde(alias = "MeshParallelSearch")]
    pub(in crate::config) mesh_parallel_search: Option<bool>,
    #[serde(alias = "MeshPublishAvailability")]
    pub(in crate::config) mesh_publish_availability: Option<bool>,
    #[serde(alias = "IdentityFriends")]
    pub(in crate::config) identity_friends: Option<bool>,
    #[serde(alias = "Solid")]
    pub(in crate::config) solid: Option<bool>,
    #[serde(alias = "ScenePodBridge")]
    pub(in crate::config) scene_pod_bridge: Option<bool>,
    #[serde(alias = "SongId")]
    pub(in crate::config) song_id: Option<bool>,
    #[serde(alias = "Mesh")]
    pub(in crate::config) mesh: Option<bool>,
    #[serde(alias = "Dht")]
    pub(in crate::config) dht: Option<bool>,
    #[serde(alias = "Pods")]
    pub(in crate::config) pods: Option<bool>,
    #[serde(alias = "SocialFederation")]
    pub(in crate::config) social_federation: Option<bool>,
    #[serde(alias = "VirtualSoulfind")]
    pub(in crate::config) virtual_soulfind: Option<bool>,
    #[serde(alias = "MultiSourceDownloads")]
    pub(in crate::config) multi_source_downloads: Option<bool>,
    #[serde(alias = "ScenePodBridgeOptions")]
    pub(in crate::config) scene_pod_bridge_options: ScenePodBridgeFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ScenePodBridgeFileConfig {
    #[serde(alias = "ProxyTransfers")]
    pub(in crate::config) proxy_transfers: Option<bool>,
    #[serde(alias = "ExportPodAvailability")]
    pub(in crate::config) export_pod_availability: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerFileConfig {
    pub(in crate::config) external_visualizer: PlayerExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerExternalVisualizerFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) path: Option<String>,
    pub(in crate::config) arguments: Option<Vec<String>>,
    pub(in crate::config) working_directory: Option<PathBuf>,
    pub(in crate::config) name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SolidFileConfig {
    #[serde(alias = "allowInsecureHttp")]
    pub(in crate::config) allow_insecure_http: Option<bool>,
    #[serde(alias = "allowLocalhostForWebId")]
    pub(in crate::config) allow_localhost_for_web_id: Option<bool>,
    #[serde(alias = "maxFetchBytes")]
    pub(in crate::config) max_fetch_bytes: Option<usize>,
    #[serde(alias = "timeoutSeconds")]
    pub(in crate::config) timeout_seconds: Option<u64>,
    #[serde(alias = "allowedHosts")]
    pub(in crate::config) allowed_hosts: Option<Vec<String>>,
    #[serde(alias = "clientIdUrl")]
    pub(in crate::config) client_id_url: Option<String>,
    #[serde(alias = "redirectPath")]
    pub(in crate::config) redirect_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SongIdFileConfig {
    pub(in crate::config) max_concurrent_runs: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindFileConfig {
    pub(in crate::config) bridge: VirtualSoulfindBridgeFileConfig,
    pub(in crate::config) disaster_mode: VirtualSoulfindDisasterModeFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindBridgeFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) port: Option<u16>,
    pub(in crate::config) bind_address: Option<String>,
    pub(in crate::config) max_clients: Option<usize>,
    pub(in crate::config) require_auth: Option<bool>,
    pub(in crate::config) password: Option<String>,
    pub(in crate::config) max_requests_per_minute: Option<u32>,
    pub(in crate::config) max_transfers_per_session: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct VirtualSoulfindDisasterModeFileConfig {
    pub(in crate::config) auto: Option<bool>,
    pub(in crate::config) force: Option<bool>,
    pub(in crate::config) unavailable_threshold_minutes: Option<u64>,
    pub(in crate::config) enable_graceful_degradation: Option<bool>,
    pub(in crate::config) recovery_check_interval_minutes: Option<u64>,
    pub(in crate::config) recovery_healthy_checks_required: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VirtualSoulfindV2FileConfig {
    pub(in crate::config) enabled: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(in crate::config) struct MediaAdvancedServiceFileOverlay {
    pub(in crate::config) feature: Option<FeatureFileConfig>,
    pub(in crate::config) player: Option<PlayerFileConfig>,
    pub(in crate::config) solid: Option<SolidFileConfig>,
    pub(in crate::config) song_id: Option<SongIdFileConfig>,
    #[serde(rename = "virtualSoulfind", alias = "virtual_soulfind")]
    pub(in crate::config) virtual_soulfind: Option<VirtualSoulfindFileConfig>,
}

impl MediaAdvancedServiceSettings {
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
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
