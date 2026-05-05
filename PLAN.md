# slskr — Plan

Independent independent Rust client/server app for the Soulseek network. Compatibility work uses public protocol documentation, interoperability tests, and observed network behavior; do not copy official client code or third-party client code unless its license is reviewed and attribution is added first. Wire protocol notes are maintained at [nicotine-plus.org/doc/SLSKPROTOCOL.html](https://nicotine-plus.org/doc/SLSKPROTOCOL.html).

Public product posture: ship one runnable app named `slskr`, with daemon/API/web UI bundled. The workspace crates are internal implementation boundaries, not separate user-facing product names except where publishing a library is intentional.

## Current Reality

The protocol and client runtime crates are the reliable core of the project.
They implement real Soulseek wire codecs, session/listener/peer runtime,
search, transfer, distributed search, obfuscation type 1, and live probe
tooling.

The `slskr` product binary now exists and runs a single-node daemon with a
manual HTTP/1.1 API server, bundled local dashboard, bearer-token/browser-cookie
auth, CSRF origin checks for protected mutating routes, share/search/browse/
transfer/message/room/user projections, `/api/events` polling, and
`/api/events/ws` WebSocket events. The web UI is partially wired to the REST API
and plain WebSocket event feed. SignalR, GraphQL, SSE, clustering, sharding,
gRPC, Redis/Postgres cache layers, and HTTP/2 performance claims are descoped.

Persistence is intentionally default-off. One proof path is real: search create
can write to SQLite and startup can hydrate `/api/searches` when
`SLSKR_PERSISTENCE_ENABLED` or `[persistence].enabled` is true. Transfer,
message, room, and user persistence remain incomplete and must be wired before
the default can flip on.

Remaining product work is narrower than the historical phase list below:
single-node daemon hardening, removal or replacement of compatibility stubs,
durable storage for the remaining app resources, API/web parity for the
documented endpoint surface, and public-posture cleanup before any release.

## Scope

In-scope (normal Soulseek client behavior):

- **Server protocol**: login, keepalive, room ops, user watch, search dispatch, peer-address lookup, privileges, parent/branch info upkeep, excluded-search-phrase list.
- **Peer protocol**: `GetShareFileList`, `SharedFileListResponse`, `FileSearchResponse`, `UserInfo*`, `TransferRequest/Response`, `QueueUpload`, `PlaceInQueue`, `FolderContents`.
- **Distributed search**: parent selection, branch-level/branch-root upkeep, child acceptance, search forwarding, embedded-server-message wrapping (code 93).
- **Init messages**: `PeerInit` and `PierceFirewall`.
- **File transfers**: `F`-typed peer connections; token + offset handshake; resume.
- **Listener**: inbound TCP on regular and obfuscated ports.
- **Obfuscation metadata + type 1 transport**: parse/report obfuscation type and obfuscated port fields; support rotated type-1 obfuscated init and peer-message (`P`) streams.
- **Indirect connect / firewall piercing**: race direct dial against server-mediated `ConnectToPeer`.
- **Share-list zlib**: Adler32 + inflate; stream-decompress browse/share payloads.

Out-of-scope:

- Distributed clustering, sharding, mesh routing, GraphQL, gRPC, Redis/Postgres
  cache layers, and HTTP/2 performance theater.
- TLS for Soulseek wire traffic. The network protocol is plaintext; protect the
  local HTTP API by binding locally or placing a deliberate reverse proxy in
  front of it.
- Presenting the repository/package as an official client or successor
  distribution.

## Architecture

Cargo workspace with one planned product binary plus internal protocol/runtime crates:

```
crates/
  slskr-protocol/   wire codec, typed messages, framing
                       no I/O, no async — pure (de)serialization
  slskr-client/     slskr-client API; tokio runtime;
                       connection managers; search, transfer,
                       distributed-tree orchestration
  slskr-cli/        internal smoke/admin/probe command runner
  slskr/            product binary: subcommands now, daemon/API/web UI next
```

The protocol/client split is the key invariant: `slskr-protocol` stays I/O-free so it can be unit-tested against captured byte fixtures, and so it is reusable by tooling (packet sniffers, fixture generators) without dragging in `tokio`. `slskr-client` is the embeddable runtime engine. The `slskr` app consumes that engine and is the only intended user-facing binary; its current subcommand surface wraps the smoke/probe runner, and the daemon/API/web UI will be built into the same package.

External Rust deps (planned, will land per-phase):

- `tokio` — async runtime, TCP.
- `tokio-util` — `LengthDelimitedCodec` for server/peer/distributed framing.
- `bytes`, `byteorder` — byte-level I/O.
- `flate2` — zlib (share-list payloads).
- `tracing` — replaces .NET `Diagnostics` events.
- `dashmap` — connection registries keyed by username/token.
- `encoding_rs` — Latin-1 fallback decoding.
- `thiserror` — typed error enums.

No crypto. No TLS.

## Wire-protocol inventory

| Category    | Codes      | Framing                                            |
|-------------|------------|----------------------------------------------------|
| Server      | ~102       | `[u32 len][u32 code][payload]` little-endian       |
| Peer        | ~18        | `[u32 len][u32 code][payload]` little-endian       |
| Distributed | ~6 (+ 93)  | `[u32 len][u32 code][payload]` little-endian       |
| Peer-init   | 2          | `[u32 len][u8  code][payload]` little-endian       |
| File        | (none)     | `[u32 token]` then optional `[u64 offset]` + bytes |

All integers LE. Strings are length-prefixed UTF-8 with Latin-1 fallback.

After init, message connections are tagged by a single character: `P` peer-messages, `F` file-transfer, `D` distributed.

## Phased plan

### Phase 0 — Repo + scaffolding *(done)*
- Git repo with dual-push remote (GitHub primary, gitlab.home mirror).
- Workspace skeleton.
- Empty `slskr-protocol` and `slskr-client` crates.
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.

### Phase 1 — Protocol primitives *(done)*
- Reader/writer for `u8`, `u32 LE`, `u64 LE`, length-prefixed UTF-8/Latin-1 strings, IPv4, bool. *(done)*
- Three frame codecs: `u32`-len + `u32`-code (server/peer/distributed); `u32`-len + `u8`-code (peer-init); raw (`F`). *(done)*
- Property-based round-trip tests. *(done)*

### Phase 2 — Server messages *(initial cut done)*
- Decoders + encoders for all ~102 codes. *(done: complete 102-code inventory, typed core login, peer-address, watch/status, peer-connect, private-message, search, distributed, privilege, and filter messages; unsupported/obsolete payloads are raw-preserved instead of guessed)*
- Permissive parsing for obsolete/unknown codes (log + skip, never crash). *(done for unmapped and unsupported server codes)*
- Round-trip tests against captured fixtures. *(done for protocol-spec login fixture, complete code inventory, unknown preservation, and typed core round trips)*

### Phase 3 — Peer + distributed + init messages *(initial cut done)*
- Same drill for ~18 peer codes, ~6 distributed codes, 2 init codes. *(done: complete known code tables, typed core peer/transfer/distributed/init messages, plus named raw-preserving variants for obsolete/undocumented known peer codes)*
- Code 93 `EmbeddedMessage` wrapping a server frame inside a distributed frame. *(done for raw distributed payloads and server-frame wrapper)*

### Phase 4 — Connection layer (`slskr-client`) *(initial cut done)*
- Server connection: dial, login, keepalive, server-message dispatch. *(done: TCP dial, typed server stream wrapper, login handshake, wait-port send, ping send, and public send/receive helpers)*
- Listener: accept inbound; demux on first byte (init code) or connection-type byte (`P`/`F`/`D`). *(done: bind/accept wrapper, connection-type tags, async frame I/O, and inbound demux for tagged sockets plus `PeerInit`/`PierceFirewall`; `F` sockets wrap typed file-transfer I/O)*
- Outbound peer connect: dial → send `PeerInit` → race with indirect path. *(done: direct `PeerInit` helpers for `P`/`F`/`D`, with typed peer/distributed/file-transfer stream wrappers)*
- Indirect connect: receive `ConnectToPeer` from server → inbound peer dials us → expect `PierceFirewall` with token. *(done: `ConnectToPeer` request builder, token generator, server request issuance, plus `PeerInit`/`PierceFirewall`/tagged inbound completion validation for `P`/`F`/`D`)*
- Per-peer `P`-connection cache (one per username). *(done: async cache with insert/replace/remove, typed send/receive helpers, and manager-level direct acquisition/reuse)*
- Obfuscation type 1 transport. *(done initial cut: pure rotated transform with public vector, obfuscated init I/O, obfuscated peer-message `P` I/O, and `SetWaitPort` metadata advertisement; still needs live interop against a real client on a reachable obfuscated port)*

### Phase 5 — Search + transfers *(initial cut done)*
- Outgoing search dispatch + responder against a pluggable share index. *(done: global/user/room/wishlist dispatch with generated tokens, `ShareIndex` trait, in-memory index, and server/distributed search responder)*
- Search-response aggregation under per-search timeout windows. *(done: token-keyed `FileSearchResponse` collector plus timed active windows, explicit finish, and expiry drain)*
- Transfer state machines: queue → place-in-queue → request → upload/download → complete; with offset/resume. *(done: download and upload state models, peer-message transitions, and `F` token/offset/chunk receive/send paths)*

### Phase 6 — Distributed search tree *(initial cut done)*
- Parent selection from `PossibleParents`. *(done: stable candidate selection ignores self and unusable ports)*
- `BranchLevel` / `BranchRoot` propagation upward. *(done: local branch state tracks parent updates and sends branch level/root/depth to parent connections)*
- Accept child connections; forward `DistributedSearch` requests downward. *(done: child registry, child-depth tracking, and fan-out with source exclusion)*
- Periodic branch-info to server. *(done: branch report messages plus interval scheduler and server-send helper)*

### Phase 7 — Misc parity *(initial cut done)*
- Stream-decompress `SharedFileListResponse` / `BrowseResponse` (zlib). *(done: reusable zlib compress/decompress helpers for shared-file-list and folder-content peer payloads)*
- Apply server-provided excluded-phrase list to outgoing search results. *(done: `ExcludedPhraseFilter` plus `SearchResponder` suppression)*
- User watch, room list, private messages, server stats. *(started: user watch/status state, global-room message state, PM inbox with ack generation; stats are preserved through existing protocol user-watch/user-stats messages)*
- Coverage of `Diagnostics` events as `tracing` spans/events. *(started: explicit tracing hooks for server, peer, and distributed messages)*

### Research/backfill — Obfuscated peer transport *(initial cut done)*
- Public algorithm source: aioslsk documents rotated type-1 obfuscation and the vector now covered by `slskr-protocol`.
- Soulfind metadata relay. *(done: contract test proves `SetWaitPort` obfuscation fields survive `GetPeerAddress`)*
- Local obfuscated `P` stream. *(done: CLI smoke and async stream tests cover obfuscated `PeerInit` plus obfuscated peer messages; listener tests also reject plain/obfuscated demux mismatches so type-1 handling is not silently falling through to the wrong path)*
- Live interop. *(pending: verify against a real client on a reachable obfuscated port; Soulfind is a server simulator and cannot prove peer byte-level compatibility by itself)*

### Phase 8 — Hardening + release *(started)*
- Product shape. *(done: external product is one runnable `slskr` app; internal crates remain `slskr-protocol`, `slskr-client`, and an internal `slskr-cli` command-runner crate during migration)*
- Pick + register a unique client-version band. *(started: local reserved band `8_800_000..=8_809_999`, default client version `175.8_800_001`; external registration still pending)*
- Live connect to the real Soulseek server with that band. *(done: `slskr login smoke` succeeds with existing local credentials; command accepts `SLSK_USERNAME`/`SLSK_PASSWORD`)*
- Controlled live-soak harness. *(done: `slskr soak live` binds a listener, advertises wait port, sends low-volume maintenance messages, optionally advertises a type-1 obfuscated listener, probes one peer or sends one explicit search, and redacts host/user details in output; bounded 10s live soak passed; `scripts/run-live-soak-24h.sh` starts the 24h obfuscated-listener soak)*
- Local two-account peer smoke. *(done initial cut: `slskr smoke local-peer` logs two accounts into the server, exercises direct loopback peer-message, obfuscated peer-message, and file-transfer handshakes, and validates server-mediated `ConnectToPeer`/`PierceFirewall` with `SLSKR_INDIRECT_HOST_OVERRIDE=127.0.0.1`; real remote listener reachability still depends on network/NAT configuration)*
- Public peer probe matrix. *(started: `slskr probe peer-address`, `probe plain-peer`, `probe obfuscated-peer`, `probe distributed-peer`, `probe file-transfer-peer`, `probe indirect-peer`, `probe metadata-relogin`, and `probe negative-indirect` cover server metadata, direct `P`, obfuscated direct `P`, direct `D`, direct `F`, server-mediated `ConnectToPeer`/`PierceFirewall`, metadata stability across relogin, and explicit negative indirect behavior; `scripts/run-proton-public-matrix.sh` rotates NAT-PMP-capable Proton listener endpoints and probes them from the other endpoint configs through isolated Linux network namespaces, with results written to `target/live-soak/*.tsv`)*
- Public Proton reachability. *(done: deep matrix uncovered that Soulseek IPv4 fields are reversed on the wire; after fixing `read_ipv4`/`write_ipv4`, public Proton NAT-PMP testing passes plain direct, obfuscated direct, and indirect `ConnectToPeer`/`PierceFirewall` for the US-CA listener / IL probe pair. The obfuscated-only advertisement shape also works: plain direct fails when regular `2234` is intentionally advertised, while obfuscated direct and indirect pass.)*
- Public D/F and negative behavior. *(started: focused US-CA listener / IL probe passes direct distributed `D`, direct file-transfer `F`, metadata stability across relogin, and normal indirect. Explicit negative indirect does not reliably return `CantConnectToPeer`; keep local timeout handling, matching warnings in other client implementations.)*
- Obfuscated real-peer probe. *(done harness: `slskr probe obfuscated-peer` logs into the real server, resolves a target peer, requires obfuscation type `1`, connects to the advertised obfuscated port, sends obfuscated `PeerInit`, and round-trips `UserInfoRequest`; passed against the live slskr soak with same-host TCP override after public-server metadata resolution and against a separate Proton endpoint after the reversed-IP fix; third-party real-client interop still needs a reachable type-1 peer from another implementation)*
- Soulfind-backed local server contract tests. *(done initial cut: `slskr-client` has optional `soulfind_contract` integration test using `SOULFIND_PATH` or opt-in `SLSK_SOULFIND_DOCKER=1`; current coverage is login, keepalive, wait-port, obfuscated-port metadata relay, local multi-client direct `PeerInit`, local multi-client indirect `ConnectToPeer`/`PierceFirewall`, local multi-client obfuscated `P` connection through server-returned obfuscated port, typed room-list observation, room join/leave commands, search dispatch, and reconnect after server restart; additional black-box client testing remains useful later, but type-1 obfuscation coverage stays slskr-to-slskr until another client advertises type `1`)*
- Terms/rules compliance review. *(done initial pass: see `COMPLIANCE.md`; public release remains gated on full expected client feature surface and non-abusive network behavior)*
- Public provenance/branding scrub. *(final pre-public task: audit README, plan, comments, commit-visible docs, package metadata, examples, UI assets, screenshots, generated fixtures, and release text; remove casual "inspiration/root/reference implementation" language and avoid names or descriptions that imply an official variant or a replacement distribution for another project; keep only accurate independent-implementation language and legally required license attribution for any copied material)*
- Bundled app crate. *(started: `crates/slskr` exists, exposes the command surface, runs `slskr serve`, creates state dir, exposes health/version/config/session/listener/share/transfer APIs, and owns a background session manager with connect/disconnect/ping commands, receive-loop status, keepalive pings, optional reconnect, explicit regular/obfuscated listener ownership, listener-owned `UserInfoRequest` responses, startup/rescan share indexing with a state-dir cache and private local-path map, browse payloads, direct peer browse execution from peer-address responses, file-search responses, transfer request projection with local-path metadata execution, peer-message `TransferRequest` negotiation, direct plain/obfuscated file-transfer `F` streaming/resume, requester-side indirect `F` fallback via `ConnectToPeer`/`PierceFirewall`, inbound shared-file serving over direct or pierced `F` sockets, and explicit transfer rejection; full web UI hardening still pending)*
- Web/API surface. *(started: one binary serves API and a bundled local dashboard with stats cards, user/share-catalog/search/transfer/message/room/browse tables, table filters, session controls and privilege checks, search start/complete, user-watch/unwatch/stats, browse-request, share-rescan, transfer queue/lifecycle with explicit progress bytes, local-path execution, max-active transfer admission control, inbound/outbound transfer allow switches, peer-message negotiation, direct plain/obfuscated `F` streaming, chunked transfer progress events, reloadable `transfer-state.json` records for restart resume, requester-side indirect `F` fallback, and inbound shared-file serving over direct or pierced `F` sockets, messaging/ack, room-list sync, room join/leave/message, refresh, bearer-token and same-site browser-cookie controls, public capabilities with negotiation, projection events/stats/metrics/telemetry, and cross-site `Origin`/`Referer` rejection for protected mutating API routes; typed TOML config loading now covers app/network/listener/profile/timeout/share/transfer-history/auth settings with env overrides; current app endpoints also have `/api/v0/*` aliases, route contract tests, bearer-token/API-cookie auth for protected routes, `/api/v0/capabilities`, `/api/v0/capabilities/negotiate`, `/api/v0/events`, `/api/v0/stats`, `/api/v0/metrics`, `/api/v0/telemetry`, `/api/v0/shares/catalog`, `/api/v0/files/:root`, `/api/v0/searches` state wired to public-network global/user/room/wishlist dispatch plus expiration/pruning, list filtering/pagination, and peer-response/external result ingestion, user watch/stats projection routes wired to server watch/status/stats commands/events, browse request/cache/failure projection with list filtering/pagination, single-entry/batched flattened result ingestion, and direct peer `GetShareFileList` execution, message/room projection routes wired to server PM/ack/room-list commands/events with list filtering/pagination, and transfer projection routes for queue/progress/complete/cancel/fail/stats with list filtering/pagination; pending static asset strategy and fuller resource APIs)*
- Rust/WASM web UI track. *(future/non-blocking: evaluate a Rust frontend rewrite after the current React/Vite dashboard reaches API parity and stabilizes. Candidate stacks are Leptos, Dioxus, or Yew, with a preference for a framework that supports browser-only WASM, component tests, ergonomic routing/forms, and incremental embedding inside the existing `slskr serve` static-asset pipeline. This is not a release blocker and should not become a product rewrite until a prototype proves comparable search, transfers, player/events, settings, auth/session, and websocket behavior with an acceptable build/test loop. Keep the current web UI as the reference implementation during evaluation; do not mix two production UIs without an explicit migration plan.)*
- App surface runbook. *(done initial cut: see `docs/app-surface.md` for command layout, HTTP shell, auth defaults, config direction, persistent TOML example, packaging target, and backfill list)*
- Install/service runbook. *(done initial cut: see `docs/install.md` for build/install, config/state paths, user/system service units, container shape, and exposure rules)*
- App/API parity backlog. *(started: `docs/legacy-port-harvest.md` captures config/API/service requirements and sequencing for the bundled app)*
- CLI consolidation. *(done initial cut: probe/admin commands are exposed as `slskr` subcommands; legacy `slskr-cli` command names remain accepted during migration)*
- Public repo/package rename. *(pending: move public-facing metadata away from `slskr`; keep `slskr` as the user-visible app name and crate family)*
- 24h soak test (search + transfer + distributed-tree health). *(running in tmux as `slskr-live-soak-proton` through Proton VPN NAT-PMP; public endpoint matrix is running separately as `slskr-proton-matrix`)*
- License posture. *(done: project is AGPL-3.0-only from first public release, with stock AGPL text in `LICENSE` and concise project-specific notices in `NOTICE`; no third-party additional terms imported)*
- Tag `0.1.0`, publish to internal registry. *(blocked until 24h soak passes and public-posture scrub is reviewed)*

## Current verification

- `scripts/check-public-posture.sh`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `SLSK_SOULFIND_DOCKER=1 cargo test -p slskr-client --test soulfind_contract`
- `cargo package --workspace --allow-dirty --no-verify`
- `SLSKR_INDIRECT_HOST_OVERRIDE=127.0.0.1 cargo run -p slskr -- smoke local-peer`
- `cargo run -p slskr -- soak live` with a bounded 10 second window
- `scripts/run-proton-public-matrix.sh` for public multi-endpoint metadata/plain/obfuscated peer probing
- `SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL=0 SLSKR_MATRIX_INDIRECT_SEND_PEER_ADDRESS=1 scripts/run-proton-public-matrix.sh ...` for public regular-port reachability
- `SLSKR_MATRIX_ADVERTISE_REGULAR_LOCAL=1 SLSKR_MATRIX_INDIRECT_SEND_PEER_ADDRESS=1 scripts/run-proton-public-matrix.sh ...` for obfuscated-only reachability

## Risks

- **Obfuscation type 1** is sparsely documented — capture independently usable wire fixtures before implementing.
- **Indirect-connect race** is ordering-sensitive; expect iteration. Keep the demuxer paranoid about ordering.
- **Distributed-tree state** is small but easy to corrupt across reconnects.
- **Obsolete codes** must parse permissively; log unknowns rather than fail. Set up a counter so we can revisit anything that fires often.
- **License version-band**: the protocol expects clients to register a unique minor-version range; pick early so the wire test fixtures use it.
- **Product packaging drift**: current code is protocol/runtime/probe heavy. Before public release, add the bundled `slskr` app so users do not have to understand internal crates to run the product.

## Backfill Gaps

- Expand typed config coverage as API/auth/search/transfer modules land.
- Replace the current TSV share-index cache with the durable app database once storage is selected.
- Replace interim JSON/TSV transfer state with database-backed resume records.
- Expand the `slskr serve` session manager into richer event projection, command routing, and API-backed state.
- Broaden app browse execution with live public-network fixture coverage once reachable peer fixtures are available.
- Build full daemon, API, and web UI into the new `slskr` app crate/binary.
- Expand service/container docs into maintained release artifacts once packaging is selected.
- Remove or fully privatize the legacy `slskr-cli` binary once scripts and operator workflows no longer need it.
- Rename public repository and package metadata from `slskr` to `slskr` before publication.
- Add end-user docs for install, first login, shares, search, transfers, rooms/messages, ports, NAT-PMP/UPnP behavior, and web UI auth.
- Expand API contract tests as new daemon resources land.
- Use `docs/legacy-port-harvest.md` as the app/API backlog source.
- Re-run compliance and public-posture scrub after the app/web docs land.

## License

AGPL-3.0-only. See `LICENSE` and `NOTICE`.

## Compatibility Research

- Protocol spec: <https://nicotine-plus.org/doc/SLSKPROTOCOL.html>
