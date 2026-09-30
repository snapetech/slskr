# MediaCore Pod Membership Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Membership Record JSON field with three updates, then
records 20 controlled-input commits. Times are milliseconds. This measures
commit work in JSDOM and does not represent browser paint or deployed-device
performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 9.308 | 17.579 | 20 |
| After extraction | 1.533 | 1.909 | 20 |

Membership management state, API actions, and UI moved into the memoized
`PodMembershipManagementPanel.jsx`. The panel receives the existing
`verifyingMembership` value and setter because the separate membership
verification workflow shares that loading flag. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod membership input render commits' --reporter=verbose`. The focused
MediaCore test file passed all 19 tests after extraction.
