---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---

AUR source and binary package definitions now run an isolated `makepkg` source/prepare smoke check in the release and package-surface gates, in addition to checksum validation.
