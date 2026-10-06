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
export BRIVEN_LOCAL_BIN="${BRIVEN_LOCAL_BIN:-/usr/local/bin/briven_local}"
mkdir -p "$BRIVEN_REPO_DIR"
if [ -z "$(ls -A "$BRIVEN_REPO_DIR" 2>/dev/null || true)" ]; then
  briven_local init --config /local-env.toml --force empty-dir-ok
fi
export LD_LIBRARY_PATH="/usr/local/v16/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
# The bundled PostgreSQL writes Neon WAL records even for the controller's
# local catalog. Recovery must preload their resource manager. Configure this
# and durable writes before the first start, not after the API is available.
controller_data="$BRIVEN_REPO_DIR/storage_controller_db"
if [ ! -f "$controller_data/PG_VERSION" ]; then
  /usr/local/v16/bin/initdb -D "$controller_data" -U neon --no-instructions
fi
settings="$controller_data/postgresql.auto.conf"
touch "$settings"
awk '!/^[[:space:]]*(fsync|shared_preload_libraries)[[:space:]]*=/' "$settings" > "$settings.new"
printf "fsync = on\nshared_preload_libraries = 'neon_rmgr'\n" >> "$settings.new"
mv "$settings.new" "$settings"
api_pid=
proxy_pid=
shutdown() {
  trap - EXIT INT TERM
  if [ -n "$proxy_pid" ]; then
    kill "$proxy_pid" 2>/dev/null || true
    wait "$proxy_pid" 2>/dev/null || true
  fi
  if [ -n "$api_pid" ]; then
    kill "$api_pid" 2>/dev/null || true
    wait "$api_pid" 2>/dev/null || true
  fi
  briven_local stop || true
}
trap shutdown EXIT
trap 'exit 0' INT TERM
briven_local start
controller_psql=/usr/local/v16/bin/psql
"$controller_psql" -X -v ON_ERROR_STOP=1 -h 127.0.0.1 -p 1235 -U neon -d storage_controller \
  -c 'ALTER SYSTEM SET fsync = on' >/dev/null
"$controller_psql" -X -v ON_ERROR_STOP=1 -h 127.0.0.1 -p 1235 -U neon -d storage_controller \
  -c 'SELECT pg_reload_conf()' >/dev/null
test "$("$controller_psql" -X -At -h 127.0.0.1 -p 1235 -U neon -d storage_controller -c 'SHOW fsync')" = on
briven_control_api &
api_pid=$!
# The proxy runs in this namespace so customer computes remain loopback-only.
# The inherited client appends a path segment without removing an empty final
# segment, so a trailing slash would send callbacks to /proxy//method.
start_proxy() {
  tls_key=/etc/briven/proxy-tls/server.key
  tls_cert=/etc/briven/proxy-tls/server.crt
  if [ "${BRIVEN_PUBLIC_DATABASE_ENABLED:-false}" = true ]; then
    certificate_generation=$(readlink /etc/briven/public-proxy-certs/current)
    tls_key=/etc/briven/public-proxy-certs/$certificate_generation/tls.key
    tls_cert=/etc/briven/public-proxy-certs/$certificate_generation/tls.crt
    test -r "$tls_key"
    test -r "$tls_cert"
    test -r /etc/briven/proxy-tls/private/tls.crt
  fi
  proxy --auth-backend control-plane --auth-endpoint http://127.0.0.1:8787/proxy \
  --proxy 0.0.0.0:5432 --mgmt 127.0.0.1:7000 --http 127.0.0.1:7001 \
  --tls-key "$tls_key" --tls-cert "$tls_cert" --certs-dir /etc/briven/proxy-tls \
  --project-info-cache size=0,max_roles=0,gc_interval=60s --wake-compute-cache size=0 &
  proxy_pid=$!
}
certificate_generation=
start_proxy
while kill -0 "$api_pid" 2>/dev/null && kill -0 "$proxy_pid" 2>/dev/null; do
  sleep 2
  if [ "${BRIVEN_PUBLIC_DATABASE_ENABLED:-false}" = true ]; then
    next_generation=$(readlink /etc/briven/public-proxy-certs/current)
    if [ "$next_generation" != "$certificate_generation" ]; then
      # Renew TLS without restarting customer computes or the control API.
      # Close existing proxy sessions rather than ever replaying their writes.
      kill "$proxy_pid" 2>/dev/null || true
      attempts=0
      while kill -0 "$proxy_pid" 2>/dev/null && [ "$attempts" -lt 10 ]; do
        sleep 1
        attempts=$((attempts + 1))
      done
      kill -9 "$proxy_pid" 2>/dev/null || true
      wait "$proxy_pid" 2>/dev/null || true
      start_proxy
      echo 'public PostgreSQL TLS certificate refreshed'
    fi
  fi
done
if kill -0 "$api_pid" 2>/dev/null; then
  wait "$proxy_pid"
else
  wait "$api_pid"
fi
