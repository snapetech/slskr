use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const STATE_VERSION: u32 = 1;
const MAX_INDEXES: usize = 1_024;
const MAX_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_AUTHORITY_NOTE_CHARS: usize = 512;
pub const DEFAULT_REALM_ID: &str = "default-realm";
pub const DEFAULT_GOVERNANCE_ROOT: &str = "default-governance";

#[derive(Debug, Default, Deserialize, Serialize)]
struct PersistedState {
    version: u32,
    indexes: BTreeMap<String, Value>,
    authority_decisions: BTreeMap<String, Value>,
}

#[derive(Debug)]
pub struct Store {
    state_path: Option<PathBuf>,
    local_realm_id: String,
    trusted_governance_roots: BTreeSet<String>,
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
    #[cfg(any(test, feature = "bounded-differential"))]
    pub fn in_memory() -> Self {
        Self::with_identity(DEFAULT_REALM_ID, [DEFAULT_GOVERNANCE_ROOT])
    }

    fn with_identity<I, R, G>(realm_id: R, governance_roots: I) -> Self
    where
        I: IntoIterator<Item = G>,
        R: AsRef<str>,
        G: AsRef<str>,
    {
        Self {
            state_path: None,
            local_realm_id: realm_id.as_ref().trim().to_owned(),
            trusted_governance_roots: governance_roots
                .into_iter()
                .map(|root| normalize(root.as_ref()))
                .filter(|root| !root.is_empty())
                .collect(),
            indexes: BTreeMap::new(),
            authority_decisions: BTreeMap::new(),
        }
    }

    pub fn load_with_identity<I, R, G>(
        state_dir: &Path,
        realm_id: R,
        governance_roots: I,
    ) -> Result<Self, String>
    where
        I: IntoIterator<Item = G>,
        R: AsRef<str>,
        G: AsRef<str>,
    {
        let state_path = state_dir.join("realm-subject-indexes.json");
        let mut store = Self::with_identity(realm_id, governance_roots);
        store.state_path = Some(state_path.clone());
        if !state_path.exists() {
            return Ok(store);
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
            local_realm_id: store.local_realm_id,
            trusted_governance_roots: store.trusted_governance_roots,
            indexes: std::mem::take(&mut persisted.indexes),
            authority_decisions: std::mem::take(&mut persisted.authority_decisions),
        })
    }

    pub fn is_same_realm(&self, realm_id: &str) -> bool {
        !realm_id.trim().is_empty() && normalize(&self.local_realm_id) == normalize(realm_id)
    }

    pub fn merge_indexes(&mut self, indexes: Vec<Value>) -> Result<usize, String> {
        if indexes.is_empty() || indexes.len() > MAX_INDEXES {
            return Err(format!(
                "realm subject-index merge requires 1 to {MAX_INDEXES} indexes"
            ));
        }
        let previous = self.indexes.clone();
        for index in indexes {
            let key = validate_index(&index, &self.local_realm_id, &self.trusted_governance_roots)?;
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
        let realm_id = realm_id.trim();
        let mut indexes = self
            .indexes
            .values()
            .filter(|index| {
                index_string(index, "realmId")
                    .is_some_and(|value| value.eq_ignore_ascii_case(realm_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        indexes.sort_by(|left, right| {
            normalized_index_string(left, "id")
                .cmp(&normalized_index_string(right, "id"))
                .then_with(|| revision(right).cmp(&revision(left)))
        });
        indexes
    }

    pub fn authority_decisions_for_realm(&self, realm_id: &str) -> Vec<Value> {
        let realm_id = realm_id.trim();
        let mut decisions = self
            .authority_decisions
            .values()
            .filter(|decision| {
                index_string(decision, "realmId")
                    .is_some_and(|value| value.eq_ignore_ascii_case(realm_id))
            })
            .cloned()
            .collect::<Vec<_>>();
        decisions.sort_by(|left, right| {
            normalized_index_string(left, "indexId").cmp(&normalized_index_string(right, "indexId"))
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
        if !self.is_same_realm(realm_id) {
            return Err("Realm id does not match the local realm.".to_owned());
        }
        if index_id.trim().is_empty() {
            return Err("Index id is required.".to_owned());
        }
        if !is_safe_opaque_reference(decided_by) {
            return Err("Decided-by identifier must be opaque and safe.".to_owned());
        }
        if note.chars().count() > MAX_AUTHORITY_NOTE_CHARS {
            return Err("Authority decision note must be 512 characters or fewer.".to_owned());
        }
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
                let matches = recording_ids(entry)
                    .iter()
                    .any(|value| value == &recording_id);
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
            normalized_index_string(left, "realmId")
                .cmp(&normalized_index_string(right, "realmId"))
                .then_with(|| {
                    normalized_index_string(left, "indexId")
                        .cmp(&normalized_index_string(right, "indexId"))
                })
                .then_with(|| {
                    left.get("entry")
                        .and_then(|entry| entry.get("workRef"))
                        .map(|work_ref| normalized_index_string(work_ref, "title"))
                        .cmp(
                            &right
                                .get("entry")
                                .and_then(|entry| entry.get("workRef"))
                                .map(|work_ref| normalized_index_string(work_ref, "title")),
                        )
                })
        });
        resolutions
    }

    pub fn conflict_report(&self, realm_id: &str) -> Value {
        let realm_id = realm_id.trim().to_owned();
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
            normalized_index_string(left, "type")
                .cmp(&normalized_index_string(right, "type"))
                .then_with(|| {
                    normalized_index_string(left, "key").cmp(&normalized_index_string(right, "key"))
                })
                .then_with(|| {
                    normalized_index_string(left, "subjectId")
                        .cmp(&normalized_index_string(right, "subjectId"))
                })
        });
        serde_json::json!({
            "realmId": realm_id,
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "indexCount": enabled_indexes.len(),
            "disabledAuthorityCount": all_indexes.len().saturating_sub(enabled_indexes.len()),
            "entryCount": entries.len(),
            "hasConflicts": !conflicts.is_empty(),
            "conflicts": conflicts,
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
        crate::write_file_atomic(path, &bytes)
            .map_err(|error| format!("realm subject-index state write failed: {error}"))
    }
}

fn validate_index(
    index: &Value,
    local_realm_id: &str,
    trusted_governance_roots: &BTreeSet<String>,
) -> Result<String, String> {
    let Some(object) = index.as_object() else {
        return Err("realm subject-index must be an object".to_owned());
    };
    let mut errors = Vec::new();
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if id.is_none() {
        errors.push("Index id is required.".to_owned());
    }
    let realm_id = object
        .get("realmId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match realm_id {
        None => errors.push("Realm id is required.".to_owned()),
        Some(realm_id) if normalize(local_realm_id) != normalize(realm_id) => {
            errors.push("Index realm does not match the local realm.".to_owned())
        }
        Some(_) => {}
    }
    if object
        .get("subjectNamespace")
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        errors.push("Subject namespace is required.".to_owned());
    }
    if object.get("revision").and_then(Value::as_i64).unwrap_or(0) < 1 {
        errors.push("Revision must be positive.".to_owned());
    }
    let entries = object.get("entries").and_then(Value::as_array);
    if entries.is_none_or(Vec::is_empty) {
        errors.push("Index must contain at least one entry.".to_owned());
    }

    let signature = object.get("signature").and_then(Value::as_object);
    let signer = signature
        .and_then(|signature| signature.get("signer"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if signer.is_empty() {
        errors.push("Signature signer is required.".to_owned());
    } else if !trusted_governance_roots.contains(&normalize(signer)) {
        errors.push("Signature signer is not trusted for this realm.".to_owned());
    }
    let signature_value = signature
        .and_then(|signature| signature.get("value"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if signature_value.is_empty() {
        errors.push("Signature value is required.".to_owned());
    }
    let expected_hash = compute_payload_hash(index);
    let payload_hash = signature
        .and_then(|signature| signature.get("payloadHash"))
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if !payload_hash.eq_ignore_ascii_case(&expected_hash) {
        errors.push("Signature payload hash does not match index contents.".to_owned());
    }

    if let Some(entries) = entries {
        for entry in entries {
            validate_entry(entry, &mut errors);
        }
    }
    if errors.is_empty() {
        Ok(index_key(
            realm_id.expect("validated realm id"),
            id.expect("validated index id"),
        ))
    } else {
        Err(errors.join(" "))
    }
}

fn validate_entry(entry: &Value, errors: &mut Vec<String>) {
    let Some(object) = entry.as_object() else {
        errors.push("Entry must be an object.".to_owned());
        return;
    };
    let subject_id = object
        .get("subjectId")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if subject_id.is_empty() {
        errors.push("Entry subject id is required.".to_owned());
    }
    let work_ref = object.get("workRef").and_then(Value::as_object);
    if work_ref.is_none_or(|work_ref| !work_ref_is_safe(work_ref)) {
        errors.push(format!(
            "Entry '{subject_id}' has an unsafe or incomplete WorkRef."
        ));
    }
    if let Some(evidence_links) = object.get("evidenceLinks") {
        if let Some(evidence_links) = evidence_links.as_array() {
            for evidence_link in evidence_links {
                let valid = evidence_link.as_str().is_some_and(is_safe_evidence_link);
                if !valid {
                    errors.push(format!("Entry '{subject_id}' has an unsafe evidence link."));
                }
            }
        } else {
            errors.push(format!("Entry '{subject_id}' has an unsafe evidence link."));
        }
    }
}

pub fn compute_payload_hash(index: &Value) -> String {
    let mut canonical = String::new();
    let realm_id = index_string(index, "realmId").unwrap_or_default();
    let namespace = index_string(index, "subjectNamespace").unwrap_or_default();
    canonical.push_str(&normalize(&realm_id));
    canonical.push('|');
    canonical.push_str(&normalize(&namespace));
    canonical.push('|');
    canonical.push_str(&revision(index).to_string());
    canonical.push('|');

    let mut entries = index
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries.sort_by_key(|entry| {
        normalize(
            index_string(entry, "subjectId")
                .as_deref()
                .unwrap_or_default(),
        )
    });
    for entry in entries {
        let work_ref = entry.get("workRef").unwrap_or(&Value::Null);
        canonical.push_str(&normalize(
            index_string(&entry, "subjectId")
                .as_deref()
                .unwrap_or_default(),
        ));
        canonical.push(':');
        canonical.push_str(&normalize(
            index_string(work_ref, "domain")
                .as_deref()
                .unwrap_or_default(),
        ));
        canonical.push(':');
        canonical.push_str(&normalize(
            index_string(work_ref, "title")
                .as_deref()
                .unwrap_or_default(),
        ));
        canonical.push(':');
        canonical.push_str(&normalize(
            index_string(work_ref, "creator")
                .as_deref()
                .unwrap_or_default(),
        ));
        canonical.push(':');

        let mut external_ids = entry
            .get("externalIds")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(key, value)| Some((normalize(key), normalize(value.as_str()?))))
            .collect::<Vec<_>>();
        external_ids.sort();
        canonical.push_str(
            &external_ids
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join(","),
        );
        canonical.push(':');

        let mut aliases = entry
            .get("aliases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(normalize)
            .collect::<Vec<_>>();
        aliases.sort();
        canonical.push_str(&aliases.join(","));
        canonical.push(':');

        let mut evidence_links = entry
            .get("evidenceLinks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(normalize)
            .collect::<Vec<_>>();
        evidence_links.sort();
        canonical.push_str(&evidence_links.join(","));
        canonical.push('|');
    }

    hex::encode(Sha256::digest(canonical.as_bytes()))
}

fn work_ref_is_safe(work_ref: &serde_json::Map<String, Value>) -> bool {
    let domain = work_ref
        .get("domain")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let title = work_ref
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if domain.is_empty() || title.is_empty() || contains_sensitive_pattern(title, false) {
        return false;
    }
    if work_ref
        .get("creator")
        .and_then(Value::as_str)
        .is_some_and(|creator| contains_sensitive_pattern(creator, false))
    {
        return false;
    }
    if let Some(external_ids) = work_ref.get("externalIds").and_then(Value::as_object) {
        for (key, value) in external_ids {
            let Some(value) = value.as_str() else {
                return false;
            };
            if contains_sensitive_pattern(key, false)
                || contains_sensitive_pattern(
                    value,
                    matches!(
                        normalize(key).as_str(),
                        "musicbrainz" | "musicbrainz_artist" | "musicbrainzartist" | "discogs"
                    ),
                )
            {
                return false;
            }
        }
    }
    if let Some(metadata) = work_ref.get("metadata").and_then(Value::as_object) {
        for (key, value) in metadata {
            let rendered_value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            if contains_sensitive_pattern(key, false)
                || contains_sensitive_pattern(&rendered_value, false)
            {
                return false;
            }
        }
    }
    true
}

fn contains_sensitive_pattern(value: &str, allow_uuid: bool) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    let lowered = value.to_ascii_lowercase();
    if value.starts_with('/')
        || value.starts_with("..")
        || value.contains(['/', '\\'])
        || lowered.starts_with("pod:")
        || lowered.starts_with("bridge:")
        || lowered.contains("localhost")
        || lowered.contains("127.0.0.1")
        || lowered.contains("192.168.")
        || lowered.contains("10.")
        || contains_ipv4_literal(value)
        || (16..=31).any(|octet| lowered.contains(&format!("172.{octet}.")))
        || (value.len() >= 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return true;
    }
    if !allow_uuid && is_uuid(value) {
        return true;
    }
    lowered
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| {
            matches!(
                word,
                "hash" | "path" | "file" | "local" | "private" | "internal"
            )
        })
}

fn is_uuid(value: &str) -> bool {
    let groups = [8, 4, 4, 4, 12];
    let mut parts = value.split('-');
    groups.iter().all(|length| {
        parts.next().is_some_and(|part| {
            part.len() == *length && part.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    }) && parts.next().is_none()
}

fn is_safe_evidence_link(value: &str) -> bool {
    if value.trim().is_empty() || value.contains('\\') {
        return false;
    }
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    matches!(
        url.scheme().to_ascii_lowercase().as_str(),
        "http" | "https" | "mbid" | "workref" | "songid"
    ) && url.username().is_empty()
}

pub fn is_safe_opaque_reference(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 128
        && !value.contains(['/', '\\', ':'])
        && !value.contains("..")
}

fn indexed_entry(index: &Value, entry: &Value) -> Option<IndexedEntry> {
    let realm_id = index_string(index, "realmId")?;
    let index_id = index_string(index, "id")?;
    let subject_id = index_string(entry, "subjectId")?;
    let work_ref = entry.get("workRef").unwrap_or(&Value::Null);
    let mut external_ids = BTreeMap::new();
    if let Some(work_ids) = work_ref.get("externalIds").and_then(Value::as_object) {
        for (key, value) in work_ids {
            if let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                insert_external_id(&mut external_ids, key, value);
            }
        }
    }
    if let Some(entry_ids) = entry.get("externalIds").and_then(Value::as_object) {
        for (key, value) in entry_ids {
            if let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                insert_external_id(&mut external_ids, key, value);
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

fn recording_ids(entry: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    for external_ids in [
        entry
            .get("workRef")
            .and_then(|work_ref| work_ref.get("externalIds")),
        entry.get("externalIds"),
    ] {
        let Some(external_ids) = external_ids.and_then(Value::as_object) else {
            continue;
        };
        for (key, value) in external_ids {
            if matches!(
                normalize(key).as_str(),
                "musicbrainz" | "musicbrainz:recording"
            ) {
                if let Some(value) = value
                    .as_str()
                    .map(normalize)
                    .filter(|value| !value.is_empty())
                {
                    ids.push(value);
                }
            }
        }
    }
    ids
}

fn insert_external_id(ids: &mut BTreeMap<String, String>, key: &str, value: &str) {
    if let Some(existing_key) = ids
        .keys()
        .find(|existing| existing.eq_ignore_ascii_case(key.trim()))
        .cloned()
    {
        ids.insert(existing_key, value.to_owned());
    } else {
        ids.insert(key.trim().to_owned(), value.to_owned());
    }
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
            if matches!(
                normalize(key).as_str(),
                "musicbrainz" | "musicbrainz:recording"
            ) {
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
    let mut groups = HashMap::<String, Vec<(&IndexedEntry, String, String)>>::new();
    for entry in entries {
        for alias in &entry.aliases {
            groups
                .entry(format!(
                    "{}:{}",
                    normalize(&entry.namespace),
                    normalize(alias)
                ))
                .or_default()
                .push((entry, alias.clone(), entry.subject_id.clone()));
        }
    }
    groups
        .into_iter()
        .filter(|(_, values)| {
            distinct_values(values.iter().map(|(_, _, subject)| subject)).len() > 1
        })
        .map(|(_, values)| {
            let first = &values[0];
            conflict(
                "alias-subject",
                &first.0.namespace,
                "",
                &first.1,
                format!("Alias '{}' maps to multiple realm subjects.", first.1),
                values
                    .iter()
                    .map(|(entry, _, subject)| conflict_value(entry, subject)),
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
    let mut values = values.collect::<Vec<_>>();
    values.sort_by(|left, right| {
        normalize_json_string(left, "authorityKey")
            .cmp(&normalize_json_string(right, "authorityKey"))
            .then_with(|| {
                normalize_json_string(left, "subjectId")
                    .cmp(&normalize_json_string(right, "subjectId"))
            })
            .then_with(|| {
                normalize_json_string(left, "value").cmp(&normalize_json_string(right, "value"))
            })
    });
    let mut seen = BTreeSet::new();
    values.retain(|value| {
        seen.insert(format!(
            "{}:{}:{}",
            normalize_json_string(value, "authorityKey"),
            normalize_json_string(value, "subjectId"),
            normalize_json_string(value, "value")
        ))
    });
    serde_json::json!({
        "id": format!("{}:{}:{}:{}", normalize(conflict_type), normalize(namespace), normalize(subject_id), normalize(key)),
        "type": conflict_type,
        "subjectNamespace": namespace,
        "subjectId": subject_id,
        "key": key,
        "description": description,
        "values": values,
    })
}

fn normalize_json_string(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(normalize)
        .unwrap_or_default()
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
        .map(str::trim)
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}

fn revision(value: &Value) -> i64 {
    value.get("revision").and_then(Value::as_i64).unwrap_or(0)
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalized_index_string(value: &Value, field: &str) -> String {
    index_string(value, field)
        .map(|value| normalize(&value))
        .unwrap_or_default()
}

fn contains_ipv4_literal(value: &str) -> bool {
    value
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .any(|candidate| {
            let octets = candidate.split('.').collect::<Vec<_>>();
            octets.len() == 4
                && octets.iter().all(|octet| {
                    !octet.is_empty()
                        && octet.len() <= 3
                        && octet.bytes().all(|byte| byte.is_ascii_digit())
                        && octet.parse::<u16>().is_ok_and(|value| value <= 255)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_index(id: &str) -> Value {
        let mut index = serde_json::json!({
            "id": id,
            "realmId": "realm-a",
            "subjectNamespace": "music",
            "revision": 1,
            "entries": [{
                "subjectId": "subject-a",
                "workRef": {
                    "domain": "music",
                    "title": "Public Title",
                    "creator": "Public Artist",
                },
                "externalIds": {
                    "musicbrainz:recording": "entry-recording"
                },
                "aliases": ["Public Alias"],
                "evidenceLinks": ["https://example.test/evidence"]
            }],
            "signature": {
                "signer": "governance-a",
                "value": "signature",
                "payloadHash": ""
            }
        });
        let payload_hash = compute_payload_hash(&index);
        index["signature"]["payloadHash"] = serde_json::json!(payload_hash);
        index
    }

    #[test]
    fn merge_requires_local_realm_trusted_signature_and_exact_payload() {
        let mut store = Store::with_identity("realm-a", ["governance-a"]);
        let mut wrong_realm = valid_index("wrong-realm");
        wrong_realm["realmId"] = serde_json::json!("realm-b");
        wrong_realm["signature"]["payloadHash"] =
            serde_json::json!(compute_payload_hash(&wrong_realm));
        let error = store.merge_indexes(vec![wrong_realm]).unwrap_err();
        assert!(error.contains("local realm"), "{error}");

        let mut untrusted = valid_index("untrusted");
        untrusted["signature"]["signer"] = serde_json::json!("governance-b");
        let error = store.merge_indexes(vec![untrusted]).unwrap_err();
        assert!(error.contains("not trusted"), "{error}");

        let mut bad_hash = valid_index("bad-hash");
        bad_hash["signature"]["payloadHash"] = serde_json::json!("not-the-payload-hash");
        let error = store.merge_indexes(vec![bad_hash]).unwrap_err();
        assert!(error.contains("payload hash"), "{error}");
        assert!(store.indexes_for_realm("realm-a").is_empty());
    }

    #[test]
    fn merge_rejects_unsafe_work_refs_and_evidence_links() {
        let mut unsafe_work_ref = valid_index("unsafe-work-ref");
        unsafe_work_ref["entries"][0]["workRef"]["title"] = serde_json::json!("/private/file.flac");
        unsafe_work_ref["signature"]["payloadHash"] =
            serde_json::json!(compute_payload_hash(&unsafe_work_ref));
        let mut store = Store::with_identity("realm-a", ["governance-a"]);
        let error = store.merge_indexes(vec![unsafe_work_ref]).unwrap_err();
        assert!(error.contains("unsafe or incomplete WorkRef"), "{error}");

        let mut unsafe_evidence = valid_index("unsafe-evidence");
        unsafe_evidence["entries"][0]["evidenceLinks"] =
            serde_json::json!(["file:///tmp/evidence"]);
        unsafe_evidence["signature"]["payloadHash"] =
            serde_json::json!(compute_payload_hash(&unsafe_evidence));
        let error = store.merge_indexes(vec![unsafe_evidence]).unwrap_err();
        assert!(error.contains("unsafe evidence link"), "{error}");
    }

    #[test]
    fn recording_resolution_keeps_work_ref_and_entry_ids() {
        let mut index = valid_index("recording-ids");
        index["entries"][0]["workRef"]["externalIds"] =
            serde_json::json!({"musicbrainz:recording": "workref-recording"});
        index["signature"]["payloadHash"] = serde_json::json!(compute_payload_hash(&index));
        let mut store = Store::with_identity("realm-a", ["governance-a"]);
        store.merge_indexes(vec![index]).unwrap();

        assert_eq!(store.resolve_recording("WORKREF-RECORDING").len(), 1);
        assert_eq!(store.resolve_recording("entry-recording").len(), 1);
    }

    #[test]
    fn payload_hash_is_stable_across_entry_order_and_case() {
        let first = valid_index("hash-stability");
        let mut second = first.clone();
        second["entries"][0]["subjectId"] = serde_json::json!("SUBJECT-A");
        second["entries"][0]["aliases"] = serde_json::json!(["public alias"]);
        second["entries"][0]["evidenceLinks"] =
            serde_json::json!(["HTTPS://EXAMPLE.TEST/EVIDENCE"]);
        assert_eq!(compute_payload_hash(&first), compute_payload_hash(&second));
    }
}
