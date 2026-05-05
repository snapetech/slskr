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
| Indirect connection | `ConnectToPeer` / `PierceFirewall` works for firewalled/NAT paths | `probe indirect-peer`, local peer smoke | `slskr` self-smoke currently proves protocol; daemon indirect remains a live gap |
| Distributed path | Distributed `D` peer init accepts ping/probe traffic | `probe distributed-peer` | Runtime/library unit coverage; daemon live probe still optional |
| File-transfer init | Transfer `F` peer init is accepted only as part of a real queued transfer | `probe file-transfer-peer` against `slskr` self-smoke | Raw token-echo probe closes with EOF against `slskd`/`slskdN`; replace with queued payload transfer proof |
| Search | Server search request emits usable result events | `soak live` with `SLSK_SOAK_SEARCH_QUERY` | Fixture search against daemon shares still to harden |
| Browse | Browse request returns shared folder/file payloads | `probe browse-peer` added | Daemon browse proof runs after target advertises a live listener |
| Download/upload bytes | Queued download opens transfer path, moves bytes, reports completion | `probe download-peer` added for negotiated file payload reads | Daemon payload proof runs after browse exposes exact fixture path and target remains connected |
| Private chat | Direct private message send/receive/ack works | `probe private-message` added to live matrix | Depends on two live accounts staying connected |
| Rooms | Join/leave/list/message flows work | `probe room-message` added to live matrix | Depends on live server accepting room join/message traffic |
| User controls | Watch/unwatch, stats, browse, ban/ignore semantics align | Partial unit coverage | Remaining live gap |
| Web/API/UI/player | Web UI drives search, transfers, chat, rooms, settings, player surfaces | Web tests/build | Browser-driven cross-client flows remain a separate E2E layer |
| Durability | Searches/transfers/messages/rooms survive restart where supported | Unit/API coverage | Daemon persistence restart matrix remains to add |
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
| `SLSK_DOWNLOAD_QUEUE_ATTEMPTS` | Attempts for queued daemon downloads before recording a payload-transfer gap; default `8` in the matrix. |
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
| `slskr` live peer smoke | Passed | direct peer-message, obfuscated peer-message, file-transfer init, and indirect peer-message passed |
| `slskr -> slskd` daemon plain peer | Passed when public server kept target connected | `scripts/run-cross-client-validation.sh`: `slskd` logged in, advertised `55100`, and returned user info over the plain peer listener |
| `slskr -> slskdN` daemon plain peer | Passed when public server kept target connected | `scripts/run-cross-client-validation.sh`: `slskdN` logged in, advertised `55110`, advertised type-1 obfuscation metadata on `55111`, and returned user info over the plain peer listener |
| daemon raw transfer-token probe | Non-blocking gap | `slskd` and `slskdN` close the raw token echo probe with EOF because no transfer is queued; this must be replaced by a real queued download/upload payload test |
| `slskr` live login after repeated daemon retries | Currently blocked by public server resets | `scripts/run-live-interop-matrix.sh`: `login failed ... I/O error: unexpected end of file` on 2026-05-04 20:00:26 America/Regina |
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

## Remaining implementation gaps

| Gap | Impact | Next implementation |
| --- | --- | --- |
| Browse daemon proof | Probe exists but latest daemon runs could not reach advertised-listener readiness because the public server reset daemon logins. | Rerun after server/account cooldown; inspect browse preview to confirm exact fixture paths. |
| Payload transfer daemon proof | Probe exists and now emits/checks SHA-256, but latest daemon runs could not reach advertised-listener readiness. | Rerun after cooldown; successful `download-peer` output is byte-for-byte evidence via SHA-256. |
| Private-message live proof | Probe exists but latest raw login failed with public-server EOF before social probes ran. | Rerun after cooldown. |
| Room live proof | Probe exists but latest raw login failed with public-server EOF before room probe ran. | Rerun after cooldown. |
| Browser/player E2E | Unit/build coverage does not prove live UX flows against daemon state. | Add Playwright flows for search, transfers, chat, rooms, settings, and player surfaces. |
| Restart/persistence matrix | Cross-client daemon restarts are not yet automated. | Extend daemon adapter to restart each target and assert persisted transfer/message/search records. |
| Public server stability | Repeated live daemon reruns can trigger immediate public-server disconnects/resets before listener metadata is advertised. | Keep daemon readiness bounded, record the failure, and rerun after cooldown or rotate accounts when public-server resets occur. |
