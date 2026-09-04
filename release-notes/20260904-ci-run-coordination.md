---
category: changed
audience: operators
area: continuous-integration
action: none
breaking: false
---
GitHub CI now cancels superseded runs for the same ref and stops stalled Linux jobs after bounded job-level deadlines, keeping validation focused on the current commit.
