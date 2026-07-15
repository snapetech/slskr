use std::{env, fs, path::Path};

use keyring::Entry;
use serde::{Deserialize, Serialize};
use slskr_client::server::LoginCredentials;

use crate::config::{AppConfig, CredentialStoreMode};

const KEYRING_SERVICE: &str = "slskr.soulseek";
const KEYRING_USERNAME_KEY: &str = "username";
const KEYRING_PASSWORD_KEY: &str = "password";
const MAX_CREDENTIAL_FILE_BYTES: u64 = 64 * 1024;

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
    &["memory", "os", "systemd", "file"]
}

pub fn writable_store_modes() -> &'static [&'static str] {
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
        CredentialStoreMode::Systemd => load_systemd(),
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
        CredentialStoreMode::Systemd => Err(
            "systemd credentials are read-only at runtime; configure LoadCredential= or LoadCredentialEncrypted= in the service unit".to_owned(),
        ),
        CredentialStoreMode::File => store_file(&config.credential_file, credentials),
    }
}

fn load_systemd() -> Result<Option<StoredCredentials>, String> {
    let Some(credentials_dir) = env::var_os("CREDENTIALS_DIRECTORY") else {
        return Ok(None);
    };
    let credentials_dir = Path::new(&credentials_dir);

    if let Some(credentials) = load_systemd_json(credentials_dir)? {
        return Ok(Some(credentials));
    }

    let username_path = credentials_dir.join("slsk-username");
    let password_path = credentials_dir.join("slsk-password");
    if !username_path.exists() && !password_path.exists() {
        return Ok(None);
    }

    let username = read_secret_text(&username_path, "systemd Soulseek username")?;
    let password = read_secret_text(&password_path, "systemd Soulseek password")?;
    if username.trim().is_empty() || password.is_empty() {
        return Ok(None);
    }

    Ok(Some(StoredCredentials {
        credentials: LoginCredentials::default_client(username.trim().to_owned(), password),
        source: "systemd",
    }))
}

fn load_systemd_json(credentials_dir: &Path) -> Result<Option<StoredCredentials>, String> {
    let path = credentials_dir.join("slskr-soulseek");
    if !path.exists() {
        return Ok(None);
    }

    let content = read_secret_text(&path, "systemd Soulseek credential JSON")?;
    let parsed = serde_json::from_str::<FileCredentials>(&content).map_err(|error| {
        format!(
            "failed to parse systemd Soulseek credential JSON {}: {error}",
            path.display()
        )
    })?;
    if parsed.username.trim().is_empty() || parsed.password.is_empty() {
        return Ok(None);
    }

    Ok(Some(StoredCredentials {
        credentials: LoginCredentials::default_client(
            parsed.username.trim().to_owned(),
            parsed.password,
        ),
        source: "systemd",
    }))
}

fn read_secret_text(path: &Path, label: &str) -> Result<String, String> {
    read_bounded_secret_file(path, label)
        .map(|value| value.trim_end_matches(['\r', '\n']).to_owned())
}

fn read_bounded_secret_file(path: &Path, label: &str) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{label} {} must be a regular file", path.display()));
    }
    if metadata.len() > MAX_CREDENTIAL_FILE_BYTES {
        return Err(format!(
            "{label} {} exceeds {MAX_CREDENTIAL_FILE_BYTES} bytes",
            path.display()
        ));
    }
    fs::read_to_string(path)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))
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
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to inspect Soulseek credential file {}: {error}",
                path.display()
            ));
        }
    }
    let content = read_bounded_secret_file(path, "Soulseek credential file")?;
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
        let metadata = fs::symlink_metadata(parent).map_err(|error| {
            format!(
                "failed to inspect Soulseek credential directory {}: {error}",
                parent.display()
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "Soulseek credential directory {} must be a real directory",
                parent.display()
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
                format!(
                    "failed to restrict Soulseek credential directory {}: {error}",
                    parent.display()
                )
            })?;
        }
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

    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(format!(
            "Soulseek credential file {} must not be a symlink",
            path.display()
        ));
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| "Soulseek credential file path has no file name".to_owned())?;
    let mut temporary = None;
    for attempt in 0..100_u32 {
        let candidate = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            attempt
        ));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true).mode(0o600);
        match options.open(&candidate) {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create temporary credential file: {error}"
                ))
            }
        }
    }
    let (temporary_path, mut file) = temporary
        .ok_or_else(|| "failed to allocate temporary Soulseek credential file".to_owned())?;
    let write_result = (|| -> Result<(), String> {
        std::io::Write::write_all(&mut file, bytes)
            .map_err(|error| format!("failed to write Soulseek credential file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync Soulseek credential file: {error}"))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("failed to replace Soulseek credential file: {error}"))
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|error| {
        format!(
            "failed to restrict Soulseek credential file {}: {error}",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("slskr-credentials-{label}-{}", std::process::id()))
    }

    #[cfg(unix)]
    #[test]
    fn credential_file_write_rejects_symlink_without_touching_target() {
        use std::os::unix::fs::symlink;

        let root = test_dir("symlink");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        let target = root.join("target");
        let linked = root.join("credentials.json");
        fs::write(&target, "keep").expect("write fixture target");
        symlink(&target, &linked).expect("create fixture symlink");

        let error = write_secret_file(&linked, b"replace").expect_err("reject symlink");
        assert!(error.contains("must not be a symlink"));
        assert_eq!(fs::read_to_string(&target).expect("read target"), "keep");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn credential_file_read_rejects_oversized_input() {
        let root = test_dir("oversized");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        let path = root.join("credentials.json");
        let file = fs::File::create(&path).expect("create fixture file");
        file.set_len(MAX_CREDENTIAL_FILE_BYTES + 1)
            .expect("grow fixture file");

        let error = read_bounded_secret_file(&path, "credential fixture")
            .expect_err("reject oversized credential file");
        assert!(error.contains("exceeds"));
        let _ = fs::remove_dir_all(root);
    }
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
