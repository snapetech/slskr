---
category: fixed
audience: operators
area: controller-compatibility
action: none
breaking: false
---
The slskdN compatibility profile now marks a connected session as awaiting
reconnect when watched obfuscation settings change. Repeated watcher events
for the same settings no longer reassert the flag.
