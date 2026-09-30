---
category: security
audience: operators
area: diagnostic-dumps
action: Retry diagnostic dump requests after an existing dump finishes.
breaking: false
---
Diagnostic dumps admit one worker at a time, including after request cancellation. Linux failures clean up the owned process group, reap the child and clear ptrace permission. The existing 60-second child deadline remains. Temporary dump, mesh-preview and relay output retains cleanup ownership through its last consumer, including canceled requests.
