use super::*;

pub(super) fn validate_controller_storage_directory(
    field: &str,
    path: &std::path::Path,
    target: ControllerProfile,
    explicitly_configured: bool,
) -> Result<(), String> {
    if !explicitly_configured {
        return Ok(());
    }
    if path.as_os_str().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    if target == ControllerProfile::Legacy && !path.is_absolute() {
        return Err(format!(
            "{field} must be an absolute path for the legacy compatibility profile"
        ));
    }
    let metadata =
        fs::metadata(path).map_err(|_| format!("{field} specifies a non-existent directory"))?;
    if !metadata.is_dir() {
        return Err(format!("{field} must specify a directory"));
    }

    let probe = path.join(format!(
        ".slskr-write-probe-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|_| format!("{field} must specify a writable directory"))?;
    drop(file);
    fs::remove_file(&probe).map_err(|_| format!("{field} writeability probe cleanup failed"))?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialStoreMode {
    Memory,
    Os,
    Systemd,
    File,
}

impl CredentialStoreMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "memory" | "runtime" | "none" => Ok(Self::Memory),
            "os" | "keyring" | "keychain" | "credential-manager" => Ok(Self::Os),
            "systemd" | "systemd-credentials" | "systemd-creds" => Ok(Self::Systemd),
            "file" | "local-file" => Ok(Self::File),
            other => Err(format!(
                "invalid SLSKR_CREDENTIAL_STORE {other:?}; expected memory, os, systemd, or file"
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Os => "os",
            Self::Systemd => "systemd",
            Self::File => "file",
        }
    }

    pub fn auto_connect_default(&self) -> bool {
        !matches!(self, Self::Memory)
    }
}

pub fn default_state_dir() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".local/state"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("slskr")
}

pub(super) fn default_config_file() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("slskr/config.toml")
}

pub fn load_file_config() -> Result<(Option<PathBuf>, FileConfig), String> {
    let explicit_path = env::var_os("SLSKR_CONFIG").map(PathBuf::from);
    let path = explicit_path.clone().or_else(|| {
        let default = default_config_file();
        default.exists().then_some(default)
    });

    let Some(path) = path else {
        return Ok((None, FileConfig::default()));
    };
    let config = read_file_config(&path)?;
    Ok((Some(path), config))
}

pub(super) fn read_file_config(path: &std::path::Path) -> Result<FileConfig, String> {
    use std::io::Read;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("failed to read config file {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("failed to inspect config file {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "config path {} is not a regular file",
            path.display()
        ));
    }
    if metadata.len() > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "config file {} is too large: {} bytes, max is {MAX_CONFIG_FILE_BYTES}",
            path.display(),
            metadata.len()
        ));
    }
    let mut body = String::new();
    file.take(MAX_CONFIG_FILE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("failed to read config file {}: {error}", path.display()))?;
    if body.len() as u64 > MAX_CONFIG_FILE_BYTES {
        return Err(format!(
            "config file {} is too large: more than {MAX_CONFIG_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let config = toml::from_str::<FileConfig>(&body)
        .map_err(|error| format!("failed to parse config file {}: {error}", path.display()))?;
    warn_insecure_config_permissions(path, &metadata, &config);
    Ok(config)
}

#[cfg(unix)]
pub(super) fn warn_insecure_config_permissions(
    path: &std::path::Path,
    metadata: &fs::Metadata,
    config: &FileConfig,
) {
    use std::os::unix::fs::PermissionsExt;

    if !config_contains_sensitive_values(config) {
        return;
    }

    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        eprintln!(
            "warning: config file {} contains secrets and is readable by group/other users; recommended mode is 0600",
            path.display()
        );
    }
}

#[cfg(not(unix))]
pub(super) fn warn_insecure_config_permissions(
    _path: &std::path::Path,
    _metadata: &fs::Metadata,
    _config: &FileConfig,
) {
}

pub(super) fn config_contains_sensitive_values(config: &FileConfig) -> bool {
    config.network.username.is_some()
        || config.network.password.is_some()
        || config.metrics.authentication.password.is_some()
        || config.auth.password.is_some()
        || config.auth.jwt.key.is_some()
        || config.auth.api_token.is_some()
        || config.auth.read_write_token.is_some()
        || config.auth.read_only_token.is_some()
        || config.auth.nowplaying_token.is_some()
        || config.integrations.spotify.client_secret.is_some()
        || config.integrations.acoustid.client_id.is_some()
        || config.integrations.lidarr.api_key.is_some()
        || config.integrations.youtube.api_key.is_some()
        || config.integrations.lastfm.api_key.is_some()
        || config.integrations.ntfy.access_token.is_some()
        || config.integrations.pushover.user_key.is_some()
        || config.integrations.pushover.token.is_some()
        || config.integrations.pushbullet.access_token.is_some()
        || config.integrations.ftp.password.is_some()
        || config.integrations.vpn.gluetun.password.is_some()
        || config.integrations.vpn.gluetun.api_key.is_some()
}
