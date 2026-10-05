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
# briven_local's upstream test harness defaults its controller catalog to
# fsync=off. Persist a production override before exposing the private API.
controller_psql=/usr/local/v16/bin/psql
"$controller_psql" -X -v ON_ERROR_STOP=1 -h 127.0.0.1 -p 5432 -U neon -d storage_controller \
  -c 'ALTER SYSTEM SET fsync = on' >/dev/null
"$controller_psql" -X -v ON_ERROR_STOP=1 -h 127.0.0.1 -p 5432 -U neon -d storage_controller \
  -c 'SELECT pg_reload_conf()' >/dev/null
test "$("$controller_psql" -X -At -h 127.0.0.1 -p 5432 -U neon -d storage_controller -c 'SHOW fsync')" = on
exec briven_control_api
