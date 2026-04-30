# soulseekR — Plan

Clean-room Rust reimplementation of [Soulseek.NET](https://github.com/jpdillingham/Soulseek.NET). The .NET project is the reference for *behavior*; we will not copy code. Wire protocol is documented at [nicotine-plus.org/doc/SLSKPROTOCOL.html](https://nicotine-plus.org/doc/SLSKPROTOCOL.html).

## Scope

In-scope (parity with Soulseek.NET):

- **Server protocol**: login, keepalive, room ops, user watch, search dispatch, peer-address lookup, privileges, parent/branch info upkeep, excluded-search-phrase list.
- **Peer protocol**: `GetShareFileList`, `SharedFileListResponse`, `FileSearchResponse`, `UserInfo*`, `TransferRequest/Response`, `QueueUpload`, `PlaceInQueue`, `FolderContents`.
- **Distributed search**: parent selection, branch-level/branch-root upkeep, child acceptance, search forwarding, embedded-server-message wrapping (code 93).
- **Init messages**: `PeerInit` and `PierceFirewall`.
- **File transfers**: `F`-typed peer connections; token + offset handshake; resume.
- **Listener**: inbound TCP on regular and obfuscated ports.
- **Obfuscation type 1**: key-rotation cipher around message connections.
- **Indirect connect / firewall piercing**: race direct dial against server-mediated `ConnectToPeer`.
- **Share-list zlib**: Adler32 + inflate; stream-decompress browse/share payloads.

Out-of-scope (initial cut):

- GUI; CLI beyond a smoke-test binary.
- Persistent share index / on-disk cache (consumer concern).
- TLS — Soulseek wire is plaintext; nothing to add.

## Architecture

Cargo workspace, two production crates plus an optional smoke binary:

```
crates/
  soulseek-protocol/   wire codec, typed messages, framing
                       no I/O, no async — pure (de)serialization
  soulseek-client/     SoulseekClient API; tokio runtime;
                       connection managers; search, transfer,
                       distributed-tree orchestration
  soulseek-cli/        optional thin binary for live smoke tests
```

The protocol/client split is the key invariant: `soulseek-protocol` stays I/O-free so it can be unit-tested against captured byte fixtures, and so it is reusable by tooling (packet sniffers, fixtures generators) without dragging in `tokio`.

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

### Phase 0 — Repo + scaffolding *(now)*
- Git repo with dual-push remote (GitHub primary, gitlab.home mirror).
- Workspace skeleton.
- Empty `soulseek-protocol` and `soulseek-client` crates.
- CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.

### Phase 1 — Protocol primitives
- Reader/writer for `u8`, `u32 LE`, `u64 LE`, length-prefixed UTF-8/Latin-1 strings, IPv4, bool.
- Three frame codecs: `u32`-len + `u32`-code (server/peer/distributed); `u32`-len + `u8`-code (peer-init); raw (`F`).
- Property-based round-trip tests.

### Phase 2 — Server messages
- Decoders + encoders for all ~102 codes.
- Permissive parsing for obsolete/unknown codes (log + skip, never crash).
- Round-trip tests against captured fixtures.

### Phase 3 — Peer + distributed + init messages
- Same drill for ~18 peer codes, ~6 distributed codes, 2 init codes.
- Code 93 `EmbeddedMessage` wrapping a server frame inside a distributed frame.

### Phase 4 — Connection layer (`soulseek-client`)
- Server connection: dial, login, keepalive, server-message dispatch.
- Listener: accept inbound; demux on first byte (init code) or connection-type byte (`P`/`F`/`D`).
- Outbound peer connect: dial → send `PeerInit` → race with indirect path.
- Indirect connect: receive `ConnectToPeer` from server → inbound peer dials us → expect `PierceFirewall` with token.
- Per-peer `P`-connection cache (one per username).
- Obfuscation type 1 transparently wraps any message connection on the obfuscated port.

### Phase 5 — Search + transfers
- Outgoing search dispatch + responder against a pluggable share index.
- Search-response aggregation under per-search timeout windows.
- Transfer state machines: queue → place-in-queue → request → upload/download → complete; with offset/resume.

### Phase 6 — Distributed search tree
- Parent selection from `PossibleParents`.
- `BranchLevel` / `BranchRoot` propagation upward.
- Accept child connections; forward `DistributedSearch` requests downward.
- Periodic branch-info to server.

### Phase 7 — Misc parity
- Stream-decompress `SharedFileListResponse` / `BrowseResponse` (zlib).
- Apply server-provided excluded-phrase list to outgoing search results.
- User watch, room list, private messages, server stats.
- Coverage of `Diagnostics` events as `tracing` spans/events.

### Phase 8 — Hardening + release
- Pick + register a unique client-version band (avoid slskd's 760–7699999).
- Live connect to the real Soulseek server with that band.
- 24h soak test (search + transfer + distributed-tree health).
- Tag `0.1.0`, publish to internal registry.

## Risks

- **Obfuscation type 1** is sparsely documented — capture wire fixtures from the reference client (Soulseek.NET via `slskd` is easiest) before implementing.
- **Indirect-connect race** is the gnarliest part of the .NET codebase; expect iteration. Keep the demuxer paranoid about ordering.
- **Distributed-tree state** is small but easy to corrupt across reconnects.
- **Obsolete codes** must parse permissively; log unknowns rather than fail. Set up a counter so we can revisit anything that fires often.
- **License version-band**: the protocol expects clients to register a unique minor-version range; pick early so the wire test fixtures use it.

## Reference

- Soulseek.NET source: <https://github.com/jpdillingham/Soulseek.NET>
- Protocol spec: <https://nicotine-plus.org/doc/SLSKPROTOCOL.html>
- slskd (active .NET-based daemon — useful for behavioral comparison): <https://github.com/slskd/slskd>
