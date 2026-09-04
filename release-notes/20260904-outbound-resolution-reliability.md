---
category: fixed
audience: users, operators
area: outbound-integrations
action: none
breaking: false
---

MusicBrainz, AcoustID, and Solid WebID requests now retain their validated DNS resolution for the connection, preventing a later DNS change from sending an approved request to a different address; Solid HTTP URLs also use their actual default port.
