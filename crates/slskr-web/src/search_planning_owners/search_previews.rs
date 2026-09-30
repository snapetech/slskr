use super::*;

pub(super) fn search_provider_labels(response: &serde_json::Value) -> Vec<String> {
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

pub(super) fn search_response_signature(response: &serde_json::Value) -> String {
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
