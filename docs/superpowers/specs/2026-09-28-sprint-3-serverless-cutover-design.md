# Sprint 3 serverless Postgres cutover design

## Decision

Briven starts with empty hosted databases. No customer rows, accounts, or project
databases are imported from the legacy website deployment. Keep its volumes
offline for rollback until the new service has passed a restore drill; the new
runtime must not connect to them.

## Product boundary

The website API owns customer sessions, organizations, billing, and control keys
in a PostgreSQL control database. The Briven v2 engine owns customer Postgres
tenants, timelines, and compute. Each website project maps to exactly one engine
tenant. The website forwards authenticated project operations to the private
engine API with short-lived signed assertions. No development token or engine
administrator connection string crosses the public API boundary.

Customer computes run PostgreSQL with pgvector installed. Project creation enables
the `vector` extension and verifies a vector query before a project becomes
ready. The schema package accepts `vector(N)` and the runtime's vector search
operators use pgvector. Existing AI application features may use that database;
this sprint does not introduce a model hosting service.

## Hosted engine

Run storage controller, pageserver, at least three safekeepers, durable object
storage, a compute scheduler, a private control API, and a TLS Postgres proxy.
Keep metadata in a separate PostgreSQL catalog with versioned migrations,
backups, and restore verification. Persist storage and WAL outside ephemeral
containers. The compute scheduler starts or resumes a project compute and
reconciles actual engine state with catalog state after process restarts.

Issue a project-scoped Postgres role with a random, expiring credential. The
public connection endpoint routes only to that project's compute, requires TLS,
and never accepts the engine administrator role. Enforce project limits on
creation and compute start. Each lifecycle operation is idempotent or records a
recoverable failure state.

## Website cutover

Apply the existing website control schema to a fresh standard PostgreSQL
database, including organization control keys. Replace legacy project database
provisioning, SQL access, snapshots, realtime, and vector-column rejection with
Briven v2 project APIs or native PostgreSQL behavior. Remove the legacy database
service, environment variables, and engine-specific query paths from the active
compose. The website does not deploy until its route set is internally
consistent; unavailable features must be removed from the public UI and API
rather than left to fail against the new database.

## Verification and release

Test authorization, project isolation, key revocation, role expiry, migration
idempotence, provisioning recovery, pgvector SQL, and tenant routing. Perform a
fresh-deploy smoke test through the public service: create an account and
organization, create a project and branch, connect with a scoped TLS credential,
write rows and vectors, run similarity search, restart the control API and one
compute, and confirm data persists. Run catalog and tenant backup/restore drills
before declaring Sprint 3 complete.

Release the engine on a private network first, then switch the website to the new
empty PostgreSQL control database and engine gateway. Keep the previous service
stopped but intact during verification. Do not publish a development compose or
sample credentials.
