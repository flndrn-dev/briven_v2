# Engine customer access

## Goal
Use existing Briven customer sessions and organization memberships to authorize engine operations. Add organization-scoped control keys with one-time disclosure, expiry, rotation, and revocation. Keep the engine isolated behind the website API.

## Tasks
1. Add `org_control_keys` to the website API schema and migration. Store only SHA-256 key hashes and a display suffix; bind each key to one active organization and a viewer/developer/admin role.
2. Implement key creation, masked listing, atomic rotation, revocation, and lookup. Require an active owner/admin membership for management; reject expired and revoked keys.
3. Add an API gateway for the engine's project routes. Resolve the organization from the current customer session or an organization key, check membership for session calls, enforce read/write roles, and mint a 60-second HS256 assertion with a separate shared secret.
4. Verify the assertion in the Rust engine, including issuer, audience, lifetime, role, and organization. Enforce viewer read-only access and disable static development keys outside loopback development.
5. Add focused auth and lifecycle tests; build both repositories; document configuration and accurately update Sprint 3 status.

## Boundaries
The website API owns customer identity and key lifecycle. The engine owns project isolation. The gateway only forwards known project routes and does not expose the local engine directly to the public internet. This work does not imply completion of the other open Sprint 3 infrastructure tasks.
