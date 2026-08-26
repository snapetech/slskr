---
category: fixed
audience: users, operators
area: web-ui
action: none
breaking: false
---
The web UI's login could never succeed: it sent only a username to `POST /session` with the password as a Bearer token header, while the controller's session endpoint requires both fields in the request body and issues a signed session token in response. Login now matches the controller's actual contract. Also replaced the e2e test harness's launch mechanism, which still expected a .NET project/DLL from before the Rust rewrite, with one that runs the real `slskr` binary directly via its documented `SLSKD_*`-prefixed environment variables — the same mechanism a Docker or systemd deployment uses.
