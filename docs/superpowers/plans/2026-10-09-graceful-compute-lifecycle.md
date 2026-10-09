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

- [ ] Persist explicit endpoint enablement, idle window, last demand, actual phase, generation and recoverable failure; retain actual project/timeline/endpoint bindings.
- [ ] A fixed 30-second tick inspects enrolled endpoints without starting them. Default idle window is 300 seconds; accepted bounded configuration is 60–3600 seconds. Failed activity sampling or any protected backend/prepared transaction keeps compute awake.
- [ ] Commit a stopping transition before requesting smart shutdown. A process exiting after an uncertain request reconciles to sleeping; a running draining process remains stopping. Never apply a fast fallback.
- [ ] Normal authenticated proxy/service demand rechecks tenant binding, updates last demand, resumes a sleeping compute once and verifies SQL readiness. Coalesce concurrent requests using the existing mutation gate; never replay client SQL.
- [ ] Document and test catalog migration/rollback behavior before activating new durable records. Preserve existing ready-project lookup semantics.

## Task 3: Resource enforcement and real acceptance

- [ ] Inspect actual host/container limits and inherited compute resource facilities. Enforce concrete admission and resource bounds through a supported adapter; do not describe PostgreSQL default settings alone as hard per-tenant memory/CPU limits.
- [ ] Reject excess compute creation/start before allocating a process; completed duplicate starts remain idempotent. Preserve current running workloads until an explicit verified policy enrolls them.
- [ ] Real isolated stage: protect open transaction and idle connection, sleep after actual disconnection and idle window, issue concurrent cold demand, prove one startup, rows/vectors survive, and reconciled state survives control API restart.
- [ ] Deploy through the exact private Dokploy engine after CI. Activate only the isolated acceptance target first. Record actual hard bounds and retain rollback/source images.

Graceful CLI support alone does not complete the idle scheduler or resource gate. No automatic sleep or production activation is claimed until all tasks pass.
