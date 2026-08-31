# Current-upstream executable parity evidence

Status: current-target supported-transport acceptance complete, including
QUIC with an official MsQuic runtime, plus VPN-scoped public Soulseek
acceptance for the available credential pool; third-party provider workflows
remain environment-dependent.

This is the evidence ledger for the current-upstream parity plan. It records
executable source, API, UI, protocol, persistence, and integration checks. A
release or parity claim must use these results rather than screenshots,
documentation, or commit-message assertions.

## Acceptance result

Confidence is high for the code, controlled runtime, and the VPN-scoped public
Soulseek checks below. The controlled fixture uses local Lidarr and MusicBrainz
services; the public acceptance uses eight distinct operator-provided
Soulseek accounts through isolated Proton WireGuard profiles. No secret values
are recorded in this ledger.

| Surface | Fresh proof | Result |
| --- | --- | --- |
| Upstream configuration | `python3 scripts/audit-upstream-config-surface.py --slskd-root ../slskdn --slskdn-root ../slskdn --require-complete` | 449 documented YAML leaves; 449 implemented; 0 partial; 0 missing. |
| Target-compatible SignalR hubs | Rust `signalr_ws` tests, `@microsoft/signalr` live profile checks, token-authenticated startup, search CREATE, and daemon replacement reconnect on 2026-08-31 | Legacy `application`, `logs`, `search`, and `metrics` hubs and native `application`, `logs`, `search`, `songid`, `listening-party`, and `transfers` hubs connected with the expected initial events; Join/Leave and unknown-method completions matched the contract; search broadcasts carried an empty response collection; token query auth and automatic reconnect passed. Negotiation advertises only the implemented WebSocket transport. |
| Current controller/API dispatch | `scripts/audit-slskdn-controller-routes.mjs` against the fixture, with `DELETE /api/v0/application` and `DELETE /api/v0/server` excluded because they intentionally mutate daemon lifecycle | 687 routes inventoried; 685 probed; 0 generic slskR 404s; 0 HTML fallbacks; 0 compatibility fallbacks; 0 probe errors. |
| Web UI controls and links | `target/ux-audit/current-live-ui-complete-hard-v6/exhaustive-audit.json` plus a rendered-control stale-label scan | 23 route shapes; 391 links; 1,292 controls; 1,232 transitions; 8 drag transitions; 43 explicitly allowed local no-ops; 0 disabled controls; 0 failures; 0 page errors; 0 rendered matches for the removed Room/Shutdown/Restart/Leave-share placeholder actions. |
| UI semantic workflows | `target/ux-audit/current-semantic-v2/semantic-audit.json` | 13/13 green: invite, nearby refresh, YAML validate/save, library scan/fix, local experience/automation reports, wishlist import, share token/manifest/stream. |
| Soulseek and mesh wire behavior | `target/evidence/credentialed-v2-all-green-rerun/slskr-slskdn-cross-client-interop.tsv` | 85 rows; 0 non-pass rows. Browse, search, transfer, message, DHT store, pods, gateway echo, QUIC directions, exact mesh bytes, and VirtualSoulfind v2 were exercised. The mesh-sync 400 is target-negative evidence, not a positive sync result. |
| Public Soulseek credential pool and VPN routing | `target/live-interop-all-accounts/slskr-login-smoke.tsv`, `target/live-interop-all-accounts/proton-public-cycle-*.tsv` | All eight available accounts logged in through Proton WireGuard isolation. Eight rotated public-server pair runs closed 64/64 required checks: listener metadata, plain, obfuscated, distributed, file-transfer, indirect, metadata relogin, and negative-indirect behavior. Six transient metadata-wait observations retried before the required checks passed. |
| VPN-scoped HTTP/API transfer | `target/live-interop-all-accounts/live-http-transfer-vpn-10.stdout.log` | Both authenticated public sessions connected; listeners advertised; peer address and browse completed; unauthenticated API access returned 401; the API queued and completed a real 52-byte transfer with matching SHA-256; both sessions stayed connected through a 10-second soak. |
| Rust backend | `cargo test -q -p slskr` | Both 427-test daemon groups and the 2 integration tests passed; 0 failures. |
| Rust client transport | `cargo test -q -p slskr-client` | 319 tests across the client transport/protocol groups passed; 0 failures. |
| Rust web rendering/actions | `cargo test -q -p slskr-web` | 86/86 passed. |

The controller-run summary is also captured in
[`current-upstream-controller-audit-green.json`](current-upstream-controller-audit-green.json).

## Latest current-target shared-port run

On 2026-08-26, the rebuilt slskR binary was run against current slskdN
`1c172f4d278b983bc8c9151bdf30922a835af84e` with the current shared-TCP profile.
The runner now defaults to that profile; a dedicated overlay listener is only
selected by an explicit compatibility override.

Artifact: `target/live-interop-current-shared-quic-rerun/slskr-slskdn-cross-client-interop.tsv`

The 85-row result contains 85 `ok`, 0 `skip`, and 0 `fail` rows. It exercised
listener metadata, plain and obfuscated Soulseek paths, browse, search, both
file-transfer directions, messages, user watch, distributed parent/child
propagation, rooms, capability exchange, pinned overlay services, exact mesh
content bytes, PodCore workflows, private gateway echo, signed DHT Store, mesh
health/statistics/tickets, UDP overlay, negative reverse routing, and the final
soak. The shared-TCP diagnostic recorded slskR's Soulseek and mesh TCP endpoint
as the same port, and the current slskdN log recorded inbound TLS mesh sessions
on its shared Soulseek listener. The target log also recorded QUIC control and
data listeners, shared UDP routing to both loopback backends, and receipt of
both probes.

The run used the official Microsoft MsQuic 2.6.0 Linux runtime. The official
Ubuntu package was downloaded and verified at SHA-256
`f4f035c674bc36deb43714fdfc26619b7e3c9bad886d36793ecac215d474bcf9`; the
official v2.6.0 source was also built locally with OpenSSL 3.6.3, and the
source-built library was loaded through `LD_LIBRARY_PATH` for this run. No
system-wide library replacement was required.

The separate all-phase certification run at
`target/certify-vpn-full/summary-20260823-093737.json` remains a diagnostic
artifact at 38/39: its A5 long-lived NAT-PMP soak hit a peer reset. That result
is not relabeled as green; the eight rotated public runs above independently
passed the indirect path, including the same ConnectToPeer/PierceFirewall
exchange.

## Integration workflows

The controlled integration run exercised real request/response paths rather
than only rendering status cards:

- Lidarr status and synchronization returned 200; wanted/missing pagination
  returned fixture records; wanted sync created two records; manual import
  returned two candidates with one rejected and one safe candidate; history
  retrieval and retry returned 200 with retry linkage.
- MusicBrainz release and recording targets returned 200; content search
  returned a real fixture recording; coverage resolved artist/release/missing
  track data; album completion returned 200; release-radar subscription
  persisted and read back successfully.
- SongID accepted a fixture WAV, returned a completed 202 run with a
  Chromaprint fingerprint, matched the fixture AcoustID recording at score
  0.97, persisted the run, and observed the `/v2/lookup` request.

These workflows use `scripts/fixture-lidarr.py` and
`scripts/fixture-metadata.py`; they are not claims about the availability or
credentials of external providers.

## What remains environment-dependent

The public Soulseek checks are green for the eight account/profile combinations
recorded above. They are not a blanket claim for arbitrary future credentials,
egress routes, or peer availability. The following remain environment-dependent:

- external Lidarr, MusicBrainz, and SongID provider availability and operator
  credentials;
- third-party VPN-provider login/renewal;
- package-manager publication or installation against external registries;
- QUIC services on hosts that lack a usable MsQuic runtime. The current host's
  controlled run is covered above with the official 2.6.0 runtime.

Those checks require additional external service access. The public-network
evidence above is the bounded acceptance result for the supplied credential
pool and the VPN routes used in that run.

## Reproduction commands

Run Rust commands with the pinned repository toolchain:

```sh
cargo test -p slskr --lib -- --nocapture
cargo test -p slskr-client --lib -- --nocapture
cargo test -p slskr-web --lib -- --nocapture
```

Run the exhaustive UI audit against a fixture daemon and use
`SLSKR_WEB_EXHAUSTIVE_FAIL_ON_HTTP_ERROR=true` so failed backend responses are
not silently treated as UI transitions. The saved JSON above is the final
hard-fail run.
