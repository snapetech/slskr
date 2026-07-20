use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use slskr_client::listener::IncomingConnection;
use slskr_client::protocol::peer::PeerMessage;
use slskr_client::protocol::server::ServerMessage;
use tokio::net::TcpStream;

use crate::config::{AppConfig, ControllerCompatibilityTarget};

pub(crate) fn is_blocked_outbound_ipv4(ip: Ipv4Addr) -> bool {
    let address = u32::from(ip);
    let in_cidr = |network: Ipv4Addr, prefix: u32| {
        address >> (32 - prefix) == u32::from(network) >> (32 - prefix)
    };
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.octets()[0] == 0
        || ip.octets()[0] >= 224
        || in_cidr(Ipv4Addr::new(100, 64, 0, 0), 10)
        || in_cidr(Ipv4Addr::new(192, 0, 0, 0), 24)
        || in_cidr(Ipv4Addr::new(192, 88, 99, 0), 24)
        || in_cidr(Ipv4Addr::new(198, 18, 0, 0), 15)
}

pub(crate) fn nat64_embedded_ipv4(ip: std::net::Ipv6Addr) -> Option<Ipv4Addr> {
    let segments = ip.segments();
    (segments[..6] == [0x0064, 0xff9b, 0, 0, 0, 0]).then(|| {
        let octets = ip.octets();
        Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15])
    })
}

pub(crate) fn is_non_global_special_use_ipv6(ip: std::net::Ipv6Addr) -> bool {
    let segments = ip.segments();
    (segments[0] == 0x0100 && segments[1..4] == [0, 0, 0])
        || segments[..3] == [0x0064, 0xff9b, 0x0001]
        || segments[..3] == [0x2001, 0x0002, 0]
        || (segments[0] == 0x2001 && matches!(segments[1] & 0xfff0, 0x0010 | 0x0020))
}

// ============================================================================
// HTTP Request Parsing
// ============================================================================

pub fn parse_route(request: &str) -> (&str, &str) {
    let mut parts = request
        .lines()
        .next()
        .unwrap_or("GET / HTTP/1.1")
        .split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    (method, path)
}

pub fn split_request_target(target: &str) -> (&str, Option<&str>) {
    match target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (target, None),
    }
}

pub fn authorization_header(request: &str) -> Option<&str> {
    request.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("authorization")
            .then_some(value.trim())
    })
}

#[derive(Clone, Debug, Default)]
pub struct RequestSecurityHeaders {
    pub host: Option<String>,
    pub origin: Option<String>,
    pub referer: Option<String>,
    pub cookie: Option<String>,
    pub x_share_token: Option<String>,
    pub remote_addr: Option<SocketAddr>,
}

impl RequestSecurityHeaders {
    pub fn from_http_headers(h: &crate::http_server::HttpHeaders) -> Self {
        Self {
            host: h.host.clone(),
            origin: h.origin.clone(),
            referer: h.referer.clone(),
            cookie: h.cookie.clone(),
            x_share_token: h.x_share_token.clone(),
            remote_addr: None,
        }
    }
}

pub fn request_body(request: &str) -> &str {
    request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default()
}

pub fn route_requires_auth(config: &AppConfig, path: &str) -> bool {
    let path = normalize_api_path(path);
    config.auth_required
        && path.starts_with("/api/")
        && path != "/api/health"
        && path != "/api/version"
        && path != "/api/capabilities"
        && path != "/api/session/enabled"
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApiAccess {
    Authenticated,
    ReadWrite,
    Administrator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ApiAuthScheme {
    Any,
    Jwt,
    ApiKey,
}

#[derive(Debug, Deserialize)]
struct ControllerAuthRule {
    method: String,
    route: String,
    access: String,
    scheme: String,
    scopes: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct RouteAuthPolicy<'a> {
    access: Option<ApiAccess>,
    scheme: ApiAuthScheme,
    scopes: &'a [String],
}

static SLSKDN_CONTROLLER_AUTH_RULES: LazyLock<Vec<ControllerAuthRule>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../data/slskdn-controller-auth-policy.json"))
        .expect("checked slskdN controller auth policy registry")
});

static SLSKD_CONTROLLER_AUTH_RULES: LazyLock<Vec<ControllerAuthRule>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../data/slskd-controller-auth-policy.json"))
        .expect("checked slskd controller auth policy registry")
});

fn route_template_matches(template: &str, path: &str) -> bool {
    let template = template.trim_end_matches('/');
    let path = path.trim_end_matches('/');
    let mut expected = template.trim_start_matches('/').split('/');
    let mut actual = path.trim_start_matches('/').split('/');
    loop {
        match (expected.next(), actual.next()) {
            (None, None) => return true,
            (Some(segment), Some(_)) if segment.starts_with("{*") && segment.ends_with('}') => {
                return true;
            }
            (Some(segment), Some(_)) if segment.starts_with('{') && segment.ends_with('}') => {}
            (Some(segment), Some(value)) if segment == value => {}
            _ => return false,
        }
    }
}

fn route_template_precedence(template: &str) -> Vec<u8> {
    template
        .trim_matches('/')
        .split('/')
        .map(|segment| {
            if segment.starts_with("{*") && segment.ends_with('}') {
                1
            } else if segment.starts_with('{') && segment.ends_with('}') {
                if segment.contains(':') {
                    3
                } else {
                    2
                }
            } else {
                4
            }
        })
        .collect()
}

fn controller_route_auth_policy(
    target: ControllerCompatibilityTarget,
    method: &str,
    path: &str,
) -> Option<RouteAuthPolicy<'static>> {
    let rules = match target {
        ControllerCompatibilityTarget::Slskd => &*SLSKD_CONTROLLER_AUTH_RULES,
        ControllerCompatibilityTarget::Slskdn => &*SLSKDN_CONTROLLER_AUTH_RULES,
    };
    let rule = rules
        .iter()
        .filter(|rule| rule.method == method && route_template_matches(&rule.route, path))
        .max_by_key(|rule| route_template_precedence(&rule.route))?;
    let access = match rule.access.as_str() {
        "anonymous" | "delegated" => None,
        "administrator" => Some(ApiAccess::Administrator),
        "read_write" => Some(ApiAccess::ReadWrite),
        _ => Some(ApiAccess::Authenticated),
    };
    let scheme = match rule.scheme.as_str() {
        "jwt" => ApiAuthScheme::Jwt,
        "api_key" => ApiAuthScheme::ApiKey,
        _ => ApiAuthScheme::Any,
    };
    Some(RouteAuthPolicy {
        access,
        scheme,
        scopes: &rule.scopes,
    })
}

#[derive(Clone, Copy, Debug)]
struct ApiCredential {
    access: ApiAccess,
    scheme: ApiAuthScheme,
    nowplaying_only: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AdminJwtClaims {
    pub jti: String,
    pub name: String,
    pub role: String,
    pub scope: String,
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
    pub iss: String,
    pub aud: String,
}

pub(crate) fn issue_admin_jwt(
    config: &AppConfig,
    username: &str,
    now: u64,
) -> Option<(String, AdminJwtClaims)> {
    let secret = config.controller_web_jwt_key.as_str();
    let claims = AdminJwtClaims {
        jti: uuid::Uuid::new_v4().simple().to_string(),
        name: username.to_owned(),
        role: "Administrator".to_owned(),
        scope: "*".to_owned(),
        iat: now,
        nbf: now,
        exp: now.saturating_add(config.controller_web_jwt_ttl_millis / 1000),
        iss: "slskd".to_owned(),
        aud: "slskd".to_owned(),
    };
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).ok()?);
    let signing_input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(signing_input.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Some((format!("{signing_input}.{signature}"), claims))
}

pub(crate) fn verify_admin_jwt(
    config: &AppConfig,
    token: &str,
    now: u64,
) -> Option<AdminJwtClaims> {
    let secret = config.controller_web_jwt_key.as_str();
    let mut parts = token.split('.');
    let header = parts.next()?;
    let payload = parts.next()?;
    let signature = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let decoded_header = URL_SAFE_NO_PAD.decode(header).ok()?;
    let decoded_header = serde_json::from_slice::<serde_json::Value>(&decoded_header).ok()?;
    if decoded_header
        .get("alg")
        .and_then(serde_json::Value::as_str)
        != Some("HS256")
    {
        return None;
    }
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(format!("{header}.{payload}").as_bytes());
    mac.verify_slice(&signature).ok()?;
    let claims =
        serde_json::from_slice::<AdminJwtClaims>(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    if claims.role != "Administrator"
        || claims.scope != "*"
        || claims.iss != "slskd"
        || claims.aud != "slskd"
        || claims.jti.is_empty()
        || claims.name.trim().is_empty()
        || claims.nbf > now
        || claims.exp <= now
        || claims.iat > now
    {
        return None;
    }
    Some(claims)
}

fn api_credential(
    config: &AppConfig,
    authorization: Option<&str>,
    cookie: Option<&str>,
    remote_addr: Option<SocketAddr>,
) -> Option<ApiCredential> {
    if let Some(value) = authorization {
        let (scheme, token) = value
            .strip_prefix("Bearer ")
            .map(|token| (ApiAuthScheme::Jwt, token))
            .or_else(|| {
                value
                    .strip_prefix("ApiKey ")
                    .map(|token| (ApiAuthScheme::ApiKey, token))
            })?;
        let matches = |expected: Option<&str>| {
            expected.is_some_and(|expected| constant_time_eq(token.as_bytes(), expected.as_bytes()))
        };
        if scheme == ApiAuthScheme::ApiKey {
            if let Some(configured) = config.controller_api_keys.values().find(|configured| {
                constant_time_eq(token.as_bytes(), configured.key.as_bytes())
                    && remote_addr.is_some_and(|remote| {
                        configured
                            .cidrs
                            .iter()
                            .any(|cidr| cidr.contains(remote.ip()))
                    })
            }) {
                let access = match configured.role.as_str() {
                    "readonly" => ApiAccess::Authenticated,
                    "readwrite" => ApiAccess::ReadWrite,
                    "administrator" => ApiAccess::Administrator,
                    _ => return None,
                };
                return Some(ApiCredential {
                    access,
                    scheme,
                    nowplaying_only: false,
                });
            }
        }
        return if matches(config.api_token.as_deref()) {
            Some(ApiCredential {
                access: ApiAccess::Administrator,
                scheme,
                nowplaying_only: false,
            })
        } else if matches(config.api_read_write_token.as_deref()) {
            Some(ApiCredential {
                access: ApiAccess::ReadWrite,
                scheme,
                nowplaying_only: false,
            })
        } else if matches(config.api_read_only_token.as_deref()) {
            Some(ApiCredential {
                access: ApiAccess::Authenticated,
                scheme,
                nowplaying_only: false,
            })
        } else if matches(config.api_nowplaying_token.as_deref()) {
            Some(ApiCredential {
                access: ApiAccess::ReadWrite,
                scheme,
                nowplaying_only: true,
            })
        } else if scheme == ApiAuthScheme::Jwt
            && verify_admin_jwt(config, token, unix_timestamp()).is_some()
        {
            Some(ApiCredential {
                access: ApiAccess::Administrator,
                scheme,
                nowplaying_only: false,
            })
        } else {
            None
        };
    }
    if config.api_cookie_auth_enabled {
        let token = parse_cookie_session_token(cookie).ok().flatten()?;
        if config
            .api_token
            .as_deref()
            .is_some_and(|expected| constant_time_eq(token.as_bytes(), expected.as_bytes()))
        {
            return Some(ApiCredential {
                access: ApiAccess::Administrator,
                scheme: ApiAuthScheme::Jwt,
                nowplaying_only: false,
            });
        }
    }
    None
}

pub fn authorize_controller_route(
    config: &AppConfig,
    method: &str,
    path: &str,
    authorization: Option<&str>,
    cookie: Option<&str>,
) -> Result<(), &'static str> {
    authorize_controller_route_from(config, method, path, authorization, cookie, None)
}

pub fn authorize_controller_route_from(
    config: &AppConfig,
    method: &str,
    path: &str,
    authorization: Option<&str>,
    cookie: Option<&str>,
    remote_addr: Option<SocketAddr>,
) -> Result<(), &'static str> {
    if !config.auth_required {
        return Ok(());
    }
    let policy = controller_route_auth_policy(config.controller_compatibility_target, method, path)
        .unwrap_or_else(|| {
            let normalized = normalize_api_path(path);
            let public = !path.starts_with("/api/")
                || matches!(
                    normalized,
                    "/api/health" | "/api/version" | "/api/capabilities" | "/api/session/enabled"
                );
            RouteAuthPolicy {
                access: (!public).then_some(ApiAccess::Administrator),
                scheme: ApiAuthScheme::Any,
                scopes: &[],
            }
        });
    let Some(required_access) = policy.access else {
        return Ok(());
    };
    let credential =
        api_credential(config, authorization, cookie, remote_addr).ok_or("unauthorized")?;
    if credential.access < required_access {
        return Err("forbidden");
    }
    if policy.scheme != ApiAuthScheme::Any && credential.scheme != policy.scheme {
        return Err("forbidden");
    }
    let requires_nowplaying = policy.scopes.iter().any(|scope| scope == "nowplaying");
    if credential.nowplaying_only != requires_nowplaying && credential.nowplaying_only {
        return Err("forbidden");
    }
    Ok(())
}

pub fn controller_route_requires_principal(config: &AppConfig, method: &str, path: &str) -> bool {
    controller_route_auth_policy(config.controller_compatibility_target, method, path).map_or_else(
        || {
            let normalized = normalize_api_path(path);
            path.starts_with("/api/")
                && !matches!(
                    normalized,
                    "/api/health" | "/api/version" | "/api/capabilities" | "/api/session/enabled"
                )
        },
        |policy| policy.access.is_some(),
    )
}

pub fn csrf_origin_allowed(
    config: &AppConfig,
    method: &str,
    path: &str,
    headers: &RequestSecurityHeaders,
) -> bool {
    if !config.auth_required || !is_unsafe_http_method(method) || !path.starts_with("/api/") {
        return true;
    }
    let Some(source) = headers.origin.as_deref().or(headers.referer.as_deref()) else {
        return !config.api_cookie_auth_enabled
            || cookie_session_token(headers.cookie.as_deref()).is_none();
    };
    let fallback_host = config.http_bind.to_string();
    let expected_host = headers.host.as_deref().unwrap_or(fallback_host.as_str());
    origin_matches_host(source, expected_host)
}

pub fn is_unsafe_http_method(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
}

pub fn origin_host(value: &str) -> Option<&str> {
    let (scheme, rest) = value.split_once("://")?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    parse_authority(authority)?;
    Some(authority)
}

pub fn same_origin_host(left: &str, right: &str) -> bool {
    matches!(
        (parse_authority(left), parse_authority(right)),
        (Some(left), Some(right)) if left == right
    )
}

pub fn request_origin_matches_host(headers: &RequestSecurityHeaders, fallback_host: &str) -> bool {
    let Some(origin) = headers.origin.as_deref() else {
        return true;
    };
    let expected_host = headers.host.as_deref().unwrap_or(fallback_host);
    origin_matches_host(origin, expected_host)
}

fn origin_matches_host(origin: &str, expected_host: &str) -> bool {
    let Some((scheme, _)) = origin.split_once("://") else {
        return false;
    };
    let Some(origin_authority) = origin_host(origin).and_then(parse_authority) else {
        return false;
    };
    let Some(expected_authority) = parse_authority(expected_host) else {
        return false;
    };
    let default_port = if scheme.eq_ignore_ascii_case("http") {
        80
    } else if scheme.eq_ignore_ascii_case("https") {
        443
    } else {
        return false;
    };
    origin_authority.0 == expected_authority.0
        && origin_authority.1.unwrap_or(default_port)
            == expected_authority.1.unwrap_or(default_port)
}

pub(crate) fn parse_authority(value: &str) -> Option<(String, Option<u16>)> {
    if value.is_empty() || value.trim() != value || value.contains('@') {
        return None;
    }
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, suffix) = rest.split_once(']')?;
        host.parse::<std::net::Ipv6Addr>().ok()?;
        let port = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':')?.parse::<u16>().ok()?)
        };
        (host.to_ascii_lowercase(), port)
    } else {
        let (host, port) = match value.rsplit_once(':') {
            Some((host, port)) => (host, Some(port.parse::<u16>().ok()?)),
            None => (value, None),
        };
        if host.is_empty()
            || !host
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        {
            return None;
        }
        (host.trim_end_matches('.').to_ascii_lowercase(), port)
    };
    (!host.is_empty()).then_some((host, port))
}

pub fn is_authorized(
    config: &AppConfig,
    authorization: Option<&str>,
    cookie: Option<&str>,
) -> bool {
    api_credential(config, authorization, cookie, None).is_some()
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();
    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }
    diff == 0
}

pub(crate) fn cookie_session_token(cookie: Option<&str>) -> Option<String> {
    parse_cookie_session_token(cookie).ok().flatten()
}

fn parse_cookie_session_token(cookie: Option<&str>) -> Result<Option<String>, ()> {
    let mut session_token = None;
    let Some(cookie) = cookie else {
        return Ok(None);
    };
    for part in cookie.split(';') {
        let Some((name, value)) = part.trim().split_once('=') else {
            continue;
        };
        if name != "slskr.session" {
            continue;
        }
        if session_token.is_some() {
            return Err(());
        }
        session_token = Some(strict_percent_decode_component(value.trim()).ok_or(())?);
    }
    Ok(session_token)
}

fn strict_percent_decode_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

pub fn percent_decode_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

// ============================================================================
// API Path Normalization
// ============================================================================

pub fn normalize_api_path(path: &str) -> &str {
    if path.starts_with("/api/v0/files/") {
        return path;
    }
    if path == "/api/v0/searches/prune" {
        return "/api/searches/prune";
    }
    if path.starts_with("/api/v0/searches/") {
        return path;
    }
    if path.starts_with("/api/v0/users/") && path != "/api/v0/users/watch" {
        return path;
    }
    if path.starts_with("/api/v0/messages/") && path != "/api/v0/messages/inbound" {
        return path;
    }
    if path.starts_with("/api/v0/rooms/") && path != "/api/v0/rooms/refresh" {
        return path;
    }
    match path {
        "/api/v0/health" => "/api/health",
        "/api/v0/version" => "/api/version",
        "/api/v0/capabilities" => "/api/capabilities",
        "/api/v0/capabilities/negotiate" => "/api/capabilities/negotiate",
        "/api/v0/config" => "/api/config",
        "/api/v0/stats" => "/api/stats",
        "/api/v0/metrics" => "/api/metrics",
        "/api/v0/telemetry" => "/api/telemetry",
        "/api/v0/events" => "/api/events",
        "/api/v0/shares" => "/api/shares",
        "/api/v0/shares/catalog" => "/api/shares/catalog",
        "/api/v0/shares/rescan" => "/api/shares/rescan",
        "/api/v0/searches" => "/api/searches",
        "/api/v0/search-responses" => "/api/search-responses",
        "/api/v0/browse" => "/api/browse",
        "/api/v0/browse-responses" => "/api/browse-responses",
        "/api/v0/session" => "/api/session",
        "/api/v0/session/enabled" => "/api/session/enabled",
        "/api/v0/session/connect" => "/api/session/connect",
        "/api/v0/session/disconnect" => "/api/session/disconnect",
        "/api/v0/session/ping" => "/api/session/ping",
        "/api/v0/session/privileges/check" => "/api/session/privileges/check",
        "/api/v0/listeners" => "/api/listeners",
        "/api/v0/users" => "/api/users",
        "/api/v0/users/watch" => "/api/users/watch",
        "/api/v0/messages" => "/api/messages",
        "/api/v0/messages/inbound" => "/api/messages/inbound",
        "/api/v0/rooms" => "/api/rooms",
        "/api/v0/rooms/refresh" => "/api/rooms/refresh",
        "/api/v0/transfers" => "/api/transfers",
        "/api/v0/transfers/stats" => "/api/transfers/stats",
        "/api/info" => "/api/application",
        "/api/downloads" => "/api/transfers/downloads",
        "/api/server/status" => "/api/server",
        "/api/slskdn/capabilities" | "/api/v0/slskdn/capabilities" => "/api/capabilities",
        "/api/v0/capabilities/mesh-peers" => "/api/capabilities/peers",
        "/api/v0/fairness/summary" => "/api/fairness",
        "/api/v0/hashdb/backfill/candidates" => "/api/backfill/candidates",
        "/api/v0/transfers/downloads/auto-replace/status" => "/api/autoreplace",
        "/api/v0/portforwarding/available-ports" => "/api/port-forwarding/available-ports",
        "/api/v0/portforwarding/stream-stats" => "/api/port-forwarding/stream-stats",
        "/api/v0/portforwarding/start" => "/api/portforwarding/start",
        _ => path,
    }
}

pub fn search_token_path(path: &str, suffix: &str) -> Option<u32> {
    let token = path
        .strip_prefix("/api/searches/")
        .or_else(|| path.strip_prefix("/api/v0/searches/"))?
        .strip_suffix(suffix)?;
    if token.contains('/') {
        return None;
    }
    token.parse().ok()
}

pub fn files_root_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/files/")
        .or_else(|| path.strip_prefix("/api/v0/files/"))
        .filter(|root| !root.is_empty() && !root.contains('/'))
}

pub fn transfer_action_path(path: &str) -> Option<(u64, &str)> {
    let rest = path
        .strip_prefix("/api/transfers/")
        .or_else(|| path.strip_prefix("/api/v0/transfers/"))?;
    let (id, action) = rest.split_once('/')?;
    if action.contains('/') {
        return None;
    }
    Some((id.parse().ok()?, action))
}

pub fn user_watch_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/watch")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn user_browse_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/browse")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn user_browse_request_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/browse/request")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn user_browse_folder_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/browse/folder")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn user_browse_fail_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/browse/fail")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn user_browse_cancel_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/browse/cancel")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn user_stats_request_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/users/")
        .or_else(|| path.strip_prefix("/api/v0/users/"))?
        .strip_suffix("/stats/request")
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn messages_user_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/messages/")
        .or_else(|| path.strip_prefix("/api/v0/messages/"))
        .filter(|username| !username.is_empty() && !username.contains('/'))
}

pub fn message_ack_path(path: &str) -> Option<u64> {
    let id = path
        .strip_prefix("/api/messages/")
        .or_else(|| path.strip_prefix("/api/v0/messages/"))?
        .strip_suffix("/ack")?;
    if id.contains('/') {
        return None;
    }
    id.parse().ok()
}

pub fn room_join_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/rooms/")
        .or_else(|| path.strip_prefix("/api/v0/rooms/"))?
        .strip_suffix("/join")
        .filter(|room| !room.is_empty() && !room.contains('/'))
}

pub fn room_messages_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/rooms/")
        .or_else(|| path.strip_prefix("/api/v0/rooms/"))?
        .strip_suffix("/messages")
        .filter(|room| !room.is_empty() && !room.contains('/'))
}

pub fn room_path(path: &str) -> Option<&str> {
    path.strip_prefix("/api/rooms/")
        .or_else(|| path.strip_prefix("/api/v0/rooms/"))?
        .split('/')
        .next()
        .filter(|room| !room.is_empty())
}

pub fn message_id_path(path: &str) -> Option<u64> {
    let rest = path
        .strip_prefix("/api/messages/")
        .or_else(|| path.strip_prefix("/api/v0/messages/"))?;
    let id = rest.split('/').next()?;
    if id.is_empty() || id.contains('/') {
        return None;
    }
    id.parse().ok()
}

// ============================================================================
// JSON Parsing
// ============================================================================

pub fn extract_json_string_field(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(ToOwned::to_owned)
}

pub fn extract_json_string_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()?
        .get(field)?
        .as_array()?
        .iter()
        .map(|value| value.as_str().map(ToOwned::to_owned))
        .collect()
}

pub struct JsonStringPrefix {
    pub value: String,
    pub consumed: usize,
}

pub fn parse_json_string_prefix(value: &str) -> Option<JsonStringPrefix> {
    let mut chars = value.char_indices();
    let (_, first) = chars.next()?;
    if first != '"' {
        return None;
    }
    let mut output = String::new();
    let mut escaped = false;
    for (index, character) in chars {
        if escaped {
            match character {
                '"' => output.push('"'),
                '\\' => output.push('\\'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                other => output.push(other),
            }
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => {
                return Some(JsonStringPrefix {
                    value: output,
                    consumed: index + 1,
                })
            }
            other => output.push(other),
        }
    }
    None
}

pub fn json_field_after_key<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    body.match_indices(key).find_map(|(index, _)| {
        let after_key = &body[index + key.len()..];
        after_key.trim_start().starts_with(':').then_some(after_key)
    })
}

// ============================================================================
// Query Parameter Parsing
// ============================================================================

pub fn query_params(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            (percent_decode(name), percent_decode(value))
        })
        .collect()
}

pub fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                    if let Ok(byte) = u8::from_str_radix(hex, 16) {
                        output.push(byte);
                        index += 3;
                        continue;
                    }
                }
                output.push(bytes[index]);
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}

pub fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

pub fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

// ============================================================================
// Transfer Status Checks
// ============================================================================

pub fn is_active_transfer_status(status: &str) -> bool {
    matches!(
        status,
        "in_progress" | "peer_lookup" | "peer_negotiating" | "accepted" | "indirect_pending"
    )
}

pub fn is_terminal_transfer_status(status: &str) -> bool {
    matches!(status, "succeeded" | "cancelled" | "failed" | "rejected")
}

// ============================================================================
// Incoming Connection Handling
// ============================================================================

pub fn bump_incoming_counter(
    snapshot: &mut super::ListenerSnapshot,
    incoming: &IncomingConnection<TcpStream>,
) {
    match incoming {
        IncomingConnection::PeerMessages(_) => snapshot.peer_messages += 1,
        IncomingConnection::ObfuscatedPeerMessages(_) => snapshot.obfuscated_peer_messages += 1,
        IncomingConnection::FileTransfer(_) => snapshot.file_transfers += 1,
        IncomingConnection::Distributed(_) => snapshot.distributed += 1,
        IncomingConnection::PeerInit { .. } => snapshot.peer_inits += 1,
        IncomingConnection::PierceFirewall { .. } => snapshot.pierce_firewalls += 1,
        IncomingConnection::UnknownInit { .. } => snapshot.unknown_inits += 1,
    }
}

pub fn incoming_connection_name(incoming: &IncomingConnection<TcpStream>) -> &'static str {
    match incoming {
        IncomingConnection::PeerMessages(_) => "peer_messages",
        IncomingConnection::ObfuscatedPeerMessages(_) => "obfuscated_peer_messages",
        IncomingConnection::FileTransfer(_) => "file_transfer",
        IncomingConnection::Distributed(_) => "distributed",
        IncomingConnection::PeerInit { .. } => "peer_init",
        IncomingConnection::PierceFirewall { .. } => "pierce_firewall",
        IncomingConnection::UnknownInit { .. } => "unknown_init",
    }
}

// ============================================================================
// Socket and Message Naming
// ============================================================================

pub fn scrub_socket_addr(address: SocketAddr) -> String {
    format!(
        "{}:{}",
        if address.is_ipv4() { "ipv4" } else { "ipv6" },
        address.port()
    )
}

pub fn peer_message_name(message: &PeerMessage) -> &'static str {
    match message {
        PeerMessage::PrivateMessage(_) => "PrivateMessage",
        PeerMessage::GetShareFileList => "GetShareFileList",
        PeerMessage::SharedFileListResponse(_) => "SharedFileListResponse",
        PeerMessage::FileSearchRequest { .. } => "FileSearchRequest",
        PeerMessage::FileSearchResponse(_) => "FileSearchResponse",
        PeerMessage::RoomInvitation(_) => "RoomInvitation",
        PeerMessage::CancelledQueuedTransfer(_) => "CancelledQueuedTransfer",
        PeerMessage::UserInfoRequest => "UserInfoRequest",
        PeerMessage::UserInfoResponse(_) => "UserInfoResponse",
        PeerMessage::SendConnectToken(_) => "SendConnectToken",
        PeerMessage::MoveDownloadToTop(_) => "MoveDownloadToTop",
        PeerMessage::FolderContentsRequest(_) => "FolderContentsRequest",
        PeerMessage::FolderContentsResponse(_) => "FolderContentsResponse",
        PeerMessage::TransferRequest(_) => "TransferRequest",
        PeerMessage::TransferResponse(_) => "TransferResponse",
        PeerMessage::PlaceholdUpload { .. } => "PlaceholdUpload",
        PeerMessage::QueueUpload { .. } => "QueueUpload",
        PeerMessage::PlaceInQueueResponse { .. } => "PlaceInQueueResponse",
        PeerMessage::UploadFailed { .. } => "UploadFailed",
        PeerMessage::ExactFileSearchRequest(_) => "ExactFileSearchRequest",
        PeerMessage::QueuedDownloads(_) => "QueuedDownloads",
        PeerMessage::IndirectFileSearchRequest(_) => "IndirectFileSearchRequest",
        PeerMessage::UploadDenied { .. } => "UploadDenied",
        PeerMessage::PlaceInQueueRequest { .. } => "PlaceInQueueRequest",
        PeerMessage::UploadQueueNotification => "UploadQueueNotification",
        PeerMessage::Unknown { .. } => "Unknown",
    }
}

pub fn server_message_name(message: &ServerMessage) -> &'static str {
    match message {
        ServerMessage::LoginRequest(_) => "LoginRequest",
        ServerMessage::LoginResponse(_) => "LoginResponse",
        ServerMessage::SetWaitPort(_) => "SetWaitPort",
        ServerMessage::GetPeerAddressRequest { .. } => "GetPeerAddressRequest",
        ServerMessage::GetPeerAddressResponse(_) => "GetPeerAddressResponse",
        ServerMessage::WatchUserRequest { .. } => "WatchUserRequest",
        ServerMessage::WatchUserResponse(_) => "WatchUserResponse",
        ServerMessage::UnwatchUser { .. } => "UnwatchUser",
        ServerMessage::GetUserStatusRequest { .. } => "GetUserStatusRequest",
        ServerMessage::GetUserStatusResponse(_) => "GetUserStatusResponse",
        ServerMessage::IgnoreUser { .. } => "IgnoreUser",
        ServerMessage::UnignoreUser { .. } => "UnignoreUser",
        ServerMessage::SayChatroomRequest { .. } => "SayChatroomRequest",
        ServerMessage::SayChatroomResponse { .. } => "SayChatroomResponse",
        ServerMessage::ConnectToPeerRequest(_) => "ConnectToPeerRequest",
        ServerMessage::ConnectToPeerResponse(_) => "ConnectToPeerResponse",
        ServerMessage::MessageUserRequest { .. } => "MessageUserRequest",
        ServerMessage::MessageUserResponse(_) => "MessageUserResponse",
        ServerMessage::MessageAcked { .. } => "MessageAcked",
        ServerMessage::FileSearchRequest(_) => "FileSearchRequest",
        ServerMessage::FileSearchIncoming { .. } => "FileSearchIncoming",
        ServerMessage::JoinRoom { .. } => "JoinRoom",
        ServerMessage::JoinedRoom(_) => "JoinedRoom",
        ServerMessage::LeaveRoom { .. } => "LeaveRoom",
        ServerMessage::SetStatus { .. } => "SetStatus",
        ServerMessage::ServerPing => "ServerPing",
        ServerMessage::SharedFoldersFiles { .. } => "SharedFoldersFiles",
        ServerMessage::GetUserStatsRequest { .. } => "GetUserStatsRequest",
        ServerMessage::GetUserStats { .. } => "GetUserStats",
        ServerMessage::Relogged => "Relogged",
        ServerMessage::RoomList(_) => "RoomList",
        ServerMessage::ExcludedSearchPhrases(_) => "ExcludedSearchPhrases",
        ServerMessage::Unknown { .. } => "Unknown",
        _ => "Unknown",
    }
}

// ============================================================================
// Timestamp
// ============================================================================

pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub fn unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// Determine cache control header for response
pub fn cache_control_header(method: &str, content_type: &str, path: &str) -> Option<String> {
    // Only cache GET requests
    if method != "GET" {
        return None;
    }

    // Don't cache HTML (dynamic dashboard)
    if content_type.contains("text/html") {
        return Some("Cache-Control: no-cache, must-revalidate\r\n".to_string());
    }

    // Only cache endpoints that are intentionally public without auth.
    if path == "/api/health" || path == "/api/version" || path == "/api/capabilities" {
        return Some("Cache-Control: public, max-age=3600\r\n".to_string()); // 1 hour
    }

    // Protected API responses can include local state, share metadata, transfer
    // status, messages, telemetry, and sanitized-but-sensitive config.
    Some("Cache-Control: no-store\r\n".to_string())
}

/// Generate ETag for response body
pub fn generate_etag(body: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(body.as_bytes());
    format!("\"{}\"", hex::encode(&digest[..16]))
}

/// Generate CORS headers for API responses
pub fn cors_headers(origin: Option<&str>, allowed_origins: &[&str]) -> String {
    let origin = match origin {
        Some(o) if !valid_header_value(o) => return String::new(),
        Some(o) if allowed_origins.contains(&"*") => o.trim(),
        Some(o) if allowed_origins.contains(&o) => o.trim(),
        Some(_) => return String::new(), // Origin not allowed
        None => return String::new(),    // No origin header
    };

    format!(
        "Access-Control-Allow-Origin: {}\r\n\
         Vary: Origin\r\n\
         Access-Control-Allow-Methods: GET, POST, PUT, DELETE, PATCH, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type, Authorization, X-API-Key, X-Share-Token\r\n\
         Access-Control-Max-Age: 86400\r\n",
        origin
    )
}

fn valid_header_value(value: &str) -> bool {
    !value
        .bytes()
        .any(|byte| matches!(byte, b'\r' | b'\n' | 0x00..=0x1f | 0x7f))
}

/// Check if request is OPTIONS preflight
pub fn is_cors_preflight(method: &str) -> bool {
    method == "OPTIONS"
}

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("req-{nanos:x}-{counter:x}")
}

/// Standard error codes for API responses
#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    // Client errors (4xx)
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    RateLimited = 429,

    // Server errors (5xx)
    InternalError = 500,
    ServiceUnavailable = 503,
}

impl ErrorCode {
    pub fn status_text(self) -> &'static str {
        match self {
            Self::BadRequest => "400 Bad Request",
            Self::Unauthorized => "401 Unauthorized",
            Self::Forbidden => "403 Forbidden",
            Self::NotFound => "404 Not Found",
            Self::RateLimited => "429 Too Many Requests",
            Self::InternalError => "500 Internal Server Error",
            Self::ServiceUnavailable => "503 Service Unavailable",
        }
    }

    pub fn code_string(self) -> &'static str {
        match self {
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::RateLimited => "RATE_LIMITED",
            Self::InternalError => "INTERNAL_ERROR",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
        }
    }
}

/// Format error response with code and message
pub fn error_response_json(code: ErrorCode, message: &str) -> String {
    format!(
        "{{\"error\":\"{}\",\"code\":\"{}\",\"message\":\"{}\"}}",
        code.code_string(),
        code.code_string(),
        crate::config::json_escape(message)
    )
}

/// Escape JSON string values
pub fn json_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            ch if ch <= '\u{1f}' => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod controller_auth_tests {
    use std::collections::{BTreeMap, HashSet};

    use super::*;
    use crate::config::{ConfigEnv, ControllerApiKeySettings, FileConfig, TrustedProxyCidr};

    #[derive(Default)]
    struct TestEnv(BTreeMap<String, String>);

    impl TestEnv {
        fn with(mut self, name: &str, value: &str) -> Self {
            self.0.insert(name.to_owned(), value.to_owned());
            self
        }
    }

    impl ConfigEnv for TestEnv {
        fn var(&self, name: &str) -> Option<String> {
            self.0.get(name).cloned()
        }
    }

    fn config(target: &str) -> AppConfig {
        AppConfig::from_layers(
            None,
            FileConfig::default(),
            &TestEnv::default()
                .with("SLSKR_CONTROLLER_COMPATIBILITY_TARGET", target)
                .with("SLSKR_API_TOKEN", "admin-token")
                .with("SLSKR_API_READ_WRITE_TOKEN", "write-token")
                .with("SLSKR_API_READ_ONLY_TOKEN", "read-token")
                .with("SLSKR_API_NOWPLAYING_TOKEN", "nowplaying-token"),
        )
        .expect("controller auth test config")
    }

    fn permitted_credential(rule: &ControllerAuthRule) -> Option<String> {
        if matches!(rule.access.as_str(), "anonymous" | "delegated") {
            return None;
        }
        let token = if rule.scopes.iter().any(|scope| scope == "nowplaying") {
            "nowplaying-token"
        } else {
            match rule.access.as_str() {
                "administrator" => "admin-token",
                "read_write" => "write-token",
                _ => "read-token",
            }
        };
        let scheme = if rule.scheme == "api_key" {
            "ApiKey"
        } else {
            "Bearer"
        };
        Some(format!("{scheme} {token}"))
    }

    fn assert_registry(target: &str, rules: &[ControllerAuthRule], expected: usize) {
        assert_eq!(rules.len(), expected, "{target} route count drift");
        let config = config(target);
        let mut keys = HashSet::new();
        for rule in rules {
            let key = format!("{} {}", rule.method, rule.route);
            assert!(keys.insert(key.clone()), "duplicate auth rule {key}");

            let absent = authorize_controller_route(&config, &rule.method, &rule.route, None, None);
            if matches!(rule.access.as_str(), "anonymous" | "delegated") {
                assert!(absent.is_ok(), "{target} {key} must be anonymous/delegated");
                continue;
            }
            assert_eq!(absent, Err("unauthorized"), "{target} {key}");

            let permitted = permitted_credential(rule).expect("protected route credential");
            assert!(
                authorize_controller_route(
                    &config,
                    &rule.method,
                    &rule.route,
                    Some(&permitted),
                    None,
                )
                .is_ok(),
                "{target} {key} rejected {permitted}"
            );

            let insufficient = match rule.access.as_str() {
                "administrator" => Some(if rule.scheme == "api_key" {
                    "ApiKey write-token"
                } else {
                    "Bearer write-token"
                }),
                "read_write" if !rule.scopes.iter().any(|scope| scope == "nowplaying") => {
                    Some(if rule.scheme == "api_key" {
                        "ApiKey read-token"
                    } else {
                        "Bearer read-token"
                    })
                }
                _ => None,
            };
            if let Some(insufficient) = insufficient {
                assert_eq!(
                    authorize_controller_route(
                        &config,
                        &rule.method,
                        &rule.route,
                        Some(insufficient),
                        None,
                    ),
                    Err("forbidden"),
                    "{target} {key} accepted insufficient access"
                );
            }

            let wrong_scheme = match rule.scheme.as_str() {
                "jwt" => Some("ApiKey admin-token"),
                "api_key" => Some("Bearer admin-token"),
                _ => None,
            };
            if let Some(wrong_scheme) = wrong_scheme {
                assert_eq!(
                    authorize_controller_route(
                        &config,
                        &rule.method,
                        &rule.route,
                        Some(wrong_scheme),
                        None,
                    ),
                    Err("forbidden"),
                    "{target} {key} accepted the wrong auth scheme"
                );
            }
        }
    }

    #[test]
    fn frozen_slskd_and_slskdn_auth_registries_are_exhaustively_enforced() {
        assert_registry("slskd", &SLSKD_CONTROLLER_AUTH_RULES, 91);
        assert_registry("slskdn", &SLSKDN_CONTROLLER_AUTH_RULES, 678);
    }

    #[test]
    fn configured_controller_api_keys_enforce_role_and_cidr() {
        let mut config = config("slskdn");
        config.controller_api_keys.insert(
            "operator".to_owned(),
            ControllerApiKeySettings {
                key: "0123456789abcdef".to_owned(),
                role: "readwrite".to_owned(),
                cidr: "127.0.0.1/32".to_owned(),
                cidrs: vec![TrustedProxyCidr::parse("127.0.0.1/32").unwrap()],
            },
        );
        let loopback = Some("127.0.0.1:1234".parse().unwrap());
        let remote = Some("192.0.2.1:1234".parse().unwrap());
        let credential = api_credential(&config, Some("ApiKey 0123456789abcdef"), None, loopback)
            .expect("configured API key must authenticate from its CIDR");
        assert_eq!(credential.access, ApiAccess::ReadWrite);
        assert!(api_credential(&config, Some("ApiKey 0123456789abcdef"), None, remote,).is_none());
        assert!(
            api_credential(&config, Some("ApiKey wrong-wrong-wrong"), None, loopback,).is_none()
        );
    }
}
