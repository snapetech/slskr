use super::*;

pub(super) fn validated_runtime_interval(name: &str, seconds: u64) -> Result<Duration, String> {
    if seconds == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    let duration = Duration::from_secs(seconds);
    if std::time::Instant::now().checked_add(duration).is_none() {
        return Err(format!("{name} exceeds the runtime timer range"));
    }
    Ok(duration)
}

pub(super) fn bounded_config_value<T>(name: &str, value: T, min: T, max: T) -> Result<T, String>
where
    T: Copy + PartialOrd + std::fmt::Display,
{
    if value < min || value > max {
        return Err(format!("{name} must be between {min} and {max}"));
    }
    Ok(value)
}

pub fn optional_env_any(env: &dyn ConfigEnv, names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env.var(name))
}

pub(super) fn profile_env_names(
    current_upstream_behavior: bool,
    native_name: &'static str,
    canonical_name: &'static str,
    compatibility_name: &'static str,
) -> Vec<&'static str> {
    if current_upstream_behavior {
        vec![native_name, canonical_name, compatibility_name]
    } else {
        vec![native_name, compatibility_name]
    }
}

pub(super) fn controller_string_array_layer<E: ConfigEnv>(
    env: &E,
    name: &str,
    file_value: Vec<String>,
) -> Vec<String> {
    env.var(name).map_or(file_value, |value| {
        if value.is_empty() {
            Vec::new()
        } else {
            value.split(';').map(str::to_owned).collect()
        }
    })
}

pub(super) fn string_array_any_layer<E: ConfigEnv>(
    env: &E,
    names: &[&str],
    file_value: Vec<String>,
) -> Vec<String> {
    names
        .iter()
        .find_map(|name| env.var(name))
        .map_or(file_value, |value| {
            if value.is_empty() {
                Vec::new()
            } else {
                value.split(';').map(str::to_owned).collect()
            }
        })
}

pub(super) fn normalized_controller_values(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.to_ascii_lowercase()))
        .collect()
}

pub(super) fn env_parse_layer<E, T>(
    env: &E,
    name: &str,
    file_value: Option<T>,
    default: T,
) -> Result<T, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env.var(name) {
        Some(value) => value
            .parse::<T>()
            .map_err(|error| format!("invalid {name}: {error}")),
        None => Ok(file_value.unwrap_or(default)),
    }
}

pub(super) fn env_parse_any_layer<E, T>(
    env: &E,
    names: &[&str],
    file_value: Option<T>,
    default: T,
) -> Result<T, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some((name, value)) = names
        .iter()
        .find_map(|name| env.var(name).map(|value| (*name, value)))
    else {
        return Ok(file_value.unwrap_or(default));
    };
    value
        .parse::<T>()
        .map_err(|error| format!("invalid {name}: {error}"))
}

pub(super) fn env_parse_any_option<E, T>(env: &E, names: &[&str]) -> Result<Option<T>, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some((name, value)) = names
        .iter()
        .find_map(|name| env.var(name).map(|value| (*name, value)))
    else {
        return Ok(None);
    };
    value
        .parse::<T>()
        .map(Some)
        .map_err(|error| format!("invalid {name}: {error}"))
}

pub(super) struct MiscControllerFlags {
    pub(super) remote_configuration: bool,
    pub(super) remote_file_management: bool,
    pub(super) controller_debug: bool,
    pub(super) controller_no_config_watch: bool,
    pub(super) controller_no_logo: bool,
    pub(super) controller_no_start: bool,
    pub(super) controller_no_version_check: bool,
    pub(super) controller_experimental: bool,
    pub(super) controller_hash_from_audio_file_enabled: bool,
    pub(super) controller_no_share_scan: bool,
    pub(super) controller_force_share_scan: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_misc_controller_flags<E: ConfigEnv>(
    env: &E,
    compatibility_remote_configuration: Option<bool>,
    compatibility_debug: Option<bool>,
    compatibility_no_config_watch: Option<bool>,
    flags_no_logo: Option<bool>,
    flags_no_start: Option<bool>,
    flags_no_version_check: Option<bool>,
    flags_experimental: Option<bool>,
    flags_hash_from_audio_file_enabled: Option<bool>,
    flags_no_share_scan: Option<bool>,
    flags_force_share_scan: Option<bool>,
) -> Result<MiscControllerFlags, String> {
    Ok(MiscControllerFlags {
        remote_configuration: env_bool_any_layer(
            env,
            &["SLSKR_REMOTE_CONFIGURATION", "SLSKD_REMOTE_CONFIGURATION"],
            compatibility_remote_configuration.unwrap_or(false),
        )?,
        remote_file_management: env_bool_any_layer(
            env,
            &[
                "SLSKR_REMOTE_FILE_MANAGEMENT",
                "SLSKD_REMOTE_FILE_MANAGEMENT",
            ],
            false,
        )?,
        controller_debug: env_bool_any_layer(
            env,
            &["SLSKR_DEBUG", "SLSKD_DEBUG"],
            compatibility_debug.unwrap_or(false),
        )?,
        controller_no_config_watch: env_bool_any_layer(
            env,
            &["SLSKR_NO_CONFIG_WATCH", "SLSKD_NO_CONFIG_WATCH"],
            compatibility_no_config_watch.unwrap_or(false),
        )?,
        controller_no_logo: env_bool_layer(env, "SLSKD_NO_LOGO", flags_no_logo.unwrap_or(false))?,
        controller_no_start: env_bool_layer(
            env,
            "SLSKD_NO_START",
            flags_no_start.unwrap_or(false),
        )?,
        controller_no_version_check: env_bool_layer(
            env,
            "SLSKD_NO_VERSION_CHECK",
            flags_no_version_check.unwrap_or(false),
        )?,
        controller_experimental: env_bool_layer(
            env,
            "SLSKD_EXPERIMENTAL",
            flags_experimental.unwrap_or(false),
        )?,
        controller_hash_from_audio_file_enabled: env_bool_layer(
            env,
            "SLSKR_CONTROLLER_YAML_HASH_FROM_AUDIO_FILE_ENABLED",
            flags_hash_from_audio_file_enabled.unwrap_or(false),
        )?,
        controller_no_share_scan: env_bool_layer(
            env,
            "SLSKD_NO_SHARE_SCAN",
            flags_no_share_scan.unwrap_or(false),
        )?,
        controller_force_share_scan: env_bool_layer(
            env,
            "SLSKD_FORCE_SHARE_SCAN",
            flags_force_share_scan.unwrap_or(false),
        )?,
    })
}

pub(super) fn env_parse_option_layer<E, T>(
    env: &E,
    name: &str,
    file_value: Option<T>,
) -> Result<Option<T>, String>
where
    E: ConfigEnv,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env.var(name) {
        Some(value) => value
            .parse::<T>()
            .map(Some)
            .map_err(|error| format!("invalid {name}: {error}")),
        None => Ok(file_value),
    }
}

pub(super) fn env_bool_layer<E: ConfigEnv>(
    env: &E,
    name: &str,
    default: bool,
) -> Result<bool, String> {
    match env.var(name) {
        Some(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "0" | "false" | "no" | "off" => Ok(false),
            _ => Err(format!("invalid {name}: expected boolean")),
        },
        None => Ok(default),
    }
}

pub(super) fn env_bool_any_layer<E: ConfigEnv>(
    env: &E,
    names: &[&str],
    default: bool,
) -> Result<bool, String> {
    let Some((name, value)) = names
        .iter()
        .find_map(|name| env.var(name).map(|value| (*name, value)))
    else {
        return Ok(default);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid {name}: expected boolean")),
    }
}
