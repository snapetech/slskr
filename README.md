# slskr

`slskr` is a Rust app for the Soulseek network. It ships as one service with a daemon, HTTP API, web UI, protocol runtime, live probes, and test automation in the same workspace.

The project goal is simple: run a dependable self-hosted Soulseek client with first-class automation, observability, and browser control.

## Responsible Use

slskr does not endorse copyright infringement or unlawful sharing. Users and operators are responsible for complying with applicable law, third-party rights, and network rules.

## What It Includes

- A bundled `slskr serve` daemon with the web UI and REST API.
- Typed Soulseek protocol codecs for server, peer, transfer, distributed, and init traffic.
- Async session and peer runtime for login, keepalive, listeners, searches, browsing, messaging, and transfers.
- Direct, obfuscated, and indirect peer connection support.
- Share scanning, share catalog APIs, safe download paths, transfer queueing, resume support, and transfer progress events.
- Bearer-token API auth, same-site browser sessions, CSRF origin checks for mutating requests, and rate limiting.
- Runtime health, metrics, telemetry, event polling, and WebSocket event feeds.
- Live smoke probes for login, peer metadata, peer messages, browsing, file transfer, private messages, rooms, and multi-account local peer paths.

## Workspace

- `crates/slskr`: the app binary, HTTP server, API surface, web UI serving, storage projections, and daemon orchestration.
- `crates/slskr-client`: async network runtime for server sessions, peer connections, listeners, searches, transfers, and interop probes.
- `crates/slskr-protocol`: protocol message types, binary codecs, and wire-format tests.
- `crates/slskr-cli`: internal probe and smoke-command runner exposed through `slskr` subcommands while the app surface matures.
- `web`: the bundled browser UI.

## Run Locally

```bash
cargo run -p slskr -- version
```

```bash
SLSK_USERNAME=<user> \
SLSK_PASSWORD=<pass> \
cargo run -p slskr -- login smoke
```

```bash
SLSK_USERNAME=<user> \
SLSK_PASSWORD=<pass> \
SLSKR_AUTO_CONNECT=true \
cargo run -p slskr -- serve
```

By default, `slskr serve` binds to `127.0.0.1:5030`. Configure it with environment variables or `SLSKR_CONFIG=/path/to/config.toml`; see [docs/slskr.config.example.toml](./docs/slskr.config.example.toml).

## Common Checks

```bash
cargo fmt --all --check
cargo test --workspace
```

```bash
scripts/check-release-package.sh
```

```bash
SLSKR_A_USERNAME=<user-a> \
SLSKR_A_PASSWORD=<pass-a> \
SLSKR_B_USERNAME=<user-b> \
SLSKR_B_PASSWORD=<pass-b> \
SLSKR_INDIRECT_HOST_OVERRIDE=127.0.0.1 \
cargo run -p slskr -- smoke local-peer
```

```bash
scripts/run-live-http-transfer-smoke.sh
```

The live HTTP transfer smoke starts isolated daemons, enforces API auth, verifies public peer metadata with a separate probe account, creates a download through the HTTP API, verifies the downloaded bytes, and can hold a bounded connected soak.

## Repository

Canonical repository: `https://github.com/snapetech/slskr`

## License

AGPL-3.0-only. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
