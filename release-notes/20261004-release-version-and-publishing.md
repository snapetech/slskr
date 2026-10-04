---
category: fixed
audience: users, operators
area: release-pipeline
action: none
breaking: false
---
Release builds and the AUR source package now report the application version in the API, build information, startup banner, and `slskr version`; the CLI also prints the Soulseek protocol version. Archive checks verify native binaries and skip foreign platforms. GHCR publishing continues if optional Docker Hub login fails. COPR now prefers Fedora password and TOTP over expiring API tokens.
