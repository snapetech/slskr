use super::*;

pub(super) fn random_controller_jwt_key() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| format!("failed to generate controller JWT signing key: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) fn validate_api_token(token: &str) -> Result<(), String> {
    if token.trim().is_empty() {
        return Err("HTTP API token must not be empty or whitespace-only".to_owned());
    }
    if token != token.trim() {
        return Err("HTTP API token must not have surrounding whitespace".to_owned());
    }
    if token.chars().any(char::is_control) {
        return Err("HTTP API token must not contain control characters".to_owned());
    }
    if token.len() > crate::http_server::MAX_API_TOKEN_BYTES {
        return Err(format!(
            "HTTP API token exceeds the maximum representable header length of {} bytes",
            crate::http_server::MAX_API_TOKEN_BYTES
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedProxyCidr {
    pub(super) network: IpAddr,
    pub(super) prefix: u8,
}

impl TrustedProxyCidr {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (addr, prefix) = value
            .split_once('/')
            .ok_or_else(|| format!("trusted proxy CIDR {value:?} must include a prefix length"))?;
        let network = addr.parse::<IpAddr>().map_err(|error| {
            format!("trusted proxy CIDR {value:?} has invalid address: {error}")
        })?;
        let prefix = prefix
            .parse::<u8>()
            .map_err(|error| format!("trusted proxy CIDR {value:?} has invalid prefix: {error}"))?;
        let max_prefix = match network {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if prefix > max_prefix {
            return Err(format!(
                "trusted proxy CIDR {value:?} prefix exceeds {max_prefix}"
            ));
        }
        Ok(Self { network, prefix })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(network), IpAddr::V4(ip)) => {
                let network = u32::from(network);
                let ip = u32::from(ip);
                self.prefix == 0 || network >> (32 - self.prefix) == ip >> (32 - self.prefix)
            }
            (IpAddr::V6(network), IpAddr::V6(ip)) => {
                let network = u128::from_be_bytes(network.octets());
                let ip = u128::from_be_bytes(ip.octets());
                self.prefix == 0 || network >> (128 - self.prefix) == ip >> (128 - self.prefix)
            }
            _ => false,
        }
    }
}

pub(super) fn trusted_proxy_cidrs_from_layers(
    env_value: Option<String>,
    file_value: Vec<String>,
) -> Result<Vec<TrustedProxyCidr>, String> {
    let values = match env_value {
        Some(value) => value
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        None => file_value,
    };
    values
        .into_iter()
        .map(|value| TrustedProxyCidr::parse(&value))
        .collect()
}

pub(super) fn controller_passthrough_cidrs(value: Option<&str>) -> Vec<TrustedProxyCidr> {
    value
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter_map(|value| {
            let normalized = if value.contains('/') {
                value.to_owned()
            } else if value.contains(':') {
                format!("{value}/128")
            } else {
                format!("{value}/32")
            };
            TrustedProxyCidr::parse(&normalized).ok()
        })
        .collect()
}

pub(super) struct ControllerWebAuthSettings {
    pub(super) controller_metrics_enabled: bool,
    pub(super) controller_metrics_url: String,
    pub(super) controller_metrics_auth_disabled: bool,
    pub(super) controller_metrics_username: String,
    pub(super) controller_metrics_password: String,
    pub(super) controller_web_auth_username: String,
    pub(super) controller_web_auth_password: String,
    pub(super) controller_web_jwt_key: String,
    pub(super) controller_web_jwt_key_configured: bool,
    pub(super) controller_web_jwt_ttl_millis: u64,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_controller_web_auth<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    auth_required: bool,
    metrics_enabled: Option<bool>,
    metrics_url: Option<String>,
    metrics_auth_disabled: Option<bool>,
    metrics_username: Option<String>,
    metrics_password: Option<String>,
    web_auth_username: Option<String>,
    web_auth_password: Option<String>,
    web_jwt_key: Option<String>,
    web_jwt_ttl: Option<u64>,
) -> Result<ControllerWebAuthSettings, String> {
    let controller_metrics_enabled =
        env_bool_layer(env, "SLSKD_METRICS", metrics_enabled.unwrap_or(false))?;
    let controller_metrics_url = env
        .var("SLSKD_METRICS_URL")
        .or(metrics_url)
        .unwrap_or_else(|| "/metrics".to_owned());
    let controller_metrics_auth_disabled = env_bool_layer(
        env,
        "SLSKD_METRICS_NO_AUTH",
        metrics_auth_disabled.unwrap_or(false),
    )?;
    let default_identity = match controller_profile {
        ControllerProfile::Legacy => "slskd",
        ControllerProfile::Native => "slskr",
    };
    let controller_metrics_username = env
        .var("SLSKD_METRICS_USERNAME")
        .or(metrics_username)
        .unwrap_or_else(|| default_identity.to_owned());
    let controller_metrics_password = env
        .var("SLSKD_METRICS_PASSWORD")
        .or(metrics_password)
        .unwrap_or_default();
    let controller_web_auth_username = env
        .var("SLSKD_USERNAME")
        .or(web_auth_username)
        .unwrap_or_else(|| default_identity.to_owned());
    let configured_web_auth_password = env.var("SLSKD_PASSWORD").or(web_auth_password);
    let controller_web_auth_password = configured_web_auth_password.clone().unwrap_or_else(|| {
        if auth_required {
            // Preserve the profile's generated-config default when no
            // password layer overrides it. No-auth mode keeps the credential
            // empty because it is not used.
            default_identity.to_owned()
        } else {
            String::new()
        }
    });
    let mut web_auth_fields = vec![("username", controller_web_auth_username.as_str())];
    if auth_required && configured_web_auth_password.is_some() {
        web_auth_fields.push(("password", controller_web_auth_password.as_str()));
    }
    for (field, value) in web_auth_fields {
        let length = value.encode_utf16().count();
        if !(1..=255).contains(&length) {
            return Err(format!(
                "web authentication {field} must contain between 1 and 255 characters"
            ));
        }
    }
    let controller_web_jwt_key = env.var("SLSKD_JWT_KEY").or(web_jwt_key);
    let controller_web_jwt_key_configured = controller_web_jwt_key.is_some();
    let controller_web_jwt_key = controller_web_jwt_key
        .map(Ok)
        .unwrap_or_else(random_controller_jwt_key)?;
    if !(32..=255).contains(&controller_web_jwt_key.encode_utf16().count()) {
        return Err(
            "web authentication JWT key must contain between 32 and 255 characters".to_owned(),
        );
    }
    let controller_web_jwt_ttl_millis = env_parse_layer(
        env,
        "SLSKD_JWT_TTL",
        web_jwt_ttl,
        if controller_profile == ControllerProfile::Legacy {
            604_800_000_u64
        } else {
            3_600_000_u64
        },
    )?;
    if controller_web_jwt_ttl_millis < 3_600 {
        return Err("web authentication JWT TTL must be at least 3600 milliseconds".to_owned());
    }
    let metrics_auth_requires_credentials =
        controller_metrics_enabled && !controller_metrics_auth_disabled;
    if metrics_auth_requires_credentials {
        for (field, value) in [
            ("username", controller_metrics_username.as_str()),
            ("password", controller_metrics_password.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!(
                    "metrics authentication {field} must be configured when metrics auth is enabled"
                ));
            }
            let length = value.encode_utf16().count();
            if !(1..=255).contains(&length) {
                return Err(format!(
                    "metrics authentication {field} must contain between 1 and 255 characters"
                ));
            }
        }
    }
    Ok(ControllerWebAuthSettings {
        controller_metrics_enabled,
        controller_metrics_url,
        controller_metrics_auth_disabled,
        controller_metrics_username,
        controller_metrics_password,
        controller_web_auth_username,
        controller_web_auth_password,
        controller_web_jwt_key,
        controller_web_jwt_key_configured,
        controller_web_jwt_ttl_millis,
    })
}

pub(super) fn resolve_controller_api_keys<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    auth_api_keys: BTreeMap<String, ControllerApiKeyFileConfig>,
) -> Result<BTreeMap<String, ControllerApiKeySettings>, String> {
    let controller_api_key_files = match env.var("SLSKD_API_KEYS_JSON") {
        Some(value) => serde_json::from_str::<BTreeMap<String, ControllerApiKeyFileConfig>>(&value)
            .map_err(|error| format!("invalid web.authentication.api_keys: {error}"))?,
        None => auth_api_keys,
    };
    let mut controller_api_keys = BTreeMap::new();
    for (name, configured) in controller_api_key_files {
        let key_length = configured.key.encode_utf16().count();
        if !(16..=255).contains(&key_length) {
            return Err(format!(
                "web.authentication.api_keys.{name}.key must contain between 16 and 255 characters"
            ));
        }
        let role = configured.role.to_ascii_lowercase();
        if !matches!(role.as_str(), "readonly" | "readwrite" | "administrator") {
            return Err(format!(
                "web.authentication.api_keys.{name}.role must be readonly, readwrite, or administrator"
            ));
        }
        let default_cidr = if controller_profile == ControllerProfile::Native {
            "127.0.0.1/32,::1/128"
        } else {
            "0.0.0.0/0,::/0"
        };
        let cidrs = configured
            .cidr
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .chain(
                configured
                    .cidr
                    .trim()
                    .is_empty()
                    .then_some(default_cidr)
                    .into_iter()
                    .flat_map(|value| value.split(',')),
            )
            .map(TrustedProxyCidr::parse)
            .collect::<Result<Vec<_>, _>>()?;
        controller_api_keys.insert(
            name,
            ControllerApiKeySettings {
                key: configured.key,
                role,
                cidr: if configured.cidr.trim().is_empty() {
                    default_cidr.to_owned()
                } else {
                    configured.cidr
                },
                cidrs,
            },
        );
    }
    Ok(controller_api_keys)
}

pub(super) struct ControllerWebResolution {
    pub(super) http_bind: SocketAddr,
    pub(super) http_binds: Vec<SocketAddr>,
    pub(super) controller_http_address: Option<String>,
    pub(super) controller_web: ControllerWebSettings,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_controller_web<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    app_http_bind: Option<String>,
    web_socket: Option<PathBuf>,
    web_url_base: Option<String>,
    web_content_path: Option<PathBuf>,
    web_logging: Option<bool>,
    web_https: HttpsFileConfig,
) -> Result<ControllerWebResolution, String> {
    let configured_native_http_bind = app_http_bind.as_deref();
    let base_http_bind = configured_native_http_bind
        .unwrap_or("127.0.0.1:5030")
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid configured HTTP bind: {error}"))?;
    let http_port = env_parse_any_layer(env, &["SLSKD_HTTP_PORT"], None, base_http_bind.port())?;
    let (http_binds, controller_http_address) = if let Some(value) = env.var("SLSKR_HTTP_BIND") {
        let address = value
            .parse::<SocketAddr>()
            .map_err(|error| format!("invalid SLSKR_HTTP_BIND: {error}"))?;
        (vec![address], Some(address.ip().to_string()))
    } else {
        match controller_profile {
            ControllerProfile::Legacy => {
                let configured = env.var("SLSKD_HTTP_IP_ADDRESS");
                let ips = match configured.as_deref() {
                    Some(value) if !value.trim().is_empty() => value
                        .split(',')
                        .map(str::trim)
                        .map(|value| {
                            parse_compat_ip_address(value)
                                .map_err(|error| format!("invalid SLSKD_HTTP_IP_ADDRESS: {error}"))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    Some(_) => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
                    None if configured_native_http_bind.is_some() => vec![base_http_bind.ip()],
                    None => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
                };
                (
                    ips.into_iter()
                        .map(|ip| SocketAddr::new(ip, http_port))
                        .collect(),
                    configured,
                )
            }
            ControllerProfile::Native => {
                let configured = env.var("SLSKD_HTTP_ADDRESS");
                let raw = configured
                    .clone()
                    .unwrap_or_else(|| base_http_bind.ip().to_string());
                let ip = if raw == "*" {
                    IpAddr::V4(Ipv4Addr::UNSPECIFIED)
                } else {
                    raw.parse::<IpAddr>()
                        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
                };
                (vec![SocketAddr::new(ip, http_port)], Some(raw))
            }
        }
    };
    let http_bind = *http_binds
        .first()
        .ok_or_else(|| "HTTP bind list must not be empty".to_owned())?;
    let controller_socket = env
        .var("SLSKD_HTTP_SOCKET")
        .map(PathBuf::from)
        .or(web_socket)
        .filter(|path| !path.as_os_str().is_empty());
    if controller_socket
        .as_deref()
        .is_some_and(|path| !path.is_absolute())
    {
        return Err("web.socket must be an absolute path".to_owned());
    }
    let mut controller_url_base = env
        .var("SLSKD_URL_BASE")
        .or(web_url_base)
        .unwrap_or_else(|| "/".to_owned());
    if !controller_url_base.starts_with('/')
        || controller_url_base.contains(['?', '#'])
        || controller_url_base
            .split('/')
            .any(|segment| segment == "..")
    {
        return Err("web.url_base must be an absolute non-traversing URL path".to_owned());
    }
    if controller_url_base.len() > 1 {
        controller_url_base = controller_url_base.trim_end_matches('/').to_owned();
    }
    let configured_content_path = env
        .var("SLSKD_CONTENT_PATH")
        .map(PathBuf::from)
        .or(web_content_path);
    let controller_content_path_raw = configured_content_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("wwwroot"));
    if controller_content_path_raw.as_os_str().is_empty()
        || controller_content_path_raw
            .to_string_lossy()
            .encode_utf16()
            .count()
            > 255
    {
        return Err("web.content_path must contain between 1 and 255 characters".to_owned());
    }
    if configured_content_path.is_some() && controller_content_path_raw.is_absolute() {
        return Err("web.content_path must be relative to the application directory".to_owned());
    }
    let controller_content_path = if controller_content_path_raw.is_absolute() {
        controller_content_path_raw.clone()
    } else {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(&controller_content_path_raw)
    };
    if configured_content_path.is_some() && !controller_content_path.is_dir() {
        return Err(format!(
            "web.content_path directory does not exist: {}",
            controller_content_path.display()
        ));
    }
    let https_disabled =
        env_bool_layer(env, "SLSKD_NO_HTTPS", web_https.disabled.unwrap_or(false))?;
    let https_port = env_parse_layer(env, "SLSKD_HTTPS_PORT", web_https.port, 5031_u16)?;
    let https_configured_ip_address = env.var("SLSKD_HTTPS_IP_ADDRESS").or(web_https.ip_address);
    let https_ips = match controller_profile {
        ControllerProfile::Legacy => match https_configured_ip_address.as_deref() {
            Some(value) if !value.trim().is_empty() => value
                .split(',')
                .map(str::trim)
                .map(parse_compat_ip_address)
                .collect::<Result<Vec<_>, _>>()?,
            _ => vec![IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)],
        },
        ControllerProfile::Native => vec![IpAddr::V4(Ipv4Addr::UNSPECIFIED)],
    };
    let https_certificate_pfx = env
        .var("SLSKD_HTTPS_CERT_PFX")
        .map(PathBuf::from)
        .or(web_https.certificate.pfx)
        .filter(|path| !path.as_os_str().is_empty());
    if https_certificate_pfx
        .as_deref()
        .is_some_and(|path| !path.is_file())
    {
        return Err("web.https.certificate.pfx must identify a readable file".to_owned());
    }
    let controller_web = ControllerWebSettings {
        socket: controller_socket,
        url_base: controller_url_base,
        content_path: controller_content_path,
        content_path_display: controller_content_path_raw.display().to_string(),
        logging: env_bool_layer(env, "SLSKD_HTTP_LOGGING", web_logging.unwrap_or(false))?,
        https: ControllerHttpsSettings {
            disabled: https_disabled,
            binds: https_ips
                .into_iter()
                .map(|ip| SocketAddr::new(ip, https_port))
                .collect(),
            configured_ip_address: https_configured_ip_address,
            force: env_bool_layer(env, "SLSKD_HTTPS_FORCE", web_https.force.unwrap_or(false))?,
            certificate_pfx: https_certificate_pfx,
            certificate_password: env
                .var("SLSKD_HTTPS_CERT_PASSWORD")
                .or(web_https.certificate.password)
                .unwrap_or_default(),
        },
    };
    Ok(ControllerWebResolution {
        http_bind,
        http_binds,
        controller_http_address,
        controller_web,
    })
}

pub(super) struct ApiAndWebHardeningSettings {
    pub(super) api_token: Option<String>,
    pub(super) api_read_write_token: Option<String>,
    pub(super) api_read_only_token: Option<String>,
    pub(super) api_nowplaying_token: Option<String>,
    pub(super) auth_required: bool,
    pub(super) api_cookie_auth_enabled: bool,
    pub(super) api_rate_limit_anonymous: u32,
    pub(super) api_rate_limit_authenticated: u32,
    pub(super) controller_web_max_request_body_size: usize,
    pub(super) controller_web_enforce_security: bool,
    pub(super) controller_web_allow_remote_no_auth: bool,
    pub(super) controller_web_passthrough_allowed_cidrs: Option<String>,
    pub(super) controller_web_passthrough_cidrs: Vec<TrustedProxyCidr>,
    pub(super) controller_diagnostics_allow_memory_dump: bool,
    pub(super) controller_diagnostics_allow_remote_dump: bool,
    pub(super) controller_web_cors: ControllerWebCorsSettings,
    pub(super) controller_web_rate_limiting: ControllerWebRateLimitingSettings,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_api_and_web_hardening<E: ConfigEnv>(
    env: &E,
    controller_profile: ControllerProfile,
    auth_api_token: Option<String>,
    auth_read_write_token: Option<String>,
    auth_read_only_token: Option<String>,
    auth_nowplaying_token: Option<String>,
    auth_disabled: Option<bool>,
    auth_cookie_auth_enabled: Option<bool>,
    auth_rate_limit_anonymous: Option<u32>,
    auth_rate_limit_authenticated: Option<u32>,
    web_max_request_body_size: Option<i64>,
    web_enforce_security: Option<bool>,
    web_allow_remote_no_auth: Option<bool>,
    web_passthrough_allowed_cidrs: Option<String>,
    diagnostics_allow_memory_dump: Option<bool>,
    diagnostics_allow_remote_dump: Option<bool>,
    web_cors: WebCorsFileConfig,
    web_rate_limiting: WebRateLimitingFileConfig,
) -> Result<ApiAndWebHardeningSettings, String> {
    let api_token = env.var("SLSKR_API_TOKEN").or(auth_api_token);
    let api_read_write_token = env
        .var("SLSKR_API_READ_WRITE_TOKEN")
        .or(auth_read_write_token);
    let api_read_only_token = env
        .var("SLSKR_API_READ_ONLY_TOKEN")
        .or(auth_read_only_token);
    let api_nowplaying_token = env
        .var("SLSKR_API_NOWPLAYING_TOKEN")
        .or(auth_nowplaying_token);
    let configured_tokens = [
        api_token.as_deref(),
        api_read_write_token.as_deref(),
        api_read_only_token.as_deref(),
        api_nowplaying_token.as_deref(),
    ];
    for token in configured_tokens.into_iter().flatten() {
        validate_api_token(token)?;
    }
    let token_count = configured_tokens.into_iter().flatten().count();
    let unique_token_count = configured_tokens
        .into_iter()
        .flatten()
        .collect::<std::collections::HashSet<_>>()
        .len();
    if token_count != unique_token_count {
        return Err("API tokens for different roles must be distinct".to_owned());
    }
    let auth_disabled = resolve_auth_disabled(env, auth_disabled.unwrap_or(false))?;
    let auth_required = !auth_disabled;
    let api_cookie_auth_enabled = env_bool_layer(
        env,
        "SLSKR_API_COOKIE_AUTH_ENABLED",
        auth_cookie_auth_enabled.unwrap_or(false),
    )?;
    let api_rate_limit_anonymous = env_parse_layer(
        env,
        "SLSKR_API_RATE_LIMIT_ANONYMOUS",
        auth_rate_limit_anonymous,
        1000_u32,
    )?;
    let api_rate_limit_authenticated = env_parse_layer(
        env,
        "SLSKR_API_RATE_LIMIT_AUTHENTICATED",
        auth_rate_limit_authenticated,
        5000_u32,
    )?;
    let web_max_request_body_size_default = if controller_profile == ControllerProfile::Native {
        10 * 1024 * 1024
    } else {
        crate::http_server::BODY_SIZE_LIMIT as i64
    };
    let controller_web_max_request_body_size = env_parse_layer(
        env,
        "SLSKD_WEB_MAX_REQUEST_BODY_SIZE",
        web_max_request_body_size,
        web_max_request_body_size_default,
    )?;
    if !(1..=i32::MAX as i64).contains(&controller_web_max_request_body_size) {
        return Err("web.max_request_body_size must be between 1 and 2147483647".to_owned());
    }
    let controller_web_max_request_body_size = controller_web_max_request_body_size as usize;
    let controller_web_enforce_security = env_bool_layer(
        env,
        "SLSKD_ENFORCE_SECURITY",
        web_enforce_security.unwrap_or(false),
    )?;
    let controller_web_allow_remote_no_auth = env_bool_layer(
        env,
        "SLSKD_ALLOW_REMOTE_NO_AUTH",
        web_allow_remote_no_auth.unwrap_or(false),
    )?;
    let controller_web_passthrough_allowed_cidrs = env
        .var("SLSKD_PASSTHROUGH_ALLOWED_CIDRS")
        .or(web_passthrough_allowed_cidrs);
    let controller_web_passthrough_cidrs =
        controller_passthrough_cidrs(controller_web_passthrough_allowed_cidrs.as_deref());
    let controller_diagnostics_allow_memory_dump = env_bool_layer(
        env,
        "SLSKD_ALLOW_MEMORY_DUMP",
        diagnostics_allow_memory_dump.unwrap_or(false),
    )?;
    let controller_diagnostics_allow_remote_dump = env_bool_layer(
        env,
        "SLSKD_ALLOW_REMOTE_DUMP",
        diagnostics_allow_remote_dump.unwrap_or(false),
    )?;
    let controller_web_cors = ControllerWebCorsSettings {
        enabled: env_bool_layer(
            env,
            "SLSKD_WEB_CORS_ENABLED",
            web_cors.enabled.unwrap_or(false),
        )?,
        allow_credentials: env_bool_layer(
            env,
            "SLSKD_WEB_CORS_ALLOW_CREDENTIALS",
            web_cors.allow_credentials.unwrap_or(false),
        )?,
        allowed_origins: controller_string_array_layer(
            env,
            "SLSKD_WEB_CORS_ALLOWED_ORIGINS",
            web_cors.allowed_origins,
        ),
        allowed_headers: controller_string_array_layer(
            env,
            "SLSKD_WEB_CORS_ALLOWED_HEADERS",
            web_cors.allowed_headers,
        ),
        allowed_methods: controller_string_array_layer(
            env,
            "SLSKD_WEB_CORS_ALLOWED_METHODS",
            web_cors.allowed_methods,
        ),
    };
    let web_rate_limiting_defaults =
        ControllerWebRateLimitingSettings::defaults(controller_profile);
    let controller_web_rate_limiting = ControllerWebRateLimitingSettings {
        enabled: env_bool_layer(
            env,
            "SLSKD_WEB_RATE_LIMITING",
            web_rate_limiting
                .enabled
                .unwrap_or(web_rate_limiting_defaults.enabled),
        )?,
        api_permit_limit: env_parse_layer(
            env,
            "SLSKD_WEB_API_PERMIT_LIMIT",
            web_rate_limiting.api_permit_limit,
            web_rate_limiting_defaults.api_permit_limit,
        )?,
        api_window_seconds: env_parse_layer(
            env,
            "SLSKD_WEB_API_WINDOW_SECONDS",
            web_rate_limiting.api_window_seconds,
            web_rate_limiting_defaults.api_window_seconds,
        )?,
        federation_permit_limit: env_parse_layer(
            env,
            "SLSKD_WEB_FEDERATION_PERMIT_LIMIT",
            web_rate_limiting.federation_permit_limit,
            web_rate_limiting_defaults.federation_permit_limit,
        )?,
        federation_window_seconds: env_parse_layer(
            env,
            "SLSKD_WEB_FEDERATION_WINDOW_SECONDS",
            web_rate_limiting.federation_window_seconds,
            web_rate_limiting_defaults.federation_window_seconds,
        )?,
        mesh_gateway_permit_limit: env_parse_layer(
            env,
            "SLSKD_WEB_MESH_GATEWAY_PERMIT_LIMIT",
            web_rate_limiting.mesh_gateway_permit_limit,
            web_rate_limiting_defaults.mesh_gateway_permit_limit,
        )?,
        mesh_gateway_window_seconds: env_parse_layer(
            env,
            "SLSKD_WEB_MESH_GATEWAY_WINDOW_SECONDS",
            web_rate_limiting.mesh_gateway_window_seconds,
            web_rate_limiting_defaults.mesh_gateway_window_seconds,
        )?,
    };
    Ok(ApiAndWebHardeningSettings {
        api_token,
        api_read_write_token,
        api_read_only_token,
        api_nowplaying_token,
        auth_required,
        api_cookie_auth_enabled,
        api_rate_limit_anonymous,
        api_rate_limit_authenticated,
        controller_web_max_request_body_size,
        controller_web_enforce_security,
        controller_web_allow_remote_no_auth,
        controller_web_passthrough_allowed_cidrs,
        controller_web_passthrough_cidrs,
        controller_diagnostics_allow_memory_dump,
        controller_diagnostics_allow_remote_dump,
        controller_web_cors,
        controller_web_rate_limiting,
    })
}

pub(super) fn resolve_auth_disabled<E: ConfigEnv>(
    env: &E,
    configured: bool,
) -> Result<bool, String> {
    env_bool_any_layer(env, &["SLSKR_AUTH_DISABLED", "SLSKD_NO_AUTH"], configured)
}
