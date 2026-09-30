# RF-042 live WebSocket subscription proof (2026-09-29 UTC)

Internal-only verification record. No release was published. Test credentials and
the temporary API token are intentionally omitted.

## Runtime

- Source revision: `b42e01c5b2ece09da1915a6ead0d6f31b8273ff2`.
- Image: `slskr-rf059:b42e01c5-trixie` on kspls0.
- The isolated process used one authorized test account, required API
  authentication, an in-memory credential store, disabled persistence, and no
  mounted host state or media directories.
- The observed service listeners were TCP and UDP `0.0.0.0:44508` for the
  shared peer/mesh/DHT port and TCP `127.0.0.1:5030` for the loopback UI.
  `SLSKD_NO_HTTPS=true` kept the optional HTTPS listener closed.

## Subscription result

The TypeScript `WebSocketClient` connected using the configured bearer-token
subprotocol and subscribed to `search.started`. An authenticated test search
returned HTTP 200 and the client received an event with type `search.started`
and topic `searches`. The client then unsubscribed, issued a second test search,
and observed no additional matching event during the 900 ms observation window.
The script issued cleanup DELETE requests for both test search IDs before
disconnecting.

An earlier attempt to use `PATCH /api/options` received HTTP 403 because remote
configuration mutations were disabled in this isolated runtime. The proof used
search events instead and did not change persisted configuration.

Rust local controller tests separately cover WebSocket authentication and event
persistence, rehydration, and rollback. Persistence was disabled in this
temporary instance, so this live result does not claim deployed durability.

## Restoration

The bounded deployment guard removed the temporary container and secret env
file, restarted `slskd.service`, and checked the existing `/health` endpoint.
After the run, `slskd.service` was active, the slskd container reported healthy,
and `/health` returned HTTP 200. No temporary container, guard, or SSH tunnel
remained.
