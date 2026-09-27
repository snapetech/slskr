use super::*;

pub(super) async fn pod_request_peer_id(state: &AppState) -> Option<String> {
    let runtime_peer = state
        .runtime_credentials
        .read()
        .await
        .as_ref()
        .map(|credentials| credentials.username.trim().to_owned())
        .filter(|value| !value.is_empty());
    let configured_peer = state
        .configured_credentials
        .read()
        .await
        .as_ref()
        .map(|credentials| credentials.username.trim().to_owned())
        .filter(|value| !value.is_empty());
    runtime_peer
        .or(configured_peer)
        .or_else(|| (!state.config.auth_required).then(|| "Anonymous".to_owned()))
}

/// True when this instance's own configured Soulseek identity
/// (`pod_request_peer_id`) holds owner/moderator role in `pod_id`.
pub(super) async fn pod_local_peer_can_moderate(state: &AppState, pod_id: &str) -> bool {
    let acting_peer_id = pod_request_peer_id(state).await;
    let pods = state.pods.read().await;
    acting_peer_id
        .as_deref()
        .is_some_and(|acting| pods.can_moderate(pod_id, acting))
}

pub(super) fn gold_star_club_available(state: &AppState) -> bool {
    pods::gold_star_club_available_with_setting(
        &state.config.state_dir,
        state.config.advanced_networking.gold_star_club_autojoin,
    )
}

const MAX_POD_JOIN_REPLAY_RECORDS: usize = 4_096;
const POD_JOIN_REPLAY_TTL_SECONDS: u64 = 300;

#[derive(Debug, Default)]
pub(super) struct PodJoinReplayStore {
    expirations: BTreeMap<String, u64>,
}

impl PodJoinReplayStore {
    pub(super) fn reserve(
        &mut self,
        input: &PodJoinSignatureInput,
        now: u64,
    ) -> Result<(), String> {
        if input.nonce.trim().is_empty() {
            return Err("pod join nonce is required when signature mode is enforce".to_owned());
        }
        let replay_key = serde_json::to_string(&(&input.pod_id, &input.peer_id, &input.nonce))
            .expect("pod join replay tuple is serializable");
        self.expirations.retain(|_, expires_at| *expires_at > now);
        if self.expirations.contains_key(&replay_key) {
            return Err("pod join nonce has already been used".to_owned());
        }
        if self.expirations.len() >= MAX_POD_JOIN_REPLAY_RECORDS {
            return Err("pod join replay cache is full".to_owned());
        }
        self.expirations
            .insert(replay_key, now.saturating_add(POD_JOIN_REPLAY_TTL_SECONDS));
        Ok(())
    }

    pub(super) fn release(&mut self, input: &PodJoinSignatureInput) {
        let replay_key = serde_json::to_string(&(&input.pod_id, &input.peer_id, &input.nonce))
            .expect("pod join replay tuple is serializable");
        self.expirations.remove(&replay_key);
    }
}

pub(super) fn pod_pending_request_path(path: &str, operation: &str) -> Option<String> {
    let prefix = format!("/api/podcore/membership/{operation}/pending/");
    let encoded = path.strip_prefix(&prefix)?;
    if encoded.is_empty() || encoded.contains('/') {
        return None;
    }
    let decoded = decoded_path_segment(encoded).trim().to_owned();
    (!decoded.is_empty()).then_some(decoded)
}

pub(super) fn pod_pending_request_has_blank_id(path: &str, operation: &str) -> bool {
    let prefix = format!("/api/podcore/membership/{operation}/pending/");
    let Some(encoded) = path.strip_prefix(&prefix) else {
        return false;
    };
    !encoded.is_empty() && !encoded.contains('/') && decoded_path_segment(encoded).trim().is_empty()
}

pub(super) fn pod_cancel_request_path(path: &str, operation: &str) -> Option<(String, String)> {
    let prefix = format!("/api/podcore/membership/{operation}/");
    let mut segments = path.strip_prefix(&prefix)?.split('/');
    let pod_id = decoded_path_segment(segments.next()?).trim().to_owned();
    let peer_id = decoded_path_segment(segments.next()?).trim().to_owned();
    if pod_id.is_empty() || peer_id.is_empty() || segments.next().is_some() {
        return None;
    }
    Some((pod_id, peer_id))
}

pub(super) async fn pod_acceptor_has_permission(
    state: &AppState,
    pod_id: &str,
    peer_id: &str,
) -> bool {
    if peer_id.trim().is_empty() {
        return false;
    }
    let role_has_permission = {
        let workflow = state.pod_membership_workflow.read().await;
        matches!(workflow.role(pod_id, peer_id), "owner" | "moderator")
    };
    if role_has_permission {
        return true;
    }
    let configured_username = state.config.username.as_deref().unwrap_or_default();
    state
        .rooms
        .read()
        .await
        .records
        .iter()
        .find(|room| room.name == pod_id)
        .is_some_and(|room| room.operated && peer_id.eq_ignore_ascii_case(configured_username))
}

#[cfg(test)]
mod replay_store_tests {
    use super::{PodJoinReplayStore, PodJoinSignatureInput};

    fn join_input(nonce: &str) -> PodJoinSignatureInput {
        PodJoinSignatureInput {
            pod_id: "pod-one".to_owned(),
            peer_id: "peer-one".to_owned(),
            requested_role: "member".to_owned(),
            timestamp_unix_ms: 1,
            message: String::new(),
            nonce: nonce.to_owned(),
            signature: String::new(),
            public_key: String::new(),
        }
    }

    #[test]
    fn replay_store_rejects_duplicate_nonce_until_released() {
        let mut store = PodJoinReplayStore::default();
        let input = join_input("nonce-one");

        assert_eq!(store.reserve(&input, 10), Ok(()));
        assert_eq!(
            store.reserve(&input, 10),
            Err("pod join nonce has already been used".to_owned())
        );
        store.release(&input);
        assert_eq!(store.reserve(&input, 10), Ok(()));
    }

    #[test]
    fn replay_store_expires_keys_at_the_ttl_boundary() {
        let mut store = PodJoinReplayStore::default();
        let input = join_input("nonce-one");

        assert_eq!(store.reserve(&input, 10), Ok(()));
        assert_eq!(store.reserve(&input, 310), Ok(()));
    }

    #[test]
    fn replay_store_requires_nonce_and_enforces_capacity() {
        let mut store = PodJoinReplayStore::default();
        assert_eq!(
            store.reserve(&join_input("  "), 10),
            Err("pod join nonce is required when signature mode is enforce".to_owned())
        );

        for index in 0..super::MAX_POD_JOIN_REPLAY_RECORDS {
            assert_eq!(
                store.reserve(&join_input(&format!("nonce-{index}")), 10),
                Ok(())
            );
        }
        assert_eq!(
            store.reserve(&join_input("nonce-over-capacity"), 10),
            Err("pod join replay cache is full".to_owned())
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::{
        pod_cancel_request_path, pod_pending_request_has_blank_id, pod_pending_request_path,
    };

    #[test]
    fn pending_request_path_decodes_one_nonblank_segment() {
        assert_eq!(
            pod_pending_request_path("/api/podcore/membership/join/pending/pod%3Aone", "join"),
            Some("pod:one".to_owned())
        );
        assert_eq!(
            pod_pending_request_path("/api/podcore/membership/join/pending/one/two", "join"),
            None
        );
        assert_eq!(
            pod_pending_request_path("/api/podcore/membership/join/pending/%20", "join"),
            None
        );
    }

    #[test]
    fn pending_request_path_recognizes_encoded_blank_ids() {
        assert!(pod_pending_request_has_blank_id(
            "/api/podcore/membership/leave/pending/%20",
            "leave"
        ));
        assert!(!pod_pending_request_has_blank_id(
            "/api/podcore/membership/leave/pending/",
            "leave"
        ));
    }

    #[test]
    fn cancel_request_path_requires_exactly_two_nonblank_segments() {
        assert_eq!(
            pod_cancel_request_path(
                "/api/podcore/membership/leave/pod%3Aone/peer%2Done",
                "leave"
            ),
            Some(("pod:one".to_owned(), "peer-one".to_owned()))
        );
        assert_eq!(
            pod_cancel_request_path(
                "/api/podcore/membership/leave/pod%3Aone/peer-one/extra",
                "leave"
            ),
            None
        );
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PodJoinSignatureInput {
    pub(super) pod_id: String,
    pub(super) peer_id: String,
    pub(super) requested_role: String,
    pub(super) timestamp_unix_ms: u64,
    pub(super) message: String,
    pub(super) nonce: String,
    pub(super) signature: String,
    pub(super) public_key: String,
}

impl PodJoinSignatureInput {
    pub(super) fn from_json(body: &str) -> Result<Self, String> {
        serde_json::from_str::<serde_json::Value>(body)
            .map_err(|error| format!("invalid pod join JSON: {error}"))?;
        let input = Self {
            pod_id: extract_json_string_field(body, "podId")
                .or_else(|| extract_json_string_field(body, "pod"))
                .or_else(|| extract_json_string_field(body, "room"))
                .unwrap_or_default()
                .trim()
                .to_owned(),
            peer_id: extract_json_string_field(body, "peerId")
                .unwrap_or_default()
                .trim()
                .to_owned(),
            requested_role: extract_json_string_field(body, "requestedRole")
                .unwrap_or_else(|| "member".to_owned())
                .trim()
                .to_owned(),
            timestamp_unix_ms: extract_json_u64_field(body, "timestampUnixMs").unwrap_or(0),
            message: extract_json_string_field(body, "message")
                .unwrap_or_default()
                .trim()
                .to_owned(),
            nonce: extract_json_string_field(body, "nonce")
                .unwrap_or_default()
                .trim()
                .to_owned(),
            signature: extract_json_string_field(body, "signature")
                .unwrap_or_default()
                .trim()
                .to_owned(),
            public_key: extract_json_string_field(body, "publicKey")
                .unwrap_or_default()
                .trim()
                .to_owned(),
        };
        for (name, value, limit) in [
            ("podId", input.pod_id.as_str(), 512usize),
            ("peerId", input.peer_id.as_str(), 512),
            ("requestedRole", input.requested_role.as_str(), 128),
            ("message", input.message.as_str(), 4 * 1024),
            ("nonce", input.nonce.as_str(), 512),
            ("signature", input.signature.as_str(), 1024),
            ("publicKey", input.public_key.as_str(), 512),
        ] {
            if value.len() > limit {
                return Err(format!("pod join {name} exceeds {limit} bytes"));
            }
        }
        if input.pod_id.trim().is_empty() {
            return Err("Valid join request with PodId and PeerId is required".to_owned());
        }
        if input.peer_id.trim().is_empty() {
            return Err("Valid join request with PodId and PeerId is required".to_owned());
        }
        Ok(input)
    }

    pub(super) fn canonical_payload(&self) -> String {
        serde_json::to_string(&(
            1,
            "join-request",
            &self.pod_id,
            &self.peer_id,
            &self.requested_role,
            self.timestamp_unix_ms,
            &self.message,
            &self.nonce,
        ))
        .expect("pod join request canonical tuple is serializable")
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PodJoinAcceptanceInput {
    pub(super) pod_id: String,
    pub(super) peer_id: String,
    pub(super) accepted_role: String,
    pub(super) acceptor_peer_id: String,
    pub(super) acceptor_public_key: String,
    pub(super) timestamp_unix_ms: u64,
    pub(super) signature: String,
    pub(super) message: String,
}

impl PodJoinAcceptanceInput {
    pub(super) fn from_json(body: &str) -> Result<Self, String> {
        serde_json::from_str::<serde_json::Value>(body)
            .map_err(|error| format!("invalid pod join acceptance JSON: {error}"))?;
        let input = Self {
            pod_id: pod_string_field(body, "podId"),
            peer_id: pod_string_field(body, "peerId"),
            accepted_role: extract_json_string_field(body, "acceptedRole")
                .unwrap_or_else(|| "member".to_owned())
                .trim()
                .to_owned(),
            acceptor_peer_id: pod_string_field(body, "acceptorPeerId"),
            acceptor_public_key: pod_string_field(body, "acceptorPublicKey"),
            timestamp_unix_ms: extract_json_u64_field(body, "timestampUnixMs").unwrap_or(0),
            signature: pod_string_field(body, "signature"),
            message: pod_string_field(body, "message"),
        };
        validate_pod_fields(
            "pod join acceptance",
            &[
                ("podId", &input.pod_id, 512),
                ("peerId", &input.peer_id, 512),
                ("acceptedRole", &input.accepted_role, 128),
                ("acceptorPeerId", &input.acceptor_peer_id, 512),
                ("acceptorPublicKey", &input.acceptor_public_key, 512),
                ("signature", &input.signature, 1024),
                ("message", &input.message, 4 * 1024),
            ],
        )?;
        require_pod_and_peer("pod join acceptance", &input.pod_id, &input.peer_id)?;
        Ok(input)
    }

    pub(super) fn canonical_payload(&self) -> String {
        serde_json::to_string(&(
            1,
            "join-acceptance",
            &self.pod_id,
            &self.peer_id,
            &self.accepted_role,
            &self.acceptor_peer_id,
            self.timestamp_unix_ms,
            &self.message,
        ))
        .expect("pod join acceptance canonical tuple is serializable")
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PodLeaveRequestInput {
    pub(super) pod_id: String,
    pub(super) peer_id: String,
    pub(super) public_key: String,
    pub(super) timestamp_unix_ms: u64,
    pub(super) signature: String,
    pub(super) message: String,
}

impl PodLeaveRequestInput {
    pub(super) fn from_json(body: &str) -> Result<Self, String> {
        serde_json::from_str::<serde_json::Value>(body)
            .map_err(|error| format!("invalid pod leave JSON: {error}"))?;
        let input = Self {
            pod_id: pod_string_field(body, "podId"),
            peer_id: pod_string_field(body, "peerId"),
            public_key: pod_string_field(body, "publicKey"),
            timestamp_unix_ms: extract_json_u64_field(body, "timestampUnixMs").unwrap_or(0),
            signature: pod_string_field(body, "signature"),
            message: pod_string_field(body, "message"),
        };
        validate_pod_fields(
            "pod leave",
            &[
                ("podId", &input.pod_id, 512),
                ("peerId", &input.peer_id, 512),
                ("publicKey", &input.public_key, 512),
                ("signature", &input.signature, 1024),
                ("message", &input.message, 4 * 1024),
            ],
        )?;
        require_pod_and_peer("pod leave", &input.pod_id, &input.peer_id)?;
        Ok(input)
    }

    pub(super) fn canonical_payload(&self) -> String {
        serde_json::to_string(&(
            1,
            "leave-request",
            &self.pod_id,
            &self.peer_id,
            self.timestamp_unix_ms,
            &self.message,
        ))
        .expect("pod leave request canonical tuple is serializable")
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PodLeaveAcceptanceInput {
    pub(super) pod_id: String,
    pub(super) peer_id: String,
    pub(super) acceptor_peer_id: String,
    pub(super) acceptor_public_key: String,
    pub(super) timestamp_unix_ms: u64,
    pub(super) signature: String,
    pub(super) message: String,
}

impl PodLeaveAcceptanceInput {
    pub(super) fn from_json(body: &str) -> Result<Self, String> {
        serde_json::from_str::<serde_json::Value>(body)
            .map_err(|error| format!("invalid pod leave acceptance JSON: {error}"))?;
        let input = Self {
            pod_id: pod_string_field(body, "podId"),
            peer_id: pod_string_field(body, "peerId"),
            acceptor_peer_id: pod_string_field(body, "acceptorPeerId"),
            acceptor_public_key: pod_string_field(body, "acceptorPublicKey"),
            timestamp_unix_ms: extract_json_u64_field(body, "timestampUnixMs").unwrap_or(0),
            signature: pod_string_field(body, "signature"),
            message: pod_string_field(body, "message"),
        };
        validate_pod_fields(
            "pod leave acceptance",
            &[
                ("podId", &input.pod_id, 512),
                ("peerId", &input.peer_id, 512),
                ("acceptorPeerId", &input.acceptor_peer_id, 512),
                ("acceptorPublicKey", &input.acceptor_public_key, 512),
                ("signature", &input.signature, 1024),
                ("message", &input.message, 4 * 1024),
            ],
        )?;
        require_pod_and_peer("pod leave acceptance", &input.pod_id, &input.peer_id)?;
        Ok(input)
    }

    pub(super) fn canonical_payload(&self) -> String {
        serde_json::to_string(&(
            1,
            "leave-acceptance",
            &self.pod_id,
            &self.peer_id,
            &self.acceptor_peer_id,
            self.timestamp_unix_ms,
            &self.message,
        ))
        .expect("pod leave acceptance canonical tuple is serializable")
    }
}

fn pod_string_field(body: &str, field: &str) -> String {
    extract_json_string_field(body, field)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn validate_pod_fields(operation: &str, fields: &[(&str, &String, usize)]) -> Result<(), String> {
    for (name, value, limit) in fields {
        if value.len() > *limit {
            return Err(format!("{operation} {name} exceeds {limit} bytes"));
        }
    }
    Ok(())
}

fn require_pod_and_peer(operation: &str, pod_id: &str, peer_id: &str) -> Result<(), String> {
    if pod_id.is_empty() || peer_id.is_empty() {
        let error = match operation {
            "pod join acceptance" => "Valid acceptance with PodId and PeerId is required",
            "pod leave" => "Valid leave request with PodId and PeerId is required",
            "pod leave acceptance" => "Valid acceptance with PodId and PeerId is required",
            _ => "Valid PodId and PeerId are required",
        };
        return Err(error.to_owned());
    }
    Ok(())
}

#[derive(Debug, Default)]
pub(super) struct PodMembershipWorkflowStore {
    pending_joins: BTreeMap<String, PodJoinSignatureInput>,
    pending_leaves: BTreeMap<String, PodLeaveRequestInput>,
    roles: BTreeMap<String, String>,
}

impl PodMembershipWorkflowStore {
    pub(super) fn key(pod_id: &str, peer_id: &str) -> String {
        serde_json::to_string(&(
            pod_id.trim().to_ascii_lowercase(),
            peer_id.trim().to_ascii_lowercase(),
        ))
        .expect("pod membership key is serializable")
    }

    pub(super) fn total_pending(&self) -> usize {
        self.pending_joins.len() + self.pending_leaves.len()
    }

    pub(super) fn add_join(&mut self, request: PodJoinSignatureInput) -> Result<(), &'static str> {
        let key = Self::key(&request.pod_id, &request.peer_id);
        if self.pending_joins.contains_key(&key) {
            return Err("Pending join request already exists");
        }
        if self.total_pending() >= MAX_POD_PENDING_MEMBERSHIP_RECORDS {
            return Err("Pending membership request capacity is full");
        }
        self.pending_joins.insert(key, request);
        Ok(())
    }

    pub(super) fn add_leave(&mut self, request: PodLeaveRequestInput) -> Result<(), &'static str> {
        let key = Self::key(&request.pod_id, &request.peer_id);
        if self.pending_leaves.contains_key(&key) {
            return Err("Pending leave request already exists");
        }
        if self.total_pending() >= MAX_POD_PENDING_MEMBERSHIP_RECORDS {
            return Err("Pending membership request capacity is full");
        }
        self.pending_leaves.insert(key, request);
        Ok(())
    }

    pub(super) fn pending_joins(&self, pod_id: &str) -> Vec<&PodJoinSignatureInput> {
        self.pending_joins
            .values()
            .filter(|request| request.pod_id.eq_ignore_ascii_case(pod_id))
            .collect()
    }

    pub(super) fn pending_leaves(&self, pod_id: &str) -> Vec<&PodLeaveRequestInput> {
        self.pending_leaves
            .values()
            .filter(|request| request.pod_id.eq_ignore_ascii_case(pod_id))
            .collect()
    }

    pub(super) fn remove_join(
        &mut self,
        pod_id: &str,
        peer_id: &str,
    ) -> Option<PodJoinSignatureInput> {
        self.pending_joins.remove(&Self::key(pod_id, peer_id))
    }

    pub(super) fn remove_leave(
        &mut self,
        pod_id: &str,
        peer_id: &str,
    ) -> Option<PodLeaveRequestInput> {
        self.pending_leaves.remove(&Self::key(pod_id, peer_id))
    }

    pub(super) fn role(&self, pod_id: &str, peer_id: &str) -> &str {
        self.roles
            .get(&Self::key(pod_id, peer_id))
            .map(String::as_str)
            .unwrap_or("member")
    }

    pub(super) fn set_role(&mut self, pod_id: &str, peer_id: &str, role: String) {
        self.roles.insert(Self::key(pod_id, peer_id), role);
    }

    pub(super) fn remove_role(&mut self, pod_id: &str, peer_id: &str) {
        self.roles.remove(&Self::key(pod_id, peer_id));
    }

    pub(super) fn clear_pending(&mut self) -> usize {
        let removed = self.pending_joins.len() + self.pending_leaves.len();
        self.pending_joins.clear();
        self.pending_leaves.clear();
        removed
    }
}

pub(super) fn verify_pod_join_signature(
    mode: PodSignatureMode,
    input: &PodJoinSignatureInput,
    now_millis: u64,
) -> Result<bool, String> {
    verify_pod_signed_payload(
        mode,
        &input.signature,
        &input.public_key,
        input.timestamp_unix_ms,
        now_millis,
        &input.canonical_payload(),
        "pod join",
    )
}

pub(super) fn verify_pod_signed_payload(
    mode: PodSignatureMode,
    signature: &str,
    public_key: &str,
    timestamp_unix_ms: u64,
    now_millis: u64,
    canonical_payload: &str,
    operation: &str,
) -> Result<bool, String> {
    if mode == PodSignatureMode::Off {
        return Ok(false);
    }
    if signature.trim().is_empty() {
        return if mode == PodSignatureMode::Enforce {
            Err(format!("{operation} Ed25519 signature is required"))
        } else {
            Ok(false)
        };
    }
    if !signature
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("ed25519:"))
    {
        return if mode == PodSignatureMode::Enforce {
            Err(format!(
                "{operation} signature must use the ed25519:<base64> format"
            ))
        } else {
            Ok(false)
        };
    }
    if public_key.trim().is_empty() {
        return Err(format!(
            "{operation} public key is required for Ed25519 verification"
        ));
    }
    if timestamp_unix_ms == 0
        || now_millis.abs_diff(timestamp_unix_ms) > POD_JOIN_TIMESTAMP_SKEW_MILLIS
    {
        return Err(format!(
            "{operation} timestamp is outside the allowed five-minute skew"
        ));
    }
    let signature_bytes = STANDARD
        .decode(&signature[8..])
        .map_err(|_| format!("{operation} signature is not valid base64"))?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| format!("{operation} Ed25519 signature must be 64 bytes"))?;
    let public_key_bytes = STANDARD
        .decode(public_key.as_bytes())
        .map_err(|_| format!("{operation} public key is not valid base64"))?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| format!("{operation} Ed25519 public key must be 32 bytes"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| format!("{operation} Ed25519 public key is invalid"))?;
    verifying_key
        .verify_strict(canonical_payload.as_bytes(), &signature)
        .map_err(|_| format!("{operation} Ed25519 signature is invalid"))?;
    Ok(true)
}
