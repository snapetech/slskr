---
category: fixed
audience: users, operators
area: share-scanning
action: none
breaking: false
---

The versioned `PUT /api/v0/shares` endpoint now matches frozen slskd and
slskdN behavior when a scan is already running: it returns an empty `200 OK`
and leaves the active scan in place. The explicit `/api/v0/shares/rescan`
route continues to report a busy scan as an error.
