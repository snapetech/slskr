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
| Protocol count/length candidates | Fixed | Re-run after new wire-count or chunk-read code is added; BUG-033 covers the accepted transfer chunk allocation bug. |
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

Current section: `Protocol count/length candidates`

Latest scanner counts:

| Candidate Class | Count |
| --- | ---: |
| Constructor/mutable collection candidates | 7 |
| Protocol count/length candidates | 41 |
| Protocol scalar emission candidates | 214 |
| Resolver/raw stream candidates | 628 |
| Task/cancellation/lifecycle candidates | 229 |
| Example Web API candidates | 283 |

### Constructor/mutable collection candidates

| Candidate | Scope | Classification | Evidence | Follow-up |
| --- | --- | --- | --- | --- |
| `client-python/slskr/batch.py` `BatchOperation.__init__(..., body: Dict)` | Python SDK | Fixed | BUG-032: constructor now deep-copies request bodies and `to_dict()` returns a deep copy. | `client-python/tests/test_client.py::test_batch_objects_copy_mutable_inputs` |
| `client-python/slskr/batch.py` `BatchResponse.__init__(results: List[BatchResult], ...)` | Python SDK | Fixed | BUG-032: response now copies the caller-supplied result list. | `client-python/tests/test_client.py::test_batch_objects_copy_mutable_inputs` |
| `crates/slskr-protocol/src/frame.rs` `MessageFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Payload is moved into an owned `Vec<u8>`; callers cannot mutate the frame through the original collection. | Covered by frame round-trip tests. |
| `crates/slskr-protocol/src/frame.rs` `InitFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Payload is moved into an owned `Vec<u8>`; encode bounds are checked before emission. | Covered by frame round-trip tests. |
| `crates/slskr-protocol/src/frame.rs` `RawFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Payload is moved into an owned `Vec<u8>` and `encode()` returns a clone. | Covered by raw frame round-trip tests. |
| `crates/slskr/src/webhooks.rs` `Webhook::new(events: Vec<WebhookEvent>, ...)` | Backend/API | Existing Guard | Events are owned by the webhook; route validation caps/validates event lists before construction. | Covered by webhook registration tests and outbound policy gate. |
| `crates/slskr-client/src/search.rs` `InMemoryShareIndex::new(entries: Vec<FileEntry>)` | Network Runtime | Existing Guard | Entries are owned by the index and exposed only through a slice or cloned search results. | Covered by share index/search tests. |

### Protocol count/length candidates

| Candidate | Scope | Classification | Evidence | Follow-up |
| --- | --- | --- | --- | --- |
| `Reader::read_string` and `Reader::read_len_prefixed_bytes` | Protocol primitives | Existing Guard | Length-prefixed values are rejected when length exceeds remaining input before allocation/copy. | Primitive tests cover trailing/invalid reads. |
| `MessageFrame::decode` and `InitFrame::decode` | Protocol frames | Existing Guard | Frame lengths reject too-short code widths and lengths larger than remaining input. | `message_frame_rejects_len_shorter_than_code`, `init_frame_rejects_zero_len`, frame round-trip tests. |
| `read_len_prefixed_frame`, `read_init_frame_with_first_len_byte_and_max`, and `read_obfuscated_len_prefixed_frame` | Client stream I/O | Existing Guard | Async frame readers reject length values over `DEFAULT_MAX_FRAME_LEN` before allocating. | `oversized_message_frame_is_rejected_before_payload_read`. |
| `read_raw_frame(reader, length)` | Client stream I/O | Existing Guard | Raw frame length is caller-supplied, not decoded from untrusted wire metadata in current call sites. | Keep in resolver/raw stream review class for call-site lifecycle classification. |
| `decode_file_entries` file and attribute counts | Peer protocol | Existing Guard | Counts are not preallocated and each entry/attribute consumes bounded remaining bytes, so impossible counts fail on EOF. | `file_search_response_rejects_untrusted_count_without_preallocating`. |
| `decode_string_vec`, `decode_room_entries`, `decode_possible_parents` | Server protocol | Existing Guard | Counts are decoded without preallocating count-sized vectors; room count vectors validate matching lengths. | Room list and possible parent round-trip/selection tests. |
| `FileTransferConnection::read_chunk` and `DownloadTransfer::receive_file_from` | Client SDKs | Fixed | BUG-033: transfer chunk reads now reject lengths above `DEFAULT_MAX_TRANSFER_CHUNK_LEN` before allocating. | `oversized_chunk_is_rejected_before_allocation` and `receive_file_rejects_oversized_remaining_before_allocation`. |
