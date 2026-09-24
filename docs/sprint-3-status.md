# Sprint 3 Status: Briven Control API

Date: 2026-09-24
Status: In progress. Local engine workflow and organization-scoped catalog proven;
hosted customer API remains.

## Built

- Authenticated, loopback-only Briven control API in `briven_control_api`.
- Project creation against the real engine: tenant, main timeline, compute.
- Branch listing and copy-on-write branch creation.
- Branch compute creation, observed compute state, and branch connection URI.
- SQL readiness checks before a project is marked ready or a connection is returned.
- Organization-scoped API keys and project authorization on every project route.
- Optional local PostgreSQL catalog with unique project names per organization
  and durable provisioning states; legacy local JSON remains available.
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
- The six focused Rust API tests and local build passed.

The engine proof used a disposable local engine directory. The organization
isolation proof used a disposable local PostgreSQL catalog. No production
database or customer data was touched.

## Sprint 3 Still Open

- Replace manually configured organization keys with customer identity,
  organization membership, and key lifecycle management.
- Move the control catalog to hosted PostgreSQL with TLS, migrations,
  backups, and reconciliation. Existing JSON data has no automatic migration.
- Add recovery for partial provisioning and project/branch lifecycle operations.
- Issue project-specific database roles and secure credentials.
- Connect to hosted compute scheduling, TLS routing, limits, and operational controls.
- Prove project creation and connection from the public Briven service.

The local adapter is a working foundation for those steps, not a deployable paid
customer control plane.
