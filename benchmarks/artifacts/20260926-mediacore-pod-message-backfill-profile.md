# MediaCore Pod Message Backfill Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Backfill Pod ID input three times, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 7.433 | 13.452 | 20 |
| After extraction | 0.596 | 0.923 | 20 |

Backfill state, API actions, derived timestamp validity, and UI moved into the
memoized `PodMessageBackfillPanel.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod message backfill input render commits' --reporter=verbose`. The
focused MediaCore test file passed all 23 tests after extraction.
