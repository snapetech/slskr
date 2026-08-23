//! Bounded, allocation-conscious identity scoring used by the SongID API.
//!
//! The runtime has several independent ways to produce candidate metadata
//! (filename metadata, library records, AcoustID, and local corpus results).
//! Keeping the text normalization and consensus math in one module prevents
//! each route from quietly implementing a different matching policy.

use std::collections::HashMap;

/// Normalize artist/title text for loose identity comparisons.
pub fn normalize_loose_text(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    let mut output = String::with_capacity(value.len());
    let mut pending_space = false;
    let mut index = 0;
    while index < value.len() {
        let rest = &value[index..];
        let alias = if rest.starts_with(" feat. ") {
            Some(("featuring", 7))
        } else if rest.starts_with(" feat ") {
            Some(("featuring", 6))
        } else if rest.starts_with(" ft. ") {
            Some(("featuring", 5))
        } else if rest.starts_with(" ft ") {
            Some(("featuring", 4))
        } else {
            None
        };
        if let Some((alias, alias_length)) = alias {
            if !output.is_empty() && !output.ends_with(' ') {
                output.push(' ');
            }
            output.push_str(alias);
            pending_space = true;
            index += alias_length;
            continue;
        }
        let character = value[index..]
            .chars()
            .next()
            .expect("index is on a char boundary");
        index += character.len_utf8();
        if character.is_ascii_alphanumeric() {
            if pending_space && !output.is_empty() && !output.ends_with(' ') {
                output.push(' ');
            }
            output.push(character);
            pending_space = false;
        } else if !output.is_empty() {
            pending_space = true;
        }
    }
    output.trim().to_owned()
}

/// Return a Jaccard-like token similarity in the range 0..=1.
pub fn compare_loose_text(left: &str, right: &str) -> f64 {
    let left = normalize_loose_text(left);
    let right = normalize_loose_text(right);
    compare_normalized(&left, &right)
}

fn compare_normalized(left: &str, right: &str) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }
    let left_tokens = left
        .split_whitespace()
        .collect::<std::collections::BTreeSet<_>>();
    let right_tokens = right
        .split_whitespace()
        .collect::<std::collections::BTreeSet<_>>();
    let intersection = left_tokens.intersection(&right_tokens).count();
    let union = left_tokens.union(&right_tokens).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Score a library identity against a remote filename using both artist and
/// title evidence. The title is weighted more heavily because filenames often
/// omit or abbreviate the artist.
pub fn filename_identity_score(artist: &str, title: &str, filename: &str) -> f64 {
    let filename = filename.rsplit_once('.').map_or(filename, |(stem, _)| stem);
    let filename = normalize_loose_text(filename);
    if filename.is_empty() {
        return 0.0;
    }
    let artist = normalize_loose_text(artist);
    let title = normalize_loose_text(title);
    let artist_score =
        compare_normalized(&artist, &filename).max(token_containment_score(&artist, &filename));
    let title_score =
        compare_normalized(&title, &filename).max(token_containment_score(&title, &filename));
    let exact_artist = !artist.is_empty() && filename.contains(&artist);
    let exact_title = !title.is_empty() && filename.contains(&title);
    let base = (artist_score * 0.4) + (title_score * 0.6);
    let exact_bonus = match (exact_artist, exact_title) {
        (true, true) => 0.25,
        (true, false) | (false, true) => 0.08,
        (false, false) => 0.0,
    };
    (base + exact_bonus).min(1.0)
}

fn token_containment_score(identity: &str, filename: &str) -> f64 {
    let identity_tokens = identity.split_whitespace().collect::<Vec<_>>();
    if identity_tokens.is_empty() {
        return 0.0;
    }
    let filename_tokens = filename
        .split_whitespace()
        .collect::<std::collections::BTreeSet<_>>();
    let matched = identity_tokens
        .iter()
        .filter(|token| filename_tokens.contains(**token))
        .count();
    matched as f64 / identity_tokens.len() as f64
}

/// Measure repeated non-empty transcript lines, bounded to the first 256
/// normalized lines so hostile input cannot create an unbounded map.
pub fn repeated_line_ratio(value: &str) -> f64 {
    let mut counts = HashMap::<String, usize>::new();
    let mut total = 0usize;
    for line in value.lines().take(256) {
        let normalized = normalize_loose_text(line);
        if normalized.is_empty() {
            continue;
        }
        total += 1;
        *counts.entry(normalized).or_default() += 1;
    }
    if total == 0 {
        return 0.0;
    }
    counts
        .values()
        .filter(|count| **count > 1)
        .map(|count| *count as f64)
        .sum::<f64>()
        / total as f64
}

/// Measure repeated three-token ngrams in bounded text.
pub fn repeated_ngram_ratio(value: &str) -> f64 {
    let tokens = value
        .to_ascii_lowercase()
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|character: char| {
                    !character.is_ascii_alphabetic() && character != '\''
                })
                .to_owned()
        })
        .filter(|token| !token.is_empty())
        .take(512)
        .collect::<Vec<_>>();
    if tokens.len() < 6 {
        return 0.0;
    }
    let mut counts = HashMap::<[String; 3], usize>::new();
    for window in tokens.windows(3) {
        let key = [window[0].clone(), window[1].clone(), window[2].clone()];
        *counts.entry(key).or_default() += 1;
    }
    let repeated = counts
        .values()
        .filter(|count| **count > 1)
        .map(|count| *count as f64)
        .sum::<f64>();
    repeated / (tokens.len().saturating_sub(2) as f64)
}

/// Combine independent identity observations into a conservative consensus.
pub fn identity_consensus(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let mut ordered = scores
        .iter()
        .copied()
        .filter(|score| score.is_finite())
        .map(|score| score.clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    if ordered.is_empty() {
        return 0.0;
    }
    ordered.sort_by(|left, right| right.total_cmp(left));
    let top = ordered[0];
    let support = ordered.iter().take(3).sum::<f64>() / ordered.len().min(3) as f64;
    (top * 0.65 + support * 0.35).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_feature_aliases_and_punctuation() {
        assert_eq!(
            normalize_loose_text("Beyoncé feat. Jay-Z"),
            "beyonc featuring jay z"
        );
        assert_eq!(normalize_loose_text("Artist & Title"), "artist title");
    }

    #[test]
    fn scores_filename_with_two_sides_of_identity() {
        assert!(filename_identity_score("Artist", "Title", "Artist - Title.flac") > 0.8);
        assert!(filename_identity_score("Artist", "Title", "Other Song.mp3") < 0.4);
    }

    #[test]
    fn repeated_text_metrics_are_bounded() {
        assert!(repeated_line_ratio("same\nsame\nother") > 0.5);
        assert!(repeated_ngram_ratio("one two three one two three") > 0.0);
    }

    #[test]
    fn consensus_prefers_supported_top_score() {
        assert!(identity_consensus(&[0.95, 0.9, 0.1]) > 0.75);
        assert_eq!(identity_consensus(&[]), 0.0);
    }
}
