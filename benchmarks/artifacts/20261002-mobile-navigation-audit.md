# Mobile navigation and accessibility follow-up (2026-10-02)

Internal verification record for the mobile navigation and semantic UI fixes.
The browser harness used deterministic mock responses. The receipt identifies
source commit `748e0a469d80708634e374769552e569f11d574b` and marks the worktree
dirty, so this is local evidence for the current changes, not hosted CI or
deployed-build evidence.

## Browser results

`scripts/run-react-nightly-audit.py` completed successfully in real Chromium.
Its success scenario checked 42 routes at desktop and mobile sizes (84 rendered
views), all with HTTP 200 documents and no route-overlap findings. It observed
2,342 mocked UI API responses, with no audit errors or live endpoint sweeps.
The loading/empty, validation/server-error, and
authorization/reconnect/restart scenarios also passed with 10, 15, and 39
mocked UI API responses, respectively. Their hashes and scenario metadata are
in [`the machine receipt`](20261002-mobile-navigation-audit-receipt.json);
the detailed per-view responses are in [`the browser audit`](20261002-mobile-navigation-browser-audit.json).

At a 390 by 844 mobile viewport, primary links now form a single horizontally
scrollable row with session controls below it. The E2E regression asserts that
the navigation is at most 130 pixels tall, the active route link is fully
visible (including after a direct `/users` load), and the `main` landmark and
Users page heading are present. Full-page Axe reports zero violations after
the app's initial theme transition settles. The captured state is
[`Users on mobile`](20261002-mobile-navigation-users-mobile.png).

## Local validation

- Web unit suite: 149 files and 962 tests passed.
- Core-page Playwright suite: 5 tests passed, including the mobile Axe check.
- ESLint, production build, build-output verification, release-note tests, and
  the full bug-council phase runner passed.
- Bundle budget: 1,087.34 KiB initial JavaScript and 598.18 KiB JavaScript gzip,
  within the 1,150 KiB and 600 KiB limits.

The work adds `release-notes/20261002-mobile-navigation-layout.md`. The
release-note preview against `HEAD..WORKTREE` included the fragment.

## Responsive overflow follow-up

The browser harness now checks document width against the viewport and records
failed asset requests. Its successful follow-up run covered the same 42 routes
at desktop and mobile widths (84 views): zero horizontal document overflows,
zero asset failures, zero overlaps, and zero audit errors. It observed 2,342 UI
responses and passed the loading/empty, validation/server-error, and
authorization/reconnect/restart scenarios. The current receipt and route data
replace the initial audit files above.

An earlier full pass captured one `/system/events` desktop CSS preload failure
and the app's reloadable error screen. A focused desktop/mobile rerun and the
final full matrix passed; the first failure is retained alongside the passing
evidence in [`the initial failure report`](20261002-mobile-navigation-overflow-first-run-failure.json)
and [`its error-screen capture`](20261002-mobile-navigation-overflow-first-run-error-screen.png).
