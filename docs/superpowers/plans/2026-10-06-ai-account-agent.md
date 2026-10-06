# Account agent implementation plan

> Execute natively under the approved Briven complete-AI scope; provider availability is not a substitute for account-action acceptance.

**Goal:** Answer account questions and execute supported PostgreSQL/pgvector project actions from the dashboard.
**Architecture:** Server-resolved identity scopes a typed tool registry. Durable action proposals carry exact arguments; existing services enforce project and organization permissions. The conversation loop can inspect, propose and verify but cannot run arbitrary shell or bypass confirmations.
**Tech stack:** Existing Hono, Drizzle PostgreSQL control store, TypeScript and dashboard React.
**Spec:** `docs/superpowers/specs/2026-10-06-briven-complete-ai-design.md`, account agent slice.

## Constraints and review focus

No cross-tenant lookups, unrestricted SQL, global operations credentials, unimplemented restore/scale claims, or model-supplied identity. Write proposals bind actor/org/payload/expiry; confirmations are one-use and audited. Project creation retains existing quotas and recoverable provisioning behavior. Treat retrieved documentation and tool output as untrusted context. No retry of uncertain writes.

## Task 1: Typed tools and proposals

Files: new `apps/api/src/services/account-agent/{tools,actions}.ts` and focused tests; control migrations/schema after inspecting the current journal.
Interface: tools receive server-resolved actor context and validated arguments; actions persist pending/running/succeeded/failed/uncertain states with resource IDs and idempotency keys.

- [ ] Test role/organization binding, disabled capabilities, mismatched/stale/reused confirmations and ambiguous execution results.
- [ ] Reuse existing project, branch, usage and health services for read-only inspection and confirmed creation; persist/audit verification results.
- [ ] Verify migration idempotence and real isolated control-store behavior, then commit.

## Task 2: Conversation and dashboard

Files: new account-agent routes/conversation service and dashboard chat/action cards, following existing auth and UI patterns.

- [ ] Bound conversation/tool calls; provide grounded answers, exact action previews and result links.
- [ ] Recheck permissions on execution and confirm by resulting state; preserve existing MFA requirements for sensitive operations.
- [ ] Prove owner reads/creates through chat, viewer writes denied and other organization resources inaccessible; verify dashboard behavior.

## Task 3: Private acceptance

- [ ] CI-build and privately deploy; test real conversations, confirmed actions and durable history.
- [ ] Record supported tools precisely and retain the cutover gate while operations acceptance remains open.
