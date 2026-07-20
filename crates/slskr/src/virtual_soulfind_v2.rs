use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MAX_TRACK_INTENTS: usize = 1_024;
const MAX_RELEASE_INTENTS: usize = 512;
const MAX_EXECUTIONS: usize = 1_024;
const MAX_TEXT_BYTES: usize = 512;

#[derive(Clone, Debug)]
pub struct CatalogueItem {
    pub source_id: String,
    pub artist: String,
    pub title: String,
    pub kind: String,
    pub created_at: u64,
}

#[derive(Clone, Debug)]
struct TrackIntent {
    domain: String,
    desired_track_id: String,
    track_id: String,
    parent_desired_release_id: Option<String>,
    priority: String,
    status: String,
    planned_sources: Option<String>,
    created_at: String,
    updated_at: String,
}

impl TrackIntent {
    fn json(&self) -> Value {
        let mut value = json!({
            "domain": self.domain,
            "desiredTrackId": self.desired_track_id,
            "trackId": self.track_id,
            "priority": self.priority,
            "status": self.status,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        });
        let object = value.as_object_mut().expect("track intent is an object");
        if let Some(parent) = &self.parent_desired_release_id {
            object.insert("parentDesiredReleaseId".to_owned(), json!(parent));
        }
        if let Some(sources) = &self.planned_sources {
            object.insert("plannedSources".to_owned(), json!(sources));
        }
        value
    }
}

#[derive(Clone, Debug)]
struct ReleaseIntent {
    desired_release_id: String,
    release_id: String,
    priority: String,
    mode: String,
    status: String,
    created_at: String,
    updated_at: String,
    notes: Option<String>,
}

impl ReleaseIntent {
    fn json(&self) -> Value {
        let mut value = json!({
            "desiredReleaseId": self.desired_release_id,
            "releaseId": self.release_id,
            "priority": self.priority,
            "mode": self.mode,
            "status": self.status,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        });
        if let Some(notes) = &self.notes {
            value
                .as_object_mut()
                .expect("release intent is an object")
                .insert("notes".to_owned(), json!(notes));
        }
        value
    }
}

#[derive(Debug, Default)]
pub struct State {
    track_intents: BTreeMap<String, TrackIntent>,
    release_intents: BTreeMap<String, ReleaseIntent>,
    executions: BTreeMap<String, Value>,
    total_processed: u64,
    success_count: u64,
    failure_count: u64,
}

impl State {
    pub fn enqueue_track(
        &mut self,
        domain: &Value,
        track_id: &str,
        priority: &Value,
        parent_release_id: Option<&str>,
    ) -> Result<Value, String> {
        if self.track_intents.len() >= MAX_TRACK_INTENTS {
            return Err("track intent capacity is full".to_owned());
        }
        let domain = parse_enum(domain, &["Music", "GenericFile"], "domain")?;
        if domain != "Music" {
            return Err("FileHash is required for GenericFile domain".to_owned());
        }
        let track_id = bounded_required(track_id, MAX_TEXT_BYTES, "TrackId")?;
        uuid::Uuid::parse_str(&track_id)
            .map_err(|_| "TrackId must be a valid UUID format for Music domain".to_owned())?;
        let priority = parse_enum(priority, &["Low", "Normal", "High", "Urgent"], "priority")?;
        let parent_desired_release_id = parent_release_id
            .map(|value| bounded_required(value, MAX_TEXT_BYTES, "ParentDesiredReleaseId"))
            .transpose()?;
        let now = timestamp_now();
        let intent = TrackIntent {
            domain,
            desired_track_id: uuid::Uuid::new_v4().to_string(),
            track_id,
            parent_desired_release_id,
            priority,
            status: "Pending".to_owned(),
            planned_sources: None,
            created_at: now.clone(),
            updated_at: now,
        };
        let response = intent.json();
        self.track_intents
            .insert(intent.desired_track_id.clone(), intent);
        Ok(response)
    }

    pub fn enqueue_release(
        &mut self,
        release_id: &str,
        priority: &Value,
        mode: &Value,
        notes: Option<&str>,
    ) -> Result<Value, String> {
        if self.release_intents.len() >= MAX_RELEASE_INTENTS {
            return Err("release intent capacity is full".to_owned());
        }
        let release_id = bounded_required(release_id, MAX_TEXT_BYTES, "ReleaseId")?;
        let priority = parse_enum(priority, &["Low", "Normal", "High", "Urgent"], "priority")?;
        let mode = parse_enum(mode, &["Wanted", "NiceToHave", "Backfill"], "mode")?;
        let notes = notes
            .map(|value| bounded_optional(value, MAX_TEXT_BYTES, "Notes"))
            .transpose()?
            .flatten();
        let now = timestamp_now();
        let intent = ReleaseIntent {
            desired_release_id: uuid::Uuid::new_v4().to_string(),
            release_id,
            priority,
            mode,
            status: "Pending".to_owned(),
            created_at: now.clone(),
            updated_at: now,
            notes,
        };
        let response = intent.json();
        self.release_intents
            .insert(intent.desired_release_id.clone(), intent);
        Ok(response)
    }

    pub fn pending_tracks(&self, limit: usize) -> Value {
        let mut intents = self
            .track_intents
            .values()
            .filter(|intent| intent.status == "Pending")
            .collect::<Vec<_>>();
        intents.sort_by(|left, right| {
            priority_rank(&right.priority)
                .cmp(&priority_rank(&left.priority))
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.desired_track_id.cmp(&right.desired_track_id))
        });
        Value::Array(
            intents
                .into_iter()
                .take(limit.min(MAX_TRACK_INTENTS))
                .map(TrackIntent::json)
                .collect(),
        )
    }

    pub fn track(&self, id: &str) -> Option<Value> {
        self.track_intents.get(id).map(TrackIntent::json)
    }

    pub fn release(&self, id: &str) -> Option<Value> {
        self.release_intents.get(id).map(ReleaseIntent::json)
    }

    pub fn update_track_status(&mut self, id: &str, status: &Value) -> Result<bool, String> {
        let status = parse_enum(
            status,
            &[
                "Pending",
                "Planned",
                "InProgress",
                "Completed",
                "Failed",
                "OnHold",
                "Cancelled",
            ],
            "status",
        )?;
        let Some(intent) = self.track_intents.get_mut(id) else {
            return Ok(false);
        };
        intent.status = status;
        intent.updated_at = timestamp_now();
        Ok(true)
    }

    pub fn stats(&self) -> Value {
        json!({
            "totalProcessed": self.total_processed,
            "successCount": self.success_count,
            "failureCount": self.failure_count,
            "inProgressCount": self.track_intents.values().filter(|intent| intent.status == "InProgress").count(),
            "pendingCount": self.track_intents.values().filter(|intent| intent.status == "Pending").count(),
        })
    }

    pub fn process_track(&mut self, id: &str, catalogue: &[CatalogueItem]) -> bool {
        let Some(intent) = self.track_intents.get_mut(id) else {
            return false;
        };
        if intent.status != "Pending" {
            return false;
        }
        intent.status = "InProgress".to_owned();
        intent.updated_at = timestamp_now();
        let track_exists = catalogue
            .iter()
            .any(|item| track_id(item) == intent.track_id);
        let now = timestamp_now();
        intent.status = if track_exists { "Completed" } else { "Failed" }.to_owned();
        intent.planned_sources = track_exists.then(|| "[\"LocalLibrary\"]".to_owned());
        intent.updated_at = now.clone();
        self.total_processed = self.total_processed.saturating_add(1);
        if track_exists {
            self.success_count = self.success_count.saturating_add(1);
        } else {
            self.failure_count = self.failure_count.saturating_add(1);
        }
        if self.executions.len() == MAX_EXECUTIONS {
            if let Some(oldest) = self.executions.keys().next().cloned() {
                self.executions.remove(&oldest);
            }
        }
        let execution_id = uuid::Uuid::new_v4().to_string();
        let mut execution = json!({
            "executionId": execution_id,
            "trackId": intent.track_id,
            "status": if track_exists { "Succeeded" } else { "Failed" },
            "currentStepIndex": 0,
            "totalSteps": if track_exists { 1 } else { 0 },
            "startedAt": now,
            "completedAt": timestamp_now(),
        });
        if !track_exists {
            execution
                .as_object_mut()
                .expect("execution is an object")
                .insert(
                    "errorMessage".to_owned(),
                    json!("No viable plan created for track"),
                );
        }
        self.executions.insert(execution_id, execution);
        true
    }

    pub fn execution(&self, id: &str) -> Option<Value> {
        self.executions.get(id).cloned()
    }
}

pub fn search_artists(catalogue: &[CatalogueItem], query: &str, limit: usize) -> Value {
    let query = query.to_ascii_lowercase();
    let mut seen = BTreeSet::new();
    Value::Array(
        catalogue
            .iter()
            .filter(|item| item.artist.to_ascii_lowercase().contains(&query))
            .filter(|item| seen.insert(item.artist.to_ascii_lowercase()))
            .take(limit.min(100))
            .map(artist_json)
            .collect(),
    )
}

pub fn artist(catalogue: &[CatalogueItem], id: &str) -> Option<Value> {
    catalogue
        .iter()
        .find(|item| artist_id(&item.artist) == id)
        .map(artist_json)
}

pub fn artist_releases(catalogue: &[CatalogueItem], id: &str, limit: usize) -> Value {
    Value::Array(
        catalogue
            .iter()
            .filter(|item| artist_id(&item.artist) == id)
            .take(limit.min(100))
            .map(|item| {
                json!({
                    "releaseGroupId": release_id(item),
                    "artistId": id,
                    "title": item.title,
                    "primaryType": release_type(&item.kind),
                    "createdAt": timestamp(item.created_at),
                    "updatedAt": timestamp(item.created_at),
                })
            })
            .collect(),
    )
}

pub fn release_tracks(catalogue: &[CatalogueItem], id: &str) -> Value {
    Value::Array(
        catalogue
            .iter()
            .filter(|item| release_id(item) == id)
            .enumerate()
            .map(|(index, item)| {
                json!({
                    "trackId": track_id(item),
                    "releaseId": id,
                    "discNumber": 1,
                    "trackNumber": index.saturating_add(1),
                    "title": item.title,
                    "tags": item.kind,
                    "createdAt": timestamp(item.created_at),
                    "updatedAt": timestamp(item.created_at),
                })
            })
            .collect(),
    )
}

pub fn create_plan(
    catalogue: &[CatalogueItem],
    domain: &Value,
    track_id_value: &str,
    mode: &Value,
    priority: &Value,
) -> Result<Value, String> {
    let domain = parse_enum(domain, &["Music", "GenericFile"], "domain")?;
    if domain != "Music" {
        return Err("FileHash is required for GenericFile domain".to_owned());
    }
    let track_id_value = bounded_required(track_id_value, MAX_TEXT_BYTES, "TrackId")?;
    uuid::Uuid::parse_str(&track_id_value)
        .map_err(|_| "TrackId must be a valid UUID format for Music domain".to_owned())?;
    let mode = parse_enum(
        mode,
        &["OfflinePlanning", "MeshOnly", "SoulseekFriendly"],
        "mode",
    )?;
    let priority = parse_enum(priority, &["Low", "Normal", "High", "Urgent"], "priority")?;
    let now = timestamp_now();
    let desired_track = json!({
        "domain": domain,
        "desiredTrackId": uuid::Uuid::new_v4().to_string(),
        "trackId": track_id_value,
        "priority": priority,
        "status": "Pending",
        "createdAt": now,
        "updatedAt": now,
    });
    let local = catalogue
        .iter()
        .find(|item| track_id(item) == track_id_value);
    let steps = local.map_or_else(Vec::new, |item| {
        vec![json!({
            "backend": "LocalLibrary",
            "candidates": [{
                "id": stable_uuid(&format!("candidate|{}", item.source_id)),
                "itemId": { "value": track_id_value },
                "backend": "LocalLibrary",
                "backendRef": item.source_id,
                "expectedQuality": 1.0,
                "trustScore": 1.0,
                "isPreferred": true,
                "isFromPrivateSource": false,
            }],
            "maxParallel": 1,
            "timeout": "00:00:30",
            "fallbackMode": "Cascade",
        })]
    });
    Ok(json!({
        "trackId": track_id_value,
        "desiredTrack": desired_track,
        "mode": mode,
        "status": if steps.is_empty() { "Planning" } else { "Ready" },
        "steps": steps,
        "createdAt": now,
        "isExecutable": !steps.is_empty(),
    }))
}

fn artist_json(item: &CatalogueItem) -> Value {
    json!({
        "artistId": artist_id(&item.artist),
        "name": item.artist,
        "sortName": item.artist,
        "createdAt": timestamp(item.created_at),
        "updatedAt": timestamp(item.created_at),
    })
}

fn artist_id(artist: &str) -> String {
    stable_uuid(&format!("artist|{}", artist.trim().to_ascii_lowercase()))
}

fn release_id(item: &CatalogueItem) -> String {
    stable_uuid(&format!("release|{}", item.source_id))
}

fn track_id(item: &CatalogueItem) -> String {
    stable_uuid(&format!("track|{}", item.source_id))
}

fn stable_uuid(value: &str) -> String {
    let mut bytes: [u8; 16] = Sha256::digest(value.as_bytes())[..16]
        .try_into()
        .expect("SHA-256 prefix is sixteen bytes");
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

fn release_type(kind: &str) -> &'static str {
    match kind.trim().to_ascii_lowercase().as_str() {
        "album" => "Album",
        "ep" => "EP",
        "single" => "Single",
        "compilation" => "Compilation",
        "live" => "Live",
        "soundtrack" => "Soundtrack",
        _ => "Other",
    }
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "Urgent" => 3,
        "High" => 2,
        "Normal" => 1,
        _ => 0,
    }
}

fn parse_enum(value: &Value, variants: &[&str], field: &str) -> Result<String, String> {
    if let Some(index) = value.as_u64() {
        return variants
            .get(usize::try_from(index).unwrap_or(usize::MAX))
            .map(|variant| (*variant).to_owned())
            .ok_or_else(|| format!("invalid {field}"));
    }
    let text = value.as_str().ok_or_else(|| format!("invalid {field}"))?;
    variants
        .iter()
        .find(|variant| variant.eq_ignore_ascii_case(text.trim()))
        .map(|variant| (*variant).to_owned())
        .ok_or_else(|| format!("invalid {field}"))
}

fn bounded_required(value: &str, max: usize, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field} is required"));
    }
    if value.len() > max || value.chars().any(char::is_control) {
        return Err(format!("{field} is invalid"));
    }
    Ok(value.to_owned())
}

fn bounded_optional(value: &str, max: usize, field: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    bounded_required(value, max, field).map(Some)
}

fn timestamp(seconds: u64) -> String {
    DateTime::from_timestamp(i64::try_from(seconds).unwrap_or(i64::MAX), 0)
        .unwrap_or(DateTime::UNIX_EPOCH)
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn timestamp_now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue() -> Vec<CatalogueItem> {
        vec![CatalogueItem {
            source_id: "lib-1".to_owned(),
            artist: "Artist".to_owned(),
            title: "Track".to_owned(),
            kind: "Album".to_owned(),
            created_at: 1_700_000_000,
        }]
    }

    #[test]
    fn catalogue_ids_are_stable_and_support_plan_creation() {
        let catalogue = catalogue();
        let artists = search_artists(&catalogue, "artist", 10);
        let artist_id = artists[0]["artistId"].as_str().unwrap();
        let releases = artist_releases(&catalogue, artist_id, 10);
        let release_id = releases[0]["releaseGroupId"].as_str().unwrap();
        let tracks = release_tracks(&catalogue, release_id);
        let track_id = tracks[0]["trackId"].as_str().unwrap();
        let plan = create_plan(
            &catalogue,
            &json!("Music"),
            track_id,
            &json!("SoulseekFriendly"),
            &json!("Normal"),
        )
        .unwrap();
        assert_eq!(plan["status"], "Ready");
        assert_eq!(plan["steps"][0]["backend"], "LocalLibrary");
    }

    #[test]
    fn processing_claims_pending_intent_once() {
        let catalogue = catalogue();
        let track_id = track_id(&catalogue[0]);
        let mut state = State::default();
        let intent = state
            .enqueue_track(&json!("Music"), &track_id, &json!("High"), None)
            .unwrap();
        let id = intent["desiredTrackId"].as_str().unwrap();
        assert!(state.process_track(id, &catalogue));
        assert!(!state.process_track(id, &catalogue));
        assert_eq!(state.track(id).unwrap()["status"], "Completed");
        assert_eq!(state.stats()["totalProcessed"], 1);
    }

    #[test]
    fn generic_file_matches_current_controller_validation_failure() {
        let mut state = State::default();
        let error = state
            .enqueue_track(&json!("GenericFile"), "hash", &json!("Normal"), None)
            .unwrap_err();
        assert_eq!(error, "FileHash is required for GenericFile domain");
    }
}
