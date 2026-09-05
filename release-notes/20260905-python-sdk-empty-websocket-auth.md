---
category: fixed
audience: users
area: sdk
action: none
breaking: false
---
The Python WebSocket client now omits the authorization header when no token is configured, matching the other SDKs and allowing unauthenticated event-stream deployments to connect.
