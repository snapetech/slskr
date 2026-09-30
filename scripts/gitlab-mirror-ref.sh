#!/bin/sh
set -eu

: "${GITHUB_MIRROR_URL:?GITHUB_MIRROR_URL is required}"
git remote add github-mirror "$GITHUB_MIRROR_URL"

if [ -n "${CI_COMMIT_TAG:-}" ]; then
    git push github-mirror "refs/tags/$CI_COMMIT_TAG:refs/tags/$CI_COMMIT_TAG"
else
    : "${CI_COMMIT_REF_NAME:?CI_COMMIT_REF_NAME is required for branch mirroring}"
    git push github-mirror "HEAD:$CI_COMMIT_REF_NAME"
fi
