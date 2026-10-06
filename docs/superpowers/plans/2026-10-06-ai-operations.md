# Proactive operations implementation plan

> Execute natively under the owner's accepted policy: automatic tested low-risk recovery, approval for disruptive/destructive/security/spending changes.

**Goal:** Learn normal Briven server behavior, notify the owner of developing incidents and verify bounded recovery.
**Architecture:** Real per-target observations feed deterministic baseline/anomaly detection and durable incident history. A policy engine separates advice from execution; only registered, tested playbooks may act. The model summarizes evidence without controlling permissions.
**Tech stack:** Existing TypeScript API, PostgreSQL control store, Prometheus/node-exporter and a private recovery executor.
**Spec:** `docs/superpowers/specs/2026-10-06-briven-complete-ai-design.md`, operations slice.

## Constraints and review focus

Missing/stale metrics are unknown. Monitor authorized Briven resources only. No Docker socket mount, inventory/stats polling, host production builds or outbound notifications without configuration. Start in observe mode. Automatic recovery requires sustained failure, observation age <=120 seconds, an approved target/playbook, no active execution, a 30-minute cooldown and at most two attempts in 24 hours. Failed/uncertain verification escalates and disables further automatic attempts. Models and learning cannot widen policy.

## Task 1: Detection and policy

Files: create `apps/api/src/services/operations/{policy,baseline}.ts` with focused tests.
Interfaces: pure policy evaluation returns deny/observe/approval/automatic with reason; healthy historical observations build a per-metric baseline and anomaly assessment.

- [ ] Test unknown actions, all approval categories, stale/missing observations, insufficient history, cooldown/budget/concurrency and failed recovery lockout.
- [ ] Implement the policy and robust baseline with anomaly exclusions; test sustained pressure and unhealthy samples without fabricating metrics.
- [ ] Typecheck and commit. These pure tests do not activate production recovery.

## Task 2: Durable observation and notifications

Files: operations store/collector/incident service, control migrations/schema, owner-only read routes and dashboard incident views; inspect existing metrics and migration journal first.

- [ ] Collect real timestamped target-specific signals at >=30-second intervals, with retention and unavailable states.
- [ ] Persist baselines/incidents, deduplicate notifications and record owner acknowledgment; keep observation independent of LLM availability.
- [ ] Prove restart persistence, multi-target isolation, invalid/stale samples, missing exporter, retention and incident visibility using real control storage.

## Task 3: Recovery executor

Files: private executor with fixed registered playbooks, deployment/runbook and acceptance proof.

- [ ] Bind target/action/evidence to short-lived signed requests, persist locks/idempotency/attempt history before acting, and verify afterward.
- [ ] Perform a controlled staging background-worker failure/recovery drill. Establish rollback/escalation; register no production target before its drill passes.
- [ ] Activate only individually tested playbooks, with observe/recommendation history and owner-visible audit. Verify disruptive actions require approval and unrelated shared-host services cannot be targeted.
