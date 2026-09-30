# slskR Troubleshooting

Start with the daemon log and the System status views in the Web UI. Protect
credentials and API tokens when sharing diagnostic output.

## The Web UI does not open

Check that the process is running and that the configured HTTP address is
reachable:

```sh
curl -i http://127.0.0.1:5030/api/health
```

If another process owns that address, select a different `[app].http_bind`
or `SLSKR_HTTP_BIND`. For a systemd user service, check
`systemctl --user status slskr.service` and
`journalctl --user -u slskr.service`.

Release archives contain the React Web UI. A source-built binary may show the
minimal dashboard when the production assets were not built or installed. Set
`SLSKR_WEB_BUILD_DIR` to the directory containing the expected `build`
assets, or use the source build instructions in the
[install runbook](install.md).

## API requests return 401 or 403

Protected routes need an API credential. Send it as either
`Authorization: Bearer <token>` or `X-API-Key: <token>`. Check that the
client uses the correct token role and that it has not been copied with extra
spaces or quotes. No shared default API password is provided.

Non-loopback binds require a configured API token unless authentication was
explicitly disabled. Keep authentication enabled when exposing the listener.
For browser requests behind a proxy, preserve the `Host`, `Origin`, and
`Referer` headers. See [HTTP deployment](http-api-deployment.md).

## The daemon does not connect to Soulseek

- Confirm the username and password in the selected credential source.
- If credentials were entered into process memory, re-enter them after a
  restart or select a persistent credential store.
- Check the configured Soulseek server address and outbound firewall policy.
- Review the startup log for the actual login or credential error.
- Run `slskr login smoke` with credentials supplied through your normal secret
  source to isolate account/session setup.

See [Credential storage](credential-storage.md) and the
[install runbook](install.md#first-run).

## Peers cannot reach this client

In native/current operation, Soulseek peer traffic uses one TCP listener and
the DHT, mesh-control, and QUIC services share that numeric port over UDP.
Forward the configured peer port for TCP and UDP when inbound peer access is
needed. Make the advertised port match the external mapping. Do not add a
separate native obfuscation, DHT, mesh, or QUIC public port.

The HTTP UI/API port is separate and does not carry peer traffic. Keep it
loopback-only or put it behind an authenticated proxy. Check listener and DHT
state in the System page or the relevant telemetry endpoints.

## A share or search result is missing

- Confirm the daemon account can read the configured share directory.
- Check share filters, hidden-file settings, symlink policy, and scan limit.
- Confirm the scan completed without root-specific errors, then request a
  rescan.
- Check that the session is connected and that the query target and search
  filters are what you intended.

Share APIs expose virtual paths rather than host filesystem paths.

## A transfer remains queued or fails

The remote peer may be offline, have no free slots, or have a long queue.
Review the transfer's latest state and failure details before retrying. A
normal Soulseek transfer is sequential and may fail over to another peer; the
HTTP multi-source swarm endpoint is a separate workflow that requires at least
two range-capable sources and a whole-file SHA-256 digest.

Check that the destination is writable and has enough space. For service
deployments, verify directory permissions and mounted paths from the service
or container's point of view.

## Mesh discovery or preview does not work

Mesh traffic depends on reachable peers and valid configuration. Verify the
reported transport endpoint, certificate fingerprint, and trusted peer record.
For content previews or source-less swarms, confirm that the metadata includes
the expected size and whole-file hash and that usable range sources exist.
Inspect the mesh and DHT status before changing port settings.

See [Architecture](architecture.md) and the annotated
[config example](slskr.config.example.toml).

## Spotify or Lidarr is unavailable

Spotify requires its integration to be enabled, a client ID, and a redirect
URI registered with the provider. Its callback runs on the existing HTTP
listener. Lidarr uses its configured URL and API key; it does not use OAuth.
Both integrations depend on their external service being reachable.

The install runbook has the supported
[integration settings](install.md#third-party-integrations).

## Collect useful diagnostics

Set `SLSKR_LOG_LEVEL=debug` for additional details, reproduce the issue, then
return to the normal log level. When asking for help, include the command,
slskR version, selected runtime profile, relevant status, and a short redacted
log excerpt. Remove credentials, tokens, private messages, and private
addresses first.

For a network or API contract issue, include the route or peer workflow and
the response/error shape. The [HTTP API reference](http-api.md) and
[live interop matrix](live-interop-test-matrix.md) describe deeper checks.
