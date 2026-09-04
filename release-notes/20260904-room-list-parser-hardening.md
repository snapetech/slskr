---
category: fixed
audience: users
area: protocol
action: none
breaking: false
---

Room-list count parsing now rejects impossible vectors early. Mesh-sync
credentials are size-checked before base64 decoding, and incoming entries with
malformed FLAC keys are discarded before database merge.
