use super::{
    join_virtual_path, json_escape, truncate_utf8_bytes, virtual_folder, FileSearchResponse,
    ProtocolTextEncoding, MAX_BROWSE_EXTENSION_BYTES, MAX_BROWSE_FILENAME_BYTES,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BrowseEntry {
    pub(crate) filename: String,
    pub(crate) size: u64,
    pub(crate) extension: String,
    pub(crate) path_encoding: ProtocolTextEncoding,
}

impl BrowseEntry {
    pub(crate) fn from_json_file(
        file: &serde_json::Value,
        directory: Option<&str>,
    ) -> Option<Self> {
        let raw_filename = file
            .get("filename")
            .or_else(|| file.get("name"))?
            .as_str()?
            .to_owned();
        let filename = match directory {
            Some(directory) if !raw_filename.contains('/') && !raw_filename.contains('\\') => {
                join_virtual_path(directory.trim_matches('/'), &raw_filename)
            }
            _ => raw_filename,
        };
        let extension = file
            .get("extension")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| filename.split('.').next_back().unwrap_or("").to_owned());
        Some(Self {
            filename: truncate_utf8_bytes(filename, MAX_BROWSE_FILENAME_BYTES),
            size: file
                .get("size")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            extension: truncate_utf8_bytes(extension, MAX_BROWSE_EXTENSION_BYTES),
            path_encoding: ProtocolTextEncoding::Utf8,
        })
    }

    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"filename\":\"{}\",\"size\":{},\"extension\":\"{}\"}}",
            json_escape(&self.filename),
            self.size,
            json_escape(&self.extension)
        )
    }
}

const MAX_REMOTE_PATH_ENCODINGS: usize = 50_000;

#[derive(Debug, Default)]
pub(crate) struct RemotePathEncodingRegistry {
    entries: BTreeMap<(String, String), ProtocolTextEncoding>,
}

impl RemotePathEncodingRegistry {
    pub(crate) fn remember(&mut self, username: &str, path: &str, encoding: ProtocolTextEncoding) {
        if username.is_empty() || path.is_empty() {
            return;
        }
        let key = (username.to_owned(), path.to_owned());
        if self.entries.len() >= MAX_REMOTE_PATH_ENCODINGS && !self.entries.contains_key(&key) {
            if let Some(oldest_key) = self.entries.keys().next().cloned() {
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(key, encoding);
    }

    pub(crate) fn encoding_for(&self, username: &str, path: &str) -> ProtocolTextEncoding {
        self.entries
            .get(&(username.to_owned(), path.to_owned()))
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn remember_browse_entries(&mut self, username: &str, entries: &[BrowseEntry]) {
        for entry in entries {
            self.remember(username, &entry.filename, entry.path_encoding);
            let mut folder = virtual_folder(&entry.filename);
            while !folder.is_empty() {
                self.remember(username, folder, entry.path_encoding);
                folder = virtual_folder(folder);
            }
        }
    }

    pub(crate) fn remember_search_response(&mut self, response: &FileSearchResponse) {
        for entry in response.results.iter().chain(&response.private_results) {
            self.remember(&response.username, &entry.filename, entry.filename_encoding);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProtocolTextEncoding;
    use super::{BrowseEntry, RemotePathEncodingRegistry, MAX_REMOTE_PATH_ENCODINGS};

    #[test]
    fn browse_entry_parses_directory_path_and_infers_extension() {
        let file = serde_json::json!({
            "filename": "track.flac",
            "size": 42,
        });

        let entry = BrowseEntry::from_json_file(&file, Some("/music/")).unwrap();

        assert_eq!(entry.filename, "music/track.flac");
        assert_eq!(entry.size, 42);
        assert_eq!(entry.extension, "flac");
        assert_eq!(entry.path_encoding, ProtocolTextEncoding::Utf8);
    }

    #[test]
    fn path_encoding_registry_isolates_users_and_rejects_empty_keys() {
        let mut registry = RemotePathEncodingRegistry::default();
        registry.remember(
            "friend",
            "music/song.flac",
            ProtocolTextEncoding::Windows1251,
        );
        registry.remember("", "ignored", ProtocolTextEncoding::Windows1251);
        registry.remember("friend", "", ProtocolTextEncoding::Windows1251);

        assert_eq!(
            registry.encoding_for("friend", "music/song.flac"),
            ProtocolTextEncoding::Windows1251
        );
        assert_eq!(
            registry.encoding_for("other", "music/song.flac"),
            ProtocolTextEncoding::Utf8
        );
        assert_eq!(registry.entries.len(), 1);
    }

    #[test]
    fn path_encoding_registry_evicts_at_its_capacity() {
        let mut registry = RemotePathEncodingRegistry::default();
        for index in 0..MAX_REMOTE_PATH_ENCODINGS {
            registry.remember(
                "friend",
                &format!("path-{index:05}"),
                ProtocolTextEncoding::Utf8,
            );
        }

        registry.remember("friend", "new-path", ProtocolTextEncoding::Windows1251);

        assert_eq!(registry.entries.len(), MAX_REMOTE_PATH_ENCODINGS);
        assert_eq!(
            registry.encoding_for("friend", "new-path"),
            ProtocolTextEncoding::Windows1251
        );
    }
}
