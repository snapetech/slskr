use super::{
    command_exists_on_path, configured_command_exists, file_exists_at_known_location_or_on_path,
    songid_scoring, IntegrationSettings, LibraryStore, Path, ShareIndexSnapshot,
};

/// Matches the oracle's real `SongIdService.BuildEvidencePackage`/
/// `SongIdRunEvidencePackage` contract: a real reshape of the same
/// stored run fields the other SongID endpoints already read/write
/// (scorecard, identityAssessment, syntheticAssessment, track/album/
/// artist candidates, segments, mixGroups, plans, options, evidence),
/// applying the oracle's real sort-by-score/truncate rules and deriving
/// warnings from the same real conditions (incomplete run, no track
/// candidates, no recognizer hits, no forensic matrix). Fields slskR's
/// synchronous, non-fingerprinting analysis pipeline never populates
/// (artifacts from clips/stems/perturbations/a full-source fingerprint,
/// a forensic matrix) are honestly reported as empty/absent rather than
/// fabricated.
pub(super) fn songid_evidence_package_json(run: &serde_json::Value) -> serde_json::Value {
    fn sorted_candidates(
        run: &serde_json::Value,
        field: &str,
        take: usize,
    ) -> Vec<serde_json::Value> {
        let mut candidates = run
            .get(field)
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        candidates.sort_by(|left, right| {
            let score = |value: &serde_json::Value, field: &str| {
                value
                    .get(field)
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            score(right, "actionScore")
                .partial_cmp(&score(left, "actionScore"))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    score(right, "identityScore")
                        .partial_cmp(&score(left, "identityScore"))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        candidates.truncate(take);
        candidates
    }
    fn taken(run: &serde_json::Value, field: &str, take: usize) -> Vec<serde_json::Value> {
        let mut values = run
            .get(field)
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        values.truncate(take);
        values
    }

    let status = run
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let track_candidates = sorted_candidates(run, "tracks", 10);
    let scorecard = run
        .get("scorecard")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let forensic_matrix = run
        .get("forensicMatrix")
        .cloned()
        .filter(|value| !value.is_null());
    let artifact_directory = run
        .get("artifactDirectory")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    let mut artifacts = Vec::new();
    if !artifact_directory.trim().is_empty() {
        artifacts.push(serde_json::json!({
            "kind": "workspace",
            "label": "SongID artifact directory",
            "path": artifact_directory,
        }));
    }
    for (field, kind) in [
        ("clips", "clip"),
        ("stems", "stem"),
        ("perturbations", "perturbation"),
    ] {
        for item in run
            .get(field)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let path = item
                .get("path")
                .or_else(|| item.get("fingerprint"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if path.trim().is_empty() {
                continue;
            }
            let label = item
                .get("label")
                .and_then(serde_json::Value::as_str)
                .filter(|label| !label.trim().is_empty())
                .or_else(|| item.get("clipId").and_then(serde_json::Value::as_str))
                .or_else(|| item.get("artifactId").and_then(serde_json::Value::as_str))
                .or_else(|| {
                    item.get("perturbationId")
                        .and_then(serde_json::Value::as_str)
                })
                .unwrap_or_default();
            artifacts.push(serde_json::json!({
                "kind": kind,
                "label": label,
                "path": path,
                "startSeconds": item.get("startSeconds").cloned().unwrap_or(serde_json::Value::Null),
                "durationSeconds": item.get("durationSeconds").cloned().unwrap_or(serde_json::Value::Null),
            }));
        }
    }
    artifacts.truncate(100);

    let mut warnings = Vec::new();
    if !status.eq_ignore_ascii_case("completed") {
        warnings.push("SongID run is not completed; evidence package may be partial.".to_owned());
    }
    if track_candidates.is_empty() {
        warnings.push("No track candidates were produced.".to_owned());
    }
    let hit_count = |field: &str| {
        scorecard
            .get(field)
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0)
    };
    if hit_count("acoustIdHitCount") == 0
        && hit_count("songRecHitCount") == 0
        && hit_count("panakoHitCount") == 0
        && hit_count("audfprintHitCount") == 0
    {
        warnings.push("No recognizer hits were recorded.".to_owned());
    }
    if forensic_matrix.is_none() {
        warnings.push("No forensic matrix was generated.".to_owned());
    }

    serde_json::json!({
        "runId": run.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "sourceType": run.get("sourceType").cloned().unwrap_or(serde_json::Value::Null),
        "status": status,
        "query": run.get("query").cloned().unwrap_or(serde_json::Value::Null),
        "createdAt": run.get("createdAt").cloned().unwrap_or(serde_json::Value::Null),
        "completedAt": if status.eq_ignore_ascii_case("completed") {
            run.get("createdAt").cloned().unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        },
        "summary": run.get("summary").cloned().unwrap_or(serde_json::Value::Null),
        "currentStage": run.get("currentStage").cloned().unwrap_or(serde_json::Value::Null),
        "percentComplete": run.get("percentComplete").cloned().unwrap_or(serde_json::json!(0.0)),
        "scorecard": scorecard,
        "identityAssessment": run.get("identityAssessment").cloned().unwrap_or_else(|| serde_json::json!({})),
        "syntheticAssessment": run.get("syntheticAssessment").cloned().unwrap_or_else(|| serde_json::json!({})),
        "forensicMatrix": forensic_matrix,
        "trackCandidates": track_candidates,
        "albumCandidates": sorted_candidates(run, "albums", 6),
        "artistCandidates": sorted_candidates(run, "artists", 6),
        "segments": taken(run, "segments", 12),
        "mixGroups": taken(run, "mixGroups", 12),
        "plans": taken(run, "plans", 20),
        "acquisitionOptions": taken(run, "options", 20),
        "evidence": taken(run, "evidence", 100),
        "artifacts": artifacts,
        "warnings": warnings,
    })
}

/// Matches the oracle's `SongIdCapabilityReporter`: real external-tool
/// presence checks on `PATH` (and, for panako/audfprint, known install
/// locations), not a hardcoded list. Integration-backed capabilities also
/// reflect the configured live clients and required local tools.
pub(super) fn songid_capabilities_json(
    integrations: Option<&IntegrationSettings>,
) -> serde_json::Value {
    const DOCKER_HINT: &str = "For Docker, run install-optional-media-tools with the matching profile or install the tool in a derived image and keep it on PATH.";
    fn capability(
        id: &str,
        label: &str,
        status: &str,
        available: bool,
        reason: String,
        requirements: &[&str],
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "label": label,
            "status": status,
            "available": available,
            "reason": reason,
            "requirements": requirements,
        })
    }

    fn missing_tools_reason(missing: &[&str]) -> String {
        format!(
            "{} {DOCKER_HINT}",
            match missing {
                [] => "Required tools were not found on PATH.".to_owned(),
                [one] => format!("{one} was not found on PATH."),
                many => format!("{} were not found on PATH.", many.join(" and ")),
            }
        )
    }

    /// Builds the "available" flag and reason for a capability that
    /// requires every named tool to be present.
    fn tool_reason(tools: &[(&str, bool)]) -> (bool, String) {
        let missing = tools
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| *name)
            .collect::<Vec<_>>();
        let available = missing.is_empty();
        let reason = if available {
            format!(
                "{} {}",
                tools
                    .iter()
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(" and "),
                if tools.len() == 1 {
                    "is available."
                } else {
                    "are available."
                }
            )
        } else {
            missing_tools_reason(&missing)
        };
        (available, reason)
    }

    let yt_dlp = command_exists_on_path("yt-dlp");
    let ffmpeg = integrations
        .map(|settings| configured_command_exists(&settings.chromaprint.ffmpeg_path))
        .unwrap_or_else(|| command_exists_on_path("ffmpeg"));
    let songrec = command_exists_on_path("songrec");
    let whisper = command_exists_on_path("whisper");
    let tesseract = command_exists_on_path("tesseract");
    let demucs = command_exists_on_path("demucs");
    let c2patool = command_exists_on_path("c2patool");
    let panako = file_exists_at_known_location_or_on_path(
        &[
            "/usr/share/java/panako.jar",
            "/usr/local/share/java/panako.jar",
            "/usr/share/panako/panako.jar",
            "/usr/local/share/panako/panako.jar",
        ],
        "panako.jar",
    );
    let audfprint = file_exists_at_known_location_or_on_path(
        &[
            "/usr/share/audfprint/audfprint.py",
            "/usr/local/share/audfprint/audfprint.py",
            "/opt/audfprint/audfprint.py",
        ],
        "audfprint.py",
    );

    let (youtube_metadata_available, youtube_metadata_reason) = tool_reason(&[("yt-dlp", yt_dlp)]);
    let (youtube_audio_available, youtube_audio_reason) =
        tool_reason(&[("yt-dlp", yt_dlp), ("ffmpeg", ffmpeg)]);
    let (songrec_available, songrec_reason) = tool_reason(&[("songrec", songrec)]);
    let (demucs_available, demucs_reason) = tool_reason(&[("demucs", demucs), ("ffmpeg", ffmpeg)]);
    let (whisper_available, whisper_reason) =
        tool_reason(&[("whisper", whisper), ("ffmpeg", ffmpeg)]);
    let (ocr_available, ocr_reason) = tool_reason(&[("tesseract", tesseract), ("ffmpeg", ffmpeg)]);
    let (c2pa_available, c2pa_reason) = tool_reason(&[("c2patool", c2patool)]);
    let musicbrainz = integrations.map(|settings| &settings.musicbrainz);
    let musicbrainz_configured =
        musicbrainz.is_some_and(|settings| reqwest::Url::parse(&settings.base_url).is_ok());
    let musicbrainz_reason = if musicbrainz_configured {
        "MusicBrainz client is configured.".to_owned()
    } else {
        "MusicBrainz base URL is not valid.".to_owned()
    };
    let chromaprint_enabled = integrations.is_some_and(|settings| settings.chromaprint.enabled);
    let chromaprint_ffmpeg = integrations
        .is_some_and(|settings| configured_command_exists(&settings.chromaprint.ffmpeg_path));
    let fpcalc = command_exists_on_path("fpcalc");
    let chromaprint_available = chromaprint_enabled && chromaprint_ffmpeg && fpcalc;
    let chromaprint_reason = if !chromaprint_enabled {
        "Chromaprint is disabled; set integration.chromaprint.enabled to enable it.".to_owned()
    } else if !chromaprint_ffmpeg {
        "Configured Chromaprint ffmpeg executable was not found.".to_owned()
    } else if !fpcalc {
        format!("fpcalc/libchromaprint was not found on PATH. {DOCKER_HINT}")
    } else {
        "Chromaprint is enabled and ffmpeg/libchromaprint are available.".to_owned()
    };
    let acoustid_configured = integrations
        .is_some_and(|settings| settings.acoustid.enabled && settings.acoustid.client_id.is_some());
    let acoustid_reason = if acoustid_configured {
        "AcoustID is enabled with a client id.".to_owned()
    } else {
        "Requires integration.acoustid.enabled and a client id.".to_owned()
    };

    serde_json::json!([
        capability(
            "text_query",
            "Text query analysis",
            "stable",
            true,
            "Text searches do not require external tools.".to_owned(),
            &[]
        ),
        capability(
            "url_parsing",
            "YouTube and Spotify URL parsing",
            "experimental",
            true,
            "URL classification and metadata fallback are implemented.".to_owned(),
            &[]
        ),
        capability(
            "musicbrainz_lookup",
            "MusicBrainz lookup",
            "experimental",
            musicbrainz_configured,
            musicbrainz_reason,
            &["musicbrainz base URL"]
        ),
        capability(
            "spotify_page_metadata",
            "Spotify page metadata",
            "experimental",
            true,
            "Spotify page metadata fetch is implemented for public pages.".to_owned(),
            &[]
        ),
        capability(
            "youtube_metadata",
            "YouTube metadata",
            "experimental",
            youtube_metadata_available,
            youtube_metadata_reason.clone(),
            &["yt-dlp"]
        ),
        capability(
            "youtube_audio",
            "YouTube audio extraction",
            "experimental",
            youtube_audio_available,
            youtube_audio_reason,
            &["yt-dlp", "ffmpeg"]
        ),
        capability(
            "local_file_intake",
            "Local file intake",
            "experimental",
            true,
            "Local files are accepted when they are under configured safe directories.".to_owned(),
            &[]
        ),
        capability(
            "chromaprint_fingerprint",
            "Chromaprint fingerprinting",
            "experimental",
            chromaprint_available,
            chromaprint_reason,
            &[
                "integration.chromaprint.enabled",
                "ffmpeg",
                "libchromaprint",
            ],
        ),
        capability(
            "acoustid_lookup",
            "AcoustID lookup",
            "experimental",
            acoustid_configured,
            acoustid_reason,
            &[
                "integration.acoustid.enabled",
                "integration.acoustid.client_id",
            ],
        ),
        capability(
            "songrec",
            "SongRec recognition",
            "experimental",
            songrec_available,
            songrec_reason,
            &["songrec"]
        ),
        capability(
            "panako",
            "Panako local corpus matching",
            "experimental",
            panako,
            if panako {
                "Panako jar found.".to_owned()
            } else {
                format!("panako.jar was not found in known locations or PATH. {DOCKER_HINT}")
            },
            &["java", "panako.jar"],
        ),
        capability(
            "audfprint",
            "Audfprint local corpus matching",
            "experimental",
            audfprint,
            if audfprint {
                "Audfprint script found.".to_owned()
            } else {
                format!("audfprint.py was not found in known locations or PATH. {DOCKER_HINT}")
            },
            &["python", "audfprint.py"],
        ),
        capability(
            "demucs",
            "Demucs stem extraction",
            "experimental",
            demucs_available,
            demucs_reason,
            &["demucs", "ffmpeg"]
        ),
        capability(
            "whisper_transcripts",
            "Whisper transcript extraction",
            "experimental",
            whisper_available,
            whisper_reason,
            &["whisper", "ffmpeg"]
        ),
        capability(
            "ocr_frames",
            "OCR frame scanning",
            "experimental",
            ocr_available,
            ocr_reason,
            &["tesseract", "ffmpeg"]
        ),
        capability(
            "comments_and_chapters",
            "YouTube comments and chapters",
            "experimental",
            youtube_metadata_available,
            youtube_metadata_reason.clone(),
            &["yt-dlp"]
        ),
        capability(
            "c2pa_provenance",
            "C2PA provenance signal detection",
            "experimental",
            c2pa_available,
            c2pa_reason,
            &["c2patool"]
        ),
        capability(
            "hash_from_audio_file_flag",
            "HashFromAudioFileEnabled flag",
            "broken",
            false,
            "Startup validation rejects this flag because PCM extraction support for that hardening path is unavailable.".to_owned(),
            &[]
        ),
    ])
}

// Library Item Models

fn songid_matches_value(
    library: &LibraryStore,
    shares: &ShareIndexSnapshot,
    query: Option<&str>,
) -> Vec<serde_json::Value> {
    library
        .records
        .iter()
        .flat_map(|item| {
            shares.entries.iter().filter_map(move |entry| {
                let filename_score = songid_scoring::filename_identity_score(
                    &item.artist,
                    &item.title,
                    &entry.filename,
                );
                let query_score = query
                    .filter(|query| !query.trim().is_empty())
                    .map(|query| songid_scoring::compare_loose_text(query, &entry.filename))
                    .unwrap_or(0.0);
                let score = if query_score > 0.0 {
                    (filename_score * 0.75 + query_score * 0.25).min(1.0)
                } else {
                    filename_score
                };
                if score < 0.15 {
                    return None;
                }
                let normalized_filename = songid_scoring::normalize_loose_text(&entry.filename);
                let normalized_artist = songid_scoring::normalize_loose_text(&item.artist);
                let normalized_title = songid_scoring::normalize_loose_text(&item.title);
                let artist_similarity =
                    songid_scoring::compare_loose_text(&item.artist, &entry.filename);
                let title_similarity =
                    songid_scoring::compare_loose_text(&item.title, &entry.filename);
                let artist_match = !normalized_artist.is_empty()
                    && normalized_filename.contains(&normalized_artist);
                let title_match =
                    !normalized_title.is_empty() && normalized_filename.contains(&normalized_title);
                let mut signals = Vec::new();
                if artist_match {
                    signals.push("artist");
                }
                if title_match {
                    signals.push("title");
                }
                if entry.extension.eq_ignore_ascii_case("flac")
                    || entry.extension.eq_ignore_ascii_case("wav")
                    || entry.extension.eq_ignore_ascii_case("alac")
                {
                    signals.push("lossless_extension");
                }
                Some(serde_json::json!({
                    "libraryItemId": item.id,
                    "artist": item.artist,
                    "title": item.title,
                    "filename": entry.filename,
                    "extension": entry.extension,
                    "size": entry.size,
                    "score": score,
                    "identityScore": score,
                    "actionScore": (score * 0.75
                        + if entry.extension.eq_ignore_ascii_case("flac")
                            || entry.extension.eq_ignore_ascii_case("wav")
                            || entry.extension.eq_ignore_ascii_case("alac")
                        {
                            0.25
                        } else {
                            0.0
                        })
                        .min(1.0),
                    "artistSimilarity": artist_similarity,
                    "titleSimilarity": title_similarity,
                    "querySimilarity": query_score,
                    "signals": signals,
                    "source": "share-index",
                }))
            })
        })
        .collect()
}

/// Matches the oracle's `SongIdService.DetectSourceType` classification. The
/// source can still fall back to a text query when optional metadata tools are
/// unavailable, but the run must retain its actual source family.
pub(super) fn songid_source_type(source: &str) -> &'static str {
    let lower = source.trim().to_ascii_lowercase();
    if Path::new(source.trim()).is_file() {
        return "local_file";
    }
    if (lower.starts_with("http://") || lower.starts_with("https://"))
        && (lower.contains("youtube.com/") || lower.contains("youtu.be/"))
    {
        return "youtube_url";
    }
    if lower.starts_with("spotify:track:")
        || ((lower.starts_with("http://") || lower.starts_with("https://"))
            && lower.contains("open.spotify.com/track/"))
    {
        return "spotify_url";
    }
    if reqwest::Url::parse(source.trim()).is_ok() {
        return "url";
    }
    "text_query"
}

pub(super) fn songid_runs_value(
    library: &LibraryStore,
    shares: &ShareIndexSnapshot,
    query: Option<&str>,
) -> Vec<serde_json::Value> {
    let matches = songid_matches_value(library, shares, query);
    if library.records.is_empty() && shares.entries.is_empty() {
        return Vec::new();
    }
    let scores = matches
        .iter()
        .filter_map(|candidate| candidate.get("score").and_then(serde_json::Value::as_f64))
        .collect::<Vec<_>>();
    let consensus = songid_scoring::identity_consensus(&scores);
    let strongest = scores.iter().copied().fold(0.0_f64, f64::max);
    let candidate_text = matches
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("filename")
                .and_then(serde_json::Value::as_str)
        })
        .take(256)
        .collect::<Vec<_>>()
        .join("\n");
    let repeated_candidate_lines = songid_scoring::repeated_line_ratio(&candidate_text);
    let repeated_candidate_ngrams = songid_scoring::repeated_ngram_ratio(&candidate_text);
    let matrix = matches
        .iter()
        .take(100)
        .map(|candidate| {
            serde_json::json!({
                "libraryItemId": candidate.get("libraryItemId").cloned().unwrap_or(serde_json::Value::Null),
                "filename": candidate.get("filename").cloned().unwrap_or(serde_json::Value::Null),
                "score": candidate.get("score").cloned().unwrap_or(serde_json::json!(0.0)),
                "signals": candidate.get("signals").cloned().unwrap_or_else(|| serde_json::json!([])),
            })
        })
        .collect::<Vec<_>>();
    vec![serde_json::json!({
        "id": "songid-local",
        "status": "completed",
        "libraryItems": library.records.len(),
        "sharedFiles": shares.entries.len(),
        "matches": matches,
        "matchCount": scores.len(),
        "scorecard": {
            "candidateCount": scores.len(),
            "strongMatchCount": scores.iter().filter(|score| **score >= 0.75).count(),
            "identityConsensus": consensus,
            "highestIdentityScore": strongest,
        },
        "identityAssessment": {
            "verdict": if strongest >= 0.8 { "strong_identity" } else if strongest >= 0.45 { "candidate_identity" } else { "weak_identity" },
            "confidence": consensus,
            "summary": if scores.is_empty() { "No matching shared files were found." } else { "Shared-file metadata produced identity candidates." },
        },
        "forensicMatrix": matrix,
        "syntheticAssessment": {
            "verdict": "insufficient_evidence",
            "confidence": "low",
            "syntheticScore": 0.0,
            "identityScore": consensus,
            "candidateLineRepetition": repeated_candidate_lines,
            "candidateNgramRepetition": repeated_candidate_ngrams,
            "summary": "No audio forensic transcript or perturbation evidence was supplied.",
        },
        "updated_at": library.updated_at,
    })]
}
