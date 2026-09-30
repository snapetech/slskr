# Rust Web live backend cadence — 2026-09-27

Source commit: `5d497f55762b08f6efb8ded099acf46307fa7330`.

The Rust Web Playwright audit used the prebuilt local Web distribution and a
fresh, isolated daemon from the current debug binary. The daemon used a
temporary state directory, disabled Soulseek auto-connect and share scans,
and raised both anonymous and controller API request limits for this audit.
The harness stopped the daemon and browser after the run.

Across 15 routes at desktop and mobile widths, the audit recorded 469 live API
requests. No route exceeded its request-count budget, and no repeated endpoint
fell below the 200 ms cadence floor. The response counts were 433 HTTP 200,
30 HTTP 204, four HTTP 403, and two HTTP 503. The strict audit exited 1 only
for three unique expected endpoint errors: protected YAML options and the
unconfigured Lidarr status. It recorded no browser, layout, request-count, or
cadence errors.

The raw audit JSON and 30 screenshots remain in ignored local
`target/rf063-live-audit-4/screenshots/`. The JSON SHA-256 is
`1fbfdce3c8d425835747a70ae0682d0f2ecf707a2dc23d240f31838097e03232`.
This is local live-backend evidence, not a deployed-device accessibility run.
