use super::*;

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
