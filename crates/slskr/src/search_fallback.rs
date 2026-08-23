//! Bounded fallback queries for network-suppressed wishlist searches.
//!
//! The fallback is intentionally small and deterministic: it only applies to
//! ordinary multi-term wishlist text, removes at most two leading/suppressed
//! terms, and never rewrites explicit query syntax.

use std::collections::HashSet;

pub const MINIMUM_RESULT_THRESHOLD: usize = 10;
pub const MAXIMUM_FALLBACK_QUERIES: usize = 2;

const KNOWN_SUPPRESSED_TERMS: &[&str] = &["linkin", "metallica"];

pub fn is_enabled_for_source(source: &str) -> bool {
    source.eq_ignore_ascii_case("wishlist")
}

pub fn needs_fallback(
    response_count: usize,
    file_count: usize,
    response_limit: usize,
    file_limit: usize,
) -> bool {
    response_count < MINIMUM_RESULT_THRESHOLD.min(response_limit)
        && file_count < MINIMUM_RESULT_THRESHOLD.min(file_limit)
}

pub fn create_queries(search_text: &str) -> Vec<String> {
    let terms = search_text
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if terms.len() < 3 || contains_explicit_query_syntax(search_text, &terms) {
        return Vec::new();
    }

    let mut queries = Vec::with_capacity(MAXIMUM_FALLBACK_QUERIES);
    let mut seen = HashSet::new();
    let suppressed_terms = terms
        .iter()
        .filter(|term| KNOWN_SUPPRESSED_TERMS.contains(&term.to_ascii_lowercase().as_str()))
        .map(|term| term.to_ascii_lowercase())
        .collect::<Vec<_>>();

    for suppressed in &suppressed_terms {
        if queries.len() >= MAXIMUM_FALLBACK_QUERIES {
            break;
        }
        let fallback = terms
            .iter()
            .filter(|term| !term.eq_ignore_ascii_case(suppressed))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        if !fallback.is_empty() && seen.insert(fallback.clone()) {
            queries.push(fallback);
        }
    }

    let candidate_count = (MAXIMUM_FALLBACK_QUERIES - queries.len() + suppressed_terms.len())
        .min(terms.len().saturating_sub(1));
    for removed_index in 0..candidate_count {
        if queries.len() >= MAXIMUM_FALLBACK_QUERIES {
            break;
        }
        if suppressed_terms
            .iter()
            .any(|term| term.eq_ignore_ascii_case(&terms[removed_index]))
        {
            continue;
        }
        let fallback = terms
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != removed_index)
            .map(|(_, term)| term.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if seen.insert(fallback.clone()) {
            queries.push(fallback);
        }
    }
    queries
}

fn contains_explicit_query_syntax(search_text: &str, terms: &[String]) -> bool {
    search_text.contains(['"', '\'', '|'])
        || terms
            .iter()
            .any(|term| term.eq_ignore_ascii_case("OR") || term.starts_with('-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_is_bounded_and_skips_explicit_syntax() {
        assert_eq!(create_queries("one two"), Vec::<String>::new());
        assert_eq!(create_queries("one OR two three"), Vec::<String>::new());
        assert!(create_queries("Artist Album Track").len() <= MAXIMUM_FALLBACK_QUERIES);
    }

    #[test]
    fn known_suppressed_terms_are_removed_first() {
        let queries = create_queries("Linkin Park Album");
        assert_eq!(queries.first().map(String::as_str), Some("Park Album"));
    }

    #[test]
    fn threshold_respects_configured_limits() {
        assert!(needs_fallback(0, 0, 10, 10));
        assert!(!needs_fallback(1, 1, 1, 1));
    }
}
