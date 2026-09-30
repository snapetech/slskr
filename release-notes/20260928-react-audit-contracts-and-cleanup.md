---
category: fixed
audience: operators
area: development-audits
action: Regenerate reusable React browser audit evidence.
breaking: false
---
React browser audits now use valid API fixtures and close their temporary server if Chromium launch or shutdown fails. Synthetic endpoint sweeps no longer count as rendered workflows. Reusable evidence must declare zero sweeps; older or swept artifacts are rejected.
