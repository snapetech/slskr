# slskR Features

This is a user-facing map of the Rust daemon's current feature areas. The
[status guide](status.md) explains which features need configuration or an
external service. The [HTTP API](http-api.md), [OpenAPI document](openapi.json),
and [client libraries](CLIENT_LIBRARIES.md) document automation contracts.

## Soulseek client

- Connect to a Soulseek account, maintain session state, reconnect, and report
  listener status.
- Search globally, for a user, or in a room; inspect results and browse a
  user's shared files.
- Share configured local folders through a bounded index with filters and
  virtual paths.
- Queue and track downloads and uploads, including progress, cancellation,
  retries, and failure state.
- Exchange private messages, join rooms, watch users, and manage contact and
  note state.
- Use direct, indirect, and type-1 obfuscated peer paths where the peer and
  network support them.

## Web UI and media workflows

One `slskr serve` process hosts the API, event stream, and bundled React Web
UI. The UI includes search, transfers, uploads, rooms and messages, users,
contacts, browse, shares, collections, integrations, player controls, system
status, and telemetry.

The media and discovery surfaces include local and peer previews, library and
source views, recommendations, metadata-assisted searches, and listening-party
workflows. Their availability depends on the relevant local data, peer
availability, and configured integrations. See the
[app surface guide](app-surface.md) and [API feature guide](http-api-features.md).

## API and automation

- Versioned `/api/v0/*` routes, selected compatibility aliases, and a
  machine-readable OpenAPI contract.
- Bearer-token and `X-API-Key` authentication for protected routes, with
  read/write, read-only, and scoped now-playing credentials where configured.
- Bounded event polling and a WebSocket event feed.
- TypeScript, Python, Go, and Rust client surfaces.
- Webhooks and runnable examples for integrations.

See [HTTP API deployment](http-api-deployment.md) before exposing an API to
another network.

## Transfers and content discovery

Ordinary Soulseek downloads use the network's peer transfer behavior and
bounded retry/failover paths. Separately, the explicit multi-source HTTP swarm
API can use two or more HTTP range sources and a whole-file SHA-256 digest. It
assigns bounded chunks, retries failed chunks, verifies the result, and
publishes the confined output only after verification. This HTTP-range path
does not make ordinary Soulseek peers range-capable.

The daemon also has bounded content-hash and recording/peer indexes for
discovery workflows. Mesh previews and source-less swarm requests require
usable, verified metadata and eligible range sources; a metadata match alone
does not supply file bytes.

## Mesh, DHT, and pods

The native/current runtime includes Mainline DHT rendezvous, TLS mesh
transport, QUIC control/data paths, private service gateway support, and
selected pod services. The default peer-facing network layout shares one
numeric port across peer TCP and DHT/mesh/QUIC UDP traffic. Mesh services
require reachable peers and, for trusted content sources, operator-pinned peer
identity and endpoint data. See [Gold Star Club](gold-star-club.md) and
[Architecture](architecture.md).

Gold Star Club membership is opt-in in ordinary native/current operation. A
local member who leaves loses local Gold Star status permanently. Read the
[Gold Star Club guide](gold-star-club.md) before enabling it.

## Integrations

- Spotify authorization callbacks are served through the existing HTTP
  listener and use a daemon-issued, single-use state value.
- Lidarr status, wanted-item, and manual-import workflows use Lidarr's API
  key; they do not use OAuth.
- An optional local visualizer command can be enabled and its launch attempts
  are recorded as events.
- Other library, source, and recommendation views report unavailable or
  disabled states when their required configuration or data is missing.

Integration setup is documented in the
[install runbook](install.md#third-party-integrations).

## Data and operations

- The share catalog, transfer state, events, and runtime projections can use
  optional SQLite persistence. Persistence is disabled by default.
- Credential sources include memory, the operating-system credential store,
  a restricted local file, systemd credentials, environment variables, and
  protected config supplied by an operator.
- Health, version, capability, metrics, and telemetry endpoints support
  monitoring.
- Release archives include the daemon and browser assets for the documented
  platform matrix. See the [release runbook](release.md).
