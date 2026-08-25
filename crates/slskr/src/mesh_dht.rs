use std::{collections::BTreeMap, sync::Mutex};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use slskr_client::{
    capabilities::peer_id_for_public_key,
    overlay::{connect_tls_overlay, MeshHello, MeshServiceCall, FEATURE_MESH_SERVICE},
};

use crate::config::TrustedMeshPeer;

const STORE_TTL_SECONDS: i32 = 3_600;
const MAX_PUBLICATIONS: usize = 240;
const MAX_DHT_NODES: usize = 1_024;
const MAX_DHT_VALUES: usize = 8_192;
const MAX_DHT_VALUES_PER_PUBLISHER: usize = 256;
const MAX_DHT_VALUES_PER_NAMESPACE: usize = 64;
const MAX_DHT_VALUE_BYTES: usize = 512 * 1024;
const MAX_DHT_TTL_SECONDS: i64 = 24 * 60 * 60;
const MIN_DHT_TTL_SECONDS: i64 = 60;
const DHT_SIGNATURE_MAX_AGE_MILLIS: i64 = 5 * 60 * 1_000;

/// Bounded in-process storage and routing state for the authenticated mesh
/// DHT service.  The public Soulseek DHT remains owned by `crate::dht`; this
/// state is only for the overlay RPC contract used by trusted mesh peers.
#[derive(Debug, Default)]
pub struct DhtServiceState {
    values: Mutex<BTreeMap<[u8; 20], DhtValue>>,
    nodes: Mutex<BTreeMap<[u8; 20], DhtNode>>,
    remote_admissions: Mutex<BTreeMap<(String, [u8; 20]), u64>>,
}

#[derive(Debug)]
struct DhtValue {
    value: Vec<u8>,
    expires_at: u64,
}

#[derive(Debug, Clone)]
struct DhtNode {
    address: String,
    last_seen_millis: i64,
}

impl DhtServiceState {
    /// Handle one authenticated overlay DHT request.
    ///
    /// The surrounding gateway has already validated the mesh envelope and
    /// bounded the call payload.  This layer validates the Kademlia payload,
    /// keeps routing/storage state bounded, and returns the same status-code
    /// meanings used by the current mesh service (`0` success, `1` internal,
    /// `2` service, `3` method, `4` payload, `6` rate limit, `7` unauthorized).
    pub async fn handle_call(
        &self,
        method: &str,
        payload: &[u8],
        remote_username: &str,
    ) -> Result<Vec<u8>, (i32, String)> {
        match method {
            "FindNode" => self.handle_find_node(payload, remote_username),
            "FindValue" => self.handle_find_value(payload, remote_username),
            "Store" => self.handle_store(payload),
            "Ping" => self.handle_ping(payload, remote_username),
            _ => Err((3, "Unknown method".to_owned())),
        }
    }

    fn handle_find_node(
        &self,
        payload: &[u8],
        remote_username: &str,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: FindNodeRequest = parse_dht_payload(payload)?;
        let target_id = decode_dht_id(&request.target_id)?;
        let requester_id = decode_dht_id(&request.requester_id)?;
        self.observe_node(requester_id, remote_username);
        let nodes = self.closest_nodes(&target_id, request.count);
        serde_json::to_vec(&FindNodeResponse {
            target_id: BASE64.encode(target_id),
            nodes,
        })
        .map_err(|_| (1, "FindNode failed".to_owned()))
    }

    fn handle_find_value(
        &self,
        payload: &[u8],
        remote_username: &str,
    ) -> Result<Vec<u8>, (i32, String)> {
        let request: FindValueRequest = parse_dht_payload(payload)?;
        let key = decode_dht_id(&request.key)?;
        let requester_id = decode_dht_id(&request.requester_id)?;
        self.observe_node(requester_id, remote_username);

        let value = {
            let mut values = self
                .values
                .lock()
                .map_err(|_| (1, "FindValue failed".to_owned()))?;
            let now = unix_seconds();
            values.retain(|_, record| record.expires_at > now);
            values.get(&key).map(|record| record.value.clone())
        };
        if let Some(value) = value {
            return serde_json::to_vec(&FindValueResponse {
                key: BASE64.encode(key),
                found: true,
                value: Some(BASE64.encode(value)),
                closest_nodes: None,
            })
            .map_err(|_| (1, "FindValue failed".to_owned()));
        }

        serde_json::to_vec(&FindValueResponse {
            key: BASE64.encode(key),
            found: false,
            value: None,
            closest_nodes: Some(self.closest_nodes(&key, request.count)),
        })
        .map_err(|_| (1, "FindValue failed".to_owned()))
    }

    fn handle_store(&self, payload: &[u8]) -> Result<Vec<u8>, (i32, String)> {
        let request: StoreRequest = parse_dht_payload(payload)?;
        let key = decode_dht_id(&request.key)?;
        let value = decode_dht_bytes(&request.value, MAX_DHT_VALUE_BYTES, "Value")?;
        let requester_id = decode_dht_id(&request.requester_id)?;
        let publisher_id = verify_store_request(&request, key, &value, requester_id)
            .map_err(|error| (7, error))?;
        let ttl_seconds =
            i64::from(request.ttl_seconds).clamp(MIN_DHT_TTL_SECONDS, MAX_DHT_TTL_SECONDS) as u64;
        if !self.admit_remote_store(&publisher_id, key, ttl_seconds) {
            return Err((6, "Remote DHT storage quota exceeded".to_owned()));
        }

        {
            let mut values = self
                .values
                .lock()
                .map_err(|_| (1, "Store failed".to_owned()))?;
            values.retain(|_, record| record.expires_at > unix_seconds());
            if values.len() >= MAX_DHT_VALUES && !values.contains_key(&key) {
                return Err((6, "Remote DHT storage quota exceeded".to_owned()));
            }
            values.insert(
                key,
                DhtValue {
                    value,
                    expires_at: unix_seconds().saturating_add(ttl_seconds),
                },
            );
        }
        self.observe_node(requester_id, &publisher_id);

        serde_json::to_vec(&StoreResponse {
            key: BASE64.encode(key),
            stored: true,
            ttl_seconds: i32::try_from(ttl_seconds).unwrap_or(i32::MAX),
            error_message: None,
        })
        .map_err(|_| (1, "Store failed".to_owned()))
    }

    fn handle_ping(&self, payload: &[u8], remote_username: &str) -> Result<Vec<u8>, (i32, String)> {
        let request: PingRequest = parse_dht_payload(payload)?;
        let requester_id = decode_dht_id(&request.requester_id)?;
        self.observe_node(requester_id, remote_username);
        serde_json::to_vec(&PingResponse {
            timestamp: unix_millis(),
        })
        .map_err(|_| (1, "Ping failed".to_owned()))
    }

    fn observe_node(&self, node_id: [u8; 20], address: &str) {
        let address = address.trim();
        if address.is_empty() {
            return;
        }
        let Ok(mut nodes) = self.nodes.lock() else {
            return;
        };
        if nodes.len() >= MAX_DHT_NODES && !nodes.contains_key(&node_id) {
            return;
        }
        nodes.insert(
            node_id,
            DhtNode {
                address: address.chars().take(512).collect(),
                last_seen_millis: unix_millis(),
            },
        );
    }

    fn closest_nodes(&self, target: &[u8; 20], requested_count: Option<usize>) -> Vec<DhtNodeInfo> {
        let count = requested_count.unwrap_or(20).clamp(1, 20);
        let Ok(nodes) = self.nodes.lock() else {
            return Vec::new();
        };
        let mut closest = nodes
            .iter()
            .map(|(node_id, node)| (*node_id, node.clone()))
            .collect::<Vec<_>>();
        closest.sort_by_key(|(node_id, _)| xor_distance(node_id, target));
        closest
            .into_iter()
            .take(count)
            .map(|(node_id, node)| DhtNodeInfo {
                node_id: BASE64.encode(node_id),
                address: node.address,
                last_seen: chrono::DateTime::from_timestamp_millis(node.last_seen_millis)
                    .map(|timestamp| timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            })
            .collect()
    }

    fn admit_remote_store(&self, publisher_id: &str, key: [u8; 20], ttl_seconds: u64) -> bool {
        let now = unix_seconds();
        let mut admissions = match self.remote_admissions.lock() {
            Ok(admissions) => admissions,
            Err(_) => return false,
        };
        admissions.retain(|_, expires_at| *expires_at > now);
        let record_key = (publisher_id.to_owned(), key);
        if let Some(expiry) = admissions.get_mut(&record_key) {
            *expiry = now.saturating_add(ttl_seconds);
            return true;
        }
        let publisher_count = admissions
            .keys()
            .filter(|(publisher, _)| publisher == publisher_id)
            .count();
        let namespace = key[0];
        let namespace_count = admissions
            .keys()
            .filter(|(publisher, candidate)| publisher == publisher_id && candidate[0] == namespace)
            .count();
        if admissions.len() >= MAX_DHT_VALUES
            || publisher_count >= MAX_DHT_VALUES_PER_PUBLISHER
            || namespace_count >= MAX_DHT_VALUES_PER_NAMESPACE
        {
            return false;
        }
        admissions.insert(record_key, now.saturating_add(ttl_seconds));
        true
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FindNodeRequest {
    #[serde(alias = "targetId")]
    target_id: String,
    #[serde(alias = "requesterId")]
    requester_id: String,
    #[serde(default, alias = "count")]
    count: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct FindNodeResponse {
    target_id: String,
    nodes: Vec<DhtNodeInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct FindValueRequest {
    #[serde(alias = "key")]
    key: String,
    #[serde(alias = "requesterId")]
    requester_id: String,
    #[serde(default, alias = "count")]
    count: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct FindValueResponse {
    key: String,
    found: bool,
    value: Option<String>,
    closest_nodes: Option<Vec<DhtNodeInfo>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PingRequest {
    #[serde(alias = "requesterId")]
    requester_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PingResponse {
    timestamp: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct DhtNodeInfo {
    node_id: String,
    address: String,
    last_seen: String,
}

fn parse_dht_payload<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, (i32, String)> {
    serde_json::from_slice(payload).map_err(|_| (4, "Invalid request payload".to_owned()))
}

fn decode_dht_id(value: &str) -> Result<[u8; 20], (i32, String)> {
    let bytes = BASE64
        .decode(value)
        .map_err(|_| (4, "Invalid request payload".to_owned()))?;
    bytes
        .try_into()
        .map_err(|_| (4, "Invalid request payload".to_owned()))
}

fn decode_dht_bytes(value: &str, max_bytes: usize, field: &str) -> Result<Vec<u8>, (i32, String)> {
    let bytes = BASE64
        .decode(value)
        .map_err(|_| (4, format!("{field} is invalid")))?;
    if bytes.len() > max_bytes {
        return Err((9, format!("{field} is too large")));
    }
    Ok(bytes)
}

fn xor_distance(left: &[u8; 20], right: &[u8; 20]) -> [u8; 20] {
    std::array::from_fn(|index| left[index] ^ right[index])
}

fn verify_store_request(
    request: &StoreRequest,
    key: [u8; 20],
    value: &[u8],
    requester_id: [u8; 20],
) -> Result<String, String> {
    let public_key = BASE64
        .decode(&request.public_key_base64)
        .map_err(|_| "Signature verification failed".to_owned())?;
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| "Signature verification failed".to_owned())?;
    let signature = BASE64
        .decode(&request.signature_base64)
        .map_err(|_| "Signature verification failed".to_owned())?;
    let signature = Signature::from_slice(&signature)
        .map_err(|_| "Signature verification failed".to_owned())?;
    let requester_digest = Sha256::digest(public_key);
    if requester_id != requester_digest[..20] {
        return Err("Signature verification failed".to_owned());
    }
    let now = unix_millis();
    if request.timestamp_unix_ms <= 0
        || request.timestamp_unix_ms > now
        || now.saturating_sub(request.timestamp_unix_ms) > DHT_SIGNATURE_MAX_AGE_MILLIS
    {
        return Err("Signature verification failed".to_owned());
    }
    let signable = store_signable_payload(
        key,
        value,
        requester_id,
        request.ttl_seconds,
        &request.public_key_base64,
        request.timestamp_unix_ms,
    );
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| "Signature verification failed".to_owned())?;
    verifying_key
        .verify(signable.as_bytes(), &signature)
        .map_err(|_| "Signature verification failed".to_owned())?;
    Ok(peer_id_for_public_key(&public_key))
}

#[derive(Clone, Debug)]
pub struct ShadowPublication {
    pub recording_id: String,
    pub peer_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PodPublication {
    pub pod_id: String,
    pub name: String,
    pub focus_content_id: Option<String>,
    pub tags: Vec<String>,
    pub channel_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct PublicationSnapshot {
    pub peer_id: String,
    pub endpoints: Vec<String>,
    pub content_ids: Vec<String>,
    pub shadows: Vec<ShadowPublication>,
    pub pods: Vec<PodPublication>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublishReport {
    pub attempted: usize,
    pub stored: usize,
    pub failed: usize,
}

#[derive(Clone, Debug)]
struct Publication {
    key: [u8; 20],
    value: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct StoreRequest {
    key: String,
    value: String,
    requester_id: String,
    ttl_seconds: i32,
    public_key_base64: String,
    signature_base64: String,
    timestamp_unix_ms: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct StoreResponse {
    #[serde(default)]
    stored: bool,
    #[serde(default)]
    key: String,
    #[serde(default)]
    ttl_seconds: i32,
    #[serde(default)]
    error_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PodMetadata<'a> {
    pod_id: &'a str,
    name: &'a str,
    visibility: i32,
    focus_content_id: &'a Option<String>,
    tags: &'a [String],
    channel_count: usize,
    published_at: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct PodIndex<'a> {
    pod_ids: &'a [String],
    updated_at: i64,
}

pub async fn publish(
    peers: &[TrustedMeshPeer],
    local_username: &str,
    signing_key: &SigningKey,
    snapshot: &PublicationSnapshot,
) -> PublishReport {
    if peers.is_empty() {
        return PublishReport::default();
    }
    let publications = build_publications(snapshot, unix_millis());
    let mut report = PublishReport::default();
    for peer in peers {
        let peer_report =
            publish_to_peer(peer, local_username, signing_key, publications.values()).await;
        report.attempted += peer_report.attempted;
        report.stored += peer_report.stored;
        report.failed += peer_report.failed;
    }
    report
}

pub async fn probe_store(
    peer: &TrustedMeshPeer,
    _local_username: &str,
    signing_key: &SigningKey,
) -> Result<(), String> {
    let key = derive_key("slskr:interop:dht-store-v1");
    // DHT STORE signatures are self-certifying.  Older native profile targets bind
    // the authenticated overlay identity to the Ed25519-derived peer ID,
    // rather than to the Soulseek account name used by other mesh calls.
    // Announce the signing identity for this DHT-only session so both target
    // generations accept the same signed request.
    let overlay_identity = dht_overlay_identity(signing_key);
    let mut hello = MeshHello::new(
        overlay_identity,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        uuid::Uuid::new_v4().simple().to_string(),
    )
    .map_err(|error| format!("DHT probe hello failed: {error}"))?;
    hello
        .authenticate(signing_key, &peer.certificate_sha256)
        .map_err(|error| format!("DHT probe authentication failed: {error}"))?;
    let mut client = connect_tls_overlay(peer.overlay_endpoint, peer.certificate_sha256, hello)
        .await
        .map_err(|error| format!("DHT probe connection failed: {error}"))?;
    if !client.remote_username.eq_ignore_ascii_case(&peer.username) {
        return Err("DHT probe remote username mismatch".to_owned());
    }
    let request = signed_store_request(
        key,
        b"slskr-slskdn-dht-store-v1",
        STORE_TTL_SECONDS,
        unix_millis(),
        signing_key,
    );
    let payload = serde_json::to_vec(&request)
        .map_err(|error| format!("DHT probe request encode failed: {error}"))?;
    let call = MeshServiceCall::new(
        uuid::Uuid::new_v4().simple().to_string(),
        "dht",
        "Store",
        payload,
    )
    .map_err(|error| format!("DHT probe call encode failed: {error}"))?;
    let reply = client
        .call(&call)
        .await
        .map_err(|error| format!("DHT probe call failed: {error}"))?;
    if reply.status_code != 0 {
        return Err(format!(
            "DHT Store status {}: {} payload={}",
            reply.status_code,
            reply.error_message.as_deref().unwrap_or("remote error"),
            String::from_utf8_lossy(&reply.payload)
        ));
    }
    let response: StoreResponse = serde_json::from_slice(&reply.payload)
        .map_err(|error| format!("DHT Store response decode failed: {error}"))?;
    if !response.stored {
        return Err("DHT Store response reported Stored=false".to_owned());
    }
    Ok(())
}

async fn publish_to_peer<'a>(
    peer: &TrustedMeshPeer,
    _local_username: &str,
    signing_key: &SigningKey,
    publications: impl Iterator<Item = &'a Publication>,
) -> PublishReport {
    let mut report = PublishReport::default();
    let overlay_identity = dht_overlay_identity(signing_key);
    let mut hello = match MeshHello::new(
        overlay_identity,
        vec![FEATURE_MESH_SERVICE.to_owned()],
        None,
        None,
        uuid::Uuid::new_v4().simple().to_string(),
    ) {
        Ok(hello) => hello,
        Err(_) => return report,
    };
    if hello
        .authenticate(signing_key, &peer.certificate_sha256)
        .is_err()
    {
        return report;
    }
    let Ok(mut client) =
        connect_tls_overlay(peer.overlay_endpoint, peer.certificate_sha256, hello).await
    else {
        return report;
    };
    if !client.remote_username.eq_ignore_ascii_case(&peer.username) {
        return report;
    }

    for publication in publications {
        report.attempted += 1;
        let timestamp = unix_millis();
        let request = signed_store_request(
            publication.key,
            &publication.value,
            STORE_TTL_SECONDS,
            timestamp,
            signing_key,
        );
        let Ok(payload) = serde_json::to_vec(&request) else {
            report.failed += 1;
            continue;
        };
        let Ok(call) = MeshServiceCall::new(
            uuid::Uuid::new_v4().simple().to_string(),
            "dht",
            "Store",
            payload,
        ) else {
            report.failed += 1;
            continue;
        };
        match client.call(&call).await {
            Ok(reply) if reply.status_code == 0 => {
                match serde_json::from_slice::<StoreResponse>(&reply.payload) {
                    Ok(response) if response.stored => report.stored += 1,
                    _ => report.failed += 1,
                }
            }
            _ => report.failed += 1,
        }
    }
    report
}

fn build_publications(
    snapshot: &PublicationSnapshot,
    timestamp: i64,
) -> BTreeMap<[u8; 20], Publication> {
    let mut publications = BTreeMap::new();
    let mut content_ids = snapshot.content_ids.clone();
    content_ids.sort();
    content_ids.dedup();
    for content_id in content_ids.iter().take(96) {
        insert_publication(
            &mut publications,
            &format!("mesh:content-peers:{content_id}"),
            encode_content_peer_hints(&snapshot.peer_id, &snapshot.endpoints, timestamp),
        );
    }
    if !content_ids.is_empty() {
        insert_publication(
            &mut publications,
            &format!("mesh:peer-content:{}", snapshot.peer_id),
            encode_string_array(&content_ids[..content_ids.len().min(96)]),
        );
    }

    for shadow in snapshot.shadows.iter().take(96) {
        let namespace = format!("slskdn-vsf-mbid-recording-v1:{}", shadow.recording_id);
        let key = derive_key(&namespace);
        publications.insert(
            key,
            Publication {
                key,
                value: encode_shadow_shard(&shadow.peer_ids, timestamp),
            },
        );
    }

    let pod_ids = snapshot
        .pods
        .iter()
        .take(32)
        .map(|pod| pod.pod_id.clone())
        .collect::<Vec<_>>();
    for pod in snapshot.pods.iter().take(32) {
        let value = serde_json::to_vec(&PodMetadata {
            pod_id: &pod.pod_id,
            name: &pod.name,
            visibility: 0,
            focus_content_id: &pod.focus_content_id,
            tags: &pod.tags,
            channel_count: pod.channel_count,
            published_at: timestamp,
        })
        .unwrap_or_default();
        insert_publication(
            &mut publications,
            &format!("pod:metadata:{}", pod.pod_id),
            value,
        );
    }
    if !pod_ids.is_empty() {
        let value = serde_json::to_vec(&PodIndex {
            pod_ids: &pod_ids,
            updated_at: timestamp,
        })
        .unwrap_or_default();
        insert_publication(&mut publications, "pod:index:listed", value);
    }

    publications.into_iter().take(MAX_PUBLICATIONS).collect()
}

fn insert_publication(
    publications: &mut BTreeMap<[u8; 20], Publication>,
    namespace: &str,
    value: Vec<u8>,
) {
    let key = derive_key(namespace);
    publications.insert(key, Publication { key, value });
}

pub fn derive_key(namespace: &str) -> [u8; 20] {
    <Sha1 as Sha1Digest>::digest(namespace.as_bytes()).into()
}

pub fn peer_id(signing_key: &SigningKey) -> String {
    peer_id_for_public_key(&signing_key.verifying_key().to_bytes())
}

fn dht_overlay_identity(signing_key: &SigningKey) -> String {
    peer_id(signing_key)
}

fn signed_store_request(
    key: [u8; 20],
    value: &[u8],
    ttl_seconds: i32,
    timestamp_unix_ms: i64,
    signing_key: &SigningKey,
) -> StoreRequest {
    let public_key = signing_key.verifying_key().to_bytes();
    let requester_digest = Sha256::digest(public_key);
    let requester_id: [u8; 20] = requester_digest[..20]
        .try_into()
        .expect("SHA-256 digest has at least 20 bytes");
    let key_base64 = BASE64.encode(key);
    let value_base64 = BASE64.encode(value);
    let requester_base64 = BASE64.encode(requester_id);
    let public_key_base64 = BASE64.encode(public_key);
    let signable = store_signable_payload(
        key,
        value,
        requester_id,
        ttl_seconds,
        &public_key_base64,
        timestamp_unix_ms,
    );
    let signature = signing_key.sign(signable.as_bytes()).to_bytes();
    StoreRequest {
        key: key_base64,
        value: value_base64,
        requester_id: requester_base64,
        ttl_seconds,
        public_key_base64,
        signature_base64: BASE64.encode(signature),
        timestamp_unix_ms,
    }
}

fn store_signable_payload(
    key: [u8; 20],
    value: &[u8],
    requester_id: [u8; 20],
    ttl_seconds: i32,
    public_key_base64: &str,
    timestamp_unix_ms: i64,
) -> String {
    let key_base64 = BASE64.encode(key);
    let value_base64 = BASE64.encode(value);
    let requester_base64 = BASE64.encode(requester_id);
    format!(
        "DhtStore|{timestamp_unix_ms}|{{\"type\":9,\"key\":\"{}\",\"value\":\"{}\",\"requester_id\":\"{}\",\"ttl_seconds\":{ttl_seconds},\"proto_version\":1,\"public_key\":\"{}\",\"timestamp_ms\":{timestamp_unix_ms}}}",
        dotnet_json_base64(&key_base64),
        dotnet_json_base64(&value_base64),
        dotnet_json_base64(&requester_base64),
        dotnet_json_base64(public_key_base64),
    )
}

fn dotnet_json_base64(value: &str) -> String {
    value.replace('+', "\\u002B")
}

fn encode_content_peer_hints(peer_id: &str, endpoints: &[String], timestamp: i64) -> Vec<u8> {
    let mut output = Vec::new();
    write_array(&mut output, 1);
    write_array(&mut output, 1);
    write_array(&mut output, 3);
    write_string(&mut output, peer_id);
    write_array(&mut output, endpoints.len());
    for endpoint in endpoints {
        write_string(&mut output, endpoint);
    }
    write_i64(&mut output, timestamp);
    output
}

fn encode_string_array(values: &[String]) -> Vec<u8> {
    let mut output = Vec::new();
    write_array(&mut output, values.len());
    for value in values {
        write_string(&mut output, value);
    }
    output
}

fn encode_shadow_shard(peer_ids: &[String], timestamp: i64) -> Vec<u8> {
    let mut output = Vec::new();
    write_array(&mut output, 6);
    write_string(&mut output, "1.0");
    write_array(&mut output, 2);
    write_timestamp(&mut output, timestamp);
    output.push(0);
    write_u64(&mut output, STORE_TTL_SECONDS as u64);
    write_array(&mut output, peer_ids.len().min(64));
    for peer_id in peer_ids.iter().take(64) {
        write_binary(&mut output, peer_id.as_bytes());
    }
    write_array(&mut output, 0);
    write_u64(&mut output, peer_ids.len().min(64) as u64);
    output
}

fn write_array(output: &mut Vec<u8>, length: usize) {
    if length < 16 {
        output.push(0x90 | length as u8);
    } else {
        output.push(0xdc);
        output.extend_from_slice(&(length as u16).to_be_bytes());
    }
}

fn write_string(output: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    if bytes.len() < 32 {
        output.push(0xa0 | bytes.len() as u8);
    } else if bytes.len() <= u8::MAX as usize {
        output.extend_from_slice(&[0xd9, bytes.len() as u8]);
    } else {
        output.push(0xda);
        output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    }
    output.extend_from_slice(bytes);
}

fn write_binary(output: &mut Vec<u8>, value: &[u8]) {
    if value.len() <= u8::MAX as usize {
        output.extend_from_slice(&[0xc4, value.len() as u8]);
    } else {
        output.push(0xc5);
        output.extend_from_slice(&(value.len() as u16).to_be_bytes());
    }
    output.extend_from_slice(value);
}

fn write_u64(output: &mut Vec<u8>, value: u64) {
    if value <= 0x7f {
        output.push(value as u8);
    } else if value <= u16::MAX as u64 {
        output.push(0xcd);
        output.extend_from_slice(&(value as u16).to_be_bytes());
    } else if value <= u32::MAX as u64 {
        output.push(0xce);
        output.extend_from_slice(&(value as u32).to_be_bytes());
    } else {
        output.push(0xcf);
        output.extend_from_slice(&value.to_be_bytes());
    }
}

fn write_i64(output: &mut Vec<u8>, value: i64) {
    output.push(0xd3);
    output.extend_from_slice(&value.to_be_bytes());
}

fn write_timestamp(output: &mut Vec<u8>, timestamp_ms: i64) {
    let seconds = timestamp_ms.div_euclid(1_000) as u64;
    let nanos = timestamp_ms.rem_euclid(1_000) as u64 * 1_000_000;
    let encoded = (nanos << 34) | seconds;
    output.extend_from_slice(&[0xd7, 0xff]);
    output.extend_from_slice(&encoded.to_be_bytes());
}

fn unix_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn unix_seconds() -> u64 {
    u64::try_from(chrono::Utc::now().timestamp()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dht_overlay_identity_is_self_certifying_peer_id() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let hello = MeshHello::new(
            dht_overlay_identity(&key),
            vec![FEATURE_MESH_SERVICE.to_owned()],
            None,
            None,
            "dht-test-nonce",
        )
        .unwrap();

        assert_eq!(hello.username, peer_id(&key));
        assert_eq!(hello.username.len(), 32);
    }

    #[test]
    fn namespaced_key_matches_native_vector() {
        assert_eq!(
            hex::encode(derive_key("mesh:content-peers:recording-1")),
            "636693889e36652eac8f48fa6c4189eae0a3be7d"
        );
    }

    #[test]
    fn signed_store_matches_frozen_dotnet_payload_and_verifies() {
        use ed25519_dalek::{Signature, Verifier as _};

        let key = SigningKey::from_bytes(&[7; 32]);
        let request = signed_store_request(
            derive_key("mesh:content-peers:recording-1"),
            &[0xfb, 0x00, 0x2a],
            1_800,
            1_700_000_000_123,
            &key,
        );
        let signable = format!(
            "DhtStore|1700000000123|{{\"type\":9,\"key\":\"{}\",\"value\":\"\\u002BwAq\",\"requester_id\":\"{}\",\"ttl_seconds\":1800,\"proto_version\":1,\"public_key\":\"{}\",\"timestamp_ms\":1700000000123}}",
            dotnet_json_base64(&request.key),
            dotnet_json_base64(&request.requester_id),
            dotnet_json_base64(&request.public_key_base64),
        );
        assert_eq!(
            request.signature_base64,
            "SdZK14zmKFaZk7tQ/oPWXkedEJxkQodrM6CINlBbuP6vlhYbZw0TwOwOa+mf1i5/rykdDe3UTx9zB08PHWcvCg=="
        );
        let signature =
            Signature::from_slice(&BASE64.decode(&request.signature_base64).unwrap()).unwrap();
        key.verifying_key()
            .verify(signable.as_bytes(), &signature)
            .unwrap();
    }

    #[tokio::test]
    async fn dht_service_round_trips_signed_store_and_find_value() {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let state = DhtServiceState::default();
        let key = derive_key("mesh:test:dht-service");
        let value = b"mesh-value";
        let request = signed_store_request(key, value, 600, unix_millis(), &signing_key);
        let store = state
            .handle_call(
                "Store",
                &serde_json::to_vec(&request).unwrap(),
                "overlay-peer",
            )
            .await
            .expect("signed DHT store should be accepted");
        let store_json: serde_json::Value = serde_json::from_slice(&store).unwrap();
        assert_eq!(store_json["Stored"], true);

        let find_request = serde_json::json!({
            "Key": BASE64.encode(key),
            "RequesterId": BASE64.encode([3_u8; 20]),
            "Count": 20,
        });
        let found = state
            .handle_call(
                "FindValue",
                &serde_json::to_vec(&find_request).unwrap(),
                "overlay-peer",
            )
            .await
            .expect("stored DHT value should be found");
        let found_json: serde_json::Value = serde_json::from_slice(&found).unwrap();
        assert_eq!(found_json["Found"], true);
        assert_eq!(
            BASE64
                .decode(found_json["Value"].as_str().unwrap())
                .unwrap(),
            value
        );
    }

    #[tokio::test]
    async fn dht_service_rejects_forged_publisher_identity() {
        let signing_key = SigningKey::from_bytes(&[10; 32]);
        let state = DhtServiceState::default();
        let key = derive_key("mesh:test:dht-forgery");
        let mut request =
            signed_store_request(key, b"mesh-value", 600, unix_millis(), &signing_key);
        let mut requester = BASE64.decode(&request.requester_id).unwrap();
        requester[0] ^= 0xff;
        request.requester_id = BASE64.encode(requester);
        let error = state
            .handle_call(
                "Store",
                &serde_json::to_vec(&request).unwrap(),
                "overlay-peer",
            )
            .await
            .expect_err("a requester id not bound to the signing key must fail");
        assert_eq!(error.0, 7);
        assert_eq!(error.1, "Signature verification failed");
    }

    #[test]
    fn shadow_messagepack_uses_dotnet_timestamp_extension_shape() {
        assert_eq!(
            BASE64.encode(encode_shadow_shard(
                &["peer-a".to_owned()],
                1_700_000_000_123,
            )),
            "lqMxLjCS1/8dU1MAZVPxAADNDhCRxAZwZWVyLWGQAQ=="
        );
    }
}
