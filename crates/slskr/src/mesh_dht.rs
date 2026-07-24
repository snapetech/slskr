use std::collections::BTreeMap;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha256;
use slskr_client::overlay::{
    connect_tls_overlay, MeshHello, MeshServiceCall, FEATURE_MESH_SERVICE,
};

use crate::config::TrustedMeshPeer;

const STORE_TTL_SECONDS: i32 = 3_600;
const MAX_PUBLICATIONS: usize = 240;

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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StoreResponse {
    stored: bool,
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
    local_username: &str,
    signing_key: &SigningKey,
) -> Result<(), String> {
    let key = derive_key("slskr:interop:dht-store-v1");
    let mut hello = MeshHello::new(
        local_username,
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
    local_username: &str,
    signing_key: &SigningKey,
    publications: impl Iterator<Item = &'a Publication>,
) -> PublishReport {
    let mut report = PublishReport::default();
    let mut hello = match MeshHello::new(
        local_username,
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
    let digest = Sha256::digest(signing_key.verifying_key().to_bytes());
    base32_lower(&digest[..20])
}

fn base32_lower(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut output = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut bits = 0_u16;
    let mut bit_count = 0_u8;
    for byte in bytes {
        bits = (bits << 8) | u16::from(*byte);
        bit_count += 8;
        while bit_count >= 5 {
            bit_count -= 5;
            output.push(ALPHABET[((bits >> bit_count) & 31) as usize] as char);
        }
    }
    if bit_count > 0 {
        output.push(ALPHABET[((bits << (5 - bit_count)) & 31) as usize] as char);
    }
    output
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
    let requester_id = &requester_digest[..20];
    let key_base64 = BASE64.encode(key);
    let value_base64 = BASE64.encode(value);
    let requester_base64 = BASE64.encode(requester_id);
    let public_key_base64 = BASE64.encode(public_key);
    let signable = format!(
        "DhtStore|{timestamp_unix_ms}|{{\"type\":9,\"key\":\"{}\",\"value\":\"{}\",\"requester_id\":\"{}\",\"ttl_seconds\":{ttl_seconds},\"proto_version\":1,\"public_key\":\"{}\",\"timestamp_ms\":{timestamp_unix_ms}}}",
        dotnet_json_base64(&key_base64),
        dotnet_json_base64(&value_base64),
        dotnet_json_base64(&requester_base64),
        dotnet_json_base64(&public_key_base64),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaced_key_matches_slskdn_vector() {
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
