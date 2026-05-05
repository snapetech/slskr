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

The first Rust shell covers the full current route/navigation inventory and the
major API-backed surfaces: application state, session control, search, wishlist,
transfers, messages, rooms, browse, identity, collections, integrations, and
system status. The shell owns route page rendering, active nav state, History
API navigation, and browser-side Rust probes against health, version,
application, and server endpoints.

## Build

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
scripts/build-rust-web.sh
```

Serve the Rust build through the daemon with:

```bash
SLSKR_WEB_BUILD_DIR=target/slskr-web cargo run -p slskr -- serve
```

## Migration Rules

- Keep the React UI as production until a Rust route reaches parity.
- Port API contracts before visual polish.
- Use `/api/v0` endpoints from Rust code.
- Keep protocol version fields separate from app/build version fields.
- Capture screenshots for every promoted route.
- Do not remove a React route until the Rust route has matching tests and
  browser verification.
- Keep the Rust route inventory in `crates/slskr-web` aligned with
  `web/src/components/App.jsx` until the React route is removed.

## Route Order

1. System: version, telemetry, metrics, config, auth state.
2. Search: list, create, detail, result filtering.
3. Transfers: downloads, uploads, progress, retry/cancel/remove.
4. Messages and rooms.
5. Browse and users.
6. Collections, contacts, share groups, playlist intake, player, and
   integrations.
