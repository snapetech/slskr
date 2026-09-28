---
category: fixed
audience: operators
area: media-process-lifecycle
action: none
breaking: false
---
Daemon-launched external visualizer children now have explicit ownership instead of detached blocking waits. Exited children are reaped periodically, the existing four-process limit is preserved, and daemon shutdown closes launch admission and kills and reaps its owned children.
