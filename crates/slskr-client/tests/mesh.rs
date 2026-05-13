use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use slskr_client::{
    capabilities::{
        decode_peer_capability_message, PeerCapabilityDescriptor, PeerCapabilityMessageType,
        FEATURE_CAPABILITIES_V1, FEATURE_MESH_V1,
    },
    mesh::{
        is_capability_probe, MeshRendezvous, MeshRendezvousOptions, MESH_RENDEZVOUS_INTEREST_TAG,
    },
    peer_cache::PeerConnectionCache,
    stream::PeerMessageConnection,
};
use tokio::io::duplex;

fn fixed_now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

fn descriptor(features: Vec<String>) -> PeerCapabilityDescriptor {
    let signing_key = SigningKey::from_bytes(&[11; 32]);
    PeerCapabilityDescriptor::unsigned(
        "local",
        features,
        vec!["tcp:127.0.0.1:2234".to_owned()],
        Duration::from_secs(60),
        &signing_key,
        fixed_now(),
    )
    .unwrap()
    .sign(&signing_key)
    .unwrap()
}

#[test]
fn mesh_rendezvous_defaults_to_passive_interest_tag_and_dedupes_candidates() {
    let mesh = MeshRendezvous::disabled();

    assert_eq!(mesh.interest_tag(), MESH_RENDEZVOUS_INTEREST_TAG);
    assert!(!mesh.active_probe_enabled());
    assert_eq!(
        mesh.publish_interest_tags(),
        vec![MESH_RENDEZVOUS_INTEREST_TAG.to_owned()]
    );
    assert_eq!(
        mesh.candidate_usernames([" alice ", "Bob", "", "ALICE"], ["bob", "carol"]),
        vec!["alice".to_owned(), "Bob".to_owned(), "carol".to_owned()]
    );
}

#[tokio::test]
async fn mesh_probe_is_noop_when_active_probe_disabled() {
    let cache = PeerConnectionCache::new();
    let (a, _) = duplex(256);
    cache.insert("peer", PeerMessageConnection::new(a)).await;
    let local = descriptor(vec![
        FEATURE_CAPABILITIES_V1.to_owned(),
        FEATURE_MESH_V1.to_owned(),
    ]);

    assert!(!MeshRendezvous::disabled()
        .probe_peer(&cache, "peer", &local, [1; 16])
        .await
        .unwrap());
    assert!(cache.contains("peer").await);
}

#[test]
fn mesh_accepts_only_mesh_capable_descriptors() {
    let mesh = descriptor(vec![
        FEATURE_CAPABILITIES_V1.to_owned(),
        FEATURE_MESH_V1.to_owned(),
    ]);
    let capabilities_only = descriptor(vec![FEATURE_CAPABILITIES_V1.to_owned()]);

    assert!(MeshRendezvous::accepts_descriptor(&mesh));
    assert!(!MeshRendezvous::accepts_descriptor(&capabilities_only));
}

#[tokio::test]
async fn mesh_probe_sends_capability_hello_when_enabled() {
    let cache = PeerConnectionCache::new();
    let (a, b) = duplex(4096);
    let mut receiver = PeerMessageConnection::new(b);
    cache.insert("peer", PeerMessageConnection::new(a)).await;
    let local = descriptor(vec![
        FEATURE_CAPABILITIES_V1.to_owned(),
        FEATURE_MESH_V1.to_owned(),
    ]);
    let mesh = MeshRendezvous::new(MeshRendezvousOptions {
        interest_tag: MESH_RENDEZVOUS_INTEREST_TAG.to_owned(),
        active_probe: true,
    });

    assert!(mesh
        .probe_peer(&cache, "peer", &local, [4; 16])
        .await
        .unwrap());

    let message = receiver.receive().await.unwrap();
    assert!(is_capability_probe(&message));
    let envelope = decode_peer_capability_message(&message).unwrap().unwrap();
    assert_eq!(envelope.message_type, PeerCapabilityMessageType::Hello);
    assert_eq!(envelope.nonce, [4; 16]);
    assert_eq!(envelope.descriptor.username, "local");
    envelope.descriptor.verify(fixed_now()).unwrap();
}

#[tokio::test]
async fn mesh_probe_reports_missing_peer_without_connecting() {
    let cache = PeerConnectionCache::<tokio::io::DuplexStream>::new();
    let local = descriptor(vec![
        FEATURE_CAPABILITIES_V1.to_owned(),
        FEATURE_MESH_V1.to_owned(),
    ]);
    let mesh = MeshRendezvous::new(MeshRendezvousOptions {
        interest_tag: MESH_RENDEZVOUS_INTEREST_TAG.to_owned(),
        active_probe: true,
    });

    assert!(!mesh
        .probe_peer(&cache, "missing", &local, [9; 16])
        .await
        .unwrap());
}
