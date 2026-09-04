---
category: security
audience: users, operators
area: rate-limiting
action: none
breaking: false
---
Rate-limit state now rejects oversized or malformed caller keys, caps Soulseek operation windows, and reclaims expired source buckets so sustained key churn cannot exhaust memory or permanently consume limiter capacity.
