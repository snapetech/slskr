---
category: security
audience: operators
area: browser-security
action: none
breaking: false
---

The remediation baseline now rejects direct HTML insertion and dynamic code execution sinks in production browser source, keeping those patterns from entering the Web UI, dashboard, or TypeScript client unnoticed. The active security candidate scan also fails when its matcher errors instead of treating that error as an empty result.
