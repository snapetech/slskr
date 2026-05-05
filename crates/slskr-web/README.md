# slskr-web

`slskr-web` is the Rust/WASM migration target for the browser UI.

The existing React UI remains the production WebUI. This crate gives the
migration a compiled Rust surface with:

- a DOM-rendered shell
- stable route/navigation metadata
- Rust-owned route page rendering, active nav state, and History API navigation
- API endpoint mapping through `/api/v0`
- browser-side Rust runtime probes for health, version, application, and server state
- route and API inventory tests that track the existing WebUI surface
- native unit tests for shell and route contracts
- a `wasm_bindgen` entry point for browser loading

Build it with:

```bash
scripts/build-rust-web.sh
```

The script writes browser-loadable assets to `target/slskr-web`.

To serve the Rust build from the daemon instead of the current React bundle:

```bash
SLSKR_WEB_BUILD_DIR=target/slskr-web cargo run -p slskr -- serve
```
