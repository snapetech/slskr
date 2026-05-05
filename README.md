# slskr

`slskr` is a self-hosted Rust application for the Soulseek network. It runs as a single daemon with a bundled browser UI, HTTP API, WebSocket event feed, protocol runtime, transfer engine, share indexer, observability endpoints, client libraries, and live interoperability tooling.

It is built for operators who want a private, scriptable music-sharing client they can run locally, in a lab, or behind their own service boundary while still having a full web control surface.

## Table Of Contents

- [Highlights](#highlights)
- [Screenshots](#screenshots)
- [Feature Index](#feature-index)
- [Architecture](#architecture)
- [Install And Run](#install-and-run)
- [Configuration](#configuration)
- [Web UI](#web-ui)
- [HTTP API](#http-api)
- [CLI Commands](#cli-commands)
- [Search And Discovery](#search-and-discovery)
- [Browse, Shares, And Library State](#browse-shares-and-library-state)
- [Transfers](#transfers)
- [Messaging, Rooms, Users, And Contacts](#messaging-rooms-users-and-contacts)
- [Player And Media Tools](#player-and-media-tools)
- [Integrations](#integrations)
- [Security Model](#security-model)
- [Observability](#observability)
- [Persistence And State](#persistence-and-state)
- [Client Libraries](#client-libraries)
- [Testing And Verification](#testing-and-verification)
- [Deployment](#deployment)
- [Repository Layout](#repository-layout)
- [Reference Documents](#reference-documents)
- [License](#license)

## Highlights

- One `slskr serve` process for the daemon, API, web UI, session runtime, share scanner, transfer state, and static assets.
- Rust protocol crates for server, peer-message, file-transfer, distributed, init, obfuscation, and wire-frame behavior.
- Browser UI for search, downloads, uploads, rooms, messages, users, contacts, browse, collections, share groups, shared-with-me views, discovery graph tools, playlist intake, integrations, system status, and playback.
- Search workflow with query history, result aggregation, result ranking, deduplication, lossless/acquisition profiles, wishlist search, discovery graph views, release-radar panels, MusicBrainz lookup, SongID workflows, and local review controls.
- Download and upload views with grouped transfers, progress, speed, queue position checks, retry/cancel/remove actions, accelerated download mode, and auto-replace controls.
- Share indexing with virtual paths, scan limits, hidden-file/symlink policy, extension summaries, deterministic catalog APIs, and safe file listing that avoids leaking host paths.
- Direct, obfuscated, and indirect peer paths for messaging, browsing, and file-transfer probes.
- Bearer-token API authentication, same-site browser session cookies, CSRF origin checks for mutating browser requests, loopback-safe defaults, and non-loopback exposure guards.
- WebSocket event feed plus bounded event polling for search, transfer, share, user, browse, message, room, and session workflows.
- Health, metrics, telemetry, release/build metadata, configuration inspection, and live smoke automation.
- TypeScript, Python, Go, and Rust client surfaces for API automation and validation.

## Screenshots

The screenshots are generated from the local React UI with mocked daemon responses, so they are safe for documentation and do not contain credentials.

| Search and discovery | Transfers |
| --- | --- |
| ![Search page with acquisition controls, discovery panels, and player chrome](./docs/screenshots/webui-searches.png) | ![Downloads page with transfer controls and player chrome](./docs/screenshots/webui-downloads.png) |

| Rooms and messaging | System status |
| --- | --- |
| ![Rooms page with joined rooms and message context](./docs/screenshots/webui-rooms.png) | ![System page with daemon and configuration status](./docs/screenshots/webui-system.png) |

Regenerate the images from a running frontend with:

```bash
cd web
SLSKR_SCREENSHOT_BASE_URL=http://127.0.0.1:3001 \
  node scripts/capture-readme-screenshots.mjs
```

## Feature Index

| Area | What slskr provides | Primary references |
| --- | --- | --- |
| Daemon | `slskr serve`, bundled UI/API, session lifecycle, listener state, reconnect policy | [docs/app-surface.md](./docs/app-surface.md), [docs/slskr.config.example.toml](./docs/slskr.config.example.toml) |
| Protocol runtime | Login, keepalive, server commands, peer init, peer messages, distributed messages, transfer sockets, obfuscation | [crates/slskr-client](./crates/slskr-client), [crates/slskr-protocol](./crates/slskr-protocol) |
| Web UI | Search, transfers, rooms, messages, browse, users, contacts, system, player, collections, integrations | [web/README.md](./web/README.md), [web/src/components](./web/src/components) |
| HTTP API | Versioned `/api/v0/*` endpoints, unversioned compatibility routes, auth, telemetry, metrics | [docs/http-api.md](./docs/http-api.md), [docs/openapi.json](./docs/openapi.json) |
| Events | Polling event log and `/api/events/ws` WebSocket stream | [docs/app-surface.md](./docs/app-surface.md) |
| Shares | Share root scanning, virtual catalog, rescan endpoint, filtered file APIs | [docs/app-surface.md](./docs/app-surface.md) |
| Transfers | Queue projections, progress updates, direct and indirect peer transfer execution, inbound serving | [docs/app-surface.md](./docs/app-surface.md), [scripts/run-live-http-transfer-smoke.sh](./scripts/run-live-http-transfer-smoke.sh) |
| Client SDKs | TypeScript, Python, Go, and Rust client packages/examples | [docs/CLIENT_LIBRARIES.md](./docs/CLIENT_LIBRARIES.md) |
| Live verification | Local peer smoke, public-network probes, HTTP transfer smoke, open fixture policy | [docs/live-interop-test-matrix.md](./docs/live-interop-test-matrix.md), [docs/open-commons-fixtures.md](./docs/open-commons-fixtures.md) |
| Deployment | Kubernetes manifests, Prometheus rules, public posture checks, release package checks | [k8s](./k8s), [scripts/check-release-package.sh](./scripts/check-release-package.sh) |

## Architecture

`slskr` is split into small crates and one browser workspace:

- `crates/slskr`: the application binary, HTTP server, API routing, web UI serving, daemon orchestration, config loading, storage projections, auth, rate limiting, logging, telemetry, webhooks, and OpenAPI embedding.
- `crates/slskr-client`: async runtime for server sessions, peer connections, listeners, searches, browsing, social/user operations, transfers, stream handling, peer cache, distributed tree state, and live probes.
- `crates/slskr-protocol`: protocol message types, binary codecs, frame parsing, primitives, peer/server/distributed/init messages, obfuscation helpers, and wire-format tests.
- `crates/slskr-cli`: internal command runner used by smoke tests and interop probes while the public binary remains `slskr`.
- `web`: React/Vite web UI served by the daemon and exercised by unit, integration, and Playwright tests.
- `client-ts`, `client-python`, `client-go`: generated or maintained API clients and examples for automation.
- `docs`, `examples`, `scripts`, and `k8s`: operator references, smoke automation, public posture checks, fixture policy, and deployment manifests.

The daemon owns the long-lived runtime state. The web UI and clients talk to the daemon through HTTP and the event feed rather than holding protocol sockets themselves.

## Install And Run

Prerequisites:

- Rust toolchain compatible with the workspace `rust-version`.
- Node.js/npm for web UI development and tests.
- Soulseek credentials for live network operation.
- Optional: Go for the Go client tests, Python for the Python client checks, Kubernetes tooling for manifest workflows.

Print the binary version:

```bash
cargo run -p slskr -- version
```

Run a low-risk login smoke:

```bash
SLSK_USERNAME=<username> \
SLSK_PASSWORD=<password> \
cargo run -p slskr -- login smoke
```

Run the daemon and bundled web UI:

```bash
SLSK_USERNAME=<username> \
SLSK_PASSWORD=<password> \
SLSKR_AUTO_CONNECT=true \
cargo run -p slskr -- serve
```

Default bind:

```text
127.0.0.1:5030
```

Then open:

```text
http://127.0.0.1:5030/
```

## Configuration

Configuration can come from environment variables, `SLSKR_CONFIG`, or the default config path under `$XDG_CONFIG_HOME/slskr/config.toml` when present. The state directory defaults under `$XDG_STATE_HOME/slskr` or `$HOME/.local/state/slskr`.

Common controls:

- `SLSKR_HTTP_BIND`: HTTP listener, default `127.0.0.1:5030`.
- `SLSKR_CONFIG`: path to TOML config.
- `SLSKR_STATE_DIR`: daemon state directory.
- `SLSKR_AUTO_CONNECT`: connect on startup when credentials exist.
- `SLSKR_RECONNECT` and `SLSKR_RECONNECT_SECONDS`: reconnect policy.
- `SLSKR_PING_SECONDS`: server keepalive interval.
- `SLSKR_LISTENER_BIND` and `SLSKR_ADVERTISED_PORT`: regular peer listener and advertised port.
- `SLSKR_OBFUSCATED_LISTENER_BIND` and `SLSKR_OBFUSCATED_ADVERTISED_PORT`: type-1 obfuscated listener and advertised port.
- `SLSKR_SHARE_DIRS`: semicolon-separated share roots.
- `SLSKR_SHARE_FOLLOW_SYMLINKS`, `SLSKR_SHARE_INCLUDE_HIDDEN`, `SLSKR_SHARE_SCAN_MAX_FILES`: share scan policy.
- `SLSKR_TRANSFER_MAX_ACTIVE`: concurrent active transfer cap.
- `SLSKR_TRANSFER_ALLOW_INBOUND` and `SLSKR_TRANSFER_ALLOW_OUTBOUND`: transfer direction policy.
- `SLSKR_API_TOKEN`: bearer token for protected API routes.
- `SLSKR_AUTH_DISABLED`: explicit auth override.
- `SLSKR_PERSISTENCE_ENABLED`: enable SQLite-backed persistence paths that are currently wired.
- `SLSKR_SPOTIFY_ENABLED`, `SLSKR_SPOTIFY_CLIENT_ID`, `SLSKR_SPOTIFY_REDIRECT_URI`: Spotify integration.
- `SLSKR_LIDARR_ENABLED`, `SLSKR_LIDARR_URL`, `SLSKR_LIDARR_API_KEY`: Lidarr integration.
- `SLSKR_EXTERNAL_VISUALIZER_COMMAND`: optional local visualizer launcher.

Use [docs/slskr.config.example.toml](./docs/slskr.config.example.toml) as the annotated config reference.

## Web UI

The bundled UI is an operator dashboard, not a marketing page. It is designed for repeated control workflows:

- Search: query entry, acquisition profile selection, current/history view, search detail pages, result filters, result ranking, duplicate folding, blocked-user hiding, local review notes, and graph-driven follow-up searches.
- Discovery Graph: graph atlas and modal views for navigating nearby artists, releases, tracks, and query branches.
- Playlist Intake: import/review workflows for source lists before turning them into searches or library actions.
- Wishlist: saved search intents and repeatable watch workflows.
- Downloads and Uploads: grouped transfer cards by peer and directory, progress, speeds, selected bulk actions, retry/cancel/remove controls, accelerated mode, and auto-replace toggles.
- Messages, Chat, and Rooms: private conversations, room lists, joined-room activity, message acknowledgement, room-user views, and navigation badges.
- Users, Contacts, Share Groups, Shared With Me, and Collections: social/library organization surfaces for peer context, shared objects, and grouped access.
- Browse: peer browse request state and browse cache display.
- System: runtime state, configuration, shares, network status, telemetry, integrations, and operational controls.
- Player: persistent bottom player bar, transport controls, spectrum visualizer, queue/list controls, lyrics pane support, and collection/local-file playback hooks.
- Theme: `slskr`, classic dark, and light theme choices.

## HTTP API

The daemon exposes versioned API routes under `/api/v0/*` plus selected compatibility aliases under `/api/*`. Security-sensitive operations require bearer-token auth when auth is enabled, and browser-origin mutating requests are checked for CSRF safety.

Core endpoint groups:

- Health/version/config: `/api/health`, `/api/v0/health`, `/api/version`, `/api/v0/version`, `/api/v0/config`.
- Capabilities: `/api/v0/capabilities`, `/api/v0/capabilities/negotiate`.
- Stats/metrics/telemetry: `/api/v0/stats`, `/api/v0/metrics`, `/api/v0/telemetry`.
- Events: `/api/v0/events`, `/api/events/ws`.
- Shares and catalog: `/api/v0/shares`, `/api/v0/shares/catalog`, `/api/v0/files/:root`, `/api/v0/shares/rescan`.
- Search: `/api/v0/searches`, `/api/v0/searches/:token`, `/api/v0/searches/:token/complete`, `/api/v0/searches/prune`, `/api/v0/search-responses`.
- Users and browse: `/api/v0/users`, `/api/v0/users/watch`, `/api/v0/users/:username/browse`, `/api/v0/users/:username/browse/request`, `/api/v0/browse-responses`.
- Messages: `/api/v0/messages`, `/api/v0/messages/:username`, `/api/v0/messages/:id/ack`.
- Rooms: `/api/v0/rooms`, `/api/v0/rooms/refresh`, `/api/v0/rooms/:room/join`, `/api/v0/rooms/:room/messages`.
- Session/listeners: `/api/v0/session`, `/api/v0/session/connect`, `/api/v0/session/disconnect`, `/api/v0/session/ping`, `/api/v0/listeners`.
- Transfers: `/api/v0/transfers`, `/api/v0/transfers/stats`, `/api/v0/transfers/:id/start`, `/api/v0/transfers/:id/progress`, `/api/v0/transfers/:id/complete`, `/api/v0/transfers/:id/cancel`, `/api/v0/transfers/:id/fail`.

Example search request:

```bash
curl -H "Authorization: Bearer $SLSKR_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"Example sound file Ogg Vorbis","target":"global"}' \
  http://127.0.0.1:5030/api/v0/searches
```

Example transfer projection:

```bash
curl -H "Authorization: Bearer $SLSKR_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"direction":"download","peer_username":"commons_peer","filename":"Example_sound_file_in_Ogg_Vorbis_format.ogg","size":153301}' \
  http://127.0.0.1:5030/api/v0/transfers
```

See [docs/http-api.md](./docs/http-api.md), [docs/http-api-features.md](./docs/http-api-features.md), and [docs/openapi.json](./docs/openapi.json).

## CLI Commands

Prefer the `slskr` binary for operator workflows:

```bash
slskr serve
slskr version
slskr login smoke
slskr soak live
slskr smoke local-peer
slskr probe peer-address
slskr probe plain-peer
slskr probe obfuscated-peer
slskr probe indirect-peer
slskr probe distributed-peer
slskr probe file-transfer-peer
slskr probe metadata-relogin
slskr probe negative-indirect
```

The probe commands are intentionally narrow. They let you verify one network behavior at a time before running broader live smoke suites.

## Search And Discovery

Search features include:

- Global, user, room, and wishlist-targeted search records.
- Public-network dispatch when connected.
- Local share snapshot matching.
- Search TTL and prune behavior.
- Result ingestion through `/api/v0/search-responses`.
- Result counts, locked counts, peer metadata fields, slot availability, average speed, and queue length.
- Browser-side filters, hidden users, blocked users, duplicate folding, and user notes.
- Acquisition profiles such as lossless-exact matching.
- MusicBrainz lookup, discography coverage, artist release radar, SongID panels, discovery graph atlas, and federated taste panels.

Useful example queries while testing:

- `Example sound file Ogg Vorbis`
- `Audacity click track eight seconds`

The open fixture policy and source list are documented in [docs/open-commons-fixtures.md](./docs/open-commons-fixtures.md).

## Browse, Shares, And Library State

Share and browse features include:

- Startup share scanning from configured roots.
- Optional symlink following and hidden-file inclusion.
- Scan caps for predictable startup behavior.
- State-dir share cache.
- Virtual share paths instead of host path disclosure.
- Root-level file APIs with `q`, `prefix`, `extension`, `limit`, and `offset`.
- Browse requests per user.
- Folder-content browse requests.
- Direct peer browse and indirect fallback paths.
- Browse status projection: requested, partial, ready, failed.
- Flattened browse-response ingestion for tests and external tools.

## Transfers

Transfer features include:

- Download and upload projection APIs.
- Queue, start, progress, complete, cancel, and fail operations.
- Transfer stats for counts and bytes.
- Restart-safe transfer state through `transfer-state.json`.
- State-dir transfer event log.
- Configurable active transfer cap.
- Inbound and outbound allow switches.
- Local-path metadata execution for file-backed projections.
- Direct plain and type-1 obfuscated file-transfer sockets.
- Indirect file-transfer fallback through server-mediated peer connection.
- Inbound shared-file serving for indexed local files.
- Resume/offset handling in the transfer handshake.
- Chunked progress events.
- Live HTTP transfer smoke automation.

Run the live HTTP transfer smoke with:

```bash
scripts/run-live-http-transfer-smoke.sh
```

## Messaging, Rooms, Users, And Contacts

Social and communication surfaces include:

- Private-message send, inbound projection, list, per-user read, and acknowledgement.
- Room list refresh, join, leave, room message projection, joined filtering, and recent room-message state.
- Watched users, watch/unwatch commands, user stats requests, and projected speed/upload/file/directory counts.
- Contacts, nearby contacts, invite flows, contact update/delete, and discovery import surfaces in the web client.
- User notes and local peer context controls in search results.

## Player And Media Tools

The web UI includes a persistent player surface:

- Transport controls.
- Queue/list controls.
- Local collection item playback hooks.
- Spectrum analyzer and visualizer components.
- Lyrics pane support.
- Listen-along/pod panel support.
- Player auto-queue, player radio, ratings, shortcuts, and now-playing helpers in the web library layer.

## Integrations

Current integration surfaces include:

- Spotify authorization callback through the same daemon HTTP port.
- Lidarr status, wanted-album sync, and manual import flows.
- ListenBrainz, source providers, source feed imports, and taste recommendation helpers in the web library layer.
- External visualizer command launch reporting.
- Bridge, relay, mesh, media-server, and federation diagnostics modules in the web client.
- Solid settings view and profile/contact identity surfaces.

Some integrations are operationally dependent on local config and API keys. The UI should show disabled or unavailable state when the daemon reports that an integration is not configured.

## Security Model

Default behavior is conservative:

- `slskr serve` binds to loopback by default.
- Protected API routes accept `Authorization: Bearer <token>` when `SLSKR_API_TOKEN` is configured.
- The browser dashboard can use a same-site `slskr.session` cookie for protected APIs.
- Mutating browser-origin API requests are checked against `Origin`/`Referer` to reduce CSRF exposure.
- Non-loopback HTTP binds require auth unless `SLSKR_AUTH_DISABLED=true` is explicitly set.
- Health, version, and public capability endpoints remain available for lightweight checks.
- Sanitized config responses do not expose credentials.
- Share APIs expose virtual paths rather than raw host paths.

Before exposing the service beyond localhost, configure an API token, use a reverse proxy you control, and decide whether public peer listener ports should be advertised directly or through a network boundary.

## Observability

Operational surfaces include:

- `/api/v0/health`: JSON health.
- `/api/v0/stats`: compact projection counts.
- `/api/v0/metrics`: Prometheus-style text metrics.
- `/api/v0/telemetry`: protected runtime snapshot.
- `/api/v0/events`: bounded event log with `kind`, `q`, `limit`, and `offset`.
- `/api/events/ws`: WebSocket event feed with topic/type/data/timestamp frames.
- Kubernetes `ServiceMonitor` and Prometheus rule examples under [k8s](./k8s).
- Public posture checks and live soak scripts under [scripts](./scripts).

## Persistence And State

Current state paths include:

- Share index cache in the state directory.
- Transfer event log and `transfer-state.json`.
- Optional SQLite persistence paths when `SLSKR_PERSISTENCE_ENABLED=true`.
- Search create/list hydration through SQLite when enabled.
- Restart-safe active transfer projection records.
- In-memory bounded event store for current event feed behavior.

The persistence boundary is intentionally explicit so operators can inspect and back up state without mixing it with secrets.

## Client Libraries

The repository includes client examples and packages for:

- TypeScript: [client-ts](./client-ts)
- Python: [client-python](./client-python)
- Go: [client-go](./client-go)
- Rust: [crates/slskr-client](./crates/slskr-client)

See [docs/CLIENT_LIBRARIES.md](./docs/CLIENT_LIBRARIES.md) for generated-client notes and examples.

## Testing And Verification

Common local checks:

```bash
cargo fmt --all --check
cargo test --workspace
```

```bash
cd web
npm test
npm run build
```

Release/package check:

```bash
scripts/check-release-package.sh
```

Local two-account peer smoke:

```bash
SLSKR_A_USERNAME=<user-a> \
SLSKR_A_PASSWORD=<pass-a> \
SLSKR_B_USERNAME=<user-b> \
SLSKR_B_PASSWORD=<pass-b> \
SLSKR_INDIRECT_HOST_OVERRIDE=127.0.0.1 \
cargo run -p slskr -- smoke local-peer
```

Open fixture workflow:

```bash
scripts/fetch-open-commons-fixtures.sh
scripts/verify-open-commons-fixtures.sh
```

Live matrix and soak scripts are under [scripts](./scripts). They are meant for deliberate operator runs because they use real accounts and live network behavior.

## Deployment

Deployment assets include:

- Kubernetes namespace, config map, deployment, service, ingress, HPA, PDB, RBAC, ServiceMonitor, and Prometheus rules under [k8s](./k8s).
- Public posture checks in [scripts/check-public-posture.sh](./scripts/check-public-posture.sh).
- Proton/NAT-PMP helper scripts for lab listener exposure.
- Release package validation in [scripts/check-release-package.sh](./scripts/check-release-package.sh).

The expected release shape is one binary, one config file, one state directory, optional container image, and explicit operator-controlled secrets.

## Repository Layout

```text
.
├── crates/
│   ├── slskr/            # daemon, API, web serving, config, storage, telemetry
│   ├── slskr-client/     # async session, peer, search, browse, transfer runtime
│   ├── slskr-cli/        # smoke/probe command implementation
│   └── slskr-protocol/   # protocol messages, codecs, frames, obfuscation
├── web/                  # React/Vite web UI
├── client-ts/            # TypeScript client
├── client-python/        # Python client
├── client-go/            # Go client
├── docs/                 # API, app-surface, install, fixture, and screenshot docs
├── examples/             # example workflows
├── fixtures/             # hash-pinned fixture manifests
├── k8s/                  # Kubernetes deployment and observability manifests
└── scripts/              # smoke, soak, posture, release, and fixture helpers
```

## Reference Documents

- [App surface](./docs/app-surface.md)
- [HTTP API reference](./docs/http-api.md)
- [HTTP API feature notes](./docs/http-api-features.md)
- [OpenAPI JSON](./docs/openapi.json)
- [Install notes](./docs/install.md)
- [Config example](./docs/slskr.config.example.toml)
- [Client libraries](./docs/CLIENT_LIBRARIES.md)
- [Live interop matrix](./docs/live-interop-test-matrix.md)
- [Open commons fixtures](./docs/open-commons-fixtures.md)
- [Integration guide](./docs/INTEGRATION_GUIDE.md)
- [Enhancements](./docs/ENHANCEMENTS.md)

Canonical repository:

```text
https://github.com/snapetech/slskr
```

## License

`slskr` is licensed under AGPL-3.0-only. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
