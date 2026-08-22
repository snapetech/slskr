#![allow(clippy::too_many_arguments)]

use std::{
    collections::BTreeSet,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::StreamExt;
use mainline::{async_dht::AsyncDht, Dht, Id};
use sha1::{Digest, Sha1};
use tokio::{sync::RwLock, time::timeout};

use crate::utils::is_blocked_outbound_ipv4;

const RENDEZVOUS_NAMES: [&str; 3] = [
    "slskdn-mesh-v1",
    "slskdn-mesh-v1-backup-1",
    "slskdn-mesh-v1-backup-2",
];
const MAX_DISCOVERED_PEERS: usize = 256;
#[cfg(test)]
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Rendezvous {
    client: AsyncDht,
    dht_backend: Option<SocketAddr>,
    shared_udp_socket: Option<Arc<UdpSocket>>,
    shared_public_port: Option<u16>,
    overlay_port: AtomicU16,
    allow_special_use_peers: bool,
    lan_only: bool,
    refresh_interval: Duration,
    lookup_timeout: Duration,
    min_neighbors: usize,
    peers: RwLock<BTreeSet<SocketAddrV4>>,
    status: RwLock<Status>,
}

#[derive(Clone, Debug, Default)]
struct Status {
    bootstrapped: bool,
    routing_nodes: usize,
    dht_size_estimate: usize,
    public_address: Option<SocketAddrV4>,
    firewalled: bool,
    server_mode: bool,
    last_refresh: Option<u64>,
    last_announce: Option<u64>,
    last_error: Option<String>,
}

impl Rendezvous {
    #[allow(dead_code)]
    pub fn new(settings: &crate::config::DhtSettings) -> Result<Self, String> {
        Self::new_with_shared_udp(settings, false)
    }

    /// Build the rendezvous node with an internal UDP endpoint when the
    /// public DHT port is shared with overlay traffic.  The gateway owns the
    /// public socket and forwards only DHT-shaped datagrams to this endpoint;
    /// the normal standalone configuration continues to bind the configured
    /// port directly.
    pub fn new_with_shared_udp(
        settings: &crate::config::DhtSettings,
        shared_udp: bool,
    ) -> Result<Self, String> {
        let bootstrap = settings
            .bootstrap_routers
            .iter()
            .map(|router| {
                if router.contains(':') {
                    router.clone()
                } else {
                    format!("{router}:6881")
                }
            })
            .collect::<Vec<_>>();
        Self::with_runtime_builder(
            settings.dht_port,
            Some(settings.effective_overlay_port()),
            Some(if settings.lan_only { &[] } else { &bootstrap }),
            settings.lan_only,
            settings.discovery_interval,
            settings
                .bootstrap_timeout
                .max(settings.cold_bootstrap_timeout),
            settings.min_neighbors,
            shared_udp,
        )
    }

    #[cfg(test)]
    fn with_builder(
        port: u16,
        overlay_port: Option<u16>,
        bootstrap: Option<&[String]>,
    ) -> Result<Self, String> {
        Self::with_runtime_builder(
            port,
            overlay_port,
            bootstrap,
            false,
            Duration::from_secs(15 * 60),
            LOOKUP_TIMEOUT,
            3,
            false,
        )
    }

    fn with_runtime_builder(
        port: u16,
        overlay_port: Option<u16>,
        bootstrap: Option<&[String]>,
        lan_only: bool,
        refresh_interval: Duration,
        lookup_timeout: Duration,
        min_neighbors: usize,
        shared_udp: bool,
    ) -> Result<Self, String> {
        let shared_udp_socket = if shared_udp {
            let socket = UdpSocket::bind(SocketAddr::new(
                std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port,
            ))
            .map_err(|error| format!("shared DHT UDP bind failed: {error}"))?;
            socket
                .set_nonblocking(true)
                .map_err(|error| format!("shared DHT UDP nonblocking setup failed: {error}"))?;
            Some(Arc::new(socket))
        } else {
            None
        };
        let mut builder = Dht::builder();
        builder
            .bind_address(Ipv4Addr::UNSPECIFIED)
            // mainline owns this endpoint.  A zero port lets the public
            // gateway demuxer retain the configured DHT port for QUIC and
            // overlay traffic without competing for the same UDP socket.
            .port(if shared_udp { 0 } else { port });
        if let Some(socket) = shared_udp_socket.as_ref() {
            builder.outbound_socket(Arc::clone(socket));
        }
        if let Some(bootstrap) = bootstrap {
            builder.bootstrap(bootstrap);
        }
        #[allow(deprecated)]
        let dht = builder
            .build()
            .map_err(|error| format!("DHT bind failed: {error}"))?;
        #[allow(deprecated)]
        let dht_backend = shared_udp.then(|| SocketAddr::V4(dht.info().local_addr()));
        let shared_public_port = shared_udp_socket
            .as_ref()
            .and_then(|socket| socket.local_addr().ok())
            .map(|address| address.port());
        let client = dht.as_async();
        Ok(Self {
            client,
            overlay_port: AtomicU16::new(overlay_port.unwrap_or(0)),
            shared_public_port,
            allow_special_use_peers: lan_only || bootstrap.is_some(),
            lan_only,
            refresh_interval,
            lookup_timeout,
            min_neighbors,
            peers: RwLock::new(BTreeSet::new()),
            status: RwLock::new(Status::default()),
            dht_backend,
            shared_udp_socket,
        })
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            self.refresh().await;
            tokio::time::sleep(self.refresh_interval).await;
        }
    }

    pub async fn refresh(&self) {
        let bootstrapped = timeout(self.lookup_timeout, self.client.bootstrapped())
            .await
            .unwrap_or(false);
        // Publish readiness as soon as the bootstrap probe completes. The
        // rendezvous lookups below can each consume their full timeout, but
        // frozen slskdN exposes DHT Ready independently of that refresh work.
        self.status.write().await.bootstrapped = bootstrapped;
        let mut discovered = BTreeSet::new();
        let mut last_error = None;
        let mut announced = false;
        for key in rendezvous_keys() {
            let port = self.overlay_port.load(Ordering::Relaxed);
            if port != 0 {
                match timeout(
                    self.lookup_timeout,
                    self.client.announce_peer(key, Some(port)),
                )
                .await
                {
                    Ok(Ok(_)) => announced = true,
                    Ok(Err(error)) => last_error = Some(format!("DHT announce failed: {error}")),
                    Err(_) => last_error = Some("DHT announce timed out".to_owned()),
                }
            }
            match self.lookup(key).await {
                Ok(peers) => {
                    for peer in peers {
                        if discovered.len() >= MAX_DISCOVERED_PEERS {
                            break;
                        }
                        discovered.insert(peer);
                    }
                }
                Err(error) => last_error = Some(error),
            }
        }
        *self.peers.write().await = discovered;
        let info = self.client.info().await;
        let routing_nodes = self.client.to_bootstrap().await.len();
        let now = crate::unix_timestamp();
        let public_address = info.public_address().map(|address| {
            self.shared_public_port
                .map_or(address, |port| SocketAddrV4::new(*address.ip(), port))
        });
        *self.status.write().await = Status {
            bootstrapped,
            routing_nodes,
            dht_size_estimate: info.dht_size_estimate().0,
            public_address,
            firewalled: info.firewalled(),
            server_mode: info.server_mode(),
            last_refresh: Some(now),
            last_announce: announced.then_some(now),
            last_error,
        };
    }

    async fn lookup(&self, key: Id) -> Result<Vec<SocketAddrV4>, String> {
        timeout(self.lookup_timeout, async {
            let mut stream = self.client.get_peers(key);
            let mut peers = BTreeSet::new();
            while let Some(batch) = stream.next().await {
                for peer in batch {
                    if valid_peer(peer, self.allow_special_use_peers)
                        && peers.len() < MAX_DISCOVERED_PEERS
                    {
                        peers.insert(peer);
                    }
                }
            }
            peers.into_iter().collect()
        })
        .await
        .map_err(|_| "DHT peer lookup timed out".to_owned())
    }

    pub async fn peers(&self) -> Vec<SocketAddr> {
        self.peers
            .read()
            .await
            .iter()
            .copied()
            .map(SocketAddr::V4)
            .collect()
    }

    /// Return the internal mainline endpoint that receives DHT datagrams
    /// from the public shared-port demuxer, when shared UDP mode is active.
    #[must_use]
    pub fn shared_udp_backend(&self) -> Option<SocketAddr> {
        self.dht_backend
    }

    /// Return the public UDP socket used for outbound DHT traffic in shared
    /// mode. The overlay gateway takes a Tokio clone of this socket so both
    /// protocols retain the configured public source port.
    #[must_use]
    pub fn shared_udp_socket(&self) -> Option<Arc<UdpSocket>> {
        self.shared_udp_socket.as_ref().map(Arc::clone)
    }

    #[cfg(any(test, feature = "bounded-differential"))]
    #[allow(dead_code)]
    pub async fn insert_test_peer(&self, peer: SocketAddrV4) {
        self.peers.write().await.insert(peer);
    }

    pub async fn status_json(&self) -> String {
        let status = self.status.read().await.clone();
        let peer_count = self.peers.read().await.len();
        serde_json::json!({
            "dhtNodeCount": status.routing_nodes,
            "isLanOnly": self.lan_only,
            "lanOnly": self.lan_only,
            "minNeighbors": self.min_neighbors,
            "isBeaconCapable": self.overlay_port.load(Ordering::Relaxed) != 0,
            "isDhtRunning": status.bootstrapped,
            "verifiedBeaconCount": 0,
            // Matches the oracle's real DhtRendezvousStats.DiscoveredPeerCount
            // -- previously emitted under a differently-named key
            // (discoveredBeaconCount), so the route's merge-defaults logic
            // never found it and silently inserted a fake 0 instead.
            "discoveredPeerCount": peer_count,
            "bootstrapped": status.bootstrapped,
            "dhtSizeEstimate": status.dht_size_estimate,
            "publicAddress": status.public_address,
            "firewalled": status.firewalled,
            "serverMode": status.server_mode,
            "rendezvousKeys": RENDEZVOUS_NAMES,
            "lastRefresh": status.last_refresh,
            "lastAnnounce": status.last_announce,
            "lastError": status.last_error,
        })
        .to_string()
    }

    pub fn set_advertised_overlay_port(&self, port: u16) {
        self.overlay_port.store(port, Ordering::Relaxed);
    }

    /// Matches the oracle's real `IsBeaconCapable`: only true once a real
    /// advertised overlay port has been configured.
    pub fn is_beacon_capable(&self) -> bool {
        self.overlay_port.load(Ordering::Relaxed) != 0
    }
}

fn rendezvous_keys() -> [Id; 3] {
    RENDEZVOUS_NAMES.map(|name| {
        let digest: [u8; 20] = Sha1::digest(name.as_bytes()).into();
        Id::from(digest)
    })
}

fn valid_peer(peer: SocketAddrV4, allow_special_use: bool) -> bool {
    peer.port() != 0
        && !peer.ip().is_unspecified()
        && !peer.ip().is_multicast()
        && !peer.ip().is_broadcast()
        && (allow_special_use || !is_blocked_outbound_ipv4(*peer.ip()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendezvous_keys_match_frozen_runtime_names() {
        let encoded = rendezvous_keys().map(|key| hex::encode(key.as_bytes()));
        assert_eq!(
            encoded,
            [
                "381dddbe5adaa5c118f8eab841848feec643247c",
                "e25b572812a32cbee1903f3d403fc2a9e3b3b676",
                "facbc54b5dd43f5109fe17514aa171ee2fd6a2f3",
            ]
        );
    }

    #[test]
    fn unusable_dht_peer_endpoints_are_rejected() {
        assert!(valid_peer("8.8.8.8:50305".parse().unwrap(), false));
        for address in [
            "0.0.0.0:50305",
            "10.0.0.1:50305",
            "100.64.0.1:50305",
            "127.0.0.1:50305",
            "169.254.1.1:50305",
            "192.0.2.1:50305",
            "224.0.0.1:50305",
            "255.255.255.255:50305",
            "8.8.8.8:0",
        ] {
            assert!(!valid_peer(address.parse().unwrap(), false), "{address}");
        }
        assert!(valid_peer("127.0.0.1:50305".parse().unwrap(), true));
    }

    #[test]
    fn shared_udp_mode_moves_mainline_to_a_bounded_internal_endpoint() {
        let rendezvous = Rendezvous::with_runtime_builder(
            50_300,
            Some(50_305),
            Some(&[]),
            true,
            Duration::from_secs(900),
            LOOKUP_TIMEOUT,
            3,
            true,
        )
        .unwrap();
        let backend = rendezvous
            .shared_udp_backend()
            .expect("shared mode has a mainline backend");
        assert_ne!(backend.port(), 50_300);
        assert_ne!(backend.port(), 0);
    }

    #[test]
    fn shared_udp_mode_exposes_the_public_socket_port() {
        let rendezvous = Rendezvous::with_runtime_builder(
            50_301,
            Some(50_305),
            Some(&[]),
            true,
            Duration::from_secs(900),
            LOOKUP_TIMEOUT,
            3,
            true,
        )
        .unwrap();
        let public_socket = rendezvous
            .shared_udp_socket()
            .expect("shared mode has a public socket");
        assert_eq!(public_socket.local_addr().unwrap().port(), 50_301);
        assert_ne!(rendezvous.shared_udp_backend().unwrap().port(), 50_301);
    }

    #[tokio::test]
    async fn local_mainline_testnet_announces_and_discovers_overlay_peer() {
        let testnet = mainline::Testnet::builder(4).build().unwrap();
        let announcer = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap()
            .as_async();
        let rendezvous = Rendezvous::with_builder(0, None, Some(&testnet.bootstrap)).unwrap();
        let key = rendezvous_keys()[0];
        announcer.announce_peer(key, Some(50_305)).await.unwrap();
        let peers = rendezvous.lookup(key).await.unwrap();
        assert!(peers.iter().any(|peer| peer.port() == 50_305));
        rendezvous.refresh().await;
        let status =
            serde_json::from_str::<serde_json::Value>(&rendezvous.status_json().await).unwrap();
        assert_eq!(status["isDhtRunning"], true);
        assert_eq!(status["bootstrapped"], true);
        assert!(status["dhtNodeCount"]
            .as_u64()
            .is_some_and(|count| count > 0));
        // Matches the oracle's real DhtRendezvousStats.DiscoveredPeerCount
        // -- must reflect the real discovered-peer count, not a key-name
        // mismatch silently defaulting to 0 at the HTTP route layer.
        assert!(status["discoveredPeerCount"]
            .as_u64()
            .is_some_and(|count| count > 0));
    }
}
