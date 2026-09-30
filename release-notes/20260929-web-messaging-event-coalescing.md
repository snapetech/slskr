---
category: fixed
audience: users, operators
area: web-messaging
action: none
breaking: false
---

The Web message workspace now limits hub-driven refreshes to one per second
while still coalescing overlapping work. Large message and room event bursts
no longer trigger repeated full workspace reads.
