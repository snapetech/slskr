# SQLite Cardinality Benchmark — 2026-09-24

Status: internal synthetic diagnostic evidence. The fixture uses the
production table columns and read projections, but contains generated rows. It
is not a measurement of an operator's database or a CI threshold.

## Workload and limits

- Generator/profiler: `benchmarks/benchmark-sqlite-cardinality.py`, reusing
  `scripts/profile-sqlite.py`
- Dataset: 1,000,000 search results across 1,000 searches; 500,000 transfers
  with 1% queued; 500,000 webhook logs across 100 webhooks
- Queries: first and offset-1,000 pages of 100 full rows; search results also
  include an offset-500 page
- Samples: three warmups and 20 measured queries per case
- SQLite: Python `sqlite3` 3.53.4; the daemon's bundled SQLite is 3.46.0
- Host: Linux 7.1.7-arch1-1, x86_64, AMD Ryzen 9 9950X3D

The synthetic row distributions make selective transfer and webhook pages
visible. Their timings cannot be projected directly onto a production
database, and the SQLite library version differs from the daemon build.

## Results

| Query | Existing indexes | Retained composite indexes | Median change |
| --- | ---: | ---: | ---: |
| Queued transfers, first page | 1.363 ms | 0.162 ms | -88.1% |
| Queued transfers, offset 1,000 | 14.936 ms | 0.180 ms | -98.8% |
| Webhook logs, first page | 6.743 ms | 0.079 ms | -98.8% |
| Webhook logs, offset 1,000 | 7.178 ms | 0.098 ms | -98.6% |
| Search results, first page | 0.091 ms | 0.091 ms | 0.0% |
| Search results, offset 500 | 0.100 ms | 0.100 ms | 0.0% |

The transfer and webhook plans change to their composite indexes. Search-result
plans continue to select the existing `(search_id)` index without a temporary
sort, including when the `(search_id, id)` index is present. Because `id` is
an `INTEGER PRIMARY KEY`, SQLite already orders entries in the single-column
index by the implicit rowid. The redundant composite index added 24,059,904
bytes to this fixture and showed no query-plan or median improvement. The
three original composite indexes added 47,071,232 bytes in total; the retained
transfer and webhook pair added 23,011,328 bytes.

The benchmark therefore retains the transfer and webhook indexes and removes
the search-result composite index at startup. The focused query-plan and
file-backed migration regressions run against the daemon's bundled SQLite;
real-database cardinality measurements remain open. Confidence is moderate in
the synthetic timing results and high that SQLite's rowid ordering makes the
search-result composite redundant.

Complete machine-readable output:

- [`20260924-sqlite-cardinality.json`](20260924-sqlite-cardinality.json)
