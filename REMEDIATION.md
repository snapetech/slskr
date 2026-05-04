# slskR Remediation Plan

This document is the single source of truth for what is real, what is theater,
and what we are going to do about it. Anyone (human or agent) picking up this
project should read this before producing more code or markdown.

If you find yourself writing `FINAL_*.md`, `*_COMPLETION_*.md`, or
`PHASE_N_DONE.md` again, stop. Update *this* document instead.

---

## 0. Snapshot (2026-05-04)

- Branch `main` is **92 commits ahead of `origin/main`**, none pushed.
- `cargo check -p slskr` **fails** with 8 errors (untracked `http_server.rs`
  referenced from `main.rs:9668`, missing `AsyncReadExt` import, missing fields
  on `RequestSecurityHeaders`, `ResponseCache: !Debug`).
- `cargo check -p slskr-protocol -p slskr-client` passes; their tests pass.
- `cargo test -p slskr` does not compile (same errors plus a fake
  `tests/integration_tests.rs`).
- `crates/slskr/src/main.rs` is **14,301 lines, 101 functions, 71 inline tests**,
  guarded by `#![allow(dead_code, unused_imports)]`.
- 54 root-level `.md` files. ~20 of them are duplicate "we are done" reports.

---

## 1. Trust map

| Area                          | Status     | Why                                                                 |
| ----------------------------- | ---------- | ------------------------------------------------------------------- |
| `crates/slskr-protocol`       | TRUST      | Real Soulseek wire codecs, 7 test files, all passing                |
| `crates/slskr-client`         | TRUST      | Real session/listener/transfer runtime, 14 test files, all passing  |
| `crates/slskr-cli`            | TRUST      | Real probe/admin commands the README documents                      |
| `scripts/`                    | TRUST      | Live-soak and Proton matrix scripts are serious infra               |
| `docs/` (most)                | TRUST      | `app-surface.md`, `install.md`, `legacy-port-harvest.md`, etc.      |
| `crates/slskr` (bin)          | DISTRUST   | God-file `main.rs`, ~20 ghost modules, fake handlers                |
| Root `*.md` (most)            | DELETE     | Repeating victory laps from prior agent runs                        |
| `web/`, `dashboard/`          | UNAUDITED  | Frontends — depend on API surface that is partly theater            |
| `client-go`, `-python`, `-ts` | UNAUDITED  | SDKs hit `/api/events/ws` which the server does not implement       |

---

## 2. Intent (what the project is actually trying to be)

Established from `README.md`, `PLAN.md`, `docs/app-surface.md`, and the legacy
slskd parity notes:

1. A real, independent Rust implementation of the Soulseek protocol.
   **— `slskr-protocol` and `slskr-client` deliver this.**
2. One bundled app, `slskr`, that runs as a daemon with an HTTP API and a
   bundled web UI, mirroring how slskd is shipped.
   **— `slskr` (bin) delivers a hand-rolled HTTP server, but most of the
     "Phase 6/8/9/10/11/12" surface around it is decorative.**
3. Probe-driven validation against live Soulseek (matrix runs, Proton NAT-PMP).
   **— `slskr-cli` and `scripts/` deliver this.**

Out of scope (and explicitly to be removed if any artifact remains):
distributed clustering, sharding, gRPC, HTTP/2 multiplexing, "500K req/sec"
performance theatre, GraphQL, SSE, three-layer Redis/Postgres caching. The
`709ff6c2` cleanup commit started this; we finish it here.

---

## 3. API surface that real consumers actually use

Pulled from `web/src`, `dashboard/src`, `client-{go,python,ts}` — node_modules
filtered out. Use this list to decide what the bin crate must keep working.

**Plain HTTP (59 distinct endpoints in use):**

```
/api/health                       /api/version                    /api/capabilities
/api/stats                        /api/metrics                    /api/sessions[/...]
/api/config                       /api/admin/config
/api/admin/api-keys[/...]         /api/admin/webhooks[/...]
/api/admin/database/{stats,vacuum,cleanup}
/api/cache/{stats,invalidate}
/api/searches[/...]               /api/search
/api/transfers[/...]              /api/messages[/...]
/api/rooms[/...]                  /api/rooms/join
/api/users[/...]                  /api/browse/...                 /api/browse/requests[/...]
/api/shares                       /api/shares/refresh
/api/library/health/issues[/by-artist|by-release|by-type|fix]
/api/library/health/scans[/...]   /api/library/health/summary
/api/events                       /api/batch                      /api/filters
```

**Streaming:**

- `/api/events/ws` — plain WebSocket. Used by all three SDKs. **Server does not
  implement this route today.** (`/api/events` polling endpoint exists.)
- SignalR hubs at `/application`, `/logs`, `/search`, `/songid`,
  `/listening-party`, `/transfers`. Used by the React web UI via
  `@microsoft/signalr` (`web/src/lib/hubFactory.js`). **Server does not
  implement SignalR; current stub returns 501.**

**Not used by anyone:**

- GraphQL — zero hits in any client.
- Server-Sent Events — zero hits in any client.
- The "v0" mesh/podcore/streams/sha256 paths grepped from clients are from a
  different project's SDK that ended up vendored — ignore.

This is the contract we owe consumers. Everything outside it can be cut.

---

## 4. Decisions (made, not negotiable in this round)

| #  | Decision                                                                                       | Rationale                                                                                  |
| -- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| D1 | **Keep the manual HTTP server in `main.rs`. Drop Axum/Tower entirely.**                        | Manual server is what's actually wired; Axum router is 495 lines of `{"created": true}`.  |
| D2 | **Finish the manual server**: fix Content-Length-driven body reads, keep-alive, headers.       | The 4 KB single-read in `handle_http_connection` truncates real bodies.                    |
| D3 | **Fold `http_server.rs` into `main.rs` as `mod http_server` (committed).**                     | It's the right idea, just untracked and uncompilable. Track it, fix it, use it.            |
| D4 | **Implement `/api/events/ws` with `tokio-tungstenite`. Delete `websocket.rs`.**                 | Three SDKs already point at it; current `websocket.rs` is a fake-handshake toy.            |
| D5 | **SignalR: replace UI's SignalR usage with plain WebSocket. Delete `signalr_hub.rs`.**          | A real SignalR server is months of work; the UI hubs are thin and easy to repoint at WS.   |
| D6 | **Delete GraphQL, SSE, middleware, filters, enrichment, versioning, response_cache, observability, rate_limiter (the duplicate), api_keys, api_integration, openapi, docs (the module), validation, pagination, compression, security, metrics, websocket_handler, axum_router, caching.** | All have **zero `module::` call sites** in `main.rs`. They're dead weight. |
| D7 | **Keep `webhooks`, `batch`, `tracing`, `logging`, `routing`, `utils`, `storage`, `config`, `persistence`, `rate_limit`.** | These are the modules `main.rs` actually calls.                                            |
| D8 | **Persistence: keep `persistence.rs`, gate behind a config flag (default off).**               | Schema is fine; current writes are all `let _ =` no-ops, so flipping the flag isn't viable yet. Wire one path (search create) end-to-end as proof-of-life before flipping the default. |
| D9 | **Delete `tests/integration_tests.rs` entirely. Replace with one real e2e test.**              | Current 689 lines compare a string formatter to itself.                                    |
| D10| **Strip `tonic`, `prost`, `sea-orm`, `deadpool-postgres`, `redis`, `moka`, `dashmap`, `axum`, `tower`, `tower-http`, `flate2`, `http`, `tokio-util`, `bytes` from the bin crate.** | Every one is either unused or used only by a module being deleted. (Keep `tokio-tungstenite` for D4.) |
| D11| **Cull root markdown.** Keep `README.md`, `PLAN.md`, `COMPLIANCE.md`, `LICENSE`, `NOTICE`, this file. Move everything else to `archive/` in one commit, then delete in a follow-up if nobody objects within a week. | They contradict the code and each other. Honest README first, then optional history.       |
| D12| **No new `FINAL_*` / `COMPLETION_*` / `PHASE_*` files ever again.** Update `REMEDIATION.md` and `PLAN.md`. That's it. |                                                                                            |

---

## 5. Phased plan

Each phase ends with `cargo check --workspace`, `cargo test --workspace
--exclude slskr` (until slskr tests are real), and a single commit. No
intermediate "phase complete" docs.

### Phase 0 — Stop the bleeding (build green, doc lake archived)

- [ ] **0.1** Track `crates/slskr/src/http_server.rs` (currently untracked). Add
      `use tokio::io::AsyncReadExt;` so `read_exact` resolves. Verify it
      compiles in isolation.
- [ ] **0.2** Fix `main.rs:9668..9700` call site of `http_server::read_http_request`:
      align return type and the `RequestSecurityHeaders` struct fields
      (`content_type`, `user_agent` are read but not declared).
- [ ] **0.3** Add `#[derive(Debug)]` to `ResponseCache` (fixes E0277), or skip
      this fix entirely if `response_cache.rs` is being deleted in Phase 1
      anyway (it is — see §7).
- [ ] **0.4** `cargo check -p slskr` is green.
- [ ] **0.5** `mkdir -p archive/` and move every root `.md` file *not* on the
      keep list (D11) into `archive/`. Single commit:
      `chore: archive prior-agent completion docs`.
- [ ] **0.6** Truncate `README.md` Status line to honestly reflect: protocol &
      client crates ship, daemon HTTP API is partial, web UI not wired
      end-to-end yet. (One-paragraph patch.)
- [ ] Commit message: `fix(slskr): get bin crate compiling; archive doc lake`

**Definition of done:** `cargo build --workspace` succeeds. Root `ls *.md`
returns ≤6 entries. No commit message contains the words "FINAL", "COMPLETE",
or "100%".

### Phase 1 — Quarantine ghost modules

Delete the modules with **zero `<module>::` call sites in `main.rs`**, plus
their `mod` declarations and any `use` statements. No replacements yet.

| Module                         | Decision  | Notes                                                |
| ------------------------------ | --------- | ---------------------------------------------------- |
| `axum_router.rs`               | DELETE    | 495 lines of placeholder handlers, never imported.   |
| `graphql.rs`                   | DELETE    | Toy parser, no real GraphQL, no client uses it.      |
| `sse.rs`                       | DELETE    | No client uses SSE.                                  |
| `middleware.rs`                | DELETE    | Never imported.                                      |
| `filters.rs`                   | DELETE    | Never imported.                                      |
| `enrichment.rs`                | DELETE    | Never imported.                                      |
| `versioning.rs`                | DELETE    | Never imported. URL `v0/` is normalized in `main.rs`.|
| `response_cache.rs`            | DELETE    | Never imported.                                      |
| `observability.rs`             | DELETE    | Never imported. Metrics live in `tracing.rs`.        |
| `rate_limiter.rs`              | DELETE    | Duplicate of wired-up `rate_limit.rs`.               |
| `api_keys.rs`                  | DELETE    | Never imported. Auth is bearer-token in `main.rs`.   |
| `api_integration.rs`           | DELETE    | Never imported.                                      |
| `openapi.rs`                   | AUDIT     | 2 call sites — verify both, then fold into `main.rs` if it's just a static-doc handler. |
| `docs.rs`                      | DELETE    | Never imported (the module; `docs/` directory stays).|
| `validation.rs`                | DELETE    | Never imported.                                      |
| `pagination.rs`                | DELETE    | Never imported.                                      |
| `compression.rs`               | DELETE    | Never imported. Self-describes as "placeholder".     |
| `security.rs`                  | DELETE    | Never imported.                                      |
| `metrics.rs`                   | DELETE    | Never imported. `tracing.rs` exposes counters.       |
| `signalr_hub.rs`               | DELETE    | See D5 — replaced by plain WS.                       |
| `websocket.rs`                 | DELETE    | See D4 — replaced by `tokio-tungstenite` impl.       |
| `websocket_handler.rs`         | DELETE    | Companion to `websocket.rs`.                         |
| `caching.rs`                   | DELETE    | Pretends to be 3-layer cache; only moka local layer; never imported. |
| `benchmarks.rs`                | DELETE    | Synthetic micro-benchmarks measuring nothing real.   |

- [ ] **1.1** For each module on the DELETE list above, `git rm` and remove its
      `mod` line from `main.rs:1..36`.
- [ ] **1.2** Audit `openapi.rs`'s 2 call sites and fold into `main.rs` if it's
      just a static-doc handler; otherwise keep it standalone.
- [ ] **1.3** Confirm `cargo check -p slskr` is still green.
- [ ] **1.4** Remove `#![allow(dead_code, unused_imports)]` from the top of
      `main.rs`. Fix the resulting warnings honestly. This is the moment of
      truth for what's actually live.

**Expected delta:** ~7,500 LOC removed from `crates/slskr/src/`. Module count
in `main.rs` header drops from ~30 to ~10.

### Phase 2 — Strip cargo-cult dependencies

After Phase 1, run `cargo machete` (or by inspection) and remove every
dependency that has zero `use` sites:

- [ ] **2.1** Drop from `crates/slskr/Cargo.toml`:
      `tonic`, `prost`, `sea-orm`, `deadpool-postgres`, `redis`, `moka`,
      `dashmap`, `axum`, `tower`, `tower-http`, `flate2`, `http`, `tokio-util`,
      `bytes`. Drop the `postgres` feature from `sqlx`. Keep
      `tokio-tungstenite` for D4; keep `sqlx`/`chrono`/`uuid` for
      `persistence.rs`; keep `reqwest` for `webhooks.rs`; keep
      `hmac`/`sha2`/`hex` for webhook signing.
- [ ] **2.2** Re-run `cargo check --workspace`. Cargo.lock should shrink by
      hundreds of crates.
- [ ] **2.3** Single commit: `chore(deps): drop unused heavy deps from slskr`.

### Phase 3 — Honest HTTP server

- [ ] **3.1** Replace the 4 KB single-read in `handle_http_connection` with a
      `BufReader` loop that:
        1. Reads request line + headers until `\r\n\r\n`.
        2. Parses `Content-Length`.
        3. Reads exactly that many body bytes (cap at 1 MiB; reject larger
           with 413).
- [ ] **3.2** Add keep-alive: HTTP/1.1 default-on, honor `Connection: close`.
- [ ] **3.3** Streaming response writer (no `format!` of full body for large
      responses — the share catalog endpoint is the obvious offender).
- [ ] **3.4** Move parsing/IO into `mod http_server` (already drafted). Have
      `main.rs` call into it. Trim duplicate parsing helpers from `main.rs`.
- [ ] **3.5** Add `tests/http_server.rs` (real, not the formatter-sham): bind
      to `127.0.0.1:0`, hit `/api/health`, `/api/version`, a POST with a 100 KB
      body, and a malformed request. Use `reqwest` (already a dep).

### Phase 4 — Real-time: WebSocket events

- [ ] **4.1** New module `events_ws.rs`: tokio-tungstenite-based handler for
      `/api/events/ws`. On connect, subscribe to the existing event bus
      (whatever `record_event` writes to) and forward as JSON frames.
- [ ] **4.2** Wire `/api/events/ws` route in `main.rs` to upgrade the
      connection and hand off to `events_ws`.
- [ ] **4.3** Add `tests/events_ws.rs`: connect with a real ws client, observe
      that `record_event` triggers a frame.
- [ ] **4.4** Update SDK READMEs (`client-go`, `-python`, `-ts`) only if their
      docs claim the route works today; otherwise leave alone.

### Phase 5 — Real-time: SignalR replacement (web UI)

This is the biggest UI-touching change. It needs a yes from whoever owns the
React UI before we cut.

- [ ] **5.1** **Decision gate:** confirm we are willing to drop SignalR from
      `web/`. (Default per D5 is yes.)
- [ ] **5.2** Replace `web/src/lib/hubFactory.js` with a thin WS client over
      `/api/events/ws` that subscribes to topic-filtered messages
      (`{topic: "transfers", ...}`).
- [ ] **5.3** Update each consumer (`createApplicationHubConnection`, etc.) to
      a topic name. Each existing hub becomes a topic the server tags events
      with.
- [ ] **5.4** Drop `@microsoft/signalr` from `web/package.json`.

### Phase 6 — Persistence proof-of-life

- [ ] **6.1** Wire one full path: search create → `db.insert_search` → on
      restart, `/api/searches` returns the persisted record. This converts
      `_db_result` (4 sites) and friends from no-ops into real writes.
- [ ] **6.2** Gate everything behind `config.persistence.enabled = false` by
      default until all transfer/message/room paths are wired the same way.
- [ ] **6.3** Add an integration test that boots two daemon processes
      back-to-back with the flag on and asserts persistence.

### Phase 7 — Honest tests

- [ ] **7.1** Delete `crates/slskr/tests/integration_tests.rs` in full. Its
      assertions are tautologies.
- [ ] **7.2** Add a real `tests/api_smoke.rs` covering: health, version,
      capabilities, search create+list, transfer list, the 4xx for missing
      auth, the 4xx for bad CSRF, the 429 for rate-limit. Use `reqwest`
      against an in-process daemon.
- [ ] **7.3** Add `tests/events_ws.rs` (already noted in 4.3).
- [ ] **7.4** Wire all of the above into `.github/workflows/` if CI exists; if
      not, at least into `scripts/run-ci.sh`.

### Phase 8 — Honesty pass on docs

- [ ] **8.1** Rewrite `README.md` "Status" paragraph: protocol/client crates
      shipping; daemon API single-instance with a defined endpoint list; web
      UI partially wired; SDKs functional for the listed endpoints.
- [ ] **8.2** Rewrite `PLAN.md` to drop everything beyond "single-node daemon
      with real persistence and real WS events". Mark previously-claimed
      phases as "not delivered, descoped" rather than complete.
- [ ] **8.3** Confirm `docs/app-surface.md`, `docs/install.md`,
      `docs/legacy-port-harvest.md` still match reality. Patch where they
      don't.
- [ ] **8.4** Delete the `archive/` directory created in 0.5 (one-week
      cooldown elapsed).

---

## 6. Anti-patterns to flag if you see them in future PRs

These are the smells that produced the current state. Treat any of them as a
blocker on review:

- New file named `*_FINAL.md`, `*_COMPLETION_*.md`, `PHASE_N_*.md`,
  `*_SUMMARY.md`.
- New module added to `main.rs` `mod` block with no call site in the same PR.
- `#![allow(dead_code, unused_imports)]` reintroduced anywhere.
- A handler that returns a hardcoded JSON shape with no read of `state`.
- Comments saying "in production, this would …" or "simplified for now".
- `let _ = …;` on a write/persist call.
- A heavy dep (Redis, Postgres driver, gRPC, GraphQL lib, framework) added
  without a code path that exercises it in the same PR.
- Tests whose assertions check a literal that the test itself constructed.
- A commit titled with a phase number.

---

## 7. Appendix A — Module disposition (full table)

Source: `crates/slskr/src/`, line counts as of 2026-05-04.

| File                         | Lines | `module::` refs in `main.rs` | Decision  |
| ---------------------------- | ----- | ----------------------------:| --------- |
| `main.rs`                    | 14301 | n/a                          | TRIM (Phase 3 + 1.4 dead-code pass) |
| `persistence.rs`             |   933 | 6 (all `let _ =`)            | KEEP, wire (Phase 6) |
| `utils.rs`                   |   798 | many                         | KEEP, audit during Phase 1.4 |
| `graphql.rs`                 |   789 | 2                            | DELETE |
| `config.rs`                  |   557 | many                         | KEEP |
| `webhooks.rs`                |   525 | many                         | KEEP |
| `axum_router.rs`             |   495 | 0                            | DELETE |
| `rate_limit.rs`              |   489 | wired                        | KEEP |
| `metrics.rs`                 |   460 | 0                            | DELETE |
| `middleware.rs`              |   451 | 0                            | DELETE |
| `observability.rs`           |   427 | 0                            | DELETE |
| `websocket.rs`               |   421 | 0                            | DELETE (replaced) |
| `api_keys.rs`                |   415 | 0                            | DELETE |
| `batch.rs`                   |   411 | 2                            | KEEP |
| `filters.rs`                 |   409 | 0                            | DELETE |
| `versioning.rs`              |   386 | 0                            | DELETE |
| `response_cache.rs`          |   376 | 0                            | DELETE |
| `signalr_hub.rs`             |   373 | 0                            | DELETE |
| `enrichment.rs`              |   372 | 0                            | DELETE |
| `openapi.rs`                 |   363 | 2                            | AUDIT, likely fold |
| `rate_limiter.rs`            |   350 | 0                            | DELETE (dup of `rate_limit.rs`) |
| `storage.rs`                 |   335 | wired                        | KEEP |
| `sse.rs`                     |   309 | 0                            | DELETE |
| `tracing.rs`                 |   307 | 5                            | KEEP |
| `benchmarks.rs`              |   257 | 0                            | DELETE |
| `logging.rs`                 |   264 | wired                        | KEEP |
| `validation.rs`              |   250 | 0                            | DELETE |
| `security.rs`                |   244 | 0                            | DELETE |
| `websocket_handler.rs`       |   231 | 0                            | DELETE |
| `caching.rs`                 |   142 | 0                            | DELETE |
| `compression.rs`             |   133 | 0                            | DELETE |
| `routing.rs`                 |   130 | wired                        | KEEP |
| `pagination.rs`              |   115 | 0                            | DELETE |
| `api_integration.rs`         |   139 | 0                            | DELETE |
| `docs.rs`                    |   138 | 0                            | DELETE |
| `http_server.rs` (untracked) |   270 | 2                            | KEEP, fix, track |

Sum of DELETE column: **~7,500 LOC** going away in Phase 1.

---

## 8. Appendix B — Dependency disposition

`crates/slskr/Cargo.toml` after Phase 2:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
slskr-client = { path = "../slskr-client" }
slskr-cli    = { path = "../slskr-cli" }
tokio = { version = "1", features = ["io-util", "macros", "net", "rt-multi-thread", "sync", "time", "signal"] }
toml = "0.8"
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls", "chrono", "uuid"] }   # drop "postgres"
chrono = { version = "0.4", features = ["serde"] }
uuid   = { version = "1",   features = ["v4", "serde"] }
reqwest = { version = "0.11", features = ["json"] }                                            # webhooks
hmac = "0.12"; sha2 = "0.10"; hex = "0.4"                                                       # webhook signing
tracing = "0.1"; tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tokio-tungstenite = "0.21"                                                                      # /api/events/ws (Phase 4)
```

Removed: `axum`, `tower`, `tower-http`, `redis`, `moka`, `dashmap`, `tonic`,
`prost`, `sea-orm`, `deadpool-postgres`, `tokio-util`, `bytes`, `flate2`,
`http`, sqlx postgres feature.

---

## 9. Appendix C — Documentation disposition

| Path                                  | Decision |
| ------------------------------------- | -------- |
| `README.md`                           | KEEP, rewrite Status (Phase 8.1) |
| `PLAN.md`                             | KEEP, rewrite (Phase 8.2)        |
| `COMPLIANCE.md`                       | KEEP                              |
| `LICENSE`, `NOTICE`                   | KEEP                              |
| `REMEDIATION.md` (this file)          | KEEP                              |
| `docs/`                               | KEEP                              |
| All `*FINAL*.md`                      | ARCHIVE (Phase 0.5), DELETE (8.4) |
| All `*COMPLETION*.md`                 | ARCHIVE then DELETE              |
| All `*SUMMARY*.md`                    | ARCHIVE then DELETE              |
| All `PHASE*.md`, `PHASE_*.md`         | ARCHIVE then DELETE              |
| All `*SESSION*.md`                    | ARCHIVE then DELETE              |
| `IMPLEMENTATION_*.md`, `_GUIDE.md`, `_REPORT.md`, `_AUDIT.md`, etc. | ARCHIVE then DELETE |
| `RELEASE_v1.0.1.md`, `RELEASE_CANDIDATE.md` | ARCHIVE then DELETE        |
| `START_HERE.md`, `QUICK_START*.md`    | ARCHIVE; if a real quick-start is needed, regenerate from current code |
| `API_*.md`, `HTTP_API_*.md`           | ARCHIVE; the openapi.json in `docs/` is the source of truth |
| `WEBUI_PORT_TEST_REPORT.md`, `STORAGE_LAYER_ANALYSIS.md`, `CODE_PATTERNS_ANALYSIS.md`, `REACT_*.md`, `REFACTORING.md` | ARCHIVE then DELETE |
| `EXACT_CODE_TO_ADD.md`, `ENDPOINT_IMPLEMENTATION_CHECKLIST.md`, `FUNCTION_SIGNATURES_REFERENCE.md`, `POST_*.md`, `ANALYSIS_README.txt` | ARCHIVE then DELETE |
| `MONITORING.md`, `DEPLOYMENT*.md`, `RATE_LIMITING.md`, `WEBHOOK_API.md`, `API_VERSIONING.md`, `API_INDEX.md`, `PERFORMANCE_OPTIMIZATION_TARGETS.md` | ARCHIVE; merge any still-true content into `docs/` first if needed |

---

## 10. Decision log

Append to this section whenever a decision in §4 needs to change. Do not edit
existing rows; add a new dated entry.

- **2026-05-04** — Initial decisions D1–D12 set above. Drafted by Claude during
  state-of-the-project review at user's request. Author: keith@snape.tech.
