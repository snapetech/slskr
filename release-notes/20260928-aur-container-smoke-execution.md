---
category: fixed
audience: operators
area: package-validation
action: Rerun the AUR smoke if its only receipt came from the Docker fallback.
breaking: false
---
Both AUR Docker smoke scripts now execute through open stdin, run in the copied package directory, and verify/extract sources with preparation enabled. Failed makepkg commands cannot emit success. Cleanup targets only the container created by the smoke.
