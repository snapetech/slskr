# Rust UI Parity Ledger

This ledger tracks the Rust/WASM UI against the current slskdN React UI. It is
intended to stay blunt: a route is not complete until the Rust page has the same
primary workflow, state handling, and route-specific actions as the React page.

Last audited: 2026-05-05.

## Current Status

Estimated completion: 55-65%.

Completed across the main Rust routes:

- native workflow pages exist for all main nav routes
- raw API cards and probe rows are hidden behind Developer by default
- bottom player no longer covers route controls in the audited layout
- native tables support filtering, sorting, row selection, bulk select, reset,
  persisted table state, an inspector, and keyboard navigation
- native workflow buttons resolve to real route actions where a backend action
  exists, falling back to visible status/toast feedback when no matching action
  exists yet
- native subpanels expose route-specific controls, fields, and status facts for
  deep workflows such as browse tabs, messaging rooms, share grants, and system
  operations
- primary native workspaces now show richer route structures for messaging,
  browse breadcrumbs/tree, collection item picking, share grants, and shared
  access manifests
- messaging, collections, share groups, shared-with-me, and browse workspaces
  now surface selected-row preview cards so selection changes expose the next
  likely action before opening the inspector
- upload, contact, Solid, shared-access, share-group, and system native buttons
  now resolve to route actions or explicit local workspace acknowledgements
- destructive native actions now use an in-app confirmation dialog so cancel,
  delete, deny, restart, shutdown, and vacuum flows stay inside the Rust shell
- native live-response parsing now handles dotted and indexed payload paths,
  array counts, string numeric/bool values, nested browse entries, and richer
  transfer progress details
- native row and bulk actions now prefer selected row data for files, users,
  contacts, collections, share groups, and share grants before falling back to
  generic form inputs
- System tabs now render route-specific operator tables across Info, Network,
  Mesh, Bridge, MediaCore, Security, Experience, Integrations, Options, Shares,
  Jobs, Automations, Providers, Analytics, Library Health, Quarantine, Files,
  Data, Events, Logs, and Metrics instead of generic placeholder facts

## Route Gaps

| Route | Current Rust Coverage | Remaining 1:1 Gaps |
| --- | --- | --- |
| Search | Query toolbar, grouped result rows, planner copy, filters, result inspector, search/download actions. | Full result expansion, search filter modal parity, exact ranking/duplicate controls, selected-result download body from real row data. |
| Discovery Graph | Seed inputs, graph labels, recommendations table, build graph action. | Canvas-level graph interaction, node inspector behavior, saved branches, weighted edge controls, recommendation queue behavior. |
| Playlist Intake | Paste/import shell, parsed row table, validation summary, preview action. | Upload/file import controls, organization plan detail, provider/MusicBrainz/SongID tabs, row-level correction workflow. |
| Wishlist | Wanted-search form/table, run/add actions, review summary. | Edit modal, quota portal behavior, discovery inbox bridge, per-row enable/auto-download toggles wired to persisted data. |
| Downloads | Active queue table, speed/slot summary, clear/download/acceleration actions. | Per-transfer retry/cancel/remove with exact selected transfer identifiers, grouped transfer rows, detailed progress/ETA controls. |
| Uploads | Upload queue table, allow/deny/policy shell, clear-completed action. | Real allow/deny backend mapping, per-peer grouping, policy editor parity, upload-specific selected-transfer identifiers. |
| Messages | Two-pane messaging shell, conversation table, reply/acknowledge/join actions. | Multi-window thread state, room/pod channel lists, unread/delete lifecycle, compose history, room create/join modals. |
| Users | Directory table, lookup/watch/note/browse/message actions. | Full selected user card, privileges/stats rendering, note modal, context menu parity, browse/message handoff. |
| Contacts | Contact table, invite/add/nearby shell, add contact action. | Invite QR flow, scan/upload invite, nearby contacts refresh behavior, groups/notes editor, remove/edit action wiring. |
| Solid | Solid status shell and WebID input. | Real WebID resolve/connect/session/sync flows, storage state rendering, related integration detail. |
| Collections | Collection list, create/add/share action mapping, item picker shell. | Create/share modals, item search result picker, remove item, audience picker, stream/download grant controls. |
| Share Groups | Group list, create/add-member/issue-token actions. | Selected group detail, member picker, grant list, token revoke/update permissions, per-row member removal. |
| Shared With Me | Inbound shares table, backfill/token/delete action mapping. | Open/stream selected item, manifest detail, copy exact token, owner/contact context, leave/revoke semantics. |
| Browse | Peer/folder inputs, browse/download actions, file table. | Full tabbed browse sessions, cached tree expansion, breadcrumbs, folder/file split, multi-select download body from selected files. |
| System | Operator dashboard shell, broad tabs, connect/disconnect/rescan/vacuum actions. | Full React System tab parity: Info, Network, Mesh, Bridge, MediaCore, Security, Experience, Integrations, Options, Shares, Jobs, Automations, Providers, Analytics, Library Health, Quarantine, Files, Data, Events, Logs, Metrics. |

## Acceptance Gates

- Headless route audit passes for every main nav route.
- No visible `GET /api/v0` text outside the Developer drawer.
- Every page has a route-specific heading, primary action, table/list, empty
  state, and inspector/detail surface.
- Every route-specific native action either performs the same backend request as
  React or is explicitly marked as unsupported in this ledger.
- Screenshots show the primary workflow in the first viewport on desktop and
  mobile.
