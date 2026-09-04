#!/usr/bin/env bash
set -euo pipefail

VERSION="${SLSKR_VERSION:-}"
DEST="${SLSKR_DEST:-/opt/slskr}"
USER="${SLSKR_USER:-slskr}"
DATA_DIR="${SLSKR_DATA_DIR:-/var/lib/slskr}"
CONFIG_DIR="${SLSKR_CONFIG_DIR:-/etc/slskr}"
CONFIG_FILE="${CONFIG_DIR}/config.toml"
SERVICE_FILE="/etc/systemd/system/slskr.service"

if [ "$(id -u)" -ne 0 ]; then
  echo "Run this installer as root." >&2
  exit 1
fi

resolve_version() {
  if [ -n "$VERSION" ]; then
    printf '%s\n' "$VERSION"
    return
  fi
  curl --fail --silent --show-error https://api.github.com/repos/snapetech/slskr/releases/latest \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1
}

VERSION="$(resolve_version)"
if [ -z "$VERSION" ]; then
  echo "Unable to resolve a slskR release." >&2
  exit 1
fi

case "$(uname -m)" in
  x86_64|amd64) rust_arch="x86_64" ;;
  aarch64|arm64) rust_arch="aarch64" ;;
  *)
    echo "Unsupported Linux architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

ARCHIVE="slskr-${VERSION#release-v}-${rust_arch}-unknown-linux-gnu.tar.gz"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
ARCHIVE_PATH="${WORK_DIR}/${ARCHIVE}"
CHECKSUMS_PATH="${WORK_DIR}/SHA256SUMS.txt"

curl --fail --location --output "$ARCHIVE_PATH" \
  "https://github.com/snapetech/slskr/releases/download/${VERSION}/${ARCHIVE}"
curl --fail --location --output "$CHECKSUMS_PATH" \
  "https://github.com/snapetech/slskr/releases/download/${VERSION}/SHA256SUMS.txt"
grep -Eq "^[0-9a-fA-F]{64}[[:space:]]+\*?${ARCHIVE//./\\.}$" "$CHECKSUMS_PATH" \
  || { echo "Release checksum does not cover ${ARCHIVE}." >&2; exit 1; }
(cd "$WORK_DIR" && sha256sum --check --ignore-missing SHA256SUMS.txt)

apt-get update -qq
apt-get install -y -qq ca-certificates curl tar
id -u "$USER" >/dev/null 2>&1 || useradd --system --home-dir "$DATA_DIR" --shell /usr/sbin/nologin "$USER"
mkdir -p "$DEST" "$DATA_DIR" "$CONFIG_DIR"
tar -xzf "$ARCHIVE_PATH" -C "$DEST"
chown -R "${USER}:${USER}" "$DEST" "$DATA_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
  cat > "$CONFIG_FILE" <<'CFG'
[web]
port = 5030

[soulseek]
username = ""
password = ""
CFG
fi
chown "${USER}:${USER}" "$CONFIG_FILE"

cat > "$SERVICE_FILE" <<SVC
[Unit]
Description=slskR Soulseek daemon
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=${USER}
Group=${USER}
WorkingDirectory=${DATA_DIR}
ExecStart=${DEST}/slskr serve --config ${CONFIG_FILE}
Restart=on-failure
RestartSec=5s
ReadWritePaths=${DATA_DIR} ${CONFIG_DIR}

[Install]
WantedBy=multi-user.target
SVC

systemctl stop slskr.service >/dev/null 2>&1 || true
systemctl daemon-reload
systemctl enable slskr.service
echo "Installed slskR ${VERSION}; configure ${CONFIG_FILE} before starting it."
