use super::{
    append_escaped_cache_field, json_escape, json_option, json_u64_option, json_usize_option,
    non_empty, parse_list_limit, query_params, summarize_extensions, unix_timestamp,
    write_file_atomic, AppConfig, ControllerProfile, FileAttribute, FileEntry, DEFAULT_LIST_LIMIT,
};
use crate::persistence;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub(super) struct CatalogFilter {
    pub(super) q: Option<String>,
    pub(super) prefix: Option<String>,
    pub(super) extension: Option<String>,
    pub(super) limit: Option<usize>,
    pub(super) offset: usize,
}

impl CatalogFilter {
    pub(super) fn from_query(query: Option<&str>) -> Self {
        let mut filter = Self {
            limit: Some(DEFAULT_LIST_LIMIT),
            ..Self::default()
        };
        for (name, value) in query_params(query.unwrap_or_default()) {
            match name.as_str() {
                "q" => filter.q = non_empty(value.to_ascii_lowercase()),
                "prefix" => filter.prefix = non_empty(value),
                "extension" => {
                    filter.extension =
                        non_empty(value.trim_start_matches('.').to_ascii_lowercase());
                }
                "limit" => filter.limit = Some(parse_list_limit(&value)),
                "offset" => filter.offset = value.parse::<usize>().unwrap_or(0),
                _ => {}
            }
        }
        filter
    }

    fn matches(&self, entry: &FileEntry) -> bool {
        if let Some(q) = &self.q {
            if !entry.filename.to_ascii_lowercase().contains(q) {
                return false;
            }
        }
        if let Some(prefix) = &self.prefix {
            if !entry.filename.starts_with(prefix) {
                return false;
            }
        }
        if let Some(extension) = &self.extension {
            if entry.extension.to_ascii_lowercase() != *extension {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ShareRoot {
    pub(crate) label: String,
    pub(crate) local_path: PathBuf,
    pub(crate) raw: String,
    pub(crate) directories: usize,
    pub(crate) files: usize,
    pub(crate) bytes: u64,
    pub(crate) extensions: Vec<ShareExtensionSummary>,
    pub(crate) statistics_ready: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ShareExtensionSummary {
    pub(crate) extension: String,
    pub(crate) files: usize,
    pub(crate) bytes: u64,
}

impl ShareExtensionSummary {
    fn json(&self) -> String {
        format!(
            "{{\"extension\":\"{}\",\"files\":{},\"bytes\":{}}}",
            json_escape(&self.extension),
            self.files,
            self.bytes
        )
    }
}

impl ShareRoot {
    pub(crate) fn json(&self) -> String {
        let extensions = self
            .extensions
            .iter()
            .map(ShareExtensionSummary::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"label\":\"{}\",\"files\":{},\"bytes\":{},\"extensions\":[{}]}}",
            json_escape(&self.label),
            self.files,
            self.bytes,
            extensions
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ShareIndexSnapshot {
    pub(crate) entries: Vec<FileEntry>,
    pub(crate) local_paths: BTreeMap<String, PathBuf>,
    pub(crate) roots: Vec<ShareRoot>,
    pub(crate) fixture_files: usize,
    pub(crate) scan_errors: Vec<String>,
    pub(crate) cache_path: PathBuf,
    pub(crate) cache_enabled: bool,
    pub(crate) cache_written_at: Option<u64>,
    pub(crate) cache_error: Option<String>,
    pub(crate) updated_at: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ShareLifecycleState {
    pub(crate) scan_pending: bool,
    pub(crate) scanning: bool,
    pub(crate) ready: bool,
    pub(crate) faulted: bool,
    pub(crate) cancelled: bool,
    pub(crate) scan_progress: f64,
    pub(crate) directories: usize,
    pub(crate) files: usize,
}

impl ShareLifecycleState {
    pub(crate) fn from_snapshot(snapshot: &ShareIndexSnapshot) -> Self {
        Self {
            scan_pending: false,
            scanning: false,
            ready: true,
            faulted: false,
            cancelled: false,
            scan_progress: 1.0,
            directories: snapshot.roots.iter().map(|root| root.directories).sum(),
            files: snapshot.entries.len(),
        }
    }

    pub(crate) fn uninitialized() -> Self {
        Self {
            scan_pending: false,
            scanning: false,
            ready: false,
            faulted: false,
            cancelled: false,
            scan_progress: 0.0,
            directories: 0,
            files: 0,
        }
    }

    pub(crate) fn json(&self, target: ControllerProfile) -> serde_json::Value {
        let scan_progress = if self.scan_progress.fract() == 0.0 {
            serde_json::json!(self.scan_progress as u64)
        } else {
            serde_json::json!(self.scan_progress)
        };
        let mut value = serde_json::json!({
            "scanPending": self.scan_pending,
            "scanning": self.scanning,
            "ready": self.ready,
            "faulted": self.faulted,
            "cancelled": self.cancelled,
            "scanProgress": scan_progress,
            "directories": self.directories,
            "files": self.files,
        });
        if self.ready {
            value["hosts"] = serde_json::json!(["local"]);
        } else if target == ControllerProfile::Native {
            value["hosts"] = serde_json::json!([]);
        }
        value
    }
}

pub(crate) fn pending_share_roots(settings: &crate::config::ShareSettings) -> Vec<ShareRoot> {
    let mut directories = settings.directories.iter().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .local_path
            .as_os_str()
            .len()
            .cmp(&left.local_path.as_os_str().len())
    });
    directories
        .into_iter()
        .map(|directory| ShareRoot {
            label: directory.alias.clone(),
            local_path: directory.local_path.clone(),
            raw: directory.raw.clone(),
            directories: 0,
            files: 0,
            bytes: 0,
            extensions: Vec::new(),
            statistics_ready: false,
        })
        .collect()
}

impl ShareIndexSnapshot {
    pub(crate) fn uninitialized(config: &AppConfig) -> Self {
        Self {
            entries: Vec::new(),
            local_paths: BTreeMap::new(),
            roots: pending_share_roots(&config.share_settings),
            fixture_files: 0,
            scan_errors: Vec::new(),
            cache_path: share_cache_path(&config.state_dir),
            cache_enabled: config.share_settings.cache_tsv_enabled,
            cache_written_at: None,
            cache_error: None,
            updated_at: unix_timestamp(),
        }
    }

    pub(crate) fn json(&self) -> String {
        let roots = self
            .roots
            .iter()
            .map(ShareRoot::json)
            .collect::<Vec<_>>()
            .join(",");
        let errors = self
            .scan_errors
            .iter()
            .map(|error| json_option(Some(error.as_str())))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"roots\":[{}],\"files\":{},\"fixture_files\":{},\"scan_errors\":[{}],\"cache_file\":\"{}\",\"cache_enabled\":{},\"cache_kind\":\"compatibility-debug\",\"cache_written_at\":{},\"cache_error\":{},\"updated_at\":{}}}",
            roots,
            self.entries.len(),
            self.fixture_files,
            errors,
            json_escape(
                self.cache_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("share-index.tsv")
            ),
            self.cache_enabled,
            json_u64_option(self.cache_written_at),
            json_option(public_share_cache_error(self.cache_error.as_deref())),
            self.updated_at
        )
    }

    pub(crate) fn summary_json(&self) -> String {
        let bytes = self.entries.iter().map(|entry| entry.size).sum::<u64>();
        format!(
            "{{\"roots\":{},\"files\":{},\"fixture_files\":{},\"bytes\":{},\"scan_errors\":{},\"cache_enabled\":{},\"cache_error\":{},\"updated_at\":{}}}",
            self.roots.len(),
            self.entries.len(),
            self.fixture_files,
            bytes,
            self.scan_errors.len(),
            self.cache_enabled,
            self.cache_error.is_some(),
            self.updated_at
        )
    }

    pub(crate) fn catalog_json(&self, query: Option<&str>) -> String {
        let filter = CatalogFilter::from_query(query);
        let mut entries = self
            .entries
            .iter()
            .filter(|entry| filter.matches(entry))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.filename.cmp(&right.filename));
        let filtered_count = entries.len();
        let total_bytes = entries.iter().map(|entry| entry.size).sum::<u64>();
        let files = entries
            .iter()
            .skip(filter.offset)
            .take(filter.limit.unwrap_or(usize::MAX))
            .map(|entry| {
                format!(
                    "{{\"path\":\"{}\",\"size\":{},\"extension\":\"{}\",\"attribute_count\":{}}}",
                    json_escape(&entry.filename),
                    entry.size,
                    json_escape(&entry.extension),
                    entry.attributes.len()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"files\":[{}],\"count\":{},\"filtered_count\":{},\"total_bytes\":{},\"offset\":{},\"limit\":{},\"updated_at\":{}}}",
            files,
            self.entries.len(),
            filtered_count,
            total_bytes,
            filter.offset,
            json_usize_option(filter.limit),
            self.updated_at
        )
    }

    pub(crate) fn from_persisted(
        config: &AppConfig,
        records: Vec<persistence::ShareFileRecord>,
    ) -> Self {
        let updated_at = records
            .iter()
            .map(|record| record.updated_at.max(0) as u64)
            .max()
            .unwrap_or_else(unix_timestamp);
        let mut local_paths = BTreeMap::new();
        let entries = records
            .iter()
            .map(|record| {
                if let Some(local_path) = record
                    .local_path
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    local_paths.insert(record.filename.clone(), PathBuf::from(local_path));
                }
                FileEntry {
                    filename_encoding: Default::default(),
                    extension_encoding: Default::default(),
                    code: 1,
                    filename: record.filename.clone(),
                    size: record.size.max(0) as u64,
                    extension: record.extension.clone(),
                    attributes: decode_share_attributes(&record.attributes_json),
                }
            })
            .collect::<Vec<_>>();
        let roots = summarize_share_roots_from_persisted(config, &records);
        let cache_path = share_cache_path(&config.state_dir);
        let (cache_written_at, cache_error) = write_share_cache_if_enabled(
            config.share_settings.cache_tsv_enabled,
            &cache_path,
            &entries,
        );
        Self {
            entries,
            local_paths,
            roots,
            fixture_files: 0,
            scan_errors: Vec::new(),
            cache_path,
            cache_enabled: config.share_settings.cache_tsv_enabled,
            cache_written_at,
            cache_error,
            updated_at,
        }
    }
}

pub(super) fn public_share_cache_error(error: Option<&str>) -> Option<&'static str> {
    error.map(|_| "share cache unavailable")
}

fn summarize_share_roots_from_persisted(
    config: &AppConfig,
    records: &[persistence::ShareFileRecord],
) -> Vec<ShareRoot> {
    let mut grouped = BTreeMap::<String, Vec<FileEntry>>::new();
    for record in records {
        grouped
            .entry(record.root_label.clone())
            .or_default()
            .push(FileEntry {
                filename_encoding: Default::default(),
                extension_encoding: Default::default(),
                code: 1,
                filename: record.filename.clone(),
                size: record.size.max(0) as u64,
                extension: record.extension.clone(),
                attributes: Vec::new(),
            });
    }
    let mut ordered_directories = config.share_settings.directories.iter().collect::<Vec<_>>();
    ordered_directories.sort_by(|left, right| {
        right
            .local_path
            .as_os_str()
            .len()
            .cmp(&left.local_path.as_os_str().len())
    });
    let mut roots = ordered_directories
        .into_iter()
        .map(|directory| {
            let entries = grouped.remove(&directory.alias).unwrap_or_default();
            ShareRoot {
                label: directory.alias.clone(),
                local_path: directory.local_path.clone(),
                raw: directory.raw.clone(),
                directories: persisted_share_directory_count(&entries),
                files: entries.len(),
                bytes: entries.iter().map(|entry| entry.size).sum(),
                extensions: summarize_extensions(&entries),
                statistics_ready: true,
            }
        })
        .collect::<Vec<_>>();
    roots.extend(grouped.into_iter().map(|(label, entries)| ShareRoot {
        local_path: PathBuf::from(&label),
        raw: label.clone(),
        directories: persisted_share_directory_count(&entries),
        label,
        files: entries.len(),
        bytes: entries.iter().map(|entry| entry.size).sum(),
        extensions: summarize_extensions(&entries),
        statistics_ready: true,
    }));
    roots
}

fn persisted_share_directory_count(entries: &[FileEntry]) -> usize {
    entries
        .iter()
        .filter_map(|entry| entry.filename.split_once('/').map(|(_, path)| path))
        .filter_map(|path| path.rsplit_once('/').map(|(directory, _)| directory))
        .flat_map(|directory| {
            let parts = directory.split('/').collect::<Vec<_>>();
            (1..=parts.len())
                .map(move |count| parts[..count].join("/"))
                .collect::<Vec<_>>()
        })
        .collect::<HashSet<_>>()
        .len()
}

pub(super) fn persisted_share_file_records(
    snapshot: &ShareIndexSnapshot,
) -> Vec<persistence::ShareFileRecord> {
    let updated_at = snapshot.updated_at as i64;
    snapshot
        .entries
        .iter()
        .map(|entry| {
            let root_label = entry
                .filename
                .split(['/', '\\'])
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("shares")
                .to_owned();
            persistence::ShareFileRecord {
                filename: entry.filename.clone(),
                size: entry.size as i64,
                extension: entry.extension.clone(),
                root_label,
                local_path: snapshot
                    .local_paths
                    .get(&entry.filename)
                    .map(|path| path.display().to_string()),
                attributes_json: encode_share_attributes(&entry.attributes),
                updated_at,
            }
        })
        .collect()
}

fn encode_share_attributes(attributes: &[FileAttribute]) -> String {
    attributes
        .iter()
        .map(|attribute| format!("{}:{}", attribute.code, attribute.value))
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_share_attributes(value: &str) -> Vec<FileAttribute> {
    value
        .split(',')
        .filter_map(|pair| {
            let (code, value) = pair.split_once(':')?;
            Some(FileAttribute {
                code: code.parse().ok()?,
                value: value.parse().ok()?,
            })
        })
        .take(slskr_client::protocol::peer::MAX_FILE_ATTRIBUTES)
        .collect()
}

pub(super) fn share_cache_path(state_dir: &Path) -> PathBuf {
    state_dir.join("share-index.tsv")
}

pub(super) fn write_share_cache_if_enabled(
    enabled: bool,
    path: &Path,
    entries: &[FileEntry],
) -> (Option<u64>, Option<String>) {
    if !enabled {
        return (None, None);
    }
    match write_share_cache(path, entries) {
        Ok(()) => (Some(unix_timestamp()), None),
        Err(error) => (None, Some(error)),
    }
}

fn write_share_cache(path: &Path, entries: &[FileEntry]) -> Result<(), String> {
    use std::fmt::Write as _;

    let mut body = String::from("slskr-share-index-v1\n");
    for entry in entries {
        let _ = write!(body, "{}\t{}\t", entry.code, entry.size);
        append_escaped_cache_field(&mut body, &entry.extension);
        body.push('\t');
        append_escaped_cache_field(&mut body, &entry.filename);
        body.push('\n');
    }
    write_file_atomic(path, body).map_err(|error| format!("share cache write failed: {error}"))
}
