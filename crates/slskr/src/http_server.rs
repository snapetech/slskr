//! Optimized HTTP server with keep-alive, proper parsing, and streaming responses
//! Replaces manual HTTP parsing in main.rs

use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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
pub const HEADER_TOTAL_LIMIT: usize = 64 * 1024;
pub const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub const BODY_READ_TIMEOUT: Duration = Duration::from_secs(30);

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
    pub upgrade: Option<String>,
    pub sec_websocket_key: Option<String>,
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
                    "upgrade" => headers.upgrade = Some(value.to_lowercase()),
                    "sec-websocket-key" => headers.sec_websocket_key = Some(value.to_string()),
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
    let Some(request_line) =
        read_limited_line(reader, REQUEST_LINE_LIMIT, HEADER_READ_TIMEOUT).await?
    else {
        return Ok(None); // Connection closed
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() != 3 {
        return Err("Invalid request line".into());
    }

    let method = parts[0].to_string();
    let request_target = parts[1];
    let http_version = parts[2];
    if !is_http_token(&method) {
        return Err("invalid HTTP method".into());
    }
    if !matches!(http_version, "HTTP/1.0" | "HTTP/1.1") {
        return Err("unsupported HTTP version".into());
    }
    if !(request_target.starts_with('/') || request_target == "*") {
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
        if header_line.trim().is_empty() {
            break;
        }

        if header_line.starts_with([' ', '\t']) {
            return Err("obsolete folded headers are not supported".to_string());
        }
        let Some(colon_idx) = header_line.find(':') else {
            return Err("malformed HTTP header".to_string());
        };
        let name = header_line[..colon_idx].trim().to_lowercase();
        let value = header_line[colon_idx + 1..].trim();
        if !is_http_token(&name) {
            return Err("invalid HTTP header name".to_string());
        }
        if value.chars().any(|ch| ch.is_control() && ch != '\t') {
            return Err("invalid HTTP header value".to_string());
        }

        match name.as_str() {
            "host" => headers.host = Some(value.to_string()),
            "origin" => headers.origin = Some(value.to_string()),
            "referer" => headers.referer = Some(value.to_string()),
            "cookie" => headers.cookie = Some(value.to_string()),
            "content-type" => headers.content_type = Some(value.to_string()),
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
            "authorization" => headers.authorization = Some(value.to_string()),
            "x-api-key" => headers.x_api_key = Some(value.to_string()),
            "upgrade" => headers.upgrade = Some(value.to_lowercase()),
            "sec-websocket-key" => headers.sec_websocket_key = Some(value.to_string()),
            "sec-websocket-version" => {
                headers.sec_websocket_version = Some(value.to_string());
            }
            "connection" => headers.connection = value.to_lowercase(),
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
    let connection_header = if keep_alive {
        "Connection: keep-alive\r\n"
    } else {
        "Connection: close\r\n"
    };

    let body_bytes = response.body.as_bytes();
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
Content-Security-Policy: default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' data: https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self' ws: wss:\r\n\
Strict-Transport-Security: max-age=31536000; includeSubDomains\r\n",
        )
        .await
        .map_err(e)?;
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
            "Connection: keep-alive",
        ];
        let headers = HttpHeaders::from_lines(&lines);
        assert_eq!(headers.origin, Some("http://localhost:3000".to_string()));
        assert_eq!(headers.content_type, Some("application/json".to_string()));
        assert_eq!(headers.content_length, Some(256));
        assert_eq!(headers.authorization, Some("Bearer token123".to_string()));
        assert_eq!(headers.x_api_key, Some("key123".to_string()));
        assert_eq!(headers.connection, "keep-alive");
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
        assert!(raw.contains(&format!("Content-Length: {}\r\n", response.body.len())));
        assert!(raw.contains("Connection: keep-alive\r\n"));
        assert!(raw.ends_with(&response.body));
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
