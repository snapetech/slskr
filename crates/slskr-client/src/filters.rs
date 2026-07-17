use std::collections::HashSet;

use slskr_protocol::server::ServerMessage;

pub const MAX_EXCLUDED_SEARCH_PHRASES: usize = 256;
pub const MAX_EXCLUDED_SEARCH_PHRASE_BYTES: usize = 256;
pub const MAX_FILTERED_SEARCH_QUERY_BYTES: usize = 4_096;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExcludedPhraseFilter {
    phrases: Vec<String>,
}

impl ExcludedPhraseFilter {
    #[must_use]
    pub fn new(phrases: impl IntoIterator<Item = String>) -> Self {
        let mut normalized = Vec::new();
        let mut seen = HashSet::new();
        for phrase in phrases {
            let phrase = truncate_utf8_bytes(phrase.trim(), MAX_EXCLUDED_SEARCH_PHRASE_BYTES)
                .to_ascii_lowercase();
            if phrase.is_empty() || !seen.insert(phrase.clone()) {
                continue;
            }
            normalized.push(phrase);
            if normalized.len() == MAX_EXCLUDED_SEARCH_PHRASES {
                break;
            }
        }
        let phrases = normalized;
        Self { phrases }
    }

    #[must_use]
    pub fn from_server_message(message: &ServerMessage) -> Option<Self> {
        match message {
            ServerMessage::ExcludedSearchPhrases(phrases) => {
                Some(Self::new(phrases.iter().cloned()))
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn phrases(&self) -> &[String] {
        &self.phrases
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.phrases.is_empty()
    }

    #[must_use]
    pub fn allows_query(&self, query: &str) -> bool {
        if query.len() > MAX_FILTERED_SEARCH_QUERY_BYTES {
            return false;
        }
        let query = query.to_ascii_lowercase();
        self.phrases
            .iter()
            .all(|phrase| !query.contains(phrase.as_str()))
    }
}

fn truncate_utf8_bytes(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &value[..boundary]
}
