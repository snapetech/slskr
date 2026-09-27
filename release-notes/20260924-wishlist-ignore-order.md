---
category: changed
audience: users
area: wishlist-search
action: none
breaking: false
---
Wishlist item edits, imports, and deletions now share one persistence order,
along with ignored-result rules and incoming wishlist search responses. This
prevents delayed item writes from restoring deleted rows and late search
snapshots from restoring ignored results after restart.
