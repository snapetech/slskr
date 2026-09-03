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

The old standalone `benchmarks/benchmark.rs` simulated requests and was not a
valid performance measurement; it has been removed.
