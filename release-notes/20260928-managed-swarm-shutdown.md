---
category: fixed
audience: operators
area: multisource-downloads
action: none
breaking: false
---
Asynchronous swarm downloads now belong to daemon shutdown: their workers are aborted and joined, unfinished jobs report failure, and temporary workspaces are removed. Requests arriving after shutdown begins receive HTTP 503 instead of a queued response for a worker that cannot run.
