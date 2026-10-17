#!/bin/sh
# Disposable CI only: surround the real CLI with one fixed transition barrier.
set -eu
actual=/usr/local/bin/briven_local
root=/tmp/briven-lifecycle-acceptance
if [ "${1:-}" != endpoint ] || [ ! -f "$root/target" ]; then
  exec "$actual" "$@"
fi
target=$(cat "$root/target")
case "$target" in ep-????????????????????????????????) ;; *) exit 91 ;; esac
case "$target" in *[!a-f0-9ep-]*) exit 91 ;; esac
[ "${3:-}" = "$target" ] || exec "$actual" "$@"
mode=$(cat "$root/mode" 2>/dev/null || printf none)
case "$mode" in none|before-stop|after-stop|before-start|after-start) ;; *) exit 92 ;; esac
hold() {
  printf '%s\n' "$$" > "$root/barrier-pid"
  printf '%s\n' "$1" > "$root/reached"
  n=0
  while [ ! -f "$root/release" ] && [ "$n" -lt 1100 ]; do
    sleep 0.1
    n=$((n+1))
  done
  [ -f "$root/release" ] || exit 93
  rm -f "$root/release"
}
case "${2:-}:$mode" in stop:after-stop) hold prepare-after-stop ;; esac
case "${2:-}:$mode" in stop:before-stop|start:before-start) hold "$mode" ;; esac
case "${2:-}" in start) printf '%s\n' "$target" >> "$root/actual-start-invocations" ;; esac
"$actual" "$@"
case "${2:-}:$mode" in stop:after-stop|start:after-start) hold "$mode" ;; esac
