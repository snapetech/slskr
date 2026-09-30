use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationsFileConfig {
    pub(in crate::config) spotify: SpotifyFileConfig,
    pub(in crate::config) lidarr: LidarrFileConfig,
    pub(in crate::config) chromaprint: ChromaprintFileConfig,
    #[serde(rename = "acoustId", alias = "acoustid", alias = "AcoustID")]
    pub(in crate::config) acoustid: AcoustIdFileConfig,
    #[serde(rename = "musicBrainz", alias = "musicbrainz")]
    pub(in crate::config) musicbrainz: MusicBrainzFileConfig,
    pub(in crate::config) youtube: SourceFeedApiKeyFileConfig,
    pub(in crate::config) lastfm: SourceFeedApiKeyFileConfig,
    pub(in crate::config) ntfy: NtfyFileConfig,
    pub(in crate::config) pushover: PushoverFileConfig,
    pub(in crate::config) pushbullet: PushbulletFileConfig,
    pub(in crate::config) ftp: FtpFileConfig,
    pub(in crate::config) vpn: VpnFileConfig,
    pub(in crate::config) scripts: BTreeMap<String, ScriptIntegrationSettings>,
    pub(in crate::config) webhooks: BTreeMap<String, FrozenWebhookSettings>,
    pub(in crate::config) bridge: BridgeFileConfig,
    pub(in crate::config) external_visualizer: ExternalVisualizerFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChromaprintFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) algorithm: Option<u32>,
    #[serde(alias = "ffmpegPath")]
    pub(in crate::config) ffmpeg_path: Option<String>,
    #[serde(alias = "sampleRate")]
    pub(in crate::config) sample_rate: Option<u32>,
    pub(in crate::config) channels: Option<u32>,
    #[serde(alias = "durationSeconds")]
    pub(in crate::config) duration_seconds: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcoustIdFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    #[serde(rename = "clientId", alias = "client_id")]
    pub(in crate::config) client_id: Option<String>,
    #[serde(alias = "baseUrl")]
    pub(in crate::config) base_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MusicBrainzFileConfig {
    #[serde(alias = "baseUrl")]
    pub(in crate::config) base_url: Option<String>,
    #[serde(alias = "userAgent")]
    pub(in crate::config) user_agent: Option<String>,
    #[serde(alias = "timeoutSeconds")]
    pub(in crate::config) timeout_seconds: Option<f64>,
    #[serde(alias = "retryAttempts")]
    pub(in crate::config) retry_attempts: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NtfyFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) url: Option<String>,
    pub(in crate::config) access_token: Option<String>,
    pub(in crate::config) notification_prefix: Option<String>,
    pub(in crate::config) notify_on_private_message: Option<bool>,
    pub(in crate::config) notify_on_room_mention: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushoverFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) user_key: Option<String>,
    pub(in crate::config) token: Option<String>,
    pub(in crate::config) notification_prefix: Option<String>,
    pub(in crate::config) notify_on_private_message: Option<bool>,
    pub(in crate::config) notify_on_room_mention: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushbulletFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) access_token: Option<String>,
    pub(in crate::config) notification_prefix: Option<String>,
    pub(in crate::config) notify_on_private_message: Option<bool>,
    pub(in crate::config) notify_on_room_mention: Option<bool>,
    pub(in crate::config) retry_attempts: Option<u32>,
    pub(in crate::config) cooldown_time: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FtpFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) address: Option<String>,
    pub(in crate::config) port: Option<u16>,
    pub(in crate::config) encryption_mode: Option<String>,
    pub(in crate::config) ignore_certificate_errors: Option<bool>,
    pub(in crate::config) username: Option<String>,
    pub(in crate::config) password: Option<String>,
    pub(in crate::config) remote_path: Option<String>,
    pub(in crate::config) overwrite_existing: Option<bool>,
    pub(in crate::config) connection_timeout: Option<u64>,
    pub(in crate::config) retry_attempts: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VpnFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) port_forwarding: Option<bool>,
    pub(in crate::config) self_hosted_relay: Option<bool>,
    pub(in crate::config) polling_interval: Option<u64>,
    pub(in crate::config) gluetun: GluetunFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GluetunFileConfig {
    pub(in crate::config) url: Option<String>,
    pub(in crate::config) timeout: Option<u64>,
    pub(in crate::config) auth: Option<String>,
    pub(in crate::config) username: Option<String>,
    pub(in crate::config) password: Option<String>,
    pub(in crate::config) api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SourceFeedApiKeyFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) api_key: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SpotifyFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) client_id: Option<String>,
    pub(in crate::config) client_secret: Option<String>,
    pub(in crate::config) redirect_uri: Option<String>,
    pub(in crate::config) timeout_seconds: Option<u64>,
    pub(in crate::config) max_items_per_import: Option<u64>,
    pub(in crate::config) market: Option<String>,
    pub(in crate::config) scopes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LidarrFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) url: Option<String>,
    pub(in crate::config) api_key: Option<String>,
    pub(in crate::config) timeout_seconds: Option<u64>,
    pub(in crate::config) sync_wanted_to_wishlist: Option<bool>,
    pub(in crate::config) sync_interval_seconds: Option<u64>,
    pub(in crate::config) max_items_per_sync: Option<u64>,
    pub(in crate::config) auto_download: Option<bool>,
    pub(in crate::config) wishlist_filter: Option<String>,
    pub(in crate::config) wishlist_max_results: Option<u64>,
    pub(in crate::config) auto_import_completed: Option<bool>,
    pub(in crate::config) import_delay_seconds: Option<u64>,
    pub(in crate::config) import_retry_max_attempts: Option<u32>,
    pub(in crate::config) import_retry_delay_seconds: Option<u64>,
    pub(in crate::config) import_path_from: Option<String>,
    pub(in crate::config) import_path_to: Option<String>,
    pub(in crate::config) import_mode: Option<String>,
    pub(in crate::config) import_replace_existing_files: Option<bool>,
    pub(in crate::config) skip_already_owned_albums: Option<bool>,
    pub(in crate::config) delete_rejected_downloads: Option<bool>,
    pub(in crate::config) blacklist_rejected_downloads: Option<bool>,
    pub(in crate::config) edition_match_mode: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BridgeFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) host: Option<String>,
    pub(in crate::config) port: Option<u16>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExternalVisualizerFileConfig {
    pub(in crate::config) command: Option<String>,
    pub(in crate::config) launch_enabled: Option<bool>,
}
