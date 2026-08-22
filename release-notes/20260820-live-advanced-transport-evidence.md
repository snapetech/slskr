---
category: changed
audience: operators
area: parity-certification
action: Provide the frozen target's native MsQuic library through LD_LIBRARY_PATH when reproducing the exact QUIC evidence run; none for normal slskR operation.
breaking: false
---

Exact frozen-target interop now records bidirectional obfuscated/distributed
transactions, UDP/QUIC control/QUIC data probes, and source-bound reverse-overlay
negative contracts. Strict transport/lifecycle derivation passes 11/11.
Reproduction requires only test-scoped MsQuic and temporary public-routing
inputs; normal slskR operation is unchanged.
