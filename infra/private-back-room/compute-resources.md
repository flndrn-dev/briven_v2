# Private compute resource adapter

This adapter is disabled unless `compose.compute-resources.yml` is explicitly selected. It is staged source, not evidence of deployed enforcement.

Host files are fixed: `/usr/local/libexec/briven/compute-resource-host.py`, `/etc/systemd/system/brivencompute.slice`, and `/etc/systemd/system/briven-compute-control.service`. Install these exact reviewed sources root-owned, without writable permissions for the engine. Start the slice and controller before adding the Compose overlay. The controller's runtime directory is root:1000 mode 0750; its socket is root:1000 mode 0660. It only accepts the exact private engine container namespace and verified executables.

The host helper has access to systemd and narrowly inspects the fixed engine's Docker labels, UID and init PID. Docker configuration/environment is never emitted. The engine receives only a read-only bind of the socket directory, without host PID/cgroup namespaces, cgroup filesystem write access, extra capabilities or a Docker socket.

Every compute starts through `briven_compute_launch`. The helper derives the host PID from `SO_PEERCRED`, creates a fixed systemd scope, and verifies actual kernel limits and PID membership before acknowledging the launcher. No PostgreSQL process is allocated if admission or verification fails. Existing live scopes cannot be replaced. Failed empty scopes can have their failure state reset only as part of an explicit new start.

Configured bounds: compute memory 512 MiB, swap zero, CPU 50 ms per 100 ms, processes 128. The enclosing slice bounds engine plus compute scopes at 8 GiB, three CPUs and 1024 processes. All descendants inherit the scope; huge pages are disabled in the resource-enabled PostgreSQL configuration. Active compute admission is four globally, two per project. Definition quotas are 128 globally, 32 per project.

Real acceptance must prove CPU throttling, isolated memory OOM, process creation refusal, normal Postgres/vector persistence, connection/transaction protection, one concurrent cold startup and durable lifecycle reconciliation after API restart. The fixed probes are invoked with a process-local `BRIVEN_RESOURCE_PROBE_ENABLED=true`; use fresh synthetic endpoint IDs with no database directory. They accept only cpu, memory or pids modes and allocate no database. A memory probe must be killed by its own scope, with the engine and existing databases remaining healthy.

Aggregate monitoring must read the parent slice because migrated compute processes are in sibling systemd scopes. Docker's engine-container statistics alone do not include them. Record actual source/image, matched Compose target, verified kernel values and operating history before considering this production-complete.

Disable enrollment and safely reconcile/wake sleeping databases before rollback. Remove this overlay only after all its scopes are empty; preserve storage, lifecycle history and current/previous source images. Do not prune volumes, stop unrelated services or reset recovery budgets.
