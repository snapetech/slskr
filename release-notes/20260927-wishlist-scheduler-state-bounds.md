---
category: fixed
audience: operators
area: wishlist
action: Repair corrupt wishlist scheduler rows if a load now reports an invalid value.
breaking: false
---
Wishlist scheduler persistence now rejects negative or oversized index and interval values instead of wrapping them during load or save.
