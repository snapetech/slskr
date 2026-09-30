# Refactor Handoff

Handoff time: 2026-09-15 17:47 local

This handoff stops the current execution session without discarding the dirty
worktree. The authoritative backlog remains
`docs/dev/refactor-audit-20260915.md`; the execution plan is
`docs/dev/refactoring-efficiency-plan.md`.

## Completed And Locally Verified

- Shared listener demux now uses bounded structural validation instead of
  first-byte guessing; plain/obfuscated collision tests pass.
- Init fields are bounded before known-frame allocation; client and protocol
  suites pass.
- Route lock-scope batches cover route groups 2-6 with snapshot/drop/
  compare-and-rollback semantics; nested multi-store acquisitions remain open.
- Distributed-child and ignored-search persistence are transactional and
  bounded; expired-search persistence now uses the bulk transition path.
- Transfer durability snapshots occur outside queue locks; restart/event-order
  coverage passes.
- Managed indirect requests have bounded timeouts and poisoned-session handling.
- Go SDK listener removal and typed API errors are implemented and Go
  test/race/vet checks pass.
- CI/release locking, GitLab release ownership, Chocolatey tag/checksum flow,
  fixture validation, docs freshness, and SDK example gates were updated.
- Web/dashboard changes include navigation and dashboard polling coalescing,
  telemetry cache, hydration gate, degraded health state, lazy routes and
  visualizer, browse indexing, render fallback, keyboard controls, and timer
  cleanup.
- Web bundle budgets are wired into CI/release gates.
- Release gate output now identifies the optional live API smoke as requiring a
  retained scheduled artifact.
- `PLAN.md` now points historical phase readers to the active refactor ledger.
- RF-055/RF-056 Web history cursor/delta and cancellation work is now locally
  verified; the implementation and evidence are recorded in the 2026-09-21
  continuation notes in the plan and audit.

## Validation Baseline

The following passed before this handoff:

- `cargo test --locked --workspace` passed.
- Latest `cargo test --locked -p slskr --lib` passed: 553 tests.
- `cargo check --locked --workspace --all-targets` passed.
- Changed-file Rust formatting passed. Do not run concurrent full-workspace
  formatting.
- Web passed 839 tests, lint, build, bundle budget, and build-output checks.
- Dashboard passed 29 tests, lint, type-check, and build.
- Go/Python/TypeScript SDK gates and example contracts passed.
- Docs, fixture, workflow, remediation, module, and dependency checks passed.
- Serialized Web E2E passed 14 available tests; 9 optional media tests skipped.
- All three `20260915-*.md` release-note fragments parse successfully.

The final source change immediately before handoff was the bulk expired-search
transition in `crates/slskr/src/lib.rs`; rerun the full workspace matrix after
that change if the worktree is resumed.

## Remaining Work

### Active Worker Scopes

There are no remaining unreviewed worker scopes from this handoff. The Web
request cancellation and cursor/delta slice was completed and reviewed in the
current worktree with focused tests, the full Web suite, lint, build, bundle
budget, and build-output checks.

The aggregate bundle worker has now completed and reported focused validation:
Web initial JS is 1083.77 KiB against a 1150 KiB budget and all-JS gzip is
582.12 KiB against a 600 KiB budget; dashboard initial JS is 241.60 KiB against
260 KiB and all-JS gzip is 87.69 KiB against 100 KiB. Review its diff and rerun
the aggregate tests after resuming; it is no longer an active worker scope.

The cross-SDK subscription worker has also completed. It added a dependency-free
local WebSocket fixture plus Go, Python, and TypeScript tests covering dotted
event types, `data.topics`, unsubscribe filtering, and reconnect
resubscription. Those focused suites pass. The fixture does not start the Rust
daemon or prove authentication, persistence, or external interoperability;
those remain external RF-042 evidence.

The former Web cancellation worker scope is closed. ChatSession, RoomSession,
the unified pod-channel panel, and the legacy Pods route now use bounded cursor
deltas, stable message identities, per-resource in-flight guards, and abortable
history reads. The optional downloaded-media fixture gap and live browser proof
remain separate.

### Correctness And Lifecycle

- RF-001: theoretical dual-valid plain/obfuscated frame ambiguity remains;
  raw tagged traffic is dedicated-listener only.
- RF-002: nested multi-store lock acquisition remains in collections/grants,
  pods/channels, and Lidarr library/runtime/relay paths.
- RF-004/RF-003: restart/rollback and statement-count proof remains for the
newly batched persistence paths.
- RF-006/RF-007: detached scheduler/listener task registry and cancellable
  share-index scans still need a JoinSet/cancellation design.
- RF-042: server subscription support exists, but all-SDK live handshake and
  reconnect artifacts are not yet retained.
- RF-043/RF-047: clean-runner lock proof, delayed-filesystem proof, and partial
  event-batch recovery remain.

### Web, Build, And Packaging

- RF-055/RF-056: locally verified for chat, room, and pod history reads; live
  browser evidence and unrelated dashboard/Search cancellation remain open.
- RF-058: large MediaCore/Integrations ownership boundaries still need
  profiler-led extraction.
- RF-063/RF-064: aggregate request/accessibility budgets and dead/config
  inventory remain beyond the low-risk gates already added.
- RF-066: optional downloaded media fixtures remain unavailable locally.
- RF-008/RF-010/RF-011/RF-028: clean GitLab, AUR/makepkg, Windows, and
  Chocolatey runners are external validation.
- RF-073: scheduled live API compatibility artifact is still external.

## Resume Procedure

1. Inspect `git status --short`, `git diff --stat`, and all three active-worker
   diffs. Preserve unrelated dirty changes.
2. Run `scripts/check-rust-format.sh`, never concurrent `cargo fmt --all`.
3. Run `cargo test --locked --workspace` after the latest source change.
4. Run Web/dashboard/SDK/docs/fixture gates serially or with low concurrency.
5. Re-run serialized E2E only after the Rust binary is built and no other Cargo
   job is active.
6. Update statuses in the audit and plan with command/artifact evidence.

No commit or push was made during this work.
