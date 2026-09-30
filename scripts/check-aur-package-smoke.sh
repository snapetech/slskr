#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
aur_root="$repo_root/packaging/aur"

if [[ ! -f "$aur_root/PKGBUILD" || ! -f "$aur_root/PKGBUILD-bin" ]]; then
  printf 'AUR package smoke failed: expected both PKGBUILD files under %s\n' "$aur_root" >&2
  exit 1
fi

run_makepkg() {
  local work_root="$1"
  shift
  (
    cd "$work_root"
    makepkg --nobuild --nodeps --clean --cleanbuild "$@"
  )
}

if command -v makepkg >/dev/null 2>&1; then
  work_root="$(mktemp -d -t slskr-aur-smoke.XXXXXX)"
  trap 'find "$work_root" -mindepth 1 -delete' EXIT
  cp -a "$aur_root/." "$work_root/"
  run_makepkg "$work_root" -p PKGBUILD
  run_makepkg "$work_root" -p PKGBUILD-bin
  printf 'AUR package smoke passed with host makepkg\n'
  exit 0
fi

if ! command -v docker >/dev/null 2>&1; then
  printf 'AUR package smoke failed: makepkg and docker are both unavailable\n' >&2
  exit 1
fi

container_work_root="$(mktemp -d -t slskr-aur-container.XXXXXX)"
container_id_file="$container_work_root/container-id"
cleanup_container() {
  local container_id=''
  if [[ -s "$container_id_file" ]]; then
    IFS= read -r container_id < "$container_id_file" || true
    if [[ "$container_id" =~ ^[0-9a-f]{64}$ ]]; then
      docker rm --force "$container_id" >/dev/null 2>&1 || true
    fi
  fi
  rm -rf -- "$container_work_root"
}
trap cleanup_container EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

docker run --rm -i --cidfile "$container_id_file" \
  -v "$repo_root:/repo:ro" \
  archlinux:base-devel \
  bash -s -- <<'CONTAINER_SCRIPT'
set -euo pipefail

pacman -Sy --noconfirm --needed sudo >/dev/null
useradd --create-home builder
work_root="$(mktemp -d -p /tmp slskr-aur-smoke.XXXXXX)"
trap 'rm -rf "$work_root"' EXIT
cp -a /repo/packaging/aur/. "$work_root/"
chown -R builder:builder "$work_root"
cd "$work_root"
runuser -u builder -- makepkg --nobuild --nodeps --clean --cleanbuild -C -p PKGBUILD
runuser -u builder -- makepkg --nobuild --nodeps --clean --cleanbuild -C -p PKGBUILD-bin
CONTAINER_SCRIPT

printf 'AUR package smoke passed in archlinux:base-devel\n'
