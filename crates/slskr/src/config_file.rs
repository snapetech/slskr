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

#[path = "config_file_parts/federation_membership.rs"]
mod federation_membership;
pub use federation_membership::{PodCoreFileConfig, RealmSettings};
// Preserve established file-model paths.
#[allow(unused_imports)]
pub use federation_membership::GoldStarClubFileConfig;
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use federation_membership::{PodJoinFileConfig, PodSecurityFileConfig};
#[path = "config_file_parts/filters_shares.rs"]
mod filters_shares;
pub use filters_shares::{
    AutoReplaceFileConfig, DownloadFiltersFileConfig, FiltersFileConfig, ShareFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use filters_shares::{
    SearchFiltersFileConfig, SearchRetentionFileConfig, ShareCacheFileConfig,
};
#[path = "config_file_parts/foundation.rs"]
mod foundation;
pub use foundation::{
    AppFileConfig, CompatibilityFileConfig, ControllerFlagsFileConfig, LoggerFileConfig,
    ManagedBlacklistFileConfig, PermissionsFileConfig, PersistenceFileConfig, RetentionFileConfig,
    TelemetryFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use foundation::{
    FilePermissionsFileConfig, FileRetentionFileConfig, TelemetryTracingFileConfig,
};
#[path = "config_file_parts/integrations.rs"]
mod integrations;
pub use integrations::{
    AcoustIdFileConfig, BridgeFileConfig, ChromaprintFileConfig, ExternalVisualizerFileConfig,
    FtpFileConfig, IntegrationsFileConfig, LidarrFileConfig, MusicBrainzFileConfig, NtfyFileConfig,
    PushbulletFileConfig, PushoverFileConfig, SourceFeedApiKeyFileConfig, SpotifyFileConfig,
    VpnFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use integrations::GluetunFileConfig;
#[path = "config_file_parts/media_services.rs"]
mod media_services;
pub use media_services::{
    FeatureFileConfig, PlayerFileConfig, SolidFileConfig, SongIdFileConfig,
    VirtualSoulfindFileConfig, VirtualSoulfindV2FileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use media_services::{
    PlayerExternalVisualizerFileConfig, ScenePodBridgeFileConfig, VirtualSoulfindBridgeFileConfig,
    VirtualSoulfindDisasterModeFileConfig,
};
#[path = "config_file_parts/network_security.rs"]
mod network_security;
pub use network_security::{
    DhtFileConfig, MeshFileConfig, MeshSyncRootFileConfig, OverlayDataFileConfig,
    OverlayFileConfig, RelayFileConfig, SecurityFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use network_security::{
    AdversarialAnonymityFileConfig, AdversarialFileConfig, AdversarialPaddingFileConfig,
    AdversarialPrivacyFileConfig, AdversarialRelayOnlyFileConfig, ContentSafetyFileConfig,
    MeshDhtFileConfig, MeshPortsFileConfig, MeshSecurityFileConfig, MeshSyncSecurityFileConfig,
    NetworkGuardFileConfig, PathGuardFileConfig, PeerReputationFileConfig, RelayAgentFileConfig,
    RelayControllerFileConfig, ViolationTrackerFileConfig,
};
#[path = "config_file_parts/peer_transport.rs"]
mod peer_transport;
pub use peer_transport::{
    ListenerFileConfig, NetworkFileConfig, PrivateMessageAutoResponseFileConfig, ProfileFileConfig,
    TimeoutFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use peer_transport::{
    SoulseekConnectionBufferFileConfig, SoulseekConnectionFileConfig,
    SoulseekConnectionTimeoutFileConfig, SoulseekDistributedFileConfig,
    SoulseekObfuscationFileConfig, SoulseekProxyFileConfig,
};
#[path = "config_file_parts/signal_policy.rs"]
mod signal_policy;
pub use signal_policy::SignalSystemFileConfig;
// Preserve established file-model paths.
#[allow(unused_imports)]
pub use signal_policy::{SignalChannelFileConfig, SignalDurationFileValue};

#[path = "config_file_parts/transfer_policy.rs"]
mod transfer_policy;
pub use transfer_policy::{
    GroupsFileConfig, TransferAutoRetryFileConfig, TransferFileConfig, TransferRescueFileConfig,
    TransferRetentionFileConfig, TransferTypeRetentionFileConfig, UserBlacklistFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use transfer_policy::{
    LeecherThresholdFileConfig, LeecherTransferGroupFileConfig,
    TransferDownloadDestinationFileConfig, TransferDownloadFileConfig,
    TransferDownloadPermissionsFileConfig, TransferDownloadRetryFileConfig,
    TransferGroupFileConfig, TransferGroupUploadFileConfig, TransferLimitFileConfig,
    TransferLimitsFileConfig, TransferUploadFileConfig, UserDefinedTransferGroupFileConfig,
};
#[path = "config_file_parts/web_security.rs"]
mod web_security;
pub use web_security::{
    AuthFileConfig, ControllerApiKeyFileConfig, DiagnosticsFileConfig, HttpsFileConfig,
    MetricsFileConfig, WebCorsFileConfig, WebFileConfig, WebRateLimitingFileConfig,
};
// Retain established paths for nested input types.
#[allow(unused_imports)]
pub use web_security::{
    AuthJwtFileConfig, HttpsCertificateFileConfig, MetricsAuthenticationFileConfig,
};

pub(super) use federation_membership::{
    FederationPublishingFileConfig, MultiRealmFileConfig, RealmFileConfig,
    SocialFederationFileConfig,
};
// Preserve established file-model paths.
#[allow(unused_imports)]
pub(super) use federation_membership::{MultiRealmBridgeFileConfig, RealmPoliciesFileConfig};

pub(super) use transfer_policy::groups_file_config_is_empty;
// Preserve established file-model paths.
#[allow(unused_imports)]
pub(super) use transfer_policy::NullableConfig;

pub(super) use network_security::MeshGatewayFileConfig;
// Preserve established file-model paths.
#[allow(unused_imports)]
pub(super) use network_security::AdvancedNetworkingFileOverlay;

// Preserve established file-model paths.
#[allow(unused_imports)]
pub(super) use media_services::MediaAdvancedServiceFileOverlay;

pub(super) use peer_transport::TrustedMeshPeerInput;
