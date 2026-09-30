use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SignalSystemFileConfig {
    #[serde(alias = "Enabled")]
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "DeduplicationCacheSize")]
    pub(in crate::config) deduplication_cache_size: Option<usize>,
    #[serde(
        alias = "DefaultTtl",
        alias = "defaultTTL",
        alias = "default_ttl_seconds"
    )]
    pub(in crate::config) default_ttl: Option<SignalDurationFileValue>,
    #[serde(alias = "MeshChannel")]
    pub(in crate::config) mesh_channel: SignalChannelFileConfig,
    #[serde(alias = "BtExtensionChannel")]
    pub(in crate::config) bt_extension_channel: SignalChannelFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct SignalChannelFileConfig {
    #[serde(alias = "Enabled")]
    pub(in crate::config) enabled: Option<bool>,
    #[serde(alias = "Priority")]
    pub(in crate::config) priority: Option<u8>,
    #[serde(alias = "RequireActiveSession")]
    pub(in crate::config) require_active_session: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SignalDurationFileValue {
    Seconds(u64),
    Text(String),
}

impl SignalSystemSettings {
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
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

pub(super) fn signal_duration_from_seconds(path: &str, seconds: u64) -> Result<Duration, String> {
    if seconds == 0 {
        return Err(format!("{path} must be greater than zero"));
    }
    Ok(Duration::from_secs(seconds))
}

pub(super) fn parse_signal_duration(path: &str, value: &str) -> Result<Duration, String> {
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
