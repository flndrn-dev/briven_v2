#!/bin/sh
set -eu
: "${BRIVEN_DB_APP_USER:?missing application user}"
: "${BRIVEN_DB_APP_PASSWORD:?missing application password}"
: "${BRIVEN_DB_APP_NAME:?missing application database}"
case "$BRIVEN_DB_APP_USER:$BRIVEN_DB_APP_NAME" in
  briven_dashboard:briven_dashboard|briven_auth:briven_auth) ;;
  *) echo 'unexpected database identity' >&2; exit 1 ;;
esac
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres \
  --set="app_user=$BRIVEN_DB_APP_USER" --set="app_password=$BRIVEN_DB_APP_PASSWORD" --set="app_database=$BRIVEN_DB_APP_NAME" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION PASSWORD %L', :'app_user', :'app_password') \gexec
SELECT format('CREATE DATABASE %I OWNER %I', :'app_database', :'app_user') \gexec
SELECT format('REVOKE CONNECT ON DATABASE %I FROM PUBLIC', :'app_database') \gexec
REVOKE CONNECT ON DATABASE postgres FROM PUBLIC;
SQL
cat > "$PGDATA/pg_hba.conf" <<EOF
local all all trust
hostnossl all all 0.0.0.0/0 reject
hostnossl all all ::/0 reject
hostssl $BRIVEN_DB_APP_NAME $BRIVEN_DB_APP_USER 0.0.0.0/0 scram-sha-256
hostssl $BRIVEN_DB_APP_NAME $BRIVEN_DB_APP_USER ::/0 scram-sha-256
hostssl all all 0.0.0.0/0 reject
hostssl all all ::/0 reject
EOF
echo 'scoped application database initialized'
