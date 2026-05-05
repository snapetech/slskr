# App/API Parity Backlog

This document tracks internal app/API parity requirements and implementation sequencing.

## Decision

Keep this repository as canonical. It has the strongest Soulseek network foundation: complete protocol inventory, listener demux, obfuscated type-1 transport, Soulfind contracts, live Proton/NAT-PMP interop, and the current `slskr serve` app shell.

Use this backlog as the parity map for:

- API routes and DTO concepts
- config hierarchy
- service boundaries
- long-range feature sequencing

Do not preserve its dirty worktree, git history, or source files.

## Near-Term Backfill

These are the pieces worth bringing into this repo first, implemented against the existing `slskr-client` runtime.

1. Durable configuration
   - YAML/TOML config file plus env overrides.
   - Preserve local env compatibility already used by probes.
   - Split settings into app, web, peer network, shares, transfers, search, users, rooms/messages, logging, metrics, and integrations.
   - Redact usernames/passwords/API keys in debug output and HTTP APIs.

2. HTTP API shape
   - Keep current minimal routes while growing toward `/api/v0/*`.
   - Current app routes now have `/api/v0/*` aliases for health, version, config, shares, session, listeners, and transfers.
   - Route contract tests now cover the current read-only JSON shapes, aggregate stats, session command routes including privilege checks, share rescan, versioned aliases, and 404 behavior.
   - Bearer-token API auth now protects non-health/version API routes when configured, and non-loopback binds require a token unless auth is explicitly disabled.

3. Search API
   - `POST /api/v0/searches`
   - `GET /api/v0/searches`
   - `GET /api/v0/searches/:token`
   - `POST /api/v0/searches/:token/complete`
   - `POST /api/v0/search-responses`
   - Model records with token, query, target, active/completed state, timestamps, flattened results, slot/free/queue/speed fields.
   - Initial route/state implementation exists for create/list/get/complete with local share-backed results, public-network global/user/room/wishlist dispatch, configurable expiration windows, expired-record pruning, list filtering/pagination, peer-response projection, and `POST /api/v0/search-responses` external result ingestion; richer grouping still needs to be attached.

4. Transfer API
   - `GET /api/v0/transfers`
   - `POST /api/v0/transfers`
   - `GET /api/v0/transfers/stats`
   - `POST /api/v0/transfers/:id/start`
   - `POST /api/v0/transfers/:id/progress`
   - `POST /api/v0/transfers/:id/complete`
   - `POST /api/v0/transfers/:id/cancel`
   - State model: queued, in_progress, succeeded, cancelled, failed, errored.
   - Record direction, peer username, remote virtual path, local target path, expected size, bytes transferred, last error, created/updated timestamps.
   - Initial app-state projection exists for create/start/progress/complete/cancel/fail plus stats and list filtering/pagination; real peer transfer execution/resume still needs to be attached.

5. Shares API
   - Current `/api/shares` and `/api/shares/rescan` are the seed.
   - `/api/v0/shares/catalog` exists with deterministic sort, virtual path, extension, attribute count, file count, filtered count, total bytes, and `q`/`prefix`/`extension`/`limit`/`offset` filters.
   - Root summaries include public labels, file counts, byte totals, and extension buckets.
   - Add richer root display naming controls.
   - Keep host absolute paths out of public JSON unless behind an authenticated local-admin view.

6. Users and Browse
   - `GET /api/v0/users`
   - `POST /api/v0/users/watch`
   - `DELETE /api/v0/users/:username/watch`
   - `GET /api/v0/users/:username/browse`
   - Track watched users, status, stats, browse result cache, and last update time.
   - Initial user-watch projection exists for list/watch/unwatch; server watch/unwatch commands, watch/status/stats event projection, and user-stats request command are attached. Browse request/cache projection, failed/partial/indirect-pending-state projection, browse list filtering/pagination, and flattened single-entry or batched `POST /api/v0/browse-responses` ingestion exist; real peer browse execution is attached for direct and indirect peer `GetShareFileList` and folder-content requests.

7. Messaging and Rooms
   - `POST /api/v0/messages`
   - `POST /api/v0/messages/inbound`
   - `GET /api/v0/messages/:username`
   - `POST /api/v0/messages/:id/ack`
   - `GET /api/v0/rooms`
   - `POST /api/v0/rooms/:room/join`
   - `POST /api/v0/rooms/:room/messages`
   - Back this with the existing server/private-message and room protocol code rather than a standalone mock.
   - Initial app-state projection exists for outbound/inbound messages, ack, room join, room list, room messages, and list filtering/pagination. Server PM/ack/room commands and inbound PM/room event projection are attached; richer room membership and delivery semantics still need to be backfilled.

8. Files
   - `GET /api/v0/files/:root` exists now as a flat virtual-file listing under one configured share-root label.
   - Root names should be configured aliases such as downloads/incomplete, not arbitrary absolute paths.
   - Expand to folder-oriented views later if the web UI needs directory navigation.

9. Events
   - Bounded in-process event log with named events exists now through `GET /api/v0/events`.
   - Plain WebSocket event delivery exists at `GET /api/events/ws`; SignalR compatibility is descoped.
   - Useful event names now include search started/completed/pruned, transfer queued/progress/completed/cancelled/failed, share scan completed, user watch/stat/browse/folder-browse requests, message sent/received/acknowledged, room join/leave/message/list requests, and session command requests.

10. Metrics and telemetry
   - `GET /api/v0/stats` exists now as a compact operational summary for session, listener, share, search, user, browse, message, room, and transfer projection counts.
   - `GET /api/v0/metrics` exists now as Prometheus-style text counters/gauges for the same projection surfaces.
   - `GET /api/v0/telemetry` exists now as a protected JSON runtime health snapshot for sanitized config flags, session/listener state, storage status, and projection counts.
   - Backfill durable counters/gauges and richer process health after the database/event-log layout lands.

## Config Surface To Recreate

Recreate these as typed config groups. Do not copy the old Rust structs directly.

- Top-level: debug, headless, remote_configuration, remote_file_management, instance_name.
- Web: bind address, port, URL base, static content path, request logging, HTTPS, auth.
- Web auth: disabled flag, username/password, JWT, API keys.
- Peer network: server address/port, username/password, description, picture, listen address/port, diagnostics, distributed-network options, timeouts, buffers, proxy.
- Distributed: disabled, disable_children, child_limit, logging.
- Shares: directories, path filters, cache storage mode, worker count, retention.
- Transfers: upload slots/speed, download slots/speed, queued/daily/weekly limits, retention by status.
- Groups: default, leechers, blacklisted, user-defined members, per-group upload policy.
- Filters: incoming search request filters.
- Throttling: incoming search concurrency, circuit breaker, response file limit.
- Logging: disk logging, Loki endpoint, no-color.
- Metrics: endpoint/auth settings.
- Integrations: VPN/Gluetun, webhooks, scripts, FTP, Pushbullet-style notifications.

## Longer-Range Modules

These should not block the Soulseek-compatible app, but they are worth preserving as future planning buckets.

- Wishlist: saved-search runner that schedules enabled wishes into normal search dispatch.
- Jobs: persistent background job queue for wishlist, library health, backfill, and media tasks.
- Library health: scan reports and cleanup recommendations.
- Now playing: Plex/Jellyfin/Emby webhooks feeding profile/status text.
- Capabilities: feature negotiation for future mesh/federation work.
- Identity/contacts: contact list, invites, LAN discovery.
- Realms/sharing: unify pods/share groups/collections as one group model if that feature returns.
- VirtualSoulfind v2: catalog/intent/match/plan pipeline.
- Hash DB/media/song ID/audio/discovery graph/backfill/streaming: useful after core search/download/share flows are durable.
- Relay: controller/agent model only after the base app is stable.
- Mesh/federation/Solid: explicitly later, optional features. Avoid rebuilding the old duplicate mesh/service-fabric stack.

## API Parity Route Inventory

Harvested route families from the old scaffold:

- Health/capabilities: `/api/v0/health`, `/api/v0/capabilities`, `/api/v0/capabilities/negotiate` exists for app capability intersection
- Session: `/api/v0/session`, `/api/v0/session/connect`, `/api/v0/session/disconnect`, `/api/v0/session/ping`, `/api/v0/session/privileges/check`
- Files: `/api/v0/files/:root`
- Shares: `/api/v0/shares/catalog`, `/api/v0/shares/rescan`
- Search: `/api/v0/searches`, `/api/v0/searches/:token`, `/api/v0/searches/:token/complete`, `/api/v0/search-responses`
- Users: `/api/v0/users`, `/api/v0/users/watch`, `/api/v0/users/:username/watch`, `/api/v0/users/:username/stats/request`, `/api/v0/users/:username/browse`
- Messages: `/api/v0/messages`, `/api/v0/messages/inbound`, `/api/v0/messages/:username`, `/api/v0/messages/:id/ack`
- Rooms: `/api/v0/rooms`, `/api/v0/rooms/refresh`, `/api/v0/rooms/:room/join`, `/api/v0/rooms/:room/messages`
- Transfers: `/api/v0/transfers`, `/api/v0/transfers/stats`, `/api/v0/transfers/:id/start`, `/api/v0/transfers/:id/progress`, `/api/v0/transfers/:id/complete`, `/api/v0/transfers/:id/cancel`, `/api/v0/transfers/:id/fail`
- Events/metrics/telemetry: `/api/v0/events`, `/api/v0/stats`, `/api/v0/metrics`, `/api/v0/telemetry`
- Wishlist/now-playing/moderation/library-health/sharing/identity/realms/jobs/signals/relay/federation/mesh/content routes are future expansion buckets, not first-release requirements.

## Sequencing

Recommended order from here:

1. Add typed config loading and config tests.
   - Initial TOML config loading exists for app, network, listener, profile, timeout, share, and transfer-history settings.
   - Environment variables still override file values for lab/probe workflows.
2. Replace ad hoc HTTP handling with a small router or bring in `axum`.
3. Add API contract tests for current health/config/session/listeners/shares/transfers routes.
4. Add search API state on top of `slskr-client` search dispatch/collector.
5. Add user watch/browse API.
6. Add messaging/rooms API.
7. Replace TSV share/transfer logs with a real database.
8. Implement transfer execution/resume using the existing `slskr-client` transfer primitives.
9. Only then consider optional app breadth such as wishlist, jobs, metrics, integrations, and relay.
