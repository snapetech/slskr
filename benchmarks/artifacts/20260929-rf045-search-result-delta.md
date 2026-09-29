# RF-045 Search Result Delta Benchmark and Restart Proof (2026-09-29 UTC)

Internal-only diagnostic. The opt-in example and its feature-gated persistence
exports do not affect default builds or runtime behavior.

The benchmark replays 10,000 deterministic results through 50 batches of 200
against the production SQLite methods. The current path appends only each
accepted batch; the historical path rewrites the complete accumulated result
projection after each batch. Three alternating trials use fresh in-memory
databases. Schema creation and final verification are outside the timed region.

## Result

| Measure | Delta append | Full projection rewrite |
| --- | ---: | ---: |
| Median persistence time | 31.47 ms | 799.82 ms |
| Result rows inserted over all batches | 10,000 | 255,000 |
| Final persisted rows verified | 10,000 | 10,000 |

The timing is a development-profile, synthetic SQLite diagnostic, not a
production workload estimate or CI threshold. The structural row-count
difference follows the 50-batch workload and is independent of wall-clock
timing.

A separate file-backed proof ran the current append path in a child process,
waited until all 50 transactions had committed, then terminated that process
abruptly. A new process reopened the database and verified the complete 10,000
row projection, first/last filenames, and search result count. This checks
recovery after committed writes, not a crash in the middle of an SQLite
transaction. All temporary files and the child process were cleaned up.

The host was an AMD Ryzen 9 9950X3D system running Linux 7.1.7 x86_64 with
rustc 1.94.0. Reproduce from the repository root:

```bash
cargo run --locked -p slskr --features rf-benchmarks \
  --example search_result_delta -- \
  --output benchmarks/artifacts/20260929-rf045-search-result-delta.json
```

The raw result is retained in
[`20260929-rf045-search-result-delta.json`](20260929-rf045-search-result-delta.json).
