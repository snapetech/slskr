use super::{
    bounded_transfer_entry, escape_cache_field, is_active_transfer_status,
    next_transfer_updated_at_ms, sync_state_directory, unix_timestamp, write_file_atomic,
    TransferEntry, MAX_TRANSFER_EVENTS_BYTES, MAX_TRANSFER_STATE_BYTES,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn transfer_events_path(state_dir: &Path) -> PathBuf {
    state_dir.join("transfer-events.tsv")
}

pub(super) fn transfer_state_path(state_dir: &Path) -> PathBuf {
    state_dir.join("transfer-state.json")
}

pub(super) fn write_transfer_events_header(path: &Path) -> Result<(), String> {
    use std::io::{Read, Seek, SeekFrom, Write};

    const HEADER: &[u8] = b"slskr-transfer-events-v2\nid\tdirection\ttoken\tsize\tbytes_transferred\tstatus\treason\tfilename\n";
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("transfer events directory create failed: {error}"))?;

    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("transfer events path must be a regular file".to_owned());
        }
    }

    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("transfer events header open failed: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("transfer events header metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("transfer events path must be a regular file".to_owned());
    }

    if metadata.len() == 0 {
        file.write_all(HEADER)
            .map_err(|error| format!("transfer events header write failed: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("transfer events header sync failed: {error}"))?;
        sync_state_directory(parent)
            .map_err(|error| format!("transfer events directory sync failed: {error}"))?;
        return Ok(());
    }

    if metadata.len() < HEADER.len() as u64 {
        return Err("transfer events file has an incomplete header".to_owned());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("transfer events header seek failed: {error}"))?;
    let mut actual = vec![0_u8; HEADER.len()];
    file.read_exact(&mut actual)
        .map_err(|error| format!("transfer events header read failed: {error}"))?;
    if actual != HEADER {
        return Err("transfer events file has an unsupported header".to_owned());
    }
    Ok(())
}

fn rotate_transfer_events_if_needed(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("transfer event metadata read failed: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("transfer event path must be a regular file".to_owned());
    }
    if metadata.len() <= MAX_TRANSFER_EVENTS_BYTES {
        return Ok(());
    }

    let rotated_path = path.with_extension("tsv.old");
    if rotated_path.exists() {
        fs::remove_file(&rotated_path)
            .map_err(|error| format!("transfer event rotation cleanup failed: {error}"))?;
    }
    fs::rename(path, &rotated_path)
        .map_err(|error| format!("transfer event rotation failed: {error}"))?;
    sync_state_directory(path.parent().unwrap_or_else(|| Path::new(".")))
        .map_err(|error| format!("transfer event rotation directory sync failed: {error}"))?;
    write_transfer_events_header(path)
}

#[derive(Debug, Deserialize, Serialize)]
struct TransferStateFile {
    version: u32,
    entries: Vec<TransferEntry>,
}

pub(super) fn load_transfer_state(
    path: &Path,
    history_limit: usize,
) -> Result<Vec<TransferEntry>, String> {
    use std::io::Read;

    #[cfg(not(unix))]
    {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(format!("transfer state metadata read failed: {error}")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("transfer state path must be a regular file".to_owned());
        }
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("transfer state open failed: {error}")),
    };
    let opened_metadata = file
        .metadata()
        .map_err(|error| format!("transfer state metadata read failed: {error}"))?;
    if !opened_metadata.is_file() {
        return Err("transfer state path must be a regular file".to_owned());
    }
    let size = opened_metadata.len();
    if size > MAX_TRANSFER_STATE_BYTES {
        return Err(format!(
            "transfer state file is too large: {size} bytes, max is {MAX_TRANSFER_STATE_BYTES}"
        ));
    }
    let mut body = String::new();
    file.take(MAX_TRANSFER_STATE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("transfer state read failed: {error}"))?;
    if body.len() as u64 > MAX_TRANSFER_STATE_BYTES {
        return Err(format!(
            "transfer state file is too large: more than {MAX_TRANSFER_STATE_BYTES} bytes"
        ));
    }
    let mut state = serde_json::from_str::<TransferStateFile>(&body)
        .map_err(|error| format!("transfer state parse failed: {error}"))?;
    if state.version != 1 {
        return Err(format!(
            "unsupported transfer state version: {}",
            state.version
        ));
    }
    let now = unix_timestamp();
    for entry in &mut state.entries {
        if is_active_transfer_status(&entry.status) {
            entry.status = "queued".to_owned();
            entry.reason = Some("resumed after restart".to_owned());
            entry.started_at = None;
            entry.start_offset = entry.bytes_transferred;
            entry.updated_at = now;
            entry.updated_at_ms = next_transfer_updated_at_ms(entry.updated_at_ms);
        }
    }
    state.entries = state
        .entries
        .into_iter()
        .map(bounded_transfer_entry)
        .collect();
    let mut seen_ids = std::collections::HashSet::new();
    state.entries.reverse();
    state.entries.retain(|entry| seen_ids.insert(entry.id));
    state.entries.reverse();
    if state.entries.len() > history_limit {
        let extra = state.entries.len() - history_limit;
        state.entries.drain(0..extra);
    }
    Ok(state.entries)
}

pub(super) fn write_transfer_state(path: &Path, entries: &[TransferEntry]) -> Result<(), String> {
    let state = TransferStateFile {
        version: 1,
        entries: entries.to_vec(),
    };
    let body = serde_json::to_string_pretty(&state)
        .map_err(|error| format!("transfer state encode failed: {error}"))?;
    write_file_atomic(path, body).map_err(|error| format!("transfer state write failed: {error}"))
}

pub(super) fn append_transfer_event(path: &Path, entry: &TransferEntry) -> Result<(), String> {
    use std::io::Write;

    rotate_transfer_events_if_needed(path)?;
    write_transfer_events_header(path)?;

    let mut file = open_transfer_event_file(path)?;
    writeln!(
        file,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        entry.id,
        entry.direction,
        entry.token,
        entry.size.map(|size| size.to_string()).unwrap_or_default(),
        entry.bytes_transferred,
        escape_cache_field(&entry.status),
        escape_cache_field(entry.reason.as_deref().unwrap_or_default()),
        escape_cache_field(&entry.filename)
    )
    .map_err(|error| format!("transfer event append failed: {error}"))?;
    file.flush()
        .map_err(|error| format!("transfer event flush failed: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("transfer event sync failed: {error}"))
}

pub(super) fn open_transfer_event_file(path: &Path) -> Result<fs::File, String> {
    let mut options = fs::OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|error| format!("transfer event open failed: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("transfer event metadata read failed: {error}"))?;
    if !metadata.is_file() {
        return Err("transfer event path must be a regular file".to_owned());
    }
    Ok(file)
}
