#!/usr/bin/env bash
set -euo pipefail
helm lint "$(dirname "$0")"
