//! Bounded security controls used by the compatibility runtime.
//!
//! The frozen native profile security surface contains several small stateful services
//! rather than one shared security database.  These controls keep their state
//! local, bounded, and redacted at the reporting boundary so the callers can
//! compose them without leaking peer identifiers or secret material.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn digest_hex(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hex::encode(hasher.finalize())
}

fn private_id(value: &str) -> String {
    digest_hex(&[value.as_bytes()])[..16].to_owned()
}

fn bounded_text(value: &str, maximum: usize) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= maximum).then(|| value.to_owned())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DisclosureTrustTier {
    Unknown,
    New,
    Basic,
    Trusted,
    Vetted,
    Friend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisclosureTier {
    Public,
    Standard,
    Limited,
    Restricted,
    Private,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisclosurePermissions {
    pub peer_tier: DisclosureTrustTier,
    pub max_disclosure_tier: DisclosureTier,
    pub can_browse: bool,
    pub can_download: bool,
}

impl DisclosurePermissions {
    pub fn can_access(&self, tier: DisclosureTier) -> bool {
        (tier as u8) <= (self.max_disclosure_tier as u8)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DisclosureStats {
    pub total_peers: usize,
    pub total_positive_interactions: u64,
    pub total_negative_interactions: u64,
}

#[derive(Debug)]
struct DisclosurePeer {
    positive: u32,
    negative: u32,
    tier: DisclosureTrustTier,
}

/// Trust-gated disclosure with the frozen 10,000-peer bound.
#[derive(Debug, Default)]
pub struct DisclosureControl {
    peers: Mutex<HashMap<String, DisclosurePeer>>,
}

impl DisclosureControl {
    pub const MAX_PEERS: usize = 10_000;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn trust_level(&self, username: &str) -> DisclosureTrustTier {
        let Some(username) = bounded_text(username, 256) else {
            return DisclosureTrustTier::Unknown;
        };
        let peers = self.peers.lock().expect("disclosure control lock");
        peers
            .get(&username.to_ascii_lowercase())
            .map(|peer| peer.tier)
            .unwrap_or(DisclosureTrustTier::Unknown)
    }

    pub fn permissions(&self, username: &str) -> DisclosurePermissions {
        let tier = self.trust_level(username);
        match tier {
            DisclosureTrustTier::Unknown => DisclosurePermissions {
                peer_tier: tier,
                max_disclosure_tier: DisclosureTier::Public,
                can_browse: false,
                can_download: false,
            },
            DisclosureTrustTier::New => DisclosurePermissions {
                peer_tier: tier,
                max_disclosure_tier: DisclosureTier::Public,
                can_browse: false,
                can_download: true,
            },
            DisclosureTrustTier::Basic => DisclosurePermissions {
                peer_tier: tier,
                max_disclosure_tier: DisclosureTier::Standard,
                can_browse: true,
                can_download: true,
            },
            DisclosureTrustTier::Trusted => DisclosurePermissions {
                peer_tier: tier,
                max_disclosure_tier: DisclosureTier::Limited,
                can_browse: true,
                can_download: true,
            },
            DisclosureTrustTier::Vetted => DisclosurePermissions {
                peer_tier: tier,
                max_disclosure_tier: DisclosureTier::Restricted,
                can_browse: true,
                can_download: true,
            },
            DisclosureTrustTier::Friend => DisclosurePermissions {
                peer_tier: tier,
                max_disclosure_tier: DisclosureTier::Private,
                can_browse: true,
                can_download: true,
            },
        }
    }

    pub fn record_positive(&self, username: &str, value: u32) -> bool {
        self.record(username, value, true)
    }

    pub fn record_negative(&self, username: &str, value: u32) -> bool {
        self.record(username, value, false)
    }

    fn record(&self, username: &str, value: u32, positive: bool) -> bool {
        let Some(username) = bounded_text(username, 256) else {
            return false;
        };
        let mut peers = self.peers.lock().expect("disclosure control lock");
        if peers.len() >= Self::MAX_PEERS && !peers.contains_key(&username.to_ascii_lowercase()) {
            return false;
        }
        let peer = peers
            .entry(username.to_ascii_lowercase())
            .or_insert(DisclosurePeer {
                positive: 0,
                negative: 0,
                tier: DisclosureTrustTier::New,
            });
        if positive {
            peer.positive = peer.positive.saturating_add(value);
        } else {
            peer.negative = peer.negative.saturating_add(value);
        }
        let score = peer.positive.saturating_sub(peer.negative);
        peer.tier = if score >= 100 {
            DisclosureTrustTier::Friend
        } else if score >= 50 {
            DisclosureTrustTier::Vetted
        } else if score >= 20 {
            DisclosureTrustTier::Trusted
        } else if score >= 5 {
            DisclosureTrustTier::Basic
        } else {
            DisclosureTrustTier::New
        };
        true
    }

    pub fn set_tier(&self, username: &str, tier: DisclosureTrustTier) -> bool {
        let Some(username) = bounded_text(username, 256) else {
            return false;
        };
        let mut peers = self.peers.lock().expect("disclosure control lock");
        if peers.len() >= Self::MAX_PEERS && !peers.contains_key(&username.to_ascii_lowercase()) {
            return false;
        }
        peers
            .entry(username.to_ascii_lowercase())
            .or_insert(DisclosurePeer {
                positive: 0,
                negative: 0,
                tier: DisclosureTrustTier::Unknown,
            })
            .tier = tier;
        true
    }

    pub fn filter_file_tiers(
        &self,
        username: &str,
        files: impl IntoIterator<Item = DisclosureTier>,
    ) -> Vec<DisclosureTier> {
        let permissions = self.permissions(username);
        files
            .into_iter()
            .filter(|tier| permissions.can_access(*tier))
            .collect()
    }

    pub fn stats(&self) -> DisclosureStats {
        let peers = self.peers.lock().expect("disclosure control lock");
        DisclosureStats {
            total_peers: peers.len(),
            total_positive_interactions: peers.values().map(|peer| u64::from(peer.positive)).sum(),
            total_negative_interactions: peers.values().map(|peer| u64::from(peer.negative)).sum(),
        }
    }

    pub fn reset(&self) {
        self.peers.lock().expect("disclosure control lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConsensusStats {
    pub active_sessions: usize,
    pub total_votes: usize,
}

#[derive(Debug)]
struct ConsensusSession {
    expected_hash: Option<String>,
    votes: HashMap<u32, HashMap<String, String>>,
    vote_count: usize,
    finalized: bool,
}

/// Bounded multi-source chunk consensus.
#[derive(Debug, Default)]
pub struct ConsensusControl {
    sessions: Mutex<HashMap<String, ConsensusSession>>,
}

impl ConsensusControl {
    pub const MAX_SESSIONS: usize = 1_000;
    pub const MINIMUM_SOURCES: usize = 3;
    pub const MAX_CHUNKS_PER_SESSION: usize = 4_096;
    pub const MAX_VOTES_PER_SESSION: usize = 4_096;
    pub const MAX_SOURCES_PER_CHUNK: usize = 64;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_session(&self, filename: &str, expected_hash: Option<&str>) -> Option<String> {
        bounded_text(filename, 4_096)?;
        let expected_hash = expected_hash.and_then(|hash| bounded_text(hash, 256));
        let mut sessions = self.sessions.lock().expect("consensus control lock");
        if sessions.len() >= Self::MAX_SESSIONS {
            let completed_id = sessions
                .iter()
                .find_map(|(id, session)| session.finalized.then_some(id.clone()))?;
            sessions.remove(&completed_id);
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        sessions.insert(
            id.clone(),
            ConsensusSession {
                expected_hash,
                votes: HashMap::new(),
                vote_count: 0,
                finalized: false,
            },
        );
        Some(id)
    }

    pub fn submit_vote(&self, session_id: &str, source: &str, chunk: u32, hash: &str) -> bool {
        let Some(source) = bounded_text(source, 256) else {
            return false;
        };
        let Some(hash) = bounded_text(hash, 256) else {
            return false;
        };
        let mut sessions = self.sessions.lock().expect("consensus control lock");
        let Some(session) = sessions.get_mut(session_id) else {
            return false;
        };
        if session.finalized {
            return false;
        }
        let replacing_vote = session
            .votes
            .get(&chunk)
            .is_some_and(|votes| votes.contains_key(&source));
        if !replacing_vote && session.vote_count >= Self::MAX_VOTES_PER_SESSION {
            return false;
        }
        if !session.votes.contains_key(&chunk)
            && session.votes.len() >= Self::MAX_CHUNKS_PER_SESSION
        {
            return false;
        }
        let votes = session.votes.entry(chunk).or_default();
        if !replacing_vote && votes.len() >= Self::MAX_SOURCES_PER_CHUNK {
            return false;
        }
        if votes.insert(source, hash).is_none() {
            session.vote_count += 1;
        }
        true
    }

    pub fn consensus_hash(&self, session_id: &str, chunk: u32) -> Option<String> {
        let sessions = self.sessions.lock().expect("consensus control lock");
        let votes = sessions.get(session_id)?.votes.get(&chunk)?;
        let mut counts = HashMap::<&str, usize>::new();
        for hash in votes.values() {
            *counts.entry(hash.as_str()).or_default() += 1;
        }
        counts
            .into_iter()
            .filter(|(_, count)| *count >= Self::MINIMUM_SOURCES)
            .max_by_key(|(_, count)| *count)
            .map(|(hash, _)| hash.to_owned())
    }

    pub fn finalize(&self, session_id: &str, actual_hash: &str) -> bool {
        let Some(actual_hash) = bounded_text(actual_hash, 256) else {
            return false;
        };
        let mut sessions = self.sessions.lock().expect("consensus control lock");
        let Some(session) = sessions.get_mut(session_id) else {
            return false;
        };
        if session.finalized {
            return false;
        }
        let ok = session
            .expected_hash
            .as_deref()
            .is_none_or(|expected| expected.eq_ignore_ascii_case(&actual_hash));
        session.finalized = true;
        ok
    }

    pub fn stats(&self) -> ConsensusStats {
        let sessions = self.sessions.lock().expect("consensus control lock");
        ConsensusStats {
            active_sessions: sessions
                .values()
                .filter(|session| !session.finalized)
                .count(),
            total_votes: sessions
                .values()
                .flat_map(|session| session.votes.values())
                .map(HashMap::len)
                .sum(),
        }
    }

    pub fn reset(&self) {
        self.sessions
            .lock()
            .expect("consensus control lock")
            .clear();
    }
}

#[derive(Clone, Debug)]
pub struct CanaryRecord {
    pub canary_id: String,
    pub user_id: String,
    pub filename_id: String,
    pub sightings: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CanaryStats {
    pub total_canaries: usize,
    pub canaries_with_sightings: usize,
    pub total_sightings: usize,
}

/// HMAC-like keyed canary registry.  Reports expose only hashed identities.
#[derive(Debug)]
pub struct CanaryControl {
    secret: [u8; 32],
    canaries: Mutex<HashMap<String, CanaryRecord>>,
}

impl Default for CanaryControl {
    fn default() -> Self {
        Self::new()
    }
}

impl CanaryControl {
    pub const MAX_CANARIES: usize = 10_000;

    pub fn new() -> Self {
        let seed = digest_hex(&[b"slskr-canary-secret", &now_seconds().to_le_bytes()]);
        let mut secret = [0_u8; 32];
        secret.copy_from_slice(&hex::decode(seed).expect("sha256 hex has 32 bytes"));
        Self {
            secret,
            canaries: Mutex::new(HashMap::new()),
        }
    }

    pub fn generate(&self, username: &str, filename: &str) -> Option<CanaryRecord> {
        let username = bounded_text(username, 256)?;
        let filename = bounded_text(filename, 4_096)?;
        let day = now_seconds() / 86_400;
        let id = digest_hex(&[
            &self.secret,
            username.as_bytes(),
            filename.as_bytes(),
            &day.to_le_bytes(),
        ])[..16]
            .to_owned();
        let record = CanaryRecord {
            canary_id: id.clone(),
            user_id: private_id(&username),
            filename_id: private_id(&filename),
            sightings: 0,
        };
        let mut canaries = self.canaries.lock().expect("canary control lock");
        if canaries.len() >= Self::MAX_CANARIES && !canaries.contains_key(&id) {
            if let Some(oldest) = canaries.keys().next().cloned() {
                canaries.remove(&oldest);
            }
        }
        canaries.insert(id, record.clone());
        Some(record)
    }

    pub fn report_sighting(&self, canary_id: &str) -> bool {
        let mut canaries = self.canaries.lock().expect("canary control lock");
        let Some(record) = canaries.get_mut(canary_id) else {
            return false;
        };
        record.sightings = record.sightings.saturating_add(1);
        true
    }

    pub fn stats(&self) -> CanaryStats {
        let canaries = self.canaries.lock().expect("canary control lock");
        CanaryStats {
            total_canaries: canaries.len(),
            canaries_with_sightings: canaries
                .values()
                .filter(|record| record.sightings > 0)
                .count(),
            total_sightings: canaries.values().map(|record| record.sightings).sum(),
        }
    }

    pub fn reset(&self) {
        self.canaries.lock().expect("canary control lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CommitmentStats {
    pub total: usize,
    pub pending: usize,
    pub verified: usize,
    pub failed: usize,
}

#[derive(Clone, Debug)]
struct CommitmentRecord {
    hash: String,
    nonce: String,
    verified: bool,
    failed: bool,
}

/// One-time hash commitments for transfer integrity.
#[derive(Debug, Default)]
pub struct CommitmentControl {
    commitments: Mutex<HashMap<String, CommitmentRecord>>,
}

impl CommitmentControl {
    pub const MAX_COMMITMENTS: usize = 10_000;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(
        &self,
        file_hash: &str,
        username: &str,
        filename: &str,
    ) -> Option<(String, String)> {
        bounded_text(file_hash, 256)?;
        bounded_text(username, 256)?;
        bounded_text(filename, 4_096)?;
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let _commitment =
            digest_hex(&[file_hash.to_ascii_lowercase().as_bytes(), nonce.as_bytes()]);
        let id = uuid::Uuid::new_v4().simple().to_string()[..16].to_owned();
        let mut commitments = self.commitments.lock().expect("commitment control lock");
        if commitments.len() >= Self::MAX_COMMITMENTS {
            let completed_id = commitments.iter().find_map(|(id, record)| {
                (record.verified || record.failed).then_some(id.clone())
            })?;
            commitments.remove(&completed_id);
        }
        commitments.insert(
            id.clone(),
            CommitmentRecord {
                hash: file_hash.to_ascii_lowercase(),
                nonce: nonce.clone(),
                verified: false,
                failed: false,
            },
        );
        Some((id, nonce))
    }

    pub fn verify(&self, id: &str, revealed_hash: &str, nonce: &str) -> bool {
        let mut commitments = self.commitments.lock().expect("commitment control lock");
        let Some(record) = commitments.get_mut(id) else {
            return false;
        };
        if record.verified || record.failed {
            return false;
        }
        let computed = digest_hex(&[
            revealed_hash.to_ascii_lowercase().as_bytes(),
            nonce.as_bytes(),
        ]);
        if computed != digest_hex(&[record.hash.as_bytes(), record.nonce.as_bytes()])
            || !revealed_hash.eq_ignore_ascii_case(&record.hash)
        {
            record.failed = true;
            return false;
        }
        record.verified = true;
        true
    }

    pub fn verify_content(&self, id: &str, content_hash: &str) -> bool {
        self.commitments
            .lock()
            .expect("commitment control lock")
            .get(id)
            .is_some_and(|record| record.verified && record.hash.eq_ignore_ascii_case(content_hash))
    }

    pub fn stats(&self) -> CommitmentStats {
        let commitments = self.commitments.lock().expect("commitment control lock");
        CommitmentStats {
            total: commitments.len(),
            pending: commitments
                .values()
                .filter(|record| !record.verified && !record.failed)
                .count(),
            verified: commitments
                .values()
                .filter(|record| record.verified)
                .count(),
            failed: commitments.values().filter(|record| record.failed).count(),
        }
    }

    pub fn reset(&self) {
        self.commitments
            .lock()
            .expect("commitment control lock")
            .clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerificationStats {
    pub active_sessions: usize,
    pub verified_chunks: usize,
    pub failed_chunks: usize,
}

#[derive(Debug)]
struct VerificationSession {
    selected: HashSet<u32>,
    passed: HashSet<u32>,
    failed: HashSet<u32>,
    finalized: bool,
}

/// Bounded probabilistic chunk verification.
#[derive(Debug, Default)]
pub struct VerificationControl {
    sessions: Mutex<HashMap<String, VerificationSession>>,
}

impl VerificationControl {
    pub const MAX_SESSIONS: usize = 1_000;
    pub const MINIMUM_CHUNKS: usize = 3;
    pub const MAXIMUM_CHUNKS: usize = 50;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self, total_chunks: u32, sample_rate: f64) -> Option<String> {
        if total_chunks == 0 || !sample_rate.is_finite() || !(0.0..=1.0).contains(&sample_rate) {
            return None;
        }
        let target = ((f64::from(total_chunks) * sample_rate).ceil() as usize)
            .clamp(Self::MINIMUM_CHUNKS, Self::MAXIMUM_CHUNKS)
            .min(total_chunks as usize);
        let selected = (0..target as u32).collect::<HashSet<_>>();
        let mut sessions = self.sessions.lock().expect("verification control lock");
        if sessions.len() >= Self::MAX_SESSIONS {
            let completed_id = sessions
                .iter()
                .find_map(|(id, session)| session.finalized.then_some(id.clone()))?;
            sessions.remove(&completed_id);
        }
        let id = uuid::Uuid::new_v4().simple().to_string();
        sessions.insert(
            id.clone(),
            VerificationSession {
                selected,
                passed: HashSet::new(),
                failed: HashSet::new(),
                finalized: false,
            },
        );
        Some(id)
    }

    pub fn should_verify(&self, id: &str, chunk: u32) -> bool {
        self.sessions
            .lock()
            .expect("verification control lock")
            .get(id)
            .is_some_and(|session| session.selected.contains(&chunk))
    }

    pub fn record(&self, id: &str, chunk: u32, passed: bool) -> bool {
        let mut sessions = self.sessions.lock().expect("verification control lock");
        let Some(session) = sessions.get_mut(id) else {
            return false;
        };
        if session.finalized || !session.selected.contains(&chunk) {
            return false;
        }
        if passed {
            session.passed.insert(chunk);
        } else {
            session.failed.insert(chunk);
        }
        true
    }

    pub fn finalize(&self, id: &str) -> bool {
        let mut sessions = self.sessions.lock().expect("verification control lock");
        let Some(session) = sessions.get_mut(id) else {
            return false;
        };
        if session.finalized {
            return false;
        }
        session.finalized = true;
        session.failed.is_empty() && session.passed.len() == session.selected.len()
    }

    pub fn stats(&self) -> VerificationStats {
        let sessions = self.sessions.lock().expect("verification control lock");
        VerificationStats {
            active_sessions: sessions
                .values()
                .filter(|session| !session.finalized)
                .count(),
            verified_chunks: sessions.values().map(|session| session.passed.len()).sum(),
            failed_chunks: sessions.values().map(|session| session.failed.len()).sum(),
        }
    }

    pub fn reset(&self) {
        self.sessions
            .lock()
            .expect("verification control lock")
            .clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StorageChallengeStats {
    pub total_challenges: usize,
    pub pending_challenges: usize,
    pub verified_challenges: usize,
    pub failed_challenges: usize,
}

#[derive(Clone, Debug)]
pub struct StorageChallenge {
    pub id: String,
    pub offset: u64,
    pub length: u32,
    pub nonce: String,
}

#[derive(Clone, Debug)]
struct StorageChallengeRecord {
    challenge: StorageChallenge,
    verified: bool,
    failed: bool,
}

/// Bounded proof-of-storage challenge registry.
#[derive(Debug, Default)]
pub struct StorageChallengeControl {
    challenges: Mutex<HashMap<String, StorageChallengeRecord>>,
}

impl StorageChallengeControl {
    pub const DEFAULT_CHALLENGE_SIZE: u32 = 4_096;
    pub const MAX_PENDING_CHALLENGES: usize = 1_000;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(
        &self,
        filename: &str,
        file_size: u64,
        username: &str,
        length: u32,
    ) -> Option<StorageChallenge> {
        bounded_text(filename, 4_096)?;
        bounded_text(username, 256)?;
        if file_size == 0 {
            return None;
        }
        let length = length.max(1).min(file_size.min(u64::from(u32::MAX)) as u32);
        let offset = file_size.saturating_sub(u64::from(length)) / 2;
        let challenge = StorageChallenge {
            id: uuid::Uuid::new_v4().simple().to_string()[..16].to_owned(),
            offset,
            length,
            nonce: uuid::Uuid::new_v4().simple().to_string(),
        };
        let mut challenges = self
            .challenges
            .lock()
            .expect("storage challenge control lock");
        if challenges.len() >= Self::MAX_PENDING_CHALLENGES {
            let completed_id = challenges.iter().find_map(|(id, record)| {
                (record.verified || record.failed).then_some(id.clone())
            })?;
            challenges.remove(&completed_id);
        }
        challenges.insert(
            challenge.id.clone(),
            StorageChallengeRecord {
                challenge: challenge.clone(),
                verified: false,
                failed: false,
            },
        );
        Some(challenge)
    }

    pub fn verify(&self, id: &str, response: &str, expected: &str) -> bool {
        let mut challenges = self
            .challenges
            .lock()
            .expect("storage challenge control lock");
        let Some(record) = challenges.get_mut(id) else {
            return false;
        };
        if record.verified || record.failed {
            return false;
        }
        if response.is_empty() || response != expected {
            record.failed = true;
            return false;
        }
        record.verified = true;
        true
    }

    pub fn stats(&self) -> StorageChallengeStats {
        let challenges = self
            .challenges
            .lock()
            .expect("storage challenge control lock");
        StorageChallengeStats {
            total_challenges: challenges.len(),
            pending_challenges: challenges
                .values()
                .filter(|record| !record.verified && !record.failed)
                .count(),
            verified_challenges: challenges.values().filter(|record| record.verified).count(),
            failed_challenges: challenges.values().filter(|record| record.failed).count(),
        }
    }

    pub fn reset(&self) {
        self.challenges
            .lock()
            .expect("storage challenge control lock")
            .clear();
    }
}

#[derive(Clone, Debug)]
struct MetadataSample {
    size: u64,
    hash: String,
    at: Instant,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TemporalStats {
    pub tracked_files: usize,
    pub tracked_peers: usize,
    pub total_changes: usize,
    pub suspicious_peers: usize,
}

/// Detects implausibly fast or repeated metadata changes.
#[derive(Debug, Default)]
pub struct TemporalConsistencyControl {
    files: Mutex<HashMap<String, VecDeque<MetadataSample>>>,
    suspicious: Mutex<HashSet<String>>,
}

impl TemporalConsistencyControl {
    pub const MAX_HISTORY_PER_FILE: usize = 50;
    pub const MAX_TRACKED_FILES: usize = 10_000;
    pub const MAX_CHANGES_PER_DAY: usize = 10;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, username: &str, filename: &str, size: u64, hash: &str) -> Option<bool> {
        let username = bounded_text(username, 256)?;
        let filename = bounded_text(filename, 4_096)?;
        let hash = bounded_text(hash, 256)?;
        let key = format!(
            "{}:{}",
            username.to_ascii_lowercase(),
            filename.to_ascii_lowercase()
        );
        let mut files = self.files.lock().expect("temporal consistency lock");
        if files.len() >= Self::MAX_TRACKED_FILES && !files.contains_key(&key) {
            return None;
        }
        let history = files.entry(key).or_default();
        let changed = history
            .back()
            .is_some_and(|last| last.size != size || last.hash != hash);
        history.push_back(MetadataSample {
            size,
            hash,
            at: Instant::now(),
        });
        while history.len() > Self::MAX_HISTORY_PER_FILE {
            history.pop_front();
        }
        let recent_changes = history
            .iter()
            .rev()
            .take(Self::MAX_CHANGES_PER_DAY + 1)
            .filter(|sample| sample.at.elapsed() < Duration::from_secs(86_400))
            .count();
        let suspicious = changed && recent_changes > Self::MAX_CHANGES_PER_DAY;
        if suspicious {
            self.suspicious
                .lock()
                .expect("temporal suspicion lock")
                .insert(username.to_ascii_lowercase());
        }
        Some(suspicious)
    }

    pub fn is_suspicious(&self, username: &str) -> bool {
        self.suspicious
            .lock()
            .expect("temporal suspicion lock")
            .contains(&username.to_ascii_lowercase())
    }

    pub fn stats(&self) -> TemporalStats {
        let files = self.files.lock().expect("temporal consistency lock");
        let suspicious = self.suspicious.lock().expect("temporal suspicion lock");
        TemporalStats {
            tracked_files: files.len(),
            tracked_peers: files
                .keys()
                .filter_map(|key| key.split_once(':').map(|(peer, _)| peer))
                .collect::<HashSet<_>>()
                .len(),
            total_changes: files.values().map(VecDeque::len).sum(),
            suspicious_peers: suspicious.len(),
        }
    }

    pub fn reset(&self) {
        self.files
            .lock()
            .expect("temporal consistency lock")
            .clear();
        self.suspicious
            .lock()
            .expect("temporal suspicion lock")
            .clear();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkBudgetConfig {
    pub enabled: bool,
    pub max_units_per_call: u32,
    pub max_units_per_peer_per_minute: u32,
}

impl Default for WorkBudgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_units_per_call: 10,
            max_units_per_peer_per_minute: 50,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkBudgetStats {
    pub tracked_peers: usize,
    pub consumed_units: u32,
    pub near_quota_peers: usize,
}

#[derive(Debug)]
struct WorkWindow {
    started: Instant,
    consumed: u32,
}

/// Per-peer bounded work accounting.
#[derive(Debug)]
pub struct WorkBudgetControl {
    config: WorkBudgetConfig,
    windows: Mutex<HashMap<String, WorkWindow>>,
}

impl WorkBudgetControl {
    pub const MAX_TRACKED_PEERS: usize = 4_096;

    pub fn new(config: WorkBudgetConfig) -> Self {
        Self {
            config,
            windows: Mutex::new(HashMap::new()),
        }
    }

    pub fn try_consume(&self, peer: &str, units: u32) -> bool {
        if !self.config.enabled {
            return true;
        }
        if units == 0 || units > self.config.max_units_per_call {
            return false;
        }
        let Some(peer) = bounded_text(peer, 256) else {
            return false;
        };
        let mut windows = self.windows.lock().expect("work budget lock");
        if windows.len() >= Self::MAX_TRACKED_PEERS && !windows.contains_key(&peer) {
            return false;
        }
        let window = windows.entry(peer).or_insert_with(|| WorkWindow {
            started: Instant::now(),
            consumed: 0,
        });
        if window.started.elapsed() >= Duration::from_secs(60) {
            window.started = Instant::now();
            window.consumed = 0;
        }
        if window.consumed.saturating_add(units) > self.config.max_units_per_peer_per_minute {
            return false;
        }
        window.consumed = window.consumed.saturating_add(units);
        true
    }

    pub fn stats(&self) -> WorkBudgetStats {
        let windows = self.windows.lock().expect("work budget lock");
        let consumed_units = windows.values().map(|window| window.consumed).sum();
        WorkBudgetStats {
            tracked_peers: windows.len(),
            consumed_units,
            near_quota_peers: windows
                .values()
                .filter(|window| {
                    window.consumed
                        >= self.config.max_units_per_peer_per_minute.saturating_mul(4) / 5
                })
                .count(),
        }
    }

    pub fn reset(&self) {
        self.windows.lock().expect("work budget lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FingerprintStats {
    pub total_fingerprints: usize,
    pub active_connections: usize,
    pub total_security_events: usize,
}

#[derive(Clone, Debug)]
struct FingerprintRecord {
    ip_hash: String,
    active: bool,
    events: usize,
}

/// Connection fingerprinting with hashed network identity and bounded events.
#[derive(Debug, Default)]
pub struct ConnectionFingerprintControl {
    fingerprints: Mutex<HashMap<String, FingerprintRecord>>,
    events: Mutex<VecDeque<String>>,
}

impl ConnectionFingerprintControl {
    pub const MAX_FINGERPRINTS: usize = 1_000;
    pub const MAX_EVENT_LOG: usize = 10_000;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_connection(&self, ip: IpAddr, port: u16, protocol: &str) -> Option<String> {
        if port == 0 || protocol.trim().is_empty() || protocol.len() > 64 {
            return None;
        }
        let fingerprint_id = digest_hex(&[
            ip.to_string().as_bytes(),
            &port.to_le_bytes(),
            protocol.as_bytes(),
        ])[..24]
            .to_owned();
        let mut fingerprints = self
            .fingerprints
            .lock()
            .expect("connection fingerprint lock");
        if fingerprints.len() >= Self::MAX_FINGERPRINTS
            && !fingerprints.contains_key(&fingerprint_id)
        {
            return None;
        }
        fingerprints.insert(
            fingerprint_id.clone(),
            FingerprintRecord {
                ip_hash: private_id(&ip.to_string()),
                active: true,
                events: 0,
            },
        );
        Some(fingerprint_id)
    }

    pub fn record_event(&self, fingerprint_id: &str, event: &str) -> bool {
        let Some(event) = bounded_text(event, 256) else {
            return false;
        };
        let mut fingerprints = self
            .fingerprints
            .lock()
            .expect("connection fingerprint lock");
        let Some(record) = fingerprints.get_mut(fingerprint_id) else {
            return false;
        };
        record.events = record.events.saturating_add(1);
        drop(fingerprints);
        let mut events = self.events.lock().expect("fingerprint event lock");
        events.push_back(event);
        while events.len() > Self::MAX_EVENT_LOG {
            events.pop_front();
        }
        true
    }

    pub fn disconnect(&self, fingerprint_id: &str) -> bool {
        self.fingerprints
            .lock()
            .expect("connection fingerprint lock")
            .get_mut(fingerprint_id)
            .is_some_and(|record| {
                record.active = false;
                true
            })
    }

    pub fn stats(&self) -> FingerprintStats {
        let fingerprints = self
            .fingerprints
            .lock()
            .expect("connection fingerprint lock");
        FingerprintStats {
            total_fingerprints: fingerprints.len(),
            active_connections: fingerprints.values().filter(|record| record.active).count(),
            total_security_events: fingerprints.values().map(|record| record.events).sum(),
        }
    }

    pub fn redacted_event_count(&self) -> usize {
        self.events.lock().expect("fingerprint event lock").len()
    }

    pub fn reset(&self) {
        self.fingerprints
            .lock()
            .expect("connection fingerprint lock")
            .clear();
        self.events.lock().expect("fingerprint event lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetworkGuardConfig {
    pub max_connections_per_ip: usize,
    pub max_global_connections: usize,
    pub max_messages_per_minute: usize,
    pub max_message_size: usize,
    pub max_pending_requests_per_ip: usize,
}

impl Default for NetworkGuardConfig {
    fn default() -> Self {
        Self {
            max_connections_per_ip: 100,
            max_global_connections: 100,
            max_messages_per_minute: 60,
            max_message_size: 65_536,
            max_pending_requests_per_ip: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkGuardStats {
    pub global_connections: usize,
    pub tracked_ips: usize,
    pub total_connections: usize,
    pub total_messages: usize,
    pub rate_limit_hits: usize,
}

#[derive(Debug, Default)]
struct NetworkIpState {
    active_connections: usize,
    pending_requests: usize,
    message_times: VecDeque<Instant>,
    total_connections: usize,
    total_messages: usize,
    rate_limit_hits: usize,
}

/// Network connection, message, and pending-request guard.
#[derive(Debug)]
pub struct NetworkGuardControl {
    config: NetworkGuardConfig,
    ips: Mutex<HashMap<IpAddr, NetworkIpState>>,
}

impl Default for NetworkGuardControl {
    fn default() -> Self {
        Self::new(NetworkGuardConfig::default())
    }
}

impl NetworkGuardControl {
    pub const MAX_TRACKED_IPS: usize = 16_384;
    const MAX_MESSAGE_TIMES_PER_IP: usize = 10_000;

    pub fn new(config: NetworkGuardConfig) -> Self {
        let config = NetworkGuardConfig {
            max_messages_per_minute: config
                .max_messages_per_minute
                .min(Self::MAX_MESSAGE_TIMES_PER_IP),
            ..config
        };
        Self {
            config,
            ips: Mutex::new(HashMap::new()),
        }
    }

    pub fn allow_connection(&self, ip: IpAddr) -> bool {
        let ips = self.ips.lock().expect("network guard lock");
        let global = ips
            .values()
            .map(|state| state.active_connections)
            .sum::<usize>();
        let per_ip = ips.get(&ip).map_or(0, |state| state.active_connections);
        global < self.config.max_global_connections && per_ip < self.config.max_connections_per_ip
    }

    pub fn register_connection(&self, ip: IpAddr) -> bool {
        let mut ips = self.ips.lock().expect("network guard lock");
        if !ips.contains_key(&ip) && ips.len() >= Self::MAX_TRACKED_IPS {
            prune_idle_network_ips(&mut ips, Instant::now());
            if ips.len() >= Self::MAX_TRACKED_IPS {
                return false;
            }
        }
        let global = ips
            .values()
            .map(|state| state.active_connections)
            .sum::<usize>();
        let per_ip = ips.get(&ip).map_or(0, |state| state.active_connections);
        if global >= self.config.max_global_connections
            || per_ip >= self.config.max_connections_per_ip
        {
            return false;
        }
        let state = ips.entry(ip).or_default();
        state.active_connections += 1;
        state.total_connections += 1;
        true
    }

    pub fn unregister_connection(&self, ip: IpAddr) {
        if let Some(state) = self.ips.lock().expect("network guard lock").get_mut(&ip) {
            state.active_connections = state.active_connections.saturating_sub(1);
        }
    }

    pub fn allow_message(&self, ip: IpAddr, size: usize) -> bool {
        let now = Instant::now();
        let mut ips = self.ips.lock().expect("network guard lock");
        if !ips.contains_key(&ip) && ips.len() >= Self::MAX_TRACKED_IPS {
            prune_idle_network_ips(&mut ips, now);
            if ips.len() >= Self::MAX_TRACKED_IPS {
                return false;
            }
        }
        let state = ips.entry(ip).or_default();
        while state
            .message_times
            .front()
            .is_some_and(|timestamp| now.duration_since(*timestamp) >= Duration::from_secs(60))
        {
            state.message_times.pop_front();
        }
        if size > self.config.max_message_size
            || state.message_times.len() >= self.config.max_messages_per_minute
        {
            state.rate_limit_hits = state.rate_limit_hits.saturating_add(1);
            return false;
        }
        state.message_times.push_back(now);
        state.total_messages = state.total_messages.saturating_add(1);
        true
    }

    pub fn allow_request(&self, ip: IpAddr) -> bool {
        let mut ips = self.ips.lock().expect("network guard lock");
        if !ips.contains_key(&ip) && ips.len() >= Self::MAX_TRACKED_IPS {
            prune_idle_network_ips(&mut ips, Instant::now());
            if ips.len() >= Self::MAX_TRACKED_IPS {
                return false;
            }
        }
        let state = ips.entry(ip).or_default();
        if state.pending_requests >= self.config.max_pending_requests_per_ip {
            return false;
        }
        state.pending_requests += 1;
        true
    }

    pub fn complete_request(&self, ip: IpAddr) {
        if let Some(state) = self.ips.lock().expect("network guard lock").get_mut(&ip) {
            state.pending_requests = state.pending_requests.saturating_sub(1);
        }
    }

    pub fn stats(&self) -> NetworkGuardStats {
        let ips = self.ips.lock().expect("network guard lock");
        NetworkGuardStats {
            global_connections: ips.values().map(|state| state.active_connections).sum(),
            tracked_ips: ips.len(),
            total_connections: ips.values().map(|state| state.total_connections).sum(),
            total_messages: ips.values().map(|state| state.total_messages).sum(),
            rate_limit_hits: ips.values().map(|state| state.rate_limit_hits).sum(),
        }
    }

    pub fn reset(&self) {
        self.ips.lock().expect("network guard lock").clear();
    }
}

fn prune_idle_network_ips(ips: &mut HashMap<IpAddr, NetworkIpState>, now: Instant) {
    for state in ips.values_mut() {
        while state
            .message_times
            .front()
            .is_some_and(|timestamp| now.duration_since(*timestamp) >= Duration::from_secs(60))
        {
            state.message_times.pop_front();
        }
    }
    ips.retain(|_, state| {
        state.active_connections > 0
            || state.pending_requests > 0
            || !state.message_times.is_empty()
    });
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconnaissanceStats {
    pub tracked_profiles: usize,
    pub known_scanners: usize,
    pub total_events: usize,
}

#[derive(Debug, Default)]
struct ReconProfile {
    attempts: usize,
    failures: usize,
    ports: HashSet<u16>,
    versions: HashSet<String>,
    user_agents: HashSet<String>,
    scanner: bool,
}

/// Detects connection fingerprinting and reconnaissance patterns.
#[derive(Debug, Default)]
pub struct ReconnaissanceControl {
    profiles: Mutex<HashMap<IpAddr, ReconProfile>>,
    events: Mutex<VecDeque<String>>,
}

impl ReconnaissanceControl {
    pub const MAX_PROFILES: usize = 1_000;
    pub const MAX_EVENTS: usize = 5_000;
    const MAX_PROFILE_VALUES: usize = 64;
    const MAX_PROFILE_VALUE_BYTES: usize = 256;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_connection(
        &self,
        ip: IpAddr,
        port: u16,
        protocol: Option<&str>,
        user_agent: Option<&str>,
        succeeded: bool,
    ) -> bool {
        let mut profiles = self.profiles.lock().expect("reconnaissance profile lock");
        if profiles.len() >= Self::MAX_PROFILES && !profiles.contains_key(&ip) {
            return false;
        }
        let profile = profiles.entry(ip).or_default();
        profile.attempts = profile.attempts.saturating_add(1);
        profile.failures = profile.failures.saturating_add(usize::from(!succeeded));
        profile.ports.insert(port);
        if let Some(protocol) = protocol.filter(|value| {
            let value = value.trim();
            !value.is_empty() && value.len() <= Self::MAX_PROFILE_VALUE_BYTES
        }) {
            if profile.versions.len() < Self::MAX_PROFILE_VALUES {
                profile.versions.insert(protocol.trim().to_owned());
            }
        }
        if let Some(user_agent) = user_agent.filter(|value| {
            let value = value.trim();
            !value.is_empty() && value.len() <= Self::MAX_PROFILE_VALUE_BYTES
        }) {
            if profile.user_agents.len() < Self::MAX_PROFILE_VALUES {
                profile.user_agents.insert(user_agent.trim().to_owned());
            }
        }
        profile.scanner = profile.ports.len() > 3
            || profile.versions.len() > 2
            || profile.user_agents.len() > 3
            || (profile.attempts > 5 && profile.failures.saturating_mul(2) > profile.attempts);
        let scanner = profile.scanner;
        drop(profiles);
        if scanner {
            let mut events = self.events.lock().expect("reconnaissance event lock");
            events.push_back(private_id(&ip.to_string()));
            while events.len() > Self::MAX_EVENTS {
                events.pop_front();
            }
        }
        scanner
    }

    pub fn stats(&self) -> ReconnaissanceStats {
        let profiles = self.profiles.lock().expect("reconnaissance profile lock");
        ReconnaissanceStats {
            tracked_profiles: profiles.len(),
            known_scanners: profiles.values().filter(|profile| profile.scanner).count(),
            total_events: self.events.lock().expect("reconnaissance event lock").len(),
        }
    }

    pub fn reset(&self) {
        self.profiles
            .lock()
            .expect("reconnaissance profile lock")
            .clear();
        self.events
            .lock()
            .expect("reconnaissance event lock")
            .clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HoneypotStats {
    pub total_interactions: usize,
    pub known_threats: usize,
    pub event_count: usize,
}

/// Decoy-file and port-probe tracker with hashed threat identities.
#[derive(Debug, Default)]
pub struct HoneypotControl {
    interactions: Mutex<HashMap<String, usize>>,
    events: Mutex<VecDeque<String>>,
}

impl HoneypotControl {
    pub const MAX_EVENTS: usize = 10_000;
    pub const MAX_PROFILES: usize = 1_000;
    const DECOYS: [&'static str; 5] = [
        "slskr_config_backup.zip",
        "admin_credentials.txt",
        "database_dump.sql",
        "private_keys.pem",
        "user_data_export.json",
    ];

    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_honeypot_file(&self, filename: &str) -> bool {
        let filename = filename.to_ascii_lowercase();
        !filename.trim().is_empty() && Self::DECOYS.iter().any(|decoy| filename.contains(decoy))
    }

    pub fn record_interaction(&self, ip: IpAddr, filename: &str) -> bool {
        if !self.is_honeypot_file(filename) {
            return false;
        }
        let key = private_id(&ip.to_string());
        let mut interactions = self.interactions.lock().expect("honeypot profile lock");
        if interactions.len() >= Self::MAX_PROFILES && !interactions.contains_key(&key) {
            return false;
        }
        *interactions.entry(key.clone()).or_default() += 1;
        drop(interactions);
        let mut events = self.events.lock().expect("honeypot event lock");
        events.push_back(key);
        while events.len() > Self::MAX_EVENTS {
            events.pop_front();
        }
        true
    }

    pub fn stats(&self) -> HoneypotStats {
        let interactions = self.interactions.lock().expect("honeypot profile lock");
        HoneypotStats {
            total_interactions: interactions.values().sum(),
            known_threats: interactions.values().filter(|count| **count >= 3).count(),
            event_count: self.events.lock().expect("honeypot event lock").len(),
        }
    }

    pub fn reset(&self) {
        self.interactions
            .lock()
            .expect("honeypot profile lock")
            .clear();
        self.events.lock().expect("honeypot event lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParanoidLevel {
    Log,
    Enforce,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParanoidStats {
    pub total_anomalies: usize,
    pub rejected_inputs: usize,
}

/// Defensive validation of peer-supplied endpoints, paths, counts, and sizes.
#[derive(Debug)]
pub struct ParanoidControl {
    level: Mutex<ParanoidLevel>,
    anomalies: Mutex<VecDeque<String>>,
    rejected: Mutex<usize>,
}

impl Default for ParanoidControl {
    fn default() -> Self {
        Self::new(ParanoidLevel::Log)
    }
}

impl ParanoidControl {
    pub const MAX_ANOMALIES: usize = 1_000;
    pub const MAX_SEARCH_RESULTS: usize = 10_000;
    pub const MAX_PEERS_PER_FILE: usize = 500;
    pub const MAX_MESSAGE_SIZE: usize = 1_048_576;

    pub fn new(level: ParanoidLevel) -> Self {
        Self {
            level: Mutex::new(level),
            anomalies: Mutex::new(VecDeque::new()),
            rejected: Mutex::new(0),
        }
    }

    pub fn validate_endpoint(&self, ip: IpAddr, port: u16) -> bool {
        let suspicious = ip_is_private_or_reserved(ip) || port == 0;
        self.validate(suspicious, "endpoint")
    }

    pub fn validate_search_result_count(&self, count: usize) -> bool {
        self.validate(count > Self::MAX_SEARCH_RESULTS, "search-results")
    }

    pub fn validate_peer_count(&self, count: usize) -> bool {
        self.validate(count > Self::MAX_PEERS_PER_FILE, "peer-count")
    }

    pub fn validate_message_size(&self, size: usize) -> bool {
        self.validate(size > Self::MAX_MESSAGE_SIZE, "message-size")
    }

    pub fn validate_username(&self, username: &str) -> bool {
        self.validate(
            username.trim().is_empty()
                || username.len() > 64
                || username.chars().any(char::is_control),
            "username",
        )
    }

    fn validate(&self, suspicious: bool, kind: &str) -> bool {
        if !suspicious {
            return true;
        }
        let mut anomalies = self.anomalies.lock().expect("paranoid anomaly lock");
        anomalies.push_back(kind.to_owned());
        while anomalies.len() > Self::MAX_ANOMALIES {
            anomalies.pop_front();
        }
        if *self.level.lock().expect("paranoid level lock") == ParanoidLevel::Enforce {
            *self.rejected.lock().expect("paranoid rejection lock") += 1;
            false
        } else {
            true
        }
    }

    pub fn stats(&self) -> ParanoidStats {
        ParanoidStats {
            total_anomalies: self.anomalies.lock().expect("paranoid anomaly lock").len(),
            rejected_inputs: *self.rejected.lock().expect("paranoid rejection lock"),
        }
    }

    pub fn reset(&self) {
        self.anomalies
            .lock()
            .expect("paranoid anomaly lock")
            .clear();
        *self.rejected.lock().expect("paranoid rejection lock") = 0;
    }
}

fn ip_is_private_or_reserved(ip: IpAddr) -> bool {
    let link_local = match ip {
        IpAddr::V4(ip) => ip.is_link_local(),
        IpAddr::V6(ip) => (ip.segments()[0] & 0xffc0) == 0xfe80,
    };
    if ip.is_loopback() || ip.is_unspecified() || link_local {
        return true;
    }
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 169 && octets[1] == 254)
        }
        IpAddr::V6(ip) => (ip.segments()[0] & 0xfe00) == 0xfc00,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EntropyStats {
    pub checks: usize,
    pub warning_checks: usize,
    pub critical_checks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntropyCheck {
    pub entropy: f64,
    pub warning: bool,
    pub critical: bool,
}

/// Bounded entropy monitor for RNG health and key material checks.
#[derive(Debug, Default)]
pub struct EntropyControl {
    history: Mutex<VecDeque<EntropyCheck>>,
}

impl EntropyControl {
    pub const SAMPLE_SIZE: usize = 4_096;
    pub const MAX_HISTORY: usize = 100;
    pub const MIN_ACCEPTABLE_ENTROPY: f64 = 7.0;
    pub const WARNING_ENTROPY: f64 = 7.75;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&self, sample: &[u8]) -> EntropyCheck {
        let mut counts = [0_usize; 256];
        for byte in sample.iter().take(Self::SAMPLE_SIZE) {
            counts[usize::from(*byte)] += 1;
        }
        let length = sample.len().min(Self::SAMPLE_SIZE) as f64;
        let entropy = if length == 0.0 {
            0.0
        } else {
            counts
                .into_iter()
                .filter(|count| *count > 0)
                .map(|count| {
                    let probability = count as f64 / length;
                    -probability * probability.log2()
                })
                .sum()
        };
        let check = EntropyCheck {
            entropy,
            warning: entropy < Self::WARNING_ENTROPY,
            critical: entropy < Self::MIN_ACCEPTABLE_ENTROPY,
        };
        let mut history = self.history.lock().expect("entropy history lock");
        history.push_back(check);
        while history.len() > Self::MAX_HISTORY {
            history.pop_front();
        }
        check
    }

    pub fn stats(&self) -> EntropyStats {
        let history = self.history.lock().expect("entropy history lock");
        EntropyStats {
            checks: history.len(),
            warning_checks: history.iter().filter(|check| check.warning).count(),
            critical_checks: history.iter().filter(|check| check.critical).count(),
        }
    }

    pub fn reset(&self) {
        self.history.lock().expect("entropy history lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DnsSecurityStats {
    pub cached_hosts: usize,
    pub pinned_tunnels: usize,
}

/// Literal-IP validation and tunnel pinning for outbound destinations.
#[derive(Debug, Default)]
pub struct DnsSecurityControl {
    cache: Mutex<HashMap<String, IpAddr>>,
    pins: Mutex<HashMap<String, IpAddr>>,
}

impl DnsSecurityControl {
    pub const MAX_CACHED_HOSTS: usize = 4_096;
    pub const MAX_PINNED_TUNNELS: usize = 4_096;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn resolve_literal(
        &self,
        hostname: &str,
        allow_private: bool,
        allow_public: bool,
    ) -> Result<IpAddr, String> {
        let ip = hostname
            .parse::<IpAddr>()
            .map_err(|_| "hostname resolution is unavailable in this bounded path".to_owned())?;
        let private = ip_is_private_or_reserved(ip);
        if (private && !allow_private) || (!private && !allow_public) {
            return Err("destination address is not allowed".to_owned());
        }
        let key = hostname.to_ascii_lowercase();
        let mut cache = self.cache.lock().expect("DNS security cache lock");
        if !cache.contains_key(&key) && cache.len() >= Self::MAX_CACHED_HOSTS {
            return Err("DNS security cache capacity is full".to_owned());
        }
        cache.insert(key, ip);
        Ok(ip)
    }

    pub fn pin_tunnel(&self, tunnel_id: &str, ip: IpAddr) -> bool {
        let Some(tunnel_id) = bounded_text(tunnel_id, 128) else {
            return false;
        };
        let mut pins = self.pins.lock().expect("DNS security pin lock");
        if !pins.contains_key(&tunnel_id) && pins.len() >= Self::MAX_PINNED_TUNNELS {
            return false;
        }
        pins.insert(tunnel_id, ip);
        true
    }

    pub fn validate_tunnel_ip(&self, tunnel_id: &str, ip: IpAddr) -> bool {
        self.pins
            .lock()
            .expect("DNS security pin lock")
            .get(tunnel_id)
            .is_some_and(|pinned| *pinned == ip)
    }

    pub fn release_tunnel(&self, tunnel_id: &str) -> bool {
        self.pins
            .lock()
            .expect("DNS security pin lock")
            .remove(tunnel_id)
            .is_some()
    }

    pub fn stats(&self) -> DnsSecurityStats {
        DnsSecurityStats {
            cached_hosts: self.cache.lock().expect("DNS security cache lock").len(),
            pinned_tunnels: self.pins.lock().expect("DNS security pin lock").len(),
        }
    }

    pub fn reset(&self) {
        self.cache.lock().expect("DNS security cache lock").clear();
        self.pins.lock().expect("DNS security pin lock").clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CoverTrafficStats {
    pub running: bool,
    pub cover_messages_sent: u64,
    pub real_messages_observed: u64,
}

/// Cover-traffic lifecycle and bounded counters.
#[derive(Debug, Default)]
pub struct CoverTrafficControl {
    state: Mutex<CoverTrafficState>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CoverTrafficState {
    running: bool,
    cover_messages_sent: u64,
    real_messages_observed: u64,
}

impl CoverTrafficControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self) -> bool {
        let mut state = self.state.lock().expect("cover traffic lock");
        if state.running {
            return false;
        }
        state.running = true;
        true
    }

    pub fn stop(&self) -> bool {
        let mut state = self.state.lock().expect("cover traffic lock");
        let was_running = state.running;
        state.running = false;
        was_running
    }

    pub fn notify_real_traffic(&self) {
        self.state
            .lock()
            .expect("cover traffic lock")
            .real_messages_observed += 1;
    }

    pub fn send_cover_message(&self) -> bool {
        let mut state = self.state.lock().expect("cover traffic lock");
        if !state.running {
            return false;
        }
        state.cover_messages_sent = state.cover_messages_sent.saturating_add(1);
        true
    }

    pub fn stats(&self) -> CoverTrafficStats {
        let state = *self.state.lock().expect("cover traffic lock");
        CoverTrafficStats {
            running: state.running,
            cover_messages_sent: state.cover_messages_sent,
            real_messages_observed: state.real_messages_observed,
        }
    }

    pub fn reset(&self) {
        *self.state.lock().expect("cover traffic lock") = CoverTrafficState::default();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrivacyStats {
    pub enabled: bool,
    pub running: bool,
    pub outbound_messages: u64,
    pub inbound_messages: u64,
    pub padding_bytes: u64,
}

/// Message padding/batching lifecycle used by the privacy layer.
#[derive(Debug)]
pub struct PrivacyLayerControl {
    enabled: bool,
    state: Mutex<PrivacyState>,
}

#[derive(Clone, Copy, Debug, Default)]
struct PrivacyState {
    running: bool,
    outbound_messages: u64,
    inbound_messages: u64,
    padding_bytes: u64,
}

impl PrivacyLayerControl {
    pub const MAX_MESSAGE_SIZE: usize = 1_048_576;
    pub const MAX_PADDED_MESSAGE_SIZE: usize = Self::MAX_MESSAGE_SIZE + 4;

    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            state: Mutex::new(PrivacyState::default()),
        }
    }

    pub fn start(&self) -> bool {
        if !self.enabled {
            return false;
        }
        self.state.lock().expect("privacy layer lock").running = true;
        true
    }

    pub fn stop(&self) {
        self.state.lock().expect("privacy layer lock").running = false;
    }

    pub fn transform_outbound(&self, message: &[u8], bucket: usize) -> Option<Vec<u8>> {
        let transformed_len = bucket.checked_add(4)?;
        if !self.enabled
            || message.len() > Self::MAX_MESSAGE_SIZE
            || bucket < message.len()
            || transformed_len > Self::MAX_PADDED_MESSAGE_SIZE
        {
            return None;
        }
        let mut transformed = Vec::with_capacity(transformed_len);
        transformed.extend_from_slice(b"SLP1");
        transformed.extend_from_slice(message);
        transformed.resize(transformed_len, 0);
        let mut state = self.state.lock().expect("privacy layer lock");
        state.outbound_messages = state.outbound_messages.saturating_add(1);
        state.padding_bytes = state
            .padding_bytes
            .saturating_add((bucket - message.len()) as u64);
        Some(transformed)
    }

    pub fn transform_inbound(&self, message: &[u8]) -> Option<Vec<u8>> {
        if !self.enabled
            || message.len() < 4
            || message.len() > Self::MAX_PADDED_MESSAGE_SIZE
            || &message[..4] != b"SLP1"
        {
            return None;
        }
        let mut state = self.state.lock().expect("privacy layer lock");
        state.inbound_messages = state.inbound_messages.saturating_add(1);
        Some(message[4..].to_vec())
    }

    pub fn stats(&self) -> PrivacyStats {
        let state = *self.state.lock().expect("privacy layer lock");
        PrivacyStats {
            enabled: self.enabled,
            running: state.running,
            outbound_messages: state.outbound_messages,
            inbound_messages: state.inbound_messages,
            padding_bytes: state.padding_bytes,
        }
    }

    pub fn reset(&self) {
        *self.state.lock().expect("privacy layer lock") = PrivacyState::default();
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransportKind {
    Direct,
    Tor,
    I2p,
    WebSocket,
    HttpTunnel,
    Meek,
    Obfs4,
    RelayOnly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransportStats {
    pub enabled_transports: usize,
    pub available_transports: usize,
    pub selected: Option<TransportKind>,
}

/// Capability/status projection for direct and optional anonymity transports.
#[derive(Debug)]
pub struct TransportControl {
    availability: Mutex<HashMap<TransportKind, bool>>,
}

impl Default for TransportControl {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportControl {
    pub fn new() -> Self {
        let mut availability = HashMap::new();
        availability.insert(TransportKind::Direct, true);
        for kind in [
            TransportKind::Tor,
            TransportKind::I2p,
            TransportKind::WebSocket,
            TransportKind::HttpTunnel,
            TransportKind::Meek,
            TransportKind::Obfs4,
            TransportKind::RelayOnly,
        ] {
            availability.insert(kind, false);
        }
        Self {
            availability: Mutex::new(availability),
        }
    }

    pub fn set_available(&self, kind: TransportKind, available: bool) {
        self.availability
            .lock()
            .expect("transport availability lock")
            .insert(kind, available);
    }

    pub fn is_available(&self, kind: TransportKind) -> bool {
        self.availability
            .lock()
            .expect("transport availability lock")
            .get(&kind)
            .copied()
            .unwrap_or(false)
    }

    pub fn select(&self, prefer_private: bool) -> Option<TransportKind> {
        let availability = self
            .availability
            .lock()
            .expect("transport availability lock");
        let order = if prefer_private {
            [
                TransportKind::Tor,
                TransportKind::I2p,
                TransportKind::RelayOnly,
                TransportKind::WebSocket,
                TransportKind::Direct,
            ]
        } else {
            [
                TransportKind::Direct,
                TransportKind::Tor,
                TransportKind::I2p,
                TransportKind::RelayOnly,
                TransportKind::WebSocket,
            ]
        };
        order
            .into_iter()
            .find(|kind| availability.get(kind).copied().unwrap_or(false))
    }

    pub fn validate_target(&self, host: &str, port: u16) -> bool {
        !host.trim().is_empty() && host.len() <= 255 && port > 0
    }

    pub fn stats(&self, prefer_private: bool) -> TransportStats {
        let availability = self
            .availability
            .lock()
            .expect("transport availability lock");
        TransportStats {
            enabled_transports: availability.len(),
            available_transports: availability
                .values()
                .filter(|available| **available)
                .count(),
            selected: {
                let order = if prefer_private {
                    [
                        TransportKind::Tor,
                        TransportKind::I2p,
                        TransportKind::RelayOnly,
                        TransportKind::Direct,
                    ]
                } else {
                    [
                        TransportKind::Direct,
                        TransportKind::Tor,
                        TransportKind::I2p,
                        TransportKind::RelayOnly,
                    ]
                };
                order
                    .into_iter()
                    .find(|kind| availability.get(kind).copied().unwrap_or(false))
            },
        }
    }

    pub fn reset(&self) {
        let mut availability = self
            .availability
            .lock()
            .expect("transport availability lock");
        availability.clear();
        availability.insert(TransportKind::Direct, true);
        for kind in [
            TransportKind::Tor,
            TransportKind::I2p,
            TransportKind::WebSocket,
            TransportKind::HttpTunnel,
            TransportKind::Meek,
            TransportKind::Obfs4,
            TransportKind::RelayOnly,
        ] {
            availability.insert(kind, false);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub reason: String,
}

/// Short-circuit security policy composition.
#[derive(Debug, Default)]
pub struct PolicyControl {
    blocked_peers: Mutex<HashSet<String>>,
}

impl PolicyControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block_peer(&self, peer: &str) -> bool {
        let Some(peer) = bounded_text(peer, 256) else {
            return false;
        };
        self.blocked_peers
            .lock()
            .expect("policy control lock")
            .insert(peer.to_ascii_lowercase());
        true
    }

    pub fn evaluate(
        &self,
        peer: &str,
        operation: &str,
        content_id: Option<&str>,
    ) -> PolicyDecision {
        if peer.trim().is_empty() {
            return PolicyDecision {
                allowed: false,
                reason: "peer identity is required".to_owned(),
            };
        }
        if self
            .blocked_peers
            .lock()
            .expect("policy control lock")
            .contains(&peer.to_ascii_lowercase())
        {
            return PolicyDecision {
                allowed: false,
                reason: "peer is blocked".to_owned(),
            };
        }
        if operation.to_ascii_lowercase().contains("consensus") {
            return PolicyDecision {
                allowed: false,
                reason: "consensus verification unavailable".to_owned(),
            };
        }
        if content_id.is_some_and(|content| content.eq_ignore_ascii_case("known-bad")) {
            return PolicyDecision {
                allowed: false,
                reason: "content is unsafe".to_owned(),
            };
        }
        PolicyDecision {
            allowed: true,
            reason: "all policies passed".to_owned(),
        }
    }

    pub fn stats(&self) -> usize {
        self.blocked_peers
            .lock()
            .expect("policy control lock")
            .len()
    }

    pub fn reset(&self) {
        self.blocked_peers
            .lock()
            .expect("policy control lock")
            .clear();
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShadowRateLimiterStats {
    pub operations_last_minute: usize,
    pub max_operations_per_minute: usize,
}

/// Sliding-window DHT write limiter for the virtual Soulfind shadow index.
#[derive(Debug)]
pub struct ShadowRateLimiter {
    maximum: usize,
    operations: Mutex<VecDeque<Instant>>,
}

impl Default for ShadowRateLimiter {
    fn default() -> Self {
        Self::new(60)
    }
}

impl ShadowRateLimiter {
    pub fn new(maximum: usize) -> Self {
        Self {
            maximum: maximum.max(1),
            operations: Mutex::new(VecDeque::new()),
        }
    }

    pub fn try_acquire(&self) -> bool {
        let now = Instant::now();
        let mut operations = self.operations.lock().expect("shadow rate limiter lock");
        while operations
            .front()
            .is_some_and(|timestamp| now.duration_since(*timestamp) >= Duration::from_secs(60))
        {
            operations.pop_front();
        }
        if operations.len() >= self.maximum {
            return false;
        }
        operations.push_back(now);
        true
    }

    pub fn release(&self) {}

    pub fn stats(&self) -> ShadowRateLimiterStats {
        let mut operations = self.operations.lock().expect("shadow rate limiter lock");
        let now = Instant::now();
        while operations
            .front()
            .is_some_and(|timestamp| now.duration_since(*timestamp) >= Duration::from_secs(60))
        {
            operations.pop_front();
        }
        ShadowRateLimiterStats {
            operations_last_minute: operations.len(),
            max_operations_per_minute: self.maximum,
        }
    }

    pub fn reset(&self) {
        self.operations
            .lock()
            .expect("shadow rate limiter lock")
            .clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommitmentControl, ConsensusControl, PrivacyLayerControl, StorageChallengeControl,
        VerificationControl, WorkBudgetConfig, WorkBudgetControl,
    };

    #[test]
    fn consensus_bounds_votes_and_reclaims_finalized_sessions() {
        let consensus = ConsensusControl::new();
        let session = consensus
            .start_session("bounded.flac", None)
            .expect("consensus session");
        for source in 0..ConsensusControl::MAX_SOURCES_PER_CHUNK {
            assert!(consensus.submit_vote(&session, &format!("source-{source}"), 0, "chunk-hash"));
        }
        assert!(!consensus.submit_vote(&session, "source-over-capacity", 0, "chunk-hash"));
        assert!(consensus.finalize(&session, "actual-hash"));

        for index in 1..ConsensusControl::MAX_SESSIONS {
            assert!(consensus
                .start_session(&format!("active-{index}.flac"), None)
                .is_some());
        }
        assert!(consensus.start_session("reclaimed.flac", None).is_some());
    }

    #[test]
    fn completed_security_records_release_capacity() {
        let commitment = CommitmentControl::new();
        let (first_commitment, first_nonce) = commitment
            .create("hash", "peer", "file")
            .expect("commitment");
        assert!(commitment.verify(&first_commitment, "hash", &first_nonce));
        for index in 1..CommitmentControl::MAX_COMMITMENTS {
            assert!(commitment
                .create(&format!("hash-{index}"), "peer", "file")
                .is_some());
        }
        assert!(commitment.create("reclaimed", "peer", "file").is_some());

        let verification = VerificationControl::new();
        let first_verification = verification.start(4, 0.5).expect("verification");
        assert!(!verification.finalize(&first_verification));
        for _ in 1..VerificationControl::MAX_SESSIONS {
            assert!(verification.start(4, 0.5).is_some());
        }
        assert!(verification.start(4, 0.5).is_some());

        let storage = StorageChallengeControl::new();
        let first_challenge = storage
            .create("file", 4_096, "peer", 1)
            .expect("storage challenge");
        assert!(storage.verify(&first_challenge.id, "response", "response"));
        for index in 1..StorageChallengeControl::MAX_PENDING_CHALLENGES {
            assert!(storage
                .create(&format!("file-{index}"), 4_096, "peer", 1)
                .is_some());
        }
        assert!(storage.create("reclaimed", 4_096, "peer", 1).is_some());
    }

    #[test]
    fn work_budget_near_quota_check_handles_maximum_configuration() {
        let budget = WorkBudgetControl::new(WorkBudgetConfig {
            max_units_per_call: u32::MAX,
            max_units_per_peer_per_minute: u32::MAX,
            ..WorkBudgetConfig::default()
        });

        assert!(budget.try_consume("peer", u32::MAX));
        assert_eq!(budget.stats().near_quota_peers, 1);
    }

    #[test]
    fn privacy_layer_rejects_unrepresentable_or_oversized_frames() {
        let privacy = PrivacyLayerControl::new(true);
        assert!(privacy.start());

        assert!(privacy.transform_outbound(b"message", usize::MAX).is_none());
        assert!(privacy
            .transform_inbound(&vec![
                0_u8;
                PrivacyLayerControl::MAX_PADDED_MESSAGE_SIZE + 1
            ])
            .is_none());
    }
}
