# Council Bughunt Playbook

This is the entrypoint for agents. Do not report `check-remediation-baseline.sh` as "the council found no bugs." That command is a regression gate. It proves prior fixes and process anchors are still wired.

## Run A Real Council Cycle

```sh
scripts/run-council-bughunt.sh
```

The bughunt entrypoint does four different things:

1. Regenerates fresh broad candidate counts with `scripts/run-council-scan.sh`.
2. Runs council process gates: `check-council-loop.sh` and `check-council-negative-space.sh`.
3. Runs calibrated semantic lenses, starting with `check-rust-protocol-taint-lens.sh`.
4. Prints pending bughunt phases from `docs/dev/bug-council-phases.md` and exits nonzero while deeper phases remain.

## How To Interpret Results

- Candidate counts are not bugs.
- Green gates are not a bughunt adjudication.
- A semantic lens finding is an unadjudicated candidate until it is either fixed or recorded in `docs/dev/council-scan-inventory.md` with evidence.
- A zero-finding semantic lens only means something if its calibration bad fixture fires and its good fixture stays silent.

## Burn-Down Rule

When a class has candidates:

1. Choose one whole candidate class from `docs/dev/council-scan-inventory.md`.
2. Convert fresh scan hits for that class into table rows.
3. Classify every row as `Accepted`, `Existing Guard`, `False Positive`, or `Out of scope`.
4. Add ledger rows for every `Accepted` bug.
5. Fix all `Accepted` rows for that class.
6. Add behavior tests and remediation gates.
7. Run sibling search before closing the class.

Do not stop after one confirmed bug unless the whole class has been adjudicated.

## Commit Wording

Fix commits must describe the slskR change, bug class, or user-visible
hardening. Do not mention council, bughunt, scanners, agents, or other discovery tooling in commit messages. The inventory and ledger can record how a
bug was found; commit history should read as normal maintenance and fix history.
