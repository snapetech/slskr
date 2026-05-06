# slskR Bug Council Phase Tracker

slskR's council methodology is mirrored from slskNet.Runtime's canonical phase tracker. The shared schema (severity/confidence, sibling-search, behavior-pinning) and the slskR-adapted negative-space gate are the council-process upgrades; the existing `scripts/run-council-scan.sh` and `scripts/check-council-loop.sh` continue to handle candidate inventory and sweep-closure gating.

The canonical multi-phase tracker for the upgrade work itself lives in `../slskNet.Runtime/docs/dev/bug-council-phases.md`. This file tracks slskR-scoped follow-up phases on top of that.

## What is mirrored

- `bug-council-severity-schema.md` — verbatim copy. Severity/confidence tiers apply to slskR findings.
- `bug-council-sibling-search.md` — verbatim copy.
- `bug-council-behavior-pinning.md` — verbatim copy.
- `bug-council-negative-space.md` — slskR-adapted, declares slskR's wire-frame and protocol boundaries.
- `scripts/check-council-negative-space.sh` — slskR-adapted gate.
- slskNet.Runtime's current canonical analyzer cycle now includes broadened `CSL0001`, a second `CSL0002` loop-bound lens, calibration fixtures, and multi-seed adversarial corpora. The Rust follow-up phases below should use the same calibration rule before treating zero findings as meaningful.

## What is intentionally not mirrored

- The Roslyn analyzer is .NET-specific. The Rust counterpart (custom Clippy/dylint lens) is a follow-up phase.
- slskNet.Runtime's bash scanner is .NET-flavored. slskR already has `scripts/run-council-scan.sh` with Rust-flavored patterns.

## slskR phases

| # | Name | Status | Owner | Exit criteria |
| --- | --- | --- | --- | --- |
| 1 | Mirror council process docs + negative-space gate | Done | (agent) | Schema/sibling/behavior/negative-space docs and `check-council-negative-space.sh` present; gate passes locally. |
| 2 | Severity/confidence retrofit on existing inventory | Done | (agent) | `docs/dev/council-scan-inventory.md` candidate rows are annotated with severity/confidence per `bug-council-severity-schema.md`. |
| 3 | Rust semantic-lens beachhead | Done | (agent) | `scripts/check-rust-protocol-taint-lens.sh` implements a calibrated protocol-derived length/count to allocation/read/loop lens with known-bad and known-good fixtures. A future Clippy/dylint port may replace it, but the council now has a bug-finding semantic gate. |
| 4 | Rust loop-bound lens + calibration corpus | Done | (agent) | `scripts/check-rust-protocol-taint-lens.sh` covers protocol-derived loop bounds as well as allocation/read sinks, with bad/good calibration fixtures proving the detector fires and stays quiet on bounded paths. |
| 5 | Multi-seed adversarial protocol corpus | Done | (agent) | `crates/slskr-protocol/tests/adversarial.rs` runs known hostile corpus inputs and multiple deterministic random seeds through frame/message decoders; `scripts/check-rust-protocol-adversarial-corpus.sh` gates the corpus and test execution. |
| 6 | Council bughunt entrypoint | Done | (agent) | `scripts/run-council-bughunt.sh` delegates to the all-phases runner so legacy muscle memory cannot skip a council phase. |
| 7 | All-phases council runner | Done | (agent) | `scripts/run-bug-council-all-phases.sh` runs fresh candidate counts, process gates, calibrated semantic lenses, adversarial protocol corpus, and pending-phase checks in one command; `scripts/check-bug-council-all-phases.sh` is wired into remediation so partial runners regress loudly. |

## How to resume

1. Read this tracker; identify the first non-Done row.
2. Run `scripts/run-bug-council-all-phases.sh` (or the compatibility alias `scripts/run-council-bughunt.sh`). If it exits with pending phases, pick the first pending row and implement it.
3. Use `docs/dev/council-bughunt-playbook.md` for adjudication rules. Green gates alone are not a bughunt result.
