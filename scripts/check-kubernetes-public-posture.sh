#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

status=0

for expected in \
  'replicas: 1' \
  'maxReplicas: 1' \
  'persistentVolumeClaim:' \
  'claimName: slskr-data' \
  'automountServiceAccountToken: false' \
  'runAsNonRoot: true' \
  'runAsGroup: 1000' \
  'seccompProfile:' \
  'readOnlyRootFilesystem: true' \
  'bearerTokenSecret:' \
  'NetworkPolicy' \
  'except:' \
  'imagePullPolicy: Always' \
  'minAvailable: 1'; do
  if ! rg -n -F "$expected" k8s >/dev/null; then
    printf 'kubernetes public posture check failed: expected manifest token missing: %s\n' "$expected" >&2
    status=1
  fi
done

if rg -n 'kind: Role$|kind: RoleBinding$|type: LoadBalancer|nodePort:|slskr-dashboard|emptyDir:\s*$' k8s; then
  printf 'kubernetes public posture check failed: unsafe/default-deprecated manifest token matched above\n' >&2
  status=1
fi

scripts/check-public-posture.sh

if [[ "$status" -ne 0 ]]; then
  exit "$status"
fi

printf 'kubernetes public posture check passed\n'
