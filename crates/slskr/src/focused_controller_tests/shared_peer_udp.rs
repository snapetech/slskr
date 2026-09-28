use super::fixtures::*;
use slskr_client::{
    overlay_control::ControlEnvelope,
    quic_control::{certificate_public_key_pin, send_quic_control},
    quic_data::send_quic_data,
};
use std::{net::UdpSocket, sync::Arc, time::Duration};

#[tokio::test]
async fn native_gateway_routes_real_control_data_and_dht_on_one_udp_socket() {
    tokio::time::timeout(Duration::from_secs(20), async {
        for enabled in [false, true] {
            let (mut state, _receiver) = test_state_with_env(MapEnv::default());
            let directory = state.config.state_dir.clone();
            let public = Arc::new(UdpSocket::bind("127.0.0.1:0").unwrap());
            public.set_nonblocking(true).unwrap();
            let address = public.local_addr().unwrap();
            if enabled {
                let mut settings = state.config.advanced_networking.dht.clone();
                settings.dht_port = address.port();
                settings.lan_only = true;
                settings.bootstrap_routers.clear();
                Arc::get_mut(&mut state).unwrap().dht = Some(Arc::new(
                    crate::dht::Rendezvous::new_with_udp_socket(&settings, true, Some(public.clone())).unwrap(),
                ));
            }
            let gateway = Arc::new(crate::private_gateway::Gateway::load_or_create_with_quic_and_data_policy_and_proxy_and_dht_socket_with_data_share_shared_tcp(
                address, &directory, Some(address), Some(address), None, Some(address),
                Some(public.clone()), None, None, 2, true,
            ).await.unwrap());
            let certificate = std::fs::read(directory.join("overlay-certificate.der")).unwrap();
            let pin = certificate_public_key_pin(&certificate).unwrap();
            gateway.clone().run(state.clone()).await.unwrap();
            let envelope = ControlEnvelope::signed_at("probe", b"payload".to_vec(), "shared-gateway", 2,
                &ed25519_dalek::SigningKey::from_bytes(&[7; 32])).unwrap();
            let (control, data) = tokio::join!(
                send_quic_control(address, &envelope, pin),
                send_quic_data(address, b"bounded payload\n", pin),
            );
            control.expect("control ALPN on shared public socket");
            assert_eq!(data.expect("data ALPN on same public socket"), 16);
            if enabled {
                let peer = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
                peer.send_to(b"d1:ad2:id20:AAAAAAAAAAAAAAAAAAAAe1:q4:ping1:t4:aaaa1:y1:qe", address).await.unwrap();
                let mut reply = [0; 2048];
                let (length, source) = tokio::time::timeout(Duration::from_secs(2), peer.recv_from(&mut reply)).await.unwrap().unwrap();
                assert_eq!(source, address);
                assert!(reply[..length].windows(6).any(|window| window == b"4:aaaa"));
            }
            state.shutdown_managed_tasks().await;
            drop((gateway, state, public));
            // Quinn closes its internal endpoint driver asynchronously.
            let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
            loop {
                if let Ok(rebound) = UdpSocket::bind(address) { drop(rebound); break; }
                assert!(tokio::time::Instant::now() < deadline, "shared socket must close at shutdown");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            std::fs::remove_dir_all(directory).unwrap();
        }
    }).await.expect("shared gateway test deadline");
}
