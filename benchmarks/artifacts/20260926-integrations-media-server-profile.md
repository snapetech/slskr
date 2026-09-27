# Integrations Media Server Base URL Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The Media Server base-URL field is warmed three times, then 20
controlled-input commits are recorded. Times are milliseconds. They measure
JSDOM commit work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 3.902 | 4.832 | 20 |
| Before extraction, run 2 | 3.704 | 4.181 | 20 |
| After extraction, run 1 | 3.996 | 5.000 | 20 |
| After extraction, run 2 | 3.650 | 4.227 | 20 |

Media Server adapter readiness, path diagnostics, sync preview, execution
contract, and their local state/actions moved into `MediaServerPanel.jsx`. The
root fell from 884 to 439 lines. Timing overlaps the baseline; no speedup is
claimed.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles media-server base-URL input render commits"
--disableConsoleIntercept`. All 24 focused Integrations tests passed. The Web
production build, targeted ESLint, and diff checks passed.
