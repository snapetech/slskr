---
category: fixed
audience: users
area: client-sdk
action: none
breaking: false
---
The TypeScript client now preserves valid falsy JSON request bodies such as `false`, `0`, empty strings, and `null` instead of silently omitting them.
