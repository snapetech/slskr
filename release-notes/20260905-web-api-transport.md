---
category: security
audience: users, operators
area: browser-api-transport
action: none
breaking: false
---
The browser API client now uses bounded Fetch responses and rejects redirects, preventing credentialed API requests from following redirects to another destination.
