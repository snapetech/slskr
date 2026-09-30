---
category: changed
audience: operators
area: search-response-dedup
action: none
breaking: false
---
Client search response retention now indexes bounded response fingerprints and
uses exact equality fallback, reducing duplicate-burst comparison work without
loosening replay detection.
