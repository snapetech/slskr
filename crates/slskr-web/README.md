# slskr-web

`slskr-web` is the Rust/WASM migration target for the browser UI.

The existing React UI remains the production WebUI. This crate gives the
migration a compiled Rust surface with:

- a DOM-rendered shell
- stable route/navigation metadata
- API endpoint mapping through `/api/v0`
- native unit tests for shell and route contracts
- a `wasm_bindgen` entry point for browser loading

Build it with:

```bash
scripts/build-rust-web.sh
```

The script writes browser-loadable assets to `target/slskr-web`.
