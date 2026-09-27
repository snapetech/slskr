# MediaCore Pod Discovery Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Pod registration JSON field with three updates, then
records 20 controlled-input commits. Times are milliseconds. This measures
commit work in JSDOM and does not represent browser paint or deployed-device
performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 8.254 | 16.453 | 20 |
| After extraction | 2.033 | 2.626 | 20 |

Pod Discovery state, actions, and UI moved into the memoized
`PodDiscoveryPanel.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod discovery input render commits' --reporter=verbose`. The focused
MediaCore test file passed all 20 tests on rerun. One earlier complete-file run
had a ContentID example assertion fail; the test passed alone and in the rerun.
