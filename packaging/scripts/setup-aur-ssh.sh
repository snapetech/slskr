#!/usr/bin/env bash
set -euo pipefail

key_file="${1:?usage: setup-aur-ssh.sh <private-key-file>}"
mkdir -p ~/.ssh
install -m 600 "$key_file" ~/.ssh/aur
cat >> ~/.ssh/config <<'EOF'
Host aur.archlinux.org
  HostName aur.archlinux.org
  User aur
  IdentityFile ~/.ssh/aur
  IdentitiesOnly yes
  BatchMode yes
  StrictHostKeyChecking accept-new
EOF
ssh-keyscan -H aur.archlinux.org >> ~/.ssh/known_hosts 2>/dev/null || true
