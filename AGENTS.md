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

## Rust build and tooling

Use Cargo and the pinned Rust toolchain directly. The workspace config keeps
the large daemon crate on one Cargo build job, disables unnecessary dev debug
metadata, and uses 16 codegen units plus ThinLTO for release builds. Do not add
shell wrappers, compiler shims, virtual-memory limits, or forced test-thread
settings around Rust commands. Run `scripts/check-rust-format.sh` for the
changed-file formatter check; it intentionally leaves the historical
multi-megabyte controller source alone.
