---
category: fixed
audience: users
area: integrations
action: none
breaking: false
---
Spotify authorization and token refresh now commit against the connection generation they started from. A disconnect invalidates older in-flight work, preventing stale credentials from reappearing afterward.
