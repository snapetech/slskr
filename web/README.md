# slskr Web UI

This is the currently shipped browser UI for `slskr`. It is a React/Vite app
that talks to the daemon HTTP API on the same origin when served by
`slskr serve`.

Release archives include the built assets from this directory. The Rust/WASM UI
under `crates/slskr-web/` is a migration target, not the browser bundle users
receive today.

## Run Locally

Install dependencies:

```bash
npm --prefix web ci
```

Run the dev server:

```bash
npm --prefix web run start -- --host 127.0.0.1
```

The default Vite port is `3001`. For real daemon data, run `slskr serve` on
`127.0.0.1:5030` and configure the dev proxy or same-origin environment the way
your local workflow expects.

## Build And Test

```bash
npm --prefix web test
npm --prefix web run build
```

The release archive script builds this app automatically unless
`SLSKR_SKIP_WEB_BUILD=1` is set. The generated static files are served by the
daemon binary with API routes under `/api` and `/api/v0`.

## Headless Audit

The UI audit script builds the app, serves it locally, drives the main routes
with mocked daemon responses, and fails on broken links, console errors, empty
screens, or missing expected route surfaces:

```bash
node web/scripts/audit-react-webui.mjs
```

Use it after route, navigation, API-client, or visual shell changes. The audit
expects a human-usable app shape: recognizable navigation, populated primary
views, working actions, stable player chrome, and slskd/slskdN-style operating
surfaces for users migrating from that family of Web UIs.

## Screenshots

README screenshots are generated from mocked daemon responses so credentials and
local file paths are not captured:

```bash
cd web
SLSKR_SCREENSHOT_BASE_URL=http://127.0.0.1:3001 \
  node scripts/capture-readme-screenshots.mjs
```

The output files live in [../docs/screenshots](../docs/screenshots) and are
embedded by the root [README](../README.md).

## Main Surfaces

The app should keep these surfaces visible and navigable:

- Search, search history, acquisition controls, discovery graph, wishlist, and
  metadata-assisted follow-up searches.
- Downloads, uploads, transfer queue actions, progress state, retries, cancel
  actions, and player controls.
- Rooms, private messages, watched users, contacts, notes, browse, shares,
  collections, share groups, and shared-with-me workflows.
- Integrations, library/source panels, visualizer state, Spotify callback
  guidance, Lidarr status, and unavailable/disabled states when config is
  missing.
- System status, network/listener state, telemetry, metrics, logs, options,
  security posture, runtime compatibility, and API token handling.

## API Expectations

- The production app is same-origin with the daemon.
- Protected routes use the bearer token configured in the browser session or the
  same token sent by automation as `Authorization: Bearer <token>` /
  `X-API-Key: <token>`.
- Browser-origin mutating requests are subject to daemon CSRF checks when auth
  is enabled.
- Event views consume the bounded `/api/v0/events` store and the
  `/api/events/ws` WebSocket stream.

## Related Docs

- [Project README](../README.md)
- [Documentation index](../docs/README.md)
- [App surface](../docs/app-surface.md)
- [HTTP API](../docs/http-api.md)
- [HTTP API deployment](../docs/http-api-deployment.md)
- [Client libraries](../docs/CLIENT_LIBRARIES.md)
