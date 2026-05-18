#!/usr/bin/env bash
set -euo pipefail

slskr_release_version_from_tag() {
  local tag="$1"
  case "$tag" in
    release-v*) printf '%s\n' "${tag#release-}" ;;
    v*) printf '%s\n' "$tag" ;;
    *) printf 'v%s\n' "$tag" ;;
  esac
}

slskr_pkgver_from_tag() {
  local release_version
  release_version="$(slskr_release_version_from_tag "$1")"
  printf '%s\n' "${release_version#v}"
}

slskr_release_tag_from_version() {
  local version="$1"
  version="${version#release-}"
  version="${version#v}"
  printf 'release-v%s\n' "$version"
}

slskr_sha256() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{ print $1 }'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{ print $1 }'
  else
    openssl dgst -sha256 -r "$file" | awk '{ print $1 }'
  fi
}

slskr_wait_for_asset() {
  local tag="$1"
  local asset="$2"
  local output="$3"
  local repo="${GITHUB_REPOSITORY:-snapetech/slskr}"
  local url="https://github.com/${repo}/releases/download/${tag}/${asset}"
  local auth_args=()
  if [[ -n "${GH_TOKEN:-}" ]]; then
    auth_args=(-H "Authorization: Bearer ${GH_TOKEN}")
  fi

  for attempt in {1..30}; do
    echo "Attempt ${attempt}: checking ${asset}"
    if curl -fsSL "${auth_args[@]}" -H "Accept: application/octet-stream" "$url" -o "$output"; then
      return 0
    fi
    sleep 20
  done

  echo "release asset not available after waiting: ${url}" >&2
  return 1
}
