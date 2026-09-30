use super::*;

pub(in crate::cli) fn required_env_any(names: &[&str]) -> Result<String, String> {
    for name in names {
        if let Ok(value) = std::env::var(name) {
            return Ok(value);
        }
    }

    Err(format!("one of {} is required", names.join(", ")))
}

#[derive(Debug, Clone)]
pub(super) struct LiveSoakConfig {
    pub(super) username: String,
    pub(super) password: String,
    pub(super) server_address: String,
    pub(super) listener_bind: String,
    pub(super) advertised_port: u16,
    pub(super) obfuscated_listener_bind: Option<String>,
    pub(super) obfuscated_advertised_port: Option<u16>,
    pub(super) duration: Duration,
    pub(super) max_events: usize,
    pub(super) ping_interval: Duration,
    pub(super) search_interval: Duration,
    pub(super) active_probes: bool,
    pub(super) peer_username: Option<String>,
    pub(super) search_query: Option<String>,
    pub(super) search_token: u32,
    pub(super) shared_folders: u32,
    pub(super) shared_files: u32,
    pub(super) watchdog_interval: Duration,
    pub(super) watchdog_stale_seconds: u64,
}

impl LiveSoakConfig {
    pub(super) fn from_env() -> Result<Self, String> {
        let listen_port = env_u16("SLSK_LISTEN_PORT", DEFAULT_LISTEN_PORT as u16)?;
        let advertised_port = env_u16("SLSK_SOAK_ADVERTISED_PORT", listen_port)?;
        Ok(Self {
            username: required_env_any(&["SLSK_USERNAME"])?,
            password: required_env_any(&["SLSK_PASSWORD"])?,
            server_address: std::env::var("SLSK_SERVER")
                .unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned()),
            listener_bind: std::env::var("SLSK_SOAK_LISTENER_BIND")
                .unwrap_or_else(|_| format!("0.0.0.0:{listen_port}")),
            advertised_port,
            obfuscated_listener_bind: optional_env("SLSK_SOAK_OBFUSCATED_LISTENER_BIND"),
            obfuscated_advertised_port: optional_env("SLSK_SOAK_OBFUSCATED_ADVERTISED_PORT")
                .map(|value| {
                    value.parse::<u16>().map_err(|error| {
                        format!("invalid SLSK_SOAK_OBFUSCATED_ADVERTISED_PORT: {error}")
                    })
                })
                .transpose()?,
            duration: env_duration_secs("SLSK_SOAK_SECONDS", 60, false)?,
            max_events: env_usize("SLSK_SOAK_MAX_EVENTS", 40)?,
            ping_interval: env_duration_secs("SLSK_SOAK_PING_SECONDS", 30, false)?,
            search_interval: env_duration_secs("SLSK_SOAK_SEARCH_INTERVAL_SECONDS", 900, false)?,
            active_probes: env_bool("SLSK_SOAK_ACTIVE_PROBES", true)?,
            peer_username: optional_env("SLSK_SOAK_PEER_USERNAME"),
            search_query: optional_env("SLSK_SOAK_SEARCH_QUERY").or_else(|| {
                env_bool("SLSK_SOAK_DEFAULT_SEARCH", true)
                    .ok()
                    .filter(|enabled| *enabled)
                    .map(|_| "commons".to_owned())
            }),
            search_token: env_u32("SLSK_SOAK_SEARCH_TOKEN", 1_000_001)?,
            shared_folders: env_u32("SLSK_SOAK_SHARED_FOLDERS", 0)?,
            shared_files: env_u32("SLSK_SOAK_SHARED_FILES", 0)?,
            watchdog_interval: env_duration_secs("SLSK_SOAK_WATCHDOG_SECONDS", 120, false)?,
            watchdog_stale_seconds: env_u64("SLSK_SOAK_WATCHDOG_STALE_SECONDS", 240)?,
        })
    }
}
