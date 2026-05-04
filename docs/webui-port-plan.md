# slskdN WebUI → slskR Port Plan

Status: design / assessment. No code changes yet.

This document assesses the slskdN (C#/.NET) repo at `../slskdn` and its React webui at
`../slskdn/src/web`, and proposes a concrete plan for porting the webui onto the
Rust `slskr serve` HTTP surface while using it as a living parity checklist.

## 1. What exists today

### slskdN (C#/.NET) – reference upstream

- `src/slskd/`: 1,114 `.cs` files. 112 `*Controller*.cs` files grouped into
  feature areas: Core, Events, Files, Integrations (MusicBrainz, Lidarr, Spotify),
  Messaging (Conversations, Rooms), Relay, Search, Shares, Telemetry, Transfers
  (+ AutoReplace/Ranking/MultiSource/Discovery), Users/Notes, Wishlist,
  Compatibility, Mesh, Native (Jobs, LibraryHealth, LibraryItems, MeshStats,
  Pods, PortForwarding, SignalSystem, WarmCache, SourceProviders),
  VirtualSoulfind (Bridge, Canonical, DisasterMode, ShadowIndex), Audio, etc.
- SignalR hubs: `/hub/application`, `/hub/logs`, `/hub/search`, `/hub/songid`,
  `/hub/listening-party`, `/hub/transfers`, plus `RelayHub`.
- API base is `/api/v0/*`; web uses `baseURL = rootUrl + '/api/v0'` and calls
  hit the hubs at `rootUrl + '/hub'`.

### slskdN webui – `src/web`

- React 18, Vite, Semantic UI React, @microsoft/signalr, axios, react-router-dom,
  react-toastify, codemirror (YAML editor), butterchurn (visualizer), qrcode,
  lz-string, yaml.
- 213 non-test source files; large app with 26 feature areas
  (`Browse/ Chat/ Collections/ Contacts/ DiscoveryInbox/ ImportStaging/
  Messaging/ Player/ PlaylistIntake/ Pods/ PortForwarding/ Rooms/ Search/
  ShareGroups/ Shares/ Solid/ System/ TrafficTicker/ Transfers/ Users/
  Wishlist`). Login form and global `AppContext`.
- `src/web/src/lib/*.js`: ~100 per-feature API wrapper modules; roughly
  **291 distinct method+path calls** across the webui (see
  `/tmp/webui_routes_with_methods.txt` captured during analysis).
- CSRF / bearer token / cookie-passthrough auth already wired in `lib/api.js`
  and `lib/token.js`. SignalR auth via `accessTokenFactory`.
- Build output is served by the .NET app from `src/slskd/wwwroot/` (with
  `assets/`, `favicon.ico`, `service-worker.js`, `manifest.json`).
- Top-level `react-router-dom` routes (from `App.jsx`): `/` (home), `/collections`,
  `/solid`, `/discovery-graph`, `/playlist-intake`, `/searches[/:id]`,
  `/wishlist`, `/browse`, `/users`, `/contacts`, `/sharegroups`, `/shared`,
  `/chat`, `/pods[/:podId[/channels/:channelId]]`, `/rooms`, `/messages`,
  `/uploads`, `/downloads`, `/system[/:tab]`.

### slskR (Rust) – current port

- Workspace with 4 crates: `slskr-protocol`, `slskr-client`, `slskr`, `slskr-cli`.
- `slskr serve` (inside `crates/slskr/src/main.rs`, ~10k lines) ships:
  - A hand-rolled HTTP handler with a single big `match (method, path)` block.
  - **47 handlers** registered at `/api/...` with a compatibility shim in
    `utils.rs` that rewrites incoming `/api/v0/*` paths down to `/api/*` (for
    everything except browse which is explicitly dual-routed).
  - Handlers cover health/version/capabilities, config, stats, telemetry,
    metrics, events, shares (+catalog/rescan), session (connect/disconnect/
    ping/privileges/check), listeners, users/watch, searches (+prune,
    search-responses), rooms (list/refresh), messages (+inbound), transfers
    (+stats, POST create), browse/browse-responses, admin (webhooks, api keys,
    database stats/cleanup/vacuum, monitoring), graphql (+schema).
  - In-memory projections backed by TSV/JSON state files; no real DB yet.
  - Bearer/cookie auth + CSRF-ish cross-site origin rejection for unsafe
    methods.
  - Bundled local dashboard at `GET /` — small operator console only.
- `dashboard/` (TypeScript + React 18 + Vite + Tailwind + recharts):
  a pure admin console. 6 pages: `Dashboard/ ApiKeys/ Webhooks/ Database/
  Monitoring/ Configuration`. Calls only `/api/health`, `/api/config`,
  `/api/metrics`, `/api/admin/*`. This is NOT the slskdN webui shape.
- No SignalR/WebSocket/SSE push surface attached to the app shell yet (there is
  a `websocket.rs` module but no hub at `/hub/*`).
- There is already a design doc (`docs/legacy-port-harvest.md`) that frames
  parity work but it is mostly `/api/v0/*` primitives and does not enumerate
  the 291 calls the real webui needs.

## 2. Gap analysis (webui calls vs slskR handlers)

Using the 291 normalized `METHOD /path` calls from `src/web/src/lib/*.js` and
the 47 routes currently in `main.rs`:

### Implemented (read/write roughly matches)

- `GET /session` (slskR has `/api/session`)
- `POST /session/connect`, `/session/ping`, `/session/disconnect`, 
  `/session/privileges/check`
- `GET /shares`, `POST /shares/rescan`
- `GET /searches`, `POST /searches`, `GET /searches/:id` (partial via prune),
  `POST /search-responses`
- `GET /users/:var/browse` (partial; slskR only has list/projection)
- `GET /rooms`, `POST /rooms/refresh`, `POST /rooms/joined/:room/join` (partial),
  `POST /rooms/joined/:room/messages` (partial — slskR uses `/api/rooms/:room/messages`
  rather than `joined/…`)
- `GET /events`, telemetry/metrics/stats
- `GET /transfers`, `GET /transfers/stats`, `POST /transfers`

### Wrong shape / path mismatch (same intent, different URL)

The webui talks to: `/rooms/joined`, `/rooms/joined/:name`,
`/rooms/joined/:name/messages`, `/rooms/joined/:name/users`, `/rooms/available`.
slskR exposes: `/api/rooms`, `POST /api/rooms/refresh`, no joined/available
split, no users subresource.

Likewise:
- `GET /conversations/:var` vs slskR `/api/messages/:username`.
- `PUT /options`, `GET /options/yaml`, `PUT /options/yaml`, 
  `GET /options/yaml/validate`, `GET /options/yaml/location`, `GET /options/debug`
  — slskR only exposes `GET /api/config` (read-only).
- `GET /session/enabled`, `POST /session`, `DELETE /server`, `PUT /server`,
  `GET /server`, `DELETE /application`, `PUT /application`, `GET /application`,
  `GET /application/version/latest` — slskR has none of these under that naming
  convention.
- `GET /users/:var/info`, `GET /users/:var/status`, `GET /users/:var/group`,
  `GET /users/:var/endpoint`, `POST /users/:var/directory` — slskR has only
  `/api/users` list + `/api/users/watch` + `/api/users/:u/stats/request`.

### Missing entirely (major webui feature areas with zero slskR coverage)

Grouping by feature → route prefix → web UI component:

- **Wishlist** — `/wishlist`, `/wishlist/:id`, `/wishlist/:id/search`,
  `/wishlist/import/csv`. UI: `components/Wishlist/*`.
- **Collections** — `/collections`, `/collections/:id`, `/collections/:id/items`,
  `/collections/items/:itemId`, `/collections/:id/items/reorder`. UI:
  `components/Collections/*`.
- **Share Groups** — `/sharegroups`, `/sharegroups/:id`,
  `/sharegroups/:id/members[/:userId]`. UI: `components/ShareGroups/*`.
- **Share Grants** — `/share-grants`, `/share-grants/:id`,
  `/share-grants/:id/backfill`, `/share-grants/:id/token`,
  `/share-grants/by-collection/:id`. UI: in `components/Shares/`.
- **Contacts** — `/contacts`, `/contacts/nearby`, `/contacts/:id`,
  `/contacts/from-discovery`, `/contacts/from-invite`. UI:
  `components/Contacts/*`.
- **Profile** — `/profile/me`, `/profile/:peerId`, `/profile/invite`.
- **Conversations/Messaging** — `/conversations/:id`, `/conversations/batch`,
  etc. UI: `components/Messaging/*`. Plus the whole webui expects
  `/rooms/joined/*` shape.
- **Now Playing** — `GET /nowplaying`, `PUT /nowplaying`, `DELETE /nowplaying`.
- **Listening Party** — `GET /listening-party` + hub
  `/hub/listening-party`. UI + hub.
- **Player** — `GET /player/external-visualizer`,
  `POST /player/external-visualizer/launch`, plus `PlayerBar`, `PlayerRadio`,
  `PlayerAutoQueue`, `PlayerRatings`, `PlayerShortcuts`.
- **Pods** — `/api/v0/pods/*`, UI under `components/Pods/*`. Multi-tier DHT
  pods overlay.
- **Port Forwarding** — `/api/v0/port-forwarding`, UI
  `components/PortForwarding/*`. VPN/NAT port status display.
- **Destinations** — `/destinations`, `/destinations/default`,
  `/destinations/validate`.
- **Discovery / DiscoveryGraph / DiscoveryInbox / DiscoveryShelf** —
  `/discovery-graph`, `/realm-subject-indexes/:realm/conflicts`, `/mesh/sync/:id`,
  plus multiple `lib/discovery*.js` modules.
- **Solid** (profile pods, identity) — `components/Solid/*` +
  `lib/identity.js`.
- **SongID** — `/songid/runs`, `/songid/runs/:id`,
  `/songid/runs/:id/forensic-matrix` + `/hub/songid`.
- **Taste Recommendations** — `/taste-recommendations[/graph-preview|
  /release-radar|/wishlist]`.
- **Soulseek recommendations / interests** — `/soulseek/interests`,
  `/soulseek/hated-interests`, `/soulseek/recommendations[/global]`,
  `/soulseek/users/similar`, `/soulseek/users/:u/interests`,
  `/soulseek/items/:item/(recommendations|similar-users)`.
- **MusicBrainz integration** — `/musicbrainz/albums/completion`,
  `/musicbrainz/artist/:mbid/discography-coverage`,
  `/musicbrainz/release-radar/(subscriptions|notifications)`,
  `/musicbrainz/targets`.
- **Lidarr integration** — `/integrations/lidarr/status|manualimport|
  wanted/(missing|sync)`.
- **Spotify integration** — `/integrations/spotify[/authorize|/status]`.
- **Library Health** — `/api/library/health/scans[/:id]`,
  `/api/library/health/issues[/by-artist|by-release|by-type/:t|:id|/fix]`.
  Note: these calls from webui **retain the `/api` prefix**, they are at
  `/api/library/health/...`, not `/api/v0/library/...`. Backend compatibility
  controllers in slskdN handle this.
- **Jobs** — `/jobs/:type`, `/jobs/discography`, `/jobs/mb-release`.
- **Transfers: AutoReplace / Ranking / MultiSource** — `/autoreplace[/enable|
  disable]`, `/transfers/downloads/(accelerated|auto-replace|find-alternative|
  replace|stuck|user-stats)`, `/transfers/speeds`, `/multisource/jobs`.
- **Relay** — `GET/PUT/DELETE /relay`.
- **VirtualSoulfind Bridge** — `/bridge/(start|stop|status|transfer/:id/progress)`
  and `/bridge/admin/(clients|config|dashboard|stats)`.
- **User Notes** — `GET/POST/DELETE /users/notes[/:id]`.
- **Source providers / Source feed imports** — `/source-providers`,
  `/source-feed-imports/preview`.
- **Swarm analytics / NowPlaying / DiscoveryGraph / HashDB backfill** —
  assorted POSTs.

### WebUI-only infrastructure slskR lacks

- **SignalR hubs** at `/hub/application`, `/hub/logs`, `/hub/search`,
  `/hub/songid`, `/hub/transfers`, `/hub/listening-party`, `/hub/relay`. The
  webui absolutely depends on these for live updates of search results,
  transfer progress, log tailing, and global application state pushes.
- **URL-base rewriting** — the webui respects `window.urlBase` set by the
  server in `index.html`. slskR must inject this if the app is served under a
  reverse-proxy path.
- **X-CSRF-TOKEN cookie-per-port** — `XSRF-TOKEN-<port>` cookie. slskR has
  cross-site origin rejection but not a cookie token the webui can read.
- **`/api/v0/`-prefixed routes** — slskR rewrites `/api/v0/X` → `/api/X` via a
  compatibility shim; that works for most calls but **the `/api/library/...`
  endpoints called by the webui never hit `/api/v0`** and would need dedicated
  handling.

## 3. Options for porting the webui

### Option A — "Plug and pray" proxy (fastest smoke test)

Run the existing webui against a Rust dev proxy. Concretely:

1. Copy `src/web/` into `slskR/web/` (untouched).
2. Change `VITE_SLSKD_PORT` / `apiBaseUrl` to point at `slskr serve`.
3. Start both: `cargo run -p slskr -- serve` and `npm run dev` in `web/`.
4. Load the UI, observe everything that 404s or 4xx's, and file it as a gap.

Result: instantly gives us a **live parity oracle**. The browser's network
panel and react-toastify error toasts become the checklist. But huge swaths of
the UI will be dark: no SignalR pushes, no wishlist/collections/etc.

### Option B — Full port of the big webui (correct end-state)

Adopt the entire `src/web/` tree as the product UI and drop the tiny
`dashboard/` (or fold its admin pages into a `System/` tab inside the big UI).

Steps, in order:

1. **Copy the webui tree** into the slskR repo as `web/` (or merge it into
   `dashboard/` after deleting the old admin pages). Keep Vite build output
   wired into `crates/slskr/src/main.rs`'s `index_html_response` via a build
   step (embed with `include_bytes!` or serve from a configured
   `SLSKR_WEB_DIST` dir).
2. **Rebrand** the three hard-coded `slskdn` strings
   (`SLSKDN_RELEASES_URL`, `NETWORK_ENDPOINT_NOTICE_STORAGE_KEY`,
   `THEME_OPTIONS` "slskdn" value) to `slskr`. Storage keys stay stable to
   avoid theme-reset churn.
3. **Introduce a real router** in `main.rs`. The current big `match` is fine
   for 47 routes but will collapse at 291. Two paths:
   - Swap to `axum` with typed path extractors (recommended). The routing
     module `src/routing.rs` already exists but is tiny.
   - Or keep the hand-rolled approach and add a small per-segment dispatcher.
4. **Stand up SignalR-compatible hubs** at `/hub/*`. Easiest path:
   - implement the SignalR WebSocket negotiate/handshake/JSON protocol (it is
     a documented, small subset for server-to-client push: negotiate → WS
     open → protocol handshake → frames with `type: 1` invocation messages).
   - or **fork the webui to use native WebSocket/SSE** against new slskR
     endpoints. Smaller fan-out but invasive changes to every real-time
     component (`Searches`, `Transfers`, `Chat`, `Logs`, `System`).
   - recommendation: build a thin SignalR JSON-protocol compatibility shim
     behind the existing `websocket.rs` so the webui is unmodified. This is
     a ~few-hundred-LoC job and buys us all push surfaces at once.
5. **Normalize paths**. Either:
   - add compatibility handlers in slskR that forward the webui's expected
     paths (e.g. `/api/v0/rooms/joined/:room/messages` →
     `/api/rooms/:room/messages`), or
   - change slskR's canonical paths to match the webui. The webui paths are
     richer (`rooms/joined` vs `rooms/available`) so the webui shape is the
     better target. Do this as part of the router refactor.
6. **Walk the 291-call list** as a parity checklist, implementing handlers
   group by group against `slskr-client` runtime. Where a feature is
   explicitly out of scope for slskR (e.g. Bridge/VirtualSoulfind,
   DiscoveryGraph, Solid, Pods overlay, SongID), take one of these choices
   per feature area, and document it:
   - Implement it (if it maps to a Soulseek feature like rooms/searches).
   - Stub the endpoints to return `501 Not Implemented` with a typed shape
     the UI can render as "disabled" (the webui already tolerates disabled
     integrations for Lidarr/Spotify).
   - Hide the UI pages via build-time flag and remove their lib modules.
7. **Auth/CSRF**: issue `XSRF-TOKEN-<port>` cookie on GET `/` so the existing
   webui axios interceptor's CSRF header works out of the box.
8. **Build integration**: add `web/` build to Cargo via `build.rs` or a pre-
   release script that runs `npm ci && npm run build` and stamps the result
   into `crates/slskr/assets/web/`. Serve it from `main.rs` with a static
   file handler keyed on the request path fall-through.

### Option C — Hybrid (recommended)

Do Option A *first*, treat it as our **parity oracle**, while doing Option B
incrementally:

1. Week 1 — land the copy + a Vite dev-server proxy to `:5030`. Nothing in
   slskR changes; UI loads with lots of broken sub-screens but the working
   core (sessions/shares/searches/transfers/rooms/messages/browse) should
   already render. Gate behind `SLSKR_WEB_DEV=1`.
2. Week 2 — router refactor in `crates/slskr/src/routing.rs`; switch to
   `axum`. Keep the old `match` wrapped behind the new router during the
   transition. Path fan-out becomes tractable.
3. Week 3 — SignalR compatibility hub prototype (application + transfers +
   search). Attach to the existing event bus.
4. Weeks 4+ — feature group by feature group: wishlist, collections,
   contacts/profile, share-groups, share-grants, destinations,
   autoreplace/ranking, integrations (start with Lidarr since it's the most
   useful), library-health, jobs, now-playing. Per group: protocol work in
   `slskr-client`, projection types, route handlers, webui smoke test.
5. Later — SongID, Pods, Solid, Bridge, VirtualSoulfind, DiscoveryGraph,
   ListeningParty, Player visualizer. These are "longer-range modules"
   already called out in `legacy-port-harvest.md` §"Longer-Range Modules"
   and can stay stubbed until the core is rock-solid.

## 4. Using the webui as a validation harness

The webui doubles as a parity test. Proposed workflow:

- **Artifact 1**: keep the canonical list `/tmp/webui_routes_with_methods.txt`
  (291 rows) in-tree as `docs/webui-endpoints.txt` and regenerate on each
  upstream webui sync. Treat it as the required-API surface.
- **Artifact 2**: script `scripts/check-webui-endpoints.sh` that diffs that
  file against the slskR route inventory (extract from `main.rs` /
  `routing.rs`) and prints:
  - Implemented and matching shape.
  - Implemented but path-mismatched.
  - Missing.
- **Artifact 3**: a Playwright e2e harness that mounts the webui against a
  running `slskr serve` fixture and walks each top-level route
  (`/searches`, `/wishlist`, `/transfers`, …) asserting that the page loads
  without network errors above some budget. The webui already has
  `playwright.config.ts` and e2e tests we can borrow.
- **Artifact 4**: use the existing `docs/openapi.json` as a second oracle —
  generate one for slskR, diff-test against the slskdN one for the subset of
  paths we commit to maintaining.

## 5. Concrete first-PR scope

Small, mergeable starting point:

1. Add this document (done).
2. Snapshot `docs/webui-endpoints.txt` (the 291-row list).
3. Add `scripts/diff-webui-endpoints.sh` that prints the gap using the
   current `main.rs` as source of truth.
4. Refactor `crates/slskr/src/routing.rs` to own the dispatch table
   (without changing any behavior).
5. Add `GET /api/v0/rooms/joined`, `GET /api/v0/rooms/available`,
   `POST /api/v0/rooms/joined`, `DELETE /api/v0/rooms/joined/:room`,
   `GET /api/v0/rooms/joined/:room/messages`,
   `GET /api/v0/rooms/joined/:room/users` as aliases/thin wrappers over the
   existing room projection. This unblocks the webui's `Rooms/` component
   without new protocol work.
6. Add `GET /api/v0/application`, `GET /api/v0/application/version/latest`,
   `GET /api/v0/server`, `GET /api/v0/session/enabled`. Purely synthesized
   from current state and version info.
7. Gate the whole webui behind `SLSKR_WEB_DEV`/`SLSKR_WEB_DIST` and document
   in `docs/install.md`.

Subsequent PRs tackle one feature area at a time and update the gap doc.

## 6. Risks and open questions

- **SignalR protocol compatibility**: the JSON protocol is small but
  binary message-pack support, reconnection quirks, group membership, and
  server-to-client streaming all need verification. Decide early whether to
  mimic SignalR or fork the webui.
- **Semantic UI React** is a 2.x library; works fine but it is heavy. We
  may want to migrate to Tailwind over time to match the current
  `dashboard/`. Keep both for now.
- **Feature surface the Rust repo has no intent of shipping** (Bridge,
  Pods, Solid, DiscoveryGraph, SongID, ListeningParty, Player visualizer,
  VirtualSoulfind, realm-subject-indexes, MusicBrainz overlays). Each
  pulls in substantial backend. Decide explicitly whether to cut the UI or
  return `501` stubs.
- **Auth bootstrap**: the webui expects a login form hitting `POST /session`
  with username/password and an API token response. slskR currently expects
  `SLSKR_API_TOKEN` at startup. We need a login endpoint or an explicit
  "no-auth/passthrough" mode the webui already supports
  (`isPassthroughEnabled()`).
- **CSRF cookie emission**: slskR currently rejects cross-origin unsafe
  methods but does not set an `XSRF-TOKEN-<port>` cookie. Add one on
  `GET /` or add a dedicated `GET /api/v0/csrf` endpoint.
- **Path prefix irregularities**: library-health uses `/api/...` (not
  `/api/v0/...`) in the webui code. This is a quirk of slskdN's compatibility
  controllers. Mirror the same irregularity or patch the webui.
