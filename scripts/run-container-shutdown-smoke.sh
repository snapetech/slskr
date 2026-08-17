#!/usr/bin/env bash
set -euo pipefail

image="${1:?usage: $0 <image>}"
container_name="slskr-shutdown-smoke-${GITHUB_RUN_ID:-local}-$$"

cleanup() {
  docker rm -f "$container_name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker pull "$image"
docker run -d \
  --name "$container_name" \
  -e SLSKR_STATE_DIR=/var/lib/slskr \
  -e SLSKR_HTTP_BIND=127.0.0.1:0 \
  -e SLSKR_AUTH_DISABLED=true \
  "$image" serve --no-connect --no-share-scan --no-logo --no-version-check >/dev/null

ready=0
for _ in {1..60}; do
  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
  if [[ "$state" != "running" ]]; then
    break
  fi
  if docker logs "$container_name" 2>&1 | rg -q 'listening on http://'; then
    ready=1
    break
  fi
  sleep 1
done

if [[ "$ready" -ne 1 ]]; then
  echo "container did not become ready: $image" >&2
  docker logs "$container_name" 2>&1 || true
  exit 1
fi

docker kill --signal=TERM "$container_name" >/dev/null
for _ in {1..30}; do
  state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
  if [[ "$state" == "exited" || "$state" == "dead" ]]; then
    break
  fi
  sleep 1
done

state="$(docker inspect -f '{{.State.Status}}' "$container_name" 2>/dev/null || true)"
if [[ "$state" != "exited" && "$state" != "dead" ]]; then
  echo "container did not stop after SIGTERM: $image" >&2
  docker logs "$container_name" 2>&1 || true
  exit 1
fi

exit_code="$(docker inspect -f '{{.State.ExitCode}}' "$container_name")"
if [[ "$exit_code" != "0" ]]; then
  echo "container exited with $exit_code after SIGTERM; expected 0: $image" >&2
  docker logs "$container_name" 2>&1 || true
  exit 1
fi

if ! docker logs "$container_name" 2>&1 | rg -q 'shutdown signal received'; then
  echo "container stopped without recording graceful signal handling: $image" >&2
  docker logs "$container_name" 2>&1 || true
  exit 1
fi

echo "container shutdown smoke passed: $image"
