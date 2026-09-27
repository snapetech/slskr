# MediaCore Pod Message Storage Action Profile

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. The Get Storage Stats action is warmed three times, then 20 update
commits are recorded. Times are milliseconds. They measure JSDOM commit work,
not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 7.426 | 7.779 | 20 |
| Before extraction, run 2 | 8.518 | 9.123 | 20 |
| After extraction | 0.442 | 0.696 | 20 |

The baseline samples form high/low clusters consistent with JSDOM scheduling
variation. Pod Message Storage state, stats/cleanup actions, retention UI, and
its existing Message Search and Message Maintenance panels moved into
`PodMessageStoragePanel.jsx`. The composition root fell from 1,177 to 1,062
lines.

The profile used `npm --prefix web test --
src/components/System/MediaCore/index.test.jsx -t "profiles pod message storage
updates" --reporter dot`. The final focused MediaCore run passed all 34 tests.
The Web production build, targeted ESLint, and diff checks passed.
