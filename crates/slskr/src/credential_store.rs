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
        return Ok(normalize_stored_credentials(credentials, "config"));
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
    Ok(normalize_stored_credentials(
        LoginCredentials::default_client(username, password),
        "systemd",
    ))
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
    Ok(normalize_stored_credentials(
        LoginCredentials::default_client(parsed.username, parsed.password),
        "systemd",
    ))
}

fn read_secret_text(path: &Path, label: &str) -> Result<String, String> {
    read_bounded_secret_file(path, label, false)
        .map(|value| value.trim_end_matches(['\r', '\n']).to_owned())
}

fn read_bounded_secret_file(
    path: &Path,
    label: &str,
    require_private_permissions: bool,
) -> Result<String, String> {
    use std::io::Read;

    #[cfg(not(unix))]
    let _ = require_private_permissions;

    #[cfg(not(unix))]
    {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("{label} {} must be a regular file", path.display()));
        }
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("failed to open {label} {}: {error}", path.display()))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("failed to inspect {label} {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{label} {} must be a regular file", path.display()));
    }
    #[cfg(unix)]
    if require_private_permissions {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(format!(
                "{label} {} must not be accessible by group or other users",
                path.display()
            ));
        }
    }
    if metadata.len() > MAX_CREDENTIAL_FILE_BYTES {
        return Err(format!(
            "{label} {} exceeds {MAX_CREDENTIAL_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let mut content = String::new();
    file.take(MAX_CREDENTIAL_FILE_BYTES + 1)
        .read_to_string(&mut content)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
    if content.len() as u64 > MAX_CREDENTIAL_FILE_BYTES {
        return Err(format!(
            "{label} {} exceeds {MAX_CREDENTIAL_FILE_BYTES} bytes",
            path.display()
        ));
    }
    Ok(content)
}

fn load_os() -> Result<Option<StoredCredentials>, String> {
    let username = match keyring_entry(KEYRING_USERNAME_KEY)?.get_password() {
        Ok(username) if !username.trim().is_empty() => username,
        Ok(_) | Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to read Soulseek username from OS credential store: {error}"
            ))
        }
    };
    let password = match keyring_entry(KEYRING_PASSWORD_KEY)?.get_password() {
        Ok(password) if !password.is_empty() => password,
        Ok(_) | Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to read Soulseek password from OS credential store: {error}"
            ))
        }
    };

    Ok(normalize_stored_credentials(
        LoginCredentials::default_client(username, password),
        "os",
    ))
}

fn store_os(credentials: &LoginCredentials) -> Result<&'static str, String> {
    store_os_with(
        credentials,
        |user| match keyring_entry(user)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        },
        |user, value| {
            keyring_entry(user)?
                .set_password(value)
                .map_err(|error| error.to_string())
        },
        |user| {
            keyring_entry(user)?
                .delete_credential()
                .map_err(|error| error.to_string())
        },
    )?;
    Ok("os")
}

fn store_os_with(
    credentials: &LoginCredentials,
    mut get: impl FnMut(&str) -> Result<Option<String>, String>,
    mut set: impl FnMut(&str, &str) -> Result<(), String>,
    mut delete: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    let previous_username = get(KEYRING_USERNAME_KEY).map_err(|error| {
        format!("failed to snapshot Soulseek username in OS credential store: {error}")
    })?;
    set(KEYRING_USERNAME_KEY, &credentials.username).map_err(|error| {
        format!("failed to store Soulseek username in OS credential store: {error}")
    })?;
    if let Err(error) = set(KEYRING_PASSWORD_KEY, &credentials.password) {
        let rollback = match previous_username {
            Some(value) => set(KEYRING_USERNAME_KEY, &value),
            None => delete(KEYRING_USERNAME_KEY),
        };
        if let Err(rollback_error) = rollback {
            return Err(format!(
                "failed to store Soulseek password in OS credential store: {error}; username rollback failed: {rollback_error}"
            ));
        }
        return Err(format!(
            "failed to store Soulseek password in OS credential store: {error}"
        ));
    }
    Ok(())
}

fn keyring_entry(user: &str) -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, user)
        .map_err(|error| format!("failed to create OS credential-store entry: {error}"))
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
    let content = read_bounded_secret_file(path, "Soulseek credential file", true)?;
    let parsed = serde_json::from_str::<FileCredentials>(&content).map_err(|error| {
        format!(
            "failed to parse Soulseek credential file {}: {error}",
            path.display()
        )
    })?;
    Ok(normalize_stored_credentials(
        LoginCredentials::default_client(parsed.username, parsed.password),
        "file",
    ))
}

fn normalize_stored_credentials(
    mut credentials: LoginCredentials,
    source: &'static str,
) -> Option<StoredCredentials> {
    credentials.username = credentials.username.trim().to_owned();
    if credentials.username.is_empty() || credentials.password.is_empty() {
        return None;
    }
    Some(StoredCredentials {
        credentials,
        source,
    })
}

fn store_file(path: &Path, credentials: &LoginCredentials) -> Result<&'static str, String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    ensure_secure_credential_parent(parent)?;

    let payload = serde_json::to_string_pretty(&FileCredentials {
        username: credentials.username.clone(),
        password: credentials.password.clone(),
    })
    .map_err(|error| format!("failed to serialize Soulseek credentials: {error}"))?;
    if payload.len() as u64 > MAX_CREDENTIAL_FILE_BYTES {
        return Err(format!(
            "Soulseek credential payload exceeds {MAX_CREDENTIAL_FILE_BYTES} bytes"
        ));
    }

    write_secret_file(path, payload.as_bytes())?;
    Ok("file")
}

fn ensure_secure_credential_parent(parent: &Path) -> Result<(), String> {
    let existed = parent.exists();
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
        if existed && metadata.permissions().mode() & 0o022 != 0 {
            return Err(format!(
                "Soulseek credential directory {} must not be writable by group or other users",
                parent.display()
            ));
        }
        if !existed {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).map_err(|error| {
                format!(
                    "failed to restrict Soulseek credential directory {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_secret_file_with(path, bytes, |temporary_path| {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(temporary_path, fs::Permissions::from_mode(0o600))
    })
}

#[cfg(unix)]
fn write_secret_file_with(
    path: &Path,
    bytes: &[u8],
    restrict_permissions: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), String> {
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

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
    let parent_directory = fs::File::open(parent).map_err(|error| {
        format!(
            "failed to open Soulseek credential directory {}: {error}",
            parent.display()
        )
    })?;
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
        restrict_permissions(&temporary_path)
            .map_err(|error| format!("failed to restrict Soulseek credential file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync Soulseek credential file: {error}"))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("failed to replace Soulseek credential file: {error}"))?;
        parent_directory
            .sync_all()
            .map_err(|error| format!("failed to sync Soulseek credential directory: {error}"))
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
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

    #[cfg(unix)]
    #[test]
    fn credential_permission_failure_preserves_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_dir("permission-rollback");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .expect("restrict fixture directory");
        let path = root.join("credentials.json");
        fs::write(&path, b"existing credentials").expect("write existing credentials");

        let error = write_secret_file_with(&path, b"replacement credentials", |_| {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        })
        .expect_err("permission hardening failure must abort publication");

        assert!(error.contains("failed to restrict"), "{error}");
        assert_eq!(
            fs::read(&path).expect("read preserved credentials"),
            b"existing credentials"
        );
        assert_eq!(
            fs::read_dir(&root).expect("read fixture directory").count(),
            1,
            "failed staging file must be removed"
        );
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

        let error = read_bounded_secret_file(&path, "credential fixture", false)
            .expect_err("reject oversized credential file");
        assert!(error.contains("exceeds"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn credential_file_write_rejects_payload_that_cannot_be_reloaded() {
        let root = test_dir("oversized-write");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        let path = root.join("credentials.json");
        let credentials = LoginCredentials::default_client(
            "user".to_owned(),
            "x".repeat(MAX_CREDENTIAL_FILE_BYTES as usize),
        );

        let error = store_file(&path, &credentials)
            .expect_err("reject credentials larger than the loader accepts");
        assert!(error.contains("exceeds"));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn credential_file_read_rejects_symlink_without_reading_target() {
        use std::os::unix::fs::symlink;

        let root = test_dir("read-symlink");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        let target = root.join("target");
        let linked = root.join("credentials.json");
        fs::write(&target, "outside-secret").expect("write fixture target");
        symlink(&target, &linked).expect("create fixture symlink");

        let error = read_bounded_secret_file(&linked, "credential fixture", false)
            .expect_err("reject symlinked credential file");
        assert!(error.contains("failed to open"));
        assert!(!error.contains("outside-secret"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn credential_parent_validation_does_not_mutate_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_dir("parent-mode");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755))
            .expect("set fixture permissions");

        ensure_secure_credential_parent(&root).expect("accept non-writable existing parent");
        let mode = fs::metadata(&root)
            .expect("read fixture metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn credential_parent_validation_rejects_shared_writable_directory() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_dir("shared-parent");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o777))
            .expect("set fixture permissions");

        let error = ensure_secure_credential_parent(&root)
            .expect_err("reject shared-writable credential parent");
        assert!(error.contains("writable by group or other"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn credential_file_load_rejects_group_or_other_access() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_dir("readable-file");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        let path = root.join("credentials.json");
        fs::write(&path, r#"{"username":"user","password":"secret"}"#)
            .expect("write credential fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640))
            .expect("make credential fixture group-readable");

        let error = load_file(&path).expect_err("reject exposed credential file");
        assert!(error.contains("must not be accessible by group or other users"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn credential_file_load_normalizes_username_before_login() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_dir("normalized-username");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture directory");
        let path = root.join("credentials.json");
        fs::write(&path, r#"{"username":"  user  ","password":"secret"}"#)
            .expect("write credential fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .expect("restrict credential fixture");

        let stored = load_file(&path)
            .expect("load credentials")
            .expect("credentials must be present");
        assert_eq!(stored.credentials.username, "user");
        assert_eq!(
            stored.credentials.into_login_request().username,
            "user",
            "normalized username must reach the wire request"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stored_credentials_reject_blank_identity_or_password() {
        assert!(normalize_stored_credentials(
            LoginCredentials::default_client("  ", "password"),
            "test"
        )
        .is_none());
        assert!(
            normalize_stored_credentials(LoginCredentials::default_client("user", ""), "test")
                .is_none()
        );
    }

    #[test]
    fn os_credential_password_failure_rolls_back_username() {
        use std::cell::RefCell;
        use std::collections::HashMap;

        let values = RefCell::new(HashMap::from([
            (KEYRING_USERNAME_KEY.to_owned(), "old-user".to_owned()),
            (KEYRING_PASSWORD_KEY.to_owned(), "old-password".to_owned()),
        ]));
        let credentials =
            LoginCredentials::default_client("new-user".to_owned(), "new-password".to_owned());

        let error = store_os_with(
            &credentials,
            |key| Ok(values.borrow().get(key).cloned()),
            |key, value| {
                if key == KEYRING_PASSWORD_KEY {
                    Err("simulated password write failure".to_owned())
                } else {
                    values.borrow_mut().insert(key.to_owned(), value.to_owned());
                    Ok(())
                }
            },
            |key| {
                values.borrow_mut().remove(key);
                Ok(())
            },
        )
        .expect_err("password failure must be reported");

        assert!(error.contains("failed to store Soulseek password"));
        assert_eq!(values.borrow()[KEYRING_USERNAME_KEY], "old-user");
        assert_eq!(values.borrow()[KEYRING_PASSWORD_KEY], "old-password");
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
