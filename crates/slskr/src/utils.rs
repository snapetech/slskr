use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use slskr_client::listener::IncomingConnection;
use slskr_client::protocol::peer::PeerMessage;
use slskr_client::protocol::server::ServerMessage;
use tokio::net::TcpStream;

use crate::config::AppConfig;

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
}

impl RequestSecurityHeaders {
    pub fn from_http_headers(h: &crate::http_server::HttpHeaders) -> Self {
        Self {
            host: h.host.clone(),
            origin: h.origin.clone(),
            referer: h.referer.clone(),
            cookie: h.cookie.clone(),
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
    config.auth_required
        && path.starts_with("/api/")
        && path != "/api/health"
        && path != "/api/version"
        && path != "/api/capabilities"
        && path != "/api/session/enabled"
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
    let Some(source_host) = origin_host(source) else {
        return false;
    };
    let fallback_host = config.http_bind.to_string();
    let expected_host = headers.host.as_deref().unwrap_or(fallback_host.as_str());
    same_origin_host(source_host, expected_host)
}

pub fn is_unsafe_http_method(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
}

pub fn origin_host(value: &str) -> Option<&str> {
    let without_scheme = value.split_once("://").map_or(value, |(_, rest)| rest);
    without_scheme
        .split(['/', '?', '#'])
        .next()
        .map(str::trim)
        .filter(|host| {
            !host.is_empty()
                && !host
                    .bytes()
                    .any(|byte| matches!(byte, b'\r' | b'\n' | 0x00..=0x1f | 0x7f))
        })
}

pub fn same_origin_host(left: &str, right: &str) -> bool {
    normalize_origin_host(left) == normalize_origin_host(right)
}

pub fn request_origin_matches_host(headers: &RequestSecurityHeaders, fallback_host: &str) -> bool {
    let Some(origin) = headers.origin.as_deref() else {
        return true;
    };
    let Some(origin_host) = origin_host(origin) else {
        return false;
    };
    let expected_host = headers.host.as_deref().unwrap_or(fallback_host);
    same_origin_host(origin_host, expected_host)
}

fn normalize_origin_host(value: &str) -> String {
    let value = value.trim();
    if let Some((host, port)) = bracketed_ipv6_authority(value) {
        return format!("[{}]:{}", host.to_ascii_lowercase(), port);
    }
    value.to_ascii_lowercase()
}

fn bracketed_ipv6_authority(value: &str) -> Option<(&str, &str)> {
    let rest = value.strip_prefix('[')?;
    let (host, after_host) = rest.split_once(']')?;
    let port = after_host.strip_prefix(':')?;
    if host.is_empty() || port.is_empty() || port.contains(':') {
        return None;
    }
    Some((host, port))
}

pub fn is_authorized(
    config: &AppConfig,
    authorization: Option<&str>,
    cookie: Option<&str>,
) -> bool {
    let Some(expected_token) = config.api_token.as_deref() else {
        return false;
    };
    let bearer_authorized = authorization
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("ApiKey "))
        })
        .is_some_and(|token| constant_time_eq(token.as_bytes(), expected_token.as_bytes()));
    bearer_authorized
        || (config.api_cookie_auth_enabled
            && cookie_session_token(cookie)
                .is_some_and(|token| constant_time_eq(token.as_bytes(), expected_token.as_bytes())))
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

fn cookie_session_token(cookie: Option<&str>) -> Option<String> {
    cookie?
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(name, value)| {
            (name == "slskr.session").then(|| percent_decode_component(value.trim()))
        })
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
         Access-Control-Allow-Headers: Content-Type, Authorization, X-API-Key\r\n\
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
