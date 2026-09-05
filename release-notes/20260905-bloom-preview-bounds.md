---
category: security
audience: operators
area: musicbrainz-bloom
action: none
breaking: false
---
MusicBrainz Bloom preview requests now reject filter parameters that would exceed the daemon's bounded allocation budget, preventing extreme precision requests from consuming excessive memory.
