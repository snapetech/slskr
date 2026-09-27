# MediaCore Similarity Subpanel Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. The raw hash input is warmed three times, then 20 controlled-input
commits are recorded. Times are milliseconds. They measure JSDOM commit work,
not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before finer split, isolated run 1 | 1.948 | 2.347 | 20 |
| Before finer split, isolated run 2 | 1.909 | 2.335 | 20 |
| After finer split | 0.558 | 0.670 | 20 |

The combined 518-line similarity owner became four independent panels for raw
hash comparison, fuzzy content matching, perceptual comparison, and text
similarity, composed by a 16-line `MediaCoreSimilarityAnalysisPanel.jsx`. The
MediaCore root remains 369 lines.

The profile used `npm --prefix web test --
src/components/System/MediaCore/index.test.jsx -t "profiles hash similarity
input render commits" --reporter dot`. The focused 36-test suite passes on the
final full rerun. The Web production build, targeted ESLint, and diff checks
passed.
