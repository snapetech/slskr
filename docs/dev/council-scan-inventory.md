# Council Scan Inventory

Date: 2026-05-05

> Council process upgrades (mirrored from slskNet.Runtime, 2026-05-06): see `bug-council-severity-schema.md`, `bug-council-sibling-search.md`, `bug-council-negative-space.md`, `bug-council-behavior-pinning.md`, and `bug-council-phases.md`. Future sweep rows on this file should adopt the severity/confidence schema; the wire-frame trust boundary is now declared and enforced by `scripts/check-council-negative-space.sh`.

This file fixes the audit loop: scanner output must be converted into a visible candidate inventory before implementation narrows to a small patch. Counts are candidate lines, not confirmed bugs. The council reviews one whole class at a time and classifies every plausible candidate in that class before moving to the next class.

Regenerate counts with:

```sh
scripts/run-council-scan.sh
```

Run the full bughunt entrypoint with:

```sh
scripts/run-council-bughunt.sh
```

`check-remediation-baseline.sh` is only a regression gate. It must not be reported as "no bugs found." The bughunt entrypoint regenerates counts, runs calibrated semantic lenses, and keeps pending phases visible.

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
| Protocol count/length candidates | Fixed | Re-run after new wire-count or chunk-read code is added; BUG-033 covers the accepted transfer chunk allocation bug and BUG-040 covers the semantic-lens count loop hardening. |
| Protocol scalar emission candidates | Fixed | Re-run after new protocol scalar casts or length-prefix writers are added; BUG-034 covers accepted API-to-protocol narrowing bugs. |
| Resolver/raw stream candidates | Fixed | Re-run after new direct socket, resolver, stream read/write, or raw connection lifecycle code is added; BUG-035 and BUG-036 cover accepted raw-stream bugs. |
| Task/cancellation/lifecycle candidates | Fixed | Re-run after new spawn, timeout, interval, channel, cancellation, or shutdown code is added; BUG-037 and BUG-038 cover accepted TypeScript lifecycle bugs. |
| Example Web API candidates | Fixed | Re-run after docs/examples add API auth, storage, WebSocket, CORS, or URL guidance; BUG-039 covers accepted stale WebSocket auth examples. |

## Classification Legend

| Status | Meaning |
| --- | --- |
| Unclassified | Candidate has not been reviewed yet. It may be a bug, an existing guard, or a false positive. |
| Accepted | Council agrees the candidate is a real bug and a ledger row is required. |
| Fixed | Code/docs changed and local regression coverage exists. |
| Existing Guard | Candidate is already protected by code and tests, with evidence recorded. |
| False Positive | Candidate does not apply after review, with the reason recorded. |

## Current Section Review

Current section: `Example Web API candidates`

Latest scanner counts:

| Candidate Class | Count |
| --- | ---: |
| Constructor/mutable collection candidates | 7 |
| Protocol count/length candidates | 41 |
| Protocol scalar emission candidates | 30 |
| Resolver/raw stream candidates | 220 |
| Task/cancellation/lifecycle candidates | 236 |
| Example Web API candidates | 289 |

### Constructor/mutable collection candidates

| Candidate | Scope | Classification | Severity | Confidence | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `client-python/slskr/batch.py` `BatchOperation.__init__(..., body: Dict)` | Python SDK | Fixed | Medium | High | BUG-032: constructor now deep-copies request bodies and `to_dict()` returns a deep copy. | `client-python/tests/test_client.py::test_batch_objects_copy_mutable_inputs` |
| `client-python/slskr/batch.py` `BatchResponse.__init__(results: List[BatchResult], ...)` | Python SDK | Fixed | Medium | High | BUG-032: response now copies the caller-supplied result list. | `client-python/tests/test_client.py::test_batch_objects_copy_mutable_inputs` |
| `crates/slskr-protocol/src/frame.rs` `MessageFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Low | High | Payload is moved into an owned `Vec<u8>`; callers cannot mutate the frame through the original collection. | Covered by frame round-trip tests. |
| `crates/slskr-protocol/src/frame.rs` `InitFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Low | High | Payload is moved into an owned `Vec<u8>`; encode bounds are checked before emission. | Covered by frame round-trip tests. |
| `crates/slskr-protocol/src/frame.rs` `RawFrame::new(payload: impl Into<Vec<u8>>)` | Protocol | Existing Guard | Low | High | Payload is moved into an owned `Vec<u8>` and `encode()` returns a clone. | Covered by raw frame round-trip tests. |
| `crates/slskr/src/webhooks.rs` `Webhook::new(events: Vec<WebhookEvent>, ...)` | Backend/API | Existing Guard | Low | High | Events are owned by the webhook; route validation caps/validates event lists before construction. | Covered by webhook registration tests and outbound policy gate. |
| `crates/slskr-client/src/search.rs` `InMemoryShareIndex::new(entries: Vec<FileEntry>)` | Network Runtime | Existing Guard | Low | High | Entries are owned by the index and exposed only through a slice or cloned search results. | Covered by share index/search tests. |

### Protocol count/length candidates

| Candidate | Scope | Classification | Severity | Confidence | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `Reader::read_string` and `Reader::read_len_prefixed_bytes` | Protocol primitives | Existing Guard | Low | High | Length-prefixed values are rejected when length exceeds remaining input before allocation/copy. | Primitive tests cover trailing/invalid reads. |
| `MessageFrame::decode` and `InitFrame::decode` | Protocol frames | Existing Guard | Low | High | Frame lengths reject too-short code widths and lengths larger than remaining input. | `message_frame_rejects_len_shorter_than_code`, `init_frame_rejects_zero_len`, frame round-trip tests. |
| `read_len_prefixed_frame`, `read_init_frame_with_first_len_byte_and_max`, and `read_obfuscated_len_prefixed_frame` | Client stream I/O | Existing Guard | Low | High | Async frame readers reject length values over `DEFAULT_MAX_FRAME_LEN` before allocating. | `oversized_message_frame_is_rejected_before_payload_read`. |
| `read_raw_frame(reader, length)` | Client stream I/O | Fixed | Medium | High | BUG-035: raw frame reads now route through `read_raw_frame_with_max` and reject lengths above `DEFAULT_MAX_FRAME_LEN` before allocating. | `oversized_raw_frame_is_rejected_before_payload_read`. |
| `decode_file_entries` file and attribute counts | Peer protocol | Existing Guard | Low | High | Counts are not preallocated and each entry/attribute consumes bounded remaining bytes, so impossible counts fail on EOF. | `file_search_response_rejects_untrusted_count_without_preallocating`. |
| `decode_string_vec`, `decode_room_entries`, `decode_possible_parents` | Server protocol | Existing Guard | Low | High | Counts are decoded without preallocating count-sized vectors; room count vectors validate matching lengths. | Room list and possible parent round-trip/selection tests. |
| `FileTransferConnection::read_chunk` and `DownloadTransfer::receive_file_from` | Client SDKs | Fixed | Medium | High | BUG-033: transfer chunk reads now reject lengths above `DEFAULT_MAX_TRANSFER_CHUNK_LEN` before allocating. | `oversized_chunk_is_rejected_before_allocation` and `receive_file_rejects_oversized_remaining_before_allocation`. |
| `decode_file_entries`, `decode_string_vec`, `decode_possible_parents`, and shared-file browse payload parsers | Protocol + Backend/API | Fixed | Medium | High | BUG-040: calibrated Rust protocol taint lens found reader-derived counts flowing into parser loops; counts now route through `Reader::read_bounded_count` and reject impossible counts based on remaining bytes before loop execution. | `scripts/check-rust-protocol-taint-lens.sh`; peer/server/shared-file count regression tests. |

### Protocol scalar emission candidates

| Candidate | Scope | Classification | Severity | Confidence | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| JSON search response token to protocol `u32` | Backend/API | Fixed | Medium | High | BUG-034: `/api/search-responses` now rejects tokens above `u32::MAX` instead of narrowing with `as u32`. | `search_response_api_rejects_oversized_protocol_token` |
| Message acknowledgement route ID to protocol `u32` | Backend/API | Fixed | Medium | High | BUG-034: message acknowledgement routes now reject IDs above `u32::MAX` before sending `MessageAcked`. | `messages_api_records_lists_and_acks_messages` oversized ack assertion |
| WebSocket outbound frame payload length casts | Backend/API | Existing Guard | Low | High | `write_frame` branches by payload length before `u8`/`u16` casts and uses `u64` for larger payloads. | WebSocket frame/auth tests. |
| WebSocket masked client frame test helper cast | Tests/Tooling | False Positive | Low | High | Test helper only builds short fixture frames and is not production frame emission. | Keep scoped to tests. |
| Protocol enum discriminant `as u8`/`as u32` methods | Protocol | Existing Guard | Low | High | Code tables are fixed protocol discriminants and covered by inventory/round-trip tests. | Server/peer/init/distributed code inventory tests. |
| Protocol and share-cache length prefix writers | Protocol + Backend/API | Existing Guard | Low | High | Length prefixes use `u32::try_from(...len())` or `write_len_prefixed_bytes`, which returns `LengthOverflow` before emission. | Frame, primitive, share payload tests. |
| Widening casts to `u64` for durations and byte lengths | Backend/API + Client SDKs | Existing Guard | Low | High | Casts widen from bounded/nonnegative values or `usize` byte lengths into `u64`; they do not truncate protocol scalars. | Existing transfer/tracing tests. |
| Fixed small constants cast to `u32` | Backend/API | Existing Guard | Low | High | `MAX_WEBHOOK_DELIVERY_TASKS` is 32 and cast only for semaphore permit acquisition. | Webhook delivery pool tests. |

### Resolver/raw stream candidates

| Candidate | Scope | Classification | Severity | Confidence | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `read_raw_frame(reader, length)` caller-supplied allocation | Client SDKs + Network Runtime | Fixed | Medium | High | BUG-035: raw frame reads now reject lengths over `DEFAULT_MAX_FRAME_LEN` by default and expose `read_raw_frame_with_max` for stricter callers. | `oversized_raw_frame_is_rejected_before_payload_read`; `cargo test -p slskr-client`. |
| `ServerConnection::connect` public SDK helper | Client SDKs + Network Runtime | Fixed | Medium | High | BUG-036: default server connects now use `DEFAULT_CONNECT_TIMEOUT`; callers needing tighter policy can call `connect_with_timeout`. | `cargo test -p slskr-client`; runtime callers already wrap with app-specific timeouts. |
| `connect_peer_messages`, `connect_distributed`, `connect_file_transfer` public SDK helpers | Client SDKs + Network Runtime | Fixed | Medium | High | BUG-036: direct peer/distributed/file-transfer SDK helpers now use `DEFAULT_CONNECT_TIMEOUT` and expose timeout-specific variants. | `cargo test -p slskr-client`; daemon call sites continue using shorter configured peer timeouts. |
| Daemon direct peer/file connect call sites | Backend/API + Network Runtime | Existing Guard | Low | High | `crates/slskr/src/main.rs` wraps direct and indirect peer/file `TcpStream::connect`, handshake, read, write, and transfer operations in `state.config.peer_response_timeout`. | Existing daemon peer/transfer tests plus client tests. |
| HTTP request body reads | Backend/API | Existing Guard | Low | High | `http_server.rs` wraps body reads in `BODY_READ_TIMEOUT` and caps body size before routing. | HTTP server tests and release gate. |
| WebSocket client frame reads | Backend/API | Existing Guard | Low | High | `events_ws.rs` caps client frames at 64 KiB, rejects malformed control frames, and the reader task is owned by connection shutdown. | WebSocket auth/frame tests. |
| Webhook outbound URL resolver | Backend/API + Release/Ops | Existing Guard | Low | High | Webhook URLs are validated/resolved before client construction, private/blocklisted addresses are rejected, and reqwest is configured with the resolved address and a delivery timeout. | `scripts/check-webhook-outbound-policy.sh`; webhook tests. |
| Lidarr integration URL resolver | Backend/API + Docs/Config | Existing Guard | Low | High | Integration base URLs reject private/reserved/public-suffix escapes and pin resolved addresses into the reqwest client with configured timeouts. | Integration resolver tests in `slskr`. |
| Test/CLI/examples socket helpers | Tests/Tooling | False Positive | Low | High | Remaining raw socket hits are contract tests, local smoke tests, CLI diagnostics, examples, README snippets, or generated/non-source artifacts excluded from scanner counts. | Keep out of production bug ledger unless promoted by a failing gate. |

### Task/cancellation/lifecycle candidates

| Candidate | Scope | Classification | Severity | Confidence | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| TypeScript SDK request abort timer cleanup | Client SDKs | Fixed | Medium | High | BUG-037: request timers are now cleared in a `finally` block when `fetch` resolves or rejects, preventing timer buildup across failed requests/retries. | `client-ts/src/client.test.ts`; `npm test -- --runInBand`; SDK gate runs TS tests/build. |
| TypeScript SDK explicit zero retry/timeout config | Client SDKs | Fixed | Medium | High | BUG-038: constructor defaults now use nullish coalescing so `retries: 0`, `timeout: 0`, and `retryDelay: 0` are honored instead of replaced by defaults. | `client-ts/src/client.test.ts`; `npm test -- --runInBand`; SDK gate runs TS tests/build. |
| Daemon session manager task | Backend/API + Network Runtime | Existing Guard | Low | High | Session commands use bounded `mpsc`, receive/readiness are wrapped in timeouts, reconnect uses configured delay, and the task exits when the command channel closes. | Session and API route tests. |
| Daemon listener connection tasks | Network Runtime | Existing Guard | Low | High | Listener accept loops update state, per-connection handling is split into tasks, and HTTP connections are capped by a semaphore. | Listener/client contract tests and remediation baseline. |
| WebSocket event stream reader and heartbeat | Backend/API | Existing Guard | Low | High | Client frame reads are capped, invalid frames close the stream, broadcast lag is replayed from bounded event history, and the reader task is aborted when the stream exits. | WebSocket event/auth tests. |
| Webhook delivery tasks | Backend/API + Release/Ops | Existing Guard | Low | High | Registered webhooks are capped, delivery concurrency is capped by semaphore, timeouts are clamped, redirects are disabled, and delivery pool saturation drops work instead of queueing unbounded requests. | Webhook tests and outbound policy gate. |
| React/dashboard abort controllers | Frontend/API Handling | Existing Guard | Low | High | Fetch hooks and player panes abort pending requests on cleanup and ignore abort errors. | Frontend test/build gates. |
| CLI/tests/examples sleeps, spawns, and contract timeouts | Tests/Tooling | False Positive | Low | High | Remaining lifecycle hits are bounded CLI diagnostics, local smoke tests, contract servers, examples, or README snippets. | Keep scoped to test/example review unless promoted by failing gates. |

### Example Web API candidates

| Candidate | Scope | Classification | Severity | Confidence | Evidence | Follow-up |
| --- | --- | --- | --- | --- | --- | --- |
| `docs/http-api-features.md` raw browser WebSocket examples | Docs/Config + Frontend/API Handling | Fixed | Medium | High | BUG-039: raw browser examples now pass the `slskr.api-token.<encoded-token>` WebSocket subprotocol instead of omitting auth. | `scripts/check-websocket-auth-coverage.sh`; docs freshness/baseline. |
| `docs/http-api-features.md` Node.js WebSocket example | Docs/Config + Client SDKs | Fixed | Medium | High | BUG-039: Node example now uses the supported WebSocket auth subprotocol array instead of an incompatible browser-style constructor object/header pattern. | `scripts/check-websocket-auth-coverage.sh`; docs freshness/baseline. |
| `Authorization: Bearer` curl examples | Docs/Config | Existing Guard | Low | High | Bearer auth remains a supported HTTP API auth mechanism and examples use placeholders or `SLSKR_API_TOKEN`, not hard-coded secrets. | Secret scanning and docs freshness gates. |
| `Access-Control-Allow-Origin` warning | Docs/Config | Existing Guard | Low | High | Deployment docs explicitly warn not to use wildcard CORS for authenticated browser deployments. | `scripts/check-docs-freshness.sh`. |
| `localhost:8080` local examples | Docs/Config + Client SDKs | Existing Guard | Low | High | Localhost URLs are local dev defaults in README, SDK examples, tests, and OpenAPI server metadata; deployment docs cover production binding/auth posture separately. | Kubernetes public posture and docs freshness gates. |
| Browser `localStorage` hits | Frontend/API Handling | Existing Guard | Low | High | Token storage regressions are covered by `scripts/check-browser-token-persistence.sh`; remaining production uses are non-secret UI preferences/caches or documented migration fallbacks, with token mentions in tests. | Browser token persistence gate. |
| `_blank` links and `window.open` | Frontend/API Handling | Existing Guard | Low | High | Links include `rel="noopener noreferrer"` and programmatic opens use `safeOpenBlank` with `noopener,noreferrer` plus `opener = null`. | `scripts/check-unsafe-blank-opens.sh`. |
| SDK `WebSocketClient` examples | Client SDKs | Existing Guard | Low | High | SDK examples route through `WebSocketClient`, whose implementation applies `websocketAuthProtocols(token)`. | TypeScript SDK build/test and WebSocket auth coverage gate. |
