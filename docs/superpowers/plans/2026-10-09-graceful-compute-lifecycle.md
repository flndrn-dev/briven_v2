# Graceful compute lifecycle implementation plan

> Continue natively under the owner's accepted idle-compute requirement. This extends the existing engine scheduler boundary; existing compute policies remain unchanged until isolated acceptance passes.

**Goal:** Suspend genuinely idle project computes without interrupting sessions, resume once on authenticated demand, persist lifecycle outcomes and enforce measured resource bounds.

**Architecture:** Keep the existing project catalog, endpoint files and authenticated proxy callbacks. Add durable, explicitly enabled endpoint lifecycle records and a fixed scheduler. Requests and scheduler transitions serialize through the existing engine mutation gate; no model selects commands or endpoints. Graceful shutdown must return immediately after requesting PostgreSQL smart shutdown so one draining connection cannot hold the shared engine gate.

**Tech Stack:** Existing Rust control API/CLI, PostgreSQL catalog and pg_stat_activity, inherited compute_ctl, private Dokploy stage.

**Spec:** `2026-09-28-sprint-3-serverless-cutover-design.md` and the accepted idle-compute/resource-control row in `2026-10-07-gap-closure.md`.

## Constraints and verified behavior

- The inherited `endpoint stop` defaults to fast shutdown. Fast shutdown aborts active transactions; it cannot implement idle sleep. PostgreSQL smart shutdown rejects new connections while allowing existing sessions to finish. See [PostgreSQL 16 shutdown](https://www.postgresql.org/docs/16/server-shutdown.html) and [pg_ctl](https://www.postgresql.org/docs/16/app-pg-ctl.html).
- `briven_local start` deliberately does not start endpoints. Durable sleeping computes must stay asleep after engine startup; demand can resume them.
- No force fallback, compute_ctl SIGTERM, data-directory destruction, credential bypass or arbitrary target/shell input in the scheduler.
- Preserve the saved-copy release, endpoint identities, tenant timelines and storage. Saved-copy timelines never acquire compute.
- A missing/error activity sample prohibits sleep. Protect every other client backend, including idle connections and idle-in-transaction sessions, plus prepared transactions. Do not count the short-lived scheduler inspection connection as customer demand.
- Configuration is off by default. Activation is limited to explicit isolated staging endpoints until real acceptance; do not silently enroll existing customer computes.
- Crash recovery and normal demand wake remain distinct. An unexpected crashed endpoint requires a recoverable failure record and operator review rather than repeated automatic restart.

## Task 1: Graceful CLI primitive

Files: `control_plane/src/endpoint.rs`, `control_plane/src/bin/neon_local.rs`.

- [x] Add `EndpointTerminateMode::Smart`; retain Fast as the default. Graceful completion must not send SIGTERM to compute_ctl.
- [x] Add `endpoint stop --mode smart --no-wait`. Permit no-wait only with Smart and without destroy; reject unsupported combinations before sending a signal.
- [x] Request `pg_ctl -m smart -W stop` and return immediately, leaving compute_ctl to finish normally. Keep ordinary stop output and existing modes compatible.
- [x] Compile focused CLI/control API tests. Twenty-three Rust tests pass, including the new force/destroy refusal checks. Disposable real PostgreSQL proof passes seven assertions: the open transaction commits, new connections are refused, the idle connected client can still query, shutdown waits for disconnect and the committed row survives restart. The fixture directory/process was removed; the shared production database was untouched. This verifies PostgreSQL's primitive behavior, not an activated engine scheduler.

## Task 2: Durable scheduler and normal demand wake

Files: focused lifecycle policy/store/scheduler modules under `control_plane/src/briven_control_api/`, existing API state/proxy control integration, versioned additive catalog state.

Implementation boundaries: `lifecycle.rs` owns validated records and fixed decisions; `lifecycle_store.rs` owns compare-and-swap persistence and the independent `briven_compute_lifecycle` schema version 1; `lifecycle_runtime.rs` owns bounded activity/process observations, the 30-second scheduler and demand transitions. Reuse `catalog::pg_client` without changing project migrations. Hosted state retains exact project/timeline/endpoint bindings and generation. Local development writes a versioned atomic, fsynced file on the existing repository volume. Hold a separate control API flock for process lifetime; the inherited CLI retains its repository-directory lock.

Enrollment is explicit, owner/admin only and restricted to configured existing endpoint IDs. `BRIVEN_COMPUTE_LIFECYCLE_ENABLED` defaults false and `BRIVEN_COMPUTE_LIFECYCLE_ENDPOINTS_JSON` defaults empty. Do not infer absence from a refused TCP connection: smart shutdown can close its listening socket while existing clients and compute_ctl remain alive. Observe both processes; unreadable/ambiguous identity prevents sleep and destructive CLI start. Reconcile an uncertain starting operation from SQL readiness without starting a second process. Persist Stopping before dispatch; an unacknowledged smart request can be safely redispatched after API interruption, while a known dispatch failure is retained for review.

Migration/rollback: the independently versioned additive lifecycle schema leaves `briven_control.schema_migrations` at version 1. Before engine rollback, disable enrollment, allow draining shutdowns to finish and explicitly wake normal sleeping computes with the current API; retain state/history. Never roll back over a live draining process or claim that the previous API understands lifecycle failures.

- [ ] Persist explicit endpoint enablement, idle window, last demand, actual phase, generation and recoverable failure; retain actual project/timeline/endpoint bindings.
- [ ] A fixed 30-second tick inspects enrolled endpoints without starting them. Default idle window is 300 seconds; accepted bounded configuration is 60–3600 seconds. Failed activity sampling or any protected backend/prepared transaction keeps compute awake.
- [ ] Commit a stopping transition before requesting smart shutdown. A process exiting after an uncertain request reconciles to sleeping; a running draining process remains stopping. Never apply a fast fallback.
- [ ] Normal authenticated proxy/service demand rechecks tenant binding, updates last demand, resumes a sleeping compute once and verifies SQL readiness. Coalesce concurrent requests using the existing mutation gate; never replay client SQL.
- [ ] Document and test catalog migration/rollback behavior before activating new durable records. Preserve existing ready-project lookup semantics.

## Task 3: Resource enforcement and real acceptance

The inspected private engine currently has cgroup v2 with unrestricted `memory.max` and `cpu.max`. The host has four CPUs and approximately 16 GiB RAM. The new adapter is an explicit, disabled-by-default Compose overlay and fixed root-owned Unix-socket service. The service authenticates UID, kernel peer PID, exact engine Compose labels, PID namespace and bundled executable; callers cannot select host PIDs, commands, paths or limits. systemd creates fixed endpoint scopes before the launcher execs compute_ctl. No host PID namespace, writable cgroup mount or Docker socket is given to the engine. This follows systemd's [scope interface](https://systemd.io/CONTROL_GROUP_INTERFACE/) and [single-writer rule](https://github.com/systemd/systemd/blob/main/docs/CGROUP_DELEGATION.md).

Candidate resource class: 512 MiB memory, zero swap, half a CPU, 128 processes per compute; parent engine envelope 8 GiB, three CPUs and 1024 processes. Admission permits four active computes total, two per project, 128 endpoint definitions total and 32 per project. Kernel values and scope membership must match before readiness; unavailable or altered controls fail closed. PostgreSQL settings fit this envelope and disable huge pages, while kernel controls remain authoritative. These are candidate bounds awaiting measured Linux acceptance, not currently enforced production limits.

Local verification: the full focused Rust suite passes 38 tests. The disposable real PostgreSQL helper passes 11 lifecycle tests, including concurrent catalog CAS, state reopening, unchanged project schema compatibility, open/idle connections and prepared work. Five host protocol tests verify fixed commands/properties, capacity and unknown-kernel-value refusal. These do not replace actual resource stress or engine cold-wake trials.

Before opt-in: inventory currently active workloads, verify systemd 255 scope properties, run the fixed CPU/memory/process stress probes, then use normal private Dokploy deployment. The owner-invoked probes require a process-local enable flag and a nonexistent synthetic endpoint; they cannot receive an arbitrary child command. Keep lifecycle enrollment empty until the isolated database trial passes. Docker container statistics exclude sibling compute scopes, so aggregate engine monitoring must also observe `brivencompute.slice`; container-only memory/CPU data is insufficient after this adapter is enabled.

Rollback keeps stored data and lifecycle state. Disable enrollment, finish draining, explicitly wake normal sleeping records, and ensure no compute scope remains populated before removing the resource overlay or host service. Retain the current and rollback engine images. Never kill or migrate a live database merely to remove a scope.

- [ ] Inspect actual host/container limits and inherited compute resource facilities. Enforce concrete admission and resource bounds through a supported adapter; do not describe PostgreSQL default settings alone as hard per-tenant memory/CPU limits.
- [ ] Reject excess compute creation/start before allocating a process; completed duplicate starts remain idempotent. Preserve current running workloads until an explicit verified policy enrolls them.
- [ ] Real isolated stage: protect open transaction and idle connection, sleep after actual disconnection and idle window, issue concurrent cold demand, prove one startup, rows/vectors survive, and reconciled state survives control API restart.
- [ ] Deploy through the exact private Dokploy engine after CI. Activate only the isolated acceptance target first. Record actual hard bounds and retain rollback/source images.

Graceful CLI support alone does not complete the idle scheduler or resource gate. No automatic sleep or production activation is claimed until all tasks pass.
