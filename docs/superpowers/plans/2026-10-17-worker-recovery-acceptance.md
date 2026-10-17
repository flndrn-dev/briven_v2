# Worker recovery final acceptance implementation plan

> Execute natively under the owner's 17 October approval and the original approved fixed-worker recovery contract.

**Goal:** Close worker recovery with final-release failure/review drills, normal owner activation and an actual automatic-loop proof.
**Architecture:** Existing private supervisor and PostgreSQL policy remain authoritative. Resumable guarded acceptance creates fixed staging children, records real observations/attempts and uses ordinary owner review/activation. A final bounded fault affects only the approved observation child.
**Tech Stack:** Existing Bun/TypeScript child processes, PostgreSQL, Hono/admin UI, Linux CI and native Dokploy.
**Spec:** `../specs/2026-10-17-worker-recovery-acceptance-design.md`, owner-approved 17 October.

## Global constraints

- Target only `briven-operations-observer` and the two fixed staging targets. Three failures at least 30 seconds apart, age ≤120 seconds; 120-second action/verification bound; 30-minute cooldown; at most two attempts per target per day.
- Preserve every attempt/block/review/drill and existing budget. No SQL bypass of normal owner review/activation; no database/host automatic action.
- Final API/supervisor image must match the accepted drill build. Finish shared saved-copy changes before the live drill. Back up all current private journals around API replacement.
- Owned child PID/parent/entrypoint guards, bounded logs, no Docker polling, no host build, no new credentials or infrastructure spend.

## Review focus

- Actual production supervisor wiring must be exercised, not just a test executor with injected handlers.
- An interrupted handler/claim must remain blocked until owner review; a late success cannot erase it.
- Owner review must retain the failed status, cooldown and daily attempt count.
- A changed image invalidates the old drill and returns the worker to observe.
- Cleanup must reap paused children even on an exception or interrupted acceptance runner.

## Task 1 — Exact-release automated gates

**Files:** website existing `services/operations/{policy,recovery-request,recovery-executor,recovery-store.integration,worker-supervisor}.test.ts` and real worker fixtures; create `scripts/run-worker-recovery-acceptance.mjs` plus matching workflow.

- [ ] Run existing unit, real PostgreSQL and real-process suites with zero skips and owned Unix-socket cleanup. Add meaningful failing regressions only for concrete uncovered failure/activation risks discovered during inspection.
- [ ] Exercise real process interruption plus durable reconciliation/late-completion refusal in disposable Linux, using production cadence and budgets. Include API/web types/build/focused lint. Record exact source, totals and cleanup.
- [ ] Fix any reproduced product issue with a focused red/green regression, then freeze the shared candidate and coordinate its image publication with the saved-copy plan.

## Task 2 — Resumable matching live drills and owner review

**Files:** private `.local/operations-closure-20261017/` journal/runner/guarded live stages; reports `docs/worker-recovery-acceptance-2026-10-17.md` and matching website report.
**Interfaces:** fixed stages `preflight`, `stage-success`, `stage-failure`, `repair-review`, `verify-review`, `verify-activation`, `automatic-loop`, `cleanup`, `postflight`. Each records safe outcomes and resumes the same journal; stage selection cannot supply a PID, executable, project, key or policy limit.

- [ ] Capture exact images/config/container and project/Auth hashes, current history/budgets and owner-facing mode. Verify no unreviewed block or active claim is silently reset.
- [ ] Start the two owned fixed staging adapters. Collect actual failures; prove one dispatch for duplicate delivery, replacement/old-process exit, three fresh healthy samples, persistence/replay/cooldown. Record a matching drill through trusted code.
- [ ] Produce one actual staged failed verification and lockout; repair the same child and record three healthy samples. Verify ordinary-user/anonymous/forged/wrong-attempt/missing-verification refusals.
- [ ] Prepare the exact owner review in the real health UI. Owner performs any credential/MFA handoff. Verify server actor, retained failed history, durable review, unchanged budget and cooldown.

## Task 3 — Owner activation and actual automatic loop

- [ ] Confirm final image/build, matching drill and three fresh production healthy readings. Prepare normal owner activation and final action-time confirmation with exact target/build/drill; owner completes any fresh verification.
- [ ] Verify normal authenticated activation audit and real UI automatic state, with host observation mode represented separately.
- [ ] Guard the sole actual observation child by parent/entrypoint identity, pause only that child, and observe the deployed supervisor's own automatic claim/restart/verification within five minutes. Verify one claim, actual replacement, three healthy readings, current status and preserved cooldown/budget. Any failure/uncertainty blocks closure and follows normal review.
- [ ] Reap all staging children and synthetic sessions, retain audit history, verify no orphan/running staged claim and exact preservation/public readiness. Save rollback-to-observe instructions.
- [ ] Commit/push scoped evidence, synchronize main and registry. Mark only this worker recovery gap closed when all required results pass; report the fixed target and accepted build/drill/attempt.
