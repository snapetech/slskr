---
category: fixed
audience: users, operators
area: web-ui
action: none
breaking: false
---

Load the application/options snapshot before opening the event feed, including
when authentication is disabled, so a disconnected daemon still selects the
configured slskd or slskdN UI profile and renders its matching navigation
immediately.
