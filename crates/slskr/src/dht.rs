use std::{
    collections::BTreeSet,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
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
    pub fn new(settings: &crate::config::DhtSettings) -> Result<Self, String> {
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
    ) -> Result<Self, String> {
        let mut builder = Dht::builder();
        builder.bind_address(Ipv4Addr::UNSPECIFIED).port(port);
        if let Some(bootstrap) = bootstrap {
            builder.bootstrap(bootstrap);
        }
        let client = builder
            .build()
            .map_err(|error| format!("DHT bind failed: {error}"))?
            .as_async();
        Ok(Self {
            client,
            overlay_port: AtomicU16::new(overlay_port.unwrap_or(0)),
            allow_special_use_peers: lan_only || bootstrap.is_some(),
            lan_only,
            refresh_interval,
            lookup_timeout,
            min_neighbors,
            peers: RwLock::new(BTreeSet::new()),
            status: RwLock::new(Status::default()),
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
        *self.status.write().await = Status {
            bootstrapped,
            routing_nodes,
            dht_size_estimate: info.dht_size_estimate().0,
            public_address: info.public_address(),
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
            "discoveredBeaconCount": peer_count,
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
    }
}
