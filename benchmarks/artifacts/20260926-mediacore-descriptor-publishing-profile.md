# MediaCore Descriptor Publishing Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Content Descriptor Publishing input three times, then
records 20 controlled-input commits. Times are milliseconds. This measures
commit work in JSDOM and does not represent browser paint or deployed-device
performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 11.344 | 42.541 | 20 |
| Before extraction, run 2 | 11.689 | 43.528 | 20 |
| After extraction | 1.969 | 2.061 | 20 |

The first six samples in both baseline runs show event-loop stalls in the
41–44 ms range. Descriptor publishing and retrieval were split into separate
memoized owner panels under the `MediaCoreDescriptorManagementPanel.jsx`
composition component. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles descriptor publishing input render commits' --reporter=verbose`. The
focused MediaCore test file passed all 29 tests after extraction.
