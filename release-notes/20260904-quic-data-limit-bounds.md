---
category: security
audience: users, operators
area: quic-data
action: none
breaking: false
---
QUIC data-plane configuration now enforces absolute payload and connection-cache ceilings, preventing extreme caller-provided limits from creating unbounded memory or connection pressure.
