# Engine query routing implementation plan

> Execute natively in this session under the owner's authorized installation, proof, and cutover request. Continue the approved private proxy plan; preserve unrelated changes and the existing website until acceptance passes.

**Goal:** Run website SQL, functions, and realtime against each project's Briven compute, with verified TLS and independent customer/platform/runtime credentials.

**Architecture:** Website project rows resolve organization and engine tenant. The private engine issues 15-minute credentials for separate customer, platform, and runtime roles. Only signed, project-scoped service assertions can call the private service-connection route. The SQL editor uses the runtime role and sets the customer role for shared application object ownership; trusted metadata operations use the platform role. The API brokers service leases for runtime/realtime behind its existing internal secret. Customer roles cannot read platform metadata; service leases never cross public API routes.

**Tech stack:** Rust/Axum, inherited Neon proxy, pgvector, TypeScript, node-postgres, Dokploy.

**Spec:** `docs/superpowers/specs/2026-09-28-sprint-3-serverless-cutover-design.md` and `docs/superpowers/plans/2026-10-05-private-proxy-routing.md`.

## Constraints and review focus

- Empty new platform databases; no legacy account or data import.
- Resolve project ownership in the control database, including soft deletion. Never derive a shared administrator DSN for engine projects.
- Reject service assertions on customer lifecycle routes and require matching tenant and service kind on the service route.
- Customer key rotation must not invalidate platform/runtime credentials; service roles remain nonsuperuser and tenant-bound.
- Verify the private CA and hostname; URI SSL parameters must not overwrite node-postgres's explicit TLS configuration.
- Deduplicate concurrent credential refreshes. Retry authentication only before beginning a transaction; never replay a committed or uncertain write.
- Retain all existing volumes; deploy through Briven/production on KVM2.

## Task 1: Engine service credentials and metadata permissions

**Files:** `control_plane/src/briven_control_api/{auth,customer_credential,proxy_control}.rs`, `control_plane/src/bin/briven_control_api.rs`.
**Interfaces:** Private `GET /v1/projects/:id/branches/:branch/service-connection` requires signed `service_project` and `service_kind` claims. Return the existing `{uri,environment,expiresAt}` contract. Separate `_platform` and `_runtime` suffix roles from the customer role. Proxy binding recognizes only these exact tenant roles.

- [ ] Test scope/kind binding and reject service tokens on regular routes.
- [ ] Issue independent credentials and enforce metadata permissions/default privileges for customer tables.
- [ ] Run focused Rust checks, publish an immutable image, deploy privately and prove service/customer isolation.

## Task 2: Project SQL broker and all query clients

**Files:** Website `packages/shared/src/engine-connection.ts` and tests; `apps/api/src/services/engine-database.ts`; API data-plane/internal/project-create/db-shell/health/env; runtime db/env; realtime poll-manager/index/env.
**Interfaces:** Broker resolves a website project ID to a tenant-bound service lease; internal route authenticates the existing runtime shared secret and returns only the runtime role. Shared helper validates the response, builds verified TLS configuration, and replaces cached pools when credentials rotate.

- [ ] Test wrong-project/admin/non-TLS leases, concurrent refresh, expiration, and pool replacement.
- [ ] Route API SQL through platform credentials; runtime and realtime through runtime credentials. Preserve the explicitly configured development backend, with no hosted fallback.
- [ ] Initialize bookkeeping tables during engine creation; customer shell continues to use the customer role.
- [ ] Remove shared administrator requirements from hosted configuration/readiness. Disable unsupported destructive reprovision operations before they reach SQL.
- [ ] Run API/runtime/realtime/shared typechecks and focused tests, then staged vector CRUD and notifications against real engine projects.

## Task 3: Fresh website and cutover acceptance

**Files:** Website `infra/dokploy/compose.serverless-postgres.yml`, release evidence, existing database migrations.

- [ ] Mount public CA certificates, join verified private networks, and use the fresh control/Auth databases.
- [ ] Keep incomplete Auth enrollment unavailable; prepare and verify the new owner account.
- [ ] Build through CI and stage before domain cutover; prove login, project creation, scoped SQL/vector queries, runtime and realtime.
- [ ] Switch through Dokploy under the prior authorization, retain old storage for rollback, and verify public readiness and owner dashboard access.

## Checkpoint on 6 October

Private image `sprint3-engine-20261005-3` built successfully (GitHub run `37386068761`), digest `sha256:7f961e7f69db7f006cf977520ac8b66b1e2d48079edbcb86592833e6cb72f517`. Startup cache options and callback base URL were repaired in commits `24bb1f129` and `4efb94a65`.

The private server passed verified TLS customer login, plaintext/wrong-password/admin refusal, organization and tenant isolation, pgvector writes/search, compute resume, credential expiry, and password rotation. Customer backups restored to a separate synthetic engine tenant and retained independent writes; catalog restore and transaction rollback also passed. Root-only backup files are in `/var/backups/briven-engine-20261006/`. Full engine restart acceptance passed with persisted rows/vector search, three safekeepers and fsync enabled. No website cutover has occurred.
