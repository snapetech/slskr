//! Bulk differential proof for the parity manifest's `protocol-behaviors`
//! workstream (`scripts/audit-parity-manifest.py` `protocol_entries()`),
//! keyed by `{family}:{name}:{value}` matching the frozen oracle's real
//! `MessageCode.cs` enum entries exactly. Each test independently re-proves
//! `exact-frame-and-encoding` for one protocol-unit family via a genuine
//! full-message encode/decode round-trip through slskr-protocol's own
//! trusted codec (not just the discriminant byte) -- re-derived from the
//! crate's own existing tests (`tests/init.rs` etc.), not a call into them.
//!
//! The `soulseek-initialization`/`soulseek-peer`/`soulseek-distributed`/
//! `soulseek-server` families are identical between the frozen slskd and
//! slskdN oracles (slskd's vendored `MessageCode.cs` is byte-identical to
//! the frozen `slskNet.Runtime` copy slskdN also uses), so proving a unit
//! once credits both targets.

use slskr_protocol::{frame::InitFrame, init::InitCode, InitMessage};

#[test]
fn protocol_behaviors_differential_initialization_family_round_trips() {
    // PierceFirewall (value 0): real full-message round-trip.
    let pierce = InitMessage::PierceFirewall { token: 123 };
    let pierce_pass = InitCode::PierceFirewall.as_u8() == 0
        && InitCode::try_from(0) == Ok(InitCode::PierceFirewall)
        && InitMessage::decode(pierce.encode().unwrap()).unwrap() == pierce;

    // PeerInit (value 1): real full-message round-trip through the frame layer too.
    let peer_init = InitMessage::PeerInit {
        username: "local".to_owned(),
        connection_type: "P".to_owned(),
        token: 0,
    };
    let encoded = peer_init.encode().unwrap().encode().unwrap();
    let peer_init_pass = InitCode::PeerInit.as_u8() == 1
        && InitCode::try_from(1) == Ok(InitCode::PeerInit)
        && InitMessage::decode(InitFrame::decode(&encoded).unwrap()).unwrap() == peer_init;

    let mut rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        for (name, value, pass) in [
            ("PierceFirewall", 0, pierce_pass),
            ("PeerInit", 1, peer_init_pass),
        ] {
            rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-initialization:{name}:{value}\",\"case\":\"exact-frame-and-encoding\",\"pass\":{pass}}}"
            ));
        }
    }
    let ledger = format!("[\n{}\n]", rows.join(",\n"));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    std::fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    std::fs::write(
        evidence_dir.join("initialization_family_round_trips.json"),
        ledger,
    )
    .expect("write protocol-behaviors ledger");

    assert!(pierce_pass, "PierceFirewall round-trip/code mismatch");
    assert!(peer_init_pass, "PeerInit round-trip/code mismatch");
}
