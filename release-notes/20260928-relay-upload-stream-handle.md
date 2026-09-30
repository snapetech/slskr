---
category: fixed
audience: users
area: relay
action: none
breaking: false
---
Relay uploads hand a readable, rewound file handle to the streaming reader after syncing the payload. This fixes bad-file-descriptor failures while retaining private staging permissions, exclusive creation, and the original file handle.
