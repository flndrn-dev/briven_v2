#!/bin/bash
# Run the actual application Store/activity tests with the bundled PostgreSQL.
set -euo pipefail
[[ "${GIT_VERSION:-}" =~ ^[0-9a-f]{40}$ ]]
[[ "${BRIVEN_ACCEPTANCE_RUN:-}" =~ ^[0-9]+-[0-9]+$ ]]
task_user=$(id -un)
[[ "$task_user" =~ ^[a-z_][a-z0-9_-]*$ ]]
test "$(id -u)" -ne 0
task_dir=$(mktemp -d /tmp/briven-lifecycle-store.XXXXXX)
task_data="$task_dir/data"
task_socket="$task_dir/socket"
task_bin=/home/nonroot/pg_install/v17/bin
task_running=false
export LD_LIBRARY_PATH="/home/nonroot/pg_install/v17/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
cleanup() {
  if "$task_running"; then
    "$task_bin/pg_ctl" -D "$task_data" -m smart -t 10 -w stop >/dev/null || return 1
    task_running=false
  fi
  case "$task_dir" in /tmp/briven-lifecycle-store.*) ;; *) return 1 ;; esac
  rm -rf -- "$task_dir"
  test ! -e "$task_dir"
}
trap 'task_status=$?; cleanup || exit 1; exit "$task_status"' EXIT
mkdir "$task_socket"
"$task_bin/initdb" -D "$task_data" -U "$task_user" --auth=trust --no-instructions >"$task_dir/init.log"
"$task_bin/pg_ctl" -D "$task_data" -l "$task_dir/postgres.log" -o "-k $task_socket -p 55451 -c listen_addresses='' -c max_connections=16 -c max_prepared_transactions=5 -c shared_buffers=16MB -c fsync=on" -w start >/dev/null
task_running=true
test "$("$task_bin/psql" -XAt -h "$task_socket" -p 55451 -U "$task_user" -d postgres -c 'SHOW data_directory')" = "$task_data"
"$task_bin/psql" -X -v ON_ERROR_STOP=1 -h "$task_socket" -p 55451 -U "$task_user" -d postgres -c 'CREATE DATABASE briven_lifecycle_fixture_ci' >/dev/null
export BRIVEN_TEST_DISPOSABLE_LIFECYCLE_URL="postgresql:///briven_lifecycle_fixture_ci?user=$task_user&host=$task_socket&port=55451"
export RUSTFLAGS="-Clinker=clang -Clink-arg=-fuse-ld=mold -Clink-arg=-Wl,--no-rosegment -Cforce-frame-pointers=yes ${ADDITIONAL_RUSTFLAGS:-}"
case "${IO_ALIGNMENT:-512}" in 512) task_features=pageserver_api/io-align-512 ;; 4k) task_features=pageserver_api/io-align-4k ;; *) exit 1 ;; esac
if ! timeout 600 cargo test --locked --release --features "$task_features" -p control_plane --bin briven_control_api lifecycle -- --test-threads=1 >"$task_dir/tests.log" 2>&1; then
  cat "$task_dir/tests.log"
  exit 1
fi
cat "$task_dir/tests.log"
grep -F 'test lifecycle_store::tests::postgres_claim_is_atomic_reopen_retains_sleep_and_old_catalog_stays_readable ... ok' "$task_dir/tests.log" >/dev/null
grep -F 'test lifecycle_runtime::tests::real_activity_protects_transactions_idle_clients_and_prepared_work ... ok' "$task_dir/tests.log" >/dev/null
grep -F 'test result: ok. 11 passed; 0 failed; 0 ignored;' "$task_dir/tests.log" >/dev/null
cleanup
trap - EXIT
printf '{"phase":"lifecycle-postgres-unit","passed":true,"source":"%s","run":"%s","tests":11,"cleanupPassed":true}\n' "$GIT_VERSION" "$BRIVEN_ACCEPTANCE_RUN"
