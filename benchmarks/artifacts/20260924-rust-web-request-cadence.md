# Rust Web request-count and cadence budgets (2026-09-24)

The headless Rust Web audit now records API request counts and repeated-endpoint
intervals for 15 routes at desktop and mobile sizes. In the default mock
success scenario, it enforces a per-route request-count cap and rejects repeated
requests to one endpoint less than 200 ms apart. These budgets run only with the
default 500 ms settle window and a mock backend; live request counts vary by
backend state and are not compared with the fixed caps.

The baseline audit made 518 API requests across 30 route views. It requested
`/transfers/speeds` twice on Downloads, Uploads, and System within 11–96 ms,
because the route summary and the persistent player read the same endpoint.
The Rust WASM fetch path now shares that response for up to 200 ms. The rebuilt
audit made 512 requests, six fewer, and had no repeated endpoint below the
200 ms floor. `/solid/status` still appears twice on the Solid route at
594.8–608.3 ms in the retained run; its count remains within the route budget.

Run the evidence gate with:

```bash
SLSKR_RUST_WEB_AUDIT_SETTLE_MS=500 \
  scripts/with-process-memory-guard.sh node scripts/audit-rust-web-ui.mjs
```

The retained per-route counts and observed repeated-endpoint intervals are in
[`20260924-rust-web-request-cadence.json`](20260924-rust-web-request-cadence.json).
This is local mock-browser evidence, not a deployed-server cadence measurement.
