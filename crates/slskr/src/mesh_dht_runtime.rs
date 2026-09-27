use super::*;

pub(super) fn spawn_mesh_dht_publisher(state: Arc<AppState>) {
    if state.config.trusted_mesh_peers.is_empty()
        || !state.config.advanced_networking.mesh.enabled
        || !state.config.advanced_networking.mesh.enable_dht
    {
        return;
    }
    let task_state = Arc::clone(&state);
    state.spawn_managed_task(async move {
        let state = task_state;
        time::sleep(Duration::from_secs(5)).await;
        let mut interval = time::interval(Duration::from_secs(30 * 60));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let snapshot = mesh_dht_publication_snapshot(&state).await;
            let local_username = pod_request_peer_id(&state)
                .await
                .unwrap_or_else(|| snapshot.peer_id.clone());
            let report = mesh_dht::publish(
                &state.config.trusted_mesh_peers,
                &local_username,
                &state.capability_signing_key,
                &snapshot,
            )
            .await;
            let level = if report.failed == 0 {
                logging::LogLevel::Info
            } else {
                logging::LogLevel::Warn
            };
            record_daemon_log(
                &state,
                level,
                "mesh-dht",
                format!(
                    "DHT publisher stored {}/{} records ({} failed)",
                    report.stored, report.attempted, report.failed
                ),
            )
            .await;
        }
    });
}

async fn mesh_dht_publication_snapshot(state: &AppState) -> mesh_dht::PublicationSnapshot {
    let peer_id = mesh_dht::peer_id(&state.capability_signing_key);
    let endpoints = state
        .private_gateway
        .as_ref()
        .map(|gateway| gateway.bind())
        .filter(|endpoint| !endpoint.ip().is_unspecified())
        .map(|endpoint| vec![endpoint.to_string()])
        .unwrap_or_default();
    let (content_ids, shadows) = {
        let discovery = state.content_discovery.read().await;
        let content_ids = discovery
            .hash_entries()
            .iter()
            .filter(|entry| !entry.music_brainz_id.trim().is_empty())
            .map(|entry| format!("content:mb:recording:{}", entry.music_brainz_id.trim()))
            .collect();
        let shadows = discovery
            .shadow_records()
            .iter()
            .map(|record| mesh_dht::ShadowPublication {
                recording_id: record.recording_id.clone(),
                peer_ids: record.peer_ids.clone(),
            })
            .collect();
        (content_ids, shadows)
    };
    let pods = state
        .pods
        .read()
        .await
        .list_visible(None)
        .into_iter()
        .filter(|pod| pod.is_public)
        .map(|pod| mesh_dht::PodPublication {
            pod_id: pod.pod_id,
            name: pod.name,
            focus_content_id: pod.focus_content_id,
            tags: pod.tags,
            channel_count: pod.channels.len(),
        })
        .collect();
    mesh_dht::PublicationSnapshot {
        peer_id,
        endpoints,
        content_ids,
        shadows,
        pods,
    }
}

const STUN_BINDING_REQUEST: u16 = 0x0001;
pub(super) const STUN_MAGIC_COOKIE: u32 = 0x2112_A442;
const STUN_MAPPED_ADDRESS: u16 = 0x0001;
const STUN_XOR_MAPPED_ADDRESS: u16 = 0x0020;
/// Public STUN servers used for best-effort NAT type detection, matching
/// the native profile oracle's `MeshOptions.StunServers` defaults exactly.
pub(super) const STUN_SERVERS: [&str; 2] = ["stun.l.google.com:19302", "stun1.l.google.com:19302"];

pub(super) struct StunMapping {
    pub(super) mapped: SocketAddr,
    pub(super) local: SocketAddr,
}

fn build_stun_binding_request(transaction_id: [u8; 12]) -> [u8; 20] {
    let mut request = [0_u8; 20];
    request[0..2].copy_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());
    request[4..8].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
    request[8..20].copy_from_slice(&transaction_id);
    request
}

/// Parses a STUN Binding response for a MAPPED-ADDRESS or XOR-MAPPED-ADDRESS
/// attribute (IPv4 only -- the default STUN servers reply over IPv4).
pub(super) fn parse_stun_mapped_address(response: &[u8]) -> Option<SocketAddr> {
    if response.len() < 20 {
        return None;
    }
    let mut offset = 20;
    while offset + 4 <= response.len() {
        let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let attr_len = u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;
        offset += 4;
        if offset + attr_len > response.len() {
            break;
        }
        if (attr_type == STUN_XOR_MAPPED_ADDRESS || attr_type == STUN_MAPPED_ADDRESS)
            && attr_len >= 8
            && response[offset + 1] == 0x01
        {
            let mut port = u16::from_be_bytes([response[offset + 2], response[offset + 3]]);
            let mut address = u32::from_be_bytes([
                response[offset + 4],
                response[offset + 5],
                response[offset + 6],
                response[offset + 7],
            ]);
            if attr_type == STUN_XOR_MAPPED_ADDRESS {
                port ^= (STUN_MAGIC_COOKIE >> 16) as u16;
                address ^= STUN_MAGIC_COOKIE;
            }
            return Some(SocketAddr::new(
                IpAddr::V4(std::net::Ipv4Addr::from(address)),
                port,
            ));
        }
        offset += (attr_len + 3) & !3;
    }
    None
}

/// Sends a single STUN Binding request to `server` from a fresh ephemeral
/// local UDP socket and returns the mapped/local endpoint pair, or `None` on
/// any resolution, transport, or timeout failure (best-effort, like the
/// oracle's `ProbeServer`).
pub(super) async fn stun_probe(server: &str) -> Option<StunMapping> {
    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await.ok()?;
    let target = tokio::time::timeout(Duration::from_secs(1), tokio::net::lookup_host(server))
        .await
        .ok()?
        .ok()?
        .next()?;
    let mut transaction_id = [0_u8; 12];
    SysRng.try_fill_bytes(&mut transaction_id).ok()?;
    let request = build_stun_binding_request(transaction_id);
    socket.send_to(&request, target).await.ok()?;
    let mut buf = [0_u8; 128];
    let count = tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
        .await
        .ok()?
        .ok()?
        .0;
    let mapped = parse_stun_mapped_address(&buf[..count])?;
    let local = socket.local_addr().ok()?;
    Some(StunMapping { mapped, local })
}

/// Detects the local NAT type via STUN, matching the oracle's
/// `StunNatDetector.DetectAsync` classification: probe the primary server
/// twice (a changed mapping means a symmetric NAT), then optionally a second
/// server (a changed mapping there also means symmetric); a mapping that's
/// stable but differs from the local endpoint is a restricted/cone NAT, and
/// no response at all is unknown. Returns `(type, detected)`.
pub(super) async fn detect_nat_type(servers: &[&str]) -> (&'static str, bool) {
    let Some(primary) = servers.first() else {
        return ("unknown", false);
    };
    let Some(mapping1) = stun_probe(primary).await else {
        return ("unknown", false);
    };
    if mapping1.mapped == mapping1.local {
        return ("direct", true);
    }
    let Some(mapping2) = stun_probe(primary).await else {
        return ("unknown", false);
    };
    if mapping1.mapped != mapping2.mapped {
        return ("symmetric", true);
    }
    if let Some(secondary) = servers.get(1) {
        match stun_probe(secondary).await {
            None => return ("restricted", true),
            Some(mapping3) if mapping1.mapped != mapping3.mapped => return ("symmetric", true),
            Some(_) => {}
        }
    }
    ("restricted", true)
}
