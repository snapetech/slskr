# Configuring slskR

The authoritative option list is the annotated
[TOML example](slskr.config.example.toml). This page explains the configuration
sources, the settings most operators need first, and the native/current network
defaults.

## Config file and environment

The default config path is `$XDG_CONFIG_HOME/slskr/config.toml`, or
`$HOME/.config/slskr/config.toml` when `XDG_CONFIG_HOME` is unset. Set
`SLSKR_CONFIG=/path/to/config.toml` to select another file.

The default state directory is `$XDG_STATE_HOME/slskr`, or
`$HOME/.local/state/slskr` when `XDG_STATE_HOME` is unset. Override it with
`SLSKR_STATE_DIR=/path/to/state`. On Unix the state path must be a real
directory, not a symlink; slskR restricts its permissions because it contains
private messages, transfer records, indexes, database files, and mesh identity
material.

Environment variables override config-file settings. The README lists common
variables; the annotated TOML example gives the exact table and field names.
Keep secrets in a secret manager, protected systemd credentials, or another
operator-controlled secret source.

## First settings to choose

| Need | Setting |
| --- | --- |
| Change the HTTP UI/API bind | `[app].http_bind` or `SLSKR_HTTP_BIND`; default `127.0.0.1:5030`. |
| Select a share root | `[shares].dirs` or `SLSKR_SHARE_DIRS`; the environment form uses semicolons between paths. |
| Connect on startup | `[app].auto_connect` or `SLSKR_AUTO_CONNECT`. |
| Choose credential storage | `[network].credential_store` or `SLSKR_CREDENTIAL_STORE`; see [Credential storage](credential-storage.md). |
| Protect API routes | Set `[auth].api_token` or `SLSKR_API_TOKEN`; use distinct scoped tokens where needed. |
| Persist runtime projections | Set `[persistence].enabled = true` or `SLSKR_PERSISTENCE_ENABLED=true`. |
| Change the peer port | `[network].listen_port` or `SLSK_LISTEN_PORT`; see the shared-port rule below. |

The supplied example intentionally uses peer port 2234. A new configuration
with native/current defaults uses peer port 50300.

## Native/current networking

The HTTP UI/API listener remains separate from the peer-facing network port.
Within native/current peer networking, the regular Soulseek listener also
handles type-1 obfuscation and mesh TLS. DHT, mesh control, and QUIC share the
same numeric port on UDP. Changing the peer port projects that number across
these peer services; it does not create one dedicated port per service.

Separate native/current obfuscation or overlay bind/advertised ports are
rejected when they do not match the peer endpoint. Do not configure a second
public peer port for those services. The frozen compatibility profile preserves
historical behavior for differential tests and is not the normal deployment
profile. See [Architecture and network layout](architecture.md).

## Credentials and API access

For an interactive install, the Web UI can store Soulseek credentials in
process memory, the operating system credential store, or a restricted local
credential file. Linux services can read systemd credentials. Environment or
protected config values are also supported for container secret managers.

Loopback-only HTTP binds may run without API authentication when no API token
is set. Non-loopback binds require an API token unless authentication is
explicitly disabled; keep authentication enabled for exposed deployments.
Protected routes accept bearer authentication and `X-API-Key`. Read-only,
read/write, and now-playing scoped tokens can be configured separately. See
[HTTP deployment](http-api-deployment.md).

## Profiles and optional features

The native controller profile is the normal application mode. Compatibility
profiles are retained for frozen differential testing and may preserve
historical defaults. Avoid setting a compatibility profile unless you are
running a compatibility harness.

SQLite persistence is off by default. Enabling it makes supported runtime
projections durable; it does not change the state directory or remove the
existing bounded file-based state used by specific subsystems.

Spotify and Lidarr settings are under `[integrations.spotify]` and
`[integrations.lidarr]`. Both remain inactive until configured and enabled.
The Spotify OAuth callback uses the existing HTTP listener; Lidarr uses its
API key. More optional settings and compatibility notes are in the annotated
[config example](slskr.config.example.toml).
