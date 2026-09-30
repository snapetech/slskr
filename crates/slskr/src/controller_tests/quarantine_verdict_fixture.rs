/// Builds a verdict body with a real, self-consistent signature --
/// the oracle's own check is content-integrity (the submitted
/// payloadHash must match a hash of the verdict's own fields), not
/// full cryptographic authentication, so a real "signer" key isn't
/// needed here, only a hash that genuinely matches.
pub(super) fn quarantine_signed_verdict_json(
    request_id: &str,
    juror: &str,
    verdict: &str,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "requestId": request_id,
        "juror": juror,
        "verdict": verdict,
    });
    let payload_hash = crate::quarantine_controller::quarantine_verdict_payload_hash(&value);
    value["signature"] = serde_json::json!({
        "signer": juror,
        "payloadHash": payload_hash,
        "value": "test-signature",
    });
    value
}
