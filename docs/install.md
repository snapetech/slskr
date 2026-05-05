# slskr Install And Service Runbook

This is the operator-facing shape for the bundled `slskr` app. It assumes one installed binary that runs the daemon, API, and web UI with `slskr serve`.

## Build

```sh
cargo build --release -p slskr
```

The binary lands at `target/release/slskr`. For a local user install:

```sh
install -Dm755 target/release/slskr "$HOME/.local/bin/slskr"
```

## Config And State

Default config file:

```text
$XDG_CONFIG_HOME/slskr/config.toml
```

If `XDG_CONFIG_HOME` is unset, the fallback is:

```text
$HOME/.config/slskr/config.toml
```

Default state directory:

```text
$XDG_STATE_HOME/slskr
```

If `XDG_STATE_HOME` is unset, the fallback is:

```text
$HOME/.local/state/slskr
```

Use `SLSKR_CONFIG=/path/to/config.toml` and `SLSKR_STATE_DIR=/path/to/state` to override those paths. Environment variables override config-file values.

Start from [slskr.config.example.toml](./slskr.config.example.toml). Keep credentials and API tokens out of git; use a local ignored env file, service environment file, or secret manager.

SQLite persistence is default-off. Enable the current durable search persistence path with `SLSKR_PERSISTENCE_ENABLED=true` or `[persistence].enabled = true`; transfer projection restart state is maintained in the slskr state directory.

## Third-Party Integrations

### Spotify OAuth

Spotify supports a browser clickthrough authorization flow. Configure:

```toml
[integrations.spotify]
enabled = true
client_id = "spotify-app-client-id"
# Optional; PKCE/browser authorization does not require a client secret.
# client_secret = "spotify-app-client-secret"
# Optional override. If omitted, slskr uses the existing WebUI/API port:
# redirect_uri = "http://127.0.0.1:5030/api/integrations/spotify/callback"
```

Register the exact redirect URI shown in the WebUI with the Spotify developer dashboard. slskr multiplexes the callback on the existing HTTP listener at `/api/integrations/spotify/callback`; no second callback service or port is required.

Security behavior:

- Authorization requests create a cryptographically random server-side state.
- Pending OAuth state expires after 10 minutes.
- State is single-use; replayed callbacks are rejected.
- Callbacks missing state, using invalid state, or using expired state are rejected.
- The callback response does not echo the authorization code.

### Lidarr

Lidarr does not provide OAuth. Configure its local URL and API key:

```toml
[integrations.lidarr]
enabled = true
url = "http://127.0.0.1:8686"
api_key = "lidarr-api-key"
```

The WebUI exposes the closest safe flow: Check Status, Load Wanted, Sync Wanted, and Manual Import actions over Lidarr's API-key authentication.

## First Run

Loopback-only HTTP binds default to no API auth unless `SLSKR_API_TOKEN` is configured. Non-loopback binds require an API token unless `SLSKR_AUTH_DISABLED=true` is explicitly set.

```sh
SLSKR_CONFIG="$HOME/.config/slskr/config.toml" slskr serve
```

Open:

```text
http://127.0.0.1:5030/
```

When API auth is enabled, enter the configured token in the dashboard's browser-session form. API clients can send the same token as:

```text
Authorization: Bearer <token>
```

## Systemd User Service

Example user unit:

```ini
[Unit]
Description=slskr daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=%h/.local/bin/slskr serve
Environment=SLSKR_CONFIG=%h/.config/slskr/config.toml
Environment=SLSKR_STATE_DIR=%h/.local/state/slskr
Restart=on-failure
RestartSec=5s
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
```

Place it at:

```text
$HOME/.config/systemd/user/slskr.service
```

Then:

```sh
systemctl --user daemon-reload
systemctl --user enable --now slskr.service
systemctl --user status slskr.service
```

## System Service

For a host-level service, create a dedicated user and keep config/state under `/etc/slskr` and `/var/lib/slskr`:

```ini
[Unit]
Description=slskr daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=slskr
Group=slskr
ExecStart=/usr/local/bin/slskr serve
Environment=SLSKR_CONFIG=/etc/slskr/config.toml
Environment=SLSKR_STATE_DIR=/var/lib/slskr
Restart=on-failure
RestartSec=5s
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/var/lib/slskr

[Install]
WantedBy=multi-user.target
```

If shared directories live outside `/var/lib/slskr`, add them to `ReadWritePaths=` or `ReadOnlyPaths=` as appropriate.

## Container Shape

The container should run the same app command:

```sh
slskr serve
```

Mount config read-only and state read-write:

```text
/config/config.toml -> SLSKR_CONFIG=/config/config.toml
/state             -> SLSKR_STATE_DIR=/state
```

Expose the HTTP bind only to the intended network. If exposing outside localhost, set `SLSKR_API_TOKEN`, keep auth enabled, and prefer a reverse proxy that preserves `Host`, `Origin`, and `Referer` headers.

Peer listener ports must match the configured advertised ports. For NAT-PMP/UPnP or VPN forwarded ports, set the advertised regular and obfuscated ports to the public mappings.

## Exposure Rules

- Default to loopback HTTP bind.
- Require `SLSKR_API_TOKEN` for non-loopback binds.
- Keep `GET /`, `GET /api/health`, `GET /api/version`, and `GET /api/v0/capabilities` public only as health/version/capability surfaces.
- Keep protected API routes behind bearer or same-site browser-cookie auth.
- Preserve same-origin headers through reverse proxies so cross-site mutating requests continue to be rejected.
- Do not check in credentials, WireGuard configs, NAT-PMP lease output, cookies, transfer state, share cache, or logs.
