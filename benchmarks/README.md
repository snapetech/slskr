# Runtime benchmarks

Status: diagnostic-only. These commands are explicit, operator-run
measurements; the repository does not claim a nightly baseline or a CI timing
threshold until a retained workload artifact is produced for the target
environment.

The maintained HTTP benchmark is [`scripts/benchmark-http.py`](../scripts/benchmark-http.py).
It talks to a real running daemon and emits JSON with status counts, errors,
throughput, and bounded latency samples.

Example:

```bash
python3 scripts/benchmark-http.py \
  --base-url http://127.0.0.1:5030 \
  --profile native \
  --persistence disabled \
  --warmup 5 \
  --duration 30 \
  --concurrency 8 \
  --output target/perf/native-disabled.json
```

Add an authorization header when the daemon requires it:

```bash
python3 scripts/benchmark-http.py \
  --base-url http://127.0.0.1:5030 \
  --bearer-token "$SLSKR_API_TOKEN" \
  --endpoint 'GET /api/stats' \
  --endpoint 'GET /api/transfers/downloads'
```

The benchmark is intentionally limited to safe read methods. Mutation and
SignalR workloads belong in explicit scenario runners so their side effects
and event ordering remain visible rather than being hidden in a generic load
test.

Compare like-for-like live artifacts with explicit regression policy:

```bash
python3 scripts/compare-benchmark.py \
  target/perf/native-disabled-before.json \
  target/perf/native-disabled-after.json \
  --max-latency-regression-percent 10 \
  --max-throughput-regression-percent 10 \
  --output target/perf/native-disabled-comparison.json
```

The comparison rejects mismatched profiles, persistence labels, endpoint
workloads, status policies, concurrency, and timing settings. It reports
latency, throughput, and failure-rate checks for the aggregate and every
endpoint case; it does not silently treat missing measurements as a pass.

For persistence work, profile explicit read-only statements with
[`scripts/profile-sqlite.py`](../scripts/profile-sqlite.py). Its artifact pairs
the real SQLite query plan with bounded timing samples and row-count ranges,
so index or query changes can be evaluated against the same database shape.

The old standalone `benchmarks/benchmark.rs` simulated requests and was not a
valid performance measurement; it has been removed.

The file-transfer payload harness compares the former per-chunk `flush` path
with the current writer over real loopback TCP sockets. It validates received
plain and obfuscated payloads and reports bounded throughput samples. Tokio's
`TcpStream::flush` is a no-op, so results measure only local method-call and
scheduling effects; they do not establish throughput changes with remote peers.

```bash
cargo bench --locked -p slskr-client --bench file_transfer_payload -- \
  --mib 64 \
  --chunk-bytes 81920 \
  --trials 5 \
  --output benchmarks/artifacts/file-transfer-loopback.json
```

Keep a copy of the complete JSON output with any before/after report. Record
the toolchain and host conditions alongside it; compare only artifacts from
the same machine and workload. Cargo runs this harness from the
`crates/slskr-client` package directory, so relative `--output` paths resolve
there.

For the RF-021 SQLite indexes, the synthetic cardinality harness creates
production-shaped tables, compares the existing indexes with the retained
transfer/webhook composites, and profiles first and deep pages. Its generated
rows are diagnostic; production database cardinality and the daemon's bundled
SQLite version are separate evidence.

```bash
python3 benchmarks/benchmark-sqlite-cardinality.py \
  --output benchmarks/artifacts/sqlite-cardinality.json
```

The RF-046 search-response diagnostic compares the current fingerprint bucket
with the former linear equality scan using 900 seeded responses, 11 files per
response, and 900 duplicates of the final seeded response. The benchmark uses
the production `SearchResults` acceptance path for the current implementation
and a direct `Vec::contains` reference for the former comparison. It is
diagnostic only and sets no CI timing threshold.

```bash
cargo bench --locked -p slskr-client --bench search_response_dedup -- \
  --output ../../benchmarks/artifacts/search-response-dedup.json
```
