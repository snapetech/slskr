# slskr App Surface

`slskr` is the user-facing binary. Internal crates exist to keep the protocol/runtime testable, but install docs, service units, containers, and operator workflows should target `slskr`.

## Command Layout

- `slskr serve`: run the bundled app shell and daemon scaffold. Defaults to `SLSKR_HTTP_BIND=127.0.0.1:5030`.
- `slskr version`: print the Soulseek-network client name and version band.
- `slskr login smoke`: low-risk login and wait-port check.
- `slskr soak live`: low-volume public-network soak.
- `slskr smoke local-peer`: two-account local peer path smoke.
- `slskr probe peer-address`: server metadata lookup.
- `slskr probe plain-peer`: direct peer-message `P` check.
- `slskr probe obfuscated-peer`: type-1 obfuscated peer-message `P` check.
- `slskr probe indirect-peer`: server-mediated `ConnectToPeer` / `PierceFirewall` check.
- `slskr probe distributed-peer`: direct distributed `D` check.
- `slskr probe file-transfer-peer`: direct file-transfer `F` check.
- `slskr probe metadata-relogin`: peer metadata stability check.
- `slskr probe negative-indirect`: explicit timeout/error behavior check.

Legacy `slskr-cli` command names remain accepted while scripts and local operator habits migrate, but public docs should prefer `slskr`.

## Daemon/API Direction

The `slskr serve` process owns the app state and should grow into:

- one Soulseek session lifecycle
- listener bind/advertise state for regular and obfuscated ports
- share indexing and excluded-phrase filtering
- search dispatch, result aggregation, and wishlist state
- transfer queue and resume state
- room, private-message, user-watch, and privilege state
- HTTP API and plain WebSocket event stream
- static web UI assets

The daemon calls `slskr-client` for protocol/runtime behavior rather than duplicating connection logic in the app crate. The current scaffold can optionally start a real server login session when credentials are provided through the environment.

## Initial HTTP Surface

The first app shell exposes:

- `GET /`: bundled local dashboard for session, listener, share, search, browse, message, room, user, catalog, and transfer projections, with controls for session connect/ping/disconnect/privilege-check, starting/completing searches, watching/unwatching users, requesting browse, rescanning shares, queueing/updating transfer projections with explicit progress bytes, sending/acknowledging private messages, joining/leaving rooms, syncing the server room list, sending room messages with an explicit sender, filtering search/transfer/catalog/message/room/browse tables, refreshing projection tables, and saving the configured API token as a same-site browser session cookie for protected APIs
- `GET /api/health`: service health JSON
- `GET /api/v0/health`: versioned alias for service health JSON
- `GET /api/version`: client name and version JSON
- `GET /api/v0/version`: versioned alias for client name and version JSON
- `GET /api/v0/capabilities`: public capability summary for API, network, storage, and experimental surfaces
- `POST /api/v0/capabilities/negotiate`: compare requested app capabilities against the daemon's current capability set and return accepted/unsupported lists
- `GET /api/config`: sanitized daemon configuration; never includes credentials
- `GET /api/v0/config`: versioned alias for sanitized daemon configuration
- `GET /api/v0/stats`: compact aggregate counts for session, listeners, shares, searches, users, browse cache, messages, rooms, and transfers
- `GET /api/v0/metrics`: Prometheus-style text counters/gauges for session, listeners, shares, searches, users, browse cache, messages, rooms, and transfers
- `GET /api/v0/telemetry`: protected JSON runtime health snapshot with sanitized config flags, listener/session state, storage status, and projection counts
- `GET /api/v0/events`: bounded in-memory event log for recent search, transfer, share, user, browse, message, room, and session workflows. Supports `kind`, `q`, `limit`, and `offset` query parameters.
- `GET /api/events/ws`: WebSocket event feed backed by the same event store as `/api/v0/events`. Frames include `topic`, `type`, `data`, `timestamp`, and the raw `event` record for SDK compatibility.
- `GET /api/shares`: sanitized share-index status, root labels, file/byte counts, per-root extension summaries, scan errors, and cache status
- `GET /api/v0/shares`: versioned alias for share-index status
- `GET /api/v0/shares/catalog`: deterministic sorted share catalog with virtual path, size, extension, attribute count, file count, filtered count, and total bytes. Supports `q`, `prefix`, `extension`, `limit`, and `offset` query parameters.
- `GET /api/v0/files/:root`: list files under one configured share-root label without exposing local host paths. Supports `q`, `prefix`, `extension`, `limit`, and `offset` query parameters.
- `POST /api/shares/rescan`: rebuild the in-memory share index from configured roots and rewrite the state-dir cache
- `POST /api/v0/shares/rescan`: versioned alias for share rescan
- `GET /api/v0/searches`: list search records. Supports `q`, `status`, `target`, `limit`, and `offset` query parameters. When persistence is enabled, startup hydrates this projection from SQLite search rows.
- `POST /api/v0/searches`: create a search record from JSON body `{"query":"..."}`, match against the current local share snapshot, optionally persist it to SQLite, and enqueue the public-network search command when connected. Optional `target` values are `global`, `user`, `room`, or `wishlist`; user searches require `username`, room searches require `room`, and `ttl_seconds` controls the active search expiration window.
- `GET /api/v0/searches/:token`: read one search record
- `POST /api/v0/searches/:token/complete`: mark one search record completed
- `POST /api/v0/searches/prune`: expire due active searches and remove expired records
- `POST /api/v0/search-responses`: merge one flattened result into a search record from JSON body with `token`, `filename`, `size`, and optional `peer_username`, `extension`, `slot_free`, `average_speed`, and `queue_length`
- `GET /api/v0/users`: list watched/user projection records
- `POST /api/v0/users/watch`: watch a username from JSON body `{"username":"..."}` and enqueue the server watch command when connected
- `DELETE /api/v0/users/:username/watch`: mark a user unwatched and enqueue the server unwatch command when connected
- `POST /api/v0/users/:username/stats/request`: enqueue a server user-stats request when connected; matching server stats update the user projection with average speed, upload count, file count, and directory count
- `GET /api/v0/browse`: list cached browse projections. Supports `q`, `status`, `limit`, and `offset` query parameters.
- `GET /api/v0/users/:username/browse`: read cached browse projection for one user
- `POST /api/v0/users/:username/browse/request`: mark browse requested and enqueue peer-address lookup when connected; when a matching peer-address response arrives, the app dials the peer directly or requests indirect peer-message connection fallback, sends `GetShareFileList`, parses the compressed share-list payload, and updates the browse cache
- `POST /api/v0/users/:username/browse/folder`: mark a folder-content browse requested from JSON body `{"folder":"..."}` and enqueue peer-address lookup when connected; matching peer-address responses send `FolderContentsRequest` directly or through indirect peer-message fallback
- `POST /api/v0/users/:username/browse/fail`: mark browse failed with optional JSON body `{"reason":"..."}`; peer browse failures also project this state automatically
- `POST /api/v0/browse-responses`: merge flattened browse results from JSON body with `username` plus either one `filename`/`size`/optional `extension` or an `entries` array of those fields. Optional `complete:false` records the browse as `partial`; omitted or true promotes it to `ready`.
- `POST /api/v0/messages`: record an outbound private-message projection from JSON body with `username` and `body`, then enqueue the server private-message command when connected
- `POST /api/v0/messages/inbound`: record an inbound private-message projection
- `GET /api/v0/messages`: list message projections. Supports `q`, `username`, `direction`, `limit`, and `offset` query parameters.
- `GET /api/v0/messages/:username`: list message projections for one user. Supports `q`, `direction`, `limit`, and `offset` query parameters.
- `POST /api/v0/messages/:id/ack`: mark a message projection acknowledged and enqueue the server acknowledgement when connected
- `GET /api/v0/rooms`: list room projections, including joined state, room kind, user count, operated flag, message count, and recent projected messages. Supports `q`, `joined`, `limit`, and `offset` query parameters.
- `POST /api/v0/rooms/refresh`: enqueue a server room-list request when connected
- `POST /api/v0/rooms/:room/join`: mark a room joined and enqueue the server room-join command when connected
- `DELETE /api/v0/rooms/:room/join`: mark a room left and enqueue the server room-leave command when connected
- `POST /api/v0/rooms/:room/messages`: record a room-message projection from JSON body with `username` and `body`, then enqueue the server room-message command when connected
- `GET /api/session`: current Soulseek session state, including last server message name and counters
- `GET /api/v0/session`: versioned alias for current session state
- `GET /api/listeners`: listener bind/local address, inbound connection counters, and redacted last inbound event
- `GET /api/v0/listeners`: versioned alias for listener state
- `GET /api/transfers`: current in-memory transfer history and state-dir event log status
- `GET /api/v0/transfers`: versioned alias for transfer history. Supports `q`, `status`, `direction`, `username`, `limit`, and `offset` query parameters.
- `POST /api/v0/transfers`: create an in-memory queued transfer projection from JSON body with `filename` and optional `direction`, `peer_username`, `local_path`, and `size`
- `GET /api/v0/transfers/stats`: aggregate transfer projection counts and transferred bytes
- `POST /api/v0/transfers/:id/start`: mark a transfer in progress; when `local_path` is present without a peer, validate the file on disk and complete or fail the projection from real metadata; when `peer_username` is present, request peer-address metadata, negotiate a peer-message `TransferRequest`/`TransferResponse`, and use the direct `F` connection token/offset handshake for local-path upload/download streaming, preferring type-1 obfuscated `F` init when advertised and falling back to plain direct `F`; if direct `F` connect fails, request server-mediated `ConnectToPeer`/`PierceFirewall` and retry the same file stream over the indirect socket. Inbound peer transfer requests for locally indexed share files are accepted and served over incoming direct `F`, `PeerInit F`, or `PierceFirewall` file-transfer sockets.
- `POST /api/v0/transfers/:id/progress`: update transferred bytes from JSON body `{"bytes_transferred":123}`
- `POST /api/v0/transfers/:id/complete`: mark a transfer succeeded
- `POST /api/v0/transfers/:id/cancel`: mark a transfer cancelled with optional `reason`
- `POST /api/v0/transfers/:id/fail`: mark a transfer failed with optional `reason`
- `POST /api/session/connect`: start a session using configured environment credentials
- `POST /api/v0/session/connect`: versioned alias for session connect
- `POST /api/session/disconnect`: drop the active session and suppress reconnect
- `POST /api/v0/session/disconnect`: versioned alias for session disconnect
- `POST /api/session/ping`: request an immediate server ping when connected
- `POST /api/v0/session/ping`: versioned alias for session ping
- `POST /api/session/privileges/check`: request a server privilege-time check when connected; matching responses update `session.privileges_seconds`
- `POST /api/v0/session/privileges/check`: versioned alias for session privilege check

Backfill the API from narrow read-only endpoints first, then add mutating operations behind local auth.

## Configuration

Initial configuration sources:

- environment variables for probes and smoke tests
- `SLSKR_CONFIG` for an optional TOML config file; if unset, `slskr serve` also loads `$XDG_CONFIG_HOME/slskr/config.toml` when it exists
- `SLSKR_HTTP_BIND` for the app HTTP listener
- `SLSKR_STATE_DIR` for daemon state, defaulting to `$XDG_STATE_HOME/slskr` or `$HOME/.local/state/slskr`
- `SLSKR_AUTO_CONNECT` to control startup login behavior; defaults to true only when username and password are configured
- `SLSKR_RECONNECT` to reconnect after session I/O failure; defaults to the auto-connect value
- `SLSKR_RECONNECT_SECONDS` for reconnect backoff; defaults to `30`
- `SLSKR_PING_SECONDS` for daemon keepalive pings; defaults to `300`
- `SLSKR_LISTENER_BIND` to enable and bind the regular peer listener, for example `0.0.0.0:2234`
- `SLSKR_ADVERTISED_PORT` for the public regular peer port advertised to the server
- `SLSKR_OBFUSCATED_LISTENER_BIND` to enable and bind the type-1 obfuscated peer listener
- `SLSKR_OBFUSCATED_ADVERTISED_PORT` for the public obfuscated peer port advertised to the server
- `SLSKR_USER_INFO_DESCRIPTION` for the minimal `UserInfoResponse` profile text; defaults to `slskr daemon`
- `SLSKR_PEER_RESPONSE_TIMEOUT_SECONDS` for listener-owned peer request handling; defaults to `5`
- `SLSKR_SHARE_DIRS` for semicolon-separated share roots. The daemon scans explicit roots into virtual `root/file` paths at startup.
- `SLSKR_SHARE_FOLLOW_SYMLINKS` to follow symlinks during share scans; defaults to `false`
- `SLSKR_SHARE_INCLUDE_HIDDEN` to include dot-prefixed path components; defaults to `false`
- `SLSKR_SHARE_SCAN_MAX_FILES` to cap startup scan size; defaults to `50000`
- `SLSKR_SHARE_FIXTURE` for temporary in-memory test entries as `path=size;path=size`
- `SLSKR_TRANSFER_HISTORY_LIMIT` for the in-memory transfer event history; defaults to `500`
- `SLSKR_TRANSFER_MAX_ACTIVE` for max concurrently active transfer records, including peer lookup/negotiation and inbound accepted transfers; defaults to `3`. Set to `0` to block transfer starts and inbound transfer acceptance.
- `SLSKR_TRANSFER_ALLOW_INBOUND` controls whether peer `TransferRequest`s for local shares are accepted; defaults to `true`.
- `SLSKR_TRANSFER_ALLOW_OUTBOUND` controls whether API-started peer transfers are allowed to begin; defaults to `true`.
- `SLSKR_API_TOKEN` for HTTP API auth. If set, protected API routes accept either `Authorization: Bearer <token>` or the dashboard's same-site `slskr.session` cookie.
- `SLSKR_API_RATE_LIMIT_ANONYMOUS` and `SLSKR_API_RATE_LIMIT_AUTHENTICATED` tune per-window HTTP API request limits; defaults are `1000` and `5000`.
- `SLSKR_AUTH_DISABLED` to explicitly disable HTTP API auth. Loopback-only binds default to disabled when no token is configured; non-loopback binds require a token unless this is set.
- `SLSKR_PERSISTENCE_ENABLED` enables the default-off SQLite persistence path. Search create/list hydration is wired; transfer projection state is also restart-safe through `transfer-state.json`. Message and room projection persistence remain event/projection work items.
- Spotify integration uses the existing slskR HTTP/WebUI port for OAuth callback handling. Configure `SLSKR_SPOTIFY_ENABLED=true` and `SLSKR_SPOTIFY_CLIENT_ID`; if `SLSKR_SPOTIFY_REDIRECT_URI` is unset, the daemon advertises `http://127.0.0.1:<http-port>/api/integrations/spotify/callback` for loopback use. The callback requires a daemon-issued cryptographically random state value, expires pending state after 10 minutes, and rejects replayed, missing, or invalid state.
- Lidarr does not provide an OAuth clickthrough surface. Configure `SLSKR_LIDARR_ENABLED=true`, `SLSKR_LIDARR_URL`, and `SLSKR_LIDARR_API_KEY`; the WebUI can then test status and run wanted/import actions using API-key authentication.
- `SLSKR_EXTERNAL_VISUALIZER_COMMAND` configures the optional local visualizer launch command; the daemon reports configured/disabled state before attempting launch.
- `SLSK_SERVER`, `SLSK_LISTEN_PORT`, `SLSK_USERNAME`, and `SLSK_PASSWORD` for the initial session scaffold
- gitignored `.secrets/` files for local lab credentials
- OpenBao paths documented under `../k3s/slskr/README.md`

Backfill target:

- expand `config.toml` coverage as API/auth/search/transfer modules land
- state directory for database; share cache currently writes `share-index.tsv`, transfer projection writes `transfer-events.tsv` with status and byte-progress events, and reloadable transfer records write `transfer-state.json`
- separate secret loading path for credentials
- maintained service/container artifacts that follow [install.md](./install.md) and never require checked-in secrets

## Auth And Exposure

Default `slskr serve` binds to loopback. Before exposing it outside localhost, add:

- local admin credential bootstrap
- explicit CORS defaults
- reverse-proxy guidance

Current behavior: bearer-token and same-site dashboard cookie auth are available for protected API routes. Auth is required automatically for non-loopback HTTP binds unless `SLSKR_AUTH_DISABLED=true` is set. When auth is enabled, unsafe API methods reject cross-site browser requests with foreign `Origin` or `Referer` headers. `GET /`, `GET /api/health`, `GET /api/version`, and `GET /api/v0/capabilities` remain public health/version/capability surfaces.

## Third-Party Authorization UX

Spotify is the only current integration in this group with a true OAuth clickthrough. slskR serves `/api/integrations/spotify/callback` on the same HTTP port as the WebUI/API, so operators do not need to expose a second listener. The WebUI shows the exact redirect URI to register in the Spotify developer dashboard. Authorization requests use a server-side state store with cryptographically random state, a 10-minute expiry, and single-use consumption before the callback is accepted.

Lidarr uses local API-key authentication rather than OAuth. The WebUI therefore presents a configure/test/sync flow: save URL and API key, check system status, preview wanted albums, then sync wanted items or request manual import.

## Packaging

Target distribution shape:

- one binary: `slskr`
- one config file
- one state directory
- maintained systemd unit
- optional container image
- no public product dependency on `slskr-cli`

## Remaining Backfill

- replace the current TSV share cache with the durable app database once the database choice is made
- finish `/api/v0/searches` parity beyond the current public-network dispatch, local share-backed projection, peer-response projection, and external response ingestion
- finish `/api/v0/users` browse parity beyond direct/indirect peer `GetShareFileList`/`FolderContentsRequest` execution, peer-address command hook, cache projection, failed/partial-state projection, and flattened browse-result ingestion
- finish durable `/api/v0/messages` and `/api/v0/rooms` persistence beyond the current server command/event projection
- expand `/api/v0/files/:root` beyond flat virtual files if folder/directory views become necessary
- expand `/api/v0/stats` from projection counts into durable health metrics once the database/event-log layout lands
- expand `/api/v0/metrics` with durable counters and process/runtime gauges once the database/event-log layout lands
- expand `/api/v0/telemetry` with process/runtime, database, and long-running task health once those subsystems land
- expand event coverage and topic mapping for `/api/events/ws` as the web UI moves more views to live updates
- broaden `/api/v0/transfers` real transfer execution with richer production policy; current implementation has reloadable projection state in `transfer-state.json`, restart-safe queued resume records for interrupted active transfers, local-path file metadata execution, configurable max-active transfer policy, inbound/outbound transfer allow switches, peer-message transfer negotiation, direct plain/obfuscated `F` streaming/resume, chunked progress events, requester-side indirect `F` fallback, inbound shared-file serving over direct or pierced `F` sockets, and live queued payload proof against adjacent daemons
- add durable config and state
- expand API route tests as new `/api/v0/*` resources land
- keep the Playwright/slskdN harness build strategy current as repo layouts move
- migrate scripts fully to `slskr` commands
- remove or hide the legacy `slskr-cli` binary before public release
