#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"
PACKAGE_DIR="${BUILD_DIR}/package"
VERSION="${SLSKR_VERSION:-0.2.38}"
ARCH="${SLSKR_SPK_ARCH:-x86_64}"

case "$ARCH" in
  x86_64|aarch64) ;;
  *) echo "SLSKR_SPK_ARCH must be x86_64 or aarch64, got: $ARCH" >&2; exit 1 ;;
esac

rm -rf "$BUILD_DIR"
mkdir -p "$PACKAGE_DIR"
if [ -n "${SLSKR_SPK_PUBLISH_DIR:-}" ]; then
  cp -a "${SLSKR_SPK_PUBLISH_DIR}"/. "$PACKAGE_DIR"/
else
  echo "Set SLSKR_SPK_PUBLISH_DIR to a self-contained slskr publish directory." >&2
  exit 1
fi
[ -x "$PACKAGE_DIR/slskr" ] || { echo "SPK payload lacks executable slskr." >&2; exit 1; }
sed \
  -e "s/^version=.*/version=\"${VERSION}\"/" \
  -e "s/^arch=.*/arch=\"${ARCH}\"/" \
  "$SCRIPT_DIR/INFO" > "$BUILD_DIR/INFO"
cp -R "$SCRIPT_DIR/scripts" "$BUILD_DIR/scripts"
cp -R "$SCRIPT_DIR/conf" "$BUILD_DIR/conf"
tar -C "$PACKAGE_DIR" -czf "$BUILD_DIR/package.tgz" .
tar -C "$BUILD_DIR" -cf "$BUILD_DIR/slskr.spk" INFO package.tgz scripts conf
