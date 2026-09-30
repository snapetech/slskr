use super::*;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiagnosticsFileConfig {
    pub(in crate::config) allow_memory_dump: Option<bool>,
    pub(in crate::config) allow_remote_dump: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebFileConfig {
    pub(in crate::config) socket: Option<PathBuf>,
    pub(in crate::config) url_base: Option<String>,
    pub(in crate::config) content_path: Option<PathBuf>,
    pub(in crate::config) logging: Option<bool>,
    pub(in crate::config) https: HttpsFileConfig,
    pub(in crate::config) enforce_security: Option<bool>,
    pub(in crate::config) allow_remote_no_auth: Option<bool>,
    pub(in crate::config) passthrough_allowed_cidrs: Option<String>,
    pub(in crate::config) max_request_body_size: Option<i64>,
    pub(in crate::config) cors: WebCorsFileConfig,
    pub(in crate::config) rate_limiting: WebRateLimitingFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpsFileConfig {
    pub(in crate::config) disabled: Option<bool>,
    pub(in crate::config) port: Option<u16>,
    pub(in crate::config) ip_address: Option<String>,
    pub(in crate::config) force: Option<bool>,
    pub(in crate::config) certificate: HttpsCertificateFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpsCertificateFileConfig {
    pub(in crate::config) pfx: Option<PathBuf>,
    pub(in crate::config) password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebCorsFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) allow_credentials: Option<bool>,
    pub(in crate::config) allowed_origins: Vec<String>,
    pub(in crate::config) allowed_headers: Vec<String>,
    pub(in crate::config) allowed_methods: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebRateLimitingFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) api_permit_limit: Option<i32>,
    pub(in crate::config) api_window_seconds: Option<i32>,
    pub(in crate::config) federation_permit_limit: Option<i32>,
    pub(in crate::config) federation_window_seconds: Option<i32>,
    pub(in crate::config) mesh_gateway_permit_limit: Option<i32>,
    pub(in crate::config) mesh_gateway_window_seconds: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsFileConfig {
    pub(in crate::config) enabled: Option<bool>,
    pub(in crate::config) url: Option<String>,
    pub(in crate::config) authentication: MetricsAuthenticationFileConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsAuthenticationFileConfig {
    pub(in crate::config) disabled: Option<bool>,
    pub(in crate::config) username: Option<String>,
    pub(in crate::config) password: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthFileConfig {
    pub(in crate::config) disabled: Option<bool>,
    pub(in crate::config) username: Option<String>,
    pub(in crate::config) password: Option<String>,
    pub(in crate::config) jwt: AuthJwtFileConfig,
    pub(in crate::config) api_token: Option<String>,
    pub(in crate::config) read_write_token: Option<String>,
    pub(in crate::config) read_only_token: Option<String>,
    pub(in crate::config) nowplaying_token: Option<String>,
    pub(in crate::config) cookie_auth_enabled: Option<bool>,
    pub(in crate::config) rate_limit_anonymous: Option<u32>,
    pub(in crate::config) rate_limit_authenticated: Option<u32>,
    pub(in crate::config) trusted_proxy_cidrs: Vec<String>,
    pub(in crate::config) api_keys: BTreeMap<String, ControllerApiKeyFileConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControllerApiKeyFileConfig {
    pub(in crate::config) key: String,
    pub(in crate::config) role: String,
    pub(in crate::config) cidr: String,
}

impl Default for ControllerApiKeyFileConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
            role: "readonly".to_owned(),
            cidr: String::new(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthJwtFileConfig {
    pub(in crate::config) key: Option<String>,
    pub(in crate::config) ttl: Option<u64>,
}
