# Briven complete AI scope

Owner-approved scope and intervention policy, 6 October 2026.

## Product boundary

Briven remains a serverless PostgreSQL, pgvector and AI database platform. Complete AI comprises database assistance, an active account agent and proactive server operations. Do not add general website-building, marketing or unrelated host administration. The owner accepted automatic tested recovery within fixed limits and approval for disruptive changes, data deletion, security changes and added spending. The website cutover remains on hold until working AI acceptance.

## Independently deliverable work

1. **Model connection:** one server-side adapter for the existing schema/function/explanation, streaming, Studio design and grounded support clients. Preserve Ollama and add explicitly selected xAI; no silent provider fallback. A provider key alone does not authorize account/server actions. Existing permissions and draft review remain in force.
2. **Account agent:** conversational questions plus typed tools for project listing/status, database/branch creation and permitted diagnostics. Resolve the authenticated actor and organization on every tool call. Validate arguments, enforce existing roles and limits, audit results and check resulting state. Read-only queries run immediately; proposed writes display exact affected resources. Bind confirmations to actor, organization, action and payload with expiry and one-time consumption. No arbitrary model-written SQL or shell execution. Ground support answers in Briven capabilities; do not invent unavailable restore/scale/Auth tools.
3. **Server operations:** collect real per-server/per-service metrics and durable incident history, learn normal behavior from healthy observations, detect sustained deviations and forecast resource pressure. Present owner notifications and investigated causes. Progress from observation to recommendations to individually tested automated remedies. Customer chat never receives global server privileges.

## Operations authority

Automatic actions are limited to explicitly registered Briven background workers with an approved recovery playbook. Enforce fresh observations, sustained failure, a per-target lock, idempotency, a 30-minute cooldown and at most two attempts per target in 24 hours. Verify recovery after each action; a failed or uncertain action stops retries and escalates. No resource may acquire automatic authority merely through model text or learned history. Unknown actions are denied. Database restarts, host reboots, destructive operations, security changes and purchases require owner approval. Data-plane recovery that Docker already performs is not evidence of this agent's completion.

Monitor only Briven's authorized KVM2 services; host metrics may inform diagnosis of shared resource pressure, but remedies cannot target unrelated workloads. Do not mount Docker's socket or poll Docker inventory/stats. Metrics scraping is at least 30 seconds apart. Missing/stale metrics are unknown, never healthy zeroes. Initially no real background worker is automatically registered until its playbook and staging recovery drill pass. Controlled test doubles are not production proof.

## Meaning of learning

Maintain time-stamped healthy baselines per resource/metric, seasonal windows when sufficient history exists, and evaluated incident outcomes. Exclude anomalies and missing samples from normal-baseline updates. New baselines require enough real history; report warming-up honestly. Learned recommendations cannot change policy or retrain/promote production models automatically. An LLM can summarize evidence and propose an allowed playbook; a deterministic policy engine decides eligibility. Continue monitoring even when the model provider is unavailable.

## Acceptance

Separate proofs cover real provider generation and streaming, authenticated account actions and cross-tenant refusal, and monitoring/notification/recovery drills with verified outcomes. Demonstrate duplicate-action prevention, stale-data refusal, cooldown/budget enforcement, restart persistence and failed recovery escalation. Owner notifications begin in the dashboard; outbound channels are configured only when requested. No crash-prevention guarantee or complete self-learning claim before sufficient operating history exists. Each subsystem has its own implementation plan; ship no unverified automatic intervention.
