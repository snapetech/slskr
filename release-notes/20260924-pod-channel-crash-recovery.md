---
category: changed
audience: operators
area: persistence
action: none
breaking: false
---
Pod updates and deletions now persist pod state before removing channel
messages. Startup prunes messages for missing pods or channels and logs the
recovery count if a process stops between those writes. Channel appends
recheck pod membership while holding the channel store, so queued messages
cannot recreate rows for a channel removed by an earlier update.
