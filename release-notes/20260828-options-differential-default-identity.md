---
category: fixed
audience: operators
area: release-pipeline
action: none
breaking: false
---
Options and Swagger differential validation now ignores only known default product identity fields, so branded slskR defaults do not fail frozen-profile release checks while configured values remain strict. Metrics reload polling also uses a bounded test-only API allowance instead of tripping the production rate limit.
