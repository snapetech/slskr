---
category: security
audience: users, operators
area: admin-dashboard-transport
action: none
breaking: false
---
Admin dashboard API and metrics requests now cap response bodies while streaming them, preventing unexpectedly large responses from consuming unbounded browser memory.
