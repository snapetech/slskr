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

The initial startup-only swap verified deployed startup, authenticated
Soulseek login, the shared single peer/DHT port, and restoration. At that
stage, deployed mesh backfill and per-grant concurrent-stream admission
remained open; RF-006 retains its broader managed-service lifecycle scope.

## Deployed recipient backfill (2026-09-29 UTC)

- A second bounded swap used the same hardened SlskR image and test accounts:
  account 4 as owner on kspls0 and account 3 as recipient in an isolated
  Proton namespace. The recipient's NAT-PMP mapping and the owner's existing
  ingress both forwarded to the existing local peer listener on port 44508.
  The owner continued to use its existing public peer port 53514; no additional
  peer listener or public port was added.
- The owner initially had `SLSKR_PEER_HOST_OVERRIDE` set to its own address.
  That setting overrides outbound peer destinations, so the capability probe
  tried the wrong IP. Removing the override allowed the owner to receive and
  validate the recipient's signed capability descriptor over the real
  Soulseek peer path.
- The recipient accepted the synthetic grant announcement with HTTP 201. Its
  signed, certificate-pinned MeshContent backfill returned HTTP 200 with one
  downloaded item and zero failures. The on-disk file was 131,072 bytes and
  matched SHA-256
  `a5ca21d995e6b075abf6bcb9e785f59a987e726477eb21541d86b7da494cc9ca`.
- The temporary owner and recipient were stopped, test tokens and fixtures
  removed, and `slskd.service` restored. The container reported healthy; the
  watchdog, ingress-renewal timer, and ingress service were active. The shared
  local peer port 44508 was listening, the temporary recipient namespace was
  gone, and no `rf066_` firewall rules remained.

This closes deployed recipient-backfill evidence. Per-grant concurrent-stream
admission remains locally verified; deployed saturation under concurrent
streams and the broader RF-006 lifecycle acceptance remain open.

## Deployed owner stream saturation (2026-09-29 UTC)

This is internal-only operational evidence; no product code or published
behavior changed.

A temporary swap used the hardened slskr-rf059:b42e01c5-trixie image and
test account 4 from the protected repository .env. The container reused the
single peer endpoint on TCP/UDP 44508 and loopback HTTP on 127.0.0.1:5030. Its
root filesystem was read-only, runtime state was temporary, and it had no host
media mounts. The 8 MiB sparse FLAC fixture was held in /run and mounted
read-only.

The authenticated owner API created a collection, added the fixture, created a
test-account-3 grant with maxConcurrentStreams=1, and issued a share token
and content-bound stream ticket. A real ranged HTTP response was left open
without draining its body: the held stream returned 206, the concurrent
request returned 429, and a new range request returned 206 after the held
socket closed. This verifies the deployed owner's stream admission and release
path. This probe did not run a second recipient process or a signed peer
announcement; the preceding backfill receipt covers that peer path.

The temporary container and credentials, fixture, test driver, restore script,
and recovery timer were removed. slskd.service returned healthy, the VPN
watchdog and ingress-renewal timers and ingress service were active, local
peer port 44508 was listening, both pre-existing Proton namespaces remained,
and the temporary paths and timer were absent.
