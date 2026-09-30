---
category: security
audience: operators
area: packaging
action: none
breaking: false
---
Docker image builds now exclude repository `.env` files and `.secrets` directories from the build context, so credentials are not copied into builder layers. Operators do not need to change their workflow.
