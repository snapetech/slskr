---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
Collection, share-grant, and share-token mutations now persist in their
in-memory order, and collection snapshot updates require the parent row to
still exist. Only collection creation can insert the parent row, so a delayed
create or update snapshot cannot restore a collection after a concurrent
delete. A token request queued behind deletion observes the removed grant.
