# Web Ownership Inventory

This is an internal-only ownership decision record. It is not a performance or
coverage claim.

Inventory date: 2026-09-29
Reviewed on top of commit: c76a255d

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
- `dashboard/src/context/ApiContext.tsx` owns the dashboard API URL and key.
  `AppContent` reads that context and passes values as page props; pages do not
  maintain a competing copy. This keeps request ownership explicit and pages
  straightforward to render in isolated tests.

## Retired legacy ownership

- RF-064 removed the unreferenced `web/src/components/Pods/Pods.jsx`, its
  route-only stylesheet and tests, plus `VpnGatewayConfig.jsx`, which had no
  other import. The production `/pods` route already rendered `Messaging.jsx`;
  the active workspace retains pod-list/detail/message error handling and
  request-cancellation regressions. The `PortForwarding` compatibility import
  and its independently tested implementation remain.
- `web/eslint.config.mjs` disables `no-unused-vars` globally. This is a
  tooling debt item, not evidence that a particular module is unused; future
  tightening must be incremental and test-backed.

The executable inventory gate is `scripts/check-web-ownership-inventory.sh`.
