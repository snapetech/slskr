use super::*;
use crate::{overlay_control::ControlEnvelope, shared_udp::SharedUdpSocket};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Driver(tokio::task::JoinHandle<()>);
impl Drop for Driver {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl Driver {
    async fn stop(mut self) {
        self.0.abort();
        let _ = (&mut self.0).await;
    }
}

fn client_config(
    certificate: CertificateDer<'static>,
    alpn: &[u8],
    fragmented: bool,
) -> quinn::ClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(certificate).unwrap();
    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols.push(alpn.to_vec());
    if fragmented {
        // A large valid ALPN offer forces CRYPTO across multiple Initial packets.
        for byte in b'a'..=b'l' {
            config.alpn_protocols.push(vec![byte; 255]);
        }
    }
    quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(config).unwrap(),
    ))
}

fn fixture(
    control: bool,
    data: bool,
) -> (
    SharedQuicServer,
    CertificateDer<'static>,
    Driver,
    Arc<AtomicUsize>,
) {
    let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let shared = SharedUdpSocket::new(&socket).unwrap();
    let (ingress, endpoint_socket) = shared.endpoint();
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let der = certificate.cert.der().clone();
    let server = SharedQuicServer::with_socket(
        endpoint_socket,
        der.clone(),
        PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der()),
        control,
        data,
        128,
        2,
    )
    .unwrap();
    let initials = Arc::new(AtomicUsize::new(0));
    let count = initials.clone();
    let driver = Driver(tokio::spawn(async move {
        let mut buffer = vec![0; 128 * 1024];
        loop {
            let meta = shared.recv(&mut buffer).await.unwrap();
            for packet in buffer[..meta.len].chunks(meta.stride) {
                if packet.len() >= 1200 && packet[0] & 0xf0 == 0xc0 {
                    count.fetch_add(1, Ordering::Relaxed);
                }
                let mut single = meta;
                single.len = packet.len();
                single.stride = packet.len();
                ingress.try_send(packet, single).unwrap();
            }
        }
    }));
    (server, der, driver, initials)
}

#[tokio::test]
async fn one_endpoint_reassembles_fragmented_handshakes_and_dispatches_both_alpns_from_one_peer() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let (server, certificate, driver, initials) = fixture(true, true);
        let address = server.local_addr().unwrap();
        let client = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
        let peer = client.local_addr().unwrap();
        let envelope = ControlEnvelope::signed_at(
            "probe",
            b"payload".to_vec(),
            "combined",
            2,
            &ed25519_dalek::SigningKey::from_bytes(&[7; 32]),
        )
        .unwrap();
        let offered = async {
            let control = client
                .connect_with(
                    client_config(certificate.clone(), QUIC_CONTROL_ALPN, true),
                    address,
                    "localhost",
                )
                .unwrap()
                .await
                .unwrap();
            let data = client
                .connect_with(
                    client_config(certificate, QUIC_DATA_ALPN, true),
                    address,
                    "localhost",
                )
                .unwrap()
                .await
                .unwrap();
            let (mut send, _) = control.open_bi().await.unwrap();
            send.write_all(&envelope.encode().unwrap()).await.unwrap();
            send.finish().unwrap();
            let (mut send, _) = data.open_bi().await.unwrap();
            send.write_all(b"data payload").await.unwrap();
            send.finish().unwrap();
            (control, data)
        };
        let accepted = async {
            let first = server.accept().await.unwrap().unwrap();
            let second = server.accept().await.unwrap().unwrap();
            assert_eq!(first.remote_address(), peer);
            assert_eq!(second.remote_address(), peer);
            let mut saw_control = false;
            let mut saw_data = false;
            for connection in [first, second] {
                match connection {
                    SharedQuicConnection::Control(connection) => {
                        assert_eq!(connection.accept_envelope().await.unwrap(), envelope);
                        saw_control = true;
                    }
                    SharedQuicConnection::Data(connection) => {
                        assert_eq!(
                            connection
                                .accept_stream()
                                .await
                                .unwrap()
                                .read_payload()
                                .await
                                .unwrap(),
                            b"data payload"
                        );
                        saw_data = true;
                    }
                }
            }
            assert!(saw_control && saw_data);
        };
        let ((control, data), ()) = tokio::join!(offered, accepted);
        assert!(
            initials.load(Ordering::Relaxed) >= 4,
            "both large ClientHellos must span Initial datagrams"
        );
        control.close(0_u32.into(), b"test complete");
        data.close(0_u32.into(), b"test complete");
        client.close(0_u32.into(), b"test complete");
        client.wait_idle().await;
        server.close();
        driver.stop().await;
    })
    .await
    .expect("combined ALPN fixture deadline");
}

#[tokio::test]
async fn disabled_alpn_is_rejected_by_tls_before_protocol_dispatch() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let (server, certificate, driver, _) = fixture(true, false);
        let client = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
        let connect = client
            .connect_with(
                client_config(certificate, QUIC_DATA_ALPN, false),
                server.local_addr().unwrap(),
                "localhost",
            )
            .unwrap();
        let (client_result, server_result) = tokio::join!(connect, server.accept());
        assert!(client_result.is_err());
        assert!(server_result.unwrap().is_err());
        client.close(0_u32.into(), b"test complete");
        client.wait_idle().await;
        server.close();
        driver.stop().await;
    })
    .await
    .expect("disabled ALPN fixture deadline");
}
