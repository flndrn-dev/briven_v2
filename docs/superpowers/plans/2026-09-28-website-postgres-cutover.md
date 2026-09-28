# Website Postgres Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run the website control and customer data paths on serverless PostgreSQL with pgvector, starting with empty databases.

**Architecture:** The website control database stays separate from customer project databases. Customer projects use PostgreSQL and pgvector; the Briven v2 engine supplies project lifecycle and connection routing as hosted compute becomes available. The website retains its session and organization authority and forwards project operations through the private signed gateway.

**Tech Stack:** Bun, Hono, Drizzle, node-postgres, Rust/Axum, PostgreSQL 17, pgvector.

**Spec:** `docs/superpowers/specs/2026-09-28-sprint-3-serverless-cutover-design.md`

## Global Constraints

- Start with empty databases; import no customer rows or accounts.
- Do not connect any new runtime service to the legacy database.
- Keep engine control private and require signed customer identity.
- Customer Postgres credentials must be scoped to one project and require TLS.
- Do not publish a development compose or sample credentials.

## Review Focus

- Project creation during a control API restart must resolve to one tenant, not duplicate it.
- A viewer or revoked key must not create a branch or obtain write credentials.
- A connection with another project's role must fail even if its hostname is changed.
- A compute restarted after extension setup must still answer vector similarity SQL.
- A failed migration must leave the catalog at the previous schema version.

---

### Task 1: Website PostgreSQL control schema

**Files:** `apps/api/drizzle/migrations/0057_org_control_keys.sql`, `apps/api/src/db/schema.ts`, `apps/api/src/db/client.ts`

**Interfaces:** Existing `getDb()` remains the control database interface.

- [x] Apply all website migrations to a fresh disposable PostgreSQL database and verify the organization key table exists.
- [ ] Remove legacy database assumptions from the active control database configuration and documentation.
- [ ] Build the API and verify session, organization membership, and key lifecycle against the fresh database.

### Task 2: PostgreSQL project data adapter

**Files:** `apps/api/src/db/data-plane.ts`, `apps/api/src/services/db-shell.ts`, `apps/runtime/src/db.ts`

**Interfaces:** Preserve `provisionProjectDatabase(projectId)`, `runInProjectDatabase(projectId, fn)`, `rotateProjectRolePassword(projectId, ttlSeconds)`, and `withProjectTx(projectId, fn)` until the engine gateway replaces provisioning.

- [x] Make project connection configuration preserve TLS settings when changing the database name.
- [x] Enable and verify the `vector` extension during project provisioning; create metadata tables.
- [x] Grant a scoped role schema access and enforce password expiry in PostgreSQL.
- [x] Remove legacy commit settings from runtime transactions.
- [x] Test provisioning, role expiry, SQL writes, and vector queries on disposable PostgreSQL 17.

### Task 3: Vector schema and query surface

**Files:** `packages/schema/src/columns.ts`, `apps/runtime/src/query-builder.ts`, `apps/runtime/src/isolate-runtime/loop.ts`

**Interfaces:** `vector(dimensions: number)` emits `vector(N)`; `ctx.db(table).vectorSearch(input)` performs a parameterized distance query.

- [x] Enable vector columns with dimension validation.
- [x] Implement vector search in both inline and Deno isolate runtimes.
- [x] Run schema, runtime, and isolate tests; verify SQL against PostgreSQL 17 with pgvector.

### Task 4: Database features and realtime

**Files:** `apps/api/src/services/studio.ts`, `apps/api/src/services/snapshots.ts`, `apps/realtime/src/poll-manager.ts`, and affected routes and tests.

**Interfaces:** Studio writes use standard PostgreSQL transactions; snapshots map to Briven timeline branches; realtime observes committed PostgreSQL changes.

- [x] Replace legacy SQL transaction flags and catalog workarounds with PostgreSQL behavior.
- [ ] Implement branch-backed snapshots through the engine API, including restore semantics.
- [x] Replace commit-hash polling with a PostgreSQL change feed and prove project isolation.
- [ ] Remove or hide routes that cannot be made coherent before cutover.

### Task 5: Hosted engine and public proof

**Files:** Engine control API, scheduler, proxy, safe compose, website gateway, deployment docs.

**Interfaces:** Signed organization assertion enters the private control API; the public TLS proxy routes a project role to exactly one tenant compute.

- [ ] Add recoverable provisioning and lifecycle states, scoped database credentials, limits, and a compute scheduler.
- [ ] Deploy private durable engine services with catalog and tenant backup/restore drills.
- [ ] Connect the website to the engine, remove the legacy service from the active compose, and verify the public create/branch/connect/vector flow.
