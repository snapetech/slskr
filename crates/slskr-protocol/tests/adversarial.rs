use std::panic::{catch_unwind, AssertUnwindSafe};

use slskr_protocol::{
    distributed::DistributedMessage,
    frame::{InitFrame, MessageFrame},
    init::InitMessage,
    peer::PeerMessage,
    server::{Direction, ServerMessage},
};

const SEEDS: [u64; 4] = [0x5111_5110, 0xC0DE_BAAD, 0xA11C_A7ED, 0x5EED_2026];

const KNOWN_CORPUS: [&[u8]; 9] = [
    &[],
    &[0x00],
    &[0xFF],
    &[0x04, 0x00, 0x00, 0x00],
    &[0xFF, 0xFF, 0xFF, 0x7F],
    &[0xFF, 0xFF, 0xFF, 0xFF],
    &[0x00, 0x00, 0x00, 0x80],
    &[0x10, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x7F],
    &[0x10, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF],
];

#[test]
fn adversarial_known_corpus_does_not_panic() {
    for bytes in KNOWN_CORPUS {
        assert_protocol_decoders_do_not_panic(bytes);
    }
}

#[test]
fn adversarial_multi_seed_random_corpus_does_not_panic() {
    for seed in SEEDS {
        let mut rng = Lcg::new(seed);
        for _ in 0..500 {
            let len = (rng.next() % 256) as usize;
            let mut bytes = vec![0_u8; len];
            for byte in &mut bytes {
                *byte = (rng.next() & 0xFF) as u8;
            }

            assert_protocol_decoders_do_not_panic(&bytes);
        }
    }
}

fn assert_protocol_decoders_do_not_panic(bytes: &[u8]) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if let Ok(frame) = MessageFrame::decode(bytes) {
            let _ = PeerMessage::decode(frame.clone());
            let _ = ServerMessage::decode(frame.clone(), Direction::ServerToClient);
            let _ = ServerMessage::decode(frame, Direction::ClientToServer);
        }

        if let Ok(frame) = InitFrame::decode(bytes) {
            let _ = InitMessage::decode(frame.clone());
            let _ = DistributedMessage::decode(frame);
        }
    }));

    assert!(
        result.is_ok(),
        "protocol decoder panicked on bytes: {bytes:?}"
    );
}

struct Lcg {
    state: u64,
}

impl Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.state
    }
}
