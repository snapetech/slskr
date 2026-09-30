---
category: fixed
audience: users
area: shares
action: none
breaking: false
---
Startup share scans now skip content reads for files outside WAV, FLAC, and MP3 when media probing is enabled. Supported probes use bounded format-specific reads and skip audio payloads and MP3 ID3 tags, reducing unnecessary disk I/O without changing shared file entries.
