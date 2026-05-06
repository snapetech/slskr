#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

target=""
version="${SLSKR_RELEASE_VERSION:-}"
profile="release"
build_web=1

usage() {
  cat <<'USAGE'
usage: scripts/build-release-archive.sh [--target <rust-target>] [--version <version>] [--skip-web-build]

Builds the slskr binary and creates a release archive containing:
  - slskr executable
  - web/build runtime assets
  - README, LICENSE, NOTICE, COMPLIANCE
  - docs/slskr.config.example.toml

Environment:
  SLSKR_RELEASE_VERSION     default release version if --version is omitted
  SLSKR_SKIP_WEB_BUILD=1    equivalent to --skip-web-build
USAGE
}

write_sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file"
  elif command -v openssl >/dev/null 2>&1; then
    local digest
    digest="$(openssl dgst -sha256 -r "$file")"
    printf '%s\n' "$digest"
  else
    echo "no SHA-256 command found; install sha256sum, shasum, or openssl" >&2
    return 1
  fi
}

while (($# > 0)); do
  case "$1" in
    --target)
      target="${2:?missing --target value}"
      shift 2
      ;;
    --version)
      version="${2:?missing --version value}"
      shift 2
      ;;
    --skip-web-build)
      build_web=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "${SLSKR_SKIP_WEB_BUILD:-0}" == "1" ]]; then
  build_web=0
fi

if [[ -z "$version" ]]; then
  version="$(git describe --tags --always --dirty 2>/dev/null || git rev-parse --short HEAD)"
fi
safe_version="$(printf '%s' "$version" | tr '/ :' '---')"

if [[ -z "$target" ]]; then
  target="$(rustc -Vv | awk '/^host:/ { print $2 }')"
fi

binary_name="slskr"
if [[ "$target" == *windows* ]]; then
  binary_name="slskr.exe"
fi

if ((build_web)); then
  npm --prefix web ci
  npm --prefix web run build
fi
node web/scripts/verify-build-output.mjs

cargo_args=(build --release -p slskr)
if [[ -n "$target" ]]; then
  cargo_args+=(--target "$target")
fi
cargo "${cargo_args[@]}"

binary_path="target/$target/$profile/$binary_name"
if [[ ! -f "$binary_path" ]]; then
  binary_path="target/$profile/$binary_name"
fi
if [[ ! -f "$binary_path" ]]; then
  echo "built binary not found for target $target" >&2
  exit 1
fi

dist_dir="target/dist"
root_name="slskr-$safe_version-$target"
stage_dir="$dist_dir/$root_name"
rm -rf "$stage_dir"
mkdir -p "$stage_dir"

cp "$binary_path" "$stage_dir/$binary_name"
cp README.md LICENSE NOTICE COMPLIANCE.md "$stage_dir/"
mkdir -p "$stage_dir/docs" "$stage_dir/web"
cp docs/slskr.config.example.toml "$stage_dir/docs/"
cp -R web/build "$stage_dir/web/build"

cat > "$stage_dir/RUN.txt" <<EOF
slskr $version ($target)

Run from this directory:

  ./$binary_name serve

The bundled web assets are expected at ./web/build. Configure with
SLSKR_CONFIG=/path/to/config.toml or environment variables. Start from
docs/slskr.config.example.toml.
EOF

mkdir -p "$dist_dir"
if [[ "$target" == *windows* ]]; then
  archive="$dist_dir/$root_name.zip"
  ARCHIVE="$archive" ROOT_NAME="$root_name" DIST_DIR="$dist_dir" python - <<'PY'
import os
import pathlib
import zipfile

archive = pathlib.Path(os.environ["ARCHIVE"])
root = pathlib.Path(os.environ["DIST_DIR"]) / os.environ["ROOT_NAME"]
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in root.rglob("*"):
        if path.is_file():
            zf.write(path, path.relative_to(root.parent).as_posix())
PY
else
  archive="$dist_dir/$root_name.tar.gz"
  tar -C "$dist_dir" -czf "$archive" "$root_name"
fi

write_sha256_file "$archive" > "$archive.sha256"
printf '%s\n' "$archive"
