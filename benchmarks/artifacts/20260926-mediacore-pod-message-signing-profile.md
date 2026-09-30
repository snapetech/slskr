# MediaCore Pod Message Signing Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the signing-message input three times, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 6.138 | 14.396 | 20 |
| After extraction | 1.068 | 1.468 | 20 |

Pod Message Signing state, API actions, and UI moved into the memoized
`PodMessageSigningPanel.jsx`. The shared `messageToVerify` input and
`verificationResult` remain parent-owned because membership message verification
and descriptor verification also use them. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod message signing input render commits' --reporter=verbose`. The
focused MediaCore test file passed all 27 tests after extraction.
