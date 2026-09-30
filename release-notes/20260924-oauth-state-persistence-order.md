---
category: changed
audience: users
area: integrations
action: none
breaking: false
---
Spotify OAuth state creation and callback consumption now share one persistence
order. Callbacks release the state-store lock during SQLite work, and a failed
deletion leaves the one-time state available for retry.
