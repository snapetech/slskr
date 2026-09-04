---
category: fixed
audience: operators
area: persistence
action: none
breaking: false
---
Pod-channel state now stays within the loader's size and record invariants,
and realm subject-index state is reopened without symlink following and
revalidated before it becomes active.
