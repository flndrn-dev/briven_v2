# Sprint 3 Status: Briven Control API

Date: 2026-09-24
Status: In progress. Local engine workflow proven; hosted customer API remains.

## Built

- Authenticated, loopback-only Briven control API in `briven_control_api`.
- Project creation against the real engine: tenant, main timeline, compute.
- Branch listing and copy-on-write branch creation.
- Branch compute creation, observed compute state, and branch connection URI.
- SQL readiness checks before a project is marked ready or a connection is returned.
- Project names and provisioning states persisted in the local engine directory.
- Free port selection so local compute creation does not collide with existing listeners.
- Briven-facing storage broker and status-check startup messages.

## Verified

- The API rejects an unauthenticated project request with HTTP 401.
- The API created project `sprint3-proof-5` and reported `ready`.
- It created a `preview` branch and started a branch-specific compute.
- Its returned preview connection executed `select current_database(), current_user, 1`,
  producing `postgres|cloud_admin|1` against PostgreSQL 17.
- When a compute was unavailable, the API withheld its connection URI.
- The Rust API test and local builds passed.

All proof data lived in a disposable local engine directory. Test services were
stopped afterward. No production database or customer data was touched.

## Sprint 3 Still Open

- Replace the single operator token with customer identity and organization scoping.
- Move project metadata to a transactional control database with reconciliation.
- Add recovery for partial provisioning and project/branch lifecycle operations.
- Issue project-specific database roles and secure credentials.
- Connect to hosted compute scheduling, TLS routing, limits, and operational controls.
- Prove project creation and connection from the public Briven service.

The local adapter is a working foundation for those steps, not a deployable paid
customer control plane.
