---
category: fixed
audience: users, operators
area: web-ui
action: none
breaking: false
---
Following on from the Dropdown fix, a chain of downstream bugs it exposed are now fixed. The `semantic-ui-react` shim's `Modal` component — like the earlier `Dropdown` — never implemented interaction for its most common usage (a `trigger` element that opens it), and never rendered the `header`/`actions` shorthand props at all; a click on "Log Out" or any other trigger-based confirmation dialog silently did nothing. It now manages its own open state and renders shorthand actions/header. `/api/v0/share-grants/announce` was hardcoded to 404 (confirmed against the Rust test suite's own assertions) — it's now implemented, gated behind `SLSKDN_E2E_SHARE_ANNOUNCE=1` since the real transport for this is the Soulseek/mesh network, not a trusted HTTP push. Viewing a share you've received now correctly fetches its manifest, streams, and backfills directly from the owner's own node — which needed CORS (`SLSKD_WEB_CORS_*`) and a `connect-src` policy update to actually reach a different node's origin from the browser, consistent with the trust already extended to `ws:`/`wss:` connections. Also: stream access on a share was never actually denied when the grant's permissions didn't allow it (only download was checked); `ShareGrantRecord`'s JSON was missing a `collectionId` field several consumers expected.
