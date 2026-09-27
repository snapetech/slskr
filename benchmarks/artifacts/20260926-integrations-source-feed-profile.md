# Integrations Spotify Source Feed Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The Spotify client-ID field is warmed three times, then 20
controlled-input commits are recorded. Times are milliseconds. They measure
JSDOM commit work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 1.651 | 1.859 | 20 |
| Before extraction, run 2 | 1.657 | 1.937 | 20 |
| After extraction, isolated run | 1.626 | 2.023 | 20 |
| After extraction, full-suite run | 1.569 | 2.020 | 20 |

Shared option/form/async helpers moved into `integrationsShared.jsx`. The
Spotify, YouTube, and Last.fm settings workflow moved into
`SourceFeedIntegrationsPanel.jsx`. Input timing remained within the observed
range, consistent with the workflow already owning local React state before
its source file was split. The composer fell from 3,915 to 3,264 lines.

The profile used `npm --prefix web test --
src/components/System/Integrations/index.test.jsx -t "profiles Spotify source-feed
input render commits" --reporter dot`. All 18 focused Integrations tests passed.
The Web production build, targeted ESLint, and diff checks passed.
