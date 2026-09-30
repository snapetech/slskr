---
category: security
audience: users
area: peer-listener
action: none
breaking: false
---
The shared peer listener rejects conflicting plain and nested obfuscated initialization headers before reading their bodies, preventing a shorter plain interpretation from desynchronizing a nested file-transfer connection. Ordinary plain, obfuscated, and nested initialization continue on the same shared port.
