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

Parity is **not achieved**. Every planned workstream has an executable
certification denominator. The literal proof-case closure ratio is
**662 / 19,122 = 3.46%**, but this is not a product-completion estimate: most
generated cases are Cartesian proof dimensions initialized as `needs-proof`,
including behavior already implemented and tested in slskR. Product completion
remains unreported until the subsystem reclassification separates absent
behavior from present-but-unlinked evidence.

| Workstream | Audited denominator | Current evidence | Closure state |
| --- | ---: | --- | --- |
| Configuration | 436 frozen YAML leaves | 245 complete, 0 partial, 191 missing | Open |
| slskd controller API | 91 routes | Route presence is covered; exhaustive behavior is not | Open |
| slskdN controller API | 678 routes | Route presence is covered; exhaustive behavior is not | Open |
| Frozen WebUI API calls | 417-call union | Call presence is covered; rendered workflows are not | Open |
| Soulseek and adjacent protocols | 1,465 proof cases | Core and several live paths pass; exhaustive case evidence remains open | Open |
| Persistence and lifecycle | 798 proof cases | Selected config and state families are proven | Open |
| Security and authorization | Route registries plus hardening gates exist | Auth conflicts are profiled; full negative-path matrix is open | Open |
| Packaging and operator behavior | 240 proof cases | Existing gates have not yet been attached to every manifest case | Open |
| Bidirectional interoperability | 310 proof cases | Selected live matrices exist; exhaustive feature-pair evidence is open | Open |

### Known implementation-gap queue

The 191 configuration contracts classified as genuinely missing are grouped by
shared implementation dependency, not processed leaf by leaf:

| Subsystem batch | Missing contracts | Families | Execution priority |
| --- | ---: | --- | ---: |
| Daemon foundation | 45 | web/HTTPS, flags, logger, retention, permissions, telemetry, search retention | 1 |
| Core workflows | 16 | interests, rooms, wishlist, shares, destinations, search throttling | 2 |
| Advanced networking and security | 88 | DHT, Mesh, PodCore, overlay, overlay data, relay, security | 3 |
| Media and advanced services | 42 | feature gates, player, Solid, SongID, VirtualSoulfind | 4 |

The 18,269 `needs-proof` cases are not presumed absent. They are certification
dimensions to be linked in bulk from the subsystem contract matrices after the
known implementation gaps are closed.

The raw proof-case closure ratio is a certification-ledger metric only. It is
not used as an implementation queue or as a product-completion estimate. Work
is selected and closed by vertical subsystem so one implementation and one
generated differential matrix can satisfy all affected route, configuration,
protocol, persistence, UI, and interoperability cases together.

`scripts/audit-parity-manifest.py --check-frozen` currently materializes 19,122
unique proof cases across all workstreams: 662 complete, 0 partial, 191 missing,
and 18,269 needing behavioral proof. There are zero `denominator-missing` cases.
The 662 complete cases include 417 frozen WebUI call-presence cases; they do not
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
