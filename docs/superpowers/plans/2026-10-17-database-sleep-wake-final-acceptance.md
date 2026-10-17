# Database sleep/wake final acceptance implementation plan

> Execute task by task natively under the owner's approved design and execution choice. The optional superpowers execution subskills are not installed in this session; use this session's native tools and review the complete scoped diff before release.

**Goal:** Prove current-release restart reconciliation and safe operating behavior, then activate the reviewed named endpoint.

**Architecture:** Run the deployed immutable engine in the existing disposable Linux durability fixture. A test-only CLI barrier and guarded API process replacement expose durable transition boundaries without changing lifecycle decisions. Keep live faults out of the shared engine; use a named synthetic endpoint and the native Dokploy configuration workflow for bounded live observation.

**Tech Stack:** Existing Rust engine, PostgreSQL catalog, Node/pg TLS client, Python fixture driver, GitHub Actions, native Dokploy.

**Spec:** `../specs/2026-10-17-database-sleep-wake-acceptance-design.md` (owner approved).

## Global constraints

- Default idle window 300 seconds; configured bounds 60–3600 seconds; scheduler cadence 30 seconds.
- Sleep uses smart shutdown only; preserve all other client backends and prepared transactions.
- Preserve exact project/timeline/endpoint binding and generation-checked durable records.
- Never replay client SQL; unexpected exit remains review-required.
- Live target is only `ep-25ab618c43b6b9fc102a4dfb111d637e`, after ownership and definition verification.
- Retain all data, saved/restore branches, Auth fixtures, volumes, routing, certificates, resource limits and observation-worker recovery authority/history.
- No privileged/host PID/socket access, deployment-host builds or daemon polling. Logs remain 10m × 3.
- Product source changes are conditional on a demonstrated defect and require a new accepted release before deployment.

## Review focus

1. An interrupted CLI must not outlive its barrier and launch a late duplicate compute: guard and terminate the owned wrapper before releasing the replacement API.
2. A draining PostgreSQL server can refuse new connections while existing work remains: independently check actual PIDs and let the existing transaction commit.
3. Status inspection and monitoring can accidentally wake a sleeping endpoint: observe status/processes without requesting connection leases or SQL.
4. A live configuration deployment can interrupt other databases: inventory actual computes/sessions and refuse a rollout affecting an unrelated active workload.
5. Failure cleanup must not leave prepared transactions, paused supervisors, sessions, credentials or public fixture resources behind: cleanup has its own required result.

## Task 1 — Immutable-image Linux acceptance

**Files:** Create `tests/acceptance/sleep-wake/run.py`, `probe.mjs`, `cli-barrier.sh`, `test_guards.py`; create `.github/workflows/sleep-wake-acceptance.yml`. Reuse `tests/acceptance/postgres-durability/run.py`, Compose and locked pg dependency without changing their existing acceptance behavior.

**Interfaces:** `Acceptance` subclasses the durability fixture, pins engine source/manifest/config from the spec, and overrides `execute`, `run_probe`, `cleanup`. `probe.mjs` communicates fixed JSON markers over stdout and waits for exact stdin acknowledgments. `cli-barrier.sh` delegates to the actual bundled CLI and holds only a named owned endpoint's start/stop boundary. Reports contain controlled labels/counts/checksums, never credentials, connection strings or raw logs.

- [x] Add and run fixture guard tests: reject wrong image/project/service/endpoint/argv and reject unknown barrier/marker/signal inputs.
- [ ] Implement fresh fixture start, normal API project creation, actual TLS row/vector/HNSW baseline, and explicit endpoint enrollment after guarded API restart with a single configured endpoint.
- [ ] Exercise denied authorization/configuration/binding, transaction/idle/prepared-work protection, failed inspection, three real sleep/eight-connection wake cycles and exact one-start assertions.
- [ ] Exercise API restarts while running/sleeping and transitions interrupted before stop dispatch, after smart dispatch while draining, before start dispatch and after the real start becomes ready. Compare durable state and actual process identity/count; retain review-required outcomes and recover only the disposable fixture explicitly.
- [ ] Exercise full-engine restart while sleeping, unexpected compute exit, ambiguous PID identity and capacity/binding refusal. Assert no duplicate launch, no SQL replay, and exact data/index preservation.
- [ ] Exercise disable/drain/normal-wake rollback, prove no further sleep for an idle window, and remove all owned fixture resources/private files with independent cleanup assertions.
- [ ] Run static Python/Node/shell checks locally, commit only scoped files on a `sleep-wake-acceptance-*` branch, push, and inspect terminal Linux result plus downloaded redacted evidence. No skipped required case is accepted.

## Task 2 — Bounded live observation and activation

**Files:** Private journaled helpers under website `.local/sleep-wake-acceptance-20261017/`; conditional product fixes under the existing lifecycle modules only if Task 1 exposes a defect.

**Interfaces:** Use existing private SSH/remote transports, normal fixture owner login/scoped connection export and existing lifecycle routes. Capture immutable preflight/postflight hashes; do not edit live catalog lifecycle records. Preserve the six existing journals across any necessary API replacement.

- [x] Read-only inventory current image/source/configuration, endpoint/timeline, all active computes and sessions, protected data/schema hashes, retained copies, resource/monitoring settings and recovery authority/history. Refuse interruption of unrelated active workloads.
- [ ] Verify the exact native engine Compose/server/three overlays and retained deployment command. Prepare the allowlist for the single reviewed target and obtain the action-time confirmation required by the approved design before enabling automatic authority.
- [ ] Perform only a reviewed native configuration/release rollout after Linux acceptance; verify source/image selectors, actual processes, health and unchanged auxiliary services.
- [ ] Use normal scoped owner/admin enrollment at 60 seconds for bounded protection/sleep/concurrent-wake trials; exercise disable/wake rollback; restore the retained 300-second setting and observe a full cycle within the 30-minute window. Record actual generations/start counts and PIDs; do not wake by monitoring.
- [ ] Revoke owned test sessions, remove only newly journaled canary data, retain lifecycle evidence, and verify golden 212-row/vector/schema hashes, both old saved/restore branches, all protected data/configuration and normal public readiness.

## Task 3 — Evidence, synchronization and completion

**Files:** `docs/database-sleep-wake-acceptance-2026-10-17.md`, this plan, approved design status, and existing owner gap tracker. Update website operating documentation only if required by the verified operating contract.

- [ ] Review the complete scoped diff against all spec requirements and fix any acceptance omissions.
- [ ] Record exact application/harness sources, image identities, CI run/evidence, live endpoint/idle setting, observed transitions, rollback proof, preservation and cleanup results. Keep other production gaps separate.
- [ ] Commit/push scoped reports, synchronize main without touching existing dirt, preserve native saved commands, and verify final live health/configuration.
- [ ] Mark only this selected gap closed when every required gate passed; report active scope and material limitations clearly.
