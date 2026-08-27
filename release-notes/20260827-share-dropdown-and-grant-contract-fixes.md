---
category: fixed
audience: users, operators
area: web-ui
action: none
breaking: false
---
The custom `semantic-ui-react` shim's `Dropdown` component never implemented interaction for its two most common usage patterns: a trigger + menu (e.g. the nav "More" overflow) rendered a menu that no click or keyboard input ever opened, and an `options`-based picker (search filters, System settings, Collections) rendered a native `<select>`, whose options live in OS-native chrome unreachable by automation. Both are now real interactive listboxes, supporting `search`, `clearable`, `selection`, and `fluid`. Fixing the picker surfaced two further contract mismatches it had been hiding: `ShareGroups`' "add member" form posted a `peerId`/`userId` field the backend has never read (it only accepts `username`), and Collections' "share with a group" flow posted a nonexistent `audienceId`/`audienceType` pair instead of resolving the group to its member usernames and creating one real grant per member. Share displays (Collections' share table, "Shared with me") also read `allowStream`/`allowDownload`/`audienceId` fields the backend has never sent — it reports a single `permissions` string and a `username` — so those always rendered as empty or "No". They now derive from the real fields.
