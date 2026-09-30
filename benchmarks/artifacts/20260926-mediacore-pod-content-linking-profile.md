# MediaCore Pod Content Linking Input Profile

Date: 2026-09-26

This React Profiler measurement uses the existing MediaCore Vitest/JSDOM test
harness. It warms the content-search input three times, then records 20
controlled-input commits. Times are milliseconds. This measures commit work in
JSDOM and does not represent browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction | 7.040 | 14.013 | 20 |
| After extraction | 0.548 | 0.801 | 20 |

Content Linking actions, search/validation state, and UI moved into the memoized
`PodContentLinkingPanel.jsx`. The `contentId` field remains parent-owned because
Content Registry registration clears it. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles pod content search input render commits' --reporter=verbose`. The
focused MediaCore test file passed all 25 tests after extraction.
