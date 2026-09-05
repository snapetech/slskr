---
category: security
audience: users, operators
area: audio-verification
action: none
breaking: false
---
Audio verification no longer reuses a persisted fingerprint for a different browser `File` that happens to share filename and metadata. Repeated checks of the same immutable file object remain cached, while persisted records are bounded and retained only as an audit trail.
