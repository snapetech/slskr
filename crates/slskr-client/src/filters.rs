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
        let phrases = phrases
            .into_iter()
            .take(MAX_EXCLUDED_SEARCH_PHRASES)
            .map(|phrase| phrase.trim().to_ascii_lowercase())
            .map(|phrase| truncate_utf8_bytes(phrase, MAX_EXCLUDED_SEARCH_PHRASE_BYTES))
            .filter(|phrase| !phrase.is_empty())
            .collect();
        Self { phrases }
    }

    #[must_use]
    pub fn from_server_message(message: &ServerMessage) -> Option<Self> {
        match message {
            ServerMessage::ExcludedSearchPhrases(phrases) => Some(Self::new(phrases.clone())),
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

fn truncate_utf8_bytes(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}
