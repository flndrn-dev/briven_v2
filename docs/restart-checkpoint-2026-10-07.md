# Briven restart checkpoint — 7 October 2026

Work paused at the owner's request to close the session and restart the project. Resume the ten-area gap closure; do not start again from the September status percentages.

## Repository state

Engine workspace: `/Users/flndrn/Desktop/brivendatabase-briven`, branch `sprint3-serverless-postgres`.
Website workspace: `related-repositories/briven-website`, branch `sprint3-serverless-postgres`.

Website work is saved in local commits `17bee33` (account assistant) and `74d0b5f` (monitoring dashboard). No push or release build was started during shutdown.

Existing engine working-tree changes were preserved: `.deploy.md`, `.neon_clippy_args` deletion, `.briven_clippy_args`, `NOTICE`, RFC template, deployment design and `infra/private-back-room/prove-runtime-startup.mjs`. They were present before this session; do not discard or stage them indiscriminately. The website's pre-existing `step-up-prompt.tsx` wording change is also preserved separately.

## Completed locally

The account assistant has typed tools for project listing, overview, usage, engine diagnostics, branch listing, database creation, display-name changes and branch creation. Writes persist exact actor/organization/payload-bound proposals with ten-minute expiry and one-use confirmation. Permissions are rechecked on execution; uncertain outcomes cannot be replayed. The model cannot supply identity or execute arbitrary SQL or shell commands.

Added durable scoped conversation history, bounded model/tool steps, concurrent-message exclusion, cancellation checks and dashboard-session-only routes under `/v1/orgs/:orgId/assistant`. The dashboard at `/dashboard/assistant` has organization selection, history, exact action cards, confirmation and result links, unavailable-provider feedback, and desktop/mobile navigation. Migration `0060_account_agent.sql` is registered in the migration journal.

The pre-existing monitoring dashboard work is saved with the health-page integration. Collection, incident persistence and deterministic recovery-policy modules were already committed before this session. Automatic recovery remains inactive; there is no recovery executor or approved production worker registration yet.

## Verification

- 50 tests passed, zero skipped: AI provider transport, account action/confirmation/history concurrency and isolation, monitoring collection/baselines/persistence and recovery policy. Database-backed tests used an isolated temporary PostgreSQL 17 schema and Unix socket, never a customer database.
- API and dashboard TypeScript checks passed.
- Focused lint passed for account-agent code, routes and assistant UI.
- API Bun build passed (1786 modules).
- `git diff --check` passed.
- The new chat UI has not received browser visual acceptance; no new dashboard production build, live provider generation/streaming, real account-agent workflow or private deployment was completed in this session.

Temporary PostgreSQL was stopped cleanly. Its data remains at `/private/tmp/briven-operations-test-20261007-data`, socket directory `/private/tmp/briven-operations-test-20261007`, port 55449, TCP disabled. The ignored `.local/test-preload.ts` sets only the test socket. To rerun the focused database suite after restarting that isolated instance:

```sh
cd related-repositories/briven-website
bun test --preload ./.local/test-preload.ts apps/api/src/services/account-agent/ apps/api/src/services/operations/ apps/api/src/services/ai-provider.test.ts
```

## Live state and dependencies

No live services were stopped or modified. The existing public website remains in place. The previously verified public PostgreSQL proxy remains active; the private website is still gated from public routing. Existing core engine/TLS/SQL/functions/realtime/backup proofs are in `docs/superpowers/plans/2026-10-06-engine-query-routing.md`.

Private website: Dokploy compose `fJzDyG3t58mi056mvcY9o`, app `briven-serverless-website-kvm2-q76ulk`, approved KVM2 server `187.77.183.190`. Live website compose: `LykAyuz6qInYZe37wIMLp`. Do not use the retired France stack or infer a different server from old notes.

SSH to the approved server works with the existing `briven_v2_server` key. The certificate export timer was checked active. The existing Dokploy browser tab at `admin.loowii.com` was signed out; no authenticated API credential was found in the checked local deployment locations. Reuse approved credentials or an authenticated panel session; do not bypass Dokploy by deploying directly through SSH.

Private website `.local/owner-login.json` and `.local/serverless-stage.json` still exist and are ignored. Never print or commit their contents. `.local/xai.env` exists but its `XAI_API_KEY` value is empty. No xAI generation call was made. The existing Ollama gateway's last recorded model failure was HTTP 502 on 6 October; it was not reverified this session. Working provider credentials/gateway and panel access are dependencies for live AI acceptance and deployment.

## Next work

1. Inspect the local commits and preserved unrelated edits. Complete visual/dashboard-build acceptance, then release API/web through GitHub CI and deploy to the verified private Dokploy stage.
2. Prove real database-assistant answers/streaming and owner account actions, plus viewer/cross-organization refusal, on the stage. Preserve the owner's working-AI launch gate.
3. Deploy monitoring in observe mode, collect real history and show warming-up honestly. Add and test the fixed-playbook recovery executor before registering any automatic target.
4. Complete engine-backed customer restore, idle sleep/demand scaling and customer Auth enrollment. The old Dolt snapshot implementation remains unused; do not reconnect it to PostgreSQL projects. Read the Auth knowledge base and matching official documentation before changing Auth.
5. Run final launch acceptance and only then perform the authorized public website cutover, retaining the old stack for rollback.

The owner asked to proceed without routine confirmations. Continue natively with existing authorization; ask only for genuinely missing access/decisions or actions outside that authorization. The later CLI command requesting `--ask-for-approval never --sandbox workspace-write` did not alter this session's managed tool permissions.
