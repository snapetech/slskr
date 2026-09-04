---
category: fixed
audience: operators
area: relay
action: none
breaking: false
---
The relay controller now bounds its in-memory completed share-upload history to
the same limit as the durable manifest, preventing repeated uploads from
causing unbounded state growth.
