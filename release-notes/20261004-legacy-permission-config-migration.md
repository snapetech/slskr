---
category: changed
audience: operators
area: configuration
action: Move permissions.file.mode to transfers.download.destination.permissions.mode before starting slskR.
breaking: true
---
The legacy configuration profile follows upstream slskd's breaking permission-setting change and rejects the old key. Move it under transfers.download.destination.permissions.mode; the new setting applies to both downloaded files and directories.
