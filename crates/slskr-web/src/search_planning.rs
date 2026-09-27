pub fn default_experience_preferences() -> serde_json::Map<String, serde_json::Value> {
    experience_preferences()
        .iter()
        .map(|preference| {
            let value = if preference.input == "checkbox" {
                serde_json::Value::Bool(preference.default_value == "true")
            } else {
                serde_json::Value::String(preference.default_value.to_string())
            };
            (preference.id.to_string(), value)
        })
        .collect()
}

fn preference_string(
    values: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: &str,
) -> String {
    values
        .get(key)
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn preference_bool(
    values: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: bool,
) -> bool {
    values
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(fallback)
}

pub fn experience_preferences_report(
    values: &serde_json::Map<String, serde_json::Value>,
) -> String {
    [
        "slskr experience preferences".to_string(),
        format!(
            "Search: ranking={}, condition={}, duplicate_folding={}, previews={}",
            preference_string(values, "searchRankingProfile", "balanced"),
            preference_string(values, "searchPreferredCondition", "lossless"),
            preference_bool(values, "searchDuplicateFolding", true),
            preference_string(values, "searchActionPreviewDensity", "detailed")
        ),
        format!(
            "Discovery: approval={}, confidence>={}, stale_days={}",
            preference_string(values, "discoveryApprovalFilter", "all"),
            preference_string(values, "discoveryConfidenceFloor", "0.70"),
            preference_string(values, "discoveryStaleDays", "14")
        ),
        format!(
            "Player: queue_auto_fill={}, radio_seed={}, ratings={}, history={}, scrobble={}, visualizer={}, shortcuts={}",
            preference_bool(values, "playerQueueAutoFill", false),
            preference_string(values, "playerRadioSeedMode", "current"),
            preference_bool(values, "playerShowRatings", true),
            preference_bool(values, "playerCaptureHistory", true),
            preference_string(values, "playerScrobbleMode", "manual"),
            preference_string(values, "playerDefaultVisualizer", "last"),
            preference_bool(values, "playerKeyboardShortcuts", true)
        ),
        format!(
            "Messages: dense={}, pinned_restore={}, unread_badges={}, search={}",
            preference_bool(values, "messagesDenseMode", false),
            preference_bool(values, "messagesPinnedRestore", true),
            preference_bool(values, "messagesUnreadBadges", true),
            preference_bool(values, "messagesSearchEnabled", true)
        ),
    ]
    .join("\n")
}

pub fn automation_summary_from_state(
    state: &serde_json::Map<String, serde_json::Value>,
) -> (usize, usize, usize) {
    let enabled = automation_recipes()
        .iter()
        .filter(|recipe| {
            state
                .get(recipe.id)
                .and_then(|entry| entry.get("enabled"))
                .and_then(|value| value.as_bool())
                .unwrap_or(recipe.enabled_by_default)
        })
        .count();
    let total = automation_recipes().len();
    (total, enabled, total.saturating_sub(enabled))
}

pub fn automation_dry_run_report(recipe: AutomationRecipe, timestamp: &str) -> serde_json::Value {
    serde_json::json!({
        "approvalGate": recipe.approval_gate,
        "cooldown": recipe.cooldown,
        "executed": false,
        "fileImpact": recipe.file_impact,
        "generatedAt": timestamp,
        "maxRunTime": recipe.max_run_time,
        "networkImpact": recipe.network_impact,
        "recipeId": recipe.id,
        "title": recipe.title,
    })
}

pub fn automation_history_report(state: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut entries = Vec::new();
    for recipe in automation_recipes() {
        let stored = state.get(recipe.id);
        let enabled = stored
            .and_then(|entry| entry.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(recipe.enabled_by_default);
        let last_dry_run = stored
            .and_then(|entry| entry.get("lastDryRunAt"))
            .map(json_scalar_preview)
            .filter(|value| !value.is_empty());
        let last_run = stored
            .and_then(|entry| entry.get("lastRunAt"))
            .map(json_scalar_preview)
            .filter(|value| !value.is_empty());
        if enabled || last_dry_run.is_some() || last_run.is_some() {
            entries.push((recipe, enabled, last_dry_run, last_run));
        }
    }

    let mut lines = vec![
        "slskr automation review history".to_string(),
        format!("Entries: {}", entries.len()),
        String::new(),
    ];
    if entries.is_empty() {
        lines.push("No enabled recipes or dry-run checkpoints.".to_string());
        return lines.join("\n");
    }
    for (recipe, enabled, last_dry_run, last_run) in entries {
        lines.push(format!("- {}", recipe.title));
        lines.push(format!("  Enabled: {}", if enabled { "yes" } else { "no" }));
        lines.push(format!(
            "  Last run: {}",
            last_run.unwrap_or_else(|| "not recorded".to_string())
        ));
        lines.push(format!(
            "  Last dry run: {}",
            last_dry_run.unwrap_or_else(|| "not recorded".to_string())
        ));
        lines.push(format!("  Network impact: {}", recipe.network_impact));
        lines.push(format!("  File impact: {}", recipe.file_impact));
    }
    lines.join("\n")
}

fn file_extension(filename: &str) -> String {
    filename
        .rsplit(['/', '\\'])
        .next()
        .and_then(|name| name.rsplit_once('.').map(|(_, extension)| extension))
        .unwrap_or_default()
        .to_lowercase()
}

fn search_response_files(response: &serde_json::Value) -> Vec<serde_json::Value> {
    ["files", "lockedFiles", "locked_files"]
        .iter()
        .flat_map(|key| {
            response
                .get(*key)
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .collect()
}

fn search_file_name(file: &serde_json::Value) -> String {
    json_track_field(file, &["filename", "fileName", "path"])
}

fn search_file_size(file: &serde_json::Value) -> u64 {
    file.get("size")
        .or_else(|| file.get("bytes"))
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
}

fn search_file_number(file: &serde_json::Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|key| file.get(*key).and_then(|value| value.as_u64()))
        .unwrap_or_default()
}

fn search_tokens(value: &str) -> Vec<String> {
    unique_nonempty(
        value
            .to_lowercase()
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == ' ' {
                    character
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .filter(|token| token.len() > 2)
            .map(ToOwned::to_owned)
            .collect(),
    )
}

fn is_lossless_extension(extension: &str) -> bool {
    matches!(
        extension,
        "aif" | "aiff" | "alac" | "ape" | "flac" | "wav" | "wv"
    )
}

fn is_lossy_extension(extension: &str) -> bool {
    matches!(extension, "aac" | "m4a" | "mp3" | "ogg" | "opus" | "wma")
}

fn is_artwork_extension(extension: &str) -> bool {
    matches!(extension, "gif" | "jpeg" | "jpg" | "png" | "webp")
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn push_unique_reason(reasons: &mut Vec<String>, reason: impl Into<String>) {
    let reason = reason.into();
    if !reason.is_empty() && !reasons.iter().any(|item| item == &reason) {
        reasons.push(reason);
    }
}

pub fn rank_search_candidate(
    response: &serde_json::Value,
    search_text: &str,
    acquisition_profile: &str,
    download_stats: Option<&serde_json::Value>,
    community_quality_summary: Option<&serde_json::Value>,
    preferred_conditions: Option<&serde_json::Value>,
) -> SearchCandidateRank {
    let files = search_response_files(response);
    let mut reasons = Vec::new();
    let mut score: i64 = 0;

    let tokens = search_tokens(search_text);
    if !tokens.is_empty() && !files.is_empty() {
        let best = files
            .iter()
            .map(|file| {
                let filename = search_file_name(file).to_lowercase();
                let matched = tokens
                    .iter()
                    .filter(|token| filename.contains(token.as_str()))
                    .count();
                matched as f64 / tokens.len() as f64
            })
            .fold(0.0_f64, f64::max);
        score += (best * 18.0).round() as i64;
        if best >= 0.8 {
            push_unique_reason(&mut reasons, "strong filename match");
        } else if best >= 0.45 {
            push_unique_reason(&mut reasons, "partial filename match");
        } else {
            push_unique_reason(&mut reasons, "weak filename match");
        }
    }

    let media_files = files
        .iter()
        .filter(|file| !is_artwork_extension(&file_extension(&search_file_name(file))))
        .collect::<Vec<_>>();
    if media_files.is_empty() {
        push_unique_reason(&mut reasons, "no media files visible");
    } else {
        let lossless_count = media_files
            .iter()
            .filter(|file| {
                is_lossless_extension(&file_extension(&search_file_name(file)))
                    || (file.get("bitDepth").is_some() && file.get("sampleRate").is_some())
            })
            .count();
        let high_bitrate_lossy_count = media_files
            .iter()
            .filter(|file| {
                is_lossy_extension(&file_extension(&search_file_name(file)))
                    && search_file_number(file, &["bitRate", "bitrate"]) >= 256
            })
            .count();
        let lossless_ratio = lossless_count as f64 / media_files.len() as f64;
        let high_bitrate_ratio = high_bitrate_lossy_count as f64 / media_files.len() as f64;
        match acquisition_profile {
            "fast-good-enough" => {
                score += (lossless_ratio * 12.0 + high_bitrate_ratio * 16.0).round() as i64;
                if lossless_count > 0 {
                    push_unique_reason(&mut reasons, "lossless fast-good-enough candidate");
                } else if high_bitrate_lossy_count > 0 {
                    push_unique_reason(&mut reasons, "high bitrate fast-good-enough candidate");
                } else {
                    push_unique_reason(&mut reasons, "limited fast-good-enough quality evidence");
                }
            }
            "album-complete" => {
                score += (lossless_ratio * 14.0).round() as i64
                    + clamp_i64(media_files.len() as i64, 0, 18);
                push_unique_reason(
                    &mut reasons,
                    if media_files.len() >= 8 {
                        "broad folder candidate"
                    } else {
                        "small folder candidate"
                    },
                );
            }
            _ => {
                score += (lossless_ratio * 28.0 + high_bitrate_ratio * 6.0).round() as i64;
                if lossless_ratio >= 0.8 {
                    push_unique_reason(&mut reasons, "mostly lossless files");
                } else if lossless_ratio > 0.0 {
                    push_unique_reason(&mut reasons, "mixed lossless files");
                } else {
                    push_unique_reason(&mut reasons, "no lossless signal");
                }
            }
        }
    }

    let audio_files = files
        .iter()
        .filter(|file| {
            let extension = file_extension(&search_file_name(file));
            is_lossless_extension(&extension) || is_lossy_extension(&extension)
        })
        .collect::<Vec<_>>();
    if !audio_files.is_empty() {
        let plausible = audio_files
            .iter()
            .filter(|file| {
                let size = search_file_size(file);
                let length = search_file_number(file, &["length", "duration"]);
                let extension = file_extension(&search_file_name(file));
                if is_lossless_extension(&extension) {
                    (8_000_000..=250_000_000).contains(&size)
                } else if length > 0 {
                    size >= (length * 8).min(2_000_000) && size <= 80_000_000
                } else {
                    (1_000_000..=80_000_000).contains(&size)
                }
            })
            .count();
        let ratio = plausible as f64 / audio_files.len() as f64;
        score += (ratio * 9.0).round() as i64;
        push_unique_reason(
            &mut reasons,
            if ratio >= 0.8 {
                "plausible file sizes"
            } else {
                "mixed file size evidence"
            },
        );
    }

    if response
        .get("hasFreeUploadSlot")
        .or_else(|| response.get("has_free_upload_slot"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        score += 12;
        push_unique_reason(&mut reasons, "free upload slot");
    } else {
        push_unique_reason(&mut reasons, "queued upload");
    }
    let queue_length = response
        .get("queueLength")
        .or_else(|| response.get("queue_length"))
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    score += clamp_i64(10 - queue_length * 2, 0, 10);
    if queue_length <= 1 {
        push_unique_reason(&mut reasons, "short queue");
    } else if queue_length >= 5 {
        push_unique_reason(&mut reasons, "long queue");
    }
    let upload_speed = response
        .get("uploadSpeed")
        .or_else(|| response.get("upload_speed"))
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    score += clamp_i64(
        ((upload_speed as f64 / 5_242_880.0) * 10.0).round() as i64,
        0,
        10,
    );
    if upload_speed >= 2_097_152 {
        push_unique_reason(&mut reasons, "fast peer");
    }

    if let Some(stats) = download_stats {
        let successes = stats
            .get("successfulDownloads")
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let failures = stats
            .get("failedDownloads")
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let history_points = clamp_i64(successes * 2 - failures * 3, -9, 10);
        score += history_points;
        if history_points >= 5 {
            push_unique_reason(&mut reasons, "trusted download history");
        } else if history_points < 0 {
            push_unique_reason(&mut reasons, "poor download history");
        } else {
            push_unique_reason(&mut reasons, "limited download history");
        }
    }

    if response
        .get("sourceProviders")
        .and_then(|value| value.as_array())
        .is_some_and(|providers| providers.iter().any(|value| value == "local"))
    {
        score += 8;
        push_unique_reason(&mut reasons, "local source available");
    } else if response
        .get("sourceProviders")
        .and_then(|value| value.as_array())
        .is_some_and(|providers| {
            providers
                .iter()
                .any(|value| value == "mesh" || value == "pod")
        })
    {
        score += 5;
        push_unique_reason(&mut reasons, "mesh source available");
    }

    if let Some(summary) = community_quality_summary {
        let quality_score = summary
            .get("score")
            .and_then(|value| value.as_i64())
            .unwrap_or_default();
        let override_mode = summary
            .get("override")
            .and_then(|value| value.get("mode"))
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        match override_mode {
            "ignore" => push_unique_reason(&mut reasons, "local quality signals ignored"),
            "trust" => {
                score += 8;
                push_unique_reason(&mut reasons, "local trust override");
            }
            "caution" => {
                score -= 6;
                push_unique_reason(&mut reasons, "local caution override");
            }
            _ if quality_score >= 8 => {
                score += quality_score.min(10);
                push_unique_reason(&mut reasons, "positive local quality signals");
            }
            _ if quality_score <= -6 => {
                score += quality_score.max(-15);
                push_unique_reason(&mut reasons, "local caution signals");
            }
            _ if quality_score != 0 => {
                score += quality_score;
                push_unique_reason(&mut reasons, "mixed local quality signals");
            }
            _ => {}
        }
    }

    if let Some(preferred) = preferred_conditions {
        if preferred
            .get("preferLossless")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            && !files.is_empty()
        {
            let lossless = files
                .iter()
                .filter(|file| is_lossless_extension(&file_extension(&search_file_name(file))))
                .count();
            if lossless > 0 {
                score += ((lossless as f64 / files.len() as f64) * 12.0)
                    .round()
                    .min(12.0) as i64;
                push_unique_reason(&mut reasons, "preferred lossless match");
            } else {
                score -= 6;
                push_unique_reason(&mut reasons, "missing preferred lossless files");
            }
        }
        if let Some(extensions) = preferred
            .get("preferExtensions")
            .and_then(|value| value.as_array())
        {
            if !extensions.is_empty() && !files.is_empty() {
                let matching = files
                    .iter()
                    .filter(|file| {
                        let extension = file_extension(&search_file_name(file));
                        extensions
                            .iter()
                            .any(|value| value.as_str() == Some(extension.as_str()))
                    })
                    .count();
                if matching > 0 {
                    score += ((matching as f64 / files.len() as f64) * 10.0)
                        .round()
                        .min(10.0) as i64;
                    push_unique_reason(&mut reasons, "preferred extension match");
                } else {
                    score -= 4;
                    push_unique_reason(&mut reasons, "missing preferred extension");
                }
            }
        }
        let min_bitrate = preferred
            .get("preferMinBitRate")
            .and_then(|value| value.as_u64())
            .unwrap_or_default();
        if min_bitrate > 0 && !files.is_empty() {
            let matching = files
                .iter()
                .filter(|file| search_file_number(file, &["bitRate", "bitrate"]) >= min_bitrate)
                .count();
            if matching > 0 {
                score += ((matching as f64 / files.len() as f64) * 8.0)
                    .round()
                    .min(8.0) as i64;
                push_unique_reason(&mut reasons, "preferred bitrate match");
            } else {
                score -= 3;
                push_unique_reason(&mut reasons, "below preferred bitrate");
            }
        }
    }

    reasons.truncate(9);
    SearchCandidateRank {
        reasons,
        score: clamp_i64(score, 0, 100) as u32,
    }
}

fn search_provider_labels(response: &serde_json::Value) -> Vec<String> {
    let mut providers = response
        .get("sourceProviders")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(primary) = response
        .get("primarySource")
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        providers.push(primary);
    }
    if providers.is_empty() {
        providers.push("soulseek".to_string());
    }
    providers.sort();
    providers.dedup();
    providers
}

fn search_response_signature(response: &serde_json::Value) -> String {
    let mut media = search_response_files(response)
        .iter()
        .filter_map(|file| {
            let filename = search_file_name(file);
            let extension = file_extension(&filename);
            if !(is_lossless_extension(&extension) || is_lossy_extension(&extension)) {
                return None;
            }
            let basename = filename
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(filename.as_str())
                .to_lowercase();
            Some((basename, search_file_size(file)))
        })
        .collect::<Vec<_>>();
    if media.is_empty() {
        return String::new();
    }
    media.sort_by(|left, right| left.0.cmp(&right.0));
    let total_size = media.iter().map(|(_, size)| *size).sum::<u64>();
    let mut parts = vec![
        media.len().to_string(),
        ((total_size as f64 / 1_000_000.0).round() as u64).to_string(),
    ];
    parts.extend(
        media
            .iter()
            .take(20)
            .map(|(name, size)| format!("{name}:{}", (size + 5_000) / 10_000)),
    );
    parts.join("|")
}

pub fn deduplicate_search_response_groups(
    responses: &[serde_json::Value],
    enabled: bool,
) -> (usize, Vec<SearchDuplicateGroup>) {
    if !enabled || responses.is_empty() {
        return (0, Vec::new());
    }
    let mut groups = std::collections::BTreeMap::<String, Vec<&serde_json::Value>>::new();
    for response in responses {
        let key = search_response_signature(response);
        if !key.is_empty() {
            groups.entry(key).or_default().push(response);
        }
    }
    let mut folded = 0;
    let duplicate_groups = groups
        .into_iter()
        .filter_map(|(key, group)| {
            if group.len() <= 1 {
                return None;
            }
            folded += group.len() - 1;
            let providers = unique_nonempty_case_insensitive(
                group
                    .iter()
                    .flat_map(|response| search_provider_labels(response))
                    .collect(),
            );
            let mut usernames = unique_nonempty_case_insensitive(
                group
                    .iter()
                    .map(|response| json_track_field(response, &["username"]))
                    .collect(),
            );
            usernames.sort();
            Some(SearchDuplicateGroup {
                candidate_count: group.len(),
                folded_count: group.len() - 1,
                key,
                providers,
                usernames,
            })
        })
        .collect::<Vec<_>>();
    (folded, duplicate_groups)
}

pub fn build_search_action_preview(
    response: &serde_json::Value,
    files: &[serde_json::Value],
    candidate_rank: Option<&SearchCandidateRank>,
    community_quality_summary: Option<&serde_json::Value>,
    route: &str,
) -> SearchActionPreview {
    let locked_count = files
        .iter()
        .filter(|file| {
            file.get("locked")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .count();
    let mut warnings = Vec::new();
    if locked_count > 0 {
        push_unique_reason(
            &mut warnings,
            format!(
                "{locked_count} selected file{} may be locked",
                if locked_count == 1 { "" } else { "s" }
            ),
        );
    }
    if !response
        .get("hasFreeUploadSlot")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        push_unique_reason(&mut warnings, "No free upload slot is currently advertised");
    }
    let queue_length = response
        .get("queueLength")
        .and_then(|value| value.as_u64())
        .unwrap_or_default();
    if queue_length >= 5 {
        push_unique_reason(&mut warnings, format!("Queue depth is {queue_length}"));
    }
    if community_quality_summary
        .and_then(|summary| summary.get("score"))
        .and_then(|value| value.as_i64())
        .unwrap_or_default()
        <= -6
    {
        push_unique_reason(&mut warnings, "Local caution signals exist for this peer");
    }
    if let Some(note) = community_quality_summary
        .and_then(|summary| summary.get("override"))
        .and_then(|value| value.get("note"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
    {
        push_unique_reason(&mut warnings, format!("Local quality note: {note}"));
    }
    if community_quality_summary
        .and_then(|summary| summary.get("override"))
        .and_then(|value| value.get("mode"))
        .and_then(|value| value.as_str())
        == Some("ignore")
    {
        push_unique_reason(
            &mut warnings,
            "Local quality signals are ignored by reviewer override",
        );
    }
    if let Some(rank) = candidate_rank {
        if rank.score > 0 && rank.score < 45 {
            push_unique_reason(
                &mut warnings,
                format!("Candidate score is {}/100", rank.score),
            );
        }
    }
    SearchActionPreview {
        candidate_score: candidate_rank.map(|rank| rank.score),
        file_count: files.len(),
        filenames: files.iter().map(search_file_name).collect(),
        locked_count,
        provider_labels: search_provider_labels(response),
        route: route.to_string(),
        total_size_bytes: files.iter().map(search_file_size).sum(),
        username: json_track_field(response, &["username"]),
        warnings,
    }
}

pub fn format_search_action_preview(preview: &SearchActionPreview) -> String {
    let mut lines = vec![
        format!("Action: {}", preview.route),
        format!(
            "Source: {}",
            if preview.username.is_empty() {
                "unknown"
            } else {
                &preview.username
            }
        ),
        format!("Providers: {}", preview.provider_labels.join(", ")),
        format!("Files: {}", preview.file_count),
        format!("Total bytes: {}", preview.total_size_bytes),
    ];
    if let Some(score) = preview.candidate_score {
        lines.push(format!("Candidate score: {score}/100"));
    }
    if !preview.warnings.is_empty() {
        lines.push("Warnings:".to_string());
        lines.extend(
            preview
                .warnings
                .iter()
                .map(|warning| format!("- {warning}")),
        );
    }
    lines.push("Selected files:".to_string());
    lines.extend(
        preview
            .filenames
            .iter()
            .map(|filename| format!("- {filename}")),
    );
    lines.join("\n")
}

pub fn search_planner_report(
    search_text: &str,
    acquisition_profile: &str,
    fold_duplicates: bool,
) -> String {
    let response = serde_json::json!({
        "files": [
            {
                "bitDepth": 16,
                "filename": "Archive Artist/Open Sessions/01 Public Domain Theme.flac",
                "sampleRate": 44100,
                "size": 24000000
            }
        ],
        "hasFreeUploadSlot": true,
        "queueLength": 0,
        "sourceProviders": ["soulseek"],
        "uploadSpeed": 4000000,
        "username": "archive-peer"
    });
    let rank = rank_search_candidate(
        &response,
        search_text,
        acquisition_profile,
        Some(&serde_json::json!({"successfulDownloads": 2, "failedDownloads": 0})),
        None,
        Some(&serde_json::json!({"preferLossless": acquisition_profile == "lossless-exact"})),
    );
    let preview = build_search_action_preview(
        &response,
        &search_response_files(&response),
        Some(&rank),
        None,
        "download",
    );
    let (folded, _) = deduplicate_search_response_groups(&[response], fold_duplicates);
    format!(
        "Search planner\nQuery: {}\nProfile: {}\nDuplicate folding: {}\nFolded duplicates: {}\nScore: {}/100\nReasons: {}\n\n{}",
        search_text,
        acquisition_profile,
        fold_duplicates,
        folded,
        rank.score,
        rank.reasons.join(", "),
        format_search_action_preview(&preview)
    )
}

fn json_track_field(track: &serde_json::Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            track
                .get(*key)
                .map(json_scalar_preview)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn unique_nonempty(values: Vec<String>) -> Vec<String> {
    values.into_iter().filter(|value| !value.is_empty()).fold(
        Vec::<String>::new(),
        |mut unique, value| {
            if !unique.iter().any(|other| other == &value) {
                unique.push(value);
            }
            unique
        },
    )
}

fn unique_nonempty_case_insensitive(values: Vec<String>) -> Vec<String> {
    values.into_iter().filter(|value| !value.is_empty()).fold(
        Vec::<String>::new(),
        |mut unique, value| {
            if !unique
                .iter()
                .any(|other| other.eq_ignore_ascii_case(&value))
            {
                unique.push(value);
            }
            unique
        },
    )
}

fn player_radio_tags(track: &serde_json::Value) -> Vec<String> {
    for key in ["tags", "genres"] {
        let values = track
            .get(key)
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(json_scalar_preview)
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !values.is_empty() {
            return values;
        }
    }
    json_track_field(track, &["genre"])
        .split('\n')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn build_player_radio_plan(track: Option<&serde_json::Value>) -> PlayerRadioPlan {
    let Some(track) = track else {
        return PlayerRadioPlan {
            basis: Vec::new(),
            primary_query: String::new(),
            queries: Vec::new(),
            ready: false,
            seed_label: "No track selected".to_string(),
        };
    };

    let artist = json_track_field(track, &["artist"]);
    let title = json_track_field(track, &["title", "fileName", "filename"]);
    let album = json_track_field(track, &["album"]);
    let tags = unique_nonempty(player_radio_tags(track));
    let track_query = unique_nonempty(vec![artist.clone(), title.clone()]).join(" ");
    let album_query = unique_nonempty(vec![artist.clone(), album.clone()]).join(" ");
    let genre_query = unique_nonempty(vec![
        artist.clone(),
        tags.first().cloned().unwrap_or_default(),
    ])
    .join(" ");
    let artist_query = artist.clone();
    let queries = unique_nonempty(vec![
        track_query.clone(),
        album_query.clone(),
        genre_query.clone(),
        artist_query,
    ])
    .into_iter()
    .enumerate()
    .map(|(index, query)| {
        let reason = if query == track_query {
            "Similar track seed"
        } else if query == album_query {
            "Album neighborhood"
        } else if query == genre_query {
            "Artist and genre seed"
        } else {
            "Artist radio seed"
        };
        PlayerRadioQuery {
            id: format!("radio-query-{}", index + 1),
            query,
            reason,
        }
    })
    .collect::<Vec<_>>();
    let seed_label = unique_nonempty(vec![artist.clone(), title.clone()]).join(" - ");
    PlayerRadioPlan {
        basis: vec![
            artist
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Artist: {artist}")),
            title
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Track: {title}")),
            album
                .is_empty()
                .then(String::new)
                .unwrap_or_else(|| format!("Album: {album}")),
            tags.is_empty().then(String::new).unwrap_or_else(|| {
                format!(
                    "Tags: {}",
                    tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                )
            }),
        ]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect(),
        primary_query: queries
            .first()
            .map(|query| query.query.clone())
            .unwrap_or_default(),
        ready: !queries.is_empty(),
        queries,
        seed_label: if !seed_label.is_empty() {
            seed_label
        } else if !title.is_empty() {
            title
        } else if !artist.is_empty() {
            artist
        } else {
            "Untitled seed".to_string()
        },
    }
}

fn percent_encode_query(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (*byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn build_player_radio_search_path(query: &str) -> String {
    let normalized = query.trim();
    if normalized.is_empty() {
        "/searches".to_string()
    } else {
        format!("/searches?q={}", percent_encode_query(normalized))
    }
}

pub fn player_radio_queries(plan: &PlayerRadioPlan, limit: usize) -> Vec<String> {
    unique_nonempty_case_insensitive(
        plan.queries
            .iter()
            .map(|item| item.query.clone())
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(limit)
    .collect()
}

fn quote_if_needed(value: &str) -> String {
    let normalized = value.trim();
    if normalized.is_empty() {
        String::new()
    } else if normalized.chars().any(char::is_whitespace) {
        format!("\"{normalized}\"")
    } else {
        normalized.to_string()
    }
}

pub fn player_radio_copy_text(plan: &PlayerRadioPlan) -> String {
    if !plan.ready {
        return String::new();
    }
    let mut lines = vec![format!("Smart radio seed: {}", plan.seed_label)];
    lines.extend(
        plan.queries
            .iter()
            .map(|item| format!("{}: {}", item.reason, quote_if_needed(&item.query))),
    );
    lines.join("\n")
}

fn player_auto_queue_tags(item: &serde_json::Value) -> Vec<String> {
    ["tags", "genres"]
        .iter()
        .flat_map(|key| {
            item.get(*key)
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .map(|value| json_scalar_preview(&value).trim().to_lowercase())
        .chain(std::iter::once(
            json_track_field(item, &["genre"]).to_lowercase(),
        ))
        .filter(|value| !value.is_empty())
        .collect()
}

fn player_title_tokens(item: &serde_json::Value) -> Vec<String> {
    json_track_field(item, &["title", "fileName", "filename"])
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == ' ' {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .map(ToOwned::to_owned)
        .collect()
}

pub fn player_similarity_score(current: &serde_json::Value, candidate: &serde_json::Value) -> u32 {
    let mut score = 0;
    let current_artist = json_track_field(current, &["artist"]).to_lowercase();
    let candidate_artist = json_track_field(candidate, &["artist"]).to_lowercase();
    if !current_artist.is_empty() && current_artist == candidate_artist {
        score += 4;
    }
    let current_album = json_track_field(current, &["album"]).to_lowercase();
    let candidate_album = json_track_field(candidate, &["album"]).to_lowercase();
    if !current_album.is_empty() && current_album == candidate_album {
        score += 3;
    }

    let current_tags = player_auto_queue_tags(current);
    let shared_tags = player_auto_queue_tags(candidate)
        .iter()
        .filter(|tag| current_tags.iter().any(|current_tag| current_tag == *tag))
        .count() as u32;
    score += (shared_tags * 2).min(4);

    let current_tokens = player_title_tokens(current);
    let shared_title_tokens = player_title_tokens(candidate)
        .iter()
        .filter(|token| {
            current_tokens
                .iter()
                .any(|current_token| current_token == *token)
        })
        .count() as u32;
    score += shared_title_tokens.min(2);
    score
}

pub fn build_similar_queue_candidates(
    current: Option<&serde_json::Value>,
    history: &[serde_json::Value],
    queue: &[serde_json::Value],
    limit: usize,
) -> Vec<SimilarQueueCandidate> {
    let Some(current) = current else {
        return Vec::new();
    };
    let mut seen = queue
        .iter()
        .filter_map(|item| {
            item.get("contentId")
                .or_else(|| item.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())
        })
        .collect::<Vec<_>>();
    let mut candidates = history
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let content_id = item
                .get("contentId")
                .or_else(|| item.get("content_id"))
                .map(json_scalar_preview)
                .filter(|value| !value.is_empty())?;
            if seen.iter().any(|seen_id| seen_id == &content_id) {
                return None;
            }
            let score = player_similarity_score(current, item);
            if score == 0 {
                return None;
            }
            seen.push(content_id);
            Some(SimilarQueueCandidate {
                index,
                item: item.clone(),
                score,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.index.cmp(&right.index))
    });
    candidates.truncate(limit);
    candidates
}

pub fn similar_queue_search_queries(
    candidates: &[SimilarQueueCandidate],
    limit: usize,
) -> Vec<String> {
    unique_nonempty_case_insensitive(
        candidates
            .iter()
            .map(|candidate| {
                unique_nonempty(vec![
                    json_track_field(&candidate.item, &["artist"]),
                    json_track_field(&candidate.item, &["title", "fileName", "filename"]),
                ])
                .join(" ")
            })
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .take(limit)
    .collect()
}

pub fn player_radio_query_from_now_playing_body(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| current_player_track(&value))
        .map(|track| build_player_radio_plan(Some(&track)).primary_query)
        .unwrap_or_default()
}

fn current_player_track(value: &serde_json::Value) -> Option<serde_json::Value> {
    value
        .get("now_playing")
        .and_then(|entry| entry.as_array())
        .and_then(|items| items.first())
        .or_else(|| value.get("current"))
        .or_else(|| value.get("track"))
        .or(Some(value))
        .cloned()
}

#[cfg(target_arch = "wasm32")]
fn player_ratings_storage(window: &web_sys::Window) -> Option<web_sys::Storage> {
    window.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn read_player_rating(window: &web_sys::Window, key: &str) -> u32 {
    if key.is_empty() {
        return 0;
    }
    player_ratings_storage(window)
        .and_then(|storage| storage.get_item("slskr.player.ratings").ok().flatten())
        .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
        .and_then(|value| value.get(key).and_then(|rating| rating.as_u64()))
        .and_then(|rating| u32::try_from(rating).ok())
        .filter(|rating| (1..=5).contains(rating))
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn write_player_rating(window: &web_sys::Window, key: &str, rating: u32) {
    if key.is_empty() {
        return;
    }
    let Some(storage) = player_ratings_storage(window) else {
        return;
    };
    let mut ratings = storage
        .get_item("slskr.player.ratings")
        .ok()
        .flatten()
        .and_then(|body| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body).ok()
        })
        .unwrap_or_default();
    if (1..=5).contains(&rating) {
        ratings.insert(key.to_string(), serde_json::Value::from(rating));
    } else {
        ratings.remove(key);
    }
    let _ = storage.set_item(
        "slskr.player.ratings",
        &serde_json::Value::Object(ratings).to_string(),
    );
}

#[cfg(target_arch = "wasm32")]
fn update_player_rating_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
    rating_key: &str,
) {
    let rating = read_player_rating(window, rating_key);
    if let Ok(Some(player)) = document.query_selector("[data-slskr-player]") {
        let _ = player.set_attribute("data-slskr-player-rating-key", rating_key);
    }
    if let Ok(buttons) = document.query_selector_all("[data-slskr-player-rating]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let Ok(button) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let value = button
                .get_attribute("data-slskr-player-rating")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or_default();
            let class = if value <= rating && rating > 0 {
                "is-active"
            } else {
                ""
            };
            let _ = button.set_attribute("class", class);
        }
    }
    if let Some(status) = document.get_element_by_id("slskr-player-rating-status") {
        status.set_text_content(Some(player_rating_summary(rating)));
    }
}

#[cfg(target_arch = "wasm32")]
fn update_player_radio_controls(document: &web_sys::Document, query: &str) {
    if let Ok(Some(player)) = document.query_selector("[data-slskr-player]") {
        let _ = player.set_attribute("data-slskr-player-radio-query", query);
    }
    if let Some(status) = document.get_element_by_id("slskr-player-radio") {
        status.set_text_content(Some(if query.is_empty() {
            "No track selected"
        } else {
            query
        }));
    }
}

#[cfg(target_arch = "wasm32")]
fn open_player_radio_search(window: &web_sys::Window, document: &web_sys::Document) {
    let query = document
        .query_selector("[data-slskr-player]")
        .ok()
        .flatten()
        .and_then(|player| player.get_attribute("data-slskr-player-radio-query"))
        .unwrap_or_default();
    if query.trim().is_empty() {
        set_player_status(document, "No track selected");
        return;
    }
    let path = build_player_radio_search_path(&query);
    if let Ok(history) = window.history() {
        let _ = history.push_state_with_url(&JsValue::NULL, "", Some(&path));
    }
    let _ = render_current_route(window, document);
    if let Ok(Some(input)) = document.query_selector(".slskr-toolbar-input") {
        if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
            input.set_value(&query);
            let _ = input.focus();
            let _ = input.select();
        }
    }
    set_player_status(document, &format!("Smart radio search ready: {query}"));
}

#[cfg(target_arch = "wasm32")]
fn player_audio_element(document: &web_sys::Document) -> Option<web_sys::HtmlAudioElement> {
    document
        .get_element_by_id("slskr-player-audio")
        .and_then(|element| element.dyn_into::<web_sys::HtmlAudioElement>().ok())
}

#[cfg(target_arch = "wasm32")]
fn toggle_player_audio(document: &web_sys::Document) {
    let Some(audio) = player_audio_element(document) else {
        set_player_status(document, "Player audio element unavailable");
        return;
    };
    if audio.get_attribute("src").unwrap_or_default().is_empty() {
        set_player_status(document, "No stream loaded");
        return;
    }
    if audio.paused() {
        let _ = audio.play();
        set_player_status(document, "Playback requested");
    } else {
        let _ = audio.pause();
        set_player_status(document, "Playback paused");
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_player_controls(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all("[data-slskr-player-action]")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let action = button
            .get_attribute("data-slskr-player-action")
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let action = action.clone();
                let window = window.clone();
                let document = document.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    set_player_status(&document, "Player action running");
                    if action == "play" {
                        toggle_player_audio(&document);
                        return;
                    }
                    if action == "radio" {
                        open_player_radio_search(&window, &document);
                        return;
                    }
                    if action == "visualizer" {
                        match toggle_rustymilk_visualizer(&window, &document) {
                            Ok(()) => set_player_status(&document, "RustyMilk visualizer ready"),
                            Err(error) => set_player_status(
                                &document,
                                &error
                                    .as_string()
                                    .unwrap_or_else(|| "RustyMilk visualizer failed".to_string()),
                            ),
                        }
                        return;
                    }
                    let result = match action.as_str() {
                        "clear" => {
                            fetch_text_with_method(
                                &window,
                                &endpoint_url("/nowplaying"),
                                "DELETE",
                                None,
                            )
                            .await
                        }
                        _ => Ok(String::new()),
                    };
                    match result {
                        Ok(body) if !body.is_empty() => {
                            set_player_status(&document, &compact_preview(&body));
                        }
                        Ok(_) => set_player_status(&document, "Player refreshed"),
                        Err(error) => {
                            let message = error
                                .as_string()
                                .unwrap_or_else(|| "player request failed".to_string());
                            set_player_status(&document, &message);
                        }
                    }
                    let _ = refresh_player_status(&window).await;
                    set_player_status(
                        &document,
                        if action == "clear" {
                            "Player cleared"
                        } else {
                            "Player refreshed"
                        },
                    );
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    let rating_buttons = document.query_selector_all("[data-slskr-player-rating]")?;
    for index in 0..rating_buttons.length() {
        let Some(node) = rating_buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let value = button
            .get_attribute("data-slskr-player-rating")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default();
        let window = window.clone();
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let key = document
                    .query_selector("[data-slskr-player]")
                    .ok()
                    .flatten()
                    .and_then(|player| player.get_attribute("data-slskr-player-rating-key"))
                    .unwrap_or_default();
                if key.is_empty() {
                    set_player_status(&document, "No track selected");
                    return;
                }
                let current = read_player_rating(&window, &key);
                let next = if current == value { 0 } else { value };
                write_player_rating(&window, &key, next);
                update_player_rating_controls(&window, &document, &key);
                set_player_status(&document, player_rating_summary(next));
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn set_player_status(document: &web_sys::Document, message: &str) {
    if let Some(status) = document.get_element_by_id("slskr-player-status") {
        status.set_text_content(Some(message));
    }
}

#[cfg(target_arch = "wasm32")]
fn update_data_card_count(card: &web_sys::Element) {
    let Ok(Some(count)) = card.query_selector("[data-slskr-card-count]") else {
        return;
    };
    let Ok(rows) = card.query_selector_all(".slskr-record-list [data-slskr-row-text]") else {
        return;
    };
    let total = rows.length();
    let mut visible = 0;
    for row_index in 0..rows.length() {
        let Some(row) = rows.item(row_index) else {
            continue;
        };
        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if !row.has_attribute("hidden") {
            visible += 1;
        }
    }
    count.set_text_content(Some(&format!("{visible} / {total}")));
}

#[cfg(target_arch = "wasm32")]
fn sort_data_card_table(card: &web_sys::Element, button: &web_sys::Element) {
    let column = button
        .get_attribute("data-slskr-sort-index")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_default();
    let next_direction = match button.get_attribute("data-slskr-sort-direction").as_deref() {
        Some("asc") => "desc",
        _ => "asc",
    };
    let Ok(Some(tbody)) = card.query_selector(".slskr-data-table tbody") else {
        return;
    };
    let Ok(rows) = tbody.query_selector_all("tr") else {
        return;
    };
    let mut elements = Vec::new();
    for row_index in 0..rows.length() {
        let Some(row) = rows.item(row_index) else {
            continue;
        };
        let Ok(row) = row.dyn_into::<web_sys::Element>() else {
            continue;
        };
        elements.push(row);
    }
    elements.sort_by(|left, right| {
        let left_value = table_cell_text(left, column);
        let right_value = table_cell_text(right, column);
        if next_direction == "asc" {
            left_value.cmp(&right_value)
        } else {
            right_value.cmp(&left_value)
        }
    });
    if let Ok(buttons) = card.query_selector_all("[data-slskr-sort-index]") {
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("data-slskr-sort-direction");
            let _ = element.remove_attribute("aria-sort");
        }
    }
    let _ = button.set_attribute("data-slskr-sort-direction", next_direction);
    let _ = button.set_attribute(
        "aria-sort",
        if next_direction == "asc" {
            "ascending"
        } else {
            "descending"
        },
    );
    for row in elements {
        let _ = tbody.append_child(&row);
    }
}

#[cfg(target_arch = "wasm32")]
fn table_cell_text(row: &web_sys::Element, column: u32) -> String {
    let selector = format!("td:nth-child({})", column + 1);
    row.query_selector(&selector)
        .ok()
        .flatten()
        .and_then(|cell| cell.text_content())
        .unwrap_or_default()
        .to_lowercase()
}

#[cfg(target_arch = "wasm32")]
fn select_data_card_record(card: &web_sys::Element, row: &web_sys::Element) {
    if let Ok(rows) = card.query_selector_all("[data-slskr-record-select]") {
        for index in 0..rows.length() {
            let Some(node) = rows.item(index) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                continue;
            };
            let _ = element.remove_attribute("aria-selected");
            let _ = element.set_attribute("class", "");
        }
    }
    let _ = row.set_attribute("aria-selected", "true");
    let _ = row.set_attribute("class", "is-selected");

    let title = row
        .get_attribute("data-slskr-record-title")
        .unwrap_or_else(|| "Selected Record".to_string());
    let detail = row
        .get_attribute("data-slskr-record-detail")
        .unwrap_or_default();
    let raw = row
        .get_attribute("data-slskr-record-json")
        .unwrap_or_default();

    if let Ok(Some(header)) = card.query_selector(".slskr-card-inspector h4") {
        header.set_text_content(Some(&title));
    }
    if let Ok(Some(description)) = card.query_selector(".slskr-card-inspector p") {
        description.set_text_content(Some(&detail));
    }
    if let Ok(Some(pre)) = card.query_selector(".slskr-card-inspector pre") {
        pre.set_text_content(Some(&raw));
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_workspace_tabs(document: &web_sys::Document) -> Result<(), JsValue> {
    let tabs = document.query_selector_all(".slskr-workspace-tab")?;
    for index in 0..tabs.length() {
        let Some(node) = tabs.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let document = document.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let Some(target) = event
                    .current_target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };
                let mode = target
                    .get_attribute("data-slskr-workspace-mode")
                    .unwrap_or_else(|| "all".to_string());

                if let Ok(tabs) = document.query_selector_all(".slskr-workspace-tab") {
                    for index in 0..tabs.length() {
                        let Some(node) = tabs.item(index) else {
                            continue;
                        };
                        let Ok(tab) = node.dyn_into::<web_sys::Element>() else {
                            continue;
                        };
                        let active = tab
                            .get_attribute("data-slskr-workspace-mode")
                            .is_some_and(|tab_mode| tab_mode == mode);
                        let class = if active {
                            "slskr-workspace-tab is-active"
                        } else {
                            "slskr-workspace-tab"
                        };
                        let _ = tab.set_attribute("class", class);
                        let _ = tab
                            .set_attribute("aria-selected", if active { "true" } else { "false" });
                    }
                }

                if let Ok(Some(grid)) = document.query_selector("[data-slskr-workspace-grid]") {
                    let class = match mode.as_str() {
                        "primary" => "slskr-workspace-grid mode-primary",
                        "secondary" => "slskr-workspace-grid mode-secondary",
                        _ => "slskr-workspace-grid",
                    };
                    let _ = grid.set_attribute("class", class);
                }
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn storage_json_object(
    window: &web_sys::Window,
    key: &str,
    fallback: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .and_then(|body| {
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body).ok()
        })
        .map(|stored| {
            let mut merged = fallback.clone();
            for (key, value) in stored {
                merged.insert(key, value);
            }
            merged
        })
        .unwrap_or(fallback)
}

#[cfg(target_arch = "wasm32")]
fn write_storage_json_object(
    window: &web_sys::Window,
    key: &str,
    value: &serde_json::Map<String, serde_json::Value>,
) {
    if let Some(storage) = window.local_storage().ok().flatten() {
        let _ = storage.set_item(key, &serde_json::Value::Object(value.clone()).to_string());
    }
}

#[cfg(target_arch = "wasm32")]
fn remove_storage_item(window: &web_sys::Window, key: &str) {
    if let Some(storage) = window.local_storage().ok().flatten() {
        let _ = storage.remove_item(key);
    }
}

#[cfg(target_arch = "wasm32")]
fn collect_experience_form(
    document: &web_sys::Document,
) -> serde_json::Map<String, serde_json::Value> {
    let mut values = serde_json::Map::new();
    if let Ok(inputs) = document.query_selector_all("[data-slskr-pref]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.item(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-slskr-pref") else {
                continue;
            };
            let value = if input.type_() == "checkbox" {
                serde_json::Value::Bool(input.checked())
            } else {
                serde_json::Value::String(input.value())
            };
            values.insert(key, value);
        }
    }
    values
}

#[cfg(target_arch = "wasm32")]
fn apply_experience_form(
    document: &web_sys::Document,
    values: &serde_json::Map<String, serde_json::Value>,
) {
    if let Ok(inputs) = document.query_selector_all("[data-slskr-pref]") {
        for index in 0..inputs.length() {
            let Some(node) = inputs.item(index) else {
                continue;
            };
            let Ok(input) = node.dyn_into::<web_sys::HtmlInputElement>() else {
                continue;
            };
            let Some(key) = input.get_attribute("data-slskr-pref") else {
                continue;
            };
            let value = values.get(&key).cloned().unwrap_or_else(|| {
                serde_json::Value::String(
                    input
                        .get_attribute("data-slskr-pref-default")
                        .unwrap_or_default(),
                )
            });
            if input.type_() == "checkbox" {
                input.set_checked(value.as_bool().unwrap_or(false));
            } else {
                input.set_value(&json_scalar_preview(&value));
            }
        }
    }
    let report = experience_preferences_report(values);
    if let Some(output) = document.get_element_by_id("slskr-experience-report") {
        output.set_text_content(Some(&report));
    }
    if let Some(summary) = document.get_element_by_id("slskr-experience-summary") {
        summary.set_text_content(Some("18 preferences"));
    }
}

#[cfg(target_arch = "wasm32")]
fn default_automation_state() -> serde_json::Map<String, serde_json::Value> {
    automation_recipes()
        .iter()
        .map(|recipe| {
            (
                recipe.id.to_string(),
                serde_json::json!({
                    "enabled": recipe.enabled_by_default,
                    "lastDryRunAt": null,
                }),
            )
        })
        .collect()
}

#[cfg(target_arch = "wasm32")]
fn apply_automation_state(
    document: &web_sys::Document,
    state: &serde_json::Map<String, serde_json::Value>,
) {
    for recipe in automation_recipes() {
        let enabled = state
            .get(recipe.id)
            .and_then(|entry| entry.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(recipe.enabled_by_default);
        let selector = format!(r#"[data-slskr-recipe-enabled="{}"]"#, recipe.id);
        if let Ok(Some(input)) = document.query_selector(&selector) {
            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                input.set_checked(enabled);
            }
        }
    }
    let (total, enabled, disabled) = automation_summary_from_state(state);
    if let Some(summary) = document.get_element_by_id("slskr-automation-summary") {
        summary.set_text_content(Some(&format!(
            "{total} recipes / {enabled} enabled / {disabled} disabled"
        )));
    }
    if let Some(report) = document.get_element_by_id("slskr-automation-report") {
        report.set_text_content(Some(&automation_history_report(state)));
    }
}

#[cfg(target_arch = "wasm32")]
fn mount_browser_local_panels(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    if document
        .query_selector("[data-slskr-experience-panel]")?
        .is_some()
    {
        let values = storage_json_object(
            window,
            "slskr:experience-preferences:v1",
            default_experience_preferences(),
        );
        apply_experience_form(document, &values);
        let buttons = document.query_selector_all("[data-slskr-pref-action]")?;
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-pref-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let values = if action == "reset" {
                        remove_storage_item(&window, "slskr:experience-preferences:v1");
                        default_experience_preferences()
                    } else {
                        collect_experience_form(&document)
                    };
                    if action == "save" {
                        write_storage_json_object(
                            &window,
                            "slskr:experience-preferences:v1",
                            &values,
                        );
                    }
                    apply_experience_form(&document, &values);
                    if action == "copy" {
                        let report = experience_preferences_report(&values);
                        copy_reference_text(&window, &document, report);
                    }
                    if let Some(status) = document.get_element_by_id("slskr-experience-status") {
                        let message = match action.as_str() {
                            "copy" => "Experience preference report copied.",
                            "reset" => "Experience preferences reset.",
                            _ => "Experience preferences saved locally.",
                        };
                        status.set_text_content(Some(message));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    if document
        .query_selector("[data-slskr-automation-panel]")?
        .is_some()
    {
        let state = storage_json_object(
            window,
            "slskr.automationRecipeState",
            default_automation_state(),
        );
        apply_automation_state(document, &state);
        let enabled_inputs = document.query_selector_all("[data-slskr-recipe-enabled]")?;
        for index in 0..enabled_inputs.length() {
            let Some(node) = enabled_inputs.item(index) else {
                continue;
            };
            let input: web_sys::HtmlInputElement = node.dyn_into()?;
            let recipe_id = input
                .get_attribute("data-slskr-recipe-enabled")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(
                move |event: web_sys::Event| {
                    let checked = event
                        .current_target()
                        .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(false);
                    let mut state = storage_json_object(
                        &window,
                        "slskr.automationRecipeState",
                        default_automation_state(),
                    );
                    let entry = state
                        .entry(recipe_id.clone())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = entry.as_object_mut() {
                        object.insert("enabled".to_string(), serde_json::Value::Bool(checked));
                    }
                    write_storage_json_object(&window, "slskr.automationRecipeState", &state);
                    apply_automation_state(&document, &state);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some("Automation recipe state saved."));
                    }
                },
            ));
            input.add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let dry_run_buttons = document.query_selector_all("[data-slskr-recipe-dry-run]")?;
        for index in 0..dry_run_buttons.length() {
            let Some(node) = dry_run_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let recipe_id = button
                .get_attribute("data-slskr-recipe-dry-run")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let Some(recipe) = automation_recipes()
                        .iter()
                        .find(|recipe| recipe.id == recipe_id)
                        .copied()
                    else {
                        return;
                    };
                    let report = automation_dry_run_report(recipe, "browser-local");
                    let mut state = storage_json_object(
                        &window,
                        "slskr.automationRecipeState",
                        default_automation_state(),
                    );
                    let entry = state
                        .entry(recipe.id.to_string())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = entry.as_object_mut() {
                        object.insert(
                            "lastDryRunAt".to_string(),
                            serde_json::Value::String("browser-local".to_string()),
                        );
                        object.insert("lastDryRunReport".to_string(), report.clone());
                    }
                    write_storage_json_object(&window, "slskr.automationRecipeState", &state);
                    apply_automation_state(&document, &state);
                    if let Some(output) = document.get_element_by_id("slskr-automation-report") {
                        output.set_text_content(Some(
                            &serde_json::to_string_pretty(&report).unwrap_or_default(),
                        ));
                    }
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status
                            .set_text_content(Some(&format!("{} dry run recorded.", recipe.title)));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let copy_plan_buttons = document.query_selector_all("[data-slskr-recipe-copy]")?;
        for index in 0..copy_plan_buttons.length() {
            let Some(node) = copy_plan_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let recipe_id = button
                .get_attribute("data-slskr-recipe-copy")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let Some(recipe) = automation_recipes()
                        .iter()
                        .find(|recipe| recipe.id == recipe_id)
                        .copied()
                    else {
                        return;
                    };
                    let report = automation_dry_run_report(recipe, "browser-local");
                    let text = serde_json::to_string_pretty(&report).unwrap_or_default();
                    copy_reference_text(&window, &document, text);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some(&format!("{} plan copied.", recipe.title)));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }

        let action_buttons = document.query_selector_all("[data-slskr-automation-action]")?;
        for index in 0..action_buttons.length() {
            let Some(node) = action_buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-automation-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    let state = if action == "reset" {
                        remove_storage_item(&window, "slskr.automationRecipeState");
                        default_automation_state()
                    } else {
                        storage_json_object(
                            &window,
                            "slskr.automationRecipeState",
                            default_automation_state(),
                        )
                    };
                    apply_automation_state(&document, &state);
                    if let Some(status) = document.get_element_by_id("slskr-automation-status") {
                        status.set_text_content(Some(if action == "reset" {
                            "Automation recipe state reset."
                        } else {
                            "Automation history report prepared."
                        }));
                    }
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    if document
        .query_selector("[data-slskr-search-planner]")?
        .is_some()
    {
        let render_search_plan =
            |document: &web_sys::Document, window: &web_sys::Window, message: &str| {
                let query = document
                    .query_selector(r#"[data-slskr-search-setting="query"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_else(|| "public domain theme".to_string());
                let profile = document
                    .query_selector(r#"[data-slskr-search-setting="profile"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_else(|| "lossless-exact".to_string());
                let fold = document
                    .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.checked())
                    .unwrap_or(true);
                if let Some(output) = document.get_element_by_id("slskr-search-planner-report") {
                    output.set_text_content(Some(&search_planner_report(&query, &profile, fold)));
                }
                if let Some(status) = document.get_element_by_id("slskr-search-planner-status") {
                    status.set_text_content(Some(message));
                }
                let mut stored = serde_json::Map::new();
                stored.insert("query".to_string(), serde_json::Value::String(query));
                stored.insert("profile".to_string(), serde_json::Value::String(profile));
                stored.insert("foldDuplicates".to_string(), serde_json::Value::Bool(fold));
                write_storage_json_object(window, "slskr.search.planner", &stored);
            };

        render_search_plan(document, window, "Search planner ready.");
        let buttons = document.query_selector_all("[data-slskr-search-action]")?;
        for index in 0..buttons.length() {
            let Some(node) = buttons.item(index) else {
                continue;
            };
            let button: web_sys::Element = node.dyn_into()?;
            let action = button
                .get_attribute("data-slskr-search-action")
                .unwrap_or_default();
            let window = window.clone();
            let document = document.clone();
            let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
                move |event: web_sys::MouseEvent| {
                    event.prevent_default();
                    if action == "reset" {
                        if let Ok(Some(input)) =
                            document.query_selector(r#"[data-slskr-search-setting="query"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_value("public domain theme");
                            }
                        }
                        if let Ok(Some(input)) =
                            document.query_selector(r#"[data-slskr-search-setting="profile"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_value("lossless-exact");
                            }
                        }
                        if let Ok(Some(input)) = document
                            .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                        {
                            if let Ok(input) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                input.set_checked(true);
                            }
                        }
                        remove_storage_item(&window, "slskr.search.planner");
                    }
                    let query = document
                        .query_selector(r#"[data-slskr-search-setting="query"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_default();
                    let profile = document
                        .query_selector(r#"[data-slskr-search-setting="profile"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_else(|| "lossless-exact".to_string());
                    let fold = document
                        .query_selector(r#"[data-slskr-search-setting="foldDuplicates"]"#)
                        .ok()
                        .flatten()
                        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.checked())
                        .unwrap_or(true);
                    if let Some(output) = document.get_element_by_id("slskr-search-planner-report")
                    {
                        output
                            .set_text_content(Some(&search_planner_report(&query, &profile, fold)));
                    }
                    if let Some(status) = document.get_element_by_id("slskr-search-planner-status")
                    {
                        status.set_text_content(Some(if action == "reset" {
                            "Search planner reset."
                        } else {
                            "Search action preview prepared."
                        }));
                    }
                    let mut stored = serde_json::Map::new();
                    stored.insert("query".to_string(), serde_json::Value::String(query));
                    stored.insert("profile".to_string(), serde_json::Value::String(profile));
                    stored.insert("foldDuplicates".to_string(), serde_json::Value::Bool(fold));
                    write_storage_json_object(&window, "slskr.search.planner", &stored);
                },
            ));
            button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callback.forget();
        }
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_toolbar_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-toolbar-command")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let action_index = button
            .get_attribute("data-slskr-toolbar-action")
            .and_then(|value| value.parse::<usize>().ok());
        let action_label = button.text_content().unwrap_or_default().trim().to_owned();
        if action_index.is_none() && action_label.is_empty() {
            continue;
        }
        let window = window.clone();
        let document = document.clone();
        let button_for_callback = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |event: web_sys::MouseEvent| {
                event.prevent_default();
                let toolbar_values = document
                    .query_selector_all(".slskr-toolbar-input")
                    .ok()
                    .map(|inputs| {
                        (0..inputs.length())
                            .filter_map(|index| {
                                inputs
                                    .item(index)
                                    .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                                    .and_then(|element| form_control_value(&element))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let route_path = window.location().pathname().unwrap_or_default();
                if route_kind(&route_path) == RouteKind::SharedWithMe {
                    let normalized = action_label.to_ascii_lowercase();
                    if matches!(
                        normalized.as_str(),
                        "open" | "open collection" | "backfill" | "leave share"
                    ) && !native_shared_grant_is_selected(
                        &document,
                        &button_for_callback,
                        &action_label,
                    ) {
                        return;
                    }
                    if normalized == "copy token" {
                        native_issue_share_token(&window, &document, &button_for_callback);
                        return;
                    }
                    if normalized == "copy manifest" {
                        native_copy_shared_manifest(&window, &document, &button_for_callback);
                        return;
                    }
                    if normalized == "stream" || normalized == "stream item" {
                        native_stream_shared_manifest(&window, &document, &button_for_callback);
                        return;
                    }
                }
                let Some(action) = route_action_for_native_label(&route_path, &action_label)
                    .or_else(|| action_index.and_then(|index| route_action_at(&route_path, index)))
                else {
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        status.set_inner_html(&format!(
                            "<strong>{}</strong> no executable route contract",
                            escape_html(&action_label)
                        ));
                    }
                    return;
                };
                let target_value =
                    if action.path.contains(":username") || action.path.contains(":roomName") {
                        toolbar_values.first().cloned().unwrap_or_default()
                    } else {
                        String::new()
                    };
                let body_value = match action.body {
                    ActionBody::ConversationMessage | ActionBody::RoomMessage => {
                        toolbar_values.get(1).cloned().unwrap_or_else(|| {
                            native_action_value(&document, &button_for_callback, action.body)
                        })
                    }
                    ActionBody::BrowseDirectory => {
                        toolbar_values.get(1).cloned().unwrap_or_else(|| {
                            native_action_value(&document, &button_for_callback, action.body)
                        })
                    }
                    ActionBody::DownloadFiles => {
                        native_action_value(&document, &button_for_callback, action.body)
                    }
                    _ => toolbar_values.first().cloned().unwrap_or_else(|| {
                        native_action_value(&document, &button_for_callback, action.body)
                    }),
                };
                if !native_route_action_is_ready(
                    &document,
                    &button_for_callback,
                    &route_path,
                    action,
                    &target_value,
                    &body_value,
                ) {
                    return;
                }
                let body = native_action_body(&document, &button_for_callback, action, &body_value);
                let window = window.clone();
                let document = document.clone();
                let method = action.method.to_string();
                let return_to_searches = route_kind(&route_path) == RouteKind::Search
                    && route_path != "/searches"
                    && matches!(action.label, "Clear Searches" | "Remove Search");
                let target = native_action_target_for_ui(
                    &document,
                    &button_for_callback,
                    action,
                    &target_value,
                );
                let id = native_action_id(&document, &button_for_callback, action);
                let path = concrete_action_path_with_target_and_id(
                    &route_path,
                    action,
                    target.as_deref(),
                    id.as_deref(),
                );
                wasm_bindgen_futures::spawn_local(async move {
                    let result =
                        fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
                    let succeeded = result.is_ok();
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        match result {
                            Ok(response) => status.set_inner_html(&format!(
                                "<strong>{}</strong> {}",
                                escape_html(&method),
                                escape_html(&compact_preview(&response))
                            )),
                            Err(error) => {
                                let message = error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string());
                                status.set_inner_html(&format!(
                                    "<strong>{}</strong> {}",
                                    escape_html(&method),
                                    escape_html(&message)
                                ));
                            }
                        }
                    }
                    if return_to_searches && succeeded {
                        let _ = window.location().set_href("/searches");
                    } else {
                        let _ = refresh_route_data(&window).await;
                    }
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn mount_route_actions(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let buttons = document.query_selector_all(".slskr-action-button")?;
    for index in 0..buttons.length() {
        let Some(node) = buttons.item(index) else {
            continue;
        };
        let button: web_sys::Element = node.dyn_into()?;
        let Some(action_index) = button
            .get_attribute("data-slskr-action-index")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            continue;
        };
        let input_selector = format!(
            "#slskr-route-actions li:nth-child({}) .slskr-action-input",
            index + 1
        );
        let window = window.clone();
        let document = document.clone();
        let button_for_callback = button.clone();
        let callback = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(
            move |_event: web_sys::MouseEvent| {
                let value = document
                    .query_selector(&input_selector)
                    .ok()
                    .flatten()
                    .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
                    .map(|input| input.value())
                    .unwrap_or_default();
                let route_path = window.location().pathname().unwrap_or_default();
                let Some(action) = route_action_at(&route_path, action_index) else {
                    return;
                };
                if !native_route_action_is_ready(
                    &document,
                    &button_for_callback,
                    &route_path,
                    action,
                    "",
                    &value,
                ) {
                    return;
                }
                let body = native_action_body(&document, &button_for_callback, action, &value);
                let window = window.clone();
                let document = document.clone();
                let method = action.method.to_string();
                let return_to_searches = route_kind(&route_path) == RouteKind::Search
                    && route_path != "/searches"
                    && matches!(action.label, "Clear Searches" | "Remove Search");
                let target =
                    native_action_target_for_ui(&document, &button_for_callback, action, "");
                let id = native_action_id(&document, &button_for_callback, action);
                let path = concrete_action_path_with_target_and_id(
                    &route_path,
                    action,
                    target.as_deref(),
                    id.as_deref(),
                );
                wasm_bindgen_futures::spawn_local(async move {
                    let result =
                        fetch_text_with_method(&window, &path, &method, body.as_deref()).await;
                    let succeeded = result.is_ok();
                    if let Some(status) = document.get_element_by_id("slskr-action-status") {
                        match result {
                            Ok(response) => status.set_inner_html(&format!(
                                "<strong>{}</strong> {}",
                                escape_html(&method),
                                escape_html(&compact_preview(&response))
                            )),
                            Err(error) => {
                                let message = error
                                    .as_string()
                                    .unwrap_or_else(|| "request failed".to_string());
                                status.set_inner_html(&format!(
                                    "<strong>{}</strong> {}",
                                    escape_html(&method),
                                    escape_html(&message)
                                ));
                            }
                        }
                    }
                    if return_to_searches && succeeded {
                        let _ = window.location().set_href("/searches");
                    } else {
                        let _ = refresh_route_data(&window).await;
                    }
                });
            },
        ));
        button.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
        callback.forget();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn refresh_runtime_status() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let Some(status) = document.get_element_by_id("slskr-runtime-status") else {
        return Ok(());
    };

    let mut rendered = String::new();
    for probe in runtime_probes() {
        let path = endpoint_url(probe.path);
        let result = fetch_text(&window, &path).await;
        let row = match result {
            Ok(body) => runtime_probe_result_html(&[(probe.label, &path, Ok(body.as_str()))]),
            Err(error) => {
                let message = error
                    .as_string()
                    .unwrap_or_else(|| "request failed".to_string());
                runtime_probe_result_html(&[(probe.label, &path, Err(message.as_str()))])
            }
        };
        rendered.push_str(&row);
        status.set_inner_html(&rendered);
    }

    Ok(())
}

pub fn player_now_playing_text(body: &str) -> (String, String) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return (
            "Queue idle".to_string(),
            "No local stream selected".to_string(),
        );
    };
    let current = value
        .get("now_playing")
        .and_then(|entry| entry.as_array())
        .and_then(|items| items.first())
        .or_else(|| value.get("current"))
        .or_else(|| value.get("track"))
        .unwrap_or(&value);
    let title = current
        .get("title")
        .or_else(|| current.get("fileName"))
        .or_else(|| current.get("filename"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Queue idle".to_string());
    let artist = current
        .get("artist")
        .or_else(|| current.get("username"))
        .map(json_scalar_preview)
        .unwrap_or_default();
    let album = current
        .get("album")
        .map(json_scalar_preview)
        .unwrap_or_default();
    let detail = [artist, album]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let detail = if detail.is_empty() {
        "No local stream selected".to_string()
    } else {
        detail
    };
    (title, detail)
}

pub fn player_transfer_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "0 down / 0 up".to_string();
    };
    let downloads = value
        .get("downloads")
        .or_else(|| value.get("downloadSpeed"))
        .or_else(|| value.get("down"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    let uploads = value
        .get("uploads")
        .or_else(|| value.get("uploadSpeed"))
        .or_else(|| value.get("up"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    format!("{downloads} down / {uploads} up")
}

pub fn player_party_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "Listening party idle".to_string();
    };
    let count = value
        .get("count")
        .or_else(|| value.get("active"))
        .map(json_scalar_preview)
        .unwrap_or_else(|| "0".to_string());
    format!("{count} listening parties")
}

pub fn player_visualizer_text(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return "Visualizer status unavailable".to_string();
    };
    value
        .get("status")
        .or_else(|| value.get("next_action"))
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Visualizer status unavailable".to_string())
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
struct RustyMilkImportedPreset {
    source: String,
    title: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, PartialEq)]
struct RustyMilkPlaylist {
    id: String,
    name: String,
    preset_keys: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
struct RustyMilkAutomationState {
    beat_count: usize,
    last_beat_at: f64,
    last_preset_at: f64,
    smoothed_energy: f64,
}

#[cfg(target_arch = "wasm32")]
impl Default for RustyMilkAutomationState {
    fn default() -> Self {
        Self {
            beat_count: 0,
            last_beat_at: 0.0,
            last_preset_at: 0.0,
            smoothed_energy: 0.0,
        }
    }
}

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_IMPORTED_PRESETS_STORAGE_KEY: &str = "slskr.rustyMilkImportedPresets";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_FAVORITE_PRESETS_STORAGE_KEY: &str = "slskr.rustyMilkFavoritePresets";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_PRESET_SEARCH_STORAGE_KEY: &str = "slskr.rustyMilkPresetSearch";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_PLAYLISTS_STORAGE_KEY: &str = "slskr.rustyMilkPresetPlaylists";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_ACTIVE_PLAYLIST_STORAGE_KEY: &str = "slskr.rustyMilkActivePresetPlaylist";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_AUTOMATION_STORAGE_KEY: &str = "slskr.rustyMilkPresetAutomation";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_FPS_STORAGE_KEY: &str = "slskr.rustyMilkFpsCap";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_QUALITY_STORAGE_KEY: &str = "slskr.rustyMilkQuality";

#[cfg(target_arch = "wasm32")]
const RUSTYMILK_TEXTURE_ASSETS_STORAGE_KEY: &str = "slskr.rustyMilkTextureAssets";

#[cfg(target_arch = "wasm32")]
fn toggle_rustymilk_visualizer(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let panel = document
        .get_element_by_id("slskr-rust-rustymilk")
        .ok_or_else(|| JsValue::from_str("RustyMilk panel is missing"))?;
    if panel.has_attribute("hidden") {
        panel.remove_attribute("hidden")?;
        start_rustymilk_visualizer(window, document)?;
    } else {
        panel.set_attribute("hidden", "")?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn start_rustymilk_visualizer(
    window: &web_sys::Window,
    document: &web_sys::Document,
) -> Result<(), JsValue> {
    let panel = document
        .get_element_by_id("slskr-rust-rustymilk")
        .ok_or_else(|| JsValue::from_str("RustyMilk panel is missing"))?;
    if panel
        .get_attribute("data-slskr-rustymilk-running")
        .as_deref()
        == Some("true")
    {
        return Ok(());
    }
    panel.set_attribute("data-slskr-rustymilk-running", "true")?;
    let stored_search = load_rustymilk_preset_search(window);
    panel.set_attribute("data-slskr-rustymilk-search", &stored_search)?;
    panel.set_attribute(
        "data-slskr-rustymilk-playlist",
        &load_rustymilk_active_playlist(window),
    )?;
    panel.set_attribute(
        "data-slskr-rustymilk-automation",
        &load_rustymilk_simple_setting(window, RUSTYMILK_AUTOMATION_STORAGE_KEY, "off"),
    )?;
    panel.set_attribute(
        "data-slskr-rustymilk-fps",
        &load_rustymilk_simple_setting(window, RUSTYMILK_FPS_STORAGE_KEY, "full"),
    )?;
    panel.set_attribute(
        "data-slskr-rustymilk-quality",
        &load_rustymilk_simple_setting(window, RUSTYMILK_QUALITY_STORAGE_KEY, "balanced"),
    )?;
    if let Some(search_input) = document
        .get_element_by_id("slskr-rustymilk-search")
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        search_input.set_value(&stored_search);
    }

    let texture_assets: Rc<RefCell<BTreeMap<String, String>>> =
        Rc::new(RefCell::new(load_rustymilk_texture_assets(window)));
    let imported_presets: Rc<RefCell<Vec<RustyMilkImportedPreset>>> =
        Rc::new(RefCell::new(load_rustymilk_imported_presets(window)));
    let favorite_presets: Rc<RefCell<BTreeSet<String>>> =
        Rc::new(RefCell::new(load_rustymilk_favorite_presets(window)));
    mount_rustymilk_preset_input(document, imported_presets.clone())?;
    mount_rustymilk_texture_input(document, texture_assets.clone())?;
    mount_rustymilk_pack_input(document, imported_presets.clone(), texture_assets.clone())?;
    mount_rustymilk_selects(document)?;
    mount_rustymilk_buttons(
        window,
        document,
        imported_presets.clone(),
        favorite_presets.clone(),
        texture_assets.clone(),
    )?;

    let canvas: web_sys::HtmlCanvasElement = document
        .get_element_by_id("slskr-rustymilk-canvas")
        .ok_or_else(|| JsValue::from_str("RustyMilk canvas is missing"))?
        .dyn_into()?;
    let renderer = Rc::new(rustymilk_renderer(&canvas, texture_assets.clone())?);
    let analyzer = Rc::new(RefCell::new(
        player_audio_element(document).and_then(|audio| RustyMilkAudioAnalyzer::new(&audio).ok()),
    ));
    let input_state = Rc::new(RefCell::new(RustyMilkInputState::default()));
    mount_rustymilk_mouse_input(&canvas, input_state.clone())?;
    let runtime = Rc::new(RefCell::new(RustyMilkFrameSetRuntime::default()));
    let automation_state = Rc::new(RefCell::new(RustyMilkAutomationState::default()));
    let last_render_ms = Rc::new(RefCell::new(0.0));
    if let Some(label) = document.get_element_by_id("slskr-rustymilk-renderer") {
        label.set_text_content(Some(renderer.label()));
    }
    update_rustymilk_texture_status(document, &texture_assets);
    set_rustymilk_active_preset(
        document,
        &panel,
        &imported_presets,
        &favorite_presets,
        panel
            .get_attribute("data-slskr-rustymilk-preset-index")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0),
        "RustyMilk visualizer ready",
    );
    let animation_handle: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> =
        Rc::new(RefCell::new(None));
    let animation_handle_for_frame = animation_handle.clone();
    let window_for_frame = window.clone();
    let document_for_frame = document.clone();
    let analyzer_for_frame = analyzer.clone();
    let imports_for_frame = imported_presets.clone();
    let favorites_for_frame = favorite_presets.clone();
    let input_for_frame = input_state.clone();
    let runtime_for_frame = runtime.clone();
    let automation_for_frame = automation_state.clone();
    let last_render_for_frame = last_render_ms.clone();

    *animation_handle_for_frame.borrow_mut() = Some(Closure::wrap(Box::new(move |time_ms: f64| {
        let Some(panel) = document_for_frame.get_element_by_id("slskr-rust-rustymilk") else {
            return;
        };
        if panel.has_attribute("hidden") {
            return;
        }
        let preset_index = panel
            .get_attribute("data-slskr-rustymilk-preset-index")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let preset_source =
            rustymilk_active_preset_source(&panel, &imports_for_frame, preset_index);
        let time = time_ms / 1000.0;
        let fps_cap = rustymilk_fps_cap_ms(&panel);
        if fps_cap > 0.0 && time_ms - *last_render_for_frame.borrow() < fps_cap {
            if let Some(callback) = animation_handle.borrow().as_ref() {
                let _ = window_for_frame.request_animation_frame(callback.as_ref().unchecked_ref());
            }
            return;
        }
        *last_render_for_frame.borrow_mut() = time_ms;
        if analyzer_for_frame.borrow().is_none() {
            if let Some(audio) = player_audio_element(&document_for_frame) {
                if let Ok(next_analyzer) = RustyMilkAudioAnalyzer::new(&audio) {
                    *analyzer_for_frame.borrow_mut() = Some(next_analyzer);
                }
            }
        }
        let audio = analyzer_for_frame
            .borrow()
            .as_ref()
            .map(|analyzer| analyzer.snapshot(time))
            .unwrap_or_else(|| RustyMilkAudioSnapshot::synthetic(time));
        maybe_advance_rustymilk_automation(
            &document_for_frame,
            &panel,
            &imports_for_frame,
            &favorites_for_frame,
            &mut automation_for_frame.borrow_mut(),
            time,
            audio.bands.bass + audio.bands.mid + audio.bands.treble,
        );
        let frame_set = runtime_for_frame
            .borrow_mut()
            .render_source_with_audio_and_input(
                &preset_source,
                time,
                audio.bands.bass,
                audio.bands.mid,
                audio.bands.treble,
                &audio.waveform,
                &audio.spectrum,
                *input_for_frame.borrow(),
            );
        renderer.render_frame_set(&frame_set, time);
        if let Some(status) = document_for_frame.get_element_by_id("slskr-rustymilk-status") {
            let shape_count = frame_set
                .entries
                .iter()
                .map(|entry| entry.frame.shape_count)
                .sum::<usize>();
            let waveform_count = frame_set
                .entries
                .iter()
                .map(|entry| entry.frame.waveform_count)
                .sum::<usize>();
            status.set_text_content(Some(&format!(
                "RustyMilk running: {} bass {:.0}% mid {:.0}% treble {:.0}% / {} preset{} / {} shapes / {} waves",
                audio.source,
                audio.bands.bass * 100.0,
                audio.bands.mid * 100.0,
                audio.bands.treble * 100.0,
                frame_set.preset_count,
                if frame_set.preset_count == 1 { "" } else { "s" },
                shape_count,
                waveform_count
            )));
        }
        update_rustymilk_library_controls(
            &document_for_frame,
            &panel,
            &imports_for_frame,
            &favorites_for_frame,
            preset_index,
        );
        if let Some(callback) = animation_handle.borrow().as_ref() {
            let _ = window_for_frame.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut(f64)>));

    if let Some(callback) = animation_handle_for_frame.borrow().as_ref() {
        window.request_animation_frame(callback.as_ref().unchecked_ref())?;
    }
    Ok(())
}
