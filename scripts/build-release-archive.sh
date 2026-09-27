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
  - production React web UI assets
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

source_date_epoch="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct HEAD)}"
if ! [[ "$source_date_epoch" =~ ^[0-9]+$ ]]; then
  echo "SOURCE_DATE_EPOCH must be a non-negative Unix timestamp" >&2
  exit 2
fi

if [[ -z "$target" ]]; then
  target="$(rustc -Vv | awk '/^host:/ { print $2 }')"
fi

binary_name="slskr"
if [[ "$target" == *windows* ]]; then
  binary_name="slskr.exe"
fi

if ((build_web)); then
  if [[ ! -x web/node_modules/.bin/vite && ! -x web/node_modules/.bin/vite.cmd ]]; then
    npm --prefix web ci
  fi
  npm --prefix web run build
fi

web_build="${SLSKR_WEB_BUILD_DIR:-web/build}"
if [[ \
  ! -f "$web_build/index.html" || \
  ! -d "$web_build/assets" \
 ]]; then
  echo "Production web assets are missing from $web_build" >&2
  echo "run npm --prefix web run build or unset --skip-web-build" >&2
  exit 1
fi

cargo_args=(build --locked --release -p slskr)
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
cp -R "$web_build" "$stage_dir/web/build"

cat > "$stage_dir/RUN.txt" <<EOF
slskr $version ($target)

Run from this directory:

  ./$binary_name serve

The bundled web assets are expected at ./web/build. Configure with
SLSKR_CONFIG=/path/to/config.toml or environment variables. Start from
docs/slskr.config.example.toml.
EOF

SOURCE_DATE_EPOCH="$source_date_epoch" STAGE_DIR="$stage_dir" python3 - <<'PY'
import os
import pathlib

epoch = int(os.environ["SOURCE_DATE_EPOCH"])
stage = pathlib.Path(os.environ["STAGE_DIR"])
for path in stage.rglob("*"):
    try:
        os.utime(path, (epoch, epoch), follow_symlinks=False)
    except FileNotFoundError:
        pass
PY

mkdir -p "$dist_dir"
if [[ "$target" == *windows* ]]; then
  archive="$dist_dir/$root_name.zip"
  ARCHIVE="$archive" ROOT_NAME="$root_name" DIST_DIR="$dist_dir" SOURCE_DATE_EPOCH="$source_date_epoch" python3 - <<'PY'
import datetime
import os
import pathlib
import zipfile

archive = pathlib.Path(os.environ["ARCHIVE"])
root = pathlib.Path(os.environ["DIST_DIR"]) / os.environ["ROOT_NAME"]
epoch = max(int(os.environ["SOURCE_DATE_EPOCH"]), 315532800)
timestamp = datetime.datetime.fromtimestamp(epoch, datetime.timezone.utc).replace(tzinfo=None)
with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        name = path.relative_to(root.parent).as_posix()
        info = zipfile.ZipInfo(name, timestamp)
        info.compress_type = zipfile.ZIP_DEFLATED
        info.create_system = 3
        info.external_attr = 0o100644 << 16
        zf.writestr(info, path.read_bytes())
PY
else
  archive="$dist_dir/$root_name.tar.gz"
  ARCHIVE="$archive" ROOT_NAME="$root_name" DIST_DIR="$dist_dir" \
    BINARY_NAME="$binary_name" SOURCE_DATE_EPOCH="$source_date_epoch" python3 - <<'PY'
import gzip
import os
import pathlib
import stat
import tarfile

archive = pathlib.Path(os.environ["ARCHIVE"])
root = pathlib.Path(os.environ["DIST_DIR"]) / os.environ["ROOT_NAME"]
binary = root / os.environ["BINARY_NAME"]
epoch = int(os.environ["SOURCE_DATE_EPOCH"])
with archive.open("wb") as output:
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0, compresslevel=9) as gz:
        with tarfile.open(fileobj=gz, mode="w", format=tarfile.PAX_FORMAT) as tf:
            for path in [root, *sorted(root.rglob("*"))]:
                metadata = path.lstat()
                info = tarfile.TarInfo(path.relative_to(root.parent).as_posix())
                info.uid = info.gid = 0
                info.uname = info.gname = ""
                info.mtime = epoch
                if stat.S_ISDIR(metadata.st_mode):
                    info.type = tarfile.DIRTYPE
                    info.mode = 0o755
                    tf.addfile(info)
                elif stat.S_ISLNK(metadata.st_mode):
                    info.type = tarfile.SYMTYPE
                    info.mode = 0o777
                    info.linkname = os.readlink(path)
                    tf.addfile(info)
                elif stat.S_ISREG(metadata.st_mode):
                    info.mode = 0o755 if path == binary else 0o644
                    info.size = metadata.st_size
                    with path.open("rb") as content:
                        tf.addfile(info, content)
                else:
                    raise RuntimeError(f"unsupported release archive entry: {path}")
PY
fi

write_sha256_file "$archive" > "$archive.sha256"
printf '%s\n' "$archive"
