# Sprint 3 Status: Briven Control API

Date: 2026-09-28
Status: In progress. Local engine workflow, organization-scoped catalog, and
fresh PostgreSQL website database paths have local integration proofs. Hosted
compute and the public cutover remain open.

The hosted cutover will start with empty PostgreSQL databases. Existing website
data will not be imported. The website still has legacy database paths and is
not ready to switch production traffic to the v2 engine.

## Built

- Authenticated Briven control API in `briven_control_api`; static development
  keys require loopback, while signed customer mode can bind privately.
- Project creation against the real engine: tenant, main timeline, compute.
- Branch listing and copy-on-write branch creation.
- Branch compute creation, observed compute state, and branch connection URI.
- SQL readiness checks before a project is marked ready or a connection is returned.
- Organization-scoped API keys and project authorization on every project route.
- PostgreSQL catalog supports loopback plaintext for local development and verified
  Rustls TLS for remote hosts; the legacy local JSON catalog remains available.
- Organization-scoped catalog names and durable project provisioning states.
- A repeated create for the same organization and project name keeps one tenant.
  A finished name is rejected. An interrupted or failed create resumes the saved
  tenant and main timeline, and starts the compute only when it is not already running.
- Transactional, versioned PostgreSQL catalog migrations that preserve existing
  project rows and refuse unknown future versions.
- Signed customer mode requires a PostgreSQL catalog and enables and checks
  pgvector on new project and branch computes before reporting success.
- Signed, short-lived customer assertions verified by the engine; viewer access is
  read-only. The website API owns customer sessions, organization membership,
  and organization control-key creation, listing, rotation, expiry, and revocation.
- Free port selection so local compute creation does not collide with existing listeners.
- Briven-facing storage broker and status-check startup messages.

## Verified

- The API rejects an unauthenticated project request with HTTP 401.
- The API created project `sprint3-proof-5` and reported `ready`.
- It created a `preview` branch and started a branch-specific compute.
- Its returned preview connection executed `select current_database(), current_user, 1`,
  producing `postgres|cloud_admin|1` against PostgreSQL 17.
- When a compute was unavailable, the API withheld its connection URI.
- A live local PostgreSQL catalog returned only organization `alpha`'s project
  to its key; organization `beta` received 404 for that project, and a request
  without a key received 401.
- The focused Rust API tests and local build passed.
- `retrying_project_creation_reuses_one_tenant` passed on 2026-09-29. It checks
  the resume rule. A full engine restart drill was not rerun in that step.
- Remote catalog hosts now require a verified TLS connection; focused host-selection
  and hosted-DSN validation tests were added.
- The new organization control-key migration ran on a disposable PostgreSQL
  database. A live service-level check passed for one-time plaintext disclosure,
  cross-organization isolation, rotation, revocation, expiry, and soft-deleted
  organization rejection.
- The engine catalog migration ran twice against an existing project row in a
  disposable PostgreSQL database; the row survived and one version was recorded.
- pgvector provisioning compiles and the focused API tests pass; a live pgvector
  compute check is still required.
- The website control migrations applied to a fresh PostgreSQL database.
- A disposable PostgreSQL 17 instance with pgvector passed project provisioning,
  vector SQL, scoped role isolation, credential expiry, Studio schema inspection,
  Auth vault bootstrap, and committed table-specific realtime notifications.
- The website runtime accepts `vector(N)` and parameterized vector search. API,
  runtime, realtime, schema, and shared package checks passed on the cutover branch.
- A separate Dokploy compose candidate uses external PostgreSQL URLs and does
  not start a legacy database service; its Compose syntax passed validation.

The engine proof used a disposable local engine directory. The organization
isolation proof and key lifecycle check used disposable local PostgreSQL. No production
database or customer data was touched.

## Sprint 3 Still Open

- Prove customer session, member removal, key rotation/revocation, and project
  isolation against a live engine. The migration and key lifecycle have local
  PostgreSQL proofs; the engine bridge does not yet own website project
  provisioning. Static organization keys remain for local development only.
- Build real restore from engine branches. The draft website already hides
  the old snapshot doors and buttons: they answer that saved copies are not
  available and do not open a project database. That check passed on
  2026-09-29 (9 tests). The change is committed and pushed as `662ac51` on
  `sprint3-serverless-postgres`. It is not on `main` and not deployed. The old
  snapshot file remains, unused.
- Connect the website project lifecycle and SQL paths to engine tenants and
  customer computes. The current PostgreSQL adapter still provisions databases
  on a shared admin endpoint for local proof.
- Add catalog backup and reconciliation. Remote PostgreSQL connections use
  verified TLS. Existing JSON data has no automatic migration.
- Add hosted recovery for partial provisioning beyond the local same-name resume,
  plus project and branch lifecycle operations such as restore.
- Issue project-specific database roles and secure credentials.
- Connect to hosted compute scheduling, TLS routing, limits, and operational controls.
- Prove project creation and connection from the public Briven service.

The local engine adapter and remote TLS catalog are foundations for those steps,
not a deployable paid customer control plane.
