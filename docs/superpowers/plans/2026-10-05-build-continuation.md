# Briven build continuation — 5 October 2026

**Goal:** Restore the owner's dashboard password login, then resume the September 30 private engine deployment plan.

**Architecture:** Keep the existing website and its control database operational. Repair only the explicitly named owner account using the deployed authentication library. Continue the private engine separately on the website's verified Dokploy server, with no public storage ports or website engine cutover.

**Tech stack:** Better Auth, PostgreSQL, Rust, GitHub Actions, Docker Compose, Dokploy.

**Specs:** `2026-09-27-briven-website-engine-deploy-design.md`, `2026-09-28-sprint-3-serverless-cutover-design.md`, and `2026-09-29-customer-database-home-design.md`.

## Constraints

- Owner account: `flndrn@hotmail.com`. Never record its password, password hash, or session token in repository files or reports.
- Preserve unrelated local edits and live customer data.
- Keep the public website on its existing stack while proving the private engine.
- Use the existing Briven Dokploy project and the server attached to website compose `LykAyuz6qInYZe37wIMLp`; validate that identity before deployment.
- Private engine: persistent storage, three disk-synced safekeepers, PostgreSQL catalog, and private MinIO.
- Do not expose engine administrator credentials. Leave customer connections unavailable until the TLS proxy actually works.
- Public engine cutover remains the separate final step of the original plan.

## Tasks

- [ ] Inspect the owner record and deployed superadmin allowlist. Verify password login; if necessary reset only that account using Better Auth's native hashing function. Verify a session, owner profile, admin authorization, and browser dashboard access.
- [ ] Correct the PostgreSQL submodule URLs while retaining the exact pinned commits. Confirm a clean GitHub checkout can fetch them.
- [ ] Build and publish a new immutable engine image. Resolve build failures and verify image contents before installing it.
- [ ] Validate the Dokploy target and create the separate private engine Compose service with dedicated volumes and securely generated settings.
- [ ] Complete the original private-room startup, catalog backup/rollback, proxy, isolation, and restart checks in order. Record evidence and stop on unresolved failures.

## Review focus

- An owner reset must never update another account or broaden the superadmin allowlist.
- Authentication proof must check protected routes, not only a successful sign-in response.
- Submodule source changes must not change the pinned PostgreSQL revisions.
- An image tag is deployable only after its build succeeds and runtime dependencies are verified.
- A healthy existing website is not proof that the new private engine works.
