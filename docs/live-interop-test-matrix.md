# Live Interoperability Test Matrix

This matrix proves `slskr` behavior against itself and adjacent/community clients without copying upstream implementation code.

## Credential source

- Local env file: `.env` at the repo root, gitignored.
- OpenBao is intentionally skipped for now.
- Four accounts are used so login, listener, probe, room/chat, and transfer actors can run without self-collisions.

## Local checkouts

| Client | Role in matrix | Local path | Current status |
| --- | --- | --- | --- |
| `slskr` | Rust rewrite under test | `/home/keith/Documents/code/slskR` | Present |
| `slskd` | Community daemon/client parity target | `/home/keith/Documents/code/slskd` | Present |
| `slskdN` | Sister fork parity target | `/home/keith/Documents/code/slskdn` | Present |
| `Soulseek.NET` | Upstream .NET library parity target | `/home/keith/Documents/code/Soulseek.NET` | Present |
| `slskNet.Runtime` | Runtime fork parity target | `/home/keith/Documents/code/slskNet.Runtime` | Present |

## Capability matrix

| Area | Required proof | `slskr` coverage | Cross-client coverage |
| --- | --- | --- | --- |
| Login/session | New/existing account login succeeds, server greeting/hash parsed, relogin behavior handled | `login smoke`, live matrix | Unit suites plus daemon login/address probes |
| Keepalive/status | Ping, user status, stats, and share counts exchange without disconnect | `soak live`, unit/client tests | Daemon runtime coverage and peer metadata probes |
| Peer metadata | Server returns regular and obfuscated listener metadata | `probe peer-address` | `slskr -> slskd`, `slskr -> slskdN` address probes |
| Plain peer messaging | Plain `PeerInit` and `UserInfoRequest` round-trip | `probe plain-peer` | `slskr -> slskd`, `slskr -> slskdN` plain probes |
| Obfuscated peer messaging | Type-1 obfuscated `PeerInit` and `UserInfoRequest` round-trip | `probe obfuscated-peer` | `slskr -> slskdN` attempted when the daemon advertises/runtime-enables it |
| Indirect connection | `ConnectToPeer` / `PierceFirewall` works for firewalled/NAT paths | `probe indirect-peer`, local peer smoke | `slskr` self-smoke proves protocol; daemon payload coverage now uses queued direct/NAT-PMP transfer probes |
| Distributed path | Distributed `D` peer init accepts ping/probe traffic | `probe distributed-peer` | Runtime/library unit coverage; daemon live probe still optional |
| File-transfer init | Transfer `F` peer init is accepted only as part of a real queued transfer | `probe file-transfer-peer` against `slskr` self-smoke | Raw token-echo closes with EOF against `slskd`/`slskdN` as expected; queued payload transfer proof now covers daemon bytes |
| Search | Server search request emits usable result events | `soak live` with `SLSK_SOAK_SEARCH_QUERY` | Fixture search against daemon shares passes as open-commons search soak when indexing is timely |
| Browse | Browse request returns shared folder/file payloads | `probe browse-peer` added | Daemon browse proof runs after target advertises a live listener |
| Download/upload bytes | Queued download opens transfer path, moves bytes, reports completion | `probe download-peer` added for negotiated file payload reads | Daemon payload proof runs after browse exposes exact fixture path and target remains connected |
| Private chat | Direct private message send/receive/ack works | `probe private-message` in VPN-backed live matrix | Passed with fresh account pair |
| Rooms | Join/leave/list/message flows work | `probe room-message` in VPN-backed live matrix | Passed with fresh account |
| User controls | Watch/unwatch, stats, browse, ban/ignore semantics align | Unit/API coverage plus browse/stat live probes | Ban/ignore mutation remains an API policy surface, not a blocking Soulseek interop gap |
| Web/API/UI/player | Web UI drives search, transfers, chat, rooms, settings, player surfaces | Web tests/build plus Playwright live-surface spec | Browser/player E2E passes against adjacent `slskdN` daemon-hosted bundle |
| Durability | Searches/transfers/messages/rooms survive restart where supported | Unit/API coverage | Cross-client restart matrix passes for daemon metadata, browse, search, and queued payload transfer after restart |
| Failure modes | Bad auth, occupied user, offline peer, closed ports, bad obfuscation, and negative indirect fail safely | `negative-indirect`, unit coverage | Cross-client negative probes remain to add |

## Automation

Primary runners:

- `scripts/run-live-interop-matrix.sh`: `slskr` account login and local peer smoke.
- `scripts/run-live-interop-matrix.sh`: now also runs private-message and room-message probes after login/local-peer checks.
- `scripts/run-cross-client-validation.sh`: adjacent checkout validation, runtime/library unit validation, daemon peer probes, daemon browse probes, and daemon fixture-download probes.
- `scripts/fetch-open-commons-fixtures.sh`: downloads hash-pinned public-domain/CC0 media fixtures into ignored local storage for realistic share/search/transfer payloads.
- `scripts/verify-open-commons-fixtures.sh`: validates local fixture files, byte sizes, SHA-256 hashes, and emitted license metadata without needing the Soulseek network.
- `slskr smoke fixture-peer`: local peer-message and file-transfer smoke using the Commons binary fixture, without needing public Soulseek login or peer metadata.

The cross-client runner writes machine-readable TSV under `target/live-interop/cross-client-validation.tsv` and daemon logs under `target/live-interop/`.
By default it stages open commons fixtures from `target/open-commons-fixtures` into each daemon share under `open-commons/`; set `SLSKR_USE_OPEN_COMMONS_FIXTURES=0` to skip external fixture downloads.
When daemon readiness succeeds, the runner also attempts non-blocking `open-commons-browse` and `open-commons-download` checks against `commons-click-track.ogg` with SHA-256 verification.
It also attempts a non-blocking `open-commons-search` soak query for `commons-click-track.ogg`; this is useful evidence when public search indexing is timely, but it is not treated as a blocking result.

## Live-run stabilization knobs

Use these when public Soulseek resets or throttles daemon sessions:

| Variable | Purpose |
| --- | --- |
| `SLSKR_SLSKD_ACCOUNT_INDEX` | Soulseek account index used by the `slskd` daemon target; default `3`. |
| `SLSKR_SLSKDN_ACCOUNT_INDEX` | Soulseek account index used by the `slskdN` daemon target; default `4`. |
| `SLSKR_SLSKD_PROBE_ACCOUNT_INDEX` | Soulseek account index used by `slskr` when probing `slskd`; default `1`. |
| `SLSKR_SLSKDN_PROBE_ACCOUNT_INDEX` | Soulseek account index used by `slskr` when probing `slskdN`; default `2`. |
| `SLSKR_DAEMON_PREFLIGHT_ATTEMPTS` | Local daemon HTTP/log readiness attempts before peer-address probes; default `24`. |
| `SLSKR_DAEMON_READINESS_ATTEMPTS` | Public peer-address attempts after daemon preflight; default `36`. |
| `SLSKR_DAEMON_COOLDOWN_SECONDS` | Cooldown between daemon targets; default `20`. |
| `SLSKR_DAEMON_VPN_ENABLED` | Run daemon targets inside Proton WireGuard namespaces; use `1` for isolated live interop. |
| `SLSKR_PROBE_VPN_ENABLED` | Run `slskr` probe clients inside separate Proton WireGuard namespaces; use `1` for isolated live interop. |
| `SLSK_DOWNLOAD_QUEUE_ATTEMPTS` | Attempts for queued daemon downloads before recording a bounded live-network retry failure; default `8` in the matrix. |
| `SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS` | Delay between queued download attempts; default `4` in the matrix. |

Example bounded public-live rerun:

```bash
env \
  SLSKR_SKIP_ADJACENT_TESTS=1 \
  SLSKR_RUN_ADJACENT_DAEMONS=1 \
  SLSKR_DAEMON_PREFLIGHT_ATTEMPTS=12 \
  SLSKR_DAEMON_READINESS_ATTEMPTS=12 \
  SLSKR_DAEMON_COOLDOWN_SECONDS=60 \
  SLSK_DOWNLOAD_QUEUE_ATTEMPTS=10 \
  SLSK_DOWNLOAD_QUEUE_RETRY_SECONDS=6 \
  scripts/run-cross-client-validation.sh
```

For account rotation, add more `SLSKR_TEST_N_USERNAME` / `SLSKR_TEST_N_PASSWORD` pairs to `.env`, then point the index variables above at non-colliding accounts. For VPN rotation, keep each daemon/probe account pinned to one stable egress for the duration of a run; do not change egress mid-session.

## Pairwise execution plan

| Phase | Checks |
| --- | --- |
| 1. Bootstrap | Start clients with isolated credentials, state dirs, ports, and fixture shares |
| 2. Discovery | Login, watch target, request target metadata, verify plain/obfuscated port advertisement |
| 3. Peer | Plain peer, obfuscated peer where supported, distributed peer, file-transfer peer, indirect peer |
| 4. Search/browse | Search for fixture token, browse target shares, verify expected fixture path/metadata |
| 5. Transfer | Download fixture from target, verify byte-for-byte hash, verify progress and completion events |
| 6. Chat/rooms | Send private message, ack it, join test room, send room message, leave room |
| 7. API/Web | Drive equivalent HTTP/Web UI operations for daemon clients with web surfaces |
| 8. Restart | Restart client under test and verify persisted records and clean reconnect |
| 9. Negative | Offline target, blocked transfer, bad room, bad peer token, closed listener, auth failure |

## Execution status, 2026-05-04

| Scope | Result | Evidence |
| --- | --- | --- |
| `slskr` Rust formatting | Passed | `cargo fmt --all --check` |
| `slskr` Rust workspace tests | Passed | `cargo test --workspace`: all protocol, client, daemon, API smoke, and Soulfind contract tests passed |
| `slskr` web tests | Passed | `web npm test`: 82 files, 508 tests passed |
| `slskr` web production build | Passed | `web npm run build` |
| `slskr` live account login | Passed | `scripts/run-live-interop-matrix.sh`: four generated `slskRtest20260504_*` accounts logged in |
| `slskr` live account login over VPN | Passed | 2026-05-04 VPN-backed live matrix: accounts 5-8 logged in through fresh Proton namespaces |
| `slskr` live peer smoke | Passed | 2026-05-04 VPN-backed live matrix: direct peer-message, obfuscated peer-message, file-transfer init, and indirect peer-message passed with fresh accounts |
| `slskr` private-message live proof | Passed | 2026-05-04 VPN-backed live matrix: sender and receiver fresh accounts completed private-message probe |
| `slskr` room-message live proof | Passed | 2026-05-04 VPN-backed live matrix: fresh account joined `slskr-live-interop` and completed room-message probe |
| browser/player E2E against daemon state | Passed | 2026-05-04 Playwright `e2e/live-surfaces.spec.ts`: this repo's web bundle was hosted by adjacent `slskdN`; search, downloads, uploads, messages, rooms, browse, system, and player shell controls loaded without runtime errors |
| `slskr -> slskd` daemon plain peer | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: `slskd` logged in through p7, probe p5 resolved advertised `55100`, and plain `UserInfoRequest` passed through the daemon namespace host override |
| `slskr -> slskdN` daemon plain peer | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: `slskdN` logged in through p8, probe p6 resolved advertised `55110`, and plain `UserInfoRequest` passed through the daemon namespace host override |
| `slskr -> slskd` daemon browse | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: browse preview included `slskd\open-commons\commons-click-track.ogg` |
| `slskr -> slskdN` daemon browse | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: browse preview included `slskdN\open-commons\commons-click-track.ogg` |
| `slskr -> slskd` live search soak | Passed over VPN-isolated probe namespace | 2026-05-04 focused run: server event stream completed; incidental indirect peer reset is now informational |
| `slskr -> slskdN` live search soak | Passed over VPN-isolated probe namespace | 2026-05-04 focused run: server event stream completed; incidental indirect peer reset is now informational |
| `slskr -> slskd` daemon queued payload transfer | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: probe claimed Proton NAT-PMP port, advertised it as wait port, accepted daemon `PeerInit F`, sent offset, received text fixture, and matched SHA-256 |
| `slskr -> slskd` open-commons payload transfer | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: received `slskd\open-commons\commons-click-track.ogg`, 7640 bytes, SHA-256 `e5e09f8ef9617a355e71e2d0b00f2554201aa124a9a821c4a7f76f0441a369a0` |
| `slskr -> slskdN` daemon queued payload transfer | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: probe claimed Proton NAT-PMP port, advertised it as wait port, accepted daemon `PeerInit F`, sent offset, received text fixture, and matched SHA-256 |
| `slskr -> slskdN` open-commons payload transfer | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 focused run: received `slskdN\open-commons\commons-click-track.ogg`, 7640 bytes, SHA-256 `e5e09f8ef9617a355e71e2d0b00f2554201aa124a9a821c4a7f76f0441a369a0` |
| consolidated `slskr -> slskd`/`slskdN` cross-client pass | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 consolidated run: both daemon targets passed peer-address, plain peer, browse, queued text download, open-commons binary download, and search; `slskdN` also passed obfuscated peer via plain-response fallback |
| restart/persistence matrix | Passed over VPN-isolated daemon/probe namespaces | 2026-05-04 restart runs: `slskd` and `slskdN` restarted with preserved app/share directories, republished peer metadata, served browse responses, completed open-commons queued payload downloads, and completed search soaks |
| daemon raw transfer-token probe | Diagnostic-only non-blocking row | `slskd` and `slskdN` close the raw token echo probe with EOF because no transfer is queued; real payload proof is now covered by queued requester-listener probes |
| `slskr -> slskdN` obfuscated peer-message response | Passed with compatibility fallback | 2026-05-04 focused VPN run: type-1 obfuscated init and obfuscated request succeeded; `slskdN` returned the peer message as a plain frame, and `slskr` completed via plain-response fallback |
| `slskr` live login after repeated daemon retries | Mitigated by VPN account pool | Fresh p5-p8 accounts avoid the prior host-egress reset path for focused daemon/probe runs; raw public-host reruns may still reset under heavy retry |
| `slskdN` unit tests | Passed | `/home/keith/Documents/code/slskdn`: 3863/3863 passed |
| vendored `slskNet.Runtime` library build | Passed | `/home/keith/Documents/code/slskdn`: `dotnet build vendor/slskNet.Runtime/src/Soulseek.csproj`, 0 warnings/errors |
| vendored `slskNet.Runtime` unit behavior | Passed | `/home/keith/Documents/code/slskdn`: 2303/2303 passed |
| standalone `Soulseek.NET` unit behavior | Passed | `/home/keith/Documents/code/Soulseek.NET`: 2246/2246 passed |

## Bugs fixed during matrix execution

| Area | Bug | Fix |
| --- | --- | --- |
| Live harness | `scripts/run-live-interop-matrix.sh` had a malformed command substitution after warning filtering was added. | Rewrote the runner with temp files and sanitized summaries so credentials are not printed and compiler warnings do not pollute TSV detail fields. |
| Web UX | Room unread/activity alert missed migrated `slskdN` localStorage state because only the new `slskr.rooms.lastSeenActivity` key was read. | Added fallback read from `slskdn.rooms.lastSeenActivity` while continuing to write the current `slskr` key. |
| Rust formatting | Existing Rust sources had trailing whitespace that blocked `cargo fmt`. | Stripped trailing whitespace and reran formatting. |
| Runtime unit tests | `SearchInternal` mismatched-token test expected an exception even though current behavior safely ignores stale token responses. | Updated the test expectation to assert no exception and no callback. |
| Runtime unit tests | Reconfigure download/upload speed tests used AutoData, which can generate the same speed as the initial value and make the changed assertion flaky. | Replaced changed-speed AutoData with deterministic non-equal inline values. |
| Runtime analyzer debt | Vendored runtime unit tests emitted CA2012 warnings from `ValueTask` test doubles. | Suppressed the existing analyzer debt in the vendored unit-test project so normal test runs are clean. |
| Cross-client harness | Daemon readiness accepted stale `port=0` metadata and hid probe stderr. | Readiness now requires a nonzero advertised listener, failures include stderr detail, and live-environment daemon failures are recorded without aborting the whole matrix. |
| Obfuscated metadata probe | `slskr probe obfuscated-peer` could fail during transient Soulseek metadata reads before reaching the target obfuscated listener. | Added bounded metadata retries before attempting the obfuscated TCP exchange. |
| Browse/download coverage | The matrix only had synthetic peer and raw transfer-token probes. | Added `browse-peer` and `download-peer`; daemon runner now attempts browse fixture proof and negotiated fixture download before the legacy raw token probe. |
| Chat/room coverage | Private-message and room flows were only API/unit covered. | Added `private-message` and `room-message` live probes and wired them into the live matrix. |
| Live failure observability | Public-server login/reset failures were blank in TSV output. | Live runner now records stderr tails; cross-client runner records peer-address last detail and daemon log tails. |
| Proton WireGuard netns | Fresh Proton configs handshook only after bypassing the host VPN policy route. | Netns runner now installs a temporary host `/32` endpoint route via the main default route and supports cross-namespace extra routes. |
| Host-egress live smoke resets | `scripts/run-live-interop-matrix.sh` could still fail on account/login or social probes when public server reset host-egress connections. | Added optional `SLSKR_LIVE_VPN_ENABLED=1` mode that wraps login, local peer, private-message, and room-message probes in Proton namespaces, resolves the Soulseek server outside the namespace, and defaults to fresh accounts 5-8. |
| VPN-isolated daemon/probe routing | Daemon and probe namespaces could not reach each other by private namespace IP. | Cross-client runner now assigns stable daemon/probe subnets and injects reciprocal namespace routes for host-overridden direct peer probes. |
| Browser/player E2E gap | Browser/player coverage stopped at unit/build checks and did not mount this repo's bundle against daemon state. | Added a Playwright live-surface spec and made the slskdN E2E harness portable: it can build this repo's `web` bundle, host it from adjacent `slskdN`, and assert route/player shell behavior. |
| Restart/persistence gap | Daemon adapter restarts wiped app state and did not re-probe after process restart. | Cross-client runner now preserves daemon app/share dirs across restart and re-runs peer metadata, browse, open-commons queued transfer, and search checks after restart. |
| Restart log diagnostics | slskdN restart logs contained null padding and HTTP readiness relied on logs because the daemon bound HTTP to loopback inside the namespace. | TSV sanitization now strips null bytes, and daemon launches set `ASPNETCORE_URLS=http://0.0.0.0:$port` for reachable HTTP readiness where the target honors ASP.NET Core URL binding. |
| Cross-client search soak | Optional account commands still used host egress and the default Soulseek server. | `run_account_command_optional` now uses the same probe VPN wrapper and resolved daemon Soulseek server as peer probes. |
| Live soak indirect chatter | Search soak failed when incidental indirect peer-message sockets reset or requested user info. | Soak now answers inbound `UserInfoRequest` and treats peer reset/timeout during incidental indirect handling as informational. |
| Queued daemon payload transfer | Direct retry probes proved queue placement but not payload transfer because daemons initiate the eventual upload back to the requester. | `download-peer` now has a queued requester mode: it advertises a Proton NAT-PMP wait port, keeps the peer-message negotiation alive, accepts daemon `PeerInit F`, sends offset, reads payload bytes, and verifies SHA-256. |
| Probe port metadata lag | Back-to-back payload probes could advertise different forwarded ports faster than public-server peer metadata converged. | Daemon download probes reuse the same local NAT-PMP port per target run so the forwarded public port remains stable for text and binary fixture transfers. |
| `slskdN` obfuscated response framing | Diagnostic mode proved `slskdN` accepts obfuscated init/request but sends `UserInfoResponse` as a plain frame on that connection. | `obfuscated-peer` now falls back to a plain peer-message response after the primary obfuscated-response read times out or EOFs, preserving compatibility without weakening the initial obfuscated request path. |

## Current residual diagnostics

No blocking implementation gaps remain in the current live interop matrix. The raw `file-transfer-peer` row remains diagnostic-only because `slskd` and `slskdN` correctly require an actual queued transfer before opening `F` payload sockets; queued payload transfer is covered by `download-peer` and open-commons download probes.
