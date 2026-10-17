# Automatic worker recovery — final release acceptance and activation

## Objective and authority

Close the selected 85% → 100% worker-recovery gap against the final deployed API/supervisor release. Reuse the owner-approved 6 October operations contract and 17 October request to activate it. Execute natively. This spec describes acceptance and activation of the fixed observation worker; the saved-copy feature has a separate spec and shares the final API release.

The single automatic target is `briven-operations-observer`, a child of the private `operations-supervisor.ts` process. Its action is `restart_worker`. Authoritative liveness comes from fresh nonce-bound IPC readings. Eligibility requires three failures at least 30 seconds apart, no reading older than 120 seconds, one active claim, a 30-minute cooldown, and fewer than two attempts per target in 24 hours. Restart plus three fresh post-action healthy readings is bounded to 120 seconds. Failed, timed-out, interrupted or uncertain outcomes persist a block until the normal owner review. Existing audit history, cooldowns and budgets remain authoritative.

## Current evidence

17 October read-only inspection: API and supervisor image source `579db5d`, healthy and restart count zero; supervisor is in observe mode with no matching drill. Last stored successful drill was 14 October on `be00b6b`. No attempts fall within the current 24-hour window, and the retained staging target is not blocked. This inspection establishes readiness to test, not recovery acceptance.

## Release and targets

Website repository: `flndrn-dev/briven-website`; Briven / production; server **briven.tech / 187.77.183.190**; website Compose `Ju2ZiCJxOw2srPHr0245C`, `infra/dokploy/compose.serverless-postgres.yml`. Revalidate the registered target through native Dokploy before mutations. Publish immutable images only after exact-source Linux gates pass. Complete any saved-copy API fixes before the final matching recovery drill: a new build invalidates activation evidence and resets the supervisor to observe.

Keep one private acceptance journal for both selected tasks, backed up alongside all five existing journals before every API replacement. Preflight/postflight record image and source identity, container/mount/port and environment hashes, current project/Auth data hashes, and operating policy. Preserve the recorded protected Mittera deletion state. Source files and actual running manifests establish release identity.

## Acceptance sequence

1. Run the real PostgreSQL recovery suite with zero skipped cases, including concurrent claims, signed-request binding, missing/stale/repeated readings, wall-clock deadlines, abandoned claims, persistence failure, owner review and unchanged cooldown/budget. Run real child-process supervisor tests and API/web types/build/lint. Use disposable isolated Linux resources; record cleanup.
2. On the final live API image, create only the fixed isolated child adapters `briven-stage-observer-drill` and `briven-stage-observer-review`. Guard their PIDs by exact parent/entrypoint identity. Pause only these owned children, collect actual failed readings at the production cadence, and prove one restart for concurrent identical delivery, actual PID replacement, three fresh healthy readings, durable completion, replay refusal and cooldown.
3. Exercise actual failed verification and a process interruption on isolated targets in disposable Linux acceptance. On the live staged review target, retain a real failed verification, prove automatic lockout, manually repair that owned child, and collect three healthy readings. Anonymous, ordinary-user, forged-actor, missing-verification and wrong-attempt review requests must fail.
4. Prepare the normal superadmin health UI to review that exact failure with a reason. The owner completes any fresh password/MFA prompt privately. The server resolves the actor and records the review; the failed attempt remains failed and its budget/cooldown remain unchanged. If any earlier block exists, review it through this same workflow before a new attempt.
5. Record the successful matching drill using trusted private drill tooling. Verify three fresh production-worker healthy readings. Prepare the normal owner activation with that exact drill/build. The owner completes any required verification and final action-time confirmation; activation must leave an audited `automatic` record bound to their real session/actor, build and drill.
6. Verify the actual automatic loop once: identify the sole observation child of the deployed production supervisor, guard its command and parent, and pause only that child. The supervisor must collect the required failures, record one automatic claim, replace the child and verify fresh health within existing deadlines. Observe the new durable attempt, three healthy readings, actual process replacement, preserved cooldown/daily count, current dashboard state and continued public readiness. Bound the whole observation window to five minutes. A failed/uncertain outcome blocks closure and goes through owner review; do not clear history or increase limits.
7. Stop all staging children, sign out test sessions, verify no orphan process or active staging claim, and retain durable failure/review/drill evidence. Commit/push scoped reports and synchronize main. Record the exact rollback procedure: normal owner switch to observe, preserve history, then restore retained image selectors through Dokploy if a software rollback is needed.

## Failure boundary

This automatic authority contains no API/database/host restart, Docker control, arbitrary executable/PID, model-selected action, spending or data removal. Manual pauses of the isolated SQL fixture belong only to the explicitly reviewed saved-copy acceptance. Host pressure is diagnosis, not restart authority. Missing metrics remain unknown. Keep all service log caps at 10m × 3 and all monitoring intervals at least 30 seconds; no Docker inventory/stat polling or deploy-host builds.

## Completion evidence

Close this gap only when exact-source Linux acceptance, matching live success/failure/review drills, real owner activation, the actual automatic-loop check, cleanup and preservation all pass. Report the fixed target, active mode, accepted build/drill/attempt and limitations. This closes one reviewed worker playbook; other application workers and database/host recovery are outside this contract.

**Review status:** Owner approved native execution on 17 October; final activation confirmed at action time. Required acceptance passed; see `../reports/2026-10-17-operations-closure-acceptance.md`.
