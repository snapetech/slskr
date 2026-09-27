# MediaCore Pod Membership Verification Input Profile

Date: 2026-09-26

These React Profiler measurements use the existing MediaCore Vitest/JSDOM test
harness. The membership Pod ID input is warmed three times, then 20 controlled
input commits are recorded. Times are milliseconds. They measure JSDOM commit
work, not browser paint or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 10.753 | 12.026 | 20 |
| Before extraction, run 2 | 9.419 | 10.707 | 20 |
| After extraction | 1.378 | 1.647 | 20 |

The baseline samples contain event-loop stalls. Pod Membership Verification
state, actions, and UI moved into `PodMembershipVerificationPanel.jsx`. The
membership loading flag remains shared with Pod Membership Management, and the
message input remains shared with Message Signing. The composition root fell
from 1,535 to 1,177 lines.

The profile used `npm --prefix web test --
src/components/System/MediaCore/index.test.jsx -t "profiles pod membership
verification input render commits" --reporter verbose`. All 33 focused
MediaCore tests passed. The Web production build, targeted ESLint, and diff
checks passed.
