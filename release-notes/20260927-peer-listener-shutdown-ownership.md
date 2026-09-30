---
category: fixed
audience: users
area: peer-listener
action: none
breaking: false
---
Shutdown now aborts and joins pending peer initialization and accepted peer handlers before final persistence. Canceled handlers release their connection permits and network-guard counts on the existing shared port.
