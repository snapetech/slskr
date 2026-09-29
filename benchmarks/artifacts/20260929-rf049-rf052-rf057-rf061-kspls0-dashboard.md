# RF-049/RF-052/RF-057/RF-061 kspls0 dashboard browser audit (2026-09-29)

Internal verification record. The test account and temporary API token are
omitted. No release was published.

## Runtime and port layout

- Backend image source: `5dbd1f43904b942dcb262f2a8f088afedf33b4f5`.
- The dashboard build was served through the same SlskR HTTP listener using a
  read-only static-directory override. Its `index.html` SHA-256 was
  `b25aa45eb340ad16b5ffab884893f454b5805b85fc4fd300855a0699d8db1a44`.
- The container used a read-only root, in-memory Soulseek credentials, a
  temporary state directory, and no host media or database mounts.
- Peer TCP and UDP shared `0.0.0.0:44508`; HTTP stayed on
  `127.0.0.1:5030`. HTTPS was disabled. No `5031` or `50300` listener appeared.
- A ten-minute automatic restore guard was armed before replacing slskd.

## Browser results

Chromium audited `/`, `/database`, `/configuration`, `/webhooks`, `/api-keys`,
and `/monitoring` at 1440x1000 and 390x844. All 12 document responses returned
HTTP 200; each route heading mounted and each required read-only API request
returned HTTP 200. The test account reported connected.

The health endpoint was then forced to return HTTP 503 twice. The dashboard kept
its last known-good view and showed the degraded-connection alert. With the
document visibility state set to hidden, health and stats request counts each
remained unchanged for longer than one poll interval. Returning to visible
issued exactly one health request and one stats request. Both controlled 503
console messages were expected; there were no other console or page errors.

Axe ran the WCAG 2.0 A/AA, 2.1 A/AA, and 2.2 AA rules on all 12 views and found
zero violations. This covered the built Tailwind v4 styles, compact mobile
navigation, contrast fixes, and dashboard target sizes.

## Restoration

The restore script removed the temporary container, imported image, static
assets, and remote credentials, then restarted `slskd.service`. Verification
found the original `ghcr.io/snapetech/slskdn:2026090321-slskdn.320` container
healthy, with its original listener layout restored. The loopback SSH tunnel,
local test credentials, and local temporary image were also removed.
