use super::*;

#[derive(Debug, Clone)]
pub(super) struct PeerSmokeCredentials {
    pub(super) a_username: String,
    pub(super) a_password: String,
    pub(super) b_username: String,
    pub(super) b_password: String,
}

impl PeerSmokeCredentials {
    pub(super) fn from_env() -> Result<Self, String> {
        Ok(Self {
            a_username: required_env_any(&["SLSKR_A_USERNAME", "SLSK_A_USERNAME"])?,
            a_password: required_env_any(&["SLSKR_A_PASSWORD", "SLSK_A_PASSWORD"])?,
            b_username: required_env_any(&["SLSKR_B_USERNAME", "SLSK_B_USERNAME"])?,
            b_password: required_env_any(&["SLSKR_B_PASSWORD", "SLSK_B_PASSWORD"])?,
        })
    }
}

#[derive(Debug, Clone)]
pub(super) struct PeerSmokeConfig {
    pub(super) credentials: PeerSmokeCredentials,
    pub(super) server_address: String,
    pub(super) indirect_listener_bind: String,
    pub(super) indirect_host_override: Option<String>,
    pub(super) indirect_timeout: Duration,
}

impl PeerSmokeConfig {
    pub(super) fn from_env() -> Result<Self, String> {
        Ok(Self {
            credentials: PeerSmokeCredentials::from_env()?,
            server_address: std::env::var("SLSK_SERVER")
                .unwrap_or_else(|_| DEFAULT_SERVER_ADDRESS.to_owned()),
            indirect_listener_bind: std::env::var("SLSKR_INDIRECT_LISTENER_BIND")
                .unwrap_or_else(|_| "0.0.0.0:0".to_owned()),
            indirect_host_override: optional_env("SLSKR_INDIRECT_HOST_OVERRIDE"),
            indirect_timeout: env_duration_secs("SLSKR_INDIRECT_TIMEOUT_SECONDS", 10, false)?,
        })
    }
}
