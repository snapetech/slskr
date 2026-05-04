use slskr_protocol::server::ServerMessage;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExcludedPhraseFilter {
    phrases: Vec<String>,
}

impl ExcludedPhraseFilter {
    #[must_use]
    pub fn new(phrases: impl IntoIterator<Item = String>) -> Self {
        let phrases = phrases
            .into_iter()
            .map(|phrase| phrase.trim().to_ascii_lowercase())
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
        let query = query.to_ascii_lowercase();
        self.phrases
            .iter()
            .all(|phrase| !query.contains(phrase.as_str()))
    }
}
