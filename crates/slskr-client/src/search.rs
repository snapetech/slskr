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
    filters::ExcludedPhraseFilter, manager::TokenGenerator, peer_cache::normalize_peer_username,
    server::ServerSession, ClientError,
};

pub const MAX_SEARCH_RESPONSES_PER_TOKEN: usize = 1_000;
pub const MAX_SEARCH_RESULT_FILES_PER_TOKEN: usize = 10_000;
pub const MAX_TRACKED_SEARCH_RESULT_TOKENS: usize = 1_024;
pub const MAX_SEARCH_RESULT_TEXT_BYTES_PER_TOKEN: usize = 4 * 1024 * 1024;
pub const MAX_SEARCH_RESULT_FILES_TOTAL: usize = 100_000;
pub const MAX_SEARCH_RESULT_TEXT_BYTES_TOTAL: usize = 64 * 1024 * 1024;
pub const MAX_SEARCH_RESPONSES_TOTAL: usize = 10_000;
pub const MAX_WISHLIST_SEARCH_TERMS: usize = 1_024;
pub const MAX_WISHLIST_SEARCH_TERM_BYTES: usize = 4_096;
pub const MAX_OUTBOUND_SEARCH_FIELD_BYTES: usize = 4_096;
pub const MAX_INBOUND_SEARCH_QUERY_BYTES: usize = 4_096;

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
        let normalized = normalize_wishlist_terms(terms);
        Ok(Self {
            terms: normalized,
            options,
            server_interval: None,
            next_index: 0,
        })
    }

    pub fn replace_terms(&mut self, terms: impl IntoIterator<Item = String>) {
        self.terms = normalize_wishlist_terms(terms);
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

fn normalize_wishlist_terms(terms: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::new();
    for term in terms {
        let term = truncate_utf8_bytes(term.trim(), MAX_WISHLIST_SEARCH_TERM_BYTES);
        if term.is_empty()
            || term.chars().any(char::is_control)
            || !seen.insert(term.to_ascii_lowercase())
        {
            continue;
        }
        normalized.push(term);
        if normalized.len() == MAX_WISHLIST_SEARCH_TERMS {
            break;
        }
    }
    normalized
}

fn truncate_utf8_bytes(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value[..boundary].to_owned()
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
        let query = bounded_search_field(query.into(), "query")?;
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
        let username = bounded_search_field(username.into(), "username")?;
        let query = bounded_search_field(query.into(), "query")?;
        self.search_targeted(SearchTarget::User(username), query)
            .await
    }

    pub async fn search_room(
        &mut self,
        room: impl Into<String>,
        query: impl Into<String>,
    ) -> Result<SearchRequestHandle, ClientError> {
        let room = bounded_search_field(room.into(), "room")?;
        let query = bounded_search_field(query.into(), "query")?;
        self.search_targeted(SearchTarget::Room(room), query).await
    }

    pub async fn search_wishlist(
        &mut self,
        query: impl Into<String>,
    ) -> Result<SearchRequestHandle, ClientError> {
        let query = bounded_search_field(query.into(), "query")?;
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

fn bounded_search_field(value: String, field: &'static str) -> Result<String, ClientError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ClientError::BlankSearchField { field });
    }
    if value.chars().any(char::is_control) {
        return Err(ClientError::InvalidSearchField { field });
    }
    if value.len() > MAX_OUTBOUND_SEARCH_FIELD_BYTES {
        return Err(ClientError::SearchFieldTooLong {
            field,
            length: value.len(),
            max: MAX_OUTBOUND_SEARCH_FIELD_BYTES,
        });
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchResults {
    by_token: HashMap<u32, Vec<FileSearchResponse>>,
    text_bytes_by_token: HashMap<u32, usize>,
    stored_files: usize,
    stored_text_bytes: usize,
    stored_responses: usize,
}

impl SearchResults {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accept_peer_message(&mut self, message: PeerMessage) -> Result<bool, ClientError> {
        match message {
            PeerMessage::FileSearchResponse(mut response) => {
                if self.by_token.len() >= MAX_TRACKED_SEARCH_RESULT_TOKENS
                    && !self.by_token.contains_key(&response.token)
                {
                    return Ok(true);
                }
                let responses = self.by_token.get(&response.token);
                if responses
                    .is_some_and(|responses| responses.len() >= MAX_SEARCH_RESPONSES_PER_TOKEN)
                {
                    return Ok(true);
                }
                if self.stored_responses >= MAX_SEARCH_RESPONSES_TOTAL {
                    return Ok(true);
                }
                response.results.truncate(MAX_SEARCH_RESULT_FILES_PER_TOKEN);
                let remaining_per_response =
                    MAX_SEARCH_RESULT_FILES_PER_TOKEN.saturating_sub(response.results.len());
                response.private_results.truncate(remaining_per_response);
                if responses.is_some_and(|responses| responses.contains(&response)) {
                    return Ok(true);
                }
                let stored_files = responses
                    .into_iter()
                    .flatten()
                    .map(|response| response.results.len() + response.private_results.len())
                    .sum::<usize>();
                let mut remaining = MAX_SEARCH_RESULT_FILES_PER_TOKEN
                    .saturating_sub(stored_files)
                    .min(MAX_SEARCH_RESULT_FILES_TOTAL.saturating_sub(self.stored_files));
                response.results.truncate(remaining);
                remaining = remaining.saturating_sub(response.results.len());
                response.private_results.truncate(remaining);
                response.results.shrink_to_fit();
                response.private_results.shrink_to_fit();
                let response_files = response.results.len() + response.private_results.len();
                let response_text_bytes = search_response_text_bytes(&response);
                let stored_text_bytes = self
                    .text_bytes_by_token
                    .get(&response.token)
                    .copied()
                    .unwrap_or(0);
                let Some(total_text_bytes) = stored_text_bytes.checked_add(response_text_bytes)
                else {
                    return Ok(true);
                };
                if total_text_bytes > MAX_SEARCH_RESULT_TEXT_BYTES_PER_TOKEN {
                    return Ok(true);
                }
                let Some(total_stored_text_bytes) =
                    self.stored_text_bytes.checked_add(response_text_bytes)
                else {
                    return Ok(true);
                };
                if total_stored_text_bytes > MAX_SEARCH_RESULT_TEXT_BYTES_TOTAL {
                    return Ok(true);
                }
                self.text_bytes_by_token
                    .insert(response.token, total_text_bytes);
                self.stored_files += response_files;
                self.stored_text_bytes = total_stored_text_bytes;
                self.stored_responses += 1;
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
        let responses = self.by_token.remove(&token).unwrap_or_default();
        let removed_files = responses
            .iter()
            .map(|response| response.results.len() + response.private_results.len())
            .sum::<usize>();
        let removed_text_bytes = self.text_bytes_by_token.remove(&token).unwrap_or(0);
        self.stored_files = self.stored_files.saturating_sub(removed_files);
        self.stored_text_bytes = self.stored_text_bytes.saturating_sub(removed_text_bytes);
        self.stored_responses = self.stored_responses.saturating_sub(responses.len());
        responses
    }

    #[must_use]
    pub fn len_for(&self, token: u32) -> usize {
        self.responses_for(token).len()
    }

    #[must_use]
    pub fn tracked_tokens_len(&self) -> usize {
        self.by_token.len()
    }

    #[must_use]
    pub const fn stored_files_len(&self) -> usize {
        self.stored_files
    }

    #[must_use]
    pub const fn stored_responses_len(&self) -> usize {
        self.stored_responses
    }
}

fn search_response_text_bytes(response: &FileSearchResponse) -> usize {
    response
        .results
        .iter()
        .chain(&response.private_results)
        .fold(response.username.len(), |total, entry| {
            total
                .saturating_add(entry.filename.len())
                .saturating_add(entry.extension.len())
        })
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
        if !self.searches.contains_key(&handle.token)
            && self.searches.len() >= MAX_TRACKED_SEARCH_RESULT_TOKENS
        {
            let _ = self.drain_expired(now);
            if self.searches.len() >= MAX_TRACKED_SEARCH_RESULT_TOKENS {
                let oldest_token = self
                    .searches
                    .iter()
                    .min_by_key(|(token, search)| (search.created_at, **token))
                    .map(|(token, _)| *token);
                if let Some(oldest_token) = oldest_token {
                    self.searches.remove(&oldest_token);
                    let _ = self.results.take(oldest_token);
                }
            }
        }
        let timed = TimedSearch {
            handle,
            created_at: now,
            expires_at: now.checked_add(self.window).unwrap_or_else(|| {
                now.checked_add(Duration::MAX)
                    .unwrap_or_else(|| farthest_deadline(now))
            }),
        };
        let token = timed.handle.token;
        let replaced = self.searches.insert(token, timed);
        if replaced.is_some() {
            let _ = self.results.take(token);
        }
        replaced
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

fn farthest_deadline(now: Instant) -> Instant {
    let mut low = Duration::ZERO;
    let mut high = Duration::MAX;
    while low < high {
        let midpoint = low + (high - low) / 2 + Duration::from_nanos(1);
        if now.checked_add(midpoint).is_some() {
            low = midpoint;
        } else {
            high = midpoint - Duration::from_nanos(1);
        }
    }
    now.checked_add(low).unwrap_or(now)
}

pub trait ShareIndex {
    fn search_limited(&self, query: &str, limit: usize) -> Vec<FileEntry>;

    fn search(&self, query: &str) -> Vec<FileEntry> {
        self.search_limited(query, usize::MAX)
    }
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
    fn search_limited(&self, query: &str, limit: usize) -> Vec<FileEntry> {
        let terms = normalize_terms(query);
        if terms.is_empty() || limit == 0 {
            return Vec::new();
        }

        self.entries
            .iter()
            .filter(|entry| {
                let filename = entry.filename.to_ascii_lowercase();
                terms.iter().all(|term| filename.contains(term))
            })
            .take(limit)
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
    pub fn new(username: impl Into<String>, index: I) -> Result<Self, ClientError> {
        let username = username.into();
        Ok(Self {
            username: normalize_peer_username(&username)?.to_owned(),
            index,
            excluded_filter: ExcludedPhraseFilter::default(),
            average_speed: 0,
            queue_length: 0,
            unknown: 0,
        })
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
        if query.len() > MAX_INBOUND_SEARCH_QUERY_BYTES || query.chars().any(char::is_control) {
            return None;
        }
        if !self.excluded_filter.allows_query(query) {
            return None;
        }

        let results = self
            .index
            .search_limited(query, MAX_SEARCH_RESULT_FILES_PER_TOKEN);
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
