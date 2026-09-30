#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
file=infra/private-back-room/compose.yml
start=infra/private-back-room/start-private.sh
config=infra/private-back-room/local-env.toml
test -f "$file"
test -f "$start"
test -f "$config"
if grep -nE '^[[:space:]]*ports:' "$file"; then
  echo "refusing published ports" >&2
  exit 1
fi
if grep -nE 'minio/minio|minio/mc' "$file"; then
  echo "refusing deleted MinIO images" >&2
  exit 1
fi
if grep -nE 'MINIO_ROOT_PASSWORD=password|PGPASSWORD=cloud_admin' "$file" "$start"; then
  echo "refusing a practice password" >&2
  exit 1
fi
if grep -nE 'traefik|Host\(`briven\.tech`\)' "$file"; then
  echo "refusing a public route" >&2
  exit 1
fi
if grep -nE '/var/lib/doltgres' "$file" "$start"; then
  echo "refusing the live database volume" >&2
  exit 1
fi
grep -q 'bitnamilegacy/minio:latest' "$file"
grep -q 'bitnamilegacy/minio-client:latest' "$file"
grep -q 'BRIVEN_ENGINE_TAG:?set the verified' "$file"
grep -q 'internal: true' "$file"
grep -q 'briven_local init --config /local-env.toml --force empty-dir-ok' "$start"
grep -q 'briven_local start' "$start"
test "$(grep -c '^\[\[safekeepers\]\]' "$config")" -eq 3
grep -q 'no_sync = false' "$config"
grep -q 'AWS_ACCESS_KEY_ID:' "$file"
grep -q 'AWS_SECRET_ACCESS_KEY:' "$file"
grep -q 'listen_addresses=' "$file"
grep -q 'host=/var/run/postgresql' "$file"
test "$(grep -c 'remote_storage = ' "$config")" -eq 4
grep -q './local-env.toml:/local-env.toml:ro' "$file"
grep -q 'briven_control_api' "$start"
echo "private room file is closed"
