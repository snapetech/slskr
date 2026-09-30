use super::*;

pub(super) fn file_extension(filename: &str) -> String {
    filename
        .rsplit(['/', '\\'])
        .next()
        .and_then(|name| name.rsplit_once('.').map(|(_, extension)| extension))
        .unwrap_or_default()
        .to_lowercase()
}

pub(super) fn search_response_files(response: &serde_json::Value) -> Vec<serde_json::Value> {
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

pub(super) fn search_file_name(file: &serde_json::Value) -> String {
    json_track_field(file, &["filename", "fileName", "path"])
}

pub(super) fn search_file_size(file: &serde_json::Value) -> u64 {
    file.get("size")
        .or_else(|| file.get("bytes"))
        .and_then(|value| value.as_u64())
        .unwrap_or_default()
}

pub(super) fn search_file_number(file: &serde_json::Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|key| file.get(*key).and_then(|value| value.as_u64()))
        .unwrap_or_default()
}

pub(super) fn search_tokens(value: &str) -> Vec<String> {
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

pub(super) fn is_lossless_extension(extension: &str) -> bool {
    matches!(
        extension,
        "aif" | "aiff" | "alac" | "ape" | "flac" | "wav" | "wv"
    )
}

pub(super) fn is_lossy_extension(extension: &str) -> bool {
    matches!(extension, "aac" | "m4a" | "mp3" | "ogg" | "opus" | "wma")
}

pub(super) fn is_artwork_extension(extension: &str) -> bool {
    matches!(extension, "gif" | "jpeg" | "jpg" | "png" | "webp")
}

pub(super) fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

pub(super) fn push_unique_reason(reasons: &mut Vec<String>, reason: impl Into<String>) {
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
