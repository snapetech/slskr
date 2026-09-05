---
category: fixed
audience: users, operators
area: client-sdks
action: none
breaking: false
---
The Python and TypeScript SDKs now reject HTTP redirects before credentials can be forwarded, and the TypeScript response adapters safely ignore out-of-range numeric timestamps instead of throwing.
