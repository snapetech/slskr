---
category: fixed
audience: users
area: networking
action: none
breaking: false
---
Expected remote Soulseek TCP closes are now treated as normal disconnect/reconnect events instead of protocol errors; malformed frames and other I/O failures remain visible as errors.
