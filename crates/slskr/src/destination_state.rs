use super::{canonicalize_with_missing_tail, normalize_absolute_path, MAX_DESTINATIONS};
use std::path::{Component, Path, PathBuf};

// Destination Models
#[derive(Clone, Debug)]
pub(crate) struct DestinationRecord {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) is_default: bool,
}

impl DestinationRecord {
    pub(crate) fn json(&self) -> String {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "path": self.path,
            "isDefault": self.is_default,
            "is_default": self.is_default,
            "exists": std::path::Path::new(&self.path).exists(),
        })
        .to_string()
    }
}

#[derive(Debug)]
pub(crate) struct DestinationStore {
    pub(crate) records: Vec<DestinationRecord>,
}

impl DestinationStore {
    pub(crate) fn new() -> Self {
        Self {
            records: vec![DestinationRecord {
                id: "default".to_string(),
                name: "Default".to_string(),
                path: "/home/user/Downloads".to_string(),
                is_default: true,
            }],
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::DestinationRecord>) -> Self {
        if records.is_empty() {
            return Self::new();
        }
        let mut seen_ids = std::collections::HashSet::new();
        let mut records = records
            .into_iter()
            .filter(|record| seen_ids.insert(record.id.clone()))
            .take(MAX_DESTINATIONS)
            .map(|record| DestinationRecord {
                id: record.id,
                name: record.name,
                path: record.path,
                is_default: record.is_default,
            })
            .collect::<Vec<_>>();
        let default_index = records
            .iter()
            .position(|record| record.is_default)
            .unwrap_or(0);
        for (index, record) in records.iter_mut().enumerate() {
            record.is_default = index == default_index;
        }
        records.sort_by(|left, right| {
            right
                .is_default
                .cmp(&left.is_default)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id))
        });
        Self { records }
    }

    pub(crate) fn from_config(
        downloads_dir: &Path,
        configured: &[crate::config::DestinationSettings],
    ) -> Self {
        let downloads = downloads_dir.display().to_string();
        let mut records = vec![DestinationRecord {
            id: "default".to_owned(),
            name: "Downloads".to_owned(),
            path: downloads.clone(),
            is_default: !configured.iter().any(|destination| destination.default),
        }];
        records.extend(
            configured
                .iter()
                .enumerate()
                .filter_map(|(index, destination)| {
                    let path = destination.path.display().to_string();
                    (path != downloads).then(|| DestinationRecord {
                        id: format!("configured-{index}"),
                        name: if destination.name.trim().is_empty() {
                            destination
                                .path
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or(path.as_str())
                                .to_owned()
                        } else {
                            destination.name.clone()
                        },
                        path,
                        is_default: destination.default,
                    })
                }),
        );
        Self { records }
    }

    pub(crate) fn list(&self) -> String {
        let records = self
            .records
            .iter()
            .map(|record| {
                serde_json::from_str::<serde_json::Value>(&record.json()).unwrap_or_default()
            })
            .collect::<Vec<_>>();
        serde_json::Value::Array(records).to_string()
    }

    pub(crate) fn versioned_record_value(record: &DestinationRecord) -> serde_json::Value {
        serde_json::json!({
            "name": if record.id == "default" && record.name == "Default" {
                "Downloads"
            } else {
                record.name.as_str()
            },
            "path": record.path,
            "isDefault": record.is_default,
            "exists": std::path::Path::new(&record.path).exists(),
        })
    }

    pub(crate) fn versioned_list(&self) -> String {
        serde_json::Value::Array(
            self.records
                .iter()
                .map(Self::versioned_record_value)
                .collect(),
        )
        .to_string()
    }

    pub(crate) fn versioned_default(&self) -> String {
        let record = self
            .records
            .iter()
            .find(|record| record.is_default)
            .unwrap_or(&self.records[0]);
        Self::versioned_record_value(record).to_string()
    }

    pub(crate) fn default(&self) -> String {
        self.records
            .iter()
            .find(|record| record.is_default)
            .unwrap_or(&self.records[0])
            .json()
    }

    pub(crate) fn normalize_explicit_path(&self, requested: &str) -> Option<PathBuf> {
        let requested = Path::new(requested.trim());
        if !requested.is_absolute() {
            return None;
        }
        let mut normalized = PathBuf::new();
        for component in requested.components() {
            match component {
                Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return None;
                    }
                }
                Component::Normal(value) => normalized.push(value),
            }
        }
        if !normalized.is_absolute() {
            return None;
        }

        self.records
            .iter()
            .any(|record| {
                let root = Path::new(&record.path);
                let Some(root) = normalize_absolute_path(root) else {
                    return false;
                };
                if !normalized.starts_with(&root) {
                    return false;
                }
                canonicalize_with_missing_tail(&normalized)
                    .zip(canonicalize_with_missing_tail(&root))
                    .is_some_and(|(canonical_path, canonical_root)| {
                        canonical_path.starts_with(canonical_root)
                    })
            })
            .then_some(normalized)
    }
}
