//! Storage module: file I/O, caching, and persistence operations.

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use slskr_client::protocol::peer::FileEntry;
use slskr_client::protocol::{Reader, Writer};
use slskr_client::share_payload::{compress_zlib_payload, decompress_zlib_payload};

const MAX_PARSED_SHARE_FOLDERS: usize = 20_000;
const MAX_PARSED_SHARE_FILES: usize = 20_000;

pub(crate) fn write_file_atomic(path: &Path, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("slskr-state");
    let temp_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));

    write_file_atomic_with_temp_path(path, &temp_path, contents.as_ref())
}

pub(crate) fn write_file_atomic_with_temp_path(
    path: &Path,
    temp_path: &Path,
    contents: &[u8],
) -> std::io::Result<()> {
    use std::io::Write;

    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(temp_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }

    match replace_file(temp_path, path) {
        Ok(()) => sync_state_directory(path.parent().unwrap_or_else(|| Path::new("."))),
        Err(error) => {
            let _ = fs::remove_file(temp_path);
            Err(error)
        }
    }
}

#[cfg(unix)]
pub(crate) fn sync_state_directory(path: &Path) -> std::io::Result<()> {
    fs::File::open(path).and_then(|directory| directory.sync_all())
}

#[cfg(not(unix))]
pub(crate) fn sync_state_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(not(unix))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    // std::fs::rename does not replace an existing destination on Windows.
    // The state directory is private and both names are fixed beneath it, so
    // removing the old entry first is the safe-code fallback available here.
    match fs::remove_file(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::rename(source, destination)
}

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
    let mut content = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            content.push('\n');
        }
        append_escaped_cache_field(&mut content, &entry.filename);
        let _ = write!(content, "\t{}\t{}\t", entry.size, entry.code);
        append_escaped_cache_field(&mut content, &entry.extension);
    }
    fs::write(path, content).map_err(|error| error.to_string())
}

pub fn escape_cache_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    append_escaped_cache_field(&mut escaped, value);
    escaped
}

fn append_escaped_cache_field(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\t' => output.push_str("\\t"),
            character => output.push(character),
        }
    }
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
    _id: u64,
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
        .read_bounded_count("shared folders", 8)
        .map_err(|e| format!("cannot read folder count: {e}"))?;
    if folder_count > MAX_PARSED_SHARE_FOLDERS {
        return Err(format!(
            "shared folders exceeds {MAX_PARSED_SHARE_FOLDERS} entries"
        ));
    }
    let mut entries = Vec::new();
    let mut parsed_files = 0;

    for _ in 0..folder_count {
        let _ = reader
            .read_string()
            .map_err(|e| format!("cannot read folder: {e}"))?;
        let file_count = reader
            .read_bounded_count("shared files", 21)
            .map_err(|e| format!("cannot read file count: {e}"))?;
        if file_count > MAX_PARSED_SHARE_FILES.saturating_sub(parsed_files) {
            return Err(format!(
                "shared files exceeds {MAX_PARSED_SHARE_FILES} entries"
            ));
        }
        parsed_files += file_count;

        entries.extend(decode_file_entries(
            &mut reader,
            file_count,
            "shared files",
        )?);
    }
    Ok(entries)
}

pub fn encode_file_entry(writer: &mut Writer, entry: &FileEntry) -> Result<(), String> {
    writer.write_u8(entry.code);
    writer
        .write_string_with_encoding(&entry.filename, entry.filename_encoding)
        .map_err(|error| error.to_string())?;
    writer.write_u64_le(entry.size);
    writer
        .write_string_with_encoding(&entry.extension, entry.extension_encoding)
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

pub fn build_folder_contents_payload(
    entries: &[FileEntry],
    folder: &str,
) -> Result<Vec<u8>, String> {
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
        .read_bounded_count("folder names", 4)
        .map_err(|e| format!("cannot read folder count: {e}"))?;
    if folder_count > MAX_PARSED_SHARE_FOLDERS {
        return Err(format!(
            "folder names exceeds {MAX_PARSED_SHARE_FOLDERS} entries"
        ));
    }
    let mut folders = Vec::new();
    for _ in 0..folder_count {
        let folder = reader
            .read_string()
            .map_err(|e| format!("cannot read folder: {e}"))?;
        folders.push(folder);
    }
    let file_count = reader
        .read_bounded_count("folder files", 21)
        .map_err(|e| format!("cannot read file count: {e}"))?;
    if file_count > MAX_PARSED_SHARE_FILES {
        return Err(format!(
            "folder files exceeds {MAX_PARSED_SHARE_FILES} entries"
        ));
    }
    let entries = decode_file_entries(&mut reader, file_count, "folder files")?;
    Ok((folders, entries))
}

fn decode_file_entries(
    reader: &mut Reader<'_>,
    file_count: usize,
    field: &str,
) -> Result<Vec<FileEntry>, String> {
    let mut entries = Vec::with_capacity(file_count);
    for _ in 0..file_count {
        let code = reader
            .read_u8()
            .map_err(|e| format!("cannot read {field} file code: {e}"))?;
        let (filename, filename_encoding) = reader
            .read_string_with_encoding()
            .map_err(|e| format!("cannot read {field} filename: {e}"))?;
        let size = reader
            .read_u64_le()
            .map_err(|e| format!("cannot read {field} file size: {e}"))?;
        let (extension, extension_encoding) = reader
            .read_string_with_encoding()
            .map_err(|e| format!("cannot read {field} extension: {e}"))?;
        let attr_count = reader
            .read_bounded_count("file attributes", 8)
            .map_err(|e| format!("cannot read {field} attr count: {e}"))?;
        let mut attributes = Vec::with_capacity(attr_count);
        for _ in 0..attr_count {
            let attr_code = reader
                .read_u32_le()
                .map_err(|e| format!("cannot read {field} attr code: {e}"))?;
            let attr_value = reader
                .read_u32_le()
                .map_err(|e| format!("cannot read {field} attr value: {e}"))?;
            attributes.push(slskr_client::protocol::peer::FileAttribute {
                code: attr_code,
                value: attr_value,
            });
        }
        entries.push(FileEntry {
            code,
            filename,
            filename_encoding,
            size,
            extension,
            extension_encoding,
            attributes,
        });
    }
    Ok(entries)
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
    let mut groups: std::collections::BTreeMap<String, Vec<FileEntry>> =
        std::collections::BTreeMap::new();
    for entry in entries {
        let folder = virtual_folder(&entry.filename).to_owned();
        groups.entry(folder).or_default().push(entry.clone());
    }
    groups.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slskr_client::protocol::peer::FileAttribute;

    fn fixture_entry(filename: &str) -> FileEntry {
        FileEntry {
            code: 1,
            filename: filename.to_owned(),
            filename_encoding: Default::default(),
            size: 42,
            extension: "mp3".to_owned(),
            extension_encoding: Default::default(),
            attributes: vec![FileAttribute {
                code: 1,
                value: 320,
            }],
        }
    }

    #[test]
    fn shared_payload_round_trips_through_bounded_decoder() {
        let entries = vec![fixture_entry("Album/track.mp3")];
        let payload = build_shared_file_list_payload(&entries).expect("encode share payload");

        assert_eq!(parse_shared_file_list_payload(&payload).unwrap(), entries);
    }

    #[test]
    fn malformed_share_folder_count_is_rejected_before_allocation() {
        let mut writer = Writer::new();
        writer.write_u32_le((MAX_PARSED_SHARE_FOLDERS + 1) as u32);
        let payload = compress_zlib_payload(&writer.into_inner()).expect("compress payload");

        let error = parse_shared_file_list_payload(&payload).unwrap_err();

        assert!(error.contains("cannot read folder count"));
    }

    #[test]
    fn cache_fields_escape_delimiters_without_chained_allocations() {
        assert_eq!(escape_cache_field("a\\b\tc\nd"), "a\\\\b\\tc\\nd");
    }
}
