# Compute group monitoring implementation plan

Continue natively under the already accepted lifecycle/launch plan; no new product or activation policy is introduced.

**Goal:** Record real aggregate compute CPU, memory and task usage separately from host history.

**Architecture:** Fixed read-only cgroup oneshot → atomic textfile → existing node exporter/Prometheus → existing operations history and dashboard.

**Spec:** `../specs/2026-10-12-compute-group-monitoring-design.md`.

## Constraints and review focus

- Fixed `brivencompute.slice`; 8 GiB, three CPUs, 1024 tasks, zero swap. No Docker polling or process control.
- 30-second producer, existing 60-second scrape/observation. Old textfiles remain stale despite fresh scrapes.
- Failed collection or modified bounds cannot produce healthy zero; malformed and ambiguous vectors are refused.
- Keep host and compute baselines separate. Missing compute metrics cannot erase healthy host readings.
- Keep lifecycle disabled/allowlist empty and recovery observe-only; retain all old installations and data.

## Task 1: Fixed host metrics producer

Files: `infra/private-back-room/compute-resource-metrics.py`, focused Python tests, `.service`, `.timer`, existing compute resource runbook.

- [x] Test actual aggregate accounting, changed/missing bounds, malformed files, symlink refusal and failure replacing previous success.
- [x] Implement bounded fixed reads, actual timestamp and atomic file replacement; no caller-provided CLI targets.
- [x] Configure unprivileged sandboxed oneshot and 30-second timer; verify units and Python tests.

## Task 2: Separate scoped history

Files: website operations compute collector/tests, existing observation/store/baseline/env/worker modules, migration `0066_operations_compute_metrics`, journal/schema and existing operations panel/Compose.

- [x] Add `collectCompute(base,target,now,fetcher): Promise<Reading[]>` for CPU/memory/tasks, enforcing exporter success and genuine source time.
- [x] Add `tasks_percent` and allow `recordCycle` to select a validated metric set; preserve host defaults and all history safeguards.
- [x] Add explicit opt-in compute target configuration to API fallback and supervised observer, with the same lock/cadence protections.
- [x] Add idempotent migration and real PostgreSQL tests for separate baselines, stale samples, missing metrics and concurrent cycles.
- [x] Label target/metric in existing dashboard; use existing styling. Enable textfile reading through existing read-only host mount.
- [x] Run focused operations/recovery tests, API/web types, lint, builds, Compose/log-cap and diff checks.

## Task 3: Matching live acceptance

- [x] Install exact reviewed host producer/units and verify read-only output without restarting the engine.
- [x] Commit only task-owned files, publish immutable API/web images through CI, then use existing Dokploy website target preserving all settings except reviewed tags and compute observation opt-in.
- [x] Verify actual identities/health, real query results, at least three independent durable compute cycles, stopped-producer staleness refusal and dashboard labels.
- [x] Re-run matching isolated worker drill for the new API build before any recovery activation. Record evidence and remaining lifecycle/device/cleanup gates in the restart checkpoint.

## Verified continuation evidence

Root host source `81b8aa0f6`, website feature `a4f9ad6e4636987646d3afb6dc3fe027fede21cc`. Four producer tests and six controller tests pass; disposable PostgreSQL operations/recovery suite passes 68 tests / 283 assertions. API types/build, focused lint, Webpack product build, product-source types and Compose/log caps pass. Standalone web package types have a pre-existing `bun:test` ambient declaration issue; the product-only check preserves existing generated-type exclusion and does not claim test type coverage. Independent fresh review is clear. Fixed static non-login producer installed and real independently timed source samples passed with engine unchanged. Real Promtool aggregate/changed/zero/missing denominator fixture passes. API publication 38260765977 succeeded; web 38262139342 still building. The queued canceled attempt and abbreviated-ref checkout failure are retained in LEAVE-OFF.md. Live API/web rollout, durable Prometheus history, stopped-producer refusal and new matching drill are not yet accepted.

## Final acceptance — 17 October 2026

The historical pending rollout above is superseded by [current-release acceptance](../../monitoring-acceptance-2026-10-17.md). All matching live gates pass on a69f19c, including real retained history, actual stale producer, durable unknown state, restored independent cycles, desktop/narrow labels and permissions. The already accepted 17 October automatic-worker build/drill remains valid and unchanged; the older observe-only instruction is superseded by that owner-approved activation. No new application image or repeat recovery fault was required.
