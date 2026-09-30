# MediaCore Pod Publishing Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the Pod JSON field with three updates, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 9.425 | 13.968 | 20 |
| After extraction | 1.026 | 1.167 | 20 |

The workflow state, actions, and view moved into memoized
`PodDhtPublishingPanel.jsx`; input changes no longer rerender unrelated
MediaCore sections. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod publication input render commits' --reporter=verbose`. The full
focused test file also passed 18 tests after extraction.
