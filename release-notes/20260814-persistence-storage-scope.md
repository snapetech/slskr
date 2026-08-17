---
category: changed
audience: operators
area: release-validation
action: none
breaking: false
---

Parity validation now records the SongID run store's real upsert/read-only
contract, rejects malformed persisted SongID state during startup, and treats
target-specific share, catalogue, ActivityPub, cache, discovery, and download
request tables as storage-layout differences when their public behavior is
covered by Rust-native bounded stores.
