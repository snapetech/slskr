# RF-006/RF-066 kspls0 Temporary SlskR Swap (2026-09-29 UTC)

Internal-only deployment validation. The existing `slskdN` systemd service was
replaced temporarily to verify SlskR startup on the same local HTTP and peer
slots, then restored. No product configuration or persistent media/state was
changed.

- The existing `slskr-rf059:b42e01c5-trixie` image was used. Product source
  files have no changes after `b42e01c5` in the checked-out revision.
- Test account 4 was loaded from the protected repository `.env` in a mode-0600
  `/run` environment file. Its values and the temporary random API token were
  never recorded. The container used a read-only root filesystem and tmpfs
  state, with no music or other host data mounts.
- The web/API health endpoint on `127.0.0.1:5030` returned HTTP 200, and the
  session log recorded successful Soulseek login.
- SlskR bound peer TCP and UDP to the existing port 44508. DHT used UDP 44508;
  mesh/QUIC share that socket. HTTP remained loopback-only on 5030. After
  disabling the default HTTPS listener, the complete SlskR listener set was
  TCP 44508, TCP 5030, and UDP 44508; there was no 5031 listener or additional
  peer port.
- A 180-second systemd restore timer was armed before the swap. The original
  `slskd.service` and VPN compatibility service were restored; the original
  `ghcr.io/snapetech/slskdn:2026090321-slskdn.320` container returned healthy.
  The test container, temporary credentials, restore script, and timer were
  removed.

This verifies deployed startup, authenticated Soulseek login, the shared
single peer/DHT port, and restoration. It does not exercise deployed mesh
backfill or per-grant concurrent-stream admission, so RF-066 remains open for
that evidence; RF-006 retains its broader managed-service lifecycle scope.
