---
category: security
audience: users, operators
area: browser-api-client
action: none
breaking: false
---
Browser API and remote-share clients now cap streamed JSON responses before parsing them, preventing unexpectedly large remote responses from consuming unbounded memory.
