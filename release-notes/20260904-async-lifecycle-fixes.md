---
category: fixed
audience: users, operators
area: runtime-lifecycle
action: none
breaking: false
---

Room joins, dashboard data, share refreshes, library-health actions, configuration rendering, and playback now ignore stale or canceled work and clean up delayed callbacks when their view is replaced. Port-forwarding statistics now use backend-reported counters and timestamps; unavailable VPN telemetry is shown as unavailable instead of fabricated values.
