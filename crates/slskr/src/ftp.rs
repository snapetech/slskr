use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use suppaftp::tokio::{
    AsyncFtpStream, AsyncRustlsConnector, AsyncRustlsFtpStream, ImplAsyncFtpStream, TokioTlsStream,
};
use suppaftp::types::FileType;
use tokio_rustls::rustls;

use crate::config::{ControllerCompatibilityTarget, FtpIntegrationSettings};

#[derive(Debug)]
struct AcceptAnyServerCertificate {
    standard: Arc<rustls::client::WebPkiServerVerifier>,
}

impl rustls::client::danger::ServerCertVerifier for AcceptAnyServerCertificate {
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

fn public_root_tls_config() -> Result<rustls::ClientConfig, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("FTP TLS protocol configuration failed: {error}"))
        .map(|builder| builder.with_root_certificates(roots).with_no_client_auth())
}

fn accept_any_tls_config() -> Result<rustls::ClientConfig, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let standard = rustls::client::WebPkiServerVerifier::builder_with_provider(
        Arc::new(roots),
        Arc::clone(&provider),
    )
    .build()
    .map_err(|error| format!("FTP TLS verifier construction failed: {error}"))?;
    rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("FTP TLS protocol configuration failed: {error}"))
        .map(|builder| {
            builder
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(AcceptAnyServerCertificate { standard }))
                .with_no_client_auth()
        })
}

fn ftp_tls_config(
    options: &FtpIntegrationSettings,
    target: ControllerCompatibilityTarget,
) -> Result<rustls::ClientConfig, String> {
    if !options.ignore_certificate_errors {
        return public_root_tls_config();
    }
    match target {
        ControllerCompatibilityTarget::Slskd => accept_any_tls_config(),
        ControllerCompatibilityTarget::Slskdn => crate::webhooks::self_issued_tls_config()
            .map_err(|error| format!("FTP TLS verifier construction failed: {error}")),
    }
}

fn ftp_connector(
    options: &FtpIntegrationSettings,
    target: ControllerCompatibilityTarget,
) -> Result<AsyncRustlsConnector, String> {
    let config = ftp_tls_config(options, target)?;
    Ok(AsyncRustlsConnector::from(
        tokio_rustls::TlsConnector::from(Arc::new(config)),
    ))
}

fn ftp_endpoint(options: &FtpIntegrationSettings) -> String {
    if options.address.contains(':') && !options.address.starts_with('[') {
        format!("[{}]:{}", options.address, options.port)
    } else {
        format!("{}:{}", options.address, options.port)
    }
}

fn remote_upload_path(
    options: &FtpIntegrationSettings,
    local_path: &Path,
) -> Result<String, String> {
    let filename = local_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "completed download filename is not valid UTF-8".to_owned())?;
    let parent = local_path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let base = options.remote_path.trim_end_matches(['/', '\\']);
    let suffix = if parent.is_empty() {
        filename.to_owned()
    } else {
        format!("{parent}/{filename}")
    };
    Ok(format!("{base}/{suffix}").replace('\\', "/"))
}

async fn create_remote_directories<T>(ftp: &mut ImplAsyncFtpStream<T>, remote_filename: &str)
where
    T: TokioTlsStream + Send,
{
    let Some((directory, _)) = remote_filename.rsplit_once('/') else {
        return;
    };
    let absolute = directory.starts_with('/');
    let mut current = if absolute {
        "/".to_owned()
    } else {
        String::new()
    };
    for segment in directory.split('/').filter(|segment| !segment.is_empty()) {
        if !current.is_empty() && current != "/" {
            current.push('/');
        }
        current.push_str(segment);
        let _ = ftp.mkdir(&current).await;
    }
}

async fn upload_on_stream<T>(
    mut ftp: ImplAsyncFtpStream<T>,
    options: &FtpIntegrationSettings,
    local_path: &Path,
) -> Result<(), String>
where
    T: TokioTlsStream + Send,
{
    ftp.login(&options.username, &options.password)
        .await
        .map_err(|error| format!("FTP login failed: {error}"))?;
    ftp.transfer_type(FileType::Binary)
        .await
        .map_err(|error| format!("FTP binary transfer setup failed: {error}"))?;
    let remote_filename = remote_upload_path(options, local_path)?;
    create_remote_directories(&mut ftp, &remote_filename).await;
    if !options.overwrite_existing
        && ftp
            .nlst(Some(&remote_filename))
            .await
            .is_ok_and(|entries| !entries.is_empty())
    {
        let _ = ftp.quit().await;
        return Ok(());
    }
    let mut file = tokio::fs::File::open(local_path)
        .await
        .map_err(|error| format!("completed download open failed: {error}"))?;
    ftp.put_file(&remote_filename, &mut file)
        .await
        .map_err(|error| format!("FTP upload failed: {error}"))?;
    let _ = ftp.quit().await;
    Ok(())
}

async fn attempt_upload(
    options: &FtpIntegrationSettings,
    target: ControllerCompatibilityTarget,
    local_path: &Path,
) -> Result<(), String> {
    let endpoint = ftp_endpoint(options);
    let timeout = Duration::from_millis(options.connection_timeout);
    match options.encryption_mode.to_ascii_lowercase().as_str() {
        "none" => {
            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
                .await
                .map_err(|_| "FTP connection timed out".to_owned())?
                .map_err(|error| format!("FTP connection failed: {error}"))?;
            upload_on_stream(ftp, options, local_path).await
        }
        "implicit" => {
            let connector = ftp_connector(options, target)?;
            let ftp = tokio::time::timeout(
                timeout,
                AsyncRustlsFtpStream::connect_secure_implicit(
                    &endpoint,
                    connector,
                    &options.address,
                ),
            )
            .await
            .map_err(|_| "FTP connection timed out".to_owned())?
            .map_err(|error| format!("implicit FTPS connection failed: {error}"))?;
            upload_on_stream(ftp, options, local_path).await
        }
        "explicit" => {
            let ftp = tokio::time::timeout(timeout, AsyncRustlsFtpStream::connect(&endpoint))
                .await
                .map_err(|_| "FTP connection timed out".to_owned())?
                .map_err(|error| format!("FTP connection failed: {error}"))?;
            let ftp = tokio::time::timeout(
                timeout,
                ftp.into_secure(ftp_connector(options, target)?, &options.address),
            )
            .await
            .map_err(|_| "explicit FTPS negotiation timed out".to_owned())?
            .map_err(|error| format!("explicit FTPS negotiation failed: {error}"))?;
            upload_on_stream(ftp, options, local_path).await
        }
        "auto" => {
            let secure = async {
                let ftp = AsyncRustlsFtpStream::connect(&endpoint).await?;
                ftp.into_secure(
                    ftp_connector(options, target).map_err(suppaftp::FtpError::SecureError)?,
                    &options.address,
                )
                .await
            };
            if let Ok(Ok(ftp)) = tokio::time::timeout(timeout, secure).await {
                return upload_on_stream(ftp, options, local_path).await;
            }
            let ftp = tokio::time::timeout(timeout, AsyncFtpStream::connect(&endpoint))
                .await
                .map_err(|_| "FTP connection timed out".to_owned())?
                .map_err(|error| format!("FTP connection failed: {error}"))?;
            upload_on_stream(ftp, options, local_path).await
        }
        _ => Err("FTP encryption mode is invalid".to_owned()),
    }
}

pub async fn upload_completed_file(
    options: &FtpIntegrationSettings,
    target: ControllerCompatibilityTarget,
    local_path: &Path,
) -> Result<(), String> {
    if !options.enabled {
        return Ok(());
    }
    let attempts = match target {
        ControllerCompatibilityTarget::Slskd => options.retry_attempts,
        ControllerCompatibilityTarget::Slskdn => options.retry_attempts.max(1),
    };
    let mut last_error = "FTP upload was not attempted".to_owned();
    for attempt in 0..attempts {
        if attempt > 0 {
            let delay = (1_u64 << attempt.min(5)).saturating_mul(500).min(30_000);
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }
        match attempt_upload(options, target, local_path).await {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn spawn_plain_ftp_fixture(
        existing: bool,
    ) -> (
        std::net::SocketAddr,
        tokio::task::JoinHandle<(Vec<String>, Vec<u8>)>,
    ) {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (control, _) = listener.accept().await.unwrap();
            let mut control = BufReader::new(control);
            control
                .get_mut()
                .write_all(b"220 fixture ready\r\n")
                .await
                .unwrap();
            let mut commands = Vec::new();
            let mut uploaded = Vec::new();
            let mut passive = None;
            loop {
                let mut line = String::new();
                if control.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                let command = line.trim_end_matches(['\r', '\n']).to_owned();
                commands.push(command.clone());
                let verb = command
                    .split_once(' ')
                    .map_or(command.as_str(), |(verb, _)| verb);
                match verb {
                    "USER" => control
                        .get_mut()
                        .write_all(b"331 password required\r\n")
                        .await
                        .unwrap(),
                    "PASS" => control
                        .get_mut()
                        .write_all(b"230 logged in\r\n")
                        .await
                        .unwrap(),
                    "TYPE" => control
                        .get_mut()
                        .write_all(b"200 type set\r\n")
                        .await
                        .unwrap(),
                    "MKD" => control
                        .get_mut()
                        .write_all(b"257 directory ready\r\n")
                        .await
                        .unwrap(),
                    "PASV" => {
                        let data = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let port = data.local_addr().unwrap().port();
                        control
                            .get_mut()
                            .write_all(
                                format!(
                                    "227 Entering Passive Mode (127,0,0,1,{},{}).\r\n",
                                    port / 256,
                                    port % 256
                                )
                                .as_bytes(),
                            )
                            .await
                            .unwrap();
                        passive = Some(data);
                    }
                    "NLST" => {
                        control
                            .get_mut()
                            .write_all(b"150 listing\r\n")
                            .await
                            .unwrap();
                        let (mut data, _) = passive.take().unwrap().accept().await.unwrap();
                        if existing {
                            data.write_all(format!("{}\r\n", &command[5..]).as_bytes())
                                .await
                                .unwrap();
                        }
                        drop(data);
                        control
                            .get_mut()
                            .write_all(b"226 listing complete\r\n")
                            .await
                            .unwrap();
                    }
                    "STOR" => {
                        control
                            .get_mut()
                            .write_all(b"150 upload ready\r\n")
                            .await
                            .unwrap();
                        let (mut data, _) = passive.take().unwrap().accept().await.unwrap();
                        data.read_to_end(&mut uploaded).await.unwrap();
                        control
                            .get_mut()
                            .write_all(b"226 upload complete\r\n")
                            .await
                            .unwrap();
                    }
                    "QUIT" => {
                        control.get_mut().write_all(b"221 bye\r\n").await.unwrap();
                        break;
                    }
                    _ => panic!("unexpected FTP command: {command}"),
                }
            }
            (commands, uploaded)
        });
        (address, server)
    }

    async fn serve_secure_ftp_control<S>(
        stream: S,
        acceptor: tokio_rustls::TlsAcceptor,
        send_greeting: bool,
    ) -> (Vec<String>, Vec<u8>)
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

        let mut control = BufReader::new(stream);
        if send_greeting {
            control
                .get_mut()
                .write_all(b"220 secure fixture ready\r\n")
                .await
                .unwrap();
        }
        let mut commands = Vec::new();
        let mut uploaded = Vec::new();
        let mut passive = None;
        loop {
            let mut line = String::new();
            if control.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            let command = line.trim_end_matches(['\r', '\n']).to_owned();
            commands.push(command.clone());
            let verb = command
                .split_once(' ')
                .map_or(command.as_str(), |(verb, _)| verb);
            match verb {
                "PBSZ" | "PROT" | "TYPE" => control
                    .get_mut()
                    .write_all(b"200 command accepted\r\n")
                    .await
                    .unwrap(),
                "USER" => control
                    .get_mut()
                    .write_all(b"331 password required\r\n")
                    .await
                    .unwrap(),
                "PASS" => control
                    .get_mut()
                    .write_all(b"230 logged in\r\n")
                    .await
                    .unwrap(),
                "MKD" => control
                    .get_mut()
                    .write_all(b"257 directory ready\r\n")
                    .await
                    .unwrap(),
                "PASV" => {
                    let data = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    let port = data.local_addr().unwrap().port();
                    control
                        .get_mut()
                        .write_all(
                            format!(
                                "227 Entering Passive Mode (127,0,0,1,{},{}).\r\n",
                                port / 256,
                                port % 256
                            )
                            .as_bytes(),
                        )
                        .await
                        .unwrap();
                    passive = Some(data);
                }
                "STOR" => {
                    control
                        .get_mut()
                        .write_all(b"150 upload ready\r\n")
                        .await
                        .unwrap();
                    let (data, _) = passive.take().unwrap().accept().await.unwrap();
                    let mut data = acceptor.accept(data).await.unwrap();
                    data.read_to_end(&mut uploaded).await.unwrap();
                    control
                        .get_mut()
                        .write_all(b"226 upload complete\r\n")
                        .await
                        .unwrap();
                }
                "QUIT" => {
                    control.get_mut().write_all(b"221 bye\r\n").await.unwrap();
                    break;
                }
                _ => panic!("unexpected secure FTP command: {command}"),
            }
        }
        (commands, uploaded)
    }

    async fn spawn_ftps_fixture(
        certificate_host: &str,
        implicit: bool,
    ) -> (
        std::net::SocketAddr,
        tokio::task::JoinHandle<Option<(Vec<String>, Vec<u8>)>>,
    ) {
        use rcgen::generate_simple_self_signed;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio_rustls::rustls::{pki_types::PrivatePkcs8KeyDer, ServerConfig};

        let certified = generate_simple_self_signed(vec![certificate_host.to_owned()]).unwrap();
        let certificate = certified.cert.der().clone();
        let private_key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
        let config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![certificate], private_key.into())
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            if implicit {
                let tls = acceptor.accept(tcp).await.ok()?;
                Some(serve_secure_ftp_control(tls, acceptor, true).await)
            } else {
                let mut plain = BufReader::new(tcp);
                plain
                    .get_mut()
                    .write_all(b"220 explicit fixture ready\r\n")
                    .await
                    .unwrap();
                let mut auth = String::new();
                plain.read_line(&mut auth).await.unwrap();
                assert_eq!(auth, "AUTH TLS\r\n");
                plain
                    .get_mut()
                    .write_all(b"234 TLS accepted\r\n")
                    .await
                    .unwrap();
                let tls = acceptor.accept(plain.into_inner()).await.ok()?;
                Some(serve_secure_ftp_control(tls, acceptor, false).await)
            }
        });
        (address, server)
    }

    async fn spawn_auto_fallback_fixture() -> (
        std::net::SocketAddr,
        tokio::task::JoinHandle<(Vec<String>, Vec<u8>)>,
    ) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (first, _) = listener.accept().await.unwrap();
            let mut first = BufReader::new(first);
            first
                .get_mut()
                .write_all(b"220 plain-only fixture ready\r\n")
                .await
                .unwrap();
            let mut auth = String::new();
            first.read_line(&mut auth).await.unwrap();
            assert_eq!(auth, "AUTH TLS\r\n");
            first
                .get_mut()
                .write_all(b"500 TLS unavailable\r\n")
                .await
                .unwrap();
            drop(first);

            let (control, _) = listener.accept().await.unwrap();
            let nested = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            drop(nested);
            let mut control = BufReader::new(control);
            control
                .get_mut()
                .write_all(b"220 fallback ready\r\n")
                .await
                .unwrap();
            let mut commands = Vec::new();
            let mut uploaded = Vec::new();
            let mut passive = None;
            loop {
                let mut line = String::new();
                if control.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                let command = line.trim_end_matches(['\r', '\n']).to_owned();
                commands.push(command.clone());
                let verb = command
                    .split_once(' ')
                    .map_or(command.as_str(), |(verb, _)| verb);
                match verb {
                    "USER" => control
                        .get_mut()
                        .write_all(b"331 password required\r\n")
                        .await
                        .unwrap(),
                    "PASS" => control
                        .get_mut()
                        .write_all(b"230 logged in\r\n")
                        .await
                        .unwrap(),
                    "TYPE" => control
                        .get_mut()
                        .write_all(b"200 type set\r\n")
                        .await
                        .unwrap(),
                    "MKD" => control
                        .get_mut()
                        .write_all(b"257 directory ready\r\n")
                        .await
                        .unwrap(),
                    "PASV" => {
                        let data = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let port = data.local_addr().unwrap().port();
                        control
                            .get_mut()
                            .write_all(
                                format!(
                                    "227 Entering Passive Mode (127,0,0,1,{},{}).\r\n",
                                    port / 256,
                                    port % 256
                                )
                                .as_bytes(),
                            )
                            .await
                            .unwrap();
                        passive = Some(data);
                    }
                    "STOR" => {
                        use tokio::io::AsyncReadExt;
                        control
                            .get_mut()
                            .write_all(b"150 upload ready\r\n")
                            .await
                            .unwrap();
                        let (mut data, _) = passive.take().unwrap().accept().await.unwrap();
                        data.read_to_end(&mut uploaded).await.unwrap();
                        control
                            .get_mut()
                            .write_all(b"226 upload complete\r\n")
                            .await
                            .unwrap();
                    }
                    "QUIT" => {
                        control.get_mut().write_all(b"221 bye\r\n").await.unwrap();
                        break;
                    }
                    _ => panic!("unexpected fallback FTP command: {command}"),
                }
            }
            (commands, uploaded)
        });
        (address, server)
    }

    #[test]
    fn remote_path_matches_frozen_parent_directory_shape() {
        let options = FtpIntegrationSettings {
            remote_path: "/incoming/".to_owned(),
            ..FtpIntegrationSettings::default()
        };
        assert_eq!(
            remote_upload_path(&options, Path::new("/downloads/Album/track.flac")).unwrap(),
            "/incoming/Album/track.flac"
        );
    }

    #[tokio::test]
    async fn completed_file_upload_uses_binary_passive_ftp_and_frozen_remote_shape() {
        let root = std::env::temp_dir().join(format!("slskr-ftp-{}", uuid::Uuid::new_v4()));
        let album = root.join("Album");
        tokio::fs::create_dir_all(&album).await.unwrap();
        let file = album.join("track.flac");
        tokio::fs::write(&file, b"fixture-audio").await.unwrap();
        let (address, server) = spawn_plain_ftp_fixture(false).await;
        let options = FtpIntegrationSettings {
            enabled: true,
            address: address.ip().to_string(),
            port: address.port(),
            encryption_mode: "none".to_owned(),
            username: "fixture-user".to_owned(),
            password: "fixture-password".to_owned(),
            remote_path: "/incoming".to_owned(),
            retry_attempts: 1,
            ..FtpIntegrationSettings::default()
        };
        upload_completed_file(&options, ControllerCompatibilityTarget::Slskdn, &file)
            .await
            .unwrap();
        let (commands, uploaded) = server.await.unwrap();
        assert!(commands.contains(&"USER fixture-user".to_owned()));
        assert!(commands.contains(&"PASS fixture-password".to_owned()));
        assert!(commands.contains(&"TYPE I".to_owned()));
        assert!(commands.contains(&"STOR /incoming/Album/track.flac".to_owned()));
        assert_eq!(uploaded, b"fixture-audio");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn overwrite_false_skips_an_existing_remote_file() {
        let root = std::env::temp_dir().join(format!("slskr-ftp-{}", uuid::Uuid::new_v4()));
        let album = root.join("Album");
        tokio::fs::create_dir_all(&album).await.unwrap();
        let file = album.join("track.flac");
        tokio::fs::write(&file, b"must-not-upload").await.unwrap();
        let (address, server) = spawn_plain_ftp_fixture(true).await;
        let options = FtpIntegrationSettings {
            enabled: true,
            address: address.ip().to_string(),
            port: address.port(),
            encryption_mode: "none".to_owned(),
            username: "fixture-user".to_owned(),
            password: "fixture-password".to_owned(),
            remote_path: "/incoming".to_owned(),
            overwrite_existing: false,
            retry_attempts: 1,
            ..FtpIntegrationSettings::default()
        };
        upload_completed_file(&options, ControllerCompatibilityTarget::Slskdn, &file)
            .await
            .unwrap();
        let (commands, uploaded) = server.await.unwrap();
        assert!(commands.contains(&"NLST /incoming/Album/track.flac".to_owned()));
        assert!(!commands.iter().any(|command| command.starts_with("STOR ")));
        assert!(uploaded.is_empty());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn ftps_modes_and_target_certificate_policies_match_frozen_profiles() {
        async fn local_file() -> (std::path::PathBuf, std::path::PathBuf) {
            let root = std::env::temp_dir().join(format!("slskr-ftps-{}", uuid::Uuid::new_v4()));
            let album = root.join("Album");
            tokio::fs::create_dir_all(&album).await.unwrap();
            let file = album.join("track.flac");
            tokio::fs::write(&file, b"secure-audio").await.unwrap();
            (root, file)
        }
        fn options(address: std::net::SocketAddr, mode: &str) -> FtpIntegrationSettings {
            FtpIntegrationSettings {
                enabled: true,
                address: address.ip().to_string(),
                port: address.port(),
                encryption_mode: mode.to_owned(),
                ignore_certificate_errors: true,
                username: "fixture-user".to_owned(),
                password: "fixture-password".to_owned(),
                remote_path: "/incoming".to_owned(),
                retry_attempts: 1,
                ..FtpIntegrationSettings::default()
            }
        }

        for (mode, implicit) in [("explicit", false), ("implicit", true)] {
            let (root, file) = local_file().await;
            let (address, server) = spawn_ftps_fixture("127.0.0.1", implicit).await;
            upload_completed_file(
                &options(address, mode),
                ControllerCompatibilityTarget::Slskdn,
                &file,
            )
            .await
            .unwrap();
            let (commands, uploaded) = server.await.unwrap().unwrap();
            if !implicit {
                assert!(commands.contains(&"PROT P".to_owned()), "{commands:?}");
            }
            assert!(commands.contains(&"STOR /incoming/Album/track.flac".to_owned()));
            assert_eq!(uploaded, b"secure-audio");
            tokio::fs::remove_dir_all(root).await.unwrap();
        }

        let (root, file) = local_file().await;
        let (address, server) = spawn_ftps_fixture("wrong.invalid", false).await;
        assert!(upload_completed_file(
            &options(address, "explicit"),
            ControllerCompatibilityTarget::Slskdn,
            &file,
        )
        .await
        .is_err());
        assert!(server.await.unwrap().is_none());
        tokio::fs::remove_dir_all(root).await.unwrap();

        let (root, file) = local_file().await;
        let (address, server) = spawn_ftps_fixture("wrong.invalid", false).await;
        upload_completed_file(
            &options(address, "explicit"),
            ControllerCompatibilityTarget::Slskd,
            &file,
        )
        .await
        .unwrap();
        let (_, uploaded) = server.await.unwrap().unwrap();
        assert_eq!(uploaded, b"secure-audio");
        tokio::fs::remove_dir_all(root).await.unwrap();

        let (root, file) = local_file().await;
        let (address, server) = spawn_ftps_fixture("127.0.0.1", false).await;
        let mut strict = options(address, "explicit");
        strict.ignore_certificate_errors = false;
        assert!(
            upload_completed_file(&strict, ControllerCompatibilityTarget::Slskdn, &file)
                .await
                .is_err()
        );
        assert!(server.await.unwrap().is_none());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn auto_encryption_falls_back_to_plain_ftp_when_auth_tls_is_unavailable() {
        let root = std::env::temp_dir().join(format!("slskr-ftp-{}", uuid::Uuid::new_v4()));
        let album = root.join("Album");
        tokio::fs::create_dir_all(&album).await.unwrap();
        let file = album.join("track.flac");
        tokio::fs::write(&file, b"fallback-audio").await.unwrap();
        let (address, server) = spawn_auto_fallback_fixture().await;
        let options = FtpIntegrationSettings {
            enabled: true,
            address: address.ip().to_string(),
            port: address.port(),
            encryption_mode: "auto".to_owned(),
            username: "fixture-user".to_owned(),
            password: "fixture-password".to_owned(),
            remote_path: "/incoming".to_owned(),
            retry_attempts: 1,
            ..FtpIntegrationSettings::default()
        };
        upload_completed_file(&options, ControllerCompatibilityTarget::Slskdn, &file)
            .await
            .unwrap();
        let (commands, uploaded) = server.await.unwrap();
        assert!(commands.contains(&"STOR /incoming/Album/track.flac".to_owned()));
        assert_eq!(uploaded, b"fallback-audio");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn zero_retry_attempts_preserves_the_target_specific_frozen_behavior() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let file = std::env::temp_dir().join(format!("slskr-ftp-{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&file, b"fixture").await.unwrap();
        let options = FtpIntegrationSettings {
            enabled: true,
            address: address.ip().to_string(),
            port: address.port(),
            encryption_mode: "none".to_owned(),
            connection_timeout: 100,
            retry_attempts: 0,
            ..FtpIntegrationSettings::default()
        };
        assert!(
            upload_completed_file(&options, ControllerCompatibilityTarget::Slskd, &file)
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(50), listener.accept())
                .await
                .is_err()
        );

        let attempted = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            drop(stream);
        });
        assert!(
            upload_completed_file(&options, ControllerCompatibilityTarget::Slskdn, &file)
                .await
                .is_err()
        );
        attempted.await.unwrap();
        tokio::fs::remove_file(file).await.unwrap();
    }
}
