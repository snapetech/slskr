---
category: fixed
audience: operators
area: integrations
action: none
breaking: false
---
Webhook deliveries still queued after worker shutdown or a daemon restart are recorded as failed with an explicit unknown-outcome reason. Confirmed success/failure records and attempt counts remain intact. Reconciliation uses a partial index over queued records and does not send another request.
