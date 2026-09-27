use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

use crate::utils::{is_blocked_outbound_ipv4, is_blocked_outbound_ipv6};

pub(super) struct ResolvedIntegrationTarget {
    pub(super) host: String,
    pub(super) addrs: Vec<SocketAddr>,
}

#[allow(dead_code)]
pub(super) fn validate_integration_base_url(
    base_url: &str,
) -> Result<ResolvedIntegrationTarget, String> {
    let parsed = reqwest::Url::parse(base_url)
        .map_err(|error| format!("integration URL is invalid: {error}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Err("integration URL scheme must be http or https".to_owned()),
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "integration URL must include a host".to_owned())?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("integration URL must not contain embedded credentials".to_owned());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("integration URL must not contain a query or fragment".to_owned());
    }
    let allow_private = matches!(
        std::env::var("SLSKR_ALLOW_PRIVATE_INTEGRATION_URLS")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    );
    if allow_private {
        let port = parsed
            .port_or_known_default()
            .ok_or_else(|| "integration URL port is unknown".to_owned())?;
        return Ok(ResolvedIntegrationTarget {
            host: host.to_owned(),
            addrs: (host, port)
                .to_socket_addrs()
                .map_err(|error| format!("integration URL resolution failed: {error}"))?
                .collect(),
        });
    }
    if host.eq_ignore_ascii_case("localhost") {
        return Err("integration URL host is private".to_owned());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_integration_ip(ip) {
            return Err("integration URL IP is private".to_owned());
        }
    }
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "integration URL port is unknown".to_owned())?;
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("integration URL resolution failed: {error}"))?
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err("integration URL did not resolve".to_owned());
    }
    if addrs
        .iter()
        .any(|addr| is_blocked_integration_ip(addr.ip()))
    {
        return Err("integration URL resolves to a private address".to_owned());
    }
    Ok(ResolvedIntegrationTarget {
        host: host.to_owned(),
        addrs,
    })
}

pub(super) fn validate_lidarr_base_url(
    base_url: &str,
) -> Result<ResolvedIntegrationTarget, String> {
    let parsed = reqwest::Url::parse(base_url)
        .map_err(|error| format!("integration URL is invalid: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("integration URL scheme must be http or https".to_owned());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "integration URL must include a host".to_owned())?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("integration URL must not contain embedded credentials".to_owned());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("integration URL must not contain a query or fragment".to_owned());
    }
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "integration URL port is unknown".to_owned())?;
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("integration URL resolution failed: {error}"))?
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err("integration URL did not resolve".to_owned());
    }
    if addrs
        .iter()
        .any(|address| address.ip().is_unspecified() || address.ip().is_multicast())
    {
        return Err("integration URL resolves to an unusable address".to_owned());
    }
    Ok(ResolvedIntegrationTarget {
        host: host.to_owned(),
        addrs,
    })
}

#[allow(dead_code)]
pub(super) fn is_blocked_integration_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_blocked_outbound_ipv4(ip),
        IpAddr::V6(ip) => is_blocked_outbound_ipv6(ip),
    }
}
