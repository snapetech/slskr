# slskdN and slskNet.Runtime Parity Ledger

Date: 2026-05-13

This ledger tracks the work required for `slskR` to reach full feature,
function, and security parity with the sibling repositories:

- `../slskdn`
- `../slskNet.Runtime`

`slskR` remains a Rust-native implementation. Parity means the externally
visible behavior, protocol semantics, security posture, API shapes, UI
workflows, and operator guarantees match unless an entry is explicitly marked
as a compatibility acknowledgement or intentionally Rust-native.

## Baseline

- `slskR` scaffold baseline: `f01cdc30`, 2026-04-30.
- `slskR` WebUI port baseline: `6b0809cd`, 2026-05-04.
- Current endpoint shape result: `scripts/check-endpoint-parity-drift.sh`
  reports `290 / 290` canonical WebUI endpoints implemented.
- `slskNet.Runtime` comparison baseline: fork changes since Soulseek.NET
  `10.0.0`, plus local commits after the `slskR` scaffold date.
- `slskdN` comparison baseline: local commits after the `slskR` scaffold date.

Endpoint coverage is not sufficient for closure. A route is complete only when
its response shape, mutation semantics, auth/CSRF/rate-limit posture, UI usage,
and regression coverage match the parity target.

## Current Status

| Area | Status | Required closure |
| --- | --- | --- |
| WebUI endpoint shape | Implemented | Keep `scripts/check-endpoint-parity-drift.sh` green. |
| Core Soulseek protocol codes | Implemented/needs-live-proof | Server, peer, init, and distributed code tables have uniqueness/completeness inventory tests, known-value mappings, unknown-code preservation, bounded-count guards, and round-trip coverage. Remaining work: optional live interop proof against upstream daemons. |
| Type-1 obfuscation | Implemented/needs-live-proof | Rotated type-1 framing, obfuscated listener demux for `P`, `D`, and `F`, obfuscated peer/file-transfer preference, and plain fallback for failed obfuscated browse/file-transfer attempts are covered locally. Remaining work: optional live interop proof. |
| Search, browse, transfer projections | Partially implemented | Download retry, resume-state preservation, rooted download-path validation, transfer progress/failure projection, peer queue-position projection, stuck-download, per-user, speed, download stats, and accelerated-download compatibility reports, local peer transfer tests, slskd-style public vs locked/private search response projection, paged search-detail result arrays, paged/filtered search response peer groups, SQLite-backed search result restart hydration, search lifecycle update projection including caller-supplied TTL bounds, search-backed transfer alternative replacement and auto-replacement, and paged/filtered slskd-shaped user browse/directory projections are covered. Remaining work: broader browse/search parity plus optional live interop. |
| Rooms, messages, and social projections | Implemented/needs-live-proof | Room creation failure projection, reconnect-aware room join `503` behavior, client/protocol/API multi-user message sends, sharegroup-backed user group projection, and durable message/room persistence tests are implemented. Remaining work: optional live interop proof against upstream daemons. |
| slskdN peer capabilities | Implemented/needs-live-proof | Signed descriptor model, canonical Ed25519 verification, registry expiry, reserved peer-message exchange, fail-closed parsing, hello/ack behavior, daemon capability inventory, and mesh consumers are covered. Remaining work: optional live interop proof. |
| Mesh rendezvous | Daemon passive discovery implemented/needs-live-proof | `slskr-client` publishes the `slskdn-mesh-v1` interest tag, dedupes similar/capability candidates, keeps active probing default-off, and can send signed capability hello probes when explicitly enabled. The daemon API now exposes passive rendezvous status, discovery users, capability records, mesh peer stats, and compatibility peer lists derived from watched/similar users plus mesh-capable descriptors. Remaining work: optional live interop proof and any future active-probe daemon automation. |
| Wishlist interval scheduling | Implemented/needs-live-proof | Client scheduler primitive honors server `WishlistInterval` with positive minimum interval validation; daemon session manager now snapshots wishlist terms, records scheduled wishlist searches, and emits `WishlistSearch` while connected. Remaining work: optional live interop proof. |
| Security posture | Gate-backed/needs-live-proof | Bind/public-posture checks, WebSocket auth coverage, CSP, OAuth callback state isolation, webhook outbound policy, rate-limit proxy policy, storage/transfer pressure gates, secret scanning, protocol scalar/adversarial gates, and remediation baseline are implemented. Remaining work: optional live exposure/interoperability proof in deployment environments. |
| Release/packaging/dependency posture | Gate-backed | Release workflow policy, package artifact matrix, release version metadata, dependency hygiene, audit-tooling inventory, SDK gates, OpenAPI drift, docs freshness, and remediation registry checks are implemented. Remaining work: keep gates green as upstream release/CI changes are classified. |

## Upstream Delta Buckets

Use `scripts/collect-upstream-parity-delta.sh` to refresh the local inventory.
The generated output is intentionally a triage input, not proof of parity.

Required classification values:

- `implemented`: behavior exists and is covered by a local gate.
- `needs-proof`: behavior appears present but lacks parity-specific tests.
- `missing`: behavior is absent.
- `compat-ack`: route or workflow intentionally returns a compatibility shell.
- `not-applicable`: .NET-only implementation detail or slskdN-only packaging
  detail with no `slskR` equivalent.

## Execution Checklist

| Upstream area | slskR target | First gate |
| --- | --- | --- |
| Runtime signed capabilities | Rust descriptor, canonical bytes, Ed25519 sign/verify, expiry validation, username-keyed registry | Unit tests cover valid, forged, expired, malformed, blank, case-insensitive lookup, and expired pruning behavior. |
| Capability exchange | Reserved custom peer-message envelope with fail-closed parsing and ack behavior | Client tests cover envelope round trip, invalid payload rejection, unrelated unknown-message ignore, registry update, and hello/ack response. |
| Mesh rendezvous | Interest tag management, similar-user discovery, optional active descriptor probe | Hermetic client and daemon tests cover passive defaults, candidate dedupe, descriptor filtering, missing peers, explicit active hello probes, discovery JSON, peer capabilities, mesh stats, and compatibility peer lists; remaining gate is optional adjacent slskdN live test. |
| Wishlist scheduling | Server-interval-aware scheduler with positive guardrails | Interval guard, term replacement, daemon term extraction, and scheduled search record tests are implemented; live daemon interop remains optional. |
| Room failures | `CantCreateRoom`/reconnect state reaches API/UI instead of timing out silently | `CantCreateRoom` projection clears optimistic joins and records `last_error`; disconnected/reconnecting joins return HTTP `503`. |
| Multi-user private messages | Deduped, bounded recipient list and compatible API surface | Client/protocol tests cover dedupe, max count, blank recipients, and command emission; daemon API tests cover `/api/conversations/batch` recording and `MessageUsers` dispatch. |
| User groups | Sharegroup membership reaches slskd-compatible user group API | Daemon API tests cover `/api/users/:username/group` deriving primary and full group membership from sharegroup members while preserving default projection for unknown users. |
| Store-backed compatibility projections | Collection, wishlist, contact, share, and bridge helper routes expose local state | Daemon API tests cover `/api/shared`, `/api/contacts/nearby`, collection item update/delete/reorder, wishlist update/delete, and `/api/bridge/transfer/:id/progress` projecting existing local stores instead of fixed compatibility shells. |
| Activity and recommendation projections | Interests, now-playing, source-feed, and bridge admin compatibility routes expose local state | Daemon API tests cover user interest projection, interest-backed Soulseek recommendations, item recommendation/similar-user projections, now-playing POST/GET, source-feed preview/feed projection, and bridge admin stats deriving transfer totals instead of fixed empty compatibility shells. |
| Library, job, and discovery projections | Library health, MusicBrainz, Lidarr fallback, taste/discovery, destinations, listening-party, and multisource routes expose local state | Daemon API tests cover library health issue grouping from library items, unconfigured Lidarr wanted/missing and sync fallback from library health, Lidarr manual import seeding local library items, MusicBrainz completion/coverage, release-radar wishlist projection and persisted subscription creation, taste recommendations and discovery graph seeds, destination validation against the local destination store, joined-room listening parties with party content messages and now-playing state, generic job lists from searches/transfers, and multisource transfer job projections. |
| System mutation compatibility projections | Admin/config, application runtime, profile, bridge, relay, wishlist import, and share grant helper routes expose local state | Daemon API tests cover admin stats deriving transfer/search totals, plugin/config summaries from integration config, application restart and GC compatibility state, profile updates mutating the session projection, profile invite/cache warm/backfill/SongID/Lidarr operation counters surfacing through runtime state, bridge start/stop/config updates surfacing through bridge status and application runtime state, autoreplace/preferences state, relay/relay-agent enable-disable state, CSV wishlist import creating items, and share-grant token/backfill helpers deriving results from grant and collection stores with persisted/local status for existing grants. |
| Action-route compatibility projections | Wishlist item search, library scans/fixes, targeted library issue updates, MusicBrainz targets, and job creation/detail routes feed local stores | Daemon API tests cover wishlist item searches creating wishlist-targeted search records, library scan responses reflecting current health issues, targeted library issue updates mutating affected local item metadata, fixable library issues mutating local item state, MusicBrainz target creation projecting into library state, and discography/MusicBrainz release job creation surfacing through `/api/jobs/:id`. |
| Inventory and operations projections | Hash DB, backfill, mesh sync, logs, events, relay controller, and batch routes derive from local runtime state | Daemon API tests cover hash entries derived from share index records, backfill candidates/stats derived from search and share stores, hash backfill queue counts, mesh sync status derived from watched/capable peers, event mutation responses and logs projected from the event store, relay controller token acknowledgements carrying relay/share state, and limited local batch execution for health/config/capabilities/stats reads. |
| Media and status projections | Profile, source-feed creation, SongID, KPI, pod, federation, Solid, and security status routes expose local state | Daemon API tests cover profile data from session state, source-feed creation seeding wishlist-backed feeds, SongID runs/details/matrices deriving matches from library and share stores, KPI summaries from transfers/searches/shares, pod and federation status from room/user/mesh state, Solid status from collections/grants, and security dashboard counts from watched peers/events/webhooks. |
| Native service compatibility projections | slskdN roots, PodCore, streams, rooms, mesh health, playback, trace, fairness/ranking, port-forwarding, signal, backfill, federation, Solid, and security ban routes expose local state | Daemon API tests cover slskdN summaries from session/share/search/transfer/user/room/library stores, slskdN library health from library issues, PodCore content search from the share index, room ticker/member compatibility mutations updating joined room state, stream availability from shares/transfers, security bans mutating local state and dashboard counts, fairness/ranking rows from watched users/searches/transfers, port-forwarding rows from listener bind state, federation rows from watched/capable peers, Solid rows from collections, specialized native service status routes, and the native compatibility fallback returning family-specific local counts/items/jobs instead of generic disabled shells. |
| Events and live updates | Shared topic taxonomy across historical and WebSocket event feeds | Daemon API and WebSocket tests cover event recording for mutating workflows, topic projection in `/api/events/records`, topic/q/kind filtering in `/api/events`, and `/api/events/ws` frames using the same topic mapper as historical event APIs. |
| Search responses | Public and locked/private result projection | Search response tests cover flattened API ingestion, slskd-style grouped response ingestion with `files`/`lockedFiles`, protocol peer response ingestion, slskd-style `files`/`lockedFiles` split, `lockedFileCount`, per-result locked flags, `limit`/`offset` paging on search detail `results` arrays without changing total counts, paged/filtered `/api/searches/:id/responses` peer groups, caller-supplied search TTL bounds, `PUT /api/searches/:id` lifecycle/query mutation, and state-preserving cancel/fail/expire action routes. Remaining gate: optional live interop. |
| Browse responses | slskd-shaped user browse and directory projection | Browse tests cover flattened API ingestion, slskd-style directory payload ingestion, peer browse payload ingestion, slskd-compatible root `directories` projection, directory `files` projection, state-backed browse status for requested/failed/cancelled/ready records, `limit`/`offset` paging without changing total directory or file counts, and `q` filtering on root directory groups and directory files. Remaining gate: optional live interop. |
| Transfers | Retryable failed downloads, rooted remote-path acceptance, terminal details before failure | Transfer API and local peer tests cover explicit failed-download retry, stale reason clearing, preserved byte count for resume, scoped download paths, progress events, peer queue-position projection, stuck-download, per-user, speed, download stats, and accelerated-download compatibility reports, search-backed alternative source replacement and auto-replacement, local peer upload/download execution, and rejection paths. Remaining gate: live interop. |
| Obfuscation | Plain/obfuscated `P`, `D`, `F` fallback matrix | Loopback tests cover obfuscated `P`/`D`/`F` demux, plain rejection of obfuscated init, obfuscated rejection of plain init, preferred obfuscated file-transfer execution, and fallback to plain `P`/`F` when advertised obfuscation fails. |
| Security hardening | Bind exposure validation, feature gates, path/SSRF/logging checks | `scripts/check-remediation-baseline.sh` covers endpoint drift, browser token persistence, unsafe blank opens, WebSocket auth, CSP, webhook outbound policy, rate-limit proxy policy, storage listing pressure, transfer event growth, workflow/release policy, package matrix, dependency hygiene, release metadata, secret scanning, SDK gates, audit tooling, module hygiene, docs drift/freshness, council gates, protocol taint/adversarial checks, shell hygiene, Kubernetes public posture, and compatibility no-op documentation. |
| UI performance parity | Implemented | Rust WebUI tests assert a single active route owns live probes/workspace data, hidden RustyMilk starts stopped, diagnostics/workspaces advertise lazy state, and initial shell markup has no hidden-pane polling loops. |

## Required Gates

- `scripts/check-endpoint-parity-drift.sh`
- `scripts/check-openapi-docs-drift.sh`
- `scripts/check-remediation-baseline.sh`
- `cargo test --workspace`
- Rust WebUI audit when UI behavior changes
- Optional live interop matrix when credentials and adjacent daemons are
  intentionally configured

## Assumptions

- Full feature parity is the target, including advanced slskdN features.
- Rust-native internals are acceptable when externally visible behavior matches.
- Live Soulseek and adjacent-daemon tests remain opt-in; hermetic tests are
  mandatory for closure.
