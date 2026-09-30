# MediaCore Pod Channel Management Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Pod ID for channel management input three times, then
records 20 controlled-input commits. Times are milliseconds. This measures
commit work in JSDOM and does not represent browser paint or deployed-device
performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Baseline run A | 9.938 | 99.999 | 20 |
| Baseline run B | 7.813 | 13.184 | 20 |
| After extraction | 0.615 | 0.817 | 20 |

Baseline run A contains an environment stall that affects several samples and
inflates its p95; run B is the stable comparison. Channel state, actions,
current-pod projection, and UI moved into the memoized
`PodChannelManagementPanel.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod channel management input render commits' --reporter=verbose`. The
focused MediaCore test file passed all 24 tests after extraction.
