# RF-059/RF-063 kspls0 deployed gzip and accessibility audit (2026-09-29 UTC)

Internal-only verification record. No release was published. Test credentials
and the temporary API token are intentionally omitted.

## Runtime and port layout

- Source revision: `b42e01c5b2ece09da1915a6ead0d6f31b8273ff2`.
- Image: `slskr-rf059:b42e01c5-trixie`, using its React bundle at
  `/usr/share/slskr/web/build`.
- The isolated process required API authentication, used in-memory credentials,
  disabled persistence, and mounted no host state or media directories.
- Listener inventory during the audit: TCP and UDP `0.0.0.0:44508` for the
  shared peer/mesh/DHT port, plus TCP `127.0.0.1:5030` for the UI. The optional
  HTTPS listener was disabled. No additional peer or DHT port was opened.

## Browser and accessibility results

Chromium audited the 15 routes below at desktop size 1440x1000 and mobile size
390x844 with device scale factor 2. The run used 150 ms network latency,
200,000 bytes/s download throughput, 93,750 bytes/s upload throughput, and 4×
CPU throttling.

| Routes, each at desktop and mobile sizes |
| --- |
| `/searches`, `/discovery-graph`, `/playlist-intake`, `/wishlist`, `/downloads`, `/uploads`, `/messages`, `/users`, `/contacts`, `/solid`, `/collections`, `/sharegroups`, `/shared`, `/browse`, `/system` |

All 30 document responses returned HTTP 200. The per-response CSP nonce matched
the `csp-nonce` metadata on every view. The run reported zero CSP console errors,
zero other browser errors, and zero axe violations for WCAG 2.0 A/AA, 2.1 A/AA,
and 2.2 AA. Axe reported 17 passing rules and one incomplete rule on each view.

Across the 30 navigations, each view loaded seven JavaScript requests with
347,827 transferred bytes and 1,112,351 decoded bytes. DCL ranged from 2,769 to
3,159 ms. The sanitized per-view measurements are retained in
[`20260929-rf063-kspls0-a11y-30-views.json`](20260929-rf063-kspls0-a11y-30-views.json).

A direct request for the production entry asset with `Accept-Encoding: gzip`
returned HTTP 200, `Content-Encoding: gzip`, `Vary: Accept-Encoding`, and a
55,952-byte body. The production bundle gate also passes at 1,086.28 KiB
initial JavaScript and 597.67 KiB aggregate JavaScript gzip against limits of
1,150 KiB and 600 KiB. The latter has 2.33 KiB of remaining budget.

The earlier pre-gzip browser baseline transferred 1,114,388 JavaScript bytes;
the current 347,827-byte route transfer is 68.8% lower. The earlier audit's
rough 9.17-second DCL is retained only as context because it covered four
routes, while this run covered 15.

## Restoration

The explicit stop marker triggered the deployment guard immediately after the
audit. The temporary container and remote env file were removed, the
`slskd.service` container returned to healthy status, and `/health` returned
HTTP 200. A process check found no temporary deployment guard or SSH tunnel.
