---
category: fixed
audience: users, operators
area: relay-security
action: none
breaking: false
---

Relay file requests now reject control-character filenames and detect shared
files whose size changed between lookup and streaming, keeping multipart
transfers well-formed and consistent with the advertised file metadata.
