# RF-008 GitLab loose-object recovery verification (2026-09-29)

Internal-only verification. No GitLab settings, refs, or object contents were
changed during this check.

- The project's current `main` commit is
  `5b7109e20cf631a481ac2ff805b7451fb71456d6`.
- Repaired object `272d9b69345294fa1aa98ec3a83a9383c87be9e2` exists in GitLab as
  a 457,048-byte blob. Re-hashing the stored contents with `git hash-object`
  returns the same object ID.
- `git fsck --full --strict --no-reflogs --no-dangling` passes for the project
  repository.
- The original damaged loose-object backup remains present, 103,870 bytes, mode
  `0444`, at the existing 2026-09-28 recovery path. The backup was not read or
  modified during this check.
- GitLab pipeline 155 on `b06c78ec` completed successfully after the repair.

This closes the corrupt Git loose-object incident. Historical GitLab avatar and
job-artifact objects from the retired MinIO store are a separate recovery issue;
the existing operational record states that their source volume backup skipped
the data, so this verification does not claim those objects were recovered.
