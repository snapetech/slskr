# Whole-Project Refactor Audit

Status: active refactor evidence; RF-024 and RF-002 remain in progress; latest addenda cover RF-002 wishlist scheduler, Lidarr rejection and wanted-sync reviews, plus RF-024 Batch 187 (2026-09-25).

Audit baseline date: 2026-09-15; evidence addenda through 2026-09-25
Baseline HEAD: `0dfab48cd16e6e7910759fa7d60e5d21b7b28be1` (local evidence; the
working tree contains the implementation batch described in the status column).

This is the evidence snapshot for the living plan in
`docs/dev/refactoring-efficiency-plan.md`. It evaluates the repository beyond
the completed 2026-09-02 refactor batch. Findings are deliberately separated
from speculative ideas: an item is actionable only when it has a concrete
location, a failure mode or measured cost, and a validation path.

## Executive Summary

The repository already had a refactor plan, but it described one completed
implementation swath rather than the whole project. The current audit found:

- One confirmed shared-listener ambiguity: the same 274 wire bytes decode as
  both a valid 9-byte plain `PierceFirewall` frame and a valid type-1
  `PeerInit` frame. The current shortest-valid-candidate policy can therefore
  misclassify obfuscated traffic; resolving intent requires a wire discriminator
  or separate-port policy.
- Multiple asynchronous lock guards held across persistence, authentication,
  network, and logging awaits in current route/lifecycle code.
- A remaining unbatched search persistence helper that omits persisted search
  metadata despite the previous plan claiming all bulk search writes were
  batched.
- CI and release topology drift: the GitLab pipeline calls missing scripts and
  can create a second GitHub release beside the tag-triggered GitHub workflow.
- SDK/example/documentation contracts that are marked verified by existing
  ledgers but are not exercised by the current gates.
- Large structural boundaries that remain worthwhile after correctness and
  gate drift are addressed: the daemon controller, Web UI ownership, and
  cross-language SDK packaging/contracts.

The repository was clean at audit start. Baseline checks passed unless marked
otherwise in the table below. The Web test suite required `npm ci` because
`web/node_modules` was absent; after installing the locked dependencies, the
suite passed with 141 files and 836 tests.

## Scope And Method

The review covered:

- Rust daemon, client, protocol, persistence, lifecycle, route dispatch, and
  Cargo features.
- React/Vite Web UI, dashboard, client TypeScript package, and their build,
  test, lint, and dependency configuration.
- Go and Python SDKs, examples, package metadata, and SDK gates.
- GitHub/GitLab CI, release publication, packaging, security gates, and release
  notes.
- Tests, benchmarks, audit ledgers, documentation freshness checks, and the
  existing refactor/performance plans.

The review used source inspection, repository search, existing audit ledgers,
and the following live checks:

| Check | Result |
| --- | --- |
| `scripts/check-rust-format.sh` | Pass |
| `cargo check -p slskr` | Pass |
| `cargo test -p slskr-client --test listener` | Pass, 23 tests |
| `cargo test -p slskr-client --test manager` | Pass, 16 tests |
| `npm --prefix web test` | Pass, 141 files and 836 tests |
| `npm --prefix web run lint` | Pass |
| Dashboard type-check, lint, test, build | Pass, 26 tests |
| `scripts/check-client-sdk-gates.sh` | Pass, Go/Python 48/TypeScript 50 tests |
| `scripts/check-workflow-release-policy.sh` | Pass |
| `scripts/check-rust-module-hygiene.sh` | Pass |
| `scripts/check-npm-dependency-hygiene.sh` | Pass |
| `scripts/check-docs-freshness.sh` | Pass, scope limitation noted below |
| `scripts/check-council-active-backlog.sh` | Pass |
| `scripts/check-web-audit.sh` | Pass, zero reported vulnerabilities |

## Findings

Priority meanings: P0 blocks safe release or can corrupt/strand live work; P1
is a correctness, lifecycle, or gate failure with a concrete user or operator
impact; P2 is a measured maintainability, efficiency, or contract gap; P3 is a
structural improvement to execute only after higher-priority work is stable.

### P0/P1 Correctness And Lifecycle

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-001 | `crates/slskr-client/src/listener.rs` shared demux previously classified the first byte as a raw connection kind before parsing a frame. Bounded plain/obfuscated candidate validation and TCP coverage for `P`, `F`, and `D` collisions are in place. `crates/slskr-client/tests/listener.rs::shared_wire_bytes_can_form_valid_plain_and_obfuscated_init_frames` proves one wire buffer decodes both as a 9-byte plain `PierceFirewall` frame and as a 274-byte type-1 `PeerInit` frame. | A valid obfuscated init can be returned as a shorter valid plain frame, leaving the rest of the stream desynchronized. | The 27 listener tests pass, including the dual-valid fixture. Raw `P`/`F`/`D` traffic remains intentionally dedicated-port only. No local parser can infer intent from a dual-valid stream; resolution requires a coordinated wire discriminator or separate-port policy. | In progress, confirmed protocol ambiguity |
| RF-002 | `crates/slskr/src/route_dispatch_group_3.rs`, `group_4.rs`, and `group_5.rs` held write guards across `persist_*`, auth, or other async calls; `group_6.rs` had the same class. Route groups 2-6 now snapshot/drop guards on migrated paths; group 7 wishlist writes also release the store guard during persistence. Lidarr manual import shares a runtime-persistence gate, writes its library item and only its own compatibility counter in one SQLite transaction, and releases state guards during persistence. Pod update/delete persist the parent before channel cleanup, restore parent state on cleanup failure, prune orphans on startup, and serialize appends before rechecking pod and channel existence. Room/pod membership workflows use a consistent room-then-workflow lock order, and acceptance rechecks authorization after waits. Collection, grant, and share-token routes serialize persistence and reject stale parent writes. Share-group/member mutations now use their own persistence turn in both dispatchers, and the legacy path releases its write guard during SQLite I/O. Incoming wishlist results and ignored-rule changes share a turn through their paired transaction; all search persistence helpers re-read the current SearchStore snapshot under one persistence turn. Persisted wishlist item routes, imports, completion counters, Lidarr sync, and auto-download take one turn through SQLite persistence. Library create/delete, health-issue patch/fix, MusicBrainz target creation, and the manual-import transaction take a library turn before mutation; manual import orders library before runtime. Contact creation through direct, discovery, and invite routes, updates, and deletes now share a contact persistence turn in both dispatchers; store guards are released before SQLite. User-note create, versioned create, update, and delete routes now share a separate persistence turn in both dispatchers. Liked/hated interest routes and the mesh-rendezvous compatibility mutation use one interest persistence turn in both dispatchers; versioned routes drop it before outbound session commands, and compatibility create/delete restore memory on SQLite failure. Persisted now-playing updates, listening-party projections, and clears use a dedicated turn in both dispatchers. Webhook registration, active-state changes, and deletion across direct/admin routes share a webhook persistence turn in both dispatchers. Security username/IP ban and unban routes across both dispatchers and overlay blocklist compatibility share a security-ban persistence turn, with SQLite writes outside the store guard and failure rollback on compatibility routes. Message creation, inbound session messages, acknowledgements, and conversation deletion share a message persistence turn across both dispatchers; compatibility acknowledgements roll back on database failure and check the URL username before mutation. Room subscription joins and leaves across both dispatchers and compatibility handlers share one persistence turn acquired before changing RoomStore; legacy paths release the store guard before SQLite and compare before rollback. Spotify OAuth state issue and callback consumption use one persistence turn; callback deletion releases the store guard during SQLite work and removes memory state after commit. Browse request, folder, fail, cancel, and response-ingest routes plus direct/indirect peer completion and failure projections now share a browse turn across both dispatchers and background callbacks; browse-store guards are released before SQLite, and failure reporting/events run after the turn is dropped. User watch/unwatch routes in both dispatchers and inbound watched-user, status, and statistics replies share a user-projection turn; store guards are released before SQLite, rollback remains inside the turn, and session/error work follows its release. API event injection in both dispatchers and background `record_event` writes share an event-history turn; EventStore guards are released before SQLite, and scripts, reporting, and event-feed publication follow turn release. Transfer SQLite full-row and progress writes now reject stale `updated_at_ms` revisions; deletion and staged rollback persist tombstones, restart reserves ids past tombstones, and event history sorts by per-transfer revision; split-router request-name, cancellation, retry, progress, and completion updates now advance revisions monotonically to match the retained dispatcher. HashDb HTTP creation and merge, mesh publish and sync, backfill, and transfer-metadata hash writers now take the shared persistence turn before mutation and retain it through SQLite snapshot persistence while releasing the store guard during I/O. The history-backfill route holds that turn from persisted cursor read through candidate selection, in-memory progress update, and cursor write. Manual database cleanup now deletes terminal transfer rows through the revision-aware tombstone helper and rolls the live queue back if SQLite rejects deletion. Age-based message cleanup calculates one cutoff and holds the message persistence turn through live `MessageStore` pruning and SQLite deletion, restoring memory when persistence fails. Webhook queued-log insertion, dispatch snapshot selection, and delivery-stat memory/SQLite updates now share the webhook persistence turn with registration, activation, and deletion; manager guards are released before SQLite, changed or deleted definitions reject stale delivery stats, and failed stats writes conditionally roll back memory. Spotify authorization/refresh and disconnect previously persisted the encrypted file and published live connection state without a shared order or stale-work check.  Legacy batch-download enqueue now resolves destination paths without retaining the transfer write guard, then rechecks active duplicates under the write lock before staging.| Database stalls previously blocked unrelated readers, unrelated runtime fields could be overwritten by stale snapshots, paired rollback could retain a failed item after an independent concurrent update, delayed snapshots could recreate deleted parent state, an interrupted pod/channel write could retain orphan messages, a stale search snapshot could restore an ignored result, and delayed library, contact, user-note, or interest writes could resurrect deleted records, a delayed now-playing update could restore a cleared track, and a delayed webhook active-state write could restore a deleted webhook, and overlapping security ban/unban writes could persist in an order different from memory, and delayed message creates or acknowledgements could cross conversation deletion or leave rollback gaps, and concurrent room join/leave requests could persist in an order different from RoomStore, and OAuth callbacks could hold the state-store guard through SQLite deletion while issue/delete writes raced, and competing browse route/callback snapshots could persist in a different order from BrowseStore, while user watch/unwatch and inbound projection snapshots could persist in an order different from UserStore, and API event injection held the EventStore write guard through SQLite and blocked history readers; delayed transfer snapshots could overwrite newer rows or restore deleted transfers, and wall-clock transfer mutations could reuse or regress revisions, and delayed HashDb snapshots could erase later accepted merges, and overlapping history-backfill requests could advance from the same stale cursor and skip older searches, and manual cleanup could hide terminal transfers in memory while leaving durable rows to return after restart, and expired message rows could disappear from SQLite while live conversation routes still served them; runtime compatibility writes also held runtime and relay write guards across SQLite latency; legacy share-grant creation and manual Lidarr import now release their store guards before SQLite and conditionally roll back unchanged candidates; legacy collection deletion also releases collection/grant guards before SQLite and conditionally rolls back unchanged stores; webhook log insertion could leave orphan rows after deletion, while delivery stats could update a replaced webhook or remain changed only in memory after failed persistence. Authorization or refresh work already in flight could finish after disconnect and recreate credentials in memory and on disk.  Destination/path awaits previously held the transfer write lock and could block concurrent transfer reads and updates.| The current 621-test daemon library suite passes. Focused regressions cover transaction rollback, field-scoped runtime updates, independent rollback, runtime-persistence ordering, responsive pod and collection reads during waits, parent/grant/token ordering, stale collection snapshot rejection, share-group deletion before a queued member add while reads remain available, pod cleanup rollback, startup orphan repair, rejection of a queued append after channel removal, pending-read availability during membership acceptance, acceptor revocation after a lock wait, queued wishlist response followed by ignore creation, a queued completed-search snapshot that preserves suppressed results, a queued wishlist item update followed by deletion that leaves reads available and SQLite consistent, a queued library health repair followed by deletion, queued contact update/delete with a responsive read and no deleted row in memory or SQLite, and queued user-note update/delete with the same read and consistency guarantees, queued liked-add/delete/hated-add with available reads and SQLite matching memory, injected mesh-compatibility create/delete failures that preserve memory, queued now-playing update/clear with reads available and the cleared record absent from memory and SQLite, and queued webhook PATCH/delete with reads available and no deleted record in memory or SQLite, and queued message-create/conversation-delete with responsive reads and matching empty memory/SQLite state; compatibility ack database-failure rollback and wrong-username no-mutation regressions, and queued room join/leave with responsive reads and no final subscription in memory or SQLite; two queued OAuth consumers with responsive state reads and exactly one persisted state consumption, and a queued browse request/cancel with responsive reads and matching cancelled memory/SQLite state, plus queued user watch/unwatch with responsive reads and matching unwatched memory/SQLite state, plus queued API/background event writes with responsive reads and matching persisted event-ID order. Pod management, pod channel lifecycle, membership-workflow, contact CRUD/concurrency, contact persistence-failure, and contact discovery/read, user-note lifecycle/persistence-failure, and library/interest/now-playing/message persistence-failure frozen-controller differentials pass; now-playing persistence-failure and delete/diagnostic tests pass, as does the native webhook differential; webhook persistence/rehydration/dispatch-log, registration, exact-path, and PATCH contract tests also pass. the versioned interest wire-command test also passes, as do the share-group, share-grant, and collection differentials. The legacy/full-controller feature combination compiles; the earlier focused share-group test invocation overflows the historical monolithic dispatcher's Tokio test-worker stack. Message and conversation persistence-failure controller tests and differentials pass. Room subscription persistence/rehydration tests and room-controller restart/failure differentials pass. OAuth issue/delete failure-injection, persistence rehydration, and Spotify authorize/callback controller differential pass. Browse persistence rollback, cache rehydration, indirect browse, cancellation, and controller differentials pass. User-watch persistence rollback, projection rehydration, and controller user/share and open-case differentials pass. The 26-test event-focused feature set, event persistence rollback/rehydration, and oversized transfer-event rotation/FIFO tests pass. The transfer stale-snapshot/deletion and legacy-schema migration regressions, transfer cleanup and full lifecycle controller differentials pass. The split-router request-name revision regression confirms memory and SQLite retain the updated name and advanced revision when wall-clock time is behind the stored revision. A queued HashDb API-writer regression confirms memory and SQLite remain unchanged while writers wait, reads stay responsive, and final rows and the sequence cursor agree. A two-request history-backfill regression confirms queued batches consume successive cursors and SQLite ends at the latest cursor. Manual cleanup success and SQLite-failure regressions verify tombstone deletion and queue rollback; the existing database-cleanup controller contract passes under split dispatch, while the retained monolithic invocation overflows its historical test-worker stack. A queued message-cleanup regression proves memory and SQLite remain unchanged during the wait and lose the same expired record after release; a closed-database regression proves message-memory rollback. HashDb domain, history-backfill, mesh-sync protocol, and publish/lookup differentials pass under the split dispatcher. The retained monolithic HashDb domain differential overflows its historical Tokio worker stack; the combined feature build passes. The webhook queued-stats regression verifies no mutation while waiting, matching memory/SQLite updates after release, and memory rollback on database failure. Webhook config/reopen/log dispatch, audit-failure reporting, CRUD rollback, and native webhook contract tests pass. A file-backed SQLite lock regression proves runtime readers and an unrelated runtime update proceed during compatibility writes; failed-write rollback preserves that update. The split runtime-control persistence-failure differential passes; the legacy-dispatcher variant overflows its historical test-worker stack. ControllerFeatureState production mutations now use an ordered blocking-worker store that persists snapshot candidates outside the async state lock and publishes after file sync; paused-worker/cancellation and injected-write-failure regressions verify responsive reads, later-writer ordering, per-operation rollback, and matching memory/file state. A scoped full-controller/legacy Clippy pass emitted no `await_holding_lock` diagnostics. The latest default daemon library suite passes 620 tests, including the queued source-feed history writer/readback regression; source-feed preview mutations in both dispatchers now share one persistence turn, and the `full-controller-tests legacy-route-dispatch` library check passes. At an earlier checkpoint, the 613-test daemon suite passed. A broad `transfer_` filter with both controller features still overflows the historical Tokio worker stack in `focused_controller_tests::versioned_swarm_rejects_oversized_transfer_limits_before_discovery`; the combined feature compile and normal full-controller differential pass. Server status, connect, and disconnect responses now snapshot and release the session guard before runtime-credential reads in both dispatchers; a deterministic held-credential regression verifies session writes remain available. Other paired persistence routes outside the now-ordered wishlist, library, search, contact, user-note, interest, now-playing, webhook, security-ban, messaging, room-subscription, OAuth-state, browse, user-projection, event-history, transfer SQLite, HashDb snapshot, history-backfill cursor, manual terminal-transfer cleanup, age-based message cleanup, source-feed history ordering, server-session credential read ordering, and split-router transfer revision ordering still need review. The focused stale-commit regression rejects old work and confirms cleared memory plus absent encrypted file. The locked full-controller/legacy library check and scoped await_holding_lock Clippy pass. The `20260924-spotify-disconnect-order.md` fragment passes the working-tree release-note preview.  A held-destinations route regression proves the transfer write lock remains available while path resolution waits; focused and full daemon tests pass (622), as do the feature compile, scoped lock Clippy, and formatter. The scoped library/runtime persistence-lock scan finds both manual-import dispatchers acquire library then runtime, while other inspected call sites take one turn; broader paired-route review remains open. | In progress |
| RF-003 | Distributed mutations previously held `distributed_network` guards across SQLite work, and tree metadata plus child rows used separate transactions. Runtime updates now publish revisioned snapshots; one worker coalesces to the latest bounded snapshot after the state guard is released. `DatabaseManager` loads both tables in one read transaction and replaces both in one write transaction. Graceful shutdown now writes the latest snapshot after managed producers stop. | Database latency previously blocked distributed-state readers, and a failed child replacement could leave branch metadata and child depths from different states. | The current 622-test daemon library suite passes, including cross-table rollback, latest-snapshot persistence, file-backed reopen, startup hydration, and a shutdown-overlap regression proving a registered task is dropped before the final runtime revision reopens from SQLite. Retained live shutdown-overlap and crash/restart evidence remain. | In progress, local transaction/hydration/shutdown proof |
| RF-004 | `crates/slskr/src/persistence.rs:4114-4184` was the remaining unbatched ignored-result path; the current batch uses bounded search/identity/result batches, preserves `fallback_attempts`, and adds metadata plus failure-injection rollback regressions. The production `serve` path now calls the shared `load_wishlist_store` startup helper. | Search metadata and large ignored-result updates were previously at risk. | The current 622-test daemon library suite passes. A file-backed close/reopen regression loads through the production startup helper and verifies the rehydrated ignore rule through API reads and search filtering. | Verified locally |
| RF-005 | `crates/slskr-client/src/manager.rs:179-224` previously held the server mutex while awaiting an unbounded `send_server_message`; the first implementation batch now bounds the send and marks the session unusable after timeout/error. | A backpressured server previously blocked every indirect request and could strand the manager. | Full manager suite passes with the non-reading peer and subsequent-request regression. | Verified |
| RF-006 | `AppState` owns a `ManagedTaskRegistry` backed by a `JoinSet`; long-lived workers, schedulers, listener managers, bridge/relay services, overlay gateway, DHT, signal/version services, Unix/HTTPS accept loops, and their HTTPS/Unix HTTP handlers are registered for joined shutdown. Plain HTTP handlers use a joined request set. All three listener types share a 256-connection semaphore. | Accepted HTTP work is bounded and tied to listener lifecycle; clean-runner/live shutdown-overlap evidence is still absent. | Focused regression proves HTTPS/Unix handlers share capacity and are aborted/joined with the managed registry; existing shutdown-flush and share-scan overlap regressions pass. Collect clean-runner and retained live shutdown-overlap evidence. | In progress, stronger local proof |
| RF-007 | The share-index worker owns the cancellation token with the scan permit, checks it during filesystem traversal, returns `SHARE_SCAN_CANCELLED_ERROR`, and refuses to publish partial snapshots. Configuration reloads and runtime share-setting changes now share an index persistence turn; generation checks reject completed scans built from older settings before SQLite replacement or live publication. | Cancelled or stale rebuilds cannot replace a newer live or durable share index, and a settings reload preserves its pending-rescan state. | Focused shutdown and watched-reload regressions plus the current 621-test daemon suite pass; retained live shutdown-overlap evidence remains. | In progress, local cancellation and ordering proof |

### P1 CI, Release, And Packaging

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-008 | `.gitlab-ci.yml:21-26` previously invoked three nonexistent Rust guard scripts. The current batch uses checked-in process guards and direct Cargo commands. | The GitLab Rust job previously could not start on a clean runner. | Static workflow policy and shell/YAML checks pass; a clean GitLab runner remains external proof. | In progress |
| RF-009 | `.gitlab-ci.yml:162-174` previously created a GitHub release while `.github/workflows/release.yml` already owned tag publication. The current batch removes the duplicate publisher. | One tag previously could race two release creators. | Workflow policy passes; a tag simulation/clean GitLab runner remains external proof. | In progress |
| RF-010 | `release-publish.yml:128-168` previously validated AUR hashes and `.SRCINFO` but did not run a clean `makepkg` source/prepare smoke. The release gate and AUR publish job now invoke `check-aur-package-smoke.sh`. | AUR source/package breakage is now exercised before publication on supported runners. | Host `makepkg` smoke passes for both source and binary PKGBUILDs; clean publication runner remains external. | Verified locally, external runner open |
| RF-011 | `windows-smoke.yml` previously filtered dashboard/client-ts changes while only building Web assets; the current filters align the job with the Rust/Web checks it actually executes. | SDK/dashboard-only changes no longer imply coverage from a Rust/Web-only job. | Workflow policy passes; Windows runner proof remains external. | Verified locally, external runner open |
| RF-012 | `client-ts/package.json` now allowlists `dist`/README publication files and the SDK gate builds, diffs, and dry-runs `npm pack`. | Published TypeScript artifacts cannot silently drift or include tests/configuration. | TypeScript package check passes with 26 allowlisted files; registry publication remains external. | Verified locally |
| RF-013 | `check-python-client-quality.sh` now runs Black, Flake8, and mypy from a pinned temporary environment when needed; the aggregate SDK gate installs its venv before quality checks. | Python quality regressions no longer pass because host tooling is absent or the gate invokes checks in the wrong order. | Quality, 49 tests, and package gates pass. | Verified locally |
| RF-014 | Python SDK now declares Python 3.10–3.13 support, bounded runtime/dev dependencies, `pyproject.toml`, constraints, and a package build/import check. | SDK builds and supported-runtime expectations are explicit and reproducible. | sdist/wheel build/import and `pip check` pass; clean cross-platform matrix remains external. | Verified locally |

### P1 SDK And Documentation Contracts

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-015 | `examples/README.md`, `docs/CLIENT_LIBRARIES.md`, and SDK examples previously referenced absent files, obsolete port/auth/topic shapes, and APIs not present in current clients. The SDK/docs batch corrected maintained examples and live matrix paths. | Operators previously copied examples that failed or silently subscribed to the wrong events. | Docs freshness, fixture, SDK, Go example, TypeScript, Python, and Playwright-list checks pass; live daemon handshake remains RF-042. | Verified locally; live handshake separate |
| RF-016 | Python pagination, Go search-detail, message field, transfer progress, and WebSocket example contracts were stale. The SDK/docs batch corrected these examples. | Examples previously could loop, duplicate data, or raise on valid responses. | The full cross-SDK gate and example contract checks pass; live daemon handshake/reconnect remains RF-042. | Verified locally; live handshake separate |
| RF-017 | TypeScript README now describes the CommonJS package as bundler-only in browsers, documents `fetch`/`WebSocket` runtime requirements, and narrows REST support to Node.js 18+ without claiming that `ws` is auto-adapted. | The previous CDN example and implicit Node WebSocket claim could send consumers to unsupported startup paths. | README contract correction is release-noted and covered by the SDK package/example gate; runtime matrix remains a separate external proof. | Verified locally |
| RF-018 | Go WebSocket listeners in `client-go/websocket.go:431-454` previously had no unsubscribe API; the current batch returns idempotent unsubscribe handles and removes closed channel references. | Long-lived dynamic consumers previously retained listener references. | Go unit/race/vet gates pass; cross-SDK live feed coverage remains RF-042. | Verified |
| RF-019 | `scripts/check-docs-freshness.sh:15-20` previously scanned four docs files only. The current batch scans maintained SDK/examples and includes a negative regression test. | Documentation gates previously reported verified while stale user-facing guidance remained. | `scripts/check-docs-freshness.sh` and its negative regression test pass; BUG-030 and BUG-039 now state the same executable freshness and SDK-contract evidence. | Verified locally |

### P1 Release And Test Coverage

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-027 | `web/e2e/README.md:58-60` previously used `cd src/web` and referenced absent fixture scripts/schema. The current batch adds a root wrapper, schema checker, corruption regression, updates E2E paths, serializes real-node workers, and permits a cold optimized build to finish. | Documented E2E setup previously was broken and zero-download fixture fetch could report success. | Serialized E2E passes 14 tests; 9 media-dependent tests skip because optional media is absent. | Verified with optional-media gap |
| RF-028 | `.github/workflows/publish-chocolatey.yml:17-48` now checks out the selected tag, uses a fully qualified release asset URL, verifies `SHA256SUMS`, and runs package smoke preparation. | Manual Chocolatey packages previously could contain invalid URLs and branch drift. | Validate workflow policy and a clean package runner before marking verified. | In progress |
| RF-029 | `.github/workflows/release-publish.yml:625-638` intentionally targets the existing `snapetech/homebrew-slskdn` compatibility tap, while `docs/dev/release-channels.md:15-18` named `snapetech/homebrew-slskr`. | Channel documentation was stale, but the publication target was not shown to be wrong. | Release-channel documentation now names the actual compatibility tap and preserves the explicit target boundary. | Verified locally, docs |
| RF-030 | `web/package.json:77-82` exposes a bundle-budget test; the current batch wires it after Web builds in `.github/workflows/ci.yml` and `scripts/run-release-gate.sh`. Serialized browser E2E now passes its available 14 tests; the React audit and retained nightly artifacts remain separate. | Bundle regressions previously could merge without CI evidence. | Workflow policy, Web budget, build-output, and available E2E checks pass; retain browser/nightly artifacts before full verification. | In progress |
| RF-031 | The release gate invokes the remediation baseline and differential checks, but the universal replacement acceptance document points to retained local artifacts and the release path does not run the full live transport/lifecycle manifest. | Historical acceptance evidence could be mistaken for fresh release evidence. | The acceptance document now explicitly labels the 2026-08-20 closure as historical and requires fresh release-gate/live artifacts for current certification. | Verified locally, policy explicit |

### P2 Efficiency And Structural Boundaries

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-020 | `crates/slskr-client/src/file_transfer.rs` now flushes only token/offset handshake writes; plain and obfuscated payload chunks use `write_all` without per-chunk flushes. Pinned Tokio 1.53.1 implements `TcpStream::flush` as a no-op. | On the production TCP path, removing these calls does not remove a socket syscall; local loopback measurements show no reproducible throughput change. Transfer queue durability is tracked separately in RF-047. | Two release-profile loopback runs compare validated old/current plain and obfuscated payloads (five 64 MiB samples per variant, 80 KiB chunks). Sample ranges overlap; retained JSON and host/toolchain details are under `crates/slskr-client/benchmarks/artifacts/`. Remote-peer throughput measurement remains open. | Verified locally; no material gain measured |
| RF-021 | `list_search_results`, `list_transfers`, and `get_webhook_logs` page full rows by ordered columns. The synthetic cardinality run uses 1,000,000 search results, 500,000 transfers, and 500,000 webhook logs. | Transfer and webhook composite indexes materially lower synthetic page-read latency. SQLite's existing `(search_id)` index already satisfies `ORDER BY id` through the implicit rowid; `(search_id, id)` adds storage but is not selected. | On Python SQLite 3.53.4, retained transfer/webhook composites reduce measured first/deep-page medians by 88–99%; search-result medians and plans do not change. The daemon removes the redundant search index on startup. Focused query-plan and file-backed migration regressions plus the 613-test suite pass. Machine-readable output is `benchmarks/artifacts/20260924-sqlite-cardinality.json`; methodology and the 3.46.0 bundled-SQLite limitation are in `benchmarks/artifacts/20260924-sqlite-cardinality.md`. Real-database measurements remain open. | Verified locally; production cardinalities separate |
| RF-022 | `scripts/check-client-sdk-gates.sh` now owns Go/Python/TypeScript SDK lifecycle, lint, advisory, build, and package checks; GitHub and release paths call it once, while GitLab keeps image-specific Go and Node feedback lanes. | CI latency and source-of-truth drift increase without more coverage. | `docs/dev/sdk-ci-ownership.md` records the split; the combined gate, GitHub/release wiring, and read-only Go lane pass locally. | Verified locally; hosted pipeline separate |
| RF-023 | `crates/slskr/Cargo.toml` has a default `focused-controller-tests` feature; the first implementation batch now selects the focused module explicitly with that feature. | Feature naming previously did not describe actual test selection. | Locked `cargo check` passes for default, `--no-default-features`, `bounded-persistence-tests`, and `full-controller-tests` configurations. | Verified locally |
| RF-024 | `crates/slskr/src/lib.rs` remains 26,054 lines after the latest ownership splits and `web/src` remains about 95k lines despite physical route/polling splits. The managed task registry, bounded HTTP task admission, and lifecycle regression live in `managed_tasks.rs`; runtime compatibility state logic lives in `runtime_compat_state.rs`; hash backfill state and persistence helpers live in `hash_backfill_state.rs`; source-discovery lifecycle state lives in `source_discovery_state.rs`; security bans, reputation, durable JWT revocation, and login-attempt throttling live in `security_state.rs`; bounded OAuth issue/consume state lives in `oauth_state.rs`; source-feed history, Lidarr sync state, Spotify connection state, and feed row/target models live in `integration_runtime_state.rs`; contact records/store live in `contact_state.rs`; destination records/store/path selection live in `destination_state.rs`; controller feature state and bounded JSON persistence live in `controller_feature_state.rs`; preview tickets live in `preview_stream_state.rs`; preview stream ticket creation and input validation live in `preview_stream_controller.rs`; versioned GET failure contracts live in `versioned_get_contract.rs`; versioned relay upload handling lives in `versioned_relay_controller.rs`; native compatibility route matching and read projections live in `native_compat_controller.rs`; Spotify and configured-provider source-feed previews live in `source_feed_preview_controller.rs`; `spawn_session_manager` lives in `session_runtime.rs`; the history-backfill route and FLAC candidate projection live in `hash_backfill_controller.rs`; wishlist auto-download planning and staged-transfer rollback live in `wishlist_auto_download.rs`; CSV parsing, simple text import, and versioned CSV import live in `wishlist_csv_import.rs`; listening-party stream limits live in `listening_party_stream_state.rs`; bounded PodCore runtime, signature, and membership verification counters, key validation, and recording logic with regression coverage now live in `podcore_runtime_stats.rs`; PodCore mutation, dynamic-get, and stats response handlers live in `podcore_controller.rs`; MediaCore extended and mutation response handlers live in `mediacore_controller.rs`. Pod membership DTOs, workflow/replay stores, canonical path parsing, acceptor authorization, and signature verification now live in `pod_membership_workflow.rs`; PodCore content-ID parsing, MusicBrainz requests, and metadata shaping live in `musicbrainz_lookup.rs`. | Ownership, review, compile feedback, and safe change isolation remain poor. | Batches 72-74 pass locked feature compiles, the 614-test daemon suite, runtime-boundary guard, and formatter. Batch 75 passes the full-controller/legacy library and test-target compiles, cancellation/order/rollback regressions, the 616-test daemon suite, runtime-boundary guard, scoped `await_holding_lock` Clippy pass, docs/plan freshness, formatter, and diff checks. Batch 76 passes the PodCore stat-key bounds and direct signing/verification counter regressions plus the locked full-controller/legacy library check. The feature-enabled test target compiles, but executing the existing signing-stat endpoint regression overflows its historical test-worker stack before assertions; the runtime-boundary guard, docs and plan freshness, formatter, and diff checks pass. The 27 listener tests passed in Batch 71 and the full release gate passed after Batch 71. Continue splitting by ownership and measure compile/test feedback after each move. Upload-peer cooldown state, key normalization, expiry, and failure classification moved with its three tests into upload_peer_cooldowns.rs; private-message auto-response cooldown tracking, limits, normalization, expiry, and two direct tests moved into private_message_auto_responses.rs; BrowseEntry and bounded remote path-encoding state now live together in browse_path_state.rs with direct entry/cap tests and the feature regression preserved; managed blacklist runtime state and bounded decision caching now live in managed_blacklist_runtime.rs, preserving target-specific matching, TTL/capacity, eviction, and replacement invalidation; shared controller regex matching and request-thread compilation now live in controller_regex.rs with existing root call sites preserved; private-message auto-response eligibility classification now lives beside its bounded cooldown tracker in private_message_auto_responses.rs; MessageRecord, bounded MessageStore state, and conversation JSON construction now live in message_store.rs while route persistence ordering and rollback remain at the crate root; RoomMessageRecord, RoomRosterEntry, RoomRecord, RoomStore, room projections, and bounded room-name handling now live in room_store.rs; UserRecord and bounded UserStore live in user_store.rs; UserNoteRecord and bounded UserNoteStore live in user_note_store.rs; InterestRecord and InterestStore live in interest_store.rs; NowPlayingRecord and bounded NowPlayingStore live in now_playing_store.rs; ShareGroupMember, ShareGroupRecord, and bounded ShareGroupStore live in share_group_store.rs; ShareGrantRecord, ShareAccessTokenRecord, and both bounded stores live in share_grant_store.rs; IncomingShareRecord and bounded IncomingShareStore live in incoming_share_store.rs; CollectionItem, CollectionRecord, and bounded CollectionStore live in collection_store.rs; WishlistItem, wishlist filters/policies, ignored-result models, and bounded WishlistStore live in wishlist_store.rs; SearchRecord, bounded SearchStore, search create outcomes, history/controller projections, and result identity handling live in search_store.rs; BrowseRecord, bounded BrowseStore state, indirect-token allocation, and controller/list projections live in browse_store.rs; library item/health/remediation models, query parsing, bounded LibraryStore, and catalog/health projections live in library_store.rs; share roots, index snapshots, lifecycle state types, and snapshot/catalog projections live in share_index_state.rs; share scan cancellation, rebuild/add lifecycle, snapshot generation checks, persistence, and publication now live in share_index_runtime.rs; HTTP connection and request-stream lifecycle now live in http_connection.rs while bounded parsing and response framing remain in http_server.rs; RelayState and conditional rollback logic live in relay_state.rs; bounded mesh sync-security state, capability registry, and mesh projections live in mesh_state.rs; controller options overlay, obfuscation reload snapshot, and restart fingerprint live in controller_options_state.rs; transfer state snapshot loading/writing and event-file I/O live in transfer_state_io.rs; generation-based durability snapshots, pending events, and TransferQueue persistence hooks live in transfer_durability.rs; TransferQueue and request/audio metadata models, ID allocation, transitions, and queue projections live in transfer_queue.rs; bounded auto-retry and underperformance rescue trackers/planners live in transfer_recovery.rs; TransferEntry serialization, redacted/native/controller projections, recovery fields, and bounded normalization live in transfer_entry.rs; file-transfer progress, retry, peer upload/download, and completed-media metadata lifecycle live in file_transfer_runtime.rs; session snapshot defaults, error mapping, and JSON summaries live in session_state.rs; ListenerSnapshot and ListenerCommand DTOs, listener JSON projection, and public listener error mapping live in listener_state.rs; DistributedRuntime models, persistence snapshots/status, connection roles, and state projections live in distributed_state.rs; storage-directory listing budgets, options, and bounded emission state live in storage_directory_state.rs; share scan inputs/results, cancellation-aware index building, and bounded filesystem traversal live in share_scanner.rs; virtual path normalization, hidden-path handling, extension extraction, and bounded media attribute probes also live in share_scanner.rs; persisted share-file projections, root summaries, attribute encoding, and compatibility cache I/O live in share_index_state.rs; SessionCommand and SearchDispatchTarget DTOs live in session_state.rs; SearchResultEntry, text-bounded search result projections, and file-attribute projection live in search_store.rs; share CatalogFilter query parsing and matching live in share_index_state.rs; local file SHA-256 hashing and bounded cache state live in local_file_hash.rs; controller rate-limit policy/model and partition/window projection live in rate_limit.rs; WishlistAutoDownloadPlan, track identity, quality ranking, edition checks, and wishlist path normalization live in wishlist_store.rs; resolved integration targets and private-address URL validation live in integration_target.rs; source provider matching, bounded ingestion responses, metadata parsing, and provider page requests live in source_feed_ingest.rs; local CSV, playlist, RSS/OPML, and bounded preview parsing live in source_feed_ingest.rs; Lidarr status/wanted HTTP clients, bounded JSON GET helper, quality-profile filter, manual-import candidate GET, and import-command POST live in lidarr_api.rs; Lidarr manual-import path mapping, candidate filtering, duplicate detection, and confined rejected-file deletion live in lidarr_import.rs; Lidarr import-history keys, status classification, persisted-record projection, and public redaction live in lidarr_import.rs; history persistence/listing and retry wrappers live in lidarr_import.rs; manual/automatic import execution, debounce, and retry sequencing live in lidarr_import.rs; database statistics projections, cleanup, retention pruning and scheduling, and vacuum handling live in database_maintenance.rs; bounded peer browse payload construction/parsing, virtual path composition, and folder projections live in browse_wire.rs; static asset root selection, path confinement, CSP policy, bounded file reads, static responses, and fallback dashboard HTML live in web_static.rs; controller options JSON response projection lives in controller_options_projection.rs; controller options debug projection and value redaction helpers live in controller_debug_view.rs; controller YAML target validation/error contracts, bounded parser, response, key conversion, and API projection helpers live in controller_yaml.rs; release-tag normalization/version comparison, scheduled refresh, and latest-version response live in controller_release_check.rs; controller capability parsing, negotiation, native response projection, network statistics, and peer capability descriptor helpers live in controller_capabilities.rs; controller event/search/HashDb/backfill storage preflight and identifier/query validation helpers live in controller_storage_preflight.rs; controller storage route projections, bounded Unix/non-Unix directory enumeration, filesystem timestamp shaping, shared scan limits, and storage query helpers live in controller_storage.rs; extended-controller mutation/search/download/dynamic-get/get responses now live in extended_controller.rs; PodCore mutation/dynamic-get/stats response handlers live in podcore_controller.rs; MediaCore extended/mutation response handlers live in mediacore_controller.rs; transfer telemetry/storage failure, native transfer validation, and auto-replace response helpers live in transfer_controller.rs Batch 179 moved `versioned_get_failure_contract` into `versioned_get_contract.rs` (517 lines); the 638-test daemon suite, both focused versioned GET differentials, locked full-controller/legacy library check, scoped `await_holding_lock` Clippy, changed-file formatter, and runtime-boundary guard pass. | In progress |
| RF-025 | The existing plan and performance report retained stale Web test counts without a clearly separated current snapshot. | Planning and release decisions could use incorrect baselines. | `docs/performance-analysis.md` labels the 2026-09-15 836/141 baseline as historical, records the 2026-09-24 local Web snapshot of 872 tests across 146 files, and keeps the 2026-09-22 dashboard snapshot of 38 tests across 11 files with commands. | Verified locally |
| RF-026 | `docs/dev/bug-burndown-ledger.md` marks several SDK/docs items verified while the current checks do not exercise the cited behavior. | Audit closure is not evidence-backed. | The ledger now names the executable checks for BUG-020, BUG-030, and BUG-039; docs freshness, SDK example contracts, the aggregate SDK gate, and the remediation baseline pass. Hosted/live compatibility rows remain explicitly separate. | Verified locally; hosted/live evidence separate |
| RF-032 | `docs/full-network-test-plan.md` mixed a historical all-pass result with a newer missing-artifact plan; `REMEDIATION.md` was a stale snapshot that said daemon tests were excluded even though CI runs them. | Operators could not tell current release evidence from archived history. | The network plan now labels its results historical and points operators to current release/live gates; REMEDIATION is explicitly historical with active-plan links. Referenced script/path validation remains part of the broader gate. | In progress, evidence boundary explicit |
| RF-033 | Council counts had drifted across `docs/dev/council-scan-inventory.md`, `.council/latest-candidate-counts.md`, and the active backlog; benchmark comparison/profile tests were not invoked; frontend configs lacked coverage thresholds. | Audit numbers drift and executable performance/coverage checks are absent. | The generated report now stamps date/commit provenance; active-backlog, inventory-closure, and council-freshness gates validate synchronized copies. Dashboard V8 coverage remains ratcheted in CI/release. CI and release now run the seven benchmark comparison/SQLite profiler unit tests and a focused Web API/session/event-lifecycle coverage gate at 84% statements, 75% branches, 72% functions, and 86% lines; the focused 50-test run passes at 85.54%, 78.13%, 73.91%, and 87.38%. | Verified locally |
| RF-034 | SDK gates, CI, and release gate reinstall/build the same TypeScript/Web assets; release archives previously used ambient mtimes/order and the SBOM serial was constant. | CI latency grows and release artifacts were not reproducible or uniquely identified. | Archives now use sorted entries, fixed source timestamps/ownership, deterministic gzip/ZIP metadata, and the CycloneDX serial derives from release version plus source commit. Two identical local builds produced SHA-256 `cb0ec78f41813cb7bf564c22057e1d56483219144a136a9d7456bb9e52ff853d`; shared CI artifacts remain open. | In progress, local deterministic proof |
| RF-035 | `crates/slskr-web/Cargo.toml:17-18` now pins lock revision `3825c9ad5e4ace15bb210012e79b2cbdbfc20434`. Locked WASM/package checks, shellcheck, npm policy, and dependency audits pass locally. | Dependency and policy drift is now covered by the local release surfaces. | Retain clean-runner/actionlint evidence before external closure. | Verified locally, external runner open |
| RF-036 | `crates/slskr-protocol/src/server.rs` repeated server-code variants, inventory metadata, numeric conversion, and direction-specific matches. | Adding a protocol code can update one table and miss another. | A declarative macro now owns the variants, numeric values, inventory order, names, and numeric decoding; protocol round-trip tests pass. Direction-specific payload dispatch remains explicit and is still a separate proof surface. | In progress, metadata source consolidated |
| RF-037 | `crates/slskr-client/src/overlay.rs` combines framing, handshake DTOs, validation, service DTOs, and request lifecycle in about 2,200 lines. | Unrelated changes share one review/compile boundary. | Framing, protocol messages/validation, and TLS/client lifecycle now have separate private modules; `overlay` preserves the public re-exports. The complete locked client package test suite passes. | Verified locally |
| RF-038 | `crates/slskr/src/storage.rs` and the production compatibility parser duplicated file-entry decoding and allocated joined cache strings through `format!`, `replace`, and `Vec<String>`. | Large share lists pay avoidable allocations and parser behavior can drift. | The production parser now uses one bounded file-entry decoder, both cache writers append escaped fields into one buffer, and malformed-count/output/escaping tests pass with the full daemon library suite. Quantitative allocation benchmarking remains separate. | Verified locally; benchmark separate |
| RF-039 | `crates/slskr/src/lib.rs:3122-3125` expires/prunes up to the 500-record cap through `delete_searches`; the current batch changes that persistence helper from one delete trio per ID to bounded `IN (...)` statements inside one transaction. | Search eviction previously could perform hundreds of round trips. | Persistence module and full workspace daemon tests pass with the bounded transaction path. | Verified locally |
| RF-040 | Python/TypeScript clients retry GETs by default; Go previously had no typed API error (`client-go/client.go:780-815`). The current batch adds structured Go `APIError` fields while explicitly documenting no automatic retries for mutation safety. | Equivalent SDK calls have different transient behavior and Go callers previously parsed error strings. | Go/Python/TypeScript SDK gates pass, with mutation retry boundaries documented; live cross-language retry/error fixtures remain separate. | Verified locally; live fixture separate |
| RF-041 | `docs/CLIENT_LIBRARIES.md:516-559` used nonexistent TypeScript methods and unsafe POST retry recipes. | Integration guidance could fail or create duplicate searches. | Maintained SDK examples and retry boundaries pass docs freshness, example-contract, and SDK package gates. | Verified locally |
| RF-042 | `crates/slskr/src/events_ws.rs:247-256` rejected all client text/binary frames, while Go, Python, and TypeScript SDKs send `subscribe`/`unsubscribe` JSON frames. The current batches accept bounded JSON commands/filtering and add a dependency-free cross-SDK fixture covering dotted event types, `data.topics`, unsubscribe filtering, and reconnect resubscription. | Every SDK subscription previously closed the event feed instead of filtering events; the release contract was contradicted. | Go, Python, and TypeScript fixture suites pass. Rust-daemon authentication/persistence and external interoperability remain separate evidence. | Verified locally, external proof open |
| RF-043 | `.github/workflows/ci.yml`, `scripts/run-release-gate.sh`, package verification, and Go SDK gates previously allowed dependency resolution to change during validation. | CI/release/package checks could silently regenerate dependency resolution. | Cargo package, metadata, tree, protocol-adversarial, and extracted-workspace checks now use `--locked`; Go tests use `-mod=readonly`, and the local package gate passes. Hosted clean-runner and stale-lock evidence remain separate. | Verified locally; hosted clean-runner separate |
| RF-044 | Direct plain/obfuscated init readers previously allocated up to the 16 MiB frame cap before applying 4 KiB peer fields. The current batch inspects fixed headers first, caps known `PeerInit` frames at 8205 bytes, and preserves `PeerUsernameTooLong`. | A peer previously could amplify handshake memory across permits with an oversized known init field. | Client/protocol/IO suites pass. The shared listener now preflights known plain and obfuscated `PeerInit` headers before buffering the body; unknown-init 16 MiB compatibility remains an explicit residual. | Verified locally; unknown-init stress separate |
| RF-045 | `crates/slskr/src/lib.rs:80830-80842`, `route_dispatch_group_2.rs:771-797`, and `persistence.rs:2345-2377` delete/reinsert a full search projection after each accepted response. | A 10k-result burst becomes O(n^2) work despite bounded SQL parameter batches. | Search acceptance now returns the newly admitted delta and persistence appends only those rows in one transaction; peer and HTTP paths use the delta, and append-preservation tests pass. Crash/restart and 10k-response benchmark evidence remain separate. | Verified locally; burst/crash proof separate |
| RF-046 | `crates/slskr-client/src/search.rs:350-381` uses full `Vec::contains` equality over response file/private-result vectors for duplicate detection under a per-token cap. | Adversarial duplicate bursts can consume O(responses x file-list) CPU while holding search state. | Search responses now use a stable fingerprint bucket with exact-equality fallback, including cleanup on token removal and duplicate/distinct-response tests. Adversarial benchmark evidence remains separate. | Verified locally; benchmark separate |
| RF-047 | `crates/slskr/src/lib.rs:84944-84958`, transfer queue mutations, and `92562-92667` previously performed full JSON rewrite, fsync, and event append while holding the async transfer write lock. The current batch snapshots pending events/state and flushes durability after the lock; event-order and restart-normalization assertions pass, with a delayed-coordinator queue-lock regression. | Slow disks previously stalled every transfer/API reader and every download chunk repeated durable full-state work. | The 561-test daemon library suite passes. Real delayed-filesystem and crash/restart stress proof remain before full closure. | In progress |
| RF-048 | `web/e2e/smoke-auth.spec.ts` and `core-pages.spec.ts` previously accepted blank/unknown UI states as success; the library E2E treated shares/content as optional. The current specs require authenticated navigation, route denial, logout, Browse, System, Shares, and table roots. | Browser regressions and broken fixtures now fail the maintained deterministic paths instead of passing through fallback branches. | Playwright passes all 14 available assertions; optional media remains a separate skipped-fixture gap. | Verified locally with optional-media gap |
| RF-049 | Dashboard now has focused tests for App health, Dashboard, Database, Monitoring, ApiKeys/Configuration/Webhooks, Sidebar, and ErrorBoundary, with an explicit V8 coverage provider and representative axe coverage. | The previous dashboard test gap for administrative and shell/error surfaces is closed locally; coverage can now regress only below the ratcheted threshold without failing the gate. | `npm run test:coverage` passes 38 tests with 67% statements, 55% branches, 55% functions, and 71% lines; CI and release run it. The latest dashboard build is 242.02 KiB initial JavaScript and 87.84 KiB gzip against 260/100 KiB budgets. | Verified locally; deployed-browser evidence remains separate |
| RF-050 | `REMEDIATION.md` is now explicitly a dated historical baseline; `PLAN.md` points current readers to the active refactor plan and evidence ledger, and the new plan-freshness check enforces those links. | Operators may execute obsolete migration/backfill steps or misread current scope when historical rows look current. | `scripts/check-plan-freshness.sh` and its negative test pass in the remediation baseline; historical phase detail remains intentionally retained. | Verified locally |

### P1/P2 Web Runtime And UI Boundaries

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-051 | `web/src/components/App.jsx:823-935` now uses scalar activity endpoints, short-circuits room/message routes, and pauses the navigation timer while hidden. | Navigation previously could fan out hundreds of requests and duplicate room/chat work. | App tests cover route exclusion, hidden-tab pause/resume, and one refresh per source after visibility returns; the full Web suite remains the final local gate. | Verified locally; deployed-browser evidence remains separate |
| RF-052 | `dashboard/src/hooks/useFetch.ts:66-75,130-141` now coalesces active requests, schedules only after settlement, pauses/resumes on visibility, and clears a pending timeout when a manual refresh starts. | Slow requests previously could be perpetually aborted and background tabs churned requests; manual refresh could also leave a duplicate poll scheduled. | Dedicated hook coverage now exercises slow, hidden, manual, and unmount paths; the full 38-test dashboard suite, type-check, and lint pass. | Verified locally; deployed-browser evidence remains separate |
| RF-053 | `web/src/components/Shared/Footer.jsx:101-142` polls every 2 seconds and invokes a six-request stats fan-out also used by Network every 5 seconds. The current batch adds a 2-second in-flight/short-TTL cache around the shared stats helper and skips the native poller entirely for the legacy footer. | Global telemetry previously created redundant server/browser work, including in legacy profiles. | Footer regression coverage and the full Web suite pass; deployed request cadence remains separate evidence. | Verified locally; deployed cadence separate |
| RF-054 | Messaging/Rooms polling and SignalR callbacks directly call full workspace hydration; the current batch adds a per-workspace refresh gate that coalesces active requests and schedules one follow-up. | Hub bursts and timer ticks previously overlapped hydrations. | Messaging has a deferred-promise test proving an event during hydration produces one serialized follow-up; the focused suite passes 18 tests and the full Web suite passes 857 tests. | Verified locally; deployed/live evidence remains separate |
| RF-055 | Chat, room, and pod panels fetched full histories despite `since` support (`ChatSession.jsx:70-152`, `RoomSession.jsx:158-183`, `Messaging.jsx:179-223`). | Payload, parsing, allocation, and render costs grew with history size. | Shared cursor deltas with full-read fallback, deduplication, bounded histories, and stable message keys are implemented; focused and full Web tests pass. | Verified locally; live browser evidence remains separate |
| RF-056 | Web polling stopped timers but could not abort callback requests; dashboard mutations and Search missing-status checks only set cancellation flags. | Route changes still consumed server work and repeated status requests could overlap. | Chat, room, and pod history reads now propagate `AbortSignal` and use per-resource in-flight gates; unmount/abort tests and the full Web suite pass. | Verified locally; unrelated dashboard/Search cancellation remains outside this slice |
| RF-057 | Dashboard health state retains the last successful response during a transient refresh failure; `App.tsx` now keeps routing usable and surfaces a distinct degraded-connection alert. | The previous implementation replaced a usable last-known-good view with the initial connection screen on any refresh error. | Dashboard App success-then-failure regression, type-check, lint, and full suite pass. | Verified locally; external browser/device evidence remains separate |
| RF-058 | App navigation still sets state when flags are unchanged; render mutates title/classes; MediaCore and Integrations remain multi-thousand-line ownership boundaries. | Shell updates rerender broad trees and large workflows remain hard to profile/change. | Navigation skips unchanged activity state, and title/theme DOM synchronization runs in lifecycle methods. Profiler evidence led to memoized message-search and external-ID resolver panels. Their JSDOM median durations improved from 9.81/10.86/9.73 ms to 0.130/0.152/0.156 ms, and from 9.39/10.48/9.90 ms to 0.130/0.321/0.329 ms, respectively. Full Web suite passes 876 tests. Broader MediaCore and Integrations boundaries still need profiler-led extraction. | In progress, broader structural boundaries open |
| RF-059 | PlayerBar previously statically imported optional Visualizer/RustyMilk; System eagerly imported most tabs; dashboard eagerly imported all pages. The current batches lazy-load the visualizer and dashboard routes and load all remaining System panes by section. | Users previously paid parsing costs for optional features/routes. | The System entry chunk fell from 236.95 KiB to 9.17 KiB; Vite emits six section chunks plus independent AdminPolicies, Integrations, and MediaCore chunks. Initial JavaScript (1,084.68 KiB) and all-JavaScript gzip (593.07 KiB) pass their 1,150/600 KiB budgets. The route-switch regression and full Web suite (873 tests across 147 files) pass. The asset graph is retained in `benchmarks/artifacts/20260924-system-pane-asset-graph.md`; deployed-device evidence remains open. | In progress, deployed evidence open |
| RF-060 | Browse tree repeatedly filtered full directory lists and compared selection by set size; normalized name handling differed. The current batch builds parent lookups, memoizes sanitized trees, compares set membership, and tests wide/deep/file-heavy malformed fixtures plus same-sized replacements. | Large shares previously stalled and same-size replacement trees could retain stale selection. | Browse fixture, replacement-tree, and full Web tests pass. | Verified locally |
| RF-061 | Shared semantic wrappers used clickable divs/icons without keyboard/accessible-name semantics; dashboard navigation lacked `aria-current`. The current batch adds keyboard activation, icon roles, active navigation state, explicit browse control names, and axe checks for representative Web/dashboard shells. | Keyboard and screen-reader users previously missed controls and context. | Semantic, Browse, Sidebar, and axe tests pass; deployed-browser/device coverage remains separate. | Verified locally; deployed evidence separate |
| RF-062 | Web mounted `App` without an error boundary; route try/catch could not catch descendant render failures. The current batch adds a reloadable render fallback. | One render error previously could blank the entire UI. | ErrorBoundary test and full Web tests pass. | Verified |
| RF-063 | Web audit records responses but not request-count/cadence/accessibility budgets. The current batch preserves per-asset budgets, adds deterministic aggregate initial-JS and gzip budgets for Web/dashboard, adds local axe checks for representative shells, and records Rust Web request-count/cadence budgets across 15 desktop/mobile route pairs. | Duplicate requests, hidden-tab churn, accessibility, and aggregate regressions previously could pass. | Web/dashboard aggregate budgets, representative axe checks, and Rust Web mock request-count/cadence budgets pass locally. The rebuilt Rust Web audit made 512 requests across 30 route views versus the 518-request baseline; the 200 ms repeated-endpoint floor passed, with Solid status repeating at 594.8–608.3 ms in the retained run. Full deployed accessibility and live-backend cadence evidence remain open. | In progress |
| RF-064 | `AppContext.js` is active around the routed App; legacy `Pods.jsx` is not the `/pods` route; unused-symbol lint is disabled; dashboard still has separate context/prop ownership to review. | Dead modules and competing ownership patterns dilute refactor/test signal. | `docs/dev/web-ownership-inventory.md` records active versus deferred ownership, and `scripts/check-web-ownership-inventory.sh` verifies the route/context/import boundary. Legacy `Pods.jsx` deletion remains explicitly deferred. | Verified locally, deletion deferred |
| RF-065 | `LyricsPane.jsx:148-159` used both `timeupdate` and a 500 ms interval for the same position update. The current batch removes the duplicate timer and adds a regression test. | Continuous duplicate state checks previously ran while lyrics were visible. | Focused LyricsPane test passes; keep the full Web suite as proof. | Verified |

### P2/P3 Evidence And Gate Hygiene

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-066 | Fixture fetchers previously recorded observed hashes instead of verifying expected values. The current batch adds a manifest checker, checksum/size enforcement, corruption regression, and one root fetch wrapper. | Remote fixture drift previously could silently change E2E/share behavior. | Fixture manifest tests pass; serialized E2E passes with static fixtures; optional remote media remains unavailable in this Linux environment. | In progress, optional-media gap |
| RF-067 | GitHub and GitLab test jobs now run the bounded docs-freshness and active-plan-freshness checks plus their negative tests before the Rust matrix. | Scope and maintained-guidance regressions are now found during review instead of only at release. | Local checks pass; hosted GitHub/GitLab runner results remain external evidence. | In progress, local/CI wiring |
| RF-068 | `benchmarks/README.md` and `docs/performance-analysis.md` describe manual one-off scripts without stored JSON baselines, thresholds, or a CI/nightly target. | Performance drift has no reproducible regression signal. | The benchmark docs now explicitly classify the commands as diagnostic-only and require retained JSON plus environment metadata for release evidence. | Verified locally, diagnostic-only |
| RF-069 | `web/package.json` exposes RustyMilk compatibility/performance/smoke scripts, but no workflow invokes them while `slskr-web` tracks the upstream main branch. | Web dependency drift can pass without compatibility evidence. | The performance note explicitly classifies these scripts as diagnostic-only; a scheduled threshold job remains intentionally absent. | Verified locally, diagnostic-only |
| RF-070 | Council count files had no scan date/commit and checks regenerated only when a report was missing. | Stale counts could pass active-backlog checks. | `run-council-scan.sh` stamps the report; `check-council-freshness.sh` validates date and full source commit across the report, inventory, and backlog, with a negative test and all-phases/remediation wiring. | Verified locally |
| RF-071 | The audit PASS matrix records a SHA but not toolchain/node/npm versions, lock hashes, or retained artifact links. | A clean checkout cannot reproduce the evidence exactly. | `scripts/collect-reproducibility-metadata.py` records the source SHA, dirty-worktree state, required tool versions, lock/config SHA-256 values, and optional artifact hashes; the remediation baseline validates a current local snapshot. Retained hosted artifact links remain external. | In progress, local metadata gate |
| RF-072 | `docs/live-interop-test-matrix.md:39,119` referenced absent `web/e2e/live-surfaces.spec.ts`; the current batch updates it to maintained specs and keeps historical results explicitly dated. | Operators previously could not reproduce the stated live matrix. | Docs freshness passes and the maintained Playwright suite runs locally; add retained live artifact links before marking verified. | In progress |
| RF-073 | `scripts/run-release-gate.sh:69-74` makes the slskd API compatibility smoke opt-in; `docs/release.md:41-63` intentionally assigns that smoke to scheduled/manual Live Parity. The current batch now labels the skipped local check as a certification artifact requirement. | This is a policy boundary, not a code defect, but release certification could be misread if artifact freshness was not visible. | Keep the opt-in behavior and require a retained scheduled artifact for certification. | In progress, policy |

## Continuation Evidence (2026-09-21)

RF-055 and RF-056 were re-checked against the current Web implementation. The
server contracts already exercised by the Rust controller tests use explicit
millisecond cursors for conversation, joined-room, and pod-channel message
reads. The Web implementation therefore sends those cursors only after a
usable cursor has been observed, merges bounded deltas by stable message
identity, and keeps full reads for profiles that do not expose one.

The focused continuation tests passed 42 tests. The full Web suite passed 848
tests in 145 files, followed by Web lint, production build, aggregate bundle
budget, and build-output checks. This local proof does not claim live
third-party browser interoperability or close the unrelated dashboard/Search
cancellation findings.

## Continuation Evidence — local gate closure (2026-09-21)

The local continuation closed several gate and lifecycle findings without
claiming external platform closure:

- RF-007 now has cancellation-aware filesystem traversal, a focused
  pre-indexing cancellation regression, and a full locked workspace test pass.
  HTTP request tasks are joined during graceful shutdown, but detached
  scheduler/listener tasks remain RF-006 residuals.
- RF-010/RF-012/RF-013/RF-014 have local packaging proof: both AUR definitions
  pass isolated `makepkg` source smoke, TypeScript packaging passes its
  allowlist, Python quality passes Black/Flake8/mypy and 49 tests, and Python
  sdist/wheel build/import plus `pip check` pass.
- RF-027/RF-048 have stronger browser evidence: the serialized Playwright run
  passes 14 available tests with 9 optional media tests skipped, and the
  tightened auth, route-guard, library, System, Shares, and table assertions
  no longer accept blank fallback UI.
- The frozen endpoint inventory passes at 64 slskd + 410 slskdN calls, the
  runtime-boundary checker accepts cancellation-aware share workers, and the
  corrected SDK/package/release gates pass locally. The top-level gate exposed
  stale assumptions during this work; corrected sub-gates and all downstream
  release commands were rerun after each fix.
- RustSec is clean after upgrading rustls 0.23.45 and rustls-webpki 0.103.15.
  The full workspace test, all-target clippy, locked WASM check, Cargo package
  extraction/rebuild, Cargo package checks, Semgrep, Trivy, Web (848 tests),
  dashboard (33 tests), and TypeScript SDK checks all pass locally.

## Continuation Evidence — full local release gate (2026-09-22)

The full `scripts/run-release-gate.sh` passed after the final refactor slices.
The local gate covered the locked Rust workspace (560 daemon-library tests),
Web (857 tests across 146 files), dashboard coverage (38 tests across 11
files), SDK gates, package extraction, deterministic artifact checks, RustSec,
Semgrep, Trivy, CORS/security differentials, and Web/dashboard build budgets.
The final Web build measured 1,084.14 KiB initial JavaScript and 585.5 KiB
aggregate gzip; the dashboard measured 242.02 KiB initial JavaScript and 87.84
KiB gzip. These are local release-gate results, not hosted-runner or deployed
browser claims.

Remaining evidence gaps are clean GitLab/Windows/Chocolatey runner results,
optional media fixtures, retained external live-parity artifacts, a complete
shared detached-task registry, and the older protocol/lock/search-delta/
benchmark findings listed in the tables above.

## Continuation Evidence — managed daemon shutdown (2026-09-22)

RF-006 received the local implementation slice. `AppState` now owns a
`ManagedTaskRegistry` that starts with a live `JoinSet`, accepts only
`Send + 'static` unit futures, takes ownership of the set during shutdown, and
aborts/joins it behind a two-second timeout. The registry covers long-lived
SongID/session/scheduler workers, configured listener managers, bridge and
relay services, overlay gateway, DHT, the signal and startup version helpers,
and Unix/HTTPS accept-loop supervisors. Lifecycle shutdown, restart, and
one-shot completion drain the HTTP request set and then the managed registry.

The focused async regression proves a pending registered task is dropped by
shutdown and that a second shutdown is harmless. This is stronger local proof,
not completion: connection-scoped handlers remain bounded by their existing
semaphores, while clean GitLab/Windows/Chocolatey execution and retained live
shutdown-overlap artifacts are still external evidence gaps.

## Continuation Evidence — bounded HTTP listener handlers (2026-09-24)

RF-006 now applies one 256-connection semaphore to plain HTTP, HTTPS, and Unix
socket HTTP. Plain HTTP handlers remain in the request `JoinSet`; HTTPS and
Unix handlers are registered directly in `ManagedTaskRegistry`, so listener
shutdown and daemon shutdown both abort and join them. The focused regression
proves shared HTTPS/Unix capacity, permit release after managed shutdown, and
rejection of new handlers after registry shutdown. The 614-test daemon library
suite, full-controller feature compile, runtime boundary guard, changed-file
formatter, and complete remediation baseline pass. Clean-runner and retained
live shutdown-overlap evidence remain external. Operator-facing behavior is recorded in
`release-notes/20260924-http-listener-task-bounds.md`.

## Continuation Evidence — dashboard polling lifecycle (2026-09-22)

RF-052 received the remaining local lifecycle proof. `useFetch` now removes a
pending poll timeout before a manual refresh starts, preventing a timer that
was scheduled by the previous successful response from issuing a duplicate
request after the refresh. The hook retains its in-flight coalescing,
hidden-document pause, visible-document refresh, and abort-on-cleanup behavior.

The focused hook suite passes 11 tests, and the full dashboard suite passes 37
tests with type-check and lint. This validates the browser-runtime contract in
the repository; deployed-browser/device evidence remains outside the local
gate.

## Continuation Evidence — navigation activity and dashboard coverage (2026-09-22)

RF-051's focused App suite now passes 18 tests, including the messaging-route
exclusion and hidden-tab pause/resume cadence. The test isolates the activity
endpoints and confirms that visibility restoration issues one refresh per
source before the regular 10-second timer resumes.

RF-049's previously stale audit description was corrected: App,
administrative-page, Dashboard, Database, Monitoring, Sidebar, and
ErrorBoundary tests are present and included in the dashboard gate. A
ratcheted coverage threshold is still open and is not implied by these focused
tests.

## Continuation Evidence — persistence rollback (2026-09-22)

RF-003 and RF-004 received stronger local persistence evidence. The new
ignored-search failure-injection test forces a result-row insert to fail and
asserts that the wishlist rule, search row, stable identity, and result rows
are all absent afterward. The existing distributed-child, transfer/event, and
search-result replacement rollback tests remain green.

The full `persistence::tests` module passes 21 tests. This is transaction and
rollback proof only; distributed in-memory lock scope and restart recovery
remain separate P0 evidence.

## Continuation Evidence — Messaging hydration concurrency (2026-09-22)

RF-054 now has an explicit deferred-promise regression in
`web/src/components/Messaging/Messaging.test.jsx`. It fires a message-hub
change while the initial `chat.getAll()` hydration is pending, confirms no
overlapping request starts, then confirms exactly one follow-up hydration after
the first request resolves. The focused Messaging suite passes 18 tests.

## Continuation Evidence — ownership and reproducibility gates (2026-09-22)

RF-064 now has an executable ownership inventory. It confirms that AppContext
is still provided by `App.jsx`, `/pods` is owned by the Messaging route, the
PortForwarding compatibility alias remains explicit, and the legacy `Pods.jsx`
implementation has no production route import. Deletion is deferred until a
separate migration defines replacement coverage.

RF-068 and RF-069 are explicitly diagnostic-only: benchmark and RustyMilk
commands require retained operator artifacts before they can be treated as
release evidence, but do not pretend to be scheduled regression thresholds.
RF-071 now has a local gate that writes
`target/reproducibility/current.json` with the source commit, dirty-worktree
state, required tool versions, provenance-file hashes, and optional artifact
hashes. Hosted retention and links remain external evidence.

RF-034's deterministic archive proof produced the same SHA-256 on two local
`scripts/build-release-archive.sh --version reproducibility-test --skip-web-build`
runs, and `scripts/verify-release-artifacts.sh target/dist` accepted the
result. The local proof does not claim CI build-artifact reuse.

## Continuation Evidence — dashboard coverage and bundle budgets (2026-09-22)

RF-049 now has an executable ratchet rather than only a test-count snapshot.
Dashboard declares `@vitest/coverage-v8`, runs coverage over the source tree,
and enforces minimums of 67% statements, 55% branches, 55% functions, and 71%
lines. The current 38-test suite reports 67.52%, 55.42%, 55.2%, and 71.83%
respectively.

RF-063 now runs both Web and dashboard aggregate bundle checks in the release
and CI paths. The current dashboard build is 242.02 KiB initial JavaScript
and 87.84 KiB gzip against 260 KiB and 100 KiB limits. Request/cadence and
accessibility budgets remain open findings.

## Continuation Evidence — full local release gate (2026-09-23)

The current dirty worktree passed `scripts/run-release-gate.sh`. Its remediation
baseline, security scans, package extraction, AUR smoke, locked Rust checks,
SDK/package gates, docs and audit checks, Web suite/build/budgets, and dashboard
suite/coverage/build/budgets all completed successfully. The full workspace
suite passed 560 daemon library tests; after adding the RF-047 regression, the
daemon library suite passed 561 tests.

The new RF-047 test holds the transfer durability coordinator to model delayed
persistence and verifies another task can acquire the queue write lock before
the persistence gate opens. Existing event-order and restart-normalization
assertions also pass. The repository still needs real delayed-filesystem and
crash/restart stress evidence before RF-047 is closed.

RF-003 and RF-004 now also use file-backed tests that close and reopen SQLite.
They confirm distributed tree state/children and ignored-wishlist rules,
search metadata, external identities, and result rows survive database reopen.
App startup rehydration and the distributed state-lock scope remain open.

The council scan was refreshed for 2026-09-23 with candidate counts
9/150/183/1012/1093/323. The active backlog, inventory closure, and freshness
checks pass. The Rust dependency report now records that `cpufeatures` and
`crypto-common` converged, and the policy gate no longer lists them as current
duplicate roots.

The optional live slskd API smoke was skipped under its documented local
policy, and local `actionlint` was unavailable. Clean hosted runners, retained
live-parity artifacts, optional media fixtures, and the other external proof
boundaries remain open.

## Continuation Evidence — distributed persistence isolation (2026-09-23)

RF-003 now serializes distributed-state writes through a single watch-backed
worker. Runtime mutations publish a monotonically revisioned snapshot while
holding the state guard, then wait for the worker after releasing that guard.
The watch channel retains only the latest snapshot, so a slow database writer
does not grow an unbounded persistence queue. Branch metadata and child depths
are saved atomically; the loader reads them in one transaction and applies the
result only if no newer in-memory mutation has occurred.

The regression set covers a failed child insert rolling back both tables,
coalesced/latest-snapshot persistence, and file-backed restart hydration through
the same helper called during session-manager startup. The complete daemon
library suite passes 564 tests, `cargo check --locked -p slskr --lib
--features full-controller-tests` compiles the historical suite, and
`cargo test --locked -p slskr --lib --features full-controller-tests
distributed --quiet` passes 13 historical distributed tests.
`scripts/check-rust-format.sh` passes. The historical search-filter test was
also updated to assert the current `(record, appended-results)` return shape.

Managed-worker shutdown overlap remains open. The post-batch
`scripts/run-release-gate.sh` passed after one transient upstream watched-picture
fixture failure; both the isolated profile/distributed rerun and subsequent
complete gate passed for frozen slskd and slskdN. The complete gate ran 564
daemon library tests, 24 server integration tests, 86 Web crate tests, 857 Web
tests across 146 files, and 38 dashboard tests across 11 files, and security
scans found no issues. The opt-in live slskd API smoke was skipped by local
policy, and hosted runners/retained live artifacts remain external evidence.
The behavior change has a validated release-note fragment at
`release-notes/20260923-distributed-state-atomic.md`.

## Continuation Evidence — library/runtime transaction ordering (2026-09-23)

RF-002 now has a focused cross-store persistence path for local Lidarr manual
imports. A shared runtime-persistence mutex orders this transaction with the
runtime compatibility writers; the SQLite transaction creates the library
item and updates only the manual-import counter/timestamp in the runtime row.
This avoids replacing unrelated persisted runtime fields with an older
snapshot. The route releases its state guards before database work.

If the transaction fails, rollback removes the candidate library row and
counter only while those values still match this operation. Concurrent
updates to other library or runtime state are preserved. A SQLite trigger
regression proves both persisted writes roll back together and that an update
to one existing runtime field remains unchanged. Focused in-memory tests cover
both rollback directions and show a runtime mutation waits for its persistence
turn without blocking readers.

`cargo test --locked -p slskr --lib --quiet` passes 567 tests;
`cargo check --locked -p slskr --lib --features full-controller-tests` and
`scripts/check-rust-format.sh` pass. The targeted
`SLSKR_OPTIONS_DIFFERENTIAL_SCENARIOS=lidarr-runtime
scripts/check-controller-options-differential.sh` passes against frozen
slskd and slskdN. The full release gate passed after a transient fixed-port
fixture collision on the first attempt; the isolated no-connect scenario and
the complete retry both passed. The successful run reported 567 daemon
library tests, 24 server integration tests, 86 Web crate tests, and 857 Web
tests across 146 files. Semgrep and Trivy reported zero findings; package,
SDK, AUR, Web, and dashboard build/budget checks passed. The opt-in live API
smoke was skipped by local policy and local `actionlint` was unavailable. The
change has a validated release note at
`release-notes/20260923-library-runtime-transaction.md`.

## Continuation Evidence — ignored-search startup hydration (2026-09-23)

RF-004 now has a testable production startup boundary. `serve` delegates
wishlist and ignored-result loading to `load_wishlist_store`, which reads both
record sets before constructing the runtime store and keeps database errors
explicit. The focused regression seeds a file-backed database, closes and
reopens it, invokes the same loader used at startup, then proves the restored
rule is visible through the wishlist API and still filters matching search
responses without suppressing an allowed directory.

The focused regression and `cargo test --locked -p slskr --lib --quiet` pass
568 tests. `cargo check --locked -p slskr --lib --features
full-controller-tests`, `scripts/check-rust-format.sh`, and `git diff --check`
also pass. This is internal-only refactor and regression coverage with no
user-facing behavior change.

## Continuation Evidence — distributed snapshot shutdown flush (2026-09-23)

RF-003 now calls a bounded final snapshot write after managed workers have
stopped. The file-backed regression first cancels the watch-backed persistence
worker, publishes a newer runtime tree revision, then invokes the production
shutdown method and confirms the latest tree and child rows were persisted.
The focused regression and 569-test daemon suite pass; the full-controller
feature compile and formatter pass as well.

The first complete release-gate attempt hit a watched-restart timing failure
in the upstream controller fixture. The isolated profile/distributed scenario
passed for frozen slskd and slskdN, then the complete release-gate retry
passed. The run covered 24 server integration tests, 86 Web crate tests, 857
Web tests across 146 files, and 38 dashboard tests. Security, packaging, AUR,
SDK, Web, and dashboard gates passed. The optional live slskd API smoke was
skipped by local policy and local `actionlint` was unavailable; hosted and
retained live shutdown-overlap evidence remains external. The operational
change has a validated release note at
`release-notes/20260923-distributed-shutdown-flush.md`.

## Continuation Evidence — pod/channel lock order (2026-09-23)

RF-002 pod update and delete now take the channel-store write lock before the
pod-store write lock. They reject unauthorized callers during an initial read
and revalidate authorization after both locks are acquired. A request waiting
behind channel persistence no longer blocks unrelated pod reads.

The focused production-route regression holds the channel write lock and
confirms GET completes while both PUT and DELETE are waiting. The daemon
library suite passes 570 tests, the `full-controller-tests` feature compile
passes, and the targeted pod-management controller differential passes. This
batch is internal-only. Cross-file crash recovery and collection/grant
consistency remain open. The Batch 21 release gate remains the latest complete
gate.

## Continuation Evidence — collection/share-grant parent check (2026-09-24)

RF-002 now rechecks a collection's existence and ownership while holding the
share-grant write lock, using the same grant-then-collection order as
collection deletion. The SQLite share-grant upsert also requires a live parent
collection, so a queued grant write cannot recreate a relationship after a
delete transaction.

A deterministic route regression queues collection deletion before grant
creation and verifies the create returns 404, memory and SQLite remain free of
the grant, and a direct orphan upsert is rejected. All 571 daemon library
tests pass, as do the four frozen-controller share-grant CRUD, persistence,
ownership, and open-case differential tests. The differential fixtures now
persist their parent collections through the production route. The user-visible
consistency fix has a validated release note at
`release-notes/20260924-share-grant-parent-consistency.md`. The Batch 21 full
release gate remains the latest complete gate.

## Continuation Evidence — collection snapshot parent check (2026-09-24)

RF-002 collection, share-grant, and share-token mutations now acquire a persistence
turn before taking the store lock, preserving the order of memory changes and
SQLite writes. Explicit collection creation may insert a parent row; update, item, and reorder
snapshots replace rows only while the parent exists in SQLite. A delayed create
or stale write therefore cannot restore a collection after deletion.

A focused regression creates and deletes through the production routes, then
replays a captured stale snapshot through the collection update helper. The
write fails and SQLite remains empty. A second regression holds the persistence
turn, confirms a waiting delete leaves reads available, then verifies ordered
completion. A queued token request returns 404 after deletion removes its grant.
All 574 daemon library tests pass, as do the three frozen-controller collection differentials, the
`full-controller-tests` feature compile, and the Rust formatter. The correction
has a validated release note at
`release-notes/20260924-collection-parent-consistency.md`. The Batch 21 full
release gate remains the latest complete gate.

## Continuation Evidence — share-group mutation ordering (2026-09-24)

RF-002 share-group create, update, delete, and membership routes now acquire a
shared persistence turn before changing memory. The split and legacy dispatchers
hold this turn through the database transaction, but release the store write
guard during SQLite I/O. Failure rollback compares the live store with the
failed snapshot before restoring prior state.

A focused route regression queues group deletion before a member add while the
persistence turn is held. Group and member reads complete while both wait; once
released, the delete succeeds, the member add returns 404, and the database has
no group or member rows. The focused regression, all 575
daemon library tests, and the share-group persistence/concurrency
controller differential pass. The `full-controller-tests legacy-route-dispatch`
combination compiles, but the focused route overflows the historical
monolithic dispatcher’s Tokio test-worker stack when executed. The persistence
correction has a validated fragment at
`release-notes/20260924-share-group-mutation-order.md`; the Batch 21 full
release gate remains the latest complete gate. The changed-file formatter,
documentation freshness, plan freshness, release-note preview, and
`git diff --check` pass.

## Continuation Evidence — managed shutdown overlap (2026-09-24)

RF-003/RF-006 now has deterministic local overlap proof. The focused test starts
a pending registered worker and begins `shutdown_managed_tasks` while holding
the distributed runtime write lock. It observes the worker drop, publishes the
final runtime revision while shutdown remains in progress, releases the lock,
and verifies that the final revision reopens from SQLite.

The focused regression and all 574 daemon library tests pass. This is an
internal test-only change with no release-note fragment. Retained live
shutdown-overlap and crash/restart evidence remain external; the Batch 21 full
release gate remains the latest complete gate.

## Continuation Evidence — share-scan shutdown overlap (2026-09-24)

RF-007 now has a deterministic production-path shutdown overlap regression.
The test holds the share-lifecycle lock after `rebuild_share_index` registers
its cancellation token, calls `initiate_graceful_shutdown`, then releases the
lock. The rebuild worker returns `SHARE_SCAN_CANCELLED_ERROR`, clears its
scanning state, marks the scan cancelled, and leaves both memory and SQLite
share-file rows unchanged.

The focused regression and all 576 daemon library tests pass. This is a test-
only change with no release-note fragment. Retained live shutdown-overlap
evidence remains external; the Batch 21 full release gate remains the latest
complete gate.

## Continuation Evidence — pod/channel recovery and append ordering (2026-09-24)

RF-002 pod updates and deletions persist `pods.json` before pruning
`pod-channel-messages.json`. If channel cleanup fails during a request, the
parent snapshot is restored; if the process stops between writes, startup
removes messages whose pod or channel no longer exists. API message posting,
incoming peer messages, and room-bridge messages take the channel lock before
rechecking their pod/channel state, so an append queued behind removal cannot
recreate an orphan.

Focused regressions prove parent rollback on cleanup failure, startup recovery
after interrupted update and delete writes, and rejection of an append queued
behind channel removal. The full daemon library suite passes all 579 tests;
pod management and channel lifecycle frozen-controller differentials pass, and
`cargo check --locked -p slskr --lib --features "full-controller-tests
legacy-route-dispatch"` passes. The operational correction has a validated
fragment at `release-notes/20260924-pod-channel-crash-recovery.md`. The Batch
21 full release gate remains the latest complete gate. Remaining RF-002 work
is review of other paired-store routes.

## Continuation Evidence — room/pod membership lock ordering (2026-09-24)

Room/pod membership request creation, direct leave, and join/leave acceptance
now use a consistent room-then-workflow lock order. Acceptance waits for room
state before taking the pending-request lock, rechecks acceptor permission
after the wait, and updates the room membership plus workflow role in one
turn. This keeps pending-request reads available while room state is busy and
prevents revoked acceptors from consuming a pending request.

A focused production-route regression holds the room store, queues acceptance,
reads pending requests while acceptance waits, then revokes the acceptor and
verifies that the pending request remains and no member is added. The
membership-workflow frozen-controller differential and all 580 daemon library
tests pass. The `full-controller-tests legacy-route-dispatch` feature compile
passes, and the behavior change has a validated fragment at
`release-notes/20260924-pod-membership-authorization-order.md`. Remaining
RF-002 work is a review of paired persistence routes outside these pod and
membership paths, especially wishlist ignored-result and search-snapshot
commit ordering.

## Continuation Evidence — wishlist ignored-result ordering (2026-09-24)

RF-002 now serializes ignored-rule create/delete and Lidarr automatic reject
rules with HTTP and inbound Soulseek search-result writes. The dedicated turn
covers reading the current ignored rules, mutating the wishlist/search stores,
and committing the paired SQLite writes. Store write guards are released
before database I/O. A deterministic route regression queues a matching
search response before ignore-rule creation and confirms both the in-memory
search and persisted result rows finish suppressed.

The focused regression, the wishlist controller differential, all 581 daemon
library tests, the `full-controller-tests legacy-route-dispatch` feature
compile, and `scripts/check-rust-format.sh` pass. The user-visible correction
has a validated fragment at
`release-notes/20260924-wishlist-ignore-order.md`. Search completion/status/
delete snapshots and other paired persistence routes remain to be reviewed;
the Batch 21 full release gate remains the latest complete gate.

## Continuation Evidence — wishlist search snapshot ordering (2026-09-24)

RF-002 now orders all search database writes through one persistence turn.
Snapshot, transition, result-delta, and deletion helpers re-read the current
SearchStore after acquiring the turn, so a stale completion/status snapshot
cannot replace a newer result-suppressed record. Result deltas persist only
entries that remain in the current record. Search-history clearing acquires the
turn before mutating the in-memory store. Ignored-rule creation and suppression
hold the same turn through their paired SQLite transaction.

A focused regression queues ignore creation before a completed-search snapshot
and verifies the status update survives while both memory and persisted result
rows remain suppressed. Both wishlist ignored-result regressions, the wishlist
controller differential, the ignored-result persistence differential, all
582 daemon library tests, the `full-controller-tests legacy-route-dispatch`
feature compile, and `scripts/check-rust-format.sh` pass. The validated
user-facing fragment is
`release-notes/20260924-wishlist-ignore-order.md`. Other paired persistence
routes remain open under RF-002; the Batch 21 full release gate remains the
latest complete gate.

## Continuation Evidence — wishlist item mutation ordering (2026-09-24)

RF-002 now serializes all persisted wishlist item mutations with ignored-rule
changes and wishlist search-result writes. Both route dispatchers and the
background sync, search-completion, and auto-download paths acquire the shared
turn before mutating the store and keep it through the corresponding SQLite
write; the store write guard is released before I/O. This prevents a delayed
item upsert from restoring a row after a newer delete.

The focused queued update/delete regression confirms a read remains available
while both writes wait and that memory and SQLite finish without the deleted
row. The full 583-test daemon library suite, wishlist controller differential,
ignored-result persistence differential, legacy/full-controller feature
compile, and Rust formatter pass. The user-facing behavior is covered by
`release-notes/20260924-wishlist-ignore-order.md`. Other paired persistence
routes remain open under RF-002; Batch 21 remains the latest complete release
gate.

## Continuation Evidence — library item mutation ordering (2026-09-24)

RF-002 now serializes all persisted library-item mutations. Create/delete,
health issue patch/fix, MusicBrainz target creation, and Lidarr manual import
acquire a dedicated library turn before changing the store; manual import
acquires library before runtime and keeps both turns through its atomic SQLite
write. Store guards are released before SQLite work.

The focused route regression queues a health-repair snapshot before deletion,
confirms a library read remains available during the wait, and verifies that
the final in-memory store and database contain no deleted item. The 584-test
daemon suite, library issue-fix and residual controller differentials,
manual-import rollback test, legacy/full-controller feature compile, and Rust
formatter pass. The residual test fixture now browses the directory containing
its sample file. The user-visible persistence correction has a validated note
at `release-notes/20260924-library-item-persistence-order.md`. Other paired
persistence routes remain open under RF-002; Batch 21 remains the latest full
release gate.

## Continuation Evidence — contact mutation ordering (2026-09-24)

RF-002 now serializes persisted contact creation through the direct,
discovery, and invite routes, along with contact updates and deletes, in both
dispatchers. Each route takes a contact persistence turn before mutating the
store and keeps it through SQLite persistence while releasing contact store
guards before I/O. This prevents a delayed update from restoring a row after a
newer delete.

The focused queued update/delete regression confirms a contact read remains
available during the wait and that both memory and SQLite finish without the
deleted row. The 585-test daemon suite, contact CRUD/concurrency and
persistence-failure controller differentials, discovery/read differential,
legacy/full-controller feature compile, and changed-file formatter pass. The
user-visible persistence correction has a validated note at
`release-notes/20260924-contact-persistence-order.md`. Other paired persistence
routes remain open under RF-002; Batch 21 remains the latest complete release
gate.

## Continuation Evidence — user-note mutation ordering (2026-09-24)

RF-002 now serializes persisted user-note creation, versioned creation,
updates, and deletes in both dispatchers. Each route acquires the user-note
persistence turn before changing memory and keeps it through SQLite writes;
the note store guard is released before I/O. A delayed update can no longer
restore a note after a newer delete.

The focused queued update/delete regression confirms reads remain available
while both writes wait and verifies that neither memory nor SQLite retains the
deleted note. The 586-test daemon suite, user-note lifecycle and persistence-
failure controller differentials, legacy/full-controller feature compile, and
changed-file formatter pass. The user-facing correction has a validated note
at `release-notes/20260924-user-note-persistence-order.md`. Interest persistence
was left for a separate RF-002 slice because liked/hated routes and the
mesh-rendezvous compatibility mutation share the store; other paired
persistence routes remain open. Batch 21 remains the latest complete release
gate.

## Continuation Evidence — liked/hated interest mutation ordering (2026-09-24)

RF-002 now orders persisted liked and hated interest changes through one
turn. Both HTTP route dispatchers and the mesh-rendezvous compatibility
mutation acquire it before changing memory and hold it through SQLite writes.
Versioned routes release the turn after the commit and before sending outbound
Soulseek interest commands.

The focused regression queues a liked-interest creation and deletion followed
by a hated-interest creation, verifies liked/hated reads stay available while
all three wait, and confirms memory and SQLite retain only the final hated
interest. A failure-injection regression verifies that compatibility create and
delete restore memory when SQLite is unavailable. The 588-test daemon suite,
persistence-failure controller differential, versioned interest wire-command
test, legacy/full-controller feature compile, and changed-file formatter pass. The user-visible correction
has a validated note at
`release-notes/20260924-interest-persistence-order.md`. Other paired persistence
routes remain open under RF-002; Batch 21 remains the latest complete release
gate.

## Continuation Evidence — now-playing mutation ordering (2026-09-24)

RF-002 now serializes persisted now-playing updates, listening-party radio
projections, and clears in both dispatchers. Each mutation acquires the
now-playing persistence turn before changing that store and keeps it through
SQLite work; the store guard is released before I/O.

The focused queued update/clear regression confirms reads remain available
while both mutations wait, then verifies memory and SQLite finish without the
cleared track. The 589-test daemon suite, now-playing persistence-failure
differential, rollback regression, delete/diagnostic and native webhook
differentials, legacy/full-controller feature compile, and changed-file
formatter pass. The user-visible correction has a validated note at
`release-notes/20260924-nowplaying-persistence-order.md`. Other paired
persistence routes remain open under RF-002; Batch 21 remains the latest
complete release gate.

## Continuation Evidence — webhook mutation ordering (2026-09-24)

RF-002 now serializes persisted webhook registration, active-state changes,
and deletion across direct and admin routes in both dispatchers. Each route
acquires the webhook persistence turn before mutating the manager and keeps it
through SQLite writes; the webhook manager guard is released before I/O.

The focused queued PATCH/delete regression confirms a GET remains available
while both writes wait and verifies that memory and SQLite finish without the
deleted webhook. The 590-test daemon suite, webhook persistence/rehydration and
dispatch-log regression, registration, exact-path, and PATCH contract tests,
legacy/full-controller feature compile, and changed-file formatter pass. The
user-visible correction has a validated note at
`release-notes/20260924-webhook-persistence-order.md`. Other paired persistence
routes remain open under RF-002; Batch 21 remains the latest complete release
gate.

## Continuation Evidence — security-ban mutation ordering (2026-09-24)

RF-002 now serializes persisted username/IP ban and unban operations across
both security route dispatchers and the overlay blocklist compatibility
mutation. Legacy security routes and the compatibility mutation release the
security store guard before SQLite. Failed compatibility create/delete writes
restore the prior in-memory bans. The automatic violation tracker also writes
the shared ban list, but it does not persist; no non-route ban persistence
writer was found.

The focused queued security-ban/overlay-delete regression confirms security
reads remain available while both mutations wait and verifies that memory and
SQLite finish without the deleted ban. Compatibility create/delete failure
injection also confirms memory rollback. The 592-test daemon library suite,
security-ban controller differential, legacy/full-controller feature compile,
changed-file formatter, plan/docs freshness, release-note preview, and diff
check pass. The regression ran through split dispatch; the combined legacy
runtime test overflows the historical monolithic test-worker stack. The
security correction has a validated note at
`release-notes/20260924-security-ban-persistence-order.md`. Other paired
persistence routes remain under RF-002 review; Batch 21 remains the latest
complete release gate.

## Continuation Evidence — message persistence ordering (2026-09-24)

RF-002 now serializes persisted message creates, single/bulk acknowledgements,
and conversation deletes through one `message_persistence_lock` across the
split and legacy route dispatchers, overlay bridge route, and inbound Soulseek
session handlers. Store write guards are dropped before SQLite I/O, and
persistence turns end before event, session, or outbound work. The legacy
compatibility acknowledgement fallback now restores its prior snapshot if
SQLite fails and checks the requested username before changing acknowledgement
state.

A focused queued create/delete regression holds the shared turn while reads
remain responsive, then verifies that memory and SQLite both finish without
the deleted messages. Compatibility tests cover database-failure rollback and
a wrong-username request that must leave the record unchanged. The message
create, inbound persistence-failure, library/interest/now-playing/message
persistence-failure, and conversation-delete frozen-controller checks pass.
The 595-test daemon library suite, `full-controller-tests` plus
`legacy-route-dispatch` feature compile, and changed-file formatter pass. The
user-facing correction has a validated fragment at
`release-notes/20260924-message-persistence-order.md`. Room subscription and
other paired persistence routes remain open; Batch 21 remains the latest full
release gate.

## Continuation Evidence — room subscription ordering (2026-09-24)

RF-002 now serializes room subscription joins and leaves through one
`room_persistence_lock` across route groups 2 and 3, the legacy dispatcher, and
the compatibility mutation handler. Each route acquires the turn before
changing RoomStore, releases the room-store write guard before SQLite, and
drops the persistence turn before event or session work. Legacy and
compatibility paths now use compare-and-rollback if SQLite fails.

The focused queued join/leave regression confirms joined-room reads remain
available during the wait and memory plus SQLite finish without the room
subscription. Room persistence-failure rollback, persisted rehydration, exact
path behavior, room controller residual and restart/failure differentials, the
596-test daemon library suite, legacy/full-controller feature compile, and
changed-file formatter pass. The user-facing correction has a validated
fragment at
`release-notes/20260924-room-subscription-persistence-order.md`. Other paired
persistence routes remain under review; Batch 21 remains the latest full
release gate.

## Continuation Evidence — OAuth state persistence ordering (2026-09-24)

RF-002 now serializes Spotify OAuth state issuance and callback consumption
through one `oauth_persistence_lock` across both dispatchers. Authorization
state is issued under the turn and rolled back if its insert fails. Callback
consumption validates under a brief state-store write guard, releases that
guard before SQLite deletion, then removes memory state after the delete
commits. A database failure leaves the one-time state available for retry.

The focused concurrent-consumer regression confirms reads remain available
while both callbacks wait and that exactly one consumes the token in memory
and SQLite. OAuth persistence creation/deletion failure tests, state
rehydration, the Spotify authorize/callback differential, the 597-test daemon
library suite, legacy/full-controller feature compile, and changed-file
formatter pass. The security-sensitive behavior has a validated note at
`release-notes/20260924-oauth-state-persistence-order.md`. Other paired
persistence routes remain open; Batch 21 remains the latest full release gate.

## Continuation Evidence — HashDb history-backfill cursor ordering (2026-09-24)

RF-002 now serializes history-backfill cursor reads and advancement with the
HashDb persistence turn. The route holds the turn while it reads the saved
cursor, selects the bounded candidate batch, updates in-memory progress, and
persists the next cursor. This prevents overlapping requests from reading the
same position and overwriting each other's progress.

The focused two-request regression confirms sequential batches, responsive
candidate reads while queued, and final SQLite cursor consistency. The existing
history-backfill controller contract, all 605 daemon library tests, the
`full-controller-tests legacy-route-dispatch` feature check, and changed-file
Rust formatter pass. The user-visible correction has a validated fragment at
`release-notes/20260924-hashdb-history-backfill-order.md`. Other paired
persistence routes remain open under RF-002; Batch 21 remains the latest full
release gate.

## Continuation Evidence — manual transfer cleanup ordering (2026-09-24)

RF-002 manual database cleanup now deletes terminal transfer rows using the
revision-aware tombstone helper after removing them from the live queue. If
SQLite rejects the deletion, the queue snapshot is restored when no later
transfer mutation has replaced it, and cleanup reports an error.

Focused success and failure regressions verify live/SQLite agreement, retained
id tombstones, and rollback on a closed database. The existing database-cleanup
contract passes under the split dispatcher, all 607 daemon library tests pass,
and the changed-file formatter passes. The retained monolithic cleanup contract
compiles but overflows its historical Tokio test-worker stack. The user-facing
correction has a validated fragment at
`release-notes/20260924-transfer-database-cleanup-order.md`. Other paired
persistence routes remain open under RF-002; Batch 21 remains the latest full
release gate.

## Continuation Evidence — age-based message cleanup ordering (2026-09-24)

RF-002 database cleanup now calculates its age cutoff once and uses the same
value to prune `MessageStore` and persisted message rows. The message
persistence turn is acquired before the live projection changes and held
through SQLite deletion; the store guard is released during I/O. If SQLite
cleanup fails, the message snapshot is restored and the endpoint reports
unhealthy persistence.

A queued regression confirms message API reads remain responsive and both
memory and SQLite keep the expired record until the persistence turn is
released, then remove it together. A closed-database regression verifies
message rollback. All 608 daemon library tests and the changed-file formatter
pass. The user-facing correction has a validated fragment at
`release-notes/20260924-message-database-cleanup-order.md`. Other paired
persistence routes remain open under RF-002; Batch 21 remains the latest full
release gate.

## Continuation Evidence — monotonic split-router transfer revisions (2026-09-24)

The split transfer router now advances `updated_at_ms` from the current
per-transfer revision for request-name, cancellation, retry, progress, and
completion mutations, matching the retained monolithic dispatcher. This keeps
same-millisecond or clock-regressed updates newer than the row they replace,
so SQLite accepts the current state and rejects delayed older snapshots.

The focused request-name regression seeds memory and SQLite with a revision
ahead of wall-clock time, sends a rename through the split router, and confirms
both projections store the new name and the advanced revision. The focused
regression and 609-test default daemon suite pass. The targeted test also
passes with `full-controller-tests legacy-route-dispatch` enabled when it
directly exercises the split dispatcher; entering through the monolithic
wrapper overflows its historical Tokio worker stack, a limitation also seen
in broader transfer tests. A validated fragment is at
`release-notes/20260924-transfer-api-revision-order.md`. Other paired
persistence routes remain open under RF-002; Batch 21 remains the latest full
release gate.

## Continuation Evidence — watched share reload ordering (2026-09-24)

Share-setting reload and runtime share changes now use a dedicated turn that
serializes them with the share index's final SQLite replacement and live
publication. Each scan captures the current settings generation. If a reload
or runtime mutation advances the generation while a scan is running, the scan
is rejected before persistence or publication, leaves `scan_pending` set, and
reports the normal cancellation result. Reload also signals the active scan
token so filesystem traversal stops promptly.

The focused regression registers an active cancellation token, applies a
watched share-directory change, and attempts to commit a snapshot from the old
generation. It verifies the token is cancelled, SQLite remains unchanged, the
new root stays projected, and the lifecycle remains pending/cancelled. The
focused test and all 610 default daemon tests pass; the combined
`full-controller-tests legacy-route-dispatch` target also passes. The
changed-file formatter, plan/docs freshness checks, and release-note preview
pass. A validated fragment is at
`release-notes/20260924-share-scan-reload-order.md`; retained live shutdown
evidence remains external.

## Continuation Evidence — transfer payload flush measurement (2026-09-24)

The `slskr-client` benchmark target compares the previous payload writer
(per-chunk `write_all` followed by `flush`) with the current writer over real
loopback TCP sockets. The receiver decodes obfuscated frames and validates all
payload bytes for both variants. Two optimized runs each measured five 64 MiB
trials per variant with 80 KiB chunks. Plain median deltas were +0.27% and
-0.12%; obfuscated median deltas were +0.43% and +0.06%. Before/after sample
ranges overlap in both runs, so these measurements show no reproducible gain.
The production type is Tokio 1.53.1 `TcpStream`, whose pinned implementation
makes `flush` a no-op; no socket syscall reduction is established. The complete
artifacts are
`crates/slskr-client/benchmarks/artifacts/20260924-file-transfer-loopback.json`
and
`crates/slskr-client/benchmarks/artifacts/20260924-file-transfer-loopback-repeat.json`.
The run used Rust 1.94.0 on Linux x86_64 with an AMD Ryzen 9 9950X3D; the
results are diagnostic local-loopback evidence, not a remote-peer throughput
claim. Confidence in the local conclusion is moderate. Remote-peer measurement
and the other RF-020-RF-023 benchmark gaps remain open.

## Continuation Evidence — SQLite index cardinality (2026-09-24)

The synthetic harness uses the production table columns and page projections
with 1,000,000 search results, 500,000 transfers, and 500,000 webhook logs. It
compares the existing single-column indexes, all three original RF-021
composites, and the retained transfer/webhook pair. On Python SQLite 3.53.4,
the retained pair changes both query plans and reduces first/deep page medians
by 88–99%. The daemon bundles SQLite 3.46.0, and the generated data does not
represent a production database, so live cardinality evidence remains open.

The search-result composite is redundant: both before and after plans select
`idx_search_results_search` without a temporary sort, and medians are unchanged.
Since `id` is an `INTEGER PRIMARY KEY`, the single-column index already carries
rowid order. The extra composite added 24,059,904 bytes in the fixture. Startup
now removes it, and a file-backed migration regression checks that the old
index disappears while the existing single-column index remains. The full
daemon library suite passes (611 tests). See
`benchmarks/artifacts/20260924-sqlite-cardinality.md` and its JSON artifact.

## Continuation Evidence — runtime compatibility persistence lock scope (2026-09-24)

Runtime compatibility mutations now snapshot their previous and resulting
state under the runtime/relay write locks, release those guards before the
SQLite mirror write, and retain the shared persistence turn through commit. On
write failure, rollback restores only fields still equal to the failed
mutation, preserving concurrent changes to other runtime fields. Operational
restart and bridge-running latches use the same persistence turn but stay
process-local: durable snapshots write both fields as false, and hydration
ignores older persisted values so a completed restart cannot resurrect them.

A file-backed regression holds SQLite's writer lock, verifies runtime reads and
an unrelated runtime update remain available while persistence waits, then
forces an insert-trigger failure and checks that the failed counter rolls back
without losing the unrelated update. `cargo test --locked -p slskr --lib
runtime_compatibility_ --quiet` passes; the full default daemon library suite
passes 613 tests. The full-controller feature compile, split runtime
persistence-failure differential, runtime compatibility rehydration, and
process-local latch regression pass. Both frozen share-scan flags differentials
pass. Running the persistence-failure differential with `legacy-route-dispatch`
overflows the historical monolithic test-worker stack.
The changed-file formatter, freshness checks, and release-note preview are
recorded with Batch 56 in the active plan. This operational improvement has a
validated fragment at `release-notes/20260924-runtime-compatibility-lock-scope.md`.

## Continuation Evidence — full release gate after Batch 59 (2026-09-24)

The complete `scripts/run-release-gate.sh` passed after the managed HTTP
listener change, task-registry module extraction, and Clippy cleanup. Locked
workspace tests, all-target Clippy, the wasm web check, Rust package and AUR
smokes, security scans, 857 Web tests, 38 dashboard tests, production builds,
and bundle budgets passed. The Web count is across 146 files; the dashboard
count is across 11 files. `actionlint` was unavailable and skipped, and the
opt-in live slskd API smoke was skipped by local policy. Clean hosted
GitLab/Windows/Chocolatey runner results and retained live shutdown/parity
artifacts remain external evidence.

## Continuation Evidence — runtime compatibility state module (2026-09-24)

Batch 60 moves the `RuntimeCompatState` definition and its persistence and
lifecycle implementation into `crates/slskr/src/runtime_compat_state.rs`.
The crate-root import preserves existing call sites, and the runtime-boundary
guard now scans the extracted module. The locked `full-controller-tests`
feature compile, 614-test daemon library suite, runtime-boundary guard,
changed-file formatter, and `git diff --check` pass. This is an internal-only
structural move with no behavior change or release-note fragment. More daemon
and Web ownership splits remain open; at this point, the latest complete release
gate was the one after Batch 59.

## Continuation Evidence — hash backfill state module (2026-09-24)

Batch 61 moves the bounded backfill state store and its related candidate and
pending-transfer records into `crates/slskr/src/hash_backfill_state.rs`.
Persistence bounds, symlink/FIFO-safe loading, atomic writes, and per-peer
daily quota behavior are unchanged. The runtime-boundary guard now scans this
module. The locked `full-controller-tests` feature compile, 614-test daemon
library suite, runtime-boundary guard, changed-file formatter, and
`git diff --check` pass. This is internal-only structural work with no
behavior change or release-note fragment. At this point, the latest complete
release gate was the one after Batch 59.

## Continuation Evidence — source-discovery state module (2026-09-24)

Batch 62 moves the source-discovery generation and token-window state into
`crates/slskr/src/source_discovery_state.rs`. The route and background worker
continue to use the same state operations through the crate-root import. The
locked `full-controller-tests` feature compile, 614-test daemon library suite,
runtime-boundary guard, changed-file formatter, and `git diff --check` pass.
This is internal-only structural work with no behavior change or release-note
fragment. At this point, the latest complete release gate was the one after
Batch 59.

## Continuation Evidence — security state module (2026-09-24)

Batch 63 moves ban/reputation state records and their bounded mutation logic to
`crates/slskr/src/security_state.rs`. The `MAX_SECURITY_BANS` test name remains
available through a feature-gated crate-root import, and the runtime-boundary
guard scans the new module. The locked `full-controller-tests` feature compile,
614-test daemon library suite, runtime-boundary guard, changed-file formatter,
and `git diff --check` pass. This is an internal-only structural move with no
behavior change or release-note fragment. At this point, the latest complete
release gate was the one after Batch 59.

## Continuation Evidence — legacy RF-002 lock scope (2026-09-24)

Batch 64 fixes the retained dispatcher’s share-grant create and Lidarr manual
import paths. Grant creation drops its store write guard before SQLite and
conditionally restores the prior store only if no later mutation changed the
candidate. Manual import drops its library, runtime, and relay guards before
the combined SQLite transaction and conditionally restores unchanged state on
failure. File-backed writer-lock regressions and the existing persistence-
failure rollback regressions pass through the active split dispatcher; the
default daemon library suite passes all 614 tests. The
`full-controller-tests legacy-route-dispatch` feature check passes, but direct
route execution through the retained monolithic dispatcher overflows the
default Tokio test-worker stack, so that path has compile and source-review
evidence only. The runtime-boundary guard, changed-file formatter, and
`git diff --check` pass. This is internal-only concurrency work with no
release-note fragment. At this point, the complete release gate after Batch 59
was the latest full gate; additional paired persistence routes remain under
RF-002.

Batch 65 fixes collection deletion in the retained dispatcher. The handler
releases collection and grant write guards before SQLite and conditionally
restores both snapshots only when neither store changed. The targeted
`collection_delete_` tests pass, including the file-backed blocked-writer
regression, the persistence-failure rollback case, and queued-token behavior.
The `full-controller-tests legacy-route-dispatch` feature compile passes;
direct runtime proof through the monolithic route remains limited by the
default Tokio test-worker stack overflow. The runtime-boundary guard, formatter,
614-test default daemon library suite, plan/docs freshness checks,
release-note preview, and `git diff --check` pass. This internal-only change
has no release-note fragment. At this point, the Batch 59 complete release
gate was the latest full gate, and additional paired persistence routes
remained open.

Batch 66 moves bounded source-feed history, its load/record/persist logic, and
Lidarr sync runtime state into `crates/slskr/src/integration_runtime_state.rs`.
Crate-root imports retain the existing store and field names, while the runtime-
boundary guard now scans the module and keeps source-feed/Lidarr tests anchored.
All four source-feed tests, both Lidarr scheduler tests, the 614-test default
daemon suite, and the `full-controller-tests legacy-route-dispatch` feature
compile pass. The formatter, runtime-boundary guard, and `git diff --check`
pass. This is an internal-only ownership move with no release-note fragment;
at this point, the Batch 59 complete release gate was still the latest full
gate, and additional daemon/Web splits remained under RF-024.

## Continuation Evidence — full release gate after Batch 66 (2026-09-24)

The complete `scripts/run-release-gate.sh` passed after Batch 66. Locked
workspace tests, all-target Clippy, the wasm web check, RustSec, Rust package
verification, AUR smoke, security scans, and Web/dashboard checks passed. The
workspace run included 614 daemon library tests, 24 protocol tests, and 86 Web
crate tests. Web passed 857 tests across 146 files; the dashboard passed 38
tests across 11 files. Python passed 49 tests, TypeScript passed 51, and the Go
client test was cached. Semgrep reported zero findings, and Trivy found no
vulnerabilities in the Cargo, Go, dashboard, or Web lockfiles. Both Web and
dashboard builds passed their bundle budgets.

`actionlint` was unavailable and skipped. The opt-in live slskd API smoke was
skipped by local policy, so retained live parity and clean hosted-runner
evidence remain open. Frozen .NET controller-reference builds emitted
NU1903/NU1902 warnings for SQLitePCLRaw.lib.e_sqlite3 2.1.11 and AngleSharp
1.4.0; the corresponding controller differentials passed. This gate refresh is
verification of internal-only refactor work and adds no release-note fragment.

## Continuation Evidence — security authentication state module (2026-09-24)

Batch 67 moves `RevokedJwtStore` and `LoginAttemptStore` into
`crates/slskr/src/security_state.rs`. Crate-root imports preserve the existing
call sites, state fields, JWT revocation file and expiry behavior, and
five-failure lockout window. Both revocation persistence/reload tests and the
failed-login throttling regression pass. The `full-controller-tests
legacy-route-dispatch` feature compile, all 614 default daemon tests, the
runtime-boundary guard, changed-file formatter, and `git diff --check` pass.
This is an internal-only ownership move with no release-note fragment; RF-024
remains open for more daemon/Web splits. The latest full release gate remains
the one after Batch 66.

## Continuation Evidence — OAuth state module (2026-09-24)

Batch 68 moves the bounded `OAuthStateStore`, persisted-state filtering, and
issue/consume transitions into `crates/slskr/src/oauth_state.rs`. The crate-root
imports preserve route, persistence, and test call sites. All seven
OAuth-state-focused tests pass, including single-use concurrent consumption,
persistence failure, restart hydration, and callback validation. The
`full-controller-tests legacy-route-dispatch` feature compile, all 614 default
daemon tests, runtime-boundary guard, changed-file formatter, and
`git diff --check` pass. This is an internal-only ownership move with no
release-note fragment; more daemon and Web splits remain open under RF-024. The
latest full release gate remains the one after Batch 66.

## Continuation Evidence — Spotify integration state models (2026-09-24)

Batch 69 moves `SpotifyConnectionStore`, `ProtectedSpotifyConnection`, and
Spotify source target/row models into
`crates/slskr/src/integration_runtime_state.rs`. The crate-root imports
preserve encryption, route, parser, and test call sites. All 15 Spotify-focused
tests and four source-feed tests pass, as do the
`full-controller-tests legacy-route-dispatch` feature compile, all 614 default
daemon tests, runtime-boundary guard, changed-file formatter, and
`git diff --check`. This is an internal-only ownership move with no
release-note fragment; more daemon/Web splits remain open under RF-024. The
latest full release gate remains the one after Batch 66.

## Continuation Evidence — Contact and destination state ownership splits (2026-09-24)

Batches 70-71 move contact record/store behavior into
`crates/slskr/src/contact_state.rs` and destination record/store/path-selection
behavior into `crates/slskr/src/destination_state.rs`. Crate-root imports
preserve production call sites; controller-test destination fixtures now name
their owner module directly. The runtime-boundary guard scans both new modules.
The shared-listener fixture also confirms the dual-valid RF-001 framing case
described in the table above.

The locked `cargo check --tests --features "full-controller-tests
legacy-route-dispatch"` target compile, all 614 default daemon library tests,
all 27 listener integration tests, runtime-boundary guard, changed-file
formatter, plan/docs freshness, and `git diff --check` pass. These are
internal-only changes with no release-note fragment. The full release gate
after Batch 66 remains the latest complete gate.

## Continuation Evidence — controller feature state ownership split (2026-09-24)

Batch 72 moves `ControllerFeatureState` and its persisted file envelope into
`crates/slskr/src/controller_feature_state.rs`. The crate-root import preserves
existing route and test call sites; the runtime-boundary guard now includes the
new module. The JSON schema, 4,096-record and 8 MiB caps, atomic replacement,
and rollback-on-write-failure behavior are unchanged.

The locked `cargo check --locked -p slskr --lib --features "full-controller-tests
legacy-route-dispatch"` and corresponding `--tests` target compile, all 614
default daemon library tests, `scripts/check-runtime-boundary-hardening.sh`,
`scripts/check-rust-format.sh`, and `git diff --check` pass.

The complete `scripts/run-release-gate.sh` passed after Batch 71 on
2026-09-24, including workspace tests, Clippy, WASM, RustSec, package and AUR
smokes, Web/dashboard checks, and security scans. The Web suite passed 857
tests across 146 files, and Trivy reported no Cargo, Go, dashboard, or Web
lockfile vulnerabilities or secrets. `actionlint` was unavailable; the
opt-in live slskd API smoke was skipped by local policy. Frozen .NET reference
builds reported the existing NU1903/NU1902 dependency warnings while their
differentials passed. Hosted runner and retained live parity evidence remain
external. Batch 72 is internal-only and adds no release-note fragment.

## Continuation Evidence — preview ticket ownership split (2026-09-24)

Batch 73 moves `PreviewStreamTicket` and `PreviewStreamTicketStore` into
`crates/slskr/src/preview_stream_state.rs`. Root imports preserve production
and controller-test call sites; ticket fields, capacity, expiry, and
source-revocation behavior are unchanged. The runtime-boundary guard includes
the module.

The locked `cargo check --locked -p slskr --tests --features "full-controller-tests
legacy-route-dispatch"` compile, all 614 default daemon library tests,
`scripts/check-runtime-boundary-hardening.sh`, `scripts/check-rust-format.sh`,
and `git diff --check` pass. This internal-only ownership move adds no
release-note fragment. The full release gate after Batch 71 remains the latest
complete local gate.

## Continuation Evidence — listening-party limiter ownership split (2026-09-24)

Batch 74 moves `ListeningPartyStreamLimits` and
`ListeningPartyStreamLimitRejection` to
`crates/slskr/src/listening_party_stream_state.rs`. The crate-root import
preserves AppState, route, and test call sites. Permit ordering, party and IP
caps, capacity rejection, and release of the party permit after an IP rejection
are unchanged. The runtime-boundary guard includes the new module.

The locked `cargo check --locked -p slskr --tests --features "full-controller-tests
legacy-route-dispatch"` compile, all 614 default daemon library tests,
`scripts/check-runtime-boundary-hardening.sh`, `scripts/check-rust-format.sh`,
and `git diff --check` pass. This internal-only ownership move adds no
release-note fragment. The full release gate after Batch 71 remains the latest
complete local gate.

## Continuation Evidence — targeted RF-002 write-scope review (2026-09-24)

A targeted static pass over representative route-group 0, 1, 2, 4, and 6
paths and webhook delivery-stat writes found their state guards scoped before
the awaited persistence/network operations, or a separate persistence-turn
mutex used to serialize the workflow. This did not prove the remaining RF-002
route inventory complete; its paired-persistence review stays open.

The scan also found a separate concrete synchronous write path:
`ControllerFeatureState::upsert`, `remove`, and bulk mutations serialize the
bounded JSON record map and call `write_file_atomic` while the AppState Tokio
write guard is held. The map is capped at 4,096 records and 8 MiB, while
`write_file_atomic` syncs the temporary file and parent directory. This can
block a Tokio worker and delays reads of this store until sync completes. No
latency measurement was collected. Any offload must preserve serialized write
order, cancellation behavior, and in-memory rollback on failure, so this is a
follow-up to investigate, not a completed RF-002 fix. The finding is
internal-only and adds no release-note fragment.

## Continuation Evidence — shadow/realm index merge rollback (2026-09-24)

The shadow-index sync handler writes content-discovery shadow records and
realm-subject indexes to separate state files. If the realm-index write fails,
the handler now restores and persists the previous shadow records, returns
`503 Service Unavailable`, and updates session error state only after dropping
both store write guards. Both split and retained handlers implement the same
behavior. A file-backed split-dispatch regression forces the realm-index write
to fail and verifies that memory and the content-discovery file retain the
pre-request state. The focused regression, all 619 default daemon library
tests, locked full-controller/legacy library check, changed-file formatter,
and scoped `-D clippy::await_holding_lock` pass; remaining paired persistence
routes stay open under RF-002. The operational change has a validated fragment at
`release-notes/20260924-shadow-index-merge-rollback.md`.

## Continuation Evidence — source-feed history ordering (2026-09-24)

Source-feed previews now use one persistence turn for Spotify, configured
provider, and local-text history updates in both split and legacy dispatch.
The store write guard is released before file persistence; write failures roll
back the candidate history. A deterministic queued-writer regression confirms
read availability, FIFO persisted history order, memory/file agreement after
reopen, and unchanged state after a forced persistence failure. The focused
regression, all 620 default daemon library tests, locked
`full-controller-tests legacy-route-dispatch` library check, scoped
`-D clippy::await_holding_lock` Clippy check, changed-file formatter, and
`HEAD`-to-`WORKTREE` release-note preview pass. Other paired persistence routes
remain open under RF-002. The correction has a validated fragment at
`release-notes/20260924-source-feed-history-order.md`.

## Continuation Evidence — server-session lock release (2026-09-24)

The split and retained handlers for `GET /api/server`, connect, and disconnect
now clone the session snapshot and drop its read/write guard before awaiting
runtime-credential state. This removes an async lock dependency between session
updates and credential reads. A deterministic regression holds the credential
write guard, polls all four response paths, and verifies the session write lock
remains available until the credential guard is released. The focused test,
all 621 default daemon library tests, locked
`full-controller-tests legacy-route-dispatch` library check, scoped
`-D clippy::await_holding_lock` Clippy check, changed-file formatter, and
`HEAD`-to-`WORKTREE` release-note preview pass. RF-002 remains open for review
of other paired persistence routes. The operator correction has a validated
fragment at `release-notes/20260924-session-status-lock-scope.md`.

## Continuation Evidence — transfer-batch lock release (2026-09-24)

`controller_enqueue_download_batch` now prepares paths without the transfer
lock and stages eligible transfers together in one short write-lock turn. The
commit rechecks active duplicates so requests queued during path resolution
cannot create duplicate entries. A deterministic held-destinations regression
proves transfer writes remain available during preparation, then inserts a
competing transfer before commit and confirms the second entry is rejected. The
focused regression and all 622 daemon library tests pass. The targeted
full-controller batch differential compiles but overflows its historical
test-worker stack before assertions; the default-suite batch route contract
test passes. The locked
`full-controller-tests legacy-route-dispatch` library check, scoped
`-D clippy::await_holding_lock` Clippy check, changed-file formatter,
plan/docs freshness, diff, and release-note preview pass. RF-002 remains in
progress while other paired persistence routes are reviewed. The validated
operator fragment is
`release-notes/20260924-transfer-batch-lock-scope.md`.

## Continuation Evidence — legacy transfer-array lock release (2026-09-24)

Legacy `POST /api/transfers` now resolves all destination paths before taking
the transfer write guard, stages the prepared entries together in one short
write turn, and persists after releasing the guard. If persistence fails, the
queue is conditionally restored only when no concurrent update has superseded
the staged snapshot. A deterministic held-destinations route regression proves
transfer writes remain available during path resolution and confirms the
queued response after the destination guard is released.

The focused regression and all 623 default daemon library tests pass. The
locked `full-controller-tests legacy-route-dispatch` library check, scoped
`-D clippy::await_holding_lock` Clippy check, changed-file formatter, plan/docs
freshness checks, diff check, and `HEAD`-to-`WORKTREE` release-note preview
pass. RF-002 remains in progress while other paired persistence routes are
reviewed. The validated operator fragment is
`release-notes/20260924-legacy-transfer-enqueue-lock-scope.md`.

## Continuation Evidence — legacy per-user transfer enqueue lock release (2026-09-24)

The legacy `/api/v0/transfers/downloads/{username}` handler also prepared
destination paths while holding the transfer write guard. It now prepares the
full request first, stages entries in one short write turn, then persists after
releasing the guard. Persistence failure conditionally restores the previous
queue snapshot only when no concurrent queue update has superseded it. A
held-destinations route regression proves a transfer write remains available
during path resolution and checks the queued response after the destination
guard is released.

The focused regression and all 624 default daemon library tests pass. The
locked `full-controller-tests legacy-route-dispatch` library check, scoped
`-D clippy::await_holding_lock` Clippy check, changed-file formatter, plan/docs
freshness checks, diff check, and `HEAD`-to-`WORKTREE` release-note preview
pass. RF-002 remains in progress while other paired persistence routes are
reviewed. The validated operator fragment is
`release-notes/20260924-per-user-transfer-enqueue-lock-scope.md`.

## Continuation Evidence — legacy per-user transfer enqueue lock release (2026-09-24)

The legacy `/api/v0/transfers/downloads/{username}` handler also prepared
destination paths while holding the transfer write guard. It now prepares the
full request first, stages entries in one short write turn, then persists after
releasing the guard. Persistence failure conditionally restores the previous
queue snapshot only when no concurrent queue update has superseded it. A
held-destinations route regression proves a transfer write remains available
during path resolution and checks the queued response after the destination
guard is released.

The focused regression and all 624 default daemon library tests pass. The
locked `full-controller-tests legacy-route-dispatch` library check, scoped
`-D clippy::await_holding_lock` Clippy check, changed-file formatter, plan/docs
freshness checks, diff check, and `HEAD`-to-`WORKTREE` release-note preview
pass. RF-002 remains in progress while other paired persistence routes are
reviewed. The validated operator fragment is
`release-notes/20260924-per-user-transfer-enqueue-lock-scope.md`.

## Corrected Or Withdrawn Findings

The initial audit hypothesis that `packaging/aur/PKGBUILD` used the wrong
GitHub archive root was withdrawn. The current upstream raw PKGBUILD confirms
that the `slskR-release-v<version>` root is intentional and matches the
repository's archive naming. The independent missing `makepkg` smoke remains
RF-010. The initial Homebrew target concern was also withdrawn: the
`homebrew-slskdn` target is the existing compatibility tap; only the channel
documentation name is stale (RF-029).

## Existing Work That Should Not Be Reopened

The following are verified foundations from the prior plan and memory pages:

- Real-response HTTP benchmarking and artifact comparison.
- Bounded persistence writes on the already migrated bulk paths.
- Shared event/projection ownership and shared Web polling.
- Typed route dispatch context and physical route-group source splits.
- System/MediaCore lazy loading and bundle-budget enforcement.
- Changed-file Rust formatting and serialized Cargo build policy.

These foundations are inputs to the next batches. The previously unbatched
RF-004 search persistence helper now has bounded writes, metadata preservation,
and rollback coverage; restart recovery proof remains in the active plan.

## Execution Rules

1. Fix gate/release correctness before invasive source movement.
2. Fix data loss, lock scope, cancellation, and protocol classification before
   optimizing query plans or bundle shape.
3. Every code batch gets focused tests first, then full affected gates.
4. Every user-facing, operational, security, or documentation behavior change
   gets a validated fragment under `release-notes/`.
5. Do not add caches, parallel stats, routing hash maps, GraphQL, or dependency
   replacements without before/after evidence.
6. Keep the plan status current: `Open`, `In progress`, `Blocked`, `Verified`,
   or `Withdrawn` with the validating command recorded.

## RF-001 Compatibility Boundary Review (2026-09-24)

The dual-valid shared-listener fixture remains a real ambiguity. In the native
current-parity profile, configuration defaults to obfuscation enabled with a
zero listen-port sentinel, no dedicated bind, and the regular port advertised
for both forms. The listener therefore uses the shared demultiplexer. The
configuration already supports an explicit dedicated obfuscated bind/port;
the frozen native profile also selects its historical adjacent dedicated port.
Those modes use a separate advertised port and the dedicated obfuscated
demultiplexer. This means a separate-port policy exists, but choosing it by
default would change the current-parity endpoint behavior.

The related [slskdN Type-1 design note](https://github.com/snapetech/slskdN/blob/main/docs/soulseek-type1-obfuscation.md)
describes the same shared-port default and a nonzero dedicated-listener option.
The fixture proves that local parsing cannot tell sender intent in the
dual-valid case. RF-001 remains in progress pending an explicit compatibility
decision to retain the current shared default or change the advertised port;
no parser-only change can resolve the collision while preserving the current
wire behavior. This is internal audit evidence and does not change release
behavior.


## RF-024 Pod Membership Ownership Split (Batch 88, 2026-09-24)

Moved the pod join/leave request and acceptance DTOs, field validation,
canonical payload construction, bounded pending-membership store, and
signature verification from `lib.rs` into `pod_membership_workflow.rs`. The
parent imports preserve existing callers. The feature-enabled frozen
canonical-signature regression, 624-test default daemon suite, locked default
library check, runtime-boundary guard, formatter, and diff check pass. The move
is internal-only; RF-024 remains in progress and the latest full release gate
remains after Batch 71.


## RF-024 Pod Join Replay Store (Batch 89, 2026-09-24)

Moved the bounded join-nonce replay cache and its reserve, release, and expiry
logic into the typed `PodJoinReplayStore` in `pod_membership_workflow.rs`. Both
legacy and split route dispatchers now use the store. Its 4,096-entry limit,
300-second TTL, key serialization, strict expiry boundary, and existing errors
are preserved. Three focused unit tests and the 627-test default daemon suite
pass, along with the locked full-controller/legacy library check, scoped
`await_holding_lock` Clippy, runtime-boundary guard, formatter, release-note
preview, and diff check. The existing route-level replay test still overflows
the historical Tokio worker stack before assertions. The move is internal-only;
RF-024 remains in progress and the latest full release gate remains after Batch
71.


## RF-024 Pod Membership Route Parsing (Batch 90, 2026-09-24)

Moved pending join/leave request path parsing, blank-ID detection, and
cancellation path parsing into `pod_membership_workflow.rs`; both dispatchers
use the imported helpers with unchanged route behavior. Focused tests verify
percent-decoded identifiers, blank IDs, and rejection of surplus segments. The
six module tests, 630-test daemon suite, locked full-controller/legacy library
check, runtime-boundary guard, formatter, and diff check pass. The move is
internal-only; RF-024 remains in progress and the latest full release gate is
after Batch 71.


## RF-024 Pod Acceptor Authorization (Batch 91, 2026-09-24)

Moved the join/leave acceptor permission helper into
`pod_membership_workflow.rs`. Its workflow-role check and operated-room fallback
remain unchanged, and the workflow guard is released before reading room state.
Both dispatcher feature configurations compile; the 630-test default daemon
suite, scoped `await_holding_lock` Clippy, runtime-boundary guard, formatter,
and diff check pass. This internal ownership move needs no release-note
fragment. RF-024 remains in progress; the latest complete release gate remains
after Batch 71.

## RF-024 MusicBrainz Lookup Ownership (Batch 92, 2026-09-24)

Moved PodCore content-ID parsing, MusicBrainz request/search helpers, and
PodCore metadata shaping into `musicbrainz_lookup.rs`. Root imports preserve
route and `route_dispatch` call sites. Three focused parser/fallback tests, the
mocked MusicBrainz release lookup test under the full-controller/legacy
dispatch features, and all 633 default daemon tests pass. The runtime-boundary
guard, formatter, and diff check pass. This is internal-only; RF-024 remains in
progress and the latest complete release gate remains after Batch 71.


## RF-024 Managed Blacklist Runtime (Batch 99, 2026-09-24)

Moved managed blacklist runtime state and operations into
`managed_blacklist_runtime.rs`; the crate-root import preserves existing
callers. The 10-minute cache TTL, 1,000-entry limit, matching behavior, and
cache invalidation on replacement are unchanged. All 638 default daemon tests
and seven feature-enabled blacklist regressions pass, along with locked
default and feature library checks, scoped `await_holding_lock` Clippy, the
runtime-boundary guard, changed-file formatter, plan freshness, release-note
preview, and diff check. This is internal-only; RF-024 remains in progress and
the latest complete release gate remains after Batch 71.


## RF-024 Controller Regex Ownership (Batch 100, 2026-09-24)

Moved shared controller regex matching and compilation into
`controller_regex.rs`; crate-root imports preserve search, blacklist, and test
callers. The native 250 ms match timeout and request-compilation worker stack
remain unchanged. The 638-test default suite and 14 feature-enabled regex
regressions pass, along with locked default and feature library checks, scoped
`await_holding_lock` Clippy, runtime-boundary guard, formatter, plan freshness,
release-note preview, and diff check. This internal-only move has no release
note; RF-024 remains in progress and the latest full gate remains after Batch
71.


## RF-024 Private Message Auto-Response Classifier (Batch 101, 2026-09-24)

Moved private-message auto-response candidate classification beside its
bounded cooldown tracker in `private_message_auto_responses.rs`; the shared
message-body limit remains owned at the crate root. The 638-test default
suite and existing feature-enabled classifier/cooldown regression pass, along
with locked default and feature library checks, scoped `await_holding_lock`
Clippy, runtime-boundary guard, formatter, plan freshness, release-note
preview, and diff check. This internal-only move has no release-note
fragment; RF-024 remains in progress and the latest full gate remains after
Batch 71.


## RF-024 Message Store Ownership (Batch 102, 2026-09-25)

Moved `MessageRecord`, bounded `MessageStore` state, and controller
conversation JSON construction to `message_store.rs`; crate-root imports
preserve existing callers. The 638-test default suite, message-store eviction
and text-bound regressions, and bounded-store ID-wrap regression pass. The
feature route test `messages_and_rooms_persist_and_rehydrate_records` aborts
with a test-thread stack overflow before assertions, leaving that route-level
persistence/rehydration check unverified. Locked default and feature library
checks, scoped `await_holding_lock` Clippy, runtime-boundary guard, formatter,
plan freshness, release-note preview, and diff check pass. This internal-only
move has no release-note fragment; RF-024 remains in progress and the latest
full gate remains after Batch 71.


## RF-024 Room Store Ownership (Batch 103, 2026-09-25)

Moved room message/roster records, bounded `RoomStore` state, room projections,
and bounded room-name handling into `room_store.rs`; crate-root imports
preserve callers. The 638-test default suite and six exact-name room-state
regressions pass, along with locked default and feature library checks, scoped
`await_holding_lock` Clippy, runtime-boundary guard, formatter, plan freshness,
release-note preview, and diff check. A broad test filter also selected a
pod-membership test that stack-overflowed before assertions; all intended room
regressions pass when selected by exact name. This internal-only move needs no
release-note fragment; RF-024 remains in progress and the latest full gate
remains after Batch 71.


## RF-002 Direct Persistence-Turn Pair Review (2026-09-25)

A comment/string-aware lexical scope scan of all daemon Rust source found five
direct nested persistence-turn acquisitions: Wishlist then Search in the two
ignored-result HTTP dispatchers and the automatic Lidarr rejection path, and
Library then Runtime in both manual-import dispatchers. The inspected paired
handlers keep this order consistently. The wishlist/search transaction helper
commits the ignored rule and changed search rows together while both turns are
held and after the store guards are dropped. Manual import similarly releases
relay, runtime, and library state guards before the paired SQLite transaction;
conditional rollback stays serialized under both turns. The scan found no
inverse direct acquisition order.

This evidence covers direct lexical nesting only. It does not close RF-002: the
remaining paired-persistence inventory and lock acquisition through helper
calls still need review. No implementation behavior changed.


## RF-024 User Store Ownership (Batch 104, 2026-09-25)

Moved `UserRecord`, bounded `UserStore`, username normalization, and
user/controller JSON projections into `user_store.rs`. The 4,096-record cap,
1,024-byte username bound, persisted-record deduplication, and watch/status/
stats updates are unchanged; persistence ordering stays in the existing route
and background call sites. The 638-test default daemon suite and two focused
feature-enabled user-store capacity/normalization tests pass. The split
full-controller watch/unwatch persistence-order regression and user API
watch/list/unwatch projection regression pass. The watch/unwatch regression
under `legacy-route-dispatch` aborts with a test-worker stack overflow before
assertions, so that legacy invocation is unverified. Default and
full-controller/legacy library checks, scoped `await_holding_lock` Clippy,
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. This is internal-only, with no release-note fragment. The
latest complete release gate remains after Batch 71.


## RF-024 User Note Store Ownership (Batch 105, 2026-09-25)

Moved `UserNoteRecord`, bounded `UserNoteStore`, note limits, and
compatibility/native JSON projections into `user_note_store.rs`. The 4,096
record limit, 16 KiB note cap, 128-byte color/icon caps, username normalization,
persisted ID deduplication, and `note-N` allocation are preserved; persistence
ordering remains in the existing route layer. The 638-test default suite and
feature-enabled `notes_and_interests_bound_growth_and_ids` regression pass.
The default library check, full-controller/legacy feature test compilation,
scoped `await_holding_lock` Clippy, formatter, runtime-boundary guard, plan
freshness, release-note preview, and diff check pass. This is internal-only,
with no release-note fragment. The latest complete release gate remains after
Batch 71.


## RF-024 Interest Store Ownership (Batch 106, 2026-09-25)

Moved `InterestRecord`, bounded `InterestStore`, and liked/hated and
recommendation projections into `interest_store.rs`. Duplicate filtering,
case-insensitive matching, separate caps, persisted rehydration, and
wrapping-safe IDs are preserved. Shared `MAX_INTERESTS_PER_KIND` and
`MAX_INTEREST_NAME_BYTES` remain at the crate root because
`ControllerFeatureState` and controller tests use them directly. The 638-test
default suite and feature-enabled `notes_and_interests_bound_growth_and_ids`
regression pass. The default library check, full-controller/legacy feature
test compilation and scoped `await_holding_lock` Clippy, formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only, with no release-note fragment. The latest
complete release gate remains after Batch 71.


## RF-024 Now-Playing Store Ownership (Batch 107, 2026-09-25)

Moved `NowPlayingRecord`, bounded `NowPlayingStore`, and record JSON into
`now_playing_store.rs`. Username normalization, persisted rehydration and
deduplication, upsert, oldest-record eviction, clear, and sort behavior remain
unchanged. The shared record and artist/title text limits remain at the crate
root for controller tests. The 638-test default suite and feature-enabled
`now_playing_and_security_state_bound_remote_keys` regression pass. The
default library check, full-controller/legacy scoped `await_holding_lock`
Clippy, formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only, with no release-note
fragment. The latest complete release gate remains after Batch 71.


## RF-024 Share Group Store Ownership (Batch 108, 2026-09-25)

Moved `ShareGroupMember`, `ShareGroupRecord`, bounded `ShareGroupStore`, and
member/group projections into `share_group_store.rs`. Rehydration,
case-insensitive membership, capacity checks, and ID allocation are preserved.
Shared group and text limits remain at the crate root for existing route/test
callers. The 638-test default suite and feature-enabled
`share_groups_bound_groups_and_case_insensitive_members` regression pass. The
default library check, full-controller/legacy scoped `await_holding_lock`
Clippy, formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only, with no release-note
fragment. The latest complete release gate remains after Batch 71.


## RF-024 Share Grant and Access Token Ownership (Batch 109, 2026-09-25)

Moved `ShareGrantRecord`, `ShareAccessTokenRecord`, the bounded grant and
token stores, token digest handling, username/permission normalization, and
grant permission projections into `share_grant_store.rs`. Existing capacity,
deduplication, ID/TTL bounds, revocation, digest-only persistence, and persisted
row validation remain unchanged. Shared limits remain at the crate root;
persistence transactions and route path parsing remain at their existing
call sites.

The 638-test default suite, direct grant-store feature regression, and token
digest persistence/rehydration regression pass. The token regression overflows
the legacy-dispatch test worker before assertions, then passes with
`full-controller-tests` alone. The default library check, scoped
full-controller/legacy `await_holding_lock` Clippy, changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only and adds no release-note fragment. RF-024 remains
in progress; the latest complete release gate remains after Batch 71.


## RF-024 Incoming Share Store Ownership (Batch 110, 2026-09-25)

Moved `IncomingShareRecord`, bounded `IncomingShareStore`, and the announcement
JSON projection into `incoming_share_store.rs`. Same-ID announcements continue
to replace their row and new IDs continue to evict the oldest entry at capacity.
Route dispatch, the opt-in environment gate, and grant permission checks remain
at their existing call sites.

The default library check and 638-test suite pass; the default route dispatch
compiles. No incoming-share-specific regression exists in the current test
harness. The changed-file formatter and runtime-boundary guard pass; scoped
full-controller/legacy `await_holding_lock` Clippy, plan freshness,
release-note preview, and diff check pass. This is internal-only
and adds no release-note fragment. RF-024 remains in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Collection Store Ownership (Batch 111, 2026-09-25)

Moved `CollectionItem`, `CollectionRecord`, bounded `CollectionStore`, item
normalization, and native/compatibility projections into `collection_store.rs`.
Per-collection and aggregate caps, persisted rehydration, ownership-scoped
reads, duplicate filtering, and wrapping-safe IDs are preserved. Ownership
checks shared with share-grant routes and persistence turns stay at their
existing call sites.

The default library check, 638-test suite, and feature-enabled
`collections_bound_nested_state_and_allocate_unique_item_ids` regression pass.
The collection rollback regression overflows the legacy-dispatch test worker
before assertions, then passes with full-controller-tests alone. Formatter and runtime-boundary guard pass. Scoped full-controller/legacy
Clippy with `-D clippy::await_holding_lock`, plan freshness, release-note
preview, and diff checks pass. This is internal-only and adds no
release-note fragment. RF-024 remains in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Wishlist Store Ownership (Batch 112, 2026-09-25)

Moved `WishlistItem`, wishlist record/filter/policy and ignored-result models,
bounded `WishlistStore`, native/compatibility projections, and native ID
projection into `wishlist_store.rs`. Limits, rehydration, ignored-result
normalization, filter behavior, and ID allocation remain unchanged. Existing
route/background persistence turns and shared limits remain at their current
call sites.

The default library check, 638-test suite, moved native-ID test, feature-enabled
wishlist capacity/ID test, and result-filter test pass. The ignored-result
rollback regression passes with `full-controller-tests` alone. Formatter,
runtime-boundary guard, scoped full-controller/legacy Clippy with
`-D clippy::await_holding_lock`, plan freshness, release-note preview, and diff
checks pass. This is internal-only and adds no release-note fragment. RF-024
remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Search Store Ownership (Batch 113, 2026-09-25)

Moved `SearchRecord`, bounded `SearchStore`, search creation outcomes, controller/history projections, and result identity handling into `search_store.rs`. Search limits, status normalization, token/identity collision handling, and wishlist ignored-result filtering remain unchanged. Persistence turns and route/background orchestration stay at their existing call sites.

The default library check and 638-test suite pass. The feature-enabled active-record/identity, peer-response result-cap, and wishlist peer/folder suppression regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Browse Store Ownership (Batch 114, 2026-09-25)

Moved `BrowseRecord`, bounded `BrowseStore`, browse status projections, and entry/username bounds into `browse_store.rs`. Record and aggregate limits, per-user entry limits, text truncation, persisted rehydration, and wrap-safe indirect-token allocation remain unchanged. Browse persistence turns, route dispatch, and rollback orchestration stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled record/entry bounds, text/aggregate bounds, and indirect-token collision regressions pass with full-controller/legacy features. `browse_cache_persists_and_rehydrates_records` overflows the historical legacy-dispatch test worker before assertions, then passes with `full-controller-tests` alone. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Library Store Ownership (Batch 115, 2026-09-25)

Moved `LibraryItemRecord`, `LibraryHealthScanRecord`, `LibraryRemediationJob`, `LibraryHealthIssueQuery`, and bounded `LibraryStore` state with its catalog and health projections into `library_store.rs`. Item/scan/job bounds, stable IDs, issue classification and repair behavior, query validation, and output shapes remain unchanged. Filesystem scanning, route dispatch, and persistence turn/rollback orchestration stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled item-cap/ID allocation, bounded health scans, health-group totals, and exact-path/bounded-repair regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Index and Lifecycle State (Batch 116, 2026-09-25)

Moved `ShareRoot`, `ShareExtensionSummary`, `ShareIndexSnapshot`, `ShareLifecycleState`, and their JSON/catalog projections and persisted-snapshot rehydration into `share_index_state.rs`. Snapshot contents, cache metadata, lifecycle flags, and projection shapes remain unchanged. Filesystem scanning, path confinement, cache I/O, persistence turns, and route cancellation stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled share index persistence/rehydration, rescan rebuild, and concurrent-scan rejection regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Relay State Ownership (Batch 117, 2026-09-25)

Moved `RelayState` and its enable, projection, rehydration, and conditional rollback methods into `relay_state.rs`. The shared `restore_changed_value_if_unchanged` helper remains at the crate root because runtime compatibility state uses it too. Relay persistence ordering and route orchestration remain unchanged.

The default library check and 638-test suite pass. The feature-enabled runtime compatibility persistence/rehydration regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Mesh State Ownership (Batch 118, 2026-09-25)

Moved `MeshState`, bounded sync violation/quarantine state, capability registry lifecycle, persisted descriptor restoration, and capability/rendezvous projections into `mesh_state.rs`. Peer-key and registry bounds, message/entry rate windows, quarantine behavior, descriptor expiry, and projection shapes remain unchanged. Protocol parsing, route dispatch, and network operations stay at their existing call sites.

The default library check and 638-test suite pass, including direct mesh violation-window and peer-capacity regressions. Feature-enabled capability-store and rendezvous API regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Options State Ownership (Batch 119, 2026-09-25)

Moved `ControllerOptionsOverlayState`, `ObfuscationReloadState`, and restart-reload fingerprinting into `controller_options_state.rs`. YAML projection, command-line environment overrides, watched values, and restart classification remain unchanged. Runtime reload ordering and transfer-policy cancellation stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled command-line-over-YAML, current-overlay lifecycle, and controller backup/reload regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer State File I/O Ownership (Batch 120, 2026-09-25)

Moved transfer state and event path resolution, bounded snapshot loading and restart normalization, snapshot serialization, event header validation, safe event-file opening, append, and rotation into `transfer_state_io.rs`. The shared atomic replacement helper, durability generation coordinator, mutation ordering, and route persistence turns remain at their existing call sites. File-size limits, symlink and FIFO rejection, restart resume behavior, event format, and rotation thresholds remain unchanged.

The default library check and 638-test suite pass. Feature-enabled state-size/FIFO loading, restart rehydration, symlink rejection, event FIFO rejection, and oversized-event rotation regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer Durability Coordinator Ownership (Batch 121, 2026-09-25)

Moved transfer persistence record/event projections, revision helpers, the generation coordinator and pending-event state, durable snapshot flushing, and the corresponding `TransferQueue` hooks into `transfer_durability.rs`. Progress-write throttling remains tied to the same per-transfer updated-at revision. Event ordering, state snapshot ordering, rollback behavior, and lock release before file/database I/O remain unchanged.

The default library check and 638-test suite pass, including stale transfer projection and tombstone regressions. Feature-enabled snapshot/reload ordering, queue availability during a delayed flush, request revision advancement, persistence error projection, and cleanup/cancel rollback regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer Queue Ownership (Batch 122, 2026-09-25)

Moved `TransferQueue`, request/audio metadata models, ID and token allocation, bounded create/update/retry operations, pending-transfer matching, and queue JSON projections into `transfer_queue.rs`. Queue caps, wrap-safe allocation, status transitions, revision advancement, resume behavior, and output shapes remain unchanged. Durability generation tracking and persistence ordering stay in `transfer_durability.rs`; route orchestration remains at its existing call sites.

The default library check and 638-test suite pass. Feature-enabled queue record/bounds/persistence/SQLite rehydration, wrap-safe IDs and tokens, and cancelled-progress regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer Recovery Planner Ownership (Batch 123, 2026-09-25)

Moved auto-retry and underperformance rescue tracker models, pruning, latest-attempt selection, audio eligibility, cached-alternative selection, and bounded plan construction into `transfer_recovery.rs`. Retry timing, per-peer limits, retry ceilings, source-size tolerance, rescue thresholds, and plan ordering remain unchanged. Transfer mutation, search dispatch, persistence, and rollback stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled tracker pruning, bounded/latest-only retry planning, alternate-source selection, queue/throughput/stall rescue planning, and atomic replacement regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer Entry Model Ownership (Batch 124, 2026-09-25)

Moved the shared `TransferEntry` model, serde defaults, elapsed/speed calculations, controller/native/download-request projections, failure and recovery classification, and bounded text normalization into `transfer_entry.rs`. Serialized shapes, redaction, local-path omission, text caps, and recovery labels remain unchanged. The shared controller status mapper and route projection orchestration stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled transfer redaction, bounded recovery projection, elapsed/speed/remaining-time projection, download-batch state, and native populated-transfer contract regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Session Snapshot Ownership (Batch 125, 2026-09-25)

Moved `SessionSnapshot`, its disconnected default, status/summary JSON projections, and public error classification into `session_state.rs`. Snapshot fields, redacted error categories, and projection shapes remain unchanged. Connect/disconnect execution, command delivery, state mutation ordering, and lock lifetime stay in the existing lifecycle code.

The default library check and 638-test suite pass. Feature-enabled session error redaction, server-session route contracts, and the queued-credentials lock-release regression pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 126 — Listener state ownership (2026-09-25)

Moved `ListenerSnapshot`, `ListenerCommand`, snapshot defaults, listener JSON projection, and public error mapping into `listener_state.rs`. Bind configuration defaults, response fields, and redacted error text remain unchanged. Listener task orchestration, bind reconfiguration, and runtime mutation ordering stay at their existing call sites. The default library check and 638-test suite, focused feature-enabled listener redaction and route-shape regressions, scoped full-controller/legacy Clippy, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required.

### Batch 127 — Distributed runtime state ownership (2026-09-25)

Moved `DistributedRuntime`, persistence snapshot/status models, connection roles, constructors/reset/depth helpers, state projections, and persisted-state load/restore/save methods into `distributed_state.rs`. Durable revision tracking, snapshot shapes, error mapping, and target-specific application projections remain unchanged. The watch worker, persistence completion sequencing, shutdown flush, and network mutation workflows stay at their existing call sites. The default library check and 638-test suite, 14 feature-enabled distributed regressions, scoped full-controller/legacy Clippy, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required.

### Batch 128 — Storage directory listing state ownership (2026-09-25)

Moved `StorageDirectoryListOptions` and `StorageDirectoryListState`, including query-derived recursion defaults and emitted/truncated accounting, into `storage_directory_state.rs`. Direct/recursive listing budgets and output behavior remain unchanged. Filesystem traversal, path confinement, ordering, recursion, and response assembly stay at their existing call sites. The default library check and 638-test suite, six feature-enabled listing regressions, scoped full-controller/legacy Clippy, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required.

### Batch 129 — Share scanner ownership (2026-09-25)

Moved `ShareScan`, its options/request models, scan results, cancellation-aware index construction, ordered root grouping, and directory scanning into `share_scanner.rs`. Entry/depth/descriptor caps, cancellation checks, symlink handling, deterministic ordering, and result/error shapes remain unchanged. Share-scan task registration, runtime lifecycle updates, persistence/commit ordering, and controller route orchestration stay at their existing call sites. The default library check and 638-test suite, eleven feature-enabled scanner/index regressions, scoped full-controller/legacy Clippy, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required.

### Batch 130 — Share scanner helper ownership (2026-09-25)

Moved virtual path normalization, component sanitization, hidden-path detection, safe share labels, extension extraction, and bounded WAV/FLAC/MP3 media attribute probing into `share_scanner.rs`. Path projection, media attribute values, and probe byte limits remain unchanged. The shared virtual-path and extension helpers remain available through the crate-root imports used by controller routes and tests. The default library check and 638-test suite, twelve feature-enabled scanner/index regressions, scoped full-controller/legacy Clippy, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required.

### Batch 131 — Share index persistence ownership (2026-09-25)

Moved persisted share-file projections, persisted root summaries, share-attribute encoding/decoding, compatibility cache paths, TSV serialization, and cache-write status projection into `share_index_state.rs`. The versioned cache format, escaping, update timestamp, redacted error shape, and atomic replacement behavior remain unchanged. The generic atomic writer and route/lifecycle persistence ordering remain shared at their existing call sites. The default library check and 638-test suite, nine feature-enabled cache/index regressions, scoped full-controller/legacy Clippy, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required.

### Batch 132 — Session command DTO ownership (2026-09-25)

Moved the `SessionCommand` and `SearchDispatchTarget` DTOs into `session_state.rs`; the root import keeps command senders, producers, the session manager, and existing tests on their current paths. Variant payloads and derives are unchanged. Session execution, dispatch order, command-channel capacity, and lock lifetimes remain at their existing call sites. The locked default library check and 638-test suite pass. The focused wishlist search/restart regression passes with `full-controller-tests`; the combined `full-controller-tests,legacy-route-dispatch` invocation overflows the historical test worker before assertions, then passes with `full-controller-tests` alone. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 133 — Search result projection ownership (2026-09-25)

Moved `SearchResultEntry`, its bounded text normalization, file-attribute extraction, and search/controller JSON projections into `search_store.rs`. Search response shapes, metadata fallbacks, field caps, and controller file projection stay unchanged. Existing root, route, wishlist, transfer-recovery, and test call sites keep using the crate-root import. The locked default library check and 638-test suite pass. Four feature-enabled search-store regressions pass for response merging, peer result caps, bounded result text/aggregate size, and controller IDs; the locked-file controller projection regression also passes. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 134 — Share catalog filter ownership (2026-09-25)

Moved `CatalogFilter` query parsing and matching into `share_index_state.rs`, beside `ShareIndexSnapshot::catalog_json`. Query keys, case/extension normalization, default and capped limits, offset fallback, and matching behavior remain unchanged; the crate-root import is retained only for the full-controller compatibility test. The locked default library check and 638-test suite pass. The feature-enabled `list_limits_are_bounded_by_default` regression passes with `full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 135 — Local file hash cache ownership (2026-09-25)

Moved local SHA-256 streaming, cache-entry state, the 4,096-entry bound, and size/mtime validation into `local_file_hash.rs`. Digest format, I/O chunk size, cache key, invalidation checks, and eviction behavior remain unchanged; content-ID resolution keeps its crate-root call site. The new module is covered by the runtime-boundary guard. The locked default library check and 638-test suite pass. The feature-enabled native-library SHA-256 content-ID regression passes with `full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 136 — Controller rate-limit policy ownership (2026-09-25)

Moved `ControllerRateLimitPolicy`, native-profile path/auth partition selection, and permit/window normalization into `rate_limit.rs`, beside the rate limiter. Partition names, path precedence, caller bypass rules, warm-cache keys, non-positive limits, fallback window, and endpoint-specific caps remain unchanged; request authorization and bucket mutation remain at their existing call sites. The locked default library check and 638-test suite pass. All 34 feature-enabled rate-limit regressions pass, covering partitions, auth precedence, trusted-proxy routing, bounds, and reload projection. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 137 — Wishlist auto-download policy ownership (2026-09-25)

Moved `WishlistAutoDownloadPlan`, result-quality scoring and grouping, track identity helpers, edition matching, and wishlist directory normalization into `wishlist_store.rs`. Quality tuple ordering, format preferences, filename normalization, and edition checks remain unchanged. `auto_download_completed_wishlist` keeps transfer staging, persistence, rollback, and command delivery at the existing lifecycle call site. The locked default library check and 638-test suite pass. The feature-enabled `wishlist_auto_download_enqueues_best_folder_and_applies_one_shot_limit` regression passes. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 138 — Integration target ownership (2026-09-25)

Moved `ResolvedIntegrationTarget`, integration and Lidarr URL validation, and integration special-use IP checks into `integration_target.rs`. Existing crate-root imports preserve callers and test construction. Address resolution, resolved-address pinning, error strings, the private-address environment override, and Lidarr's unspecified/multicast checks remain unchanged. The module is included in the runtime-boundary guard.

The locked default library check and 638-test suite pass. The feature-enabled `integration_ssrf_filter_blocks_special_use_ip_ranges` and `notification_integrations_emit_frozen_wire_requests` regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 139 — Source feed ingestion ownership (2026-09-25)

Moved source-provider matching, loose source-row creation, bounded byte/JSON response readers, provider metadata URL validation and fetching, and the bounded HTML metadata parser into `source_feed_ingest.rs`. Provider host matching, the 512 KiB response cap, metadata precedence, URL checks, disabled redirects, DNS resolution pinning, and request headers remain unchanged. Root imports preserve source orchestration, local RSS/OPML parsing, Spotify/AcoustID helpers, and test call sites. The new module is included in the runtime-boundary guard.

The warning-free locked default library check and 638-test suite pass. Seven feature-enabled provider regressions pass for host spoofing, metadata fallback, declared/chunked response limits, provider fetch contracts, and the frozen catalog/edge differentials. The feature-enabled local-format parsing regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 140 — Local source feed parser ownership (2026-09-25)

Moved local source-kind detection, CSV/playlist/XML parsing, bounded preview item parsing, and local preview projection into `source_feed_ingest.rs`. CSV headers and album handling, M3U/RSS/OPML row order and IDs, entity decoding, namespace matching, the 10,000-row limit, and preview output remain unchanged. Root imports preserve route call sites. The locked default library check and 638-test suite pass. The feature-enabled `source_feed_local_formats_match_frozen_parsing_and_deduplication` regression and `controller_api_differential_bridge_admin_stats_and_source_feed_preview` pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 141 — Lidarr API helper ownership (2026-09-25)

Moved Lidarr system-status and wanted-release clients, the shared bounded JSON GET helper, and quality-profile filter construction into `lidarr_api.rs`. URL validation, resolved-address pinning, request timeout/header behavior, page bounds, error mapping, and quality selection remain unchanged. Sync, import, wishlist mutation, and persistence orchestration stay at their existing call sites. The module is included in the runtime-boundary guard.

The warning-free locked default library check and 638-test suite pass. The feature-enabled `controller_api_differential_lidarr_and_source_feed_contracts` regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 142 — Lidarr manual-import helper ownership (2026-09-25)

Moved Lidarr path mapping, candidate validation, portable filename extraction, already-owned lookup, manual-import candidate requests, and rejected-file deletion into `lidarr_import.rs`. Virtual/POSIX and Windows path mapping, accepted rejection reasons, filename deduplication, pinned requests, and canonical-directory confinement remain unchanged. Import workflow, transfer/wishlist mutation, persistence, and rollback stay at their existing call sites. The module is included in the runtime-boundary guard.

The warning-free locked default library check and 638-test suite pass. Twelve feature-enabled Lidarr regressions pass for candidate selection, path mapping, confined deletion, projection, scheduler state, rejection policy, manual-import rollback and lock release, and the controller differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 143 — Lidarr import history projection ownership (2026-09-25)

Moved the Lidarr import-history key prefix and builder, result-status classifier, history-record construction, and public record redaction into `lidarr_import.rs`. Persisted key names, record fields, timestamps, retry linkage, and removal of the private source directory from public output remain unchanged. Controller-feature persistence and history listing stay at their existing call sites.

The warning-free locked default library check and 638-test suite pass. Twelve feature-enabled Lidarr tests pass, including manual-import history through the controller differential, persistence-failure rollback, and lock-release regressions. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 144 — Lidarr import history lifecycle ownership (2026-09-25)

Moved Lidarr import-history persistence/listing and manual/automatic retry wrappers into `lidarr_import.rs` beside their record projections. History ordering, public redaction, persistence timing, success/error propagation, and retry linkage remain unchanged. Import execution and wishlist/transfer mutations remain at their existing call sites.

The warning-free locked default library check and 638-test suite pass. All 12 feature-enabled `lidarr_` regressions pass, including the controller differential, history projections, persistence-failure rollback, and lock-release checks. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 145 — Lidarr import execution ownership (2026-09-25)

Moved Lidarr completed-directory import execution, its manual wrapper, and automatic delay/retry loop into `lidarr_import.rs`. The per-directory debounce, ownership pre-check, import gate, candidate checks, import mode, retry count/backoff, and result/error behavior remain unchanged. Route, history persistence, and completion callbacks keep calling the module through crate-root imports.

The warning-free locked default library check and 638-test suite pass. All 12 feature-enabled `lidarr_` regressions pass, covering path/candidate/deletion behavior, import result contracts, rejection policy, rollback, lock release, and the controller differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 146 — Lidarr manual import API ownership (2026-09-25)

Moved the Lidarr manual-import candidate GET and import-command POST into `lidarr_api.rs` with the other HTTP clients. DNS resolution and pinning, API-key headers, timeouts, redirect/proxy policy, query/body shapes, response bounds, and error strings remain unchanged. `lidarr_import.rs` keeps the sequencing and result policy.

The warning-free locked default library check and 638-test suite pass. All 12 feature-enabled `lidarr_` regressions pass, including the candidate and import-command fixture requests in the controller differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 147 — database maintenance ownership (2026-09-25)

Moved database statistics projections, terminal-transfer and old-message cleanup, retention pruning and scheduler startup, and database vacuum handling into `database_maintenance.rs`. Existing route entry points and scheduler startup continue through crate-root imports. Persistence ordering, lock release points, retention filters, response shapes, and error handling remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Three feature-enabled database-cleanup regressions pass for transfer tombstones, SQLite-failure rollback, and message-projection ordering. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 148 — browse wire ownership (2026-09-25)

Moved bounded peer browse payload construction and parsing, virtual path composition, and folder grouping helpers into `browse_wire.rs`. Existing route and test call sites continue through crate-root imports. Wire layout, entry/count bounds, parse errors, legacy path decoding, and folder projection remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Seven feature-enabled `shared_file_list_payload` regressions and all 33 feature-enabled browse-filter tests pass; the folder contents path-selection regression also passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 149 — static web asset ownership (2026-09-25)

Moved static asset root selection, request-path confinement, CSP/header construction, bounded file readers, and static response writing into `web_static.rs`. Existing HTTP handlers and test access continue through crate-root imports, with the original platform and test cfgs preserved. Root confinement, symlink handling, size limits, response headers, and SPA selection remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Eight feature-enabled static asset regressions pass for root confinement, symlink escape rejection, size and UTF-8 bounds, CSP behavior, and profile selection. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 150 — controller options projection ownership (2026-09-25)

Moved the controller options JSON projection into `controller_options_projection.rs`, beside the controller options state module. Root and route call sites continue through the crate-root import. Frozen defaults, profile-specific shapes, current/startup distinctions, secret redaction, and volatile-overlay handling remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Twenty-five feature-enabled options regressions pass for frozen defaults, startup aliases, redaction, current/startup projections, and overlay lifecycle. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 151 — controller debug projection ownership (2026-09-25)

Moved the controller options debug projection and its provider, path lookup, scalar formatting, and secret-redaction helpers into `controller_debug_view.rs`. Route and test call sites continue through the crate-root import. Provider selection, default values, sensitivity filtering, current/startup handling, and output shape remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Two feature-enabled debug-view projection regressions and the debug-gating regression pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 152 — controller YAML ownership (2026-09-25)

Moved the controller YAML text response, bounded body validation/parser, key conversion, and API projection helpers into `controller_yaml.rs`. Route and state-module callers continue through crate-root imports; compatibility file I/O and configuration watching remain at their existing lifecycle call sites. YAML size/depth/node bounds, validation errors, target-specific projection, precedence, and response shape remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Twenty-one feature-enabled YAML regressions pass for target-specific validation, startup aliases, projection, native adversarial updates, and durable configuration behavior. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 153 — controller release check ownership (2026-09-25)

Moved release-tag normalization and version comparison, scheduled refresh, and latest-version response construction into `controller_release_check.rs`. The route and startup call sites continue through crate-root imports. Release URL selection, native/.NET version rules, request/error handling, and cached update projection remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Three feature-enabled regressions pass for frozen release comparison rules, the real latest-version lookup path, and the controller residual route differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

### Batch 154 — controller capability and network helpers (2026-09-25)

Moved native capability response projection, capability parsing and negotiation, network statistics projection, and peer capability descriptor helpers into controller_capabilities.rs. Root imports preserve route and mesh callers. Capability grammar, negotiation intersection, response shapes, and persisted descriptor behavior remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Ten feature-enabled capability contract tests and the native network-statistics edge contract pass. Scoped full-controller/legacy Clippy with -D clippy::await_holding_lock, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 155 — controller storage preflight ownership (2026-09-25)

Moved controller event/search read-failure responses, native search identifier and query validation, and HashDb/backfill database-availability checks into controller_storage_preflight.rs. Root imports preserve route-dispatch callers. Controller profile selection, response status and body text, accepted identifier aliases, query bounds, and database checks remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Four feature-enabled controller regressions pass for native event edges, native search compatibility, HashDb paging, and versioned backfill candidates. Scoped full-controller/legacy Clippy with -D clippy::await_holding_lock, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 156 — controller storage listing ownership (2026-09-25)

Moved shared/user file projections, share and user directory JSON, target response shaping, filesystem timestamp formatting, bounded storage-directory walking, Unix confined enumeration, scan/delete limits, and storage query helpers into controller_storage.rs. Root imports preserve route and delete-handler callers; storage_directory_state.rs continues to own the bounded emission state. Response fields, ordering, pagination behavior, path confinement, symlink handling, recursion depth, and entry caps remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Eight feature-enabled regressions pass for bounded and recursive listing, pagination handling, user-browse filtering and paging, API browse parity, and share/relay lifecycle. Scoped full-controller/legacy Clippy with -D clippy::await_holding_lock, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-002 Persistence-Turn Helper Review (2026-09-25)

Incoming wishlist search responses acquire the Wishlist persistence turn before
reading the filter policy, then call `persist_search_result_delta`, which
acquires the Search turn. This follows the same Wishlist-then-Search order as
ignored-result transactions. The split search-result dispatcher calls the
helper without holding the Wishlist turn. The event-history paths inspected do
not acquire Wishlist while holding the Event turn: API event injection drops
its turn after persistence, and `record_event` drops it before session error
handling and feed publication.

Automatic wishlist downloads previously retained their Wishlist turn while
calling `record_event`, which takes the Event turn. The code now drops the
Wishlist turn immediately after the wishlist item persists and before transfer
commands and event publication. This narrows the serialized section without
changing the database transaction or rollback path. The existing automatic
wishlist-download regression passes (1/1), as do the warning-free locked
default library check, 638-test daemon library suite, scoped full-controller /
legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check.
This is an internal lock-lifetime change and needs no release-note fragment.

This is partial helper-based evidence; other RF-002 paired-persistence and
helper-call paths remain to be reviewed. RF-002 remains in progress.

### Batch 157 — controller YAML target validation ownership (2026-09-25)

Moved web-authentication, transfer, Soulseek connection/profile/distributed,
and aggregate target-specific YAML validation into `controller_yaml.rs`. The
crate-root import preserves route and test call sites. Validation order,
target-specific error text, accepted values, and parser bounds remain unchanged.
The runtime-boundary guard scans the expanded module.

The warning-free locked default library check and 638-test daemon library suite
pass. All 21 feature-enabled YAML regressions pass, including the legacy/native
error contracts and durable configuration paths. Scoped full-controller/
legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This internal ownership move needs no release-note fragment. RF-024
remains in progress; the latest complete release gate remains after Batch 71.

### Batch 158 — fallback dashboard ownership (2026-09-25)

Moved the fallback dashboard HTML template into `web_static.rs`. The public
crate-root `fallback_dashboard_html()` remains as a wrapper, preserving
existing route and external call sites. The rendered template, version
substitution, and route behavior remain unchanged.

The warning-free locked default library check and 638-test daemon library suite
pass. Both feature-enabled fallback-dashboard route regressions pass. Scoped
full-controller/legacy Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This internal ownership move needs no release-note
fragment. RF-024 remains in progress; the latest complete release gate remains
after Batch 71.

### Batch 159 — PodCore controller response ownership (2026-09-25)

Moved the PodCore mutation, dynamic-get, and statistics response handlers into
`podcore_controller.rs`. Crate-root imports preserve both route dispatchers'
call sites; response shapes, route validation, persistence order, and error
mapping remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite
pass. Focused feature-enabled regressions pass for PodCore channel CRUD, stats
GETs, and content-metadata validation (3/3). Scoped full-controller/legacy
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This internal ownership move needs no release-note fragment. RF-024
remains in progress; the latest complete release gate remains after Batch 71.

### Batch 160 — MediaCore controller response ownership (2026-09-25)

Moved the MediaCore extended response and mutation response handlers into
`mediacore_controller.rs`. Crate-root imports preserve both route dispatchers'
call sites; route validation, response fields, metrics, and persistence flow
remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite
pass. The feature-enabled MediaCore mutation differential and descriptor
lifecycle differential pass (2/2). Scoped full-controller/legacy Clippy with
`-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. This internal
ownership move needs no release-note fragment. RF-024 remains in progress; the
latest complete release gate remains after Batch 71.

### Batch 161 — extended-controller response ownership (2026-09-25)

Moved extended-controller mutation, search, download, dynamic-get, and get response handlers into `extended_controller.rs`; crate-root imports preserve existing dispatcher call sites and route classifiers remain at the boundary. Response contracts and dispatch behavior remain unchanged. The runtime-boundary guard scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. Focused feature-enabled regressions pass for the mutation differential and native versioned extended GET contracts (2/2). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 162 — transfer-controller response ownership (2026-09-25)

Moved transfer telemetry/storage failure responses, native transfer request validation, and auto-replace status/mutation responses into `transfer_controller.rs`. Crate-root imports preserve both dispatcher call sites. Validation and persistence behavior remain unchanged; the runtime-boundary guard scans the module.

The warning-free locked default library check and 638-test daemon library suite pass. The focused native auto-replace edge-contract regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 163 — share-index runtime lifecycle ownership (2026-09-25)

Moved share-scan cancellation coordination, bounded rebuild admission, runtime share addition/rollback, snapshot generation validation, persistence, and publication into `share_index_runtime.rs`. The state types stay in `share_index_state.rs`, and root imports preserve existing route and test call sites. The runtime-boundary guard scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. Four feature-enabled regressions pass for concurrent-scan rejection, persistence rollback, shutdown cancellation, and error redaction (4/4). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 164 — HTTP connection lifecycle ownership (2026-09-25)

Moved the HTTP connection wrapper and request-stream lifecycle into `http_connection.rs`. Bounded parsing and response framing stay in `http_server.rs`; crate-root imports preserve server and test call sites. Request limits, response handling, routing, logging, and keep-alive behavior remain unchanged. The runtime-boundary guard scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. Feature-enabled request-stream regressions pass for the application-dump differential and filtered events total-count header (2/2). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 165 — file-transfer runtime ownership (2026-09-25)

Moved transfer capability probing, accepted/indirect transfer execution, upload/download progress and retry helpers, completed-download permissions/content checks, and audio metadata extraction into `file_transfer_runtime.rs`. Root imports preserve peer, route, and test call sites; peer socket negotiation remains separate. The inbound resume regression now expects the documented remote position including its existing offset. No transfer behavior changed.

The warning-free locked default library check and 638-test daemon library suite pass. Focused regressions pass for inbound transfer request cases (3/3), audio metadata parsing (1/1), and the download service/path/retry differential (1/1). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal refactor and test expectation correction need no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 166 — peer transport ownership (2026-09-25)

Moved peer socket setup, regular and obfuscated dialing, SOCKS5 negotiation, peer browse/message transport, and plain/obfuscated protocol negotiation into `peer_transport.rs`. Crate-root imports preserve existing callers; the runtime-boundary guard scans the module. No runtime behavior changed. Corrected the existing resume fixture to seed saved progress while its queued transfer is transitioned into an active status.

The warning-free locked default library check and 638-test daemon library suite pass. Eight feature-enabled peer-address response regressions pass, including resume; the obfuscated-transfer preference and remembered legacy-encoding regressions pass (2/2). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move and fixture correction need no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

### Batch 167 — legacy route dispatcher core ownership (2026-09-25)

Moved the feature-gated legacy HTTP route handler (18,479 lines) from `lib.rs` into `legacy_route_dispatch.rs`. Its root wrapper and `legacy-route-dispatch` feature gate are unchanged; the runtime-boundary guard scans the module. Since extraction brought the remaining root below the formatter's generic byte cutoff, `check-rust-format.sh` now explicitly preserves the historical root exclusion. The new module passes standalone rustfmt.

The warning-free locked default library check and 638-test daemon library suite pass. The full-controller/legacy library check and scoped Clippy with `-D clippy::await_holding_lock` pass. The changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. Two legacy runtime differential attempts overflowed a Tokio worker stack before assertions; a broader relay-open case remained in local I/O polling beyond 60 seconds and was stopped. This reproduces the existing documented limitation of the monolithic legacy dispatcher; runtime parity is not newly established by the ownership extraction. Internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress.

### RF-002 wishlist-search scheduler failure lock scope (2026-09-25)

Reviewed `send_due_wishlist_search`: completed-search persistence precedes wishlist completion persistence under the wishlist/search turn. On wishlist persistence failure, it rolls back the wishlist and then conditionally rolls back the associated search record. The paired turn used to cover the following session error update after both stores were restored. It now drops the turn before that unrelated update; the persisted record and compensation order are unchanged.

The warning-free locked default library check and 638-test daemon suite pass, as do scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check. This is helper-based evidence, not closure of all paired routes. RF-002 remains in progress; the internal lock-lifetime change needs no release-note fragment.

### Batch 168 — daemon startup ownership (2026-09-25)

Moved the daemon `serve` startup and service-orchestration function into `daemon_serve.rs`; `run_daemon` calls it through the new module. Startup checks, service construction, managed-task registration, listener setup, and shutdown wiring are unchanged. The runtime-boundary guard scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. The locked full-controller/legacy library check and scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal extraction needs no release-note fragment. RF-024 and RF-002 remain in progress.

### RF-002 Lidarr rejection paired-persistence turn review (2026-09-25)

Reviewed `apply_lidarr_rejection_policy`: it acquires Wishlist before Search, persists ignored-rule creation and search suppression together, and restores Wishlist before conditionally restoring Search on failure. The successful path publishes changed-search updates before dropping Search and Wishlist turns, then proceeds to rejected-file deletion. The turns now end before filesystem cleanup; transaction and event ordering are unchanged.

The existing `lidarr_rejection_policy_blacklists_wishlist_origin_and_deletes_files` regression passes (1/1), as do the warning-free locked default library check, 638-test suite, scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check. This is partial helper-based evidence; RF-002 remains in progress. Internal-only; no release-note fragment is required.

### Batch 169 — controller reload lifecycle ownership (2026-09-25)

Moved compatibility-config path resolution and YAML read/write, options
location projection, watcher startup, and watched configuration load/apply
into `controller_reload.rs`; root imports preserve existing callers and the
runtime-boundary guard scans the new module. The crate root is 36,368 lines.
The bounded API group-4 feature compile also exposed full-only cfgs on test
support imports and fixture writes, now widened to `bounded-differential`.

The warning-free locked default library check and 638-test daemon suite pass.
The focused watched share-reload test and full-controller watched-CORS test
pass (1/1 each), as do the full-controller/legacy library check and scoped
Clippy with `-D clippy::await_holding_lock`. The bounded API group-4 library
check passes with eight unused-import warnings from other feature-gated test
helpers. Changed-file formatter, runtime-boundary guard, plan freshness,
release-note preview, and diff check pass. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

### Batch 170 — MusicBrainz controller response ownership (2026-09-25)

Moved the MusicBrainz mutation response handler and its artist-radar opaque
reference validator (1,034 formatted lines) into `musicbrainz_controller.rs`.
The mutation dispatcher preserves its existing root call; the new module is
included in the runtime-boundary guard. The crate root is 35,370 lines.

The warning-free locked default library check and 638-test daemon suite pass.
The existing MusicBrainz residual differential passes (1/1), along with the
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock`. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. This is
internal-only; no release-note fragment is required. RF-024 and RF-002 remain
in progress.

### Batch 171 — ActivityPub controller ownership (2026-09-25)

Moved ActivityPub signature parsing/verification, signing-key output, actor
validation, relationship handling/projections, actor and webfinger responses,
and music-activity projections (746 formatted lines) into
`activitypub_controller.rs`. Root imports preserve both route dispatchers,
the miscellaneous mutation handler, and test call sites. The runtime-boundary
guard scans the new module; the crate root is 34,643 lines.

The warning-free locked default library check and 638-test daemon suite pass.
All 15 existing ActivityPub regressions pass. The full-controller/legacy
library check and scoped Clippy with `-D clippy::await_holding_lock` pass.
Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only; no release-note fragment
is required. RF-024 and RF-002 remain in progress.

### Batch 172 — session runtime ownership (2026-09-25)

Moved session-command handling, peer-message handling, server reconnection,
due-wishlist-search dispatch, and server-message projection (1,807 formatted
lines) into `session_runtime.rs`. Root imports preserve runtime and test
callers. The RF-002-reviewed wishlist/search persistence and rollback order
is unchanged; the runtime-boundary guard scans the new module. The crate root
is 32,844 lines.

The warning-free locked default library check and 638-test daemon suite pass.
Focused tests pass for verified peer capability ingestion, server reconnect
projection, and inbound-message replay (3/3). The full-controller/legacy
library check and scoped Clippy with `-D clippy::await_holding_lock` pass.
Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only; no release-note fragment
is required. RF-024 and RF-002 remain in progress.

### Batch 173 — security controller response ownership (2026-09-25)

Moved `security_extended_response` (379 lines) into `security_controller.rs`.
Both bounded and legacy dispatchers retain their call sites through the root
import. The runtime-boundary guard scans the new module; `lib.rs` is 32,467
lines.

The warning-free locked default library check and 638-test daemon suite pass.
The existing security controller residual differential passes (1/1), along
with the full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock`. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. This is
internal-only; no release-note fragment is required. RF-024 and RF-002 remain
in progress.

### Batch 174 — miscellaneous mutation response ownership (2026-09-25)

Moved the 591-line `misc_controller_mutation_response` into
`misc_controller_mutations.rs` (598 formatted lines with module imports). The
extended-controller dispatcher retains its existing call through the root
import. The runtime-boundary guard scans the new module; `lib.rs` is 31,878
lines.

The warning-free locked default library check and 638-test daemon suite pass.
The existing ActivityPub inbox relationship differential passes (1/1), as do
the full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock`. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. This is
internal-only; no release-note fragment is required. RF-024 and RF-002 remain
in progress.

### Batch 175 — feature mutation and multisource ownership (2026-09-25)

Moved the feature mutation dispatcher, multisource download/swarm handlers,
and adjacent radar/WorkRef helpers (1,343 formatted lines) into
`feature_mutation_controller.rs`. Root imports preserve bounded/legacy route
dispatch, MusicBrainz use, and tests; the WorkRef text helper was restored
with its original implementation after an extraction-boundary compile check.
The runtime-boundary guard scans the new module; `lib.rs` is 30,559 lines.

The warning-free locked default library check and 638-test suite pass. The
WorkRef/taste differential passes (1/1). The full-controller/legacy library
check and scoped Clippy with `-D clippy::await_holding_lock` pass. The
multisource residual differential reports 10 false cases across the sync and
async swarm routes: its `skipVerification: true` request omits the digest
required by the verified executor. This mismatch is present in the base
`HEAD` handler and test; this extraction keeps both unchanged. It remains an
unresolved baseline contract gap, not a regression from Batch 175. Formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

### Batch 176 — VirtualSoulfind v2 controller ownership (2026-09-25)

Moved the VirtualSoulfind v2 catalogue projection, route dispatcher, and
route-specific segment helpers (432 formatted lines including imports) into
`virtual_soulfind_v2_controller.rs`. Root imports preserve both route
dispatchers and the feature-gated catalogue test call. The route body matches
base `HEAD` apart from rustfmt layout and visibility changes; the runtime
boundary guard scans the new module. `lib.rs` is 30,146 lines.

The warning-free locked default library check and 638-test daemon suite pass.
The focused VirtualSoulfind v2 run reports 8 passes and one unchanged baseline
failure: `virtual_soulfind_v2_routes_execute_bounded_local_intent_workflow`
observes `Failed` instead of `Completed`; repeating it produces the same
result. The test and route body match base `HEAD`, and both VirtualSoulfind v2
API differential tests pass. The full-controller/legacy library check and
scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. This is internal-only; no release-note
fragment is required. RF-024 and RF-002 remain in progress.

### Batch 177 — extended-controller download-batch ownership (2026-09-25)

Moved `controller_enqueue_download_batch` and `transfer_batch_with_entries`
(362 formatted lines) into `extended_controller.rs`. Root imports preserve
bounded/legacy route dispatch and test/helper callers. Other transfer
preparation and persistence helpers remain at the root because they have
separate call sites. `lib.rs` is 29,785 lines.

The warning-free locked default library check and 638-test suite pass. Focused
checks pass for local batch projection, enqueue/persistence, transfer-batch
cleanup, and download-edge contracts (4/4). The full-controller/legacy
library check and scoped Clippy with `-D clippy::await_holding_lock` pass.
Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

### Batch 178 — Lidarr wishlist sync ownership and paired persistence (2026-09-25)

Moved `sync_lidarr_wanted_to_wishlist` (527 formatted lines) into
`lidarr_wishlist_sync.rs`; root imports preserve bounded and legacy route
callers. The dedup set is rebuilt while holding the shared
`wishlist_search_persistence_lock`, after remote fetches. Each changed page is
written using the atomic bulk upsert. If persistence fails,
`rollback_wishlist_if_unchanged` restores the prior snapshot only when no later
mutation replaced the page state; the shared turn remains held through this
rollback. The runtime-boundary guard includes the new module; `lib.rs` is
29,267 lines.

The warning-free locked default library check and 638-test suite pass. Existing
integrations and wishlist residual differentials pass (1/1 each). The locked
full-controller/legacy check and scoped `await_holding_lock` Clippy pass, as
do changed-file formatting and the runtime-boundary guard. The operational
persistence change has a new validated-fragment candidate
`20260925-lidarr-wishlist-sync-atomicity.md`; plan freshness, release-note preview, and diff check pass. No tests were added; enabled-sync
write-failure injection is not covered by the existing suite. RF-002 remains
in progress because broader paired-route review is open.

### Batch 179 — versioned GET contract ownership (2026-09-25)

Moved `versioned_get_failure_contract` into `versioned_get_contract.rs` (517
formatted lines); root imports preserve both route-dispatch call paths. The
runtime-boundary guard includes the new module. `lib.rs` is 28,753 lines.

The locked default library check and 638-test daemon suite pass. Both focused
versioned GET contract differentials pass (2/2). The locked full-controller /
legacy library check and scoped `await_holding_lock` Clippy pass, as do the
changed-file formatter and runtime-boundary guard. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

### Batch 180 — versioned relay upload controller ownership (2026-09-25)

Moved `versioned_relay_request`, its byte-oriented route handler, and relay
upload staging and cleanup helpers into `versioned_relay_controller.rs` (389
formatted lines). Root imports preserve bounded and legacy dispatch, the HTTP
connection path, preview cleanup, and existing test callers. `lib.rs` is
28,360 lines.

The locked default library check and 638-test suite pass. Both focused versioned
relay controller tests and the relay controller route differential pass (3/3).
The warning-free locked full-controller/legacy library check and scoped
`await_holding_lock` Clippy pass, as do changed-file formatting and the
runtime-boundary guard. Plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024 and
RF-002 remain in progress.

### Batch 181 — preview stream ticket controller ownership (2026-09-25)

Moved `create_preview_stream_ticket` and its identifier, filename, and traversal
validation helpers into `preview_stream_controller.rs` (338 formatted lines).
The root import preserves bounded, legacy, and extended-controller callers.
`lib.rs` is 28,025 lines.

The locked default library check and 638-test suite pass. Peer and mesh ticket
creation, validation, and size-limit differentials pass (3/3). The warning-free
locked full-controller/legacy library check and scoped
`await_holding_lock` Clippy pass, as do changed-file formatting and the
runtime-boundary guard. Plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024 and
RF-002 remain in progress.

### Batch 182 — native compatibility controller ownership (2026-09-25)

Moved `native_compat_route` and `native_compat_response` into
`native_compat_controller.rs` (344 formatted lines). Root imports preserve the
bounded and legacy route dispatchers. `lib.rs` is 27,688 lines.

The locked default library check and 638-test suite pass. Native compatibility
route-shape and read-projection differentials pass (2/2). The warning-free
locked full-controller/legacy library check and scoped
`await_holding_lock` Clippy pass, as do changed-file formatting and the
runtime-boundary guard. Plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024 and
RF-002 remain in progress.

### Batch 183 — source feed preview controller ownership (2026-09-25)

Moved `preview_spotify_source_feed`, `preview_configured_provider_source_feed`,
and provider identifier parsers into `source_feed_preview_controller.rs` (621
formatted lines). Root imports preserve both dispatchers and existing test
callers. `lib.rs` is 27,072 lines.

The locked default library check and 638-test suite pass. Five focused Spotify
and configured-provider preview tests pass. The warning-free locked
full-controller/legacy check and scoped `await_holding_lock` Clippy pass, as
do changed-file formatting and the runtime-boundary guard. Plan freshness,
release-note preview, and diff check pass. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

### Batch 184 — wishlist auto-download controller ownership (2026-09-25)

Moved `auto_download_completed_wishlist`, recent-completion selection, and
staged-transfer rollback helpers into `wishlist_auto_download.rs` (354
formatted lines). Root imports preserve bounded, legacy, session-runtime, and
test callers. The existing wishlist persistence turn, atomic counter write,
and conditional rollback remain unchanged. `lib.rs` is 26,721 lines.

The locked default library check and 638-test suite pass. The focused
auto-download ranking and one-shot limit regression passes. The warning-free
locked full-controller/legacy check and scoped `await_holding_lock` Clippy
pass, as do changed-file formatting and the runtime-boundary guard. Plan
freshness, release-note preview, and diff check pass. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

### Batch 185 — wishlist CSV import ownership (2026-09-25)

Moved wishlist CSV parsing, simple text import parsing, header helpers, and
`versioned_wishlist_csv_import_response` into `wishlist_csv_import.rs` (290
formatted lines). Root imports preserve both route dispatchers and source-feed
ingestion. `lib.rs` is 26,440 lines.

The locked default library check and 638-test suite pass. The oversized CSV
row-batch regression and wishlist controller residual differential pass. The
warning-free locked full-controller/legacy check and scoped
`await_holding_lock` Clippy pass, as do changed-file formatting and the
runtime-boundary guard. Plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024 and
RF-002 remain in progress.

### Batch 186 — session manager loop ownership (2026-09-25)

Moved `spawn_session_manager` into `session_runtime.rs`, alongside command
handling, reconnection, due-wishlist dispatch, and server-message projection.
The root import preserves the daemon startup caller; test-only helper imports
are feature-gated to keep default builds warning-free. `lib.rs` is 26,198
lines.

The locked default library check and 638-test suite pass. Existing reconnect
state and VPN/session readiness regressions pass (2/2). The warning-free locked
full-controller/legacy check and scoped `await_holding_lock` Clippy pass, as do
changed-file formatting and the runtime-boundary guard. Plan freshness, release
preview, and diff check pass. This is internal-only; no release-note fragment
is required. RF-024 and RF-002 remain in progress.

### Batch 187 — HashDb history backfill controller ownership (2026-09-25)

Moved `hashdb_backfill_from_history_response` and its FLAC candidate selector
into `hash_backfill_controller.rs` (145 formatted lines). Root imports preserve
bounded and legacy route calls. The persistence turn remains held from
progress read through candidate selection and progress write. `lib.rs` is
26,054 lines.

The locked default library check and 638-test suite pass. The focused cursor
ordering regression and HashDb history-backfill route differential pass (2/2).
The warning-free locked full-controller/legacy check and scoped
`await_holding_lock` Clippy pass, as do changed-file formatting and the
runtime-boundary guard. Plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024 and
RF-002 remain in progress.
