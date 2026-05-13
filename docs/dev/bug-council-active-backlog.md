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
- `Guarded` - narrow probe is empty and protected by remediation checks.
- `Accepted` - confirmed bug class exists and is being fixed.
- `Existing guard` - candidates are covered by existing behavior and gates.
- `False positive` - scanner shape is not a bug for the listed rationale.
- `Out of scope` - candidate belongs outside this council.

| Section | Candidate count | Status | Current classification | Next action |
| --- | ---: | --- | --- | --- |
| `Constructor/mutable collection candidates` | 7 | Open | Small ownership queue. Prior SDK/runtime sweeps fixed confirmed mutable captures, but remaining constructors should be classified as snapshots, immutable wrappers, or accepted ownership leaks. | Table every remaining constructor candidate and promote any caller-owned mutable capture to the ledger. |
| `Protocol count/length candidates` | 46 | Open | Protocol-derived count/length queue. Fresh scan adds folded smoke/probe runner and package-gate code to the broad count; calibrated Rust taint/adversarial gates still cover the highest-risk unbounded parser loops. | Split parser counts, bounded reads, transfer chunk lengths, smoke/probe paths, and fixture rows; accept only unbounded wire-derived allocation/read/loop flows. |
| `Protocol scalar emission candidates` | 42 | Open | Scalar narrowing/emission queue. Fresh scan adds folded smoke/probe runner and release-gate code to the broad count; known API protocol narrowing bugs are fixed, but command builders and fixtures still need a stable classification register. | Classify checked conversions vs raw casts and add a narrow gate for any remaining protocol-visible wrap risk. |
| `Resolver/raw stream candidates` | 225 | Open | Broad stream and resolver queue. Fresh scan adds folded smoke/probe runner call sites; confirmed SDK raw-frame/connect-timeout bugs were fixed, but remaining candidates mix safe stream plumbing, route handlers, and protocol frame helpers. | Split SDK stream helpers, daemon resolver outputs, smoke/probe connectors, route body streams, and test fixtures; burn down accepted subgroups in batches. |
| `Task/cancellation/lifecycle candidates` | 246 | Open | Broad async lifecycle queue across daemon, SDK, web, tests, and the folded smoke/probe runner. Existing gates cover several release/runtime lifecycle classes, but this pile is not fully classified. | Split cancellation ownership, spawned tasks, timeout paths, channel shutdown, smoke/probe waits, and test-only rows; promote silent hangs/leaks into focused gates. |
| `Example Web API candidates` | 287 | Open | Broad HTTP/API queue. Existing route/auth/storage/OpenAPI gates cover many classes, but compatibility and example-shaped HTTP paths still need durable subgroup classification. | Split path/query validation, response-body exposure, compatibility no-ops, OpenAPI drift, and test fixture rows. |
