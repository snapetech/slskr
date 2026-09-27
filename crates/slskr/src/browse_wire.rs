use super::*;

pub(super) const MAX_BROWSE_WIRE_SECTIONS_PER_RESPONSE: usize = 64;
pub(super) const MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE: usize = 20_000;
pub(super) const MAX_BROWSE_WIRE_FILES_PER_RESPONSE: usize = 20_000;

pub(super) fn build_shared_file_list_payload(entries: &[FileEntry]) -> Result<Vec<u8>, String> {
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

pub(super) fn build_empty_browse_payload() -> Result<Vec<u8>, String> {
    let mut writer = Writer::new();
    writer.write_u32_le(0);
    writer.write_u32_le(0);
    writer.write_u32_le(0);
    compress_zlib_payload(&writer.into_inner()).map_err(|error| error.to_string())
}

pub(super) fn build_folder_contents_payload(
    entries: &[FileEntry],
    token: u32,
    folder: &str,
    folder_encoding: ProtocolTextEncoding,
) -> Result<Vec<u8>, String> {
    let matching = entries
        .iter()
        .filter(|entry| virtual_folder(&entry.filename) == folder)
        .map(|entry| {
            let mut file = entry.clone();
            file.filename = entry
                .filename
                .rsplit_once('/')
                .map(|(_, filename)| filename.to_owned())
                .unwrap_or_else(|| entry.filename.clone());
            file
        })
        .collect::<Vec<_>>();
    let mut writer = Writer::new();
    writer.write_u32_le(token);
    writer
        .write_string_with_encoding(folder, folder_encoding)
        .map_err(|error| error.to_string())?;
    writer.write_u32_le(1);
    writer
        .write_string_with_encoding(folder, folder_encoding)
        .map_err(|error| error.to_string())?;
    writer.write_u32_le(
        u32::try_from(matching.len()).map_err(|_| "too many folder files".to_owned())?,
    );
    for file in &matching {
        encode_file_entry(&mut writer, file)?;
    }
    compress_zlib_payload(&writer.into_inner()).map_err(|error| error.to_string())
}

pub(super) fn parse_shared_file_list_payload(payload: &[u8]) -> Result<Vec<BrowseEntry>, String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let mut entries = Vec::new();
    let mut wire_section_count = 0;
    let mut wire_folder_count = 0;
    let mut wire_file_count = 0;
    loop {
        if wire_section_count >= MAX_BROWSE_WIRE_SECTIONS_PER_RESPONSE {
            return Err(format!(
                "shared file list exceeds {MAX_BROWSE_WIRE_SECTIONS_PER_RESPONSE} sections"
            ));
        }
        wire_section_count += 1;
        parse_shared_file_list_section(
            &mut reader,
            &mut entries,
            &mut wire_folder_count,
            &mut wire_file_count,
        )?;
        if reader.is_empty() {
            break;
        }
    }
    Ok(entries)
}

fn parse_shared_file_list_section(
    reader: &mut Reader<'_>,
    entries: &mut Vec<BrowseEntry>,
    wire_folder_count: &mut usize,
    wire_file_count: &mut usize,
) -> Result<(), String> {
    let folder_count = reader
        .read_bounded_count("shared folders", 8)
        .map_err(|error| error.to_string())?;
    add_browse_wire_count(
        wire_folder_count,
        folder_count,
        "shared folders",
        MAX_BROWSE_WIRE_FOLDERS_PER_RESPONSE,
    )?;
    for _ in 0..folder_count {
        let (folder, folder_encoding) = reader
            .read_string_with_encoding()
            .map_err(|error| error.to_string())?;
        let file_count = reader
            .read_bounded_count("shared files", 21)
            .map_err(|error| error.to_string())?;
        add_browse_wire_count(
            wire_file_count,
            file_count,
            "shared files",
            MAX_BROWSE_WIRE_FILES_PER_RESPONSE,
        )?;
        parse_browse_file_entries(
            reader,
            file_count,
            &folder,
            folder_encoding,
            entries,
            "shared file",
            "shared file attributes",
        )?;
    }
    Ok(())
}

pub(super) fn parse_folder_file_list_payload(
    payload: &[u8],
    folder: &str,
    folder_encoding: ProtocolTextEncoding,
) -> Result<Vec<BrowseEntry>, String> {
    let decompressed = decompress_zlib_payload(payload).map_err(|error| error.to_string())?;
    let mut reader = Reader::new(&decompressed);
    let file_count = reader
        .read_bounded_count("folder files", 21)
        .map_err(|error| error.to_string())?;
    if file_count > MAX_BROWSE_WIRE_FILES_PER_RESPONSE {
        return Err(format!(
            "folder files exceeds {MAX_BROWSE_WIRE_FILES_PER_RESPONSE} entries"
        ));
    }
    let mut entries = Vec::new();
    parse_browse_file_entries(
        &mut reader,
        file_count,
        folder,
        folder_encoding,
        &mut entries,
        "folder file",
        "folder file attributes",
    )?;
    reader.finish().map_err(|error| error.to_string())?;
    Ok(entries)
}

fn parse_browse_file_entries(
    reader: &mut Reader<'_>,
    file_count: usize,
    folder: &str,
    folder_encoding: ProtocolTextEncoding,
    entries: &mut Vec<BrowseEntry>,
    entry_label: &str,
    attribute_label: &'static str,
) -> Result<(), String> {
    for _ in 0..file_count {
        let code = reader.read_u8().map_err(|error| error.to_string())?;
        let (filename, filename_encoding) = reader
            .read_string_with_encoding()
            .map_err(|error| error.to_string())?;
        let size = reader.read_u64_le().map_err(|error| error.to_string())?;
        let (extension, _) = reader
            .read_string_with_encoding()
            .map_err(|error| error.to_string())?;
        let attribute_count = reader
            .read_bounded_count(attribute_label, 8)
            .map_err(|error| error.to_string())?;
        for _ in 0..attribute_count {
            let _code = reader.read_u32_le().map_err(|error| error.to_string())?;
            let _value = reader.read_u32_le().map_err(|error| error.to_string())?;
        }
        if code != 1 {
            continue;
        }
        if entries.len() >= MAX_BROWSE_ENTRIES_PER_USER {
            return Err(format!(
                "{entry_label} list exceeds {MAX_BROWSE_ENTRIES_PER_USER} entries"
            ));
        }
        entries.push(bounded_browse_entry(BrowseEntry {
            filename: join_virtual_path(folder, &filename),
            size,
            extension,
            path_encoding: if folder_encoding == ProtocolTextEncoding::Utf8 {
                filename_encoding
            } else {
                folder_encoding
            },
        }));
    }
    Ok(())
}

pub(super) fn add_browse_wire_count(
    total: &mut usize,
    additional: usize,
    field: &str,
    maximum: usize,
) -> Result<(), String> {
    if additional > maximum.saturating_sub(*total) {
        return Err(format!("{field} exceeds {maximum} entries"));
    }
    *total += additional;
    Ok(())
}

pub(super) fn join_virtual_path(folder: &str, filename: &str) -> String {
    if folder.is_empty() {
        filename.to_owned()
    } else {
        format!("{folder}/{filename}")
    }
}

pub(super) fn virtual_folder(filename: &str) -> &str {
    filename
        .rsplit_once('/')
        .map(|(folder, _)| folder)
        .unwrap_or("")
}

pub(super) fn group_share_entries(entries: &[FileEntry]) -> Vec<(String, Vec<FileEntry>)> {
    let mut folders: Vec<(String, Vec<FileEntry>)> = Vec::new();
    for entry in entries {
        let (folder, filename) = entry
            .filename
            .rsplit_once('/')
            .map(|(folder, filename)| (folder.to_owned(), filename.to_owned()))
            .unwrap_or_else(|| ("".to_owned(), entry.filename.clone()));
        let mut file = entry.clone();
        file.filename = filename;
        if let Some((_, files)) = folders
            .iter_mut()
            .find(|(existing_folder, _)| *existing_folder == folder)
        {
            files.push(file);
        } else {
            folders.push((folder, vec![file]));
        }
    }
    folders
}

pub(super) fn group_browse_entries(entries: &[BrowseEntry]) -> Vec<(String, Vec<BrowseEntry>)> {
    let mut folders: Vec<(String, Vec<BrowseEntry>)> = Vec::new();
    for entry in entries {
        let (folder, filename) = entry
            .filename
            .rsplit_once('/')
            .map(|(folder, filename)| (folder.to_owned(), filename.to_owned()))
            .unwrap_or_else(|| ("".to_owned(), entry.filename.clone()));
        let mut file = entry.clone();
        file.filename = filename;
        if let Some((_, files)) = folders
            .iter_mut()
            .find(|(existing_folder, _)| *existing_folder == folder)
        {
            files.push(file);
        } else {
            folders.push((folder, vec![file]));
        }
    }
    folders
}

fn encode_file_entry(writer: &mut Writer, entry: &FileEntry) -> Result<(), String> {
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
