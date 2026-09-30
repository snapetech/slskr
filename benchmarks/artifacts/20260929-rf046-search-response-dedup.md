# RF-046 Duplicate Search Response Benchmark (2026-09-29 UTC)

Internal-only diagnostic. No user-facing behavior or CI timing threshold was
changed.

The release-profile benchmark compares the production fingerprinted
`SearchResults` acceptance path with the former `Vec::contains` equality scan.
It seeds 900 responses with 11 file entries each, where the first ten entries
are shared and the final entry is unique. It then submits 900 duplicates of
the last seeded response across seven measured trials after one warmup.

The current path measured a 716,366 ns median; the former linear scan measured
31,642,837 ns, or 44.171× slower for this synthetic duplicate-heavy case. The
linear upper bound is 810,000 full-response comparisons and 8,910,000
file-entry comparisons; the fingerprint path hashes 9,900 file entries across
the measured duplicate attempts. Machine-readable samples are in
[`20260929-rf046-search-response-dedup.json`](20260929-rf046-search-response-dedup.json).

Environment: Rust 1.94.0, optimized Cargo bench profile, Linux x86_64, AMD
Ryzen 9 9950X3D. This isolates the in-memory duplicate check; it does not claim
production-peer latency or an end-to-end search speedup. Re-run with:

```bash
cargo bench --locked -p slskr-client --bench search_response_dedup -- \
  --output ../../benchmarks/artifacts/20260929-rf046-search-response-dedup.json
```
