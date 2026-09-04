#!/usr/bin/env bash
set -e

# Run as root inside a Debian/Ubuntu Proxmox LXC.
VERSION="${SLSKR_VERSION:-}"
DEST="/opt/slskr"
USER="slskr"
DATA_DIR="/var/lib/slskr"
CONFIG_DIR="/etc/slskr"
CONFIG_FILE="${CONFIG_DIR}/config.toml"

if [ "$(id -u)" -ne 0 ]; then
  echo "Run this setup script as root." >&2
  exit 1
fi

apt-get update -qq
apt-get install -y -qq ca-certificates curl tar

if [ -z "$VERSION" ]; then
  VERSION="$(curl --fail --silent --show-error https://api.github.com/repos/snapetech/slskr/releases/latest \
    | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)"
fi
[ -n "$VERSION" ] || { echo "Unable to resolve a slskR release." >&2; exit 1; }

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
curl --fail --location --output "${WORK_DIR}/${ARCHIVE}" \
  "https://github.com/snapetech/slskr/releases/download/${VERSION}/${ARCHIVE}"
curl --fail --location --output "${WORK_DIR}/SHA256SUMS.txt" \
  "https://github.com/snapetech/slskr/releases/download/${VERSION}/SHA256SUMS.txt"
grep -Eq "^[0-9a-fA-F]{64}[[:space:]]+\*?${ARCHIVE//./\\.}$" "${WORK_DIR}/SHA256SUMS.txt" \
  || { echo "Release checksum does not cover ${ARCHIVE}." >&2; exit 1; }
(cd "$WORK_DIR" && sha256sum --check --ignore-missing SHA256SUMS.txt)

id -u "$USER" >/dev/null 2>&1 || useradd --system --home-dir "$DATA_DIR" --shell /usr/sbin/nologin "$USER"
mkdir -p "$DEST" "$DATA_DIR" "$CONFIG_DIR"
tar -xzf "${WORK_DIR}/${ARCHIVE}" -C "$DEST"
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

cat > /etc/systemd/system/slskr.service <<SVC
[Unit]
Description=slskR Soulseek daemon in Proxmox LXC
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
echo "Installed slskR ${VERSION}; start with systemctl start slskr.service."
