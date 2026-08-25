//! Outbound relay-agent client.
//!
//! The controller side of relay is an authenticated SignalR hub plus two
//! token-bound HTTP workflows.  This module is the corresponding agent side:
//! it authenticates to the hub, publishes the local share snapshot, answers
//! file-upload requests, and receives completed-download notifications.

use std::{sync::Arc, time::Duration};

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
    time,
};
use tokio_rustls::rustls;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message, Connector};
use x509_parser::prelude::parse_x509_certificate;

use crate::{
    config::{ControllerProfile, RelaySettings},
    relay, AppState,
};

const SIGNALR_RECORD_SEPARATOR: char = '\x1e';
const RELAY_RETRY_DELAY: Duration = Duration::from_secs(5);
const RELAY_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const RELAY_FILE_CHUNK_BYTES: usize = 64 * 1024;
const MAX_RELAY_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;

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
    let http_client = build_http_client(settings, target)?;
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
    let (mut socket, _) = connect_async_tls_with_config(websocket_url, None, true, connector)
        .await
        .map_err(|error| format!("relay controller websocket connection failed: {error}"))?;

    send_signalr_json(&mut socket, &json!({ "protocol": "json", "version": 1 })).await?;
    let challenge = wait_for_challenge(&mut socket).await?;
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
    wait_for_completion(&mut socket, login_id).await?;

    let share_token = "relay-share-token";
    send_invocation(&mut socket, share_token, "BeginShareUpload", Vec::new()).await?;
    let share_token = wait_for_completion(&mut socket, share_token)
        .await?
        .and_then(|value| value.as_str().map(str::to_owned))
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
            messages = next_signalr_messages(&mut socket) => {
                for message in messages? {
                    if message.get("type").and_then(Value::as_u64) != Some(1) {
                        continue;
                    }
                    let target_name = message
                        .get("target")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if target_name == "RequestFileUpload" {
                        let filename = message
                            .get("arguments")
                            .and_then(Value::as_array)
                            .and_then(|arguments| arguments.first())
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let start_offset = message
                            .get("arguments")
                            .and_then(Value::as_array)
                            .and_then(|arguments| arguments.get(1))
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let token = message
                            .get("arguments")
                            .and_then(Value::as_array)
                            .and_then(|arguments| arguments.get(2))
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        let task_state = Arc::clone(state);
                        let task_settings = settings.clone();
                        let task_client = http_client.clone();
                        let task_instance = instance_name.clone();
                        let task_token = token.clone();
                        pending_uploads.push(Box::pin(async move {
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
) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(RELAY_REQUEST_TIMEOUT)
        .no_proxy();
    if settings.controller.ignore_certificate_errors
        || !relay_pins(&settings.controller.pinned_spki).is_empty()
    {
        builder = builder.use_preconfigured_tls(relay_tls_config(settings, target)?);
    }
    builder
        .build()
        .map_err(|error| format!("relay HTTP client construction failed: {error}"))
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
    let mut text = value.to_string();
    text.push(SIGNALR_RECORD_SEPARATOR);
    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|error| format!("relay websocket send failed: {error}"))
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
                return Err(format!("relay hub invocation failed: {error}"));
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
                let values = text
                    .split(SIGNALR_RECORD_SEPARATOR)
                    .filter(|part| !part.is_empty())
                    .map(|part| {
                        serde_json::from_str::<Value>(part)
                            .map_err(|error| format!("relay SignalR JSON is invalid: {error}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
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
    let _ = fs::remove_file(&database_path).await;
    result
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
            let token = arguments.get(1).and_then(Value::as_str).unwrap_or_default();
            let info = crate::find_shared_local_file(state, filename).await;
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
            let filename = arguments
                .first()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let start_offset = arguments.get(1).and_then(Value::as_u64).unwrap_or(0);
            let token = arguments
                .get(2)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if let Err(error) = upload_file(
                state,
                settings,
                runtime_profile,
                client,
                instance_name,
                &filename,
                start_offset,
                &token,
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
            if settings.controller.downloads && !filename.is_empty() && !token.is_empty() {
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
    let shared = crate::find_shared_local_file(state, filename)
        .await
        .ok_or_else(|| "requested relay file was not found".to_owned())?;
    if start_offset > shared.size {
        return Err("relay file start offset exceeds file length".to_owned());
    }
    let mut file = fs::File::open(&shared.local_path)
        .await
        .map_err(|error| format!("relay shared file open failed: {error}"))?;
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
    let mut output = fs::File::create(&temporary)
        .await
        .map_err(|error| format!("relay download destination create failed: {error}"))?;
    let mut bytes_written = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("relay download body failed: {error}"))?;
        bytes_written = bytes_written.saturating_add(chunk.len() as u64);
        if bytes_written > MAX_RELAY_DOWNLOAD_BYTES {
            let _ = fs::remove_file(&temporary).await;
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
    drop(output);
    fs::rename(&temporary, &destination)
        .await
        .map_err(|error| format!("relay download commit failed: {error}"))?;
    temporary_guard.commit();
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
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
