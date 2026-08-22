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

**Closed 2026-08-20 for the pinned frozen boundary.** The fresh universal gate
reports 19,216/19,216 materialized cases complete, with zero partial, missing,
needs-proof, or denominator-missing cases. The strict live transport/lifecycle
artifact passes 11/11, the current UI comparison artifact passes all 9 workflow
health cases and 18 target/profile comparisons, and the exact live interop
ledger is closed with the fresh mesh retry/failure supplement. The detailed
checkpoint narrative below is retained as historical evidence; its older
percentages and open-work statements are not the current closure status.

### Historical checkpoint narrative

**Updated 2026-08-16**: re-ran the bounded controller evidence workflow
against the frozen slskd/slskdN pins (worktrees at the commits below). The
configuration workstream remains fully closed at 436/436 complete, 0 missing.
The mesh-sync batch added frozen-compatible canonical signing and verification,
hash-entry signatures, and real daemon private-message dispatch evidence for
Hello, ReqDelta, PushDelta, ReqKey, ReqChunk, and their generated responses.
The follow-on mesh-controller batch matched ticket validation, response shape,
feature-disabled behavior, missing/malformed paths, ticket readback, and the
1,000-ticket limit. The latest mesh health/transport evidence also covers the
empty health snapshot and a live populated overlay transport projection. The
HashDb controller validation/empty-contract evidence now also proves the
frozen slskdN numeric-binding, empty-state, unknown-key, zero-size, and fixed-
route contracts, including strict versioned-v0 rejection paths. The complete
count before the latest focused controller batch was 14,144/19,282, with 0
partial, 0 missing, and 5,138 needs-proof cases. The latest focused batch
below moves that count to 14,194/19,282, with 5,088 needs-proof cases. The
latest slskd runtime-failure controller differential closes all 24
remaining open slskd controller rows: transfer detail, queue-position, and
upload-detail routes now preserve the frozen closed-SQLite failure contract,
while unrelated application, session, options, rooms, server, shares,
telemetry, browse, and control routes preserve their frozen contracts under
the same fault. The slskdN TransfersController runtime-failure differential
then closes 26 more rows: all 24 database-backed transfer projections and
mutations preserve the frozen closed-SQLite `500` contract, while the
process-local accelerated download GET/PUT pair remains available with `200`
responses. The Batches persistence differential now proves the slskd Transfers
database schema, create/read, update/delete, restart, duplicate-write atomicity,
and corrupt-options failure cases against a dedicated SQLite table. The HashDb
persistence differential now proves the slskdN HashDb and HashDbState schema,
controller write/read, update/delete, restart rehydration, snapshot atomicity,
and corrupt-state failure cases against SQLite. The latest
mesh-controller evidence additionally covers
merge/publish idempotency and concurrent requests, restart reload of persisted
mesh state, and the generic failure contract for unsupported and unavailable
mesh-sync peers. The gateway evidence covers malformed route rejection,
service mutation/readback, concurrent joins, and reload persistence. The React
Network workflow now exercises a successful mesh-sync button action. The first
mesh security-control slices now match the frozen relaxed remote payload cap,
signed-entry compatibility setting, mesh-sync quarantine profile, gateway
authentication/origin checks, durable self-signed gateway identity, token-bucket
and DHT quotas, replay/connection guards, SPKI pinning and rotation persistence,
endpoint pin selection, transport-policy specificity, and redacted security
events; 104 slskdN security-control cases are backed by focused executable
ledgers. The follow-on security differential now also covers the shared
controller API-key authentication and administrator-JWT components for both
targets across default, nominal, rejection, secret-free output, and
rotation/reload cases. The atomic file-writer differential now proves
missing-parent creation, confined symlink replacement, and restart retention
in addition to nominal overwrite. The frozen controller scanner was also
corrected to honor method-level `[Route("...")]` attributes. The authoritative
inventory is now 96 slskd and 683 slskdN routes, restoring the five real
options action routes per target and removing the invented `POST /api/v0/options`
route. The live controller gate passes all materialized routes with zero
generic 404s, fallbacks, or probe errors. This inventory correction adds 109
complete proof cases, including the corresponding authorization rows. The
follow-on options-action differential links 43 more controller cases for the
real startup, debug, YAML, YAML-location, YAML-validation, and reload paths
across both targets. The EventsController edge differential then closes 11
slskdN cases covering empty, malformed, populated, closed-SQLite, restart,
concurrent, and failed-write behavior, including the frozen `500` rollback
contract. The DestinationsController edge differential closes 11 more cases:
exact versioned `Path` model binding and `Path is required` rejection,
config-backed empty-state defaults, closed-SQLite independence, non-mutating
validation, and concurrent validation. The full 203-test controller
differential suite and refreshed authoritative frozen audit pass. The
NowPlayingController differential then closes 12 webhook and model-binding
cases: generic, Plex, and Jellyfin payloads, stop/pause clearing, malformed and
empty payloads, process-local restart behavior, concurrent updates, and
closed-SQLite independence. Together with the existing edge-state ledger, the
full 204-test controller suite and authoritative audit now report 12,770
complete cases.
The SourceProviders edge differential then closes six read-only cases across
the versioned and unversioned aliases: malformed extra segments, empty-state
catalog projection, and closed-SQLite independence. The full 205-test
controller suite and refreshed authoritative audit pass.
The fixed-version Signals edge differential then closes six configuration
and status cases: malformed extra segments, empty projections, and
closed-SQLite independence for both DTOs. The authoritative audit now reports
12,782 complete cases.
The compatibility info/fairness differential then closes eight cases: the
slskdN `/api/info` projection, durable fairness totals and ratios, malformed
routes, populated state, and closed-SQLite behavior. The playback differential
adds both process-local runtime-failure cases, and the trace-summary
differential closes nominal, malformed, populated, and closed-SQLite cases.
The discovery-graph edge differential then closes malformed, empty,
runtime-failure, restart, and concurrency cases. The mesh-stream ticket
differential adds nominal ticket reads, runtime failure, restart, and
concurrent ticket creation. The full controller differential suite now passes
210 tests, and the authoritative audit reports 12,810/19,282 complete
(66.44%), with 6,472 needs-proof and slskdn-controller-api at 2,625/4,704.
The follow-on edge batch adds strict invalid-boolean rejection for network
stats, closed-SQLite independence for logs, federation diagnostics, WebFinger,
collections, sharegroups, and Solid status, plus the corresponding empty and
malformed projections. The full controller differential suite now passes 213
tests, and the latest authoritative audit reports 12,829/19,282 complete
(66.53%), with 6,453 needs-proof and slskdn-controller-api at 2,644/4,704.
The Solid resolver lifecycle differential then adds a local Turtle
fetch/readback case plus closed-SQLite failure, restart, and concurrent
failure cases. The full controller differential suite now passes 214 tests,
and the latest authoritative audit reports 12,833/19,282 complete (66.55%),
with 6,449 needs-proof and slskdn-controller-api at 2,648/4,704.
The slskdN SearchCompatibilityController differential then closes seven
cases for the unversioned `/api/search` route: trimmed queries, exact result
projection and 32-hex search IDs, malformed and non-positive-limit rejection,
closed-SQLite failure normalization with rollback, persistence rehydration,
and concurrent unique searches. The full controller differential suite now
passes 215 tests, and the latest authoritative audit reports 12,840/19,282
complete (66.59%), with 6,442 needs-proof and slskdn-controller-api at
2,655/4,704.
The Capabilities differential then closes 21 peer-capability projection and
failure cases, and the Profile differential closes 17 self-profile, public
lookup, update, invite, validation, and unknown-peer cases. The full
controller differential suite reaches 218 passing tests, and the authoritative
audit reports 12,895/19,282 complete (66.88%), with 6,387 needs-proof and
slskdn-controller-api at 2,710/4,704. The follow-on PodCore channel lifecycle
differential adds 10 executed create/update/delete cases covering storage
failure rollback, reload persistence, and concurrent mutations; the frozen
storage-error mappings now return the controller's 500 contracts. The full
controller differential suite reaches 219 passing tests, and the latest
authoritative audit reports 12,905/19,282 complete (66.93%), with 6,377
needs-proof and slskdn-controller-api at 2,720/4,704. The PodCore message
storage differential then closes all 26 cleanup, stats, search/count,
rebuild-index, and vacuum cases, including persisted-file failure contracts,
reloads, and concurrent idempotency. The WebUI audit runner now bounds DOM
navigation and network-idle settling so the full desktop/mobile audit
completes without an unbounded browser wait. The full controller differential
suite reaches 230 passing tests, and the latest authoritative audit reports
13,368/19,282 complete (69.33%), with 5,914 needs-proof and
slskdn-controller-api at 3,183/4,704. The DhtRendezvous residual
differential closes all 56 remaining versioned DHT status, peer, announce,
discovery, overlay stats, connection, blocklist, certificate-pin, reset, and
concurrency cases. The VirtualSoulfind v2 residual
differential then closes all 65 remaining catalogue, intent, execution,
planning, processing, reset, and concurrency cases, including the frozen
400 response for blank dynamic IDs. The PodCore discovery-storage
differential closes 22 persisted-register/update/unregister/refresh and read-
projection cases; the corrected PodCore membership-storage ledger closes all
23 cases; and the PodJoinLeave residual differential closes all 20 remaining
cancel, pending-projection, join, acceptance, leave, and leave-acceptance
runtime, reset, and concurrency cases. The frozen PodJoinLeave service uses
in-memory pending-request storage, so its restart cases intentionally verify
reset semantics. The versioned SecurityController residual ledgers also close
18 ban/unban and 43 diagnostics validation, empty-state, populated-state, and
optional-service cases, including the frozen positive-count/limit query
contracts. The versioned SoulseekDiscovery residual ledger closes 64 item
normalization, interest mutation/idempotency, recommendation/item DTO,
disabled-rendezvous, user-interest, and capability projection cases; the
versioned interest handlers now publish item values and retain NoContent
semantics for duplicate and absent mutations. The versioned MultiSource
residual ledger then closes all 66 remaining job, search, source-discovery,
verification, download, swarm, restart, and concurrency cases while projecting
the frozen slskdN DTOs. The PodsController residual differential then closes
all 60 remaining pod list/detail, membership, ban, channel bind/unbind,
message, create/update/delete, runtime-failure, restart, and concurrency cases,
including the frozen 400 validation and 500 storage-error contracts. The
authoritative audit's React WebUI phase also
completes all 82 desktop/mobile route captures with zero browser-audit errors.
The WishlistController residual differential then closes all 58 remaining
Guid-bound item, ignored-result, search, CSV-import, viewed-state, persistence,
restart, and concurrency cases, including the frozen 400 malformed-id, 200
manual-search, and 500 storage-failure contracts. The full controller
differential suite reaches 231 passing tests. The legacy VirtualSoulfind
controller differential then closes all 28 canonical, shadow-index, and
disaster-mode cases across versioned and unversioned routes, including blank
MBID validation, canonical/shadow query failures, populated variant DTOs, and
the storage-independent disaster status projection. The full controller
differential suite reaches 232 passing tests. The RoomsController residual
differential then closes all 62 remaining compatibility and versioned room
cases, including blank route validation, disconnected available-room reads,
tracker mutation/readback, runtime failure, restart reset, and concurrent
leave behavior. The full controller differential suite reaches 233 passing
tests, and that authoritative audit reported 13,516/19,282 complete
(70.10%), with 5,766 needs-proof and slskdn-controller-api at 3,331/4,704.
The BridgeController and BridgeAdminController residual differential then
closes all 87 remaining Bridge cases across malformed, empty, populated,
runtime, versioned, restart, and concurrent paths. It matches the frozen
search and download validation/error contracts, transfer-progress lookup and
404 behavior, bridge status/admin DTO projections, lifecycle status responses,
and non-persisting configuration update response. The full controller
differential suite reaches 234 passing tests, and the refreshed authoritative
audit reports 13,603/19,282 complete (70.55%), with 5,679 needs-proof and
slskdn-controller-api at 3,418/4,704. The follow-on MediaCore residual
differential then closes all 86 remaining MediaCore controller cases across
runtime projections, restart reloads, and concurrent mutations. It also fixes
the multisource differential's previously unreachable concurrency case. The
full controller differential suite reaches 236 passing tests, and the latest
authoritative audit reports 13,787/19,282 complete (71.50%), with 5,495
needs-proof and slskdn-controller-api at 3,602/4,704. The follow-on MusicBrainz
residual differential closes all 80 previously open MusicBrainz controller
cases, covering artist release graphs, discography wishlist promotion, library
bloom snapshots/diffs/wishlist, overlay edits/export/routes, release-radar
subscriptions/observations/notifications, and targets. The route-ordering fix
also keeps the existing discography-coverage route exact. The full controller
differential suite reaches 237 passing tests, and the refreshed authoritative
audit reports 13,867/19,282 complete (71.91%), with 5,415 needs-proof and
slskdn-controller-api at 3,682/4,704.
The follow-on Jobs residual differential closes all 66 previously open Jobs
cases, covering native job lists/details, exact discography and label-crate
detail routes, versioned empty/malformed validation, job mutation readback,
restart reload of persisted projections, and concurrent creation. The full
controller differential suite reaches 238 passing tests, and the refreshed
authoritative audit reports 13,933/19,282 complete (72.26%), with 5,349
needs-proof and slskdn-controller-api at 3,748/4,704.
The follow-on Library residual differential closes all 57 previously open
slskdN Library controller cases, covering health projections, scan readback,
native library-item detail/browser shapes, malformed validation, compatibility
scan behavior, version-negotiation rejection, restart/reset, and concurrency.
The full controller differential suite reaches 239 passing tests, and the
refreshed authoritative audit reports 13,990/19,282 complete (72.55%), with
5,292 needs-proof and slskdn-controller-api at 3,805/4,704. The follow-on
SecurityController residual differential closes all 56 remaining slskdN
Security cases, covering configured adversarial/canary/Tor projections,
scanner/threat and transport state, circuit lifecycle and unavailable-builder
behavior, entropy validation, and adversarial/disclosure/reputation mutation
contracts. The full controller differential suite reaches 240 passing tests,
and the refreshed authoritative audit reports 14,046/19,282 complete
(72.85%), with 5,236 needs-proof and slskdn-controller-api at 3,861/4,704.
The follow-on Integrations residual differential closes all 51 remaining
slskdN integration-controller cases, covering Spotify authorization,
callback, status, disconnect, and PKCE/error paths plus Lidarr status,
wanted/missing, synchronization, and manual-import contracts. The full
controller differential suite reaches 241 passing tests, and the refreshed
authoritative audit reports 14,097/19,282 complete (73.11%), with 5,185
needs-proof and slskdn-controller-api at 3,912/4,704.
The follow-on BackfillController residual differential closes all 47 remaining
slskdN backfill cases, covering candidate/config/stats projections, malformed
binding and route rejection, closed-HashDb failures, scheduler enable/idle/
busy/trigger state transitions, file validation, reset behavior, and
concurrent requests. The full controller differential suite reaches 242
passing tests, and the refreshed authoritative audit reports
14,144/19,282 complete (73.35%), with 5,138 needs-proof and
slskdn-controller-api at 3,959/4,704.

**2026-08-16 bounded controller recheck**: the guarded
`scripts/check-slskdn-controller-parity.sh` gate rebuilt the production daemon
with one Rust job and a 12 GiB per-process virtual-memory ceiling, then
materialized and handled all 779 frozen routes (683 slskdN and 96 slskd) with
zero generic 404s, HTML fallbacks, compatibility fallbacks, or probe errors.
The refreshed evidence snapshot therefore closes route-presence for both
targets; with the existing behavioral ledger, slskdN controller behavior is
4,704/4,704 complete and slskd is 607/656 complete, leaving 49 slskd behavior
cases concentrated in file-storage, transfer, and room contracts. A full
`slskr` test-binary rebuild was not used for this batch: LLVM hit the guard's
12 GiB ceiling during test-target compilation, while the host remained healthy.
The remaining cases require focused production or split-test evidence rather
than another full test-target build.

**2026-08-16 focused controller residual batch**: the new
`focused-controller-tests` feature builds only the bounded residual test
target, under the same one-job/12 GiB guard. Its single executable
`slskd` file-transfer/room test passed all 51 emitted contract rows; 49 of
those rows were the remaining behavioral gaps and the other two refreshed
already-covered rows. The focused evidence closes slskd controller behavior
at 656/656 and leaves slskdN at 4,704/4,704. A second focused test also
closes the slskd `Files/FileService` existing/missing/overwrite lifecycle row.
The tests write evidence only after every assertion passes. No unbounded or
full monolithic test-target build is part of this proof.

**2026-08-16 frozen-ledger reconciliation**: the new
`scripts/audit-parity-manifest.py --reuse-evidence` mode rechecked the exact
frozen source objects and retained differential, WebUI, operator, and live
artifacts without starting Cargo, npm, or browser proof processes. It
materialized 19,216 cases as complete, with no incomplete materialized rows.
The final capped slskd run passed all 24/24 checks, including target-initiated
public search, user-watch, bidirectional browse and folder-contents probes,
restart persistence, rooms, and downloads. This is a frozen proof-ledger
result, not a universal product-completion claim: it reuses retained evidence,
classifies several target-local dimensions as not applicable, and does not
close the live/UI gaps listed in
[`universal-replacement-acceptance.md`](universal-replacement-acceptance.md).

**2026-08-17 universal replacement goal active**: the stronger target is now
complete end-user behavioral equivalence across both frozen targets. The prior
**19,216 / 19,216 = 100.00%** figure must not be used as completion for this
goal. Fresh live-backend UI evidence, all transport
directions and failure/reconnect paths, shared DHT/UDP and QUIC data paths,
relay certification, and the remaining target-specific workflow validations
are still required. Parity is **not achieved** under the universal
replacement contract.

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
case brought the workstream to 651/5,300. The mediacore descriptor-delete
route's real unpublished-baseline response, and the mesh-streams-
ticket route's real family-specific validation (proven with a much
smaller fixture than the full raw-TCP preview-fetch machinery its
source test uses), brought the workstream to 653/5,300.

The next evidence-pool sweep converted the existing compatibility-alias route
contract test into a `controller_api_differential_*` producer. Its real
dispatcher calls now record 80 route cases: 75 nominal status/header/body
contracts and five explicit missing-query validation contracts. A second
existing native-capability/library-health contract test was wired the same way
for six more cases. Duplicate-key inspection against the frozen registries
showed 54 genuinely new credits (the rest were already covered by other
differentials): `slskdn-controller-api` moved from 653 to **707/4,674** and
the unchanged `slskd-controller-api` remains **21/626**. Combined controller
API proof is **728/5,300** at that behavioral-only checkpoint; the
route-presence gate below is tracked separately and does not replace those
behavioral cases.

With the controller-api `count==1` AWK tier exhausted, pivoted to the
`persistence-lifecycle` workstream (798 cases, only 34 complete
before this pivot) -- checking its own evidence directory directly
(same discipline as the controller-api work) showed only 2 of its 6
per-domain cases (`create-and-read-roundtrip`, `restart-rehydration`)
had EVER been credited, for any domain, across the whole workstream.
The other 4 cases (`schema-create-and-migrate`, `update-delete-and-
readback`, `transaction-and-concurrency-atomicity`, `corrupt-state-
and-upgrade-failure`) were completely untouched even on already-
partly-credited domains. Added `update-delete-and-readback` for the 8
domains (Collections, CollectionItems, UserNotes, WishlistItems,
Contacts, ShareGrants, ShareGroups, ShareGroupMembers) the existing
`create-and-read-roundtrip` differential already covers, reading
persisted rows back directly from a real `DatabaseManager` after a
real PUT/DELETE dispatch. persistence-lifecycle moved 34 -> 42/798.
A follow-on batch added `update-delete-and-readback` for Searches
(real PUT/DELETE, though the creation response's `searchId` field
turned out not to round-trip through the store's own identifier
lookup -- worked around by reading the real persisted id back from
the store) and Conversations/PrivateMessages (real ack + real per-
user history delete, both targets), moving persistence-lifecycle to
48/798. Events and Transfers were investigated and deliberately
skipped for this case -- neither has a real update/delete HTTP path
reachable without bypassing dispatch or inventing a route.

`DatabaseManager::initialize()` runs `CREATE TABLE IF NOT EXISTS` for
every real table once, at construction, which makes `schema-create-
and-migrate` cheaply provable for every domain already touched: open
a brand-new in-memory database and call the domain's real `list_*`
accessor before writing any data at all -- a missing or broken schema
surfaces as a real SQL error, not an empty result, so this is genuine
executable evidence and not a route-presence shortcut. Credited across
all 13 domains touched so far (18 target/domain pairs: the 8 slskdN-
only domains from the collections/notes/wishlist/sharing batch, plus
Searches/Events/Conversations/PrivateMessages/Transfers for both
slskd and slskdn -- Events and Transfers get this case even though
they were skipped for `update-delete-and-readback` above, since schema
creation doesn't depend on a reachable update/delete HTTP path), moving
persistence-lifecycle to 66/798.

`transaction-and-concurrency-atomicity` credited next for 13 of those
same domains (all but Transfers, whose only "creation" route has no
registry entry in either frozen target): concurrent-create proofs
(N simultaneous creates of N distinct rows fired through the real
dispatcher against the same connection pool, then read back to assert
none were lost) for Collections/CollectionItems/UserNotes/
WishlistItems/ShareGroupMembers/Searches/Conversations/PrivateMessages/
Events, and concurrent-update proofs (N pre-seeded rows, N simultaneous
updates each with its own distinct value, then read back to assert
each row got its OWN writer's value and not a neighbor's) for
Contacts/ShareGrants/ShareGroups. Events has no HTTP route in either
registry, so it reused the same internal `record_event` call the
existing `create-and-read-roundtrip` differential already uses as its
real write path. Writing this surfaced and fixed a real test-harness
bug: `DatabaseManager::in_memory()`'s default multi-connection pool
against a `sqlite::memory:` DSN shares data across connections via
SQLite's shared cache mode, and shared cache mode's
`SQLITE_LOCKED_SHAREDCACHE` error on concurrent cross-connection table
writes is not retried by `busy_timeout` (a documented SQLite
limitation, distinct from `SQLITE_BUSY`) -- under real concurrent
access this produced spurious "database table is locked" 503s that
can't happen against the real single-writer file-backed database
`DatabaseManager::new()` actually uses in production. Fixed by pinning
`in_memory()` to a single pooled connection, which makes the pool
queue concurrent transactions instead of racing two live connections;
test-only, doesn't change production behavior. persistence-lifecycle
moved 66 -> 82/798.

`corrupt-state-and-upgrade-failure` credited next for 10 of the 13
domains (`ShareGroupMembers` excluded: composite primary key, no
single `id` column; `Transfers` excluded as always, no registry-backed
creation route): a real row is created, then one of its columns is
directly corrupted via a new raw-SQL test-only escape hatch (SQLite's
weak typing lets a non-numeric string land in an `INTEGER` column
without being rejected at write time -- a realistic stand-in for a
botched manual edit or a real schema-upgrade bug), then the real
`list_*` method the production startup path (`serve()`) itself uses
for rehydration is called and asserted to return a clean `Err`, not a
panic. This is the last of the 6 case names to be opened at all for
this workstream; all 13 already-touched domains now have every
applicable case proven (Events/Transfers/ShareGroupMembers each have
narrower, explicitly-documented per-case exclusions, not oversights).
persistence-lifecycle moved 82 -> 97/798.

The first file-lifecycle classifier slice is now explicit rather than
hardcoded: five slskdN cases are credited from real tests for
`Common/IO/AtomicFileWriter` and `Core/API/Controllers/OptionsController`
(nominal bytes, overwrite, and backup-symlink rejection). A second slice
credits four `Files/FileService` cases from a real directory-listing
differential for both targets: each configured downloads/incomplete root is
selected independently and returned file byte metadata matches. The remaining
file-writer domains and cases stay open; no generic source-name matching is
used.

A third slice credits three slskdN `Transfers/Downloads/DownloadService`
cases: completed-path selection, partial-file bytes/metadata, and resume versus
overwrite truncation. The same runtime assertions execute under both target
profiles, but only slskdN has this source subject in the frozen file-writer
inventory; slskd delegates those writes through `Files/FileService`.

The real VPN-status projection on the application route,
the real wire-command dispatch behind an interest mutation, the real
empty/disabled bridge-admin-clients baseline, one genuinely uncredited
route from a 17-route materialized-empty-state table, and real
adversarial-settings YAML persistence/readback brought the workstream
to 637/5,300.

The versioned discovery-graph route is now backend-wired rather than
request-seeded: `songid_run` requests resolve the persisted SongID run
store, and `artist` requests resolve slskR's existing local
MusicBrainz release-graph projection. The request-derived graph remains
only as an explicit no-backend-data fallback, with the response summary
identifying that condition. The focused contract test and differential
controller evidence cover both backend branches.

The relay-agent/controller subsystem is now implemented through the local
controller/agent boundary, but is not yet live-interop certified. The versioned
HTTP surface has target-specific credentials (PBKDF2-AES-Base62 for slskd and
PBKDF2-HMAC-SHA256-Base64 for slskdN), CIDR-bound agent registration,
expiring connection-bound request tokens, one-use upload tokens, confined
download serving, a raw SignalR JSON hub, outbound agent reconnect, share
snapshot upload, target-specific validated SQLite share repositories,
file-info callbacks, concurrent streamed multipart forwarding, length-integrity
checks, download completion handling, and restart rehydration of an uploaded
foreign share database into a durable remote host projection. The remaining
relay debt is a live cross-client run. The frozen oracle's complete
implementation is 2,880 lines (`RelayHub.cs`/`RelayService.cs`/`RelayClient.cs`/`RelayController.cs`).

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
protocol-behaviors: 4 -> 70/1465. The server-family differential now maps
85 of the 90 frozen server units for both targets, including typed coverage
for the private-room, recommendation, room-ticker, distributed-configuration,
and embedded-message families -- protocol-behaviors: 136 -> 236/1465. The
remaining five server units have no standalone wire implementation in the
frozen runtime (`SendSpeed`, `QueuedDownloads`, `ExactFileSearch`,
`PrivateRoomUnknown`, and `RelatedSearch`); the slskdN-only protocol
extensions still need new codec/behavior coverage.

The next protocol pass added 18 real slskdN extension/overlay proof rows
through the existing client codecs, seven mesh-service exact-frame rows
through the existing service-call codec, then added eight exact-frame
virtual-Soulfind bridge rows through the real bounded TCP bridge codec. The
bridge proof covers the eight message types slskR actually dispatches
(`Login` through `RoomListResponse`); the frozen enum's `RoomJoinRequest`
through `FileTransfer` values remain uncredited because slskR does not
implement handlers for them. Protocol-behaviors moved from 236 to 269
complete cases.

| Workstream | Audited denominator | Current evidence | Closure state |
| --- | ---: | --- | --- |
| Configuration | 436 frozen YAML leaves | 436 complete, 0 partial, 0 missing | **Closed** (2026-07-29) |
| slskd controller API | 96 routes / 656 cases | 656 complete, including route presence and behavior | **Closed** (2026-08-16) |
| slskdN controller API | 683 routes / 4,704 cases | 4,704 complete, including route presence and behavior | **Closed** (2026-08-16) |
| Frozen WebUI API calls | 417-call union / 2,085 cases | All call-presence, success, empty/loading, validation/error, and authorization/reconnect cases complete | **Closed** (2026-08-16) |
| Soulseek and adjacent protocols | 1,465 proof cases | 1,465 complete | **Closed** (2026-08-16) |
| Persistence and lifecycle | 732 proof cases | 732 complete, including explicit not-applicable frozen contracts | **Closed** (2026-08-16) |
| Security authorization | 7,790 credential-profile proof cases | 7,790 complete via exhaustive live-dispatch differential | **Closed** (2026-08-16) |
| Security controls | 798 proof cases | 798 complete via exact frozen-component evidence | **Closed** (2026-08-16) |
| Packaging and operator behavior | 240 proof cases | 240 complete via exact frozen artifact evidence and explicit scope | **Closed** (2026-08-16) |
| Bidirectional interoperability | 310 frozen proof cases | 310 retained ledger rows; strict 11/11 transport/lifecycle artifact passes, including the fresh source-bound mesh retry/failure supplement | **Closed** (2026-08-20) |

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

The 6,730 `needs-proof` cases are not presumed absent. They are certification
dimensions to be linked in bulk from the subsystem contract matrices now that
the known configuration-layer implementation gaps are closed.

The 2026-08-01 transport pass added the frozen slskdN QUIC control wire to the
Rust client and daemon: `slskdn-overlay` ALPN, bounded bidirectional streams,
MessagePack envelopes, public-key-value pin validation, and Pod routing for
explicit non-shared-port pins. The 2026-08-17 pass adds the separate
`slskdn-overlay-data` ALPN, bounded raw payload streams, TLS 1.3/SPKI pin
validation, daemon receive wiring, reusable bidirectional streams, bounded
public-QUIC proxy admission, and the frozen `AUTH`/`RELAY_TCP` framing with
destination, byte, duration, and concurrency limits. Shared DHT/UDP
demultiplexing, shared-port proxy wiring, application-level data delivery, and
live cross-runtime QUIC interoperability remain open certification work.

The raw proof-case closure ratio is a certification-ledger metric only. It is
not used as an implementation queue or as a product-completion estimate. Work
is selected and closed by vertical subsystem so one implementation and one
generated differential matrix can satisfy all affected route, configuration,
protocol, persistence, UI, and interoperability cases together.

The retained evidence-only recheck materializes 19,216 unique frozen proof
cases across all workstreams as complete, with zero denominator-missing cases.
That result is not the universal replacement gate: retained WebUI artifacts
are not live-backend UX proof, and target-local `not-applicable` rows do not
prove every end-user transport or deployment path.

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

**2026-08-12 prior measured checkpoint (before the discovery-graph and mesh-stream edge batch)**: the frozen audit
completed all focused Rust differential suites and the browser route audit.
The mesh-sync evidence now covers exact frames and malformed inputs for all
nine numeric message types, canonical Ed25519 message signing and freshness
verification, hash-entry signatures, and a real daemon private-message exchange
covering Hello, delta export, key lookup, bounded chunk reads, and pushed-entry
acknowledgement. The versioned `/api/v0/mesh/message` controller now also
dispatches signed Hello, ReqDelta, PushDelta, ReqKey, and ReqChunk requests
through the same runtime, with mutation/readback and malformed/unsigned cases.
Its differential also proves missing query/body validation, failed chunk reads,
concurrent dispatch, and restart persistence. The mesh HTTP gateway differential
proves malformed route rejection, real pod-service mutation/readback, concurrent
joins, and restart persistence; the React Network page exercises a successful
mesh sync action against a populated peer.
The mesh-stream ticket controller now matches the frozen MeshStreamsController
for exact response keys and content types, identifier/filename/hash/size/audio
validation, disabled and missing states, ticket readback, and the frozen
1,000-ticket limit response. The same controller differential now proves the
frozen empty mesh-health snapshot and a populated mesh-transport response with
an actual open overlay tunnel, including exact keys and live counts. The React
route audit completed all 41 routes at desktop and mobile viewports with zero
browser errors. The follow-on controller differential proves concurrent and
idempotent mesh merge/publish behavior, restart persistence for both stores,
and the frozen 400 JSON failure shape for unsupported and transport-unavailable
mesh sync.
The security differential then adds executable proof for both target-specific
blacklist semantics and slskdN JWT-revocation persistence, while the mesh
transport security ledger covers rate/DHT quotas, replay and connection
guards, certificate pins, endpoint selection, transport policy, and redacted
events. The authentication differential also covers both targets' shared
API-key authentication and administrator-JWT services for default activation,
nominal acceptance, tamper rejection, secret-free output, and rotation/reload;
request quota/lockout remains open because it belongs to a separate limiter
component. The file-lifecycle ledger proves durable certificate-pin creation,
overwrite, confinement, and restart retention; cancellation cleanup remains
open. The atomic file-writer ledger additionally proves automatic parent
creation, confined replacement of a destination symlink, and reload retention.
The persistence differential additionally covers the slskdN
WishlistIgnoredResults entity, PodCore Pods, Members, MembershipRecords, and
Messages, the slskd Transfers Batches table, and the slskdN HashDb and
HashDbState tables through real route writes, SQLite restart rehydration,
atomic snapshots, concurrency, cleanup, and corrupt-state rejection. The Gold
Star revocation marker now has
matching path, byte, overwrite, and restart file evidence. These additions
move security-controls to 104/798 and persistence-lifecycle to 164/798. The
frozen controller route scanner now honors action-level `[Route("...")]`
templates, so the audit inventory is 96 slskd routes and 683 slskdN routes
rather than the previous incomplete 91/678 inventory. The controller gate
re-ran against both frozen trees and the live slskR daemon with zero generic
404s, HTML fallbacks, compatibility fallbacks, or probe errors. The corrected
inventory adds the real options startup/debug/YAML/validation actions and their
authorization proof rows; the follow-on differential then exercised those
actions against both targets, including malformed and disabled states, YAML
mutation/readback, concurrent validation, and reload retention. No behavior
was credited from the scanner correction alone.
The HashDb controller differential then added 46 previously open slskdn
controller cases: the frozen verification request `{filename,size,byteHash}`
shape and derived FLAC key, populated inventory projections, schema version,
idempotent mesh merge, restart rehydration, optimizer lifecycle, and closed-
SQLite read/write failure contracts. All HashDb controller cases are now linked
to executable evidence, including the POST-hash restart-lifecycle dimension.
The slskd runtime-failure differential then closed the final 24 slskd
controller cases. It proves closed-SQLite failures for transfer download
detail, queue position, and upload detail, while unrelated application,
session, options, rooms, server, shares, telemetry, browse, and control
routes retain their frozen success/error contracts. The DELETE-shares case
also proves the frozen 204 response while a share scan is held and the
database is closed. All 24 rows passed in the isolated differential ledger.
The follow-on slskdN SecurityController runtime-failure differential closed
31 more rows. It verifies the frozen process-local security projections under
a closed SQLite pool, ban create/delete rollback and `503` responses,
versioned unavailable transport responses, the disabled adversarial `404/500`
contracts, and non-persistent disclosure/reputation updates. All 31 rows
passed, and the full controller suite now runs 194 tests with no failures.
The slskdN TransfersController runtime-failure differential then closed 26
more rows: all 24 database-backed transfer projections and mutations return
the frozen closed-SQLite `500` contract, while the process-local accelerated
download GET/PUT pair remains available with `200` responses. All 26 rows
passed, and the full 195-test controller suite passed again.
The residual slskd core differential then added 20 previously open controller
cases with real session-enabled and administrator-JWT readback, non-mutating
YAML validation, atomic YAML update/concurrency and filesystem-failure
behavior, volatile option serialization, application restart idempotency,
share-scan rollback/busy handling, and room-subresource missing/idempotent
contracts. The full 195-test controller suite and the aggregate frozen audit
both passed after this batch.
The follow-on slskdN TransfersController edge differential then closed 59
more controller cases. It covers empty and missing transfer state, malformed
IDs, usernames, query values, and request bodies, populated batch and queue
position projections, auto-replace/find-alternative/replace mutations,
accelerated-download state and readback, and auto-replace status. The full
198-test controller suite passed with no failures; that checkpoint's
authoritative frozen audit reported 12,693/19,282 complete.
The transfer restart/concurrency differential then closed 18 more cases:
download and upload cancellation persist across rehydration, repeated
cleanup and mutation calls remain idempotent, semaphore saturation preserves
the frozen `429` boundary without state mutation, and process-local
accelerated-download state resets across runtime reconstruction. The full
199-test controller suite and refreshed authoritative audit passed.
The slskdN AutoReplaceController edge differential then closed 11 more cases:
status-query and empty-state reads, enable/disable DTOs, malformed requests,
closed-SQLite behavior, toggle persistence, and repeated mutation calls. The
toggle follows the frozen process-local state-file contract while retaining a
best-effort SQLite mirror. The follow-on OptionsController edge differential
closed 14 more cases: malformed and invalid-current reads, startup/location
availability under closed SQLite, invalid-options failures for snapshot-backed
actions, non-mutating YAML validation and reset-on-restart, malformed and
missing YAML updates, and concurrent YAML replacement. The versioned debug,
location, and YAML-update handlers now expose the frozen invalid-options failure
contract. The full 201-test controller suite and refreshed authoritative audit
passed.
The Soulseek initialization, distributed, structured peer, and 85 implemented
server-message units now also have executable truncated/oversize/unknown input
proof. Opaque peer payload units and the five frozen server inventory units with
no standalone runtime wire implementation remain open rather than being
classified as rejected malformed inputs.
The ledger reports **12,801/19,282 complete (66.36%)**:

| Workstream | Complete | Total |
| --- | ---: | ---: |
| configuration | 436 | 436 |
| security-authorization | 7,790 | 7,790 |
| persistence-lifecycle | 164 | 798 |
| protocol-behaviors | 541 | 1,465 |
| slskd-controller-api | 656 | 656 |
| slskdn-controller-api | 2,616 | 4,704 |
| webui-workflows | 494 | 2,085 |
| security-controls | 104 | 798 |
| operator-packaging | 0 | 240 |
| live-interop | 0 | 310 |

The remaining **6,481** cases are still open under the zero-gap completion
rule. The previous checkpoint follows for historical context.

**2026-08-08 historical measured checkpoint (batch172)**: the frozen controller
route gate now probes all 91 slskd and 678 slskdN routes serially against a
live slskR daemon. It previously hid probe failures and could trigger a worker
stack overflow on the monolithic dispatcher; the probe now fails on transport
errors and the daemon uses an 8 MiB Tokio worker stack for that dispatcher.
All 769 `route-presence` cases pass. Batch105 added 17 executed slskdN PodCore
malformed-route-value contracts; batch106 added nine executed slskdN PodCore
request-validation contracts; batch107 added four executed slskdN PodCore empty
opinion aggregate/statistics/action contracts; batch108 adds seven executed
slskdN PodCore missing-empty opinion/action contracts; batch109 adds eight
executed slskdN PodCore backfill last-seen, DHT metadata, pending-membership,
and routing-seen contracts; batch110 adds one executed slskdN PodCore populated
routing-seen contract; batch111 adds 15 executed slskdN PodCore membership
retrieval/verification and moderation publication contracts; batch112 adds two
executed slskdN PodCore membership self-publish nominal contracts; batch113 adds
six executed slskdN PodCore populated opinion getter contracts; batch114 adds
one executed slskdN PodCore nominal routing contract; batch115 adds three
executed slskdN PodCore opinion publication nominal, mutation/readback, and
missing-field validation contracts; batch116 adds two executed slskdN PodCore
populated opinion refresh and member-affinity update mutation/readback
contracts; batch117 adds one executed slskdN PodCore signing-verification
mutation/readback contract; batch118 adds one executed slskdN PodCore empty
channel-list contract; batch119 adds one executed slskdN PodCore join-accept
mutation/readback contract; batch120 adds one executed slskdN PodCore leave
mutation/readback contract; batch121 adds two executed slskdN PodCore missing
leave-request and missing leave-acceptance contracts; batch122 adds two executed
slskdN PodCore duplicate-join and repeated-membership-removal contracts; batch123
adds three executed slskdN PodCore membership-stats malformed-query and
membership-cleanup malformed/empty-request contracts; batch124 adds one
executed slskdN PodCore membership-cleanup concurrency/idempotency contract;
batch125 adds one executed slskdN jobs-list nominal contract; batch126 adds five
executed slskdN versioned Lidarr, listening-party, and MusicBrainz nominal
contracts; batch127 adds one executed slskdN release-radar notification-list
populated-state contract; batch128 adds five executed slskdN quarantine-jury
request-list, request-detail, and route-list readback contracts; batch129 adds
two executed slskdN MusicBrainz overlay release-graph nominal and populated-
state contracts; batch130 adds five executed slskdN MusicBrainz overlay route
malformed, backend-unavailable, persisted-attempt, and route-list readback
contracts. Batch131 adds four executed slskdN MusicBrainz release-radar route
missing-notification, backend-unavailable, persisted-attempt, and empty-route-
list contracts. Batch132 adds two executed slskdN quarantine-jury route
missing-request and persisted-failed-attempt readback contracts. Batch133 adds
two executed slskdN source-provider populated-state projections for the
unversioned and versioned routes. Batch134 adds two executed slskdN
security-reputation empty suspicious/trusted list projections. The full
manifest now reports **11,215/19,122 complete (58.65%)**. Batch135 adds three
executed slskdN unversioned Spotify status empty, nominal, and populated-state
projections. Batch136 adds two executed slskdN PodCore content metadata/search
missing-query projections. The full manifest now reports
**11,220/19,122 complete (58.68%)**. Batch137 adds five executed slskdN
unversioned source-feed history empty/populated list and missing/nominal/
populated detail projections. Batch138 adds two executed slskdN PodCore
empty DHT-publication-stats and empty discovery-registration-stats projections.
The full manifest now reports **11,227/19,122 complete (58.71%)**. Batch139
adds seven executed slskdN PodCore stats malformed-query projections across
backfill, DHT, discovery, messages, routing, signing, and verification. Batch140
adds 12 executed slskdN MediaCore empty-state projections for the ContentID,
IPLD, descriptor-retrieval, publishing, dashboard, and individual statistics
routes. Batch141 adds 12 executed slskdN MediaCore malformed-query
projections across those same statistics routes. The full manifest now reports
**11,258/19,122 complete (58.87%)**. Batch142 adds 14 executed slskdN
MediaCore resource malformed-query
projections across ContentID, IPLD, descriptor retrieval/query, and static
algorithm/strategy routes. The full manifest now reports
**11,272/19,122 complete (58.95%)**. Batch143 adds five executed slskdN
MediaCore empty-state projections for IPLD traversal, descriptor retrieval,
and static algorithm/strategy routes. The full manifest now reports
**11,277/19,122 complete (58.97%)**. Batch144 adds five executed slskdN
MediaCore query-by-domain nominal/populated and statistics dashboard,
registry, and IPLD populated-state projections. The full manifest now reports
**11,282/19,122 complete (59.00%)**. Batch145 adds three executed slskdN
MediaCore fuzzy, perceptual, and portability statistics populated-state
projections. Batch146 adds nine executed slskdN MediaCore malformed path/query/
body projections for fuzzy matching, perceptual hashing, metadata portability,
and retrieval verification. Batch147 adds 18 executed slskdN MediaCore
request-validation and supported-algorithm/strategy populated-state projections.
Batch148 adds six executed slskdN MediaCore portability-export, batch-publish,
and republish nominal/readback projections. Batch149 adds nine executed slskdN
MediaCore empty-request and malformed-body/path projections across fuzzy-find,
cache-clear, statistics reset, batch retrieval/publishing, descriptor update,
and descriptor deletion. The full manifest now reports
**11,327/19,122 complete (59.24%)**. Batch150 adds two executed slskdN
MediaCore descriptor-update nominal failure and no-side-effect readback
projections. Batch151 adds 12 executed slskdN unversioned bridge API-version
validation projections across search, download, start, and stop. Batch152 adds
33 executed slskdN unversioned mutating-route API-version validation projections
across analyzer migration, jobs, library health, source-feed preview, Spotify,
bridge configuration, and issue patch routes; 32 are newly credited because one
case was already complete. Batch153 adds four executed slskdN swarm-analytics
query-validation projections; three unique manifest cases are newly credited
because both invalid dashboard queries share one case key. Batch154 adds five
executed slskdN compatibility-tail projections for the profile, share-grant
token, disabled relay-controller download, disabled mesh-rendezvous users, and
disabled STUN/NAT detection routes; four are newly credited because the
share-grant token case was already complete. The full manifest now reports
**11,380/19,122 complete (59.51%)**. Batch155 adds four executed slskdN
runtime-switch projections: populated SignalSystem configuration/status and
disabled mesh-stats/DHT-peer guards. All four are newly credited; the full
manifest now reports **11,384/19,122 complete (59.53%)**. Batch156 adds one
executed slskdN populated VirtualSoulfind bridge-admin configuration projection,
newly credited. The full manifest now reports **11,385/19,122 complete
(59.54%)**. Batch157 adds one executed slskdN populated versioned
application-state projection for `GET /api/v0/application`, newly credited.
The full manifest now reports **11,386/19,122 complete (59.54%)**. Batch158
adds one executed slskdN populated versioned bridge-client projection for
`GET /api/v0/bridge/admin/clients`, newly credited. Batch159 corrects the
slskdN bridge-room DTO to the frozen `{ rooms: [{ name, memberCount }] }`
contract and adds one executed populated projection for
`GET /api/v0/bridge/rooms`, newly credited. The full manifest now reports
**11,388/19,122 complete (59.55%)**. Batch160 adds one executed slskdN
populated versioned auto-replace status projection for
`GET /api/v0/autoreplace` after a real enable/readback mutation, newly
credited. Batch161 adds 40 executed slskd compatibility contracts covering
downloads/incomplete file roots and nested listings, file and directory deletion
side effects, transfer queue/detail/batch lifecycles, and room nominal and
missing-resource behavior. It also aligns missing storage-directory listings
with the frozen `404 Not Found` contract. All 40 are newly credited; the full
manifest now reports **11,429/19,122 complete (59.77%)**. Batch162 adds 38
newly credited slskd core application, session, event, and telemetry report
contracts, including populated-state projections, mutation readback, filtered
report windows, and empty/error behavior. Batch163 adds 56 newly credited
slskd users, shares, rooms/conversations, and server lifecycle contracts,
including target-specific DTO/status behavior, populated readback, mutation
lifecycle, and missing-state handling. The full manifest now reports
**11,523/19,122 complete (60.26%)**. Batch164 adds 34 newly credited slskd
search, upload, and user-browse lifecycle contracts, including populated
projections, target-specific cancel/delete statuses, missing/malformed handling,
and cleanup readback. The full manifest now reports **11,557/19,122 complete
(60.44%)**. Batch165 adds 13 newly credited slskd download lifecycle edge
contracts covering per-user and batch projections, queue-position readback,
malformed identifiers, and completed-download cleanup. The full manifest now
reports **11,570/19,122 complete (60.51%)**. Batch166 adds 7 newly credited
slskd options overlay contracts covering redacted current state, valid and
invalid overlays, remote-configuration denial, volatile readback, and restart
reset. Batch167 adds 71 newly credited slskd controller contracts covering
search create/update/delete persistence and rollback, room subscription and
conversation restart/idempotency/failure behavior, transfer batch and terminal
cleanup persistence, and individual download/upload cancellation rollback.
The full manifest now reports **11,648/19,122 complete (60.91%)**. Batch168
adds 11 newly credited slskd controller contracts for nested incomplete-file
readback, missing storage-directory/file deletion, populated application
version, logs, session, room roster, and available-room projections. The full
manifest now reports **11,659/19,122 complete (60.97%)**. Batch169 adds 11
newly credited slskd controller contracts for enabled/disabled relay-agent
lifecycle, persistence-failure rollback, repeated relay operations, and
share-scan cancellation/reset. Batch170 adds 27 newly credited slskd controller
contracts for invalid recursive file queries, fixed-route malformed-path
fallthrough, repeated server connect/disconnect behavior, and malformed server
disconnect-body validation. The full manifest now reports
**11,697/19,122 complete (61.17%)**. Batch171 adds 45 newly credited slskd
controller contracts for parameterized route-shape rejection, invalid boolean
query handling, and malformed transfer, room, conversation, user, share,
telemetry, search, and relay paths. The full manifest now reports
**11,742/19,122 complete (61.41%)**. Batch172 adds 39 newly credited slskd
controller contracts for empty/missing-state behavior across application,
conversation, file, room, search, server, session, share, telemetry, transfer,
user, and relay routes, plus relay-controller download/upload/share
nominal, populated, mutation/readback, and replay/idempotency behavior. The
full manifest now reports **11,781/19,122 complete (61.61%)**:

| Workstream | Complete | Total |
| --- | ---: | ---: |
| configuration | 436 | 436 |
| security-authorization | 7,690 | 7,690 |
| persistence-lifecycle | 110 | 798 |
| protocol-behaviors | 289 | 1,465 |
| slskd-controller-api | 504 | 626 |
| slskdn-controller-api | 2,259 | 4,674 |
| webui-workflows | 493 | 2,085 |
| security-controls | 0 | 798 |
| operator-packaging | 0 | 240 |
| live-interop | 0 | 310 |

This checkpoint also promotes the passing controller contracts into
the frozen evidence ledger: library-health/job projections, Lidarr and
source-feed history, transfer reports including populated exceptions/Pareto,
search restart rehydration, wishlist
CSV idempotency, opinion creation readback, opinion deletion status,
security-ban readback, now-playing state transitions, wishlist deletion
readback, overlay blocklist state, now-playing nominal/populated coverage,
pod detail/update coverage, pod-channel message readback, content-linked pod
readback, ignored-result list/delete readback, wishlist item reads, and
mark-viewed persistence, pod-channel creation readback, username blocklist
create/unblock status, populated port-forward status, username-ban
create/delete readback, four content-ID nominal GETs, populated
port-forward-list status, ContentID resolve/validation/stats coverage,
IPLD graph/traverse populated-state coverage, transfer list/changes/history
populated-state coverage, search detail nominal/populated-state coverage,
search duplicate-ID idempotency and missing/conflict validation, playback
feedback priority/readback/reset, content-bound stream populated state, browse
root/status projections, joined-room
detail/roster projections, Soulseek user-interests projections, library
case-filter projection, activity/unacknowledged and rooms/activity
populated-state projections, network-stats populated-state projection,
PodCore content metadata nominal/populated projections, PodCore content search
nominal/populated projections, PodCore DHT metadata/stats projections,
PodCore DHT refresh nominal response, PodCore discovery-name empty/populated
projections, PodCore discovery all/content empty/populated projections,
PodCore discovery validation and empty-state projections,
PodCore discovery-stats populated projection, PodCore stats
empty-state projections, MediaCore ContentID/IPLD/retrieval/publishing and
statistics empty-state and malformed-query projections, MediaCore resource
malformed-query and static-route empty-state projections, MediaCore
descriptor-query nominal/populated and dashboard/registry/IPLD statistics
populated-state projections, MediaCore fuzzy/perceptual/portability statistics
populated-state projections, MediaCore fuzzy-matching, perceptual-hash,
portability, and retrieval-verification malformed path/query/body projections,
MediaCore request-validation and supported-algorithm/strategy populated-state
projections,
MediaCore portability-export, batch-publish, and republish nominal/readback
projections,
MediaCore empty-request and malformed-body/path projections across fuzzy-find,
cache-clear, statistics reset, batch retrieval/publishing, descriptor update,
and descriptor deletion,
MediaCore descriptor-update nominal failure and no-side-effect readback
projections,
unversioned bridge API-version validation projections for search, download,
start, and stop, unversioned mutating-route API-version validation projections
for analyzer migration, jobs, library health, source-feed preview, Spotify,
bridge configuration, and issue patch routes,
swarm-analytics malformed-query projections for dashboard, peer rankings, and
trends, compatibility-tail profile/share-grant-token projections, and disabled
relay-controller, mesh-rendezvous, and STUN/NAT guard projections,
SignalSystem populated configuration/status projections, and disabled mesh-stats
and DHT-peer guards, and populated VirtualSoulfind bridge-admin configuration
and versioned application-state, bridge-client, and bridge-room projections,
plus the 40 slskd file, transfer, and room contracts from batch161, the 38
slskd application, session, event, and telemetry contracts from batch162, and
the 56 slskd users, shares, rooms/conversations, and server lifecycle contracts
from batch163, and the 34 slskd search, upload, and user-browse lifecycle
contracts from batch164, and the 13 slskd download lifecycle edge contracts
from batch165, and the 7 slskd options overlay contracts from batch166,
PodCore channel CRUD validation/system-channel guards/
readback projections, PodCore message-cleanup nominal projections, PodCore
opinion/recommendation/affinity empty-state, aggregate/statistics,
refresh/update action, and missing-empty projections, PodCore backfill
last-seen nominal/populated and stats empty-state projections, PodCore DHT
metadata nominal projection, pending membership empty-state projections, and
routing-seen nominal/missing/populated projections, PodCore discovery
tag/multi-tag empty/populated projections, PodCore membership join/leave
cancellation lifecycle projections, PodCore message count/search empty/
populated/no-result/validation projections, PodCore membership-verification
DTO, banned-membership, and role-hierarchy/missing-member projections,
PodCore membership retrieval/verification signed-record and moderation
ban/unban/role publication projections,
PodCore membership self-publish nominal projections,
PodCore populated opinion/variant/aggregate/recommendation/statistics and
member-affinity projections,
PodCore opinion publication response/readback and missing-field validation
projections,
PodCore populated opinion refresh and member-affinity update projections,
PodCore signing-verification counter/readback projection,
PodCore empty channel-list projection,
PodCore membership join-accept mutation/readback projection,
PodCore membership leave mutation/readback projection,
PodCore missing leave-request and leave-acceptance projections,
PodCore duplicate join and repeated membership-removal projections,
PodCore membership-stats malformed-query and membership-cleanup malformed/
empty-request projections, PodCore membership-cleanup
concurrency/idempotency projection, slskdN jobs-list nominal projection,
versioned Lidarr missing-items, listening-party, MusicBrainz completion,
artist-coverage and release-radar nominal projections, release-radar
notification-list populated projection, and quarantine-jury request-list,
request-detail, and route-list readback projections, and MusicBrainz overlay
release-graph nominal/populated projections, and MusicBrainz overlay route
malformed, backend-unavailable, persisted-attempt, and route-list readback
projections, release-radar route missing-notification, backend-unavailable,
persisted-attempt, and empty-route-list projections, quarantine-jury route
missing-request and persisted-failed-attempt readback projections, source-
provider populated-state projections,
PodCore nominal routing projection,
ActivityPub collection empty/unknown-actor, WebFinger missing, and populated
inbox/outbox projections, malformed outbox-page validation, oversized
verification-route validation, and PodCore DHT metadata/refresh/unpublish
blank-ID validation,
PodCore backfill/message-count/membership route-value validation, pending
membership/routing-seen and channel/discovery/message-cleanup/backfill
mutation route-value validation, channel-read and opinion/affinity route-value
validation, PodCore join/leave acceptance validation, routing-to-peers
validation, signing/verification validation, and opinion action validation,
PodCore DHT/discovery maintenance
malformed and
missing-resource projections, PodCore backfill sync nominal/mutation
projections, PodCore backfill-stats populated projection, user-notes
empty/populated/validation/lookup/idempotent-delete/persistence/concurrency
projections, share-grants
collection/CRUD edge projections, collections/items CRUD nominal/populated/edge/
missing-state/
persistence/concurrency projections, sharegroups/member CRUD nominal/populated/empty/missing/
edge/runtime/persistence/concurrency projections, share-grants
runtime/restart/concurrency projections, contacts versioned CRUD
malformed/nominal/mutation/restart/concurrency plus list/item/nearby
empty/populated projections, library-health versioned/empty/error alias
projections, now-playing delete reset/concurrency and PUT reset/concurrency,
playback diagnostics nominal/malformed, versioned options current/overlay
nominal, populated readback, volatile reset, validation-failure, and
concurrency projections, application version/latest nominal and populated
state plus build populated-state projections, populated server-status and
slskdN-capability projections, versioned capability peer-list, mesh-peer-list,
and peer-detail projections, and versioned destinations list/default
populated-state projections, versioned mesh hello and mesh/hashdb lookup and
hash-by-size populated-state projections, bridge-admin config/clients/rooms/
status/stats/dashboard versioned GET projections, versioned backfill
stats/config and empty room-directory GET projections, versioned telemetry
metrics and KPI projections, versioned mesh-health and signal-system
configuration/status projections, hashdb key and no-partial discovery-count
projections, nominal DHT status projections, versioned auto-replace status,
empty hash-by-size/inventory-by-size projections, versioned Soulseek
recommendation/global-recommendation, item-discovery, similar-user,
capability-peer nominal, destination list/default, backfill-candidate, and
multisource job-list, download-request list/detail/rename/cancel, transfer
detail, queue-position, transfer-batch lookup (nominal/populated/malformed/
missing), accelerated and stuck-download projections, upload diagnostics,
download/upload-cancel projections, and library-health remediation/patch
restart persistence,
swarm-trends, swarm-dashboard, transfer-history,
upload-list, stuck-download-list, transfer-change, transfer-summary, and
transfer-histogram projections, including populated transfer-summary and
transfer-histogram state, populated global and user-scoped transfer-directory
grouping, joined-room listing and join readback, populated user endpoint/status,
and populated share
root/content projections, populated search-list readback, and
source-provider catalog projections, security-reputation empty suspicious/
trusted list projections, unversioned Spotify status empty/nominal/populated
projections, PodCore content metadata/search missing-query projections,
unversioned source-feed history empty/populated list and missing/nominal/
populated detail projections, PodCore empty DHT-publication-stats and
discovery-registration-stats projections, PodCore stats malformed-query
projections, Mesh merge
nominal/mutation and delta
populated projections,
Mesh stats nominal/populated projections, adversarial-settings nominal response
shape, and peer/mesh stream-ticket nominal, populated, and missing projections.
The
full controller differential prefix and the deterministic WebUI audit pass;
the newly linked cases are slskdN-only where the frozen route subject is
slskdN-specific.

The ContentID registry now supports versioned external-ID resolution with
durable registry readback and explicit not-found responses, matching the
frozen slskdN controller contract. The IPLD graph and link-filtered traversal
endpoints now recursively project persisted link nodes, paths, and visited
state, matching their frozen controller contracts. The discovery-graph versioned endpoint now reads persisted SongID runs and
the local MusicBrainz release graph, with an explicit fallback only when those
backends have no usable data. Relay now has a tested local controller/agent
data plane: target-specific credential profiles, authenticated SignalR login,
share snapshot upload with target-specific SQLite validation, file-info and
file-upload callbacks, concurrent multipart file forwarding, outbound TLS SPKI
pinning, length-integrity checks, download completion handling, and durable
foreign-database restart rehydration. Live cross-client certification remains
open.

The nine slskdN `mesh-sync` message units now have a typed codec for the
frozen numeric-type, snake_case JSON payloads and the `MESH:<TYPE>:<JSON>`
private-message envelope. Exact-frame round trips pass for Hello, ReqDelta,
PushDelta, ReqKey, RespKey, Ack, ReqChunk, RespChunk, and DhtStore. The codec
also matches the frozen canonical-signing behavior, including freshness checks
and hash-entry signatures. The daemon runtime now proves signed dispatch,
database side effects, bounded chunk reads, and acknowledgements; frozen-target
live interoperability and reconnect/retry cases remain open.

The eight slskdN `virtual-soulfind-bridge` message units now also have
dispatcher evidence through the real bridge client loop: login, search,
download queueing, and room-list requests produce the frozen response frames,
with the download path proving its real session-command side effect. Malformed
login, search, and download requests are rejected by the live bridge handler.
Timeout/reconnect/live interop cases remain open.

The WebUI workflow audit now exercises up to twelve non-destructive visible
controls per route at desktop and mobile viewports and covers all 41 declared
React routes, including every system tab. The broadened sweep added
rendered-success evidence for the MusicBrainz release-radar notifications and
subscriptions calls plus the system-tab Bridge, MediaCore, integration, mesh,
job, quarantine, source-provider, swarm-analytics, and metrics endpoints; the
audit remains clean across all 41 routes.

The `/users` and `/browse` audit routes now seed only deterministic fixture
state for an active `commons_peer` user and saved browse tab. This exercises
the real user status/endpoint and browse note/status/data workflows without
entering a polling loop or sending a live browse request.

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
all 96 slskd and 683 slskdN routes. Drive all 417 frozen WebUI call workflows
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
