# Sprint 3 leave-off — 2026-09-29

## Direction already decided

- Start the cutover with **empty PostgreSQL databases**. Do not import old website accounts, projects, or customer data.
- Use serverless PostgreSQL with **pgvector** for database and AI application features. **Do not use Doltgres** in the new project stack.
- Keep the old production volumes intact for rollback. No production database or service was changed in this work session.
- Sprint 3 is **in progress**, not deployed or complete.

## Repositories and exact saved state

| Repository | Branch | Last implementation commit | Remote |
| --- | --- | --- | --- |
| This engine repo, `flndrn-dev/briven_v2` | `sprint3-serverless-postgres` | `60215b012` — control catalog and signed PostgreSQL provisioning | `origin/sprint3-serverless-postgres` |
| Website, `flndrn-dev/briven-website` | `sprint3-serverless-postgres` | `cd38f6b` — PostgreSQL Studio metadata proof | `github/sprint3-serverless-postgres` in the temporary checkout |

Both branches were pushed. Neither repository's `main` was updated, so the website's automatic Dokploy deployment was not triggered. The website checkout used during development was `/private/tmp/briven-website-seed-2026-09-27`; it was clean at the last check, but a Mac restart may clear `/private/tmp`. The remote branch is the durable copy.

This engine checkout is on `sprint3-serverless-postgres`. It also has **pre-existing, unstaged changes outside the Sprint 3 commit**: `.deploy.md`, deletion of `.neon_clippy_args`, `NOTICE`, `docs/rfcs/YYYY-MM-DD-copy-me.md`, `docs/superpowers/specs/2026-09-27-briven-website-engine-deploy-design.md`, and untracked `.briven_clippy_args`. Preserve and review these; do not reset or discard them while resuming.

## What was implemented and checked

- Engine control API: signed organization assertions, PostgreSQL catalog with transactional versioned migrations and verified TLS for remote catalog connections, organization isolation, project and branch creation, and pgvector enable/check in signed customer mode.
- Website: organization control keys and signed gateway; fresh PostgreSQL control migrations; PostgreSQL project database adapter; scoped project service roles, separate expiring shell credentials, and project isolation; Briven Auth vault bootstrap on PostgreSQL; Studio transaction/catalog compatibility; pgvector columns and parameterized vector search; PostgreSQL `LISTEN/NOTIFY` for table-scoped realtime updates.
- Draft website compose: `infra/dokploy/compose.serverless-postgres.yml` on the website branch. It requires external PostgreSQL URLs and contains no Doltgres service. It is **not** the active Dokploy compose and has not been deployed.
- Local proofs passed on disposable PostgreSQL 17 with pgvector: website migrations, vector query, project provisioning, role expiry and cross-project denial, platform-table denial, Auth vault schema bootstrap, Studio operations and primary-key metadata, and notification delivery only after commit. The relevant API/runtime/realtime/schema/shared builds or typechecks passed. Focused Rust control API tests and catalog migration proof passed. The draft Compose passed `docker compose config --quiet` with dummy values.
- Disposable PostgreSQL test clusters under `/private/tmp` were stopped. No customer database was touched.

## Exact remaining work

1. Decide the hosted target for customer databases: Briven v2 on the existing Dokploy KVM4 server, or an external serverless PostgreSQL provider. This question is **still unanswered**. If external, obtain the provider and location of the new secrets; do not place credentials in Git or this file.
2. Replace the website's legacy snapshot implementation with engine-backed branches and coherent restore behavior, or remove snapshot routes and controls for the initial cutover. The current snapshot service still contains legacy database calls.
3. Make website project provisioning and SQL access use Briven v2 tenants and computes. The current PostgreSQL adapter provisions databases on a shared admin endpoint for local proof; it is not yet connected to the hosted engine lifecycle.
4. Add hosted compute scheduling and restart reconciliation, secure customer database credentials, a TLS PostgreSQL proxy, limits, partial-provisioning recovery, and catalog/tenant backup and restore drills.
5. Run a full customer-session and organization-isolation proof through the live engine, then a public create/branch/connect/vector smoke test. Only after those pass should the new empty-database website stack replace the active deployment.

See `docs/sprint-3-status.md`, `docs/superpowers/specs/2026-09-28-sprint-3-serverless-cutover-design.md`, and `docs/superpowers/plans/2026-09-28-website-postgres-cutover.md` for the detailed status, design, and checklist. The old `docker-compose/docker-compose.yml` in this engine repo is a development stack with sample credentials and must never be deployed publicly.

## First steps after the Mac restarts

1. Open this engine checkout and run `git status --short` and `git branch --show-current`. Keep the unstaged files listed above.
2. Check whether `/private/tmp/briven-website-seed-2026-09-27` still exists. If it does, run `git status --short` there. If it does not, clone `flndrn-dev/briven-website` and check out its remote `sprint3-serverless-postgres` branch. Do not work from its `main` by mistake.
3. Resume from the remaining-work list. Do not deploy either draft branch or switch the website compose while snapshots and hosted engine routing are unresolved.
