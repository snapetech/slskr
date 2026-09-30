use super::*;

pub(super) fn load_or_create_certificate(
    state_dir: &Path,
) -> Result<(CertificateDer<'static>, PrivatePkcs8KeyDer<'static>), String> {
    let certificate_path = state_dir.join("overlay-certificate.der");
    let private_key_path = state_dir.join("overlay-private-key.der");
    let certificate = read_identity_file(
        &certificate_path,
        "certificate",
        MAX_CERTIFICATE_BYTES,
        false,
    )?;
    let private_key = read_identity_file(
        &private_key_path,
        "private key",
        MAX_PRIVATE_KEY_BYTES,
        true,
    )?;
    match (certificate, private_key) {
        (Some(certificate), Some(private_key)) => {
            return Ok((
                CertificateDer::from(certificate),
                PrivatePkcs8KeyDer::from(private_key),
            ));
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(
                "overlay TLS identity is incomplete; both certificate and private key are required"
                    .to_owned(),
            );
        }
        (None, None) => {}
    }
    fs::create_dir_all(state_dir)
        .map_err(|error| format!("overlay state directory creation failed: {error}"))?;
    let certified = generate_simple_self_signed(vec!["localhost".to_owned()])
        .map_err(|error| format!("overlay certificate generation failed: {error}"))?;
    let certificate = certified.cert.der().to_vec();
    let private_key = certified.signing_key.serialize_der();
    write_new_identity(
        &certificate_path,
        &private_key_path,
        &certificate,
        &private_key,
    )?;
    Ok((
        CertificateDer::from(certificate),
        PrivatePkcs8KeyDer::from(private_key),
    ))
}

pub(super) fn write_new_identity(
    certificate_path: &Path,
    private_key_path: &Path,
    certificate: &[u8],
    private_key: &[u8],
) -> Result<(), String> {
    write_secret(certificate_path, certificate)?;
    if let Err(error) = write_secret(private_key_path, private_key) {
        return match fs::remove_file(certificate_path) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!(
                "{error}; overlay certificate rollback failed: {cleanup_error}"
            )),
        };
    }
    Ok(())
}

pub(super) fn read_identity_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
    require_private_permissions: bool,
) -> Result<Option<Vec<u8>>, String> {
    #[cfg(not(unix))]
    let _ = require_private_permissions;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("overlay {label} metadata failed: {error}")),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    if metadata.len() > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("overlay {label} read failed: {error}"))?;
    let opened_metadata = file
        .metadata()
        .map_err(|error| format!("overlay {label} metadata failed: {error}"))?;
    if !opened_metadata.is_file() {
        return Err(format!("overlay {label} must be a regular file"));
    }
    #[cfg(unix)]
    if require_private_permissions {
        use std::os::unix::fs::PermissionsExt;
        if opened_metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "overlay {label} must not be accessible by group or other users"
            ));
        }
    }
    if opened_metadata.len() > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    let mut bytes = Vec::new();
    std::io::Read::take(&mut file, max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("overlay {label} read failed: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("overlay {label} is too large"));
    }
    if bytes.is_empty() {
        return Err(format!("overlay {label} is empty"));
    }
    Ok(Some(bytes))
}

pub(super) fn write_secret(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4().simple()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_CLOEXEC);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("overlay identity creation failed: {error}"))?;
    if let Err(error) = std::io::Write::write_all(&mut file, bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(format!("overlay identity write failed: {error}"));
    }
    drop(file);
    if let Err(error) = fs::hard_link(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(format!("overlay identity publish failed: {error}"));
    }
    if let Err(error) = fs::remove_file(&temporary) {
        return Err(format!(
            "overlay identity published but temporary cleanup failed: {error}"
        ));
    }
    Ok(())
}
