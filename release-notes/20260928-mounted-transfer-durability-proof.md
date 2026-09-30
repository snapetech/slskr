---
category: changed
audience: operators
area: transfer-durability
action: none
breaking: false
---
A Linux validation fixture now exercises transfer durability on an isolated mounted filesystem with delayed fsync requests. It verifies queue access during sync delays and ordered state recovery after three writer crashes. The harness bounds runtime, unmounts its temporary filesystem, and reaps owned processes.
