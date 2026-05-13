use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use slskr_protocol::{
    distributed::DistributedSearch,
    peer::{FileEntry, FileSearchResponse, PeerMessage},
    server::{SearchRequest, ServerMessage, TargetedSearchRequest},
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    filters::ExcludedPhraseFilter, manager::TokenGenerator, server::ServerSession, ClientError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequestHandle {
    pub token: u32,
    pub query: String,
    pub target: SearchTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchTarget {
    Global,
    User(String),
    Room(String),
    Wishlist,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WishlistSearchSchedulerOptions {
    pub minimum_interval: Duration,
    pub override_interval: Option<Duration>,
}

impl WishlistSearchSchedulerOptions {
    pub fn new(
        minimum_interval: Duration,
        override_interval: Option<Duration>,
    ) -> Result<Self, ClientError> {
        if minimum_interval.is_zero() {
            return Err(ClientError::InvalidInterval {
                field: "minimum_interval",
            });
        }
        if override_interval.is_some_and(|duration| duration.is_zero()) {
            return Err(ClientError::InvalidInterval {
                field: "override_interval",
            });
        }
        Ok(Self {
            minimum_interval,
            override_interval,
        })
    }

    #[must_use]
    pub fn next_interval(&self, server_interval: Option<Duration>) -> Duration {
        self.override_interval
            .or(server_interval)
            .unwrap_or(self.minimum_interval)
            .max(self.minimum_interval)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WishlistSearchScheduler {
    terms: Vec<String>,
    options: WishlistSearchSchedulerOptions,
    server_interval: Option<Duration>,
    next_index: usize,
}

impl WishlistSearchScheduler {
    pub fn new(
        terms: impl IntoIterator<Item = String>,
        options: WishlistSearchSchedulerOptions,
    ) -> Result<Self, ClientError> {
        let mut normalized = Vec::new();
        for term in terms {
            let term = term.trim().to_owned();
            if !term.is_empty() {
                normalized.push(term);
            }
        }
        Ok(Self {
            terms: normalized,
            options,
            server_interval: None,
            next_index: 0,
        })
    }

    pub fn replace_terms(&mut self, terms: impl IntoIterator<Item = String>) {
        let mut normalized = Vec::new();
        for term in terms {
            let term = term.trim().to_owned();
            if !term.is_empty() {
                normalized.push(term);
            }
        }
        self.terms = normalized;
        if self.next_index >= self.terms.len() {
            self.next_index = 0;
        }
    }

    pub fn apply_server_message(&mut self, message: &ServerMessage) -> bool {
        if let ServerMessage::WishlistInterval { seconds } = message {
            self.server_interval = Some(Duration::from_secs(u64::from(*seconds)));
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn interval(&self) -> Duration {
        self.options.next_interval(self.server_interval)
    }

    pub fn next_search_message(&mut self, token: u32) -> Option<ServerMessage> {
        let query = self.terms.get(self.next_index)?.clone();
        self.next_index = (self.next_index + 1) % self.terms.len();
        Some(ServerMessage::WishlistSearch(SearchRequest {
            token,
            query,
        }))
    }
}

#[derive(Debug)]
pub struct SearchDispatcher<S> {
    server: ServerSession<S>,
    tokens: TokenGenerator,
}

impl<S> SearchDispatcher<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    #[must_use]
    pub const fn new(server: ServerSession<S>, token_seed: u32) -> Self {
        Self {
            server,
            tokens: TokenGenerator::new(token_seed),
        }
    }

    pub fn into_inner(self) -> ServerSession<S> {
        self.server
    }

    pub async fn search_global(
        &mut self,
        query: impl Into<String>,
    ) -> Result<SearchRequestHandle, ClientError> {
        let query = query.into();
        let token = self.tokens.next_token();
        self.server
            .send_server_message(ServerMessage::FileSearchRequest(SearchRequest {
                token,
                query: query.clone(),
            }))
            .await?;
        Ok(SearchRequestHandle {
            token,
            query,
            target: SearchTarget::Global,
        })
    }

    pub async fn search_user(
        &mut self,
        username: impl Into<String>,
        query: impl Into<String>,
    ) -> Result<SearchRequestHandle, ClientError> {
        let username = username.into();
        let query = query.into();
        self.search_targeted(SearchTarget::User(username), query)
            .await
    }

    pub async fn search_room(
        &mut self,
        room: impl Into<String>,
        query: impl Into<String>,
    ) -> Result<SearchRequestHandle, ClientError> {
        let room = room.into();
        let query = query.into();
        self.search_targeted(SearchTarget::Room(room), query).await
    }

    pub async fn search_wishlist(
        &mut self,
        query: impl Into<String>,
    ) -> Result<SearchRequestHandle, ClientError> {
        let query = query.into();
        let token = self.tokens.next_token();
        self.server
            .send_server_message(ServerMessage::WishlistSearch(SearchRequest {
                token,
                query: query.clone(),
            }))
            .await?;
        Ok(SearchRequestHandle {
            token,
            query,
            target: SearchTarget::Wishlist,
        })
    }

    async fn search_targeted(
        &mut self,
        target: SearchTarget,
        query: String,
    ) -> Result<SearchRequestHandle, ClientError> {
        let token = self.tokens.next_token();
        let target_name = match &target {
            SearchTarget::User(username) | SearchTarget::Room(username) => username.clone(),
            SearchTarget::Global | SearchTarget::Wishlist => unreachable!("targeted only"),
        };
        let request = TargetedSearchRequest {
            target: target_name,
            token,
            query: query.clone(),
        };
        let message = match &target {
            SearchTarget::User(_) => ServerMessage::UserSearch(request),
            SearchTarget::Room(_) => ServerMessage::RoomSearch(request),
            SearchTarget::Global | SearchTarget::Wishlist => unreachable!("targeted only"),
        };
        self.server.send_server_message(message).await?;
        Ok(SearchRequestHandle {
            token,
            query,
            target,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchResults {
    by_token: HashMap<u32, Vec<FileSearchResponse>>,
}

impl SearchResults {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept_peer_message(&mut self, message: PeerMessage) -> Result<bool, ClientError> {
        match message {
            PeerMessage::FileSearchResponse(response) => {
                self.by_token
                    .entry(response.token)
                    .or_default()
                    .push(response);
                Ok(true)
            }
            message => Err(ClientError::UnexpectedSearchMessage(Box::new(message))),
        }
    }

    #[must_use]
    pub fn responses_for(&self, token: u32) -> &[FileSearchResponse] {
        self.by_token
            .get(&token)
            .map_or(&[], std::vec::Vec::as_slice)
    }

    #[must_use]
    pub fn take(&mut self, token: u32) -> Vec<FileSearchResponse> {
        self.by_token.remove(&token).unwrap_or_default()
    }

    #[must_use]
    pub fn len_for(&self, token: u32) -> usize {
        self.responses_for(token).len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedSearch {
    pub handle: SearchRequestHandle,
    pub created_at: Instant,
    pub expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct TimedSearchResults {
    results: SearchResults,
    searches: HashMap<u32, TimedSearch>,
    window: Duration,
}

impl TimedSearchResults {
    #[must_use]
    pub fn new(window: Duration) -> Self {
        Self {
            results: SearchResults::new(),
            searches: HashMap::new(),
            window,
        }
    }

    pub fn track(&mut self, handle: SearchRequestHandle, now: Instant) -> Option<TimedSearch> {
        let timed = TimedSearch {
            handle,
            created_at: now,
            expires_at: now + self.window,
        };
        self.searches.insert(timed.handle.token, timed)
    }

    pub fn accept_peer_message(&mut self, message: PeerMessage) -> Result<bool, ClientError> {
        match &message {
            PeerMessage::FileSearchResponse(response)
                if self.searches.contains_key(&response.token) =>
            {
                self.results.accept_peer_message(message)
            }
            PeerMessage::FileSearchResponse(_) => Ok(false),
            _ => self.results.accept_peer_message(message),
        }
    }

    #[must_use]
    pub fn responses_for(&self, token: u32) -> &[FileSearchResponse] {
        self.results.responses_for(token)
    }

    #[must_use]
    pub fn len_for(&self, token: u32) -> usize {
        self.results.len_for(token)
    }

    #[must_use]
    pub fn is_active(&self, token: u32) -> bool {
        self.searches.contains_key(&token)
    }

    #[must_use]
    pub fn active_len(&self) -> usize {
        self.searches.len()
    }

    pub fn finish(&mut self, token: u32) -> Option<(TimedSearch, Vec<FileSearchResponse>)> {
        let search = self.searches.remove(&token)?;
        let responses = self.results.take(token);
        Some((search, responses))
    }

    pub fn drain_expired(&mut self, now: Instant) -> Vec<(TimedSearch, Vec<FileSearchResponse>)> {
        let expired: HashSet<u32> = self
            .searches
            .iter()
            .filter_map(|(token, search)| (search.expires_at <= now).then_some(*token))
            .collect();

        let mut drained = Vec::with_capacity(expired.len());
        for token in expired {
            if let Some(done) = self.finish(token) {
                drained.push(done);
            }
        }
        drained
    }
}

pub trait ShareIndex {
    fn search(&self, query: &str) -> Vec<FileEntry>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryShareIndex {
    entries: Vec<FileEntry>,
}

impl InMemoryShareIndex {
    #[must_use]
    pub fn new(entries: Vec<FileEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }
}

impl ShareIndex for InMemoryShareIndex {
    fn search(&self, query: &str) -> Vec<FileEntry> {
        let terms = normalize_terms(query);
        if terms.is_empty() {
            return Vec::new();
        }

        self.entries
            .iter()
            .filter(|entry| {
                let filename = entry.filename.to_ascii_lowercase();
                terms.iter().all(|term| filename.contains(term))
            })
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResponder<I> {
    username: String,
    index: I,
    excluded_filter: ExcludedPhraseFilter,
    average_speed: u32,
    queue_length: u32,
    unknown: u32,
}

impl<I> SearchResponder<I>
where
    I: ShareIndex,
{
    #[must_use]
    pub fn new(username: impl Into<String>, index: I) -> Self {
        Self {
            username: username.into(),
            index,
            excluded_filter: ExcludedPhraseFilter::default(),
            average_speed: 0,
            queue_length: 0,
            unknown: 0,
        }
    }

    #[must_use]
    pub const fn with_stats(mut self, average_speed: u32, queue_length: u32, unknown: u32) -> Self {
        self.average_speed = average_speed;
        self.queue_length = queue_length;
        self.unknown = unknown;
        self
    }

    #[must_use]
    pub fn with_excluded_phrases(mut self, phrases: impl IntoIterator<Item = String>) -> Self {
        self.excluded_filter = ExcludedPhraseFilter::new(phrases);
        self
    }

    #[must_use]
    pub fn respond_to_server_search(&self, message: &ServerMessage) -> Option<PeerMessage> {
        match message {
            ServerMessage::FileSearchIncoming { token, query, .. } => {
                self.response_message(*token, query)
            }
            ServerMessage::UserSearch(request) | ServerMessage::RoomSearch(request) => {
                self.response_message(request.token, &request.query)
            }
            ServerMessage::WishlistSearch(request) | ServerMessage::FileSearchRequest(request) => {
                self.response_message(request.token, &request.query)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn respond_to_distributed_search(&self, search: &DistributedSearch) -> Option<PeerMessage> {
        self.response_message(search.token, &search.query)
    }

    fn response_message(&self, token: u32, query: &str) -> Option<PeerMessage> {
        if !self.excluded_filter.allows_query(query) {
            return None;
        }

        let results = self.index.search(query);
        if results.is_empty() {
            return None;
        }

        Some(PeerMessage::FileSearchResponse(FileSearchResponse {
            username: self.username.clone(),
            token,
            results,
            slot_free: true,
            average_speed: self.average_speed,
            queue_length: self.queue_length,
            unknown: self.unknown,
            private_results: Vec::new(),
        }))
    }
}

fn normalize_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect()
}
