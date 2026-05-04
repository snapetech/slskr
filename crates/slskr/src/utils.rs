#![allow(dead_code)]

use std::net::SocketAddr;
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

#[derive(Clone, Copy, Debug, Default)]
pub struct RequestSecurityHeaders<'a> {
    pub host: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub referer: Option<&'a str>,
    pub cookie: Option<&'a str>,
}

impl<'a> RequestSecurityHeaders<'a> {
    pub fn from_request(request: &'a str) -> Self {
        let mut headers = Self::default();
        for line in request.lines().skip(1) {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            if name.eq_ignore_ascii_case("host") {
                headers.host = Some(value);
            } else if name.eq_ignore_ascii_case("origin") {
                headers.origin = Some(value);
            } else if name.eq_ignore_ascii_case("referer") {
                headers.referer = Some(value);
            } else if name.eq_ignore_ascii_case("cookie") {
                headers.cookie = Some(value);
            }
        }
        headers
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
}

pub fn csrf_origin_allowed(
    config: &AppConfig,
    method: &str,
    path: &str,
    headers: &RequestSecurityHeaders<'_>,
) -> bool {
    if !config.auth_required || !is_unsafe_http_method(method) || !path.starts_with("/api/") {
        return true;
    }
    let Some(source) = headers.origin.or(headers.referer) else {
        return true;
    };
    let Some(source_host) = origin_host(source) else {
        return false;
    };
    let fallback_host = config.http_bind.to_string();
    let expected_host = headers.host.unwrap_or(fallback_host.as_str());
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
        .filter(|host| !host.is_empty())
}

pub fn same_origin_host(left: &str, right: &str) -> bool {
    normalize_origin_host(left) == normalize_origin_host(right)
}

fn normalize_origin_host(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase()
}

pub fn is_authorized(config: &AppConfig, authorization: Option<&str>, cookie: Option<&str>) -> bool {
    let Some(expected_token) = config.api_token.as_deref() else {
        return false;
    };
    let bearer_authorized = authorization
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| token == expected_token);
    bearer_authorized || cookie_session_token(cookie).is_some_and(|token| token == expected_token)
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
        .split('/').next()
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
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let value = after_colon.strip_prefix('"')?;
    let mut output = String::new();
    let mut escaped = false;
    for character in value.chars() {
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
            '"' => return Some(output),
            other => output.push(other),
        }
    }
    None
}

pub fn extract_json_string_array_field(body: &str, field: &str) -> Option<Vec<String>> {
    let key = format!("\"{}\"", field);
    let after_key = json_field_after_key(body, &key)?;
    let mut value = after_key
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('[')?;
    let mut items = Vec::new();
    loop {
        value = value.trim_start();
        if value.starts_with(']') {
            return Some(items);
        }
        let item = parse_json_string_prefix(value)?;
        value = &value[item.consumed..];
        items.push(item.value);
        value = value.trim_start();
        if let Some(rest) = value.strip_prefix(',') {
            value = rest;
            continue;
        }
        return value.starts_with(']').then_some(items);
    }
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
