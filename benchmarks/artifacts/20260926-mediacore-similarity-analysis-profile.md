# MediaCore Similarity Analysis Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. The first hash input is warmed three times, then 20 controlled-input
commits are recorded. Times are milliseconds. They measure JSDOM commit work,
not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 5.719 | 5.995 | 20 |
| Before extraction, run 2 | 4.932 | 5.204 | 20 |
| After extraction, isolated run 1 | 1.948 | 2.347 | 20 |
| After extraction, isolated run 2 | 1.909 | 2.335 | 20 |
| After extraction, full-suite run | 0.550 | 0.723 | 20 |

Raw hash comparison, fuzzy content search, perceptual ContentID comparison, and
text similarity workflows moved into `MediaCoreSimilarityAnalysisPanel.jsx`.
The MediaCore composition root fell from 820 to 369 lines. The panel is 518
lines and remains scheduled for a finer split. Full-suite timing differs from
isolated timing, so the isolated runs are used for the before/after comparison.

The profile used `npm --prefix web test --
src/components/System/MediaCore/index.test.jsx -t "profiles hash similarity
input render commits" --reporter dot`. All 36 focused MediaCore tests passed.
The Web production build, targeted ESLint, and diff checks passed.
