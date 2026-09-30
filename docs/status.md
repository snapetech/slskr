# slskR Status And Support Boundaries

This page describes how to interpret the Rust port's current product surface.
It avoids treating a route name, compatibility response, configuration field,
or design note as proof that every matching workflow is available.

## Available for ordinary operation

The main daemon workflows are implemented in the Rust runtime:

- Soulseek login/session management and network health.
- Search, result review, user browse, shares, downloads, and uploads.
- Rooms, private messages, watched users, contacts, and notes.
- The bundled React Web UI, HTTP API, event polling/WebSocket feed, and client
  libraries.
- Local health, version, capability, metrics, and telemetry reporting.

Real network workflows still depend on valid account credentials, peer
availability, firewall/NAT configuration, and readable local share paths.

## Available with configuration or external services

These surfaces need extra operator setup or suitable external peers:

- Persistence through SQLite is optional and disabled by default.
- Spotify requires a registered application; Lidarr requires a reachable
  service and API key.
- Mesh content discovery requires consistent hash metadata and usable range
  sources. Trusted peers need operator-verified identity and endpoint data.
- Multi-source downloads use HTTP range sources with a whole-file SHA-256
  digest. Ordinary Soulseek transfers remain sequential peer transfers.
- Systemd credential loading, non-loopback API access, and reverse-proxy
  operation require the relevant service and network configuration.
- Gold Star Club enrollment is explicitly opt-in in native/current operation.

Read the linked setup guide before treating these as enabled merely because
their UI or API surfaces are present.

## Compatibility and test surfaces

slskR pursues behavioral compatibility with useful slskd/slskdN workflows but
has an independent Rust implementation and its own configuration and release
contracts. Compatibility routes may preserve familiar request/response shapes
without implementing every side effect of the reference application.

Some option and config mutation aliases validate requests and report
`runtimeMutationEnabled: false` and `configPersisted: false`; they are
compatibility acknowledgements, not remote configuration support. Remote
configuration is disabled by default. The legacy/frozen controller profiles
exist for differential compatibility work; ordinary installations should use
the default native/current behavior.

The public capability response at `GET /api/v0/capabilities` describes
capabilities recognized by the running daemon. It complements these docs; it
does not replace the specific setup requirements above.

## Network port boundary

The HTTP UI/API listener is a separate application listener, loopback-only by
default at `127.0.0.1:5030`. In the native/current network profile, Soulseek
peer and type-1 obfuscated traffic share the peer TCP listener. DHT, mesh
control, and QUIC use the same numeric port on UDP, so these services do not
require separate per-service port numbers. See
[Architecture and network layout](architecture.md).

## Release support

The release runbook defines the supported archive targets, CI proof, package
boundaries, and release-note rules. Do not infer a supported installer or
publication channel from slskdn's release list; use only the slskR channels
listed in the [release guide](release.md).

## Report a behavior gap

For an API contract, include the endpoint, request, response status, and a
redacted response body. For a network issue, include the selected profile,
reported listener state, and a redacted log excerpt. Do not attach credentials,
API tokens, raw private messages, or unredacted configuration.
