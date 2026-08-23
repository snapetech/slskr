---
category: changed
audience: users, operators
area: slskdn-compatibility
action: Set `[podcore.gold_star_club].autojoin = true` or the matching environment variable to enable native/current auto-join.
breaking: false
---

Gold Star Club auto-join is now opt-in for native/current slskR launches, with a documented TOML setting; frozen slskdN compatibility launches retain their default-on parity behavior and can opt out through TOML or `SLSKDN_POD_GOLD_STAR_CLUB_AUTOJOIN=false`.
