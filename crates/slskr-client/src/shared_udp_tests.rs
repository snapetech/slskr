use super::*;
use crate::{
    overlay_control::ControlEnvelope,
    quic_control::{certificate_public_key_pin, send_quic_control, QuicControlServer},
    quic_data::{send_quic_data, QuicDataServer},
};
use std::time::Duration;

fn metadata(length: usize) -> RecvMeta {
    RecvMeta {
        addr: "127.0.0.1:34567".parse().unwrap(),
        len: length,
        stride: length,
        ecn: Some(quinn::udp::EcnCodepoint::Ect0),
        dst_ip: Some("127.0.0.1".parse().unwrap()),
    }
}

fn shared() -> SharedUdpSocket {
    SharedUdpSocket::new(&std::net::UdpSocket::bind("127.0.0.1:0").unwrap()).unwrap()
}

#[tokio::test]
async fn ingress_preserves_peer_and_metadata_and_sends_from_public_port() {
    let shared = shared();
    let address = shared.local_addr().unwrap();
    let (ingress, socket) = shared.endpoint();
    assert_eq!(socket.local_addr().unwrap(), address);
    let original = metadata(4);
    ingress.try_send(b"test", original).unwrap();
    let mut bytes = [0; 16];
    let mut buffers = [IoSliceMut::new(&mut bytes)];
    let mut meta = [RecvMeta::default()];
    assert_eq!(
        poll_fn(|cx| socket.poll_recv(cx, &mut buffers, &mut meta))
            .await
            .unwrap(),
        1
    );
    assert_eq!(&bytes[..4], b"test");
    assert_eq!(meta[0].addr, original.addr);
    assert_eq!(meta[0].dst_ip, original.dst_ip);
    assert_eq!(meta[0].ecn, original.ecn);
    assert_eq!(meta[0].stride, 4);
    let peer = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let transmit = Transmit {
        destination: peer.local_addr().unwrap(),
        ecn: None,
        contents: b"reply",
        segment_size: None,
        src_ip: None,
    };
    let mut poller = Arc::clone(&socket).create_io_poller();
    poll_fn(|cx| poller.as_mut().poll_writable(cx))
        .await
        .unwrap();
    socket.try_send(&transmit).unwrap();
    let (length, source) = tokio::time::timeout(Duration::from_secs(2), peer.recv_from(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(&bytes[..length], b"reply");
    assert_eq!(source, address);
}

#[tokio::test]
async fn ingress_rejects_oversize_batches_full_and_closed_queues() {
    let (ingress, socket) = shared().endpoint();
    for _ in 0..SHARED_QUIC_QUEUE_CAPACITY {
        ingress.try_send(b"x", metadata(1)).unwrap();
    }
    assert_eq!(
        ingress.try_send(b"x", metadata(1)).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    let oversized = vec![0; MAX_SHARED_DATAGRAM_BYTES + 1];
    assert_eq!(
        ingress
            .try_send(&oversized, metadata(oversized.len()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        ingress.try_send(b"", metadata(0)).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    let mut batch = metadata(4);
    batch.stride = 2;
    assert_eq!(
        ingress.try_send(b"test", batch).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    let mut invalid = metadata(1);
    invalid.addr.set_port(0);
    assert_eq!(
        ingress.try_send(b"x", invalid).unwrap_err().kind(),
        io::ErrorKind::InvalidInput
    );
    drop(socket);
    assert_eq!(
        ingress.try_send(b"x", metadata(1)).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
}

// The driver belongs to this fixture and is cancelled even if an assertion fails.
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

fn drive(shared: SharedUdpSocket, ingress: SharedQuicIngress) -> Driver {
    Driver(tokio::spawn(async move {
        let mut buffer = vec![0; 128 * 1024];
        loop {
            let meta = shared.recv(&mut buffer).await.unwrap();
            assert!(meta.stride > 0 && meta.len <= buffer.len());
            for packet in buffer[..meta.len].chunks(meta.stride) {
                let mut individual = meta;
                individual.len = packet.len();
                individual.stride = packet.len();
                ingress.try_send(packet, individual).unwrap();
            }
        }
    }))
}

#[tokio::test]
async fn control_and_data_servers_use_one_public_socket_with_real_pinned_handshakes() {
    tokio::time::timeout(Duration::from_secs(15), async {
        let shared = shared();
        let address = shared.local_addr().unwrap();
        let identity = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
        let certificate = identity.cert.der().clone();
        let pin = certificate_public_key_pin(&certificate).unwrap();
        let key = || {
            tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(
                identity.signing_key.serialize_der(),
            )
        };
        let (control_ingress, control_socket) = shared.endpoint();
        let (data_ingress, data_socket) = shared.endpoint();
        let control =
            QuicControlServer::with_socket(control_socket, certificate.clone(), key()).unwrap();
        let data = QuicDataServer::with_socket(data_socket, certificate, key(), 128, 2).unwrap();
        assert_eq!(control.local_addr().unwrap(), address);
        assert_eq!(data.local_addr().unwrap(), address);
        // Exercise each ALPN separately; production protocol selection is tested
        // by the gateway. Both endpoints remain bound to this same socket.
        let driver = drive(shared.clone(), control_ingress);
        let envelope = ControlEnvelope::signed_at(
            "probe",
            b"payload".to_vec(),
            "shared",
            2,
            &ed25519_dalek::SigningKey::from_bytes(&[7; 32]),
        )
        .unwrap();
        let (sent, received) = tokio::join!(send_quic_control(address, &envelope, pin), async {
            control
                .accept()
                .await
                .unwrap()
                .unwrap()
                .accept_envelope()
                .await
                .unwrap()
        });
        sent.unwrap();
        assert_eq!(received, envelope);
        received.verify().unwrap();
        control.close();
        driver.stop().await;
        let driver = drive(shared, data_ingress);
        let payload = b"shared bounded payload";
        let (sent, received) = tokio::join!(send_quic_data(address, payload, pin), async {
            data.accept()
                .await
                .unwrap()
                .unwrap()
                .accept_stream()
                .await
                .unwrap()
                .read_payload()
                .await
                .unwrap()
        });
        assert_eq!(sent.unwrap(), payload.len());
        assert_eq!(received, payload);
        data.close();
        driver.stop().await;
    })
    .await
    .expect("shared QUIC fixture deadline");
}
