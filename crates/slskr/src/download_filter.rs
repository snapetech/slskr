//! Daemon-wide outbound download exclusion policy.

/// Return the first configured literal exclusion contained in a remote path.
/// Matching trims both values, treats path separators as equivalent, and is
/// case-insensitive, matching the current upstream policy.
pub fn matching_exclusion(remote_filename: &str, exclusions: &[String]) -> Option<String> {
    let normalized_filename = normalize(remote_filename).to_lowercase();
    if normalized_filename.is_empty() {
        return None;
    }

    exclusions.iter().find_map(|configured| {
        let exclusion = configured.trim();
        if exclusion.is_empty() {
            return None;
        }
        let normalized_exclusion = normalize(exclusion).to_lowercase();
        normalized_filename
            .contains(&normalized_exclusion)
            .then(|| exclusion.to_owned())
    })
}

pub fn is_excluded(remote_filename: &str, exclusions: &[String]) -> bool {
    matching_exclusion(remote_filename, exclusions).is_some()
}

fn normalize(value: &str) -> String {
    value.trim().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{is_excluded, matching_exclusion};

    fn exclusions(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn matches_literal_terms_case_insensitively() {
        let configured = exclusions(&[" lossless ", "SAMPLE"]);
        assert_eq!(
            matching_exclusion("Artist\\Album\\Lossless.flac", &configured),
            Some("lossless".to_owned())
        );
        assert!(is_excluded("Artist/SAMPLE.flac", &configured));
    }

    #[test]
    fn ignores_blank_terms_and_empty_paths() {
        let configured = exclusions(&[" ", "\t"]);
        assert_eq!(matching_exclusion("Artist/Track.flac", &configured), None);
        assert!(!is_excluded("", &configured));
    }

    #[test]
    fn returns_the_first_configured_match() {
        let configured = exclusions(&["artist", "track"]);
        assert_eq!(
            matching_exclusion("Artist/Track.flac", &configured),
            Some("artist".to_owned())
        );
    }

    #[test]
    fn folds_unicode_case_without_treating_separators_as_significant() {
        let configured = exclusions(&["Beyoncé"]);
        assert!(is_excluded("Artist\\BEYONCÉ\\Track.flac", &configured));
    }
}
