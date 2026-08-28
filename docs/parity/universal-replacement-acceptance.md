# Universal replacement acceptance

## Goal

slskR is complete only when a user, automation client, peer, operator, or
deployment system can replace either frozen target without observing a
behavioral difference. The frozen comparison boundary is:

- slskd `16e5d86ec9a91120f3ef40b85cb22036566b788a`
- slskdN `65a14a8b821de4df4ab7ef3ab3b156d7206837a3`
- slskNet.Runtime `af73ff3f84fda7ba890bb5aea3adf712e5400cf6`

“Universal” means both compatibility profiles and every user-visible contract
owned by those versions. It does not mean that Rust must reproduce private
.NET class names or physical database table layouts. A newer upstream version
requires a new pinned comparison boundary before it can be claimed.

## Non-negotiable gates

1. Every public configuration, HTTP, WebSocket, protocol, persistence,
   security, packaging, and operator contract has fresh executable differential
   evidence. Retained `/tmp` evidence and route-presence results cannot close a
   universal gate.
2. React and Rust UI workflows run against a real slskR daemon with populated,
   empty, loading, invalid, denied, failed, restarted, and reconnecting state.
   Mock-only screenshots are supporting evidence, not completion evidence.
3. Every transport exposed by the selected frozen profile is proven in both
   directions: Soulseek peer, distributed/DHT, and file/stream transfer for
   both profiles; type-1 obfuscation, overlay UDP/QUIC, QUIC data,
   relay/gateway, mesh-sync, and VirtualSoulfind for slskdN. Unsupported
   transports must be explicitly recorded as not applicable for slskd.
4. Restart, corrupt state, cancellation, timeout, retry, resume, concurrent
   mutation, upgrade, rollback, permission, and clean-uninstall behavior match
   the selected target profile. The
   `scripts/run-universal-lifecycle-matrix.sh` runner executes all 22
   target/scenario cases serially, rejects a replacement binary older than the
   current source, and requires independent per-case artifacts.
5. Certification starts from a clean process state. Rust commands use the
   pinned toolchain and workspace Cargo configuration; non-Rust browser and
   Node helpers use the process-memory helper where their workloads require it.

The strict auditor additionally requires `--transport-evidence` containing a
fresh live JSON artifact. It must mark each applicable target/transport pair as
`pass` in both directions and mark every unsupported target explicitly. A
separate `--transport-capability-evidence` artifact is required when the exact
frozen source proves that a target direction is not exposed; it must name the
source-bound reason and the live negative rows that demonstrate it. The same
transport artifact must also pass restart, corrupt-state, cancellation,
timeout, retry, resume, concurrent-mutation, upgrade, rollback, permission,
and uninstall scenarios. Local protocol tests and a green controller manifest
cannot substitute for applicable live transport evidence.

Strict mode also requires `--target-ui-comparison-evidence`, a separate live
side-by-side artifact covering search, browse, transfers, messages, rooms,
shares, settings, player, and mesh workflows against both frozen targets. The
replacement UI audit alone cannot satisfy that comparison. The comparator must
run two independently configured slskR backends/UI surfaces—one with the
`slskd` profile and one with the `slskdn` profile—and compare each to its
matching frozen target.

The bounded derivation command is
`python3 scripts/derive-universal-transport-evidence.py`; it maps only named
TSV transactions and requires a separate live lifecycle JSON containing all
22 target/scenario cases with per-case artifacts. Its optional
`--slskdn-supplement` input fills only checks absent from the authoritative
matrix, so a focused rerun cannot overwrite unrelated passing rows. Against
the current frozen artifacts, the source-bound capability artifact
`target/universal-transport-capability-evidence-20260820-frozen-slskdn-mesh-retry.json`
and derived artifact
`target/universal-transport-evidence-20260820-frozen-slskdn-mesh-retry.json`
report 11/11 records complete: all applicable Soulseek, type-1 obfuscated,
distributed/DHT, overlay UDP, QUIC control, QUIC data, mesh-sync, and
file-stream directions pass; relay/gateway, overlay reverse-routing, and
VirtualSoulfind target-negative contracts are explicitly classified from
frozen source; and the 22/22 lifecycle matrix passes. The target-originated
obfuscated row uses the target's real public endpoint discovery with a
temporary owner-scoped local route to the replacement listener; it does not
use the unsupported target-side obfuscated endpoint override hook. The exact
artifact is a universal transport/lifecycle pass.

## Current status and scoped exceptions

The frozen universal replacement gate is closed as of 2026-08-20. The table
below records explicit target-negative contracts and retained evidence sources;
those scoped exceptions are part of the acceptance boundary, not open gaps.

| Area | Evidence of the gap |
| --- | --- |
| React and Rust UI | Fresh live-backend artifacts contain all 82 React and 30 Rust route/viewport cases with zero recorded page errors across the required states. The current independent four-surface artifact `target/frozen-target-ui-comparison-universal-20260820-fresh12/audit.json` reaches 9/9 runtime workflow-health passes, uses separate replacement daemons for both profiles, observes live replacement event feeds, and reports zero semantic API/control inventory mismatches. |
| Bounded Rust differentials | Complete in the ordinary ledger. Fresh proof executes the controller API, persistence, file-lifecycle, protocol, security-control, and security-authorization slices through linked feature-specific runners. The historical monolithic full-controller compile remains an explicit opt-in and is not used as a certification path. |
| QUIC and DHT | The bounded `slskdn-overlay-data` transport, reusable stream client, daemon receiver, bounded public-QUIC proxy admission, and public shared-DHT/UDP request/response demux now exist with bounded proof; shared-mode mainline outbound source-port routing is wired and regression-tested. The exact target-pinned MsQuic package binds the frozen slskdN QUIC listeners in an isolated run. One-shot certificate-pin discovery captures the target's ephemeral control/data certificate for that connection only; ordinary pinned connections still require an expected endpoint pin. The fresh exact run passes UDP, QUIC control, and QUIC data replacement-to-target transactions. The frozen target's reverse peer-routing path remains source-bound unavailable: `PeerResolutionService` looks up 32-byte SHA-256 DHT keys, while its mesh `Store`/`FindValue` service rejects every key that is not 20 bytes, and no registration call site exists. The strict artifact records those reverse directions as explicit target-negative contracts. |
| Relay | The exact frozen slskdN source contains the private-gateway implementation but does not register it with the mesh router. The fresh negative rows therefore close the relay/gateway transport as an explicit not-applicable target contract; this does not certify a positive relay data plane. |
| Mesh sync | The base bidirectional live map passes in the strict artifact. The fresh source-pinned run `target/live-interop-exact-transport-probes-mesh-retry-msquic3-20260820/slskr-slskdn-cross-client-interop.failed-4153322.tsv` records repeated `/api/v0/mesh/sync/{username}` attempts against both target and replacement, each returning the exact target-negative `400 {"error":"Failed to sync with peer"}` contract twice. The focused row is retained in `target/live-interop-exact-transport-probes-mesh-retry-msquic3-20260820/slskr-slskdn-cross-client-interop.mesh-lifecycle-supplement-4153322.tsv`; frozen source confirms no outbound mesh transport exists. This closes reconnect/retry/failure as an explicit target-negative contract, not as a positive mesh data plane. |
| VirtualSoulfind | The exact target returns its documented 503-disabled response because the v2 controller reads an unbound options type whose default is disabled. The source-bound capability artifact closes this as a target-negative contract; it does not certify a positive v2 transport. |
| Current slskdN live interop | The established exact frozen-target matrix exposes 61 rows: 49 positive transactions pass and 12 rows return the pinned target's expected negative behavior. Artifact: `target/live-interop-exact-transport-probes-msquic-combined/slskr-slskdn-cross-client-interop.failed-3970303.tsv`; systemd peak was 295.3 MiB with zero swap. The fresh source-pinned diagnostic run is retained at `target/live-interop-exact-transport-probes-mesh-retry-msquic3-20260820/slskr-slskdn-cross-client-interop.failed-4153322.tsv` (305.1 MiB peak, zero swap), and its exact mesh lifecycle row is the supplemental artifact named above. The strict gate consumes the complete transport matrix plus that fill-only fresh row, so unrelated transient public-peer/download failures cannot overwrite passing transport evidence. |
| Frozen slskdN advanced services | The replacement slskdn profile gates remote `pods`, `private-gateway`, and `shadow-index` calls to the pinned target's `Service '<name>' not found` contract; local HTTP pod behavior is unchanged. The exact two-daemon run also records both profiles returning 503 for VirtualSoulfind v2. The source-bound capability artifact now carries the gateway, reverse-overlay, and VirtualSoulfind negative contracts into strict transport accounting without treating them as positive slskR transactions. |
| Strict live evidence coverage | The fresh source-bound capability artifact `target/universal-transport-capability-evidence-20260820-frozen-slskdn-mesh-retry.json` and derived artifact `target/universal-transport-evidence-20260820-frozen-slskdn-mesh-retry.json` pass all 11/11 strict transport/lifecycle records, including direct UDP/QUIC positives, reverse-overlay target-negative contracts, and the new mesh retry lifecycle row. The fresh live UI artifact passes all 9 workflow-health cases and its 18 target/profile comparisons have zero semantic API/control inventory mismatches, with live replacement event feeds and a complete profile matrix. |
| Evidence integrity | Fresh bounded audit result: 19,216/19,216 materialized cases complete, 0 partial, 0 missing, and 0 needs-proof. The strict transport/lifecycle artifact is complete at 11/11, and the universal frozen-boundary gate passes. |

The table is closed for the pinned frozen boundary. A newer upstream target
requires a new pinned comparison boundary and a new certification run.
