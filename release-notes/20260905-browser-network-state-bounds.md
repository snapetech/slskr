---
category: security
audience: users, operators
area: browser-network-state
action: none
breaking: false
---
Browser network endpoint snapshots, RustyMilk automation settings, and fallback event WebSocket messages now have bounded parsing and known-shape validation so malformed or oversized state cannot consume unbounded UI resources.
