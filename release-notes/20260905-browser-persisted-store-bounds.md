---
category: security
audience: users, operators
area: browser-persisted-stores
action: none
breaking: false
---
Browser-local discovery, acquisition, playlist, watchlist, community-quality, and player-rating stores now validate text and collection sizes and reject oversized persisted JSON before parsing, keeping malformed local state from causing excessive memory or rendering work.
