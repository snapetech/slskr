use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const STATE_VERSION: u32 = 1;
const MAX_INDEXES: usize = 1_024;
const MAX_STATE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Default, Deserialize, Serialize)]
struct PersistedState {
    version: u32,
    indexes: BTreeMap<String, Value>,
    authority_decisions: BTreeMap<String, Value>,
}

#[derive(Debug)]
pub struct Store {
    state_path: Option<PathBuf>,
    indexes: BTreeMap<String, Value>,
    authority_decisions: BTreeMap<String, Value>,
}

#[derive(Clone, Debug)]
struct IndexedEntry {
    entry: Value,
    realm_id: String,
    index_id: String,
    revision: i64,
    namespace: String,
    subject_id: String,
    title: String,
    creator: String,
    aliases: Vec<String>,
    external_ids: BTreeMap<String, String>,
}

impl Store {
    pub fn in_memory() -> Self {
        Self {
            state_path: None,
            indexes: BTreeMap::new(),
            authority_decisions: BTreeMap::new(),
        }
    }

    pub fn load(state_dir: &Path) -> Result<Self, String> {
        let state_path = state_dir.join("realm-subject-indexes.json");
        if !state_path.exists() {
            return Ok(Self {
                state_path: Some(state_path),
                ..Self::in_memory()
            });
        }
        let bytes = fs::read(&state_path)
            .map_err(|error| format!("realm subject-index state read failed: {error}"))?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err("realm subject-index state exceeds the 8 MiB limit".to_owned());
        }
        let mut persisted = serde_json::from_slice::<PersistedState>(&bytes)
            .map_err(|error| format!("realm subject-index state parse failed: {error}"))?;
        if persisted.version != STATE_VERSION
            || persisted.indexes.len() > MAX_INDEXES
            || persisted.authority_decisions.len() > MAX_INDEXES
        {
            return Err("realm subject-index state is unsupported or over capacity".to_owned());
        }
        Ok(Self {
            state_path: Some(state_path),
            indexes: std::mem::take(&mut persisted.indexes),
            authority_decisions: std::mem::take(&mut persisted.authority_decisions),
        })
    }

    pub fn merge_indexes(&mut self, indexes: Vec<Value>) -> Result<usize, String> {
        if indexes.is_empty() || indexes.len() > MAX_INDEXES {
            return Err(format!(
                "realm subject-index merge requires 1 to {MAX_INDEXES} indexes"
            ));
        }
        let previous = self.indexes.clone();
        for index in indexes {
            let key = validate_index(&index)?;
            if self.indexes.len() >= MAX_INDEXES && !self.indexes.contains_key(&key) {
                self.indexes = previous;
                return Err("realm subject-index capacity is full".to_owned());
            }
            self.indexes.insert(key, index);
        }
        if let Err(error) = self.persist() {
            self.indexes = previous;
            return Err(error);
        }
        Ok(self.indexes.len().saturating_sub(previous.len()))
    }

    pub fn indexes_for_realm(&self, realm_id: &str) -> Vec<Value> {
        let realm_id = normalize(realm_id);
        let mut indexes = self
            .indexes
            .values()
            .filter(|index| index_string(index, "realmId").is_some_and(|value| value == realm_id))
            .cloned()
            .collect::<Vec<_>>();
        indexes.sort_by(|left, right| {
            index_string(left, "id")
                .cmp(&index_string(right, "id"))
                .then_with(|| revision(right).cmp(&revision(left)))
        });
        indexes
    }

    pub fn authority_decisions_for_realm(&self, realm_id: &str) -> Vec<Value> {
        let realm_id = normalize(realm_id);
        let mut decisions = self
            .authority_decisions
            .values()
            .filter(|decision| {
                index_string(decision, "realmId").is_some_and(|value| value == realm_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        decisions.sort_by(|left, right| {
            index_string(left, "indexId").cmp(&index_string(right, "indexId"))
        });
        decisions
    }

    pub fn set_authority_decision(
        &mut self,
        realm_id: &str,
        index_id: &str,
        enabled: bool,
        decided_by: &str,
        note: &str,
        decided_at: &str,
    ) -> Result<Value, String> {
        let key = index_key(realm_id, index_id);
        if !self.indexes.contains_key(&key) {
            return Err("Index authority was not found.".to_owned());
        }
        let decision = serde_json::json!({
            "isAccepted": true,
            "realmId": realm_id,
            "indexId": index_id,
            "enabled": enabled,
            "decidedBy": decided_by,
            "note": note,
            "decidedAt": decided_at,
            "errors": [],
        });
        let previous = self.authority_decisions.insert(key, decision.clone());
        if let Err(error) = self.persist() {
            if let Some(previous) = previous {
                self.authority_decisions
                    .insert(index_key(realm_id, index_id), previous);
            } else {
                self.authority_decisions
                    .remove(&index_key(realm_id, index_id));
            }
            return Err(error);
        }
        Ok(decision)
    }

    pub fn resolve_recording(&self, recording_id: &str) -> Vec<Value> {
        let recording_id = normalize(recording_id);
        let mut resolutions = Vec::new();
        for index in self.indexes.values().filter(|index| self.is_enabled(index)) {
            let Some(entries) = index.get("entries").and_then(Value::as_array) else {
                continue;
            };
            for entry in entries {
                let Some(indexed) = indexed_entry(index, entry) else {
                    continue;
                };
                let matches = indexed.external_ids.iter().any(|(key, value)| {
                    (key == "musicbrainz" || key == "musicbrainz:recording")
                        && value == &recording_id
                });
                if matches {
                    let realm_id = indexed.realm_id.clone();
                    let index_id = indexed.index_id.clone();
                    let revision = indexed.revision;
                    let provenance =
                        format!("realm:{realm_id}:subject-index:{index_id}:r{revision}");
                    resolutions.push(serde_json::json!({
                        "entry": indexed.entry,
                        "realmId": realm_id,
                        "indexId": index_id,
                        "revision": revision,
                        "provenance": provenance,
                    }));
                }
            }
        }
        resolutions.sort_by(|left, right| {
            index_string(left, "realmId")
                .cmp(&index_string(right, "realmId"))
                .then_with(|| index_string(left, "indexId").cmp(&index_string(right, "indexId")))
        });
        resolutions
    }

    pub fn conflict_report(&self, realm_id: &str) -> Value {
        let realm_id = normalize(realm_id);
        let all_indexes = self.indexes_for_realm(&realm_id);
        let enabled_indexes = all_indexes
            .iter()
            .filter(|index| self.is_enabled(index))
            .cloned()
            .collect::<Vec<_>>();
        let entries = enabled_indexes
            .iter()
            .flat_map(|index| {
                index
                    .get("entries")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| indexed_entry(index, entry))
            })
            .collect::<Vec<_>>();
        let mut conflicts = Vec::new();
        conflicts.extend(external_id_conflicts(&entries));
        conflicts.extend(recording_subject_conflicts(&entries));
        conflicts.extend(work_identity_conflicts(&entries));
        conflicts.extend(alias_subject_conflicts(&entries));
        conflicts.sort_by(|left, right| {
            index_string(left, "type")
                .cmp(&index_string(right, "type"))
                .then_with(|| index_string(left, "key").cmp(&index_string(right, "key")))
                .then_with(|| {
                    index_string(left, "subjectId").cmp(&index_string(right, "subjectId"))
                })
        });
        serde_json::json!({
            "realmId": realm_id,
            "generatedAt": crate::unix_timestamp(),
            "indexCount": enabled_indexes.len(),
            "disabledAuthorityCount": all_indexes.len().saturating_sub(enabled_indexes.len()),
            "entryCount": entries.len(),
            "hasConflicts": !conflicts.is_empty(),
            "realm": realm_id,
            "conflicts": conflicts,
            "count": conflicts.len(),
        })
    }

    fn is_enabled(&self, index: &Value) -> bool {
        let key = index_key(
            &index_string(index, "realmId").unwrap_or_default(),
            &index_string(index, "id").unwrap_or_default(),
        );
        self.authority_decisions
            .get(&key)
            .and_then(|decision| decision.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    fn persist(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&PersistedState {
            version: STATE_VERSION,
            indexes: self.indexes.clone(),
            authority_decisions: self.authority_decisions.clone(),
        })
        .map_err(|error| format!("realm subject-index state serialization failed: {error}"))?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err("realm subject-index state exceeds the 8 MiB limit".to_owned());
        }
        let Some(path) = self.state_path.as_deref() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("realm subject-index state directory failed: {error}"))?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, &bytes)
            .map_err(|error| format!("realm subject-index state write failed: {error}"))?;
        fs::rename(&temporary, path)
            .map_err(|error| format!("realm subject-index state commit failed: {error}"))
    }
}

fn validate_index(index: &Value) -> Result<String, String> {
    let Some(object) = index.as_object() else {
        return Err("realm subject-index must be an object".to_owned());
    };
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Index id is required.".to_owned())?;
    let realm_id = object
        .get("realmId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Realm id is required.".to_owned())?;
    if object
        .get("entries")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        return Err("Index must contain at least one entry.".to_owned());
    }
    Ok(index_key(realm_id, id))
}

fn indexed_entry(index: &Value, entry: &Value) -> Option<IndexedEntry> {
    let realm_id = index_string(index, "realmId")?;
    let index_id = index_string(index, "id")?;
    let subject_id = index_string(entry, "subjectId")?;
    let work_ref = entry.get("workRef").unwrap_or(&Value::Null);
    let mut external_ids = external_ids(entry);
    if let Some(work_ids) = work_ref.get("externalIds").and_then(Value::as_object) {
        for (key, value) in work_ids {
            if let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                external_ids.insert(normalize(key), value.to_owned());
            }
        }
    }
    Some(IndexedEntry {
        entry: entry.clone(),
        realm_id,
        index_id,
        revision: revision(index),
        namespace: index_string(index, "subjectNamespace").unwrap_or_else(|| "music".to_owned()),
        subject_id,
        title: index_string(work_ref, "title").unwrap_or_default(),
        creator: index_string(work_ref, "creator").unwrap_or_default(),
        aliases: entry
            .get("aliases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|alias| {
                alias
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .map(str::to_owned)
            .collect(),
        external_ids,
    })
}

fn external_ids(entry: &Value) -> BTreeMap<String, String> {
    entry
        .get("externalIds")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| Some((normalize(key), value.as_str()?.trim().to_owned())))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .collect()
}

fn external_id_conflicts(entries: &[IndexedEntry]) -> Vec<Value> {
    let mut groups = HashMap::<String, Vec<(&IndexedEntry, String, String)>>::new();
    for entry in entries {
        for (key, value) in &entry.external_ids {
            groups
                .entry(format!(
                    "{}:{}:{}",
                    normalize(&entry.namespace),
                    normalize(&entry.subject_id),
                    key
                ))
                .or_default()
                .push((entry, key.clone(), value.clone()));
        }
    }
    groups
        .into_iter()
        .filter(|(_, values)| distinct_values(values.iter().map(|(_, _, value)| value)).len() > 1)
        .map(|(_, values)| {
            let first = &values[0];
            conflict(
                "external-id",
                &first.0.namespace,
                &first.0.subject_id,
                &first.1,
                format!(
                    "Subject '{}' has conflicting external id values for '{}'.",
                    first.0.subject_id, first.1
                ),
                values
                    .iter()
                    .map(|(entry, _, value)| conflict_value(entry, value)),
            )
        })
        .collect()
}

fn recording_subject_conflicts(entries: &[IndexedEntry]) -> Vec<Value> {
    let mut groups = HashMap::<String, Vec<(&IndexedEntry, String)>>::new();
    for entry in entries {
        for (key, value) in &entry.external_ids {
            if key == "musicbrainz" || key == "musicbrainz:recording" {
                groups
                    .entry(value.clone())
                    .or_default()
                    .push((entry, entry.subject_id.clone()));
            }
        }
    }
    groups
        .into_iter()
        .filter(|(_, values)| distinct_values(values.iter().map(|(_, subject)| subject)).len() > 1)
        .map(|(recording_id, values)| {
            let first = &values[0];
            conflict(
                "recording-subject",
                &first.0.namespace,
                "",
                &recording_id,
                format!("Recording '{recording_id}' maps to multiple realm subjects."),
                values
                    .iter()
                    .map(|(entry, subject)| conflict_value(entry, subject)),
            )
        })
        .collect()
}

fn work_identity_conflicts(entries: &[IndexedEntry]) -> Vec<Value> {
    let mut groups = HashMap::<String, Vec<(&IndexedEntry, String)>>::new();
    for entry in entries {
        groups
            .entry(format!(
                "{}:{}",
                normalize(&entry.namespace),
                normalize(&entry.subject_id)
            ))
            .or_default()
            .push((entry, format!("{}|{}", entry.title, entry.creator)));
    }
    groups
        .into_iter()
        .filter(|(_, values)| {
            distinct_values(values.iter().map(|(_, identity)| identity)).len() > 1
        })
        .map(|(_, values)| {
            let first = &values[0];
            conflict(
                "workref-identity",
                &first.0.namespace,
                &first.0.subject_id,
                "workref",
                format!(
                    "Subject '{}' has conflicting title or creator values.",
                    first.0.subject_id
                ),
                values
                    .iter()
                    .map(|(entry, identity)| conflict_value(entry, identity)),
            )
        })
        .collect()
}

fn alias_subject_conflicts(entries: &[IndexedEntry]) -> Vec<Value> {
    let mut groups = HashMap::<String, Vec<(&IndexedEntry, String)>>::new();
    for entry in entries {
        for alias in &entry.aliases {
            groups
                .entry(format!(
                    "{}:{}",
                    normalize(&entry.namespace),
                    normalize(alias)
                ))
                .or_default()
                .push((entry, entry.subject_id.clone()));
        }
    }
    groups
        .into_iter()
        .filter(|(_, values)| distinct_values(values.iter().map(|(_, subject)| subject)).len() > 1)
        .map(|(_, values)| {
            let first = &values[0];
            conflict(
                "alias-subject",
                &first.0.namespace,
                "",
                &first.0.aliases.first().cloned().unwrap_or_default(),
                format!(
                    "Alias '{}' maps to multiple realm subjects.",
                    first.0.aliases.first().cloned().unwrap_or_default()
                ),
                values
                    .iter()
                    .map(|(entry, subject)| conflict_value(entry, subject)),
            )
        })
        .collect()
}

fn conflict(
    conflict_type: &str,
    namespace: &str,
    subject_id: &str,
    key: &str,
    description: String,
    values: impl Iterator<Item = Value>,
) -> Value {
    serde_json::json!({
        "id": format!("{}:{}:{}:{}", normalize(conflict_type), normalize(namespace), normalize(subject_id), normalize(key)),
        "type": conflict_type,
        "subjectNamespace": namespace,
        "subjectId": subject_id,
        "key": key,
        "description": description,
        "values": values.collect::<Vec<_>>(),
    })
}

fn conflict_value(entry: &IndexedEntry, value: &str) -> Value {
    let authority_key = format!("{}:{}:r{}", entry.realm_id, entry.index_id, entry.revision);
    let provenance = format!(
        "realm:{}:subject-index:{}:r{}",
        entry.realm_id, entry.index_id, entry.revision
    );
    serde_json::json!({
        "realmId": entry.realm_id.clone(),
        "indexId": entry.index_id.clone(),
        "revision": entry.revision,
        "subjectId": entry.subject_id.clone(),
        "value": value,
        "workTitle": entry.title.clone(),
        "workCreator": entry.creator.clone(),
        "aliases": entry.aliases.clone(),
        "externalIds": entry.external_ids.clone(),
        "authorityKey": authority_key,
        "provenance": provenance,
    })
}

fn distinct_values<'a>(values: impl Iterator<Item = &'a String>) -> BTreeSet<String> {
    values.map(|value| normalize(value)).collect()
}

fn index_key(realm_id: &str, index_id: &str) -> String {
    format!("{}:{}", normalize(realm_id), normalize(index_id))
}

fn index_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(normalize)
        .filter(|value| !value.is_empty())
}

fn revision(value: &Value) -> i64 {
    value.get("revision").and_then(Value::as_i64).unwrap_or(0)
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
