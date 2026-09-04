---
category: fixed
audience: users, operators
area: reliability
action: none
breaking: false
---

Legacy profile reads now match the native route when capability generation is
unavailable, while mesh signing failures and unsuccessful external visualizer
processes are surfaced in logs instead of being silently discarded. QUIC data
reads now reject an invalid already-consumed count instead of silently
accepting an over-limit payload, and retention cleanup failures are recorded
for operators.
