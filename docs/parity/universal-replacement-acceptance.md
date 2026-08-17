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
3. Every supported transport is proven in both directions: Soulseek peer,
   obfuscated peer, distributed/DHT, overlay UDP, overlay QUIC control, QUIC
   data, relay/gateway, mesh-sync, VirtualSoulfind, and file/stream transfer.
4. Restart, corrupt state, cancellation, timeout, retry, resume, concurrent
   mutation, upgrade, rollback, permission, and clean-uninstall behavior match
   the selected target profile.
5. Certification starts from a clean process state and runs under the
   repository memory guard. No command may bypass the exclusive lock, one-job
   Rust limit, or virtual-memory ceiling.

The strict auditor additionally requires `--transport-evidence` containing a
fresh live JSON artifact. It must mark every supported transport as `pass` for
both frozen targets and both directions: Soulseek peer, obfuscated peer,
distributed/DHT, overlay UDP, overlay QUIC control, QUIC data, relay/gateway,
mesh-sync, VirtualSoulfind, and file/stream transfer. The same artifact must
also pass restart, corrupt-state, cancellation, timeout, retry, resume,
concurrent-mutation, upgrade, rollback, permission, and uninstall scenarios.
Local protocol tests and a green controller manifest cannot substitute for
this artifact.

## Current blockers

These are open until fresh evidence or implementation closes them:

| Area | Evidence of the gap |
| --- | --- |
| React and Rust UI | Both UI auditors now support live-backend evidence; fresh React proof must cover all 41 routes at both viewports and fresh Rust proof must cover all 15 routes at both viewports. Workflow actions, populated/empty/error/reconnect states, and target-profile differences remain open until those cases are run. |
| QUIC and DHT | The bounded `slskdn-overlay-data` transport, reusable stream client, daemon receiver, bounded public-QUIC proxy admission, and public shared-DHT/UDP request/response demux now exist with bounded proof; mainline outbound source-port/routing semantics through the proxy and live receiver interoperability remain open. |
| Relay | The local relay data plane exists, but live cross-client certification remains open. |
| Mesh sync | Exact codec/runtime evidence exists, but frozen-target live interoperability and reconnect/retry proof remain open. |
| VirtualSoulfind | Dispatcher and nominal runtime evidence exist, but timeout, reconnect, and live interop cases remain open. |
| Evidence integrity | The old manifest can report 100% while reusing retained evidence and treating target-local rows as not applicable; that mode is not sufficient for this goal. |

The goal remains open until this table is empty and a fresh universal gate
passes.
