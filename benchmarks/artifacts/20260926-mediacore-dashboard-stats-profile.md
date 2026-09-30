# MediaCore Statistics Dashboard Update Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Registry Stats action three times, then records 20 update
commits. Times are milliseconds. This measures commit work in JSDOM and does not
represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 5.530 | 66.832 | 20 |
| Before extraction, run 2 | 5.599 | 66.517 | 20 |
| After extraction | 1.969 | 2.468 | 20 |

The first three samples in both baseline runs show event-loop stalls in the
66–68 ms range. The baseline median was stable across runs; p95 includes those
stalls. Dashboard, registry, descriptor, fuzzy-matching, perceptual-hashing,
IPLD, portability, and publishing statistics state and actions moved into the
memoized `MediaCoreStatisticsDashboardPanel.jsx`.

Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles MediaCore dashboard statistics updates' --reporter=verbose`. The
focused MediaCore test file passed all 28 tests after extraction.
