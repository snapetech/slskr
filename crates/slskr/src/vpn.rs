use std::{net::IpAddr, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{header, StatusCode};
use serde::Deserialize;

use crate::config::{ControllerProfile, VpnIntegrationSettings};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Status {
    pub is_ready: bool,
    pub is_connected: bool,
    pub public_ip_address: Option<IpAddr>,
    pub location: String,
    pub forwarded_port: Option<u16>,
    pub port_forwards: Vec<PortForward>,
    pub relay: Option<RelayStatus>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PortForward {
    pub slot: i32,
    pub local_port: i32,
    pub target_port: i32,
    pub proto: String,
    pub public_port: i32,
    pub public_ip_address: Option<IpAddr>,
    pub namespace: String,
}

impl PortForward {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "slot": self.slot,
            "localPort": self.local_port,
            "targetPort": self.target_port,
            "proto": self.proto,
            "publicPort": self.public_port,
            "publicIPAddress": self.public_ip_address.map(|value| value.to_string()),
            "namespace": self.namespace,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RelayStatus {
    pub mode: String,
    pub transport: String,
    pub connected: bool,
    pub latency_ms: Option<serde_json::Value>,
    pub rx_bytes: i64,
    pub tx_bytes: i64,
    pub active_connections: i32,
    pub connection_limit: i32,
    pub bandwidth_limit_mbit: i32,
    pub latest_handshake_at: Option<String>,
    pub path: String,
}

impl RelayStatus {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "mode": self.mode,
            "transport": self.transport,
            "connected": self.connected,
            "latencyMs": self.latency_ms,
            "rxBytes": self.rx_bytes,
            "txBytes": self.tx_bytes,
            "activeConnections": self.active_connections,
            "connectionLimit": self.connection_limit,
            "bandwidthLimitMbit": self.bandwidth_limit_mbit,
            "latestHandshakeAt": self.latest_handshake_at,
            "path": self.path,
        })
    }
}

#[derive(Debug, Deserialize)]
struct PublicIpResponse {
    #[serde(default)]
    public_ip: String,
    #[serde(default)]
    country: String,
    #[serde(default)]
    city: String,
}

#[derive(Debug, Deserialize)]
struct PortForwardResponse {
    port: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
struct PortForwardsResponse {
    #[serde(default)]
    forwards: Vec<PortForwardResponseN>,
}

#[derive(Debug, Deserialize)]
struct PortForwardResponseN {
    #[serde(default)]
    slot: i32,
    #[serde(default)]
    local_port: i32,
    #[serde(default)]
    target_port: i32,
    #[serde(default)]
    proto: String,
    #[serde(default)]
    public_port: i32,
    #[serde(default)]
    public_ip: String,
    #[serde(default)]
    namespace: String,
}

#[derive(Debug, Deserialize)]
struct RelayResponse {
    #[serde(default, alias = "Mode")]
    mode: String,
    #[serde(default, alias = "Transport")]
    transport: String,
    #[serde(default, alias = "Connected")]
    connected: bool,
    #[serde(default, alias = "latencyMs", alias = "LatencyMs")]
    latency_ms: Option<serde_json::Value>,
    #[serde(default, alias = "rxBytes", alias = "RxBytes")]
    rx_bytes: i64,
    #[serde(default, alias = "txBytes", alias = "TxBytes")]
    tx_bytes: i64,
    #[serde(default, alias = "activeConnections", alias = "ActiveConnections")]
    active_connections: i32,
    #[serde(default, alias = "connectionLimit", alias = "ConnectionLimit")]
    connection_limit: i32,
    #[serde(default, alias = "bandwidthLimitMbit", alias = "BandwidthLimitMbit")]
    bandwidth_limit_mbit: i32,
    #[serde(default, alias = "latestHandshakeAt", alias = "LatestHandshakeAt")]
    latest_handshake_at: Option<String>,
    #[serde(default, alias = "Path")]
    path: String,
}

fn endpoint(root: &str, path: &str) -> String {
    format!("{}{}", root.trim_end_matches('/'), path)
}

fn request(
    client: &reqwest::Client,
    url: String,
    options: &VpnIntegrationSettings,
) -> reqwest::RequestBuilder {
    let request = client.get(url);
    if !options.gluetun.api_key.trim().is_empty() {
        request.header("X-API-Key", &options.gluetun.api_key)
    } else if !options.gluetun.username.trim().is_empty() {
        let credentials = STANDARD.encode(format!(
            "{}:{}",
            options.gluetun.username, options.gluetun.password
        ));
        request.header(header::AUTHORIZATION, format!("Basic {credentials}"))
    } else {
        request
    }
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    options: &VpnIntegrationSettings,
    path: &str,
) -> Result<T, String> {
    let response = request(client, endpoint(&options.gluetun.url, path), options)
        .send()
        .await
        .map_err(|error| format!("Gluetun request failed: {error}"))?;
    let response = response
        .error_for_status()
        .map_err(|error| format!("Gluetun request failed: {error}"))?;
    response
        .json::<T>()
        .await
        .map_err(|error| format!("Unexpected Gluetun response: {error}"))
}

async fn get_optional_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    options: &VpnIntegrationSettings,
    path: &str,
) -> Result<Option<T>, String> {
    let response = request(client, endpoint(&options.gluetun.url, path), options)
        .send()
        .await
        .map_err(|error| format!("Gluetun request failed: {error}"))?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response
        .error_for_status()
        .map_err(|error| format!("Gluetun request failed: {error}"))?;
    response
        .json::<T>()
        .await
        .map(Some)
        .map_err(|error| format!("Unexpected Gluetun response: {error}"))
}

pub(crate) async fn poll_once(
    options: &VpnIntegrationSettings,
    target: ControllerProfile,
) -> Result<Status, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_millis(options.gluetun.timeout))
        .build()
        .map_err(|error| format!("failed to initialize Gluetun client: {error}"))?;

    let relay = if options.self_hosted_relay {
        get_optional_json::<RelayResponse>(&client, options, "/v1/slskr/relay")
            .await?
            .map(|relay| RelayStatus {
                mode: relay.mode,
                transport: relay.transport,
                connected: relay.connected,
                latency_ms: relay.latency_ms,
                rx_bytes: relay.rx_bytes,
                tx_bytes: relay.tx_bytes,
                active_connections: relay.active_connections,
                connection_limit: relay.connection_limit,
                bandwidth_limit_mbit: relay.bandwidth_limit_mbit,
                latest_handshake_at: relay.latest_handshake_at,
                path: relay.path,
            })
    } else {
        None
    };
    let public_ip = get_json::<PublicIpResponse>(&client, options, "/v1/publicip/ip").await?;
    if public_ip.public_ip.is_empty() {
        return Ok(Status {
            relay,
            ..Status::default()
        });
    }
    let parsed_public_ip = public_ip.public_ip.parse::<IpAddr>().map_err(|_| {
        format!(
            "Invalid public IP returned by Gluetun: {}",
            public_ip.public_ip
        )
    })?;

    let mut forwarded_port = None;
    let mut port_forwards = Vec::new();
    if options.port_forwarding {
        let primary = get_json::<PortForwardResponse>(&client, options, "/v1/portforward").await?;
        forwarded_port = primary
            .port
            .filter(|port| *port > 0)
            .and_then(|port| u16::try_from(port).ok());

        if target == ControllerProfile::Native {
            let response = request(
                &client,
                endpoint(&options.gluetun.url, "/v1/slskr/portforwards"),
                options,
            )
            .send()
            .await
            .map_err(|error| format!("Gluetun request failed: {error}"))?;
            if response.status() != StatusCode::NOT_FOUND {
                let response = response
                    .error_for_status()
                    .map_err(|error| format!("Gluetun request failed: {error}"))?;
                let multi = response
                    .json::<PortForwardsResponse>()
                    .await
                    .map_err(|error| format!("Unexpected Gluetun response: {error}"))?;
                port_forwards = multi
                    .forwards
                    .into_iter()
                    .filter(|forward| forward.public_port > 0)
                    .map(|forward| PortForward {
                        slot: forward.slot,
                        local_port: forward.local_port,
                        target_port: forward.target_port,
                        proto: forward.proto,
                        public_port: forward.public_port,
                        public_ip_address: forward.public_ip.parse().ok(),
                        namespace: forward.namespace,
                    })
                    .collect();
                if forwarded_port.is_none() {
                    forwarded_port = port_forwards
                        .iter()
                        .find(|forward| forward.slot == 0)
                        .and_then(|forward| u16::try_from(forward.public_port).ok());
                }
            }
        }
    }

    Ok(Status {
        is_ready: !options.port_forwarding || forwarded_port.is_some(),
        is_connected: true,
        public_ip_address: Some(parsed_public_ip),
        location: format!("{}, {}", public_ip.city, public_ip.country),
        forwarded_port,
        port_forwards,
        relay,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn options(url: String) -> VpnIntegrationSettings {
        VpnIntegrationSettings {
            enabled: true,
            port_forwarding: true,
            self_hosted_relay: false,
            polling_interval: 500,
            gluetun: crate::config::GluetunIntegrationSettings {
                url,
                timeout: 1_000,
                auth: String::new(),
                username: "user".to_owned(),
                password: "password".to_owned(),
                api_key: "api-secret".to_owned(),
            },
        }
    }

    #[tokio::test]
    async fn self_hosted_relay_status_is_optional_and_projects_all_fields() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in [
                r#"{"mode":"self-hosted-relay","transport":"tailscale","connected":true,"latencyMs":12.5,"rxBytes":123456,"txBytes":654321,"activeConnections":3,"connectionLimit":128,"bandwidthLimitMbit":100,"latestHandshakeAt":"2026-08-01T12:34:56Z","path":"direct"}"#,
                r#"{"public_ip":"203.0.113.8","city":"Regina","country":"Canada"}"#,
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).await.unwrap();
                requests.push(String::from_utf8_lossy(&buffer[..read]).into_owned());
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        let mut options = options(format!("http://{address}"));
        options.port_forwarding = false;
        options.self_hosted_relay = true;
        let status = poll_once(&options, ControllerProfile::Native)
            .await
            .unwrap();
        let relay = status.relay.unwrap();
        assert_eq!(relay.mode, "self-hosted-relay");
        assert_eq!(relay.transport, "tailscale");
        assert!(relay.connected);
        assert_eq!(relay.latency_ms, Some(serde_json::json!(12.5)));
        assert_eq!(relay.rx_bytes, 123_456);
        assert_eq!(relay.tx_bytes, 654_321);
        assert_eq!(relay.active_connections, 3);
        assert_eq!(relay.connection_limit, 128);
        assert_eq!(relay.bandwidth_limit_mbit, 100);
        assert_eq!(
            relay.latest_handshake_at.as_deref(),
            Some("2026-08-01T12:34:56Z")
        );
        assert_eq!(relay.path, "direct");
        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("GET /v1/slskr/relay "));
        assert!(requests[1].starts_with("GET /v1/publicip/ip "));
    }

    #[tokio::test]
    async fn native_poll_uses_api_key_and_projects_multi_forward_fallback() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in [
                r#"{"public_ip":"203.0.113.5","city":"Regina","country":"Canada"}"#,
                r#"{"port":0}"#,
                r#"{"forwards":[{"slot":0,"local_port":50300,"target_port":50300,"proto":"tcp","public_port":44444,"public_ip":"203.0.113.5","namespace":"slskdn"}]}"#,
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).await.unwrap();
                requests.push(String::from_utf8_lossy(&buffer[..read]).into_owned());
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        let status = poll_once(
            &options(format!("http://{address}")),
            ControllerProfile::Native,
        )
        .await
        .unwrap();
        assert!(status.is_connected);
        assert!(status.is_ready);
        assert_eq!(status.forwarded_port, Some(44_444));
        assert_eq!(status.port_forwards.len(), 1);
        let requests = server.await.unwrap();
        assert!(requests.iter().all(|request| request
            .to_ascii_lowercase()
            .contains("x-api-key: api-secret")));
        assert!(requests
            .iter()
            .all(|request| !request.to_ascii_lowercase().contains("authorization:")));
        assert!(requests[2].starts_with("GET /v1/slskr/portforwards "));
    }

    #[tokio::test]
    async fn controller_poll_uses_basic_auth_and_skips_multi_forward_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in [
                r#"{"public_ip":"203.0.113.6","city":"","country":""}"#,
                r#"{"port":55555}"#,
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0_u8; 4096];
                let read = stream.read(&mut buffer).await.unwrap();
                requests.push(String::from_utf8_lossy(&buffer[..read]).into_owned());
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        let mut options = options(format!("http://{address}"));
        options.gluetun.api_key.clear();
        let status = poll_once(&options, ControllerProfile::Legacy)
            .await
            .unwrap();
        assert_eq!(status.forwarded_port, Some(55_555));
        let requests = server.await.unwrap();
        assert!(requests.iter().all(|request| request
            .to_ascii_lowercase()
            .contains("authorization: basic dxnlcjpwyxnzd29yza==")));
    }

    #[tokio::test]
    async fn empty_public_ip_is_disconnected_and_invalid_ip_fails() {
        for (body, expected_error) in [
            (r#"{"public_ip":""}"#, false),
            (r#"{"public_ip":"not-an-ip"}"#, true),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer).await.unwrap();
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            });
            let result = poll_once(
                &options(format!("http://{address}")),
                ControllerProfile::Native,
            )
            .await;
            server.await.unwrap();
            assert_eq!(result.is_err(), expected_error);
            if let Ok(status) = result {
                assert!(!status.is_connected);
                assert!(!status.is_ready);
            }
        }
    }

    #[tokio::test]
    async fn native_missing_multi_forward_endpoint_is_compatible() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for (status, body) in [
                (
                    "200 OK",
                    r#"{"public_ip":"203.0.113.7","city":"Regina","country":"Canada"}"#,
                ),
                ("200 OK", r#"{"port":45678}"#),
                ("404 Not Found", ""),
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer).await.unwrap();
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });
        let status = poll_once(
            &options(format!("http://{address}")),
            ControllerProfile::Native,
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert!(status.is_ready);
        assert_eq!(status.forwarded_port, Some(45_678));
        assert!(status.port_forwards.is_empty());
    }
}
