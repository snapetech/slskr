use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferRetentionFileConfig {
    pub(in crate::config) upload: TransferTypeRetentionFileConfig,
    pub(in crate::config) download: TransferTypeRetentionFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferTypeRetentionFileConfig {
    pub(in crate::config) succeeded: Option<u64>,
    pub(in crate::config) errored: Option<u64>,
    pub(in crate::config) cancelled: Option<u64>,
    pub(in crate::config) failed: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupsFileConfig {
    pub(in crate::config) default: TransferGroupFileConfig,
    pub(in crate::config) leechers: LeecherTransferGroupFileConfig,
    pub(in crate::config) blacklisted: UserBlacklistFileConfig,
    pub(in crate::config) user_defined: BTreeMap<String, UserDefinedTransferGroupFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferGroupFileConfig {
    pub(in crate::config) upload: TransferGroupUploadFileConfig,
    pub(in crate::config) limits: Option<TransferLimitsFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LeecherTransferGroupFileConfig {
    pub(in crate::config) upload: TransferGroupUploadFileConfig,
    pub(in crate::config) limits: Option<TransferLimitsFileConfig>,
    pub(in crate::config) thresholds: LeecherThresholdFileConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserDefinedTransferGroupFileConfig {
    pub(in crate::config) upload: TransferGroupUploadFileConfig,
    pub(in crate::config) limits: Option<TransferLimitsFileConfig>,
    pub(in crate::config) members: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferGroupUploadFileConfig {
    pub(in crate::config) priority: Option<u32>,
    #[serde(alias = "queue_strategy")]
    pub(in crate::config) strategy: Option<String>,
    pub(in crate::config) slots: Option<u32>,
    pub(in crate::config) speed_limit: Option<u32>,
    pub(in crate::config) allowed_file_types: Vec<String>,
    pub(in crate::config) limits: Option<TransferLimitsFileConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LeecherThresholdFileConfig {
    pub(in crate::config) files: Option<u32>,
    pub(in crate::config) directories: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferLimitsFileConfig {
    pub(in crate::config) queued: NullableConfig<TransferLimitFileConfig>,
    pub(in crate::config) daily: NullableConfig<TransferLimitFileConfig>,
    pub(in crate::config) weekly: NullableConfig<TransferLimitFileConfig>,
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
pub(in crate::config) enum NullableConfig<T> {
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
    pub(in crate::config) files: Option<u32>,
    pub(in crate::config) megabytes: Option<u32>,
    pub(in crate::config) failures: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserBlacklistFileConfig {
    pub(in crate::config) members: Vec<String>,
    pub(in crate::config) patterns: Vec<String>,
    pub(in crate::config) cidrs: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferFileConfig {
    pub(in crate::config) history_limit: Option<usize>,
    pub(in crate::config) max_active: Option<usize>,
    pub(in crate::config) allow_inbound: Option<bool>,
    pub(in crate::config) allow_outbound: Option<bool>,
    pub(in crate::config) auto_retry: TransferAutoRetryFileConfig,
    pub(in crate::config) rescue: TransferRescueFileConfig,
    pub(in crate::config) completed_path_template: Option<String>,
    pub(in crate::config) upload: TransferUploadFileConfig,
    pub(in crate::config) download: TransferDownloadFileConfig,
    pub(in crate::config) groups: GroupsFileConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferUploadFileConfig {
    pub(in crate::config) slots: Option<u32>,
    pub(in crate::config) speed_limit: Option<u32>,
    pub(in crate::config) limits: Option<TransferLimitsFileConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadFileConfig {
    pub(in crate::config) slots: Option<u32>,
    pub(in crate::config) speed_limit: Option<u32>,
    pub(in crate::config) retry: TransferDownloadRetryFileConfig,
    pub(in crate::config) destination: TransferDownloadDestinationFileConfig,
    pub(in crate::config) completed_layout: Option<String>,
    pub(in crate::config) completed_path_template: Option<String>,
    pub(in crate::config) auto_replace_stuck: Option<bool>,
    pub(in crate::config) auto_replace_threshold: Option<f64>,
    pub(in crate::config) auto_replace_interval: Option<u64>,
    pub(in crate::config) auto_retry: TransferAutoRetryFileConfig,
    pub(in crate::config) cost_based_scheduling: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadRetryFileConfig {
    pub(in crate::config) partial: Option<String>,
    pub(in crate::config) incomplete: Option<String>,
    pub(in crate::config) attempts: Option<u32>,
    pub(in crate::config) delay: Option<u64>,
    pub(in crate::config) max_delay: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadDestinationFileConfig {
    pub(in crate::config) subdirectory: NullableConfig<String>,
    pub(in crate::config) exists: Option<String>,
    pub(in crate::config) permissions: TransferDownloadPermissionsFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferDownloadPermissionsFileConfig {
    pub(in crate::config) mode: Option<String>,
}

impl TransferLimitSettings {
    pub(super) fn from_file(value: TransferLimitFileConfig, path: &str) -> Result<Self, String> {
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
    pub(super) fn from_file(
        value: Option<TransferLimitsFileConfig>,
        path: &str,
    ) -> Result<Self, String> {
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
    pub(super) fn from_file(
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
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
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

pub(in crate::config) fn groups_file_config_is_empty(value: &GroupsFileConfig) -> bool {
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
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
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
    pub(in crate::config) fn from_layers<E: ConfigEnv>(
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
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) retry_delay_seconds: Option<u64>,
    pub(in crate::config) check_interval_seconds: Option<u64>,
    pub(in crate::config) max_attempts: Option<usize>,
    pub(in crate::config) max_files_per_cycle: Option<usize>,
    pub(in crate::config) max_files_per_peer_per_cycle: Option<usize>,
    pub(in crate::config) peer_cooldown_seconds: Option<u64>,
    pub(in crate::config) alternate_sources_enabled: Option<bool>,
    pub(in crate::config) max_alternate_source_searches_per_cycle: Option<usize>,
    pub(in crate::config) alternate_source_size_tolerance_percent: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransferRescueFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) max_queue_time_seconds: Option<u64>,
    pub(in crate::config) min_throughput_kbps: Option<u64>,
    pub(in crate::config) min_duration_seconds: Option<u64>,
    pub(in crate::config) stalled_timeout_seconds: Option<u64>,
    pub(in crate::config) check_interval_seconds: Option<u64>,
    pub(in crate::config) retry_cooldown_seconds: Option<u64>,
    pub(in crate::config) max_files_per_cycle: Option<usize>,
    pub(in crate::config) alternate_source_size_tolerance_percent: Option<u32>,
}
