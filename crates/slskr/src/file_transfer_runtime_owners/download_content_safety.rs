use super::*;

pub(super) async fn validate_configured_path_policy(
    state: &AppState,
    path: &str,
) -> Result<(), String> {
    let policy = state
        .advanced_networking
        .read()
        .await
        .security
        .path_guard
        .clone();
    // Traversal confinement is unconditional; the switch controls only the
    // configurable length/depth limits, matching frozen native profile.
    if path
        .split(['/', '\\'])
        .any(|segment| segment == ".." || segment == ".")
    {
        return Err("download path contains a traversal segment".to_owned());
    }
    if policy.enabled {
        let depth = path
            .split(['/', '\\'])
            .filter(|segment| !segment.is_empty())
            .count();
        if path.len() > policy.max_path_length || depth > policy.max_path_depth {
            return Err("download path exceeds configured path guard limits".to_owned());
        }
    }
    Ok(())
}

pub(crate) async fn enforce_completed_download_content_safety(
    state: &AppState,
    path: &Path,
    expected_path: &Path,
) -> Result<(), String> {
    let security = state.advanced_networking.read().await.security.clone();
    if !security.enabled || !security.content_safety.enabled {
        return Ok(());
    }
    let extension = expected_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let executable = matches!(
        extension.as_str(),
        "exe"
            | "bat"
            | "cmd"
            | "com"
            | "pif"
            | "scr"
            | "ps1"
            | "vbs"
            | "vbe"
            | "js"
            | "jse"
            | "wsf"
            | "msi"
            | "msp"
            | "hta"
            | "cpl"
            | "jar"
            | "app"
    );
    let mut suspicious = executable && security.content_safety.block_executables;
    if security.content_safety.verify_magic_bytes && !suspicious {
        let mut header = [0_u8; 12];
        let read = fs::File::open(path)
            .and_then(|mut file| file.read(&mut header))
            .map_err(|error| format!("download content-safety read failed: {error}"))?;
        let header = &header[..read];
        suspicious = match extension.as_str() {
            "flac" => !header.starts_with(b"fLaC"),
            "wav" => !(header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WAVE")),
            "ogg" | "opus" => !header.starts_with(b"OggS"),
            "mp3" => {
                !(header.starts_with(b"ID3")
                    || header
                        .first()
                        .zip(header.get(1))
                        .is_some_and(|(a, b)| *a == 0xff && (*b & 0xe0) == 0xe0))
            }
            _ => false,
        };
    }
    if !suspicious {
        return Ok(());
    }
    if security.content_safety.quarantine_suspicious {
        let root = if security
            .content_safety
            .quarantine_directory
            .as_os_str()
            .is_empty()
        {
            state.config.state_dir.join("quarantine")
        } else {
            security.content_safety.quarantine_directory
        };
        fs::create_dir_all(&root)
            .map_err(|error| format!("download quarantine creation failed: {error}"))?;
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("suspicious-download");
        let destination = root.join(format!("{}-{name}", uuid::Uuid::new_v4().simple()));
        fs::rename(path, destination)
            .map_err(|error| format!("download quarantine move failed: {error}"))?;
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("suspicious download removal failed: {error}"))?;
    }
    Err("download rejected by configured content safety policy".to_owned())
}
