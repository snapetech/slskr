# Frozen 1:1 parity completion plan

## Goal

Achieve and prove externally observable 1:1 feature parity and bidirectional
interoperability between slskR and both frozen targets:

- slskd `16e5d86ec9a91120f3ef40b85cb22036566b788a`
- slskdN `65a14a8b821de4df4ab7ef3ab3b156d7206837a3`
- slskNet.Runtime `af73ff3f84fda7ba890bb5aea3adf712e5400cf6`

Rust-native internals are allowed. Externally observable differences are not.
Where the two targets conflict, an explicit compatibility profile must reproduce
each target rather than choosing one compromise behavior.

## Current position

**Updated 2026-07-29**: re-ran `scripts/audit-parity-manifest.py --check-frozen`
against the frozen slskd/slskdN pins (worktrees at the commits below) after this
document had gone stale (last regenerated 2026-07-17). The configuration
workstream is now fully closed: 436/436 complete, 0 missing (was 245/436, 191
missing). All 191 configuration contracts in the implementation-gap queue below
closed in the intervening real implementation work (see git log 2026-07-17
through 2026-07-27 for the daemon-foundation, DHT/mesh/PodCore, and
media/services commits that did this). Spot-checked a sample of the previously
"missing" families (dht.*, mesh.*, PodCore.*, feature.SongId, feature.Solid,
player.*) directly against the fresh manifest entries to confirm this wasn't a
scoring artifact -- confirmed, all report `complete` with
`lifecycleValidationDifferential: complete`.

Parity is **not achieved**. Every planned workstream has an executable
certification denominator. The literal proof-case closure ratio is now
**9,385 / 19,122 = 49.08%** (was 853 / 19,122 = 4.46% at the start of this
review cycle), but this is not a product-completion estimate: most
generated cases are Cartesian proof dimensions initialized as `needs-proof`,
including behavior already implemented and tested in slskR. Product
completion remains unreported until the subsystem reclassification
separates absent behavior from present-but-unlinked evidence.

**2026-08-03 reclassification**: `scripts/audit-parity-manifest.py`'s
`api_entries()` hardcoded every `security-authorization` case as
`needs-proof` unconditionally -- unlike the `configuration` workstream, there
was no code path that could ever mark one complete, regardless of how much
real test evidence existed. Added an exhaustive in-process differential test
(`security_authorization_matrix_matches_declared_policy_for_every_frozen_route`
in `crates/slskr/src/main.rs`) that drives all 769 declared routes across
both frozen controller-auth-policy registries (`crates/slskr/data/slskd(n)-
controller-auth-policy.json`) through all 10 manifest credential profiles via
the real `route_http_request`/`check_route_auth` dispatch path -- the exact
gate the live HTTP server uses -- and emits a machine-readable ledger the
manifest script now reads (`security_authorization_ledger()`) to promote
proven cases to `complete`. All 7,690 cases pass against current slskR (0
mismatches): the `security-authorization` workstream moved from 0/7,690 to
7,690/7,690 complete. This is genuine executable evidence (verified with a
deliberate negative control that the harness does detect real mismatches),
not a route-presence shortcut. Followed immediately by a first
controller-api batch: 5 more `controller_api_differential_*` tests
(`crates/slskr/src/main.rs`) crediting 123 of the 5,300 controller-api
cases (74 of which matched a real manifest entry after the frozen-registry
re-run) via the same pattern, reusing the shared production contract
`versioned_get_failure_contract` and independently re-verified real
DB-close fault-injection tests. An Explore audit estimated roughly
500-650 of the 5,300 controller-api cases already have genuine,
non-duplicate passing evidence sitting in the test suite unwired to any
ledger -- the next-highest-value work is wiring more of that existing
evidence, not writing new tests from scratch. A second controller-api
batch (library/interests/now-playing/messages, 15 cases) brought the
total to 138. See the `parity_manifest_reclassification` session memory
for the full family breakdown and estimate if picking this back up.

`persistence-lifecycle` (798 cases) reclassification also started: same
hardcoded-`needs-proof` structure fixed in `persistence_entries()`, one
differential landed crediting 16 cases (Searches/Events/Conversations/
PrivateMessages, the cleanest oracle-EF-domain-to-slskR-table matches).
Most of the other ~60 domains either have a clean table match not yet
wired (~13 more, including Transfers where the test already exists) or
are stored in slskR's generic `controller_features` KV table rather than
a dedicated SQL table and need a real per-domain rehydration check, not
a name-matching exercise -- see session memory for the full domain-name
mapping table before resuming this.

A follow-on batch wired Transfers plus 8 more clean-match domains
(Collections, CollectionItems, UserNotes, WishlistItems, Contacts,
ShareGrants, ShareGroups, ShareGroupMembers), bringing persistence-
lifecycle to 34/798, and one more controller-api batch (10 PodCore
routes' full CRUD lifecycle) brought that workstream to 148/5,300.
Subsequent controller-api batches (Collections/CollectionItems CRUD +
reorder, activity/hashdb/transport GETs, mesh-rendezvous/capabilities
GETs, swarm-analytics GETs, podcore membership/message stats GETs,
podcore routing route/route-to-peers/stats, podcore DHT/discovery/
membership/routing/messages/backfill maintenance mutations, share-grants
CRUD lifecycle with a real readback/404-after-delete check the original
test didn't have, mediacore fuzzy-match/perceptual-hash/portability/
retrieval/stats routes, mediacore publish-descriptor/retrieve/stats
lifecycle, mediacore content-id/fuzzy-find/IPLD-link routes, 17 materialized
empty-state GETs spanning nowplaying/listening-party/mediacore/podcore/
pods/quarantine-jury/security/traces, the openapi-mutation-dtos tail
covering content-id validation/nowplaying/content-pod/wishlist-lifecycle/
overlay-blocklist/quarantine-jury/security-bans, Collections real
per-caller ownership scoping, share-grants real transitive ownership
scoping, Soulfind-bridge search/download/admin-clients routes, SongID
run-lifecycle routes, listening-party membership-gated forgery-resistant
event lifecycle plus transports-status, ActivityPub music-actor/
WebFinger discovery, hashdb entries/sync-since paging, discovery-graph/opinions/contacts-
from-discovery/search-item download-stream, deterministic-openapi-
mutations spanning autoreplace/destinations/DHT/hashdb-optimize/
nowplaying-DELETE/integrations/transfers/library-health/overlay-
blocklist, the 26-route versioned-openapi-validation rejection-path
table plus its large-DTO success-path remainder (multisource/musicbrainz-
bloom/songid-DTO/taste-recommendations/portforwarding/realm-authority-
decision/podcore sequence), auxiliary mutations spanning warm-cache
hints/shares-scan/profile/wishlist-CSV-import/podcore-content-validate/
sharegroups/quarantine-jury-verdicts, release-radar subscriptions/
observations/notification-routing, library-health issues/by-artist/
by-codec/by-type/summary/dashboard/scans/remediation -- found via a
`route_http_request` call-density scan after the original test-naming
catalogue was exhausted, bridge-admin config/start/stop/status plus
federation-diagnostics/logs from the 53-call sibling test, bridge
admin-stats/dashboard plus source-feed-imports preview, opinions/
security-circuits/mediacore-descriptor-stats/pods/podcore-channels plus
the pod signing keypair/sign/verify real-ed25519-crypto pipeline,
quarantine-jury signed-verdict/quorum/idempotent-acceptance lifecycle,
content-bound stream tickets with real revocation-on-grant-delete,
port-forwarding bounded-listing and gateway-pinning security checks,
security-reputation real-score/violation-tracking checks,
realm-subject-index conflict-detection and authority-decision
lifecycle, musicbrainz-overlay export-review/approval real-gating and
idempotent-approval checks, pod-membership-workflow real queued join/
leave/accept/cancel lifecycle, pod-channel-messages sender-identity-
spoofing rejection and incremental-cursor pagination, hashdb history-
backfill real batched-progress persistence, user-group blacklist-over-
privileged precedence and leecher/privileged live classification)
brought the workstream to 499/5,300, and a separate fix relabeling 20
POST/PUT/DELETE cases mis-tagged `populated-dynamic-state` (a
GET-only case name the manifest classifier silently ignores for other
methods) to the correct `mutation-side-effects-and-readback` brought
it to 519/5,300 -- a real, no-new-tests correctness fix, not a new
batch (a `pod_management_routes_persist_crud_members_and_bindings`
differential was also found already credited from earlier in this
session and a duplicate attempt was discarded before committing).
After lowering the call-density scan's threshold from `count > 3` to
`> 2`, credited virtual-soulfind-v2's real end-to-end catalogue-search
-> plan -> intent -> process -> completed workflow (8 routes), and
source-discovery's real session-command dispatch, overlapping-run
rejection, and search-result projection (6 routes), plus pod/jury
verification and stats routes (real membership/signature checks,
real per-status audit counts and staleness, forged-sender rejection
counted in stats), bringing the workstream to 556/5,300. A further
batch credited the real slskdN `PlaybackController` (buffer-derived
priority thresholds, nanosecond-precision last-write-wins feedback
storage with a real multi-write readback check) and, while verifying
it, found and fixed a recurrence of the `populated-dynamic-state`
mislabeling bug in 9 already-committed `mediacore` fuzzy-match/
perceptual-hash cases (a 3-argument `record!` macro variant that
hardcodes `"method": "POST"` in the ledger, invisible to the earlier
targeted fix), bringing the workstream to 573/5,300. A follow-on batch
credited the real ActivityPub actor inbox/outbox/followers/following
routes (genuine HTTP Signature enforcement, a real relationship store
behind the followers/following collections, idempotent re-delivery,
and a real Undo), split into two smaller tests after the combination
of the heavy signature-fixture and the ledger-macro pattern overflowed
the default test-thread stack -- `Box::pin`-wrapping the direct
`route_http_request_with_headers` calls (mirroring the existing
production idiom) fixed it -- bringing the workstream to 583/5,300. A
follow-on attempt to credit `runtime-failure-and-timeout` (almost
uncovered in the manifest) across 22 routes from the `*_roll_back_
when_persistence_fails` fault-injection family mostly turned out to be
duplicate work: checking `/tmp/slskr-parity-evidence/controller-api/
*.json` directly (not just grepping source-test names) showed 21 of
the 22 were already credited by pre-existing differentials from
earlier in this session. Corrected down to the 1 genuinely new case
(`DELETE /api/v0/conversations/{username}`), bringing the workstream
to 584/5,300 -- see session memory for the now-mandatory
evidence-directory-first duplicate-check rule this established.
Applying that rule immediately paid off: the real Spotify PKCE OAuth
flow (authorize/callback/status/disconnect, real code-challenge
generation, real server-issued-state validation, a real token-exchange
+ profile-fetch round trip through a local fixture server driving the
same `complete_spotify_authorization` the production callback handler
calls) had only one pre-existing case credited across all 4 routes --
confirmed via the evidence directory before writing, so all 5 new
credits landed real, bringing the workstream to 589/5,300. The real
mesh HTTP gateway proxy route (`POST /mesh/http/{serviceName}/
{method}`: config-driven service allowlist rejection, a genuine "no
providers" 503, and a real local `private_gateway::Gateway` instance
proving the nominal dispatch path reaches an actual service handler)
had no prior credit either, bringing the workstream to 592/5,300. The
real Solid protocol status/WebID-resolution routes (real configured
clientId/redirectPath reflection, malformed and policy-blocked WebIDs
genuinely rejected pre-fetch, and a real successful resolution
extracting oidcIssuer triples from an actual fetched Turtle profile
via a local fixture server) had no prior credit either, bringing the
workstream to 597/5,300. The real pod-membership self-publish routes
(impersonation and non-self/non-moderator updates genuinely rejected,
self-escalation attempts in the request body pinned back to the real
stored role) had no prior credit either, bringing the workstream to
601/5,300. The transfer-reports family's missing-direction 400 (a real
`Enum.TryParse<TransferDirection>` failure path, not silently treated
as "no filter") was the one still-open case among 3 routes already
partly credited, bringing the workstream to 604/5,300. The real
transfer-download-cancel route (404 on a nonexistent download, the
frozen 204 contract on a real cancel) had zero prior credit, bringing
the workstream to 606/5,300. The analyzer-migration
(version-required, exact `{"updated":0}` shape), hashdb-optimize
(real observed query data, not hardcoded zeros), and telemetry-KPI
(real `application/json` vs the base route's `text/plain`) route
families were nearly entirely open, bringing the workstream to
612/5,300. The application/build route (real app version, not the
wire-protocol version) and the file-delete route (forbidden by
default, a real file removal with readback confirmation, path-
traversal rejection) had zero prior credit either, bringing the
workstream to 616/5,300. A further sweep extended the runtime-
failure-and-timeout fault-injection coverage across security-ban,
collection/share-grant, sharegroup, and shares-rebuild routes (6
routes' worth of real closed-database rollback proof, plus a real
held-scan-permit concurrency proof for the shares-rebuild route),
bringing the workstream to 625/5,300. `PUT /api/v0/options`'s
forbidden-by-default gate (PATCH already had this credited, PUT
didn't), real per-recipient database persistence for conversations-
batch, and the real replay-deduplication projection reflected by the
conversation-history GET route brought the workstream to 628/5,300.
The Spotify-authorize OAuth-state persistence-failure rollback, real
caller-identity enforcement on pod creation, the mesh handshake's real
baseline shape, and the listening-party directory's real content-id
encoding each had zero prior credit, bringing the workstream to
632/5,300. The storage-directory route's real unknown-query-parameter
tolerance and recursive-listing truncation budget, plus the server
route's real disconnected-state sentinel shape for the slskdN target,
brought the workstream to 640/5,300 (a third candidate route, `GET
/api/v0/telemetry`, turned out to be a real slskR-internal handler
with no registered route in either frozen oracle -- dropped rather
than credited after regenerating both registries fresh, since a
stale mid-session route snapshot had been checked with an overlooked
negative result). Real dispatch-unavailable rollback for the searches
route, plus the previously entirely uncredited external-visualizer-
launch route across all 3 of its real scenarios (successful launch
with command redaction, failed launch with command redaction, and a
real held process-pool semaphore blocking a concurrent launch),
brought the workstream to 644/5,300. Real secret-redacting options
overlay mutation and non-object-body rejection, real dynamic DHT-
status reflection of a watched config change, and real internal-
host/port redaction across 3 previously entirely uncredited bridge
projection routes plus the application route's distinct redaction
case brought the workstream to 651/5,300. The real VPN-status projection on the application route,
the real wire-command dispatch behind an interest mutation, the real
empty/disabled bridge-admin-clients baseline, one genuinely uncredited
route from a 17-route materialized-empty-state table, and real
adversarial-settings YAML persistence/readback brought the workstream
to 637/5,300.

The relay-agent/controller subsystem (deferred as a genuinely missing
capability, see earlier notes) was investigated further this session to
confirm scope rather than just estimate it: the frozen oracle's real
implementation is 2,880 lines (`RelayHub.cs`/`RelayService.cs`/
`RelayClient.cs`/`RelayController.cs`), and there is no smaller honest
slice available -- even the HTTP-only controller surface requires real
agent-registration/token-issuance state first. Remains deferred.

A 4th workstream, `protocol-behaviors` (1,465 cases), was also confirmed
to have the same hardcoded-`needs-proof` bug and opened this session:
`crates/slskr-protocol`'s own trusted codec has Rust enums matching the
frozen oracle's real Soulseek message codes by name and numeric value
exactly, making this workstream's proof unusually mechanical. First
differential closes the `soulseek-initialization` family (2 units, both
targets since slskd/slskdN share byte-identical `MessageCode.cs`
base families): 0 -> 4/1465. `crates/slskr-protocol`'s existing test
files (`tests/peer.rs`/`tests/server.rs`/`tests/distributed.rs`) have
many more real round-trip test groups not yet mapped to specific oracle
units -- the single best next lever available (potentially hundreds of
cases from tests that already exist and pass), see session memory for
the specific test names identified.

Update: the `soulseek-peer` (25 units) and `soulseek-distributed` (6
units) families are now fully closed too, both frozen targets --
protocol-behaviors: 4 -> 70/1465. A follow-on batch mapped 35 of the 90
`soulseek-server` units (using a value-lookup table rather than hand-
pairing oracle names, after an early draft found that approach mismatches
names silently) -- protocol-behaviors: 70 -> 136/1465. The remaining ~55
server units and the slskdN-only protocol extensions have no existing
round-trip test and need new tests, not just wiring.

| Workstream | Audited denominator | Current evidence | Closure state |
| --- | ---: | --- | --- |
| Configuration | 436 frozen YAML leaves | 436 complete, 0 partial, 0 missing | **Closed** (2026-07-29) |
| slskd controller API | 91 routes | Route presence is covered; exhaustive behavior is not | Open |
| slskdN controller API | 678 routes | Route presence is covered; exhaustive behavior is not | Open |
| Frozen WebUI API calls | 417-call union | Call presence is covered; rendered workflows are not | Open |
| Soulseek and adjacent protocols | 1,465 proof cases | Core and several live paths pass; exhaustive case evidence remains open | Open |
| Persistence and lifecycle | 798 proof cases | Selected config and state families are proven | Open |
| Security and authorization | 7,690 credential-profile proof cases | 7,690 complete via exhaustive live-dispatch differential (2026-08-03) | **Closed** (2026-08-03) |
| Packaging and operator behavior | 240 proof cases | Existing gates have not yet been attached to every manifest case | Open |
| Bidirectional interoperability | 310 proof cases | Selected live matrices exist; exhaustive feature-pair evidence is open | Open |

### Known implementation-gap queue

**Closed as of 2026-07-29.** The 191 configuration contracts previously
classified as genuinely missing (grouped below by shared implementation
dependency) all now report `complete` in a fresh, `--check-frozen`-validated
manifest run. Kept for historical reference:

| Subsystem batch | Missing contracts (as of 2026-07-17) | Families | Execution priority |
| --- | ---: | --- | ---: |
| Daemon foundation | 45 (now 0) | web/HTTPS, flags, logger, retention, permissions, telemetry, search retention | done |
| Core workflows | 16 (now 0) | interests, rooms, wishlist, shares, destinations, search throttling | done |
| Advanced networking and security | 88 (now 0) | DHT, Mesh, PodCore, overlay, overlay data, relay, security | done |
| Media and advanced services | 42 (now 0) | feature gates, player, Solid, SongID, VirtualSoulfind | done |

This closes the **configuration-contract presence and validation** layer only
(YAML/environment/CLI acceptance, defaults, and basic validation matching both
frozen targets). It does not close the corresponding controller-API,
security-authorization, protocol-behavior, persistence, or live-interop
`needs-proof` cases for these same families -- see the 2026-07-29 review
findings for a sampled, code-verified estimate of how much of that remaining
work is a genuine feature gap versus unlinked-but-correct evidence.

The 18,269 `needs-proof` cases are not presumed absent. They are certification
dimensions to be linked in bulk from the subsystem contract matrices now that
the known configuration-layer implementation gaps are closed.

The 2026-08-01 transport pass added the frozen slskdN QUIC control wire to the
Rust client and daemon: `slskdn-overlay` ALPN, bounded bidirectional streams,
MessagePack envelopes, public-key-value pin validation, and Pod routing for
explicit non-shared-port pins. Shared DHT/UDP demultiplexing, public QUIC
proxying, QUIC data transport, and live QUIC interoperability are still open
certification work.

The raw proof-case closure ratio is a certification-ledger metric only. It is
not used as an implementation queue or as a product-completion estimate. Work
is selected and closed by vertical subsystem so one implementation and one
generated differential matrix can satisfy all affected route, configuration,
protocol, persistence, UI, and interoperability cases together.

`scripts/audit-parity-manifest.py --check-frozen` currently materializes 19,122
unique proof cases across all workstreams: 853 complete, 0 partial, 0 missing,
and 18,269 needing behavioral proof. There are zero `denominator-missing` cases.
The 853 complete cases include 417 frozen WebUI call-presence cases; they do not
claim that the corresponding rendered workflows are complete.

The 14 frozen `transfers.download` leaves are closed. Both target profiles now
have exact startup/CLI/environment/YAML projection and validation, watched
lifecycle and restart proof, bounded retry backoff, resume/overwrite behavior in
the incomplete directory, collision rename/overwrite, completed layout,
permissions, slot admission, aggregate pacing, and slskdN auto-replacement
enable/threshold/interval consumers. The focused frozen differential artifact is
`/tmp/slskr-options-differential.GRw1tx`.

The 12 frozen `soulseek.connection` leaves are closed. Both target profiles now
have exact defaults, YAML/environment/CLI precedence, projection, secret
handling, validation, watched current/startup state, and frozen lifecycle
behavior. slskR applies connect/inactivity/transfer deadlines, control and
transfer socket buffers on outbound and accepted sockets, the bounded outbound
write-work queue, and SOCKS5 no-auth or username/password negotiation across
server, regular, obfuscated, direct, indirect, and transfer dials. The focused
frozen differential artifact is `/tmp/slskr-options-differential.cGgsRb`; the
full slskR regression result is 840 unit tests plus 2 API smoke tests.

The six frozen Soulseek profile and distributed-network leaves are closed as a
single subsystem batch. slskR matches both target profiles for picture,
diagnostic level (including the actual runtime enum's `trace` value), distributed
disable/child acceptance/limit/logging, CLI/environment/YAML layering, exact
validation, current/startup/watch/restart lifecycle, disconnected and live
application DTOs, response-time picture reads and failures, diagnostic filtering,
parent/child ownership, branch and depth propagation, capacity changes, search
forwarding, disconnect cleanup, and socket framing. The frozen options artifact
is `/tmp/slskr-options-differential.5Vcv3Q`; the peer-wire picture lifecycle
artifact is `/tmp/slskr-options-differential.BcY9zS`.

The Lidarr configuration/runtime family is closed through paged wanted sync,
background scheduling, wishlist policy, completed-directory import, path
mapping, and manual-import differentials. The Spotify configuration/runtime
family is closed through PKCE authorization, encrypted token persistence,
refresh and disconnect lifecycle, client-credentials fallback, provider target
parsing, paged source imports, market selection, timeout enforcement, and
frozen controller/configuration differentials. This closes those integration
families only; it does not close the remaining global API, UI, protocol, or
interoperability proof cases.

The YouTube and Last.fm configuration leaves are also closed through frozen
startup/watch/restart and validation differentials plus real fixture-backed
provider retrieval. YouTube playlist imports page through API results; Last.fm
imports loved, recent, and top-track shapes with configured credentials.

The nine frozen VPN/Gluetun leaves are closed through exact YAML/environment/CLI
layering, validation and secret projection, target-specific API behavior,
API-key-over-Basic authentication precedence, timeout/no-redirect HTTP polling,
single- and slskdN multi-port-forward discovery, application-state projection,
Soulseek connection gating, disconnect-on-loss, and reconnect-on-recovery.

The 21 frozen script-integration inventory paths are closed through dynamic
script dictionaries, event validation, target-specific controller projection,
command/args/arglist execution modes, the per-instance script directory,
`SLSKD_SCRIPT_DATA` event serialization, slskdN command safeguards and timeout,
and live process-output differentials against both frozen daemons.

The 38 frozen upload-slot and transfer-group configuration leaves are closed
through exact target projection, YAML/environment/CLI precedence, validation,
watched current/startup state, restart persistence, blacklist membership, group
controller APIs, real `QueueUpload` and `PlaceInQueue` protocol handling,
priority/FIFO/round-robin scheduling, aggregate slot and bandwidth enforcement,
and frozen differentials against both daemons. This closes that configuration
and runtime subfamily only; broader transfer API, UI, persistence, and
interoperability proof cases remain open.

## Definition of complete

The goal reaches 1:1 only when all of the following are true for both target
profiles:

1. Every frozen surface is present in a machine-readable inventory.
2. Every inventory entry is implemented and behaviorally proven. There are zero
   `missing`, `partial`, `needs-proof`, compatibility-shell, excluded, or
   unclassified entries.
3. Defaults, YAML/environment/CLI precedence, validation, secret handling,
   watch/reconnect/restart semantics, persistence, and corrupt-state behavior
   match where applicable.
4. API status, headers, content type, DTO bytes after documented normalization,
   auth/CSRF/rate-limit policy, mutations, errors, timeouts, and concurrency
   behavior match.
5. Every frozen UI workflow is tested as a rendered user action through success,
   empty, loading, validation, authorization, server-error, reconnect, and
   restart states where applicable.
6. Both-direction protocol and feature exchanges pass against slskd and slskdN,
   including reconnect, resume, cancellation, malformed input, and failure paths.
7. Packaging, service lifecycle, signals, logging, telemetry, filesystem
   permissions, upgrade/restart, and supported deployment modes match.
8. The complete hermetic, differential, live-network, security, WebUI, workspace,
   packaging, and release gate set passes from a clean process state.

Passing route-presence tests, returning a plausible DTO, or accepting a config
key does not satisfy these conditions.

## Execution order

### 0. Classify the existing implementation by subsystem

Map the existing implementation and tests to a dependency-ordered subsystem
matrix. Distinguish absent behavior from behavior that is already implemented
but has not yet been linked to generated proof. Use the 19,122-case manifest as
the final zero-gap certification ledger, not as 19,122 implementation tasks.

### 1. Close the shared daemon foundation

Finish startup and configuration lifecycle, HTTP/HTTPS, authentication and
security policy, Soulseek connection/listener behavior, logging, metrics,
retention, throttling, and compatibility-profile conflicts. These behaviors are
dependencies of nearly every later differential and therefore outrank the
largest isolated leaf family.

### 2. Close core user workflows as vertical slices

Close search, browse, shares, downloads, uploads, rooms, conversations, users,
wishlist, and playback. Each slice includes configuration, API, WebUI, runtime
protocol, persistence/restart, malformed and denied requests, and live exchanges
with both frozen targets.

### 3. Close integrations and library/media workflows

Close Lidarr, Spotify, scripts, webhooks, MusicBrainz, library management,
discovery, destinations, and related jobs. The 92-leaf `integrations` family is
handled as consumer-backed subfamilies, not as projection-only config work.

### 4. Close slskdN advanced services

Close DHT, mesh/overlay, PodCore, relay and VPN, Solid, VirtualSoulfind, SongID,
federation, streaming, and all associated security/operator controls. Retain
live cross-runtime proof for every wire-facing service.

### 5. Exhaust both controller and WebUI matrices

Run generated success, empty, malformed, unauthorized, forbidden, conflict,
not-found, timeout, mutation, persistence, restart, and concurrency cases over
all 91 slskd and 678 slskdN routes. Drive all 417 frozen WebUI call workflows
through rendered state transitions for the matching compatibility profile.

### 6. Final certification

Run every focused and aggregate gate from clean process state, then run the full
bidirectional live matrix against both frozen daemons. Re-run restart,
corrupt-state, packaging, security, and upgrade cases. The literal parity gate
must fail on any non-complete manifest entry and pass only at zero gaps.

## Work-selection rules

- Keep one complete vertical subsystem in progress at a time.
- Select the next batch by dependency fan-out first, then shared-target coverage,
  observable feature breadth, and proximity to complete proof.
- A rare edge case that does not block another family is recorded in the manifest
  and deferred to that family's exhaustive certification pass.
- Each subsystem gets table-driven contract and differential tests that attach
  evidence to all affected manifest cases in bulk. Compile and live-oracle runs
  happen at subsystem boundaries, not per configuration leaf or route.
- Frozen slskd and slskdN trees are read-only behavioral oracles. Upstream defects
  are reported to the user; no upstream changes or PRs are made.
- Counts move only when executable evidence satisfies the full completion rule.

## Immediate critical path

1. Classify all existing slskR code and tests into the subsystem matrix and
   produce a real absent-versus-present-unproven gap list.
2. Close the shared daemon foundation in one batch: configuration engine,
   process lifecycle, HTTP/auth/security, Soulseek connection/listeners, and
   distributed tree.
3. Close core Soulseek workflows as complete vertical slices: search, browse,
   shares, transfers, messages, rooms, users, recommendations, and privileges.
4. Close slskdN advanced services through shared DHT, overlay, persistence,
   identity, authorization, and streaming layers rather than route-by-route.
5. Generate controller, WebUI, persistence, operator, and live-interoperability
   matrices from the subsystem contracts and attach proof in bulk.
