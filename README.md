# slskr

`slskr` is the public product name for this Rust Soulseek-network client/server app.

Goal: ship one runnable app, `slskr`, with the daemon/API/web UI bundled the way the .NET app is bundled, while keeping the Rust internals split into focused crates for protocol and runtime testing.

`slskr` is an independent protocol implementation for the Soulseek network.

Status: **Phases 1-7 initial cuts done; Phase 8 hardening started; 24h soak still pending**. See [PLAN.md](./PLAN.md) for the full roadmap.

Compliance and network-use notes are tracked in [COMPLIANCE.md](./COMPLIANCE.md).
The app command/API direction is tracked in [docs/app-surface.md](./docs/app-surface.md).
Install and service guidance is tracked in [docs/install.md](./docs/install.md).
App/API parity notes are tracked in [docs/legacy-port-harvest.md](./docs/legacy-port-harvest.md).

## Product Shape

External users should see one app:

- `slskr`: the installed service/app, including daemon/API/web UI.

Internal crates are implementation boundaries, not separate product brands:

- `slskr-protocol`: pure wire codecs and typed protocol messages; no I/O and no async runtime.
- `slskr-client`: embeddable Soulseek runtime engine for login/session management, server and peer connections, search, transfers, distributed search, listeners, and events.
- `slskr-cli`: internal command-runner crate retained during migration; probe/admin commands are exposed through `slskr` subcommands.

Current gap: the workspace has the protocol/runtime/probe pieces and a first `slskr serve` daemon scaffold with a background session manager, listener ownership, peer metadata responses, startup/rescan share indexing with a state-dir cache, share catalog/root file APIs, browse/search responses, direct and indirect transfer execution/resume with chunked progress events, max-active admission control, inbound/outbound transfer allow switches, reloadable transfer records, bearer-token and same-site browser-cookie auth, cross-site mutating-request rejection, a bundled local dashboard, session privilege checks, projection event/stats/metrics/telemetry endpoints, user watch/stats/browse controls, room-list sync plus room join/leave/message controls, and initial install/service docs. It still needs durable app storage beyond interim JSON/TSV state and fuller parity across browse, messages, rooms, and long-running health views.

## Commands

- `cargo run -p slskr -- version`
- `SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> cargo run -p slskr -- login smoke`
- `SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> cargo run -p slskr -- soak live`
- `SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> cargo run -p slskr -- probe peer-address`
- `SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> cargo run -p slskr -- probe plain-peer`
- `SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_PEER_USERNAME=<peer> cargo run -p slskr -- probe indirect-peer`
- `SLSK_USERNAME=<user> SLSK_PASSWORD=<pass> SLSK_OBFUSCATED_PEER_USERNAME=<peer> cargo run -p slskr -- probe obfuscated-peer`
- `SLSKR_A_USERNAME=<user> SLSKR_A_PASSWORD=<pass> SLSKR_B_USERNAME=<user> SLSKR_B_PASSWORD=<pass> cargo run -p slskr -- smoke local-peer`

`login smoke` defaults to `server.slsknet.org:2242` and listen port `2234`; override with `SLSK_SERVER` and `SLSK_LISTEN_PORT`.

`slskr serve` can load persistent app settings from `SLSKR_CONFIG=/path/to/config.toml` or `$XDG_CONFIG_HOME/slskr/config.toml`. Environment variables still override file values. See [docs/slskr.config.example.toml](./docs/slskr.config.example.toml).

`soak live` is deliberately low-volume. It logs in, binds a listener, advertises the wait port, sends normal keepalive/status/share-count messages, observes server events, and accepts inbound peer attempts for a bounded window. Defaults: `SLSK_SOAK_SECONDS=60`, `SLSK_SOAK_MAX_EVENTS=40`, `SLSK_SOAK_PING_SECONDS=30`, `SLSK_SOAK_SHARED_FOLDERS=0`, `SLSK_SOAK_SHARED_FILES=0`. Optional probes are explicit: `SLSK_SOAK_PEER_USERNAME=<user>` requests watch/status/stats/address for one peer, and `SLSK_SOAK_SEARCH_QUERY=<query>` sends one search.

Set `SLSK_SOAK_OBFUSCATED_LISTENER_BIND=0.0.0.0:0` to bind and advertise a type-1 obfuscated peer-message listener during `soak live`. The helper `scripts/run-live-soak-24h.sh` runs the low-volume 24h soak with this enabled and writes logs under `target/live-soak/`.

For Proton VPN NAT-PMP testing, store WireGuard configs under the gitignored `.secrets/` directory and use `scripts/run-live-soak-proton-natpmp.sh`. It claims Proton NAT-PMP TCP mappings, renews them during the soak, and exports `SLSK_SOAK_ADVERTISED_PORT` / `SLSK_SOAK_OBFUSCATED_ADVERTISED_PORT` so the public server sees the forwarded ports instead of local bind ports.

`scripts/run-proton-public-matrix.sh` rotates NAT-PMP-capable Proton listener configs and probes them from the other Proton configs through isolated Linux network namespaces. It records metadata lookup, plain direct peer, obfuscated direct peer, distributed direct, file-transfer direct, indirect `ConnectToPeer` / `PierceFirewall`, metadata stability, and negative indirect results under `target/live-soak/*.tsv`. NAT-PMP-off configs are useful as probe-only/firewalled clients.

Useful matrix variants:

- `SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL=0` advertises the Proton NAT-PMP public regular port. This should pass plain direct, obfuscated direct, and indirect reachability on working endpoints.
- `SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL=1` advertises local regular port `2234` while still advertising the forwarded obfuscated port. This models obfuscated-only public reachability: plain direct should fail, obfuscated direct and indirect should still pass.
- `SLSKR_MATRIX_INDIRECT_SEND_PEER_ADDRESS=1` sends `GetPeerAddress` immediately after `ConnectToPeer`, matching the request sequencing used by some clients.
- `SLSKR_MATRIX_PLAIN_PEER_INIT_TOKEN=<n>` and `SLSKR_MATRIX_OBFUSCATED_PEER_INIT_TOKEN=<n>` test nonzero direct `PeerInit` tokens for legacy compatibility.
- `SLSKR_MATRIX_SHOW_PEER_IP=1` prints server-returned peer IPs in matrix output. Use only for private lab runs.

`probe peer-address` repeatedly requests public peer metadata for one target and prints the regular port, obfuscation type, and obfuscated port returned by the server.

`probe plain-peer` logs one account into the configured server, requests `SLSK_PEER_USERNAME`, connects to the returned regular peer port, sends plain `PeerInit`, and round-trips `UserInfoRequest`. Timeout defaults to `SLSK_PLAIN_PROBE_TIMEOUT_SECONDS=15`.

`probe indirect-peer` logs one account into the configured server, binds a listener, advertises `SLSK_INDIRECT_ADVERTISED_PORT`, sends `ConnectToPeer` for `SLSK_PEER_USERNAME`, accepts the peer's `PierceFirewall`, and answers `UserInfoRequest`. Timeout defaults to `SLSK_INDIRECT_PROBE_TIMEOUT_SECONDS=20`.

`probe distributed-peer` connects to the peer's regular listener with `PeerInit` type `D` and sends a distributed ping. `probe file-transfer-peer` connects with `PeerInit` type `F` and verifies a raw transfer token echo. `probe metadata-relogin` checks that peer metadata stays stable across a second login. `probe negative-indirect` intentionally advertises no listener and waits for `CantConnectToPeer`; real clients must still keep their own timeout because the server/peer path does not reliably produce that message.

`probe obfuscated-peer` logs one account into the configured server, requests `SLSK_OBFUSCATED_PEER_USERNAME`, requires advertised obfuscation type `1`, connects to the returned obfuscated port, sends an obfuscated `PeerInit`, and round-trips `UserInfoRequest`. Use `SLSK_OBFUSCATED_HOST_OVERRIDE=<host>` only for same-host or lab routing; otherwise it uses the server-returned address. Timeout defaults to `SLSK_OBFUSCATED_PROBE_TIMEOUT_SECONDS=15`.

`smoke local-peer` logs two accounts into the configured server, then exercises direct loopback peer-message, obfuscated peer-message, and file-transfer handshakes locally. It also validates the server-mediated indirect `ConnectToPeer`/`PierceFirewall` path. Use `SLSKR_A_USERNAME`, `SLSKR_A_PASSWORD`, `SLSKR_B_USERNAME`, and `SLSKR_B_PASSWORD` for the two test accounts.

Indirect smoke defaults to the server-returned peer address. For same-host or same-LAN test accounts behind NAT, set `SLSKR_INDIRECT_HOST_OVERRIDE=127.0.0.1` to validate the server-issued token and local listener handshake without depending on NAT hairpinning. Other knobs: `SLSKR_INDIRECT_LISTENER_BIND=0.0.0.0:0` and `SLSKR_INDIRECT_TIMEOUT_SECONDS=10`.

## Local Protocol Simulator

The `slskr-client` integration suite can use Soulfind as a local dev-time server simulator. It is optional and never a runtime dependency.

- `SOULFIND_PATH=/path/to/soulfind cargo test -p slskr-client --test soulfind_contract`
- `SLSK_SOULFIND_DOCKER=1 cargo test -p slskr-client --test soulfind_contract`

Without either setting, the Soulfind contract test skips cleanly.

## Obfuscated Peer Transport

`slskr-protocol` implements Soulseek rotated obfuscation type `1`, including the public aioslsk vector. `slskr-client` includes obfuscated init and obfuscated peer-message (`P`) stream wrappers, plus `SetWaitPort` metadata advertisement for a regular and obfuscated port.

Soulfind is used to validate server metadata relay and local multi-client reachability: `SetWaitPort` with obfuscation fields is preserved through `GetPeerAddress`, and the contract suite runs multiple local sessions/listeners through direct `PeerInit`, indirect `ConnectToPeer`/`PierceFirewall`, and obfuscated peer-message handshakes without CLI host overrides. Dedicated listener tests also reject plain traffic on the obfuscated path and obfuscated traffic on the plain path. Soulfind does not validate compatibility with proprietary client implementations by itself; real-client interop still needs a reachable obfuscated listener test.

Current interop matrix:

| Path | Status |
| --- | --- |
| `slskr` plain peer to `slskr` plain peer | Tested |
| `slskr` obfuscated type-1 peer to `slskr` obfuscated type-1 peer | Tested |
| Public Proton plain direct peer after reversed-IP fix | Tested |
| Public Proton obfuscated direct peer after reversed-IP fix | Tested |
| Public Proton distributed `D` direct peer after reversed-IP fix | Tested |
| Public Proton file-transfer `F` direct peer after reversed-IP fix | Tested |
| Public Proton indirect `ConnectToPeer`/`PierceFirewall` after reversed-IP fix | Tested |
| Public Proton negative indirect `CantConnectToPeer` | Timeout required; do not rely on server response |
| Plain traffic accepted by obfuscated listener | Rejected by tests |
| Obfuscated traffic accepted by plain listener | Rejected by tests |
| Real-client obfuscated peer interop | Harness exists; reachable peer still pending |

## Verification

- `scripts/check-public-posture.sh`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `SLSK_SOULFIND_DOCKER=1 cargo test -p slskr-client --test soulfind_contract`
- `cargo package --workspace --allow-dirty --no-verify`
- `SLSKR_INDIRECT_HOST_OVERRIDE=127.0.0.1 cargo run -p slskr -- smoke local-peer`
- `cargo run -p slskr -- soak live`
- `scripts/run-live-soak-24h.sh`
- `scripts/run-live-soak-proton-natpmp.sh`
- `scripts/run-proton-public-matrix.sh`

## Repos

- Primary: GitHub (`snapetech/soulseekR`, private; rename/public posture still pending)
- Mirror: gitlab.home (`securityops/soulseekR`, private; rename/public posture still pending)

`origin` is configured to push to both.

## Backfill Gaps

- Build out the `slskr` daemon/API/web UI inside the existing `slskr` binary package.
- Keep `slskr-cli` as an internal migration crate until the app surface is mature, then remove or privatize it.
- Rename public repository/package metadata away from `soulseekR` before any public release.
- Finish durable app storage beyond interim JSON/TSV state and remaining browse/message/room parity.
- Re-run the public-posture scrub after renaming so docs do not present this as an endorsed release or successor package for another project.

## License

AGPL-3.0-only. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
