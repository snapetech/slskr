# slskR Bug Council Negative-Space Gate

This document declares slskR's trust boundaries and the validator each one must run, so a missing validator is itself a CI failure. The gate is enforced by `scripts/check-council-negative-space.sh`.

slskR already runs `scripts/run-council-scan.sh` and `scripts/check-council-loop.sh` to inventory candidates and gate sweep closure. The negative-space gate is an inversion of that flow: instead of cataloging existing call sites that look suspicious, it lists trust boundaries by name and asserts the validator is wired.

## Boundaries

| Boundary | Source | Sink file(s) | Required validator |
| --- | --- | --- | --- |
| Wire-frame length bounds | TCP from server / peer | `crates/slskr-client/src/io.rs` | `DEFAULT_MAX_FRAME_LEN`, `read_raw_frame_with_max` |
| Length-prefixed reads | Wire frame body | `crates/slskr-protocol/src` | `read_string`, `read_len_prefixed_bytes` (each must reject lengths beyond remaining input) |
| Decompressed payload | Wire frame body | `crates/slskr-protocol/src` | bounded decompression (no unbounded `Vec::with_capacity` from a network scalar) |

## Adding a new boundary

1. Add a row above with the boundary, the file(s) it lives in, and the validator symbol you've placed in those file(s).
2. Add an `assert_validator_present` line to `scripts/check-council-negative-space.sh`.
3. Add a behavior-pinned test per `docs/dev/bug-council-behavior-pinning.md`.

## Removing a boundary

Removing a row requires a council sweep entry explaining why the boundary no longer exists. The remediation baseline must be updated in the same change.

## Why this matters

Most council catches in mature codebases are of the shape "a guard exists for boundary A, was forgotten for boundary B." The negative-space gate inverts the search: it lists every boundary by name and asserts the guard symbol is in place. That makes "I added a new boundary and forgot to think about it" the failure mode that's hardest to commit.
