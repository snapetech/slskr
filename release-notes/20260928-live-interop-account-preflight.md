---
category: changed
audience: operators
area: live-interop
action: Update the `SLSKR_LIVE_INTEROP_ENV` repository secret with account pairs 1–6 for the default matrix or 1–8 when VPN probes are enabled.
breaking: false
---
The credentialed live-interop runner now validates every account selected by its login, peer, private-message, and room-message probes before resolving the public Soulseek host. The default account range is six; VPN login probes require accounts 5–8 as well.
