# Council Scan Inventory

Date: 2026-05-05

This file fixes the audit loop: scanner output must be converted into a visible candidate inventory before implementation narrows to a small patch. Counts are candidate lines, not confirmed bugs. The council reviews one whole class at a time and classifies every plausible candidate in that class before moving to the next class.

Regenerate counts with:

```sh
scripts/run-council-scan.sh
```

## Loop Rules

1. Run `scripts/run-council-scan.sh`.
2. Pick one candidate class from `Remaining Candidate Classes`.
3. Convert that whole scan section into a review table, not a single finding.
4. Classify rows as `Unclassified`, `Accepted`, `Fixed`, `Existing Guard`, or `False Positive`.
5. Add ledger rows for all `Accepted` bugs before implementing.
6. Burn down all `Accepted` rows for that class in ownership batches.
7. Move to the next class only after the current class has no `Unclassified` or `Accepted` rows.

## Remaining Candidate Classes

| Candidate Class | Current State | Next Action |
| --- | --- | --- |
| Constructor/mutable collection candidates | Fixed | Re-run after new constructors are added; BUG-032 covers the accepted Python mutable input bug. |
| Protocol count/length candidates | Unclassified | Classify every protocol count, length, allocation, and capacity candidate. |
| Protocol scalar emission candidates | Unclassified | Classify scalar writes, narrowing casts, and encoded length emission candidates. |
| Resolver/raw stream candidates | Unclassified | Classify direct socket, resolver, stream read/write, and raw connection lifecycle candidates. |
| Task/cancellation/lifecycle candidates | Unclassified | Classify spawn, timeout, interval, channel, cancellation, and shutdown candidates. |
| Example Web API candidates | Unclassified | Classify docs/examples for stale auth, storage, WebSocket, CORS, and URL guidance. |

## Classification Legend

| Status | Meaning |
| --- | --- |
| Unclassified | Candidate has not been reviewed yet. It may be a bug, an existing guard, or a false positive. |
| Accepted | Council agrees the candidate is a real bug and a ledger row is required. |
| Fixed | Code/docs changed and local regression coverage exists. |
| Existing Guard | Candidate is already protected by code and tests, with evidence recorded. |
| False Positive | Candidate does not apply after review, with the reason recorded. |

## Current Section Review

Current section: `Constructor/mutable collection candidates`

Latest scanner counts:

| Candidate Class | Count |
| --- | ---: |
| Constructor/mutable collection candidates | 7 |
| Protocol count/length candidates | 987 |
| Protocol scalar emission candidates | 214 |
| Resolver/raw stream candidates | 628 |
| Task/cancellation/lifecycle candidates | 229 |
| Example Web API candidates | 283 |

| Candidate | Scope | Classification | Evidence | Follow-up |
| --- | --- | --- | --- | --- |
| `client-python/slskr/batch.py` `BatchOperation.__init__(..., body: Dict)` | Python SDK | Fixed | BUG-032: constructor now deep-copies request bodies and `to_dict()` returns a deep copy. | `client-python/tests/test_client.py::test_batch_objects_copy_mutable_inputs` |
| `client-python/slskr/batch.py` `BatchResponse.__init__(results: List[BatchResult], ...)` | Python SDK | Fixed | BUG-032: response now copies the caller-supplied result list. | `client-python/tests/test_client.py::test_batch_objects_copy_mutable_inputs` |
| `crates/slskr-protocol/src/frame.rs` `MessageFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Payload is moved into an owned `Vec<u8>`; callers cannot mutate the frame through the original collection. | Covered by frame round-trip tests. |
| `crates/slskr-protocol/src/frame.rs` `InitFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Payload is moved into an owned `Vec<u8>`; encode bounds are checked before emission. | Covered by frame round-trip tests. |
| `crates/slskr-protocol/src/frame.rs` `RawFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Payload is moved into an owned `Vec<u8>` and `encode()` returns a clone. | Covered by raw frame round-trip tests. |
| `crates/slskr/src/webhooks.rs` `Webhook::new(events: Vec<WebhookEvent>, ...)` | Backend/API | Existing Guard | Events are owned by the webhook; route validation caps/validates event lists before construction. | Covered by webhook registration tests and outbound policy gate. |
| `crates/slskr-client/src/search.rs` `InMemoryShareIndex::new(entries: Vec<FileEntry>)` | Network Runtime | Existing Guard | Entries are owned by the index and exposed only through a slice or cloned search results. | Covered by share index/search tests. |
