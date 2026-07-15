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

Smoke/probe command implementations live inside the `slskr` binary crate, and public docs and scripts should call `slskr` directly.

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

The daemon calls `slskr-client` for protocol/runtime behavior rather than duplicating connection logic in the app crate. It can start a real server login session from Web UI-supplied credentials, stored credentials, environment variables, or a protected config file.

## Mesh, Pod, And Service-Fabric Compatibility

`slskr` is designed to preserve compatibility-oriented operating surfaces while using an independent Rust daemon and protocol runtime. The compatibility target is the externally visible behavior operators depend on: mesh-network state, pod/service-fabric deployment shape, API and WebUI workflows, event feeds, transfer/search/browse/message/room behavior, player controls, integration affordances, and network-health reporting.

The current app surface preserves the following compatibility concepts:

- mesh and service-fabric status projections for DHT, mesh peers, hashes/catalog sequence state, sync/backfill activity, swarm jobs, transfer rates, and NAT/overlay health
- one pod-friendly daemon process that owns the Soulseek session, HTTP API, WebUI, event stream, static assets, share scanner, transfer engine, runtime telemetry, and integration callbacks
- versioned `/api/v0/*` APIs plus selected compatibility aliases for existing client and automation habits
- WebUI surfaces for search, transfers, uploads, rooms, private messages, users, contacts, browse, collections, share groups, shared-with-me views, playlist intake, integrations, system state, and player/media tools
- live interoperability scripts and fixtures that compare slskr behavior against slskd, Soulseek.NET-family clients, and other compatible runtimes

This is a behavioral compatibility commitment rather than a source-code lineage statement. slskr should interoperate with existing deployment and client habits, but implementation details remain Rust-native and separately maintained.

Compatibility acknowledgements are explicit where a preserved route is not a
durable slskr feature. Options/config mutation aliases validate request shape
and report `runtimeMutationEnabled: false` / `configPersisted: false`; when
SQLite persistence is enabled their acknowledgement counters also update the
durable runtime compatibility state. Bridge start/stop/config aliases keep
stable response shapes while updating durable runtime compatibility state.
Profile invite, cache warm, backfill, SongID, and Lidarr operation counters
surface through the same persisted runtime state.
Pending OAuth callback states also write through to SQLite when persistence is
enabled so short-lived authorization flows survive daemon restarts until their
normal expiry. Webhook configuration and queued delivery log projections also
hydrate from and write through to SQLite when persistence is enabled.
Unconfigured Lidarr
wanted/sync/import aliases use local library state as a fallback. Logs,
destination validation, listening-party content helpers, share-grant
token/backfill helpers, profile updates, and MusicBrainz release-radar
subscription helpers now derive from or mutate local runtime stores. Collections,
collection items, library items, browse cache records, destination records,
now-playing records, wishlist items, contacts, share groups and members, share
grants, watched/user projection records, user notes, liked/hated interests, and
username/IP bans also write through to SQLite when persistence is enabled.

## Initial HTTP Surface

The first app shell exposes:

- `GET /`: bundled local dashboard for session, listener, share, search, browse, message, room, user, catalog, and transfer projections, with controls for session connect/ping/disconnect/privilege-check, starting/completing searches, watching/unwatching users, requesting browse, rescanning shares, queueing/updating transfer projections with explicit progress bytes, sending/acknowledging private messages, joining/leaving rooms, syncing the server room list, sending room messages with an explicit sender, filtering search/transfer/catalog/message/room/browse tables, refreshing projection tables, and using the configured API token as an in-memory browser bearer token for protected APIs
- `GET /api/health`: service health JSON
- `GET /api/v0/health`: versioned alias for service health JSON
- `GET /api/version`: client name and version JSON
- `GET /api/v0/version`: versioned alias for client name and version JSON
- `GET /api/v0/capabilities`: public capability summary for API, network, storage, and experimental surfaces
- `POST /api/v0/capabilities/negotiate`: compare requested app capabilities against the daemon's current capability set and return accepted/unsupported lists
- `GET /api/config`: sanitized daemon configuration; never includes credentials
- `GET /api/v0/config`: versioned alias for sanitized daemon configuration
- `GET /api/v0/stats`: compact aggregate counts for session, listeners, shares, searches, users, browse cache, messages, rooms, transfers, and durable database/projection health
- `GET /api/v0/metrics`: Prometheus-style text counters/gauges for session, listeners, shares, searches, users, browse cache, messages, rooms, transfers, runtime operations, and persisted SQLite row counts
- `GET /api/v0/telemetry`: protected JSON runtime health snapshot with sanitized config flags, listener/session state, storage status, database health, and projection counts
- `GET /api/v0/events`: bounded event log for recent search, transfer, share, user, browse, message, room, listener, relay, bridge, mesh, security, library, integration, player, telemetry, and session workflows. Supports `kind`, `topic`, `q`, `limit`, and `offset` query parameters. When persistence is enabled, event rows hydrate from and write through to SQLite.
- `GET /api/events/ws`: WebSocket event feed backed by the same event store and topic taxonomy as `/api/v0/events`. Frames include `topic`, `type`, structured `data`, and `timestamp`.
- `GET /api/shares`: sanitized share-index status, root labels, file/byte counts, per-root extension summaries, scan errors, SQLite persistence status, and compatibility cache status
- `GET /api/v0/shares`: versioned alias for share-index status
- `GET /api/v0/shares/catalog`: deterministic sorted share catalog with virtual path, size, extension, attribute count, file count, filtered count, and total bytes. Supports `q`, `prefix`, `extension`, `limit`, and `offset` query parameters.
- `GET /api/v0/files/:root`: list files and immediate directory summaries under one configured share-root label without exposing local host paths. Supports `q`, `folder`/`path`/`prefix`, `recursive`, `extension`, `limit`, and `offset` query parameters; the default flat root listing is preserved for compatibility.
- `POST /api/shares/rescan`: rebuild the in-memory share index from configured roots, replace the durable SQLite `share_files` snapshot when persistence is enabled, and rewrite the state-dir TSV compatibility cache
- `POST /api/v0/shares/rescan`: versioned alias for share rescan
- `GET /api/v0/application`: automation-compatible application state with version, server, share, room, and watched-user summaries.
- `GET /api/v0/server`: automation-compatible server connection state.
- `POST /api/v0/server` and `DELETE /api/v0/server`: aliases for session connect/disconnect.
- `GET /api/v0/session/enabled`: reports whether HTTP auth is enabled.
- `GET /api/v0/searches`: list search records as a slskd-compatible top-level array for automation clients. Each search retains at most 10,000 file results. Supports `q`, `status`, `target`, `limit`, and `offset` query parameters. When persistence is enabled, startup hydrates this projection from SQLite search rows plus persisted result rows, and lifecycle mutations, response ingestion, prune, update, delete, and clear operations write through to SQLite.
- `GET /api/v0/searches/records`: list search records with the slskr metadata envelope (`entries`, counts, pagination, and next token) for the dashboard and richer native callers.
- `POST /api/v0/searches`: create a search record from JSON body `{"query":"..."}` or automation-compatible `{"searchText":"..."}`, match against the current local share snapshot, optionally persist it to SQLite, and enqueue the public-network search command when connected. Optional `target` values are `global`, `user`, `room`, or `wishlist`; user searches require `username`, room searches require `room`, and `ttl_seconds` controls the active search expiration window.
- `GET /api/v0/searches/:token`: read one search record
- `POST /api/v0/searches/:token/complete`: mark one search record completed
- `POST /api/v0/searches/prune`: expire due active searches and remove expired records
- `POST /api/v0/search-responses`: merge one flattened result into a search record from JSON body with `token`, `filename`, `size`, and optional `peer_username`, `extension`, `slot_free`, `average_speed`, and `queue_length`
- `GET /api/v0/events`: list events as a slskd-compatible top-level array.
- `GET /api/v0/events/records`: list events with the slskr metadata envelope for dashboard filtering and pagination.
- `GET /api/v0/users`: list up to 4,096 watched/user projection records. When persistence is enabled, startup hydrates this bounded projection from SQLite.
- `POST /api/v0/users/watch`: watch a username from JSON body `{"username":"..."}`, write the projection to SQLite when persistence is enabled, and enqueue the server watch command when connected
- `DELETE /api/v0/users/:username/watch`: mark a user unwatched, write the projection to SQLite when persistence is enabled, and enqueue the server unwatch command when connected
- `POST /api/v0/users/:username/stats/request`: enqueue a server user-stats request when connected; matching server stats update the user projection with average speed, upload count, file count, and directory count, and write the projection to SQLite when persistence is enabled
- `GET /api/v0/browse`: list cached browse projections. Supports `q`, `status`, `limit`, and `offset` query parameters.
- `GET /api/v0/users/:username/browse`: read cached browse projection for one user; slskd-shaped browse and directory aliases include directory/file totals, filtered counts, byte totals, and pagination metadata
- `POST /api/v0/users/:username/browse/request`: mark browse requested and enqueue peer-address lookup when connected; when a matching peer-address response arrives, the app dials the peer directly or requests indirect peer-message connection fallback, sends `GetShareFileList`, parses the compressed share-list payload, updates the browse cache, and writes the projection to SQLite when persistence is enabled
- `POST /api/v0/users/:username/browse/folder`: mark a folder-content browse requested from JSON body `{"folder":"..."}` and enqueue peer-address lookup when connected; matching peer-address responses send `FolderContentsRequest` directly or through indirect peer-message fallback and update persisted browse projection state when enabled
- `POST /api/v0/users/:username/browse/fail`: mark browse failed with optional JSON body `{"reason":"..."}`; peer browse failures also project this state automatically and write through to SQLite when persistence is enabled
- `POST /api/v0/browse-responses`: merge flattened browse results from JSON body with `username` plus either one `filename`/`size`/optional `extension` or an `entries` array of those fields. Optional `complete:false` records the browse as `partial`; omitted or true promotes it to `ready`. Browse result projections write through to SQLite when persistence is enabled.
- `POST /api/v0/messages`: record an outbound private-message projection from JSON body with `username` and `body`, optionally persist it to SQLite, then enqueue the server private-message command when connected
- `POST /api/v0/messages/inbound`: record an inbound private-message projection and optionally persist it to SQLite
- `GET /api/v0/messages`: list message projections. Supports `q`, `username`, `direction`, `limit`, and `offset` query parameters.
- `GET /api/v0/messages/:username`: list message projections for one user. Supports `q`, `direction`, `limit`, and `offset` query parameters.
- `POST /api/v0/messages/:id/ack`: mark a message projection acknowledged, optionally persist the acknowledgement to SQLite, and enqueue the server acknowledgement when connected
- `GET /api/v0/rooms`: list up to 1,024 room projections, including joined state, room kind, user count, operated flag, message count, up to 10,000 projected members, and the latest 1,000 projected messages per room. Supports `q`, `joined`, `limit`, and `offset` query parameters.
- `POST /api/v0/rooms/refresh`: enqueue a server room-list request when connected
- `POST /api/v0/rooms/:room/join`: mark a room joined, optionally persist the subscription to SQLite, and enqueue the server room-join command when connected
- `DELETE /api/v0/rooms/:room/join`: mark a room left, optionally persist the unsubscribed state to SQLite, and enqueue the server room-leave command when connected
- `POST /api/v0/rooms/:room/messages`: record a room-message projection from JSON body with `username` and `body`, then enqueue the server room-message command when connected
- `GET /api/users/notes`, `POST /api/users/notes`, `GET/PUT/DELETE /api/users/notes/:id`: maintain user-note projections; create/update/delete operations write through to SQLite when persistence is enabled
- `GET /api/soulseek/interests`, `POST /api/soulseek/interests`, and `DELETE /api/soulseek/interests/:id`: maintain liked-interest projections used by recommendation compatibility routes, with SQLite write-through when persistence is enabled
- `GET /api/soulseek/hated-interests`, `POST /api/soulseek/hated-interests`, and `DELETE /api/soulseek/hated-interests/:id`: maintain hated-interest projections with SQLite write-through when persistence is enabled
- `GET /api/*/bans`, `POST /api/*/bans/username`, `DELETE /api/*/bans/username/:username`, `POST /api/*/bans/ip`, and `DELETE /api/*/bans/ip/:ip`: maintain local username/IP ban projections used by security dashboards, with SQLite write-through when persistence is enabled
- `GET/POST/PUT/DELETE /api/wishlist` item routes, `/api/wishlist/import/csv`, and release-radar/source-feed wishlist helpers: maintain wishlist projections with SQLite write-through when persistence is enabled
- `GET/POST/PUT/DELETE /api/contacts` plus discovery/invite contact helpers: maintain contact projections with SQLite write-through when persistence is enabled
- `GET/POST/PUT/DELETE /api/sharegroups` and `/api/sharegroups/:id/members` routes: maintain share-group and membership projections used by user-group compatibility routes, with SQLite write-through when persistence is enabled
- `GET/POST/PUT/DELETE /api/share-grants` plus collection lookup/token/backfill helpers: maintain share-grant projections with SQLite write-through when persistence is enabled
- `GET/POST/PUT/DELETE /api/nowplaying` plus listening-party content helpers: maintain current playback projections with SQLite write-through when persistence is enabled
- `GET /api/session`: current Soulseek session state, including last server message name and counters
- `GET /api/v0/session`: versioned alias for current session state
- `GET /api/listeners`: listener bind/local address, inbound connection counters, and redacted last inbound event
- `GET /api/v0/listeners`: versioned alias for listener state
- `GET /api/transfers`: current in-memory transfer history, SQLite transfer event trail status when persistence is enabled, and state-dir event log mirror status
- `GET /api/v0/transfers`: versioned alias for transfer history. Supports `q`, `status`, `direction`, `username`, `limit`, and `offset` query parameters.
- `GET /api/v0/transfers/downloads` and `GET /api/v0/transfers/uploads`: automation-compatible grouped transfer views by peer and directory.
- `POST /api/v0/transfers/downloads/:username`: enqueue one or more download records from a body containing `files`. When persistence is enabled, queued transfer records write through to SQLite.
- `POST /api/v0/transfers`: create a queued transfer projection from JSON body with `filename` and optional `direction`, `peer_username`, `local_path`, and `size`. When persistence is enabled, create/start/retry/progress/complete/cancel/delete/prune and replacement mutations write through to SQLite, transition/progress events append to SQLite `transfer_events`, and the reloadable transfer projection remains mirrored in `transfer-state.json`.
- `GET /api/v0/transfers/stats`: aggregate transfer projection counts and transferred bytes
- `POST /api/v0/transfers/:id/start`: mark a transfer in progress; when `local_path` is present without a peer, validate the file on disk and complete or fail the projection from real metadata; when `peer_username` is present, request peer-address metadata, negotiate a peer-message `TransferRequest`/`TransferResponse`, and use the direct `F` connection token/offset handshake for local-path upload/download streaming, preferring type-1 obfuscated `F` init when advertised and falling back to plain direct `F`; if direct `F` connect fails, request server-mediated `ConnectToPeer`/`PierceFirewall` and retry the same file stream over the indirect socket. Inbound peer transfer requests for locally indexed share files are accepted and served over incoming direct `F`, `PeerInit F`, or `PierceFirewall` file-transfer sockets.
- `POST /api/v0/transfers/:id/progress`: update transferred bytes from JSON body `{"bytes_transferred":123}`
- `POST /api/v0/transfers/:id/complete`: mark a transfer succeeded
- `POST /api/v0/transfers/:id/cancel`: mark a transfer cancelled with optional `reason`
- `POST /api/v0/transfers/:id/fail`: mark a transfer failed with optional `reason`
- `GET /api/v0/database/stats`, `/api/database/stats`, and `/api/admin/database/stats`: live persisted SQLite counts plus current projection counts for search, transfer, message, user, browse, room, social/security, wishlist/contact/sharegroup/share-grant, collection, library, destination, now-playing, webhook, and runtime compatibility stores
- `POST /api/v0/database/cleanup`, `/api/database/cleanup`, and `/api/admin/database/cleanup`: remove old persisted message rows using optional `{"days":30}` and prune terminal transfer projections from the reloadable transfer state
- `POST /api/v0/database/vacuum`, `/api/database/vacuum`, and `/api/admin/database/vacuum`: run SQLite `VACUUM` when persistence is enabled and return a structured skipped status otherwise
- `POST /api/session/connect`: start a session using Web UI-supplied credentials, stored credentials, or configured env/config credentials
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
- `SLSKR_AUTO_CONNECT` to control startup login behavior; defaults to true when username/password are configured or when a persistent Soulseek credential store is enabled
- `SLSKR_CREDENTIAL_STORE` for Soulseek credential storage: `os` uses the platform credential store, `systemd` reads `$CREDENTIALS_DIRECTORY` service credentials, `memory` keeps Web UI credentials runtime-only, and `file` uses the restricted local credential-file fallback
- `SLSKR_CREDENTIAL_FILE` for the `file` credential-store path; defaults to `soulseek-credentials.json` under `SLSKR_STATE_DIR`
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
- `SLSKR_SHARE_CACHE_TSV_ENABLED` to emit the `share-index.tsv` compatibility/debug mirror; defaults to `true`. SQLite `share_files` remains the durable share index when persistence is enabled.
- `SLSKR_SHARE_FIXTURE` for temporary in-memory test entries as `path=size;path=size`
- `SLSKR_TRANSFER_HISTORY_LIMIT` for the in-memory transfer event history; defaults to `500`
- `SLSKR_TRANSFER_MAX_ACTIVE` for max concurrently active transfer records, including peer lookup/negotiation and inbound accepted transfers; defaults to `3`. Set to `0` to block transfer starts and inbound transfer acceptance.
- `SLSKR_TRANSFER_ALLOW_INBOUND` controls whether peer `TransferRequest`s for local shares are accepted; defaults to `true`.
- `SLSKR_TRANSFER_ALLOW_OUTBOUND` controls whether API-started peer transfers are allowed to begin; defaults to `true`.
- `SLSKR_API_TOKEN` for HTTP API auth. If set, protected API routes accept `Authorization: Bearer <token>` and automation-compatible `X-API-Key: <token>`. Legacy same-site `slskr.session` cookie auth is accepted only when `SLSKR_API_COOKIE_AUTH_ENABLED=true`. Browser clients should keep tokens in memory or session storage rather than long-lived persistent storage.
- `SLSKR_API_RATE_LIMIT_ANONYMOUS` and `SLSKR_API_RATE_LIMIT_AUTHENTICATED` tune per-window HTTP API request limits; defaults are `1000` and `5000`.
- `SLSKR_AUTH_DISABLED` to explicitly disable HTTP API auth. Loopback-only binds default to disabled when no token is configured; non-loopback binds require a token unless this is set.
- `SLSKR_PERSISTENCE_ENABLED` enables the default-off SQLite persistence path. Share index, event log, search, transfer, user, browse, message, room, collection, library, destination, now-playing, wishlist, contact, sharegroup, share-grant, user-note, interest, security-ban, OAuth-state, webhook, and runtime compatibility projections write through to SQLite; transfer projection state also remains restart-safe through `transfer-state.json`.
- Spotify integration uses the existing slskr HTTP/WebUI port for OAuth callback handling. Configure `SLSKR_SPOTIFY_ENABLED=true` and `SLSKR_SPOTIFY_CLIENT_ID`; if `SLSKR_SPOTIFY_REDIRECT_URI` is unset, the daemon advertises `http://127.0.0.1:<http-port>/api/integrations/spotify/callback` for loopback use. The callback requires a daemon-issued cryptographically random state value, expires pending state after 10 minutes, and rejects replayed, missing, or invalid state.
- Lidarr does not provide an OAuth clickthrough surface. Configure `SLSKR_LIDARR_ENABLED=true`, `SLSKR_LIDARR_URL`, and `SLSKR_LIDARR_API_KEY`; the WebUI can then test status and run wanted/import actions using API-key authentication.
- `SLSKR_EXTERNAL_VISUALIZER_COMMAND` configures the optional local visualizer launch command. `SLSKR_EXTERNAL_VISUALIZER_LAUNCH_ENABLED=true` is also required before the daemon will spawn that command; launch attempts are recorded as events.
- `SLSK_SERVER`, `SLSK_LISTEN_PORT`, `SLSK_USERNAME`, and `SLSK_PASSWORD` for the initial session scaffold or env-backed secret-manager deployments
- local `.secrets/` files for lab credentials
- external secret-manager deployment notes are intentionally environment-specific; the
  maintained in-repo operator guidance is [install.md](./install.md) and
  [http-api-deployment.md](./http-api-deployment.md)

Backfill target:

- continue expanding `config.toml` coverage as compatibility and integration
  modules graduate from environment-only switches
- keep state directory ownership explicit; share index writes SQLite
  `share_files` plus optional mirrored `share-index.tsv` compatibility/debug
  cache, transfer projection writes SQLite `transfer_events` plus mirrored
  `transfer-events.tsv` status/byte-progress events, and reloadable transfer
  records write `transfer-state.json`
- keep Soulseek credential loading explicit through Web UI runtime entry,
  platform OS credential storage, read-only systemd credentials for Linux
  services, the restricted local credential-file fallback, environment
  variables, protected config files, or the operator's secret manager
- maintain service/container artifacts that follow [install.md](./install.md)
  and keep secrets in operator-controlled deployment storage

## Auth And Exposure

Current behavior: bearer-token and `X-API-Key` auth are available for protected API routes; legacy same-site dashboard cookie auth is disabled unless explicitly opted in. Auth is required automatically for non-loopback HTTP binds unless `SLSKR_AUTH_DISABLED=true` is set. When auth is enabled, unsafe API methods reject cross-site browser requests with foreign `Origin` or `Referer` headers. `GET /`, `GET /api/health`, `GET /api/version`, `GET /api/session/enabled`, and `GET /api/v0/capabilities` remain public health/version/capability surfaces.

Default `slskr serve` binds to loopback. Before exposing it outside localhost,
set `SLSKR_API_TOKEN`, keep auth enabled, preserve forwarded `Host`, `Origin`,
and `Referer` headers through your reverse proxy, and review
[install.md](./install.md) plus [http-api-deployment.md](./http-api-deployment.md).

## Third-Party Authorization UX

Spotify is the only current integration in this group with a true OAuth clickthrough. slskr serves `/api/integrations/spotify/callback` on the same HTTP port as the WebUI/API, so operators do not need to expose a second listener. The WebUI shows the exact redirect URI to register in the Spotify developer dashboard. Authorization requests use a server-side state store with cryptographically random state, a 10-minute expiry, single-use consumption before the callback is accepted, and SQLite hydration/write-through when persistence is enabled.

Lidarr uses local API-key authentication rather than OAuth. The WebUI therefore presents a configure/test/sync flow: save URL and API key, check system status, preview wanted albums, then sync wanted items or request manual import.

## Packaging

Target distribution shape:

- one binary: `slskr`
- one config file
- one state directory
- maintained systemd unit
- optional container image
- smoke/probe commands built into `slskr`

## Remaining Hardening

The large backfill phase is closed for the current cut: endpoint drift gates
report 290/290 canonical WebUI routes, the share TSV mirror is toggleable, the
event APIs share one topic taxonomy, and SQLite write-through/hydration covers
the current compatibility stores.

Open app-surface work is now production hardening rather than endpoint enumeration:

- decide when default-off SQLite persistence is ready to become the default,
  including migration/backfill policy, operator rollback guidance, and soak
  evidence for long-running state directories
- keep `/api/v0/searches`, browse, and transfer behavior aligned with live
  Soulseek peers as optional live interop runs expose edge cases that hermetic
  tests cannot reproduce
- deepen `/api/v0/files/:root` only when new storage-management workflows need
  mutation support; the current read-side folder views already provide
  summaries, folder selection, recursive listing, filtering, and pagination
  without exposing local host paths
- expand `/api/v0/stats`, `/api/v0/metrics`, and `/api/v0/telemetry` as new
  long-running task health signals are promoted into the daemon
- add route tests with each new `/api/v0/*` resource or compatibility mutation
  instead of treating route shape as the remaining parity problem
- keep the Playwright/slskr harness build strategy current as repo layouts move
