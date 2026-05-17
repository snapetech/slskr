use std::{fs, path::Path};

use keyring::Entry;
use serde::{Deserialize, Serialize};
use slskr_client::server::LoginCredentials;

use crate::config::{AppConfig, CredentialStoreMode};

const KEYRING_SERVICE: &str = "slskr.soulseek";
const KEYRING_USERNAME_KEY: &str = "username";
const KEYRING_PASSWORD_KEY: &str = "password";

#[derive(Debug, Serialize, Deserialize)]
struct FileCredentials {
    username: String,
    password: String,
}

#[derive(Clone, Debug)]
pub struct StoredCredentials {
    pub credentials: LoginCredentials,
    pub source: &'static str,
}

pub fn supported_store_modes() -> &'static [&'static str] {
    &["memory", "os", "file"]
}

pub fn load(config: &AppConfig) -> Result<Option<StoredCredentials>, String> {
    if let Some(credentials) = config.credentials() {
        return Ok(Some(StoredCredentials {
            credentials,
            source: "config",
        }));
    }

    match config.credential_store {
        CredentialStoreMode::Memory => Ok(None),
        CredentialStoreMode::Os => load_os(),
        CredentialStoreMode::File => load_file(&config.credential_file),
    }
}

pub fn store(
    config: &AppConfig,
    mode: &CredentialStoreMode,
    credentials: &LoginCredentials,
) -> Result<&'static str, String> {
    match mode {
        CredentialStoreMode::Memory => Ok("runtime"),
        CredentialStoreMode::Os => store_os(credentials),
        CredentialStoreMode::File => store_file(&config.credential_file, credentials),
    }
}

fn load_os() -> Result<Option<StoredCredentials>, String> {
    let username = match keyring_entry(KEYRING_USERNAME_KEY).get_password() {
        Ok(username) if !username.trim().is_empty() => username,
        Ok(_) | Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to read Soulseek username from OS credential store: {error}"
            ))
        }
    };
    let password = match keyring_entry(KEYRING_PASSWORD_KEY).get_password() {
        Ok(password) if !password.is_empty() => password,
        Ok(_) | Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to read Soulseek password from OS credential store: {error}"
            ))
        }
    };

    Ok(Some(StoredCredentials {
        credentials: LoginCredentials::default_client(username, password),
        source: "os",
    }))
}

fn store_os(credentials: &LoginCredentials) -> Result<&'static str, String> {
    keyring_entry(KEYRING_USERNAME_KEY)
        .set_password(&credentials.username)
        .map_err(|error| {
            format!("failed to store Soulseek username in OS credential store: {error}")
        })?;
    keyring_entry(KEYRING_PASSWORD_KEY)
        .set_password(&credentials.password)
        .map_err(|error| {
            format!("failed to store Soulseek password in OS credential store: {error}")
        })?;
    Ok("os")
}

fn keyring_entry(user: &str) -> Entry {
    Entry::new(KEYRING_SERVICE, user).expect("static keyring service and user names are valid")
}

fn load_file(path: &Path) -> Result<Option<StoredCredentials>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to read Soulseek credential file {}: {error}",
                path.display()
            ))
        }
    };
    let parsed = serde_json::from_str::<FileCredentials>(&content).map_err(|error| {
        format!(
            "failed to parse Soulseek credential file {}: {error}",
            path.display()
        )
    })?;
    if parsed.username.trim().is_empty() || parsed.password.is_empty() {
        return Ok(None);
    }
    Ok(Some(StoredCredentials {
        credentials: LoginCredentials::default_client(parsed.username, parsed.password),
        source: "file",
    }))
}

fn store_file(path: &Path, credentials: &LoginCredentials) -> Result<&'static str, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create Soulseek credential directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let payload = serde_json::to_string_pretty(&FileCredentials {
        username: credentials.username.clone(),
        password: credentials.password.clone(),
    })
    .map_err(|error| format!("failed to serialize Soulseek credentials: {error}"))?;

    write_secret_file(path, payload.as_bytes())?;
    Ok("file")
}

#[cfg(unix)]
fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::{
        fs::OpenOptions,
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
    };

    let mut options = OpenOptions::new();
    options.create(true).truncate(true).write(true).mode(0o600);
    std::io::Write::write_all(
        &mut options.open(path).map_err(|error| {
            format!(
                "failed to open Soulseek credential file {}: {error}",
                path.display()
            )
        })?,
        bytes,
    )
    .map_err(|error| {
        format!(
            "failed to write Soulseek credential file {}: {error}",
            path.display()
        )
    })?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| {
        format!(
            "failed to restrict Soulseek credential file {}: {error}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|error| {
        format!(
            "failed to write Soulseek credential file {}: {error}",
            path.display()
        )
    })
}
