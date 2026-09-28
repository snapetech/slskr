# RF-059 kspls0 temporary deployment audit (2026-09-28)

Internal-only verification record. The temporary instance was removed after the
audit; this did not publish a release or leave a slskR service on kspls0.

## Build and runtime

- Source revision: `c297d4721f5ddcc0731fc594b8272be36aefa9ca`.
- The daemon was built with the pinned Rust 1.94.0 toolchain in Debian 12
  Bookworm, then the current React Web bundle was built from the same checkout.
- The production-style temporary container used host networking and a separate
  `/tmp` state directory. Its config and state directory were deleted after
  restoration. It skipped share scanning and disabled transfers, so the audit
  did not scan or write the host media directories.
- The HTTP UI kept its existing loopback bind at `127.0.0.1:5030`. Soulseek,
  mesh, and DHT used the shared peer port `44508`; no additional listener was
  added. During the run, TCP and UDP both bound to port 44508.
- At one idle audit sample, the container used 18.53 MiB and 0.19% CPU; it was
  healthy and not OOM-killed. This is a single sample without shares or
  transfers, not a production-load benchmark.

## Live route results

The existing React Web Chromium audit ran against the live kspls0 backend with
click actions and screenshots disabled. It covered four routes at desktop
1440x1000 and mobile 390x844:

| Route | Desktop API requests | Mobile API requests | API responses | Visible controls, desktop/mobile |
| --- | ---: | ---: | --- | ---: |
| `/` | 21 | 21 | all 200 | 16 / 16 |
| `/system` | 21 | 21 | all 200 | 9 / 9 |
| `/system/mediacore` | 29 | 29 | all 200 | 191 / 191 |
| `/system/integrations` | 18 | 18 | all 200 | 110 / 110 |

All eight documents returned HTTP 200, mounted the React root, and had zero
detected control overlaps. The run recorded 178 successful API responses.
Four browser console errors remain: blocked inline styles under the current
`style-src` policy on the desktop and mobile System and Integrations views. The
strict audit therefore exited nonzero despite the successful route, API, and
layout checks; the CSP finding needs follow-up.

The rebuilt bundle emitted a 9.38 KiB `System` entry chunk, an 87.08 KiB
`Integrations` chunk, and a 156.74 KiB `MediaCore` chunk. Aggregate budgets
remain covered by the existing local build gate.

## Restoration

The running slskdn service is healthy again and serves HTTP 200 on port 5030.
The active container uses the same slskdn image, host networking, command, and
four original config/state/media bind mounts. Its container ID changed during
the swap because the original container had Docker auto-remove enabled; the
replacement container is `944675c603307cf588d8e34cf92fe19e72fbd6188fa49bc615adadaf7100449f`.
The original config file and persistent state/media paths were not modified.

