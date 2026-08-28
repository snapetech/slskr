# slskr Compliance Notes

This is an engineering compliance checklist, not legal advice.

## Sources Reviewed

- Soulseek Terms of Use: <https://www.slsknet.org/news/node/682>
- Soulseek Rules: <https://www.slsknet.org/news/node/681>
- Nicotine+ protocol documentation: <https://nicotine-plus.org/doc/SLSKPROTOCOL.html>

## Current Conclusions

- This project is licensed AGPL-3.0-only from its first public release. Operators who modify and run a public network service must preserve the AGPL source-availability path for users interacting with that modified service.
- We do not copy or link the official Soulseek client. The official client software license restrictions do not grant permission to decompile, reverse engineer, redistribute, or create derivative works of that official software.
- This project is a independent Rust implementation from public protocol notes and observed behavior. Keep it that way: do not import official client code or disassembly.
- Network access is governed by Soulseek's service terms and rules. Those rules say access can be revoked, prohibit abuse/spam/bots, and tolerate alternative clients only when they implement the expected Soulseek feature surface.
- The protocol docs ask client authors not to extend the protocol without administrator approval and to avoid impersonating existing clients by reusing their version numbers.
- Third-party client version ranges are useful operational guidance, but do not apply as license terms unless this project uses that code or binaries.
- No public-source requirement was found to report independent reverse engineering work before private development or low-volume testing. Re-check before broad public distribution.
- Do not falsely claim endorsement, affiliation, compatibility level, or origin. Public materials should say the client is an independent Soulseek-compatible implementation, and should credit public protocol documentation when used.

## Attribution Policy

- Preserve `LICENSE` and `NOTICE` in source, binary, package, and container distributions.
- Container images should set `org.opencontainers.image.licenses=AGPL-3.0-only` and include `org.opencontainers.image.source` for the source repository/version being distributed.
- Public README/docs may describe `slskr` as an independent Rust implementation for the Soulseek network.
- Public README/docs should not say the project is based on another implementation unless code is actually derived from that project and its license is satisfied.
- Mentioning that public protocol notes and observed network behavior informed compatibility work is acceptable and more accurate than naming a single "root" project.
- If code, fixtures, examples, or documentation are copied from another project later, preserve license headers and add the required attribution before merging.

## Naming and Branding Policy

- Use `slskr` for the Rust branch and `slskr-*` for Rust crates/packages.
- Do not use names that visually imply another project's distribution, fork, or language-port branding.
- Do not describe public artifacts as a fork, compatibility claim, or replacement distribution for another project.
- Public UI branding must be visually distinct: do not copy another project's logo, color system, layout, screenshots, package descriptions, or trade dress.
- Include a non-affiliation statement in public README/package/docs surfaces.

## Version Policy

- `slskr` uses major version `175` and a private minor band `8_800_000..=8_809_999`.
- Default login version is `175.8_800_001`.
- Do not use known reserved major versions for other popular clients:
  - `157` Soulseek NS / SoulseekQt
  - `160` Nicotine+
  - `170` known third-party implementation
- Do not use documented third-party minor range `760..=7_699_999`.
- Before public release, re-check upstream client/version guidance and consider contacting Soulseek administrators if the client will be broadly distributed.

## Release Gates

- No public release while the client behaves like a narrow bot/script. It must support the normal feature surface expected of tolerated alternative clients: chat, search, wishlist, downloads, uploads, and privilege handling.
- Do not add proprietary protocol extensions on the public Soulseek network.
- Do not advertise third-party real-client obfuscated peer compatibility until a reachable type-1 peer has been tested. Local slskr-to-slskr type-1 support may be described as implemented and covered by local/Soulfind tests.
- Do not run high-rate searches, crawls, scraping, or unattended network load tests against the public network.
- Honor server-provided excluded search phrases for outgoing results.
- Do not bundle, print, log, or commit Soulseek credentials.
- Use `scripts/check-release-package.sh` as the Rust packaging release gate.
  The app crate depends on unpublished internal workspace crates, so
  `cargo package -p slskr` is intentionally not a valid standalone package
  check.

## Current Status

- Live login smoke passed with existing local credentials.
- The client is not yet ready for public distribution or unattended soak. The next safe step is a controlled long-running smoke with low message volume, reachable listen port, and explicit operator supervision.
