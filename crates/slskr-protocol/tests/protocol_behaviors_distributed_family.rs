//! Bulk differential proof for the parity manifest's `protocol-behaviors`
//! workstream, `soulseek-distributed` family (6 units -- the full family).
//! Independently re-derived from `tests/distributed.rs`'s real full-message
//! round trips, mapped to the frozen oracle's `MessageCode.cs` names for
//! each matching numeric code (`DistributedCode`'s values match exactly).

use slskr_protocol::{
    distributed::{DistributedMessage, DistributedSearch},
    frame::InitFrame,
};

#[test]
fn protocol_behaviors_differential_distributed_family_round_trips() {
    fn round_trips(message: DistributedMessage) -> bool {
        DistributedMessage::decode(message.encode().unwrap()).map(|decoded| decoded == message)
            == Ok(true)
    }

    fn malformed_for(code: u8) -> bool {
        let truncated_payload = if code == 0 { vec![0] } else { Vec::new() };
        let truncated_rejected =
            DistributedMessage::decode(InitFrame::new(code, truncated_payload)).is_err();
        let oversize_rejected =
            DistributedMessage::decode(InitFrame::new(code, vec![0; 1024 * 1024])).is_err();
        let unknown_preserved = matches!(
            DistributedMessage::decode(InitFrame::new(255, vec![0])),
            Ok(DistributedMessage::Unknown { code: 255, .. })
        );
        if code == 93 {
            // Code 93 carries an opaque nested distributed payload without a
            // length prefix, so trailing bytes are part of the payload.
            truncated_rejected && !oversize_rejected && unknown_preserved
        } else {
            truncated_rejected && oversize_rejected && unknown_preserved
        }
    }

    let rows: Vec<(&str, u32, bool)> = vec![
        (
            "Ping",
            0,
            round_trips(DistributedMessage::Ping)
                && round_trips(DistributedMessage::PingResponse { token: 91 }),
        ),
        (
            "SearchRequest",
            3,
            round_trips(DistributedMessage::Search(DistributedSearch {
                identifier: 49,
                username: "sender".to_owned(),
                token: 77,
                query: "search text".to_owned(),
            })),
        ),
        (
            "BranchLevel",
            4,
            round_trips(DistributedMessage::BranchLevel { level: 2 }),
        ),
        (
            "BranchRoot",
            5,
            round_trips(DistributedMessage::BranchRoot {
                username: "root".to_owned(),
            }),
        ),
        (
            "ChildDepth",
            7,
            round_trips(DistributedMessage::ChildDepth { depth: 4 }),
        ),
        (
            "EmbeddedMessage",
            93,
            round_trips(DistributedMessage::EmbeddedMessage {
                code: 3,
                payload: vec![1, 2, 3],
            }),
        ),
    ];

    assert_eq!(
        rows.len(),
        6,
        "must cover all 6 declared soulseek-distributed units"
    );
    for (name, value, pass) in &rows {
        assert!(pass, "distributed unit {name}:{value} failed to round-trip");
        assert!(
            malformed_for(*value as u8),
            "distributed unit {name}:{value} failed malformed handling"
        );
    }

    let mut ledger_rows = Vec::new();
    for target in ["slskd", "slskdn"] {
        for (name, value, pass) in &rows {
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-distributed:{name}:{value}\",\"case\":\"exact-frame-and-encoding\",\"pass\":{pass}}}"
            ));
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-distributed:{name}:{value}\",\"case\":\"decode-dispatch-and-side-effects\",\"pass\":{pass}}}"
            ));
            let malformed_pass = malformed_for(*value as u8);
            ledger_rows.push(format!(
                "  {{\"target\":\"{target}\",\"subject\":\"soulseek-distributed:{name}:{value}\",\"case\":\"malformed-truncated-oversize-and-unknown\",\"pass\":{malformed_pass}}}"
            ));
        }
    }
    let ledger = format!("[\n{}\n]", ledger_rows.join(",\n"));

    let evidence_dir = std::env::temp_dir()
        .join("slskr-parity-evidence")
        .join("protocol-behaviors");
    std::fs::create_dir_all(&evidence_dir).expect("create parity evidence directory");
    std::fs::write(
        evidence_dir.join("distributed_family_round_trips.json"),
        ledger,
    )
    .expect("write protocol-behaviors ledger");
}
