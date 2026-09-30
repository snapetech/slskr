# Integrations Servarr Readiness Prop Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The configured Lidarr URL is changed on 20 prop updates, and the
profile wraps the full Integrations tree. Times are milliseconds; they measure
JSDOM commits, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 7.853 | 18.247 | 20 |
| Before extraction, run 2 | 8.156 | 17.175 | 20 |
| After extraction, run 1 | 7.690 | 17.994 | 20 |
| After extraction, run 2 | 7.696 | 16.682 | 20 |

Servarr readiness checks, compatibility preview, copy, and wanted-sync actions
moved into `ServarrReadinessPanel.jsx`. The root fell from 439 to 211 lines.
The full-tree measurements overlap the broad baseline range; no speedup is
claimed.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles Servarr readiness option render commits" --disableConsoleIntercept`.
All 25 focused Integrations tests passed. The Web production build, targeted
ESLint, and diff checks passed.
