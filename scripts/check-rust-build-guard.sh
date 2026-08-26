#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0
if [[ ! -x scripts/with-build-guard.sh ]]; then
  printf 'Rust build guard check failed: scripts/with-build-guard.sh is missing or not executable\n' >&2
  status=1
fi
if ! rg -q '^jobs = 1$' .cargo/config.toml; then
  printf 'Rust build guard check failed: .cargo/config.toml must set jobs = 1\n' >&2
  status=1
fi
if ! rg -q '^rustc-wrapper = "scripts/with-build-guard.sh"$' .cargo/config.toml; then
  printf 'Rust build guard check failed: .cargo/config.toml must configure the compiler wrapper\n' >&2
  status=1
fi
if ! rg -q '^RUST_TEST_THREADS = \{ value = "1", force = true \}$' .cargo/config.toml; then
  printf 'Rust build guard check failed: .cargo/config.toml must force one test thread\n' >&2
  status=1
fi
if ! rg -q 'export RUST_TEST_THREADS=1' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must force one test thread\n' >&2
  status=1
fi
if ! rg -q 'build_guard_context_active=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: externally supplied nesting markers must be context-validated\n' >&2
  status=1
fi
if ! rg -q 'process_memory_guard_context_active=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: process-memory nesting markers must be context-validated\n' >&2
  status=1
fi
if ! rg -q 'export CARGO_PROFILE_DEV_DEBUG=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must disable dev debug info by default\n' >&2
  status=1
fi
if ! rg -q 'export CARGO_PROFILE_DEV_CODEGEN_UNITS=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must bound production codegen units by default\n' >&2
  status=1
fi
if ! rg -q 'export CARGO_PROFILE_TEST_DEBUG=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must disable test debug info by default\n' >&2
  status=1
fi
if ! rg -q 'export CARGO_PROFILE_TEST_CODEGEN_UNITS=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must bound test codegen units\n' >&2
  status=1
fi
if ! rg -q 'export CARGO_PROFILE_TEST_LTO=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must disable test LTO by default\n' >&2
  status=1
fi
if ! rg -q 'export CARGO_INCREMENTAL=' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper must disable incremental compilation by default\n' >&2
  status=1
fi
if ! rg -q '^max_virtual_memory_kib=12582912$' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the wrapper hard ceiling must be 12 GiB\n' >&2
  status=1
fi
if ! rg -q 'cargo fmt is disabled for this repository' scripts/with-build-guard.sh \
  || ! rg -q 'scripts/check-rust-format\.sh' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: cargo fmt must be rejected in favor of the bounded formatter checker\n' >&2
  status=1
fi
if [[ ! -x scripts/with-rustfmt-guard.sh ]]; then
  printf 'Rust build guard check failed: direct rustfmt wrapper is missing or not executable\n' >&2
  status=1
fi
if ! rg -q 'SLSKR_PROCESS_MEMORY_MAX_KIB=1048576' scripts/with-rustfmt-guard.sh \
  || ! rg -q 'rustfmt --check is disabled' scripts/with-rustfmt-guard.sh; then
  printf 'Rust build guard check failed: direct rustfmt must use the 1 GiB bounded emit path\n' >&2
  status=1
fi
for shim_file in scripts/rust-tool-shim.sh scripts/install-rust-tool-shims.sh scripts/test-rust-tool-shims.sh; do
  if [[ ! -x "$shim_file" ]]; then
    printf 'Rust build guard check failed: Rust tool shim is missing or not executable: %s\n' "$shim_file" >&2
    status=1
  fi
done
if ! rg -q 'with-build-guard\.sh.*cargo' scripts/rust-tool-shim.sh \
  || ! rg -q 'with-rustfmt-guard\.sh' scripts/rust-tool-shim.sh; then
  printf 'Rust build guard check failed: workstation Rust shims must route repository commands through guards\n' >&2
  status=1
fi
if ! rg -q 'full-controller-tests is rejected under the 12 GiB memory-safe profile' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the known monolithic controller test profile must be rejected\n' >&2
  status=1
fi
if ! rg -q 'legacy-route-dispatch is rejected' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: the monolithic legacy route dispatcher must be rejected\n' >&2
  status=1
fi
if ! rg -q -- '--all-features' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: all-features test requests must be covered by the monolithic-profile rejection\n' >&2
  status=1
fi
if ! rg -q 'SLSKR_ALLOW_FULL_CONTROLLER_TESTS' scripts/with-build-guard.sh \
  || ! rg -q 'SLSKR_PROCESS_MEMORY_GUARD_HELD' scripts/with-build-guard.sh; then
  printf 'Rust build guard check failed: full-controller-tests opt-in must require the process-memory guard\n' >&2
  status=1
fi
if ! rg -q '^ulimit -v "\$interop_virtual_memory_kib"$' scripts/run-live-interop-matrix.sh; then
  printf 'Rust build guard check failed: the live interop launcher must enforce the 12 GiB ceiling before reuse or build\n' >&2
  status=1
fi
while IFS= read -r -d '' file; do
  while IFS= read -r match; do
    case "$match" in
      *'cargo fmt is disabled'*|*'cargo fmt must be rejected'*|*'raw cargo fmt'*)
        # These are guard diagnostics and regression assertions, not command
        # invocations. Keep the scan strict for all other Cargo subcommands.
        continue
        ;;
    esac
    [[ "$match" == *"with-build-guard.sh"* ]] && continue
    printf 'unguarded Rust command in %s: %s\n' "$file" "$match" >&2
    status=1
  done < <(
    rg -n --pcre2 '(^|[;&|(:`[:space:]])cargo[[:space:]]+(build|check|test|clippy|fmt|run|package|metadata|tree|audit|install|bench)([[:space:]]|$|[`])' "$file" || true
  )
done < <(
  find .github scripts docs -type f \( -name '*.sh' -o -name '*.yml' -o -name '*.yaml' -o -name '*.md' \) -print0
  printf '%s\0' Dockerfile packaging/aur/PKGBUILD
  printf '%s\0' .gitlab-ci.yml
  printf '%s\0' PLAN.md README.md COMPLIANCE.md REMEDIATION.md
  find web/scripts -type f \( -name '*.js' -o -name '*.mjs' \) -print0
)

# Adding the bounded differential binary means the slskr Cargo runner is no
# longer self-selecting. Keep every production invocation explicit so a
# proof-runner addition cannot silently break certification, smoke, or daemon
# launch scripts.
if rg -n --pcre2 \
  'cargo[[:space:]]+run([[:space:]]+-q)?[[:space:]]+-p[[:space:]]+slskr[[:space:]]+--(?!bin[[:space:]]+slskr([[:space:]]|$))' \
  .github scripts docs README.md PLAN.md crates/slskr-web/README.md 2>/dev/null; then
  printf 'Rust build guard check failed: production slskr invocations must select --bin slskr explicitly\n' >&2
  status=1
fi

# Catch direct child-process Cargo launches in Node/Python helpers, including
# multi-line subprocess argument lists in shell here-docs. These bypass the
# shell-command scan above, so keep them on the same guard path as Cargo CLI
# entrypoints.
while IFS= read -r -d '' file; do
  for pattern in \
    'subprocess\.(run|check_call|check_output|Popen)\(\s*\[\s*"cargo"' \
    "subprocess\\.(run|check_call|check_output|Popen)\\(\\s*\\[\\s*'cargo'" \
    '(spawn|spawnSync|exec|execSync|execFile|execFileSync)\(\s*"cargo"' \
    "(spawn|spawnSync|exec|execSync|execFile|execFileSync)\\(\\s*'cargo'"; do
    while IFS= read -r match; do
      printf 'unguarded Cargo child process in %s: %s\n' "$file" "$match" >&2
      status=1
    done < <(rg -n -U --pcre2 "$pattern" "$file" || true)
  done
done < <(
  find scripts web/scripts -type f \( -name '*.sh' -o -name '*.py' -o -name '*.js' -o -name '*.mjs' \) -print0
)

if [[ "$status" -ne 0 ]]; then
  printf 'Rust build guard check failed. Route every Cargo subcommand through scripts/with-build-guard.sh.\n' >&2
  exit "$status"
fi

printf 'Rust build guard static check passed\n'
