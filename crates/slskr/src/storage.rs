//! Storage module: file I/O, caching, and persistence operations.

use std::fs;
use std::path::{Path, PathBuf};

use slskr_client::protocol::peer::FileEntry;
use slskr_client::protocol::{Reader, Writer};
use slskr_client::share_payload::{compress_zlib_payload, decompress_zlib_payload};

// ============================================================================
// Shared Files and Transfer Capacity
// ============================================================================

pub struct SharedLocalFile {
    pub local_path: PathBuf,
    pub size: u64,
}

// ============================================================================
// Share Indexing and Caching
// ============================================================================

pub fn share_cache_path(state_dir: &Path) -> PathBuf {
    state_dir.join("shares.tsv")
}

pub fn write_share_cache(path: &Path, entries: &[FileEntry]) -> Result<(), String> {
    let content = entries
        .iter()
        .map(|entry| {
            format!(
                "{}\t{}\t{}\t{}",
                escape_cache_field(&entry.filename),
                entry.size,
                entry.code,
                escape_cache_field(&entry.extension)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content).map_err(|error| error.to_string())
}

pub fn escape_cache_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

pub fn extension_for(filename: &str) -> String {
    filename
        .rfind('.')
        .and_then(|index| {
            let ext = &filename[index + 1..];
            if ext.is_empty() || ext.contains('/') || ext.contains('\\') {
                None
            } else {
                Some(ext.to_lowercase())
            }
        })
        .unwrap_or_default()
}

// ============================================================================
// Transfer State Management
// ============================================================================

pub fn transfer_events_path(state_dir: &Path) -> PathBuf {
    state_dir.join("transfer.events.tsv")
}

pub fn transfer_state_path(state_dir: &Path) -> PathBuf {
    state_dir.join("transfer.state.json")
}

pub fn write_transfer_events_header(path: &Path) -> Result<(), String> {
    fs::write(path, "timestamp\tstatus\tbytes_transferred\tfilename\n")
        .map_err(|error| error.to_string())
}

pub struct TransferStateFile {
    pub id: u64,
    pub status: String,
    pub bytes_transferred: Option<u64>,
    pub reason: Option<String>,
}

pub fn append_transfer_event(
    path: &Path,
    id: u64,
    status: &str,
    bytes_transferred: Option<u64>,
    filename: &str,
    updated_at: u64,
) -> Result<(), String> {
    let line = format!(
        "{}\t{}\t{}\t{}\n",
        updated_at,
        status,
        bytes_transferred.unwrap_or(0),
        escape_cache_field(filename)
    );
    use std::fs::OpenOptions;
    use std::io::Write;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(line.as_bytes()))
        .map_err(|error| error.to_string())
}

// ============================================================================
// File Entry Serialization
// ============================================================================

pub fn build_shared_file_list_payload(entries: &[FileEntry]) -> Result<Vec<u8>, String> {
    let mut writer = Writer::new();
    let folders = group_share_entries(entries);
    writer.write_u32_le(
        u32::try_from(folders.len()).map_err(|_| "too many shared folders".to_owned())?,
    );
    for (folder, files) in folders {
        writer
            .write_string(&folder)
            .map_err(|error| error.to_string())?;
        writer.write_u32_le(
            u32::try_from(files.len()).map_err(|_| "too many shared files".to_owned())?,
        );
        for file in files {
            encode_file_entry(&mut writer, &file)?;
        }
    }
    compress_zlib_payload(&writer.into_inner()).map_err(|error| error.to_string())
}

pub fn parse_shared_file_list_payload(payload: &[u8]) -> Result<Vec<FileEntry>, String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let folder_count = reader
        .read_u32_le()
        .map_err(|e| format!("cannot read folder count: {e}"))?;
    let mut entries = Vec::new();

    for _ in 0..folder_count {
        let _ = reader
            .read_string()
            .map_err(|e| format!("cannot read folder: {e}"))?;
        let file_count = reader
            .read_u32_le()
            .map_err(|e| format!("cannot read file count: {e}"))?;

        for _ in 0..file_count {
            let code = reader
                .read_u8()
                .map_err(|e| format!("cannot read file code: {e}"))?;
            let filename = reader
                .read_string()
                .map_err(|e| format!("cannot read filename: {e}"))?;
            let size = reader
                .read_u64_le()
                .map_err(|e| format!("cannot read file size: {e}"))?;
            let extension = reader
                .read_string()
                .map_err(|e| format!("cannot read extension: {e}"))?;
            let attr_count = reader
                .read_u32_le()
                .map_err(|e| format!("cannot read attr count: {e}"))?;
            let mut attributes = Vec::new();
            for _ in 0..attr_count {
                let attr_code = reader
                    .read_u32_le()
                    .map_err(|e| format!("cannot read attr code: {e}"))?;
                let attr_value = reader
                    .read_u32_le()
                    .map_err(|e| format!("cannot read attr value: {e}"))?;
                attributes.push(slskr_client::protocol::peer::FileAttribute {
                    code: attr_code,
                    value: attr_value,
                });
            }
            entries.push(FileEntry {
                code,
                filename,
                size,
                extension,
                attributes,
            });
        }
    }
    Ok(entries)
}

pub fn encode_file_entry(writer: &mut Writer, entry: &FileEntry) -> Result<(), String> {
    writer.write_u8(entry.code);
    writer
        .write_string(&entry.filename)
        .map_err(|error| error.to_string())?;
    writer.write_u64_le(entry.size);
    writer
        .write_string(&entry.extension)
        .map_err(|error| error.to_string())?;
    writer.write_u32_le(
        u32::try_from(entry.attributes.len()).map_err(|_| "too many attributes".to_owned())?,
    );
    for attribute in &entry.attributes {
        writer.write_u32_le(attribute.code);
        writer.write_u32_le(attribute.value);
    }
    Ok(())
}

pub fn build_folder_contents_payload(entries: &[FileEntry], folder: &str) -> Result<Vec<u8>, String> {
    let matching = entries
        .iter()
        .filter(|entry| virtual_folder(&entry.filename) == folder)
        .cloned()
        .collect::<Vec<_>>();
    build_shared_file_list_payload(&matching)
}

pub fn parse_folder_file_list_payload(
    payload: &[u8],
) -> Result<(Vec<String>, Vec<FileEntry>), String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let folder_count = reader
        .read_u32_le()
        .map_err(|e| format!("cannot read folder count: {e}"))?;
    let mut folders = Vec::new();
    for _ in 0..folder_count {
        let folder = reader
            .read_string()
            .map_err(|e| format!("cannot read folder: {e}"))?;
        folders.push(folder);
    }
    let file_count = reader
        .read_u32_le()
        .map_err(|e| format!("cannot read file count: {e}"))?;
    let mut entries = Vec::new();
    for _ in 0..file_count {
        let code = reader
            .read_u8()
            .map_err(|e| format!("cannot read file code: {e}"))?;
        let filename = reader
            .read_string()
            .map_err(|e| format!("cannot read filename: {e}"))?;
        let size = reader
            .read_u64_le()
            .map_err(|e| format!("cannot read file size: {e}"))?;
        let extension = reader
            .read_string()
            .map_err(|e| format!("cannot read extension: {e}"))?;
        let attr_count = reader
            .read_u32_le()
            .map_err(|e| format!("cannot read attr count: {e}"))?;
        let mut attributes = Vec::new();
        for _ in 0..attr_count {
            let attr_code = reader
                .read_u32_le()
                .map_err(|e| format!("cannot read attr code: {e}"))?;
            let attr_value = reader
                .read_u32_le()
                .map_err(|e| format!("cannot read attr value: {e}"))?;
            attributes.push(slskr_client::protocol::peer::FileAttribute {
                code: attr_code,
                value: attr_value,
            });
        }
        entries.push(FileEntry {
            code,
            filename,
            size,
            extension,
            attributes,
        });
    }
    Ok((folders, entries))
}

// ============================================================================
// Folder Navigation
// ============================================================================

pub fn join_virtual_path(folder: &str, filename: &str) -> String {
    if folder.is_empty() {
        filename.to_owned()
    } else {
        format!("{}/{}", folder, filename)
    }
}

pub fn virtual_folder(filename: &str) -> &str {
    filename
        .rfind('/')
        .and_then(|index| {
            let folder = &filename[..index];
            if folder.is_empty() {
                None
            } else {
                Some(folder)
            }
        })
        .unwrap_or("")
}

fn folder_parent(filename: &str, parent: &str) -> Option<String> {
    if !filename.starts_with(parent) {
        return None;
    }
    let relative = filename[parent.len()..].trim_start_matches('/');
    relative.split('/').next().and_then(|first| {
        if !first.is_empty() && relative != first {
            Some(join_virtual_path(parent, first))
        } else {
            None
        }
    })
}

// ============================================================================
// Share Grouping
// ============================================================================

pub fn group_share_entries(entries: &[FileEntry]) -> Vec<(String, Vec<FileEntry>)> {
    let mut groups: std::collections::BTreeMap<String, Vec<FileEntry>> = std::collections::BTreeMap::new();
    for entry in entries {
        let folder = virtual_folder(&entry.filename).to_owned();
        groups.entry(folder).or_insert_with(Vec::new).push(entry.clone());
    }
    groups.into_iter().collect()
}
