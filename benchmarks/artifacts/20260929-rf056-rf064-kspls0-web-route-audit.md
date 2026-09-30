# RF-056/RF-064 kspls0 Web route audit (2026-09-29)

Internal verification record. The browser used the temporary test account with
click actions disabled; no API mutations were issued and no credentials or
screenshots are retained.

## Results

The deployed React Web bundle from `5dbd1f43904b942dcb262f2a8f088afedf33b4f5`
was loaded from kspls0 over the loopback-only HTTP listener. Chromium visited
all 42 maintained routes at desktop and mobile sizes, for 84 document views.
All documents returned HTTP 200, the React root mounted on every route, there
were no layout-overlap findings, and the audit recorded no unexpected errors.

The run observed 1,704 API responses: 1,696 returned HTTP 200, two returned the
expected HTTP 400 for the audit's non-existent `commons-example-sound` search
fixture ID, and six returned the explicit HTTP 404 contract for the native
adversarial-settings GET. Those exact two GET contracts were allowlisted; no
other live status was ignored.

The browser made no action clicks, so it did not create searches, send
messages, or change configuration. Dashboard health/degraded-state and
hidden-tab behavior are recorded separately in
[`20260929-rf049-rf052-rf057-rf061-kspls0-dashboard.md`](20260929-rf049-rf052-rf057-rf061-kspls0-dashboard.md).
