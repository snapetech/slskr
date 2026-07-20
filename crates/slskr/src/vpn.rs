use std::{net::IpAddr, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::{header, StatusCode};
use serde::Deserialize;

use crate::config::{ControllerCompatibilityTarget, VpnIntegrationSettings};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Status {
    pub is_ready: bool,
    pub is_connected: bool,
    pub public_ip_address: Option<IpAddr>,
    pub location: String,
    pub forwarded_port: Option<u16>,
    pub port_forwards: Vec<PortForward>,
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

pub(crate) async fn poll_once(
    options: &VpnIntegrationSettings,
    target: ControllerCompatibilityTarget,
) -> Result<Status, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .timeout(Duration::from_millis(options.gluetun.timeout))
        .build()
        .map_err(|error| format!("failed to initialize Gluetun client: {error}"))?;

    let public_ip = get_json::<PublicIpResponse>(&client, options, "/v1/publicip/ip").await?;
    if public_ip.public_ip.is_empty() {
        return Ok(Status::default());
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

        if target == ControllerCompatibilityTarget::Slskdn {
            let response = request(
                &client,
                endpoint(&options.gluetun.url, "/v1/slskdn/portforwards"),
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
    async fn slskdn_poll_uses_api_key_and_projects_multi_forward_fallback() {
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
            ControllerCompatibilityTarget::Slskdn,
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
        assert!(requests[2].starts_with("GET /v1/slskdn/portforwards "));
    }

    #[tokio::test]
    async fn slskd_poll_uses_basic_auth_and_skips_multi_forward_endpoint() {
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
        let status = poll_once(&options, ControllerCompatibilityTarget::Slskd)
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
                ControllerCompatibilityTarget::Slskdn,
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
    async fn slskdn_missing_multi_forward_endpoint_is_compatible() {
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
            ControllerCompatibilityTarget::Slskdn,
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert!(status.is_ready);
        assert_eq!(status.forwarded_port, Some(45_678));
        assert!(status.port_forwards.is_empty());
    }
}
