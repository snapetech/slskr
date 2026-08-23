# Current-upstream executable parity evidence

Status: controlled-runtime acceptance complete; external-service acceptance is
environment-dependent.

This is the evidence ledger for the current-upstream parity plan. It records
executable source, API, UI, protocol, persistence, and integration checks. A
release or parity claim must use these results rather than screenshots,
documentation, or commit-message assertions.

## Acceptance result

Confidence is high for the code and controlled runtime. The current fixture
uses local Lidarr and MusicBrainz services, a local Soulseek peer fixture, and
fixture credentials. It does not establish that those credentials work on the
public Soulseek network.

| Surface | Fresh proof | Result |
| --- | --- | --- |
| Upstream configuration | `python3 scripts/audit-upstream-config-surface.py --slskd-root ../slskdn --slskdn-root ../slskdn --require-complete` | 449 documented YAML leaves; 449 implemented; 0 partial; 0 missing. |
| Current controller/API dispatch | `scripts/audit-slskdn-controller-routes.mjs` against the fixture, with `DELETE /api/v0/application` and `DELETE /api/v0/server` excluded because they intentionally mutate daemon lifecycle | 687 routes inventoried; 685 probed; 0 generic slskR 404s; 0 HTML fallbacks; 0 compatibility fallbacks; 0 probe errors. |
| Web UI controls and links | `target/ux-audit/current-live-ui-complete-hard-v6/exhaustive-audit.json` plus a rendered-control stale-label scan | 23 route shapes; 391 links; 1,292 controls; 1,232 transitions; 8 drag transitions; 43 explicitly allowed local no-ops; 0 disabled controls; 0 failures; 0 page errors; 0 rendered matches for the removed Room/Shutdown/Restart/Leave-share placeholder actions. |
| UI semantic workflows | `target/ux-audit/current-semantic-v2/semantic-audit.json` | 13/13 green: invite, nearby refresh, YAML validate/save, library scan/fix, local experience/automation reports, wishlist import, share token/manifest/stream. |
| Soulseek and mesh wire behavior | `target/evidence/credentialed-v2-all-green-rerun/slskr-slskdn-cross-client-interop.tsv` | 85 rows; 0 non-pass rows. Browse, search, transfer, message, DHT store, pods, gateway echo, QUIC directions, exact mesh bytes, and VirtualSoulfind v2 were exercised. The mesh-sync 400 is target-negative evidence, not a positive sync result. |
| Rust backend | `./scripts/with-build-guard.sh cargo test -p slskr --lib -- --nocapture` | 423/423 passed. |
| Rust client transport | `./scripts/with-build-guard.sh cargo test -p slskr-client --lib -- --nocapture` | 76/76 passed. |
| Rust web rendering/actions | `./scripts/with-build-guard.sh cargo test -p slskr-web --lib -- --nocapture` | 86/86 passed. |

The controller-run summary is also captured in
[`current-upstream-controller-audit-green.json`](current-upstream-controller-audit-green.json).

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

There is no validated public Soulseek account in this workspace. The fixture
configuration contains test-only credentials and runs with auto-connect
disabled for deterministic acceptance. Therefore the following are not marked
green here:

- public Soulseek login and server-session acceptance;
- acceptance against a non-fixture remote peer outside the controlled
  interop matrix;
- third-party VPN-provider login/renewal;
- package-manager publication or installation against external registries.

Those checks require operator-supplied credentials or external service access.
They do not convert the green local API/UI/protocol results into a claim of
public-network operation.

## Reproduction commands

Run Rust commands through the repository guard:

```sh
./scripts/with-build-guard.sh cargo test -p slskr --lib -- --nocapture
./scripts/with-build-guard.sh cargo test -p slskr-client --lib -- --nocapture
./scripts/with-build-guard.sh cargo test -p slskr-web --lib -- --nocapture
```

Run the exhaustive UI audit against a fixture daemon and use
`SLSKR_WEB_EXHAUSTIVE_FAIL_ON_HTTP_ERROR=true` so failed backend responses are
not silently treated as UI transitions. The saved JSON above is the final
hard-fail run.
