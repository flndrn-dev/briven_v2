#!/bin/sh
set -eu
if [ -z "${BRIVEN_CONTROL_IDENTITY_SECRET:-}" ]; then
  echo "missing hall-pass secret" >&2
  exit 1
fi
if [ -z "${BRIVEN_CONTROL_DATABASE_URL:-}" ]; then
  echo "missing catalog database" >&2
  exit 1
fi
export BRIVEN_REPO_DIR=/var/lib/briven
export BRIVEN_ENV=production
export BRIVEN_CONTROL_API_BIND=0.0.0.0:8787
export BRIVEN_LOCAL_BIN="${BRIVEN_LOCAL_BIN:-briven_local}"
mkdir -p "$BRIVEN_REPO_DIR"
if [ -z "$(ls -A "$BRIVEN_REPO_DIR" 2>/dev/null || true)" ]; then
  briven_local init --config /local-env.toml --force empty-dir-ok
fi
briven_local start
exec briven_control_api
