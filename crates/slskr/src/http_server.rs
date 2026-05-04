//! Optimized HTTP server with keep-alive, proper parsing, and streaming responses
//! Replaces manual HTTP parsing in main.rs

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use bytes::BytesMut;

use crate::routing::HttpResponse;

/// HTTP request parsed from stream
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: HttpHeaders,
    pub body: String,
}

/// HTTP headers
#[derive(Debug, Clone, Default)]
pub struct HttpHeaders {
    pub origin: Option<String>,
    pub content_type: Option<String>,
    pub content_length: Option<usize>,
    pub authorization: Option<String>,
    pub connection: String, // "keep-alive" or "close"
    pub user_agent: Option<String>,
}

impl HttpHeaders {
    /// Parse headers from raw HTTP header lines
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
                    "origin" => headers.origin = Some(value.to_string()),
                    "content-type" => headers.content_type = Some(value.to_string()),
                    "content-length" => {
                        headers.content_length = value.parse().ok();
                    }
                    "authorization" => headers.authorization = Some(value.to_string()),
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
    let header_lines: Vec<&str> = lines
        .by_ref()
        .take_while(|l| !l.is_empty())
        .collect();

    let headers = HttpHeaders::from_lines(&header_lines);

    Some((method, path, query, headers))
}

/// Read HTTP request from stream with proper buffering
pub async fn read_http_request(
    reader: &mut BufReader<&mut TcpStream>,
) -> Result<Option<(HttpRequest, bool)>, Box<dyn std::error::Error>> {
    let mut request_line = String::new();
    let bytes_read = reader.read_line(&mut request_line).await?;

    if bytes_read == 0 {
        return Ok(None); // Connection closed
    }

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid request line".into());
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

    // Read headers
    let mut headers = HttpHeaders {
        connection: "close".to_string(),
        ..Default::default()
    };
    let mut content_length = 0;

    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).await?;

        if header_line.trim().is_empty() {
            break; // End of headers
        }

        if let Some(colon_idx) = header_line.find(':') {
            let name = header_line[..colon_idx].trim().to_lowercase();
            let value = header_line[colon_idx + 1..].trim();

            match name.as_str() {
                "origin" => headers.origin = Some(value.to_string()),
                "content-type" => headers.content_type = Some(value.to_string()),
                "content-length" => {
                    content_length = value.parse().unwrap_or(0);
                    headers.content_length = Some(content_length);
                }
                "authorization" => headers.authorization = Some(value.to_string()),
                "connection" => headers.connection = value.to_lowercase(),
                "user-agent" => headers.user_agent = Some(value.to_string()),
                _ => {}
            }
        }
    }

    // Read body if content-length is set
    let body = if content_length > 0 {
        let mut buf = vec![0_u8; content_length];
        reader.read_exact(&mut buf).await?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        String::new()
    };

    let keep_alive = headers.connection.contains("keep-alive");

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

/// Write HTTP response to stream with minimal allocations (streaming)
pub async fn write_http_response(
    writer: &mut BufWriter<&mut TcpStream>,
    response: &HttpResponse,
    keep_alive: bool,
    extra_headers: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection_header = if keep_alive {
        "Connection: keep-alive\r\n"
    } else {
        "Connection: close\r\n"
    };

    let body_bytes = response.body.as_bytes();

    // Write status line
    writer.write_all(b"HTTP/1.1 ").await?;
    writer.write_all(response.status.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;

    // Write headers
    writer.write_all(b"Content-Type: ").await?;
    writer.write_all(response.content_type.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;

    writer.write_all(b"Content-Length: ").await?;
    writer.write_all(body_bytes.len().to_string().as_bytes()).await?;
    writer.write_all(b"\r\n").await?;

    writer.write_all(connection_header.as_bytes()).await?;
    writer.write_all(extra_headers.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;

    // Write body
    writer.write_all(body_bytes).await?;
    writer.flush().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "Connection: keep-alive",
        ];

        let headers = HttpHeaders::from_lines(&lines);

        assert_eq!(headers.origin, Some("http://localhost:3000".to_string()));
        assert_eq!(
            headers.content_type,
            Some("application/json".to_string())
        );
        assert_eq!(headers.content_length, Some(256));
        assert_eq!(
            headers.authorization,
            Some("Bearer token123".to_string())
        );
        assert_eq!(headers.connection, "keep-alive");
    }
}
