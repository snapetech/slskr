---
category: fixed
audience: operators
area: controller-compatibility
action: none
breaking: false
---
Configuration-watch restart indicators now compare each valid reload with the
previous watched configuration instead of startup defaults, preventing
ordinary live changes from falsely reporting that an application restart is
pending.
