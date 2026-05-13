# Bug Council Active Backlog

This backlog is the durable handoff for `scripts/run-council-scan.sh`.
A green all-phases council run is not proof that no bugs exist; this file
records the active discovery piles that still need review, splitting, or
burn-down.

Every council scan candidate class must have a row below with the current
candidate count. `scripts/check-council-active-backlog.sh` fails when a class is
missing, left `Untriaged`, or has a stale count.

Status meanings:

- `Open` - broad queue still needs classification or narrower subgroup probes.
- `Guarded` - current candidates are classified and protected by remediation checks.
- `Accepted` - confirmed bug class exists and is being fixed.
- `Existing guard` - candidates are covered by existing behavior and gates.
- `False positive` - scanner shape is not a bug for the listed rationale.
- `Out of scope` - candidate belongs outside this council.

| Section | Candidate count | Status | Current classification | Next action |
| --- | ---: | --- | --- | --- |
| `Constructor/mutable collection candidates` | 7 | Guarded | Current constructor candidates are classified in `docs/dev/council-scan-inventory.md`; the accepted Python mutable-input bug is fixed and Rust constructors own or clone their inputs. | Reopen only when fresh candidates are not covered by BUG-032 or the existing ownership evidence. |
| `Protocol count/length candidates` | 46 | Guarded | Current count/length candidates are classified; accepted raw-frame, transfer-chunk, and protocol loop-bound bugs are fixed, with taint/adversarial gates covering high-risk parser paths. | Reopen only when a fresh wire-derived allocation/read/loop flow is not covered by BUG-033, BUG-035, BUG-040, or existing bounded-count evidence. |
| `Protocol scalar emission candidates` | 42 | Guarded | Current scalar emission candidates are classified; accepted API-to-protocol narrowing bugs are fixed and protocol-visible length/code emissions are checked or inventory-tested. | Reopen only when a fresh protocol-visible narrowing path lacks a checked conversion or discriminant inventory evidence. |
| `Resolver/raw stream candidates` | 225 | Guarded | Current raw stream candidates are classified; accepted raw-frame and connect-timeout bugs are fixed, and daemon/SDK stream paths are covered by timeout, resolver, and frame-size guards. | Reopen only when a fresh direct socket/resolver/read path lacks timeout, address policy, or size-bound evidence. |
| `Task/cancellation/lifecycle candidates` | 246 | Guarded | Current lifecycle candidates are classified; accepted TypeScript timer/default bugs are fixed, and daemon/WebSocket/webhook task ownership is covered by bounded channels, timeouts, and shutdown tests. | Reopen only when a fresh spawn/timeout/channel path lacks shutdown, bounded queue, or cleanup evidence. |
| `Example Web API candidates` | 287 | Guarded | Current web/API examples are classified; stale WebSocket auth examples are fixed and remaining examples are covered by token, unsafe-open, CORS, and docs freshness gates. | Reopen only when a fresh example bypasses the SDK auth helpers, hard-codes secrets, or contradicts deployment posture. |
