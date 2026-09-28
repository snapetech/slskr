---
category: fixed
audience: users
area: native-web-ui
action: none
breaking: false
---
The native player and System page now share their initial transfer-speed response for up to one second, avoiding a duplicate request during slower page loads. The bounded cache expires before later refreshes and handles clock reversal. Existing request-count and cadence budgets remain enforced.
