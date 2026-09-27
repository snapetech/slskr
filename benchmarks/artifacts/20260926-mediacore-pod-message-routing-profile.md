# MediaCore Pod Message Routing Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the routing-message JSON field three times, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 8.088 | 13.211 | 20 |
| After extraction | 1.361 | 1.723 | 20 |

Routing state, actions, and UI moved into the memoized
`PodMessageRoutingPanel.jsx`. The duplicated search-index rebuild and database
vacuum controls now share local state and API calls in
`PodMessageMaintenanceActions.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod message routing input render commits' --reporter=verbose`. The
focused MediaCore test file passed all 22 tests after extraction.
