---
category: fixed
audience: users
area: sdk
action: none
breaking: false
---
The TypeScript SDK now applies its request timeout while reading response bodies, so a stalled API response cannot hang indefinitely after headers arrive.
