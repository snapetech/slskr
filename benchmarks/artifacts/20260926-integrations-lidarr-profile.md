# Integrations Lidarr Import Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The manual import-directory field is warmed three times, then 20
controlled-input commits are recorded. Times are milliseconds. They measure
JSDOM commit work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 1.214 | 1.622 | 20 |
| Before extraction, run 2 | 1.238 | 1.795 | 20 |
| After extraction, run 1 | 1.206 | 1.329 | 20 |
| After extraction, run 2 | 1.228 | 1.404 | 20 |

Lidarr status, wanted sync, import history/retry, and manual import state and
actions moved to `LidarrPanel.jsx`. The root fell from 1,235 to 884 lines.
Median timing stayed similar; p95 was lower in the two after runs.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles Lidarr import-directory input render commits"
--disableConsoleIntercept`. All 23 focused Integrations tests passed. The Web
production build, targeted ESLint, and diff checks passed.
