# MediaCore Pod Join/Leave Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Join Request JSON field three times, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Run 1 median / p95 | Run 2 median / p95 |
| --- | ---: | ---: |
| Before extraction | 9.208 / 125.933 | 8.650 / 17.240 |
| After extraction | 1.415 / 1.854 | — |

The first baseline run contains an environment stall that affects two samples
and inflates its p95; its median is consistent with the second run. Pod Join/
Leave state, API actions, and UI moved into the memoized
`PodJoinLeavePanel.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod join request input render commits' --reporter=verbose`. The focused
MediaCore test file passed all 21 tests after extraction.
