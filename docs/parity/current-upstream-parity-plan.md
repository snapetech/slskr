# Current-upstream parity rebase plan

Status: implementation complete; current-target supported-transport verification complete

This plan supersedes the frozen-boundary work plan for new implementation work.
The frozen compatibility evidence remains valid and is not rewritten.

## Goal

Bring slskR to code-level 1:1 parity with the current upstream target while
preserving the existing frozen compatibility profiles. Parity is determined by
executable source, runtime behavior, wire behavior, persistence, API/UI
contracts, security decisions, packaging executables, VPN/ancillary agents,
and operational tooling. Documentation, changelogs, and commit messages are
not parity evidence.

Current source boundary:

- frozen target: `65a14a8b821de4df4ab7ef3ab3b156d7206837a`
- current target: `1c172f4d278b983bc8c9151bdf30922a835af84e`
- post-boundary executable delta: current release-317 Admin Policies
  validation/generated-certificate behavior; release, dependency, and
  `slskdn`-branded packaging metadata remain intentionally out of scope for
  slskR parity.
- core backend/web/vendor delta: 271 files, `+15,707/-2,453`
- ancillary executable delta included in the full scope: 284 files,
  `+16,960/-2,478`

## Acceptance contract

Every source-derived ledger entry must be `implemented` or explicitly marked
as a frozen compatibility contract. `partial`, `missing`, and
`needs-proof` entries are release blockers.

Rust-native internals are allowed when externally observable behavior matches.
Existing frozen profiles remain testable where current behavior conflicts with
the frozen target.

## Verification status

The implementation portion of this plan is complete. The evidence companion
[`current-upstream-evidence.md`](current-upstream-evidence.md) records the
fresh controlled-runtime acceptance runs, including the current shared-TCP
topology and an official MsQuic-enabled run. The latest current-target matrix
has 85 passing rows, 0 skips, and 0 failures. The remaining external
acceptance boundary is deliberately separate: the fixture uses local test
services and fixture Soulseek credentials, so it does not prove login to the
public Soulseek network or a third-party VPN provider.

| Gate | Result |
| --- | --- |
| Source/config inventory | 449/449 documented upstream YAML leaves implemented; 0 partial; 0 missing. |
| Current controller dispatch | 687 routes inventoried; 685 non-destructive probes; 0 generic 404, HTML, compatibility-fallback, or probe-error results. |
| Live web UI | 23 route shapes, 391 links, 1,292 controls; 1,232 normal transitions, 8 drag transitions, 43 explicitly allowed local no-ops, 0 disabled controls, 0 failures. |
| Semantic UI workflows | 13 checks green, including invite, nearby refresh, YAML validate/save, library scan/fix, wishlist import, share token/manifest/stream, and local report/plan actions. |
| Protocol interoperability | Frozen credentialed v2 matrix: 85/85 pass; latest current-target shared-TCP matrix: 85 pass, 0 skip, 0 fail with QUIC control/data enabled by official MsQuic. The mesh-sync 400 case remains a documented target-negative contract. |
| Rust regression suites | Recorded full guarded baseline: `slskr` 427-test daemon groups plus 2 integration tests, `slskr-client` 319, and `slskr-web` 86; current shared-port focus: listener 23/23, config 1/1, and gateway ownership 1/1. |

## Current executable ledger

The code-level implementation ledger is closed against current source behavior
with the following fresh evidence. A Rust-native implementation is considered
equivalent when it preserves the observable API, wire, persistence, security,
and operational contract; source-language identity is not a requirement.

| Workstream | Executable closure | Fresh evidence |
| --- | --- | --- |
| Configuration and reload | Current canonical names, frozen aliases, YAML normalization, reload, validation, auto-replace, filters, Lidarr, overlay, and advertised-port settings are wired. | `audit-upstream-config-surface.py --require-complete`: 449/449 leaves implemented; recorded guarded `slskr` baseline passed. Current config regression: 1/1. |
| Persistence and integrations | Additive migrations and restart hydration cover retry attempts, wishlist fallback state, Lidarr metadata/history, editions, and atomic bulk updates. | Recorded full guarded `slskr` baseline passed; persistence and integration cases are included in that run. |
| Transfers and search | Filename policy is enforced at every enqueue/retry/auto-replace boundary; retries persist attempt/backoff state; wishlist search fallback, dedupe, quality/edition ranking, and bounded auto-download are live. | Recorded full guarded `slskr` baseline passed, including policy, fallback, ranking, and persistence tests. |
| Soulseek and mesh transport | Shared plain/obfuscated TCP demultiplexing, shared UDP/QUIC routing, ALPN classification, self-certifying DHT store validation, and frozen/current listener profiles are implemented. | Recorded full guarded client and daemon suites passed; current listener focus: 23/23, config: 1/1, gateway ownership: 1/1. |
| Media and SongID | MusicBrainz/discography targets, media descriptors, identity evidence, capability reporting, scored SongID consensus, and forensic API data are executable. | Recorded full guarded `slskr` baseline passed; live API/UI route probes exercised the exposed metadata surfaces. |
| API and web UI | Native API controls and Rust UI actions cover destinations, exclusions, YAML validation/save, search bulk actions, transfer columns, history/retry, metadata/integration surfaces, share manifests/tickets, and error/reconnect states. | Guarded `slskr-web` library: 86 passed; live exhaustive audit: 23 route shapes, 391 links, 1,292 controls, 0 failures; semantic audit: 13/13 checks green. |
| Identity and compatibility | Native runtime/UI/package identity is `slskR`; the frozen compatibility selector and legacy wire labels remain profile-gated. | Identity-leak gate passed; current and frozen profile tests passed. |
| VPN, relay, and packaging executables | Current ingress renewal/reclaim behavior, WireGuard health checks, self-hosted relay API/systemd artifacts, packaging aliases, WASM, and package contents are wired. | `bash -n`, Python compilation, CSP/build-guard/identity gates passed; guarded package contents verification passed with `--allow-dirty`. |

The local fixture contains non-public test credentials only. No credential in
this workspace has been validated against the public Soulseek service, and no
third-party VPN provider acceptance run was performed. Those are operator
acceptance checks, not missing code evidence; they must not be reported as
successful network-login tests.

## Current-head audit boundary

The historical classification ledger and its check script intentionally remain
frozen through slskdN `db9d6eee2af1b484c62495d4fb3683f7009a15f4`; they are not a
claim that later commits were ignored. The current comparison target is
slskdN `1c172f4d278b983bc8c9151bdf30922a835af84e`. The executable source delta
after the previous parity pin is limited to the current Admin Policies
validation/generated-certificate behavior, plus dependency metadata. The
current Admin Policies behavior is ported and tested here; dependency, release,
documentation, and `slskdn`-branded packaging metadata are intentionally not
copied into slskR.

The current shared listener behavior itself is covered by the earlier upstream
port-merge commits already inside the comparison boundary and by the
current-target runtime matrix below.

## Workstreams

### Foundation

- Add current configuration, validation, reload, defaults, and feature gates.
- Add transfer, Lidarr, wishlist, listener, DHT, QUIC, and advertised-port
  persistence/configuration fields.
- Add additive SQLite migrations, restart hydration, indexes, and failure
  rollback behavior.
- Update API DTOs, authorization, error shapes, and compatibility dispatch.

### Transport

- Implement shared TCP plain/obfuscated connection classification.
- Implement shared DHT UDP and QUIC ALPN demultiplexing.
- Port advertised-port and listener lifecycle/reconfigure semantics.
- Preserve frozen transport profiles and test malformed, reconnect, and failure
  paths in both directions.

### Transfers and search

- Add global download exclusions and apply them to every enqueue, retry,
  swarm, and auto-replacement path.
- Add stable request linkage, persistent auto-replace attempts, bounded retry
  behavior, source ranking, verification, cooldowns, and normal-transfer
  multi-source integration.
- Add smart search fallback, suppressed terms, provider/scene search,
  aggregation, duplicate folding, destination selection, and response metadata.

### Integrations and media

- Add Lidarr import history/retry, delay/backoff, edition matching,
  already-owned filtering, partial-album tracking, and current synchronization
  fields.
- Add wishlist atomic bulk filtering and current Lidarr-linked fields.
- Port descriptor retrieval/validation, IPLD mapping, metadata portability,
  perceptual/fuzzy matching, and metadata-processing state.
- Port SongID queue lifecycle, canonical/corpus scoring, quality consensus, and
  configured-tool capability detection.

### Security, mesh, and social

- Port current consensus, canary, fingerprint, cryptographic commitment,
  identity separation, proof-of-storage, reputation, anonymity transport,
  middleware, and security-event decisions.
- Port VirtualSoulfind, shadow-index, disaster-mode, discovery graph, mesh
  planner/resolver, and taste-recommendation behavior.

### UI and executable operations

- Update web API clients and controls for exclusions, history/retry, wishlist
  bulk operations, provider/folding search, and metadata processing.
- Update packaging, VPN/ancillary agents, scripts, workflows, and operational
  executables included by the current source boundary.
- Remove stale product identifiers from generated and runtime responses.

## Evidence and gates

- Maintain a source-derived parity ledger with file/symbol/behavior evidence.
- Run every Cargo command through `scripts/with-build-guard.sh`.
- Add unit/property tests for parsers, policy, scoring, migrations, retries,
  state machines, and transport demultiplexing.
- Run API differential tests for nominal, malformed, authorization, persistence,
  restart, concurrency, and storage-failure cases.
- Run headless browser coverage for every page, tab, link, control, mutation,
  and error state.
- Run current-target and frozen-profile live interoperability matrices,
  including restart and package/container/VPN smoke tests.
- Add validated release-note fragments for user-facing, security, operational,
  or user-facing documentation changes.

## Execution order

1. Ledger and contract inventory.
2. Configuration, schema, migrations, and API foundations.
3. Shared transport and vendor runtime.
4. Transfers, search, Lidarr, and wishlist.
5. MediaCore, SongID, security, mesh, and social behavior.
6. Web UI, packaging, VPN, and operational executables.
7. Differential/live verification and release acceptance.
