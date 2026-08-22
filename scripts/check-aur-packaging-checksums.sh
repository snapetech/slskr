#!/usr/bin/env bash
set -euo pipefail

# Regression guard for the class of bug reported at
# https://github.com/snapetech/slskr/pull/58#issuecomment-5330870207:
# a locally-hashed AUR source file (packaging/aur/*.service, *.sysusers,
# *.tmpfiles, ...) was edited without updating the sha256sums= entry that
# PKGBUILD/PKGBUILD-bin declare for it, which makes makepkg fail outright.
# This check re-derives each declared checksum from the file actually
# checked into the repo and fails loudly on drift, on a missing file, or
# on a source()/sha256sums() length mismatch. It also catches an
# install= line pointing at a file that no longer exists.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

check_pkgbuild() {
  local pkgbuild="$1"
  local pkg_dir
  pkg_dir="$(cd "$(dirname "$pkgbuild")" && pwd)"

  local report
  if report="$(
    bash -c '
      set -euo pipefail
      pkgbuild="$1"
      pkg_dir="$2"
      # shellcheck disable=SC1090
      source "$pkgbuild"

      if [[ "${#sha256sums[@]}" -ne "${#source[@]}" ]]; then
        printf "ARRAY_MISMATCH\t%s\t%s\n" "${#source[@]}" "${#sha256sums[@]}"
      fi

      exit_status=0
      for i in "${!source[@]}"; do
        entry="${source[$i]}"
        sum="${sha256sums[$i]:-}"
        name="${entry##*::}"
        [[ "$entry" == *://* ]] && continue
        [[ "$sum" == "SKIP" ]] && continue
        file="$pkg_dir/$name"
        if [[ ! -f "$file" ]]; then
          printf "MISSING\t%s\n" "$name"
          exit_status=1
          continue
        fi
        actual="$(sha256sum "$file" | cut -d" " -f1)"
        if [[ "$actual" != "$sum" ]]; then
          printf "MISMATCH\t%s\t%s\t%s\n" "$name" "${sum:-<none>}" "$actual"
          exit_status=1
        fi
      done

      if [[ -n "${install:-}" && ! -f "$pkg_dir/$install" ]]; then
        printf "MISSING_INSTALL\t%s\n" "$install"
        exit_status=1
      fi

      exit "$exit_status"
    ' _ "$pkgbuild" "$pkg_dir"
  )"; then
    :
  else
    status=1
  fi

  while IFS=$'\t' read -r kind a b c; do
    [[ -z "$kind" ]] && continue
    case "$kind" in
      MISMATCH)
        printf 'AUR packaging checksum check failed: %s declares sha256 %s for %s but the file on disk hashes to %s\n' \
          "$pkgbuild" "$b" "$a" "$c" >&2
        status=1
        ;;
      MISSING)
        printf 'AUR packaging checksum check failed: %s lists %s in source() but the file is missing from %s\n' \
          "$pkgbuild" "$a" "$pkg_dir" >&2
        status=1
        ;;
      MISSING_INSTALL)
        printf 'AUR packaging checksum check failed: %s sets install=%s but that file is missing from %s\n' \
          "$pkgbuild" "$a" "$pkg_dir" >&2
        status=1
        ;;
      ARRAY_MISMATCH)
        printf 'AUR packaging checksum check failed: %s has %s source() entries but %s sha256sums() entries\n' \
          "$pkgbuild" "$a" "$b" >&2
        status=1
        ;;
    esac
  done <<<"$report"
}

check_pkgbuild packaging/aur/PKGBUILD
check_pkgbuild packaging/aur/PKGBUILD-bin

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'AUR packaging checksum check passed\n'
