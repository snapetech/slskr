# MediaCore Content Registry and Graph Input Profiles

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. Each target input is warmed three times, then 20 controlled-input
commits are recorded. Times are milliseconds. They measure JSDOM commit work,
not browser paint or deployed-device performance.

| Workflow | State | Median | p95 | Commits |
| --- | --- | ---: | ---: | ---: |
| ContentID Registry | Before extraction | 4.906 | 22.814 | 20 |
| ContentID Registry | After extraction | 1.776 | 2.237 | 20 |
| Content Graph | Before extraction | 5.523 | 23.195 | 20 |
| Content Graph | After extraction | 1.196 | 1.502 | 20 |

ContentID registration, resolution, validation, domain search, and example field
coordination moved into `MediaCoreContentRegistryPanel.jsx`. Graph traversal,
graph lookup, and inbound-link workflows moved into
`MediaCoreContentGraphPanel.jsx`. Dedicated measurements used
`npm --prefix web test -- src/components/System/MediaCore/index.test.jsx -t
'profiles (ContentID registry input render commits|content graph traversal input render commits)' --reporter=verbose`.
The focused MediaCore test file passed all 31 tests after extraction.
