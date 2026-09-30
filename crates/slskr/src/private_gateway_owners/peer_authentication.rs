use super::*;

pub(super) fn gateway_peer_identity(username: &str) -> String {
    username.to_ascii_lowercase()
}

pub(super) fn gateway_replay_nonce_key(username: &str, nonce: &str) -> (String, String) {
    (gateway_peer_identity(username), nonce.to_owned())
}

pub(super) fn valid_open_tunnel_request(request: &OpenTunnelRequest) -> bool {
    !request.pod_id.trim().is_empty()
        && request.pod_id.len() <= MAX_POD_ID_BYTES
        && !request.destination_host.trim().is_empty()
        && request.destination_host.len() <= MAX_DESTINATION_HOST_BYTES
        && request.destination_port != 0
        && request.service_name.as_ref().is_none_or(|service| {
            !service.trim().is_empty() && service.len() <= MAX_SERVICE_NAME_BYTES
        })
        && !request.request_nonce.trim().is_empty()
        && request.request_nonce.len() <= MAX_REQUEST_NONCE_BYTES
}

pub(super) async fn resolve_destination(host: &str, port: u16) -> Result<SocketAddr, String> {
    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
        .await
        .map_err(|_| "Destination resolution timed out".to_owned())?
        .map_err(|_| "Destination resolution failed".to_owned())?;
    addresses
        .find(|address| valid_destination_ip(address.ip()))
        .ok_or_else(|| "Destination did not resolve to a usable address".to_owned())
}

pub(super) async fn resolve_public_relay_destination(
    host: &str,
    port: u16,
) -> Result<SocketAddr, String> {
    let mut addresses = timeout(DESTINATION_RESOLVE_TIMEOUT, lookup_host((host, port)))
        .await
        .map_err(|_| "Relay destination resolution timed out".to_owned())?
        .map_err(|_| "Relay destination resolution failed".to_owned())?;
    addresses
        .find(|address| valid_public_relay_ip(address.ip()))
        .ok_or_else(|| "Relay destination is not public".to_owned())
}

pub(super) async fn authenticate_overlay_peer(
    state: &crate::AppState,
    hello: &MeshHello,
    remote_ip: IpAddr,
    gateway_certificate_sha256: &[u8; 32],
) -> Result<(), String> {
    if state
        .security
        .read()
        .await
        .is_blocked("ip", &remote_ip.to_string())
        || state
            .security
            .read()
            .await
            .is_blocked("username", &hello.username)
    {
        return Err("overlay peer is blocklisted".to_owned());
    }
    let public_key = state
        .mesh
        .read()
        .await
        .capability_records
        .iter()
        .find(|record| {
            record.username.eq_ignore_ascii_case(&hello.username)
                && record.expires_at_unix > crate::unix_timestamp()
        })
        .map(|record| record.public_key)
        .ok_or_else(|| "overlay peer has no fresh authenticated capability record".to_owned())?;
    verify_overlay_peer_authentication(hello, &public_key, gateway_certificate_sha256)?;
    let expected = crate::request_peer_endpoint(state, &hello.username)
        .await
        .map_err(|_| "overlay peer Soulseek endpoint is unavailable".to_owned())?;
    if remote_ip != IpAddr::V4(expected.ip) {
        return Err("overlay peer IP does not match its Soulseek endpoint".to_owned());
    }
    Ok(())
}

pub(super) fn verify_overlay_peer_authentication(
    hello: &MeshHello,
    expected_public_key: &[u8; 32],
    gateway_certificate_sha256: &[u8; 32],
) -> Result<(), String> {
    hello
        .verify_authentication(expected_public_key, gateway_certificate_sha256)
        .map_err(|_| "overlay peer failed capability-key authentication".to_owned())
}

pub(super) fn valid_destination_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && !address.is_broadcast()
                && (address.is_private() || address.is_loopback() || address.is_link_local())
        }
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && (address.is_unique_local()
                    || address.is_loopback()
                    || address.is_unicast_link_local())
        }
    }
}

pub(super) fn valid_public_relay_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => !crate::utils::is_blocked_outbound_ipv4(address),
        IpAddr::V6(address) => !crate::utils::is_blocked_outbound_ipv6(address),
    }
}
