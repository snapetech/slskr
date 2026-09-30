use super::{
    json_escape, query_bounded_usize, query_params, truncate_utf8_bytes, unix_seconds_rfc3339,
    unix_timestamp, AppState, MAX_LIBRARY_HEALTH_SCANS, MAX_LIBRARY_ITEMS,
    MAX_LIBRARY_REMEDIATION_JOBS, MAX_LIST_ARTIST_BYTES, MAX_LIST_KIND_BYTES, MAX_LIST_TITLE_BYTES,
};
use std::collections::BTreeMap;

// Library Item Models
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryItemRecord {
    pub(crate) id: String,
    pub(crate) artist: String,
    pub(crate) title: String,
    pub(crate) kind: String,
    pub(crate) created_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryHealthScanRecord {
    pub(crate) id: String,
    pub(crate) library_path: String,
    pub(crate) items: usize,
    pub(crate) issues: Vec<serde_json::Value>,
    pub(crate) status: String,
    pub(crate) started_at: u64,
    pub(crate) updated_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryRemediationJob {
    pub(crate) id: String,
    pub(crate) issue_ids: Vec<String>,
    pub(crate) status: String,
    pub(crate) fixed_count: usize,
    pub(crate) error: Option<String>,
    pub(crate) created_at: u64,
    pub(crate) updated_at: u64,
}

impl LibraryRemediationJob {
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "kind": "library_remediation",
            "issueIds": self.issue_ids,
            "status": self.status,
            "fixedCount": self.fixed_count,
            "error": self.error,
            "createdAt": self.created_at,
            "updatedAt": self.updated_at,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LibraryHealthIssueQuery {
    pub(crate) library_path: String,
    pub(crate) types: Vec<String>,
    pub(crate) severities: Vec<String>,
    pub(crate) statuses: Vec<String>,
    pub(crate) musicbrainz_release_id: String,
    pub(crate) limit: usize,
    pub(crate) offset: usize,
}

impl LibraryHealthIssueQuery {
    pub(crate) fn from_query(query: Option<&str>) -> Result<Self, String> {
        let params = query_params(query.unwrap_or_default());
        let values = |name: &str| {
            params
                .iter()
                .filter(|(key, _)| key.eq_ignore_ascii_case(name))
                .flat_map(|(_, value)| value.split(','))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let canonicalize = |name: &str, allowed: &[&str]| -> Result<Vec<String>, String> {
            values(name)
                .into_iter()
                .map(|value| {
                    allowed
                        .iter()
                        .find(|candidate| candidate.eq_ignore_ascii_case(&value))
                        .map(|candidate| (*candidate).to_owned())
                        .ok_or_else(|| format!("invalid {name} value: {value}"))
                })
                .collect()
        };
        let limit = query_bounded_usize(query, "limit", 1, 250)
            .map_err(|()| "limit must be between 1 and 250".to_owned())?
            .unwrap_or(100);
        let offset = query_bounded_usize(query, "offset", 0, usize::MAX)
            .map_err(|()| "offset must be non-negative".to_owned())?
            .unwrap_or(0);
        Ok(Self {
            library_path: values("libraryPath").into_iter().next().unwrap_or_default(),
            types: canonicalize(
                "types",
                &[
                    "SuspectedTranscode",
                    "NonCanonicalVariant",
                    "TrackNotInTaggedRelease",
                    "MissingTrackInRelease",
                    "CorruptedFile",
                    "MissingMetadata",
                    "MultipleVariants",
                    "WrongDuration",
                ],
            )?,
            severities: canonicalize("severities", &["Info", "Low", "Medium", "High", "Critical"])?,
            statuses: canonicalize(
                "statuses",
                &[
                    "Detected",
                    "Acknowledged",
                    "Ignored",
                    "Fixing",
                    "Resolved",
                    "Failed",
                ],
            )?,
            musicbrainz_release_id: values("musicBrainzReleaseId")
                .into_iter()
                .next()
                .unwrap_or_default(),
            limit,
            offset,
        })
    }
}

impl LibraryHealthScanRecord {
    pub(crate) fn json(&self) -> String {
        serde_json::json!({
            "id": self.id,
            "status": self.status,
            "libraryPath": self.library_path,
            "items": self.items,
            "issues_found": self.issues.len(),
            "issues": self.issues,
            "updated_at": self.updated_at,
            "startedAt": self.started_at,
        })
        .to_string()
    }
}

impl LibraryItemRecord {
    pub(crate) fn json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"artist\":\"{}\",\"title\":\"{}\",\"kind\":\"{}\",\"created_at\":{}}}",
            json_escape(&self.id),
            json_escape(&self.artist),
            json_escape(&self.title),
            json_escape(&self.kind),
            self.created_at
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryStore {
    pub(crate) records: Vec<LibraryItemRecord>,
    pub(crate) next_id: u64,
    pub(crate) health_scans: Vec<LibraryHealthScanRecord>,
    pub(crate) next_health_scan_id: u64,
    pub(crate) updated_at: u64,
    pub(crate) remediation_jobs: Vec<LibraryRemediationJob>,
    pub(crate) next_remediation_job_id: u64,
}

impl LibraryStore {
    pub(crate) fn new() -> Self {
        Self {
            records: Vec::new(),
            next_id: 1,
            health_scans: Vec::new(),
            next_health_scan_id: 1,
            updated_at: unix_timestamp(),
            remediation_jobs: Vec::new(),
            next_remediation_job_id: 1,
        }
    }

    pub(crate) fn from_persisted(records: Vec<crate::persistence::LibraryItemRecord>) -> Self {
        let mut next_id = 1;
        let mut updated_at = unix_timestamp();
        for record in &records {
            if let Some(number) = record
                .id
                .strip_prefix("lib-")
                .and_then(|value| value.parse::<u64>().ok())
            {
                next_id = next_id.max(number.saturating_add(1));
            }
        }
        let mut seen_ids = std::collections::HashSet::new();
        let records = records
            .into_iter()
            .filter(|record| seen_ids.insert(record.id.clone()))
            .take(MAX_LIBRARY_ITEMS)
            .map(|record| {
                let created_at = u64::try_from(record.created_at).unwrap_or(0);
                updated_at = updated_at.max(created_at);
                LibraryItemRecord {
                    id: record.id,
                    artist: record.artist,
                    title: record.title,
                    kind: record.kind,
                    created_at,
                }
            })
            .collect();
        Self {
            records,
            next_id,
            health_scans: Vec::new(),
            next_health_scan_id: 1,
            updated_at,
            remediation_jobs: Vec::new(),
            next_remediation_job_id: 1,
        }
    }

    pub(crate) fn create_health_scan(
        &mut self,
        library_path: String,
    ) -> Option<LibraryHealthScanRecord> {
        let id = format!("scan-{}", self.allocate_health_scan_id());
        let record = LibraryHealthScanRecord {
            id,
            library_path,
            items: self.records.len(),
            issues: self.health_issues(),
            status: "completed".to_owned(),
            started_at: self.updated_at,
            updated_at: self.updated_at,
        };
        if self.health_scans.len() == MAX_LIBRARY_HEALTH_SCANS {
            self.health_scans.remove(0);
        }
        self.health_scans.push(record.clone());
        Some(record)
    }

    pub(crate) fn start_health_scan(
        &mut self,
        library_path: String,
    ) -> Result<LibraryHealthScanRecord, String> {
        self.refresh_health_scans();
        if let Some(active) = self
            .health_scans
            .iter()
            .find(|scan| scan.status == "running")
        {
            return Err(active.id.clone());
        }
        let id = format!("scan-{}", self.allocate_health_scan_id());
        let now = unix_timestamp();
        let record = LibraryHealthScanRecord {
            id,
            library_path,
            items: self.records.len(),
            issues: Vec::new(),
            status: "running".to_owned(),
            started_at: now,
            updated_at: now,
        };
        if self.health_scans.len() == MAX_LIBRARY_HEALTH_SCANS {
            self.health_scans.remove(0);
        }
        self.health_scans.push(record.clone());
        Ok(record)
    }

    pub(crate) fn refresh_health_scans(&mut self) {
        let now = unix_timestamp();
        let issues = self.health_issues();
        for scan in &mut self.health_scans {
            if scan.status == "running" && now.saturating_sub(scan.started_at) >= 1 {
                scan.status = "completed".to_owned();
                scan.issues = issues.clone();
                scan.updated_at = now;
            }
        }
    }

    pub(crate) fn health_scan(&self, id: &str) -> Option<LibraryHealthScanRecord> {
        self.health_scans.iter().find(|scan| scan.id == id).cloned()
    }

    pub(crate) fn remediation_job(&self, id: &str) -> Option<LibraryRemediationJob> {
        self.remediation_jobs
            .iter()
            .find(|job| job.id == id)
            .cloned()
    }

    pub(crate) fn remediate_selected(
        &mut self,
        issue_ids: &[String],
    ) -> Result<LibraryRemediationJob, String> {
        let issues = self.health_issues();
        let mut fixed = Vec::new();
        for issue_id in issue_ids {
            let Some(issue) = issues
                .iter()
                .find(|issue| issue["id"].as_str() == Some(issue_id.as_str()))
            else {
                continue;
            };
            if issue["type"].as_str() != Some("missing_kind") {
                continue;
            }
            let Some(item_id) = issue["item_id"].as_str() else {
                continue;
            };
            if let Some(item) = self.records.iter_mut().find(|item| item.id == item_id) {
                item.kind = "Audio".to_owned();
                fixed.push(issue_id.clone());
            }
        }
        if fixed.is_empty() {
            return Err("no fixable issues provided".to_owned());
        }
        let now = unix_timestamp();
        self.updated_at = now;
        let fixed_count = fixed.len();
        let job = LibraryRemediationJob {
            id: format!("library-remediation-{}", self.next_remediation_job_id),
            issue_ids: fixed,
            status: "completed".to_owned(),
            fixed_count,
            error: None,
            created_at: now,
            updated_at: now,
        };
        self.next_remediation_job_id = self.next_remediation_job_id.saturating_add(1).max(1);
        if self.remediation_jobs.len() == MAX_LIBRARY_REMEDIATION_JOBS {
            self.remediation_jobs.remove(0);
        }
        self.remediation_jobs.push(job.clone());
        Ok(job)
    }

    pub(crate) fn create(
        &mut self,
        artist: String,
        title: String,
        kind: String,
    ) -> Option<LibraryItemRecord> {
        if self.records.len() >= MAX_LIBRARY_ITEMS {
            return None;
        }
        let id = format!("lib-{}", self.allocate_id());
        let now = unix_timestamp();
        let record = LibraryItemRecord {
            id,
            artist,
            title,
            kind,
            created_at: now,
        };
        self.records.push(record.clone());
        self.updated_at = now;
        Some(record)
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let mut candidate = self.next_id.max(1);
        for _ in 0..=self.records.len() {
            let id = format!("lib-{candidate}");
            if !self.records.iter().any(|record| record.id == id) {
                self.next_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded library store must leave an available u64 id")
    }

    pub(crate) fn allocate_health_scan_id(&mut self) -> u64 {
        let mut candidate = self.next_health_scan_id.max(1);
        for _ in 0..=self.health_scans.len() {
            let id = format!("scan-{candidate}");
            if !self.health_scans.iter().any(|record| record.id == id) {
                self.next_health_scan_id = candidate.wrapping_add(1).max(1);
                return candidate;
            }
            candidate = candidate.wrapping_add(1).max(1);
        }
        unreachable!("bounded library scan store must leave an available u64 id")
    }

    pub(crate) fn get(&self, id: &str) -> Option<LibraryItemRecord> {
        self.records.iter().find(|r| r.id == id).cloned()
    }

    pub(crate) fn delete(&mut self, id: &str) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            self.updated_at = unix_timestamp();
            true
        } else {
            false
        }
    }

    pub(crate) fn json(&self) -> String {
        let records = self
            .records
            .iter()
            .map(LibraryItemRecord::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"items\":[{}],\"count\":{},\"updated_at\":{}}}",
            records,
            self.records.len(),
            self.updated_at
        )
    }

    pub(crate) fn health_issues(&self) -> Vec<serde_json::Value> {
        self.records
            .iter()
            .flat_map(|item| {
                let mut issues = Vec::new();
                if item.artist.trim().is_empty() {
                    issues.push(serde_json::json!({
                        "id": format!("{}-missing-artist", item.id),
                        "item_id": item.id,
                        "artist": item.artist,
                        "title": item.title,
                        "type": "missing_artist",
                        "severity": "warning",
                        "message": "Library item has no artist",
                    }));
                }
                if item.title.trim().is_empty() {
                    issues.push(serde_json::json!({
                        "id": format!("{}-missing-title", item.id),
                        "item_id": item.id,
                        "artist": item.artist,
                        "title": item.title,
                        "type": "missing_title",
                        "severity": "warning",
                        "message": "Library item has no title",
                    }));
                }
                if item.kind.trim().is_empty() {
                    issues.push(serde_json::json!({
                        "id": format!("{}-missing-kind", item.id),
                        "item_id": item.id,
                        "artist": item.artist,
                        "title": item.title,
                        "type": "missing_kind",
                        "severity": "warning",
                        "message": "Library item has no media kind",
                    }));
                }
                issues
            })
            .collect()
    }

    pub(crate) fn fix_health_issues(&mut self) -> Vec<serde_json::Value> {
        let now = unix_timestamp();
        let mut fixed = Vec::new();
        for item in &mut self.records {
            if item.kind.trim().is_empty() {
                item.kind = "Audio".to_owned();
                fixed.push(serde_json::json!({
                    "id": format!("{}-missing-kind", item.id),
                    "item_id": item.id,
                    "type": "missing_kind",
                    "fixed": true,
                    "action": "defaulted_kind",
                    "kind": item.kind,
                }));
            }
        }
        if !fixed.is_empty() {
            self.updated_at = now;
        }
        fixed
    }

    pub(crate) fn patch_health_issue(
        &mut self,
        issue_id: &str,
        artist: Option<String>,
        title: Option<String>,
        kind: Option<String>,
    ) -> Option<serde_json::Value> {
        let issues = self.health_issues();
        let issue = issues
            .iter()
            .find(|issue| issue.get("id").and_then(serde_json::Value::as_str) == Some(issue_id))?;
        let item_id = issue.get("item_id")?.as_str()?.to_owned();
        let issue_type = issue.get("type")?.as_str()?.to_owned();
        let item = self.records.iter_mut().find(|item| item.id == item_id)?;
        let mut action = "unchanged";
        match issue_type.as_str() {
            "missing_artist" => {
                item.artist = truncate_utf8_bytes(
                    artist
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "Unknown Artist".to_owned()),
                    MAX_LIST_ARTIST_BYTES,
                );
                action = "defaulted_artist";
            }
            "missing_title" => {
                item.title = truncate_utf8_bytes(
                    title
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "Untitled".to_owned()),
                    MAX_LIST_TITLE_BYTES,
                );
                action = "defaulted_title";
            }
            "missing_kind" => {
                item.kind = truncate_utf8_bytes(
                    kind.filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "Audio".to_owned()),
                    MAX_LIST_KIND_BYTES,
                );
                action = "defaulted_kind";
            }
            _ => {}
        }
        self.updated_at = unix_timestamp();
        Some(serde_json::json!({
            "id": issue_id,
            "item_id": item.id,
            "type": issue_type,
            "updated": true,
            "fixed": action != "unchanged",
            "action": action,
            "item": serde_json::from_str::<serde_json::Value>(&item.json()).unwrap_or_else(|_| serde_json::json!({ "id": item.id })),
            "updated_at": self.updated_at,
        }))
    }

    pub(crate) fn health_summary_json(&self, library_path: String) -> String {
        self.health_summary_value(library_path).to_string()
    }

    pub(crate) fn health_summary_value(&self, library_path: String) -> serde_json::Value {
        let total_issues = self.health_issues().len();
        serde_json::json!({
            "libraryPath": library_path,
            "totalIssues": total_issues,
            "issuesOpen": total_issues,
            "issuesResolved": 0,
        })
    }

    pub(crate) fn health_api_issues(&self) -> Vec<serde_json::Value> {
        let mut issues = self
            .health_issues()
            .into_iter()
            .map(|issue| {
                let issue_id = issue
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                let issue_kind = issue
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("missing_metadata");
                let remediation = self.remediation_jobs.iter().find(|job| {
                    job.issue_ids.iter().any(|id| id == issue_id)
                });
                let detected_at = issue
                    .get("item_id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|item_id| self.records.iter().find(|item| item.id == item_id))
                    .and_then(|item| i64::try_from(item.created_at).ok())
                    .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
                    .map(|timestamp| {
                        timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                    })
                    .unwrap_or_else(|| chrono::DateTime::UNIX_EPOCH.to_rfc3339());
                serde_json::json!({
                    "issueId": issue_id,
                    "type": "MissingMetadata",
                    "severity": "Medium",
                    "filePath": "",
                    "musicBrainzRecordingId": "",
                    "musicBrainzReleaseId": "",
                    "artist": issue.get("artist").and_then(serde_json::Value::as_str).unwrap_or_default(),
                    "album": "",
                    "title": issue.get("title").and_then(serde_json::Value::as_str).unwrap_or_default(),
                    "reason": issue.get("message").and_then(serde_json::Value::as_str).unwrap_or("Library metadata is incomplete"),
                    "metadata": {
                        "itemId": issue.get("item_id").and_then(serde_json::Value::as_str).unwrap_or_default(),
                        "missingField": issue_kind,
                    },
                    "canAutoFix": issue_kind == "missing_kind",
                    "suggestedAction": "Review and complete the missing library metadata",
                    "remediationJobId": remediation.map(|job| job.id.clone()).unwrap_or_default(),
                    "status": if remediation.is_some() { "Resolved" } else { "Detected" },
                    "detectedAt": detected_at,
                    "resolvedAt": remediation
                        .map(|job| unix_seconds_rfc3339(job.updated_at))
                        .map_or(serde_json::Value::Null, serde_json::Value::String),
                    "resolvedBy": "",
                })
            })
            .collect::<Vec<_>>();
        issues.sort_by(|left, right| {
            right["detectedAt"]
                .as_str()
                .cmp(&left["detectedAt"].as_str())
                .then_with(|| left["issueId"].as_str().cmp(&right["issueId"].as_str()))
        });
        issues
    }

    pub(crate) fn health_issues_json(&self, filter: &LibraryHealthIssueQuery) -> String {
        let issues =
            self.health_api_issues()
                .into_iter()
                .filter(|issue| {
                    (filter.types.is_empty()
                        || filter
                            .types
                            .iter()
                            .any(|value| value == issue["type"].as_str().unwrap_or_default()))
                        && (filter.severities.is_empty()
                            || filter.severities.iter().any(|value| {
                                value == issue["severity"].as_str().unwrap_or_default()
                            }))
                        && (filter.statuses.is_empty()
                            || filter
                                .statuses
                                .iter()
                                .any(|value| value == issue["status"].as_str().unwrap_or_default()))
                        && (filter.musicbrainz_release_id.is_empty()
                            || filter.musicbrainz_release_id
                                == issue["musicBrainzReleaseId"].as_str().unwrap_or_default())
                })
                .collect::<Vec<_>>();
        let total_count = issues.len();
        let issues = issues
            .into_iter()
            .skip(filter.offset)
            .take(filter.limit)
            .collect::<Vec<_>>();
        serde_json::json!({
            "issues": issues,
            "totalCount": total_count,
            "filter": {
                "libraryPath": filter.library_path,
                "types": filter.types,
                "severities": filter.severities,
                "statuses": filter.statuses,
                "musicBrainzReleaseId": filter.musicbrainz_release_id,
                "limit": filter.limit,
                "offset": filter.offset,
            },
        })
        .to_string()
    }

    pub(crate) fn health_issue_type_summaries(&self) -> Vec<serde_json::Value> {
        let count = self.health_issues().len();
        if count == 0 {
            Vec::new()
        } else {
            vec![serde_json::json!({
                "type": "MissingMetadata",
                "count": count,
                "bySeverity": { "Medium": count },
            })]
        }
    }

    pub(crate) fn health_issue_artist_summaries(&self, limit: usize) -> Vec<serde_json::Value> {
        let mut grouped = std::collections::BTreeMap::<String, usize>::new();
        for issue in self.health_issues() {
            let Some(artist) = issue
                .get("artist")
                .and_then(serde_json::Value::as_str)
                .filter(|artist| !artist.trim().is_empty())
            else {
                continue;
            };
            let artist = artist.to_owned();
            *grouped.entry(artist).or_default() += 1;
        }
        let mut rows = grouped
            .into_iter()
            .map(|(artist, count)| {
                serde_json::json!({
                    "artist": artist,
                    "count": count,
                    "byType": { "MissingMetadata": count },
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right["count"]
                .as_u64()
                .cmp(&left["count"].as_u64())
                .then_with(|| left["artist"].as_str().cmp(&right["artist"].as_str()))
        });
        rows.truncate(limit);
        rows
    }

    pub(crate) fn health_issues_by_artist_json(&self, limit: usize) -> String {
        let rows = self.health_issue_artist_summaries(limit);
        let total_artists = rows.len();
        serde_json::json!({
            "groups": rows,
            "totalArtists": total_artists,
        })
        .to_string()
    }

    pub(crate) fn health_issues_by_release_json(&self, limit: usize) -> String {
        let mut grouped = BTreeMap::<(String, String), usize>::new();
        for item in &self.records {
            let count = [
                item.artist.trim().is_empty(),
                item.title.trim().is_empty(),
                item.kind.trim().is_empty(),
            ]
            .into_iter()
            .filter(|missing| *missing)
            .count();
            if count > 0 && !item.title.trim().is_empty() {
                *grouped
                    .entry((item.artist.clone(), item.title.clone()))
                    .or_default() += count;
            }
        }
        let mut rows = grouped
            .into_iter()
            .map(|((artist, album), count)| {
                serde_json::json!({
                    "artist": artist,
                    "album": album,
                    "musicBrainzReleaseId": "",
                    "count": count,
                    "byType": { "MissingMetadata": count },
                })
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right["count"]
                .as_u64()
                .cmp(&left["count"].as_u64())
                .then_with(|| left["artist"].as_str().cmp(&right["artist"].as_str()))
                .then_with(|| left["album"].as_str().cmp(&right["album"].as_str()))
        });
        rows.truncate(limit);
        let total_releases = rows.len();
        serde_json::json!({
            "groups": rows,
            "totalReleases": total_releases,
        })
        .to_string()
    }

    pub(crate) fn health_issues_by_type_json(&self, issue_type: Option<&str>) -> String {
        let rows = if issue_type.is_some_and(|requested| {
            !requested.eq_ignore_ascii_case("MissingMetadata") && !requested.starts_with("missing_")
        }) {
            Vec::new()
        } else {
            self.health_issue_type_summaries()
        };
        let total_issues = rows
            .iter()
            .filter_map(|row| row.get("count").and_then(serde_json::Value::as_u64))
            .sum::<u64>();
        serde_json::json!({
            "groups": rows,
            "totalIssues": total_issues,
        })
        .to_string()
    }

    pub(crate) fn health_dashboard_json(
        &self,
        library_path: String,
        artist_limit: usize,
        issue_limit: usize,
    ) -> String {
        let issues = self.health_api_issues();
        let total_issues = issues.len();
        serde_json::json!({
            "summary": self.health_summary_value(library_path),
            "issuesByType": self.health_issue_type_summaries(),
            "issuesByArtist": self.health_issue_artist_summaries(artist_limit),
            "issues": issues.into_iter().take(issue_limit).collect::<Vec<_>>(),
            "totalIssues": total_issues,
        })
        .to_string()
    }

    pub(crate) fn health_issues_by_codec_json(&self) -> String {
        let total_issues = self.health_issues().len();
        let groups = if total_issues == 0 {
            Vec::new()
        } else {
            vec![serde_json::json!({
                "codec": "UNKNOWN",
                "count": total_issues,
                "transcodeSuspect": 0,
            })]
        };
        serde_json::json!({ "groups": groups, "totalIssues": total_issues }).to_string()
    }

    pub(crate) fn musicbrainz_completion_json(&self) -> String {
        let mut grouped = std::collections::BTreeMap::<String, (usize, usize)>::new();
        for item in &self.records {
            let artist = if item.artist.trim().is_empty() {
                "(unknown)"
            } else {
                item.artist.as_str()
            };
            let entry = grouped.entry(artist.to_owned()).or_default();
            entry.0 += 1;
            if !item.title.trim().is_empty() {
                entry.1 += 1;
            }
        }
        let rows = grouped
            .into_iter()
            .map(|(artist, (total, complete))| {
                let completion = if total == 0 {
                    0.0
                } else {
                    complete as f64 / total as f64
                };
                serde_json::json!({
                    "artist": artist,
                    "items": total,
                    "complete": complete,
                    "completion": completion,
                })
            })
            .collect::<Vec<_>>();
        let average = if rows.is_empty() {
            0.0
        } else {
            rows.iter()
                .filter_map(|row| row.get("completion").and_then(serde_json::Value::as_f64))
                .sum::<f64>()
                / rows.len() as f64
        };
        serde_json::json!({
            "completion_status": rows,
            "average_completion": average,
            "count": rows.len(),
            "updated_at": self.updated_at,
        })
        .to_string()
    }

    pub(crate) fn discography_coverage_json(&self, artist: &str) -> String {
        let releases = self
            .records
            .iter()
            .filter(|item| item.artist.eq_ignore_ascii_case(artist))
            .count();
        serde_json::json!({
            "artist": artist,
            "coverage": if releases == 0 { 0.0 } else { 1.0 },
            "releases": releases,
            "updated_at": self.updated_at,
        })
        .to_string()
    }

    pub(crate) fn target_json(&self, target: &str) -> String {
        let rows = self
            .records
            .iter()
            .filter(|item| item.artist.eq_ignore_ascii_case(target) || item.id == target)
            .map(|item| {
                serde_json::json!({
                    "id": item.id,
                    "artist": item.artist,
                    "title": item.title,
                    "kind": item.kind,
                    "created_at": item.created_at,
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "target": target,
            "items": rows,
            "count": rows.len(),
            "updated_at": self.updated_at,
        })
        .to_string()
    }
}

pub(super) fn persisted_library_item(
    record: &LibraryItemRecord,
) -> crate::persistence::LibraryItemRecord {
    crate::persistence::LibraryItemRecord {
        id: record.id.clone(),
        artist: record.artist.clone(),
        title: record.title.clone(),
        kind: record.kind.clone(),
        created_at: i64::try_from(record.created_at).unwrap_or(i64::MAX),
    }
}

pub(super) async fn persist_library_item_checked(
    state: &AppState,
    record: &LibraryItemRecord,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.upsert_library_item(&persisted_library_item(record))
        .await
        .map_err(|error| format!("library persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_library_items_checked(
    state: &AppState,
    records: &[LibraryItemRecord],
) -> Result<bool, String> {
    if records.is_empty() {
        return Ok(state.db.is_some());
    }
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    let persisted = records
        .iter()
        .map(persisted_library_item)
        .collect::<Vec<_>>();
    db.upsert_library_items(&persisted)
        .await
        .map_err(|error| format!("library persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn persist_library_item_delete_checked(
    state: &AppState,
    id: &str,
) -> Result<bool, String> {
    let Some(db) = state.db.as_ref() else {
        return Ok(false);
    };
    db.delete_library_item(id)
        .await
        .map_err(|error| format!("library deletion persistence failed: {error}"))?;
    Ok(true)
}

pub(super) async fn rollback_library_if_unchanged(
    state: &AppState,
    previous: LibraryStore,
    mutated: &LibraryStore,
) {
    let mut library = state.library.write().await;
    if *library == *mutated {
        *library = previous;
    }
}
