---
category: fixed
audience: users
area: wishlist
action: none
breaking: false
---
When a rejected Lidarr download adds an ignored-result rule, the wishlist rule and matching search updates now persist in one ordered transaction so concurrent writes cannot leave the two stores out of sync.
