# MediaCore Metadata Portability Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. The metadata package input is warmed three times, then 20 controlled
input commits are recorded. Times are milliseconds. They measure JSDOM commit
work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 7.502 | 14.822 | 20 |
| Before extraction, run 2 | 5.827 | 15.078 | 20 |
| After extraction | 1.124 | 1.343 | 20 |

Both baseline samples show a high-duration cluster, consistent with JSDOM
event-loop stalls; treat the before/after comparison as directional. Export and
import state, conflict-strategy loading, actions, and both portability cards
moved into `MediaCoreMetadataPortabilityPanel.jsx`. The composition root fell
from 1,851 to 1,535 lines.

The dedicated profile used `npm --prefix web test --
src/components/System/MediaCore/index.test.jsx -t "profiles metadata import input
render commits" --reporter verbose`. The focused MediaCore suite passed all 32
tests on its final run. Targeted ESLint, the Web production build, and diff
checks passed.
