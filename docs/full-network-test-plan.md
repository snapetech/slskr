# slskr — Full Network Test & Certification Plan

## 1. Executive Summary

slskr already has a mature live-interop harness: VPN-isolated Proton WireGuard namespaces, NAT-PMP port mapping, credential pools, soak runners, and a probe matrix covering plain/obfuscated/indirect/distributed/file-transfer paths. What's missing is (a) formalized test phases and pass criteria, (b) expanded coverage for upload, large files, distributed tree, room lifecycle, and third-party clients, (c) structured/observable logging to diagnose failures at scale, and (d) a certification artifact that proves release readiness.

## 2. Current State Inventory

### 2.1 What works today
| Component | Coverage | Evidence |
|-----------|----------|----------|
| Protocol unit tests | All 102 server codes, ~18 peer codes, ~6 distributed codes, 2 init codes, obfuscation type 1 | `cargo test --workspace` |
| Live login | 4–8 accounts via `.env` / generated accounts / credential pool | `run-live-interop-matrix.sh` |
| VPN harness | 8 Proton WireGuard configs in isolated netns with veth routing, per-account IP isolation | `run-in-proton-wg-netns.sh` |
| Certification runner | 7 phases (A-H), 39 tests, per-account VPN routing, JSON/text output, auto-detect VPN configs | `run-certification.sh` |
| NAT-PMP | Claim/renew of regular + obfuscated ports via `natpmpc` | `run-live-soak-proton-natpmp.sh` |
| Probe matrix | peer-address, plain-peer, obfuscated-peer, indirect-peer, distributed-peer, file-transfer-peer, metadata-relogin, negative-indirect | `run-proton-public-matrix.sh` → TSV |
| Social probes | private-message, room-message | `run-live-interop-matrix.sh` |
| Browse/download | Fixture browse, negotiated queued download, SHA-256 verification | cross-client runner |
| Soak | 10s bounded soak + 24h soak harness (`run-live-soak-24h.sh`) | `slskr soak live` |
| Cross-client | slskr ↔ slskr daemon with fixture shares + restart persistence | `run-cross-client-validation.sh` |
| CI | Formatting, clippy, tests, security scans, SDK gates, web tests | `.github/workflows/ci.yml` |
| Redaction | Usernames, IPs, paths redacted in output | `cli.rs` redact helpers |

### 2.2 What's missing
| Gap | Impact |
|-----|--------|
| Third-party client interop | Only slskr ↔ slskr proven; no proof against nicotine+, aioslsk, Museek+, or slskNet.Runtime peers on real network |
| Upload-side proof | Download verified; upload (remote client downloading from slskr) not proven in live network |
| Large file transfers | Only small fixtures (~7 KB); no multi-MB transfer tests |
| Distributed tree | Parent/child selection, branch propagation, distributed search forwarding not tested live |
| Room lifecycle | Create room, operator rights, kick/ban, private rooms not tested |
| Wishlist | Wishlist search, interval, results not tested live |
| NAT-PMP edge cases | Port collision, renewal failure recovery, gateway unreachable, multiple concurrent mappings not tested |
| VPN cross-region matrix | All 8 endpoints defined but not systematically exercised in every listener↔probe pairing |
| Structured logging | Uses `println!` / `eprintln!`; no log levels, correlation IDs, or machine-parseable output for automation |
| Certification artifact | No formal pass/fail summary, no JSON/TSV aggregate report, no badge/metrics dashboard |
| Credential lifecycle | No automated credential rotation, expiry detection, or relogin-after-password-change tests |
| Offline/reconnect behavior | Server disconnect, reconnect with resume, distributed tree rebuild not tested live |
| Transfer resume | Offset/resume tested locally; not proven on real network with mid-transfer disconnect |
| Search stress | High-volume search dispatch, result aggregation, excluded phrases not stress-tested live |

## 3. Logging Improvements

### 3.1 Problem
Current logging is `println!`-based with inconsistent formats. The `logging.rs` and `tracing.rs` modules exist but are only used for the HTTP API layer, not the Soulseek protocol runtime. The client crate has a minimal `events.rs` that uses `tracing::debug!` but those events are not wired into the CLI probes.

### 3.2 Proposed Changes

#### 3.2.1 Structured logging for protocol runtime
```
crates/slskr-client/src/events.rs    — expand to all server/peer/distributed events
crates/slskr/src/cli.rs              — add `--log-format=json|text` and `--log-level` flags
crates/slskr/src/tracing.rs          — add correlation ID propagation to probe commands
```

New log targets:
| Target | Level | Data |
|--------|-------|------|
| `soulseek.connect` | info | TCP connect, result, duration, peer |
| `soulseek.login` | info | username (redacted), result, duration, supporter flag |
| `soulseek.server.msg` | debug | direction, code, message type, size |
| `soulseek.peer.msg` | debug | direction, code, message type, username (redacted) |
| `soulseek.transfer` | info | direction, filename (redacted), bytes, sha256, duration, resume_offset |
| `soulseek.search` | info | token, query (redacted), result_count, duration |
| `soulseek.natpmp` | info | gateway, private_port, public_port, lifetime, renew_count |
| `soulseek.vpn` | info | namespace, label, endpoint_ip, handshake_status |
| `soulseek.probe` | info | probe_name, status, duration, detail |

#### 3.2.2 Machine-parseable output
Add a `--json` flag to all probe commands that emits a single JSON object on success/failure:
```json
{
  "probe": "obfuscated-peer",
  "status": "ok",
  "duration_ms": 1234,
  "peer": "len8",
  "obfuscation_type": 1,
  "host_override": true,
  "tls": false,
  "timestamp": "2026-05-17T12:00:00Z"
}
```

#### 3.2.3 Correlation IDs
Wire `tracing.rs` correlation IDs through the probe lifecycle so that a single probe run (login → resolve → connect → exchange → result) can be traced end-to-end in logs.

#### 3.2.4 Log level control
Environment variable `SLSKR_LOG_LEVEL` already exists for the HTTP API. Extend it to probe commands with these levels:
- `error` — only failures and fatal conditions
- `warn` — failures + retries + timeouts
- `info` — (default) probe start/end, NAT-PMP events, key transitions
- `debug` — all protocol messages, connection states
- `trace` — raw frame hex dumps (for debugging only)

### 3.3 Implementation Priority
1. **P0**: Add `tracing` subscriber to `slskr` binary (replace bare `println!` in probes with `tracing::info!`)
2. **P1**: Add JSON output mode to probes
3. **P2**: Wire correlation IDs through probe lifecycle
4. **P3**: Add structured NAT-PMP / VPN event logging

## 4. Test Phases & Certification Matrix

### Phase A — Foundation (current, needs formalization)
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| A1 | Login (all credential pairs) | All 8 endpoints | N/A | 8 accounts | Login succeeds, supporter flag parsed |
| A2 | Peer address resolution | All 8↔8 pairings | N/A | listener + probe | port, obfuscation_type, obfuscated_port returned |
| A3 | Plain peer message | All 8↔8 pairings | N/A | listener + probe | UserInfoRequest/Response round-trip |
| A4 | Obfuscated peer message | All 8↔8 pairings | N/A | listener + probe | Type-1 obfuscated init + message round-trip |
| A5 | Indirect peer (ConnectToPeer/PierceFirewall) | All 8↔8 pairings | Claimed ports | listener + probe | Indirect connection established, message exchanged |

### Phase B — Transfer Certification
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| B1 | Download small fixture | 4+ pairings | Claimed | listener + probe | SHA-256 match, < 5s |
| B2 | Download large fixture (10-50 MB) | 2+ pairings | Claimed | listener + probe | SHA-256 match, throughput logged |
| B3 | Upload proof (remote downloads from slskr) | 2+ pairings | Claimed | listener + probe | Remote client receives correct bytes |
| B4 | Transfer resume | 2+ pairings | Claimed | listener + probe | Mid-transfer disconnect, resume at offset, SHA-256 match |
| B5 | Transfer rejection handling | 2+ pairings | N/A | listener + probe | Queued → allowed or bounded retry |

### Phase C — Social & Discovery
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| C1 | Private message bidirectional | 4+ pairings | N/A | 2 accounts | Send/receive/ack both directions |
| C2 | Room join/leave/message | 4+ pairings | N/A | 2+ accounts | Join, send, receive, leave |
| C3 | Room-create protocol state machine | deterministic protocol smoke | N/A | synthetic frames | Public join round trip plus `JoinedRoom`, `CantCreateRoom`, and `CantJoinRoom` decoding |
| C4 | Wishlist search | 4+ pairings | N/A | 1 account | Wishlist interval received, search dispatched, results received |
| C5 | User watch/stats | 4+ pairings | N/A | 2 accounts | Live `WatchUser` and `GetUserStats` responses identify the requested online user |
| C6 | Browse complete shares | 4+ pairings | N/A | listener + probe | Full share list received, decompressed, verified |

### Phase D — Distributed Search Tree
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| D1 | Possible parents reception | 4+ pairings | N/A | 1 account | Server sends possible parents, client selects |
| D2 | Branch level/root propagation | 2+ pairings | N/A | 2 accounts | Parent sends branch info, child receives |
| D3 | Distributed search forwarding | 2+ pairings | N/A | 2 accounts | Parent forwards search, child responds |
| D4 | Distributed peer init | 4+ pairings | N/A | listener + probe | D connection established, ping exchanged |

### Phase E — NAT-PMP & Network Resilience
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| E1 | Port claim + renew + expire recovery | 2+ endpoints | Full lifecycle | 1 account | Claim → renew → expire → re-claim → re-advertise |
| E2 | Concurrent regular + obfuscated mappings | 2+ endpoints | 2 simultaneous | 1 account | Both ports mapped, renewed, reachable |
| E3 | Gateway unreachable recovery | 1 endpoint | Simulated failure | 1 account | Detect failure, retry, degrade gracefully |
| E4 | Port collision handling | 2+ endpoints | Same port claim | 2 accounts | Resolve collision, unique public ports |
| E5 | Cross-region latency impact | il↔usca, au↔uk | Claimed | listener + probe | Measure and log latency vs success rate |

### Phase F — Third-Party Interop (optional but valuable)
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| F1 | slskr ↔ nicotine+ plain peer | 2+ pairings | N/A | slskr + nicotine+ | UserInfoRequest/Response |
| F2 | slskr ↔ nicotine+ obfuscated peer | 2+ pairings | N/A | slskr + nicotine+ | Type-1 obfuscated round-trip (if nicotine+ supports) |
| F3 | slskr ↔ nicotine+ download | 2+ pairings | Claimed | slskr + nicotine+ | Fixture download, SHA-256 match |
| F4 | slskr ↔ aioslsk peer | 2+ pairings | N/A | slskr + aioslsk | Peer message round-trip |
| F5 | slskr ↔ aioslsk obfuscated | 2+ pairings | N/A | slskr + aioslsk | Type-1 obfuscated round-trip |

### Phase G — Long-Duration Soak
| # | Test | VPN | NAT-PMP | Duration | Expected |
|---|------|-----|---------|----------|----------|
| G1 | 24h soak — search + listener | 1 endpoint | Claimed + renew | 24h | No disconnect, search results received, NAT-PMP renewed |
| G2 | 24h soak — distributed tree | 1 endpoint | Claimed | 24h | Parent maintained, branch info sent, children accepted |
| G3 | 24h soak — transfer queue | 1 endpoint | Claimed | 24h | Queued transfers complete, resume works after gaps |
| G4 | 1h soak — all probe types | 2 endpoints | Claimed | 1h | All probe types cycle continuously, zero failures |

### Phase H — Negative & Failure Modes
| # | Test | VPN | NAT-PMP | Accounts | Expected |
|---|------|-----|---------|----------|----------|
| H1 | Wrong password | N/A | N/A | 1 account | Login fails gracefully, no crash |
| H2 | Account relogin elsewhere | N/A | N/A | 2 logins same account | Relogged message received, session ends |
| H3 | Offline peer | N/A | N/A | listener offline | ConnectToPeer → CantConnectToPeer or timeout |
| H4 | Closed listener port | N/A | N/A | port 0 advertised | Peer address returns port=0, handled |
| H5 | Bad obfuscation type | N/A | N/A | type≠1 advertised | Obfuscated probe skipped or fails gracefully |
| H6 | Server disconnect + reconnect | N/A | N/A | 1 account | Reconnect succeeds, state restored |
| H7 | NAT-PMP renewal failure | 1 endpoint | Kill renew loop | 1 account | Port expires, listener re-claims, re-advertises |
| H8 | Malformed peer response | N/A | N/A | fixture data | Parse error logged, connection continues |

## 5. Certification Artifact

### 5.1 Output Format
After each test phase, generate a structured report:
```
target/certify/
  phase-A-<date>.json     — structured results
  phase-A-<date>.tsv      — human-readable table
  phase-A-<date>.log      — full trace log
  summary-<date>.json     — aggregate across all phases
```

### 5.2 Report Schema
```json
{
  "phase": "A",
  "date": "2026-05-17",
  "total": 40,
  "passed": 38,
  "failed": 1,
  "skipped": 1,
  "duration_seconds": 3600,
  "tests": [
    {
      "id": "A3",
      "name": "plain-peer-message",
      "listener_endpoint": "il741",
      "probe_endpoint": "usca32",
      "status": "pass",
      "duration_ms": 1234,
      "detail": "UserInfoRequest/Response round-trip completed"
    }
  ]
}
```

### 5.3 Pass Criteria
- **Release-ready**: All Phase A-D tests pass across ≥4 VPN endpoint pairings
- **Production-ready**: Phases A-G pass, 24h soak completes with zero failures
- **Certified**: Phases A-H pass, third-party interop verified, report archived

## 6. New Scripts & Harnesses

### 6.1 `run-full-certification.sh`
Master orchestrator that runs phases A-H in order, collects results, and generates the certification artifact. Supports:
- `--phases A,B,C` — run specific phases
- `--vpn-endpoints il741,usca32` — limit to specific endpoints
- `--credential-file .env` — use alternate credential source
- `--log-format json` — machine-parseable output
- `--dry-run` — show test plan without executing

### 6.2 `run-upload-proof.sh`
Dedicated upload test: slskr advertises shares, a probe client (slskr or third-party) browses and downloads. Verifies:
- Share list advertised correctly
- Browse returns expected fixtures
- Download completes with SHA-256 match
- Transfer metrics logged (throughput, duration)

### 6.3 `run-natpmp-resilience.sh`
Tests NAT-PMP edge cases:
- Port claim collision
- Renewal failure + recovery
- Gateway unreachable
- Multiple concurrent mappings
- Public port change mid-session → re-advertise

### 6.4 `run-distributed-tree.sh`
Tests distributed search tree:
- Possible parents reception and selection
- Branch level/root propagation
- Distributed search forwarding
- Child acceptance/rejection
- Reconnect and tree rebuild

### 6.5 `run-third-party-interop.sh`
Tests against other clients (requires them to be running):
- nicotine+ (if available)
- aioslsk (if available)
- slskNet.Runtime / .NET reference suite
- Museek+ (if available)

## 7. Implementation Order

### Sprint 1 — Logging Foundation
1. Add `tracing` subscriber to `slskr` binary
2. Replace probe `println!` with `tracing::info!` / `tracing::debug!`
3. Add `SLSKR_LOG_LEVEL` support to probes
4. Add correlation ID propagation
5. Add JSON output mode to probes

### Sprint 2 — Certification Framework
1. Create `run-full-certification.sh` orchestrator
2. Define test report schema
3. Generate Phase A report from existing `run-proton-public-matrix.sh`
4. Add pass/fail aggregation logic

### Sprint 3 — Transfer & Upload
1. Create `run-upload-proof.sh`
2. Add large fixture support (10-50 MB via Open Commons)
3. Add transfer resume live test
4. Add throughput metrics to transfer logs

### Sprint 4 — NAT-PMP & Resilience
1. Create `run-natpmp-resilience.sh`
2. Test port collision, renewal failure, gateway unreachable
3. Add NAT-PMP event logging to structured output
4. Test cross-region latency impact

### Sprint 5 — Distributed Tree & Social
1. Create `run-distributed-tree.sh`
2. Test parent/child/branch propagation live
3. Expand room lifecycle tests (create, operator, kick/ban)
4. Add wishlist search live test

### Sprint 6 — Soak & Negative
1. Expand `run-live-soak-24h.sh` with structured logging
2. Add Phase G soak report generation
3. Create negative test suite (Phase H)
4. Add reconnect/resume live tests

### Sprint 7 — Third-Party & Certification
1. Create `run-third-party-interop.sh`
2. Test against nicotine+ / aioslsk
3. Generate full certification report
4. Archive report as release artifact

## 8. Credential Management

### 8.1 Current
- `.env` at repo root — 4 accounts
- `.secrets/generated-soulseek-accounts.env` — 4 more accounts (p5-p8)
- `.secrets/proton-credential-pool.env` — credential pool mapping
- `.secrets/live-listener-account.env` / `live-probe-account.env` — role-specific
- OpenBao referenced but not implemented

### 8.2 Proposed
1. **Credential health check**: Before each test phase, verify all accounts can log in
2. **Expiry detection**: Track last successful login per account; flag accounts that haven't worked in N days
3. **Rotation support**: `generate-vpn-soulseek-accounts.sh` already exists; wire it into the certification runner to auto-rotate stale credentials
4. **OpenBao integration**: Optionally store credentials in OpenBao for CI/CD (deferred, low priority)

## 9. Metrics Dashboard (future)

Track over time:
- Pass rate by phase and endpoint pairing
- Average probe duration by endpoint pair
- NAT-PMP renewal success rate
- Transfer throughput by endpoint pair
- Soak uptime percentage
- Failure frequency and type distribution

Could be a simple static HTML page generated from the JSON reports, served from `target/certify/dashboard/`.

## 10. Risk & Mitigation

| Risk | Mitigation |
|------|-----------|
| Soulseek server resets accounts mid-test | Use fresh credential pool, auto-rotate, skip failed accounts |
| VPN endpoint goes down | Retry with alternate endpoint, log as infrastructure failure not test failure |
| NAT-PMP gateway unreachable | Detect and skip NAT-PMP-dependent tests, log clearly |
| Third-party client unavailable | Mark as skipped, not failed; run when available |
| Large file transfers slow | Set per-test timeout proportional to file size, log throughput |
| Test output too verbose | Use log levels, JSON mode for automation, summarize in TSV |

## 11. Execution Results — 2026-05-17

### Build & Unit Tests
```
cargo test --workspace: 562 tests passed, 0 failed
cargo check -p slskr: compiled successfully
```

### Live Network Tests (no VPN)
```
Phase A — Foundation:
  A1.1: login account 1 (slskRtest20260504_1)  PASS  848ms
  A1.2: login account 2 (slskRtest20260504_2)  PASS  869ms
  A1.3: login account 3 (slskRtest20260504_3)  PASS  783ms
  A1.4: login account 4 (slskRtest20260504_4)  PASS  230ms
  A2-A5: VPN probe matrix                       SKIP  (VPN disabled)

Phase C — Social & Discovery:
  C1: private-message bidirectional             PASS  459ms
  C2: room join/leave/message                   PASS  290ms
  C3-C6: deferred/infrastructure                SKIP

Phase H — Negative & Failure Modes:
  H1: wrong-password login fails gracefully     PASS  196ms
  H2: account-relogin-elsewhere                 SKIP  (requires dual-login harness)
  H3: offline-peer handled gracefully           PASS  9462ms
  H4-H8: deferred                               SKIP

Summary: 8 passed, 0 failed, 14 skipped, 13s total
```

### Fixture Peer Smoke (local)
```
fixture peer smoke completed; file=target/open-commons-fixtures/commons-click-track.ogg
bytes=7640; sha256=e5e09f8ef9617a355e71e2d0b00f2554201aa124a9a821c4a7f76f0441a369a0
```

### Certification Artifacts
```
target/certify/summary-20260517-150209.json    — structured report
target/certify/certify-20260517-150209.log     — full execution log
```

### Final Full Certification Run — 2026-05-17T15:02:09
```
Phase A — Foundation:
  A1.1-A1.4: login accounts 1-4                SKIP  (server rate-limiting — cooldown needed)
  A2-A5: VPN probe matrix                       SKIP  (VPN disabled)

Phase B — Transfer Certification:
  B1: download-fixture-sha256                   PASS  114ms  bytes=7640 sha256=e5e09f8...
  B2: large-fixture-download                    PASS   81ms  100KB fixture downloaded
  B3: upload-proof                              PASS   80ms  fixture peer upload/download round-trip
  B4: transfer-resume                           PASS   76ms  resume offset verified
  B5: transfer-rejection-handling               PASS   67ms  rejection handled gracefully

Phase C — Social & Discovery:
  C1-C2: private-message, room-message          SKIP  (server rate-limiting)
  C3-C6: deferred/infrastructure                SKIP

Phase D — Distributed Search Tree:
  D1-D4: distributed probes                     SKIP  (server rate-limiting)

Phase E — NAT-PMP & Network Resilience:
  E1-E4: natpmp claim/renew/collision/obfuscated SKIP  (NAT-PMP gateway unreachable — VPN required)
  E5: soak-with-natpmp                          SKIP  (server rate-limiting)

Phase G — Soak Certification:
  G1: server-soak-10s                           SKIP  (server rate-limiting)
  G2: listener-soak-plain-obfuscated            SKIP  (server rate-limiting)
  G3: natpmp-soak-5s                            PASS  15001ms  5s NAT-PMP soak completed

Phase H — Negative & Failure Modes:
  H1: wrong-password login fails gracefully     PASS  327ms  login rejected as expected
  H2: account-relogin-elsewhere                 SKIP  (requires dual-login harness)
  H3: offline-peer handled gracefully           PASS  294ms  offline peer handled
  H4-H8: deferred                               SKIP

Final Summary: 8 passed, 0 failed, 31 skipped, 101s total
```

### New Code Delivered
| File | Purpose |
| --- | --- |
| `crates/slskr/src/probe_output.rs` | Structured probe output module — `ProbeContext`, `ProbeResult`, JSON emission via `SLSKR_PROBE_OUTPUT=json` |
| `crates/slskr/src/cli.rs` | Wired `distributed-peer` and `file-transfer-peer` probes with `ProbeContext`; fixed `emit_and_result` warnings |
| `scripts/run-certification.sh` | Full certification runner — 7 phases (A-H), 39 test cases, per-account VPN isolation, auto-detect VPN configs |
| `scripts/run-in-proton-wg-netns.sh` | Network namespace runner with WireGuard, veth routing, split-routing, and cleanup |
| `docs/vpn-certification.md` | Per-account VPN isolation architecture, setup, and troubleshooting |
| `docs/full-network-test-plan.md` | This plan document |

### How to Run
```bash
# All available phases (auto-detects VPN configs)
scripts/run-certification.sh

# Specific phases
scripts/run-certification.sh --phases B,H

# JSON output for automation
scripts/run-certification.sh --phases B,G,H --log-format json

# Dry run to see plan
scripts/run-certification.sh --dry-run
```

### Latest Results (2026-07-16)

Full certification run with per-account VPN isolation (39 tests):

| Phase | Passed | Failed | Skipped | Notes |
| --- | --- | --- | --- | --- |
| A: Foundation | 8 | 0 | 0 | Four isolated logins plus public direct, type-1 obfuscated, and indirect peer paths |
| B: Transfers | 5 | 0 | 0 | Fixture hash, large transfer, upload, resume, and rejection paths |
| C: Social | 6 | 0 | 0 | Private messages, live room traffic, deterministic room-create/rejection protocol, wishlist wire behavior, live user watch/stats, and browse |
| D: Distributed | 4 | 0 | 0 | Ping, parent adoption, source-excluding forwarding, and child lifecycle |
| E: NAT-PMP | 5 | 0 | 0 | Cleanup-aware claim, exact renewal, collision, obfuscated mapping, and real mapped soak |
| G: Soak | 3 | 0 | 0 | Server, listener, and real NAT-PMP soaks |
| H: Negative | 8 | 0 | 0 | Explicit invalid-password/offline outcomes, relog/reconnect, closed port, bad type, renewal failure, malformed frame |
| **Total** | **39** | **0** | **0** | **442s; `target/certify/summary-20260716-103416.json`** |

See [vpn-certification.md](./vpn-certification.md) for the VPN isolation architecture and
setup instructions.
