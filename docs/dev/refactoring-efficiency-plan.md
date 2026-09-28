# Whole-Project Refactor Audit

Status: active whole-project implementation plan, as of 2026-09-28 UTC; RF-002, RF-003, RF-007, RF-032, and RF-036 are verified locally. RF-034 is verified locally and through the retained hosted shared-artifact matrix. RF-024 source ownership is verified locally and through hosted baseline `29165746`; later lifecycle/audit fixes have separate validation. RF-066 recipient backfill and per-grant owner admission now have local integration proof; RF-006's nine-site blocking-work inventory is recorded, while non-cancellable mounted-filesystem joins remain open. The live/hosted evidence items listed below remain in progress. RF-001 retains a shared-wire compatibility limit.


Audit baseline date: 2026-09-15; evidence addenda through 2026-09-28 UTC
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
  `PeerInit` frame. The shared listener now rejects prefixes that advertise
  both known init forms before handing off the stream. Ordinary plain and
  obfuscated traffic continues on one port. Extension and nested-form
  ambiguity still requires a coordinated wire policy.
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
| RF-001 | `crates/slskr-client/src/listener.rs` shared demux previously classified the first byte as a raw connection kind before parsing a frame. Bounded plain/obfuscated candidate validation and TCP coverage for `P`, `F`, and `D` collisions are in place. `crates/slskr-client/tests/listener.rs::shared_wire_bytes_can_form_valid_plain_and_obfuscated_init_frames` proves one wire buffer decodes both as a 9-byte plain `PierceFirewall` frame and as a 274-byte type-1 `PeerInit` frame. | A valid obfuscated init could be returned as a shorter valid plain frame, leaving the stream desynchronized. | The shared listener rejects prefixes that advertise both known init forms and plausible nested init interpretations. Unambiguous unknown extensions preserve subsequent bytes. A regression demonstrates unknown-versus-known ambiguity: a local strict-rejection experiment also rejected existing valid obfuscated-key vectors because extension payloads are opaque. The single-port policy is explicit: a recognized init wins over an opaque unknown candidate to preserve peer interoperability, while known-known and unsafe nested candidates are rejected. Both interpretations cannot be distinguished without a negotiated wire marker. All 38 listener tests pass, including the ambiguity regression. All peer traffic continues over the shared port. | Verified locally; shared-port ambiguity policy documented |
| RF-002 | `crates/slskr/src/route_dispatch_group_3.rs`, `group_4.rs`, and `group_5.rs` held write guards across `persist_*`, auth, or other async calls; `group_6.rs` had the same class. Route groups 2-6 now snapshot/drop guards on migrated paths; group 7 wishlist writes also release the store guard during persistence. Lidarr manual import shares a runtime-persistence gate, writes its library item and only its own compatibility counter in one SQLite transaction, and releases state guards during persistence. Pod update/delete persist the parent before channel cleanup, restore parent state on cleanup failure, prune orphans on startup, and serialize appends before rechecking pod and channel existence. Room/pod membership workflows use a consistent room-then-workflow lock order, and acceptance rechecks authorization after waits. Collection, grant, and share-token routes serialize persistence and reject stale parent writes. Share-group/member mutations now use their own persistence turn in both dispatchers, and the legacy path releases its write guard during SQLite I/O. Incoming wishlist results and ignored-rule changes share a turn through their paired transaction; all search persistence helpers re-read the current SearchStore snapshot under one persistence turn. Persisted wishlist item routes, imports, completion counters, Lidarr sync, and auto-download take one turn through SQLite persistence. Library create/delete, health-issue patch/fix, MusicBrainz target creation, and the manual-import transaction take a library turn before mutation; manual import orders library before runtime. Contact creation through direct, discovery, and invite routes, updates, and deletes now share a contact persistence turn in both dispatchers; store guards are released before SQLite. User-note create, versioned create, update, and delete routes now share a separate persistence turn in both dispatchers. Liked/hated interest routes and the mesh-rendezvous compatibility mutation use one interest persistence turn in both dispatchers; versioned routes drop it before outbound session commands, and compatibility create/delete restore memory on SQLite failure. Persisted now-playing updates, listening-party projections, and clears use a dedicated turn in both dispatchers. Webhook registration, active-state changes, and deletion across direct/admin routes share a webhook persistence turn in both dispatchers. Security username/IP ban and unban routes across both dispatchers and overlay blocklist compatibility share a security-ban persistence turn, with SQLite writes outside the store guard and failure rollback on compatibility routes. Message creation, inbound session messages, acknowledgements, and conversation deletion share a message persistence turn across both dispatchers; compatibility acknowledgements roll back on database failure and check the URL username before mutation. Room subscription joins and leaves across both dispatchers and compatibility handlers share one persistence turn acquired before changing RoomStore; legacy paths release the store guard before SQLite and compare before rollback. Spotify OAuth state issue and callback consumption use one persistence turn; callback deletion releases the store guard during SQLite work and removes memory state after commit. Browse request, folder, fail, cancel, and response-ingest routes plus direct/indirect peer completion and failure projections now share a browse turn across both dispatchers and background callbacks; browse-store guards are released before SQLite, and failure reporting/events run after the turn is dropped. User watch/unwatch routes in both dispatchers and inbound watched-user, status, and statistics replies share a user-projection turn; store guards are released before SQLite, rollback remains inside the turn, and session/error work follows its release. API event injection in both dispatchers and background `record_event` writes share an event-history turn; EventStore guards are released before SQLite, and scripts, reporting, and event-feed publication follow turn release. Transfer SQLite full-row and progress writes now reject stale `updated_at_ms` revisions; deletion and staged rollback persist tombstones, restart reserves ids past tombstones, and event history sorts by per-transfer revision; split-router request-name, cancellation, retry, progress, and completion updates now advance revisions monotonically to match the retained dispatcher. HashDb HTTP creation and merge, mesh publish and sync, backfill, and transfer-metadata hash writers now take the shared persistence turn before mutation and retain it through SQLite snapshot persistence while releasing the store guard during I/O. The history-backfill route holds that turn from persisted cursor read through candidate selection, in-memory progress update, and cursor write. Manual database cleanup now deletes terminal transfer rows through the revision-aware tombstone helper and rolls the live queue back if SQLite rejects deletion. Age-based message cleanup calculates one cutoff and holds the message persistence turn through live `MessageStore` pruning and SQLite deletion, restoring memory when persistence fails. Webhook queued-log insertion, dispatch snapshot selection, and delivery-stat memory/SQLite updates now share the webhook persistence turn with registration, activation, and deletion; manager guards are released before SQLite, changed or deleted definitions reject stale delivery stats, and failed stats writes conditionally roll back memory. Spotify authorization/refresh and disconnect previously persisted the encrypted file and published live connection state without a shared order or stale-work check.  Legacy batch-download enqueue now resolves destination paths without retaining the transfer write guard, then rechecks active duplicates under the write lock before staging. | Database stalls previously blocked unrelated readers, unrelated runtime fields could be overwritten by stale snapshots, paired rollback could retain a failed item after an independent concurrent update, delayed snapshots could recreate deleted parent state, an interrupted pod/channel write could retain orphan messages, a stale search snapshot could restore an ignored result, and delayed library, contact, user-note, or interest writes could resurrect deleted records, a delayed now-playing update could restore a cleared track, and a delayed webhook active-state write could restore a deleted webhook, and overlapping security ban/unban writes could persist in an order different from memory, and delayed message creates or acknowledgements could cross conversation deletion or leave rollback gaps, and concurrent room join/leave requests could persist in an order different from RoomStore, and OAuth callbacks could hold the state-store guard through SQLite deletion while issue/delete writes raced, and competing browse route/callback snapshots could persist in a different order from BrowseStore, while user watch/unwatch and inbound projection snapshots could persist in an order different from UserStore, and API event injection held the EventStore write guard through SQLite and blocked history readers; delayed transfer snapshots could overwrite newer rows or restore deleted transfers, and wall-clock transfer mutations could reuse or regress revisions, and delayed HashDb snapshots could erase later accepted merges, and overlapping history-backfill requests could advance from the same stale cursor and skip older searches, and manual cleanup could hide terminal transfers in memory while leaving durable rows to return after restart, and expired message rows could disappear from SQLite while live conversation routes still served them; runtime compatibility writes also held runtime and relay write guards across SQLite latency; legacy share-grant creation and manual Lidarr import now release their store guards before SQLite and conditionally roll back unchanged candidates; legacy collection deletion also releases collection/grant guards before SQLite and conditionally rolls back unchanged stores; webhook log insertion could leave orphan rows after deletion, while delivery stats could update a replaced webhook or remain changed only in memory after failed persistence. Authorization or refresh work already in flight could finish after disconnect and recreate credentials in memory and on disk.  Destination/path awaits previously held the transfer write lock and could block concurrent transfer reads and updates. | The current 621-test daemon library suite passes. Focused regressions cover transaction rollback, field-scoped runtime updates, independent rollback, runtime-persistence ordering, responsive pod and collection reads during waits, parent/grant/token ordering, stale collection snapshot rejection, share-group deletion before a queued member add while reads remain available, pod cleanup rollback, startup orphan repair, rejection of a queued append after channel removal, pending-read availability during membership acceptance, acceptor revocation after a lock wait, queued wishlist response followed by ignore creation, a queued completed-search snapshot that preserves suppressed results, a queued wishlist item update followed by deletion that leaves reads available and SQLite consistent, a queued library health repair followed by deletion, queued contact update/delete with a responsive read and no deleted row in memory or SQLite, and queued user-note update/delete with the same read and consistency guarantees, queued liked-add/delete/hated-add with available reads and SQLite matching memory, injected mesh-compatibility create/delete failures that preserve memory, queued now-playing update/clear with reads available and the cleared record absent from memory and SQLite, and queued webhook PATCH/delete with reads available and no deleted record in memory or SQLite, and queued message-create/conversation-delete with responsive reads and matching empty memory/SQLite state; compatibility ack database-failure rollback and wrong-username no-mutation regressions, and queued room join/leave with responsive reads and no final subscription in memory or SQLite; two queued OAuth consumers with responsive state reads and exactly one persisted state consumption, and a queued browse request/cancel with responsive reads and matching cancelled memory/SQLite state, plus queued user watch/unwatch with responsive reads and matching unwatched memory/SQLite state, plus queued API/background event writes with responsive reads and matching persisted event-ID order. Pod management, pod channel lifecycle, membership-workflow, contact CRUD/concurrency, contact persistence-failure, and contact discovery/read, user-note lifecycle/persistence-failure, and library/interest/now-playing/message persistence-failure frozen-controller differentials pass; now-playing persistence-failure and delete/diagnostic tests pass, as does the native webhook differential; webhook persistence/rehydration/dispatch-log, registration, exact-path, and PATCH contract tests also pass. the versioned interest wire-command test also passes, as do the share-group, share-grant, and collection differentials. The legacy/full-controller feature combination compiles; the earlier focused share-group test invocation overflows the historical monolithic dispatcher's Tokio test-worker stack. Message and conversation persistence-failure controller tests and differentials pass. Room subscription persistence/rehydration tests and room-controller restart/failure differentials pass. OAuth issue/delete failure-injection, persistence rehydration, and Spotify authorize/callback controller differential pass. Browse persistence rollback, cache rehydration, indirect browse, cancellation, and controller differentials pass. User-watch persistence rollback, projection rehydration, and controller user/share and open-case differentials pass. The 26-test event-focused feature set, event persistence rollback/rehydration, and oversized transfer-event rotation/FIFO tests pass. The transfer stale-snapshot/deletion and legacy-schema migration regressions, transfer cleanup and full lifecycle controller differentials pass. The split-router request-name revision regression confirms memory and SQLite retain the updated name and advanced revision when wall-clock time is behind the stored revision. A queued HashDb API-writer regression confirms memory and SQLite remain unchanged while writers wait, reads stay responsive, and final rows and the sequence cursor agree. A two-request history-backfill regression confirms queued batches consume successive cursors and SQLite ends at the latest cursor. Manual cleanup success and SQLite-failure regressions verify tombstone deletion and queue rollback; the existing database-cleanup controller contract passes under split dispatch, while the retained monolithic invocation overflows its historical test-worker stack. A queued message-cleanup regression proves memory and SQLite remain unchanged during the wait and lose the same expired record after release; a closed-database regression proves message-memory rollback. HashDb domain, history-backfill, mesh-sync protocol, and publish/lookup differentials pass under the split dispatcher. The retained monolithic HashDb domain differential overflows its historical Tokio worker stack; the combined feature build passes. The webhook queued-stats regression verifies no mutation while waiting, matching memory/SQLite updates after release, and memory rollback on database failure. Webhook config/reopen/log dispatch, audit-failure reporting, CRUD rollback, and native webhook contract tests pass. A file-backed SQLite lock regression proves runtime readers and an unrelated runtime update proceed during compatibility writes; failed-write rollback preserves that update. The split runtime-control persistence-failure differential passes; the legacy-dispatcher variant overflows its historical test-worker stack. ControllerFeatureState production mutations now use an ordered blocking-worker store that persists snapshot candidates outside the async state lock and publishes after file sync; paused-worker/cancellation and injected-write-failure regressions verify responsive reads, later-writer ordering, per-operation rollback, and matching memory/file state. A scoped full-controller/legacy Clippy pass emitted no `await_holding_lock` diagnostics. The latest default daemon library suite passes 620 tests, including the queued source-feed history writer/readback regression; source-feed preview mutations in both dispatchers now share one persistence turn, and the `full-controller-tests legacy-route-dispatch` library check passes. At an earlier checkpoint, the 613-test daemon suite passed. A broad `transfer_` filter with both controller features still overflows the historical Tokio worker stack in `focused_controller_tests::versioned_swarm_rejects_oversized_transfer_limits_before_discovery`; the combined feature compile and normal full-controller differential pass. Server status, connect, and disconnect responses now snapshot and release the session guard before runtime-credential reads in both dispatchers; a deterministic held-credential regression verifies session writes remain available. Other paired persistence routes outside the now-ordered wishlist, library, search, contact, user-note, interest, now-playing, webhook, security-ban, messaging, room-subscription, OAuth-state, browse, user-projection, event-history, transfer SQLite, HashDb snapshot, history-backfill cursor, manual terminal-transfer cleanup, age-based message cleanup, source-feed history ordering, server-session credential read ordering, and split-router transfer revision ordering still need review. The focused stale-commit regression rejects old work and confirms cleared memory plus absent encrypted file. The locked full-controller/legacy library check and scoped await_holding_lock Clippy pass. The `20260924-spotify-disconnect-order.md` fragment passes the working-tree release-note preview.  A held-destinations route regression proves the transfer write lock remains available while path resolution waits; focused and full daemon tests pass (622), as do the feature compile, scoped lock Clippy, and formatter. The scoped library/runtime persistence-lock scan finds both manual-import dispatchers acquire library then runtime, while other inspected call sites take one turn. The wishlist-search scheduler now drops its paired persistence turn after both rollback steps and before updating session error state. Lidarr rejection handling acquires Wishlist then Search for its paired ignored-result transaction and releases both turns before publishing updates and performing rejected-file cleanup. Lidarr wanted sync now refreshes dedup keys under the shared turn, uses an atomic bulk upsert per page, and restores the prior in-memory page snapshot after persistence failure when the current state still matches. The existing integrations and wishlist residual differentials pass; direct enabled-sync failure injection passes. This remains partial helper-based evidence; broader paired-route review is open. Final bounded production-source audit covers the 92 DatabaseManager write methods and their call sites, plus every checked persistence-helper call in crates/slskr/src. The only paired Wishlist/Search transaction has three production callers; split, legacy, and Lidarr paths acquire Wishlist then Search before mutation and persistence. Outliers are startup-only writes, single-owner wishlist scheduler saves, transfer revision-checked writes, and helper bodies whose callers own turns. The full 640-test daemon suite, targeted contention/failure regressions, full-controller/legacy compile, and scoped await_holding_lock Clippy pass. | Verified locally |
| RF-003 | Distributed mutations previously held `distributed_network` guards across SQLite work, and tree metadata plus child rows used separate transactions. Runtime updates now publish revisioned snapshots; one worker coalesces to the latest bounded snapshot after the state guard is released. `DatabaseManager` loads both tables in one read transaction and replaces both in one write transaction. Graceful shutdown now writes the latest snapshot after managed producers stop. | Database latency previously blocked distributed-state readers, and a failed child replacement could leave branch metadata and child depths from different states. | The current 622-test daemon library suite passes, including cross-table rollback, latest-snapshot persistence, file-backed reopen, startup hydration, and a shutdown-overlap regression proving a registered task is dropped before the final runtime revision reopens from SQLite. A clean-worktree native process proof at `6fb4d2f6` drives a distributed child through the existing peer listener while a real share scan is active. Graceful SIGTERM closes the child and persists the final disconnected tree; after a committed depth-7 update, SIGKILL/restart preserves both tree and child rows with SQLite integrity `ok`. The source/binary/harness-bound artifact is `benchmarks/artifacts/20260927-rf-distributed-shutdown-crash-restart.json`. | Verified locally, retained live shutdown/crash proof |
| RF-004 | `crates/slskr/src/persistence.rs:4114-4184` was the remaining unbatched ignored-result path; the current batch uses bounded search/identity/result batches, preserves `fallback_attempts`, and adds metadata plus failure-injection rollback regressions. The production `serve` path now calls the shared `load_wishlist_store` startup helper. | Search metadata and large ignored-result updates were previously at risk. | The current 622-test daemon library suite passes. A file-backed close/reopen regression loads through the production startup helper and verifies the rehydrated ignore rule through API reads and search filtering. | Verified locally |
| RF-005 | `crates/slskr-client/src/manager.rs:179-224` previously held the server mutex while awaiting an unbounded `send_server_message`; the first implementation batch now bounds the send and marks the session unusable after timeout/error. | A backpressured server previously blocked every indirect request and could strand the manager. | Full manager suite passes with the non-reading peer and subsequent-request regression. | Verified |
| RF-006 | `AppState` owns a `ManagedTaskRegistry` backed by a `JoinSet`; long-lived workers, schedulers, listener managers, bridge/relay services, overlay gateway, DHT, signal/version services, Unix/HTTPS accept loops, and their HTTPS/Unix HTTP handlers are registered for joined shutdown. Plain HTTP handlers use a joined request set. Distributed parent and child socket loops now also use the managed task registry. All three listener types share a 256-connection semaphore. | Accepted HTTP work is bounded and tied to listener lifecycle; retained live HTTP/share-scan shutdown proof passes, while clean-runner coverage for other managed services remains absent. | Focused regression proves HTTPS/Unix handlers share capacity and are aborted/joined with the managed registry; existing shutdown-flush and share-scan overlap regressions pass. The retained `55be6aec` live HTTP/share-scan overlap proof passes. Downloaded clean-runner artifacts at `f9a25f9d` and `a43155de` verify shared peer TCP/UDP listener ownership, TLS capacity, client closure, and socket reuse; the latter includes three all-enabled DHT/control/data QUIC cycles with no extra UDP port. Forwarding, script, WebSocket, CLI-probe, and bounded FTP ownership have subsequent local checkpoints; joined relay cancellation cleanup and interrupted webhook outcomes have additional local checkpoints; the nine production `spawn_blocking` sites are classified in `docs/dev/blocking-work-ownership.md`; after managed async shutdown, runtime teardown now waits up to five seconds for blocking workers and logs if the deadline is reached; a focused held-worker regression verifies bounded teardown and eventual worker completion after release. An operating-system call blocked in the kernel can still outlive the Tokio runtime, so this bounds teardown without claiming to join an uninterruptible filesystem call. Windows Smoke run [`36466395263`](https://github.com/snapetech/slskr/actions/runs/36466395263) on `d4d185cc` passed the bounded blocking-shutdown regression within its 694-test Rust run; GitHub CI run [`36467270728`](https://github.com/snapetech/slskr/actions/runs/36467270728) on `6e9a6c00` also passed `cargo test --locked --workspace`, the shared peer-gateway shutdown proof, and reproducibility artifact collection. Its Linux AArch64 job passed; remaining platform jobs are active or queued. | In progress; Windows and Linux hosted shutdown proofs pass; remaining matrix and mounted-I/O caveat open |
| RF-007 | The share-index worker owns the cancellation token with the scan permit, checks it during filesystem traversal, returns `SHARE_SCAN_CANCELLED_ERROR`, and refuses to publish partial snapshots. Configuration reloads and runtime share-setting changes now share an index persistence turn; generation checks reject completed scans built from older settings before SQLite replacement or live publication. | Cancelled or stale rebuilds cannot replace a newer live or durable share index, and a settings reload preserves its pending-rescan state. | Focused shutdown and watched-reload regressions pass. A clean-worktree native process run at `55be6aec` observed a real 20,000-file scan during SIGTERM, returned 503 to the in-flight scan request, exited zero in 0.114 seconds, retained zero partial SQLite share rows, and reopened with zero files. The retained source/binary/harness-bound proof is `benchmarks/artifacts/20260927-rf-shutdown-overlap.json`. | Verified locally, retained live shutdown proof |

### P1 CI, Release, And Packaging

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-008 | `.gitlab-ci.yml:21-26` previously invoked three nonexistent Rust guard scripts. The current batch uses checked-in process guards and direct Cargo commands. | The GitLab Rust job previously could not start on a clean runner. | Static workflow policy and shell/YAML checks pass. On 2026-09-27, GitLab rejected both a normal and `--no-thin` push of `902574ad` while unpacking existing-base blob `272d9b69` (`scripts/check-controller-options-differential.sh`); local `git cat-file` reads it and GitHub accepted the tip. Fresh pushes of `b9e50c4d`, `dd10c822`, `3049b9fd`, `a4133bc4`, `c02c77bc`, `08f76d7e`, `d4d185cc`, `c81c055a`, `6e9a6c00`, `37ddb72d`, and `b42efb97` on 2026-09-28 were rejected by the same corrupt object, while GitHub accepted those tips. On 2026-09-28, the damaged loose blob was backed up at `/var/opt/gitlab/backups/object-repair-272d9b69345294fa1aa98ec3a83a9383c87be9e2-20260928/damaged.loose-object`, atomically rebuilt from the verified local blob, and confirmed by `git fsck --full --strict --no-reflogs`; GitLab `main` advanced from `576fea92` to `dc7dd5b5` across two successful pushes after repair. No GitLab pipeline has appeared since the repair (latest project pipeline remains #81 from 2026-05-17), so hosted clean-runner proof remains open. On 2026-09-28, GitLab accepted pushes through `b9679f30` and returned HTTP 200 from its internal post-receive endpoint, but direct pipeline-table inspection still found no record newer than #81 from 2026-05-17. CI is enabled and the default `.gitlab-ci.yml` path is configured; the GitLab MCP API token returns 401. Sanitized investigation is in `benchmarks/artifacts/20260928-gitlab-postreceive-pipeline-status.md`. | In progress; repository repaired and tip pushed, GitLab runner proof pending |
| RF-009 | `.gitlab-ci.yml:162-174` previously created a GitHub release while `.github/workflows/release.yml` already owned tag publication. The current batch removes the duplicate publisher. | One tag previously could race two release creators. | Workflow policy passes. A temporary bare-repository simulation of the `github:mirror` tag refspec pushed a synthetic tag to a local bare remote pointing at commit `dc7dd5b5`, without touching real tags or releases. Clean GitLab runner proof remains open; the repaired repository accepts pushes, but no pipeline has appeared since 2026-05-17. | In progress |
| RF-010 | `release-publish.yml:128-168` previously validated AUR hashes and `.SRCINFO` but did not run a clean `makepkg` source/prepare smoke. The release gate and AUR publish job now invoke `check-aur-package-smoke.sh`. | AUR source/package breakage is now exercised before publication on supported runners. | Actual hosted Docker makepkg verifies and extracts both PKGBUILDs in CI `36405007921` at `7a064d37`; retained full matrix and execution markers establish source/prepare smoke. | Verified hosted smoke at `7a064d37`; publication separate |
| RF-011 | `windows-smoke.yml` previously filtered dashboard/client-ts changes while only building Web assets; the current filters align the job with the Rust/Web checks it actually executes. | SDK/dashboard-only changes no longer imply coverage from a Rust/Web-only job. | Workflow policy passes; Windows Smoke run `36462085991` completed on `46854887` with the Windows Rust test suite, WASM check, and Web build green. A newer attempt `36465244591` on `08f76d7e` stopped at Rust formatting before tests: its only diff was Windows CRLF versus rustfmt output line endings. The formatter gate now normalizes trailing CR for comparison; Windows Smoke run [`36466395263`](https://github.com/snapetech/slskr/actions/runs/36466395263) then passed the Rust tests, WASM check, and Web build on `d4d185cc`. | Verified; Windows Smoke `36466395263` passed after the formatter EOL fix |
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
| RF-028 | `.github/workflows/publish-chocolatey.yml:17-48` now checks out the selected tag, uses a fully qualified release asset URL, verifies `SHA256SUMS`, and runs package smoke preparation. | Manual Chocolatey packages previously could contain invalid URLs and branch drift. | Workflow/package policy and eight actual PowerShell checksum regressions pass. Clean Windows validation-only run `36383308751` downloads and checks the actual `release-v0.2.40` asset, packs/smokes the nupkg, and retains it. Downloaded package/archive hashes, installer URL/checksum, and version are verified; no package was pushed. | Verified clean Windows package runner |
| RF-029 | `.github/workflows/release-publish.yml:625-638` intentionally targets the existing `snapetech/homebrew-slskdn` compatibility tap, while `docs/dev/release-channels.md:15-18` named `snapetech/homebrew-slskr`. | Channel documentation was stale, but the publication target was not shown to be wrong. | Release-channel documentation now names the actual compatibility tap and preserves the explicit target boundary. | Verified locally, docs |
| RF-030 | `web/package.json:77-82` exposes a bundle-budget test; the current batch wires it after Web builds in `.github/workflows/ci.yml` and `scripts/run-release-gate.sh`. Serialized browser E2E now passes its available 14 tests; the React audit and retained nightly artifacts remain separate. | Bundle regressions previously could merge without CI evidence. | Workflow policy, Web budget, build-output, and available E2E checks pass. The retained React audit passes 84 desktop/mobile rendering checks and four workflow scenarios with zero synthetic sweeps; 106 workflow cases are observed through actual UI requests. A daily/manual workflow now retains four real-browser mock scenarios, screenshots, and source-bound receipts for 30 days under memory/swap/task/runtime limits. The successful manual hosted run `36382238277` at `3f0b6010` retained all four reports and 84 screenshots; downloaded audit hashes match the clean checkout receipt. The same workflow is scheduled daily. | Verified hosted browser retention |
| RF-031 | The release gate invokes the remediation baseline and differential checks, but the universal replacement acceptance document points to retained local artifacts and the release path does not run the full live transport/lifecycle manifest. | Historical acceptance evidence could be mistaken for fresh release evidence. | The acceptance document now explicitly labels the 2026-08-20 closure as historical and requires fresh release-gate/live artifacts for current certification. | Verified locally, policy explicit |

### P2 Efficiency And Structural Boundaries

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-020 | `crates/slskr-client/src/file_transfer.rs` now flushes only token/offset handshake writes; plain and obfuscated payload chunks use `write_all` without per-chunk flushes. Pinned Tokio 1.53.1 implements `TcpStream::flush` as a no-op. | On the production TCP path, removing these calls does not remove a socket syscall; local loopback measurements show no reproducible throughput change. Transfer queue durability is tracked separately in RF-047. | Two release-profile loopback runs compare validated old/current plain and obfuscated payloads (five 64 MiB samples per variant, 80 KiB chunks). Sample ranges overlap; retained JSON and host/toolchain details are under `crates/slskr-client/benchmarks/artifacts/`. Remote-peer throughput measurement remains open. | Verified locally; no material gain measured |
| RF-021 | `list_search_results`, `list_transfers`, and `get_webhook_logs` page full rows by ordered columns. The synthetic cardinality run uses 1,000,000 search results, 500,000 transfers, and 500,000 webhook logs. | Transfer and webhook composite indexes materially lower synthetic page-read latency. SQLite's existing `(search_id)` index already satisfies `ORDER BY id` through the implicit rowid; `(search_id, id)` adds storage but is not selected. | On Python SQLite 3.53.4, retained transfer/webhook composites reduce measured first/deep-page medians by 88–99%; search-result medians and plans do not change. The daemon removes the redundant search index on startup. Focused query-plan and file-backed migration regressions plus the 613-test suite pass. Machine-readable output is `benchmarks/artifacts/20260924-sqlite-cardinality.json`; methodology and the 3.46.0 bundled-SQLite limitation are in `benchmarks/artifacts/20260924-sqlite-cardinality.md`. Real-database measurements remain open. | Verified locally; production cardinalities separate |
| RF-022 | `scripts/check-client-sdk-gates.sh` now owns Go/Python/TypeScript SDK lifecycle, lint, advisory, build, and package checks; GitHub and release paths call it once, while GitLab keeps image-specific Go and Node feedback lanes. | CI latency and source-of-truth drift increase without more coverage. | `docs/dev/sdk-ci-ownership.md` records the split; the combined gate, GitHub/release wiring, and read-only Go lane pass locally. | Verified locally; hosted pipeline separate |
| RF-023 | `crates/slskr/Cargo.toml` has a default `focused-controller-tests` feature; the first implementation batch now selects the focused module explicitly with that feature. | Feature naming previously did not describe actual test selection. | Locked `cargo check` passes for default, `--no-default-features`, `bounded-persistence-tests`, and `full-controller-tests` configurations. | Verified locally |
| RF-024 | `crates/slskr/src/lib.rs` is now 2,301 lines, down from the 18,161-line audit baseline; `web/src` has 105,422 JavaScript/JSX lines (112,823 lines across all files) after route/polling splits. The managed task registry, bounded HTTP task admission, and lifecycle regression live in `managed_tasks.rs`; runtime compatibility state logic lives in `runtime_compat_state.rs`; hash backfill state and persistence helpers live in `hash_backfill_state.rs`; source-discovery lifecycle state lives in `source_discovery_state.rs`; source-discovery dispatch, source projection, and scheduled searches live in `source_discovery_runtime.rs`; security bans, reputation, durable JWT revocation, and login-attempt throttling live in `security_state.rs`; bounded OAuth issue/consume state lives in `oauth_state.rs`; source-feed history, Lidarr sync state, Spotify connection state, and feed row/target models live in `integration_runtime_state.rs`; contact records/store live in `contact_state.rs`; destination records/store/path selection live in `destination_state.rs`; controller feature state and bounded JSON persistence live in `controller_feature_state.rs`; preview tickets live in `preview_stream_state.rs`; preview stream ticket creation and input validation live in `preview_stream_controller.rs`; versioned GET failure contracts live in `versioned_get_contract.rs`; versioned relay upload, stream, and download handling lives in `versioned_relay_controller.rs`; native compatibility route matching and read projections live in `native_compat_controller.rs`; Spotify and configured-provider source-feed previews live in `source_feed_preview_controller.rs`; the static provider catalog projection lives in `source_provider_catalog.rs`; SongID evidence-package and capability projections live in `songid_controller.rs`; source metadata and fingerprint analysis, queued workers, failure recording, and persisted-run recovery live in `songid_runtime.rs`; `spawn_session_manager` and the Gold Star Club auto-join supervisor live in `session_runtime.rs`; Soulfind bridge TCP framing, client handling, and managed server runtime live in `soulfind_bridge_runtime.rs`; mesh gateway auth, origin validation, and HTTP service responses live in `mesh_gateway_controller.rs`; ranking-history and source-candidate responses live in `ranking_controller.rs`; the history-backfill route and FLAC candidate projection live in `hash_backfill_controller.rs`; the idle-gated backfill scheduler and cycle execution live in `hash_backfill_runtime.rs`; wishlist auto-download planning and staged-transfer rollback live in `wishlist_auto_download.rs`; CSV parsing, simple text import, and versioned CSV import live in `wishlist_csv_import.rs`; listening-party stream limits live in `listening_party_stream_state.rs`; bounded PodCore runtime, signature, and membership verification counters, key validation, and recording logic with regression coverage now live in `podcore_runtime_stats.rs`; PodCore mutation, dynamic-get, and stats response handlers live in `podcore_controller.rs`; MediaCore extended and mutation response handlers live in `mediacore_controller.rs`. Pod membership DTOs, workflow/replay stores, canonical path parsing, acceptor authorization, and signature verification now live in `pod_membership_workflow.rs`; PodCore content-ID parsing, MusicBrainz requests, and metadata shaping live in `musicbrainz_lookup.rs`. | Ownership, review, compile feedback, and safe change isolation remain poor. | Batches 72-74 pass locked feature compiles, the 614-test daemon suite, runtime-boundary guard, and formatter. Batch 75 passes the full-controller/legacy library and test-target compiles, cancellation/order/rollback regressions, the 616-test daemon suite, runtime-boundary guard, scoped `await_holding_lock` Clippy pass, docs/plan freshness, formatter, and diff checks. Batch 76 passes the PodCore stat-key bounds and direct signing/verification counter regressions plus the locked full-controller/legacy library check. The feature-enabled test target compiles, but executing the existing signing-stat endpoint regression overflows its historical test-worker stack before assertions; the runtime-boundary guard, docs and plan freshness, formatter, and diff checks pass. The 27 listener tests passed in Batch 71 and the full release gate passed after Batch 71. Continue splitting by ownership and measure compile/test feedback after each move. Upload-peer cooldown state, key normalization, expiry, and failure classification moved with its three tests into upload_peer_cooldowns.rs; private-message auto-response cooldown tracking, limits, normalization, expiry, and two direct tests moved into private_message_auto_responses.rs; BrowseEntry and bounded remote path-encoding state now live together in browse_path_state.rs with direct entry/cap tests and the feature regression preserved; managed blacklist runtime state and bounded decision caching now live in managed_blacklist_runtime.rs, preserving target-specific matching, TTL/capacity, eviction, and replacement invalidation; shared controller regex matching and request-thread compilation now live in controller_regex.rs with existing root call sites preserved; private-message auto-response eligibility classification now lives beside its bounded cooldown tracker in private_message_auto_responses.rs; MessageRecord, bounded MessageStore state, and conversation JSON construction now live in message_store.rs while route persistence ordering and rollback remain at the crate root; RoomMessageRecord, RoomRosterEntry, RoomRecord, RoomStore, room projections, and bounded room-name handling now live in room_store.rs; UserRecord and bounded UserStore live in user_store.rs; UserNoteRecord and bounded UserNoteStore live in user_note_store.rs; InterestRecord and InterestStore live in interest_store.rs; NowPlayingRecord and bounded NowPlayingStore live in now_playing_store.rs; ShareGroupMember, ShareGroupRecord, and bounded ShareGroupStore live in share_group_store.rs; ShareGrantRecord, ShareAccessTokenRecord, and both bounded stores live in share_grant_store.rs; IncomingShareRecord and bounded IncomingShareStore live in incoming_share_store.rs; CollectionItem, CollectionRecord, and bounded CollectionStore live in collection_store.rs; WishlistItem, wishlist filters/policies, ignored-result models, and bounded WishlistStore live in wishlist_store.rs; SearchRecord, bounded SearchStore, search create outcomes, history/controller projections, and result identity handling live in search_store.rs; the managed search-expiry supervisor lives in search_runtime.rs; VPN polling, advertised-port synchronization, reconnect requests, and reconnect wake handling live in vpn_runtime.rs; BrowseRecord, bounded BrowseStore state, indirect-token allocation, and controller/list projections live in browse_store.rs; library item/health/remediation models, query parsing, bounded LibraryStore, and catalog/health projections live in library_store.rs; share roots, index snapshots, lifecycle state types, and snapshot/catalog projections live in share_index_state.rs; share scan cancellation, rebuild/add lifecycle, snapshot generation checks, persistence, and publication now live in share_index_runtime.rs; HTTP connection and request-stream lifecycle now live in http_connection.rs while bounded parsing and response framing remain in http_server.rs; RelayState and conditional rollback logic live in relay_state.rs; bounded mesh sync-security state, capability registry, and mesh projections live in mesh_state.rs; controller options overlay, obfuscation reload snapshot, and restart fingerprint live in controller_options_state.rs; transfer state snapshot loading/writing and event-file I/O live in transfer_state_io.rs; generation-based durability snapshots, pending events, and TransferQueue persistence hooks live in transfer_durability.rs; TransferQueue and request/audio metadata models, ID allocation, transitions, and queue projections live in transfer_queue.rs; bounded auto-retry and underperformance rescue trackers/planners live in transfer_recovery.rs; TransferEntry serialization, redacted/native/controller projections, recovery fields, and bounded normalization live in transfer_entry.rs; file-transfer progress, retry, peer upload/download, inbound request handling, bounded backfill receive, and completed-media metadata lifecycle live in file_transfer_runtime.rs; session snapshot defaults, error mapping, and JSON summaries live in session_state.rs; ListenerSnapshot and ListenerCommand DTOs, listener JSON projection, and public listener error mapping live in listener_state.rs; listener binding, lifecycle supervision, and incoming-connection handling live in listener_runtime.rs; DistributedRuntime models, persistence snapshots/status, connection roles, and state projections live in distributed_state.rs; parent/child link lifecycle, search forwarding, branch/depth updates, and persisted settings coordination live in distributed_runtime.rs; storage-directory listing budgets, options, and bounded emission state live in storage_directory_state.rs; share scan inputs/results, cancellation-aware index building, and bounded filesystem traversal live in share_scanner.rs; virtual path normalization, hidden-path handling, extension extraction, and bounded media attribute probes also live in share_scanner.rs; persisted share-file projections, root summaries, attribute encoding, and compatibility cache I/O live in share_index_state.rs; SessionCommand and SearchDispatchTarget DTOs live in session_state.rs; SearchResultEntry, text-bounded search result projections, and file-attribute projection live in search_store.rs; share CatalogFilter query parsing and matching live in share_index_state.rs; local file SHA-256 hashing and bounded cache state live in local_file_hash.rs; controller rate-limit policy/model, partition/window projection, and the managed periodic cleanup task live in rate_limit.rs; WishlistAutoDownloadPlan, track identity, quality ranking, edition checks, and wishlist path normalization live in wishlist_store.rs; resolved integration targets and private-address URL validation live in integration_target.rs; source provider matching, bounded ingestion responses, metadata parsing, and provider page requests live in source_feed_ingest.rs; local CSV, playlist, RSS/OPML, and bounded preview parsing live in source_feed_ingest.rs; Lidarr status/wanted HTTP clients, bounded JSON GET helper, quality-profile filter, manual-import candidate GET, and import-command POST live in lidarr_api.rs; Lidarr manual-import path mapping, candidate filtering, duplicate detection, and confined rejected-file deletion live in lidarr_import.rs; Lidarr import-history keys, status classification, persisted-record projection, and public redaction live in lidarr_import.rs; history persistence/listing and retry wrappers live in lidarr_import.rs; manual/automatic import execution, debounce, and retry sequencing live in lidarr_import.rs; database statistics projections, cleanup, retention pruning and scheduling, and vacuum handling live in database_maintenance.rs; bounded peer browse payload construction/parsing, virtual path composition, and folder projections live in browse_wire.rs; collection, wishlist, VirtualSoulfind, share, user, room, bridge, library-health, and share-group route parsing and blank-ID contracts live in `controller_route_inputs.rs`; shared request parsing and bounded array/response checks live in request_input.rs; static asset root selection, path confinement, CSP policy, bounded file reads, static responses, and fallback dashboard HTML live in web_static.rs; controller options JSON response projection lives in controller_options_projection.rs; controller options debug projection and value redaction helpers live in controller_debug_view.rs; controller_yaml.rs owns YAML parsing and responses; target validation/error contracts live in `controller_yaml_validation.rs`; key conversion and API projection live in `controller_yaml_projection.rs`; controller CLI flag mapping, serve argument parsing, and command-line environment projection live in controller_cli.rs; bounded swarm analytics aggregation, peer ranking, and recommendation projection live in swarm_analytics.rs; release-tag normalization/version comparison, scheduled refresh, and latest-version response live in controller_release_check.rs; controller capability parsing, negotiation, native response projection, network statistics, and peer capability descriptor helpers live in controller_capabilities.rs; controller event/search/HashDb/backfill storage preflight and identifier/query validation helpers live in controller_storage_preflight.rs; controller storage route projections, bounded Unix/non-Unix directory enumeration, filesystem timestamp shaping, shared scan limits, and storage query helpers live in controller_storage.rs; extended-controller mutation, dynamic-get, and get handlers live in `extended_controller_mutations.rs`, `extended_controller_dynamic_get.rs`, and `extended_controller_get.rs`, while shared search/download helpers remain in `extended_controller.rs`; PodCore mutation, dynamic-get, and stats handlers live in `podcore_mutations.rs`, `podcore_queries.rs`, and `podcore_stats_controller.rs`, with shared helpers in `podcore_controller.rs`; MediaCore GET and mutation handlers live in `mediacore_extended.rs` and `mediacore_mutations.rs`, with metric and descriptor helpers in `mediacore_controller.rs`; transfer telemetry/storage failure, native transfer validation, and auto-replace response helpers live in transfer_controller.rs; peer socket setup, regular and obfuscated dialing, SOCKS5, peer browse/message transport, and plain/obfuscated negotiation live in peer_transport.rs; incoming plain/obfuscated peer-message loops, request dispatch, blacklist handling, upload-queue scheduling, and queue projections live in peer_message_runtime.rs; the feature-gated legacy HTTP dispatcher preflight and ordered composition live in `legacy_route_dispatch.rs`, with its 399 compatibility route arms split in source order across `legacy_route_dispatch_group_00.rs` through `legacy_route_dispatch_group_10.rs`; the bounded route-dispatch group 6 is an orchestration layer over admin/discovery, telemetry, media/jobs, security/shares, integrations, and MusicBrainz handlers in six focused modules; the 1,342-line telemetry handler now composes metrics/jobs, Pod routes, federation/security, and multisource/graph handlers in four focused modules; the group 6 admin/discovery handler now composes configuration/recommendation and relay-controller routes in two focused modules;  route-dispatch group 0 now composes controller identity/application/session routes and mesh/HashDb/VirtualSoulfind routes in two owner modules, preserving route-arm order; route-dispatch group 1 now composes discovery/backfill/docs/batch, configuration/telemetry, events/webhooks/options, and shares/files handlers in four ownership modules, preserving all route arms in their original order; route-dispatch group 3 now composes room/user/browse, admin-control, room-membership, and options/diagnostics handlers in four ownership modules, preserving route-arm order; route-dispatch group 2 now composes five modules for session/search, downloads, transfer status/files, and room routes; groups 4, 5, and 7 now each compose four modules for collection/wishlist/contact, library/conversation/configuration, and media/stream/PodCore/network handler ownership; daemon startup, shutdown-signal supervision, process replacement, and service orchestration live in daemon_serve.rs; controller compatibility config paths, YAML persistence, watcher, and watched reload/application live in controller_reload.rs; MusicBrainz mutation responses and opaque artist-radar route-reference validation live in musicbrainz_controller.rs; MusicBrainz target and discography-coverage helpers now live in musicbrainz_targets.rs included within route_dispatch, preserving their route_dispatch call paths; the 692-line request-flow block and RouteDispatchContext now live in route_dispatch_request_flow.rs, leaving route_dispatch.rs at 435 lines; ActivityPub signatures, actor and relationship handling, webfinger, and music-activity responses live in activitypub_controller.rs; session-command and peer-message handling, server reconnection, wishlist-search dispatch and smart fallback, Gold Star Club auto-join supervision, and server-message projection live in session_runtime.rs; extended security route responses live in security_controller.rs; quarantine-jury mutation, aggregate/audit/acceptance, and dynamic-read handlers live in quarantine_controller.rs; miscellaneous mutation route responses live in misc_controller_mutations.rs; download-batch response handling and projection helpers also live in extended_controller.rs; VirtualSoulfind v2 catalogue and route handling live in virtual_soulfind_v2_controller.rs; Lidarr wanted synchronization, its managed scheduler and cycle-state projection live in lidarr_wishlist_sync.rs; transfer and auto-retry task supervisors, auto-replace settings projection, persisted rescue-search creation, verified mesh-swarm promotion, retry execution, and rollback coordination live in transfer_recovery_runtime.rs; overlay DHT publication scheduling, snapshot construction, and result logging live in mesh_dht_runtime.rs; PlayerBar visual tiles and stored visualizer-engine selection live in PlayerVisualTiles.jsx; PlayerBar collection and local-file browsing live in PlayerLauncher.jsx; PlayerBar listening statistics, history import/export, and recommendation actions live in PlayerStatsModal.jsx; PlayerBar smart-radio search and Wishlist controls live in PlayerRadioModal.jsx; PlayerBar queue and similar-track handoff controls live in PlayerQueueModal.jsx; PlayerBar is now 1,323 lines with visual, collection, stats, radio, queue, and discovery owners split into modules; Visualizer preset persistence, normalization, and import helpers live in visualizerPresetLibrary.js; App network endpoint normalization, persistence, and ingress migration notice live in NetworkEndpointNotice.jsx; App.jsx is now 2,034 lines and preserves getStoredNetworkEndpointSnapshot; App connection-status menu behavior lives in ModeSpecificConnectButton.jsx; App.jsx is now 1,889 lines; SongIDPanel display/dedup helpers live in songIdPanelHelpers.js; its 1,085-line analysis column lives in SongIDAnalysisResults.jsx, leaving SongIDPanel.jsx at 646 lines; Messaging workspace state is owned by messagingWorkspaceState.js and pod message polling/send lifecycle by PodChannelSession.jsx; Messaging.jsx is now 1,214 lines; AdminPolicies form presentation lives in AdminPoliciesForm.jsx with save/reset YAML lifecycle in the 573-line parent; Adversarial Settings tab and pane rendering lives in AdversarialSettingsView.jsx while data and connectivity request lifecycles stay in its parent; Network peer and infrastructure presentation lives in NetworkDetails.jsx with polling and request lifecycles in the 827-line Network parent; SearchDetail filter and album-candidate panels live in SearchResultControls.jsx, with search/result lifecycle in the 850-line parent; Response.jsx is now 999 lines with its DownloadActionPreviewModal presentation in a separate module; Visualizer overlay control presentation lives in VisualizerOverlayControls.jsx, with audio/engine and persisted state in the 1,356-line parent; Wishlist.jsx is now 599 lines with row, editor, CSV importer, and shared mounted-state ownership in sibling modules; Searches.jsx is now 733 lines with legacy and modern list-page views and persisted collapsible sections in SearchesListView.jsx; Collections.jsx is now 925 lines, with create, share, and add-item modal views in dedicated child modules; Messaging.jsx is now 964 lines, with its sidebar and batch private-message modal in dedicated child modules; PlayerBar.jsx is now 1,111 lines, with its integrations modal, tool button, and rating controls in sibling components while playback, token persistence, and external-visualizer request lifecycles remain in PlayerBar; CompatibilityDashboard.jsx is now 870 lines; chart transforms, series, and graph rendering live in CompatibilityGraph.jsx, with report fetching and tabs in the parent; App.jsx is now 1,774 lines, with the right-side header menu in AppHeaderMenu.jsx and theme/logout/connect behavior passed in as callbacks; App.jsx is now 1,476 lines, with primary route navigation and its notification icon in AppNavigationPrimary.jsx; App.jsx is now 1,104 lines, with lazy route imports, agent/native route tables, and route-miss diagnostics in AppRouteTable.jsx; withTokenCheck remains App-owned Final 2026-09-27 closure scan: the largest non-test Rust source in crates/slskr/src is config.rs at 4,703 lines, the largest Web JS/JSX source is 1,689 lines, and the largest dashboard source is 279 lines. lib.rs is 2,305 lines, persistence.rs 2,313, and cli.rs 2,346. The 640-test daemon library suite, full-controller/legacy feature compile, Web ownership inventory, runtime boundary hardening guard, formatter, Rust module hygiene, and focused Web build/tests recorded in Batches 109-383 pass. The split router now converts an exhausted route search to HTTP 404; its direct and batch regression passes. | Verified owners locally and hosted at `29165746` |
| RF-025 | The existing plan and performance report retained stale Web test counts without a clearly separated current snapshot. | Planning and release decisions could use incorrect baselines. | `docs/performance-analysis.md` labels the 2026-09-15 836/141 baseline as historical, records the 2026-09-24 local Web snapshot of 872 tests across 146 files, and keeps the 2026-09-22 dashboard snapshot of 38 tests across 11 files with commands. | Verified locally |
| RF-026 | `docs/dev/bug-burndown-ledger.md` marks several SDK/docs items verified while the current checks do not exercise the cited behavior. | Audit closure is not evidence-backed. | The ledger now names the executable checks for BUG-020, BUG-030, and BUG-039; docs freshness, SDK example contracts, the aggregate SDK gate, and the remediation baseline pass. Hosted/live compatibility rows remain explicitly separate. | Verified locally; hosted/live evidence separate |
| RF-032 | `docs/full-network-test-plan.md` mixed a historical all-pass result with a newer missing-artifact plan; `REMEDIATION.md` was a stale snapshot that said daemon tests were excluded even though CI runs them. | Operators could not tell current release evidence from archived history. | The network plan labels dated results historical and points operators to current release/live gates; REMEDIATION is explicitly historical with active-plan links. A scoped local-link and referenced-script audit found no broken current paths; absent script names appear only as proposed work. The server-code count is corrected to the 103-code validated inventory, and docs freshness passes. | Verified locally; fresh live certification remains separate |
| RF-033 | Council counts had drifted across `docs/dev/council-scan-inventory.md`, `.council/latest-candidate-counts.md`, and the active backlog; benchmark comparison/profile tests were not invoked; frontend configs lacked coverage thresholds. | Audit numbers drift and executable performance/coverage checks are absent. | The generated report now stamps date/commit provenance; active-backlog, inventory-closure, and council-freshness gates validate synchronized copies. Dashboard V8 coverage remains ratcheted in CI/release. CI and release now run the seven benchmark comparison/SQLite profiler unit tests and a focused Web API/session/event-lifecycle coverage gate at 84% statements, 75% branches, 72% functions, and 86% lines; the focused 50-test run passes at 85.54%, 78.13%, 73.91%, and 87.38%. | Verified locally |
| RF-034 | SDK gates, CI, and release gate reinstall/build the same TypeScript/Web assets; release archives previously used ambient mtimes/order and the SBOM serial was constant. | CI latency grows and release artifacts were not reproducible or uniquely identified. | Archives now use sorted entries, fixed source timestamps/ownership, deterministic gzip/ZIP metadata, and the CycloneDX serial derives from release version plus source commit. Two current Linux tar builds with a fixed source timestamp produced identical SHA-256 `89a5be824e5600b51ac6d774cff07342decfb7cde28446ccb27a9fa797b58e6d`; macOS and Linux hosted archives passed, and Windows archive construction, verification, and packaged-binary smoke passed at `339e6812`; all seven hosted archive jobs reuse one verified Web artifact and the complete CI/package workflow passes at `47440cfb`, with retained job/artifact metadata; SDK checks use one canonical TypeScript package build. | Verified locally and hosted archive matrix |
| RF-035 | `crates/slskr-web/Cargo.toml:17-18` now pins lock revision `3825c9ad5e4ace15bb210012e79b2cbdbfc20434`. Locked WASM/package checks, shellcheck, npm policy, and dependency audits pass locally. | Dependency and policy drift is now covered by the local release surfaces. | Hosted `Lint GitHub workflows` executes actionlint successfully in clean CI `36405007921` at `7a064d37`; full job/step receipt and locked metadata retained. | Verified hosted workflow/dependency gates at `7a064d37` |
| RF-036 | `crates/slskr-protocol/src/server.rs` repeated server-code variants, inventory metadata, numeric conversion, and direction-specific matches. | Adding a protocol code can update one table and miss another. | A declarative macro owns the variants, numeric values, inventory order, names, and numeric decoding. The 103-code two-direction test inventory classifies every code as typed or opaque and fails if dispatch drifts. All 25 server protocol tests pass. | Verified locally |
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
| RF-047 | Transfer queue mutations previously performed full JSON rewrite, fsync, and event append while holding the async transfer write lock. The current path snapshots pending events/state and flushes durability after the lock. | Slow disks previously stalled transfer/API readers and download chunks repeated durable full-state work. | A file-backed append paused immediately before fsync permits a concurrent queue mutation, then persists its ordered events and restart state. A separate writer process persists 32 transfers, is killed without shutdown, and a fresh process rehydrates all 32 and their 64 events. The 642-test daemon library suite passes. An actual isolated FUSE mount delays event-file fsync by 500 ms: queue access remains available, and three killed writers recover 24 transfers and all 48 ordered events. The fixture and source-bound evidence are retained. | Verified locally; mounted delay and crash stress |
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
| RF-058 | App navigation still sets state when flags are unchanged; render mutates title/classes; MediaCore and Integrations remain multi-thousand-line ownership boundaries. | Shell updates rerender broad trees and large workflows remain hard to profile/change. | Navigation skips unchanged activity state, and title/theme DOM synchronization runs in lifecycle methods. Profiler evidence led to memoized message-search and external-ID resolver panels, then Batch 300 moved PodCore DHT Publishing state, actions, and UI into a memoized local-state panel. Its 20-update JSDOM profile improved from 9.425 ms median/13.968 ms p95 to 1.026 ms/1.167 ms; the same measurement passed in the focused MediaCore suite (18 tests), and the Web build plus targeted lint pass. The MediaCore composition file fell from 8,547 to 8,192 lines. The last recorded full Web suite passed 876 tests; it was not rerun for Batch 300. Batch 301 extracted Pod Membership Management state, actions, and UI into `PodMembershipManagementPanel.jsx`, retaining the existing shared verification loading flag through a parent prop. Its 20-update JSDOM profile improved from 9.308 ms median/17.579 ms p95 to 1.533 ms/1.909 ms. The focused MediaCore suite passes 19 tests, and the Web build, targeted lint, and diff checks pass. The composition file is now 7,676 lines. Batch 302 extracted Pod Discovery state, actions, and UI into `PodDiscoveryPanel.jsx`. Its 20-update JSDOM profile improved from 8.254 ms median/16.453 ms p95 to 2.033 ms/2.626 ms. The repeated focused MediaCore run passes 20 tests; the Web build, targeted lint, and diff checks pass. The composition file is now 7,112 lines. Batch 303 extracted Pod Join/Leave state, actions, and UI into `PodJoinLeavePanel.jsx`. Its two baseline runs measured 8.650–9.208 ms median (one run had a JSDOM stall affecting p95); after extraction it measured 1.415 ms median/1.854 ms p95. All 21 focused MediaCore tests pass; the Web build, targeted lint, and diff checks pass. The composition file is now 6,734 lines. Batch 304 extracted Pod Message Routing into `PodMessageRoutingPanel.jsx` and its duplicated search-index/vacuum controls into shared `PodMessageMaintenanceActions.jsx`. The 20-update JSDOM profile improved from 8.088 ms median/13.211 ms p95 to 1.361 ms/1.723 ms. All 22 focused MediaCore tests pass; the Web build, targeted lint, and diff checks pass. The composition file is now 6,266 lines. Batch 305 moved Pod Message Backfill state, validation/actions, and last-seen timestamp projection into `PodMessageBackfillPanel.jsx`. Its 20-update JSDOM profile improved from 7.433 ms median/13.452 ms p95 to 0.596 ms/0.923 ms. All 23 focused MediaCore tests pass; the Web build, targeted lint, and diff checks pass. The composition file is now 5,992 lines. Batch 306 moved Pod Channel Management state, actions, current-pod projection, and UI into `PodChannelManagementPanel.jsx`. Its stable 20-update JSDOM profile improved from 7.813 ms median/13.184 ms p95 to 0.615 ms/0.817 ms; a second baseline run had an environment stall and is recorded in the artifact. All 24 focused MediaCore tests pass; the Web build, targeted lint, and diff checks pass. The composition file is now 5,649 lines. Batch 307 moved Pod Content Linking state, actions, and UI into `PodContentLinkingPanel.jsx`; the shared `contentId` remains in the parent because Content Registry clears it after registration. Its 20-update JSDOM profile improved from 7.040 ms median/14.013 ms p95 to 0.548 ms/0.801 ms. All 25 focused MediaCore tests pass; the Web build, targeted lint, and diff checks pass. The composition file is now 5,346 lines. Batch 308 extracted Pod Opinion Management and aggregation into `PodOpinionsPanel.jsx`; its 20-update JSDOM profile improved from 7.002 ms median/16.247 ms p95 to 1.149 ms/1.493 ms. All 26 focused MediaCore tests, the Web production build, targeted ESLint, and diff checks pass, and the composition file is now 4,527 lines. Batch 309 extracted Pod Message Signing into `PodMessageSigningPanel.jsx`; shared verification input and result state remain parent-owned because other verification workflows use them. Its 20-update profile improved from 6.138 ms median/14.396 ms p95 to 1.068 ms/1.468 ms. All 27 focused MediaCore tests and the Web build, targeted ESLint, and diff checks pass. Batch 310 extracted the MediaCore statistics dashboard state, actions, and cards into `MediaCoreStatisticsDashboardPanel.jsx`. Two baseline profiles measured 5.530–5.599 ms median, with JSDOM stalls raising p95 to 66.517–66.832 ms; after extraction it measured 1.969 ms median/2.468 ms p95. All 28 focused MediaCore tests, the Web build, targeted ESLint, and diff checks pass. The composition root is now 3,469 lines. Batch 311 split Content Descriptor Publishing and Retrieval into local-state panels under a 14-line `MediaCoreDescriptorManagementPanel.jsx` composition component. The 20-update JSDOM input profile improved from baseline medians of 11.344–11.689 ms (p95 included repeatable stalls at 42.541–43.528 ms) to 1.969 ms median/2.061 ms p95. All 29 focused MediaCore tests, the Web build, targeted ESLint, and diff checks pass. The composition root is now 2,461 lines. Batch 312 extracted the ContentID Registry and Content Graph workflows into `MediaCoreContentRegistryPanel.jsx` and `MediaCoreContentGraphPanel.jsx`. The registry panel keeps registration form state local while the parent owns its polled summary and the ContentID shared with Pod linking. The two 20-update input profiles improved from 4.906/22.814 ms to 1.776/2.237 ms and from 5.523/23.195 ms to 1.196/1.502 ms median/p95. All 31 focused MediaCore tests, the Web build, targeted ESLint, and diff checks pass. The root is now 1,851 lines. Batch 313 extracted Metadata Portability state, conflict-strategy loading, actions, and cards into `MediaCoreMetadataPortabilityPanel.jsx`. Its profile improved from two variable baselines (5.827–7.502 ms median and 14.822–15.078 ms p95, with event-loop stalls) to 1.124/1.343 ms median/p95. The focused 32-test suite passed on its final run; the first full run had an unrelated Pod Opinions refresh failure, which passed alone and on the full rerun. The Web build, targeted ESLint, and diff checks pass. The root is now 1,535 lines. Batch 314 moved Pod Membership Verification state, actions, and UI into `PodMembershipVerificationPanel.jsx`. The existing loading flag remains shared with Pod Membership Management, and the message input remains shared with Message Signing. Its profile moved from two noisy baselines at 9.419–10.753 ms median and 10.707–12.026 ms p95 to 1.378/1.647 ms. All 33 focused MediaCore tests, the Web build, targeted ESLint, and diff checks pass. The composition root is now 1,177 lines. Batch 315 moved Pod Message Storage state, stats/cleanup actions, retention UI, and its existing search and maintenance panels into `PodMessageStoragePanel.jsx`. Its 20-action profile improved from two noisy baselines (7.426/7.779 ms and 8.518/9.123 ms median/p95) to 0.442/0.696 ms. The final focused 34-test suite passes; one Pod Opinions statistics refresh assertion failed on the first full run, then passed alone and in the full rerun. The Web build, targeted ESLint, and diff checks pass. The composition root is now 1,062 lines. Batch 316 moved audio/image hashing state, actions, and cards into `MediaCoreContentHashingPanel.jsx`. Supported-algorithm metadata stays parent-owned because its summary is rendered after other workflows; the hashing panel receives it as a stable prop. Its profile improved from two baselines (6.531–6.739 ms median and 7.305–8.212 ms p95) to 1.037/1.144 ms. All 35 focused MediaCore tests, the Web build, targeted ESLint, and diff checks pass. The composition root is now 820 lines. Batch 317 moved raw hash comparison, fuzzy content search, perceptual ContentID comparison, and text similarity state/actions/cards into `MediaCoreSimilarityAnalysisPanel.jsx`. Its isolated hash-input profile improved from two baselines (4.932–5.719 ms median and 5.204–5.995 ms p95) to 1.909/2.335 ms; a full-suite profile sample was lower at 0.550/0.723 ms, showing JSDOM timing variance. All 36 focused MediaCore tests, the Web build, targeted ESLint, and diff checks pass. The root remains 369 lines. Batch 318 split the 518-line similarity owner into a 16-line composer plus four independent memoized owners for raw hash comparison, fuzzy content matching, perceptual comparison, and text similarity (144–175 lines each). The hash-input profile improved from 1.909/2.335 ms to 0.558/0.670 ms median/p95 after the finer split. The final full MediaCore suite passes all 36 tests; the Web build, targeted ESLint, and diff checks pass. MediaCore ownership extraction is complete. Batch 319 moved shared option/form/async helpers to `integrationsShared.jsx` and the Spotify, YouTube, and Last.fm settings workflow to `SourceFeedIntegrationsPanel.jsx`. The Integrations composer fell from 3,915 to 3,264 lines. Spotify client-ID input profiling remained stable (before 1.651–1.657 ms median / 1.859–1.937 ms p95; after 1.626/2.023 ms), consistent with its state already being local to a React panel before the file split. All 18 focused Integrations tests, the Web build, targeted ESLint, and diff checks pass. Batch 320 moved Notification state, apply/save actions, and UI into `NotificationIntegrationsPanel.jsx`; the composition root fell from 3,264 to 2,628 lines. Its 20-update JSDOM profile stayed within the observed range: two before runs measured 2.099/2.870 ms and 2.121/2.840 ms median/p95; two after runs measured 2.210/2.858 ms and 2.243/3.109 ms. All 19 focused Integrations tests, the Web production build, targeted ESLint, and diff checks pass. Batch 321 moved Metadata and Servarr configuration form construction, state, actions, and UI into `MetadataSettingsPanel.jsx`; the root fell from 2,628 to 1,830 lines. The MusicBrainz user-agent input profile stayed within JSDOM variation (two before medians 3.007/2.908 ms, p95s 3.280/3.167; two after medians 2.964/3.003 ms, p95s 3.389/3.599). All 20 focused Integrations tests pass, with build, targeted ESLint, and diff checks. Batch 322 moved FTP encryption choices, form construction, and apply/save workflow into `FtpIntegrationPanel.jsx`; the root fell from 1,830 to 1,417 lines. Its profile overlapped the before range (two before runs at 1.478/1.751 ms and 1.396/1.720 ms median/p95; two after runs at 1.492/1.772 ms and 1.608/1.992 ms). All 21 focused Integrations tests, the build, targeted ESLint, and diff checks pass. Batch 323 moved the VPN readiness/status view into `VpnPanel.jsx`; the root fell from 1,417 to 1,235 lines. Its 20-update state-prop profile covers the full Integrations tree and stayed noisy: before medians were 6.521/6.698 ms with p95 12.599/14.397; after medians were 6.967/6.320 ms with p95 12.943/13.647. All 22 focused tests, the build, targeted ESLint, and diff checks pass. Batch 324 moved Lidarr live status, wanted sync, import history/retry, and manual import state/actions into `LidarrPanel.jsx`; the root fell from 1,235 to 884 lines. The import-directory profile kept similar medians (before 1.214/1.238 ms, after 1.206/1.228 ms) while p95 was lower in both after runs (1.329/1.404 ms versus 1.622/1.795 ms). All 23 focused Integrations tests, build, targeted ESLint, and diff checks pass. Batch 325 moved Media Server adapter readiness, path diagnostics, preview, and execution-contract state/actions/UI into `MediaServerPanel.jsx`; the root fell from 884 to 439 lines. Its profile overlapped the baseline (before medians 3.902/3.704 ms, p95 4.832/4.181; after medians 3.996/3.650, p95 5.000/4.227). All 24 focused tests, build, targeted ESLint, and diff checks pass. Batch 326 moved Servarr readiness checks, compatibility preview, copy, and wanted-sync actions into `ServarrReadinessPanel.jsx`; the root fell from 439 to 211 lines. Its prop-update profile remained within the broad baseline range (before medians 7.853/8.156 ms, p95 18.247/17.175; after medians 7.690/7.696, p95 17.994/16.682). All 25 focused tests, build, targeted ESLint, and diff checks pass. Batch 327 moved Federation Diagnostics async loading, error handling, state, and read-only posture UI into `FederationDiagnosticsPanel.jsx`. The Integrations root is now a 61-line composer, down from 3,915 lines. The diagnostics-load profile median was 0.924/0.896 ms before and 0.751/0.808 ms after; p95 varied from 1.367–1.538 ms before to 1.316–1.606 ms after. All 26 focused tests pass, including the now-awaited async notification success assertion; Integrations ESLint, Web build, and diff checks pass. MediaCore is 369 lines. RF-058 ownership extraction is complete. | Verified locally |
| RF-059 | PlayerBar previously statically imported optional Visualizer/RustyMilk; System eagerly imported most tabs; dashboard eagerly imported all pages. The current batches lazy-load the visualizer and dashboard routes and load all remaining System panes by section. | Users previously paid parsing costs for optional features/routes. | The System entry chunk fell from 236.95 KiB to 9.17 KiB; Vite emits six section chunks plus independent AdminPolicies, Integrations, and MediaCore chunks. Initial JavaScript (1,084.68 KiB) and all-JavaScript gzip (593.07 KiB) pass their 1,150/600 KiB budgets. The route-switch regression and full Web suite (873 tests across 147 files) pass. The asset graph is retained in `benchmarks/artifacts/20260924-system-pane-asset-graph.md`; deployed-device evidence remains open. The kspls0 audit covered eight desktop/mobile route views: all documents and 178 API responses were HTTP 200, with no tested control overlaps. Four inline-style CSP errors appeared on System and Integrations. Source review found nonce metadata was coupled to optional runtime-profile disclosure; nonce injection is now independent, with a targeted Rust regression passing locally. A deployed rerun and device-performance evidence remain open. | In progress; deployed CSP and device-performance evidence open |
| RF-060 | Browse tree repeatedly filtered full directory lists and compared selection by set size; normalized name handling differed. The current batch builds parent lookups, memoizes sanitized trees, compares set membership, and tests wide/deep/file-heavy malformed fixtures plus same-sized replacements. | Large shares previously stalled and same-size replacement trees could retain stale selection. | Browse fixture, replacement-tree, and full Web tests pass. | Verified locally |
| RF-061 | Shared semantic wrappers used clickable divs/icons without keyboard/accessible-name semantics; dashboard navigation lacked `aria-current`. The current batch adds keyboard activation, icon roles, active navigation state, explicit browse control names, and axe checks for representative Web/dashboard shells. | Keyboard and screen-reader users previously missed controls and context. | Semantic, Browse, Sidebar, and axe tests pass; deployed-browser/device coverage remains separate. | Verified locally; deployed evidence separate |
| RF-062 | Web mounted `App` without an error boundary; route try/catch could not catch descendant render failures. The current batch adds a reloadable render fallback. | One render error previously could blank the entire UI. | ErrorBoundary test and full Web tests pass. | Verified |
| RF-063 | Web audit records responses but not request-count/cadence/accessibility budgets. The current batch preserves per-asset budgets, adds deterministic aggregate initial-JS and gzip budgets for Web/dashboard, adds local axe checks for representative shells, and records Rust Web request-count/cadence budgets across 15 desktop/mobile route pairs. | Duplicate requests, hidden-tab churn, accessibility, and aggregate regressions previously could pass. | Web/dashboard aggregate budgets, representative axe checks, and Rust Web mock request-count/cadence budgets pass locally. Hosted Live Parity run `36460398465` recorded all 30 views but exposed the wishlist budget missing one intentional post-action list refresh: desktop made 11 requests against a 10-request budget, with the repeated list request 812 ms apart. The budget now records 11 for that action path; the corrected full local audit passes all 30 views with zero errors; hosted run `36464045259` on `c02c77bc` also passed all views and retained the audit under [`live-parity-36464045259`](https://github.com/snapetech/slskr/actions/runs/36464045259/artifacts/10988987235). A separate isolated live-backend audit recorded 469 requests across 30 route views, with zero route-budget or sub-200 ms cadence violations; its strict exit was due only to protected YAML (403) and unconfigured Lidarr (503) responses. The sanitized evidence is `benchmarks/artifacts/20260927-rust-web-live-backend-cadence.md`. Full deployed accessibility remains open. | In progress; hosted 30-view audit passed; deployed accessibility open |
| RF-064 | `AppContext.js` is active around the routed App; legacy `Pods.jsx` is not the `/pods` route; unused-symbol lint is disabled; dashboard still has separate context/prop ownership to review. | Dead modules and competing ownership patterns dilute refactor/test signal. | `docs/dev/web-ownership-inventory.md` records active versus deferred ownership, and `scripts/check-web-ownership-inventory.sh` verifies the route/context/import boundary. Legacy `Pods.jsx` deletion remains explicitly deferred. | Verified locally, deletion deferred |
| RF-065 | `LyricsPane.jsx:148-159` used both `timeupdate` and a 500 ms interval for the same position update. The current batch removes the duplicate timer and adds a regression test. | Continuous duplicate state checks previously ran while lyrics were visible. | Focused LyricsPane test passes; keep the full Web suite as proof. | Verified |

### P2/P3 Evidence And Gate Hygiene

| ID | Evidence | Impact | Action and validation | Status |
| --- | --- | --- | --- | --- |
| RF-066 | Fixture fetchers previously recorded observed hashes instead of verifying expected values. The current batch adds a manifest checker, checksum/size enforcement, corruption regression, and one root fetch wrapper. | Remote fixture drift previously could silently change E2E/share behavior. | Fixture manifest/corruption tests pass; serialized E2E passes with static fixtures. Repository discovery and media file-presence checks are corrected and tested. Genuine Sintel and Aria downloads are pinned and verified. Five real sharing cases and two range/seek cases pass, including decoded H.264 browser playback. Recipient backfill now uses authenticated, certificate-pinned MeshContent on the shared native peer port; grant/recipient/permission and file/range limits are enforced, downloaded bytes are SHA-256 verified before no-overwrite publication, and a real three-peer E2E verifies the exact downloaded fixture. Per-grant ticketed stream admission now has a focused local regression proving a second stream is rejected at limit one, another grant retains independent capacity, and dropping a lease releases capacity. Fresh deployed-device concurrency evidence remains open. | In progress; recipient backfill and owner stream admission locally verified, hosted/deployed evidence open |
| RF-067 | GitHub and GitLab test jobs now run the bounded docs-freshness and active-plan-freshness checks plus their negative tests before the Rust matrix. | Scope and maintained-guidance regressions are now found during review instead of only at release. | Local docs-freshness and active-plan checks pass. GitHub CI run `36467270728` completed its Rust job successfully on `6e9a6c00`; both freshness checks and their negative regressions passed. GitHub CI run `36471242977` on `85b43091` passed the Rust job, including both freshness checks and their negative regressions; macOS x64 remains in progress. The GitLab repository now accepts pushes, but no pipeline appeared after the repair; latest project pipeline remains #81 from 2026-05-17. The latest pushed GitLab tip `b9679f30` also has no pipeline record: post-receive returned HTTP 200, but project 44 still has #81 (2026-05-17) as its newest pipeline. The MCP API returns 401. The investigation is retained in `benchmarks/artifacts/20260928-gitlab-postreceive-pipeline-status.md`. | In progress; GitHub freshness checks passed; GitLab pipeline has not started |
| RF-068 | `benchmarks/README.md` and `docs/performance-analysis.md` describe manual one-off scripts without stored JSON baselines, thresholds, or a CI/nightly target. | Performance drift has no reproducible regression signal. | The benchmark docs now explicitly classify the commands as diagnostic-only and require retained JSON plus environment metadata for release evidence. | Verified locally, diagnostic-only |
| RF-069 | `web/package.json` exposes RustyMilk compatibility/performance/smoke scripts, but no workflow invokes them while `slskr-web` tracks the upstream main branch. | Web dependency drift can pass without compatibility evidence. | The performance note explicitly classifies these scripts as diagnostic-only; a scheduled threshold job remains intentionally absent. | Verified locally, diagnostic-only |
| RF-070 | Council count files had no scan date/commit and checks regenerated only when a report was missing. | Stale counts could pass active-backlog checks. | `run-council-scan.sh` stamps the report; `check-council-freshness.sh` validates date and the SHA-256 digest of tracked scan inputs across the report, inventory, and backlog, with a negative test and all-phases/remediation wiring. Generated records are excluded so committing them cannot invalidate their own stamp, including on shallow CI checkouts. | Verified locally |
| RF-071 | The audit PASS matrix records a SHA but not toolchain/node/npm versions, lock hashes, or retained artifact links. | A clean checkout cannot reproduce the evidence exactly. | The collector validates source/run identity, observed tool versions, required lock/config and artifact hashes, and worktree state. The GitHub Rust gate now retains metadata, coverage summary, and the Web entry point for 30 days with a direct job-summary link. Twelve regressions and local policy gates pass. The successful Rust job at `37f2e049` retained the artifact; downloaded metadata, artifact hashes, and all seven lock/three configuration hashes match that exact clean checkout. | Verified main-gate hosted retention |
| RF-072 | `docs/live-interop-test-matrix.md:39,119` referenced absent `web/e2e/live-surfaces.spec.ts`; the current batch updates it to maintained specs and keeps historical results explicitly dated. | Operators previously could not reproduce the stated live matrix. | Docs freshness and maintained Playwright pass locally. The 2026-09-27 local matrix passed four login, one local-peer, and two social probes; its sanitized result and raw TSV hashes are in `benchmarks/artifacts/20260927-live-interop-summary.md`. On 2026-09-28, the credentialed matrix passed locally and on hosted Live Parity after validating eight unique test accounts. Hosted run [`36499213873`](https://github.com/snapetech/slskr/actions/runs/36499213873) passed both the Rust UI/API job and the credentialed public interop job; the retained interop and full parity artifacts are linked in `benchmarks/artifacts/20260928-live-interop-hosted-summary.md`. | Verified locally and hosted |
| RF-073 | `scripts/run-release-gate.sh:69-74` makes the slskd API compatibility smoke opt-in; `docs/release.md:41-63` intentionally assigns that smoke to scheduled/manual Live Parity. The current batch labels the skipped local check as a certification artifact requirement. | This is a policy boundary, not a code defect, but release certification could be misread if artifact freshness was not visible. | Manual run `36456533118` retained the UI audit artifact but exposed a stale cached Python target that hid `SlskdClient`; a fresh local reproduction confirmed the target collision. The smoke now installs its pinned client into unique temporary state, clears any stale summary at start, isolates config/state, keeps generated private keys outside artifacts, disables its unused HTTPS listener, and uses an available loopback port for the shared native peer TCP/UDP listener. The local smoke passes all 91 API calls. Manual Live Parity run [`36464045259`](https://github.com/snapetech/slskr/actions/runs/36464045259) passed the corrected UI audit and API smoke on `c02c77bc`; retained artifact [`live-parity-36464045259`](https://github.com/snapetech/slskr/actions/runs/36464045259/artifacts/10988987235) contains the passing 91-call summary. Its credentialed public interop job skipped because `SLSKR_LIVE_INTEROP_ENV` is absent; that matrix proof remains open under RF-072. | Verified hosted API smoke; credentialed interop tracked in RF-072 |

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
The user selected the single-port policy. The shared demultiplexer now rejects
a prefix whose plain and obfuscated forms both advertise known init forms,
before it can return the shorter frame and desynchronize the stream. This
rejects the exact dual-valid legacy sequence because sender intent is absent
from the wire bytes. A coordinated wire discriminator would be required to
accept that sequence safely. Ordinary plain and obfuscated handshakes retain
the shared port; the random obfuscated writer avoids plain-length keys. All 29
listener tests pass, including a TCP collision test. Extension and nested-form
collisions remain a separate legacy framing limit under the current parser.

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
rollback regression passes with `full-controller-tests` alone. Formatter and
runtime-boundary guard pass. Scoped full-controller/legacy Clippy with
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

## RF-024 Listener State Ownership (Batch 126, 2026-09-25)

Moved `ListenerSnapshot`, `ListenerCommand`, snapshot defaults, listener JSON projection, and public error mapping into `listener_state.rs`. Bind configuration defaults, response fields, and redacted error text remain unchanged. Listener task orchestration, bind reconfiguration, and runtime mutation ordering stay at their existing call sites.

The default library check and 638-test suite pass. Feature-enabled listener error-redaction and read-only API contract regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Distributed Runtime State Ownership (Batch 127, 2026-09-25)

Moved `DistributedRuntime`, persistence snapshot/status models, connection roles, constructors/reset/depth helpers, state projections, and persisted-state load/restore/save methods into `distributed_state.rs`. Durable revision tracking, snapshot shapes, error mapping, and target-specific application projections remain unchanged. The watch worker, persistence completion sequencing, shutdown flush, and network mutation workflows stay at their existing call sites.

The default library check and 638-test suite pass. The feature-enabled distributed filter passes all 14 matching persistence, restart, branch/depth, child-socket, and shutdown regressions. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Storage Directory Listing State Ownership (Batch 128, 2026-09-25)

Moved `StorageDirectoryListOptions` and `StorageDirectoryListState`, including query-derived recursion defaults and emitted/truncated accounting, into `storage_directory_state.rs`. Direct/recursive listing budgets and output behavior remain unchanged. Filesystem traversal, path confinement, ordering, recursion, and response assembly stay at their existing call sites.

The default library check and 638-test suite pass. Six feature-enabled listing regressions pass for bounded output, symlink confinement, route query behavior, recursion budget/depth, and storage root response shape. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Scanner Ownership (Batch 129, 2026-09-25)

Moved `ShareScan`, its options/request models, scan results, cancellation-aware index construction, ordered root grouping, and directory scanning into `share_scanner.rs`. Entry/depth/descriptor caps, cancellation checks, symlink handling, deterministic ordering, and result/error shapes remain unchanged. Share-scan task registration, runtime lifecycle updates, persistence/commit ordering, and controller route orchestration stay at their existing call sites.

The default library check and 638-test suite pass. Eleven feature-enabled scanner and index regressions pass for cancellation, bounds, symlink handling, alias/exclusion behavior, metadata probing, deterministic workers, persistence/rehydration, stale scan rejection, and case-sensitive filtering. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Scanner Helper Ownership (Batch 130, 2026-09-25)

Moved virtual path normalization, component sanitization, hidden-path detection, safe share labels, extension extraction, and bounded WAV/FLAC/MP3 media attribute probing into `share_scanner.rs`. Path projection, media attribute values, and probe byte limits remain unchanged. The shared virtual-path and extension helpers remain available through the crate-root imports used by controller routes and tests.

The default library check and 638-test suite pass. Twelve feature-enabled scanner/index regressions pass for path projection, media probing, cancellation, bounds, symlink handling, deterministic workers, filters, persistence/rehydration, and route fallback. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Index Persistence Ownership (Batch 131, 2026-09-25)

Moved persisted share-file projections, persisted root summaries, share-attribute encoding/decoding, compatibility cache paths, TSV serialization, and cache-write status projection into `share_index_state.rs`. The versioned cache format, escaping, update timestamp, redacted error shape, and atomic replacement behavior remain unchanged. The generic atomic writer and route/lifecycle persistence ordering remain shared at their existing call sites.

The default library check and 638-test suite pass. Nine feature-enabled cache/index regressions pass for cache escaping and disablement, SQLite rehydration, cache error redaction, persistence rollback, rescan, runtime share updates, and catalog output. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Session Command DTO Ownership (Batch 132, 2026-09-25)

Moved the `SessionCommand` and `SearchDispatchTarget` DTOs into `session_state.rs`; the root import keeps command senders, producers, the session manager, and existing tests on their current paths. Variant payloads and derives are unchanged. Session execution, dispatch order, command-channel capacity, and lock lifetimes remain at their existing call sites. The locked default library check and 638-test suite pass. The focused wishlist search/restart regression passes with `full-controller-tests`; the combined `full-controller-tests,legacy-route-dispatch` invocation overflows the historical test worker before assertions, then passes with `full-controller-tests` alone. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Search Result Projection Ownership (Batch 133, 2026-09-25)

Moved `SearchResultEntry`, its bounded text normalization, file-attribute extraction, and search/controller JSON projections into `search_store.rs`. Search response shapes, metadata fallbacks, field caps, and controller file projection stay unchanged. Existing root, route, wishlist, transfer-recovery, and test call sites keep using the crate-root import. The locked default library check and 638-test suite pass. Four feature-enabled search-store regressions pass for response merging, peer result caps, bounded result text/aggregate size, and controller IDs; the locked-file controller projection regression also passes. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Catalog Filter Ownership (Batch 134, 2026-09-25)

Moved `CatalogFilter` query parsing and matching into `share_index_state.rs`, beside `ShareIndexSnapshot::catalog_json`. Query keys, case/extension normalization, default and capped limits, offset fallback, and matching behavior remain unchanged; the crate-root import is retained only for the full-controller compatibility test. The locked default library check and 638-test suite pass. The feature-enabled `list_limits_are_bounded_by_default` regression passes with `full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Local File Hash Cache Ownership (Batch 135, 2026-09-25)

Moved local SHA-256 streaming, cache-entry state, the 4,096-entry bound, and size/mtime validation into `local_file_hash.rs`. Digest format, I/O chunk size, cache key, invalidation checks, and eviction behavior remain unchanged; content-ID resolution keeps its crate-root call site. The new module is covered by the runtime-boundary guard. The locked default library check and 638-test suite pass. The feature-enabled native-library SHA-256 content-ID regression passes with `full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Rate-Limit Policy Ownership (Batch 136, 2026-09-25)

Moved `ControllerRateLimitPolicy`, native-profile path/auth partition selection, and permit/window normalization into `rate_limit.rs`, beside the rate limiter. Partition names, path precedence, caller bypass rules, warm-cache keys, non-positive limits, fallback window, and endpoint-specific caps remain unchanged; request authorization and bucket mutation remain at their existing call sites. The locked default library check and 638-test suite pass. All 34 feature-enabled rate-limit regressions pass, covering partitions, auth precedence, trusted-proxy routing, bounds, and reload projection. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Wishlist Auto-Download Policy Ownership (Batch 137, 2026-09-25)

Moved `WishlistAutoDownloadPlan`, result-quality scoring and grouping, track identity helpers, edition matching, and wishlist directory normalization into `wishlist_store.rs`. Quality tuple ordering, format preferences, filename normalization, and edition checks remain unchanged. `auto_download_completed_wishlist` keeps transfer staging, persistence, rollback, and command delivery at the existing lifecycle call site. The locked default library check and 638-test suite pass. The feature-enabled `wishlist_auto_download_enqueues_best_folder_and_applies_one_shot_limit` regression passes. Scoped Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only and adds no release-note fragment. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Integration Target Ownership (Batch 138, 2026-09-25)

Moved `ResolvedIntegrationTarget`, integration and Lidarr URL validation, and integration special-use IP checks into `integration_target.rs`. Existing crate-root imports preserve callers and test construction. Address resolution, resolved-address pinning, error strings, the private-address environment override, and Lidarr's unspecified/multicast checks remain unchanged. The module is included in the runtime-boundary guard.

The locked default library check and 638-test suite pass. The feature-enabled `integration_ssrf_filter_blocks_special_use_ip_ranges` and `notification_integrations_emit_frozen_wire_requests` regressions pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Source Feed Ingestion Ownership (Batch 139, 2026-09-25)

Moved source-provider matching, loose source-row creation, bounded byte/JSON response readers, provider metadata URL validation and fetching, and the bounded HTML metadata parser into `source_feed_ingest.rs`. Provider host matching, the 512 KiB response cap, metadata precedence, URL checks, disabled redirects, DNS resolution pinning, and request headers remain unchanged. Root imports preserve source orchestration, local RSS/OPML parsing, Spotify/AcoustID helpers, and test call sites. The new module is included in the runtime-boundary guard.

The warning-free locked default library check and 638-test suite pass. Seven feature-enabled provider regressions pass for host spoofing, metadata fallback, declared/chunked response limits, provider fetch contracts, and the frozen catalog/edge differentials. The feature-enabled local-format parsing regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Local Source Feed Parser Ownership (Batch 140, 2026-09-25)

Moved local source-kind detection, CSV/playlist/XML parsing, bounded preview item parsing, and local preview projection into `source_feed_ingest.rs`. CSV headers and album handling, M3U/RSS/OPML row order and IDs, entity decoding, namespace matching, the 10,000-row limit, and preview output remain unchanged. Root imports preserve route call sites. The locked default library check and 638-test suite pass. The feature-enabled `source_feed_local_formats_match_frozen_parsing_and_deduplication` regression and `controller_api_differential_bridge_admin_stats_and_source_feed_preview` pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Lidarr API Helper Ownership (Batch 141, 2026-09-25)

Moved Lidarr system-status and wanted-release clients, the shared bounded JSON GET helper, and quality-profile filter construction into `lidarr_api.rs`. URL validation, resolved-address pinning, request timeout/header behavior, page bounds, error mapping, and quality selection remain unchanged. Sync, import, wishlist mutation, and persistence orchestration stay at their existing call sites. The module is included in the runtime-boundary guard.

The warning-free locked default library check and 638-test suite pass. The feature-enabled `controller_api_differential_lidarr_and_source_feed_contracts` regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Lidarr Manual Import Helper Ownership (Batch 142, 2026-09-25)

Moved Lidarr path mapping, candidate validation, portable filename extraction, already-owned lookup, manual-import candidate requests, and rejected-file deletion into `lidarr_import.rs`. Virtual/POSIX and Windows path mapping, accepted rejection reasons, filename deduplication, pinned requests, and canonical-directory confinement remain unchanged. Import workflow, transfer/wishlist mutation, persistence, and rollback stay at their existing call sites. The module is included in the runtime-boundary guard.

The warning-free locked default library check and 638-test suite pass. Twelve feature-enabled Lidarr regressions pass for candidate selection, path mapping, confined deletion, projection, scheduler state, rejection policy, manual-import rollback and lock release, and the controller differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Lidarr Import History Projection Ownership (Batch 143, 2026-09-25)

Moved the Lidarr import-history key prefix and builder, result-status classifier, history-record construction, and public record redaction into `lidarr_import.rs`. Persisted key names, record fields, timestamps, retry linkage, and removal of the private source directory from public output remain unchanged. Controller-feature persistence and history listing stay at their existing call sites.

The warning-free locked default library check and 638-test suite pass. Twelve feature-enabled Lidarr tests pass, including manual-import history through the controller differential, persistence-failure rollback, and lock-release regressions. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Lidarr Import History Lifecycle Ownership (Batch 144, 2026-09-25)

Moved Lidarr import-history persistence/listing and manual/automatic retry wrappers into `lidarr_import.rs` beside their record projections. History ordering, public redaction, persistence timing, success/error propagation, and retry linkage remain unchanged. Import execution and wishlist/transfer mutations remain at their existing call sites.

The warning-free locked default library check and 638-test suite pass. All 12 feature-enabled `lidarr_` regressions pass, including the controller differential, history projections, persistence-failure rollback, and lock-release checks. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Lidarr Import Execution Ownership (Batch 145, 2026-09-25)

Moved Lidarr completed-directory import execution, its manual wrapper, and automatic delay/retry loop into `lidarr_import.rs`. The per-directory debounce, ownership pre-check, import gate, candidate checks, import mode, retry count/backoff, and result/error behavior remain unchanged. Route, history persistence, and completion callbacks keep calling the module through crate-root imports.

The warning-free locked default library check and 638-test suite pass. All 12 feature-enabled `lidarr_` regressions pass, covering path/candidate/deletion behavior, import result contracts, rejection policy, rollback, lock release, and the controller differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Lidarr Manual Import API Ownership (Batch 146, 2026-09-25)

Moved the Lidarr manual-import candidate GET and import-command POST into `lidarr_api.rs` with the other HTTP clients. DNS resolution and pinning, API-key headers, timeouts, redirect/proxy policy, query/body shapes, response bounds, and error strings remain unchanged. `lidarr_import.rs` keeps the sequencing and result policy.

The warning-free locked default library check and 638-test suite pass. All 12 feature-enabled `lidarr_` regressions pass, including the candidate and import-command fixture requests in the controller differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Database Maintenance Ownership (Batch 147, 2026-09-25)

Moved database statistics projections, terminal-transfer and old-message cleanup, retention pruning and scheduler startup, and database vacuum handling into `database_maintenance.rs`. Existing route entry points and scheduler startup continue through crate-root imports. Persistence ordering, lock release points, retention filters, response shapes, and error handling remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Three feature-enabled database-cleanup regressions pass for transfer tombstones, SQLite-failure rollback, and message-projection ordering. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Browse Wire Ownership (Batch 148, 2026-09-25)

Moved bounded peer browse payload construction and parsing, virtual path composition, and folder grouping helpers into `browse_wire.rs`. Existing route and test call sites continue through crate-root imports. Wire layout, entry/count bounds, parse errors, legacy path decoding, and folder projection remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Seven feature-enabled `shared_file_list_payload` regressions and all 33 feature-enabled browse-filter tests pass; the folder contents path-selection regression also passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Static Web Asset Ownership (Batch 149, 2026-09-25)

Moved static asset root selection, request-path confinement, CSP/header construction, bounded file readers, and static response writing into `web_static.rs`. Existing HTTP handlers and test access continue through crate-root imports, with the original platform and test cfgs preserved. Root confinement, symlink handling, size limits, response headers, and SPA selection remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Eight feature-enabled static asset regressions pass for root confinement, symlink escape rejection, size and UTF-8 bounds, CSP behavior, and profile selection. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Options Projection Ownership (Batch 150, 2026-09-25)

Moved the controller options JSON projection into `controller_options_projection.rs`, beside the controller options state module. Root and route call sites continue through the crate-root import. Frozen defaults, profile-specific shapes, current/startup distinctions, secret redaction, and volatile-overlay handling remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Twenty-five feature-enabled options regressions pass for frozen defaults, startup aliases, redaction, current/startup projections, and overlay lifecycle. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Debug Projection Ownership (Batch 151, 2026-09-25)

Moved the controller options debug projection and its provider, path lookup, scalar formatting, and secret-redaction helpers into `controller_debug_view.rs`. Route and test call sites continue through the crate-root import. Provider selection, default values, sensitivity filtering, current/startup handling, and output shape remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Two feature-enabled debug-view projection regressions and the debug-gating regression pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller YAML Ownership (Batch 152, 2026-09-25)

Moved the controller YAML text response, bounded body validation/parser, key conversion, and API projection helpers into `controller_yaml.rs`. Route and state-module callers continue through crate-root imports; compatibility file I/O and configuration watching remain at their existing lifecycle call sites. YAML size/depth/node bounds, validation errors, target-specific projection, precedence, and response shape remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Twenty-one feature-enabled YAML regressions pass for target-specific validation, startup aliases, projection, native adversarial updates, and durable configuration behavior. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Release Check Ownership (Batch 153, 2026-09-25)

Moved release-tag normalization and version comparison, scheduled refresh, and latest-version response construction into `controller_release_check.rs`. The route and startup call sites continue through crate-root imports. Release URL selection, native/.NET version rules, request/error handling, and cached update projection remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Three feature-enabled regressions pass for frozen release comparison rules, the real latest-version lookup path, and the controller residual route differential. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 remains in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Capability and Network Helpers (Batch 154, 2026-09-25)

Moved native capability response projection, capability parsing and negotiation, network statistics projection, and peer capability descriptor projection helpers into controller_capabilities.rs. Root imports preserve route and mesh callers. Capability grammar, negotiation intersection, response shapes, and persisted descriptor behavior remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Ten feature-enabled capability contract tests and the native network-statistics edge contract pass. Scoped full-controller/legacy Clippy with -D clippy::await_holding_lock, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Storage Preflight Ownership (Batch 155, 2026-09-25)

Moved controller event/search read-failure responses, native search identifier and query validation, and HashDb/backfill database-availability checks into controller_storage_preflight.rs. Root imports preserve route-dispatch callers. Controller profile selection, response status and body text, accepted identifier aliases, query bounds, and database checks remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Four feature-enabled controller regressions pass for native event edges, native search compatibility, HashDb paging, and versioned backfill candidates. Scoped full-controller/legacy Clippy with -D clippy::await_holding_lock, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Storage Listing Ownership (Batch 156, 2026-09-25)

Moved shared/user file projections, share and user directory JSON, target response shaping, filesystem timestamp formatting, bounded storage-directory walking, Unix confined enumeration, scan/delete limits, and storage query helpers into controller_storage.rs. Root imports preserve route and delete-handler callers; storage_directory_state.rs continues to own the bounded emission state. Response fields, ordering, pagination behavior, path confinement, symlink handling, recursion depth, and entry caps remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test suite pass. Eight feature-enabled regressions pass for bounded and recursive listing, pagination handling, user-browse filtering and paging, API browse parity, and share/relay lifecycle. Scoped full-controller/legacy Clippy with -D clippy::await_holding_lock, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-002 Persistence-Turn Helper Review (2026-09-25)

Incoming wishlist search responses acquire the Wishlist persistence turn before
reading the result policy, then call `persist_search_result_delta`, which
acquires the Search turn. This keeps the Wishlist-then-Search order used by
ignored-result transactions. The split search-result dispatcher calls the
delta helper without holding the Wishlist turn. The event-history paths
inspected do not acquire Wishlist while holding the Event turn: API event
injection drops the turn after persistence, and `record_event` drops it before
session error handling and feed publication.

Automatic wishlist downloads previously kept the Wishlist turn while calling
`record_event`, which takes the Event turn. They now release the Wishlist turn
immediately after the item persists and before transfer commands and event
publication. The existing automatic wishlist-download regression passes (1/1),
as do the warning-free locked default library check, 638-test daemon library
suite, scoped full-controller/legacy Clippy with
`-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard,
plan freshness, release-note preview, and diff check. This internal lock-lifetime
change needs no release-note fragment.

This is partial helper-based evidence; other RF-002 paired-persistence and
helper-call paths remain to be reviewed. RF-002 remains in progress.

## RF-024 Controller YAML Target Validation Ownership (Batch 157, 2026-09-25)

Moved web-authentication, transfer, Soulseek connection/profile/distributed,
and aggregate target-specific YAML validation into `controller_yaml.rs`. The
crate-root import preserves route and test call sites. Validation order,
target-specific error text, accepted values, and parser bounds remain unchanged.
The runtime-boundary guard scans the expanded module.

The warning-free locked default library check and 638-test daemon library suite
pass. All 21 feature-enabled YAML regressions pass, including legacy/native
error contracts and durable configuration paths. Scoped full-controller/
legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This internal ownership move needs no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Fallback Dashboard Ownership (Batch 158, 2026-09-25)

Moved the fallback dashboard HTML template into `web_static.rs`. The public
crate-root `fallback_dashboard_html()` remains as a wrapper, preserving
existing route and external call sites. The rendered template, version
substitution, and route behavior remain unchanged.

The warning-free locked default library check and 638-test daemon library suite
pass. Both feature-enabled fallback-dashboard route regressions pass. Scoped
full-controller/legacy Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This internal ownership move needs no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release gate
remains after Batch 71.

## RF-024 PodCore Controller Response Ownership (Batch 159, 2026-09-25)

Moved the PodCore mutation, dynamic-get, and statistics response handlers into
`podcore_controller.rs`. Crate-root imports preserve both route dispatchers'
call sites; response shapes, route validation, persistence order, and error
mapping remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite
pass. Focused feature-enabled regressions pass for PodCore channel CRUD, stats
GETs, and content-metadata validation (3/3). Scoped full-controller/legacy
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This internal ownership move needs no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 MediaCore Controller Response Ownership (Batch 160, 2026-09-25)

Moved the MediaCore extended response and mutation response handlers into
`mediacore_controller.rs`. Crate-root imports preserve both route dispatchers'
call sites; route validation, response fields, metrics, and persistence flow
remain unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite
pass. The feature-enabled MediaCore mutation differential and descriptor
lifecycle differential pass (2/2). Scoped full-controller/legacy Clippy with
`-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. This internal
ownership move needs no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.

## RF-024 Extended Controller Response Ownership (Batch 161, 2026-09-25)

Moved the extended-controller mutation, search, download, dynamic-get, and get response handlers into `extended_controller.rs`. Crate-root imports preserve existing dispatcher call sites and classifiers remain at the route boundary. Response contracts and dispatch behavior remain unchanged; the runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite pass. Focused feature-enabled regressions pass for extended-controller mutation differential and native versioned extended GET contracts (2/2). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer Controller Response Ownership (Batch 162, 2026-09-25)

Moved transfer telemetry/storage failure responses, native transfer request validation, and auto-replace status/mutation responses into `transfer_controller.rs`. Crate-root imports preserve the legacy and bounded dispatcher call sites. Validation and persistence behavior remain unchanged; the runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite pass. The focused native auto-replace edge-contract regression passes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Index Runtime Lifecycle Ownership (Batch 163, 2026-09-25)

Moved share-scan cancellation coordination, bounded rebuild admission, runtime share addition/rollback, snapshot generation validation, persistence, and publication into `share_index_runtime.rs`. `share_index_state.rs` retains the lifecycle and snapshot types; route call sites remain available through crate-root imports. Scan, persistence-turn, rollback, and publication ordering are unchanged. The runtime-boundary guard now scans the module.

The warning-free locked default library check and 638-test daemon library suite pass. Four feature-enabled share rebuild regressions pass for concurrent-scan rejection, persistence rollback, shutdown cancellation, and error redaction (4/4). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 HTTP Connection Lifecycle Ownership (Batch 164, 2026-09-25)

Moved the HTTP connection wrapper and full request-stream lifecycle into `http_connection.rs`. Bounded HTTP parsing and response framing remain in `http_server.rs`; root imports preserve server and test call sites. Request limits, response handling, routing, logging, and keep-alive behavior remain unchanged. The runtime-boundary guard now scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. Feature-enabled request-stream regressions pass for the application-dump differential and filtered events total-count header (2/2). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal ownership move needs no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 File Transfer Runtime Ownership (Batch 165, 2026-09-25)

Moved transfer capability probing, accepted/indirect transfer execution, upload/download progress and retry helpers, completed-download permissions/content checks, and audio metadata extraction into `file_transfer_runtime.rs`. Root imports preserve peer, route, and test call sites; peer socket negotiation remains at the separate transport boundary. No transfer behavior changed. The runtime-boundary guard now scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. Focused feature regressions pass for inbound transfer request cases (3/3), completed audio metadata parsing (1/1), and the download service/path/retry differential (1/1). The inbound resume regression now expects the documented total remote position: two existing offset bytes plus two newly sent bytes. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This internal refactor and test expectation correction need no release-note fragment. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Peer Transport Ownership (Batch 166, 2026-09-25)

Moved peer socket setup, regular and obfuscated dialing, SOCKS5 negotiation, peer browse/message transport, and plain/obfuscated protocol negotiation into `peer_transport.rs`. Crate-root imports preserve current callers; folder parsing and peer transport types now live with the connection boundary. The runtime-boundary guard scans the new module. No runtime behavior changed. One existing resume fixture now seeds queued transfer progress through the active-status transition so its saved offset matches the scenario it tests.

The warning-free locked default library check and 638-test daemon library suite pass. Feature-enabled peer-address response regressions pass (8/8), including resume, and the obfuscated-transfer preference and remembered legacy-encoding regressions pass (2/2). Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. The fixture correction and ownership move are internal; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Legacy Route Dispatcher Core Ownership (Batch 167, 2026-09-25)

Moved the 18,479-line feature-gated legacy HTTP route handler from `lib.rs` into `legacy_route_dispatch.rs`. The crate-root wrapper still selects it under the same `legacy-route-dispatch` feature and passes through the same arguments and response. The runtime-boundary guard now scans the extracted module. Structural extraction lowered `lib.rs` below the formatter's generic size cutoff, so `check-rust-format.sh` explicitly retains the existing exclusion for this repository-wide formatting-debt root; the new module passes rustfmt directly. Runtime behavior is unchanged.

The warning-free locked default library check and 638-test daemon library suite pass. The locked full-controller/legacy library check and scoped Clippy with `-D clippy::await_holding_lock` pass. The changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. Existing runtime attempts through the legacy feature overflow the Tokio worker stack before assertions; a broader relay-open case remained in I/O polling beyond 60 seconds and was stopped. This matches the documented limitation of the historical monolithic dispatcher, so runtime parity remains unverified for this ownership-only move. The work and formatter exception are internal; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-002 Wishlist Search Failure Lock Scope Review (2026-09-25)

Reviewed `send_due_wishlist_search`: it persists the completed search first, then updates and persists the associated wishlist item under the wishlist/search persistence turn. If the wishlist write fails, the handler restores the wishlist and conditionally restores the search record in that order. The turn previously remained held while it awaited an unrelated session error projection. It is now released after both rollbacks and before `update_session`; persistence and compensation ordering are unchanged. Other RF-002 paired routes remain open.

The warning-free locked default library check and 638-test suite pass, along with scoped full-controller/legacy Clippy using `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check. This internal lock-lifetime change requires no release-note fragment. RF-002 remains in progress.

## RF-024 Daemon Startup Ownership (Batch 168, 2026-09-25)

Moved the daemon `serve` startup and service-orchestration function into `daemon_serve.rs`. `run_daemon` delegates through the new module under the same invocation; startup checks, service construction, task registration, listeners, and shutdown wiring are unchanged. The runtime-boundary guard scans the new module.

The warning-free locked default library check and 638-test daemon library suite pass. The locked full-controller/legacy library check and scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check pass. This is internal-only; no release-note fragment is required. RF-024 and RF-002 remain in progress; the latest complete release gate remains after Batch 71.

## RF-002 Lidarr Rejection Paired-Persistence Turn Review (2026-09-25)

Reviewed `apply_lidarr_rejection_policy`: the ignored-result path acquires Wishlist then Search turns, mutates both stores, and persists their changes through the paired transaction helper. On failure it restores the wishlist and conditionally restores the search store before returning. On success it publishes changed-search events while preserving commit order, then releases the Search and Wishlist turns before deleting rejected files. The change shortens the serialized scope without changing transaction, rollback, or event order.

The existing `lidarr_rejection_policy_blacklists_wishlist_origin_and_deletes_files` regression passes (1/1). The warning-free locked default library check and 638-test suite pass, as do scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary guard, plan freshness, release-note preview, and diff check. This is partial RF-002 evidence; other paired-persistence paths remain under review. The internal lock-lifetime change needs no release-note fragment.

## RF-024 Controller Reload Lifecycle Ownership (Batch 169, 2026-09-25)

Moved controller compatibility-config path selection and YAML reads/writes,
options-location JSON, config-watcher startup, and watched configuration load
and apply into `controller_reload.rs`. The root imports preserve existing route,
startup, and test call sites; `NATIVE_DEFAULT_ADVERSARIAL_YAML` moved with the
compatibility code. The runtime-boundary guard scans the new module. The root
`lib.rs` is now 36,368 lines.

The warning-free locked default library check and 638-test suite pass. The
focused watched share-reload regression and the full-controller watched-CORS
regression pass (1/1 each). The full-controller/legacy library check and
scoped Clippy with `-D clippy::await_holding_lock` pass. A bounded API group-4
library check initially exposed test-support items that were gated only for
the full-controller suite; those imports and the fixture write helper now
follow `bounded-differential`, and the feature check passes with eight unused
imports in unrelated feature-gated helper areas. The changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024 and
RF-002 remain in progress.

## RF-024 MusicBrainz Controller Response Ownership (Batch 170, 2026-09-25)

Moved the MusicBrainz mutation response handler and its artist-radar opaque
reference validator into `musicbrainz_controller.rs`. The feature mutation
dispatcher continues to call the handler through a crate-root import; response
selection, validation, persistence, status codes, and JSON bodies are
unchanged. The runtime-boundary guard scans the new module. The root
`lib.rs` is now 35,370 lines.

The warning-free locked default library check and 638-test suite pass. The
existing MusicBrainz residual differential passes (1/1). The full-controller/
legacy library check and scoped Clippy with `-D clippy::await_holding_lock`
pass. The changed-file formatter, runtime-boundary guard, plan freshness,
release-note preview, and diff check pass. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

## RF-024 ActivityPub Controller Ownership (Batch 171, 2026-09-25)

Moved ActivityPub signing-key response and parsing, inbox signature
verification, actor validation, relationship mutation/projection, actor and
webfinger reads, and music-activity projections into
`activitypub_controller.rs`. Root imports preserve both route dispatchers,
the existing miscellaneous mutation path, and full-controller test call
sites. Response and signature behavior remain unchanged. The runtime-boundary
guard scans the new module. The root `lib.rs` is now 34,643 lines.

The warning-free locked default library check and 638-test suite pass. All 15
existing ActivityPub regressions pass, including signature freshness and
tamper rejection, actor/webfinger routes, inbox relationship updates, and
outbox/undo cases. The full-controller/legacy library check and scoped Clippy
with `-D clippy::await_holding_lock` pass. The changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024
and RF-002 remain in progress.

## RF-024 Session Runtime Ownership (Batch 172, 2026-09-25)

Moved session-command handling, peer-message handling, server reconnection,
due-wishlist-search dispatch, and server-message projection into
`session_runtime.rs`. Existing daemon, peer, route, and test callers continue
through root imports. The wishlist/search persistence turn and rollback order
reviewed under RF-002 remain unchanged. The runtime-boundary guard scans the
new module; the root `lib.rs` is now 32,844 lines.

The warning-free locked default library check and 638-test suite pass. Focused
feature regressions pass for verified peer capability ingestion, server
reconnect projection, and inbound-message replay (3/3). The
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024
and RF-002 remain in progress.

## RF-024 Security Controller Response Ownership (Batch 173, 2026-09-25)

Moved `security_extended_response` into `security_controller.rs`. Both the
bounded and legacy route dispatchers keep their existing root-level call
through an import. Security route selection, authorization, persistence,
status codes, and response bodies are unchanged. The runtime-boundary guard
scans the new module; `lib.rs` is now 32,467 lines.

The warning-free locked default library check and 638-test suite pass. The
existing security controller residual differential passes (1/1). The
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024
and RF-002 remain in progress.

## RF-024 Miscellaneous Mutation Response Ownership (Batch 174, 2026-09-25)

Moved `misc_controller_mutation_response` into `misc_controller_mutations.rs`.
The extended-controller dispatcher keeps its existing call through the root
import. ActivityPub inbox/outbox and the remaining miscellaneous mutation
routes retain their current validation, persistence order, and responses. The
runtime-boundary guard scans the new module; `lib.rs` is now 31,878 lines.

The warning-free locked default library check and 638-test suite pass. The
existing ActivityPub inbox relationship differential passes (1/1). The
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no release-note fragment is required. RF-024
and RF-002 remain in progress.

## RF-024 Feature Mutation and Multisource Ownership (Batch 175, 2026-09-25)

Moved the feature mutation dispatcher, multisource download/swarm response
handlers, and adjacent radar/WorkRef helpers into
`feature_mutation_controller.rs` (1,343 formatted lines). Root imports preserve
the existing bounded and legacy dispatchers, MusicBrainz response path, and
test call sites. Restored `work_ref_search_text` with its original behavior
after checking the extraction boundary. The runtime-boundary guard scans the
new module; `lib.rs` is 30,559 lines.

The warning-free locked default library check and 638-test daemon suite pass.
The focused WorkRef/taste differential passes (1/1). The full-controller/
legacy library check and scoped Clippy with `-D clippy::await_holding_lock`
pass. The multisource residual differential still reports 10 false cases on
`/api/v0/multisource/swarm` and `/api/v0/multisource/swarm/async`: the test
sends `skipVerification: true` without an expected hash, while the verified
executor requires a SHA-256 digest. This is a pre-existing mismatch: the same
handler and differential case are present at base `HEAD`; the extraction
preserves that handler. The endpoint contract mismatch remains unresolved and
is not counted as a regression from this batch. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. This is internal-only; no
release-note fragment is required. RF-024 and RF-002 remain in progress.

## RF-024 VirtualSoulfind v2 Controller Ownership (Batch 176, 2026-09-25)

Moved the VirtualSoulfind v2 catalogue projection, route dispatcher, and
route-specific path-segment helpers into `virtual_soulfind_v2_controller.rs`
(432 formatted lines including imports). Root imports preserve bounded and
legacy route dispatch and feature-gated controller-test callers. The route
implementation matches base `HEAD` aside from formatting and visibility
needed by the new module boundary. The runtime-boundary guard scans the new
module; `lib.rs` is 30,146 lines.

The warning-free locked default library check and 638-test daemon suite pass.
The focused VirtualSoulfind v2 feature run reports 8 passing and 1 failing
test: `virtual_soulfind_v2_routes_execute_bounded_local_intent_workflow`
observes `Failed` where it expects `Completed`; a focused rerun fails the same
way. Both API differential tests pass. The failing route and test are
unchanged at base `HEAD`, so this is an unresolved baseline workflow gap, not
a regression from the extraction. The full-controller/legacy library check
and scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only; no release-note
fragment is required. RF-024 and RF-002 remain in progress.

## RF-024 Extended Controller Download Batch Ownership (Batch 177, 2026-09-25)

Moved `controller_enqueue_download_batch` and its response projection helper
into `extended_controller.rs` (362 formatted lines). The root imports preserve
legacy and bounded dispatch call sites. Transfer preparation and persistence
helpers that also have direct test or route callers remain at the root. The
root `lib.rs` is 29,785 lines.

The warning-free locked default library check and 638-test daemon suite pass.
Focused tests pass for local batch projection, enqueue/dispatch/persistence,
transfer-batch cleanup, and download-edge contracts (4/4). The
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. This is internal-only; no release-note fragment is
required. RF-024 and RF-002 remain in progress.

## RF-024 Lidarr Wishlist Sync Ownership and Persistence (Batch 178, 2026-09-25)

Moved `sync_lidarr_wanted_to_wishlist` into `lidarr_wishlist_sync.rs` (527
formatted lines). Root imports preserve bounded and legacy dispatch. The handler
now computes duplicate keys after acquiring `wishlist_search_persistence_lock`,
so it sees current items after remote Lidarr fetches. For each page it snapshots
`WishlistStore`, applies changes, persists created/updated items through the
transactional bulk upsert, and conditionally restores the snapshot if
persistence fails. The turn stays held through persistence and rollback. The
runtime-boundary guard scans the new module; `lib.rs` is 29,267 lines.

The warning-free locked default library check and 638-test daemon suite pass.
The integrations and wishlist residual differentials pass (1/1 each). The
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. The operational persistence correction has a new
release-note fragment, `20260925-lidarr-wishlist-sync-atomicity.md`; plan
freshness, release-note preview, and diff check pass. No new tests
were added; direct failure injection for an enabled sync remains untested.
RF-024 and RF-002 remain in progress.

## RF-024 Versioned GET Contract Ownership (Batch 179, 2026-09-25)

Moved `versioned_get_failure_contract` into `versioned_get_contract.rs` (517
formatted lines including imports). The root import preserves bounded and
legacy route-dispatch callers. The runtime-boundary guard includes the new
module; `lib.rs` is now 28,753 lines.

The locked default library check and 638-test daemon suite pass. Both focused
versioned GET contract differentials pass (2/2). The locked full-controller /
legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. This is internal-only; no release-note fragment
is required. RF-024 and RF-002 remain in progress.

## RF-024 Versioned Relay Upload Controller Ownership (Batch 180, 2026-09-25)

Moved `versioned_relay_request`, its byte-oriented route handler, and the
relay upload staging and cleanup helpers into `versioned_relay_controller.rs`
(389 formatted lines). Root imports preserve bounded and legacy dispatch, the
HTTP connection path, preview cleanup, and existing test callers. The
runtime-boundary guard scans the new module; `lib.rs` is now 28,360 lines.

The locked default library check and 638-test daemon suite pass. Both focused
versioned relay controller tests and the relay controller route differential
pass (3/3). The warning-free locked full-controller/legacy library check and
scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 Preview Stream Ticket Controller Ownership (Batch 181, 2026-09-25)

Moved `create_preview_stream_ticket` and its identifier, filename, and traversal
validation helpers into `preview_stream_controller.rs` (338 formatted lines).
The root import preserves bounded, legacy, and extended-controller callers.
The runtime-boundary guard scans the new module; `lib.rs` is now 28,025 lines.

The locked default library check and 638-test daemon suite pass. The peer and
mesh ticket creation, validation, and size-limit differentials pass (3/3). The
warning-free locked full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 Native Compatibility Controller Ownership (Batch 182, 2026-09-25)

Moved `native_compat_route` and `native_compat_response` into
`native_compat_controller.rs` (344 formatted lines). Root imports preserve the
bounded and legacy route dispatchers. The runtime-boundary guard scans the new
module; `lib.rs` is now 27,688 lines.

The locked default library check and 638-test daemon suite pass. The native
compatibility route-shape and read-projection differentials pass (2/2). The
warning-free locked full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 Source Feed Preview Controller Ownership (Batch 183, 2026-09-25)

Moved `preview_spotify_source_feed`, `preview_configured_provider_source_feed`,
and their provider identifier parsers into `source_feed_preview_controller.rs`
(621 formatted lines). Root imports preserve both dispatchers and existing
test callers. The runtime-boundary guard scans the new module; `lib.rs` is now
27,072 lines.

The locked default library check and 638-test daemon suite pass. Five focused
Spotify and configured-provider preview tests pass. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 Wishlist Auto-Download Controller Ownership (Batch 184, 2026-09-25)

Moved `auto_download_completed_wishlist`, recent-completion selection, and
staged-transfer rollback helpers into `wishlist_auto_download.rs` (354
formatted lines). Root imports preserve bounded, legacy, and session-runtime
callers. The existing wishlist persistence turn, atomic counter write, and
conditional rollback remain unchanged. The runtime-boundary guard scans the
new module; `lib.rs` is now 26,721 lines.

The locked default library check and 638-test daemon suite pass. The focused
auto-download ranking and one-shot limit regression passes. The warning-free
locked full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 Wishlist CSV Import Ownership (Batch 185, 2026-09-25)

Moved wishlist CSV parsing, simple text import parsing, header helpers, and
`versioned_wishlist_csv_import_response` into `wishlist_csv_import.rs` (290
formatted lines). Root imports preserve bounded and legacy route calls plus
source-feed ingestion. The runtime-boundary guard scans the new module;
`lib.rs` is now 26,440 lines.

The locked default library check and 638-test daemon suite pass. The oversized
CSV row-batch regression and wishlist controller residual differential pass.
The warning-free locked full-controller/legacy library check and scoped
Clippy with `-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 Session Manager Loop Ownership (Batch 186, 2026-09-25)

Moved `spawn_session_manager` into `session_runtime.rs`, alongside command
handling, reconnection, due-wishlist dispatch, and server-message projection.
The root import preserves the daemon startup caller; test-only helper imports
are feature-gated to keep default builds warning-free. `lib.rs` is now 26,198
lines.

The locked default library check and 638-test daemon suite pass. Existing
reconnect-state and VPN/session readiness regressions pass (2/2). The
warning-free locked full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. This is internal-only; no release-note fragment is required.
RF-024 and RF-002 remain in progress.

## RF-024 HashDb History Backfill Controller Ownership (Batch 187, 2026-09-25)

Moved `hashdb_backfill_from_history_response` and its FLAC candidate selector
into `hash_backfill_controller.rs` (145 formatted lines). Root imports preserve
bounded and legacy route calls. The existing shared persistence-turn sequence
remains intact from progress read through candidate selection and progress
write. The runtime-boundary guard scans the new module; `lib.rs` is now 26,054
lines.

The locked default library check and 638-test daemon suite pass. The focused
cursor-order regression and HashDb history-backfill route differential pass
(2/2). The warning-free locked full-controller/legacy library check and
scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file
formatter and runtime-boundary guard pass. Plan freshness, release-note
preview, and diff check pass. This is internal-only; no release-note fragment
is required. RF-024 and RF-002 remain in progress.

## RF-024 Relay Stream and Download Handler Ownership (Batch 188, 2026-09-25)

Moved `relay_versioned_stream_content_id`, `open_relay_controller_stream`,
`relay_versioned_download_token`, and `open_relay_controller_download` into
`versioned_relay_controller.rs` (637 formatted lines total). Root imports
preserve HTTP and test callers. `LocalStreamFile` remains at the root for the
other preview and download handlers; the existing relay upload staging and
persistence ordering remain unchanged. The module was already covered by the
runtime-boundary guard. `lib.rs` is now 25,807 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The focused download-token regression and relay-controller route
differential pass (2/2). The focused
`relay_stream_requests_agent_and_serves_matching_upload` test still fails when
reading the staged file with `Bad file descriptor`: the staging helper opens
the file write-only and returns that same handle to the stream response. The
helper and handler logic match the pre-extraction `HEAD`; the test also existed
there, so this failure predates Batch 188 and remains an explicit baseline
issue. The locked full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Source Provider Catalog Projection Ownership (Batch 189, 2026-09-25)

Moved the static `source_provider_catalog_json` provider and profile projection
into `source_provider_catalog.rs` (222 formatted lines). The root import keeps
the bounded route dispatcher, legacy dispatcher, and existing test callers on
the same function. Provider ordering, activation rules, disabled reasons, and
JSON fields remain unchanged. This module has no runtime I/O or untrusted-input
boundary; no runtime-guard entry was needed. `lib.rs` is now 25,586 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The source-provider catalog and edge-contract differentials pass
(2/2), covering both route aliases, configuration-derived activation,
malformed paths, and closed-database behavior. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 SongID Response Projection Ownership (Batch 190, 2026-09-25)

Moved `songid_evidence_package_json` and `songid_capabilities_json` into
`songid_controller.rs` (474 formatted lines). The root imports preserve the
bounded and legacy route callers plus direct regression callers. Evidence
candidate ordering, truncation, warnings, artifact projection, capability
availability checks, PATH probing, and configured-integration handling remain
unchanged. Shared executable-discovery helpers remain at the root for their
other callers. `lib.rs` is now 25,120 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Existing SongID capability and evidence-package regressions plus
the SongID open-case route differential pass (3/3). The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Quarantine Jury Controller Ownership (Batch 191, 2026-09-25)

Moved quarantine-jury mutation, aggregate, audit, acceptance, route-request,
and dynamic-read handling into `quarantine_controller.rs` (1,001 formatted
lines). Root imports preserve feature-mutation, extended-controller, and
full-controller test callers. Shared trusted-mesh and QUIC peer-routing helpers
remain at the root because PodCore and quarantine-jury both call them. The
runtime-boundary guard now scans the new module. Quorum rules, verdict
validation and signatures, acceptance snapshots, audit output, and persistence
behavior remain unchanged. `lib.rs` is now 24,130 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Five focused quarantine-jury regressions pass (5/5), covering
request validation, quorum acceptance, audit projection, and both route
differentials. The warning-free locked full-controller/legacy library check
and scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.

## RF-024 Controller CLI Ownership (Batch 192, 2026-09-25)

Moved `ServeInvocation`, `ControllerCliEnv`, controller-specific flag to
environment mapping, and `parse_serve_args` into `controller_cli.rs` (330
formatted lines). Root imports preserve the process entry point, daemon
startup, configuration reload, and test callers. Environment names, secret
option mappings, multi-value handling, duplicate detection, and inline-value
semantics remain unchanged. The runtime-boundary guard now scans the module.
`lib.rs` is now 23,803 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Seven focused full-controller CLI regressions pass (7/7), covering
startup option mappings, serve argument validation, and multi-value regex
flags. The warning-free locked full-controller/legacy library check and scoped
Clippy with `-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Swarm Analytics Projection Ownership (Batch 193, 2026-09-25)

Moved `swarm_analytics_dashboard` and its ratio, averaging, duration, percent,
and recommendation helpers into `swarm_analytics.rs` (266 formatted lines).
The root import preserves both bounded and legacy route callers. Snapshot
selection, query-window cutoff, peer ranking, utilization math, recommendation
thresholds, ordering, and JSON fields remain unchanged; the routes still pass
their existing bounded store snapshot. The module only aggregates in-memory
swarm records and projects JSON, so it needs no separate runtime-guard entry.
`lib.rs` is now 23,540 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Both focused swarm-analytics regressions pass (2/2), covering the
shared bounded snapshot and route response differential. The warning-free
locked full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Transfer Recovery Runtime Ownership (Batch 194, 2026-09-25)

Moved run_transfer_rescue_cycle, verified mesh-swarm promotion and cleanup,
and run_download_auto_retry_cycle_with_settings with their private helpers
into transfer_recovery_runtime.rs (566 formatted lines). Root imports preserve
the daemon loop callers. The existing direct full-controller test entry points
for auto-retry and conditional transfer rollback remain available at the root.
Search/transfer persistence ordering, rollback behavior, session-command
dispatch, and event/log publication remain unchanged. The runtime-boundary
guard now scans the new module. lib.rs is now 22,980 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Nine focused auto-retry regressions and six rescue regressions pass.
The warning-free locked full-controller/legacy library check and scoped Clippy
with -D clippy::await_holding_lock pass. Changed-file formatter and
runtime-boundary guard, plan freshness, release-note preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Listener Runtime Ownership (Batch 195, 2026-09-25)

Moved configured listener startup, bind and status updates, accept-loop
supervision, incoming peer dispatch, shared-mesh connection handling, and
listener diagnostic classification into listener_runtime.rs (637 formatted
lines). The root import preserves daemon startup; listener state models and
JSON projection remain in listener_state.rs. Listener binding behavior,
managed-task shutdown, inbound capacity and network limits, obfuscation and
shared-mesh routing, and diagnostic redaction remain unchanged. The
runtime-boundary guard now scans the module. lib.rs is now 22,347 lines.

The warning-free locked default library check and 638-test daemon library
suite pass, including the existing incoming peer error-classification test.
The warning-free locked full-controller/legacy library check and scoped Clippy
with -D clippy::await_holding_lock pass. Changed-file formatter and
runtime-boundary guard, plan freshness, release-note preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Distributed Network Runtime Ownership (Batch 196, 2026-09-25)

Moved distributed parent connection, child registration, link supervision,
message handling, embedded search routing, branch/depth notifications, reset,
and settings application into distributed_runtime.rs (681 formatted lines).
Root imports preserve session, listener, reload, and full-controller regression
callers. Distributed topology state remains in distributed_state.rs. Parent
and child state ordering, persistence-turn behavior, failure reporting,
session-command publication, and search forwarding remain unchanged. The
runtime-boundary guard now scans the module. lib.rs is now 21,674 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Fourteen focused distributed regressions pass, covering branch
and depth propagation, embedded-search broadcasts, child socket shutdown,
persistence rollback, and restart rehydration. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and
runtime-boundary guard, plan freshness, release-note preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Inbound File Transfer Runtime Ownership (Batch 197, 2026-09-25)

Moved handle_inbound_file_transfer into the existing file_transfer_runtime.rs
module. Its root import preserves the listener-runtime and full-controller
regression callers. Pending backfill header handling, token validation,
transfer state transitions and persistence, completed-download processing,
and queue scheduling remain unchanged. The runtime-boundary guard already
scans this module. lib.rs is now 21,577 lines; file_transfer_runtime.rs is
1,488 formatted lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Three focused inbound transfer regressions and the bounded
backfill-header receive regression pass. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and
runtime-boundary guard, plan freshness, release-note preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Peer Message Runtime Ownership (Batch 198, 2026-09-25)

Moved incoming plain and obfuscated peer-message loops, request and blacklist
handling, upload/download queue dispatch, queue scheduling and projections,
and peer frame receive helpers into peer_message_runtime.rs (749 formatted
lines). Root imports preserve listener, transfer-runtime, session-runtime,
route, and regression callers. Peer wire responses, transfer-group policy,
queue priority and position math, and existing lock sequencing remain
unchanged. The runtime-boundary guard now scans the module. lib.rs is now
20,847 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Four focused peer-message and upload-queue regressions pass. The
warning-free locked full-controller/legacy library check and scoped Clippy
with -D clippy::await_holding_lock pass. Changed-file formatter and
runtime-boundary guard, plan freshness, release-note preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Transfer Recovery Scheduler Ownership (Batch 199, 2026-09-25)

Moved the auto-retry, auto-replace, and rescue task supervisors, the
auto-replace settings projection, and persisted rescue-search creation into
transfer_recovery_runtime.rs. Root imports preserve daemon startup, both
source-discovery dispatchers, and the existing settings regression caller.
Connected-session checks, configured intervals, missed-tick behavior, retry
tracking, search rollback, and event publication remain unchanged. The
runtime-boundary guard already scans this module. lib.rs is now 20,715 lines;
transfer_recovery_runtime.rs is 706 formatted lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Nine auto-retry regressions, six rescue regressions, and two
auto-replace settings regressions pass. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Source Discovery Runtime Ownership (Batch 200, 2026-09-25)

Moved source-discovery search dispatch, bounded source projection, and the
scheduled discovery supervisor into `source_discovery_runtime.rs` (100
formatted lines). The root imports preserve daemon startup and both route
dispatchers. Search reservation and creation ordering, global dispatch,
interval skip behavior, managed-task ownership, generation checks that ignore
stale completions, and warning logging remain unchanged. The runtime-boundary
guard now scans this module. `lib.rs` is now 20,620 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The existing source-discovery route regression and stale-generation
regression pass with the split dispatcher / full-controller feature set. The
stale-generation regression also passes with legacy dispatch enabled; the
route regression with `legacy-route-dispatch` enabled overflows its historical
test-worker stack before assertions. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Search Expiry Runtime Ownership (Batch 201, 2026-09-25)

Moved the managed search-expiry scheduler into `search_runtime.rs` (40
formatted lines). The daemon-startup import remains stable. One-second cadence,
skipped missed ticks, wishlist preservation while enabled and connected,
expiry snapshot ordering, rollback on persistence failure, and warning logging
remain unchanged. The runtime-boundary guard now scans this module. `lib.rs`
is now 20,583 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The warning-free locked full-controller/legacy library check and
scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.

## RF-024 VPN Runtime Ownership (Batch 202, 2026-09-25)

Moved VPN status polling, Soulseek advertised-port synchronization, and the
reconnect wake helper into `vpn_runtime.rs` (182 formatted lines). Root imports
preserve daemon startup, session reconnection, and regression callers. Poll
cadence, managed-task ownership, error fallback/logging, DHT port sync, VPN
readiness transitions, reconnect command publication, shared-obfuscated-port
handling, and command-interrupted reconnect delay behavior remain unchanged.
The runtime-boundary guard now scans this module. `lib.rs` is now 20,407 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The existing reconnect-command interruption and VPN readiness
regressions pass with `full-controller-tests`. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Overlay DHT Publisher Runtime Ownership (Batch 203, 2026-09-25)

Moved overlay DHT publisher scheduling and publication-snapshot construction
into `mesh_dht_runtime.rs` (97 formatted lines). The root import preserves
daemon startup. Trusted-peer and feature gates, five-second initial delay,
thirty-minute interval with skipped missed ticks, signing identity and endpoint
projection, content/shadow/public-pod snapshot selection, username fallback,
publication, and result logging remain unchanged. The runtime-boundary guard
now scans this module. `lib.rs` is now 20,313 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Nine existing DHT regressions pass. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Lidarr Scheduler Ownership (Batch 204, 2026-09-25)

Moved the managed Lidarr wishlist-sync scheduler and its cycle into the
existing `lidarr_wishlist_sync.rs` module. Root imports preserve daemon
startup and full-controller regression callers. The scheduler interval floor,
configuration gates, syncing/error state, next-cycle projection, and external
sync behavior remain unchanged. The runtime-boundary guard already scans this
module. `lib.rs` is now 20,288 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Both existing Lidarr scheduler regressions pass, covering disabled
sync policy and external failure cleanup. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 HashDb Backfill Runtime Ownership (Batch 205, 2026-09-25)

Moved the managed backfill scheduler and `run_backfill_cycle` into
`hash_backfill_runtime.rs` (70 formatted lines). Root imports preserve daemon
startup and both route dispatchers. The initial wait, skipped missed ticks,
idle and minimum-idle gates, candidate cap and daily peer limit, native/offline
peer exclusions, completion counters, and next-cycle timestamps remain
unchanged. The runtime-boundary guard now scans this module. `lib.rs` is now
20,221 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The existing full-controller backfill residual regression passes.
The warning-free locked full-controller/legacy library check and scoped Clippy
with `-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Rate-Limit Cleanup Runtime Ownership (Batch 206, 2026-09-25)

Moved the managed periodic rate-limit cleanup task into `rate_limit.rs`, next
to the limiter state and cleanup operation. The daemon-startup import remains
stable. Managed-task ownership, 60-second interval, and cleanup call remain
unchanged. The runtime-boundary guard now scans this module. `lib.rs` is now
20,209 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. All 23 existing rate-limit tests pass. The warning-free locked
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 SongID Analysis and Worker Runtime Ownership (Batch 207, 2026-09-25)

Moved SongID metadata and fingerprint analysis, the bounded worker pool,
queued-run processing, failure reporting, and persisted-run requeue into
`songid_runtime.rs` (728 formatted lines). Root imports preserve dispatcher,
daemon-startup, and regression-test callers. Source fallback, Spotify metadata,
Chromaprint/AcoustID behavior, queue and semaphore limits, runtime-state
transitions, hub events, failure handling, and restart recovery remain
unchanged. Test-only root imports preserve direct existing helper regressions.
The runtime-boundary guard now scans this module. `lib.rs` is now 19,491 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. All 18 SongID-filtered regressions pass, including run lifecycle,
open-case, source classification, Chromaprint, and AcoustID cases. The
warning-free locked full-controller/legacy library check and scoped Clippy
with `-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Wishlist Smart-Fallback Runtime Ownership (Batch 208, 2026-09-25)

Moved the delayed wishlist smart-fallback task into `session_runtime.rs`,
beside wishlist-search dispatch. The existing session-runtime caller remains
unchanged. The five-second response window, active/source checks, result policy
and suppression thresholds, fallback-attempt selection, persisted reset with
rollback, event recording, and global session-command dispatch remain
unchanged. The runtime-boundary guard already scans this module. `lib.rs` is
now 19,408 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The warning-free locked full-controller/legacy library check and
scoped Clippy with `-D clippy::await_holding_lock` pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Gold Star Club Auto-Join Runtime Ownership (Batch 209, 2026-09-25)

Moved the Gold Star Club auto-join supervisor into session_runtime.rs beside
the session manager. The daemon startup call remains unchanged, and the
availability helper stays in lib.rs for route checks. The auto-join policy
gate, managed-task lifecycle, connected-session polling, identity selection,
pod creation and membership operation, logging, and one-shot completion remain
unchanged. The runtime-boundary guard already scans this module. lib.rs is
now 19,330 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The warning-free locked full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Shutdown Signal Supervisor Ownership (Batch 210, 2026-09-25)

Moved the managed shutdown-signal supervisor and its Unix/non-Unix signal wait
into daemon_serve.rs, beside the only startup caller. The shared graceful
shutdown sequence remains in lib.rs for both signal and route callers. Managed
task registration, Unix SIGTERM/SIGINT/SIGQUIT and non-Unix Ctrl-C handling,
shutdown logging, and the disconnect-before-stop sequence remain unchanged.
The runtime-boundary guard already scans daemon_serve.rs. lib.rs is now 19,286
lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The warning-free locked full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Process Replacement Helper Ownership (Batch 211, 2026-09-25)

Moved the process replacement helper into daemon_serve.rs beside the restart
lifecycle caller. Executable resolution, argument forwarding, child process
creation, and error mapping remain unchanged. The daemon startup module already
owns this helper's caller; no boundary guard change was needed. lib.rs is now
19,276 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The warning-free locked full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Soulfind Bridge Runtime Ownership (Batch 212, 2026-09-25)

Moved the bounded Soulfind bridge TCP frame codec, client session and request
handling, and managed listener supervisor into soulfind_bridge_runtime.rs.
daemon_serve.rs keeps the startup call, and HTTP bridge response projections
remain with their existing controller paths. Crate-root aliases preserve the
existing feature-gated protocol test helpers. The runtime-boundary guard now
scans the new module. lib.rs is now 18,679 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. All five existing soulfind_bridge-filtered tests pass under the
full-controller/legacy feature set. The warning-free locked full-controller/
legacy library check and scoped Clippy with -D clippy::await_holding_lock
pass. Changed-file formatter, runtime-boundary guard, plan freshness,
release-note preview, and diff check pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.

## RF-024 Transfer Completion Integration Ownership (Batch 261, 2026-09-26)

Moved completed-download Lidarr import, rejection policy and cleanup, relay
download-token issuance, and FTP upload handling into `transfer_completion.rs`.
Route dispatchers and transfer-runtime callers retain crate-root imports. The
bounded integration JSON reader stays shared at the root for other integration
modules. Completion side effects and transfer lifecycle ordering are unchanged.
`lib.rs` is now 6,445 lines; the new module is 275 lines.

The warning-free locked default library check and all 638 default library tests
pass. The existing
`lidarr_rejection_policy_blacklists_wishlist_origin_and_deletes_files`
regression passes with `full-controller-tests`. The locked combined
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Transfer Batch Controller Ownership (Batch 260, 2026-09-26)

Moved transfer resource-path parsing, transfer-batch persistence and record
projection, transfer request decoding, and queued batch preparation/commit
into `transfer_batch_controller.rs`. The split and legacy dispatchers and
extended controller retain crate-root imports; shared generic path/body helpers
remain in the root. Transfer ordering, staging, rollback, and responses are
unchanged. `lib.rs` is now 6,711 lines; the new module is 478 lines.

The warning-free locked default library check and all 638 default library tests
pass. The existing
`controller_api_differential_controller_transfer_batch_cleanup_and_failures`
regression passes with `full-controller-tests`. The locked combined
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Controller Options and YAML Ownership (Batch 259, 2026-09-26)

Moved transfer-option response shaping, bounded native YAML parsing and updates,
options overlay validation and mutation, options response projection, and
server-state JSON projection into `controller_options_controller.rs`.
Crate-root imports preserve the existing dispatchers, controller options
projection/state, feature-mutation, reload, and YAML callers. Validation bounds,
persisted settings, and response shapes are unchanged. `lib.rs` is now 7,172
lines; the new controller module is 919 lines.

The warning-free locked default library check and all 638 default library tests
pass. The existing `controller_api_differential_native_options_edge_contracts`
regression passes with `full-controller-tests`. The locked combined
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Library Browser and Native Search Ownership (Batch 258, 2026-09-26)

Moved library content-ID lookup, media-kind classification, browser query and
filter projection, and native library-item search into `library_controller.rs`.
Route groups, the compatibility dispatcher, VirtualSoulfind, and preview-stream
callers retain their crate-root imports. Search, matching, and content-ID
behavior are unchanged. `lib.rs` is now 8,080 lines; the new controller module
is 428 lines.

The warning-free locked default library check and all 638 default library tests
pass. The existing `controller_api_differential_library_jobs_and_discovery_projections`
regression passes with `full-controller-tests`. The locked combined
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 HashDb Inventory Helper Ownership (Batch 255, 2026-09-26)

Moved the HashDb FLAC inventory key prefix, key and row conversion, candidate
filtering, and peer projection into `hash_db_store.rs` beside snapshot
persistence. Crate-root imports preserve the backfill controller, split and
legacy routes, extended controller, and existing HashDb contract caller. The
inventory shape and filtering behavior remain unchanged. `lib.rs` is now
9,332 lines.

The locked default library suite passes all 638 tests. The existing HashDb
domain controller contract regression passes with `full-controller-tests`.
The locked package check and scoped full-controller/legacy Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Search Persistence Helper Ownership (Batch 254, 2026-09-26)

Moved search status and target mapping, persisted record conversion, snapshot
and delta writes, paired deletes, and conditional rollback into
`search_persistence.rs`. Crate-root imports preserve route, session, search
store, expiry scheduler, transfer recovery, and database-maintenance callers.
The shared search persistence turn and current-snapshot checks remain
unchanged. The generic `RecordListFilter` remains in the crate root. `lib.rs`
is now 9,390 lines.

The locked default library suite passes all 638 tests. All 17 existing
search-focused tests pass with `focused-controller-tests`, including ignored
wishlist rule/search ordering; the search list and pagination route regression
passes with `full-controller-tests`. The locked default and
full-controller/legacy package checks and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Mesh Gateway HTTP Controller Ownership (Batch 213, 2026-09-25)

Moved the mesh gateway disabled/error/auth responses, localhost-origin
validation, service listing, and service-call response handling into
mesh_gateway_controller.rs. Both dispatchers and the feature mutation route
retain root imports. Request policy and response behavior remain unchanged.
The runtime-boundary guard now scans the module. lib.rs is now 18,423 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. Five existing mesh_gateway tests and two mesh_http tests pass with
the split dispatcher. The same five-test gateway filter with
legacy-route-dispatch overflows the historical worker stack in
mesh_gateway_enabled_enforces_allowlist_and_provider_discovery after one test
passes. The warning-free locked full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Ranking Controller Ownership (Batch 214, 2026-09-25)

Moved ranking history and source-candidate parsing, scoring, and response
construction into ranking_controller.rs. Extended GET routes and feature
mutation routes keep their crate-root imports. Ranking limits, candidate
normalization, scoring weights, response contracts, and storage error handling
remain unchanged. The runtime-boundary guard now scans the module. lib.rs is
now 18,161 lines.

The warning-free locked default library check and 638-test daemon library
suite pass. The existing controller_api_differential_native_ranking_contracts
regression passes with the split dispatcher. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 MusicBrainz Radar and Native Compatibility Validation Ownership (Batch 215, 2026-09-25)

Moved the release-radar wishlist filter, subscription-body parser, and
WorkRef decoder into the existing musicbrainz_controller.rs. Moved the shared
native model-validation response into native_compat_controller.rs. Crate-root
imports preserve both split and legacy dispatcher callers; both destination
modules were already covered by the runtime-boundary guard. Existing shared
DHT socket regressions now bind ephemeral ports and assert that the reported
public socket port remains distinct from the internal mainline endpoint, so
they remain runnable beside local daemons that own fixed service ports.
lib.rs is now 18,007 lines.

The locked default library check and all 638 default library tests pass. Both
existing release-radar regressions pass with the split dispatcher, as do the
two shared-DHT regressions. The full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file formatter
and runtime-boundary guard pass. No tests were added; the existing DHT test
fixtures were made port-independent. This is internal-only and adds no
release-note fragment. Plan freshness, release-note preview, and diff check
pass. RF-024 and RF-002 remain in progress; the latest complete release gate
remains after Batch 71.


## RF-024 Atomic File Writer Ownership (Batch 216, 2026-09-25)

Moved atomic file replacement, temporary-file writing, platform-specific
replacement, and parent-directory syncing into storage.rs. Crate-root aliases
preserve production callers and the feature-gated controller test helpers.
Write ordering, cleanup on failure, sync behavior, and Windows replacement
handling are unchanged. The storage module was already scanned by the
runtime-boundary guard. lib.rs is now 17,951 lines.

The locked default library check and all 638 default library tests pass. The
existing atomic-state-writer and file-lifecycle atomic-writer regressions pass.
The warning-free locked full-controller/legacy library check and scoped Clippy
with -D clippy::await_holding_lock pass. Changed-file formatter,
runtime-boundary guard, plan freshness, release-note preview, and diff check
pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Incoming Search Response Ownership (Batch 217, 2026-09-25)

Moved local share matching and bounded incoming file-search response
construction into search_runtime.rs beside the search-expiry supervisor.
Crate-root imports preserve split and legacy dispatcher callers and existing
controller-test paths. Request filters, response limits, username fallback,
and response fields are unchanged. The runtime-boundary guard already scans
search_runtime.rs. lib.rs is now 17,890 lines.

The locked default library check and all 638 default library tests pass. The
existing incoming-search filter/limit regressions and share-fixture matching
regression pass. The warning-free locked full-controller/legacy library check
and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Direct Peer Transfer Response Ownership (Batch 218, 2026-09-25)

Moved pending direct peer-transfer negotiation and response projection into
file_transfer_runtime.rs beside accepted and indirect transfer handling.
Crate-root imports preserve split and legacy route callers and controller-test
paths. Queue-capacity handling, token checks, transfer-state transitions,
persistence, and scheduler behavior are unchanged. The runtime-boundary guard
already scans file_transfer_runtime.rs. lib.rs is now 17,762 lines.

The locked default library check and all 638 default library tests pass. All
eight existing peer_address_response regressions pass with the split
dispatcher. The warning-free locked full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Peer Browse Response Runtime Ownership (Batch 219, 2026-09-25)

Moved direct and indirect peer-browse response orchestration into the new
browse_runtime.rs. Peer transport retains socket fetch/decode behavior, and
browse_store.rs retains bounded state and projections. Crate-root imports
preserve session callbacks and controller-test paths. Persistence ordering,
indirect fallback, event reporting, and error handling remain unchanged. The
runtime-boundary guard now scans browse_runtime.rs. lib.rs is now 17,543 lines.

The locked default library check and all 638 default library tests pass. The
existing peer-address response regressions pass (8 tests), as does the
indirect browse response regression. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and
runtime-boundary guard pass. Plan freshness, release-note preview, and diff
check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Session Message and Search Dispatch Ownership (Batch 220, 2026-09-25)

Moved server search-message construction, inbound PODMSG validation and
storage, bounded incoming-search response scheduling, and room-to-pod
bridging into session_runtime.rs beside the session command and message
handlers. Crate-root aliases preserve the existing controller-test callers.
Search targets, payload validation, signature and membership checks,
persistence, and queue behavior are unchanged. The existing inbound-message
test fixture now uses a valid UUID MessageId required by PodChannelStore.
lib.rs is now 17,263 lines.

The locked default library check and all 638 default library tests pass. The
existing incoming-public-search, inbound-pod-message, and room/pod-bridge
regressions pass. The warning-free locked full-controller/legacy library check
and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Queued Download Scheduler Ownership (Batch 221, 2026-09-25)

Moved queued-download selection, peer lookup, status transitions, persistence,
and dispatch-failure handling into file_transfer_runtime.rs beside direct and
indirect transfer execution. No external root aliases were needed; all
callers now resolve within that module. Queue ordering and retry behavior are
unchanged. The runtime-boundary guard already scans file_transfer_runtime.rs.
lib.rs is now 17,213 lines.

The locked default library check and all 638 default library tests pass. All
eight existing peer-address response regressions pass with the split
dispatcher. The warning-free locked full-controller/legacy library check and
scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 FLAC Hash Backfill Runtime Ownership (Batch 222, 2026-09-25)

Moved backfill candidate filtering, FLAC header parsing and receipt, and
per-file HashDb persistence into the existing hash_backfill_runtime.rs beside
the scheduler and cycle. Peer endpoint lookup remains shared at the crate
root; crate-root imports preserve controller, legacy-route, transfer-runtime,
and full-controller test callers. Candidate ordering and bounds, transfer
negotiation, persistence ordering, and rollback behavior are unchanged. The
runtime-boundary guard already scans this module. lib.rs is now 16,870 lines.

The locked default library check and all 638 default library tests pass. The
existing FLAC header policy regression passes with full-controller-tests. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. Plan freshness, release-note preview, and diff check pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 Peer Endpoint Cache and Lookup Ownership (Batch 223, 2026-09-25)

Moved the bounded peer-endpoint cache, test endpoint override mapping, and
server lookup/wait flow into peer_transport.rs. The endpoint record limit and
TTL now live with those operations. Crate-root imports preserve session,
controller, transport-runtime, private-gateway, and full-controller test
callers. Username matching, expiry, request timeout, and endpoint selection
behavior remain unchanged. The runtime-boundary guard already scans this
module. lib.rs is now 16,811 lines.

The locked default library check and all 638 default library tests pass. The
existing case-distinct peer-endpoint cache regression passes with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter and runtime-boundary guard pass. Plan freshness, release-note
preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.


## RF-024 PodCore Verification and Message Helper Ownership (Batch 224, 2026-09-25)

Moved PodCore peer-message routing, signing, canonical payload construction,
signature and membership verification, member-affinity projection and scoring,
and signed membership-record builders into podcore_controller.rs. Crate-root
imports preserve session signature checks, quarantine trust lookup, and the
existing controller-test call path. The runtime-boundary guard already scans
this module. Verification rules, signature-mode handling, peer routing, and
response shapes are unchanged. lib.rs is now 16,164 lines.

The locked default library check and all 638 default library tests pass. The
existing affinity, unknown-sender signature, message membership/signature,
and PodCore routing regressions pass with full-controller-tests. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. Plan freshness, release-note preview, and diff check pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 MediaCore Descriptor Retrieval Ownership (Batch 225, 2026-09-25)

Moved MediaCore retrieval metrics, the retrieval result type, descriptor
verification, and cached descriptor retrieval into mediacore_controller.rs.
Every caller already belonged to that module, so no external root aliases were
needed. Cache expiry and bypass rules, hit/miss accounting, persisted cache
updates, and response shapes remain unchanged. The runtime-boundary guard
already scans this module. lib.rs is now 15,998 lines.

The locked default library check and all 638 default library tests pass. The
existing MediaCore descriptor, cache, and retrieval-metric regression passes
with full-controller-tests. The warning-free locked full-controller/legacy
library check and scoped Clippy with -D clippy::await_holding_lock pass.
Changed-file formatter and runtime-boundary guard pass. Plan freshness,
release-note preview, and diff check pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 MusicBrainz Dynamic Read Ownership (Batch 226, 2026-09-25)

Moved the MusicBrainz dynamic-read handler, exportability and proposed-change
helpers, export-review projection, and edit-type inventory into
musicbrainz_controller.rs. The extended-controller caller retains its existing
path through a crate-root import. Dynamic route matching and response
contracts remain unchanged. The runtime-boundary guard already scans this
module. lib.rs is now 15,796 lines.

The locked default library check and all 638 default library tests pass. The
existing MusicBrainz export-review and approval regression passes with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter and runtime-boundary guard pass. Plan freshness, release-note
preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.


## RF-024 Search Action and Collection Item Controller Ownership (Batch 227, 2026-09-25)

Moved the native search-result action response into
feature_mutation_controller.rs and collection-item mutation/reorder handling
into extended_controller.rs, beside their existing call sites. No crate-root
aliases were needed. Search action validation, collection persistence turns,
rollback, and response contracts remain unchanged. The runtime-boundary guard
already scans both modules. lib.rs is now 15,550 lines.

The locked default library check and all 638 default library tests pass. The
existing native search-action differential and collection CRUD/reorder
lifecycle differential pass with full-controller-tests. The warning-free
locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. Plan freshness, release-note preview, and diff check pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 Search, Bridge, and Playback Helper Ownership (Batch 228, 2026-09-25)

Moved search problem/action responses into feature_mutation_controller.rs and
bridge search/download/progress projections plus listening-party and playback
feedback helpers into extended_controller.rs. Existing dispatch callers keep
their crate-root imports for bridge progress, listening-party event windows,
and playback priority; the test constant reference now uses its owning module.
Response behavior and route contracts remain unchanged. lib.rs is now 15,180
lines.

The locked default library check and all 638 default library tests pass. Five
existing full-controller regressions pass for bridge routes, bridge residuals,
search dispatch, native search actions, and listening-party event cases. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 PodCore Discovery and Backfill Helper Ownership (Batch 229, 2026-09-25)

Moved local backfill projection, PodCore storage-error mapping, visibility and
DHT discovery helpers, and discovery refresh into podcore_controller.rs beside
their route callers. podcore_runtime_stats now references the shared visibility
formatter through its owning module. PodCore response behavior and persistence
ordering remain unchanged. lib.rs is now 14,929 lines.

The locked default library check and all 638 default library tests pass. The
existing PodCore backfill and discovery-stat regressions pass with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 MediaCore Fuzzy Matching and Descriptor Publication Ownership (Batch 230, 2026-09-25)

Moved text similarity, phonetic matching, content-domain normalization,
perceptual-hash comparison, descriptor validation/versioning, and publication
into mediacore_controller.rs beside their route callers. Existing direct
edit-distance regressions now reference the owning module. The canonical
PodCore payload comment was restored above pod_message_canonical_payload,
where it describes the implementation. Response and persistence behavior
remain unchanged. lib.rs is now 14,597 lines.

The locked default library check and all 638 default library tests pass. The
existing MediaCore fuzzy/IPLD, descriptor publication, and both edit-distance
regressions pass with full-controller-tests. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 Transfer Reporting Projection Ownership (Batch 231, 2026-09-25)

Moved transfer summary, versioned summary and histogram, speed and download
statistics, accelerated/stuck views, leaderboard, exception, directory, and
per-user report helpers into transfer_controller.rs. Crate-root imports keep
route-dispatch groups 1, 2, and 5, the legacy dispatcher, and existing direct
tests on their current paths. Report contracts and filtering remain unchanged.
lib.rs is now 13,740 lines.

The locked default library check and all 638 default library tests pass. Seven
existing transfer-report regressions pass for measured speeds, leaderboard and
directory filters, standard contracts, and versioned reports. The warning-free
locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 PodCore Route Classification and DHT Verification Ownership (Batch 232, 2026-09-25)

Moved PodCore dynamic-route matching and DHT publication verification into
podcore_controller.rs. The root dispatcher calls the route matcher through its
owning module; PodCore response and signature behavior remain unchanged.
lib.rs is now 13,682 lines.

The locked default library check and all 638 default library tests pass. The
existing PodCore routing and DHT metadata-verification regressions pass with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and
diff check pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Extended Dynamic Read and Telemetry Projection Ownership (Batch 233, 2026-09-25)

Moved dynamic realm-subject reads, Prometheus metric JSON projection, and the
VirtualSoulfind disaster-mode projection into extended_controller.rs. Root
imports preserve split/legacy telemetry route callers; the existing unit check
now addresses the disaster-mode helper through its owner module. Response
shapes and values remain unchanged. lib.rs is now 13,618 lines.

The locked default library check and all 638 default library tests pass. The
existing realm-subject, telemetry open-cases, and VirtualSoulfind residual
regressions pass with full-controller-tests. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 Preview Stream and Shared-File Runtime Ownership (Batch 234, 2026-09-25)

Moved shared-file lookup/opening, listening-party stream-ticket issuance,
application-dump file creation, primary/local/remote preview stream handling,
route-path projections, and stream response writers into
preview_stream_controller.rs. Root imports preserve HTTP, route, relay,
session, transfer, and gateway callers. LocalStreamFile remains the shared
crate-root response type; PeerPreviewStream is visible to its HTTP consumer.
Stream and error behavior remain unchanged. lib.rs is now 12,852 lines.

The locked default library check and all 638 default library tests pass. Nine
existing regressions pass for authenticated/local/remote streams, mesh
verification and cleanup, HEAD metadata, application dumps, listening-party
ranges, ticket lifecycle, and ticket bounds. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and
RF-002 remain in progress; the latest complete release gate remains after
Batch 71.


## RF-024 File Path and Scoped Storage Helper Ownership (Batch 235, 2026-09-25)

Moved configured download destination rendering and confined download path/open
helpers into file_transfer_runtime.rs. Moved base64 storage-path decoding and
scoped file deletion helpers into controller_storage.rs. Crate-root imports
preserve sibling route callers, and direct tests now use the owning modules for
test-only helpers and storage limits. Path validation, stream behavior, and
delete behavior remain unchanged. lib.rs is now 12,159 lines.

The locked default library check and all 638 default library tests pass. The 36
existing tests selected by the download filter pass with full-controller-tests.
The warning-free locked full-controller/legacy library check and scoped Clippy
with -D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.


## RF-024 Transfer Admission and Upload Group Policy Ownership (Batch 236, 2026-09-25)

Moved transfer and download capacity checks, transfer-group classification,
upload-limit aggregation, and inbound upload admission into
file_transfer_runtime.rs. Crate-root imports preserve route, session, and peer
runtime callers; direct tests now address the owning module. Admission decisions,
limit precedence, and queue behavior remain unchanged. lib.rs is now 11,837
lines.

The locked default library check and all 638 default library tests pass. Seven
existing transfer-group regressions and the download-capacity slot regression
pass with full-controller-tests. The warning-free locked full-controller/legacy
library check and scoped Clippy with -D clippy::await_holding_lock pass.
Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Session Synchronization and Room Lifecycle Ownership (Batch 237, 2026-09-26)

Moved connected-session synchronization, room join/leave dispatch and failure
projection, post-login transfer scheduling, room replay, ping, and active server
message helpers into session_runtime.rs. Crate-root imports preserve sibling
route and gateway callers; the direct post-login test now addresses the owner
module. Session and room behavior remain unchanged. lib.rs is now 11,423
lines.

The locked default library check and all 638 default library tests pass. Twenty-
seven existing room regressions and the queued-download-after-login regression
pass with full-controller-tests. The warning-free locked full-controller/legacy
library check and scoped Clippy with -D clippy::await_holding_lock pass.
Changed-file formatter, runtime-boundary guard, plan freshness, release-note
preview, and diff check pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.


## RF-024 Session Command and State Helper Ownership (Batch 238, 2026-09-26)

Moved queued-response normalization, peer connect request handling, session
command enqueue helpers, and session/listener snapshot updates into
session_runtime.rs. Crate-root imports preserve sibling callers, and direct
unit tests now address the owner module. Session dispatch, state updates, and
peer handling remain unchanged. lib.rs is now 11,302 lines.

The locked default library check and all 638 default library tests pass. Fourteen
existing session regressions and six indirect peer-command regressions pass with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter, runtime-boundary guard, plan freshness, release-note preview, and diff
check pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release gate
remains after Batch 71.


## RF-024 Transfer Retry and Failure Recording Ownership (Batch 239, 2026-09-26)

Moved the auto-retry cycle wrapper and replacement rollback helper into
transfer_recovery_runtime.rs, expected upload-failure cooldown recording into
upload_peer_cooldowns.rs, and rejected-transfer persistence into
transfer_durability.rs. Existing tests now address retry helpers through their
owner; retry ordering, cooldown classification, and rejection persistence are
unchanged. lib.rs is now 11,243 lines.

The locked default library check and all 638 default library tests pass. Nine
existing auto-retry regressions, three upload-cooldown regressions, and three
inbound-transfer admission regressions pass with full-controller-tests. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.


## RF-024 Event and Diagnostic Logging Runtime Ownership (Batch 240, 2026-09-26)

Moved event mutation and persistence, daemon disk/Loki logging, Soulseek
diagnostic filtering, and HTTP transaction logging into event_runtime.rs beside
the EventStore type/projection module. Crate-root imports preserve existing
route/runtime callers, including the direct event-persistence caller; the
diagnostic-level test now addresses the new owner. Event ordering, filtering,
redaction, and delivery behavior remain unchanged. lib.rs is now 11,038
lines.

The locked default library check and all 638 default library tests pass. Fifty-
four existing event regressions, the HTTP log redaction regression, and the
Soulseek diagnostic filtering regression pass with full-controller-tests. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter, runtime-boundary
guard, plan freshness, release-note preview, and diff check pass. No tests were
added. This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.


## RF-024 OAuth State Persistence Helper Ownership (Batch 241, 2026-09-26)

Moved checked OAuth state upsert, unchanged-snapshot rollback, and serialized
callback consumption into oauth_state.rs beside OAuthStateStore. Crate-root
imports preserve both route-dispatch callers; the direct concurrency regression
now calls the owner module. Callback deletion still completes before the
in-memory record is removed. lib.rs is now 10,982 lines.

The locked default library check and all 638 default library tests pass. All six
existing OAuth-filtered regressions pass with full-controller-tests. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Webhook Lifecycle Runtime Ownership (Batch 242, 2026-09-26)

Moved webhook persistence conversion and hydration, event parsing, checked
create/delete persistence, rollback, audit-log persistence, and standard plus
frozen event dispatch into webhooks.rs beside WebhookManager and
WebhookDispatcher. Crate-root imports preserve route and startup callers; direct
tests now address the owner module. Webhook filtering, persistence ordering,
audit error reporting, and delivery behavior remain unchanged. lib.rs is now
10,700 lines.

The locked default library check and all 638 default library tests pass. Forty
existing webhook-filtered regressions pass with full-controller-tests. The
warning-free locked full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Message Persistence Helper Ownership (Batch 243, 2026-09-26)

Moved message record conversion, conversation storage-failure projection,
checked single/batch message and acknowledgement persistence, conditional
rollback, and conversation deletion persistence into message_store.rs. Root
imports preserve route and session callers; the direct cleanup seed now names
the owner module. Persistence and rollback behavior remain unchanged. lib.rs is
now 10,585 lines.

The locked default library check and all 638 default library tests pass. Sixty-
two existing message-filtered regressions pass with full-controller-tests,
including queued create/delete ordering, cleanup consistency, rollback, batch
atomicity, and rehydration. The warning-free locked full-controller/legacy
library check and scoped Clippy with -D clippy::await_holding_lock pass.
Changed-file formatter and runtime-boundary guard pass. No tests were added.
This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.


## Recovery Record — Reconstructed Daemon Root (2026-09-26)

Rebuilt the truncated `crates/slskr/src/lib.rs` from the saved root, current
extracted modules, and recorded session edits. The saved edit history assumes
earlier uncommitted refactor state, so it cannot be replayed byte-for-byte from
Git HEAD. Before the next ownership split, the reconstructed root was 9,926
lines; Batch 252 records 10,101 lines. The full-controller test gate and
default suite pass, but exact source identity is unavailable.

The locked default library suite passes all 638 tests. The existing
now-playing update/clear ordering regression passes with
`focused-controller-tests`, and its persistence rollback regression passes
with `full-controller-tests`. The warning-free locked package check and
full-controller/legacy compile pass; scoped Clippy with
`-D clippy::await_holding_lock`, runtime-boundary guard, changed-file
formatter, and diff checks pass. No tests were added. This recovery is
internal-only and adds no release-note fragment. The optional regression
invocation with both `full-controller-tests` and `legacy-route-dispatch`
overflows on the retained monolithic route worker stack; the plan's
full-controller-only rollback gate passes.


## RF-024 HashDb Persistence Helper Ownership (Batch 253, 2026-09-26)

Moved HashDb record conversion, request decoding, ordered snapshot persistence,
rollback, and persisted state JSON helpers into `hash_db_store.rs`. Crate-root
imports preserve the split and legacy route, backfill, mesh sync, startup
hydration, and controller-test callers. The shared persistence turn and
mutation/rollback behavior remain unchanged. `lib.rs` is now 9,728 lines.

The locked default library suite passes all 638 tests. Both existing HashDb
writer-order and backfill-cursor-order regressions pass with
`focused-controller-tests`; the now-playing route rollback regression passes
with `full-controller-tests`. The locked package check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Room Subscription Persistence Helper Ownership (Batch 244, 2026-09-26)

Moved checked room join/leave subscription persistence into session_runtime.rs
beside room dispatch, replay, and failure projection. Crate-root imports preserve
both route dispatchers and extended-controller callers. Subscription persistence
ordering and rollback behavior remain unchanged. lib.rs is now 10,566 lines.

The locked default library check and all 638 default library tests pass. All three
existing room-subscription regressions pass with full-controller-tests,
including queued join/leave ordering and route rollback. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 User Note and Interest Persistence Helper Ownership (Batch 245, 2026-09-26)

Moved checked user-note create/update/delete persistence into user_note_store.rs
and checked liked/hated interest upsert/delete persistence into interest_store.rs.
Crate-root imports preserve both route dispatchers and extended-controller
callers; direct test fixtures now name their owner modules. Persisted record
shapes and failure behavior remain unchanged. lib.rs is now 10,508 lines.

The locked default library check and all 638 default library tests pass. Fourteen
existing persistence-order regressions and fourteen existing route rollback
regressions pass with full-controller-tests. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Security Ban Persistence Helper Ownership (Batch 246, 2026-09-26)

Moved security-ban upsert, conditional rollback comparison, and canonicalized
unban persistence into security_state.rs beside the ban and reputation state.
Crate-root imports preserve both route dispatchers and compatibility callers.
Ban matching, rollback, and durable deletion behavior remain unchanged. lib.rs
is now 10,421 lines.

The locked default library check and all 638 default library tests pass. Six
existing security-ban regressions and two security-unban regressions pass with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter and runtime-boundary guard pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 User and Contact Projection Persistence Helper Ownership (Batch 247, 2026-09-26)

Moved checked user-projection persistence into user_store.rs and contact
upsert/delete persistence into contact_state.rs. Crate-root imports preserve
route, session, and legacy-dispatch callers; the direct user-projection
concurrency regression now addresses its owner module. Persisted projections
and failure behavior remain unchanged. lib.rs is now 10,375 lines.

The locked default library check and all 638 default library tests pass. Fourteen
existing persistence-order regressions and fourteen existing route rollback
regressions pass with full-controller-tests. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Share Grant and Access Token Persistence Helper Ownership (Batch 248, 2026-09-26)

Moved checked share-grant upsert/delete and access-token expiry cleanup plus
digest persistence into share_grant_store.rs beside the grant and token stores.
Crate-root imports preserve both route dispatchers. Token cleanup ordering,
digest-only persistence, and grant rollback behavior remain unchanged. lib.rs is
now 10,327 lines.

The locked default library check and all 638 default library tests pass. Thirteen
existing share-grant regressions and two access-token persistence regressions
pass with full-controller-tests. The warning-free locked full-controller/legacy
library check and scoped Clippy with -D clippy::await_holding_lock pass.
Changed-file formatter and runtime-boundary guard pass. No tests were added.
This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.


## RF-024 Share Group Persistence Helper Ownership (Batch 249, 2026-09-26)

Moved checked share-group upsert/delete persistence and unchanged-store
comparison into share_group_store.rs beside ShareGroupStore. Crate-root imports
preserve both route dispatchers. Persisted record conversion, rollback matching,
and failure behavior remain unchanged. lib.rs is now 10,266 lines.

The locked default library check and all 638 default library tests pass. The
three existing share-group member revocation, group revocation, and write
rollback regressions pass with full-controller-tests. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Collection Persistence Helper Ownership (Batch 250, 2026-09-26)

Moved checked collection create/update/delete persistence and conditional
rollback into collection_store.rs beside CollectionStore. Crate-root imports
preserve route and extended-controller callers; the stale-parent regression now
calls the owner module. Collection snapshots and rollback behavior remain
unchanged. lib.rs is now 10,188 lines.

The locked default library check and all 638 default library tests pass. The
stale-parent regression passes with focused-controller-tests; existing
collection route rollback, grant-revocation rollback, and responsive-read during
delete regressions pass with full-controller-tests. The warning-free locked
full-controller/legacy library check and scoped Clippy with
-D clippy::await_holding_lock pass. Changed-file formatter and runtime-boundary
guard pass. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-024 Library Persistence Helper Ownership (Batch 251, 2026-09-26)

Moved library record conversion, checked single/batch upserts and deletion, and
conditional rollback into library_store.rs beside LibraryStore. Crate-root
imports preserve route, legacy-dispatch, and extended-controller callers.
Persisted row conversion and failure behavior remain unchanged. lib.rs is now
10,128 lines.

The locked default library check and all 638 default library tests pass. The
existing library snapshot update/delete ordering regression passes with
focused-controller-tests, and the library route rollback regression passes with
full-controller-tests. The warning-free locked full-controller/legacy library
check and scoped Clippy with -D clippy::await_holding_lock pass. Changed-file
formatter and runtime-boundary guard pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.


## RF-024 Now-Playing Persistence Helper Ownership (Batch 252, 2026-09-26)

Moved checked now-playing upsert and clear persistence into
now_playing_store.rs beside NowPlayingStore. Crate-root imports preserve both
route dispatchers. Persisted values and update/clear ordering remain unchanged.
lib.rs is now 10,101 lines.

The warning-free locked default library check and all 638 default library tests
pass. The existing update/clear ordering regression passes with
focused-controller-tests, and the now-playing route rollback regression passes
with full-controller-tests. The warning-free locked full-controller/legacy
library check and scoped Clippy with -D clippy::await_holding_lock pass.
Changed-file formatter and runtime-boundary guard pass. No tests were added.
This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.


## RF-024 Shared Controller List Filter Ownership (Batch 256, 2026-09-26)

Moved `RecordListFilter` and `parse_list_limit` into `record_list_filter.rs`;
existing route and store imports continue through the crate root. Filter
defaults, query parsing, and list-limit bounds are unchanged. This ownership
split moves 49 lines and reduces `lib.rs` from 9,332 to 9,286 lines after
module wiring. It is a small cleanup and does not materially resolve the
oversized controller root.

The locked default library check and all 638 default library tests pass. The
existing filtered-list and pagination route regression passes with
`full-controller-tests`; the warning-free combined full-controller/legacy
library check and scoped Clippy with `-D clippy::await_holding_lock` pass.
Changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress.


## RF-024 Spotify Integration Ownership (Batch 257, 2026-09-26)

Moved Spotify authorization URL construction, encrypted connection-file
loading and persistence, OAuth completion, token refresh, Spotify provider
requests and paging, response projections, callback HTML, and ordered
connection update/disconnect helpers into `spotify_integration.rs`. Existing
crate-root imports preserve daemon startup, both route dispatchers, provider
preview/ingestion, and regression-test callers. Persistence order, encryption,
authorization, and response behavior are unchanged. `lib.rs` is now 8,494
lines; the new module is 810 lines.

The warning-free locked default library check and all 638 default library tests
pass. The existing full-controller Spotify authorization/callback differential
passes. The locked combined full-controller/legacy library check, scoped Clippy
with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, and diff check pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.

## RF-024 Controller Status Response Ownership (Batch 262, 2026-09-26)

Moved health and mesh-health responses, federation diagnostics, shared
pod-signature and auth-warning projections, version and Swagger responses, and
metrics path/authentication helpers into `controller_status.rs`. Existing
crate-root imports preserve route, daemon, HTTP-connection, and legacy-dispatch
callers. The constant-time byte comparator remains at the crate root because
mesh-gateway authentication also uses it. Response and authentication behavior
remain unchanged. `lib.rs` is now 6,217 lines; the new module is 231 lines.

The warning-free locked default library check and the combined
full-controller/legacy library check pass. Existing health-warning,
federation-diagnostics, metrics-authentication, and target-specific Swagger
regressions all pass with `full-controller-tests`. Scoped Clippy with
`-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary
guard, and diff checks pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.

## RF-024 SongID Controller and Runtime Ownership (Batch 263, 2026-09-26)

Moved library/share matching, SongID run projections, and source
classification into `songid_controller.rs`; queued run creation and local-file
safe-root checks now live in `songid_runtime.rs` beside the workers and run
lifecycle. Crate-root imports preserve route, legacy, worker, and regression
callers. Matching, source policy, queue behavior, and response data are
unchanged. `lib.rs` is now 5,949 lines; `songid_controller.rs` is 653 lines and
`songid_runtime.rs` is 821 lines.

The warning-free locked default library check and combined full-controller/
legacy library check pass. Existing SongID matching, source-classification and
local-root, and run-lifecycle regressions all pass with
`full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Notification Transport and Dispatch Ownership (Batch 264, 2026-09-26)

Moved Ntfy, Pushover, and Pushbullet request construction plus private-message
and room-mention notification dispatch into `notification_runtime.rs`.
Crate-root imports preserve session-runtime dispatch and existing provider wire
regressions. Retry, cooldown, authentication, and response handling remain
unchanged. `lib.rs` is now 5,721 lines; the new module is 236 lines.

The warning-free locked default library check and combined full-controller/
legacy library check pass. The existing
`notification_integrations_emit_frozen_wire_requests` regression passes with
`full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 HTTP Request Security Helper Ownership (Batch 265, 2026-09-26)

Moved authenticated rate-limit identity, trusted-proxy address resolution,
forwarded-header parsing, and WebSocket subprotocol credential parsing into
`request_security.rs`. Crate-root imports preserve the HTTP connection layer
and existing direct contract regressions; two directly tested forwarded-header
parsers remain re-exported for test callers. Trust-chain and credential
handling remain unchanged. `lib.rs` is now 5,498 lines; the new module is 229
lines.

The warning-free locked default library check and combined full-controller/
legacy library check pass. Five trusted-proxy tests, three WebSocket credential
tests, and the WebSocket event-feed authorization test pass with
`full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Daemon Runtime Setup Ownership (Batch 266, 2026-09-26)

Moved TCP and Unix HTTP listener binding, controller TLS acceptor setup,
startup telemetry initialization, and private state/storage directory
preparation into `daemon_runtime_setup.rs`. Crate-root imports preserve daemon
startup, Spotify storage setup, and regression callers. Socket, TLS, telemetry,
and filesystem behavior remain unchanged. `lib.rs` is now 5,213 lines; the new
module is 292 lines.

The warning-free locked default library check and combined full-controller/
legacy library check pass. Existing private-state symlink and Spotify OAuth
callback regressions pass with `full-controller-tests`. Scoped Clippy with
`-D clippy::await_holding_lock`, changed-file formatter, runtime-boundary
guard, and diff checks pass. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.

## RF-024 Route Dispatch Catalog and Adversarial Controller Ownership (Batch 267, 2026-09-26)

Moved shared decoded-segment handling and extended-controller route
classification into `route_dispatch_catalog.rs`, beside dispatch ownership.
Moved the native adversarial-settings mutation response into
`security_controller.rs`, where the extended controller can call the same
root-imported handler. Route matching, settings validation, and persistence
behavior remain unchanged. `lib.rs` is now 4,874 lines; the route catalog is
265 lines and `security_controller.rs` is 459 lines.

The warning-free locked default library check and combined full-controller/
legacy library check pass. Existing materialized dynamic-route and adversarial
mutation differential regressions pass with `full-controller-tests`. Scoped
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.

## RF-024 Security Reputation and Ban Helper Ownership (Batch 268, 2026-09-26)

Moved reputation score/trust projections, profile response shaping, ban-value
normalization, ban-option decoding, and ban-route parsing into
`security_controller.rs` with the security handlers. Crate-root imports
preserve both dispatchers and direct route regressions. Authentication,
canonicalization, and response behavior remain unchanged. `lib.rs` is now
4,689 lines; `security_controller.rs` is 647 lines.

The warning-free locked default library check and combined full-controller/
legacy library check pass. The security-reputation route regression and all
three matching security-ban route regressions pass with
`full-controller-tests`. Scoped Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Application State Type Ownership (Batch 269, 2026-09-26)

Moved the `AppState` type into `app_state.rs`; its fields are visible to
existing crate-internal route and runtime modules. Crate-root imports preserve
current constructors, projections, and callers without changing state layout
or lifecycle behavior. `lib.rs` is now 4,528 lines; `app_state.rs` is 165 lines.

The warning-free locked default library check and all 638 default library tests
pass. The combined full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.

## RF-024 AppState Accessor and Managed Task Ownership (Batch 270, 2026-09-26)

Moved the application-state projections, listener reconfiguration helpers,
user-info projections, and managed-task spawn and shutdown methods beside
`AppState` in `app_state.rs`. Existing crate-internal callers retain the same
entry points and lifecycle behavior. `lib.rs` is now 4,335 lines;
`app_state.rs` is 370 lines.

The warning-free locked default library check and all 638 default library tests
pass. The combined full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. Changed-file formatter,
runtime-boundary guard, and diff checks pass. No tests were added. This is
internal-only and adds no release-note fragment. RF-024 and RF-002 remain in
progress; the latest complete release gate remains after Batch 71.

## RF-024 HTTP Request Security Ownership (Batch 271, 2026-09-26)

Moved controller CORS policy and value validation, plus JWT revocation checks,
into `request_security.rs` alongside trusted-proxy and WebSocket credential
handling. Route and connection callers retain their existing crate-internal
entry points; header projection and authorization behavior are unchanged.
`lib.rs` is now 4,220 lines; `request_security.rs` is 350 lines.

The warning-free locked default library check passes. The combined
full-controller/legacy library check and scoped Clippy with
`-D clippy::await_holding_lock` pass. All six existing CORS regressions pass.
Changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Wishlist Persistence Ownership (Batch 272, 2026-09-26)

Moved wishlist DB record conversion, single and bulk writes, ignored-result
transactions, rollback, startup loading, and storage-failure response mapping
into `wishlist_persistence.rs`. Route and runtime call sites keep the same
crate-internal helper names and persistence behavior. `lib.rs` is now 4,045
lines; the new module is 189 lines.

The locked default library check, combined full-controller/legacy library
check, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, and diff checks pass. The existing
`wishlist_persistence_batches_across_sqlite_parameter_boundaries` regression
passes. `wishlist_routes_roll_back_when_persistence_fails` aborts with a test
thread stack overflow (exit 101), so that route regression did not complete.
No tests were added. This is internal-only and adds no release-note fragment.
RF-024 and RF-002 remain in progress; the latest complete release gate remains
after Batch 71.

## RF-024 STUN and NAT Runtime Ownership (Batch 273, 2026-09-26)

Moved STUN request encoding, mapped-address parsing, UDP probing, and NAT
classification into `mesh_dht_runtime.rs` beside the mesh runtime. The mesh
route handler and existing test callers retain crate-internal entry points;
probe and classification behavior are unchanged. `lib.rs` is now 3,929
lines; `mesh_dht_runtime.rs` is 215 lines.

The warning-free locked default and combined full-controller/legacy library
checks pass. Both STUN wire/probe tests and all three NAT-classification
regressions pass. Scoped Clippy with `-D clippy::await_holding_lock`,
changed-file formatter, runtime-boundary guard, and diff checks pass. No tests
were added. This is internal-only and adds no release-note fragment. RF-024
and RF-002 remain in progress; the latest complete release gate remains after
Batch 71.

## RF-024 Solid HTTP Ownership (Batch 274, 2026-09-26)

Moved Solid problem responses, version-dependent resolution errors, the client
ID document response, and private/reserved address policy into `solid.rs`
beside Solid profile parsing. Existing controller callers keep their imported
entry points and response behavior. `lib.rs` is now 3,838 lines;
`solid.rs` is 291 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, and diff checks pass. All four Solid
profile-parser unit tests pass. Three focused controller-level Solid tests
abort with test-thread stack overflows before assertions, so those route
regressions did not complete. No tests were added. This is internal-only and
adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Preview Stream Ticket and Media Type Ownership (Batch 275, 2026-09-26)

Moved the ticket-status projection into `preview_stream_state.rs` beside its
store, and moved the compatibility and mesh media-type selectors into
`preview_stream_controller.rs`. Route callers retain their imported helpers
and content-type behavior. `lib.rs` is now 3,778 lines; preview state is 172
lines and the controller module is 1,171 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, and diff checks pass. The local, peer, and
mesh preview streaming regressions pass. The differential ticket-status test
aborts with a test-thread stack overflow before assertions, so that route test
did not complete. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.

## RF-024 Capability Identity Ownership (Batch 276, 2026-09-26)

Moved secure peer-capability key generation and persistence, signed local
capability descriptor construction, local peer ID projection, and friend-code
formatting into `controller_capabilities.rs`. Daemon startup and route callers
retain their imported helpers; key permissions and identity behavior are
unchanged. `lib.rs` is now 3,646 lines; `controller_capabilities.rs` is 674
lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, and diff checks pass. Both existing
capability identity regressions pass, covering stable reload and symlink
rejection. No tests were added. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 OAuth State Ownership (Batch 277, 2026-09-26)

Moved secure OAuth state and Spotify PKCE verifier generation plus persisted
OAuth-state loading into `oauth_state.rs` beside the state store and lifecycle.
Daemon startup and cross-module token callers retain their imports; token
format, randomness handling, expiry cleanup, and persistence behavior are
unchanged. `lib.rs` is now 3,610 lines; `oauth_state.rs` is 194 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, and diff checks pass. Existing deterministic
randomness and expired-state cleanup regressions pass. No tests were added.
This is internal-only and adds no release-note fragment. RF-024 and RF-002
remain in progress; the latest complete release gate remains after Batch 71.

## RF-024 Share Token Boundary Ownership (Batch 278, 2026-09-26)

Moved ShareGrant token generation into `share_grant_store.rs` and request
header/Bearer token extraction into `request_security.rs`. Grant creation and
route callers retain their imported entry points; entropy handling and
credential precedence are unchanged. `lib.rs` is now 3,581 lines;
`request_security.rs` is 373 lines and `share_grant_store.rs` is 418 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, and diff checks pass. The deterministic
token-generation regression and the header/Bearer content-bound ticket route
regression pass. No tests were added. This is internal-only and adds no
release-note fragment. RF-024 and RF-002 remain in progress; the latest
complete release gate remains after Batch 71.

## RF-024 Shared Request Input Boundary (Batch 279, 2026-09-26)

Moved shared URL component encoding, query parsing and bounded query-value
conversion, JSON body scalar extraction, and generic path-segment parsing into
`request_input.rs`. Dispatcher and controller callers retain the same helpers
through the root import; parsing behavior and limits are unchanged. `lib.rs` is
now 3,468 lines; `request_input.rs` is 124 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Native Now-Playing Webhook Ownership (Batch 280, 2026-09-26)

Moved native now-playing webhook parsing for Plex, Jellyfin/Emby, and the
compatibility payload form, together with its local set/clear helpers, into
`now_playing_store.rs`. The route caller retains its imported response helper;
accepted payloads, state updates, and response behavior are unchanged.
`lib.rs` is now 3,388 lines; `now_playing_store.rs` is 248 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Distributed Runtime Persistence Ownership (Batch 281, 2026-09-26)

Moved the distributed persistence snapshot worker, revision-aware state update
coordinator, and startup hydration into `distributed_runtime.rs` beside the
connection and hierarchy lifecycle. Daemon startup, session startup, and
controller-test callers retain imported helpers; revision ordering, write
serialization, and persistence behavior are unchanged. `lib.rs` is now 3,319
lines; `distributed_runtime.rs` is 750 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Runtime Compatibility Mutation Ownership (Batch 282, 2026-09-26)

Moved the durable and in-memory runtime compatibility mutation coordinators
into `runtime_compat_state.rs` beside the state and persistence projection.
Route, runtime, and reload callers retain their imported helpers; persistence
serialization, guard release during SQLite I/O, and compare-before-rollback
behavior are unchanged. `lib.rs` is now 3,269 lines;
`runtime_compat_state.rs` is 654 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Mesh Range Source Discovery Ownership (Batch 283, 2026-09-26)

Moved trusted-mesh and capability-record range-source discovery into
`transfer_recovery_runtime.rs` beside its recovery caller. Multisource route
and legacy-dispatch callers retain the same imported helper; peer selection,
source deduplication, endpoint choice, and source limit are unchanged.
`lib.rs` is now 3,205 lines; `transfer_recovery_runtime.rs` is 799 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Listening-Party Normalization Ownership (Batch 284, 2026-09-26)

Moved the listening-party event normalizer into `misc_controller_mutations.rs`
beside the mutation route that consumes it. Event vocabulary, content ID
validation, server-derived fields, tag bounds, and response handling are
unchanged. `lib.rs` is now 3,109 lines; `misc_controller_mutations.rs` is 694
lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Route Request Entry Ownership (Batch 285, 2026-09-26)

Moved feature-gated selection between the split route dispatcher and retained
compatibility dispatcher into `route_request_entry.rs`. The selector keeps the
stack-safe hash-merge, library-browser, and stream paths together with the
compatibility fallback; route choice and response behavior are unchanged.
`lib.rs` is now 2,982 lines; `route_request_entry.rs` is 131 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Versioned Relay Route Ownership (Batch 286, 2026-09-26)

Moved versioned relay route recognition and feature-mode authorization, plus
multipart boundary parsing, into `versioned_relay_controller.rs` beside the
relay handlers. Both dispatchers and the versioned GET contract retain the
same route policy through the root import; accepted routes, mode gating, and
media-type behavior are unchanged. `lib.rs` is now 2,919 lines;
`versioned_relay_controller.rs` is 701 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Store Size-Bound Ownership (Batch 287, 2026-09-26)

Moved room count and text bounds into `room_store.rs`, browse record and text
bounds into `browse_store.rs`, browse wire payload caps into `browse_wire.rs`,
message bounds into `message_store.rs`, and username bounds into
`user_store.rs`. Existing values and enforcement are unchanged. Existing
controller checks now reference bounds through their owning modules where
needed. `lib.rs` is now 2,898 lines; the owner modules are 665, 631, 305, 517,
and 329 lines respectively.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Pod Controller and Startup Helper Ownership (Batch 288, 2026-09-26)

Moved pod route segment parsing and the versioned blank-segment response into
`podcore_controller.rs`; local peer identity, moderation checks, and the
Gold Star Club setting wrapper into `pod_membership_workflow.rs`; and pod and
channel store recovery into `daemon_serve.rs`. Existing route layouts,
decoding and blank-value rules, credential precedence, moderator policy, and
orphan-channel cleanup behavior are unchanged. Shared callers keep their
existing helper names, while startup recovery checks reference the daemon
owner. `lib.rs` is now 2,785 lines; the owner modules are 4,124, 701, and
1,141 lines respectively.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Podcore Route Module Ownership (Batch 289, 2026-09-26)

Split podcore mutation dispatch, dynamic GET handling, and stats handling out
of `podcore_controller.rs` into `podcore_mutations.rs`, `podcore_queries.rs`,
and `podcore_stats_controller.rs`. The controller retains shared pod runtime
helpers and re-exports the same route entry points. Request matching,
validation, state updates, and response construction are unchanged. The
controller file is now 1,042 lines; the new route modules are 1,750, 883, and
463 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Extended Controller Route Ownership (Batch 290, 2026-09-26)

Split extended-controller mutation, dynamic GET, and GET handling into
`extended_controller_mutations.rs`, `extended_controller_dynamic_get.rs`, and
`extended_controller_get.rs`. `extended_controller.rs` retains shared helper
logic and re-exports the existing handler names, so dispatcher routing and
response behavior are unchanged. The former 3,829-line controller is now a
1,031-line owner module plus route modules of 1,195, 639, and 978 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Extended Controller Module Split (Batch 291, 2026-09-26)

Moved the extended-controller mutation entry point, dynamic GET handler, and
GET handler into dedicated nested route modules. The shared parent retains
search, download, and common response helpers; existing crate-root handler
imports and routing call sites are unchanged. `extended_controller.rs` is now
1,031 lines; its three route modules are 1,195, 639, and 978 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 MediaCore Route Ownership (Batch 292, 2026-09-26)

Moved MediaCore GET and mutation handling into `mediacore_extended.rs` and
`mediacore_mutations.rs`. `mediacore_controller.rs` retains metric recording,
descriptor verification and retrieval, and shared publication helpers. The
existing handler names and route responses are unchanged. The owner module is
now 503 lines; the route modules are 819 and 1,233 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Controller YAML Ownership Split (Batch 293, 2026-09-26)

Moved controller YAML key conversion and API projection into
`controller_yaml_projection.rs`, and target-profile validation and error
contracts into `controller_yaml_validation.rs`. Bounded YAML parsing and
configuration response helpers remain in `controller_yaml.rs`. Existing
root imports and validation outcomes are unchanged. The modules are now 83,
633, and 1,625 lines respectively.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Shared Request Bounds Ownership (Batch 294, 2026-09-26)

Moved generic JSON-array limits, field-array parsing, and bounded browse,
search, conversation, wishlist, and collection response checks into
`request_input.rs` beside query, path, and JSON-body parsing. Existing shared
helper names and caps are unchanged. `lib.rs` is now 2,706 lines;
`request_input.rs` is 203 lines. The runtime-boundary guard now scans the new
owner module for its existing size-limit anchors.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Controller Route Input Ownership (Batch 295, 2026-09-26)

Moved collection, wishlist, VirtualSoulfind, share, user, room, bridge,
library-health, and share-group path parsing and blank-identifier response
contracts into `controller_route_inputs.rs`. Both dispatchers and route
handlers keep their existing helper names and validation behavior. `lib.rs`
is now 2,301 lines; `controller_route_inputs.rs` is 417 lines.

The warning-free locked default and combined full-controller/legacy library
checks, scoped Clippy with `-D clippy::await_holding_lock`, changed-file
formatter, runtime-boundary guard, plan freshness, docs freshness, and diff
checks pass. No tests were added or run in this batch. This is internal-only
and adds no release-note fragment. RF-024 and RF-002 remain in progress; the
latest complete release gate remains after Batch 71.

## RF-024 Legacy Dispatcher Route Module Split (Batch 296, 2026-09-26)

Split the feature-gated compatibility dispatcher’s 399 route arms, in their
existing order, across eleven `legacy_route_dispatch_group_00.rs` through
`legacy_route_dispatch_group_10.rs` modules. The parent dispatcher is now 710
lines; the route modules range from 918 to 2,019 lines. Shared request context
keeps the parsed route, normalized `String` path, authorization, body, headers,
and dispatch flags. Group functions retain the original early-return and
request-tracing behavior, and the final unmatched-route response remains in the
parent. The runtime-boundary inventory now scans all eleven groups and the
`transfer_batch_controller.rs` owner alongside the parent dispatcher.

The locked default and combined full-controller/legacy library checks, scoped
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, docs freshness, and diff checks pass.
No tests were added or run. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Bounded Dispatcher Group 6 Ownership Split (Batch 297, 2026-09-26)

Split the 4,137-line `route_dispatch_group_6.rs` match into six ordered route
modules for admin/discovery, telemetry, media jobs, security and shares,
integrations, and MusicBrainz. The parent group is now a 19-line sequential
router; the new modules range from 135 to 1,342 lines. The existing
`source_discovery_state_matches` helper moved with the discovery handlers.
Each group returns the existing unhandled sentinel so route precedence and the
outer response-completion path remain unchanged. The runtime-boundary inventory
scans all six new modules.

The locked default and combined full-controller/legacy library checks, scoped
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, docs freshness, and diff checks pass.
No tests were added or run. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Bounded Dispatcher Group 2 Ownership Split (Batch 298, 2026-09-26)

Split the 3,372-line `route_dispatch_group_2.rs` match into five ordered
modules for session/search, downloads, transfer status, transfer file handling,
and search/room routes. The parent is now 16 lines; the modules range from 412
to 958 lines. `remove_transfer_file_if_present` moved with the transfer-file
handlers. Each module returns the existing unhandled sentinel, preserving route
precedence and the outer response-completion path. The runtime-boundary
inventory scans all five modules.

The locked default and combined full-controller/legacy library checks, scoped
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, docs freshness, and diff checks pass.
No tests were added or run. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.

## RF-024 Bounded Dispatcher Groups 4, 5, and 7 Ownership Split (Batch 299, 2026-09-26)

Split `route_dispatch_group_4.rs` (2,069 lines), `route_dispatch_group_5.rs`
(2,228 lines), and `route_dispatch_group_7.rs` (2,497 lines) into four ordered
route modules each. The three parent groups are now 13 lines apiece; the new
modules range from 376 to 929 lines. The new groups keep collection, wishlist,
contact, library, conversation, bridge, media, stream, PodCore, and network
handlers in focused files while preserving the original source order and the
existing unhandled-sentinel chain. The runtime-boundary inventory scans every
new module.

The locked default and combined full-controller/legacy library checks, scoped
Clippy with `-D clippy::await_holding_lock`, changed-file formatter,
runtime-boundary guard, plan freshness, docs freshness, and diff checks pass.
No tests were added or run. This is internal-only and adds no release-note
fragment. RF-024 and RF-002 remain in progress; the latest complete release
gate remains after Batch 71.


## RF-058 PodCore DHT Publishing Panel Extraction (Batch 300, 2026-09-26)

Moved the PodCore DHT publishing workflow state, API actions, and UI from
`MediaCore/index.jsx` into the memoized `PodDhtPublishingPanel.jsx` owner. The
panel retains publish/retrieve/unpublish/statistics actions, warning and
confirmation behavior, and result/error views. The parent MediaCore file fell
from 8,547 to 8,192 lines.

The retained JSDOM profiler warms the Pod JSON input three times, then measures
20 input commits. Median/p95 changed from 9.425/13.968 ms before extraction to
1.026/1.167 ms afterward. The focused MediaCore test file passes all 18 tests;
the Web production build, targeted ESLint, and diff checks pass. The prior full
Web suite result remains historical and was not rerun in this batch. The
measurement is an in-JSDOM comparison, not a deployed-browser performance
claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Membership Management Panel Extraction (Batch 301, 2026-09-26)

Moved Pod Membership Management state, API actions, and UI from
`MediaCore/index.jsx` into the memoized `PodMembershipManagementPanel.jsx`.
The pre-existing `verifyingMembership` loading flag is also used by the separate
verification workflow, so the panel receives that value and setter from its
parent; the remaining management form state and actions are local. The parent
file fell from 8,192 to 7,676 lines.

The retained JSDOM profiler warms the membership JSON input three times, then
measures 20 input commits. Median/p95 changed from 9.308/17.579 ms before
extraction to 1.533/1.909 ms afterward. All 19 focused MediaCore tests, the Web
production build, targeted ESLint, and diff checks pass. The earlier full Web
suite result was not rerun in this batch. The measurement is an in-JSDOM
comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Discovery Panel Extraction (Batch 302, 2026-09-26)

Moved Pod Discovery state, API actions, and UI from `MediaCore/index.jsx` into
the memoized `PodDiscoveryPanel.jsx` owner. Registration/unregistration,
name/tag/content lookup, full scans, and discovery statistics remain in their
existing sequence and retain their current presentation. The composition file
fell from 7,676 to 7,112 lines.

The retained JSDOM profiler warms the registration JSON input three times, then
measures 20 input commits. Median/p95 changed from 8.254/16.453 ms before
extraction to 2.033/2.626 ms afterward. The focused MediaCore suite passes all
20 tests on rerun; an earlier run had one transient ContentID example assertion
fail, which passed both in isolation and on the full-file rerun. The Web
production build, targeted ESLint, and diff checks pass. The measurement is an
in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Join/Leave Panel Extraction (Batch 303, 2026-09-26)

Moved Pod Join/Leave state, API actions, and UI from `MediaCore/index.jsx` into
the memoized `PodJoinLeavePanel.jsx`. Join requests, acceptances, leave
requests, pending-request reads, and their results/errors remain grouped under
one owner. The composition file fell from 7,112 to 6,734 lines.

The retained JSDOM profiler warms the join-request JSON input three times, then
measures 20 input commits. Two baseline runs measured 9.208 ms median/125.933
ms p95 and 8.650 ms median/17.240 ms p95; the 125 ms sample set contains an
obvious environment stall. After extraction the measurement was 1.415 ms
median/1.854 ms p95. The focused MediaCore suite passes all 21 tests; the Web
production build, targeted ESLint, and diff checks pass. This is an in-JSDOM
comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Message Routing Panel Extraction (Batch 304, 2026-09-26)

Moved message routing, targeted-peer routing, deduplication checks, and routing
statistics state/actions/UI into `PodMessageRoutingPanel.jsx`. The search-index
rebuild and database vacuum controls appear in both the routing and storage
views, so their state/actions now live in shared `PodMessageMaintenanceActions.jsx`
with the existing small/medium button sizing preserved. The composition file
fell from 6,734 to 6,266 lines.

The retained JSDOM profiler warms the routing-message JSON input three times,
then measures 20 input commits. Median/p95 changed from 8.088/13.211 ms before
extraction to 1.361/1.723 ms afterward. All 22 focused MediaCore tests pass;
the Web production build, targeted ESLint, and diff checks pass. This is an
in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Message Backfill Panel Extraction (Batch 305, 2026-09-26)

Moved Pod Message Backfill state, API actions, derived last-seen timestamp
validity, and UI from `MediaCore/index.jsx` into the memoized
`PodMessageBackfillPanel.jsx`. Backfill-stat response validation, current-pod
checks, error preservation, and stale-target reset behavior remain in the
owner. The composition file fell from 6,266 to 5,992 lines.

The retained JSDOM profiler warms the backfill Pod ID input three times, then
measures 20 input commits. Median/p95 changed from 7.433/13.452 ms before
extraction to 0.596/0.923 ms afterward. All 23 focused MediaCore tests pass;
the Web production build, targeted ESLint, and diff checks pass. This is an
in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Channel Management Panel Extraction (Batch 306, 2026-09-26)

Moved Pod Channel Management state, API actions, current-channel projection,
and UI from `MediaCore/index.jsx` into the memoized
`PodChannelManagementPanel.jsx`. Channel listing, create/update/delete actions,
editing state, and stale-pod result filtering remain together under the panel.
The composition file fell from 5,992 to 5,649 lines.

The retained JSDOM profiler warms the channel-management Pod ID input three
times, then measures 20 input commits. A stable baseline run measured
7.813/13.184 ms median/p95; a second baseline run measured 9.938/99.999 ms and
contained a clear environment stall. After extraction the measurement was
0.615/0.817 ms. All 24 focused MediaCore tests pass; the Web production build,
targeted ESLint, and diff checks pass. This is an in-JSDOM comparison, not a
deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Content Linking Panel Extraction (Batch 307, 2026-09-26)

Moved Pod Content Linking actions, search/validation state, and UI into the
memoized `PodContentLinkingPanel.jsx`. The `contentId` value/setter stays in the
parent because the separate Content Registry registration handler clears this
field; the panel receives that shared state while search and result state remain
local. The composition file fell from 5,649 to 5,346 lines.

The retained JSDOM profiler warms the content-search input three times, then
measures 20 input commits. Median/p95 changed from 7.040/14.013 ms before
extraction to 0.548/0.801 ms afterward. All 25 focused MediaCore tests pass;
the Web production build, targeted ESLint, and diff checks pass. This is an
in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Opinions Panel Extraction (Batch 308, 2026-09-26)

Moved Pod Opinion Management and aggregation state, query projections, actions,
and UI into the memoized `PodOpinionsPanel.jsx`. The MediaCore composition file
fell from 5,346 to 4,527 lines.

The retained JSDOM profiler warms the opinion query input three times, then
measures 20 input commits. Median/p95 changed from 7.002/16.247 ms before
extraction to 1.149/1.493 ms afterward. All 26 focused MediaCore tests pass;
the Web production build, targeted ESLint, and diff checks pass. This is an
in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 Pod Message Signing Panel Extraction (Batch 309, 2026-09-26)

Moved Pod Message Signing state, actions, and UI into the memoized
`PodMessageSigningPanel.jsx`. The shared `messageToVerify` input and
`verificationResult` remain parent-owned because membership message verification
and descriptor verification also use them. The composition file fell from
4,527 to 4,237 lines.

The retained JSDOM profiler warms the signing-message input three times, then
measures 20 input commits. Median/p95 changed from 6.138/14.396 ms before
extraction to 1.068/1.468 ms afterward. All 27 focused MediaCore tests pass;
the Web production build, targeted ESLint, and diff checks pass. This is an
in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.


## RF-058 MediaCore Statistics Dashboard Panel Extraction (Batch 310, 2026-09-26)

Moved the MediaCore dashboard, registry, descriptor, fuzzy-matching,
perceptual-hashing, IPLD, portability, and publishing statistics state, API
actions, reset behavior, and cards into the memoized
`MediaCoreStatisticsDashboardPanel.jsx`. The composition file fell from 4,237
to 3,469 lines.

The retained JSDOM profiler warms the Registry Stats action three times, then
measures 20 update commits. Two baseline runs measured 5.530/66.832 ms and
5.599/66.517 ms median/p95; both had event-loop stalls among the first three
samples. After extraction it measured 1.969/2.468 ms. All 28 focused MediaCore
tests pass; the Web production build, targeted ESLint, and diff checks pass.
This is an in-JSDOM comparison, not a deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Content Descriptor Publishing and Retrieval Split (Batch 311, 2026-09-26)

Split descriptor creation, batch publishing, descriptor updates, and publishing
statistics into `ContentDescriptorPublishingPanel.jsx`; retrieval, batch
retrieval, domain query, verification, and cache controls now live in
`ContentDescriptorRetrievalPanel.jsx`. Both panels own their workflow state and
actions. `MediaCoreDescriptorManagementPanel.jsx` remains a 14-line composition
component. The MediaCore composition file fell from 3,469 to 2,461 lines.

The retained JSDOM profiler warms the Content Descriptor Publishing input three
times, then measures 20 controlled-input commits. Two baseline runs measured
11.344/42.541 ms and 11.689/43.528 ms median/p95; the first six samples in both
runs show event-loop stalls in the 41–44 ms range. After extraction it measured
1.969/2.061 ms. All 29 focused MediaCore tests pass; the Web production build,
targeted ESLint, and diff checks pass. This is an in-JSDOM comparison, not a
deployed-browser performance claim.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Content Registry and Graph Panel Extraction (Batch 312, 2026-09-26)

Moved ContentID registration, resolution, validation, domain search, and example
field coordination into the memoized `MediaCoreContentRegistryPanel.jsx`.
Registration result refreshes still update the parent-owned registry summary,
and the shared `contentId` setter remains parent-owned for Pod Content Linking.
Moved traversal, graph lookup, and inbound-link state, API actions, and cards
into the memoized `MediaCoreContentGraphPanel.jsx`. The MediaCore composition
file fell from 2,461 to 1,851 lines.

The retained JSDOM profilers warm their target inputs three times, then measure
20 controlled-input commits. ContentID Registry median/p95 changed from
4.906/22.814 ms to 1.776/2.237 ms; Content Graph changed from 5.523/23.195 ms to
1.196/1.502 ms. The baseline p95s include JSDOM event-loop delays. All 31
focused MediaCore tests pass; the Web production build, targeted ESLint, and diff
checks pass. These are in-JSDOM comparisons, not deployed-browser performance
claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Metadata Portability Panel Extraction (Batch 313, 2026-09-26)

Moved Metadata Portability export/import state, conflict-strategy loading,
actions, and both cards into the memoized
`MediaCoreMetadataPortabilityPanel.jsx`. The MediaCore composition file fell
from 1,851 to 1,535 lines.

The retained JSDOM profiler warms the metadata import input three times, then
measures 20 controlled-input commits. Two baseline runs measured 7.502/14.822 ms
and 5.827/15.078 ms median/p95, with event-loop stalls visible in both sample
sets. After extraction it measured 1.124/1.343 ms. The focused MediaCore suite
passed all 32 tests on its final run. The first full run had one unrelated Pod
Opinions refresh failure; that test passed alone and in the full rerun. The Web
production build, targeted ESLint, and diff checks pass. These are in-JSDOM
measurements, not deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Pod Membership Verification Panel Extraction (Batch 314, 2026-09-26)

Moved Pod Membership Verification's local state, API actions, and UI into the
memoized `PodMembershipVerificationPanel.jsx`. The Pod membership loading flag
remains parent-owned because Pod Membership Management uses the same flag. The
message input remains parent-owned because Message Signing shares it. The
MediaCore composition file fell from 1,535 to 1,177 lines.

The retained JSDOM profiler warms the membership Pod ID input three times, then
measures 20 controlled-input commits. Two baselines measured 10.753/12.026 ms
and 9.419/10.707 ms median/p95, with event-loop stalls in both runs. After
extraction it measured 1.378/1.647 ms. All 33 focused MediaCore tests pass; the
Web production build, targeted ESLint, and diff checks pass. These are
in-JSDOM measurements, not deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Pod Message Storage Panel Extraction (Batch 315, 2026-09-26)

Moved Pod Message Storage loading/result state, stats and cleanup actions, and
the retention card into the memoized `PodMessageStoragePanel.jsx`. The existing
Message Search and Message Maintenance panels are composed inside it. The
MediaCore composition file fell from 1,177 to 1,062 lines.

The retained JSDOM profiler warms the Get Storage Stats action three times,
then measures 20 update commits. Two baseline runs measured 7.426/7.779 ms and
8.518/9.123 ms median/p95, with high/low sample clusters in both runs. After
extraction it measured 0.442/0.696 ms. The final focused MediaCore suite passes
all 34 tests. The first full run had one Pod Opinions statistics refresh
failure; that test passed alone and in the full rerun. The Web production build,
targeted ESLint, and diff checks pass. These are in-JSDOM measurements, not
deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Content Hashing Panel Extraction (Batch 316, 2026-09-26)

Moved audio and image hash state, API actions, and cards into the memoized
`MediaCoreContentHashingPanel.jsx`. Supported-algorithm metadata remains in the
composition root because its summary is rendered after the other workflows;
the hashing panel receives that metadata as a stable prop. The root fell from
1,062 to 820 lines.

The retained JSDOM profiler warms the audio samples input three times, then
measures 20 controlled-input commits. Two baselines measured 6.739/8.212 ms and
6.531/7.305 ms median/p95. After extraction it measured 1.037/1.144 ms. All 35
focused MediaCore tests pass; the Web production build, targeted ESLint, and diff
checks pass. These are in-JSDOM measurements, not deployed-browser performance
claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Similarity Analysis Panel Extraction (Batch 317, 2026-09-26)

Moved raw hash comparison, fuzzy content search, perceptual ContentID
comparison, and text similarity state, actions, and cards into the memoized
`MediaCoreSimilarityAnalysisPanel.jsx`. The MediaCore composition root fell
from 820 to 369 lines. The new panel is 518 lines and remains open for a finer
ownership split.

The retained JSDOM profiler warms the first hash input three times, then
measures 20 controlled-input commits. Two isolated baselines measured
5.719/5.995 ms and 4.932/5.204 ms median/p95. Two isolated post-extraction runs
measured 1.948/2.347 ms and 1.909/2.335 ms. The same profile inside the full
suite measured 0.550/0.723 ms, demonstrating JSDOM timing variation; the
isolated comparison is recorded here. All 36 focused MediaCore tests pass; the
Web production build, targeted ESLint, and diff checks pass. These are
in-JSDOM measurements, not deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 remains open for the remaining MediaCore and Integrations ownership
boundaries.

## RF-058 Similarity Workflow Subpanels (Batch 318, 2026-09-26)

Split the 518-line `MediaCoreSimilarityAnalysisPanel.jsx` into the 16-line
composition file plus `MediaCoreHashSimilarityPanel.jsx`,
`MediaCoreFuzzyContentPanel.jsx`, `MediaCorePerceptualSimilarityPanel.jsx`, and
`MediaCoreTextSimilarityPanel.jsx`. Each focused panel owns its state, API
actions, and card. The MediaCore root remains 369 lines.

The retained JSDOM profiler warms the raw hash input three times, then measures
20 controlled-input commits. The combined panel measured 1.948/2.347 ms and
1.909/2.335 ms median/p95 on two isolated runs. After the finer split it
measured 0.558/0.670 ms. A full-suite run initially had two intermittent
assertions in existing MediaCore workflows; the subsequent full rerun passed
all 36 tests. The Web production build, targeted ESLint, and diff checks pass.
These are in-JSDOM measurements, not deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
RF-058 still has the Integrations boundary open.

## RF-058 Integrations Helpers and Source Feed Panel (Batch 319, 2026-09-26)

Moved shared option accessors, status formatting, async guards, and form
builders into `integrationsShared.jsx`. Moved the Spotify, YouTube, and Last.fm
settings workflow and its Spotify redirect URI helper into
`SourceFeedIntegrationsPanel.jsx`; `index.jsx` remains the composer and
re-exports the existing redirect helper for its current caller. The composition
file fell from 3,915 to 3,264 lines.

The retained JSDOM profiler warms Spotify client-ID input three times, then
measures 20 controlled-input commits. Two before runs measured 1.651/1.859 ms
and 1.657/1.937 ms median/p95; after extraction it measured 1.626/2.023 ms. A
full-suite profile measured 1.569/2.020 ms. Timing stayed within the observed
range, consistent with the workflow already owning local React state before its
source file was split. All 18 focused Integrations tests pass. The Web
production build, targeted ESLint, and diff checks pass. These are in-JSDOM
measurements, not deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
The remaining Integrations panels still need ownership extraction.


## RF-058 Integrations Notification Panel Extraction (Batch 320, 2026-09-26)

Moved Notifications form state, apply/save actions, and UI into
`NotificationIntegrationsPanel.jsx`. Its state and behavior are unchanged; the
composition root fell from 3,264 to 2,628 lines. The extracted component body
was checked against the pre-edit snapshot.

The retained JSDOM profiler warms the Pushbullet title-prefix field three times,
then measures 20 controlled-input commits. Two baselines measured 2.099/2.870 ms
and 2.121/2.840 ms median/p95. Two after runs measured 2.210/2.858 ms and
2.243/3.109 ms. The measurements remain in the same noisy range, so this batch
claims a module ownership boundary and no rendering speedup. The 19-test focused
Integrations suite passes, including apply/save regressions; the Web production
build, targeted ESLint, and diff checks pass. These JSDOM measurements are not
deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
The remaining Integrations panels still need ownership extraction.


## RF-058 Integrations Metadata Settings Panel Extraction (Batch 321, 2026-09-26)

Moved Metadata and Servarr configuration form construction, local state, save
actions, and UI into `MetadataSettingsPanel.jsx`. The Chromaprint algorithm
options and combined metadata/Servarr form builder moved with their owner. The
Integrations composition root fell from 2,628 to 1,830 lines.

The retained JSDOM profiler warms the MusicBrainz user-agent field three times,
then measures 20 controlled-input commits. Two baselines measured 3.007/3.280 ms
and 2.908/3.167 ms median/p95. Two after runs measured 2.964/3.389 ms and
3.003/3.599 ms. The values overlap the observed range, so no rendering speedup
is claimed for the structural extraction. All 20 focused Integrations tests
pass; the Web production build, targeted ESLint, and diff checks pass. These
JSDOM measurements are not deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
The remaining Integrations panels still need ownership extraction.


## RF-058 Integrations FTP Panel Extraction (Batch 322, 2026-09-26)

Moved FTP encryption options, form construction, local state, apply/save
actions, and UI into `FtpIntegrationPanel.jsx`. The FTP configuration helpers
are no longer in the Integrations composition root, which fell from 1,830 to
1,417 lines.

The retained JSDOM profiler warms the FTP server address input three times, then
measures 20 controlled-input commits. Two baselines measured 1.478/1.751 ms and
1.396/1.720 ms median/p95. Two after runs measured 1.492/1.772 ms and
1.608/1.992 ms. These values overlap the observed range, so no rendering
speedup is claimed. All 21 focused Integrations tests pass, including FTP
secret/apply/save behavior; the Web production build, targeted ESLint, and diff
checks pass. These JSDOM measurements are not deployed-browser performance
claims.

This is internal-only refactoring, so no release-note fragment is required.
The remaining Integrations panels still need ownership extraction.


## RF-058 Integrations VPN Panel Extraction (Batch 323, 2026-09-26)

Moved the read-only VPN readiness, provider, port-forward, and self-hosted relay
status view into `VpnPanel.jsx`. It receives the existing options and runtime
state props unchanged. The Integrations composition root fell from 1,417 to
1,235 lines.

The retained JSDOM profiler changes the forwarded VPN port on 20 prop updates.
It measures the full Integrations tree because the read-only panel has no local
input. Two baselines measured 6.521/12.599 ms and 6.698/14.397 ms median/p95;
two after runs measured 6.967/12.943 ms and 6.320/13.647 ms. Timing is noisy
and shows no speedup. All 22 focused Integrations tests pass; the Web production
build, targeted ESLint, and diff checks pass. These JSDOM timings are not
deployed-browser performance claims.

This is internal-only refactoring, so no release-note fragment is required.
The remaining Integrations panels still need ownership extraction.


## RF-058 Integrations Lidarr Panel Extraction (Batch 324, 2026-09-26)

Moved the Lidarr live status, wanted-album synchronization, import-history and
retry controls, and manual import form into `LidarrPanel.jsx`. Its API actions,
error handling, and local state remain together in the panel. The Integrations
root fell from 1,235 to 884 lines.

The retained JSDOM profiler warms the manual import-directory field three
times, then measures 20 controlled-input commits. Before runs measured
1.214/1.622 ms and 1.238/1.795 ms median/p95. After runs measured
1.206/1.329 ms and 1.228/1.404 ms. Medians stayed similar; the two after p95
values were lower, which is an in-JSDOM observation rather than a browser-speed
claim. All 23 focused Integrations tests pass. The Web production build,
targeted ESLint, and diff checks pass.

This is internal-only refactoring, so no release-note fragment is required.
The remaining Integrations panels still need ownership extraction.


## RF-058 Integrations Media Server Panel Extraction (Batch 325, 2026-09-26)

Moved Media Server adapter readiness, path diagnostics, sync preview, execution
contract, state, and copy actions into `MediaServerPanel.jsx`. The panel retains
its local configuration state and the same UI. The Integrations composition
root fell from 884 to 439 lines.

The retained JSDOM profiler warms the Media Server base-URL field three times,
then measures 20 controlled-input commits. Before runs measured 3.902/4.832 ms
and 3.704/4.181 ms median/p95. After runs measured 3.996/5.000 ms and
3.650/4.227 ms. The values overlap the baseline range, so no rendering speedup
is claimed. All 24 focused Integrations tests pass, including adapter readiness,
path mapping, execution-contract, and copy behavior. The Web production build,
targeted ESLint, and diff checks pass.

This is internal-only refactoring, so no release-note fragment is required.
The Servarr readiness and federation diagnostics panels remain open.


## RF-058 Integrations Servarr Readiness Panel Extraction (Batch 326, 2026-09-26)

Moved Servarr readiness checks, compatibility preview, report copy, and the
explicit wanted-sync action into `ServarrReadinessPanel.jsx`. Its local busy and
copy state remain with that owner. The Integrations root fell from 439 to 211
lines.

The retained JSDOM profiler changes the configured Lidarr URL on 20 prop
updates. It measures the full Integrations tree. Two baselines measured
7.853/18.247 ms and 8.156/17.175 ms median/p95; after extraction, two runs
measured 7.690/17.994 ms and 7.696/16.682 ms. Values remain within the broad
JSDOM baseline range, so no rendering speedup is claimed. All 25 focused
Integrations tests pass, including review copy and the opt-in wanted-sync
action. The Web production build, targeted ESLint, and diff checks pass.

This is internal-only refactoring, so no release-note fragment is required.
Federation Diagnostics and the final composer cleanup remain.


## RF-058 Integrations Federation Diagnostics Panel Extraction (Batch 327, 2026-09-26)

Moved Federation Diagnostics API loading, local async state, error handling, and
read-only posture UI into `FederationDiagnosticsPanel.jsx`. The Integrations
composition root is now 61 lines, down from 3,915 lines at the start of the
Integrations extraction. All Integrations workflows now have local owner
modules, and MediaCore's composition root remains 369 lines.

The retained JSDOM profile warms and measures 20 diagnostics-load commits. Two
baselines measured 0.924/1.538 ms and 0.896/1.367 ms median/p95; two after runs
measured 0.751/1.316 ms and 0.808/1.606 ms. Median observations were lower
after extraction while p95 varied, so these are JSDOM observations and not a
deployed-browser performance claim.

The final focused Integrations suite passes all 26 tests. The notification apply
regression now awaits its asynchronous success message after observing the API
call, removing an assertion race. Integrations ESLint, the Web production
build, and diff checks pass. RF-058 ownership extraction is complete and
verified locally. This is internal-only refactoring, so no release-note
fragment is required.

## RF-024 Bounded Dispatcher Group 1 Ownership Split (Batch 328, 2026-09-26)

Split the 1,935-line `route_dispatch_group_1.rs` into a 13-line ordered
composer and four route-owner files: discovery/backfill/docs/batch (551 lines),
configuration/telemetry (471), events/webhooks/options (557), and shares/files
(404). The 1,864 match-arm lines were checked byte-for-byte against the original
and remain in their original dispatch order. Context fields are borrowed by the
new handlers; the batch owner keeps the original single `state_arc` clone.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, plan freshness, docs freshness, and diff checks
pass. No tests were added or run. This is internal-only work and adds no
release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Bounded Dispatcher Group 3 Ownership Split (Batch 329, 2026-09-26)

Split the 1,431-line `route_dispatch_group_3.rs` into a 13-line ordered
composer and four route-owner files: rooms/users/browse (642 lines), admin
controls (207), room membership (372), and options/diagnostics (257). The
1,414 match-arm lines were checked byte-for-byte against the original and
remain in their original order. The handlers read the shared dispatch context
without cloning it.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, and diff checks pass. No tests were added or run.
This is internal-only work and adds no release-note fragment. RF-024 and RF-002
remain in progress.

## RF-024 Bounded Dispatcher Group 0 Ownership Split (Batch 330, 2026-09-26)

Split the 1,192-line `route_dispatch_group_0.rs` into a 7-line ordered
composer and two route-owner files: controller identity/application/session
(519 lines) and mesh/HashDb/VirtualSoulfind (688). The 1,175 match-arm lines
were checked byte-for-byte against the original and remain in their original
order. The handlers borrow fields from the shared dispatch context.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, and diff checks pass. No tests were added or run.
This is internal-only work and adds no release-note fragment. RF-024 and RF-002
remain in progress.

## RF-024 Bounded Dispatcher Group 6 Telemetry Ownership Split (Batch 331, 2026-09-26)

Split the 1,342-line telemetry handler into a 15-line ordered composer and
four route-owner files: metrics/jobs (121 lines), Pod routes (729),
federation/security (358), and multisource/graph (179). The 1,323 match-arm
lines were checked byte-for-byte against the original and remain in their
original order. All handlers borrow the shared dispatch context.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, and diff checks pass. No tests were added or run.
This is internal-only work and adds no release-note fragment. RF-024 and RF-002
remain in progress.

## RF-024 MusicBrainz Target and Coverage Helper Ownership (Batch 332, 2026-09-26)

Move the contiguous 600-line MusicBrainz target and discography-coverage
helper block out of the 1,722-line `route_dispatch.rs` into
`musicbrainz_targets.rs` (599 lines). `route_dispatch.rs` now has 1,123 lines
and includes the new source in the same module scope, preserving all caller
paths and visibility. Replacing the include with the extracted block
reconstructs the pre-split router byte-for-byte.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, and diff checks pass. No tests were added or run.
This is internal-only work and adds no release-note fragment. RF-024 and RF-002
remain in progress.

## RF-024 Bounded Dispatcher Group 6 Admin and Relay Ownership Split (Batch 333, 2026-09-26)

Split the 1,041-line `route_dispatch_group_6_admin_discovery.rs` into a
9-line ordered composer and two route-owner files: admin/config/recommendation
(292 lines) and relay-controller (775). The 1,007 route-arm lines were moved
in source order. The trailing `source_discovery_state_matches` helper was
restored from the pre-split snapshot after the first compile caught its
omission; its tokens were checked against the snapshot. Extracted source was
formatted with rustfmt.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, and diff checks pass. No tests were added or run.
This is internal-only work and adds no release-note fragment. RF-024 and RF-002
remain in progress.

## RF-024 Route Dispatch Request-Flow Ownership (Batch 334, 2026-09-26)

Move the 692-line `route_http_request_inner` / batch-dispatch block and
`RouteDispatchContext` from `route_dispatch.rs` into
`route_dispatch_request_flow.rs` (691 lines). `route_dispatch.rs` is now 435
lines. Replacing the include with the extracted block reconstructs the
pre-split router byte-for-byte; function names and module visibility are
unchanged.

`cargo check --locked -p slskr`, the combined `full-controller-tests` and
`legacy-route-dispatch` check, and scoped Clippy with
`-D clippy::await_holding_lock` pass. The changed-file Rust formatter,
runtime-boundary hardening, and diff checks pass. No tests were added or run.
This is internal-only work and adds no release-note fragment. RF-024 and RF-002
remain in progress.

## RF-024 PlayerBar Visual Tile Ownership Split (Batch 335, 2026-09-26)

Move `PlayerVisualTile`, `PlayerAnalyzerTile`, and the persisted visualizer
tile-engine selector from the 3,478-line `PlayerBar.jsx` into
`PlayerVisualTiles.jsx` (255 lines). `PlayerBar.jsx` is now 3,230 lines; the
existing call sites and state updates remain in place.

All 24 focused `PlayerBar.test.jsx` tests pass. Targeted ESLint and the Web
production build pass. This structural change makes no performance claim and
is internal-only, with no release-note fragment. RF-024 and RF-002 remain in
progress.

## RF-024 PlayerLauncher Ownership Split (Batch 336, 2026-09-26)

Move the 582-line `PlayerLauncher` component and its bounded browser helpers
from `PlayerBar.jsx` into `PlayerLauncher.jsx`. The component keeps its
collection/file browser state and API lifecycle locally; playback remains
controlled through the existing `onPlayItem` prop. The body was verified
verbatim against the pre-edit snapshot. `PlayerBar.jsx` is now 2,614 lines.

Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and
the Web production build passes (535 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 PlayerStatsModal Ownership Split (Batch 337, 2026-09-26)

Move the 483-line `PlayerStatsModal` from `PlayerBar.jsx` into
`PlayerStatsModal.jsx`, keeping its local-history state, guarded async
actions, import/export flow, and UI together. Its existing `open`,
`onClose`, and `onOpenSearch` contract is unchanged; the component body was
verified verbatim against the pre-edit snapshot. `PlayerBar.jsx` is now
2,121 lines.

Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and
the Web production build passes (536 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 PlayerRadioModal Ownership Split (Batch 338, 2026-09-26)

Move the 226-line `PlayerRadioModal` into `PlayerRadioModal.jsx`, including
its guarded search/Wishlist actions and radio-plan copy UI. Its `current`,
`open`, `onClose`, and `onOpenSearch` contract is unchanged, and the component
body was verified verbatim against the pre-edit snapshot. The route-building
helper remains in `PlayerBar.jsx`; that file is now 1,890 lines.

Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and
the Web production build passes (537 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 PlayerQueueModal Ownership Split (Batch 339, 2026-09-26)

Move the 320-line `PlayerQueueModal` and its queue-label helper into
`PlayerQueueModal.jsx`, keeping queue controls, recent history, and similar
track handoff actions together. Its existing parent callbacks and queue props
are unchanged; the extracted helper/component block was verified verbatim
against the pre-edit snapshot. `PlayerBar.jsx` is now 1,564 lines.

Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and
the Web production build passes (538 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 PlayerDiscoveryShelfModal Ownership Split (Batch 340, 2026-09-26)

Move the 231-line `PlayerDiscoveryShelfModal` to
`PlayerDiscoveryShelfModal.jsx`, grouping shelf state, policy preview,
clipboard reporting, and local review actions together. Its `open` and
`onClose` contract is unchanged; the component body was verified verbatim
against the pre-edit snapshot. `PlayerBar.jsx` is now 1,323 lines.

Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and
the Web production build passes (539 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Visualizer Preset Library Ownership Split (Batch 341, 2026-09-26)

Move the 689-line preset, persistence, validation, and import helper block
from `Visualizer.jsx` into `visualizerPresetLibrary.js`. The helper block
was checked intact against the pre-edit snapshot; only helpers used by the
renderer are exported. `Visualizer.jsx` is now 1,984 lines, with its render
and engine lifecycle left in place.

Targeted ESLint passes, all 24 focused `Visualizer.test.jsx` tests pass, and
the Web production build passes (540 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Network Endpoint Notice Ownership Split (Batch 342, 2026-09-26)

Move network endpoint snapshot normalization/storage and the VPN ingress-port
migration notice into `NetworkEndpointNotice.jsx`. The helper, limits, and
notice blocks match the pre-edit source snapshot exactly. `App.jsx` imports
the moved notice and callbacks; its public `getStoredNetworkEndpointSnapshot`
named export remains available. `App.jsx` is now 2,034 lines.

Targeted ESLint passes, all 18 focused `App.test.jsx` tests pass, and the Web
production build passes (541 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 App Connection Control Ownership Split (Batch 343, 2026-09-26)

Move the 140-line `ModeSpecificConnectButton` into
`ModeSpecificConnectButton.jsx`, leaving the app class responsible for
supplying the existing eight props and callbacks. The component body was
verified verbatim against the pre-edit snapshot. `App.jsx` is now 1,889
lines.

Targeted ESLint passes, all 18 focused `App.test.jsx` tests pass, and the Web
production build passes (542 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 SongID Panel Helper Ownership Split (Batch 344, 2026-09-26)

Move the 190-line SongID display, scoring, and deduplication helper block
into `songIdPanelHelpers.js`, exporting the helpers shared by the run list
and result presentation. The block matches the pre-edit snapshot exactly.
Targeted ESLint passes. This is internal-only, makes no performance claim,
and adds no release-note fragment; RF-024 and RF-002 remain in progress.

## RF-024 SongID Analysis Results Ownership Split (Batch 345, 2026-09-26)

Move the 133-line result render/data helper block and 1,085-line analysis
column into `SongIDAnalysisResults.jsx`. It receives 19 explicit props;
async SongID actions, graph lifecycle state, and request handling remain in
`SongIDPanel.jsx`, now 646 lines. Both moved blocks were verified intact
against the pre-edit snapshot.

Targeted ESLint passes and the Web production build passes (544 modules).
No direct `SongIDPanel` test exists in `web/src`; no new tests were added.
This structural change makes no performance claim and is internal-only, with
no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Messaging Workspace and Pod Session Ownership Split (Batch 346, 2026-09-26)

Move the 122-line persisted workspace/target helper block to
`messagingWorkspaceState.js` and the 309-line Pod channel session to
`PodChannelSession.jsx`. The Pod component keeps refresh coalescing, abort,
cursor, send, and local error state together. The workspace retains its
existing panel/hydration behavior. Both moved blocks were verified against
the pre-edit snapshot. `Messaging.jsx` is now 1,214 lines.

Targeted ESLint passes, all 18 focused `Messaging.test.jsx` tests pass, and
the Web production build passes (546 modules). This structural change makes
no performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Admin Policies Form Ownership Split (Batch 347, 2026-09-26)

Move the 1,019-line Admin Policies render and its `boolLabel` renderer into
`AdminPoliciesForm.jsx`. The child receives eight explicit values/callbacks;
YAML parsing, save/reset, request lifecycle, and mounted-state handling stay
in `index.jsx`, now 573 lines. The render and helper were verified intact
against the pre-edit snapshot.

Targeted ESLint passes, all 8 focused `index.test.jsx` tests pass, and the
Web production build passes (547 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Adversarial Settings View Ownership Split (Batch 348, 2026-09-26)

Move the tab options, four settings panes, and page rendering into
`AdversarialSettingsView.jsx` (1,054 lines). The 606-line
`AdversarialSettings.jsx` retains load/save/reset, refresh, and connectivity
request lifecycles. The view receives 17 state values and 11 actions. The
render block was checked against the pre-edit snapshot; transport and Tor test
callbacks were lifted to named parent actions with their request IDs, mounted
guards, status/error updates, and follow-up refresh behavior preserved.

Targeted ESLint passes, all 3 focused `AdversarialSettings.test.jsx` tests pass,
and the Web production build passes (548 modules). This structural change
makes no performance claim and is internal-only, with no release-note
fragment. RF-024 and RF-002 remain in progress.

## RF-024 Network Details View Ownership Split (Batch 349, 2026-09-26)

Move the 473-line peer, mesh-security, hash-database, backfill-scheduler, and
active-swarm render block plus `formatTimeAgo` into `NetworkDetails.jsx`. The
child receives 15 data values and two callbacks. `System/Network/index.jsx` is
now 827 lines and retains polling, request/result handling, error state, sync,
and backfill lifecycles. `formatBytes` stays parent-owned because an earlier
parent panel also uses it; the value is passed to the extracted view. The moved
render block matches the pre-edit snapshot exactly.

Targeted ESLint passes, all 14 focused `index.test.jsx` tests pass, and the Web
production build passes (549 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Search Detail Controls Ownership Split (Batch 350, 2026-09-26)

Move the 323-line result-filter and album-candidate presentation block plus
`sortDropdownOptions` into `SearchResultControls.jsx`. It receives 15 state
values and 15 callbacks in separate `state` and `actions` objects. Search
hydration, ranking/deduplication, persistence, and handler ownership remain in
`SearchDetail.jsx`, now 850 lines. The moved block matches the pre-edit
snapshot exactly.

Targeted ESLint passes, all 7 focused `SearchDetail.test.jsx` tests pass, and
the Web production build passes (550 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Search Response Preview Modal Ownership Split (Batch 351, 2026-09-26)

Move the 63-line download-action preview modal render into
`DownloadActionPreviewModal.jsx`. `Response.jsx` retains preview-open state,
copy behavior, and request/action handling, and is now 999 lines. The modal UI
matches the pre-edit snapshot apart from receiving explicit `open`, `onClose`,
and `onCopy` values.

Targeted ESLint passes, the 7 focused `SearchDetail.test.jsx` tests pass, and
the Web production build passes (551 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Visualizer Overlay Controls Ownership Split (Batch 352, 2026-09-26)

Move the 699-line overlay-control JSX block into
`VisualizerOverlayControls.jsx`. The view receives 27 state values, 34 action
callbacks, and two input refs, grouped by ownership. Audio graph/render-loop
lifecycle, engine operations, and persisted state remain in `Visualizer.jsx`,
now 1,356 lines. The extracted block matches the pre-edit snapshot exactly.

Targeted ESLint passes, all 24 focused `Visualizer.test.jsx` tests pass, and the
Web production build passes (552 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Wishlist Component Ownership Split (Batch 353, 2026-09-26)

Move `WishlistItemRow` (148 lines), `WishlistModal` (234 lines), and
`CsvImportModal` (163 lines) into sibling modules, and share their mounted-ref
hook through `wishlistHooks.js`. `Wishlist.jsx` now owns the 599-line page,
list loading, and bulk operations. Row search state, editor save/restore state,
and CSV import state stay with their respective extracted components. Each
component and the hook match the pre-edit snapshot exactly.

Targeted ESLint passes, all 8 focused `Wishlist.test.jsx` tests pass, and the
Web production build passes (556 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Searches List-Page View Ownership Split (Batch 354, 2026-09-26)

Move the 296-line legacy/modern list-page render and 71-line persisted
`CollapsibleSection` into `SearchesListView.jsx`. The view receives explicit
state, action, and input-ref values. `Searches.jsx` is now 733 lines and keeps
route-detail selection, hub connection/subscription, hydration, and
create/stop/remove request lifecycles. The moved render and collapsible section
match the pre-edit snapshot exactly.

Targeted ESLint passes, all 13 focused `Searches.test.jsx` tests pass, and the
Web production build passes (557 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Collections Modal Ownership Split (Batch 355, 2026-09-26)

Move the create, share, and add-item modal render blocks into
`CreateCollectionModal.jsx`, `CollectionShareModal.jsx`, and
`CollectionAddItemModal.jsx`. The 925-line `Collections.jsx` class retains
form state, request IDs, API calls, modal-close resets, and search actions. The
moved UI keeps the same fields and callbacks; their render blocks were checked
against the pre-edit snapshot with inline parent handlers lifted into props.

Targeted ESLint passes, all 5 focused `Collections.test.jsx` tests pass, and
the Web production build passes (560 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 Messaging Sidebar and Batch Modal Ownership Split (Batch 356, 2026-09-26)

Move the 237-line saved-chat/room/pod sidebar into `MessagingSidebar.jsx` and
the 46-line batch private-message modal into `BatchPrivateMessageModal.jsx`.
The 964-line `Messaging.jsx` retains workspace hydration, hub subscriptions,
action request IDs, state, and session rendering. The moved JSX blocks match
the pre-edit snapshot exactly.

Targeted ESLint passes, all 18 focused `Messaging.test.jsx` tests pass, and the
Web production build passes (562 modules). This structural change makes no
performance claim and is internal-only, with no release-note fragment.
RF-024 and RF-002 remain in progress.

## RF-024 PlayerBar Presentation Ownership Split (Batch 357, 2026-09-26)

Move the Player tool button, now-playing rating controls, and integrations modal into `PlayerToolButton.jsx`, `PlayerRatingControls.jsx`, and `PlayerIntegrationsModal.jsx`. `PlayerBar.jsx` decreases from 1,323 to 1,111 lines. The parent retains playback, ListenBrainz token state and persistence, and external-visualizer request lifecycles; the modal receives state and action callbacks.

Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and the Web production build passes (565 modules). This is internal-only structural work, with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Compatibility Dashboard Chart Ownership Split (Batch 358, 2026-09-26)

Move dashboard chart-data transforms, series configuration, aggregation helpers, and SVG graph rendering into `CompatibilityGraph.jsx`. `CompatibilityDashboard.jsx` decreases from 1,053 to 870 lines. The parent retains report requests, dashboard state, and tab behavior; the chart module exports shared normalization/aggregation helpers used by the parent.

Targeted ESLint passes, all 4 focused `CompatibilityDashboard.test.jsx` tests pass, and the Web production build passes (566 modules). This is internal-only structural work, with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 App Header Menu Ownership Split (Batch 359, 2026-09-26)

Move the right-side connection/theme/update/logout menu into `AppHeaderMenu.jsx`. `App.jsx` decreases from 1,889 to 1,774 lines. The parent retains authentication, routing, hub, and theme lifecycle ownership; the extracted menu receives state and action callbacks.

Targeted ESLint passes, all 18 focused `App.test.jsx` tests pass, and the Web production build passes (567 modules). This is internal-only structural work, with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 App Primary Navigation Ownership Split (Batch 360, 2026-09-27)

Move the 293-line primary route-navigation view and its notification icon into `AppNavigationPrimary.jsx`. `App.jsx` decreases from 1,774 to 1,476 lines. The App parent retains authentication, hub, route, and navigation-activity lifecycle ownership; the extracted navigation receives the already-derived profile and activity state.

Targeted ESLint passes, all 18 focused `App.test.jsx` tests pass, and the Web production build passes (568 modules). This is internal-only structural work, with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 App Route-Table Ownership Split (Batch 361, 2026-09-27)

Move lazy page imports, agent/native route tables, and route-miss diagnostics into `AppRouteTable.jsx`. `App.jsx` decreases from 1,476 to 1,104 lines. Authentication remains in the App-owned `withTokenCheck` callback, passed to the route table; the provider and suspense boundary remain in the shell.

Targeted ESLint passes, all 18 focused `App.test.jsx` tests pass, and the Web production build passes (569 modules). The first validation caught the route table’s omitted `semanticTheme` prop; it is now passed explicitly. This is internal-only structural work, with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-002 Transfer Failure Rollback and Enabled Sync Injection (Batches 362–364, 2026-09-27)

Batch 362 adds direct enabled-Lidarr-sync failure injection and verifies that an SQLite error restores the page's in-memory wishlist changes. Batch 363 introduces row-wise transfer mutation snapshots so failed writes restore or remove only rows that still match the failed mutation. Batch 364 applies that rollback to transfer cancellation, retry/start, progress, completion, queue creation, maintenance cleanup, staged auto-download rollback, and recovery paths in both the split and retained dispatchers. Staged batch metadata is removed only when every staged row was rolled back.

The focused rollback regression covers a failed row update followed by a newer update to the same row, a concurrent allocation, failed deletion restoration, preservation of a concurrently added row, and conditional removal of stale or unchanged staged rows. The enabled Lidarr sync regression covers the direct persistence failure path.

`cargo test --locked -p slskr --lib transfer_mutation_rollback_preserves_newer_rows`, `cargo test --locked -p slskr --lib --features full-controller-tests auto_retry_rollback_preserves_concurrent_transfer_allocations`, and `cargo test --locked -p slskr --lib --features full-controller-tests lidarr_wanted_sync_rolls_back_wishlist_when_persistence_fails` each pass. `cargo check --locked -p slskr --lib --features legacy-route-dispatch` passes with dead-code warnings from the opt-in dispatcher glue. `scripts/check-rust-format.sh` passes for 234 changed workspace files and skips the documented historical large sources. RF-002 remains in progress while the other evidence recorded in the inventory is outstanding.

## RF-024 Navigation Activity Lifecycle Ownership Split (Batch 365, 2026-09-27)

Move bounded room-activity storage and chat/room unread-activity polling, route suppression, hidden-tab pause/resume, and timer cleanup into `AppNavigationActivity.js`. `App.jsx` decreases from 1,104 to 938 lines; it retains the visible `navActivity` state and passes current route/authentication callbacks into the lifecycle owner.

Targeted ESLint passes, all 18 focused `App.test.jsx` tests pass, including room-activity baselining and hidden-tab refresh behavior, and the Web production build passes (570 modules). The Web ownership inventory and executable guard now assign route definitions to `AppRouteTable.jsx` and polling ownership to `AppNavigationActivity.js`; the ownership guard, shell-script hygiene, docs freshness, and plan freshness checks pass. This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Visualizer Preset and Overlay Ownership Split (Batch 366, 2026-09-27)

Move bounded preset persistence, validation, import helpers, and texture asset handling into `visualizerPresetLibrary.js`, and move the complete engine/preset control overlay into `VisualizerOverlayControls.jsx`. `Visualizer.jsx` decreases from 2,617 to 1,356 lines while retaining engine initialization, rendering, and action ownership.

Targeted ESLint passes, all 24 focused `Visualizer.test.jsx` tests pass, and the Web production build passes (570 modules). This structural change makes no performance claim and is internal-only, with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 SongID Result Evidence Ownership Split (Batch 367, 2026-09-27)

Move identity and synthetic summaries, scorecard and provenance, artifact and corpus findings, segment decomposition, ranked acquisition-option details, clip/transcript/OCR/comment/chapter evidence, and forensic matrix rendering to `SongIDEvidenceDetails.jsx`. `SongIDAnalysisResults.jsx` decreases from 1,272 to 533 lines; it retains the result shell, best-candidate actions, minimap, mix clusters, and track/album/artist candidate lists. Run data and action callbacks are passed through to the extracted view.

A pre-edit snapshot and exact reconstruction audit confirm that all moved helpers, derived values, and JSX are preserved; only module imports and parent props changed. Targeted ESLint passes, the focused `SongIDEvidenceDetails.test.jsx` regression passes, and the Web production build passes (571 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Visualizer Engine Lifecycle Ownership Split (Batch 368, 2026-09-27)

Move the frame-capped render loop and asynchronous visualizer-engine creation, failure fallback, resize observation, animation scheduling, and disposal lifecycle into `useVisualizerEngineLifecycle.js`. `Visualizer.jsx` decreases from 1,356 to 1,210 lines; it retains the engine refs and passes its current audio, canvas, and state callbacks into the lifecycle owner.

A pre-edit snapshot and exact reconstruction audit confirm that the render callback and lifecycle effect are preserved. Targeted ESLint passes, all 24 focused `Visualizer.test.jsx` tests pass, and the Web production build passes (572 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Visualizer Preset File Operations Ownership Split (Batch 369, 2026-09-27)

Move bounded preset/fragment file import, texture asset association, validation feedback, and preset/fragment export callbacks into `useVisualizerPresetFileOperations.js`. `Visualizer.jsx` decreases from 1,210 to 1,032 lines and receives the operations through one focused hook; the existing request guard, persisted preset state, error reporting, and canvas refresh flow are retained.

A pre-edit snapshot and exact reconstruction audit confirm the callbacks and imports were moved without changing the parent body. Targeted ESLint passes, all 24 focused `Visualizer.test.jsx` tests pass, and the Web production build passes (573 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Visualizer Preset Editor Actions Ownership Split (Batch 370, 2026-09-27)

Move native shape/wave removal, parameter edits, and preset randomization into `useVisualizerPresetEditorActions.js`. `Visualizer.jsx` decreases from 1,032 to 883 lines. The hook receives current edit values and fragment indices along with the existing operation guard, state setters, and refresh callbacks.

A pre-edit snapshot and exact reconstruction audit confirm the editor callbacks were moved intact. Targeted ESLint passes, all 24 focused `Visualizer.test.jsx` tests pass, and the Web production build passes (574 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 PlayerBar External Visualizer Lifecycle Ownership Split (Batch 371, 2026-09-27)

Move external-visualizer status/launch state, request-ID refs, refresh and launch callbacks, and modal-open request invalidation into `useExternalVisualizerControl.js`. `PlayerBar.jsx` decreases from 1,111 to 1,017 lines. The hook keeps the stale-response checks, duplicate-launch guard, loading/message state, and existing error text.

A pre-edit snapshot and exact reconstruction audit confirm that the external status/launch code was moved intact. Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and the Web production build passes (575 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 PlayerBar Stream Source Lifecycle Ownership Split (Batch 372, 2026-09-27)

Move source state, stream-ticket request IDs, direct/ticketed URL selection, and stale-request cleanup into `usePlayerStreamSource.js`. `PlayerBar.jsx` decreases from 1,017 to 971 lines and consumes the resolved stream source from the hook.

A pre-edit snapshot and exact reconstruction audit confirm that the streaming effect and fallback behavior were moved intact. Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and the Web production build passes (576 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.
## RF-002 Lidarr Rejection Helper Serialization (Batch 373, 2026-09-27)

Reinspection found that `apply_lidarr_rejection_policy` updated Wishlist and Search and called their paired SQLite transaction without acquiring either persistence turn, despite the earlier inventory saying the helper used Wishlist-then-Search ordering. The helper now acquires those turns in the established order before resolving or changing the wishlist item, keeps them through paired persistence and conditional rollback, then releases them before search event publication and rejected-file cleanup.

Added `lidarr_rejection_waits_for_search_turn_before_mutating_wishlist` to prove the helper leaves Wishlist unchanged while the Search turn is held and completes after release. `cargo test --locked -p slskr --lib lidarr_rejection_waits_for_search_turn_before_mutating_wishlist` and `cargo test --locked -p slskr --lib --features full-controller-tests lidarr_rejection_policy_blacklists_wishlist_origin_and_deletes_files` pass. Scoped full-controller/legacy Clippy with `-D clippy::await_holding_lock`, the repository formatter for 234 changed workspace files, and direct rustfmt checking of the extracted `transfer_completion.rs` pass. The user-facing consistency fix is recorded in `release-notes/20260927-wishlist-rejection-persistence-order.md`; RF-002 remains in progress while the remaining helper-call paths are audited.

## RF-024 PlayerBar Picture-in-Picture Lifecycle Ownership Split (Batch 374, 2026-09-27)

Move document-Picture-in-Picture window creation, animated spectrum-canvas rendering, stale-request checks, and unmount cleanup into `usePlayerPictureInPicture.js`. `PlayerBar.jsx` decreases from 970 to 897 lines. The hook is called where the former cleanup effect was registered, preserving the cleanup order alongside audio and keyboard lifecycles.

A pre-edit snapshot confirms the PiP callback and cleanup match the original source exactly. Targeted ESLint passes, all 24 focused `PlayerBar.test.jsx` tests pass, and the Web production build passes (577 modules). This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Config Schema and Test Ownership Split (Batch 375, 2026-09-27)

Move the file-backed schema and conversion block into `config_file.rs` and the 79 inline config tests into `config_tests.rs`. `config.rs` decreases from 15,249 to 8,227 lines; the new files are 3,231 and 3,797 lines. The config parent re-exports `FileConfig` and keeps other schema types visible only within the config subtree. A saved-source comparison confirms the schema block matches its original after normalizing the new `pub(super)` visibility and rustfmt; the test module also matches its rustfmt-normalized original.

All 79 focused config tests pass. The full-controller and legacy feature combination compiles. Direct rustfmt checks of both extracted files, the repository changed-file formatter check for 237 files, and `git diff --check` pass. This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Integration Config Ownership Split (Batch 376, 2026-09-27)

Move the contiguous integration settings and conversion block into `config_integrations.rs`. `config.rs` decreases from 8,227 to 6,664 lines; the extracted module is 1,568 lines and preserves its public settings API through a re-export. A saved-source comparison after rustfmt confirms the implementation body was moved without changes.

All 79 focused config tests pass, the full-controller and legacy feature combination compiles, direct rustfmt checks of the extracted config modules pass, the repository changed-file formatter check passes for 238 files, and `git diff --check` passes. This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Controller YAML Ownership Split (Batch 377, 2026-09-27)

Move the controller YAML environment adapter, frozen-name mapping, alias normalization, and bounded YAML-shape validation into `config_yaml.rs`. `config.rs` decreases from 6,664 to 5,486 lines; the extracted module is 1,183 lines. Only the adapter fields and functions called by the config parent are exposed within that subtree. A saved-source comparison after visibility normalization and rustfmt confirms the implementation body was moved without changes.

All 79 focused config tests pass, the full-controller and legacy feature combination compiles, direct rustfmt checks of the extracted config modules pass, the repository changed-file formatter check passes for 239 files, and `git diff --check` passes. This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Core Config Settings Ownership Split (Batch 378, 2026-09-27)

Move the daemon, retention, core workflow, Web, Soulseek, and transfer settings definitions and conversions into `config_settings.rs`. `config.rs` decreases from 5,486 to 4,703 lines; the extracted module is 788 lines. Ten conversion methods used by the config parent or file schema are visible only within the config subtree. A saved-source comparison after visibility normalization and rustfmt confirms the conversion bodies were moved unchanged.

All 79 focused config tests pass, the full-controller and legacy feature combination compiles, direct rustfmt checks of the extracted config modules pass, the repository changed-file formatter check passes for 240 files, and `git diff --check` passes. This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-024 Persistence Test Ownership Split (Batch 379, 2026-09-27)

Move the 1,533-line inline persistence test module into `persistence_tests.rs` while retaining the `persistence::tests` path. A saved-source comparison after dedent and rustfmt confirms the tests were moved unchanged. `persistence.rs` decreases from 7,664 to 6,131 lines. All 27 focused persistence tests pass. This is internal-only structural work with no release-note fragment.

## RF-024 DatabaseManager Domain Ownership Split (Batch 380, 2026-09-27)

Move the contiguous OAuth, search, transfer batch, HashDb, transfer, activity, people, wishlist, collaboration, library/runtime, webhook, and distributed persistence methods into twelve domain modules. `persistence.rs` decreases to 2,313 lines, and the largest method module is 562 lines. A saved-source reconstruction after rustfmt confirms the method bodies, documentation, and section headers match the original. The `DatabaseManager` API and SQLite transaction boundaries are retained.

All 27 focused persistence tests pass, the full-controller and legacy feature combination compiles, the repository changed-file formatter check passes for 253 files, direct rustfmt checks of the extracted files pass, the Rust module hygiene check passes, and `git diff --check` passes. This is internal-only structural work with no release-note fragment. RF-024 and RF-002 remain in progress.

## RF-036 Server Code Directional Dispatch Inventory (Batch 381, 2026-09-27)

Add an explicit two-direction test inventory for all 103 `ServerCode` variants. Every code is classified as typed in both directions, typed in one direction, or opaque in both directions. The test compares those declared categories against actual decoder dispatch using empty frames, so a new or changed code cannot silently fall through to `Unknown` without updating the inventory. The existing exhaustive `ServerMessage` encoder match remains compiler checked.

All 25 server protocol tests pass, including the new inventory test. Direct rustfmt, the repository changed-file formatter check for 254 files, and `git diff --check` pass. This is internal-only contract proof with no release-note fragment; RF-036 is verified locally.

## RF-032 Historical Documentation Path Audit (Batch 382, 2026-09-27)

Audit the local links and referenced shell scripts in `docs/full-network-test-plan.md` and `REMEDIATION.md`. No current links or invoked script paths are missing. Five absent network script names are explicitly proposed work in the network plan; the absent `scripts/run-ci.sh` is likewise a historical proposal in REMEDIATION. Correct the network plan's server-code count from 102 to the validated 103-code inventory.

The docs freshness gate and local path audit pass. The user-facing documentation correction has a validated fragment at `release-notes/20260927-network-plan-protocol-inventory.md`. RF-032 is verified locally; fresh live certification remains tracked separately.

## RF-024 CLI Smoke and Soak Ownership Split (Batch 383, 2026-09-27)

Move the inline CLI tests to `cli_tests.rs` and the local/fixture smoke plus live soak implementations to `cli_smoke_soak.rs`. `cli.rs` decreases from 5,397 to 2,346 lines; the new smoke/soak module is 2,914 lines and the formatted test module is 145 lines. The CLI parent retains command parsing and direct probe entry points. Thirty-seven functions used by the parent or its tests are visible only within the CLI subtree; internal helpers remain private. Saved-source comparisons after visibility normalization and rustfmt confirm the implementations and tests were moved unchanged.

All 9 focused CLI tests pass without warnings. The full-controller and legacy feature combination compiles, the repository changed-file formatter check passes for 257 files, direct rustfmt checks of the extracted modules pass, the Rust module hygiene check passes, and `git diff --check` passes. This is internal-only structural work with no release-note fragment. RF-024 remains in progress.

## RF-024 Structural Closure and Missing-Route Dispatch (Batch 384, 2026-09-27)

The final size scan finds no non-test Rust source above 5,000 lines in `crates/slskr/src`, no Web JS/JSX source above 2,000 lines, and no dashboard source above 300 lines. The largest Rust source is `config.rs` at 4,703 lines; `lib.rs` is 2,305, `persistence.rs` is 2,313, and `cli.rs` is 2,346. The Web ownership inventory confirms active route/context boundaries. The extracted Rust bodies have saved-source comparisons, focused tests, and feature compiles in Batches 375-383; earlier Web splits have their focused tests and production builds in the corresponding addenda.

The full daemon library run found a deterministic split-router regression: an unknown route left the internal `ROUTE_NOT_HANDLED` marker as an error, which batch execution reported as HTTP 500. The router now converts that marker to HTTP 404 after the final route group. The existing batch regression also checks a direct missing route. Its focused test and the full 640-test daemon library suite pass. The full-controller/legacy feature compile, Web ownership inventory, Rust module hygiene, repository formatter check for 257 changed files, and runtime boundary hardening gate pass. The latter gate now reads config regression names from the extracted `config_tests.rs`. The routing behavior fix has a validated fragment at `release-notes/20260927-missing-api-routes.md`. RF-024 is verified locally.

## RF-002 Production Persistence Call-Site Audit (Batch 385, 2026-09-27)

Scan production Rust sources for calls to the 92 mutating `DatabaseManager` methods and all checked persistence helpers, then inspect paths without a nearby persistence turn. Helper bodies delegate turn ownership to callers. Startup share-index, stale-grant, token, and HashDb writes occur before app state publication; the two wishlist scheduler saves run in the owning session loop; transfer projection writes use monotonic revision checks; PodCore's `vacuum` call is a different store. Lidarr wanted sync holds its Wishlist turn through the page write despite the call lying farther than the scan window from acquisition. The paired Wishlist/Search transaction has exactly three production callers, and each acquires Wishlist then Search before mutation and commit.

The full 640-test daemon library suite passes, including the direct/batch route regression; the focused Wishlist contention and Lidarr failure tests recorded in Batches 362-373 pass. The full-controller/legacy feature compile and scoped Clippy with `-D clippy::await_holding_lock` pass. This closes the local write-ordering and held-guard audit for RF-002; the existing release-note fragments cover its user-facing fixes.

## Live Parity Artifact Scope (2026-09-27)

The manual/scheduled Live Parity workflow placed its temporary `live.env` in
`target/live-interop/` and previously uploaded that entire directory. Its upload
now selects only `*.tsv`, and the workflow policy check rejects the broad glob.
All 12 accessible historical `credentialed-live-interop-*` artifacts were
inspected through the GitHub API: each contains only
`credentialed-live-interop.tsv` with status `skipped`. No accessible artifact
contains `live.env`; the credentialed hosted run remains unproven because the
GitHub secret has not been configured. The narrowed upload has a validated
operator release note.

## RF-024 Tracked-Source Inventory Correction (2026-09-27)

The earlier closure scan covered non-test daemon Rust and Web/dashboard JavaScript,
not every tracked source file. A complete `git ls-files` line inventory found
`crates/slskr/src/controller_tests.rs` at 149,731 lines,
`crates/slskr-web/src/lib.rs` at 22,436 lines,
`scripts/check-controller-options-differential.sh` at 9,481 lines,
`crates/slskr/src/focused_controller_tests.rs` at 7,209 lines, and
`scripts/audit-parity-manifest.py` at 6,063 lines. RF-024 is reopened to split
these boundaries by reviewable ownership while preserving the existing code
and validating the affected feature and policy gates. The prior daemon and
JavaScript closure evidence remains valid for its stated scope.

The 2026-09-27 manually triggered Live Parity workflow run
[36339374480](https://github.com/snapetech/slskr/actions/runs/36339374480)
completed successfully at `5d497f55`. Its Rust UI and slskd API parity job
passed and retained a 298-file artifact. The credentialed live-interoperability
job produced a skipped TSV because the hosted secret is not configured; this
does not close RF-072 or the scheduled-artifact requirement in RF-073.

The first broader RF-024 pass split `slskr-web/src/lib.rs` into seven bounded
files (largest 5,822 lines), preserving an exact 881,329-byte reconstruction
before formatting. Host and WASM checks, all 86 crate tests, and the changed-file
formatter pass. The tests now inspect the React route, navigation, and header
owners after their earlier split. `controller_tests.rs` was then split into 29
test segments (largest 5,855 lines), with an exact 5,310,963-byte
reconstruction before formatting. Fixture paths were adjusted for the new
directory. The full-controller and legacy-route test target compiles, and six
source-owner policy guards, module hygiene, shell hygiene, and formatting pass.
The segments still share one Rust module through `include!`, so test ownership
is not yet isolated; RF-024 remains in progress.

`focused_controller_tests.rs` is now three included segments of at most 2,543
lines. The pre-format extraction reconstructed all 252,983 original bytes.
Its relative fixture references were adjusted, the runtime boundary guard and
formatter pass, and the default daemon library suite passes all 642 tests.

The 9,481-line controller-options differential shell script is now a 539-line
entry point and six sourced parts under 1,900 lines each. The extraction
reconstructed all 457,048 original bytes. Shell syntax/hygiene checks include
the new parts, and the complete differential matrix passed for both frozen
slskd and slskdN targets in the `6163c7f8` release gate and the later
`c6eedb8b` gate retry.

The 6,063-line parity audit Python entry point is now 3,759 lines. Its seven
frozen-source exception constants/functions moved into the 2,336-line
`scripts/parity_not_applicable.py` module, with each extracted AST item checked
byte-for-byte against the committed source. Audit tooling, CLI import, and diff
checks pass. This is an actual module boundary; the Rust test segment splits
above remain flat-scope interim work.

## Council Scan Provenance And Scalar Boundary Review (2026-09-27)

Council freshness previously required the stamp to equal `HEAD`; committing a
refreshed stamp immediately made it stale. The source stamp now hashes the
tracked files read by the scan and excludes its generated inventory/backlog.
The protocol scalar scan now includes extracted production daemon modules and
excludes controller test segments. It reports 305 candidate lines: 24 `u8`,
21 `u16`, 42 `u32`, 184 `u64` casts, with the balance mainly checked
length conversions. Review of the narrowing candidates and suspicious signed
conversions found four concrete wrapping paths. Oversized share media values
now saturate; distributed and wishlist scheduler SQLite values reject invalid
depths/indices/intervals; persisted user timestamps clamp negative values.
Focused regressions pass, as do the protocol taint lens and adversarial corpus.
Council candidate counts remain heuristic line counts, not a proof that every
cast is dangerous.

## Release Gate And Hosted Packaging Follow-up (2026-09-27)

The complete local release gate passed at `6163c7f8`, including the frozen
controller-options matrix, workspace Rust tests, security scans, Web tests,
dashboard checks, SDK gates, and artifact checks. GitHub Rust CI and all Linux
platform jobs and all CodeQL language jobs also passed at `c6eedb8b`.

Hosted macOS arm64 and x64 archive jobs passed at `c6eedb8b` after replacing
GNU-specific tar flags with a sorted Python tar writer. Hosted Windows x64
found two separate ZIP packaging errors: the staging timestamp call used an
unsupported `follow_symlinks` argument, and `ZipInfo` received a `datetime`
instead of its required six-field tuple. Both are corrected in the current
batch. With a fixed `SOURCE_DATE_EPOCH`, two local tar builds have identical
SHA-256 `89a5be824e5600b51ac6d774cff07342decfb7cde28446ccb27a9fa797b58e6d`;
the release artifact verifier accepts all 115 normalized entries. Executing
the exact ZIP writer block twice against the same staged tree produced
identical SHA-256 `6e1e2d6ddfe082906077158451b7ac2eaf286292a9707ebd6fa1202525f2e307`;
the artifact verifier accepted its 110 entries. Hosted Windows proof is still
pending at this source revision.

An exact-tip local gate at `c6eedb8b` had one frozen slskdN regex-protocol
startup bind failure. That isolated differential passed for both frozen targets
on retry, and the subsequent full matrix passed. The same gate later had one
intermittent MediaCore opinion test failure; that test and its 36-test file
passed separately. The test suite now resets queued mock responses before each
case, and the complete Web suite passes 905 tests across 148 files. The next
full gate and hosted CI run will validate the combined packaging and test fixes.

The subsequent local gate completed successfully; its log is
`target/rf-release-gate-339e6812.log`. It began at `339e6812`, and the workflow
artifact reuse and Python protocol inventory split were committed while it ran,
so it is working-tree evidence rather than immutable exact-tip certification.
Semgrep reported zero findings from 710 rules across 2,160 files; Trivy, Rust,
Web, dashboard, and the complete frozen options matrix passed. The later
single-port TLS classification change has its focused listener proof below.

Hosted Windows archive construction, archive verification, and the packaged
binary smoke all passed at `339e6812` in GitHub CI run
[36354468661](https://github.com/snapetech/slskr/actions/runs/36354468661).
The macOS and all Linux platform jobs also passed at that revision. The overall
workflow and downstream package-surface job completed successfully, including
Debian/RPM packages for both architectures and deployment/package policy checks.

## RF-034 Shared Web Asset Build (2026-09-27)

The seven platform CI jobs previously ran `npm ci` and built the same production
Web assets independently. CI now has one `web-assets` job that installs, builds,
verifies, and uploads `web/build`. Each platform job downloads that artifact
before its release archive build, while retaining its platform-specific binary
build and archive verification. The workflow passes actionlint and the release
workflow policy check locally. The shared-artifact path still needs hosted
matrix proof; RF-034 remains in progress until that run completes.

## RF-024 Protocol Parity Inventory Ownership (2026-09-27)

The protocol enum/string inventory readers and per-case manifest entry builder
now live in `scripts/parity_protocol_inventory.py`; the main parity audit
imports their public functions. The four extracted function bodies were
preserved byte-for-byte from a pre-edit snapshot. Both frozen target inventories
and all generated entry rows match the original implementation (123/615 for
slskd and 170/850 for slskdN). Python compilation, CLI import, and the audit
tooling check pass. This is a real module boundary, but RF-024 remains open
while the large Rust test segments still share flat `include!` scope.

## RF-001 Shared-Port TLS Classification (2026-09-27)

The mesh branch of the shared listener previously classified only the first
two bytes, `0x16 0x03`, as TLS. A valid 790-byte plain Soulseek `PeerInit` frame
has the same length prefix and could be routed to the mesh gateway. The listener
now peeks at the bounded six-byte TLS record and ClientHello prefix before
routing, without consuming either protocol's bytes. A regression sends the
colliding plain frame through the real TCP listener, and all 30 listener tests
pass. This fixes that classification error on the single shared port. RF-001
still retains the legacy plain/obfuscated framing ambiguity recorded above.

## RF-006/RF-007 Live Shutdown Overlap Harness (2026-09-27)

`scripts/run-rf-shutdown-overlap.py` creates bounded real filesystem fixtures,
starts an isolated native daemon with persistence, observes an active share
scan through the API, sends SIGTERM, and reopens the same state. Its local run
passes: the in-flight HTTP scan returns 503, both processes exit zero, SQLite
integrity remains `ok`, and neither the durable store nor restart exposes a
partial index. DHT, mesh, overlay, and HTTPS are disabled only in the temporary
fixture, so this proves the HTTP/share-scan overlap rather than every managed
service. The harness bounds startup, requests, and shutdown, kills failed child
processes, removes fixtures, and records source/binary/harness metadata. The
clean-worktree run at `55be6aec` is retained in
`benchmarks/artifacts/20260927-rf-shutdown-overlap.json`: shutdown and restarted
shutdown both completed in 0.114 seconds, the scan returned 503, SQLite remained
valid with zero partial rows, and restart reported zero files. This closes the
retained live overlap requirement for RF-007. RF-006 still requires clean-runner
coverage for the other managed services.

## RF-006 Distributed Link Shutdown Ownership (2026-09-27)

The distributed parent and child socket loops still used detached `tokio::spawn`
calls after the broader task-registry migration. Both now use the managed task
registry, which aborts and joins producers before the final distributed snapshot
is persisted. A file-backed regression registers a real TCP child, receives the
branch metadata, sends a depth update, shuts down the registry, observes socket
closure, and reloads the latest child depth from SQLite. The focused regression
and all 647 daemon library tests pass, as do the full-controller/legacy feature
compile and changed-file formatter. Broader clean-runner and live parent/service
coverage remains open.

## RF-003 Retained Distributed Shutdown And Crash Proof (2026-09-27)

The extended live harness passed at clean commit `6fb4d2f6`. Its retained
`benchmarks/artifacts/20260927-rf-distributed-shutdown-crash-restart.json` records
source, binary, harness, and toolchain metadata. A live child depth-3 snapshot
exists before SIGTERM overlaps a real 20,000-file share scan; graceful shutdown
closes the socket, exits zero in 0.114 seconds, and persists the final
disconnected tree with no children. Restart restores that final state. A new
committed depth-7 snapshot then survives SIGKILL and another restart, with both
distributed tables matching and SQLite integrity `ok`. This closes RF-003's
retained live shutdown/crash requirement. RF-006 still needs clean-runner
coverage for the other managed services.

## RF-024 Focused Controller Test Ownership (2026-09-27)

The three flat focused-test includes now form eight named Rust test modules
with a shared fixture module. The extraction audit accounts for all 255,275
original bytes and all 91 test functions; the pre-edit files and audit remain
under `target/rf-focused-controller-before-modules/` and
`target/rf-focused-controller-module-audit.json`. The subprocess durability
test now selects its nested module path. Bounded controller fixtures reference
their actual capability, session, mesh, bridge, and quarantine owners rather
than feature-gated root aliases. The bounded authorization runner emitted
7,790 passing rows. All 647 default daemon library tests, strict all-targets Clippy, and the
full-controller/legacy all-targets compile pass on the final extraction.
The formatter excludes deleted files when checking a module move. RF-024
remains open for the larger full-controller include segments. This work is
internal-only and preserves the production listener's single-port design.

## RF Validation Harness HTTP Ownership (2026-09-27)

Hosted Semgrep at `a7bba2d8` flagged dynamic urllib use in the isolated live
shutdown fixture. The fixture now uses an explicit loopback HTTP connection
with a five-second timeout and closes it in a `finally` block. The live
shutdown/distributed crash-restart proof still passes with this adapter.
The existing retained artifacts keep their original source and harness hashes;
a later hosted run must confirm the scanner result for the replacement.
This is an internal-only validation-tool change.

## RF-024 Full Controller Bridge Contract Ownership (2026-09-27)

The bridge client-isolation and search/download oracle-shape regressions now
live in a real `controller_tests::bridge_contracts` module. They import only
the shared state/share fixtures and use the crate's route entry point. Exact
reconstruction of the moved bodies is recorded alongside pre-edit snapshots
in `target/rf-controller-bridge-before-module/`. The full-controller/legacy
all-targets compile passes. This internal-only step starts ownership boundaries
inside the remaining full-controller includes; RF-024 remains open.

## RF-001 Unknown Initialization Stream Boundary (2026-09-27)

The shared-port parser retained the shortest unknown frame while probing a
longer alternate interpretation. If neither was known, it returned the shorter
frame with an already advanced stream. The fallback now rejects this case as
ambiguous. A real TCP regression constructs two valid unknown interpretations
of different lengths; another checks that an unambiguous unknown initialization
retains its payload and subsequent bytes. The listener still uses the existing
shared port. This closes the fallback stream-boundary defect; the broader
legacy plain/obfuscated ambiguity remains open under RF-001.

## RF-006 Peer Listener Task Ownership (2026-09-27)

The listener's accepted initialization workers and peer handlers still used
detached tasks. Both now register with managed shutdown. A network-guard lease
releases admission counts when a handler completes, fails before dispatch, or
is canceled; connection permits remain owned by the handler. The regression
opens one stalled handshake and one initialized peer on the same shared
listener, then checks managed shutdown closes both sockets, returns capacity,
and clears the per-IP guard counts. All 648 daemon library tests pass,
including this real TCP shutdown regression. Broader clean-runner service
coverage remains open under RF-006.

## RF-006 Session Worker Ownership (2026-09-27)

Peer capability probes, the five-second wishlist fallback, and incoming search
responses now use managed task ownership. A queue lease releases incoming
search admission counts even if shutdown cancels a worker waiting for a permit
or rejects its future before polling. The focused regression holds every
search permit, schedules work, joins shutdown, and verifies both queued and
subsequent rejected work leave a zero queue count. All 649 daemon library
tests pass. Other service coverage remains open under RF-006.

## RF-024 Media And Song Identification Test Ownership (2026-09-27)

Nine media/analyzer/hash-database contract tests and eight song-identification
contract tests now live in real `controller_tests::media_contracts` and
`controller_tests::songid_contracts` modules. Their fixture imports are explicit,
and implementation paths name the crate owner. The pre-edit source and audit
in `target/rf-controller-media-before-modules/` account for 38,382 moved bytes
and all 17 test bodies with exact reconstruction before formatting. The
remaining full-controller includes still need ownership boundaries; this
internal-only step does not close RF-024.

## RF-003/RF-006/RF-007 Owned-Worker Live Revalidation (2026-09-28 UTC)

The native daemon was rebuilt at clean commit `dbc188d9` after the peer listener
and session workers moved into managed shutdown. The retained
`benchmarks/artifacts/20260928-rf-owned-workers-shutdown-crash-restart.json`
records source, binary, harness, and Rust toolchain metadata. SIGTERM during
an observed real 20,000-file scan and an attached distributed child exits
zero in 0.164 seconds, closes the child socket, returns HTTP 503 to the scan,
and leaves zero partial share rows with SQLite integrity `ok`. The final
disconnected distributed snapshot survives restart; a later committed depth-7
child snapshot survives SIGKILL and another restart. The direct loopback HTTP
adapter also passes hosted security scans at `47440cfb`. RF-003/RF-007 retain
their verified local status; RF-006 still needs broader service coverage.

## RF-024 Mesh Controller Test Ownership (2026-09-28 UTC)

Eight mesh controller/runtime contract tests now live in an owned
`controller_tests::mesh_contracts` module with explicit state/share/capability
fixture imports. Six differential entry points remain available to bounded
API group 1 through feature-gated exports. The pre-edit source and audit under
`target/rf-controller-mesh-before-module/` account for all 59,932 moved bytes
and all eight bodies, with only root-owner paths and bounded visibility
adapted. The full-controller/legacy all-targets compile passes. This
internal-only step leaves RF-024 open for the other include segments. The
bounded API group 1 runner now passes after the separately documented fixture
and storage-error corrections; all 58 rows in its six mesh ledgers pass.

## RF-001 Fixed Firewall Initialization Bound (2026-09-28 UTC)

Known firewall initialization has a five-byte body. The shared parser now
checks that bound in its header validation, for both plain and obfuscated
interpretations, before resizing the receive buffer. The regression advertises
the maximum general frame length while sending only each header and keeping
the connection open; rejection must arrive without reading a body. This
closes the oversized known-firewall allocation defect on the existing shared
port; all 33 listener tests pass. Broader legacy framing ambiguity remains
open under RF-001.

## Bounded Search Lifecycle Assertion Correction (2026-09-28 UTC)

The bounded API group 1 run stopped before the extracted mesh cases because
its old search lifecycle test expected cancellation and an explicit status
update to overwrite an already completed search. `SearchStore` already rejects
terminal-state changes. The test now checks that completed state survives both
requests, that its metadata can still update, and that cancellation changes a
separate active search to `cancelled` before deletion. This internal-only
fixture correction retains cancellation persistence coverage.

## Mesh Unavailable-Storage Response Classification (2026-09-28 UTC)

The owned mesh differential exposed two stale runtime-failure assertions that
expected success after closing durable storage. Rollback already occurred,
but the hash-database error prefix was not classified as storage unavailability,
and mesh merge bypassed the shared response classifier. Both paths now return
a sanitized HTTP 503 for unavailable storage; validation errors keep their
client-error response. The differential checks 503 and missing rolled-back
entries, and a default focused regression checks both routes retain sequence
zero and hide closed-pool details. All 650 default daemon library tests pass.
The shared classifier also covers hash-database reads/writes that already use
it. This behavior fix has a new release fragment.

## RF-024 Retained Owned Mesh Differential Proof (2026-09-28 UTC)

A fresh bounded API group 1 run at clean commit `84e42960` passes. All six
owned mesh ledgers were rewritten during that run and contain 58 passing rows.
`benchmarks/artifacts/20260928-rf-owned-mesh-bounded-differential.json` retains
the rows, ledger digests, source commit, binary/module hashes, toolchain,
platform, command, and timing metadata. This records real execution through
the new module exports and the unavailable-storage rollback assertions.
RF-024 remains open for the other controller include owners.

## RF-001 Unknown Initialization Allocation Bound (2026-09-28 UTC)

Unknown initialization codes previously retained the general 16 MiB frame
limit on the shared listener even though known initialization is bounded.
Header validation now applies `MAX_PEER_INIT_FRAME_LEN` to these extensions
for both encodings, keeping receive allocation bounded before authentication.
A header-only regression advertises the old general maximum and requires
immediate rejection; another accepts extensions exactly at the initialization
bound and preserves following bytes. All changes use the existing shared
port. The legacy byte-level framing ambiguity remains open under RF-001.

## RF-034 Retained Shared Artifact And SDK Closure (2026-09-28 UTC)

GitHub CI run [36359792162](https://github.com/snapetech/slskr/actions/runs/36359792162)
at `47440cfb` completed successfully: the Web producer, all seven platform
archive jobs, Windows packaged-binary smoke, native AArch64 tests, Rust/security
gates, and the downstream package/deployment job.
`benchmarks/artifacts/20260928-rf-shared-web-hosted-archive-matrix.json` retains
job links, source commit, the eight artifact identifiers/digests/expiry dates,
and successful archive verification/download steps. One additional duplicate
TypeScript build remained in the combined SDK script; the package checker now
owns that single build and its tracked-dist/package verification. The full
Go/Python/TypeScript SDK gate passes in 22.2 seconds with a 452.8 MiB peak and
zero swap. Together with the earlier deterministic TAR/ZIP/SBOM evidence, this
closes RF-034. The latest source tip still has its own queued hosted CI run.

## RF-024 Controller Segment 07 Owner Completion (2026-09-28 UTC)

The remaining segment 07 functions now live in five named contract owners
(native routes, collection authorization, controller surfaces, quarantine, and
listening parties) and one shared quarantine-verdict fixture module. The two
API group 1 entry points retain feature-gated exports. Exact pre-format
reconstruction accounts for all 79,986 bytes and all 16 functions; snapshots
and the per-owner audit remain under
`target/rf-controller-segment-07-before-owners/`. The old include is removed.
The full-controller/legacy all-targets compile and bounded API group 1 runner
pass. This is internal-only; RF-024 remains open for the other 28 includes.

## RF-024 Remaining Controller Test Owner Extraction (2026-09-28 UTC)

All 28 remaining flat controller test includes are replaced by domain modules
and shared fixture owners. Rust syntax spans account for 915 test functions,
58 helper functions, and every one of the original 5,131,884 bytes before
formatting. Code-token path changes and visibility insertions are reversed in
the extraction audit to verify exact source reconstruction; strings and
comments are preserved. The formatted test-name inventory matches the original
exactly. The 114 new owners are at most 2,399 lines; the existing manual owners
remain in place. Shared state, network, federation, persistence, route fixtures,
and bounded runners have their own modules. Feature guards and the public
bounded-runner entry point are preserved.

`benchmarks/artifacts/20260928-rf-controller-domain-owner-extraction.json`
retains the source revision, original file/item hashes, function-to-owner map,
and formatted module hashes. Snapshots remain under
`target/rf-controller-domain-owner-backup/`. The Rust hygiene gate now rejects
flat includes and controller test owners over 2,500 lines. This is internal-only
work. The full-controller/legacy all-targets compile, runtime boundary hardening,
formatter, and ownership/hygiene gates pass. RF-024 remains open for its broader
tracked-source scope; bounded execution gates are recorded after they finish.

## MusicBrainz Overlay Routing Differential Repair (2026-09-28 UTC)

Bounded API group 2 exposed a real validation mismatch: overlay routing used
an opaque-reference validator that rejected its generated `edit:<id>` channel
and namespaced `actor:<id>` peers. Overlay routing now shares the existing
bounded artist-radar identifier validator. A focused regression checks empty
peers, namespaced peers, path/URL metadata and targets, oversized identifiers,
and durable readback of every failed attempt. All 651 default daemon tests and
bounded API group 2 pass; strict daemon all-targets Clippy passes. The behavior
fix has its own release fragment. API group 1 also passes through the extracted
controller owners at clean commit `3552f9ed`.

## RF-024 Native Web Rendering Ownership (2026-09-28 UTC)

A complete tracked-source scan found the 5,822-line native Web renderer outside
the earlier daemon/Web-JSX closure inventory. Its 121 functions now live in
nine owners: route data projections, reference panels, native rows, tabs,
workspace panels, workflow rendering, shell startup, table navigation, and
transfer controls. WASM-only navigation and controls retain module guards;
public rendering and WASM exports retain their signatures. Exact pre-format
reconstruction accounts for all 278,732 bytes, and the formatted function-name
inventory matches. The largest owner is 1,339 lines.

`benchmarks/artifacts/20260928-rf-native-web-rendering-owner-extraction.json`
retains the source/item and formatted-module hashes; source snapshots remain
under `target/rf-web-rendering-owner-backup/`. All 86 native Web tests, strict
all-targets Web Clippy, and the locked WASM-target compile pass. This is an
internal-only ownership move. Across tracked Rust files, the largest remaining
source is daemon `config.rs` at 4,703 lines; RF-024 stays open while broader
ownership and bounded execution gates continue.

## RF-024 Bounded API Group 3 Fixture Repair (2026-09-28 UTC)

The owned group 3 run exposed stale MultiSource success expectations for
`skipVerification` requests. Verified swarm execution already requires an
expected hash. The differential now checks that bypass requests return 400
without queuing a job, and uses verified requests with blocked unspecified
source addresses for bounded failure execution without HTTP I/O. Async
responses retain the existing 202/queued/id contract; the initial worker is
waited to completion before its fixture directory is removed. A PodCore
bad-channel check also compared a body string directly with a JSON object;
it now parses the response before comparing the existing error contract.
Bounded API group 3 passes through the extracted owners. These are internal-only
fixture repairs; no production behavior changes in this checkpoint.

## RF-001 Shared-Port Random-Key TLS Exclusion (2026-09-28 UTC)

Random obfuscated initialization keys now avoid the `0x16, 0x03` TLS prefix in
addition to plausible plain frame lengths. An exhaustive unit check covers all
65,536 keys in that prefix range; the random writer regression checks both
exclusions. All 359 client tests, including 35 listener tests, and strict
client all-targets Clippy pass. This uses the existing shared port and has a
release fragment. Broader legacy framing ambiguity remains open under RF-001.

## RF-024 Bounded API Group 4 Completion And Relay Repairs (2026-09-28 UTC)

All four bounded API groups now execute successfully through the new controller
owners. Group 4 exposed additional stale assertions: the Spotify fixture now
calls its real helper owner; restart requests remain current-process latches
and stay cleared in durable/rehydrated state; controller transfer IDs are
strings; and audio canonical/dedupe checks now verify exactly the seeded key,
size, hash, and variant instead of expecting empty populated arrays.

The relay fixture exposed two production defects. Invalid authorization consumed
its one-use upload token but left its waiter pending; it now wakes the waiter
with failure immediately, while replay remains rejected. Uploaded staging files
were opened for writing only and caused bad-file-descriptor errors in the HTTP
reader; they now retain a readable, rewound original handle after syncing,
with exclusive creation and private permissions unchanged. Direct regressions
cover rejected credentials, filename mismatch, token consumption, waiter
removal, readable bytes, permissions, and overwrite rejection. Both fixes have
release fragments. Fixture stream/hub waits now have five-second deadlines and
abort/join HTTP workers on timeout; large-stack controller cases have a
60-second deadline. The stalled owned process was stopped and reaped.

`benchmarks/artifacts/20260928-rf-controller-owned-api-group-4.json` retains 96
passing rows across the relay, audio, upload, and core restart ledgers, binary
and changed-source hashes, command, toolchain, platform, and the working-tree
evidence scope. All 653 default daemon tests and strict daemon all-targets
Clippy and the full-controller/legacy all-targets compile pass.
RF-024 and RF-006 remain open for their broader scopes and remaining gates.

The refreshed council counts are protocol scalars 307, resolver/raw streams
1,027, and task/lifecycle 1,275. New scalar candidates are bounded key tests;
the new readback candidate reads a six-byte local staging fixture; lifecycle
additions are the fixture completion/abort/deadline checks above. These are
guarded test paths rather than newly accepted production bugs.

## RF-001 Nested Shared-Port Initialization Collision (2026-09-28 UTC)

A real nested collision is now covered: the same wire decodes as a plain
`PierceFirewall` and a supported nested obfuscated file-transfer `PeerInit`.
Previously the known-only prefix check returned the shorter plain frame. The
listener now recognizes potentially nested headers by the outer bounded length
and its necessary inner-length low byte, and rejects conflicting init headers
at nine bytes before reading either body. An open-sender regression proves
early rejection; a separate case proves an unambiguous nested file init retains
the following bytes. All 361 client tests, including 37 listener tests, and
strict client all-targets Clippy pass. A release fragment records the fix.
No new listener or port is added.

A broader rule treating every unknown bounded code as another known init was
rejected after it failed ordinary plain-handshake regressions. Unknown arbitrary
legacy extensions cannot establish sender intent when their bytes also encode
a known init; this compatibility boundary remains explicit under RF-001.

## RF-024 Remaining Owned Differential Gates (2026-09-28 UTC)

The clean committed source at `ec6d6889` passed the combined bounded persistence,
file-lifecycle, protocol, security-control, and security-authorization gates.
`benchmarks/artifacts/20260928-rf-controller-owned-remaining-differentials.json`
retains 53 freshly generated ledgers with 773 passing rows, plus the 7,790-row
passing authorization matrix, command, source revision, and binary/log hashes.
Together with the four bounded API groups, this validates the selected
differential runners through their extracted owners. It does not claim execution
of every opt-in full-controller test. The process exited successfully and was
reaped. Broader production ownership and lifecycle work remains open.

## RF-006 Managed Asynchronous Swarm Execution (2026-09-28 UTC)

Both asynchronous swarm route paths now register execution with the daemon's
managed task registry. Admission is atomic with registry closure; a rejected
request marks only its own job failed and returns HTTP 503. After managed
workers are aborted and joined, shutdown marks queued/in-progress swarm jobs
failed while preserving completed/failed jobs. Existing workspace drop cleanup
removes unpublished temporary chunks when cancellation drops the executor.
No new listener or port is added. A release fragment records the lifecycle fix.

A real two-source stalled transfer proves managed shutdown removes its
temporary workspace and rejects subsequent task admission. Bounded source
waits prevent a stale harness process. Additional tests cover direct stopped
admission, both async URL forms in both controller profiles, and preservation
of terminal job records. All 656 default daemon tests, strict daemon
all-targets Clippy, full-controller/legacy all-targets compile, and bounded API
group 3 pass. RF-006 remains open for remaining service lifecycles and hosted
proof.

## RF-024 Configuration Contract Test Owners (2026-09-28 UTC)

Extracted all 79 configuration tests from the 3,797-line `config_tests.rs`
into seven real domain modules: peer transport, Web security, media integration,
transfer policy, file layers, runtime policy, and federation/membership. The
formatted registry is 41 lines; the largest owner is 872 lines. The shared
environment fixture stays in the registry. No test body or expected contract
was removed. Only token-identified parent paths are relocated to `crate::config`.

`benchmarks/artifacts/20260928-rf-config-test-owner-extraction.json` accounts for
all 140,311 original bytes, per-test hashes, fixture bytes, formatted owner
hashes, and exact reconstruction after reversing every path relocation. All
79 test names match the original inventory. All 79 configuration tests, all
656 default daemon tests, strict daemon all-targets Clippy, and full-controller/
legacy all-targets compile pass. Module hygiene now enforces a 1,200-line
configuration-test owner budget and forbids flat includes in the registry.
This checkpoint is internal-only. Production configuration ownership and other
RF-024 scopes remain open.

## RF-024 Production Configuration Domain Owners (2026-09-28 UTC)

Reduced `config.rs` from 4,703 lines to a 290-line aggregate and registry.
Ten real owners hold startup orchestration, environment layers, peer transport,
network/security settings, Web security, transfer policy, federation/membership,
media services, file loading, and projection. The largest owner is 891 lines.
All 57 original public function/type declarations retain their configuration
namespace bindings; existing settings/integration exports and the configuration
environment trait remain available. Private members gain only the visibility
needed to retain access within the original configuration boundary. Four
public compatibility exports have a narrow unused-import allowance so their
existing paths remain available in builds without current callers.

`benchmarks/artifacts/20260928-rf-config-production-owner-extraction.json`
accounts for all 172,044 original bytes and all 154 original top-level items,
retains per-item and formatted-owner hashes, and proves exact reconstruction
after reversing visibility relocations. All 100 function/method declarations
remain. All 79 configuration tests, 656 default daemon tests, strict daemon
all-targets Clippy, and full-controller/legacy all-targets compile pass.
Boundary checks search the real owners; module hygiene forbids flat includes,
caps this aggregate at 400 lines, and caps its owners at 1,200 lines.
This checkpoint is internal-only. The 3,231-line file-configuration model and
other RF-024 scopes remain open.

## RF-024 File Configuration Model And Validation Owners (2026-09-28 UTC)

Reduced `config_file.rs` from 3,231 lines to a 210-line aggregate and registry.
Ten domain owners contain the input models and their validation/default
implementations: federation/membership, filters/shares, foundation, integration,
media services, network security, peer transport, signal policy, transfer policy,
and Web security. The largest owner is 863 lines. All 129 original public and
configuration-scoped declaration bindings remain available from the file-model
namespace. Nested input-type compatibility exports have targeted unused-import
allowances; implementation bodies have no such allowance.

Moved `pub(super)` members explicitly retain `crate::config` visibility, and
moved private members retain file-configuration visibility. Serde attributes,
field values, layer ordering, validation bodies, and default implementations
are preserved. The retained audit at
`benchmarks/artifacts/20260928-rf-config-file-model-owner-extraction.json`
accounts for all 123,927 original bytes and all 150 original items, proves exact
reconstruction after reversing visibility edits, and preserves all 22 function/
method declarations. All 79 configuration tests, 656 default daemon tests,
strict daemon all-targets Clippy, and full-controller/legacy all-targets compile
pass. Hygiene caps the aggregate at 300 lines and these owners at 1,200 lines.
This checkpoint is internal-only. Native Web ownership and other RF-024 scopes
remain open.

## RF-024 Native Web Contract Test Owners (2026-09-28 UTC)

Replaced the included native Web test module with a real test-module registry.
All 86 tests now have eleven domain owners, plus one shared fixture owner.
The registry is 32 lines and the largest owner is 766 lines. RustyMilk runtime,
preset, geometry, GPU, and shader contracts are owned separately from native
actions, workflow rendering, response projection, player/search, shell lifecycle,
and route inventory contracts. Test names and assertions remain intact.

`benchmarks/artifacts/20260928-rf-native-web-test-owner-extraction.json`
accounts for all 156,690 original bytes and 4,092 original lines, including
the moved module wrapper, all 93 items, and both shared fixtures. It verifies
exact reconstruction after reversing visibility and dependency-path relocations.
The two moved `include_str!` dependencies resolve to their original files with
matching hashes. All 86 Web tests, strict Web all-targets Clippy, and the locked
WASM target check pass. Hygiene caps the registry at 100 lines and owners at
1,000 lines, and forbids a return to flat test includes. This checkpoint is
internal-only. Production Web action ownership remains open under RF-024.

## RF-024 Native Web Action Domain Owners (2026-09-28 UTC)

Replaced the 3,630-line `web_actions.rs` flat include with thirteen real owners:
WASM action execution, form values, wishlist operations, share access, row
context, filters, selection/inspection, sorting, reference controls, action
projection, and player/search/experience models. The largest owner is 567
lines. WASM-only modules and imports retain their target guards; the action
projection keeps its original WASM-or-test availability. All eleven original
public declaration bindings remain at the crate root.

`benchmarks/artifacts/20260928-rf-native-web-action-owner-extraction.json`
accounts for all 133,129 original bytes, all 95 original items, and all 87
functions, including exact reconstruction after reversing visibility/path
relocations. Source backups and original committed source remain available.
All 86 Web tests, strict Web all-targets Clippy, and the locked WASM target
check pass. Hygiene requires real action modules and caps owners at 1,000
lines. This checkpoint is internal-only.

The complete tracked Rust scan now has six files above 2,500 lines:
`private_gateway.rs` (3,524), `session_runtime.rs` (3,083), `cli_smoke_soak.rs`
(2,914), native Web `search_planning.rs` (2,857), native Web `rustymilk_ui.rs`
(2,802), and `file_transfer_runtime.rs` (2,524). RF-024 remains open for these
and its other scope/evidence requirements.

## RF-024 Native Search, Player, and Workspace Owners (2026-09-28 UTC)

Replaced the 2,857-line `search_planning.rs` flat include with eleven real
modules for search ranking/previews, player radio/queue planning and browser
controls, experience reports, workspace tables/actions, browser preferences,
player status projection, and visualizer startup. The largest owner is 520
lines. WASM guards and all 22 public root bindings remain intact.

`benchmarks/artifacts/20260928-rf-search-planning-owner-extraction.json`
accounts for every original byte and all 87 items, with exact reconstruction
and reversed visibility relocations. All 75 functions remain present. All 86
Web tests, strict all-targets Clippy, and the locked WASM check pass. Hygiene
forbids the old flat include and caps owners at 1,000 lines. This checkpoint
is internal-only. Five tracked Rust files remain above 2,500 lines; broader
RF-024 scope remains open.

## RF-024 Visualizer and Live Web Runtime Owners (2026-09-28 UTC)

Replaced the 2,802-line `rustymilk_ui.rs` flat include with twelve real
WASM-only owners for browser controls, library projection/storage/actions,
playlists, preset editing, automation, file imports, audio analysis, live
player and route refresh, and HTTP request caching. The largest owner is
417 lines. Request-cache ownership stays together, and the original target
and method guards remain intact.

`benchmarks/artifacts/20260928-rf-rustymilk-ui-owner-extraction.json`
accounts for every original byte and all 88 items, with exact reconstruction
and reversed visibility relocations. All 84 function/method declarations
remain present. All 86 Web tests, strict all-targets Clippy, and the locked
WASM check pass. Hygiene forbids the old flat include and caps owners at
1,000 lines. This checkpoint is internal-only. Four tracked Rust files
remain above 2,500 lines: private gateway, session runtime, CLI smoke/soak,
and file-transfer runtime. Broader RF-024 scope remains open.

## RF-024 Session Runtime Domain Owners (2026-09-28 UTC)

Replaced the 3,083-line session runtime implementation with a 50-line registry
and twelve real owners for login replay, room dispatch, server transport,
peer connection and messages, command dispatch, session connection,
wishlist dispatch, server projections, supervision, pod room bridges, and
incoming search. The largest owner is 474 lines. Existing parent-scoped
bindings and managed worker lifetimes remain intact; shared-port connection
behavior is preserved.

`benchmarks/artifacts/20260928-rf-session-runtime-owner-extraction.json`
accounts for all 113411 original bytes and 38 items, with exact
reconstruction after reversing scope relocations. All 36 functions/methods
remain present. All 656 default daemon library tests, strict all-targets
Clippy, and the full-controller/legacy-route all-targets feature compile pass.
Runtime boundary checks include all new owners. Hygiene caps the registry
at 150 lines and owners at 1,200 lines. This checkpoint is internal-only.
Three tracked Rust files remain above 2,500 lines; RF-024 remains open.

## RF-024 Private Gateway Transport And Policy Owners (2026-09-28 UTC)

Replaced the 3,524-line private gateway with a 144-line registry and eleven
real owners for gateway models/transport/services, QUIC proxy admission and
relay I/O, service policy, mesh content projections, pod request models, peer
authentication, identity files, and contracts. The largest owner is 999 lines.
Existing shared TCP admission, certificate handling, replay protection, relay
limits, and public gateway type bindings remain intact. No dedicated port
was added.

`benchmarks/artifacts/20260928-rf-private-gateway-owner-extraction.json`
accounts for all 131592 original bytes and 119 items, with exact
reconstruction after reversing scope relocations. All 119 functions/methods
and 25 gateway contract tests remain present. The test module wrapper moves
to the registry while its original body and dependency scopes are preserved.
All 656 default daemon tests, strict all-targets Clippy, and the full-controller/
legacy-route all-targets feature compile pass. Hygiene caps the registry at
150 lines and owners at 1,200 lines. This checkpoint is internal-only. Two
tracked Rust files and the parity audit script remain above 2,500 lines;
RF-024 remains open.

## RF-024 Transfer And CLI Proof Runtime Owners (2026-09-28 UTC)

Replaced `file_transfer_runtime.rs` (2,524 lines) with an 80-line registry and
eleven real owners for paths, capacity/upload policy, negotiation, permissions,
audio metadata, indirect transfer, upload/download streaming, content safety,
progress, and inbound transfer. Replaced `cli_smoke_soak.rs` (2,914 lines)
with a 74-line registry and fourteen real owners for fixture transfers, live
server/peer runtime, configuration, probes, scenarios, protocol names, and
redaction. The largest transfer and CLI owners are 527 and 357 lines.
Original conditional helpers retain their guards; narrow unused-import
allowances preserve original parent-scoped compatibility aliases.

The two `20260928-rf-*-owner-extraction.json` artifacts for file-transfer
runtime and CLI smoke/soak account for every original byte and item, with
exact reconstruction after reversing scope relocations. All 56 transfer and
70 CLI function/method declarations remain present. All 656 default daemon
tests, strict all-targets Clippy, and the full-controller/legacy-route
all-targets compile pass against both extractions together. Boundary checks
include the transfer owners. Hygiene caps both registries at 150 lines and
owners at 1,200 lines. This checkpoint is internal-only.

A complete tracked Rust scan now finds no file above 2,500 lines. The broader
tracked code scan still finds `scripts/audit-parity-manifest.py` at 3,585
lines. Its current live-backfill source guard still targets the historical
`lib.rs` location and fails before producing a manifest; repair that guard
against the real backfill owner before extracting the audit domains. RF-024
remains open for that script and the remaining evidence requirements.

## RF-024 Parity Audit Source-Owner Guard Repair (2026-09-28 UTC)

The frozen config inventory now scans nested Rust implementation owners and
excludes named test suites, so test literals cannot replace missing owned
configuration settings. All 436 mapped frozen configuration leaves pass.
The live backfill source guard reads `hash_backfill_runtime.rs` and verifies
its daemon module registration while retaining remote-route, transfer-token,
and parsed-hash checks. Six boundary regressions prove positive wiring and
rejection of disconnected/missing owners, registry decoys, removed token and
route checks, and configuration test decoys. Audit tooling and process-memory
policy checks pass. The operator fragment captures these audit behavior fixes.

`benchmarks/artifacts/20260928-rf-parity-source-owner-guards.json` retains the
frozen source revisions and validation hashes. The normal reused-evidence
manifest remains blocked by absent `target/react-webui-audit/audit.json`.
A separate extraction-equivalence fixture disables differential execution and
browser proof, preserving needs-proof states; it is not live parity evidence.
RF-024 and the missing UI/evidence tasks remain open.

## RF-024 Parity Audit Domain Owners (2026-09-28 UTC)

Replaced the 3,589-line parity manifest implementation with a 605-line entry
point and sixteen real modules for API/security, configuration, WebUI,
persistence, files, operators, protocol, live catalogs/contracts/evidence,
transport/UI evidence, completeness, subprocess admission, and shared
constants. Every original function and constant binding remains at the entry
point. No dynamic execution or shared global namespace is used.

`benchmarks/artifacts/20260928-rf-parity-audit-owner-extraction.json`
accounts for every original byte and item with exact reconstruction. All 41
public function bodies and all constant declarations match the original AST,
including nested function bodies. Seven boundary/ownership regressions,
Python compilation, audit tooling, and process-memory policy gates pass.
The complete before/after extraction fixtures match all 19,216 entries when
fed identical captured frozen inventory inputs. Those fixtures disable live
and differential proof execution and preserve needs-proof states; they are
not live parity evidence. Fresh controller inventory provenance can vary
because unsorted `rg --files` results choose among duplicate route sources;
that separate determinism issue remains to repair.

The normal reused-evidence manifest still requires the absent React audit
artifact. Rust and script owners are structurally bounded, while the broader
source scan also identifies native Web `static/styles.css` at 3,316 lines.
RF-024 remains open for that stylesheet and its remaining evidence scope.
This extraction checkpoint is internal-only.

## RF-024 Native Stylesheet Source Owners (2026-09-28 UTC)

Replaced the 3,316-line native stylesheet with fourteen ordered source owners
and a small manifest. The largest owner is 403 lines. The existing native
Web build now assembles one distribution `styles.css`; it adds no browser
requests and no dedicated ports. All 57,794 original bytes, all 456 top-level
rules, and their exact cascade order survive unchanged. The distribution
stylesheet remains a generated artifact, rather than a large tracked source.

`benchmarks/artifacts/20260928-rf-native-style-owner-extraction.json`
retains owner hashes and exact reconstruction. The complete release WASM /
wasm-bindgen packaging build passes and emits a stylesheet with the original
hash. All 86 Web tests, strict all-targets Clippy, the locked WASM check, six
builder regressions, audit tooling, and style-owner hygiene pass. The native
confirmation-modal contract reads the ordered source owners. The builder
rejects empty, duplicate, omitted, escaping, and noncanonical manifests.
The operator fragment records the source/build materialization change.

The broad tracked source scan now has no code file above 2,500 lines;
RF-024's structural source split is complete. The retained extraction and
feature proofs remain distinct from missing deployed/live parity evidence.
Hosted validation and the remaining full-program lifecycle/evidence work
remain open; this checkpoint does not declare the RF program complete.

## RF-024 Source Budget And Stable Audit Provenance (2026-09-28 UTC)

Module hygiene now enforces the 2,500-line limit across every tracked code
file in the broad ownership inventory's suffix scope, alongside the tighter
domain-owner budgets. This guards against recreating oversized source files.
The controller audit sorts discovery paths before its existing last-declaration
selection, making duplicate-route provenance stable. A synthetic duplicate
fixture and two independent frozen slskdN inventories pass: both 683-route
JSON outputs are byte-identical. The operator fragment records the audit
provenance fix; proof requirements and route counts remain unchanged.

`benchmarks/artifacts/20260928-rf-controller-inventory-determinism.json`
retains the frozen revision, output hashes, selection policy, and gate logs.
RF-024 source ownership is structurally complete; hosted validation remains
open. The remaining RF program tasks retain their separate lifecycle and
live/deployed evidence requirements.

## RF-006 Managed Bridge Client Ownership (2026-09-28 UTC)

Removed the Soulfind bridge's detached accepted-client spawn. Every accepted
handler now enters the managed task registry. The listener owns a cancellation
channel and bounded completion tracking, reaps completed/panicked handlers,
and cancels/joins pending clients on listener stop. Dropping the listener
owner also closes cancellation, so client I/O does not outlive its owner.
Closed registry admission drops the rejected socket and removes its record.
After managed daemon shutdown joins the registered handlers, ephemeral bridge
client records and the running flag are cleared. No listener port was added.

`benchmarks/artifacts/20260928-rf-managed-bridge-client-shutdown.json`
retains source and gate hashes. Four deadline-bounded socket regressions prove
listener stop, listener-owner drop, joined daemon cleanup, and rejected late
admission. All 660 default daemon tests, strict all-targets Clippy, and the
full-controller/legacy-route all-targets feature compile pass. The operator
fragment records the lifecycle correction. Broader RF-006 clean-runner proof
for the remaining managed services remains open.


## Dashboard Hosted Coverage Gate Repair (2026-09-28 UTC)

Hosted run `36369051212` failed only the dashboard branch coverage threshold:
38 tests passed, but branch coverage was 54.21% against the required 55%.
The webhook lifecycle suite now exercises loading, invalid envelopes, create
validation and refresh, cancellation, confirmed deletion with encoded IDs,
test delivery, and failed mutations preserving existing rows. Explicit DOM
cleanup isolates the administrative page tests. The URL input and icon actions
have accessible names; loading and errors expose status and alert semantics.

All 48 dashboard tests pass. Coverage is 75.36% statements, 63.45% branches,
71.2% functions, and 79.4% lines, with every threshold unchanged. Type checking,
ESLint, production build, and bundle budgets pass. Evidence is retained in
`benchmarks/artifacts/20260928-rf-dashboard-webhook-lifecycle.json`; the release
fragment records the accessible controls. This fixes the observed local
reproduction; exact-tip hosted validation remains pending. No port was added.


## RF-006 Managed Shared Gateway Workers (2026-09-28 UTC)

Shared-listener TLS handlers, UDP control, QUIC control/data listeners and
accepted handlers, and DHT response forwarding now register with daemon
shutdown rather than detaching. Existing connection semaphores and rate limits
remain intact. A synchronous admission guard releases the rate limiter on
normal completion, cancellation, panic, or rejection before first polling.

Four deadline-bounded regressions prove stalled TLS socket closure on joined
shutdown and late rejection, plus admission release on cancelled and rejected
tasks. All 664 daemon tests, strict all-targets Clippy, and full-controller/legacy
all-targets compilation pass. Evidence is retained in
`benchmarks/artifacts/20260928-rf-managed-gateway-workers.json`. No production
port was added. The release fragment records the operational lifecycle fix.
Tunnel readers, proxy-session workers, outbound metadata cleanup, and broader
clean-runner service evidence remain RF-006 work; this checkpoint does not
claim their completion.


## Nested Audit Memory Guard Repair (2026-09-28 UTC)

Regenerating the missing React browser ledger exposed a guard nesting defect:
the Python audit already ran in a 4 GiB, zero-swap cgroup, but its guarded Node
build also received a 4 GiB virtual-address ceiling. Node failed to reserve
WebAssembly memory before compilation. A separately guarded build succeeded,
confirming the nesting path caused the failure.

Nested commands now reuse verified inherited limits. Linux validation checks
the actual cgroup `memory.max` and `memory.swap.max`, rather than trusting a
unit name or environment marker. Smaller nested requests still establish
their requested limit. Spoofed-marker, fallback, tighter-nesting, environment,
working-directory, Node heap, and real cgroup regressions pass; shell syntax,
process policy, and the complete audit-tooling gate pass.

The browser ledger regeneration is progressing in Chromium under the bounded
cgroup; complete UI evidence remains pending. Evidence is retained in
`benchmarks/artifacts/20260928-rf-nested-process-memory-guards.json`. Rust
commands continue to use Cargo directly without these wrappers.


## RF-006 Joined Gateway Child Resources (2026-09-28 UTC)

The managed registry now exposes an abort handle while retaining each task in
its join set. Tunnel readers use this admission path, and their existing local
Drop cancellation remains intact. QUIC proxy workers use the same registry;
session Drop cancels the worker, and closed daemon admission rejects creation
while releasing the session lease and socket. No production gateway owner
contains a detached `tokio::spawn`; the runtime boundary gate enforces this.

Overlay metadata uses short synchronous lock operations that never span an
await. Guards remove metadata directly, including on threads without a Tokio
runtime. Inbound TLS and QUIC guards also clear peer limiter state on cancelled
handlers. After joining workers, daemon shutdown clears gateway tunnels,
metadata, and replay entries.

All 667 daemon tests, strict all-targets Clippy, and full-controller/legacy
all-targets compilation pass. Bounded regressions verify session/daemon
cancellation, real UDP socket rebind, admission lease release, late rejection,
tunnel reader joining with TCP EOF, gateway record cleanup, and metadata Drop
outside Tokio. Evidence is retained in
`benchmarks/artifacts/20260928-rf-managed-gateway-child-resources.json`.
No production port was added. Broader RF-006 clean-runner coverage remains open.


## React Browser Contracts, Cleanup, And Honest Workflow Evidence (2026-09-28 UTC)

Regenerating the real Chromium audit exposed invalid success fixtures for
unread activity, rooms, jobs, metadata processing, auto-replace, and analytics.
Ten tests now validate the fixture responses through the actual frontend API
adapters. A failed Chromium launch previously left the temporary HTTP server
running; launch now belongs to cleanup, including when browser close fails.
A bounded subprocess regression proves launch failure exits promptly.

The audit also credited synthetic endpoint sweeps as rendered workflows,
inflating all four categories to 417 cases each. The authoritative workflow
ledger now removes inherited sweep settings and never requests sweeps. Fresh
and reusable reports must declare an integer zero `endpointSweepCount`;
unmarked, swept, malformed, and failed reports cannot become workflow proof.
Six evidence regressions and the launch regression run through audit tooling,
now wired into the GitHub Rust job.

The corrected Chromium run passes 84 route/viewport rendering checks and all
four scenarios, with zero sweeps and errors. Actual UI requests establish 73
success, 8 loading/empty, 11 error, and 14 authorization cases. The isolated
frozen manifest reuses these reports successfully: WebUI has 523 complete
cases (417 call-presence plus 106 workflow cases), with 1,562 still needing
proof. Six differential families were deliberately skipped for this UI-only
check; these counts are not whole-program certification or a replacement for
the separately retained differential evidence.

Full Web validation passes 915 tests across 149 files, ESLint, audit tooling,
workflow policy, artifact-matrix policy, and repository boundaries. Evidence
is retained in `benchmarks/artifacts/20260928-rf-react-webui-ui-request-evidence.json`.
This is real-browser evidence against deterministic mocks, not live daemon,
deployed-device, or credential-backed interoperability proof. No production
port was added. RF-030 nightly retention and the deployed evidence tasks stay
open.


## RF-024 Hosted Ownership Validation Baseline (2026-09-28 UTC)

GitHub run `36376549126` succeeds at immutable source `29165746`, including
all 11 jobs and the seven-platform archive matrix. Eight artifacts are retained;
the Linux GNU x64 archive was downloaded and its SHA-256 matches the retained
checksum. Run, job, and artifact links are recorded in
`benchmarks/artifacts/20260928-rf-ownership-hosted-validation.json`.
This closes structural source-ownership validation at that baseline. Later
gateway lifecycle and browser evidence corrections have separate local gates
and hosted runs; this receipt is not their exact-tip release certification.
All tracked source remains covered by the 2,500-line budget and tighter owner
budgets.

## RF-071 Hosted Reproducibility Artifact Wiring (2026-09-28 UTC)

The main Rust CI gate now collects validated metadata after its tests and
frontend builds, retaining it alongside the actual dashboard coverage summary
and Web entry point for 30 days. A job-summary link points to the uploaded
artifact. Metadata schema 2 adds operating system/architecture, required Cargo
configuration hashes, and source-bound GitHub run identity. Lock hashes,
observed tool versions, optional artifact hashes, and worktree state remain.
Generic CI-local checks do not invent an artifact name; unavailable worktree
status cannot be reported as clean. Tokens and other arbitrary environment
variables are not captured.

Twelve regressions cover local/hosted collection, real file hashing, required
inputs, duplicate paths, invalid digests/sizes, source mismatch, malformed run
context, dirty/unknown worktree state, and legacy schema 1 validation. The
actual local collector, default reproducibility check, audit tooling, workflow
policy, and package matrix gates pass. Evidence is retained in
`benchmarks/artifacts/20260928-rf-hosted-reproducibility-retention.json`.
Exact-tip hosted artifact validation remains RF-071 work; this wiring is not
yet a hosted receipt. No production port was added.


## RF-006 Single-Port Gateway Without DHT (2026-09-28 UTC)

Current native TCP sharing no longer depends on DHT enablement. Overlay bind
selection, startup projection, and the shared-listener predicate agree when
DHT or mesh DHT is disabled. The existing peer listener carries both ordinary
peer initialization and mesh TLS; gateway UDP uses the same port number.
Frozen profiles and explicit bind choices retain their existing contracts.

The daemon library suite passes 668 tests, strict Clippy passes, and the full
controller/legacy configuration compiles. The bounded live harness completes
three isolated daemon cycles with DHT disabled: one peer TCP listener, UDP on
that same port, established and partial TLS clients closed during graceful
shutdown, exit zero, and immediate TCP/UDP rebind. Three completed TLS
handshakes occupy the per-IP capacity and a fourth client is rejected. The
fixture's existing HTTP API listener is separate from the peer transport;
no new production port was introduced. Shutdown takes approximately 0.114
seconds locally. Six helper regressions and audit/workflow policy gates pass.

The harness always kills and reaps its owned daemon before saving logs on
failure. CI now runs and retains the three-cycle proof inside the reproducibility
artifact. Its hosted result remains pending. Evidence is retained under
`benchmarks/artifacts/20260928-rf-shared-tcp-shutdown-proof.json`.
Broader managed-service clean-runner coverage remains RF-006 work.

## RF-071 Downloaded Hosted Receipt (2026-09-28 UTC)

The Rust job at `37f2e049` completed successfully in run `36379384611`.
The actual `ci-reproducibility-rust` artifact was downloaded and validated:
metadata records a clean source checkout, all retained file hashes match,
and seven lock plus three configuration hashes match that exact Git commit.
Observed Cargo, Rust, Node, npm, and Python versions are retained, together
with immutable run and artifact links, in
`benchmarks/artifacts/20260928-rf-hosted-reproducibility-receipt.json`.
RF-071 main-gate hosted metadata retention is verified. This receipt does not
certify the later single-port change or substitute for its hosted execution.


## RF-030 Bounded Nightly React Evidence (2026-09-28 UTC)

The dedicated `React Nightly Audit` workflow runs daily and supports manual
dispatch without live credentials. Its four deterministic mock scenarios
exercise the actual React UI through Chromium, with desktop/mobile success
rendering and focused loading/empty, validation/error, and authorization/
reconnect/restart workflows. It retains reports, screenshots, logs, and a
source-bound receipt for 30 days. This is browser proof against mocks;
it does not certify deployed devices or credentialed upstream interop.

The runner clears inherited live-backend settings, tokens, endpoint sweeps,
and route restrictions. It rejects missing, failed, swept, mismatched, or
empty browser evidence and removes stale reports before every launch. Build and focused scenario commands have five-minute deadlines; the full
desktop/mobile scenario has an eight-minute deadline. Timeout and interruption
terminate the owned process group; a grace period lets the memory guard stop
its external systemd service before hard cleanup. Six regressions include a
real guarded-child timeout and confirm that child is reaped.

Hosted browser work runs in an owned service with 4 GiB resident memory,
zero swap allowance, 512 tasks, and a 15-minute deadline. The workflow itself
has a 25-minute deadline, cleanup trap, pinned actions, read-only permissions,
and a required artifact upload. Workflow policy guards this contract.
The retained hosted result is still required before RF-030 is closed.


## RF-006 Downloaded Single-Port Clean-Runner Receipt (2026-09-28 UTC)

The Rust job at `f9a25f9d` succeeded in run `36380983771`. Its actual retained
artifact was downloaded: reproducibility metadata, artifact hashes, and all
lock/configuration hashes match the exact clean source checkout. The live
proof passes all three cycles with DHT disabled, exactly one peer TCP
listener and gateway UDP on the same port. Established and partial TLS
clients close during graceful shutdown; TCP/UDP rebind succeeds. The fourth
client is rejected after three completed TLS handshakes occupy per-IP capacity.
The source-bound receipt and immutable artifact link are retained in
`benchmarks/artifacts/20260928-rf-hosted-single-port-shutdown-receipt.json`.
This closes shared-gateway clean-runner evidence within RF-006; other managed
services remain in its scope. No dedicated peer transport port was added.

## RF-047 Actual Mounted Sync Delay and Crash Stress (2026-09-28 UTC)

The isolated Linux FUSE fixture forwards file operations into an owned
throwaway directory and delays actual event-file fsync requests by 500 ms.
A preflight observes the kernel mount and measures an actual fsync delay.
The regression verifies the transfer queue is writable within 250 ms while
that filesystem request remains blocked, then verifies ordered durable events
and restart state. Three independent writer processes each persist eight
transfers on the delayed mount, are killed without shutdown, and recover all
24 transfers and 48 ordered events. This is a controlled delayed filesystem,
not a hardware performance benchmark or a pause injected into product code.

The ignored regression requires this explicit mount fixture and is run by
`python3 scripts/run-rf-delayed-filesystem-proof.py`. Cargo uses the pinned
toolchain directly. The harness bounds mount readiness and test runtime,
cleans owned process groups, unmounts the fixture, and reaps the filesystem
process. Writer guards kill and reap on failure. Five helper regressions
cover normal, failed, and timed-out cleanup. RF-047's delayed-filesystem and
crash-stress requirement is verified locally; product durability semantics
remain unchanged. Retained evidence is
`benchmarks/artifacts/20260928-rf-delayed-filesystem-proof.json`.


## RF-030 Hosted Deadline Adjustment (2026-09-28 UTC)

The first hosted nightly run completed 79 of 84 screenshots before the
five-minute success-scenario deadline. Its failed receipt and screenshots
were retained, and its owned service terminated with zero swap use. The
local final runner had already passed from a clean checkout. The full
84-check success scenario now has an eight-minute deadline; build and
focused workflow scenarios retain five-minute deadlines, and the outer
15-minute service plus 25-minute job limits remain. This adjustment does
not count partial screenshots or a failed receipt as passing evidence.
A fresh hosted run is required.


## RF-006 Bounded CLI Live-Soak Ownership (2026-09-28 UTC)

The CLI live-soak harness previously detached accepted peer handlers and
indirect peer probes with raw spawn calls. Each plain/obfuscated listener
and the server probe loop now owns a bounded 64-worker JoinSet. Completed
work is reaped before admission; capacity overflow drops the new future,
and shutdown rejects late work before aborting and joining existing work.
Listener deadlines close stalled peer handlers. The server loop joins its
probe workers on normal completion and on error. The top-level listener and
watchdog tasks remain in one owned set; the watchdog is joined and a failed
worker cancels the remaining tasks. No transport listener port was added.

Four focused regressions prove capacity/resource release, completed-worker
readmission, child socket closure on parent cancellation, and actual plain/
obfuscated TCP client EOF plus listener rebinding at the soak deadline.
The full daemon suite passes 672 tests with the separately exercised mounted
filesystem test ignored by default; strict Clippy passes. The runtime guard
rejects detached spawn calls in the three live-soak production owners.
Broader RF-006 managed-service clean-runner coverage remains open.


## RF-030 Downloaded Hosted Browser Receipt (2026-09-28 UTC)

The manually dispatched daily-workflow runner at `3f0b6010` completed
successfully in run `36382238277`. Its actual artifact was downloaded and
verified: the receipt records the exact clean checkout, the harness hash
matches that Git commit, and all four audit hashes match the downloaded
reports. The success report has 84 desktop/mobile route checks and all 84
referenced screenshots are present. Every scenario declares zero synthetic
sweeps and no audit errors. The receipt and immutable artifact link are in
`benchmarks/artifacts/20260928-rf-hosted-react-nightly-receipt.json`.
RF-030's browser retention scope is verified. This manual execution of the
daily workflow does not claim a scheduled event already occurred, deployed
device proof, or credentialed interop certification.

## RF-028 Validation-Only Chocolatey Dispatch (2026-09-28 UTC)

The existing manual Chocolatey workflow gains a boolean `publish` input.
Its default preserves the existing publication behavior; `publish=false`
runs the same actual release download, checksum validation, tag/version
rendering, `choco pack`, and nupkg smoke without executing the push step.
It retains the nupkg, checksums, and source/tag/archive/package-bound receipt
for 30 days. A 20-minute job deadline bounds this package runner. Local
workflow/package policy and PowerShell parser checks pass. A clean Windows
validation-only run remains required before RF-028 is closed.


## RF-028 Published Checksum Prefix Regression (2026-09-28 UTC)

The first clean Windows validation-only run failed before packing: the
actual `release-v0.2.40` SHA256SUMS entry prefixes the Windows filename with
`./`, while the existing workflow required a bare basename. The verifier
now accepts exactly the bare basename or its root-relative `./` spelling,
with an optional checksum binary marker. It still requires one unique match
and verifies the actual archive digest. Eight regressions execute the real
workflow PowerShell block, covering both valid spellings, the binary marker,
and missing, duplicated, wrong-hash, foreign-directory, and suffix entries.
A fresh hosted validation remains required; the first failure is not credited
as successful package proof.


## RF-028 Downloaded Windows Package Receipt (2026-09-28 UTC)

Validation-only run `36383308751` at workflow source `23e53097` passed on
Windows against the actual existing `release-v0.2.40` release. The nupkg and
receipt were downloaded. Its package hash matches; the independently
downloaded Windows release archive matches the published SHA256SUMS entry;
the packed installer contains the fully qualified tag/asset URL and exact
archive hash, and the nuspec records version 0.2.40. The receipt reports
`publishRequested=false`. Evidence and the immutable artifact link are in
`benchmarks/artifacts/20260928-rf-hosted-chocolatey-validation-receipt.json`.
RF-028 clean package-runner validation is verified. This is verification of
an existing released asset, not publication or certification of a new release.

## RF-063/RF-073 Native Speed Snapshot Sharing (2026-09-28 UTC)

The first maintained Live Parity run found two requests for transfer speeds
on System, at 240–343 ms intervals: the native player's initial request and
the System endpoint catalogue load outlasted the 200 ms sharing window. The
one-entry Promise cache now shares that initial speed snapshot for at most
one second. It expires at the boundary and rejects reversed/non-finite time
inputs. The product can display a cached speed snapshot for up to one second.

Two temporal regressions execute in the native test target; the Rust Web
suite passes 88 tests, strict Clippy and WASM compilation pass. A fresh actual
Chromium audit passes all 15 desktop/mobile route pairs with zero errors:
System makes exactly one speed request and stays at its unchanged 39-request
budget on both viewports. Source-bound mock evidence is retained in
`benchmarks/artifacts/20260928-rf-native-transfer-speed-request-sharing.json`.
This does not close deployed accessibility or credentialed interop scope.
A fresh retained Live Parity result is still required for RF-073.


## RF-006 Shared DHT Without a Receive Port (2026-09-28 UTC)

The shared DHT path formerly bound a second mainline receive socket and
forwarded raw datagrams from a local relay. A validated four-byte transaction
fixture confirms the old path sends replies to that relay, changing the
original peer identity. The initial two-byte transaction fixture is excluded
from this evidence because the pinned library does not accept that encoding.

Mainline now reuses the public socket for sending and accepts bounded
in-process datagrams from the gateway with the kernel-observed remote
address. It never reads the public socket itself, so the gateway remains its
single reader. No dedicated DHT receive or forwarding port is needed.
Packets are capped at the existing 2,048-byte parser MTU, queue capacity is
256, and full/closed/oversized admission never blocks. Builds without the
bundled shared transport reject shared mode rather than opening another port.

Five focused regressions verify the old relay's wrong destination, the new
actual ping/reply source and destination, equal actor/public socket ports,
absence of a dedicated endpoint, and bounded queue/packet/closed admission.
The daemon suite passes 675 tests with one separately exercised mount fixture
ignored by default. Original files are preserved under the ignored
`target/rf-shared-udp-source-backup` before editing. This closes the DHT input
identity defect; current-profile port projection and broader shared QUIC
layout are being checked separately against the single-port requirement.


## RF-006 Shared QUIC Socket Adapter (2026-09-28 UTC)

The client transport now exposes control/data constructors over an existing
Quinn socket adapter. Shared endpoints send through the public socket and
receive one classified datagram through an in-process queue; no socket bind
or relay worker is created by the adapter. The receive owner retains GRO
stride, remote address, destination IP, and ECN metadata. Independent native
send pollers preserve concurrent endpoint wakeups. Each queue is capped at
128 packets of at most 65,535 bytes, reserves capacity before copying, and
rejects malformed, full, or closed admission without blocking.

Three focused tests verify metadata, real public-port replies, admission
bounds, and both certificate-pinned ALPN handshakes on one socket. All 88
client tests and strict all-targets client Clippy pass. This is an internal
transport prerequisite: daemon startup and routing still need to adopt it
before the remaining QUIC relay ports can be removed.


## RF-006 Native Single-Port UDP Routing (2026-09-28 UTC)

Native/current startup now projects DHT, overlay UDP, and both QUIC ALPNs to
the peer listener's actual port. A prebound public socket preserves its bind
interface. Separate native overlay/obfuscation listener overrides fail with
an actionable error; an explicit matching obfuscation bind is normalized to
the shared listener. Frozen profiles retain their compatibility configuration.
The native QUIC path creates no loopback backend or per-peer forwarding
socket. Each endpoint receives bounded in-process packets through the client
adapter. The gateway is the single reader and splits GRO batches while
preserving source, destination, and ECN metadata. Existing admission caps
remain; inspection attempts are limited before Initial decryption, and a
one-second sweep releases expired pending/idle leases even without traffic.
Only a completed handshake marks the address as validated.

Actual encrypted traffic uncovered incorrect v1/v2 Initial salts and v2 key
labels in the old selector. Independent rustls-encrypted fixtures now verify
both versions and reject tampering. A protected short header can begin with
`d`, so native DHT classification validates the bounded message instead of
using that byte alone. Negotiated fixed-bit greasing is preserved. LAN-only
DHT explicitly serves local peers without external bootstrap observations.
One-shot control/data sends now await bounded cleanup in the caller's scope;
CLI exit cannot abandon a detached cleanup worker or retain server admission.

The daemon suite passes 681 tests (one separately exercised mount fixture is
ignored by default); all 88 client tests and strict daemon/client/mainline
Clippy pass. The full-controller/legacy-dispatch target compiles. Seven
shutdown harness helper tests and tooling/workflow/source gates pass. Real
concurrent certificate-pinned control/data handshakes pass with DHT enabled
and disabled, and DHT replies reach the original remote from the shared port.
Three live daemon cycles with all UDP services enabled and three with DHT
and QUIC disabled observe exactly HTTP plus one peer TCP listener and one
same-port UDP socket, clean SIGTERM exits, closed stalled clients, and TCP/UDP
rebinding. The all-enabled fixture performs measured CLI pin-discovery/pinned
sends for both ALPNs and a real DHT ping. Source/binary/harness hashes and dirty
worktree provenance are retained in
`benchmarks/artifacts/20260928-rf-native-single-port-all-enabled-proof.json`
and `benchmarks/artifacts/20260928-rf-native-single-port-dht-disabled-proof.json`.
CI retains both variants in its reproducibility artifact.

RF-006 remains open for its broader service ownership inventory. RF-001 still
needs complete shared-protocol proof; this selector admits a complete known
Initial ALPN and rejects ambiguous/incomplete selection. It does not claim
fragmented ClientHello or every legacy transport boundary is verified.


## RF-001/RF-006 Negotiated Shared QUIC Endpoint (2026-09-28 UTC)

This supersedes the prior native first-Initial ALPN-selection limitation.
Native QUIC now has one endpoint for both control/data ALPNs. Quinn owns
connection IDs and handshake reassembly; dispatch uses the TLS-negotiated
protocol after a completed handshake. No Initial decryption or ALPN sniffing
is needed in the native gateway. Multiple connections from one peer UDP
socket can use different ALPNs without selecting conflicting backend routes.
Disabled protocols fail at TLS negotiation. Frozen relay inspection remains
separate and retains the corrected v1/v2 encrypted-packet regression coverage.

Pending endpoint handshakes are capped at 128, each pending buffer at 64 KiB,
and their aggregate at 8 MiB. The single ingress queue retains its 128-packet
bound; inbound stream limits and data payload limits remain explicit. The
existing peer/prefix attempt caps, periodic session sweep, managed accept and
connection workers, certificate validation/pins, and shared public socket
remain in force.

Two new client regressions perform real certificate-verified handshakes with
large ALPN offers that fragment CRYPTO across Initial datagrams. They verify
both protocols and exact control/data payloads from the same client UDP
endpoint, and reject a disabled data ALPN. Native daemon integration separately
checks concurrent pinned control/data sends, DHT reply identity, and socket
closure/rebinding with DHT enabled and disabled. All 90 client and 681 daemon
tests pass (the separately exercised mount fixture remains ignored by
default), strict client/daemon Clippy passes, and the full-controller/legacy
compile and source/tooling/release gates pass. The static runtime guard rejects
native first-ALPN inspection and detached QUIC cleanup workers.

Three fresh all-enabled live cycles and three DHT/QUIC-disabled cycles retain
one peer TCP/UDP port, no backend/forwarding socket, completed CLI probes,
clean exits at about 0.114 seconds, closed stalled clients, and rebinding.
Exact source/binary/harness hashes and worktree provenance are retained in
`benchmarks/artifacts/20260928-rf-single-quic-endpoint-all-enabled-proof.json`
and `benchmarks/artifacts/20260928-rf-single-quic-endpoint-dht-disabled-proof.json`.
This closes the native fragmented-ALPN selection limitation; broader RF-001
TCP/legacy boundaries and RF-006 service inventory remain open.


## RF-006 Forwarding and Administrative Job Ownership (2026-09-28 UTC)

Forwarding listeners now own a joined connection set under the existing
128-rule/128-connection limits. The listener handle aborts on owner drop;
forced listener cancellation drops its child set. Bidirectional tunnel pumps
are scoped futures, so cancellation drops both pumps and their socket halves
before remote tunnel cleanup. Activity accounting uses a drop guard, avoiding
stale counters when a handler is aborted. Normal stop permits bounded remote
cleanup and joins children; manager shutdown closes admission, drains rules
concurrently, and is part of `AppState` shutdown. The standalone optional
forwarding feature retains its explicitly requested loopback listeners; no
native peer transport port is added by this lifecycle change.

Share-rescan, both maintained administrative webhook routes, script event
workers, and completed-download FTP jobs now register with daemon task
ownership. Script dispatch preserves its existing concurrent-run permits and
cannot create work after registry shutdown. The retained legacy dispatcher
receives the same script registry parameter for diagnostic compilation.
Forwarding tests now have a separate owner, preserving the full previous
suite. Originals are backed up under ignored `target/rf-forwarding-source-backup`.

Regressions exercise normal shutdown and forced cancellation during a stalled
real gateway TCP/TLS setup, closure of local/gateway sockets, reclaimed
connection permits and activity counts, listener rebinding, daemon-level
forwarding shutdown, and rejected late script/rule admission. The forwarding
filter runs 12 matching tests including unchanged existing data-path coverage;
the complete daemon suite passes 684 tests with one separately exercised
mount fixture ignored by default. Strict daemon Clippy, full-controller/legacy
all-targets compilation, and the three-cycle all-enabled shared-peer live
proof pass. Source hashes, test/check outcomes, and the retained live result
are in `benchmarks/artifacts/20260928-rf-forwarding-and-service-ownership.json`.
FTP concurrency policy, script descendant-process cleanup, remaining service
ownership, and fresh hosted service proof remain open; this is not RF-006 closure.

## RF-006 Unix Script Descendant Cancellation (2026-09-28 UTC)

Integration scripts now establish a separate Unix process group before spawn.
An owned guard kills the group when cancellation, timeout, or output collection
failure drops the running worker. The guard drops before the Tokio child, and
is disarmed immediately when waiting reaps the leader, avoiding subsequent
signaling through a reusable numeric process ID. Invalid, zero, and init IDs
are rejected. Existing output and concurrency limits remain in place.

A Linux regression runs a real shell with a live sleep child and verifies no
live descendant survives either timeout or explicit task cancellation. Its
failure cleanup holds a kernel pidfd, so assertion failure cannot accidentally
signal a subsequently reused PID. The original script tests are preserved in
a separate test owner; the pre-change source remains backed up under ignored
`target/rf-script-process-source-backup`. This covers descendants remaining in
the owned Unix process group; Windows process-tree cleanup and deliberately
detached jobs after a normally completed script are not covered. Broader
service inventory and FTP concurrency policy remain RF-006 work.

Validation: 686 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
Hashes and scope are retained in
`benchmarks/artifacts/20260928-rf-script-process-group-ownership.json`.

## RF-028 Hosted Checksum Fixture Startup Deadline (2026-09-28 UTC)

The completed Rust job `108820375569` in CI run `36388979601` failed in audit
tooling before Cargo: the first PowerShell checksum fixture exceeded its
10-second subprocess deadline. The remaining seven fixtures completed, and
all eight actual PowerShell fixtures pass locally. The fixture deadline is
now 60 seconds, retaining `subprocess.run` timeout kill/wait cleanup and all
checksum assertions. The audit-tooling gate passes. This is internal test
harness work; the production checksum verifier and package behavior are
unchanged. Fresh hosted CI completion remains required, and this failed run
is not counted as successful shared-QUIC evidence.

## RF-006 Scoped WebSocket Readers and Track Processing (2026-09-28 UTC)

Event, compatibility SignalR, and relay WebSocket readers are now scoped
futures polled alongside their connection writer. Reader completion still
drains the existing bounded frame queue; connection cancellation immediately
drops the reader without detaching a task. A regression waits until a reader
actually stalls, cancels the event/SignalR parent, joins it, and verifies the
reader was dropped. Existing framing, heartbeat, subscription, and transport
tests remain intact. Relay protocol/global-registration cancellation cleanup
is separate remaining work and is not claimed by this reader checkpoint.

Track-intent processing now uses daemon task admission and returns 503 after
shutdown rather than reporting accepted detached work. A regression verifies
that the intent stays Pending after rejected admission. Runtime source guards
reject detached reader/track workers. Originals remain backed up under ignored
`target/rf-websocket-source-backup`; no native peer port is added. Broader
RF-006 service inventory and FTP concurrency policy remain open.

Validation: 688 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
Source/log hashes and scoped outcomes are retained in
`benchmarks/artifacts/20260928-rf-websocket-and-track-ownership.json`.

## RF-006 Downloaded All-Transport Single-Port Hosted Receipt (2026-09-28 UTC)

Successful hosted Rust job in run `36390159596` at exact source `a43155de`
retained artifact `10955962868`, `ci-reproducibility-rust`. The downloaded
all-enabled and disabled transport results each contain three successful
live shutdown cycles with exactly one peer TCP listener and one UDP listener
on the same numeric port, closed client sockets, and TCP/UDP rebinding.
All-enabled probes include pinned control/data QUIC and actual DHT replies
from the public port. Source hashes were independently checked against the
exact Git commit; downloaded proof hashes match the reproducibility manifest.

The retained receipt is
`benchmarks/artifacts/20260928-rf-hosted-single-peer-port-receipt.json`.
This verifies the native shared UDP/QUIC endpoint on a clean Linux runner,
including DHT-enabled routing. The full platform matrix was still running at
receipt download; later script/WebSocket ownership changes are not covered
by this earlier job. Broader RF-006 and deployed interop remain open.

## RF-006 CLI Probe Child Cancellation (2026-09-28 UTC)

Peer accept tasks and peer/server/transfer fixture workers now use an owned
join handle. Awaiting preserves existing results and panic reporting; dropping
a probe, failing before its final join, or timing out requests child abortion.
There is no detached cleanup worker. Regressions wait for real pending TCP
listeners, then verify handle drop, timeout, and parent task cancellation
release child resources and permit rebinding. The parent cancellation case
joins its parent, and child drop notification confirms its listener was
released. Normal completion and child panic results are also covered.

The existing probe helpers and fixtures remain intact; the original sources
are backed up under ignored `target/rf-cli-probe-source-backup`. Source guards
reject raw spawns in the five maintained fixture/accept owners. This adds no
production listener or native transport port. Broader RF-006 service inventory,
relay registration cleanup, and FTP concurrency policy remain open.

Validation: 691 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
Source/log hashes and cancellation scope are retained in
`benchmarks/artifacts/20260928-rf-cli-probe-child-ownership.json`.

## RF-006 Bounded Completed-Download FTP Admission (2026-09-28 UTC)

Each daemon now owns FTP upload admission with at most 64 total admitted jobs
and four active uploads. Full admission waits before spawning a worker, so
completion events cannot create unbounded queued upload tasks. Accepted jobs
retain their admission slot through queueing and upload completion. Existing
FTP retry, connection-timeout, TLS, and transfer settings remain in effect;
this does not introduce a transfer-duration cutoff for large files.

Shutdown closes both admission and active-work gates before joining managed
workers, wakes blocked completion callers, rejects late work, and returns
held slots as cancellation drops futures. Regressions fill all 64 slots,
observe exactly four stalled active jobs, verify caller backpressure and
shutdown wakeup, confirm all accepted queued jobs execute when slots become
available, and check normal completion/closed-registry slot release. A daemon
fixture verifies shutdown ordering and late admission rejection. Original
sources remain backed up under ignored `target/rf-ftp-admission-source-backup`.
This uses no transport socket or additional native port. FTP admission policy
is implemented locally; hosted proof, relay registration cleanup, and the
remaining RF-006 service inventory are separate work.

Validation: 695 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
The full daemon suite includes the existing real FTP protocol fixtures.
Source/log hashes, admission limits, and scope are retained in
`benchmarks/artifacts/20260928-rf-ftp-upload-admission.json`.

## RF-006 Event Webhook Delivery Ownership (2026-09-28 UTC)

The remaining direct webhook spawns were real production delivery paths,
not test code. Both ordinary event delivery and compatibility delivery now
use daemon task admission under their existing delivery permits. Closed
admission drops the unstarted worker, returns its permit, and ordinary
webhook delivery persists a failed outcome and delivery statistics; the
compatibility path records rejected admission in the daemon log. Existing
webhook DNS/TLS/SSRF, retry, and concurrency boundaries remain in place.

The rejection regression covers both a full delivery pool and a closed task
registry against the in-memory database, verifying failed status, error
reason, persisted statistics, and permit recovery without sending a request.
The previous webhook test suite is preserved in a separate test owner; its
original source is backed up under ignored `target/rf-webhook-source-backup`.
Runtime guards reject detached webhook workers. Cancellation outcome
reconciliation for a delivery already running, relay registration cleanup,
and remaining RF-006 hosted service coverage are separate work. No native
peer listener or transport port is added.

An additional regression sends an actual request to an isolated stalled HTTP
fixture through an owned compatibility delivery worker, then joins daemon
registry shutdown and verifies socket closure and returned delivery capacity.
Validation: 697 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
Source/log hashes and scope are retained in
`benchmarks/artifacts/20260928-rf-webhook-delivery-ownership.json`.

## RF-006 Reserved Relay Connection Cleanup (2026-09-28 UTC)

Relay connections now reserve cleanup capacity before issuing a challenge.
There are at most 256 outstanding reservations/queued cleanup requests per
daemon. A dropped connection synchronously removes its global hub sender and
uses its reserved queue slot to request protocol deregistration, including
registered-agent/request/waiter removal. No cancellation path spawns a new
task or needs to acquire the asynchronous protocol lock. Normal connection
completion deregisters directly and releases the reservation.

One managed cleanup worker processes requests and holds a weak reference to
daemon state, avoiding a reference cycle while waiting for work. Full
reservation capacity applies backpressure. Shutdown closes reservation
admission before joined worker cancellation, then explicitly clears live
protocol connections and requests, covering queued cleanup that cannot run
once the worker is cancelled. Completed share-upload history is retained.
Original sources are backed up under ignored `target/rf-relay-cleanup-source-backup`.

Regressions cover authenticated agent removal after a contended protocol lock,
immediate hub sender removal, the 256-reservation limit, blocked admission
wakeup at shutdown, active registration removal and late reservation rejection,
retained completed share-upload history, and cancellation of a pending relay
WebSocket reader alongside event/SignalR readers. No native peer transport
port is added. Already-started webhook delivery outcome reconciliation and
fresh hosted service evidence remain separate RF-006 work.

Validation: 701 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
Source/log hashes, reservation limit, and cancellation scope are retained in
`benchmarks/artifacts/20260928-rf-relay-connection-cleanup.json`.

## RF-006 Interrupted Webhook Outcome Reconciliation (2026-09-28 UTC)

After joined delivery-worker shutdown, queued webhook records are reconciled
as failed with an explicit unknown-outcome reason. Startup performs the same
reconciliation after startup validation and before accepting work, covering
unclean interruption. The `no-start` path returns before reconciliation.
This records that local confirmation did not complete; it does not assert
that the remote endpoint received nothing and does not send another request.
Confirmed success/failure records, attempts, response data, and timestamps
are retained. A partial index contains only queued records, avoiding a scan
through archived delivery outcomes on shutdown/restart.

A real SQLite reopen regression verifies interrupted record recovery,
idempotence, preserved terminal outcomes/data/attempts, and use of the queued
index in the query plan. A daemon-state regression verifies worker cancellation
before reconciliation and the persisted shutdown reason. Original sources
are backed up under ignored `target/rf-webhook-reconciliation-source-backup`.
This closes the local cancelled-delivery persistence gap identified by the
webhook ownership checkpoint. Fresh hosted service evidence and the complete
RF-006 audit remain separate work; no native transport port is added.

Validation: 703 daemon tests pass with one default-ignored mount fixture;
strict daemon Clippy and full-controller/legacy all-targets compilation pass.
Source/log hashes and recovery scope are retained in
`benchmarks/artifacts/20260928-rf-webhook-outcome-reconciliation.json`.

## RF-006 Completed Hosted Single-Port Matrix (2026-09-28 UTC)

CI run `36390159596` at `a43155de` completed successfully across all eleven
jobs: Rust, production web, Linux AArch64, Linux/musl x64/arm64, macOS
x64/arm64, Windows x64, and package/deployment surfaces. The exact GitHub
job/step receipt and link/hash to the downloaded all-enabled single-port
artifact are retained in
`benchmarks/artifacts/20260928-rf-hosted-a431-complete-matrix.json`.
This is a definitive successful matrix for the shared-QUIC/DHT and forwarding
source; subsequent lifecycle commits require their own hosted completion.

## RF-066 E2E Fixture Repository Discovery (2026-09-28 UTC)

Optional media discovery assumed the current directory was exactly three
levels below the repository, so normal launches from the repo root or `web`
could inspect the wrong fixture tree. The node harness also had a separate
cwd assumption. Both now use one ancestor search for the workspace and daemon
Cargo manifests. Absolute fixture directories validate independently of cwd.
Media availability requires nonempty regular files, excluding empty files or
directories. The fetch helper passes the script path through `execFile`
arguments instead of interpolating a shell command.

Three actual Playwright utility regressions verify discovery from five cwd
locations, absolute fixture paths, checksum validation, rejected missing/empty/
directory media, and an explicit failure outside a checkout. Their temporary
presence stubs are not encoded playback media. The CI Rust job now executes
these utility tests without launching nodes or requiring browser installation.
The complete web suite passes 915 tests; the seven-file static manifest and
its corruption regressions pass. Original sources remain backed up under
ignored `target/rf-fixture-path-source-backup`.

RF-066 remains open: the committed media manifest declares no downloadable
audio/video assets, and the optional playback/sharing matrix still needs
actual supplied or explicitly declared media. The previous wording implying
Linux codec unavailability was unsupported; missing declarations and the cwd
bug are independently established causes. This checkpoint does not count
presence stubs as playback proof or alter native peer transport ports.

## RF-006/RF-066 Native E2E Peer Port and Child Ownership (2026-09-28 UTC)

The real-node E2E harness no longer allocates separate DHT or overlay ports.
Its TCP listener, overlay bind, and DHT configuration use the same loopback
peer port, matching the native transport implementation. HTTP remains the
existing application API service. A shared configuration regression checks
all four peer settings and rejects invalid ports.

Nodes enter the harness registry before startup; failed starts call stop
before removal. Stop records cancellation before further process launch,
waits for a live child's close event, and clears its force-kill timer. An
actual Node child regression verifies termination and repeated stop; a
failed-start regression verifies early registration and cleanup. Together
with fixture discovery, six Playwright utility tests pass without media or
browser dependencies. CI runs both utility suites. Original harness sources
are backed up under ignored `target/rf-e2e-single-port-source-backup`.

This checkpoint does not claim cancellation of shared build commands or
complete RF-006/RF-066 acceptance. Those ownership and playback checks remain
part of the continuing program.

## RF-066 Verified Optional Media Downloads (2026-09-28 UTC)

The optional manifest now declares the genuine Sintel stereo movie and Open
Goldberg Aria recording, with license/attribution/source metadata, exact byte
counts, and SHA-256 pins. Independently published source sizes and SHA-1 hashes
were checked before curating the SHA-256 pins. Both assets were subsequently
downloaded through the hardened helper and verified again before installation.
FFprobe identifies real H.264/AAC movie streams and a Vorbis audio stream.
No binary is committed; the seven tracked static files retain their hashes.

The Python downloader enforces manifest/count/byte budgets, safe relative paths
without symlinks, HTTPS including redirects, 64 KiB reads, socket timeouts,
and a per-file deadline. It writes to a temporary file, checks expected size
and hash, and installs atomically; failed partials are removed. Invalid existing
caches are preserved and rejected. Fetching never regenerates observed hashes.
The shell entry point invokes this single implementation. Static corruption
tests now copy only the seven manifest files, avoiding optional media copies.

Eight offline downloader regressions, six Playwright utility checks, static
manifest/corruption checks, and the full audit-tooling gate pass. Evidence and
source hashes are retained in
`benchmarks/artifacts/20260928-rf-pinned-media-fetch.json`. Movie sharing and
streaming gates now require Sintel alone, matching their actual dependency.
The real nine-case sharing/streaming run is pending at this checkpoint;
RF-066 remains open until its actual acceptance evidence is complete.

## RF-066 Real Invite/Contact Browser Flow (2026-09-28 UTC)

The first actual optional-media run exposed product/test contract failures:
four cases failed and five serial cases did not run. The contact UI supplied
an invite link but omitted the username required by the current contact API.
The native profile/invite generator also read the deliberately redacted
session snapshot, producing a masked contact address. Native profile/invite
display names now use the runtime/configured identity resolver already used
by capability descriptors; session summary redaction remains intact.

The web adapter decodes bounded version-1 UTF-8 invite payloads, rejects invalid
or expired links before posting, and sends the extracted address-book username.
Username-only contact records display as named, unverified contacts. Imports
do not authenticate profiles or grant transport/share permissions. The real
`invite_add_friend` browser case passes against three rebuilt native nodes,
each using one loopback peer TCP/UDP port, with external login disabled.

Validation passes 704 daemon tests (one ignored mount fixture), 930 web tests,
strict Clippy, and full-controller/legacy all-targets compilation. Source/log
hashes and scope are retained in
`benchmarks/artifacts/20260928-rf-native-profile-contact-invite.json`.
Other playback cases still use stale group setup, outgoing-share discovery,
or raw query tokens instead of the current stream-ticket exchange. These
failures are established by a real run and keep RF-066 open.

## RF-006 Later Completed Hosted Lifecycle Matrix (2026-09-28 UTC)

Run `36391848047` at exact commit `59ddda17` completed successfully across all
eleven CI jobs, including Windows archive smoke and package/deployment surfaces.
The retained job/step receipt is
`benchmarks/artifacts/20260928-rf-hosted-59dd-complete-matrix.json`. It covers
the script process-group and scoped WebSocket reader ownership checkpoints.
Later lifecycle/media/profile changes still require their own hosted result.

## RF-066 Genuine Ticketed Movie Playback (2026-09-28 UTC)

Five sharing cases and two streaming cases now pass with the genuine pinned
Sintel movie. The tests select its exact filename/content hash rather than a
poster, discover incoming grants through the incoming endpoint, send bearer
tokens only in `X-Share-Token`, and exchange them for short-lived stream tickets.
Actual movie responses satisfy start/offset/suffix ranges and the 77,410,288-byte
length. Missing/invalid tickets and forbidden query share tokens are rejected.
The recipient UI opens the ticketed movie in a browser popup; the regression
requires readyState at least 2, nonzero intrinsic video width, a decoded frame
counter greater than zero, and no media error. Linux codec unavailability is
not an established blocker for this actual browser run.

Primary movie file streams now label MP4/WebM/Matroska with video MIME types;
the frozen audio-preview mapping and audio extensions retain their contracts.
The harness no longer treats ignored build directory mtimes as source changes,
and future local fallback builds use locked debug Cargo. A timestamp regression
passes with the existing six utility checks. Optional pack discovery tracks
the declared movie/audio assets and passes the supported fixture-root setting.

705 daemon tests, strict Clippy, full-controller/legacy all-targets compilation,
seven Playwright utility tests, and static manifest/corruption checks pass.
Source/log/binary hashes and explicit remaining scope are retained in
`benchmarks/artifacts/20260928-rf-ticketed-movie-playback.json`. Original E2E
sources remain in Git and the ignored `target/rf-ticketed-media-source-backup`.
The recipient-download and per-grant concurrency cases remain open; their
assertions are not weakened or reported as passed.

## RF-006 Completed Hosted Later Ownership Matrix (2026-09-28 UTC)

Run `36395807187` at clean exact commit `cf37f4bd` completed all eleven CI jobs
successfully. Downloaded Rust artifact `10959255675` contains three disabled
and three all-enabled single-port shutdown cycles plus reproducibility metadata.
Proof hashes match the artifact manifest, and every transport source, lockfile,
and configuration hash independently matches `git show` at that source.
The full job/step receipt, metadata, and both actual proofs are retained in
`benchmarks/artifacts/20260928-rf-hosted-cf37-complete-matrix.json`. This covers
the later CLI, FTP, webhook, relay, and fixture-discovery changes at that commit;
subsequent harness/media/profile/MIME changes still need their own hosted result.
The remaining production lifecycle-command inventory is separate work.

## RF-006 Joined Lifecycle Command Owner (2026-09-28 UTC)

The crate root still spawned a detached task to delay shutdown/restart commands.
That path now lives in `lifecycle_controller.rs` and enters the daemon's managed
registry. The existing 100 ms HTTP response-flush delay, command channel, and
bounded best-effort Soulseek disconnect behavior are preserved. Joined shutdown
releases delayed or channel-blocked senders; closed admission drops later work.
The crate root contains no raw `tokio::spawn` after this extraction.

Two regressions verify normal delayed command delivery, canceled delay, a full
command queue, released sender ownership, and rejected scheduling after registry
shutdown. Validation passes 707 daemon tests (one ignored mount fixture), strict
Clippy, full-controller/legacy all-targets compilation, module hygiene, and the
runtime ownership guard. Three real all-enabled single-peer-port shutdown cycles
pass with actual DHT/control-QUIC/data-QUIC activity, client closure, and socket
reuse. Source hashes and the actual proof are retained in
`benchmarks/artifacts/20260928-rf-lifecycle-command-owner.json`.
Original root/lifecycle test sources remain in Git and the ignored
`target/rf-lifecycle-command-source-backup`. Complete blocking-work inventory and
fresh hosted completion remain separate RF-006 acceptance work.

## RF-010 Hosted AUR Smoke False Positive (2026-09-28 UTC)

The clean `cf37f4bd` package log contains an AUR success marker, but inspection
found that the Docker fallback invokes `bash -s` with a heredoc without keeping
stdin open (`docker run -i`). The script can therefore be skipped while the
container exits successfully. That marker is not accepted as a real makepkg
receipt; RF-010 remains open pending corrected container execution, working
directory verification, and an actual fresh clean-runner result.

## RF-010 Executed AUR Source/Prepare Smoke (2026-09-28 UTC)

The Docker fallback now opens stdin with `-i`, changes into the copied package
directory before invoking makepkg, and records the container ID for cleanup
on success, failure, or interruption. Cleanup can remove only that validated
ID. Both host and container paths use `--nobuild`; the previous simultaneous
`--verifysource` flag exited before extraction and any `prepare()` function.
Actual host makepkg now verifies and extracts both source and binary PKGBUILDs.

Two offline regressions execute the actual heredoc Bash script with explicit
Docker/package-manager doubles. They verify both makepkg calls, correct working
directory, enabled extraction/preparation, suppression of false success on a
failed binary package, and exact owned-container cleanup. They are part of the
full audit-tooling gate, which passes. Source/log hashes and the doubles' limited
scope are retained in
`benchmarks/artifacts/20260928-rf-aur-container-smoke-execution.json`.
Original smoke source is backed up in ignored `target/rf-aur-container-source-backup`.
Fresh actual clean-runner Docker completion is still required for RF-010;
the previous marker does not establish that completion.

## RF-011/RF-035 Native Windows Smoke and Actual Workflow Lint (2026-09-28 UTC)

Explicit Windows Smoke run `36401784119` at `9b8e52f2` failed before Rust tests:
the memory-guard regression compared native Node cwd `D:\a\slskr\slskr` with
Git Bash's POSIX path. It now obtains the expected native cwd outside the guard
and compares it with native cwd inside the guard. The actual local guard
regression passes; this is path representation correction, not a bypass.

Windows Smoke now prefers the pinned rustup Cargo binary, restores Rust cache,
and selects full Perl for vendored OpenSSL through the same helper as main CI.
The already used main-matrix Perl implementation is extracted unchanged to
`scripts/select-windows-perl.ps1`; PowerShell syntax validation passes.
CI installed actionlint but did not execute it. An explicit actionlint step
now runs, and policy requires that execution and the shared Perl helper in
both workflows. Pinned actionlint v1.7.12 and the full existing shellcheck gate
pass locally. No Rust wrapper, virtual-memory cap, or test-thread override is
added to Rust commands.

The failed hosted receipt, source/log hashes, and local proof scope are retained
in `benchmarks/artifacts/20260928-rf-windows-smoke-native-cwd.json`; originals
remain in ignored `target/rf-windows-smoke-source-backup`. Fresh actual Windows
Smoke and hosted actionlint completion are still required before closure.

## RF-006/RF-066 Owned E2E Build Commands (2026-09-28 UTC)

Shared Cargo/npm promises previously had no deadline or cancellation owner.
`BuildOwner.ts` now owns commands with a ten-minute deadline, POSIX process-group
termination (TERM then KILL after five seconds), and Windows PID-scoped
`taskkill /T /F` on cancellation. Command promises settle after child close. Numeric process-group targets are
never signaled after Node reaps their leader; independently detached or orphaned
descendants are not claimed as joined by this harness.
Each node owns a build cancellation signal and joins its pending build waits
before stop returns. Sharing is limited to the running build: one canceled
consumer leaves other consumers intact, while the last consumer cancels and
waits for cleanup. A subsequent stale build can run again instead of reusing
a permanently resolved static promise. Cargo still runs directly with the
pinned workspace configuration; no compiler shim or memory cap is introduced.

Eleven fixture/lifecycle utility regressions pass, including real child close,
real POSIX descendant cancellation/reaping, command deadline/failure, shared
consumer cancellation, retry, and build freshness. A targeted TypeScript check
passes. ESLint does not cover these TypeScript harness files, so its ignored-file
output is not counted as lint proof. Actual Windows command-tree cleanup and
fresh hosted utility completion remain required; the local POSIX receipt does
not establish those. Source originals remain in ignored
`target/rf-e2e-command-owner-source-backup` and Git history.

## RF-006 External Visualizer Child Ownership (2026-09-28 UTC)

The blocking-work inventory found an unbounded detached `child.wait()` for
external visualizers in both production dispatch and the opt-in legacy oracle.
A dedicated child registry now owns Tokio children, keeps the existing four
semaphore permits, reaps exited children every second in a managed supervisor,
and closes admission under the same lock used to launch. Shutdown kills and
waits for its direct children before aborting managed tasks; the reap wait has
a five-second deadline with logged failure and kill-on-drop fallback. No detached
blocking wait remains in either visualizer dispatcher. Independent descendants
are not claimed as owned or reaped by this direct-child registry.

Original sources remain in ignored `target/rf-visualizer-owner-source-backup`
and Git. Full-controller semaphore saturation fixtures retain their original
capacity assertions. The remaining filesystem blocking-worker inventory is
separate: bounded byte counts and awaited request futures do not prove that
already running blocking work is joined when those requests are canceled.

## RF-035 Hosted Web Lint Correction (2026-09-28 UTC)

CI run `36402572470` at `e7d0dd6a` passed its new actionlint step but the Rust job
failed at web lint. The invite decoder's ASCII-control regular expression
violated `no-control-regex`. Equivalent character-code validation now preserves
rejection of every code from 0 through 31 and 127, with explicit regressions for
all 33 codes. This is an internal validation correction; invite permissions,
verification status, payload bounds, username bounds, and expiry checks retain
their existing behavior. Fresh hosted completion remains required.

Visualiser validation checkpoint: 710 daemon tests pass (one ignored mounted
fixture), strict all-targets Clippy passes, and full-controller/legacy all-targets
compilation passes. Three actual all-enabled single-peer-port shutdown cycles
pass on the rebuilt debug daemon. Web lint and all 47 invite tests pass. Actual
proof and source/log hashes are retained in
`benchmarks/artifacts/20260928-rf-visualizer-child-owner.json`. The eight remaining
production blocking-worker call sites and precise cancellation limits are in
`docs/dev/blocking-work-ownership.md`; RF-006 is not marked complete.

## RF-011 Completed Named Windows Smoke (2026-09-28 UTC)

Windows Smoke run `36402974583` completed successfully at `e7d0dd6a`. The
native-cwd memory regression, pinned Rust setup, shared full-Perl helper, Rust
format gate, and actual workspace tests all passed on Windows. The full
job/step receipt and source/log hashes are retained in
`benchmarks/artifacts/20260928-rf-named-windows-smoke-success.json`. This validates
the named workflow and its setup fixes at that exact source; later runtime/share
changes continue to require their own complete hosted matrix.

## RF-066 Enforced Owner Share Stream Concurrency (2026-09-28 UTC)

Owner grants previously ignored `maxConcurrentStreams` even though the E2E
case expected it. Explicit limits from 1 through 64 now persist with the grant
through an idempotent SQLite column migration. Existing grants default to no
explicit limit, preserving their response contract and behavior; null clears
an explicit limit. Limit-only updates retain the original permissions, and
persistence failure restores only an unchanged transaction candidate, including
its stream policy. Invalid persisted limits discard the grant rather than
silently turning it into an unlimited grant.

A bounded grant-count registry holds leases through ticketed HTTP responses.
It tracks unlimited ticketed responses too, so setting a limit while a response
is already active counts that response. Lowered limits deny new work until
capacity becomes available; completion, error, and cancellation release leases.
Capacity is scoped to the grant, and shutdown closes registry admission.
No native transport listener or peer port is added.

The repaired concurrency E2E creates an actual owner grant and exchanges its
header bearer token for a content-bound ticket. A real paused socket holds the
77,410,288-byte Sintel response open: the next GET returns 429, and after socket
close a subsequent range GET returns 206. No query bearer token, ignored
share-group payload, or SPA `/__routes` response is used. All eight real sharing
and streaming cases pass together, including decoded movie frames. The ninth,
recipient backfill, remains open; its owner-side metadata receipt does not
write the recipient's downloads directory.

Validation passes 716 daemon tests (one ignored mounted fixture), strict
all-targets Clippy, full-controller/legacy compilation, targeted E2E TypeScript,
and three actual all-enabled shared-peer-port shutdown cycles. Old E2E helper
null handling and URL predicates were corrected without weakening assertions.
The run also exposed harness log FileHandle cleanup and late contact-response
registration; those remain follow-up ownership/test work. Actual proof and
source/log hashes are retained in
`benchmarks/artifacts/20260928-rf-owner-share-stream-concurrency.json`.
Source originals remain in ignored `target/rf-share-stream-limit-source-backup`
and Git. Fresh hosted completion remains separate acceptance work.

## RF-006/RF-066 Joined Harness Log Ownership (2026-09-28 UTC)

The actual eight-case media run exposed log FileHandles being closed only by
garbage collection. `NodeProcessLogs.ts` now owns both handles and backpressure
pipelines, joins writes, and closes handles before app-directory removal.
Normal stop and failed launch use the same cleanup path. Startup work is tracked
and joined, spawn errors reject startup instead of throwing from an event
callback, diagnostic writes are joined, and stop interrupts TCP/health polling.
In-memory log tails are capped at one MiB per stream while complete output is
still written to the artifact files. `ss` diagnostics have a two-second timeout
and 256 KiB output cap. No process, listener, or peer port is added.

Thirteen utility regressions pass, including exact real child stdout/stderr
capture, both closed file descriptors, unlaunched handles, actual failed spawn,
startup cleanup, command ownership, and native child close. Both sharing and
streaming specs pass the targeted TypeScript check. The contacts response wait
now starts before navigation and asserts the actual successful JSON array;
its old swallowed timeout is removed. All eight actual media cases pass again
in 29.6 seconds without FileHandle garbage-collection warnings or missing-contact
response errors. Recipient backfill and the eight blocking-worker shutdown
joins remain open.

The corrupt persisted-limit regression now uses a distinct recipient, proving
invalid policy rejection independently of duplicate-grant filtering. That
focused migration/persistence/reset/rollback regression passes again. Actual
logs and source hashes are retained in
`benchmarks/artifacts/20260928-rf-harness-log-owner.json`. Originals remain in
ignored `target/rf-harness-log-owner-source-backup` and Git history.

## RF-006/RF-010/RF-035 Completed Hosted Ownership and Packaging Matrix (2026-09-28 UTC)

CI `36405007921` at exact clean commit `7a064d37` completed all eleven jobs.
Downloaded Rust artifact `10962112931` contains three disabled and three
all-enabled single-peer-port shutdown cycles. Proof hashes match metadata;
transport source, seven lockfile and three configuration hashes independently
match `git show` at that source. The all-enabled cycles use one peer TCP
listener and UDP on the same numeric port, close clients and rebind sockets.

The actual package job `108882375286` downloaded the Arch Linux Docker image
and ran both source and binary makepkg invocations through validation,
extraction and sources-ready. This replaces the earlier stdin false positive.
It proves source/prepare smoke, not full package compilation or publication.
The hosted workflow lint step executed actionlint successfully. Complete
job/step receipts, metadata, both transport proofs, package-log hash and actual
execution markers are retained in
`benchmarks/artifacts/20260928-rf-hosted-7a-complete-matrix.json`. Later stream
limit and harness log changes require their own hosted matrix. RF-006 blocking
filesystem cancellation and external acceptance items remain open.

## RF-001 Single Public Peer-Port Advertisement (2026-09-28 UTC)

Native/current already rejects a dedicated obfuscation bind, but its default
obfuscated advertised port used the local listen port rather than the public
regular advertised port. A configured NAT mapping could therefore direct
obfuscated peers to the wrong port. Both now advertise the same public port;
conflicting explicit obfuscated advertisement settings fail configuration
validation. No new listener is introduced. Frozen compatibility and remote
peers' advertised transport ports remain separate contracts.

Four current-native configuration regressions, 717 daemon tests (one ignored
mounted-filesystem fixture), strict Clippy and three actual all-enabled
single-peer-port shutdown cycles pass. The harness uses the platform temporary
directory instead of hard-coded `/tmp`; its ten lifecycle tests and targeted
TypeScript check pass locally. Sources, log hashes and the complete real socket
proof are retained in
`benchmarks/artifacts/20260928-rf-shared-peer-advertisement.json`. Originals
remain in Git and ignored `target/rf-shared-advertisement-source-backup`. This
fix does not resolve the previously retained dual-valid legacy wire ambiguity.

## RF-006 Owned Diagnostic Child and Temporary Stream Cleanup (2026-09-28 UTC)

The Linux dump path returned directly on `try_wait()` error without killing
or reaping the child or resetting its ptrace authorization. `core_dump_process.rs`
now owns the child, process group, permission and partial output. Every failure
return and unwind goes through cleanup; a reaped leader immediately disarms
numeric process-group targeting. Successful output is explicitly retained.
Integration scripts and dumps reuse the same process-group guard, with safe
standard-library process-group creation and no unsafe code.

Dump admission permits one blocking worker and stays inside that worker after
HTTP requester cancellation. This also serializes Linux's process-wide ptrace
authorization. The existing 60-second child deadline remains. Temporary local
stream files now live in `local_stream_file.rs`; an owned cleanup path survives
cloned consumers and removes abandoned output on final drop. It covers returned
dump, mesh-preview and relay files without changing their response contracts.
Cleanup failures are logged; mounted filesystem calls still have no finite
completion proof, and running blocking workers remain outside daemon registry
shutdown joins. RF-006 is not closed by these changes.

Validation for the diagnostic/stream ownership batch passes 722 daemon tests
(one ignored mounted-filesystem fixture), strict all-target Clippy and
full-controller/legacy all-target compilation. Three freshly rebuilt actual
all-enabled single-peer-port shutdown cycles pass. Five new regressions cover
actual child reap/output ownership, cancellation retaining the blocking-worker
permit and last-consumer temporary-file cleanup. They do not invoke real gcore
or grant ptrace permission. Source/log hashes and the complete transport proof
are retained in
`benchmarks/artifacts/20260928-rf-diagnostic-child-stream-ownership.json`.
Original sources are retained in Git and ignored
`target/rf-core-dump-owner-source-backup`.

## RF-001 Shared Listen-Port Alias and RF-006 Script Wait Ownership (2026-09-28 UTC)

A native/current obfuscation listen-port alias equal to the regular physical
bind was accepted but retained a nonzero obfuscation listen-port projection.
VPN synchronization consequently did not recognize that listener as shared.
The accepted alias now projects the zero shared-listener sentinel; its physical
port does not override the common public advertised port under a NAT mapping.
An explicit conflicting public obfuscated advertisement remains rejected.
The existing mapped-port regression now covers the accepted physical alias.

Integration script cleanup now disarms its process-group guard only after a
successful wait result. A wait error returns through the still-owned group
cleanup path. There is no await between successful reaping and disarming.

At the time this evidence entry was first written, RF-066 recipient backfill
was unimplemented and the HTTP owner endpoint only acknowledged the request.
The later RF-066 continuation entry at the end of this plan records the local
implementation and acceptance proof. The retained design uses the authenticated,
certificate-pinned mesh service on the shared native peer TCP port, with
grant/recipient/permission checks, bounded confined writes, and exact
size/SHA-256 verification. No dedicated listener or global HTTP network-filter
exception is planned.

The shared-alias/script-wait batch passes 722 daemon tests (one ignored mount
fixture), strict all-target Clippy, full-controller/legacy all-target compile
and three newly rebuilt all-enabled single-peer-port shutdown cycles. The
public-port alias regression and actual transport proof, with source/log hashes,
are retained in
`benchmarks/artifacts/20260928-rf-shared-peer-alias-wait-ownership.json`.
Original sources remain in Git and ignored `target/rf-shared-alias-source-backup`.
The projection check is not deployed VPN proof, and no kernel wait error is
injected by the existing actual script cancellation tests.

## RF-066 Verified Recipient Mesh Backfill (2026-09-28 UTC)

Incoming-share backfill now retrieves the owner manifest and bounded file
ranges through the authenticated, certificate-pinned `MeshContent` service.
The recipient selects the owner from its configured trusted mesh peers; the
announced HTTP endpoint does not control the network destination. The owner
revalidates the share token, direct recipient or current group membership, and
download permission for the manifest and each range. Grant IDs, access tokens,
and content IDs with Unicode control characters are rejected at that mesh
boundary; a regression covers C0, DEL, and NEL values in those fields.
Native/current profiles only are supported. Both ends bound item counts and
total bytes; the recipient
stages private temporary files, verifies exact size and SHA-256, then publishes
without overwriting. Range calls are paced below the existing per-connection
overlay rate limit and use the existing shared native peer port. The Web button
calls the recipient's authenticated local API, with no external HTTP fetch or
network-filter exception.

The focused full-controller Rust integration passes with distinct owner and
recipient identities. The real three-peer Playwright E2E passes the invite,
group membership, collection announcement, manifest, and recipient backfill
cases. The backfill downloads the 399,906-byte fixture and checks its bytes and
SHA-256 (`2e93caf3f954e8e8457d9846ad7756f74ccf192dab77b7247d48ba134a8e2c1b`)
on disk. The focused Web collection tests pass (15 tests); changed-file Rust
formatting and active-plan freshness pass. This proves local behavior over the
one shared native peer port; focused owner per-grant admission is also verified locally. Fresh hosted or deployed-device evidence remains open under RF-066 and the associated live-evidence rows. A focused full-controller regression holds a stream lease from the real ticket-admission helper, rejects a second ticket under the same grant's limit of one, allows another grant concurrently, and verifies capacity recovery after release. Live socket saturation and deployed-peer evidence remain open.

## RF-006 Blocking Work Inventory Checkpoint (2026-09-28 UTC)

`docs/dev/blocking-work-ownership.md` classifies all nine production
`spawn_blocking` call sites, including their bounds, cancellation behavior, and
shutdown gaps. The inventory is complete; joined daemon shutdown for a blocking
filesystem operation already inside a stalled mounted I/O remains unproved and
is not represented as complete. Windows Smoke run
[36462085991](https://github.com/snapetech/slskr/actions/runs/36462085991)
passed its Rust, WASM, and Web checks on `46854887`, before the runtime teardown
cap was added. Fresh hosted proof for that policy is pending. The GitLab mirror
remains blocked by its corrupt existing object recorded under RF-008.

## RF-073 Live Parity Smoke Isolation (2026-09-28 UTC)

The manual Live Parity run
[36456533118](https://github.com/snapetech/slskr/actions/runs/36456533118)
failed in the `slskd-api` import after pip warned that the pinned package's
target directories already existed. A clean reproduction showed that an old
empty `slskd_api` namespace directory shadows the real package when it is
installed into a restored Cargo target cache. The compatibility smoke now
installs its pinned dependency into the run's unique temporary state directory.

The smoke clears a previous API summary before each run so failure cannot upload
stale success evidence. It reads no per-user configuration and uses a private
temporary state directory that is deleted at exit. Generated TLS and mesh key material is
therefore outside the retained artifact tree. The workflow retains only the
smoke's passing API-call summary, rather than the target subdirectory. Its HTTPS
listener is disabled because the compatibility calls use the authenticated
loopback HTTP API. The peer, mesh, and DHT port settings project onto one
dynamically selected loopback peer port; DHT rendezvous is disabled for this
offline API smoke. The local run
passes all 91 public `slskd-api` calls. Manual Live Parity run
[36464045259](https://github.com/snapetech/slskr/actions/runs/36464045259) on
`c02c77bc` passed the UI audit and smoke; the retained
[`live-parity-36464045259` artifact](https://github.com/snapetech/slskr/actions/runs/36464045259/artifacts/10988987235)
contains the 91-call summary. The credentialed public interop job remains
skipped because its repository secret is absent; that evidence remains open under
RF-072.

The stale-report regression seeded an older summary in the workflow artifact
directory, then ran without the API token. The smoke exited with its expected
configuration error and removed the old summary before exiting. A subsequent
authenticated local run rewrote the report after all 91 API calls passed; its
daemon and temporary state were cleaned up.

## RF-063 Hosted Wishlist Request Budget (2026-09-28 UTC)

Live Parity run
[36460398465](https://github.com/snapetech/slskr/actions/runs/36460398465)
recorded all 30 route/viewport views before failing one mock-audit budget. The
wishlist view made nine startup requests; its exercised row action issued the
search POST and then refreshed the wishlist with a GET, for 11 total. The
repeated GET interval was 812 ms, above the 200 ms cadence floor. The route's
budget is now 11 to account for that one action and refresh. Replaying the exact
retained Web build locally passes all 30 views with no audit errors. Hosted run
[36464045259](https://github.com/snapetech/slskr/actions/runs/36464045259) then
passed all 30 views and retained the audit in
[`live-parity-36464045259`](https://github.com/snapetech/slskr/actions/runs/36464045259/artifacts/10988987235).
Deployed accessibility remains open.

## RF-011 Windows Smoke Current-Source Proof (2026-09-28 UTC)

Windows Smoke run
[36462085991](https://github.com/snapetech/slskr/actions/runs/36462085991)
completed successfully on `46854887`. Its Rust tests, WASM check, and Web build
all passed on the Windows runner, replacing the older `e7d0dd6a` evidence.

## RF-006 Bounded Blocking Worker Runtime Teardown (2026-09-28 UTC)

After managed async shutdown returns, `run()` now consumes the Tokio runtime
with a five-second blocking-work deadline and logs when that deadline is reached.
A focused regression holds a real `spawn_blocking` closure, confirms runtime
teardown returns at the configured deadline, then releases the worker and
observes it complete. This prevents ordinary blocked file work from holding the
daemon's runtime teardown open indefinitely. The log and design explicitly do
not claim to join an operating-system call stuck in uninterruptible kernel I/O;
Windows Smoke run
[36466395263](https://github.com/snapetech/slskr/actions/runs/36466395263) on
`d4d185cc` passed the exact bounded-shutdown regression in its 694-test
Rust run. GitHub CI run
[36467270728](https://github.com/snapetech/slskr/actions/runs/36467270728) on
`6e9a6c00` passed the full locked Rust workspace tests, including the bounded
shutdown regression, then passed the shared peer-gateway shutdown proof and
reproducibility artifact collection. Its Linux AArch64 job passed; the other
native platform jobs remain active or queued. Later commits only changed plan
or inventory documentation.

## RF-001 Shared-Port Ambiguity Policy (2026-09-28 UTC)

The user selected the single-port policy. The shared demultiplexer now has an
explicit precedence rule: it rejects competing known initialization forms and
unsafe nested interpretations, prefers a fully recognized init over an opaque
unknown extension candidate for peer interoperability, rejects unknown
alternatives that consume conflicting stream lengths, and preserves subsequent
bytes for an unambiguous unknown frame. Existing peer vectors fail under strict
unknown-versus-known rejection because the extension payload is opaque. The
listener suite's 38 tests cover these cases while all peer traffic stays on the
shared port; no wire marker or dedicated listener is introduced.


## RF-011 Windows Formatter Line-Ending Check (2026-09-28 UTC)

Windows Smoke run
[36465244591](https://github.com/snapetech/slskr/actions/runs/36465244591)
checked `08f76d7e` and stopped before Rust tests. The formatter diff showed
identical Rust text with CRLF-only differences from the Windows checkout. The
changed-file formatter now normalizes trailing CR on both source and rustfmt
output before comparing. A synthetic CRLF copy passes the local gate alongside
the ordinary LF source. Windows Smoke run
[36466395263](https://github.com/snapetech/slskr/actions/runs/36466395263) on
`d4d185cc` subsequently passed its Rust tests, WASM check, and Web build.

## RF-059 kspls0 Temporary Deployed Route Audit (2026-09-28 UTC)

The current source and Web bundle were served temporarily from kspls0 on the
existing loopback HTTP slot and the shared peer/mesh/DHT port. The isolated
instance skipped share scanning and disabled transfers; no host media or
slskd state was mounted writable. Eight live desktop/mobile route views loaded
with HTTP 200, all 178 observed API responses were 200, and no tested controls
overlapped. The strict browser audit did report four blocked-inline-style CSP
console errors on System and Integrations; they remain open for follow-up. The
container, source revision, route counts, CSP finding, and restoration record
are retained in
[`20260928-rf059-kspls0-deployed-route-audit.md`](../../benchmarks/artifacts/20260928-rf059-kspls0-deployed-route-audit.md).
RF-059 remains in progress pending device-performance evidence and resolution
of the deployed CSP finding.

## RF-059 CSP Nonce Metadata Follow-up (2026-09-28 UTC)

The kspls0 audit exposed four blocked inline-style CSP errors on System and
Integrations. Source review found that static HTML always received a per-request
nonce in its CSP header, while the matching `csp-nonce` meta tag was injected
only when runtime-profile disclosure was enabled. The React code editor reads
that meta tag to nonce its dynamic style elements. Static HTML now receives the
nonce metadata independently; `slskr-runtime-profile` disclosure remains
opt-in. The focused `web_static::tests::csp_nonce_is_exposed_without_disclosing_runtime_profile`
test passes. The correction has not been deployed to kspls0 for a repeat audit,
so the original browser finding remains open pending deployed revalidation.

## RF-008/RF-067 GitLab Post-Receive Check (2026-09-28 UTC)

The repaired GitLab accepted pushes through `4cc18c26`, and its internal
`post_receive` endpoint returned HTTP 200 for the latest push at 22:39 UTC.
Direct pipeline-table inspection
still shows no run newer than pipeline #81 from 2026-05-17, despite the project
having CI enabled and using the repository's default `.gitlab-ci.yml` path. The
configured GitLab MCP token returns 401. Post-receive worker lease warnings
were observed, but their relationship to pipeline scheduling is unproven. The
sanitized service evidence is in
[`20260928-gitlab-postreceive-pipeline-status.md`](../../benchmarks/artifacts/20260928-gitlab-postreceive-pipeline-status.md).
RF-008 and RF-067 remain open pending a post-repair pipeline run.

## RF-072 Live Account Preflight (2026-09-28 UTC)

The credentialed matrix previously checked only `SLSKR_TEST_ACCOUNT_COUNT`
entries (default four), then used accounts 5 and 6 for peer and social probes;
VPN mode also defaults login probes to accounts 5–8. It resolved the public
Soulseek hostname before discovering missing credentials. The runner now
validates the unique union of its base account range and selected probe
indices, rejects invalid/missing accounts before DNS lookup, and defaults the
base range to six. A local regression covers missing account 5 in the ordinary
matrix, missing account 6 at the default count, and missing account 7 in VPN
mode. Both CI workflows run this regression. The credentialed local and hosted
matrix now passes; the hosted receipt is recorded below.

## RF-072 Credentialed Live Parity (2026-09-28 UTC)

Eight unique account credentials from the protected local credential files
passed the preflight. The local matrix passed four logins, one local-peer probe,
and private-message and room-message probes. GitHub Live Parity run
[`36499213873`](https://github.com/snapetech/slskr/actions/runs/36499213873)
passed both its Rust UI/API job and credentialed public interop job on
`506a5dd5`. The credentialed TSV artifact and full UI/API parity artifact are
linked from the sanitized receipt in
[`20260928-live-interop-hosted-summary.md`](../../benchmarks/artifacts/20260928-live-interop-hosted-summary.md).
