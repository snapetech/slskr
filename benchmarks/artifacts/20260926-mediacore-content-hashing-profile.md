# MediaCore Content Hashing Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. The audio samples input is warmed three times, then 20 controlled-input
commits are recorded. Times are milliseconds. They measure JSDOM commit work,
not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 6.739 | 8.212 | 20 |
| Before extraction, run 2 | 6.531 | 7.305 | 20 |
| After extraction | 1.037 | 1.144 | 20 |

Audio and image hash input state, computation actions, and cards moved into
`MediaCoreContentHashingPanel.jsx`. Supported-algorithm metadata remains shared
with the summary rendered at the end of the composition root and is passed to
the hashing panel. The root fell from 1,062 to 820 lines.

The profile used `npm --prefix web test --
src/components/System/MediaCore/index.test.jsx -t "profiles audio hash input
render commits" --reporter dot`. All 35 focused MediaCore tests passed. The
Web production build, targeted ESLint, and diff checks passed.
