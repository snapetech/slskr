# Integrations Metadata Settings Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The MusicBrainz user-agent field is warmed three times, then 20
controlled-input commits are recorded. Times are milliseconds. They measure
JSDOM commit work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 3.007 | 3.280 | 20 |
| Before extraction, run 2 | 2.908 | 3.167 | 20 |
| After extraction, run 1 | 2.964 | 3.389 | 20 |
| After extraction, run 2 | 3.003 | 3.599 | 20 |

Metadata and Servarr configuration form construction, state, save actions, and
UI moved to `MetadataSettingsPanel.jsx`. The root fell from 2,628 to 1,830
lines. Timing overlaps the observed range, so this is a structural module
boundary and no rendering speedup is claimed.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles metadata settings input render commits" --disableConsoleIntercept`.
All 20 focused Integrations tests passed. The Web production build, targeted
ESLint, and diff checks passed.
