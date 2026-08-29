---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
The release gate now explicitly builds the debug daemon before parity checks, its web differentials use each profile's default credentials, and isolated description probes disable unrelated DHT startup, so tooling changes no longer fail on missing binaries, intentional profile identities, or port collisions.
