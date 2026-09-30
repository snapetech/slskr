use super::*;

#[derive(Clone, Debug, Default)]
pub struct IntegrationSettings {
    pub spotify: Box<SpotifyIntegrationSettings>,
    pub lidarr: Box<LidarrIntegrationSettings>,
    pub chromaprint: ChromaprintIntegrationSettings,
    pub acoustid: AcoustIdIntegrationSettings,
    pub musicbrainz: MusicBrainzIntegrationSettings,
    pub youtube: SourceFeedApiKeySettings,
    pub lastfm: SourceFeedApiKeySettings,
    pub ntfy: NtfyIntegrationSettings,
    pub pushover: PushoverIntegrationSettings,
    pub pushbullet: PushbulletIntegrationSettings,
    pub ftp: FtpIntegrationSettings,
    pub vpn: VpnIntegrationSettings,
    pub scripts: BTreeMap<String, ScriptIntegrationSettings>,
    pub frozen_webhooks: BTreeMap<String, FrozenWebhookSettings>,
    pub bridge: BridgeIntegrationSettings,
    pub external_visualizer: ExternalVisualizerSettings,
}

impl IntegrationSettings {
    pub fn from_layers<E: ConfigEnv>(
        file_config: IntegrationsFileConfig,
        env: &E,
        current_upstream_behavior: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            spotify: Box::new(SpotifyIntegrationSettings::from_layers(
                file_config.spotify,
                env,
            )?),
            lidarr: Box::new(LidarrIntegrationSettings::from_layers(
                file_config.lidarr,
                env,
                current_upstream_behavior,
            )?),
            chromaprint: ChromaprintIntegrationSettings::from_layers(
                file_config.chromaprint,
                env,
            )?,
            acoustid: AcoustIdIntegrationSettings::from_layers(file_config.acoustid, env)?,
            musicbrainz: MusicBrainzIntegrationSettings::from_layers(file_config.musicbrainz, env)?,
            youtube: SourceFeedApiKeySettings::from_layers(
                file_config.youtube,
                env,
                "SLSKD_YOUTUBE",
                "SLSKD_YOUTUBE_API_KEY",
                "YouTube source feed imports are enabled but integrations.youtube.api_key is empty.",
            )?,
            lastfm: SourceFeedApiKeySettings::from_layers(
                file_config.lastfm,
                env,
                "SLSKD_LASTFM",
                "SLSKD_LASTFM_API_KEY",
                "Last.fm source feed imports are enabled but integrations.lastfm.api_key is empty.",
            )?,
            ntfy: NtfyIntegrationSettings::from_layers(file_config.ntfy, env)?,
            pushover: PushoverIntegrationSettings::from_layers(file_config.pushover, env)?,
            pushbullet: PushbulletIntegrationSettings::from_layers(file_config.pushbullet, env)?,
            ftp: FtpIntegrationSettings::from_layers(file_config.ftp, env)?,
            vpn: VpnIntegrationSettings::from_layers(file_config.vpn, env)?,
            scripts: ScriptIntegrationSettings::from_layers(file_config.scripts, env)?,
            frozen_webhooks: FrozenWebhookSettings::from_layers(file_config.webhooks, env)?,
            bridge: BridgeIntegrationSettings::from_layers(file_config.bridge, env)?,
            external_visualizer: ExternalVisualizerSettings::from_layers(
                file_config.external_visualizer,
                env,
            )?,
        })
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"spotify\":{},\"lidarr\":{},\"chromaprint\":{},\"acoustid\":{},\"musicbrainz\":{},\"youtube\":{},\"lastfm\":{},\"ntfy\":{},\"pushover\":{},\"pushbullet\":{},\"ftp\":{},\"vpn\":{},\"script_count\":{},\"webhook_count\":{},\"bridge\":{},\"external_visualizer\":{}}}",
            self.spotify.sanitized_json(),
            self.lidarr.sanitized_json(),
            self.chromaprint.sanitized_json(),
            self.acoustid.sanitized_json(),
            self.musicbrainz.sanitized_json(),
            self.youtube.sanitized_json(),
            self.lastfm.sanitized_json(),
            self.ntfy.sanitized_json(),
            self.pushover.sanitized_json(),
            self.pushbullet.sanitized_json(),
            self.ftp.sanitized_json(),
            self.vpn.sanitized_json(),
            self.scripts.len(),
            self.frozen_webhooks.len(),
            self.bridge.sanitized_json(),
            self.external_visualizer.sanitized_json()
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromaprintIntegrationSettings {
    pub enabled: bool,
    pub algorithm: u32,
    pub ffmpeg_path: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub duration_seconds: u32,
}

impl ChromaprintIntegrationSettings {
    fn from_layers<E: ConfigEnv>(
        file_config: ChromaprintFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_any_layer(
                env,
                &["SLSKD_CHROMAPRINT_ENABLED", "SLSKR_CHROMAPRINT_ENABLED"],
                file_config.enabled.unwrap_or(false),
            )?,
            algorithm: env_parse_any_layer(
                env,
                &["SLSKD_CHROMAPRINT_ALGORITHM", "SLSKR_CHROMAPRINT_ALGORITHM"],
                file_config.algorithm,
                1,
            )?,
            ffmpeg_path: optional_env_any(
                env,
                &[
                    "SLSKD_CHROMAPRINT_FFMPEG_PATH",
                    "SLSKR_CHROMAPRINT_FFMPEG_PATH",
                ],
            )
            .or(file_config.ffmpeg_path)
            .unwrap_or_else(|| "ffmpeg".to_owned())
            .trim()
            .to_owned(),
            sample_rate: env_parse_any_layer(
                env,
                &[
                    "SLSKD_CHROMAPRINT_SAMPLE_RATE",
                    "SLSKR_CHROMAPRINT_SAMPLE_RATE",
                ],
                file_config.sample_rate,
                44_100,
            )?,
            channels: env_parse_any_layer(
                env,
                &["SLSKD_CHROMAPRINT_CHANNELS", "SLSKR_CHROMAPRINT_CHANNELS"],
                file_config.channels,
                2,
            )?,
            duration_seconds: env_parse_any_layer(
                env,
                &[
                    "SLSKD_CHROMAPRINT_DURATION_SECONDS",
                    "SLSKR_CHROMAPRINT_DURATION_SECONDS",
                ],
                file_config.duration_seconds,
                120,
            )?,
        };
        if settings.ffmpeg_path.is_empty() {
            return Err("Chromaprint ffmpeg path must not be empty.".to_owned());
        }
        if settings.enabled && settings.algorithm == 0 {
            return Err("Chromaprint algorithm must be positive.".to_owned());
        }
        if settings.enabled && settings.sample_rate == 0 {
            return Err("Chromaprint sample rate must be positive.".to_owned());
        }
        if settings.enabled && settings.channels == 0 {
            return Err("Chromaprint channels must be positive.".to_owned());
        }
        if settings.enabled && settings.duration_seconds == 0 {
            return Err("Chromaprint duration must be positive seconds.".to_owned());
        }
        Ok(settings)
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"algorithm\":{},\"ffmpeg_path_configured\":{},\"sample_rate\":{},\"channels\":{},\"duration_seconds\":{}}}",
            self.enabled,
            self.algorithm,
            !self.ffmpeg_path.is_empty(),
            self.sample_rate,
            self.channels,
            self.duration_seconds,
        )
    }
}

impl Default for ChromaprintIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: 1,
            ffmpeg_path: "ffmpeg".to_owned(),
            sample_rate: 44_100,
            channels: 2,
            duration_seconds: 120,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcoustIdIntegrationSettings {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub base_url: String,
}

impl AcoustIdIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: AcoustIdFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_any_layer(
                env,
                &["SLSKD_ACOUSTID_ENABLED", "SLSKR_ACOUSTID_ENABLED"],
                file_config.enabled.unwrap_or(false),
            )?,
            client_id: optional_env_any(
                env,
                &["SLSKD_ACOUSTID_CLIENT_ID", "SLSKR_ACOUSTID_CLIENT_ID"],
            )
            .or(file_config.client_id)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
            base_url: optional_env_any(
                env,
                &["SLSKD_ACOUSTID_BASE_URL", "SLSKR_ACOUSTID_BASE_URL"],
            )
            .or(file_config.base_url)
            .unwrap_or_else(|| "https://api.acoustid.org/v2".to_owned())
            .trim()
            .trim_end_matches('/')
            .to_owned(),
        };
        if settings.enabled && settings.client_id.is_none() {
            return Err("AcoustID client id must be supplied when enabled.".to_owned());
        }
        let parsed = reqwest::Url::parse(&settings.base_url)
            .map_err(|_| "AcoustID base URL must be an absolute URL.".to_owned())?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("AcoustID base URL must be an absolute URL.".to_owned());
        }
        Ok(settings)
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"client_id_configured\":{},\"base_url_configured\":{}}}",
            self.enabled,
            self.client_id.is_some(),
            !self.base_url.is_empty(),
        )
    }
}

impl Default for AcoustIdIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            client_id: None,
            base_url: "https://api.acoustid.org/v2".to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicBrainzIntegrationSettings {
    pub base_url: String,
    pub user_agent: String,
    pub timeout_seconds: f64,
    pub retry_attempts: u32,
}

impl MusicBrainzIntegrationSettings {
    fn from_layers<E: ConfigEnv>(
        file_config: MusicBrainzFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        let settings = Self {
            base_url: env
                .var("SLSKD_MUSICBRAINZ_BASE_URL")
                .or(file_config.base_url)
                .unwrap_or_else(|| "https://musicbrainz.org/ws/2".to_owned())
                .trim()
                .to_owned(),
            user_agent: env
                .var("SLSKD_MUSICBRAINZ_USER_AGENT")
                .or(file_config.user_agent)
                .unwrap_or_else(|| "slskR/0.0.0 (https://github.com/snapetech/slskr)".to_owned())
                .trim()
                .to_owned(),
            timeout_seconds: env_parse_layer(
                env,
                "SLSKD_MUSICBRAINZ_TIMEOUT_SECONDS",
                file_config.timeout_seconds,
                20.0,
            )?,
            retry_attempts: env_parse_layer(
                env,
                "SLSKD_MUSICBRAINZ_RETRY_ATTEMPTS",
                file_config.retry_attempts,
                2,
            )?,
        };
        let parsed = reqwest::Url::parse(&settings.base_url)
            .map_err(|_| "The BaseUrl field must be an absolute URL.".to_owned())?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
            return Err("The BaseUrl field must be an absolute URL.".to_owned());
        }
        if settings.user_agent.is_empty() {
            return Err("The UserAgent field must not be empty.".to_owned());
        }
        if !settings.timeout_seconds.is_finite() || settings.timeout_seconds <= 0.0 {
            return Err("The TimeoutSeconds field must be a positive number.".to_owned());
        }
        if settings.retry_attempts == 0 {
            return Err("The RetryAttempts field must be greater than zero.".to_owned());
        }
        Ok(settings)
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"base_url\":{},\"user_agent\":{},\"timeout_seconds\":{},\"retry_attempts\":{}}}",
            json_option(Some(&self.base_url)),
            json_option(Some(&self.user_agent)),
            self.timeout_seconds,
            self.retry_attempts,
        )
    }
}

impl Default for MusicBrainzIntegrationSettings {
    fn default() -> Self {
        Self {
            base_url: "https://musicbrainz.org/ws/2".to_owned(),
            user_agent: "slskR/0.0.0 (https://github.com/snapetech/slskr)".to_owned(),
            timeout_seconds: 20.0,
            retry_attempts: 2,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookSettings {
    pub on: Vec<String>,
    pub call: FrozenWebhookCallSettings,
    pub timeout: i32,
    pub retry: FrozenWebhookRetrySettings,
}
impl Default for FrozenWebhookSettings {
    fn default() -> Self {
        Self {
            on: Vec::new(),
            call: FrozenWebhookCallSettings::default(),
            timeout: 5_000,
            retry: FrozenWebhookRetrySettings::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookCallSettings {
    pub url: String,
    pub headers: Vec<FrozenWebhookHeaderSettings>,
    pub ignore_certificate_errors: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookHeaderSettings {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FrozenWebhookRetrySettings {
    pub attempts: i32,
}
impl Default for FrozenWebhookRetrySettings {
    fn default() -> Self {
        Self { attempts: 1 }
    }
}

impl FrozenWebhookSettings {
    fn from_layers<E: ConfigEnv>(
        file: BTreeMap<String, Self>,
        env: &E,
    ) -> Result<BTreeMap<String, Self>, String> {
        let hooks = match env.var("SLSKR_FROZEN_WEBHOOKS_JSON") {
            Some(value) => serde_json::from_str::<BTreeMap<String, Self>>(&value)
                .map_err(|_| "invalid integrations.webhooks configuration".to_owned())?,
            None => file,
        };
        for (name, hook) in &hooks {
            if hook.call.url.trim().is_empty() {
                return Err(format!("Webhook {name} Url must not be empty"));
            }
            if !hook.call.url.starts_with("http://") && !hook.call.url.starts_with("https://") {
                return Err("The Url field must contain a fully qualified URL, including protocol (e.g. http:// or https://)".to_owned());
            }
            if hook.timeout < 500 {
                return Err("The field Timeout must be between 500 and 2147483647.".to_owned());
            }
            if hook.retry.attempts < 1 {
                return Err("The field Attempts must be between 1 and 2147483647.".to_owned());
            }
            if hook
                .call
                .headers
                .iter()
                .any(|header| header.name.trim().is_empty())
            {
                return Err("Webhook header Name must not be empty".to_owned());
            }
        }
        Ok(hooks)
    }
}

#[derive(Clone, Debug)]
pub struct NtfyIntegrationSettings {
    pub enabled: bool,
    pub url: String,
    pub access_token: String,
    pub notification_prefix: String,
    pub notify_on_private_message: bool,
    pub notify_on_room_mention: bool,
}

impl NtfyIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: NtfyFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_NTFY", file.enabled.unwrap_or(false))?,
            url: env.var("SLSKD_NTFY_URL").or(file.url).unwrap_or_default(),
            access_token: env
                .var("SLSKD_NTFY_TOKEN")
                .or(file.access_token)
                .unwrap_or_default(),
            notification_prefix: env
                .var("SLSKD_NTFY_NOTIFICATION_PREFIX")
                .or(file.notification_prefix)
                .unwrap_or_else(|| "slskR".to_owned()),
            notify_on_private_message: env_bool_layer(
                env,
                "SLSKD_NTFY_NOTIFY_ON_PRIVATE_MESSAGE",
                file.notify_on_private_message.unwrap_or(true),
            )?,
            notify_on_room_mention: env_bool_layer(
                env,
                "SLSKD_NTFY_NOTIFY_ON_ROOM_MENTION",
                file.notify_on_room_mention.unwrap_or(true),
            )?,
        };
        if settings.enabled && settings.url.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no Url has been specified for Ntfy.".to_owned(),
            );
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!("{{\"enabled\":{},\"url\":{},\"access_token_configured\":{},\"notification_prefix\":{},\"notify_on_private_message\":{},\"notify_on_room_mention\":{}}}", self.enabled, json_option(Some(&self.url)), !self.access_token.is_empty(), json_option(Some(&self.notification_prefix)), self.notify_on_private_message, self.notify_on_room_mention)
    }
}

impl Default for NtfyIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            url: String::new(),
            access_token: String::new(),
            notification_prefix: "slskR".to_owned(),
            notify_on_private_message: true,
            notify_on_room_mention: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushoverIntegrationSettings {
    pub enabled: bool,
    pub user_key: String,
    pub token: String,
    pub notification_prefix: String,
    pub notify_on_private_message: bool,
    pub notify_on_room_mention: bool,
}

impl PushoverIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: PushoverFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_PUSHOVER", file.enabled.unwrap_or(false))?,
            user_key: env
                .var("SLSKD_PUSHOVER_USER_KEY")
                .or(file.user_key)
                .unwrap_or_default(),
            token: env
                .var("SLSKD_PUSHOVER_TOKEN")
                .or(file.token)
                .unwrap_or_default(),
            notification_prefix: env
                .var("SLSKD_PUSHOVER_NOTIFICATION_PREFIX")
                .or(file.notification_prefix)
                .unwrap_or_else(|| "slskR".to_owned()),
            notify_on_private_message: env_bool_layer(
                env,
                "SLSKD_PUSHOVER_NOTIFY_ON_PRIVATE_MESSAGE",
                file.notify_on_private_message.unwrap_or(true),
            )?,
            notify_on_room_mention: env_bool_layer(
                env,
                "SLSKD_PUSHOVER_NOTIFY_ON_ROOM_MENTION",
                file.notify_on_room_mention.unwrap_or(true),
            )?,
        };
        if settings.enabled && settings.user_key.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no UserKey has been specified for Pushover."
                    .to_owned(),
            );
        }
        if settings.enabled && settings.token.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no Token has been specified for Pushover."
                    .to_owned(),
            );
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!("{{\"enabled\":{},\"user_key_configured\":{},\"token_configured\":{},\"notification_prefix\":{},\"notify_on_private_message\":{},\"notify_on_room_mention\":{}}}", self.enabled, !self.user_key.is_empty(), !self.token.is_empty(), json_option(Some(&self.notification_prefix)), self.notify_on_private_message, self.notify_on_room_mention)
    }
}

impl Default for PushoverIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            user_key: String::new(),
            token: String::new(),
            notification_prefix: "slskR".to_owned(),
            notify_on_private_message: true,
            notify_on_room_mention: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PushbulletIntegrationSettings {
    pub enabled: bool,
    pub access_token: String,
    pub notification_prefix: String,
    pub notify_on_private_message: bool,
    pub notify_on_room_mention: bool,
    pub retry_attempts: u32,
    pub cooldown_time: i32,
}

impl PushbulletIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: PushbulletFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_PUSHBULLET", file.enabled.unwrap_or(false))?,
            access_token: env
                .var("SLSKD_PUSHBULLET_ACCESS_TOKEN")
                .or(file.access_token)
                .unwrap_or_default(),
            notification_prefix: env
                .var("SLSKD_PUSHBULLET_NOTIFICATION_PREFIX")
                .or(file.notification_prefix)
                .unwrap_or_else(|| "From slskR:".to_owned()),
            notify_on_private_message: env_bool_layer(
                env,
                "SLSKD_PUSHBULLET_NOTIFY_ON_PRIVATE_MESSAGE",
                file.notify_on_private_message.unwrap_or(true),
            )?,
            notify_on_room_mention: env_bool_layer(
                env,
                "SLSKD_PUSHBULLET_NOTIFY_ON_ROOM_MENTION",
                file.notify_on_room_mention.unwrap_or(true),
            )?,
            retry_attempts: env_parse_layer(
                env,
                "SLSKD_PUSHBULLET_RETRY_ATTEMPTS",
                file.retry_attempts,
                3,
            )?,
            cooldown_time: env_parse_layer(
                env,
                "SLSKD_PUSHBULLET_COOLDOWN_TIME",
                file.cooldown_time,
                900_000,
            )?,
        };
        if settings.retry_attempts > 5 {
            return Err("The field RetryAttempts must be between 0 and 5.".to_owned());
        }
        if settings.enabled && settings.access_token.trim().is_empty() {
            return Err(
                "The Enabled field is true, but no AccessToken has been specified.".to_owned(),
            );
        }
        Ok(settings)
    }
    fn sanitized_json(&self) -> String {
        format!("{{\"enabled\":{},\"access_token_configured\":{},\"notification_prefix\":{},\"notify_on_private_message\":{},\"notify_on_room_mention\":{},\"retry_attempts\":{},\"cooldown_time\":{}}}", self.enabled, !self.access_token.is_empty(), json_option(Some(&self.notification_prefix)), self.notify_on_private_message, self.notify_on_room_mention, self.retry_attempts, self.cooldown_time)
    }
}

impl Default for PushbulletIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            access_token: String::new(),
            notification_prefix: "From slskR:".to_owned(),
            notify_on_private_message: true,
            notify_on_room_mention: true,
            retry_attempts: 3,
            cooldown_time: 900_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FtpIntegrationSettings {
    pub enabled: bool,
    pub address: String,
    pub port: u16,
    pub encryption_mode: String,
    pub ignore_certificate_errors: bool,
    pub username: String,
    pub password: String,
    pub remote_path: String,
    pub overwrite_existing: bool,
    pub connection_timeout: u64,
    pub retry_attempts: u32,
}

impl FtpIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: FtpFileConfig, env: &E) -> Result<Self, String> {
        let port = env_parse_layer(env, "SLSKD_FTP_PORT", file.port, 21_u16)?;
        if port == 0 {
            return Err("The field Port must be between 1 and 65535.".to_owned());
        }
        let encryption_mode = env
            .var("SLSKD_FTP_ENCRYPTION_MODE")
            .or(file.encryption_mode)
            .unwrap_or_else(|| "auto".to_owned());
        if !matches!(
            encryption_mode.to_ascii_lowercase().as_str(),
            "none" | "implicit" | "explicit" | "auto"
        ) {
            return Err("The field EncryptionMode is invalid.".to_owned());
        }
        let retry_attempts =
            env_parse_layer(env, "SLSKD_FTP_RETRY_ATTEMPTS", file.retry_attempts, 3_u32)?;
        if retry_attempts > 5 {
            return Err("The field RetryAttempts must be between 0 and 5.".to_owned());
        }
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_FTP", file.enabled.unwrap_or(false))?,
            address: env
                .var("SLSKD_FTP_ADDRESS")
                .or(file.address)
                .unwrap_or_default(),
            port,
            encryption_mode,
            ignore_certificate_errors: env_bool_layer(
                env,
                "SLSKD_FTP_IGNORE_CERTIFICATE_ERRORS",
                file.ignore_certificate_errors.unwrap_or(false),
            )?,
            username: env
                .var("SLSKD_FTP_USERNAME")
                .or(file.username)
                .unwrap_or_default(),
            password: env
                .var("SLSKD_FTP_PASSWORD")
                .or(file.password)
                .unwrap_or_default(),
            remote_path: env
                .var("SLSKD_FTP_REMOTE_PATH")
                .or(file.remote_path)
                .unwrap_or_else(|| "/".to_owned()),
            overwrite_existing: env_bool_layer(
                env,
                "SLSKD_FTP_OVERWRITE_EXISTING",
                file.overwrite_existing.unwrap_or(true),
            )?,
            connection_timeout: env_parse_layer(
                env,
                "SLSKD_FTP_CONNECTION_TIMEOUT",
                file.connection_timeout,
                5_000_u64,
            )?,
            retry_attempts,
        };
        if settings.connection_timeout > i32::MAX as u64 {
            return Err("The field ConnectionTimeout must be between 0 and 2147483647.".to_owned());
        }
        if settings.enabled && settings.address.trim().is_empty() {
            return Err("The Enabled field is true, but no Address has been specified.".to_owned());
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"address\":{},\"port\":{},\"encryption_mode\":{},\"ignore_certificate_errors\":{},\"username\":{},\"password_configured\":{},\"remote_path\":{},\"overwrite_existing\":{},\"connection_timeout\":{},\"retry_attempts\":{}}}",
            self.enabled,
            json_option(Some(&self.address)),
            self.port,
            json_option(Some(&self.encryption_mode)),
            self.ignore_certificate_errors,
            json_option(Some(&self.username)),
            !self.password.is_empty(),
            json_option(Some(&self.remote_path)),
            self.overwrite_existing,
            self.connection_timeout,
            self.retry_attempts,
        )
    }
}

impl Default for FtpIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            address: String::new(),
            port: 21,
            encryption_mode: "auto".to_owned(),
            ignore_certificate_errors: false,
            username: String::new(),
            password: String::new(),
            remote_path: "/".to_owned(),
            overwrite_existing: true,
            connection_timeout: 5_000,
            retry_attempts: 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VpnIntegrationSettings {
    pub enabled: bool,
    pub port_forwarding: bool,
    pub self_hosted_relay: bool,
    pub polling_interval: u64,
    pub gluetun: GluetunIntegrationSettings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GluetunIntegrationSettings {
    pub url: String,
    pub timeout: u64,
    pub auth: String,
    pub username: String,
    pub password: String,
    pub api_key: String,
}

impl VpnIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file: VpnFileConfig, env: &E) -> Result<Self, String> {
        let polling_interval = env_parse_layer(
            env,
            "SLSKD_VPN_POLLING_INTERVAL",
            file.polling_interval,
            2_500_u64,
        )?;
        if !(500..=i32::MAX as u64).contains(&polling_interval) {
            return Err("The field PollingInterval must be between 500 and 2147483647.".to_owned());
        }
        let timeout = env_parse_layer(
            env,
            "SLSKD_VPN_GLUETUN_TIMEOUT",
            file.gluetun.timeout,
            1_000_u64,
        )?;
        if !(500..=10_000).contains(&timeout) {
            return Err("The field Timeout must be between 500 and 10000.".to_owned());
        }
        let settings = Self {
            enabled: env_bool_layer(env, "SLSKD_VPN", file.enabled.unwrap_or(false))?,
            port_forwarding: env_bool_layer(
                env,
                "SLSKD_VPN_PORT_FORWARDING",
                file.port_forwarding.unwrap_or(false),
            )?,
            self_hosted_relay: env_bool_layer(
                env,
                "SLSKD_VPN_SELF_HOSTED_RELAY",
                file.self_hosted_relay.unwrap_or(false),
            )?,
            polling_interval,
            gluetun: GluetunIntegrationSettings {
                url: env
                    .var("SLSKD_VPN_GLUETUN_URL")
                    .or(file.gluetun.url)
                    .unwrap_or_default(),
                timeout,
                auth: file.gluetun.auth.unwrap_or_default(),
                username: env
                    .var("SLSKD_VPN_GLUETUN_USERNAME")
                    .or(file.gluetun.username)
                    .unwrap_or_default(),
                password: env
                    .var("SLSKD_VPN_GLUETUN_PASSWORD")
                    .or(file.gluetun.password)
                    .unwrap_or_default(),
                api_key: env
                    .var("SLSKD_VPN_GLUETUN_API_KEY")
                    .or(file.gluetun.api_key)
                    .unwrap_or_default(),
            },
        };
        if settings.enabled {
            if settings.gluetun.url.trim().is_empty() {
                return Err("VPN is enabled but no client is configured".to_owned());
            }
            let url = reqwest::Url::parse(&settings.gluetun.url).map_err(|_| {
                "The gluetun URL must be absolute, e.g. 'http://127.0.0.1:8000'".to_owned()
            })?;
            if url.cannot_be_a_base() {
                return Err(
                    "The gluetun URL must be absolute, e.g. 'http://127.0.0.1:8000'".to_owned(),
                );
            }
            if settings.self_hosted_relay && !settings.port_forwarding {
                return Err("Self-hosted relay mode requires VPN port forwarding".to_owned());
            }
        }
        Ok(settings)
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"port_forwarding\":{},\"self_hosted_relay\":{},\"polling_interval\":{},\"gluetun\":{{\"url\":{},\"timeout\":{},\"auth\":{},\"username\":{},\"password_configured\":{},\"api_key_configured\":{}}}}}",
            self.enabled,
            self.port_forwarding,
            self.self_hosted_relay,
            self.polling_interval,
            json_option(Some(&self.gluetun.url)),
            self.gluetun.timeout,
            json_option(Some(&self.gluetun.auth)),
            json_option(Some(&self.gluetun.username)),
            !self.gluetun.password.is_empty(),
            !self.gluetun.api_key.is_empty(),
        )
    }
}

impl Default for VpnIntegrationSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port_forwarding: false,
            self_hosted_relay: false,
            polling_interval: 2_500,
            gluetun: GluetunIntegrationSettings {
                url: String::new(),
                timeout: 1_000,
                auth: String::new(),
                username: String::new(),
                password: String::new(),
                api_key: String::new(),
            },
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ScriptIntegrationSettings {
    pub on: Vec<String>,
    pub run: ScriptRunSettings,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ScriptRunSettings {
    pub command: String,
    pub executable: String,
    pub args: String,
    #[serde(alias = "args_list")]
    pub arglist: Option<Vec<String>>,
}

impl ScriptIntegrationSettings {
    fn from_layers<E: ConfigEnv>(
        file: BTreeMap<String, Self>,
        env: &E,
    ) -> Result<BTreeMap<String, Self>, String> {
        let scripts = match env.var("SLSKR_FROZEN_SCRIPTS_JSON") {
            Some(value) => serde_json::from_str::<BTreeMap<String, Self>>(&value)
                .map_err(|_| "invalid integrations.scripts configuration".to_owned())?,
            None => file,
        };
        const EVENTS: &[&str] = &[
            "None",
            "Any",
            "DownloadFileComplete",
            "DownloadDirectoryComplete",
            "UploadFileComplete",
            "DownloadFileFailed",
            "PrivateMessageReceived",
            "RoomMessageReceived",
            "SearchResponsesReceived",
            "PeerSearchedUs",
            "PeerDownloadedFromUs",
            "SoulseekClientConnected",
            "SoulseekClientDisconnected",
            "Noop",
        ];
        for script in scripts.values() {
            if script
                .on
                .iter()
                .any(|event| !EVENTS.iter().any(|known| known.eq_ignore_ascii_case(event)))
            {
                return Err("The field On contains an invalid event type.".to_owned());
            }
            let command_set = !script.run.command.trim().is_empty();
            let executable_set = !script.run.executable.trim().is_empty();
            if command_set == executable_set {
                return Err("One and only one of the fields Command or Executable may be specified for a single script. If you intend to use the system shell, omit 'executable'. If you intend to use an executable other than the system shell, omit 'command' and specify either 'args' or 'args_list'.".to_owned());
            }
            if !script.run.args.trim().is_empty() && script.run.arglist.is_some() {
                return Err("Only one of the fields Args or Arglist may be specified for a single script. Specify 'args' if you intend to construct a single quoted string yourself, and specify 'args_list' if you'd like slskd to handle quoting for you.".to_owned());
            }
        }
        Ok(scripts)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SourceFeedApiKeySettings {
    pub enabled: bool,
    pub api_key: Option<String>,
}

impl SourceFeedApiKeySettings {
    fn from_layers<E: ConfigEnv>(
        file_config: SourceFeedApiKeyFileConfig,
        env: &E,
        enabled_name: &str,
        api_key_name: &str,
        missing_key_error: &str,
    ) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_layer(env, enabled_name, file_config.enabled.unwrap_or(false))?,
            api_key: optional_env_any(env, &[api_key_name]).or(file_config.api_key),
        };
        if settings.enabled
            && settings
                .api_key
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(missing_key_error.to_owned());
        }
        Ok(settings)
    }

    pub fn configured(&self) -> bool {
        self.enabled
            && self
                .api_key
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"api_key_configured\":{}}}",
            self.enabled,
            self.api_key.is_some()
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct SpotifyIntegrationSettings {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub timeout_seconds: u64,
    pub max_items_per_import: u64,
    pub market: String,
    pub scopes: String,
}

impl SpotifyIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: SpotifyFileConfig, env: &E) -> Result<Self, String> {
        let settings = Self {
            enabled: env_bool_any_layer(
                env,
                &["SLSKR_SPOTIFY_ENABLED", "SLSKD_SPOTIFY"],
                file_config.enabled.unwrap_or(false),
            )?,
            client_id: optional_env_any(
                env,
                &["SLSKR_SPOTIFY_CLIENT_ID", "SLSKD_SPOTIFY_CLIENT_ID"],
            )
            .or(file_config.client_id),
            client_secret: optional_env_any(
                env,
                &["SLSKR_SPOTIFY_CLIENT_SECRET", "SLSKD_SPOTIFY_CLIENT_SECRET"],
            )
            .or(file_config.client_secret),
            redirect_uri: optional_env_any(
                env,
                &["SLSKR_SPOTIFY_REDIRECT_URI", "SLSKD_SPOTIFY_REDIRECT_URI"],
            )
            .or(file_config.redirect_uri),
            timeout_seconds: env_parse_any_layer(
                env,
                &["SLSKR_SPOTIFY_TIMEOUT_SECONDS", "SLSKD_SPOTIFY_TIMEOUT"],
                file_config.timeout_seconds,
                20_u64,
            )?,
            max_items_per_import: env_parse_any_layer(
                env,
                &[
                    "SLSKR_SPOTIFY_MAX_ITEMS_PER_IMPORT",
                    "SLSKD_SPOTIFY_MAX_ITEMS_PER_IMPORT",
                ],
                file_config.max_items_per_import,
                500_u64,
            )?,
            market: optional_env_any(env, &["SLSKR_SPOTIFY_MARKET", "SLSKD_SPOTIFY_MARKET"])
                .or(file_config.market)
                .unwrap_or_else(|| "US".to_owned()),
            scopes: env
                .var("SLSKR_SPOTIFY_SCOPES")
                .or(file_config.scopes)
                .unwrap_or_else(|| {
                    "user-library-read user-follow-read playlist-read-private playlist-read-collaborative"
                        .to_owned()
                }),
        };
        if !(1..=120).contains(&settings.timeout_seconds) {
            return Err("Spotify timeout must be between 1 and 120 seconds".to_owned());
        }
        if !(1..=5_000).contains(&settings.max_items_per_import) {
            return Err("Spotify maximum items per import must be between 1 and 5000".to_owned());
        }
        if settings.enabled
            && settings
                .client_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(
                "Spotify is enabled but integrations.spotify.client_id is empty.".to_owned(),
            );
        }
        if settings.enabled && settings.market.encode_utf16().count() != 2 {
            return Err("Spotify market must be a two-letter market code.".to_owned());
        }
        Ok(settings)
    }

    pub fn configured(&self) -> bool {
        self.enabled
            && self
                .client_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"client_id_configured\":{},\"client_secret_configured\":{},\"redirect_uri\":null,\"redirect_uri_configured\":{},\"timeout_seconds\":{},\"max_items_per_import\":{},\"market\":\"{}\",\"scopes\":\"{}\"}}",
            self.enabled,
            self.client_id.is_some(),
            self.client_secret.is_some(),
            self.redirect_uri.is_some(),
            self.timeout_seconds,
            self.max_items_per_import,
            json_escape(&self.market),
            json_escape(&self.scopes)
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct LidarrIntegrationSettings {
    pub enabled: bool,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub timeout_seconds: u64,
    pub sync_wanted_to_wishlist: bool,
    pub sync_interval_seconds: u64,
    pub max_items_per_sync: u64,
    pub auto_download: bool,
    pub wishlist_filter: String,
    pub wishlist_max_results: u64,
    pub auto_import_completed: bool,
    pub import_delay_seconds: u64,
    pub import_retry_max_attempts: u32,
    pub import_retry_delay_seconds: u64,
    pub import_path_from: String,
    pub import_path_to: String,
    pub import_mode: String,
    pub import_replace_existing_files: bool,
    pub skip_already_owned_albums: bool,
    pub delete_rejected_downloads: bool,
    pub blacklist_rejected_downloads: bool,
    pub edition_match_mode: String,
}

impl LidarrIntegrationSettings {
    fn from_layers<E: ConfigEnv>(
        file_config: LidarrFileConfig,
        env: &E,
        current_upstream_behavior: bool,
    ) -> Result<Self, String> {
        let url = optional_env_any(
            env,
            &profile_env_names(
                current_upstream_behavior,
                "SLSKR_LIDARR_URL",
                "LIDARR_URL",
                "SLSKD_LIDARR_URL",
            ),
        )
        .or(file_config.url);
        if let Some(url) = url.as_deref().filter(|value| !value.trim().is_empty()) {
            let parsed = reqwest::Url::parse(url)
                .map_err(|error| format!("Lidarr URL is invalid: {error}"))?;
            if !matches!(parsed.scheme(), "http" | "https") {
                return Err("Lidarr URL scheme must be http or https".to_owned());
            }
            if parsed.host_str().is_none() {
                return Err("Lidarr URL must include a host".to_owned());
            }
            if !parsed.username().is_empty() || parsed.password().is_some() {
                return Err("Lidarr URL must not contain embedded credentials".to_owned());
            }
            if parsed.query().is_some() || parsed.fragment().is_some() {
                return Err("Lidarr URL must not contain a query or fragment".to_owned());
            }
        }
        let settings = Self {
            enabled: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_ENABLED",
                    "LIDARR",
                    "SLSKD_LIDARR",
                ),
                file_config.enabled.unwrap_or(false),
            )?,
            url,
            api_key: optional_env_any(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_API_KEY",
                    "LIDARR_API_KEY",
                    "SLSKD_LIDARR_API_KEY",
                ),
            )
            .or(file_config.api_key),
            timeout_seconds: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_TIMEOUT_SECONDS",
                    "LIDARR_TIMEOUT",
                    "SLSKD_LIDARR_TIMEOUT",
                ),
                file_config.timeout_seconds,
                20_u64,
            )?,
            sync_wanted_to_wishlist: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_SYNC_WANTED",
                    "LIDARR_SYNC_WANTED",
                    "SLSKD_LIDARR_SYNC_WANTED",
                ),
                file_config.sync_wanted_to_wishlist.unwrap_or(false),
            )?,
            sync_interval_seconds: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_SYNC_INTERVAL",
                    "LIDARR_SYNC_INTERVAL",
                    "SLSKD_LIDARR_SYNC_INTERVAL",
                ),
                file_config.sync_interval_seconds,
                3_600_u64,
            )?,
            max_items_per_sync: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_SYNC_MAX_ITEMS",
                    "LIDARR_SYNC_MAX_ITEMS",
                    "SLSKD_LIDARR_SYNC_MAX_ITEMS",
                ),
                file_config.max_items_per_sync,
                100_u64,
            )?,
            auto_download: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_AUTO_DOWNLOAD",
                    "LIDARR_AUTO_DOWNLOAD",
                    "SLSKD_LIDARR_AUTO_DOWNLOAD",
                ),
                file_config.auto_download.unwrap_or(false),
            )?,
            wishlist_filter: optional_env_any(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_WISHLIST_FILTER",
                    "LIDARR_WISHLIST_FILTER",
                    "SLSKD_LIDARR_WISHLIST_FILTER",
                ),
            )
            .or(file_config.wishlist_filter)
            .unwrap_or_default(),
            wishlist_max_results: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_WISHLIST_MAX_RESULTS",
                    "LIDARR_WISHLIST_MAX_RESULTS",
                    "SLSKD_LIDARR_WISHLIST_MAX_RESULTS",
                ),
                file_config.wishlist_max_results,
                100_u64,
            )?,
            auto_import_completed: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_AUTO_IMPORT_COMPLETED",
                    "LIDARR_AUTO_IMPORT_COMPLETED",
                    "SLSKD_LIDARR_AUTO_IMPORT_COMPLETED",
                ),
                file_config.auto_import_completed.unwrap_or(false),
            )?,
            import_delay_seconds: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_DELAY",
                    "LIDARR_IMPORT_DELAY",
                    "SLSKD_LIDARR_IMPORT_DELAY",
                ),
                file_config.import_delay_seconds,
                0_u64,
            )?,
            import_retry_max_attempts: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_RETRY_MAX_ATTEMPTS",
                    "LIDARR_IMPORT_RETRY_MAX_ATTEMPTS",
                    "SLSKD_LIDARR_IMPORT_RETRY_MAX_ATTEMPTS",
                ),
                file_config.import_retry_max_attempts,
                2_u32,
            )?,
            import_retry_delay_seconds: env_parse_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_RETRY_DELAY",
                    "LIDARR_IMPORT_RETRY_DELAY",
                    "SLSKD_LIDARR_IMPORT_RETRY_DELAY",
                ),
                file_config.import_retry_delay_seconds,
                30_u64,
            )?,
            import_path_from: optional_env_any(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_PATH_FROM",
                    "LIDARR_IMPORT_PATH_FROM",
                    "SLSKD_LIDARR_IMPORT_PATH_FROM",
                ),
            )
            .or(file_config.import_path_from)
            .unwrap_or_default(),
            import_path_to: optional_env_any(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_PATH_TO",
                    "LIDARR_IMPORT_PATH_TO",
                    "SLSKD_LIDARR_IMPORT_PATH_TO",
                ),
            )
            .or(file_config.import_path_to)
            .unwrap_or_default(),
            import_mode: optional_env_any(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_MODE",
                    "LIDARR_IMPORT_MODE",
                    "SLSKD_LIDARR_IMPORT_MODE",
                ),
            )
            .or(file_config.import_mode)
            .unwrap_or_else(|| "move".to_owned()),
            import_replace_existing_files: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_IMPORT_REPLACE_EXISTING",
                    "LIDARR_IMPORT_REPLACE_EXISTING",
                    "SLSKD_LIDARR_IMPORT_REPLACE_EXISTING",
                ),
                file_config.import_replace_existing_files.unwrap_or(false),
            )?,
            skip_already_owned_albums: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_SKIP_ALREADY_OWNED_ALBUMS",
                    "LIDARR_SKIP_ALREADY_OWNED_ALBUMS",
                    "SLSKD_LIDARR_SKIP_ALREADY_OWNED_ALBUMS",
                ),
                file_config.skip_already_owned_albums.unwrap_or(true),
            )?,
            delete_rejected_downloads: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_DELETE_REJECTED_DOWNLOADS",
                    "LIDARR_DELETE_REJECTED_DOWNLOADS",
                    "SLSKD_LIDARR_DELETE_REJECTED_DOWNLOADS",
                ),
                file_config.delete_rejected_downloads.unwrap_or(false),
            )?,
            blacklist_rejected_downloads: env_bool_any_layer(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_BLACKLIST_REJECTED_DOWNLOADS",
                    "LIDARR_BLACKLIST_REJECTED_DOWNLOADS",
                    "SLSKD_LIDARR_BLACKLIST_REJECTED_DOWNLOADS",
                ),
                file_config.blacklist_rejected_downloads.unwrap_or(false),
            )?,
            edition_match_mode: optional_env_any(
                env,
                &profile_env_names(
                    current_upstream_behavior,
                    "SLSKR_LIDARR_EDITION_MATCH_MODE",
                    "LIDARR_EDITION_MATCH_MODE",
                    "SLSKD_LIDARR_EDITION_MATCH_MODE",
                ),
            )
            .or(file_config.edition_match_mode)
            .unwrap_or_else(|| "exclude".to_owned())
            .trim()
            .to_ascii_lowercase(),
        };
        if settings.import_delay_seconds > 600 {
            return Err("Lidarr import delay must be between 0 and 600 seconds".to_owned());
        }
        if settings.import_retry_max_attempts > 10 {
            return Err("Lidarr import retry attempts must be between 0 and 10".to_owned());
        }
        if !(1..=3_600).contains(&settings.import_retry_delay_seconds) {
            return Err("Lidarr import retry delay must be between 1 and 3600 seconds".to_owned());
        }
        if !(1..=120).contains(&settings.timeout_seconds) {
            return Err("Lidarr timeout must be between 1 and 120 seconds".to_owned());
        }
        if settings.sync_interval_seconds < 300 {
            return Err("Lidarr sync interval must be at least 300 seconds".to_owned());
        }
        if !(1..=1_000).contains(&settings.max_items_per_sync) {
            return Err("Lidarr maximum items per sync must be between 1 and 1000".to_owned());
        }
        if !(10..=1_000).contains(&settings.wishlist_max_results) {
            return Err("Lidarr wishlist maximum results must be between 10 and 1000".to_owned());
        }
        if settings.enabled {
            if settings
                .url
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(
                    "Lidarr is enabled but integrations.lidarr.url is not an absolute URL."
                        .to_owned(),
                );
            }
            if settings
                .api_key
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                return Err(
                    "Lidarr is enabled but integrations.lidarr.api_key is empty.".to_owned(),
                );
            }
            let from_configured = !settings.import_path_from.trim().is_empty();
            let to_configured = !settings.import_path_to.trim().is_empty();
            if settings.auto_import_completed && from_configured != to_configured {
                return Err(
                    "Lidarr import path mapping requires both import_path_from and import_path_to."
                        .to_owned(),
                );
            }
            if !matches!(
                settings.import_mode.as_str(),
                "move" | "copy" | "Move" | "Copy"
            ) {
                return Err("Lidarr import_mode must be 'move' or 'copy'.".to_owned());
            }
            if !matches!(
                settings.edition_match_mode.as_str(),
                "exclude" | "prefer" | "off"
            ) {
                return Err(
                    "Lidarr edition_match_mode must be 'exclude', 'prefer', or 'off'.".to_owned(),
                );
            }
        }
        Ok(settings)
    }

    pub fn configured(&self) -> bool {
        self.enabled
            && self
                .url
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
            && self
                .api_key
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"url\":null,\"url_configured\":{},\"api_key_configured\":{},\"timeout_seconds\":{},\"sync_wanted_to_wishlist\":{},\"sync_interval_seconds\":{},\"max_items_per_sync\":{},\"auto_download\":{},\"wishlist_filter\":\"{}\",\"wishlist_max_results\":{},\"auto_import_completed\":{},\"import_delay_seconds\":{},\"import_retry_max_attempts\":{},\"import_retry_delay_seconds\":{},\"import_path_from\":\"{}\",\"import_path_to\":\"{}\",\"import_mode\":\"{}\",\"import_replace_existing_files\":{},\"skip_already_owned_albums\":{},\"delete_rejected_downloads\":{},\"blacklist_rejected_downloads\":{},\"edition_match_mode\":\"{}\"}}",
            self.enabled,
            self.url.is_some(),
            self.api_key.is_some(),
            self.timeout_seconds,
            self.sync_wanted_to_wishlist,
            self.sync_interval_seconds,
            self.max_items_per_sync,
            self.auto_download,
            json_escape(&self.wishlist_filter),
            self.wishlist_max_results,
            self.auto_import_completed,
            self.import_delay_seconds,
            self.import_retry_max_attempts,
            self.import_retry_delay_seconds,
            json_escape(&self.import_path_from),
            json_escape(&self.import_path_to),
            json_escape(&self.import_mode),
            self.import_replace_existing_files,
            self.skip_already_owned_albums,
            self.delete_rejected_downloads,
            self.blacklist_rejected_downloads,
            json_escape(&self.edition_match_mode),
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct BridgeIntegrationSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

impl BridgeIntegrationSettings {
    fn from_layers<E: ConfigEnv>(file_config: BridgeFileConfig, env: &E) -> Result<Self, String> {
        Ok(Self {
            enabled: env_bool_layer(
                env,
                "SLSKR_BRIDGE_ENABLED",
                file_config.enabled.unwrap_or(false),
            )?,
            host: env
                .var("SLSKR_BRIDGE_HOST")
                .or(file_config.host)
                .unwrap_or_else(|| "localhost".to_owned()),
            port: env_parse_layer(env, "SLSKR_BRIDGE_PORT", file_config.port, 3000_u16)?,
        })
    }

    pub fn endpoint_configured(&self) -> bool {
        !self.host.trim().is_empty() && self.port != 0
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"enabled\":{},\"host\":null,\"port\":null,\"endpoint_configured\":{}}}",
            self.enabled,
            self.endpoint_configured()
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExternalVisualizerSettings {
    pub command: Option<String>,
    pub launch_enabled: bool,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub name: String,
}

impl ExternalVisualizerSettings {
    fn from_layers<E: ConfigEnv>(
        file_config: ExternalVisualizerFileConfig,
        env: &E,
    ) -> Result<Self, String> {
        Ok(Self {
            command: env
                .var("SLSKR_EXTERNAL_VISUALIZER_COMMAND")
                .or(file_config.command),
            launch_enabled: env_bool_layer(
                env,
                "SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED",
                file_config.launch_enabled.unwrap_or(false),
            )?,
            arguments: Vec::new(),
            working_directory: None,
            name: "MilkDrop3".to_owned(),
        })
    }

    pub fn configured(&self) -> bool {
        self.command
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }

    pub fn sanitized_json(&self) -> String {
        format!(
            "{{\"configured\":{},\"launch_enabled\":{},\"command\":null,\"argument_count\":{},\"working_directory_configured\":{},\"name\":{}}}",
            self.configured(),
            self.launch_enabled,
            self.arguments.len(),
            self.working_directory.is_some(),
            serde_json::to_string(&self.name).unwrap_or_else(|_| "\"MilkDrop3\"".to_owned()),
        )
    }
}
