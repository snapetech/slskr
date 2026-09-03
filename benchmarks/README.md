# Runtime benchmarks

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

The old standalone `benchmarks/benchmark.rs` simulated requests and was not a
valid performance measurement; it has been removed.
