# PostgreSQL Durability Acceptance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking. The owner selected native execution and no extra agents; implement and review natively.

**Goal:** Close the selected PostgreSQL/branches/vectors restart and durability gap with real failures on the production engine artifact.

**Architecture:** A disposable Linux CI runner owns a private bounded test stack. A guarded Python orchestrator injects faults after a Node PostgreSQL probe emits explicit checkpoints. The probe records only acknowledged expected data and verifies the same branch/vector state after recovery.

**Tech Stack:** Docker Compose, current Briven image, PostgreSQL16, pgvector, Python3, Node/pg.

**Spec:** `docs/superpowers/specs/2026-10-14-postgres-durability-design.md`

## Global Constraints

- Exact source743cab2/image manifest4bf8842; three synchronous safekeepers.
- Fresh isolated volumes/credentials/internal network; no host ports or production mounts.
- Engine4GiB/1.5CPUs/512tasks, catalog256MiB/.25CPU/128tasks, object512MiB/.25CPU/128tasks.
- Lifecycle stays disabled; no changes to the live engine or customer data.
- No secret artifacts, credential arguments, write retries after unknown outcomes or delegated agents.

## Review Focus

- A spoofed namespace or foreign container must fail before any signal.
- A crash with a live uncommitted transaction must not turn it into committed data.
- A two-safekeeper outage must not acknowledge a new COMMIT without quorum.
- Restarts must preserve branch identities/indexes and independent writes.
- A failed probe must restore its paused services and close/stop owned resources without deleting unrelated state.

### Task1: Isolated fixture and fault guards

**Files:** `tests/acceptance/postgres-durability/{compose.yml,run.py,test_guards.py,package.json,package-lock.json}`; `.github/workflows/postgres-durability.yml`.

**Interfaces:** `guard_container(row, project, service)` rejects unrelated resources; `run.py` runs the fixed matrix and writes `evidence.jsonl`/`summary.json` under the runner's private fixture directory.

- [x] Write guard tests for production names, wrong labels/images, customer mounts and valid fixture resources; verify they fail before the guard exists.
- [x] Implement CI-only namespace/volume/mount checks and the bounded private Compose fixture.
- [x] Verify guard tests, Compose parsing and script syntax.
- [x] Commit/push only the acceptance files/design/plan on the task branch; the workflow starts only for that branch or explicit dispatch.

### Task2: SQL, branch/vector and failure matrix

**Files:** `tests/acceptance/postgres-durability/probe.mjs`; existing engine source only if a reproduced defect requires correction.

**Interfaces:** Probe fixed actions create the fixture, establish golden branch hashes, emit crash/quorum markers and verify recovery. Python receives only allowed redacted marker records and targets only owned containers/PIDs.

- [x] Implement the spec's six runtime assertions, real TLS/role checks and exact branch/index identities.
- [x] Run the real matrix in CI against immutable743 and preserve failures as evidence.
- [x] Fix any reproduced product issue with a focused regression, matching publication and repeated acceptance; never relax an assertion just to pass.
- [x] Review the complete harness for secret handling, timeouts, false acknowledgement and cleanup.

### Task3: Acceptance and handoff

**Files:** `gap-closing-10102026.md`, `LEAVE-OFF.md`, CI redacted artifact.

- [x] Verify successful source/image identities and every required fault assertion from the artifact, including cleanup.
- [x] Verify live engine/image, protected Mittera bindings and public health unchanged before/after the drill.
- [x] Sync the local checkout back to main while preserving unrelated edits; retain the acceptance branch and evidence.
- [x] Mark only the selected checklist complete, report concrete evidence/limits, and wait for the owner's next gap task.

## Accepted result

CI38582956268 passed at628bcab5b, including cleanup. Reproduced controller PID and compute socket-lock reuse were fixed with14 focused restart tests;10 fault/diagnostic guards pass. Dokploy Done at628 installs the exact accepted startup hash, while binary743 and the immutable image remain unchanged. Live production bindings,33other containers, both Mittera projects and actual external TLS/vector checks passed. Original rollout command restored. Automatic recovery remains separate by owner choice. See `docs/postgres-durability-acceptance-2026-10-14.md`.
