//! Outbound relay-agent client.
//!
//! The controller side of relay is an authenticated SignalR hub plus two
//! token-bound HTTP workflows.  This module is the corresponding agent side:
//! it authenticates to the hub, publishes the local share snapshot, answers
//! file-upload requests, and receives completed-download notifications.

use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures_util::{
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt},
    SinkExt,
};
use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom},
    net::{lookup_host, TcpStream},
    sync::Semaphore,
    time,
};
use tokio_rustls::rustls;
use tokio_tungstenite::{
    client_async_tls_with_config,
    tungstenite::{protocol::WebSocketConfig, Message},
    Connector,
};
use x509_parser::prelude::parse_x509_certificate;

use crate::{
    config::{ControllerProfile, RelaySettings},
    relay, AppState,
};

const SIGNALR_RECORD_SEPARATOR: char = '\x1e';
const RELAY_RETRY_DELAY: Duration = Duration::from_secs(5);
const RELAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const RELAY_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const RELAY_FILE_CHUNK_BYTES: usize = 64 * 1024;
const MAX_RELAY_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_RELAY_SIGNALR_FRAME_BYTES: usize = 1024 * 1024;
const MAX_RELAY_MESSAGES_PER_FRAME: usize = 256;
const MAX_RELAY_FILENAME_BYTES: usize = 4 * 1024;
const MAX_RELAY_TOKEN_BYTES: usize = 512;
const MAX_RELAY_ERROR_BYTES: usize = 4 * 1024;
const MAX_RELAY_UPLOADS: usize = 16;

type RelaySocket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

pub(crate) fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        run(state).await;
    });
}

async fn run(state: Arc<AppState>) {
    loop {
        let settings = state.advanced_networking.read().await.relay.clone();
        if !settings.enabled || !matches!(settings.mode.as_str(), "agent" | "debug") {
            return;
        }
        if let Err(error) = run_connection(&state, &settings).await {
            tracing::warn!(%error, "relay agent connection stopped");
        }
        time::sleep(RELAY_RETRY_DELAY).await;
    }
}

async fn run_connection(state: &Arc<AppState>, settings: &RelaySettings) -> Result<(), String> {
    let instance_name = state.config.instance_name.trim().to_owned();
    if instance_name.is_empty() {
        return Err("relay agent instance name is empty".to_owned());
    }
    let target = state.config.controller_profile;
    let relay_target = time::timeout(
        RELAY_CONNECT_TIMEOUT,
        resolve_relay_target(&settings.controller.address),
    )
    .await
    .map_err(|_| "relay controller address resolution timed out".to_owned())??;
    let http_client = build_http_client(settings, target, &relay_target)?;
    let websocket_url =
        relay_websocket_url(&settings.controller.address, &settings.controller.api_key)?;
    let connector = if settings.controller.ignore_certificate_errors
        || !relay_pins(&settings.controller.pinned_spki).is_empty()
    {
        Some(Connector::Rustls(Arc::new(relay_tls_config(
            settings, target,
        )?)))
    } else {
        None
    };
    let websocket_config = WebSocketConfig::default()
        .max_message_size(Some(MAX_RELAY_SIGNALR_FRAME_BYTES))
        .max_frame_size(Some(MAX_RELAY_SIGNALR_FRAME_BYTES));
    let mut socket = time::timeout(
        RELAY_CONNECT_TIMEOUT,
        connect_relay_websocket(
            &websocket_url,
            &relay_target,
            Some(websocket_config),
            connector,
        ),
    )
    .await
    .map_err(|_| "relay controller websocket connection timed out".to_owned())?
    .map_err(|error| format!("relay controller websocket connection failed: {error}"))?;

    send_signalr_json(&mut socket, &json!({ "protocol": "json", "version": 1 })).await?;
    let challenge = time::timeout(RELAY_REQUEST_TIMEOUT, wait_for_challenge(&mut socket))
        .await
        .map_err(|_| "relay controller challenge timed out".to_owned())??;
    let login_id = "relay-login";
    send_invocation(
        &mut socket,
        login_id,
        "Login",
        vec![
            Value::String(instance_name.clone()),
            Value::String(relay::credential_for_target(
                target,
                &settings.controller.secret,
                &instance_name,
                &challenge,
            )),
        ],
    )
    .await?;
    time::timeout(
        RELAY_REQUEST_TIMEOUT,
        wait_for_completion(&mut socket, login_id),
    )
    .await
    .map_err(|_| "relay controller login timed out".to_owned())??;

    let share_token = "relay-share-token";
    send_invocation(&mut socket, share_token, "BeginShareUpload", Vec::new()).await?;
    let share_token = time::timeout(
        RELAY_REQUEST_TIMEOUT,
        wait_for_completion(&mut socket, share_token),
    )
    .await
    .map_err(|_| "relay controller share token request timed out".to_owned())??
    .and_then(|value| value.as_str().map(str::to_owned))
    .filter(|token| valid_relay_token(token))
    .ok_or_else(|| "relay controller returned an invalid share token".to_owned())?;
    upload_shares(
        state,
        settings,
        target,
        &http_client,
        &instance_name,
        &share_token,
    )
    .await?;

    let mut pending_uploads: FuturesUnordered<BoxFuture<'static, (String, Result<(), String>)>> =
        FuturesUnordered::new();
    let mut pending_failures = std::collections::VecDeque::new();
    let upload_slots = Arc::new(Semaphore::new(MAX_RELAY_UPLOADS));
    loop {
        while let Some((token, error)) = pending_failures.pop_front() {
            send_signalr_json(
                &mut socket,
                &json!({
                    "type": 1,
                    "target": "NotifyFileUploadFailed",
                    "arguments": [token, error],
                }),
            )
            .await?;
        }
        tokio::select! {
            messages = time::timeout(
                RELAY_REQUEST_TIMEOUT,
                next_signalr_messages(&mut socket),
            ) => {
                let messages = messages
                    .map_err(|_| "relay controller read timed out".to_owned())??;
                for message in messages {
                    if message.get("type").and_then(Value::as_u64) != Some(1) {
                        continue;
                    }
                    let target_name = message
                        .get("target")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if target_name == "RequestFileUpload" {
                        let (filename, start_offset, token) =
                            match relay_upload_request(&message) {
                                Ok(request) => request,
                                Err(error) => {
                                    if let Some(token) = relay_upload_failure_token(&message) {
                                        pending_failures.push_back((token, error));
                                    } else {
                                        tracing::warn!(%error, "relay upload request was invalid");
                                    }
                                    continue;
                                }
                            };
                        let Ok(upload_permit) = Arc::clone(&upload_slots).try_acquire_owned()
                        else {
                            pending_failures.push_back((
                                token,
                                "relay upload capacity reached; retry later".to_owned(),
                            ));
                            continue;
                        };
                        let task_state = Arc::clone(state);
                        let task_settings = settings.clone();
                        let task_client = http_client.clone();
                        let task_instance = instance_name.clone();
                        let task_token = token.clone();
                        pending_uploads.push(Box::pin(async move {
                            let _upload_permit = upload_permit;
                            let result = upload_file(
                                &task_state,
                                &task_settings,
                                target,
                                &task_client,
                                &task_instance,
                                &filename,
                                start_offset,
                                &task_token,
                            )
                            .await;
                            (task_token, result)
                        }));
                    } else {
                        handle_server_invocation(
                            state,
                            settings,
                            target,
                            &http_client,
                            &instance_name,
                            &mut socket,
                            &message,
                        )
                        .await?;
                    }
                }
            }
            completed = pending_uploads.next(), if !pending_uploads.is_empty() => {
                if let Some((token, Err(error))) = completed {
                    pending_failures.push_back((token, error));
                }
            }
        }
    }
}

fn build_http_client(
    settings: &RelaySettings,
    target: ControllerProfile,
    relay_target: &ResolvedRelayTarget,
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(RELAY_REQUEST_TIMEOUT)
        .no_proxy()
        .resolve_to_addrs(&relay_target.host, &relay_target.addrs);
    if settings.controller.ignore_certificate_errors
        || !relay_pins(&settings.controller.pinned_spki).is_empty()
    {
        builder = builder.use_preconfigured_tls(relay_tls_config(settings, target)?);
    }
    builder
        .build()
        .map_err(|error| format!("relay HTTP client construction failed: {error}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedRelayTarget {
    host: String,
    port: u16,
    addrs: Vec<SocketAddr>,
}

async fn resolve_relay_target(address: &str) -> Result<ResolvedRelayTarget, String> {
    let parsed = reqwest::Url::parse(address.trim().trim_end_matches('/'))
        .map_err(|error| format!("relay controller address is invalid: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("relay controller address must use http or https".to_owned());
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("relay controller address must not contain embedded credentials".to_owned());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "relay controller address must include a host".to_owned())?
        .to_owned();
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "relay controller address port is unknown".to_owned())?;
    let mut addrs = lookup_host((host.as_str(), port))
        .await
        .map_err(|error| format!("relay controller address resolution failed: {error}"))?
        .collect::<Vec<_>>();
    addrs.sort_unstable();
    addrs.dedup();
    if addrs.is_empty() {
        return Err("relay controller address did not resolve".to_owned());
    }
    Ok(ResolvedRelayTarget { host, port, addrs })
}

async fn connect_relay_websocket(
    websocket_url: &str,
    relay_target: &ResolvedRelayTarget,
    config: Option<WebSocketConfig>,
    connector: Option<Connector>,
) -> Result<RelaySocket, String> {
    let websocket = reqwest::Url::parse(websocket_url)
        .map_err(|error| format!("relay websocket URL is invalid: {error}"))?;
    let websocket_port = websocket
        .port_or_known_default()
        .ok_or_else(|| "relay websocket URL port is unknown".to_owned())?;
    if websocket.host_str() != Some(relay_target.host.as_str())
        || websocket_port != relay_target.port
    {
        return Err("relay websocket target does not match the resolved controller".to_owned());
    }
    let socket = TcpStream::connect(relay_target.addrs.as_slice())
        .await
        .map_err(|error| format!("relay controller websocket TCP connection failed: {error}"))?;
    socket
        .set_nodelay(true)
        .map_err(|error| format!("relay websocket TCP setup failed: {error}"))?;
    let (socket, _) = client_async_tls_with_config(websocket_url, socket, config, connector)
        .await
        .map_err(|error| format!("relay controller websocket handshake failed: {error}"))?;
    Ok(socket)
}

#[derive(Debug)]
struct AcceptAnyRelayCertificate {
    standard: Arc<rustls::client::WebPkiServerVerifier>,
}

impl rustls::client::danger::ServerCertVerifier for AcceptAnyRelayCertificate {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.standard.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.standard.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.standard.supported_verify_schemes()
    }
}

#[derive(Debug)]
struct PinnedRelayCertificate {
    standard: Arc<rustls::client::WebPkiServerVerifier>,
    pins: Vec<String>,
}

impl rustls::client::danger::ServerCertVerifier for PinnedRelayCertificate {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let Some(pin) = relay_certificate_pin(end_entity) else {
            return Err(rustls::Error::General(
                "relay controller certificate SPKI could not be parsed".to_owned(),
            ));
        };
        if !self.pins.iter().any(|expected| expected == &pin) {
            return Err(rustls::Error::General(
                "relay controller certificate SPKI pin mismatch".to_owned(),
            ));
        }
        let Ok((_, certificate)) = parse_x509_certificate(end_entity.as_ref()) else {
            return Err(rustls::Error::General(
                "relay controller certificate could not be parsed".to_owned(),
            ));
        };
        if !certificate.tbs_certificate.validity.is_valid() {
            return Err(rustls::Error::General(
                "relay controller certificate is outside its validity period".to_owned(),
            ));
        }
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.standard.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.standard.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.standard.supported_verify_schemes()
    }
}

fn relay_pins(raw: &str) -> Vec<String> {
    let mut pins = raw
        .split(',')
        .map(str::trim)
        .filter(|pin| !pin.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    pins.sort();
    pins.dedup();
    pins
}

fn der_tlv<'a>(input: &'a [u8], offset: &mut usize) -> Option<(u8, &'a [u8], &'a [u8])> {
    let start = *offset;
    let tag = *input.get(*offset)?;
    *offset += 1;
    let first = *input.get(*offset)?;
    *offset += 1;
    let length = if first & 0x80 == 0 {
        usize::from(first)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > std::mem::size_of::<usize>() {
            return None;
        }
        let mut length = 0_usize;
        for _ in 0..count {
            length = length
                .checked_mul(256)?
                .checked_add(usize::from(*input.get(*offset)?))?;
            *offset += 1;
        }
        length
    };
    let content_start = *offset;
    let end = content_start.checked_add(length)?;
    let content = input.get(content_start..end)?;
    *offset = end;
    Some((tag, input.get(start..end)?, content))
}

fn relay_certificate_pin(certificate: &rustls::pki_types::CertificateDer<'_>) -> Option<String> {
    let (_, parsed) = parse_x509_certificate(certificate.as_ref()).ok()?;
    let mut spki_offset = 0;
    let (_, _, spki_body) = der_tlv(parsed.tbs_certificate.subject_pki.raw, &mut spki_offset)?;
    let mut key_offset = 0;
    der_tlv(spki_body, &mut key_offset)?;
    let (_, encoded_key, _) = der_tlv(spki_body, &mut key_offset)?;
    Some(STANDARD.encode(Sha256::digest(encoded_key)))
}

fn relay_tls_config(
    settings: &RelaySettings,
    target: ControllerProfile,
) -> Result<rustls::ClientConfig, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let standard = rustls::client::WebPkiServerVerifier::builder_with_provider(
        Arc::new(roots.clone()),
        Arc::clone(&provider),
    )
    .build()
    .map_err(|error| format!("relay TLS verifier construction failed: {error}"))?;
    let pins = relay_pins(&settings.controller.pinned_spki);
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("relay TLS protocol configuration failed: {error}"))?;
    if !pins.is_empty() {
        return Ok(builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PinnedRelayCertificate { standard, pins }))
            .with_no_client_auth());
    }
    if settings.controller.ignore_certificate_errors {
        if target == ControllerProfile::Native {
            return crate::webhooks::self_issued_tls_config()
                .map_err(|error| format!("relay TLS configuration failed: {error}"));
        }
        return Ok(builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyRelayCertificate { standard }))
            .with_no_client_auth());
    }
    Ok(builder.with_root_certificates(roots).with_no_client_auth())
}

fn relay_websocket_url(address: &str, api_key: &str) -> Result<String, String> {
    let address = address.trim().trim_end_matches('/');
    let (scheme, rest) = if let Some(rest) = address.strip_prefix("https://") {
        ("wss", rest)
    } else if let Some(rest) = address.strip_prefix("http://") {
        ("ws", rest)
    } else {
        return Err("relay controller address must use http or https".to_owned());
    };
    if rest.is_empty() || api_key.is_empty() {
        return Err("relay controller address and API key are required".to_owned());
    }
    Ok(format!(
        "{scheme}://{rest}/hub/relay?access_token={}",
        crate::url_encode(api_key)
    ))
}

fn relay_http_url(address: &str, path: &str) -> Result<String, String> {
    let address = address.trim().trim_end_matches('/');
    if !(address.starts_with("http://") || address.starts_with("https://")) {
        return Err("relay controller address must use http or https".to_owned());
    }
    Ok(format!("{address}{path}"))
}

async fn send_signalr_json(socket: &mut RelaySocket, value: &Value) -> Result<(), String> {
    let text = signalr_json_text(value)?;
    time::timeout(
        RELAY_REQUEST_TIMEOUT,
        socket.send(Message::Text(text.into())),
    )
    .await
    .map_err(|_| "relay websocket send timed out".to_owned())?
    .map_err(|error| format!("relay websocket send failed: {error}"))
}

fn signalr_json_text(value: &Value) -> Result<String, String> {
    let mut text = value.to_string();
    text.push(SIGNALR_RECORD_SEPARATOR);
    if text.len() > MAX_RELAY_SIGNALR_FRAME_BYTES {
        return Err("relay SignalR frame exceeds the 1 MiB limit".to_owned());
    }
    Ok(text)
}

async fn send_invocation(
    socket: &mut RelaySocket,
    invocation_id: &str,
    target: &str,
    arguments: Vec<Value>,
) -> Result<(), String> {
    send_signalr_json(
        socket,
        &json!({
            "type": 1,
            "invocationId": invocation_id,
            "target": target,
            "arguments": arguments,
        }),
    )
    .await
}

async fn wait_for_challenge(socket: &mut RelaySocket) -> Result<String, String> {
    loop {
        for message in next_signalr_messages(socket).await? {
            if message.get("type").and_then(Value::as_u64) == Some(1)
                && message.get("target").and_then(Value::as_str) == Some("Challenge")
            {
                return message
                    .get("arguments")
                    .and_then(Value::as_array)
                    .and_then(|arguments| arguments.first())
                    .and_then(Value::as_str)
                    .filter(|challenge| valid_relay_token(challenge))
                    .map(str::to_owned)
                    .ok_or_else(|| "relay challenge payload is invalid".to_owned());
            }
        }
    }
}

async fn wait_for_completion(
    socket: &mut RelaySocket,
    invocation_id: &str,
) -> Result<Option<Value>, String> {
    loop {
        for message in next_signalr_messages(socket).await? {
            if message.get("type").and_then(Value::as_u64) != Some(3)
                || message.get("invocationId").and_then(Value::as_str) != Some(invocation_id)
            {
                continue;
            }
            if let Some(error) = message.get("error").and_then(Value::as_str) {
                return Err(format!(
                    "relay hub invocation failed: {}",
                    bounded_relay_error(error)
                ));
            }
            return Ok(message.get("result").cloned());
        }
    }
}

async fn next_signalr_messages(socket: &mut RelaySocket) -> Result<Vec<Value>, String> {
    loop {
        let Some(message) = socket.next().await else {
            return Err("relay controller websocket closed".to_owned());
        };
        let message =
            message.map_err(|error| format!("relay websocket receive failed: {error}"))?;
        match message {
            Message::Text(text) => {
                let values = parse_signalr_text(&text)?;
                if !values.is_empty() {
                    return Ok(values);
                }
            }
            Message::Ping(payload) => {
                socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|error| format!("relay websocket pong failed: {error}"))?;
            }
            Message::Close(_) => return Err("relay controller closed the websocket".to_owned()),
            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

fn parse_signalr_text(text: &str) -> Result<Vec<Value>, String> {
    if text.len() > MAX_RELAY_SIGNALR_FRAME_BYTES {
        return Err("relay SignalR frame exceeds the 1 MiB limit".to_owned());
    }
    let mut values = Vec::new();
    for part in text
        .split(SIGNALR_RECORD_SEPARATOR)
        .filter(|part| !part.is_empty())
    {
        if values.len() >= MAX_RELAY_MESSAGES_PER_FRAME {
            return Err("relay SignalR frame contains too many messages".to_owned());
        }
        values.push(
            serde_json::from_str::<Value>(part)
                .map_err(|error| format!("relay SignalR JSON is invalid: {error}"))?,
        );
    }
    Ok(values)
}

fn relay_upload_request(message: &Value) -> Result<(String, u64, String), String> {
    let arguments = message
        .get("arguments")
        .and_then(Value::as_array)
        .ok_or_else(|| "relay upload request arguments are missing".to_owned())?;
    let filename = arguments
        .first()
        .and_then(Value::as_str)
        .filter(|filename| valid_relay_filename(filename))
        .map(str::to_owned)
        .ok_or_else(|| "relay upload request filename is invalid".to_owned())?;
    let start_offset = arguments
        .get(1)
        .and_then(Value::as_u64)
        .ok_or_else(|| "relay upload request offset is invalid".to_owned())?;
    let token = arguments
        .get(2)
        .and_then(Value::as_str)
        .filter(|token| valid_relay_token(token))
        .map(str::to_owned)
        .ok_or_else(|| "relay upload request token is invalid".to_owned())?;
    Ok((filename, start_offset, token))
}

fn valid_relay_filename(filename: &str) -> bool {
    !filename.trim().is_empty()
        && filename.len() <= MAX_RELAY_FILENAME_BYTES
        && !filename.chars().any(char::is_control)
}

fn valid_relay_token(token: &str) -> bool {
    !token.trim().is_empty()
        && token.len() <= MAX_RELAY_TOKEN_BYTES
        && !token.chars().any(char::is_control)
}

fn bounded_relay_error(error: &str) -> String {
    if error.trim().is_empty()
        || error.len() > MAX_RELAY_ERROR_BYTES
        || error.chars().any(char::is_control)
    {
        "relay controller returned an invalid error".to_owned()
    } else {
        error.to_owned()
    }
}

fn relay_upload_failure_token(message: &Value) -> Option<String> {
    message
        .get("arguments")
        .and_then(Value::as_array)
        .and_then(|arguments| arguments.get(2))
        .and_then(Value::as_str)
        .filter(|token| valid_relay_token(token))
        .map(str::to_owned)
}

async fn upload_shares(
    state: &Arc<AppState>,
    settings: &RelaySettings,
    target: crate::config::ControllerProfile,
    client: &reqwest::Client,
    instance_name: &str,
    token: &str,
) -> Result<(), String> {
    let (share_roots, shares) = {
        let shares = state.shares.read().await;
        let roots = shares
            .roots
            .iter()
            .map(crate::controller_share_value)
            .collect::<Vec<_>>();
        let files = shares
            .entries
            .iter()
            .map(|entry| relay::RemoteShare {
                filename: entry.filename.clone(),
                size: entry.size,
            })
            .collect::<Vec<_>>();
        (roots, files)
    };
    let shares_json = serde_json::to_string(&share_roots)
        .map_err(|error| format!("relay share serialization failed: {error}"))?;
    let relay_directory = state.config.state_dir.join("relay");
    fs::create_dir_all(&relay_directory)
        .await
        .map_err(|error| format!("relay share database directory create failed: {error}"))?;
    let database_path =
        relay_directory.join(format!("agent-shares-{}.db", uuid::Uuid::new_v4().simple()));
    let mut database_guard = RelayTemporaryFile::new(database_path.clone());
    relay::write_share_database(&database_path, target, &shares).await?;
    let database_file = fs::File::open(&database_path)
        .await
        .map_err(|error| format!("relay share database open failed: {error}"))?;
    let stream = futures_util::stream::unfold(database_file, |mut file| async move {
        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(length) => {
                buffer.truncate(length);
                Some((Ok::<Vec<u8>, std::io::Error>(buffer), file))
            }
            Err(error) => Some((Err(error), file)),
        }
    });
    let database_part = Part::stream(reqwest::Body::wrap_stream(stream)).file_name("shares.db");
    let form = Form::new()
        .text("shares", shares_json)
        .part("database", database_part);
    let result = post_relay_form(
        client,
        settings,
        target,
        instance_name,
        token,
        "/api/v0/relay/controller/shares/",
        form,
    )
    .await;
    let cleanup = fs::remove_file(&database_path).await;
    match (result, cleanup) {
        (Ok(()), Ok(())) => {
            database_guard.commit();
            Ok(())
        }
        (Err(error), Ok(())) => {
            database_guard.commit();
            Err(error)
        }
        (Ok(()), Err(error)) => Err(format!(
            "relay share upload completed but temporary database cleanup failed: {error}"
        )),
        (Err(error), Err(cleanup_error)) => Err(format!(
            "{error}; temporary relay share database cleanup failed: {cleanup_error}"
        )),
    }
}

async fn handle_server_invocation(
    state: &Arc<AppState>,
    settings: &RelaySettings,
    runtime_profile: crate::config::ControllerProfile,
    client: &reqwest::Client,
    instance_name: &str,
    socket: &mut RelaySocket,
    message: &Value,
) -> Result<(), String> {
    let hub_target = message
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let arguments = message
        .get("arguments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    match hub_target {
        "RequestFileInfo" => {
            let filename = arguments
                .first()
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(token) = arguments
                .get(1)
                .and_then(Value::as_str)
                .filter(|token| valid_relay_token(token))
            else {
                tracing::warn!("relay file-info callback token was invalid");
                return Ok(());
            };
            let info = if valid_relay_filename(filename) {
                crate::find_shared_local_file(state, filename).await
            } else {
                None
            };
            send_signalr_json(
                socket,
                &json!({
                    "type": 1,
                    "target": "ReturnFileInfo",
                    "arguments": [
                        token,
                        info.is_some(),
                        info.as_ref().map(|file| file.size).unwrap_or(0),
                    ],
                }),
            )
            .await?;
        }
        "RequestFileUpload" => {
            let Some(filename) = arguments
                .first()
                .and_then(Value::as_str)
                .filter(|filename| valid_relay_filename(filename))
            else {
                tracing::warn!("relay upload callback filename was invalid");
                return Ok(());
            };
            let start_offset = arguments.get(1).and_then(Value::as_u64).unwrap_or(0);
            let Some(token) = arguments
                .get(2)
                .and_then(Value::as_str)
                .filter(|token| valid_relay_token(token))
            else {
                tracing::warn!("relay upload callback token was invalid");
                return Ok(());
            };
            if let Err(error) = upload_file(
                state,
                settings,
                runtime_profile,
                client,
                instance_name,
                filename,
                start_offset,
                token,
            )
            .await
            {
                send_signalr_json(
                    socket,
                    &json!({
                        "type": 1,
                        "target": "NotifyFileUploadFailed",
                        "arguments": [token, error],
                    }),
                )
                .await?;
            }
        }
        "NotifyFileDownloadCompleted" => {
            let filename = arguments
                .first()
                .and_then(Value::as_str)
                .unwrap_or_default();
            let token = arguments.get(1).and_then(Value::as_str).unwrap_or_default();
            if settings.controller.downloads
                && valid_relay_filename(filename)
                && valid_relay_token(token)
            {
                download_completed_file(
                    state,
                    settings,
                    runtime_profile,
                    client,
                    instance_name,
                    filename,
                    token,
                )
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn upload_file(
    state: &Arc<AppState>,
    settings: &RelaySettings,
    target: crate::config::ControllerProfile,
    client: &reqwest::Client,
    instance_name: &str,
    filename: &str,
    start_offset: u64,
    token: &str,
) -> Result<(), String> {
    if !valid_relay_filename(filename) {
        return Err("requested relay filename is invalid".to_owned());
    }
    if !valid_relay_token(token) {
        return Err("relay upload token is invalid".to_owned());
    }
    let shared = crate::find_shared_local_file(state, filename)
        .await
        .ok_or_else(|| "requested relay file was not found".to_owned())?;
    if start_offset > shared.size {
        return Err("relay file start offset exceeds file length".to_owned());
    }
    let file = crate::open_shared_local_file(state, &shared.local_path)
        .await
        .map_err(|error| format!("relay shared file open failed: {error}"))?;
    let mut file = fs::File::from_std(file);
    let metadata = file
        .metadata()
        .await
        .map_err(|error| format!("relay shared file metadata failed: {error}"))?;
    if !metadata.is_file() || metadata.len() != shared.size {
        return Err("relay shared file changed after share lookup".to_owned());
    }
    if start_offset > metadata.len() {
        return Err("relay file start offset exceeds file length".to_owned());
    }
    file.seek(SeekFrom::Start(start_offset))
        .await
        .map_err(|error| format!("relay shared file seek failed: {error}"))?;
    let stream = futures_util::stream::unfold(file, |mut file| async move {
        let mut buffer = vec![0_u8; RELAY_FILE_CHUNK_BYTES];
        match file.read(&mut buffer).await {
            Ok(0) => None,
            Ok(length) => {
                buffer.truncate(length);
                Some((Ok::<Vec<u8>, std::io::Error>(buffer), file))
            }
            Err(error) => Some((Err(error), file)),
        }
    });
    let part = Part::stream(reqwest::Body::wrap_stream(stream)).file_name(filename.to_owned());
    let form = Form::new().part("file", part);
    post_relay_form(
        client,
        settings,
        target,
        instance_name,
        token,
        "/api/v0/relay/controller/files/",
        form,
    )
    .await
}

async fn post_relay_form(
    client: &reqwest::Client,
    settings: &RelaySettings,
    target: crate::config::ControllerProfile,
    instance_name: &str,
    token: &str,
    path_prefix: &str,
    form: Form,
) -> Result<(), String> {
    if !valid_relay_token(token) {
        return Err("relay upload token is invalid".to_owned());
    }
    let url = relay_http_url(
        &settings.controller.address,
        &format!("{path_prefix}{}", crate::url_encode(token)),
    )?;
    let credential =
        relay::credential_for_target(target, &settings.controller.secret, instance_name, token);
    let response = client
        .post(url)
        .header("X-API-Key", &settings.controller.api_key)
        .header("X-Relay-Agent", instance_name)
        .header("X-Relay-Credential", credential)
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("relay upload request failed: {error}"))?;
    if response.status().is_success() {
        return Ok(());
    }
    Err(format!("relay upload returned HTTP {}", response.status()))
}

pub(crate) async fn download_completed_file(
    state: &Arc<AppState>,
    settings: &RelaySettings,
    target: crate::config::ControllerProfile,
    client: &reqwest::Client,
    instance_name: &str,
    filename: &str,
    token: &str,
) -> Result<(), String> {
    if !valid_relay_filename(filename) {
        return Err("relay download filename is invalid".to_owned());
    }
    if !valid_relay_token(token) {
        return Err("relay download token is invalid".to_owned());
    }
    let root = crate::effective_downloads_dir(state);
    let destination_name = if settings.mode == "debug" {
        format!("{filename}.relayed")
    } else {
        filename.to_owned()
    };
    let destination = crate::safe_download_path(&root, &destination_name)?;
    crate::ensure_scoped_download_path(&root, destination.to_string_lossy().as_ref())?;
    let temporary = destination.with_file_name(format!(
        ".{}.relay-{}.part",
        destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("download"),
        uuid::Uuid::new_v4().simple()
    ));
    let mut temporary_guard = RelayTemporaryFile::new(temporary.clone());
    let url = relay_http_url(
        &settings.controller.address,
        &format!(
            "/api/v0/relay/controller/downloads/{}",
            crate::url_encode(token)
        ),
    )?;
    let credential =
        relay::credential_for_target(target, &settings.controller.secret, instance_name, token);
    let response = client
        .get(url)
        .header("X-API-Key", &settings.controller.api_key)
        .header("X-Relay-Agent", instance_name)
        .header("X-Relay-Credential", credential)
        .header(
            "X-Relay-Filename-Base64",
            STANDARD.encode(filename.as_bytes()),
        )
        .send()
        .await
        .map_err(|error| format!("relay download request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "relay download returned HTTP {}",
            response.status()
        ));
    }
    let expected_length = response.content_length();
    let mut output_options = private_relay_download_options();
    output_options.write(true).create_new(true);
    let mut output = output_options
        .open(&temporary)
        .await
        .map_err(|error| format!("relay download destination create failed: {error}"))?;
    let mut bytes_written = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("relay download body failed: {error}"))?;
        bytes_written = bytes_written.saturating_add(chunk.len() as u64);
        if bytes_written > MAX_RELAY_DOWNLOAD_BYTES {
            return Err("relay download exceeds the 1 GiB limit".to_owned());
        }
        output
            .write_all(&chunk)
            .await
            .map_err(|error| format!("relay download write failed: {error}"))?;
    }
    output
        .flush()
        .await
        .map_err(|error| format!("relay download flush failed: {error}"))?;
    output
        .sync_all()
        .await
        .map_err(|error| format!("relay download sync failed: {error}"))?;
    if let Some(expected_length) = expected_length {
        if expected_length != bytes_written {
            return Err(format!(
                "relay download length mismatch: expected {expected_length}, received {bytes_written}"
            ));
        }
    }
    drop(output);
    fs::rename(&temporary, &destination)
        .await
        .map_err(|error| format!("relay download commit failed: {error}"))?;
    temporary_guard.commit();
    sync_download_directory(destination.parent().unwrap_or_else(|| Path::new("."))).await?;
    Ok(())
}

fn private_relay_download_options() -> fs::OpenOptions {
    let mut options = fs::OpenOptions::new();
    #[cfg(unix)]
    {
        options.mode(0o600);
    }
    options
}

#[cfg(unix)]
async fn sync_download_directory(path: &Path) -> Result<(), String> {
    let directory = fs::File::open(path)
        .await
        .map_err(|error| format!("relay download parent directory open failed: {error}"))?;
    directory
        .sync_all()
        .await
        .map_err(|error| format!("relay download parent directory sync failed: {error}"))
}

#[cfg(not(unix))]
async fn sync_download_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

struct RelayTemporaryFile {
    path: std::path::PathBuf,
    committed: bool,
}

impl RelayTemporaryFile {
    fn new(path: std::path::PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for RelayTemporaryFile {
    fn drop(&mut self) {
        if !self.committed {
            match std::fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    tracing::warn!(
                        path = %self.path.display(),
                        %error,
                        "relay temporary download cleanup failed"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signalr_parser_rejects_oversized_frames_and_message_batches() {
        let oversized = "x".repeat(MAX_RELAY_SIGNALR_FRAME_BYTES + 1);
        assert!(parse_signalr_text(&oversized)
            .expect_err("oversized frame")
            .contains("1 MiB"));

        let too_many = std::iter::repeat_n("{}", MAX_RELAY_MESSAGES_PER_FRAME + 1)
            .collect::<Vec<_>>()
            .join(&SIGNALR_RECORD_SEPARATOR.to_string());
        assert!(parse_signalr_text(&too_many)
            .expect_err("oversized message batch")
            .contains("too many messages"));
    }

    #[test]
    fn relay_upload_request_rejects_invalid_protocol_arguments() {
        let invalid = serde_json::json!({
            "arguments": ["track.flac", "not-an-offset", "upload-token"]
        });
        assert!(relay_upload_request(&invalid)
            .expect_err("invalid upload offset")
            .contains("offset"));

        let valid = serde_json::json!({
            "arguments": ["track.flac", 42, "upload-token"]
        });
        assert_eq!(
            relay_upload_request(&valid).expect("valid upload request"),
            ("track.flac".to_owned(), 42, "upload-token".to_owned())
        );

        let control_token = serde_json::json!({
            "arguments": ["track.flac", 42, "upload\n-token"]
        });
        assert!(relay_upload_request(&control_token)
            .expect_err("control character in upload token")
            .contains("token"));
    }

    #[test]
    fn relay_callbacks_bound_tokens_errors_and_outbound_frames() {
        assert!(valid_relay_token("relay-token"));
        assert!(!valid_relay_token("relay\n-token"));
        assert!(!valid_relay_token(&"x".repeat(MAX_RELAY_TOKEN_BYTES + 1)));
        assert_eq!(
            bounded_relay_error("relay failed"),
            "relay failed".to_owned()
        );
        assert_eq!(
            bounded_relay_error(&"x".repeat(MAX_RELAY_ERROR_BYTES + 1)),
            "relay controller returned an invalid error".to_owned()
        );
        assert!(signalr_json_text(&serde_json::json!({"ok": true}))
            .expect("small outbound frame")
            .ends_with(SIGNALR_RECORD_SEPARATOR));
        assert!(signalr_json_text(&serde_json::json!({
            "payload": "x".repeat(MAX_RELAY_SIGNALR_FRAME_BYTES)
        }))
        .expect_err("oversized outbound frame")
        .contains("1 MiB"));
    }

    #[tokio::test]
    async fn relay_controller_resolution_rejects_embedded_credentials() {
        let resolved = resolve_relay_target("http://127.0.0.1:4242")
            .await
            .expect("literal controller target");
        assert_eq!(resolved.host, "127.0.0.1");
        assert_eq!(resolved.port, 4242);
        assert_eq!(resolved.addrs, vec!["127.0.0.1:4242".parse().unwrap()]);
        assert!(resolve_relay_target("https://user:secret@127.0.0.1:4242")
            .await
            .expect_err("embedded controller credentials")
            .contains("embedded credentials"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn relay_download_staging_files_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let path = std::env::temp_dir().join(format!(
            "slskr-relay-download-mode-{}.part",
            uuid::Uuid::new_v4().simple()
        ));
        let file = private_relay_download_options()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .expect("create private relay download staging file");
        let mode = file
            .metadata()
            .await
            .expect("read relay staging file metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
        drop(file);
        fs::remove_file(path)
            .await
            .expect("remove relay staging file fixture");
    }

    #[test]
    fn relay_spki_pin_matches_certificate_public_key_value() {
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let pin = relay_certificate_pin(&certificate).expect("certificate SPKI pin");
        assert_eq!(pin.len(), 44);
        assert_eq!(relay_pins(&format!(" {pin}, {pin} ")), vec![pin]);
    }

    #[test]
    fn relay_tls_pin_parser_rejects_blank_values() {
        assert!(relay_pins(" ,\t,").is_empty());
        assert_eq!(relay_pins("z, a, z"), vec!["a", "z"]);
    }

    #[test]
    fn relay_temporary_file_guard_cleans_up_uncommitted_files() {
        let path = std::env::temp_dir().join(format!(
            "slskr-relay-temporary-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::write(&path, b"temporary").unwrap();
        drop(RelayTemporaryFile::new(path.clone()));
        assert!(!path.exists());

        std::fs::write(&path, b"committed").unwrap();
        let mut guard = RelayTemporaryFile::new(path.clone());
        guard.commit();
        drop(guard);
        assert_eq!(std::fs::read(&path).unwrap(), b"committed");
        std::fs::remove_file(path).unwrap();
    }
}
