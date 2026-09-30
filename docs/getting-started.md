# Getting Started With slskR

This guide takes a new operator from a release archive to a first Soulseek
search. For systemd, containers, Kubernetes, and network exposure, continue
with the [install runbook](install.md) and
[HTTP deployment guide](http-api-deployment.md).

## 1. Install slskR

Download the archive for your operating system and CPU from
[GitHub Releases](https://github.com/snapetech/slskr/releases). Releases
currently provide Linux x86-64 and AArch64 (GNU and musl), macOS Intel and
Apple silicon, and Windows x86-64 archives.

Extract the archive and make the binary available as `slskr` on your PATH.
Check it starts:

```sh
slskr version
```

The release archive includes the browser assets. If you build from a source
checkout, see [Install and service runbook](install.md); a source build without
the React assets serves the built-in operator dashboard instead.

## 2. Start the daemon

```sh
slskr serve
```

Open <http://127.0.0.1:5030/> in a browser. The HTTP listener is loopback-only
by default. The full Web UI and HTTP API use this same listener.

On first use, enter your Soulseek username and password in the Web UI. Choose
where credentials live: temporary process memory, the operating system
credential store, or the restricted local credential file. For a service,
systemd credentials or a secret manager can supply credentials without putting
them in a normal config file. See [Credential storage](credential-storage.md).

## 3. Connect and set up shares

Use the Web UI to connect the Soulseek session. To connect automatically after
restart, enable `auto_connect` in the app settings or set
`SLSKR_AUTO_CONNECT=true`.

Set share roots in `[shares].dirs` in the TOML config, or use
`SLSKR_SHARE_DIRS` with semicolon-separated paths. The service account must
be able to read those directories. slskR publishes virtual share paths; it does
not expose the host's absolute paths through the share catalog.

The annotated [configuration example](slskr.config.example.toml) shows the
available options. Defaults and config/state paths are explained in
[Configuration](configuration.md).

## 4. Run a search and download

Open Search in the Web UI, enter a query, and select a target. Results show
available peer and file details. Choose a result to queue a download; use
Downloads to follow its queue, progress, retry, completion, or error state.

The HTTP API and language clients expose the same major workflows for scripts.
Start with the [HTTP API reference](http-api.md), the
[OpenAPI contract](openapi.json), or [client libraries](CLIENT_LIBRARIES.md).

## Network ports

The Web UI/API and Soulseek peer traffic have separate application listeners.
In the native/current profile, peer-facing Soulseek, type-1 obfuscation, and
mesh TLS use the shared peer TCP listener. DHT, mesh control, and QUIC share
the same numeric port on UDP; they do not add a separate public port per
service.

| Purpose | Default | Notes |
| --- | --- | --- |
| Web UI and HTTP API | `127.0.0.1:5030/tcp` | Keep loopback-only unless remote access is needed. |
| Peer traffic | `50300/tcp` and `50300/udp` | Native/current mode uses one peer port number across TCP and UDP. A custom peer port is projected across these services. |

Forward the peer port number for both TCP and UDP only when your network needs
inbound peer access. Do not expose the HTTP listener directly to the public
internet; use an authenticated reverse proxy when remote browser/API access is
needed. See [Architecture and network layout](architecture.md) and
[HTTP deployment](http-api-deployment.md).

## Next steps

- Review the [feature guide](FEATURES.md) and
  [current status and limits](status.md).
- Choose the credential method in [Credential storage](credential-storage.md).
- Set up a long-running service with the [install runbook](install.md).
- If something does not work, use [Troubleshooting](troubleshooting.md).
