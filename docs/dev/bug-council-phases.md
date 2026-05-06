# slskR Bug Council Phase Tracker

slskR's council methodology is mirrored from slskNet.Runtime's canonical phase tracker. The shared schema (severity/confidence, sibling-search, behavior-pinning) and the slskR-adapted negative-space gate are the council-process upgrades; the existing `scripts/run-council-scan.sh` and `scripts/check-council-loop.sh` continue to handle candidate inventory and sweep-closure gating.

The canonical multi-phase tracker for the upgrade work itself lives in `../slskNet.Runtime/docs/dev/bug-council-phases.md`. This file tracks slskR-scoped follow-up phases on top of that.

## What is mirrored

- `bug-council-severity-schema.md` — verbatim copy. Severity/confidence tiers apply to slskR findings.
- `bug-council-sibling-search.md` — verbatim copy.
- `bug-council-behavior-pinning.md` — verbatim copy.
- `bug-council-negative-space.md` — slskR-adapted, declares slskR's wire-frame and protocol boundaries.
- `scripts/check-council-negative-space.sh` — slskR-adapted gate.

## What is intentionally not mirrored

- The Roslyn analyzer is .NET-specific. The Rust counterpart (custom Clippy/dylint lens) is a follow-up phase.
- slskNet.Runtime's bash scanner is .NET-flavored. slskR already has `scripts/run-council-scan.sh` with Rust-flavored patterns.

## slskR phases

| # | Name | Status | Owner | Exit criteria |
| --- | --- | --- | --- | --- |
| 1 | Mirror council process docs + negative-space gate | Done | (agent) | Schema/sibling/behavior/negative-space docs and `check-council-negative-space.sh` present; gate passes locally. |
| 2 | Severity/confidence retrofit on existing inventory | Pending | (agent) | `docs/dev/council-scan-inventory.md` rows annotated with severity/confidence per `bug-council-severity-schema.md`. |
| 3 | Rust semantic-lens beachhead | Pending | (agent) | One Clippy custom lens or `dylint` driver implementing TaintToAllocation in Rust (read_u32_le → Vec::with_capacity without bound). |

## How to resume

1. Read this tracker; identify the first non-Done row.
2. Run `scripts/check-council-negative-space.sh` and `scripts/check-council-loop.sh` to confirm a green baseline.
3. Pick up the row, update its status, and follow its exit criteria.
