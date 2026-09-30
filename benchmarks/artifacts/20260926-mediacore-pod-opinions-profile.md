# MediaCore Pod Opinions Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Pod Opinion Management input three times, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 7.002 | 16.247 | 20 |
| After extraction | 1.149 | 1.493 | 20 |

Pod Opinion Management and aggregation state, query projections, actions, and
UI moved into the memoized `PodOpinionsPanel.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod opinion input render commits' --reporter=verbose`. The focused
MediaCore test file passed all 26 tests after extraction.
