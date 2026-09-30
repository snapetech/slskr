# Integrations FTP Settings Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The FTP server address input is warmed three times, then 20
controlled-input commits are recorded. Times are milliseconds. They measure
JSDOM commit work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 1.478 | 1.751 | 20 |
| Before extraction, run 2 | 1.396 | 1.720 | 20 |
| After extraction, run 1 | 1.492 | 1.772 | 20 |
| After extraction, run 2 | 1.608 | 1.992 | 20 |

FTP encryption choices, form construction, state, and apply/save UI moved into
`FtpIntegrationPanel.jsx`. The root fell from 1,830 to 1,417 lines. Input
timing overlaps the observed range; no rendering speedup is claimed.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles FTP settings input render commits" --disableConsoleIntercept`.
All 21 focused Integrations tests passed. The Web production build, targeted
ESLint, and diff checks passed.
