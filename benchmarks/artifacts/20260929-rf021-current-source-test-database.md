# RF-021 Current-Source Isolated SlskR Database Profile — 2026-09-29

Status: internal diagnostic evidence. This profile uses synthetic test rows;
it is not a measurement of a production SlskR database.

## Current-source database

The database was initialized from the x86_64 musl binary in GitHub CI run
36613453093, built from source
69cf4141325f234bdd63c3f52df179b069607781. The CI archive SHA-256 is
de9d0850d9f4f7950424fe040c6a4b2b3c5e63badc65e35941f7233ff2d1c18b.
On kspls0, the SlskR server command ran with persistence enabled in an Alpine
container using network mode none, a read-only container root, no credentials,
and no published host ports. The no-start option exited after the current
database migrations; Docker removed the short-lived container. No host
listener or production service was changed.

The newly initialized tables were empty. A deterministic fixture from
[benchmark-sqlite-cardinality.py](../benchmark-sqlite-cardinality.py) then
seeded 1,000,000 search results, 500,000 transfers, and 500,000 webhook logs
into those migration-created tables. The resulting SQLite file was
555,638,784 bytes and PRAGMA integrity_check returned ok. The machine report
retains row counts, table indexes, query plans, timings, and exact source and
CI artifact hashes in
[the JSON output](20260929-rf021-current-source-test-database.json).

## Page measurements

The profile used three warmups and 20 measured queries per case, returning 100
full rows per page. First-page and deep-page medians were:

| Query | First page | Deep page | Query plan |
| --- | ---: | ---: | --- |
| Search results | 0.313 ms | 0.341 ms at offset 500 | idx_search_results_search |
| Queued transfers | 0.510 ms | 0.560 ms at offset 1,000 | idx_transfers_status_started |
| Webhook logs | 0.271 ms | 0.240 ms at offset 1,000 | idx_webhook_logs_webhook_timestamp |

Every case returned 100 rows on all measured iterations. The query plans use
their intended indexes without a temporary sort.

The profiler ran with Python 3.14.7 and SQLite 3.53.4 on Linux x86_64 with 32
logical CPUs. As with the existing cardinality benchmark, this is a Python
SQLite profile rather than a timing through the daemon-linked SQLite library.

## Limit

The workload is generated test data and does not represent production
cardinalities. There is no production SlskR database on kspls0. This closes the
requested test-instance path and confirms the current schema and indexes at the
existing scale; it does not make a production-cardinality claim.
