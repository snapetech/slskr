# Web Ownership Inventory

This is an internal-only ownership decision record. It is not a performance or
coverage claim.

Inventory date: 2026-09-27
Source commit: 0dfab48cd16e6e7910759fa7d60e5d21b7b28be1

## Active ownership

- `web/src/components/AppContext.js` is active. `App.jsx` imports it and owns
  the provider around the routed application; it is not a dead module.
- `App.jsx` owns application authentication, shell state, and provider
  composition. It passes the token-check callback and profile state to
  `AppRouteTable.jsx`, which owns lazy page imports, route definitions, and
  route-miss diagnostics.
- `AppNavigationActivity.js` owns bounded room-activity storage, chat/room
  activity requests, and visibility-aware polling. `App.jsx` owns the visible
  `navActivity` state and supplies current path/authentication callbacks.
- `/pods` is declared in `AppRouteTable.jsx` and rendered by `Messaging.jsx`.
  The parameterized `/pods/:podId` and
  `/pods/:podId/channels/:channelId` paths redirect to `/messages`.
- `web/src/components/PortForwarding/PortForwarding.jsx` is a maintained
  compatibility import that re-exports the implementation owned by
  `web/src/components/Pods/PortForwarding.jsx`.

## Deferred legacy ownership

- `web/src/components/Pods/Pods.jsx` has no production route import. Its
  dedicated `Pods.test.jsx` coverage is retained while the old route-shaped
  implementation remains available for an explicit removal/migration change.
- The current decision is to defer deletion: removing the file in this
  refactor would discard legacy behavior and its regression surface without a
  replacement-coverage decision.
- `web/eslint.config.mjs` disables `no-unused-vars` globally. This is a
  tooling debt item, not evidence that a particular module is unused; future
  tightening must be incremental and test-backed.

The executable inventory gate is `scripts/check-web-ownership-inventory.sh`.
