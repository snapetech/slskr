# Integrations Federation Diagnostics Load Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. Each measured sample mounts the Integrations tree and waits for
the read-only Federation diagnostics load. Twenty update commits are recorded
after three warmups. Times are milliseconds. They measure JSDOM commit work,
not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 0.924 | 1.538 | 20 |
| Before extraction, run 2 | 0.896 | 1.367 | 20 |
| After extraction, run 1 | 0.751 | 1.316 | 20 |
| After extraction, run 2 | 0.808 | 1.606 | 20 |

The asynchronous Federation and Pod Diagnostics status owner moved into
`FederationDiagnosticsPanel.jsx`. The Integrations root fell from 3,915 to 61
lines. Median observations were lower after extraction; p95 varied, so no
deployed-browser speed claim is made.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles federation diagnostics load render commits"
--disableConsoleIntercept`. All 26 focused Integrations tests passed. The
notification apply assertion now waits for the async UI message. Integrations
ESLint, the Web production build, and diff checks passed.
