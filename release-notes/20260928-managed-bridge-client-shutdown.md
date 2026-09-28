---
category: fixed
audience: operators
area: soulfind-bridge
action: none
breaking: false
---
Soulfind bridge client handlers now belong to managed daemon shutdown and stop with their listener. Stalled clients are cancelled, completed handlers are reaped, and active-client records are cleared after joined daemon shutdown. New client work is rejected once shutdown closes task admission.
