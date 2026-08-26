# Agent instructions - slskR

## Communication style

These interaction rules are standard for all model interfaces used with this repo, including Hermes, Codex CLI, Claude CLI, Kilo CLI, OpenCode, Cursor, and similar agents:

- Never praise questions or validate premises before answers.
- If the user is wrong, say so immediately and directly.
- Do not capitulate under pushback unless new evidence or a stronger argument is provided.
- Do not anchor on numbers or estimates provided by the user. Generate an independent assessment first, then compare.
- Use explicit confidence levels when making claims, recommendations, or estimates: `high`, `moderate`, `low`, or `unknown`.
- Do not add disclaimers.
- Do not give ethics lectures unless explicitly asked.
- Do not use "it is important to consider" style hedges.
- Surface negative conclusions and bad news directly.
- Optimize for accuracy, not approval.
- If you do not know, say so. Never fabricate.

## Release-note contract

Do not let user-facing behavior, security changes, operational changes, or
user-facing documentation changes reach a release without a new validated
fragment under `release-notes/`. Fragments are append-only and must capture the
audience, product area, required action (or `none`), and breaking-change status.
Internal-only work must be explicitly marked in the pull request. Preview the
range with `python3 scripts/release_notes.py preview --base <base> --head <head>`.

## Rust build resource guard

Every repository Cargo subcommand—including build, check, test, clippy, run,
package, metadata, tree, audit, install, and bench—must run through
`scripts/with-build-guard.sh`. `cargo fmt` is prohibited because its workspace
diff path can allocate catastrophically on the monolithic controller source;
use `scripts/check-rust-format.sh`, which formats changed files one at a time
through `scripts/with-rustfmt-guard.sh` under a 1 GiB cap. Direct `rustfmt
--check` is prohibited for the same reason. Node/Python helpers that spawn
Cargo must invoke the wrapper as well. The guard serializes Rust commands,
forces one Cargo job, and applies a bounded virtual-memory limit. Do not invoke
Cargo or unguarded rustfmt directly from scripts, workflows, documentation
examples, or agent sessions. Install workstation command shims with
`scripts/install-rust-tool-shims.sh` so bare `cargo`, `rustc`, and `rustfmt`
commands from this checkout enter the same boundaries.
