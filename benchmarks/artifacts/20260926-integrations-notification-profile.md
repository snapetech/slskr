# Integrations Notification Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The Pushbullet title-prefix field is warmed three times, then 20
controlled-input commits are recorded. Times are milliseconds. They measure
JSDOM commit work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 2.099 | 2.870 | 20 |
| Before extraction, run 2 | 2.121 | 2.840 | 20 |
| After extraction, run 1 | 2.210 | 2.858 | 20 |
| After extraction, run 2 | 2.243 | 3.109 | 20 |

Notification form state, apply/save actions, and UI moved into
`NotificationIntegrationsPanel.jsx`. The Integrations composition root fell
from 3,264 to 2,628 lines. Timing stayed within the observed JSDOM range, so no
rendering speedup is claimed for this ownership split.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles notification integration input render commits"
--disableConsoleIntercept`. All 19 focused Integrations tests passed. The Web
production build, targeted ESLint, and diff checks passed.
