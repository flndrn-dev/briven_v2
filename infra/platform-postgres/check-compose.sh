#!/bin/sh
set -eu
cd "$(dirname "$0")/../.."
file=infra/platform-postgres/compose.yml
test -f "$file"
if grep -nE '^[[:space:]]*ports:|/var/lib/doltgres|traefik|dokploy-network|briven-website-kvm2' "$file"; then
  echo 'refusing live storage or a public route' >&2
  exit 1
fi
grep -q 'internal: true' "$file"
grep -q 'ssl=on' "$file"
grep -q 'fsync=on' "$file"
test "$(grep -c 'max-size:' "$file")" -eq 1
test "$(grep -c 'logging: \*logging' "$file")" -eq 2
grep -q 'BRIVEN_CONTROL_DB_APP_PASSWORD:?' "$file"
grep -q 'BRIVEN_AUTH_DB_APP_PASSWORD:?' "$file"
for script in create-certificates.sh initialize-database.sh; do
  sh -n "infra/platform-postgres/$script"
done
echo 'platform databases have private storage and TLS'
