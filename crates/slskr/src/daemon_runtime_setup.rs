use super::*;

pub(super) fn bind_http_listeners(addresses: &[SocketAddr]) -> Result<Vec<TcpListener>, String> {
    let has_ipv4 = addresses.iter().any(SocketAddr::is_ipv4);
    addresses
        .iter()
        .map(|address| {
            let socket = Socket::new(
                Domain::for_address(*address),
                Type::STREAM,
                Some(Protocol::TCP),
            )
            .map_err(|error| format!("failed to create HTTP socket for {address}: {error}"))?;
            socket.set_reuse_address(true).map_err(|error| {
                format!("failed to configure HTTP socket for {address}: {error}")
            })?;
            if address.is_ipv6() && has_ipv4 {
                socket.set_only_v6(true).map_err(|error| {
                    format!("failed to configure IPv6 HTTP socket for {address}: {error}")
                })?;
            }
            socket.set_nonblocking(true).map_err(|error| {
                format!("failed to configure HTTP socket for {address}: {error}")
            })?;
            socket
                .bind(&(*address).into())
                .map_err(|error| format!("failed to bind {address}: {error}"))?;
            socket
                .listen(256)
                .map_err(|error| format!("failed to listen on {address}: {error}"))?;
            TcpListener::from_std(socket.into())
                .map_err(|error| format!("failed to register HTTP socket for {address}: {error}"))
        })
        .collect()
}

pub(super) fn controller_tls_acceptor(
    settings: &crate::config::ControllerHttpsSettings,
) -> Result<tokio_rustls::TlsAcceptor, String> {
    use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

    let (certificates, private_key) = if let Some(path) = settings.certificate_pfx.as_deref() {
        let der = fs::read(path).map_err(|error| {
            format!(
                "failed to read HTTPS certificate {}: {error}",
                path.display()
            )
        })?;
        let bundle = openssl::pkcs12::Pkcs12::from_der(&der)
            .and_then(|bundle| bundle.parse2(&settings.certificate_password))
            .map_err(|error| format!("failed to parse HTTPS PFX certificate: {error}"))?;
        let certificate = bundle
            .cert
            .ok_or_else(|| "HTTPS PFX certificate contains no leaf certificate".to_owned())?;
        let private_key = bundle
            .pkey
            .ok_or_else(|| "HTTPS PFX certificate contains no private key".to_owned())?;
        let mut certificates =
            vec![CertificateDer::from(certificate.to_der().map_err(
                |error| format!("failed to decode HTTPS certificate: {error}"),
            )?)];
        if let Some(chain) = bundle.ca {
            for certificate in chain {
                certificates.push(CertificateDer::from(certificate.to_der().map_err(
                    |error| format!("failed to decode HTTPS certificate chain: {error}"),
                )?));
            }
        }
        let private_key = private_key
            .private_key_to_pkcs8()
            .map_err(|error| format!("failed to decode HTTPS private key: {error}"))?;
        (
            certificates,
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(private_key)),
        )
    } else {
        let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .map_err(|error| format!("failed to generate HTTPS certificate: {error}"))?;
        (
            vec![certified.cert.der().clone()],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
                certified.signing_key.serialize_der(),
            )),
        )
    };
    let server = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certificates, private_key)
        .map_err(|error| format!("invalid HTTPS certificate: {error}"))?;
    Ok(tokio_rustls::TlsAcceptor::from(Arc::new(server)))
}

#[cfg(unix)]
pub(super) fn bind_http_unix_listener(path: &Path) -> Result<tokio::net::UnixListener, String> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};

    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.file_type().is_socket() {
            return Err(format!(
                "refusing to replace non-socket web.socket path {}",
                path.display()
            ));
        }
        fs::remove_file(path)
            .map_err(|error| format!("failed to remove stale web socket: {error}"))?;
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create web socket directory: {error}"))?;
    }
    let listener = tokio::net::UnixListener::bind(path)
        .map_err(|error| format!("failed to bind web socket {}: {error}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
        .map_err(|error| format!("failed to set web socket permissions: {error}"))?;
    Ok(listener)
}

pub(super) async fn initialize_telemetry(config: &AppConfig) -> Result<(), String> {
    let tracing = &config.telemetry_tracing;
    if !tracing.enabled {
        return Ok(());
    }
    let now = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .max(0) as u64;
    let trace_id = uuid::Uuid::new_v4().simple().to_string();
    let span_id = &trace_id[..16];
    let payload = serde_json::json!({
        "resourceSpans": [{
            "resource": {"attributes": [{"key": "service.name", "value": {"stringValue": "slskr"}}]},
            "scopeSpans": [{"scope": {"name": "slskr"}, "spans": [{
                "traceId": trace_id,
                "spanId": span_id,
                "name": "slskr.startup",
                "kind": 1,
                "startTimeUnixNano": now.to_string(),
                "endTimeUnixNano": (now + 1).to_string(),
                "status": {"code": 1}
            }]}]
        }]
    });
    if tracing.exporter == "console" {
        eprintln!("{}", payload);
        return Ok(());
    }
    let configured = if tracing.exporter == "jaeger" {
        tracing.jaeger_endpoint.as_deref()
    } else {
        tracing.otlp_endpoint.as_deref()
    };
    let mut endpoint = configured
        .map(str::to_owned)
        .unwrap_or_else(|| "http://127.0.0.1:4318".to_owned());
    if !endpoint.contains("://") {
        endpoint = format!("http://{endpoint}");
    }
    if tracing.exporter == "jaeger"
        && tracing.jaeger_port.is_some()
        && endpoint
            .split_once("://")
            .is_some_and(|(_, authority)| !authority.contains(':'))
    {
        endpoint.push(':');
        endpoint.push_str(&tracing.jaeger_port.unwrap_or(4318).to_string());
    }
    if !endpoint.ends_with("/v1/traces") {
        endpoint = format!("{}/v1/traces", endpoint.trim_end_matches('/'));
    }
    let response = time::timeout(
        Duration::from_secs(5),
        reqwest::Client::builder()
            // Keep telemetry on the explicitly configured collector. A
            // redirect could otherwise move startup data to a different host.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| format!("telemetry exporter client failed: {error}"))?
            .post(endpoint)
            .json(&payload)
            .send(),
    )
    .await
    .map_err(|_| "telemetry exporter request timed out".to_owned())?
    .map_err(|error| format!("telemetry exporter request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "telemetry exporter returned HTTP {}",
            response.status()
        ));
    }
    Ok(())
}

pub(super) fn ensure_private_state_dir(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder
            .create(path)
            .map_err(|error| format!("failed to create state dir {}: {error}", path.display()))?;

        let metadata = std::fs::symlink_metadata(path)
            .map_err(|error| format!("failed to inspect state dir {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "state dir {} must be a real directory, not a symlink or file",
                path.display()
            ));
        }
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
            |error| {
                format!(
                    "failed to restrict state dir permissions for {}: {error}",
                    path.display()
                )
            },
        )?;
    }

    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(path)
            .map_err(|error| format!("failed to create state dir {}: {error}", path.display()))?;
        if !path.is_dir() {
            return Err(format!("state dir {} must be a directory", path.display()));
        }
    }

    Ok(())
}

pub(super) fn ensure_private_storage_dir(path: &Path, label: &str) -> Result<(), String> {
    let existed = match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(format!(
                    "{label} directory {} must be a real directory, not a symlink or file",
                    path.display()
                ));
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            return Err(format!(
                "failed to inspect {label} directory {}: {error}",
                path.display()
            ));
        }
    };
    if existed {
        return Ok(());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(path).map_err(|error| {
            format!(
                "failed to create {label} directory {}: {error}",
                path.display()
            )
        })?;
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(path).map_err(|error| {
        format!(
            "failed to create {label} directory {}: {error}",
            path.display()
        )
    })?;

    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        format!(
            "failed to inspect {label} directory {}: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!(
            "{label} directory {} must be a real directory, not a symlink or file",
            path.display()
        ));
    }
    Ok(())
}
