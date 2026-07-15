//! Optimized HTTP server with keep-alive, proper parsing, and streaming responses
//! Replaces manual HTTP parsing in main.rs

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::SysRng, TryRng};
use std::collections::HashSet;
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt,
};
use tokio::time::{self, Duration};

use crate::routing::HttpResponse;

/// HTTP request parsed from stream
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn body_as_str(&self) -> Result<&str, String> {
        std::str::from_utf8(&self.body).map_err(|_| "request body must be valid UTF-8".to_string())
    }
}

pub const BODY_SIZE_LIMIT: usize = 1024 * 1024; // 1 MiB
pub const REQUEST_LINE_LIMIT: usize = 8 * 1024;
pub const HEADER_LINE_LIMIT: usize = 8 * 1024;
pub const MAX_API_TOKEN_BYTES: usize = HEADER_LINE_LIMIT - b"X-API-Key: \r\n".len();
pub const HEADER_TOTAL_LIMIT: usize = 64 * 1024;
pub const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub const BODY_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(60);
pub const RESPONSE_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// HTTP headers
#[derive(Debug, Clone, Default)]
pub struct HttpHeaders {
    pub host: Option<String>,
    pub origin: Option<String>,
    pub referer: Option<String>,
    pub cookie: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub transfer_encoding: Option<String>,
    pub authorization: Option<String>,
    pub x_api_key: Option<String>,
    pub x_share_token: Option<String>,
    pub range: Option<String>,
    pub forwarded: Option<String>,
    pub x_forwarded_for: Option<String>,
    pub upgrade: Option<String>,
    pub sec_websocket_key: Option<String>,
    pub sec_websocket_protocol: Option<String>,
    pub sec_websocket_version: Option<String>,
    pub connection: String, // "keep-alive" or "close"
    pub user_agent: Option<String>,
}

impl HttpHeaders {
    pub fn connection_has_token(&self, token: &str) -> bool {
        header_has_token(&self.connection, token)
    }

    /// Parse headers from raw HTTP header lines
    #[allow(dead_code)]
    pub fn from_lines(lines: &[&str]) -> Self {
        let mut headers = HttpHeaders {
            connection: "close".to_string(), // Default to close
            ..Default::default()
        };

        for line in lines {
            if let Some(colon_idx) = line.find(':') {
                let name = line[..colon_idx].trim().to_lowercase();
                let value = line[colon_idx + 1..].trim();

                match name.as_str() {
                    "host" => headers.host = Some(value.to_string()),
                    "origin" => headers.origin = Some(value.to_string()),
                    "referer" => headers.referer = Some(value.to_string()),
                    "cookie" => headers.cookie = Some(value.to_string()),
                    "content-type" => headers.content_type = Some(value.to_string()),
                    "content-length" => {
                        headers.content_length = value.parse().ok();
                    }
                    "transfer-encoding" => headers.transfer_encoding = Some(value.to_lowercase()),
                    "authorization" => headers.authorization = Some(value.to_string()),
                    "x-api-key" => headers.x_api_key = Some(value.to_string()),
                    "x-share-token" => headers.x_share_token = Some(value.to_string()),
                    "range" => headers.range = Some(value.to_string()),
                    "forwarded" => headers.forwarded = Some(value.to_string()),
                    "x-forwarded-for" => headers.x_forwarded_for = Some(value.to_string()),
                    "upgrade" => headers.upgrade = Some(value.to_lowercase()),
                    "sec-websocket-key" => headers.sec_websocket_key = Some(value.to_string()),
                    "sec-websocket-protocol" => {
                        headers.sec_websocket_protocol = Some(value.to_string());
                    }
                    "sec-websocket-version" => {
                        headers.sec_websocket_version = Some(value.to_string());
                    }
                    "connection" => headers.connection = value.to_lowercase(),
                    "user-agent" => headers.user_agent = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        headers
    }
}

/// Parse HTTP request from raw data
#[allow(dead_code)]
pub fn parse_http_request(data: &str) -> Option<(String, String, Option<String>, HttpHeaders)> {
    let mut lines = data.lines();

    let request_line = lines.next()?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_string();
    let request_target = parts[1];

    // Split path and query
    let (path, query) = if let Some(qmark) = request_target.find('?') {
        let p = request_target[..qmark].to_string();
        let q = request_target[qmark + 1..].to_string();
        (p, Some(q))
    } else {
        (request_target.to_string(), None)
    };

    // Collect header lines until blank line
    let header_lines: Vec<&str> = lines.by_ref().take_while(|l| !l.is_empty()).collect();

    let headers = HttpHeaders::from_lines(&header_lines);

    Some((method, path, query, headers))
}

/// Read HTTP request from stream with proper buffering.
/// Returns `Ok(None)` on clean EOF, `Ok(Some(...))` on success, `Err(msg)` on protocol error.
pub async fn read_http_request<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Option<(HttpRequest, bool)>, String> {
    read_http_request_with_timeout(reader, REQUEST_READ_TIMEOUT).await
}

async fn read_http_request_with_timeout<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    timeout: Duration,
) -> Result<Option<(HttpRequest, bool)>, String> {
    time::timeout(timeout, read_http_request_inner(reader))
        .await
        .map_err(|_| "request read deadline exceeded".to_owned())?
}

async fn read_http_request_inner<R: AsyncBufRead + Unpin>(
    reader: &mut R,
) -> Result<Option<(HttpRequest, bool)>, String> {
    let Some(request_line) =
        read_limited_line(reader, REQUEST_LINE_LIMIT, HEADER_READ_TIMEOUT).await?
    else {
        return Ok(None); // Connection closed
    };

    let request_line = request_line
        .strip_suffix("\r\n")
        .ok_or_else(|| "Invalid request line".to_owned())?;
    let mut parts = request_line.split(' ');
    let (Some(method), Some(request_target), Some(http_version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("Invalid request line".into());
    };
    if method.is_empty() || request_target.is_empty() || http_version.is_empty() {
        return Err("Invalid request line".into());
    }

    let method = method.to_string();
    if !is_http_token(&method) {
        return Err("invalid HTTP method".into());
    }
    if !matches!(http_version, "HTTP/1.0" | "HTTP/1.1") {
        return Err("unsupported HTTP version".into());
    }
    if !valid_request_target(&method, request_target) {
        return Err("unsupported request target".into());
    }

    // Split path and query
    let (path, query) = if let Some(qmark) = request_target.find('?') {
        let p = request_target[..qmark].to_string();
        let q = request_target[qmark + 1..].to_string();
        (p, Some(q))
    } else {
        (request_target.to_string(), None)
    };

    // Read headers
    let mut headers = HttpHeaders {
        connection: "close".to_string(),
        ..Default::default()
    };
    let mut content_length: usize = 0;
    let mut saw_content_length = false;
    let mut saw_transfer_encoding = false;
    let mut saw_connection = false;
    let mut singleton_headers = HashSet::new();

    let mut total_header_bytes = request_line.len();

    loop {
        let Some(header_line) =
            read_limited_line(reader, HEADER_LINE_LIMIT, HEADER_READ_TIMEOUT).await?
        else {
            return Err("unexpected EOF while reading headers".to_string());
        };
        total_header_bytes = total_header_bytes.saturating_add(header_line.len());
        if total_header_bytes > HEADER_TOTAL_LIMIT {
            return Err(format!(
                "request headers too large: {} bytes (limit {})",
                total_header_bytes, HEADER_TOTAL_LIMIT
            ));
        }
        if header_line == "\r\n" {
            break;
        }

        if header_line.starts_with([' ', '\t']) {
            return Err("obsolete folded headers are not supported".to_string());
        }
        let Some(colon_idx) = header_line.find(':') else {
            return Err("malformed HTTP header".to_string());
        };
        let raw_name = &header_line[..colon_idx];
        if !is_http_token(raw_name) {
            return Err("invalid HTTP header name".to_string());
        }
        let name = raw_name.to_lowercase();
        let value = header_line[colon_idx + 1..].trim();
        if value.chars().any(|ch| ch.is_control() && ch != '\t') {
            return Err("invalid HTTP header value".to_string());
        }

        match name.as_str() {
            "host" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                if value.is_empty() {
                    return Err("Host header must not be empty".to_owned());
                }
                headers.host = Some(value.to_string());
            }
            "origin" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.origin = Some(value.to_string());
            }
            "referer" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.referer = Some(value.to_string());
            }
            "cookie" => append_cookie_header(&mut headers.cookie, value),
            "content-type" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.content_type = Some(value.to_string());
            }
            "content-length" => {
                if saw_content_length {
                    return Err("duplicate Content-Length header".to_string());
                }
                if value.starts_with(['+', '-']) || value.contains(',') {
                    return Err("invalid Content-Length header".to_string());
                }
                content_length = value
                    .parse()
                    .map_err(|_| "invalid Content-Length header".to_string())?;
                headers.content_length = Some(content_length);
                saw_content_length = true;
            }
            "transfer-encoding" => {
                headers.transfer_encoding = Some(value.to_lowercase());
                saw_transfer_encoding = true;
            }
            "authorization" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.authorization = Some(value.to_string());
            }
            "x-api-key" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.x_api_key = Some(value.to_string());
            }
            "x-share-token" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.x_share_token = Some(value.to_string());
            }
            "range" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.range = Some(value.to_string());
            }
            "forwarded" => append_list_header(&mut headers.forwarded, value),
            "x-forwarded-for" => append_list_header(&mut headers.x_forwarded_for, value),
            "upgrade" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.upgrade = Some(value.to_lowercase());
            }
            "sec-websocket-key" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.sec_websocket_key = Some(value.to_string());
            }
            "sec-websocket-protocol" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.sec_websocket_protocol = Some(value.to_string());
            }
            "sec-websocket-version" => {
                reject_duplicate_singleton(&mut singleton_headers, &name)?;
                headers.sec_websocket_version = Some(value.to_string());
            }
            "connection" => {
                let value = value.to_lowercase();
                if saw_connection {
                    headers.connection.push_str(", ");
                    headers.connection.push_str(&value);
                } else {
                    headers.connection = value;
                    saw_connection = true;
                }
            }
            "user-agent" => headers.user_agent = Some(value.to_string()),
            _ => {}
        }
    }

    if saw_transfer_encoding {
        if saw_content_length {
            return Err("Transfer-Encoding with Content-Length is not supported".to_string());
        }
        return Err("Transfer-Encoding is not supported".to_string());
    }
    if headers.authorization.is_some() && headers.x_api_key.is_some() {
        return Err("multiple HTTP authentication mechanisms are not supported".to_owned());
    }
    if http_version == "HTTP/1.1" && headers.host.is_none() {
        return Err("HTTP/1.1 requires a Host header".to_owned());
    }
    if headers
        .host
        .as_deref()
        .is_some_and(|host| crate::utils::parse_authority(host).is_none())
    {
        return Err("invalid Host header authority".to_owned());
    }

    // Reject oversized bodies before reading
    if content_length > BODY_SIZE_LIMIT {
        return Err(format!(
            "request body too large: {} bytes (limit {})",
            content_length, BODY_SIZE_LIMIT
        ));
    }

    // Read body if content-length is set
    let body = if content_length > 0 {
        let mut buf = vec![0_u8; content_length];
        time::timeout(BODY_READ_TIMEOUT, reader.read_exact(&mut buf))
            .await
            .map_err(|_| "request body read timed out".to_string())?
            .map_err(|e| e.to_string())?;
        buf
    } else {
        Vec::new()
    };

    let keep_alive = if http_version == "HTTP/1.1" {
        !headers.connection_has_token("close")
    } else {
        headers.connection_has_token("keep-alive")
    };

    Ok(Some((
        HttpRequest {
            method,
            path,
            query,
            headers,
            body,
        },
        keep_alive,
    )))
}

fn reject_duplicate_singleton(seen: &mut HashSet<String>, name: &str) -> Result<(), String> {
    if !seen.insert(name.to_owned()) {
        return Err(format!("duplicate {name} header"));
    }
    Ok(())
}

fn append_list_header(header: &mut Option<String>, value: &str) {
    if let Some(existing) = header {
        existing.push_str(", ");
        existing.push_str(value);
    } else {
        *header = Some(value.to_owned());
    }
}

fn append_cookie_header(header: &mut Option<String>, value: &str) {
    if let Some(existing) = header {
        existing.push_str("; ");
        existing.push_str(value);
    } else {
        *header = Some(value.to_owned());
    }
}

fn header_has_token(value: &str, token: &str) -> bool {
    value
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case(token))
}

fn is_http_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            matches!(
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
                    | b'0'..=b'9'
                    | b'A'..=b'Z'
                    | b'a'..=b'z'
            )
        })
}

fn valid_request_target(method: &str, target: &str) -> bool {
    if target == "*" {
        return method == "OPTIONS";
    }
    if !target.starts_with('/') {
        return false;
    }

    let bytes = target.as_bytes();
    let path_end = target.find('?').unwrap_or(target.len());
    let path = &target[..path_end];
    let encoded_stream_separator_allowed = [
        "/api/streams/",
        "/api/v0/streams/",
        "/api/v1/streams/",
        "/api/v2/streams/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix));
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            let Some(high) = bytes.get(index + 1).copied().and_then(http_hex_value) else {
                return false;
            };
            let Some(low) = bytes.get(index + 2).copied().and_then(http_hex_value) else {
                return false;
            };
            let decoded_byte = (high << 4) | low;
            if index < path_end
                && (matches!(decoded_byte, b'?' | b'#')
                    || (decoded_byte == b'/' && !encoded_stream_separator_allowed))
            {
                return false;
            }
            decoded.push(decoded_byte);
            index += 3;
            continue;
        }
        if !(byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
                    | b'/'
                    | b'?'
            ))
        {
            return false;
        }
        decoded.push(byte);
        index += 1;
    }
    if std::str::from_utf8(&decoded).is_err()
        || decoded
            .iter()
            .any(|byte| byte.is_ascii_control() || *byte == b'\\')
    {
        return false;
    }
    let decoded_path_end = decoded
        .iter()
        .position(|byte| *byte == b'?')
        .unwrap_or(decoded.len());
    let decoded_path = &decoded[..decoded_path_end];
    let mut segments = decoded_path.split(|byte| *byte == b'/').peekable();
    let _leading_empty = segments.next();
    while let Some(segment) = segments.next() {
        if matches!(segment, b"." | b"..") || (segment.is_empty() && segments.peek().is_some()) {
            return false;
        }
    }
    true
}

fn http_hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

async fn read_limited_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    limit: usize,
    timeout: Duration,
) -> Result<Option<String>, String> {
    let mut bytes = Vec::new();
    loop {
        let available = time::timeout(timeout, reader.fill_buf())
            .await
            .map_err(|_| "request header read timed out".to_string())?
            .map_err(|error| error.to_string())?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }

        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(available.len());
        if bytes.len().saturating_add(take) > limit {
            return Err(format!("request header line too large (limit {limit})"));
        }
        bytes.extend_from_slice(&available[..take]);
        reader.consume(take);
        if bytes.ends_with(b"\n") {
            break;
        }
    }
    if !bytes.ends_with(b"\r\n") {
        return Err("HTTP lines must end with CRLF".to_string());
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| "request headers must be valid UTF-8".to_string())
}

/// Write HTTP response to stream with minimal allocations (streaming)
pub async fn write_http_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &HttpResponse,
    keep_alive: bool,
    extra_headers: &str,
) -> Result<(), String> {
    write_http_response_with_timeout(
        writer,
        response,
        keep_alive,
        extra_headers,
        RESPONSE_WRITE_TIMEOUT,
    )
    .await
}

async fn write_http_response_with_timeout<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &HttpResponse,
    keep_alive: bool,
    extra_headers: &str,
    timeout: Duration,
) -> Result<(), String> {
    time::timeout(
        timeout,
        write_http_response_inner(writer, response, keep_alive, extra_headers),
    )
    .await
    .map_err(|_| "response write deadline exceeded".to_owned())?
}

async fn write_http_response_inner<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &HttpResponse,
    keep_alive: bool,
    extra_headers: &str,
) -> Result<(), String> {
    let connection_header = if keep_alive {
        "Connection: keep-alive\r\n"
    } else {
        "Connection: close\r\n"
    };

    let (body, csp_header) = body_with_content_security_policy(response)?;
    let body_bytes = body.as_bytes();
    let e = |err: std::io::Error| err.to_string();

    // Write status line and headers
    writer.write_all(b"HTTP/1.1 ").await.map_err(e)?;
    writer
        .write_all(response.status.as_bytes())
        .await
        .map_err(e)?;
    writer.write_all(b"\r\n").await.map_err(e)?;
    writer.write_all(b"Content-Type: ").await.map_err(e)?;
    writer
        .write_all(response.content_type.as_bytes())
        .await
        .map_err(e)?;
    writer.write_all(b"\r\n").await.map_err(e)?;
    writer.write_all(b"Content-Length: ").await.map_err(e)?;
    writer
        .write_all(body_bytes.len().to_string().as_bytes())
        .await
        .map_err(e)?;
    writer.write_all(b"\r\n").await.map_err(e)?;
    writer
        .write_all(
            b"X-Content-Type-Options: nosniff\r\n\
Referrer-Policy: no-referrer\r\n\
Strict-Transport-Security: max-age=31536000; includeSubDomains\r\n",
        )
        .await
        .map_err(e)?;
    writer.write_all(csp_header.as_bytes()).await.map_err(e)?;
    writer
        .write_all(connection_header.as_bytes())
        .await
        .map_err(e)?;
    writer
        .write_all(extra_headers.as_bytes())
        .await
        .map_err(e)?;
    writer.write_all(b"\r\n").await.map_err(e)?;

    // Write body
    writer.write_all(body_bytes).await.map_err(e)?;
    writer.flush().await.map_err(e)?;

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileResponseResult {
    pub status_code: u16,
    pub content_length: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ByteRange {
    start: u64,
    length: u64,
}

fn parse_byte_range(value: &str, total_length: u64) -> Result<ByteRange, ()> {
    let value = value.trim();
    let Some(spec) = value.strip_prefix("bytes=") else {
        return Err(());
    };
    if total_length == 0 || spec.contains(',') {
        return Err(());
    }
    let (start, end) = spec.split_once('-').ok_or(())?;
    if start.is_empty() {
        let suffix_length = end.parse::<u64>().map_err(|_| ())?;
        if suffix_length == 0 {
            return Err(());
        }
        let length = suffix_length.min(total_length);
        return Ok(ByteRange {
            start: total_length - length,
            length,
        });
    }

    let start = start.parse::<u64>().map_err(|_| ())?;
    if start >= total_length {
        return Err(());
    }
    let end = if end.is_empty() {
        total_length - 1
    } else {
        end.parse::<u64>().map_err(|_| ())?.min(total_length - 1)
    };
    if end < start {
        return Err(());
    }
    Ok(ByteRange {
        start,
        length: end - start + 1,
    })
}

/// Write a regular file without buffering its contents in memory. Only a single
/// HTTP byte range is accepted; multipart ranges are rejected with 416.
#[expect(
    clippy::too_many_arguments,
    reason = "the streaming response boundary requires explicit HTTP metadata"
)]
pub async fn write_file_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    file: std::fs::File,
    total_length: u64,
    content_type: &str,
    range: Option<&str>,
    accept_ranges: bool,
    include_body: bool,
    keep_alive: bool,
    extra_headers: &str,
) -> Result<FileResponseResult, String> {
    let selected = match range.filter(|_| accept_ranges) {
        Some(value) => match parse_byte_range(value, total_length) {
            Ok(range) => Some(range),
            Err(()) => {
                let connection = if keep_alive { "keep-alive" } else { "close" };
                let headers = format!(
                    "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Type: application/json\r\nContent-Length: 0\r\nContent-Range: bytes */{total_length}\r\nAccept-Ranges: bytes\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nStrict-Transport-Security: max-age=31536000; includeSubDomains\r\nConnection: {connection}\r\n{extra_headers}\r\n"
                );
                time::timeout(RESPONSE_WRITE_TIMEOUT, async {
                    writer.write_all(headers.as_bytes()).await?;
                    writer.flush().await
                })
                .await
                .map_err(|_| "response write deadline exceeded".to_owned())?
                .map_err(|error| error.to_string())?;
                return Ok(FileResponseResult {
                    status_code: 416,
                    content_length: 0,
                });
            }
        },
        None => None,
    };
    let (status, start, content_length, content_range) = if let Some(range) = selected {
        (
            "206 Partial Content",
            range.start,
            range.length,
            format!(
                "Content-Range: bytes {}-{}/{}\r\n",
                range.start,
                range.start + range.length - 1,
                total_length
            ),
        )
    } else {
        ("200 OK", 0, total_length, String::new())
    };
    let connection = if keep_alive { "keep-alive" } else { "close" };
    let accept_ranges = if accept_ranges { "bytes" } else { "none" };
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {content_length}\r\n{content_range}Accept-Ranges: {accept_ranges}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nStrict-Transport-Security: max-age=31536000; includeSubDomains\r\nConnection: {connection}\r\n{extra_headers}\r\n"
    );
    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(headers.as_bytes()))
        .await
        .map_err(|_| "response write deadline exceeded".to_owned())?
        .map_err(|error| error.to_string())?;

    if include_body && content_length > 0 {
        let mut file = tokio::fs::File::from_std(file);
        if start > 0 {
            time::timeout(
                RESPONSE_WRITE_TIMEOUT,
                file.seek(std::io::SeekFrom::Start(start)),
            )
            .await
            .map_err(|_| "file seek deadline exceeded".to_owned())?
            .map_err(|error| error.to_string())?;
        }
        let mut remaining = content_length;
        let mut buffer = vec![0_u8; 64 * 1024];
        while remaining > 0 {
            let wanted = usize::try_from(remaining.min(buffer.len() as u64))
                .expect("bounded by stream buffer length");
            let read = time::timeout(RESPONSE_WRITE_TIMEOUT, file.read(&mut buffer[..wanted]))
                .await
                .map_err(|_| "file read deadline exceeded".to_owned())?
                .map_err(|error| error.to_string())?;
            if read == 0 {
                return Err("file ended before the advertised content length".to_owned());
            }
            time::timeout(RESPONSE_WRITE_TIMEOUT, writer.write_all(&buffer[..read]))
                .await
                .map_err(|_| "response write deadline exceeded".to_owned())?
                .map_err(|error| error.to_string())?;
            remaining -= read as u64;
        }
    }
    time::timeout(RESPONSE_WRITE_TIMEOUT, writer.flush())
        .await
        .map_err(|_| "response write deadline exceeded".to_owned())?
        .map_err(|error| error.to_string())?;
    Ok(FileResponseResult {
        status_code: if selected.is_some() { 206 } else { 200 },
        content_length,
    })
}

fn body_with_content_security_policy(response: &HttpResponse) -> Result<(String, String), String> {
    if response.content_type.starts_with("text/html") {
        let nonce = csp_nonce()?;
        let body = response
            .body
            .replace("<script>", &format!(r#"<script nonce="{nonce}">"#))
            .replace("<style>", &format!(r#"<style nonce="{nonce}">"#));
        let header = format!(
            "Content-Security-Policy: default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' 'nonce-{nonce}'; style-src 'self' 'nonce-{nonce}'; img-src 'self' data:; connect-src 'self' ws: wss:\r\n"
        );
        return Ok((body, header));
    }

    Ok((
        response.body.clone(),
        "Content-Security-Policy: default-src 'none'; base-uri 'none'; frame-ancestors 'none'; object-src 'none'\r\n"
            .to_owned(),
    ))
}

fn csp_nonce() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "operating system randomness is unavailable".to_owned())?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

    // ── parse_http_request (synchronous helper) ──────────────────────────────

    #[test]
    fn test_parse_http_request_get() {
        let data = "GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n";
        let (method, path, query, headers) = parse_http_request(data).unwrap();
        assert_eq!(method, "GET");
        assert_eq!(path, "/api/health");
        assert_eq!(query, None);
        assert_eq!(headers.connection, "keep-alive");
    }

    #[test]
    fn test_parse_http_request_with_query() {
        let data = "GET /api/search?q=test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (method, path, query, _) = parse_http_request(data).unwrap();
        assert_eq!(method, "GET");
        assert_eq!(path, "/api/search");
        assert_eq!(query, Some("q=test".to_string()));
    }

    #[test]
    fn test_parse_headers() {
        let lines = vec![
            "Origin: http://localhost:3000",
            "Content-Type: application/json",
            "Content-Length: 256",
            "Authorization: Bearer token123",
            "X-API-Key: key123",
            "X-Share-Token: share-secret",
            "Range: bytes=10-19",
            "Forwarded: for=198.51.100.24;proto=https",
            "X-Forwarded-For: 198.51.100.24, 127.0.0.1",
            "Connection: keep-alive",
            "Sec-WebSocket-Protocol: slskr.api-token.route%2Dtoken",
        ];
        let headers = HttpHeaders::from_lines(&lines);
        assert_eq!(headers.origin, Some("http://localhost:3000".to_string()));
        assert_eq!(headers.content_type, Some("application/json".to_string()));
        assert_eq!(headers.content_length, Some(256));
        assert_eq!(headers.authorization, Some("Bearer token123".to_string()));
        assert_eq!(headers.x_api_key, Some("key123".to_string()));
        assert_eq!(headers.x_share_token, Some("share-secret".to_string()));
        assert_eq!(headers.range, Some("bytes=10-19".to_string()));
        assert_eq!(
            headers.forwarded,
            Some("for=198.51.100.24;proto=https".to_string())
        );
        assert_eq!(
            headers.x_forwarded_for,
            Some("198.51.100.24, 127.0.0.1".to_string())
        );
        assert_eq!(headers.connection, "keep-alive");
        assert_eq!(
            headers.sec_websocket_protocol,
            Some("slskr.api-token.route%2Dtoken".to_string())
        );
    }

    // ── read_http_request (async, over in-memory duplex stream) ──────────────

    #[tokio::test]
    async fn test_read_get_request() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n",
            )
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let (req, keep_alive) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/health");
        assert_eq!(req.query, None);
        assert!(keep_alive);
        assert!(req.body.is_empty());
    }

    #[tokio::test]
    async fn test_read_post_with_100kb_body() {
        let (mut client, server) = tokio::io::duplex(256 * 1024);
        let body = vec![b'x'; 100 * 1024];
        let header = format!(
            "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\n\r\n",
            body.len()
        );
        client.write_all(header.as_bytes()).await.unwrap();
        client.write_all(&body).await.unwrap();

        let mut reader = BufReader::new(server);
        let (req, _keep_alive) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/api/echo");
        assert_eq!(req.body.len(), 100 * 1024);
    }

    #[tokio::test]
    async fn test_binary_body_is_preserved() {
        let (mut client, server) = tokio::io::duplex(4096);
        let body = [0xff, 0x00, 0x80, b'a'];
        let header = format!(
            "POST /api/upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        client.write_all(header.as_bytes()).await.unwrap();
        client.write_all(&body).await.unwrap();

        let mut reader = BufReader::new(server);
        let (req, _) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(req.body, body);
        assert!(req.body_as_str().is_err());
    }

    #[tokio::test]
    async fn test_transfer_encoding_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"POST /api/echo HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
            )
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("Transfer-Encoding"), "{err}");
    }

    #[tokio::test]
    async fn test_invalid_content_length_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: garbage\r\n\r\nhello")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("Content-Length"), "{err}");
    }

    #[tokio::test]
    async fn test_duplicate_content_length_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello",
            )
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("duplicate Content-Length"), "{err}");
    }

    #[tokio::test]
    async fn test_http11_requires_one_nonempty_host_header() {
        for request in [
            b"GET / HTTP/1.1\r\n\r\n".as_slice(),
            b"GET / HTTP/1.1\r\nHost:\r\n\r\n".as_slice(),
            b"GET / HTTP/1.1\r\nHost: first.example\r\nHost: second.example\r\n\r\n".as_slice(),
        ] {
            let (mut client, server) = tokio::io::duplex(4096);
            client.write_all(request).await.unwrap();
            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("Host") || error.contains("host"), "{error}");
        }
    }

    #[tokio::test]
    async fn test_malformed_host_authorities_are_rejected() {
        for host in [
            "user@localhost",
            "localhost,example.test",
            "local host",
            "localhost:bad",
            "localhost:65536",
            "[::1",
            "[::1]garbage",
            "[::1]:bad",
        ] {
            let (mut client, server) = tokio::io::duplex(4096);
            let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n");
            client.write_all(request.as_bytes()).await.unwrap();
            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("Host"), "{host:?}: {error}");
        }
    }

    #[tokio::test]
    async fn test_ipv6_host_authority_is_accepted() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: [::1]:5030\r\n\r\n")
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let (request, _) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(request.headers.host.as_deref(), Some("[::1]:5030"));
    }

    #[tokio::test]
    async fn test_duplicate_authentication_headers_are_rejected() {
        for header in ["Authorization", "X-Api-Key", "Origin"] {
            let (mut client, server) = tokio::io::duplex(4096);
            let request = format!(
                "GET / HTTP/1.1\r\nHost: localhost\r\n{header}: first\r\n{header}: second\r\n\r\n"
            );
            client.write_all(request.as_bytes()).await.unwrap();
            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("duplicate"), "{error}");
        }
    }

    #[tokio::test]
    async fn test_mixed_authentication_headers_are_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"GET / HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer first\r\nX-API-Key: second\r\n\r\n",
            )
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let error = read_http_request(&mut reader).await.unwrap_err();
        assert!(error.contains("authentication mechanisms"), "{error}");
    }

    #[tokio::test]
    async fn test_repeated_forwarding_headers_are_combined_in_wire_order() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-For: 198.51.100.1\r\nX-Forwarded-For: 10.0.0.2\r\nForwarded: for=198.51.100.1\r\nForwarded: for=10.0.0.2\r\n\r\n",
            )
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let (request, _) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(
            request.headers.x_forwarded_for.as_deref(),
            Some("198.51.100.1, 10.0.0.2")
        );
        assert_eq!(
            request.headers.forwarded.as_deref(),
            Some("for=198.51.100.1, for=10.0.0.2")
        );
    }

    #[tokio::test]
    async fn test_repeated_list_headers_preserve_all_values() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"GET / HTTP/1.1\r\nHost: localhost\r\nCookie: first=1\r\nCookie: second=2\r\nConnection: keep-alive\r\nConnection: close\r\n\r\n",
            )
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let (request, keep_alive) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(request.headers.cookie.as_deref(), Some("first=1; second=2"));
        assert_eq!(request.headers.connection, "keep-alive, close");
        assert!(!keep_alive);
    }

    #[tokio::test]
    async fn test_oversized_body_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        // Claim 2 MiB — exceeds the 1 MiB cap; no body bytes needed
        let over_limit = BODY_SIZE_LIMIT + 1;
        let header = format!(
            "POST /api/echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
            over_limit
        );
        client.write_all(header.as_bytes()).await.unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(
            err.contains("too large"),
            "expected 'too large', got: {err}"
        );
    }

    #[tokio::test]
    async fn test_malformed_request_line() {
        let (mut client, server) = tokio::io::duplex(4096);
        // Only one token on the request line — not a valid HTTP request
        client.write_all(b"GARBAGE\r\n\r\n").await.unwrap();
        drop(client); // signal EOF

        let mut reader = BufReader::new(server);
        let result = read_http_request(&mut reader).await;
        assert!(result.is_err(), "expected Err, got: {result:?}");
    }

    #[tokio::test]
    async fn test_noncanonical_request_line_whitespace_rejected() {
        for request_line in ["GET  / HTTP/1.1", "GET\t/\tHTTP/1.1", " GET / HTTP/1.1"] {
            let (mut client, server) = tokio::io::duplex(4096);
            let request = format!("{request_line}\r\nHost: localhost\r\n\r\n");
            client.write_all(request.as_bytes()).await.unwrap();

            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("request line"), "{request_line:?}: {error}");
        }
    }

    #[tokio::test]
    async fn test_invalid_http_version_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client.write_all(b"GET / FOO\r\n\r\n").await.unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("HTTP version"), "{err}");
    }

    #[tokio::test]
    async fn test_absolute_request_target_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET http://example.test/api/health HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("request target"), "{err}");
    }

    #[tokio::test]
    async fn test_ambiguous_request_targets_are_rejected() {
        for target in [
            "/api\\health",
            "/api/health#fragment",
            "/api/health%",
            "/api/health%2",
            "/api/health%GG",
            "/api/health%FF",
            "/api/health%C3%28",
            "/api/health%00",
            "/api/health%5Cadmin",
            "/api%2Fhealth",
            "/api/health%3Fadmin=true",
            "/api/health%23fragment",
            "/api//health",
            "/api/./health",
            "/api/../health",
            "/api/%2e/health",
            "/api/%2E%2E/health",
            "/api/\0health",
            "/api/❤",
        ] {
            let (mut client, server) = tokio::io::duplex(4096);
            let request = format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n");
            client.write_all(request.as_bytes()).await.unwrap();
            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("request target"), "{target:?}: {error}");
        }
    }

    #[tokio::test]
    async fn test_percent_encoded_utf8_request_target_is_accepted() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"GET /api/%E2%9D%A4?q=%2Fvalue%3Fpart%23anchor HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let (request, _) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(request.path, "/api/%E2%9D%A4");
        assert_eq!(request.query.as_deref(), Some("q=%2Fvalue%3Fpart%23anchor"));
    }

    #[tokio::test]
    async fn stream_targets_accept_encoded_content_separators_only() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(
                b"GET /api/v0/streams/Library%2Ftrack.flac HTTP/1.1\r\nHost: localhost\r\n\r\n",
            )
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let (request, _) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(request.path, "/api/v0/streams/Library%2Ftrack.flac");

        for target in [
            "/api/health%2Fadmin",
            "/api/v0/streams/Library%2F..%2Fsecret",
            "/api/v0/streams/Library%2F%2Fsecret",
        ] {
            let (mut client, server) = tokio::io::duplex(4096);
            client
                .write_all(format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n").as_bytes())
                .await
                .unwrap();
            let mut reader = BufReader::new(server);
            assert!(read_http_request(&mut reader).await.is_err(), "{target}");
        }
    }

    #[tokio::test]
    async fn test_canonical_trailing_slash_is_accepted() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET /api/transfers/downloads/ HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();
        let mut reader = BufReader::new(server);
        let (request, _) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert_eq!(request.path, "/api/transfers/downloads/");
    }

    #[tokio::test]
    async fn test_asterisk_request_target_is_options_only() {
        for (method, accepted) in [("OPTIONS", true), ("GET", false)] {
            let (mut client, server) = tokio::io::duplex(4096);
            let request = format!("{method} * HTTP/1.1\r\nHost: localhost\r\n\r\n");
            client.write_all(request.as_bytes()).await.unwrap();
            let mut reader = BufReader::new(server);
            let result = read_http_request(&mut reader).await;
            assert_eq!(result.is_ok(), accepted, "{method}: {result:?}");
        }
    }

    #[tokio::test]
    async fn test_malformed_header_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\r\nHost localhost\r\n\r\n")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("malformed HTTP header"), "{err}");
    }

    #[tokio::test]
    async fn test_whitespace_before_header_colon_rejected() {
        for request in [
            b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length : 5\r\n\r\nhello".as_slice(),
            b"GET / HTTP/1.1\r\nHost : localhost\r\n\r\n".as_slice(),
        ] {
            let (mut client, server) = tokio::io::duplex(4096);
            client.write_all(request).await.unwrap();
            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("header name"), "{error}");
        }
    }

    #[tokio::test]
    async fn test_folded_header_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n Content-Length: 5\r\n\r\nhello")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("folded headers"), "{err}");
    }

    #[tokio::test]
    async fn test_whitespace_only_header_line_rejected() {
        for whitespace in [" ", "\t", " \t"] {
            let (mut client, server) = tokio::io::duplex(4096);
            let request = format!(
                "GET / HTTP/1.1\r\nHost: localhost\r\n{whitespace}\r\nX-Ignored: value\r\n\r\n"
            );
            client.write_all(request.as_bytes()).await.unwrap();

            let mut reader = BufReader::new(server);
            let error = read_http_request(&mut reader).await.unwrap_err();
            assert!(error.contains("folded headers"), "{whitespace:?}: {error}");
        }
    }

    #[tokio::test]
    async fn test_bare_lf_request_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\nHost: localhost\n\n")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("CRLF"), "{err}");
    }

    #[tokio::test]
    async fn test_truncated_header_line_rejected() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost")
            .await
            .unwrap();
        drop(client);

        let mut reader = BufReader::new(server);
        let err = read_http_request(&mut reader).await.unwrap_err();
        assert!(err.contains("CRLF"), "{err}");
    }

    #[tokio::test]
    async fn test_request_deadline_is_not_reset_by_partial_progress() {
        let (mut client, server) = tokio::io::duplex(4096);
        tokio::spawn(async move {
            for byte in b"GET / HTTP/1.1\r\n" {
                client.write_all(&[*byte]).await.expect("write slow byte");
                time::sleep(Duration::from_millis(20)).await;
            }
        });

        let mut reader = BufReader::new(server);
        let error = read_http_request_with_timeout(&mut reader, Duration::from_millis(100))
            .await
            .expect_err("request must hit the absolute deadline");
        assert!(error.contains("deadline exceeded"), "{error}");
    }

    #[tokio::test]
    async fn test_connection_close_disables_keep_alive() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let (_req, keep_alive) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert!(!keep_alive);
    }

    #[tokio::test]
    async fn test_connection_header_uses_tokens() {
        let (mut client, server) = tokio::io::duplex(4096);
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: closer\r\n\r\n")
            .await
            .unwrap();

        let mut reader = BufReader::new(server);
        let (_req, keep_alive) = read_http_request(&mut reader).await.unwrap().unwrap();
        assert!(keep_alive);
    }

    // ── write_http_response ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_write_response_format() {
        let (client, mut server) = tokio::io::duplex(4096);
        let response = HttpResponse {
            status: "200 OK",
            content_type: "application/json",
            body: r#"{"status":"ok"}"#.to_string(),
        };
        let mut writer = BufWriter::new(client);
        write_http_response(&mut writer, &response, true, "")
            .await
            .unwrap();
        drop(writer);

        let mut raw = String::new();
        server.read_to_string(&mut raw).await.unwrap();
        assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(raw.contains("Content-Type: application/json\r\n"));
        assert!(raw.contains("X-Content-Type-Options: nosniff\r\n"));
        assert!(raw.contains("Referrer-Policy: no-referrer\r\n"));
        assert!(raw.contains("Content-Security-Policy: "));
        assert!(raw.contains("default-src 'none'"));
        assert!(!raw.contains("'unsafe-inline'"));
        assert!(!raw.contains("wasm-unsafe-eval"));
        assert!(raw.contains(&format!("Content-Length: {}\r\n", response.body.len())));
        assert!(raw.contains("Connection: keep-alive\r\n"));
        assert!(raw.ends_with(&response.body));
    }

    #[tokio::test]
    async fn test_response_write_deadline_releases_blocked_writer() {
        let (mut writer, _unread_peer) = tokio::io::duplex(64);
        let response = HttpResponse {
            status: "200 OK",
            content_type: "application/octet-stream",
            body: "x".repeat(1024 * 1024),
        };
        let error = write_http_response_with_timeout(
            &mut writer,
            &response,
            false,
            "",
            Duration::from_millis(50),
        )
        .await
        .expect_err("blocked response writer must time out");
        assert!(error.contains("deadline exceeded"), "{error}");
    }

    #[tokio::test]
    async fn test_html_response_uses_nonce_csp_without_unsafe_inline() {
        let (client, mut server) = tokio::io::duplex(4096);
        let response = HttpResponse {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body: "<!doctype html><style>body{color:black}</style><script>window.ok=true</script>"
                .to_string(),
        };
        let mut writer = BufWriter::new(client);
        write_http_response(&mut writer, &response, false, "")
            .await
            .unwrap();
        drop(writer);

        let mut raw = String::new();
        server.read_to_string(&mut raw).await.unwrap();
        assert!(raw.contains("Content-Security-Policy: "));
        assert!(raw.contains("'nonce-"));
        assert!(raw.contains("<style nonce=\""));
        assert!(raw.contains("<script nonce=\""));
        assert!(!raw.contains("'unsafe-inline'"));
        assert!(!raw.contains("wasm-unsafe-eval"));
    }

    #[tokio::test]
    async fn file_response_streams_a_bounded_single_range() {
        let path = std::env::temp_dir().join(format!(
            "slskr-http-stream-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"0123456789").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let (client, mut server) = tokio::io::duplex(4096);
        let mut writer = BufWriter::new(client);
        let result = write_file_response(
            &mut writer,
            file,
            10,
            "audio/flac",
            Some("bytes=2-5"),
            true,
            true,
            false,
            "X-Request-ID: test\r\n",
        )
        .await
        .unwrap();
        drop(writer);
        let mut raw = Vec::new();
        server.read_to_end(&mut raw).await.unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(result.status_code, 206);
        assert_eq!(result.content_length, 4);
        let split = raw
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let headers = String::from_utf8(raw[..split].to_vec()).unwrap();
        assert!(headers.starts_with("HTTP/1.1 206 Partial Content\r\n"));
        assert!(headers.contains("Content-Range: bytes 2-5/10\r\n"));
        assert!(headers.contains("Accept-Ranges: bytes\r\n"));
        assert_eq!(&raw[split + 4..], b"2345");

        let path = std::env::temp_dir().join(format!(
            "slskr-http-stream-invalid-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"0123456789").unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let (client, mut server) = tokio::io::duplex(4096);
        let mut writer = BufWriter::new(client);
        let result = write_file_response(
            &mut writer,
            file,
            10,
            "audio/flac",
            Some("bytes=10-"),
            true,
            true,
            false,
            "",
        )
        .await
        .unwrap();
        drop(writer);
        let mut raw = String::new();
        server.read_to_string(&mut raw).await.unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(result.status_code, 416);
        assert!(raw.starts_with("HTTP/1.1 416 Range Not Satisfiable\r\n"));
        assert!(raw.contains("Content-Range: bytes */10\r\n"));
    }

    #[test]
    fn byte_range_parser_rejects_multipart_and_handles_suffixes() {
        assert_eq!(
            parse_byte_range("bytes=-3", 10),
            Ok(ByteRange {
                start: 7,
                length: 3
            })
        );
        assert_eq!(
            parse_byte_range("bytes=8-99", 10),
            Ok(ByteRange {
                start: 8,
                length: 2
            })
        );
        assert!(parse_byte_range("bytes=10-", 10).is_err());
        assert!(parse_byte_range("bytes=0-1,4-5", 10).is_err());
        assert!(parse_byte_range("items=0-1", 10).is_err());
    }

    // ── round-trip over a real TCP socket ─────────────────────────────────────

    #[tokio::test]
    async fn test_round_trip_over_tcp() {
        use tokio::net::{TcpListener, TcpStream};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (rh, wh) = stream.into_split();
            let mut reader = BufReader::new(rh);
            let mut writer = BufWriter::new(wh);

            let (req, keep_alive) = read_http_request(&mut reader).await.unwrap().unwrap();
            let body = format!(r#"{{"method":"{}","path":"{}"}}"#, req.method, req.path);
            let response = HttpResponse {
                status: "200 OK",
                content_type: "application/json",
                body,
            };
            write_http_response(&mut writer, &response, keep_alive, "")
                .await
                .unwrap();
        });

        let mut conn = TcpStream::connect(addr).await.unwrap();
        conn.write_all(b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();

        let mut raw = String::new();
        conn.read_to_string(&mut raw).await.unwrap();
        assert!(raw.starts_with("HTTP/1.1 200 OK\r\n"), "got: {raw}");
        assert!(raw.contains("/api/health"), "path not echoed: {raw}");
    }
}
