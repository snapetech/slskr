---
category: fixed
audience: users, operators
area: controller-api
action: none
breaking: false
---
Conversation batch, wishlist bulk-filter, and capability negotiation requests now reject oversized JSON string arrays before building request work or response state. Versioned wishlist bulk-filter requests are also routed to the correct endpoint instead of being treated as invalid item IDs.
