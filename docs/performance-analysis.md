# slskR performance analysis

This is the maintained performance note. It records what has been measured and
what still needs measurement; source size, test counts, and intuition are not
runtime benchmarks.

## Current inventory

As of the 2026-09-02 completed refactoring swath:

- Rust workspace source: approximately 370,000 lines.
- `crates/slskr/src/lib.rs`: approximately 90,000 lines; the opt-in
  historical differential suite is maintained in a separate source file.
- `crates/slskr/src/route_dispatch.rs`: approximately 1,548 lines, plus
  eight included route-group files totaling approximately 18,068 lines.
- `web/src`: approximately 95,500 lines.
- Default daemon unit tests: 442.
- Web tests: 514.

These are structural measurements only. No latency or throughput number is
claimed here until it comes from the real benchmark artifact.

## Real benchmark

Run a read-only benchmark against a running daemon:

```bash
python3 scripts/benchmark-http.py \
  --base-url http://127.0.0.1:5030 \
  --profile native \
  --persistence disabled \
  --warmup 5 \
  --duration 30 \
  --concurrency 8 \
  --output target/perf/native-disabled.json
```

Use `--profile legacy` for the slskd-compatible projection, repeat with
persistence enabled, and add explicit safe read endpoints for the workload
being compared. The tool keeps one HTTP connection per worker, drains real
responses, records actual statuses and errors, and bounds retained latency
samples. A non-zero exit status means at least one measured request failed.

Profile concrete SQLite reads before changing indexes or caches:

```bash
python3 scripts/profile-sqlite.py \
  --database /path/to/slskr.db \
  --query 'messages=SELECT id, username, created_at FROM messages ORDER BY created_at DESC LIMIT 100' \
  --query 'transfers=SELECT id, status, started_at FROM transfers ORDER BY started_at DESC LIMIT 100' \
  --warmup 2 \
  --iterations 10 \
  --output target/perf/sqlite-baseline.json
```

This read-only profiler records each statement's `EXPLAIN QUERY PLAN`, result
row-count range, and measured minimum/median/p95/maximum latency. It reports
the selected database and workload; it does not claim that a small local
database represents production behavior.

Compare two artifacts only when the workload metadata matches:

```bash
python3 scripts/compare-benchmark.py \
  target/perf/native-disabled-before.json \
  target/perf/native-disabled-after.json \
  --max-latency-regression-percent 10 \
  --max-throughput-regression-percent 10 \
  --output target/perf/native-disabled-comparison.json
```

The comparison checks the aggregate and each endpoint case. Missing metrics
and workload drift are invalid results, not successful comparisons.

For an isolated local daemon comparison, disable the native controller
limiter and raise the API quota only in the benchmark process environment:

```bash
export SLSKD_WEB_RATE_LIMITING=false
export SLSKR_API_RATE_LIMIT_ANONYMOUS=4294967295
export SLSKR_API_RATE_LIMIT_AUTHENTICATED=4294967295
```

Do not carry those measurement-only values into a deployment configuration.

## Refactoring swath evidence

The native release comparison used commit `df501d30` as the baseline and the
completed worktree as the candidate. Both daemons used persistence-enabled
fresh SQLite state, four persistent HTTP connections, a two-second warmup, a
ten-second run, and the same two changed read endpoints: `/api/stats` and
`/api/admin/database/stats`. Every response was HTTP 200 and both artifacts
had zero failures. The aggregate changed from 23,836 requests at 2,383.215
RPS with p95 latency 2.131 ms to 136,611 requests at 13,659.795 RPS with p95
latency 0.441 ms; the 10% comparison gate passed. This is local evidence for
the changed persistence paths, not a production-scale claim.

The same release comparison under the legacy profile also passed with zero
failures: 11,733 requests at 2,346.116 RPS and p95 latency 1.075 ms before,
versus 70,962 requests at 14,190.126 RPS and p95 latency 0.214 ms after. The
legacy workload used the two stats endpoints and the same isolated persistence
and limiter settings.

A separate seven-endpoint diagnostic run also returned only HTTP 200 responses
with zero failures. Its comparison was not accepted as a general latency gate
because untouched scalar routes showed small absolute latency increases while
the persistence-heavy routes improved. The focused changed-path gate is the
reported acceptance result; no universal latency improvement is claimed.

The current SQLite profiler recorded the supporting plans on a fresh migrated
database: `idx_messages_username_created` serves username-plus-created-at
message reads and `idx_library_items_created` serves recent library reads,
with no temporary sort B-tree in either plan. Other tested search, transfer,
share, and recent-list paths retained their existing indexes.

The Web build emitted a 331,410-byte System chunk and a separate 150,875-byte
MediaCore chunk; the prior combined System chunk was 481,703 bytes. The named
bundle-budget check passed, including the 632,535-byte vendor chunk.

The Web UI's shared polling controller is used for periodic refreshes that
remain active while a screen is mounted. It waits for each request to finish,
pauses timers while the document is hidden, and owns cleanup on route or
component teardown. Event-driven refreshes remain separate so they can still
react immediately to hub notifications.

Mutation and SignalR performance scenarios must remain explicit lifecycle
scenarios. They are not hidden in a generic load test because event ordering,
side effects, and reconnect behavior are compatibility contracts.

## Compile and process measurements

Capture Cargo's build timing report before changing module or dependency
boundaries:

```bash
cargo build -p slskr --timings
cargo test -p slskr --lib
npm --prefix web test
npm --prefix web run build
npm --prefix web run test:bundle-budget
```

Record wall time externally with the shell or CI runner, and record daemon
CPU/RSS with the platform's process tools during the HTTP benchmark. The
workspace's one-job Cargo setting and release profile are deliberate responses
to the daemon's size; change them only when timing and peak-memory evidence
show a safe improvement.

## Priority measurement points

The first candidates are:

1. request routing and compatibility projection/serialization;
2. SQLite reads, writes, indexes, and pagination;
3. search-result and transfer-event update paths;
4. share/file scans and browse responses;
5. SignalR/WebSocket fan-out and queue backpressure;
6. peer connection setup and reuse;
7. Web UI polling, repeated requests, render counts, and bundle chunks.

No cache, parallel aggregation, routing-table replacement, or dependency
replacement is justified until one of these measurements identifies it as a
costly path. Any optimization must be compared against the same profile,
dataset, persistence mode, warmup, concurrency, and daemon build.

## Acceptance rules for an optimization

Each change must provide:

- a before/after artifact from the real benchmark or compile timing;
- a targeted regression test for the changed path;
- unchanged target-visible status, payload, ordering, persistence, and error
  behavior;
- bounded memory and queue behavior under the same workload;
- the appropriate per-swath and release gates.

The detailed implementation sequence is in
[`docs/dev/refactoring-efficiency-plan.md`](dev/refactoring-efficiency-plan.md).
