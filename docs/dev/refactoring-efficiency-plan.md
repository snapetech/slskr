# Refactoring and efficiency plan

Status: internal implementation plan, started 2026-09-02.

This plan covers structural refactoring, runtime efficiency, build/test
efficiency, and Web UI efficiency after the universal replacement gate was
closed. It does not reopen or weaken the parity contract.

## Fixed invariants

Every swath must preserve the pinned slskd/slskdN compatibility profiles and
the current pinned upstream boundary. In particular, refactors must not change
HTTP status codes, JSON field omission/null rules, profile-specific defaults,
SignalR method names, event ordering, reconnect/error behavior, persistence
recovery, or supported/unsupported transport classification.

Work stays on `main`; no feature branches are required. Each coherent swath
gets its own commit so it can be reviewed, bisected, or reverted without
mixing unrelated behavior changes.

## Current inventory

The inventory is intentionally descriptive, not a performance claim:

- The Rust workspace contains about 370,000 source lines.
- `crates/slskr/src/lib.rs` is about 235,000 lines. It contains production
  state, projections, routing, and an opt-in historical differential suite.
- `crates/slskr/src/route_dispatch.rs` is about 19,600 lines. The production
  bounded dispatcher and the historical dispatcher still coexist.
- `web/src` is about 95,500 lines. The largest current areas include the
  System/MediaCore surface, `App.jsx`, and the player/search surfaces.
- The default daemon test binary contains 440 Rust unit tests; the web suite
  contains 511 tests.
- The existing Cargo configuration already serializes workspace jobs and
  limits debug metadata. Compiler settings must be changed only after timing
  evidence.
- The former `benchmarks/benchmark.rs` was not wired into Cargo and simulated
  requests with random success/failure. It was not valid runtime evidence.

## Swath 1: trustworthy measurement

1. Replace the synthetic benchmark with the standard-library HTTP benchmark
   in `scripts/benchmark-http.py`.
2. Exercise a real running daemon using persistent worker connections,
   deterministic endpoint workloads, warmups, bounded samples, status/error
   accounting, and JSON output suitable for before/after comparison.
3. Capture separate baselines for legacy/native profiles and persistence
   modes. Include health/version/capabilities, stats, transfers, searches,
   browse, and SignalR handshake workloads where the environment supports
   them.
4. Capture Cargo compile timings and record process-level CPU/RSS externally;
   do not infer those values from source inspection.
5. Rewrite the stale performance report so unmeasured numbers are not
   presented as facts.

Exit criteria: repeatable baseline artifacts, a documented command, and
regression thresholds chosen from observed variance rather than guesswork.

## Swath 2: source and test boundaries

1. Move the opt-in historical controller differential module and shared test
   fixtures out of the production source file while retaining the same feature
   names and runners.
2. Keep the focused default tests small and private-state access explicit
   through a test-support boundary.
3. Split `lib.rs` by ownership into modules for state, projections, lifecycle,
   compatibility data, and route domains. File moves are structural only.
4. Measure incremental compile time after each boundary move. Create a new
   crate only if timing data shows a real compile benefit and the dependency
   direction remains one-way.

Exit criteria: focused and full differential commands remain available, all
existing parity counts remain unchanged, and default edit/test feedback is no
slower.

## Swath 3: controller and event architecture

1. Establish typed route context/response helpers and migrate route domains
   one group at a time.
2. Make the bounded dispatcher the single production implementation; retain
   the historical path only as an explicit diagnostic oracle until its
   differential coverage is no longer needed.
3. Centralize domain event creation and project each event once into durable
   history, HTTP/WebSocket feeds, and SignalR hubs.
4. Keep profile differences in explicit contract adapters/tables instead of
   scattered conditionals.

Exit criteria: route and event differential proofs are unchanged or stronger,
and live hub snapshots/actions still match target order and payload shape.

## Swath 4: state, persistence, and runtime efficiency

1. Identify actual hot queries with SQLite query plans and timings before
   adding indexes or caches.
2. Narrow lock scopes and snapshot data before serialization or network I/O;
   no asynchronous I/O runs while an in-memory lock is held.
3. Batch compatible persistence writes transactionally, preserve restart
   semantics, and add queue/backpressure/cancellation metrics.
4. Target measured costs in search-result updates, transfer-event growth,
   share scans, JSON serialization, hub fan-out, and peer connection reuse.
5. Avoid semantic event coalescing where target-visible ordering is part of
   the contract.

Exit criteria: benchmark improvements are reproducible, memory remains
bounded, and persistence/lifecycle/live-interoperability gates pass.

## Swath 5: Web UI efficiency

1. Measure duplicate requests, polling intervals, SignalR subscriptions,
   component render counts, and route transition timing.
2. Consolidate duplicate polling and request caching only where freshness and
   target behavior remain equivalent.
3. Add cancellation on route changes, split oversized component ownership, and
   memoize only measured render hot spots.
4. Track bundle budgets and inspect the large System/MediaCore and vendor
   chunks before changing dependencies or CSS systems.

Exit criteria: live success/loading/empty/validation/error/reconnect audits
remain clean, while network, render, and bundle measurements improve or stay
neutral.

## Verification cadence

The cadence is deliberately batched:

- Per edit: changed-file Rust format check, `cargo check -p slskr`, focused
  Rust tests, and the affected web test subset.
- Per swath: full Rust tests, full web tests/build, both profile compile
  checks, relevant differential gates, and a before/after benchmark.
- Release boundary: strict universal manifest, live interop and UI evidence,
  packaging checks, and release-note validation.

An internal-only structural change is recorded in the implementation commit.
Any user-visible, operational, security, or documentation behavior change
also requires a validated fragment under `release-notes/`.

## Recommended execution order

The order is measurement, test/source boundaries, controller/event
architecture, persistence/runtime hot paths, then Web UI. Do not begin with
speculative response caches, parallel stats, a routing hash map, GraphQL, or
dependency replacement; none is justified until the baseline identifies it as
a bottleneck.
