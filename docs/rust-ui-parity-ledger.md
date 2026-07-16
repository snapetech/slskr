# Rust UI Parity Ledger

This ledger tracks the Rust/WASM UI against the current React UI. It is
intended to stay blunt: a route is not complete until the Rust page has the same
primary workflow, state handling, and route-specific actions as the React page.

Last audited: 2026-07-15.

## Current Status

Estimated completion: 95-98%.

The 2026-07-15 headless audit passed all 15 main routes on desktop and mobile,
including mocked daemon responses, keyboard row selection, selected-row
actions, confirmation flows, hidden Developer surfaces, bottom-player spacing,
and browser console/error checks. The percentage remains below 100 because the
route-specific live-backend validations listed below have not all been run.

The Rust UI now has human-usable workflow parity across the audited slskdN page
set. The remaining work is no longer generic workbench removal or missing page
surface; it is live-backend behavioral validation for edge cases that require a
running Soulseek session, real peers, real transfer state, Solid auth, or real
share grants.

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
- Browse now exposes tabbed peer sessions, cached folder state, breadcrumb
  controls, file filtering, refresh-folder controls, and a selected-file
  download manifest so the page behaves closer to the expected session browser
- Messages now exposes explicit conversation lifecycle state, room and pod side
  state, selected-thread metadata, transcript preview, message actions, search,
  and compose history inside the native two-pane messenger
- Wishlist, Users, Contacts, Collections, Share Groups, Shared With Me, and
  System now expose native editor/modal surfaces for the route-specific create,
  edit, note, audience, grant, token, inbound-access, and settings workflows
  that were previously scattered across table buttons and side panels
- Native rows now expose structured route data attributes for filenames, peers,
  usernames, contacts, collections, share groups, owners, permissions, browse
  paths, transfer states, and system areas. The WASM action resolver uses those
  values before generic fallbacks so row actions can build closer selected-item
  payloads across search, transfers, contacts, sharing, messaging, and browse.
- Native action path resolution now separates selected route targets from
  selected item IDs. Wishlist runs, transfer cancel/allow/deny, contact remove,
  share-group member edits, and share-grant update/token/backfill/delete actions
  can use live row IDs instead of hardcoded demo IDs when API rows provide them.
- Selected rows now carry route-specific action summaries, and the Rust runtime
  uses those summaries in the selection status, inspector, and preview cards so
  users see the concrete operation target before firing row actions.
- Selected rows now also carry route-specific field summaries for every
  workflow route. The inspector and preview cards expose human labels like peer,
  queue, progress, permissions, owner, path, and next action after selection.
- Selected rows now carry route-specific context action menus. The inspector
  swaps its generic buttons for the selected row's Search, transfer, message,
  user, browse, collection, share, and system actions.
- Native tables now render route-specific row state controls for high-traffic
  parity gaps: transfer rows show progress/ETA controls, wishlist rows expose
  enabled and auto-download toggles, inbound shares show permission actions,
  share groups expose grant/token controls, and browse rows distinguish folder
  open from file queue actions.
- Search now exposes the old filter-modal controls inline in the native Rust
  workspace: include/exclude terms, format, bitrate, size, duration, queue,
  duplicate folding, free-slot preference, locked-file filtering, ranking
  profile, result expansion, and ranking/duplicate chips.
- Every native route now includes a route-specific final parity panel for the
  remaining old WebUI sub-workflows: discovery graph canvas, playlist upload and
  correction, wishlist quota/inbox, grouped transfers, upload policy,
  multi-window messaging, selected-user context cards, contact QR invites, Solid
  session/storage setup, collection share drafts, share-group member/token
  mutations, inbound share manifests, cached browse sessions, and operator tabs.

## Route Closure

| Route | Rust parity status | Residual validation |
| --- | --- | --- |
| Search | Query toolbar, grouped result rows, result expansion controls, inline filter-modal controls, ranking profile, duplicate folding, planner copy, download preview, selected row inspector, and structured peer/file action payloads. | Validate ranking and duplicate folding against a live result set with multiple peers/providers. |
| Discovery Graph | Seed inputs, graph canvas parity panel, node/recommendation inspector, weighted-edge controls, saved-branch controls, acquisition profile actions, and recommendation queue controls. | Validate graph persistence with live discovery-graph responses. |
| Playlist Intake | Paste/upload shell, parsed row table, row correction controls, import validation, provider/MusicBrainz/SongID supporting tabs, and acquisition plan queue controls. | Validate file upload and provider enrichment against live provider fixtures. |
| Wishlist | Wanted-search table/form, enabled and auto-download row toggles, quota portal summary, persisted discovery inbox controls, run/add/import actions, and review copy surface. | Validate quota counters and persisted inbox replay against daemon state. |
| Downloads | Active/queued/completed/failed tabs, grouped transfer controls, progress meter, ETA/speed/slot controls, retry/cancel/remove, acceleration, and selected transfer action payloads. | Validate transfer grouping against live multi-peer transfers. |
| Uploads | Upload queue tabs, per-peer grouping controls, upload policy editor, allow/deny/clear controls, progress/ETA state, and selected upload action payloads. | Validate policy edits against live upload requests. |
| Messages | Two-pane messenger, conversation search, thread transcript, unread/delete lifecycle controls, compose history, rooms/pods side state, room create/join/leave controls, and selected-thread actions. | Validate draft restoration and unread counts against live conversations. |
| Users | User directory, selected user card, status/privilege/stat labels, context menu controls, note/watch editor, browse/message handoff, and selected-user action payloads. | Validate live privilege/stat rendering with real user info/status endpoints. |
| Contacts | Contact manager, groups/nearby/invites tabs, QR invite create/scan controls, notes/group editor, message/browse/watch/remove actions, and selected-contact payloads. | Validate QR scan/upload with browser image APIs and real invite payloads. |
| Solid | Solid-specific identity/status shell, WebID resolve, session/connect controls, storage root state, linked-data sync controls, and related integration detail. | Validate auth/session transitions against a real Solid provider. |
| Collections | Collection library, selected collection detail, item picker, persisted create/share draft controls, remove item action, audience picker, stream/download grant controls, and selected collection payloads. | Validate live library item search and grant mutations against daemon data. |
| Share Groups | Group list, selected group detail, member picker, add/remove member controls, token issue/revoke, grant mutation controls, permission matrix, and selected grant/group payloads. | Validate revoke/update/member removal against live share-group records. |
| Shared With Me | Inbound grants/tokens table, manifest preview, owner/contact context, open/stream/backfill/copy token/leave controls, permission state, and selected grant payloads. | Validate exact token copy and stream/open behavior against live inbound grants. |
| Browse | Peer browser, tabbed sessions, cached tree state, breadcrumb persistence controls, folder expansion, file/folder split, file filter, multi-select manifest, and queue selected action payloads. | Validate cache restore against live browse status and folder payloads. |
| System | Operator dashboard with full React tab parity across Info, Network, Mesh, Bridge, MediaCore, Security, Experience, Integrations, Options, Shares, Jobs, Automations, Providers, Analytics, Library Health, Quarantine, Files, Data, Events, Logs, and Metrics. | Validate every operator action against a live daemon with non-empty telemetry/jobs/shares. |

## Acceptance Gates

- Headless route audit passes for every main nav route. Enforced by
  `node scripts/audit-rust-web-ui.mjs`, which builds the Rust/WASM bundle,
  serves it with mocked daemon responses, visits every main route on desktop
  and mobile, captures screenshots in `target/ux-audit/`, selects a workflow row
  by keyboard, exercises a selected-row action, and fails if primary workflow,
  inspector/detail, action feedback or confirmation, hidden Developer drawer,
  bottom-player spacing, selectable live rows, or browser-error checks regress.
- No visible `GET /api/v0` text outside the Developer drawer.
- Every page has a route-specific heading, primary action, table/list, empty
  state, and inspector/detail surface.
- Every route-specific native action either performs the same backend request as
  React or exposes a route-local acknowledgement where React also has no daemon
  mutation.
- Screenshots show the primary workflow in the first viewport on desktop and
  mobile.
