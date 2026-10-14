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
# BEGIN CONTROLLER RESTART GUARD
prepare_controller_restart() {
  restart_data=$1
  restart_proc=$2
  restart_binary=$3
  restart_sockets=$4
  restart_pidfile="$restart_data/postmaster.pid"
  [ -e "$restart_pidfile" ] || return 0
  restart_contents=$(cat "$restart_pidfile") || return 1
  restart_pid=$(printf '%s\n' "$restart_contents" | sed -n '1p')
  case "$restart_pid" in ''|*[!0-9]*|0|1) return 1 ;; esac
  restart_realdata=$(cd "$restart_data" && pwd -P) || return 1
  [ "$(printf '%s\n' "$restart_contents" | sed -n '2p')" = "$restart_realdata" ] || return 1
  [ "$(printf '%s\n' "$restart_contents" | sed -n '4p')" = 1235 ] || return 1
  [ "$(printf '%s\n' "$restart_contents" | sed -n '5p')" = "$restart_sockets" ] || return 1
  [ -r "$restart_proc/1/comm" ] || return 1
  # This runs in the engine's private PID namespace, before briven_local starts
  # any services. kill(pid, 0) cannot distinguish a recycled PID after SIGKILL.
  # Preserve a live postmaster and refuse an identity that cannot be inspected.
  for restart_process in "$restart_proc"/[0-9]*; do
    [ -d "$restart_process" ] || continue
    [ -r "$restart_process/comm" ] || return 1
    restart_name=$(cat "$restart_process/comm") || return 1
    [ "$restart_name" = postgres ] || continue
    restart_exe=$(readlink "$restart_process/exe") || return 1
    restart_cwd=$(readlink "$restart_process/cwd") || return 1
    if [ "$restart_cwd" = "$restart_realdata" ]; then
      [ "$restart_exe" = "$restart_binary" ] || return 1
      return 0
    fi
  done
  restart_lock="$restart_sockets/.s.PGSQL.1235.lock"
  restart_socket="$restart_sockets/.s.PGSQL.1235"
  if [ -e "$restart_lock" ]; then
    [ "$(sed -n '1p' "$restart_lock")" = "$restart_pid" ] || return 1
    [ "$(sed -n '2p' "$restart_lock")" = "$restart_realdata" ] || return 1
  elif [ -e "$restart_socket" ]; then
    return 1
  fi
  [ "$(cat "$restart_pidfile")" = "$restart_contents" ] || return 1
  # Remove only verified stale runtime locks, never the controller data, WAL,
  # durable catalog, safekeeper files or customer storage.
  rm -f -- "$restart_pidfile" "$restart_lock" "$restart_socket"
  echo 'controller runtime locks cleared after verified postmaster exit'
}
# END CONTROLLER RESTART GUARD
if [ ! -f "$controller_data/PG_VERSION" ]; then
  /usr/local/v16/bin/initdb -D "$controller_data" -U neon --no-instructions
fi
settings="$controller_data/postgresql.auto.conf"
touch "$settings"
awk '!/^[[:space:]]*(fsync|shared_preload_libraries)[[:space:]]*=/' "$settings" > "$settings.new"
printf "fsync = on\nshared_preload_libraries = 'neon_rmgr'\n" >> "$settings.new"
mv "$settings.new" "$settings"
prepare_controller_restart "$controller_data" /proc /usr/local/v16/bin/postgres /tmp
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
