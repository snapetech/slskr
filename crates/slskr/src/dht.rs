use std::{
    collections::BTreeSet,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
};

use futures_util::StreamExt;
use mainline::{async_dht::AsyncDht, Dht, Id};
use sha1::{Digest, Sha1};
use tokio::{sync::RwLock, time::timeout};

const RENDEZVOUS_NAMES: [&str; 3] = [
    "slskdn-mesh-v1",
    "slskdn-mesh-v1-backup-1",
    "slskdn-mesh-v1-backup-2",
];
const MAX_DISCOVERED_PEERS: usize = 256;
const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Rendezvous {
    client: AsyncDht,
    overlay_port: Option<u16>,
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
    pub fn new(port: u16, overlay_port: Option<u16>) -> Result<Self, String> {
        Self::with_builder(port, overlay_port, None)
    }

    fn with_builder(
        port: u16,
        overlay_port: Option<u16>,
        bootstrap: Option<&[String]>,
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
            overlay_port,
            peers: RwLock::new(BTreeSet::new()),
            status: RwLock::new(Status::default()),
        })
    }

    pub async fn run(self: Arc<Self>) {
        loop {
            self.refresh().await;
            tokio::time::sleep(REFRESH_INTERVAL).await;
        }
    }

    pub async fn refresh(&self) {
        let bootstrapped = timeout(LOOKUP_TIMEOUT, self.client.bootstrapped())
            .await
            .unwrap_or(false);
        let mut discovered = BTreeSet::new();
        let mut last_error = None;
        let mut announced = false;
        for key in rendezvous_keys() {
            if let Some(port) = self.overlay_port {
                match timeout(LOOKUP_TIMEOUT, self.client.announce_peer(key, Some(port))).await {
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
        timeout(LOOKUP_TIMEOUT, async {
            let mut stream = self.client.get_peers(key);
            let mut peers = BTreeSet::new();
            while let Some(batch) = stream.next().await {
                for peer in batch {
                    if valid_peer(peer) && peers.len() < MAX_DISCOVERED_PEERS {
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
            "isLanOnly": false,
            "lanOnly": false,
            "isBeaconCapable": self.overlay_port.is_some(),
            "isDhtRunning": true,
            "verifiedBeaconCount": peer_count,
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
}

fn rendezvous_keys() -> [Id; 3] {
    RENDEZVOUS_NAMES.map(|name| {
        let digest: [u8; 20] = Sha1::digest(name.as_bytes()).into();
        Id::from(digest)
    })
}

fn valid_peer(peer: SocketAddrV4) -> bool {
    peer.port() != 0 && !peer.ip().is_unspecified() && !peer.ip().is_multicast()
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
    }
}
