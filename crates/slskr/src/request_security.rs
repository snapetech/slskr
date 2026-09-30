use super::*;

pub(super) fn rate_limit_user_key(authenticated_token: &str) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(authenticated_token.as_bytes());
    format!("auth:{}", hex::encode(&digest[..16]))
}

pub(super) fn authenticated_rate_limit_user_key(
    config: &AppConfig,
    authorization: Option<&str>,
    cookie: Option<&str>,
    remote_addr: Option<SocketAddr>,
) -> Option<String> {
    if !utils::is_authorized_from(config, authorization, cookie, remote_addr) {
        return None;
    }
    utils::api_authorization_token(authorization)
        .map(str::to_owned)
        .or_else(|| cookie_session_token(cookie))
        .map(|token| rate_limit_user_key(&token))
}

pub(super) fn rate_limit_remote_addr(
    config: &AppConfig,
    remote_addr: Option<SocketAddr>,
    headers: &http_server::HttpHeaders,
) -> Option<SocketAddr> {
    let remote_addr = remote_addr?;
    if !config
        .trusted_proxy_cidrs
        .iter()
        .any(|cidr| cidr.contains(remote_addr.ip()))
    {
        return Some(remote_addr);
    }

    forwarded_client_ip(config, remote_addr.ip(), headers)
        .map(|ip| SocketAddr::new(ip, 0))
        .or(Some(remote_addr))
}

fn forwarded_client_ip(
    config: &AppConfig,
    remote_ip: IpAddr,
    headers: &http_server::HttpHeaders,
) -> Option<IpAddr> {
    let forwarded_ips = if let Some(value) = headers.forwarded.as_deref() {
        forwarded_header_client_ips(value)?
    } else {
        let value = headers.x_forwarded_for.as_deref()?;
        x_forwarded_for_client_ips(value)?
    };

    forwarded_ips
        .into_iter()
        .chain(std::iter::once(remote_ip))
        .rev()
        .find(|ip| {
            !config
                .trusted_proxy_cidrs
                .iter()
                .any(|cidr| cidr.contains(*ip))
        })
}

fn x_forwarded_for_client_ips(value: &str) -> Option<Vec<IpAddr>> {
    let ips = value
        .split(',')
        .map(parse_forwarded_ip_token)
        .collect::<Option<Vec<_>>>()?;
    (!ips.is_empty()).then_some(ips)
}

fn forwarded_header_client_ips(value: &str) -> Option<Vec<IpAddr>> {
    let ips = value
        .split(',')
        .map(parse_forwarded_element_ip)
        .collect::<Option<Vec<_>>>()?;
    (!ips.is_empty()).then_some(ips)
}

pub(super) fn parse_forwarded_element_ip(entry: &str) -> Option<IpAddr> {
    let mut forwarded_ip = None;
    for part in entry.split(';') {
        let (name, value) = part.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case("for") {
            continue;
        }
        if forwarded_ip.is_some() {
            return None;
        }
        forwarded_ip = Some(parse_forwarded_ip_token(value)?);
    }
    forwarded_ip
}

pub(super) fn parse_forwarded_ip_token(value: &str) -> Option<IpAddr> {
    let value = value.trim();
    let value = match (value.strip_prefix('"'), value.strip_suffix('"')) {
        (Some(without_prefix), Some(_)) => without_prefix.strip_suffix('"')?,
        (None, None) => value,
        _ => return None,
    };
    if value.is_empty() || value.contains(['"', '\\']) {
        return None;
    }
    if value.eq_ignore_ascii_case("unknown") || value.starts_with('_') {
        return None;
    }
    if let Some(bracketed) = value.strip_prefix('[') {
        let (host, suffix) = bracketed.split_once(']')?;
        if !suffix.is_empty()
            && suffix
                .strip_prefix(':')
                .and_then(|port| port.parse::<u16>().ok())
                .is_none()
        {
            return None;
        }
        return host.parse::<std::net::Ipv6Addr>().ok().map(IpAddr::V6);
    }
    if value.contains(['[', ']']) {
        return None;
    }
    if let Ok(ip) = value.parse::<IpAddr>() {
        return Some(ip);
    }
    let (host, port) = value.rsplit_once(':')?;
    port.parse::<u16>().ok()?;
    host.parse::<std::net::Ipv4Addr>().ok().map(IpAddr::V4)
}

pub(super) fn websocket_auth_protocol(protocol_header: Option<&str>) -> Option<&str> {
    protocol_header?
        .split(',')
        .map(str::trim)
        .find(|protocol| decode_websocket_auth_protocol(protocol).is_some())
}

fn websocket_auth_protocol_count(protocol_header: Option<&str>) -> usize {
    protocol_header
        .into_iter()
        .flat_map(|header| header.split(','))
        .map(str::trim)
        .filter(|protocol| decode_websocket_auth_protocol(protocol).is_some())
        .take(2)
        .count()
}

pub(super) fn websocket_protocol_authorization(protocol_header: Option<&str>) -> Option<String> {
    let protocol = websocket_auth_protocol(protocol_header)?;
    let token = decode_websocket_auth_protocol(protocol)?;
    Some(format!("Bearer {token}"))
}

pub(super) fn mixed_websocket_auth_credentials(
    headers: &http_server::HttpHeaders,
    websocket_authorization: Option<&str>,
) -> bool {
    websocket_auth_protocol_count(headers.sec_websocket_protocol.as_deref()) > 1
        || (websocket_authorization.is_some()
            && (headers.authorization.is_some() || headers.x_api_key.is_some()))
}

fn decode_websocket_auth_protocol(protocol: &str) -> Option<String> {
    if !is_websocket_protocol_token(protocol) {
        return None;
    }
    let encoded = protocol.strip_prefix(WEBSOCKET_AUTH_PROTOCOL_PREFIX)?;
    let token = strict_percent_decode(encoded)?;
    (!token.is_empty()
        && !token
            .chars()
            .any(|character| character.is_control() || character == '\u{7f}'))
    .then_some(token)
}

fn strict_percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = strict_hex_value(*bytes.get(index + 1)?)?;
            let low = strict_hex_value(*bytes.get(index + 2)?)?;
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn strict_hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn is_websocket_protocol_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

pub(super) fn controller_cors_headers(
    config: &AppConfig,
    headers: &http_server::HttpHeaders,
    fallback_host: &str,
    preflight: bool,
) -> String {
    if config.controller_profile == ControllerProfile::Legacy {
        let security_headers = RequestSecurityHeaders::from_http_headers(headers);
        let Some(origin) = security_headers.origin.as_deref() else {
            return String::new();
        };
        return if request_origin_matches_host(&security_headers, fallback_host) {
            cors_headers(Some(origin), &[origin])
        } else {
            String::new()
        };
    }

    let settings = &config.controller_web_cors;
    if !settings.enabled || settings.allowed_origins.is_empty() {
        return String::new();
    }
    let Some(origin) = headers
        .origin
        .as_deref()
        .filter(|value| valid_cors_header_value(value))
    else {
        return String::new();
    };
    let allow_any_origin = !settings.allow_credentials
        && settings
            .allowed_origins
            .iter()
            .any(|candidate| matches!(candidate.as_str(), "*" | "/*"));
    if !allow_any_origin
        && !settings
            .allowed_origins
            .iter()
            .any(|candidate| candidate == origin)
    {
        return String::new();
    }

    let mut output = String::new();
    if settings.allow_credentials {
        output.push_str("Access-Control-Allow-Credentials: true\r\n");
    }
    output.push_str("Access-Control-Allow-Origin: ");
    output.push_str(if allow_any_origin { "*" } else { origin });
    output.push_str("\r\n");

    if preflight {
        let allowed_headers = if settings.allowed_headers.is_empty() {
            normalized_cors_list(headers.access_control_request_headers.as_deref())
        } else {
            valid_cors_values(&settings.allowed_headers).then(|| settings.allowed_headers.join(","))
        };
        if let Some(allowed_headers) = allowed_headers.filter(|value| !value.is_empty()) {
            output.push_str("Access-Control-Allow-Headers: ");
            output.push_str(&allowed_headers);
            output.push_str("\r\n");
        }
        let allowed_methods = if settings.allowed_methods.is_empty() {
            normalized_cors_list(headers.access_control_request_method.as_deref())
        } else {
            valid_cors_values(&settings.allowed_methods).then(|| settings.allowed_methods.join(","))
        };
        if let Some(allowed_methods) = allowed_methods.filter(|value| !value.is_empty()) {
            output.push_str("Access-Control-Allow-Methods: ");
            output.push_str(&allowed_methods);
            output.push_str("\r\n");
        }
        output.push_str("Access-Control-Max-Age: 3600\r\n");
    } else {
        output.push_str("Access-Control-Expose-Headers: X-URL-Base,X-Total-Count\r\n");
    }
    if !allow_any_origin && settings.allowed_origins.len() > 1 {
        output.push_str("Vary: Origin\r\n");
    }
    output
}

fn valid_cors_header_value(value: &str) -> bool {
    !value
        .bytes()
        .any(|byte| matches!(byte, b'\r' | b'\n' | 0x00..=0x1f | 0x7f))
}

fn valid_cors_values(values: &[String]) -> bool {
    values.iter().all(|value| valid_cors_header_value(value))
}

fn normalized_cors_list(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if !valid_cors_header_value(value) {
        return None;
    }
    Some(
        value
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(","),
    )
}

pub(super) async fn request_uses_revoked_jwt(
    state: &AppState,
    authorization: Option<&str>,
) -> bool {
    let Some(token) = utils::bearer_authorization_token(authorization) else {
        return false;
    };
    let now = unix_timestamp();
    let Some(claims) = utils::verify_admin_jwt(&state.config, token, now) else {
        return false;
    };
    state.revoked_jwts.write().await.contains(&claims.jti, now)
}

pub(super) fn request_share_token(
    authorization: Option<&str>,
    headers: &RequestSecurityHeaders,
) -> Option<String> {
    if let Some(token) = headers
        .x_share_token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        return Some(token.to_owned());
    }

    let authorization = authorization?.trim();
    let (scheme, value) = authorization.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let (prefix, token) = value.trim().split_once(':')?;
    let token = token.trim();
    (prefix.eq_ignore_ascii_case("share") && !token.is_empty()).then(|| token.to_owned())
}
