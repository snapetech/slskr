---
category: security
audience: operators
area: observability-transport
action: none
breaking: false
---
Telemetry and Loki clients now keep startup traces and log records on the configured collector by rejecting redirects instead of silently following them to another destination.
