---
category: changed
audience: users
area: dashboard-polling
action: none
breaking: false
---
Dashboard refreshes now avoid duplicate polling after a manual refresh, pause
while the browser tab is hidden, wait for slow requests to settle, and stop
cleanly when the view is unmounted.
