# Rust Web UI Migration

`slskr-web` is the Rust/WASM migration target for the browser UI.

The production UI is still the React app in `web`. The migration should move one
route at a time while preserving the daemon API contract, screenshots, and
operator workflows.

## Current Slice

- Workspace crate: `crates/slskr-web`
- Browser shell: `crates/slskr-web/static/index.html`
- Styles: `crates/slskr-web/static/styles.css`
- Build script: `scripts/build-rust-web.sh`
- Output directory: `target/slskr-web`

The first Rust shell covers navigation metadata and the major API-backed
surfaces: search, transfers, messages, rooms, browse, and system status.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
scripts/build-rust-web.sh
```

## Migration Rules

- Keep the React UI as production until a Rust route reaches parity.
- Port API contracts before visual polish.
- Use `/api/v0` endpoints from Rust code.
- Keep protocol version fields separate from app/build version fields.
- Capture screenshots for every promoted route.
- Do not remove a React route until the Rust route has matching tests and
  browser verification.

## Route Order

1. System: version, telemetry, metrics, config, auth state.
2. Search: list, create, detail, result filtering.
3. Transfers: downloads, uploads, progress, retry/cancel/remove.
4. Messages and rooms.
5. Browse and users.
6. Collections, contacts, share groups, playlist intake, player, and
   integrations.
