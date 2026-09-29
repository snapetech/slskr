# RF-038 Share Cache Allocation Benchmark (2026-09-29 UTC)

Internal-only diagnostic. The opt-in example and its `stats_alloc` dev
dependency do not affect default builds or runtime behavior.

The benchmark compares the current production share-cache writer with the
historical `replace`/`format!`/`Vec<String>` serializer on 50,000 generated
entries. Seven measured trials alternate writer order. It first asserts
byte-for-byte equality, then records allocation statistics and elapsed time
for synchronous file writes. It runs without a daemon, database, network, or
peer port; its temporary directory is removed at exit.

## Result

| Measure | Current writer | Historical serializer |
| --- | ---: | ---: |
| Median allocator calls (allocations + reallocations) | 20 | 408,828 |
| Median requested bytes | 4,194,304 | 14,370,775 |
| Median elapsed time, including file write | 23.71 ms | 35.14 ms |
| Output bytes | 2,702,955 | 2,702,955 |

The allocation counts are process-wide `System` allocator observations from
`stats_alloc` 0.1.10, sampled only around each write. Elapsed times are
diagnostic: allocator instrumentation and filesystem caching affect them, so
they are not a stable speedup claim or CI threshold. The machine was an AMD
Ryzen 9 9950X3D host running Linux 7.1.7 x86_64 with rustc 1.94.0.

Reproduce from the repository root:

```bash
cargo run --locked -p slskr --features rf-benchmarks \
  --example share_cache_allocations -- \
  --output benchmarks/artifacts/20260929-rf038-share-cache-allocations.json
```

The raw result is retained in
[`20260929-rf038-share-cache-allocations.json`](20260929-rf038-share-cache-allocations.json).
