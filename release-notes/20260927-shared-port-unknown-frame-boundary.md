---
category: fixed
audience: users
area: peer-listener
action: none
breaking: false
---
The shared peer and mesh port rejects an unknown initialization fallback if an alternate frame interpretation has already consumed bytes beyond it. Unambiguous unknown frames keep their following stream bytes intact.
